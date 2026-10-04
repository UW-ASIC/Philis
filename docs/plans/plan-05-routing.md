# Plan 05 — Routing (RTE): exact, current-aware, parasitic-driven analog routing

Plan ID prefix: `RTE-NN`. Written 2026-09-28 against the working tree (HEAD `dad330c` plus uncommitted changes).

Inputs read completely: `docs/plans/audit-05-routing.md`, `audit-02-analog-rules-and-core.md`, `ref-lampaert-performance-driven-layout.md`, `ref-lienig-electromigration.md`, `ref-hastings-15-ch15-assembling-die-and-appendices.md`, `ref-hastings-05-ch05-failure-mechanisms.md`, `ref-hastings-14-ch14-special-topics.md`, `ref-surveys-state-of-the-art.md`, `ref-balasa-graeb-survey-part2-ch04-07.md`, `ref-common-centroid-papers.md`, `ref-00-prior-notes-handbook.md`. The other study docs were searched for routing items (AA-11/13/14, AC-05/06/07/17, AP-02/17, AV-06/07/09/11/15/17, AF-05/06/12/24/25/35, H13-36/51/52, H08-20..29, H06-14/18, MM-33, SUB-30/43/44).

Code read for this plan (by me, not only through the audits): `backend/gr/src/lib.rs` 1–1180 (all code, test names), `backend/dr/src/lib.rs` 1–2230 (all code, test helpers and names), `kernel/analog/src/routing/{mod,differential,em,shield,coupling,crosstalk,common_node,ir,stack}.rs` (all non-test code), `kernel/core/src/{routes,report,layout,units,macro}.rs` (types), `kernel/analog/src/rule.rs` 60–210, `frontend/library/src/elaborate.rs` 150–460, `frontend/library/src/lib.rs` 255–345, 600–720, 755–810, 985–1060, `backend/annotator/src/extract.rs` 1–127, `backend/verify/src/pdk.rs` 330–440, 515–560, 690–740, `kernel/cells/src/mosfet.rs` 452–484, `frontend/library/src/cellgen.rs` 335–380, 937–966, `benchmarks/tests/signoff_fixtures.rs` 1–45, `frontend/library/tests/ota_cross_pdk.rs` 188–232, `pdks/sky130.json` 1–135, and the pinned GPurify deck `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/pdks/sky130.deck` lines 348–404, 491–495, 573–598.

Conventions:
- `path:line` is relative to the repo root. `sky130.deck:N` is the pinned GPurify deck above.
- Reference cites give the study-doc entry ID, the source section/equation, and the reftext line range (`hastings.txt`, `lampaert.txt`, `lienig_em.txt`, `balasa_graeb_survey.txt`, `todaes22.txt`, `perf_driven_survey.txt`, `cc_dac_constructive.txt`, `cc_review.txt`).
- A number marked **derived** was computed by hand from deck rules and the formula cited. It was not produced by running the tool.
- A constant marked **tuning default** is a Philis choice with no source value; it is named so it can be calibrated.
- No cargo command was run for this plan.

---

## 0. Scope & boundaries

### 0.1 What this plan owns

| Area | Files |
|---|---|
| Detailed router | `backend/dr/src/lib.rs` (all) |
| Search core (lattice, PathFinder, geometry) | `backend/gr/src/lib.rs` (the `GlobalRoute` coarse router is retired by RTE-07; the shared core stays) |
| Routing rule types and their measurements | `kernel/analog/src/routing/*.rs`: CommonNode(s), CouplingBudget, CrosstalkExclusion, Differential, Shield, the Stack measurement helpers they use, new `metal_over_gate.rs`, new `plate_ratio.rs`. **Not RTE:** the EM, IR and antenna models (`em.rs`, `ir.rs`, `Stack::antenna`, new `current.rs`) are REL-01..04 (plan-06 §0.1); `PerformanceBudget` **measurement** (performance.rs, RTE-21 step 3) and `ParasiticBudget` cost (RTE-32) are RTE; its row construction is EXT-25 and its sensitivity data PERF-12 |
| Router configuration from the deck | `frontend/library/src/elaborate.rs`: `routing_stack`, `detailed_router`, new `blockages` (FLOW owns the rest of the file) |
| Routing glue in the flow | `frontend/library/src/lib.rs`: the route step of `Flow::epoch` (lib.rs:614–652), `common_nodes` (763–801, shared with MAT-06 and EXT-24). FLOW owns the file and the epoch/history lifecycle (FLOW-08); REL owns `em_rules` (991–1028) and `pin_currents` (1035–1055) (REL-01, REL-03) |
| One core function | `kernel/core/src/routes.rs`: `Routes::debug_check_joined` (RTE-03). `Routes::terms` and `Routes::gates: Vec<Vec<GatePin>>` are REL-03 / REL-02 |
| Tests | `backend/dr`, `backend/gr` unit tests; new `frontend/library/tests/dr_deck_drc.rs` |

The owned topics are: symmetric/mirrored differential routing with matched R and C; matched-parasitic routing for capacitor (CC) arrays; shielding and spacing for sensitive nets; crosstalk avoidance; Kelvin sensing and star/tree supply routing; current-aware wire and via-array sizing; parasitic-budget-driven routing; antenna-aware layer assignment; pin access; DRC-clean detailed routing; routing runtime.

### 0.2 What this plan consumes

Every input has a fallback that keeps today's behaviour, so RTE items can land before their producers.

| Producer | Data | Shape RTE consumes | Used by | Fallback until delivered |
|---|---|---|---|---|
| REL-01 | Per-pin current shares (finger adjacency, else `min(1, 2/n)`); unknown device currents stay `None` | `DetailedCfg::pin_ua: Vec<Vec<(String, Option<i32>)>>` × `pin_shares` (plan-06 REL-01 steps 1–2) | RTE-14, 24 | Today's whole-terminal value (over-counts, AT-07) |
| REL-02 | Antenna model: gate area per piece, deck-only diode credit, `known` | `Routes::gates: Vec<Vec<GatePin { at, dev, nm2 }>>`, `Stack::antenna(shapes, cell, gates, fallback_nm2)` | RTE-06 | Today's whole-net model (AR-13) |
| REL-03 | Routed terminals with DC currents; per-shape currents; hard per-shape/per-cut-group EM rule incl. pin-access layer and cut | `Routes::terms: Vec<Vec<Terminal { at, ua: Option<f32> }>>` (dr fills it), `routing::current::net_flow(stack, shapes, terms) -> Option<NetFlow { shape_ua, drop_uv }>`, `Electromigration { net, limits: [(u16, Limit); MAX_LAYERS], stack }` | RTE-05, 14, 17, 24, 27 | None: RTE-14/24/27 wait for REL-03 |
| REL-04 | Terminal-resolved IR drop on the same flow | `IrDrop::drop_uv` from `NetFlow::drop_uv` | RTE-14, 21 | Today's worst-path bound |
| REL-12 | Effective cuts of a via group by current direction | `em.rs` front-row count | RTE-27 | Group size |
| REL-17 | ESD width floor per declared net | `Electromigration::esd_area_um2` | RTE-14 (as a `k` floor) | none |
| EXT-09 | `Differential` in the **budget** arm, from recognised DiffPair leaves | `reqs.budget` | RTE-05, 09, 15 | Today's hard `Differential::extract` pairs |
| EXT-24 | Matched net pairs of compounds; `CrosstalkExclusion` by net class; `shield_ref` per victim; `CommonNodeReq`, `StarReq { net, root, branches }`, `KelvinReq { device, term, sense }`; `em_critical`; `RcClass` per net incl. DAC plates | `Intent` (plan-01 EXT-12/24) | RTE-02, 14, 15, 17, 18, 19, 20 | Today's 2-device common nodes (lib.rs:780–785), ground shields (extract.rs:87–104), RTE-14's own EM filter |
| EXT-18 | Net classes incl. Noisy / DigitalSwitching / Reference (AA-11, H15-27/28) → per-net aggressor weight in [0, 1] | `&'static [f32]` indexed by `NetId` | RTE-02, 05, 16, 18 | `CouplingBudget::default_weights` (RTE-02): 0 for Supply/Ground/Substrate, 1 otherwise |
| EXT / CELL | Capacitor plate sets (CC-14) | `PlateSet { top, bits: [(net, units)], c_unit_af }` | RTE-20 | Derived from `cellgen::dac_banks` (cellgen.rs:760, made `pub(crate)`), RTE-20 step 1 |
| CELL-01 | What each macro drew that metal must avoid | `Macro::keepouts: Vec<Keepout { rect, why: KeepWhy::{Gate{owner}, ResistorBody{owner}, CapPlate{owner}} }>`, moved by `place_macro` | RTE-16 | Gate rects = cell `poly` ∩ `diff` role shapes (RTE-16 step 1) |
| MAT-07 / FLOW-06 | Match class per matched set | `pnr_core::MatchClass { Minimal, Moderate, Exceptional }` on `Unitization.class` (default `Moderate`) | RTE-16 | `Moderate` for every matched cell (open question 1) |
| MAT-06 | Remaining offset allowance per pair | `CommonNode.max_delta_ohm` from `offset_allowances` | RTE-17 | Today's `systematic_allowance_mv` path (lib.rs:792–797) |
| PLC-28 | Symmetry axes on the routing lattice (contract in RTE-15 step 3) | `Layout::axis[a] ≡ p0/2 (mod p0·max(1, s_max/2))` | RTE-15 | Joint routing falls back to the mirror guide |
| EXT-25 / PERF-06 / PERF-12 | Budget rows (EXT-25 builds them, adverse direction only, BAL2-16) and the router weights (PERF-12 step 2 `perf::router_weights` writes `cfg.r_weight`, `cfg.pair_weight`) | `PerformanceBudget` fields `r_nets`, `r_weights`, `diff_pairs`, `diff_weights` (EXT-25 step 3), `coupling: Vec<(NetId, NetId, f32)>` (PERF-12 step 3), `limit: f32` (PERF-06 step 2); RTE-21 step 3 **measures** all of them (the struct is RTE-owned, performance.rs:19–27) | RTE-21, 22 | `net_weight` as today (lib.rs:312–316); `r_weight`, `pair_weight` empty; new row fields empty |
| PERF-28 | Top-η simulation of alternative width vectors (SURV-33) | calls `dr::width_candidates` | RTE-22 | Linear-model choice only |
| FLOW-08/09 | History lifecycle (reset per cold epoch, one accumulation per epoch), `RunStats.stage_ms`, Θ/|V| accounting (FLOW-02) | — | RTE-08, 13 | — |

### 0.3 What this plan provides

| Consumer | Data | Item |
|---|---|---|
| PLC (PLC-15, PLC-28) | `LatticeSpec { p0, strides, origin_multiple }` for axis snapping; `RouteStats::congestion` per region (replaces the `gr::OverflowMap` PLC-15 names, because RTE-07 retires `gr`'s coarse router) | RTE-15, RTE-25 |
| PERF, benchmarks | `RouteStats` per-stage timers inside dr, expanded nodes, single-cut via count (FLOW-09 owns `RunStats.stage_ms`) | RTE-13, RTE-27 |
| PERF-28 | `dr::width_candidates`, `DetailedCfg::width_mult` | RTE-22 |
| EXT-25, PERF-06/12 | `PerformanceBudget::used` measuring ground C (union), series R, pair ΔC and coupling, with `limit`; `DetailedCfg::{r_weight, pair_weight}` fields for PERF-12 to fill | RTE-21 steps 1, 3 |
| everyone reading `Stack::ground_af` | Ground C on the per-layer union with full-perimeter fringe, as GPurify extracts | RTE-32 |
| FLOW-11 | `routing_stack` returning `Result` and taking a top-metal cap (AF-35) | RTE-11 |
| REL (REL-02 risk note) | Antenna repair that fixes what REL-02's model reports, on the drawn geometry | RTE-06 |
| MAT, EXT, verify | Honest measurements: Differential terminal-resolved RC mismatch, CommonNode ΔR and star topology, metal-over-gate area, per-bit plate lead C | RTE-05, 16, 17, 20 |

### 0.4 Not in this plan

Net classification, matched-set and star/Kelvin detection (EXT); match classes and offset budgets (MAT); axis placement, routability-aware placement and halos (PLC); the EM/IR/antenna models and checks — per-pin current shares, `Routes::terms`, `net_flow`, the hard per-shape EM rule, terminal-resolved IR, gate area per antenna piece, diode credit — plus EM limit values, derating, mission profiles, RMS/peak data, latchup, ESD networks, voltage-aware spacing data (REL, with FLOW for deck keys); `PerformanceBudget` row construction (EXT-25), sensitivity computation and router-weight export (PERF-12), the simulation loop and the hand-layout benchmark (PERF); cap-array internal routing, MOS strap sizing and in-cell shield plates (CELL); deck grammar, sidecar keys, CLI, Θ accounting, PathFinder history lifecycle across epochs, `docs/CRATES.md` (FLOW). The antenna diode sizing (AF-35) stays with FLOW/REL; RTE fixes only the antenna repair on drawn geometry (RTE-06).

---

## 1. Audit digest (current state, confirmed in the code)

### 1.1 Where routing runs

- `Flow::epoch` routes every epoch: `gr::GlobalRoute::route(...)` whose `Report` is discarded (`let (global, _)`, lib.rs:617–618), then `dr::DetailedRoute::route` (lib.rs:626–631). If an `Antenna` hard rule is still violated, `elaborate::antenna_diodes` places diodes and **dr runs again from scratch** (lib.rs:635–652). One `gr::Negotiation` lives for the whole solve (lib.rs:342).
- Router configuration is fixed per solve from the deck: stack (lib.rs:279), EM limits at one die temperature (lib.rs:280, elaborate.rs:248–262), supply nets, pin currents and net weights (lib.rs:300–316). The router-only loop `elaborate::route_built` repeats the gr→dr pair (elaborate.rs:166–190).

### 1.2 Configuration and the derived sky130 lattice

- `routing_stack` keeps routing metals `take_while(min_width ≤ first-via pad)` and always splits the bottom metal off as pin access (elaborate.rs:210–219). On sky130 the first via is mcon: pad 290 (audit-05 §1.1, derived), met3 `m3.1` width 300 (sky130.deck:360–362) stops the take, so **only met1 and met2 are routed**; gf180 and ihp lose metal1 (audit-05 table, derived).
- One wire width for every layer = `max(first-via pad, max min_width)` = 290 on sky130 (elaborate.rs:398); one pitch `routing_pitch(max(wire, largest pad)) = 320 + 140 = 460` (elaborate.rs:399, pdk.rs:423–433; derived). Pads are squares at the larger of the symmetric and asymmetric enclosure (pdk.rs:354–358), although Hastings' via-to-via pitch uses "the smallest overlap allowed for two opposite sides" (H15-20, Eq 15.19, hastings.txt:48196–48219).
- Fatten caps `fat_signal = 2·wire`, `fat_supply` = 2·grid under the smallest, over the routed layers, of each layer's largest wide-metal threshold (3 µm − 10 nm on sky130) (elaborate.rs:429–435). Electrical step costs `layer_c`, `beside_c`, `layer_r`, `via_r` normalised to the cheapest layer (elaborate.rs:436–451).

### 1.3 `gr` (backend/gr/src/lib.rs)

- 2-D `N×N` gcell grid, `N = 16`, capacity 6, 40 iterations, p_fac 3, history 0.5 (gr:23–35). The frame spans the origin to the far corner (`die_extent`, gr:361–368; `gcell_terms(.., (0, 0))`, gr:186), so pins at negative coordinates saturate into gcell 0 (gr:474–475).
- Output is 1-nm centre-line shapes on `layers[0]` (gr:207–219, 371–374). The only consumer is dr's frame extent and corridors (dr:216, 238–242, 432–451), which restrict only the first `CORRIDOR_EPOCHS = 8` of 150 iterations (gr:1013, 1032).
- History keyed by absolute 500 nm buckets, `max`-folded, never decayed (gr:43–92, 55). p_fac constant across PathFinder iterations; `moved` is true on any reroute, even an identical one (gr:1019–1043).
- `price_group` (gr:108–133) routes an obstacle-free grid, so `reachable` is always true; `cellgen::price` still uses it (cellgen.rs:360, 374), and `seed_assignment` builds its `gr::GlobalCfg` (cellgen.rs:332).
- `route_net` (gr:882–1010): multi-source Dijkstra from the growing tree, targets sorted by Manhattan distance from the root (gr:943–948), no A* predictor. Node cost = history + p_fac·overuse + penalty + `Elec` + `KEEPOUT_COST` 2.0 over a foreign matched cell (gr:936–941, 757). `Elec` prices `weight·layer_c + beside_c·Σ(weight + neighbour sens)` and `current·layer_r` (gr:908–927); via C is not priced and R is at the drawn width (gr:853–854 ponytail).
- `TrackGrid` (gr:515–661): capacity 1, one pitch, even layers horizontal and odd vertical with no wrong-way edges (gr:630–638), pitch silently raised to `max(die)/1200` (gr:528). Landing candidates on layers 0–1 only, with a fixed ±2-row stacked-via exclusion (gr:586, 595–599).

### 1.4 `dr` (backend/dr/src/lib.rs)

- Pins are landed first-come in first-seen order (dr:196–204, 360–431). The frame origin is `low − halo − margin` with no alignment to any symmetry axis (dr:213–218).
- **Current over-count:** `pin_ua` returns the whole terminal current for every pin with that name (dr:227–232), and a MOSFET emits one `d{k}:S`/`d{k}:D` pin per diffusion region (mosfet.rs:458–481). `branch_currents` sums them (dr:897–934), so trunks carry ≈ (number of regions)·I (AT-07).
- Differential nets route blind to parasitics (`plain`, dr:469–472). Matched-cell keep-out is the whole bbox, soft, foreign nets only (dr:476–498).
- **Foreign metal is soft:** ring bands and cell metal are charged `usage = max(1)` (dr:507–524, 1153–1162). `cross_net_shorts` compares only routed wires (dr:2092–2113), so a trunk over cell metal is reported only as Θ overuse (dr:1855–1857).
- IR-broken nets reroute with series R priced (dr:531–550). Rule repair (dr:1410–1515): Differential → `copy_tree` (exact bins, dr:1643–1691) or `mirror_guide` (1.0/node, axis = mean terminal x, dr:1697–1718); CommonNode → Manhattan balance field (dr:1792–1805); crosstalk/coupling → 8-neighbour `COUPLE_COST` (dr:1807–1822); Antenna → jumper per layer, lift, plain (dr:1486–1498); everything else → blind reroute at doubled p_fac (dr:1499–1505). A new `Dij` is allocated per trial (dr:1535). Repair is judged on `probe` = pre-access, pre-fattening geometry (dr:555).
- Shields are added after repair on free stretches, all-or-nothing (dr:585–596, 1556–1621).
- Geometry: access jogs (dr:612, 1174–1317) may be "narrow" = `min(wire, pin.w, pin.h)` (dr:384, 1209); shorts are broken by deleting shapes in index order (dr:617–635, first_short O(S²) in a loop, dr:2075–2088); trunks are fattened afterwards, ~270 width candidates per segment each scanning all foreign shapes (dr:637–715); via arrays fill the overlap with equal sharing assumed (dr:717–806); access cuts are always one cut and never EM-checked (the pin-access cut, e.g. mcon, is not in `cuts`, so the array loop pushes it unchanged, dr:743–745; an access cut between stack layers sees no tree-edge current, `cuts(0)` = 1, dr:748–749; access metal is past `pre_access`, dr:654); the lighter side of a pair gets a dangling stub (`trim_pair`, dr:808–817, 944–990); same-net slivers/notches are filled (dr:819–877).
- Report: open nets by layer-blind xy touch (dr:2117–2135), sacrificed pins and drawn shorts in V; EM underwidth and EM cuts and overuse in **Θ** (dr:1852–1857, 882–884).

### 1.5 Routing rules (kernel/analog/src/routing)

- `Electromigration` passes if **any one** segment is wide enough for the largest single-terminal current (em.rs:109–120, 128–130; lib.rs:991–1028) — the weak test is hard, dr's real per-segment test is Θ (AR-05). `Limit::width_nm` and `cuts` are correct (em.rs:48–70) and derating is correct but inert without deck (T_ref, Ea, n) (em.rs:16–21, pdk.rs:542).
- `IrDrop` charges the whole net current across a BFS-tree diameter (ir.rs:32–36, stack.rs:94–146); the terminal-resolved `fed_resistance_ohm` exists unused for IR (stack.rs:167–177).
- `Differential` is registered **hard** (extract.rs:53–54) at 5 %; stack mode compares ground C and the runs' **summed** R with pads and cuts dropped (differential.rs:55–65, runs at :63), which `trim_pair` games; no coupling term; no `known`; extracted from `is_diff_pair` with no W/L/model check (AR-39).
- `Shield` coverage is `min(covered_below, covered_above)` instead of the intersection (**bug**, shield.rs:59).
- `CouplingBudget` sums every other net's same-layer 1/gap coupling, including the victim's own shield reference and the rails (coupling.rs:49–68; extract.rs:78–104), with no screening and no inter-layer term (AR-07, AR-14).
- `CrosstalkExclusion` is a point clearance, spacing 0 until the annotator sets `route_space × multiple` (crosstalk.rs:21–35; extract.rs:57–60).
- `Stack::antenna` charges a piece the whole net's gate area and grants diode credit from the first stage (stack.rs:280–291).
- `CommonNodes` measures |R_a − R_b| from feeds on the port graph (common_node.rs:41–53), but only for 2-device DiffPair/CurrentMirror/Load leaves (lib.rs:780–785); no star/Kelvin topology exists.

### 1.6 Bugs and placeholders to fix first

1. Multi-region current over-count (AT-07) → **REL-01** (consumed; RTE-01 is cut).
2. Shield coverage `min` instead of intersection (AR-06); coupling counts the victim's own shield (AR-07); `CrosstalkExclusion` has no `known` (AR-44) → RTE-02.
2a. `Stack::ground_af` sums overlapping shapes (a pad on a wire counts twice, stack.rs:52–64 ponytail) and uses long sides only for fringe while GPurify uses the union's full perimeter; `ParasiticBudget::cost` is `len²·1e-6` even when the budget is C (AR-23); four copies of the rect-touch predicate (AR-20) → RTE-32.
3. Soft foreign metal; shorts to cell metal invisible; layer-blind opens; overuse as Θ (AT-05, AT-28, AT-29) → RTE-03.
4. Inverted EM tiers, access geometry never EM-checked, worst-path IR (AR-05, AT-06, AR-21, AF-24) → **REL-03, REL-04** for the checks (RTE-04 is cut); RTE-14 makes dr size to them.
5. Differential hard at 5 % (→ EXT-09), summed-R metric gamed by a dangling stub, no coupling, no `known` (AR-12, AT-20, AR-44, AR-39) → RTE-05.
6. In-loop antenna disagrees with signoff: `dac4` fails `erc/ar.met2.1` at `feedback_iters = 1` (AV-15, signoff_fixtures.rs:139) [UNVERIFIED: a runtime result from audit-06's G.3 run; no cargo test was run by the fact-check]; whole-net gate area per piece (AR-13) → **REL-02** for the model; RTE-06 for repair on the drawn geometry.
7. gr frame anchored at the origin, report discarded, meaningless capacity, `reachable` always true (AT-08, AT-09, AT-32) → RTE-07.
8. History never decays across placements; dr runs twice per epoch with antenna diodes and accumulates history twice (AT-10, AT-34, AF-06) → **FLOW-08** (lifecycle); RTE-08 fixes the gr-side keys and PathFinder convergence (AT-11).
9. `kind()` string-suffix dispatch (AT-31) → RTE-09.

---

## 2. Target — what "better than hand layout" means for routing

A hand layout of an analog block is DRC/LVS clean, routes matched nets as exact mirror images, keeps metal off matched gates, sizes current paths wide and short with via arrays, returns matched currents through star or Kelvin connections, and shields or separates sensitive nets (Hastings §15.4.4, hastings.txt:48502–48843; perf_driven_survey.txt L80–91). The router beats it when it does all of that **and proves it** on every net of every candidate, and when it spends parasitics where the circuit's sensitivities say they cost least (Lampaert eq 5.10, lampaert.txt:6099–6178; TODAES Table 11, todaes22.txt:895–920).

| # | Metric | Hand-layout reference | Today | Target | Measured by |
|---|---|---|---|---|---|
| T1 | Routing-layer DRC findings | 0 (signoff) | 0 on 7 small sky130 fixtures; excluded as "dr debt" in `ota_cross_pdk`, which runs only sky130 and generic_finfet (ota_cross_pdk.rs:196–224, 338–358); no test gates gf180/ihp routing DRC | 0 on every fixture × {sky130, gf180mcu, ihp_sg13g2, generic_finfet} | `signoff_fixtures`, `ota_cross_pdk`, new `dr_deck_drc` (RTE-26) |
| T2 | Router V entries `open`, `drawn short`, `pin access sacrificed`, `unresolved congestion` | 0 | shorts to cell metal invisible (AT-05) | 0 | dr `Report` |
| T3 | Routed metal layers (sky130) | all layers the block needs | 2 (met1, met2; derived) | 4 (met1–met4; derived) | `routing_stack` unit test (RTE-11) |
| T4 | Track pitch met1/met2 (sky130) | via-to-via pitch Eq 15.19: 400 / 420 nm (derived) | 460 on both (derived) | base pitch 420, met3/met4 at 840 (derived) | lattice unit test (RTE-10) |
| T5 | Matched pair route mismatch | "Exact routing": "the pair of wire lengths be the same for every layer" (perf_driven_survey L86–89) | soft guide + stub that equalises summed R only (AT-20) | 0.0 % per-layer signature and via count when the PLC axis contract holds; otherwise terminal-R and ground-C mismatch ≤ 1 % and per-aggressor coupling asymmetry ≤ 1 % of pair C | `Differential` (RTE-05, RTE-15) |
| T6 | Routed metal over MOD/EXC matched gate area | none (Hastings rule 17, hastings.txt:42603–42613) | soft cost, foreign nets only (dr:476–498) | 0 µm² on the final GDS | `MetalOverGate` rule (RTE-16) |
| T7 | DC EM: shapes/cuts over limit, including access jogs and pin cuts | "a lead must meet electromigration rules at every point along its length" (Hastings, hastings.txt:48739–48744) | best-segment hard rule; per-segment check in Θ; access unchecked | 0 V; `unknown` only without an operating point | REL-03 `Electromigration` (check), RTE-14 (dr sizes to it) |
| T8 | IR drop | per-terminal budget | whole current × worst path | every `IrDrop` budget met | REL-04 `IrDrop` (check), RTE-14/21 (repair) |
| T9 | Matched returns | star node / Kelvin (Hastings Figs 15.26–15.27, hastings.txt:48509–48549) | ΔR only | ΔR ≤ allowance for 100 % of common nodes; 100 % of requested stars/Kelvins pass the topology check | `CommonNodes` (RTE-17) |
| T10 | Coupling onto sensitive nets | ΔV = C·R·dV/dt budget (Hastings Eq 15.23, hastings.txt:48574–48595) | same-layer, all pairs, self-shield counted | weighted C (lateral with screening + crossings) ≤ budget; router estimate within 20 % of GPurify PEX on the OTA sensitive nets | `CouplingBudget` (RTE-18) |
| T11 | Antenna | 0 | red on dac4 at 1 feedback iteration (AV-15) | 0 at any iteration count | `signoff_fixtures` (REL-02 model + RTE-06 repair) |
| T12 | Performance-driven width | designer R/RC annotation loses to automatic sizing (TS-OTA2 FOM 0.92 vs 0.74, todaes22.txt:897–904) | EM/IR-only widening | post-layout FOM ≥ max(uniform 1×, uniform 2×) on PERF's OTA testbenches | PERF bench (RTE-22) |
| T13 | Runtime | — | unmeasured per stage | per-stage timers reported; dr p50 per call on `ota` ≤ ⅓ of the M0 baseline (p50 over every dr call of one `bench local` run of `ota`, seed 1) | `RouteStats` (RTE-13); `RunStats.stage_ms` (FLOW-09) |

---

## 3. Work items

### RTE-01 (cut — duplicated by REL-01)

- Moved to "Cut or deferred items", C1. REL-01 (plan-06) delivers the same fix with an exact per-finger share (`pnr_core::macro::pin_shares`, finger adjacency from `Macro::units`, bound `min(1, 2/n)` elsewhere) instead of this item's equal `I/k` split, changes `DetailedCfg::pin_ua` to `Vec<Vec<(String, Option<i32>)>>`, and carries the test `a_multi_finger_terminal_is_not_counted_per_pin`.
- RTE items that need a pin's own current (RTE-14 access sizing, RTE-24 ampacity filter) read `table value × pin_shares[pin index]` from REL-01. The ID is kept so cross-references stay valid.

### RTE-02 Shield coverage by interval intersection; the victim's shield is not an aggressor; crosstalk `known`

Status: done in M0 (`de602df`).

- Priority: P0. Effort: S. Depends on: none.
- Why: AR-06 (bug), AR-07, AR-44 (CrosstalkExclusion reads an unrouted pair as satisfied), AR-35 (crosstalk.rs has no test), NOTES-45; BAL2-10 ("not to worsen the effects of crosstalk, the shielding wire must be connected to a dc potential or it must be grounded", balasa_graeb_survey.txt:8971–8979; that the shield then does not count as an aggressor is this plan's inference).
- Current:
  - `Shield::coverage` adds `side(true).min(side(false))` per victim run (shield.rs:59): a reference strap below on [0, 5 µm] and above on [5, 10 µm] reads 50 % although no point is double-shielded.
  - `CouplingBudget::total_af` sums every other net (coupling.rs:55–66). For Sensitive nets in a design with a clock the annotator emits both a ground `Shield` at ≤ 2·route_space and a `CouplingBudget` (extract.rs:78–104), so the shield the rule asks for is booked as coupling (≈ 43 aF/µm/side at 280 nm per AR-07).
- Change:
  1. `shield.rs`: `side(below)` returns the merged, clipped intervals `Vec<(i32, i32)>` instead of a length; `covered += intersection_len(&below, &above)` (two-pointer sweep over the two sorted lists, O(a + b)).
  2. `coupling.rs`:
     ```rust
     pub struct CouplingBudget {
         pub net: NetId,
         pub max_coupling_af: i64,
         pub margin_pct: u8,
         pub stack: Option<&'static Stack>,
         /// The victim's shield reference: never an aggressor.
         pub exclude: Option<NetId>,
         /// Aggressor weight in [0, 1] by NetId; a missing index reads 1.0; None = all 1.0.
         pub aggressor_weight: Option<&'static [f32]>,
     }
     impl CouplingBudget {
         /// `vec![1.0; n_nets]`, then 0 for nets whose `NetClass` is Supply, Ground or
         /// Substrate (analog::metadata::NetClass, metadata.rs:18–26; AR-07's "quiet rails").
         pub fn default_weights(classes: &[NetClassification], n_nets: usize) -> Vec<f32>;
     }
     ```
     `total_af` skips `other == exclude` and multiplies each pair term by `weight(other)`.
  3. Coordinated edit in the annotator (EXT's file, extract.rs:78–104): where a victim gets both a `Shield` and a `CouplingBudget`, set `exclude: Some(shield.reference)`; set `aggressor_weight: Some(Box::leak(CouplingBudget::default_weights(classes, n_nets).into_boxed_slice()))` once per `annotate` call. EXT-24 step 2 later supplies `Intent.nets[victim].shield_ref` and EXT-18's classes; the fields stay the same.
  4. `crosstalk.rs`: `fn known(self, r: &Routes) -> bool { !r.shapes(self.a).is_empty() && !r.shapes(self.b).is_empty() }` (AR-44; `Rule::known` default is `true`, rule.rs:59).
- Tests:
  - `shield.rs`: `complementary_halves_cover_nothing` (below [0, 5] µm, above [5, 10] µm on a 10 µm run → 0.0); `overlapping_halves_cover_the_overlap` (below [0, 7], above [3, 10] → 0.4 ± 1e-6).
  - `coupling.rs`: `the_victims_own_shield_is_not_an_aggressor`: 10 µm victim with reference tracks both sides at 280 nm; without `exclude` total = 2·12·10 000/280 = 857.1 ± 0.5 aF (fallback `EPS_H_AF`); with `exclude = Some(reference)` total = 0.0; `a_quiet_rail_weighs_nothing` (rail weight 0 → 0.0); `default_weights_zero_only_the_rails` (classes Supply net 0, Signal net 1, Ground net 2, `n_nets = 4` → `[0, 1, 0, 1]`).
  - `crosstalk.rs` (first tests in the file): `an_unrouted_pair_is_unknown` (net `b` has no shapes → `known == false`); `the_floor_is_an_edge_gap` (two 260 nm-high met1 wires on one layer, edge gap 280 nm, `min_spacing_nm = 280` → satisfied, residual 0; gap 270 → violated, residual `over(10, 280)`).
- Acceptance: no Θ entry from a `CouplingBudget` whose only aggressor is its own shield on any fixture; no hard `CrosstalkExclusion` counts as satisfied with an unrouted net; all existing shield/coupling tests pass.
- Risks / notes: `aggressor_weight` is `&'static` because `Rule` values are `Copy` (rule.rs:9–130), the same pattern as `Stack`.

### RTE-03 Foreign metal is a hard obstacle; shorts, opens and congestion are measured as violations

Status: not merged through review; the code is on m0 as `ba49ae2` (ported outside the item review) with acceptance 3 not met. Owner decision open (keep with the residual congestion handed to RTE-08, or revert); see the Status note under Acceptance and 00-MASTER-PLAN Status. M1: `ba49ae2` stays and RTE-08 met acceptance 3; the decision is still not recorded (§6.4 Q10, default keep).

Field report: FR-6, see RTE-08 (the residual shorts and congestion are this item's measure and RTE-08's convergence).

- Priority: P0. Effort: M. Depends on: none (RTE-13's `ShapeIndex` speeds it up later).
- Why: AT-05, AT-28, AT-29, AR-31; audit-05 §2.2 items 2–4.
- Current:
  - Ring bands and cell metal on routing layers are charged `usage = max(1)` (dr:507–524, `charge_rect` dr:1153–1162). A trunk left on them is a physical short or spacing error. The ring comment's concern ("a hard block would seal the enclosure on a two-layer stack") only applies when a ring has bands on every routed layer.
  - `cross_net_shorts` compares routed wires only (dr:2092–2113); cell and ring metal are not in `routes.wires`.
  - Residual overuse goes to Θ `routing overuse` (dr:1855–1857).
  - `unreachable_shapes` joins shapes on any two layers that overlap in xy (dr:2117–2135), so an open between non-adjacent layers is invisible.
  - The short resolver deletes the first touching shape in `(a, b, i, j)` order (dr:617–635).
- Change:
  1. `const BLOCKED: u32 = NONE - 2;` a `reserved` owner that matches no net and is never a landing site (unlike `CONTESTED`, dr:35).
  2. After pin reservation (dr:344–358), in place of the two `charge_rect` loops (dr:507–524), for every ring or cell shape on lattice layer `l`: grow the rect by `g = cfg.space(l, min(s.w, s.h), wire_l, S_l) + wire_l/2` (`DetailedCfg::space(layer, a, b, fallback)`, dr:133–139; before RTE-10 `wire_l = cfg.wire_width` and `S_l` = the layer's `spacing` minimum) and mark every covered on-track node of layer `l` whose `reserved` is `NONE`. Ownership comes from `cell_metal` (dr:1726–1750): build `owner: HashMap<(LayerId, Rect), u32>` from `cell_abs[n]` (a shape in net `n`'s list → `n`); a ring shape or a shape in no list has no owner. Without a stack `cell_metal` returns nothing, so every cell shape blocks (the dr unit tests run without a stack):
     - `BLOCKED` when the shape belongs to no net or to a net other than the one whose pin reserves the node;
     - reserved for its own net when `cell_metal` (dr:1726–1750) attributes the shape to a net (a trunk may merge with its own strap).
     `charge_rect` (dr:1153–1162) is deleted.
  3. `cross_net_shorts(wires, joins, foreign: &[(Option<u32>, Shape)])`: also reports a routed shape of net `n` touching, on a shared conductor (`conductor_layers_meet`, dr:2057–2060), a foreign cell/ring shape whose owner is not `Some(n)`. V entry: `drawn short net {n} to cell metal`.
  4. Residual overuse after PathFinder: V entry `unresolved congestion` with `margin = overuse` (replaces the Θ entry).
  5. `unreachable_shapes` → `open_components(shapes, joins) -> usize`: two shapes connect iff they touch in xy and `conductor_layers_meet` (same layer, or a cut and one of the two metals it joins). The same relation replaces the layer-blind test in `Routes::debug_check` (routes.rs:43–75) through a new `debug_check_joined(&self, ctx, joins: &[(LayerId, LayerId, LayerId)])` (core file, coordinated with FLOW; AR-31).
  6. Short resolver (replaces the `while let Some(..) = first_short(..)` loop, dr:617–635, and `first_short`, dr:2075–2088): compute the list of touching cross-net pairs `(a, b, i, j)` once (one O(S²) pass with the same predicate as `first_short`; RTE-13's `ShapeIndex` later makes it near-linear); for each pair, in list order, skip it if either shape is already deleted; if one side is an access shape (`index ≥ pre_access[net]`, dr:654) delete the access shape of the net that comes later in `cold.order` (else the other net's access shape) and count it `sacrificed`; a trunk-vs-trunk pair deletes nothing and is reported as V with the existing label `drawn short nets {a}/{b}` (dr:1850).
- Tests: `backend/dr/src/lib.rs`
  - `a_ring_band_is_a_hard_obstacle`: ring with a band on layer 0 only between two pins of another net on layer 0; route succeeds on layer 1 over the band; no tree node lies inside the grown band on layer 0.
  - `a_route_touching_foreign_cell_metal_is_a_short`: `cross_net_shorts` called directly with net 0's met1 wire (0, 0, 2 000 × 260) and `foreign = [(Some(1), met1 (1 000, 0, 170 × 170))]` → one pair; with `foreign = [(Some(0), same rect)]` → none.
  - `resolver_sacrifices_the_later_access_shape`: two nets whose access jogs overlap, net 1 later in `cold.order` → net 1's jog deleted, `sacrificed[1] == 1`, net 0 intact.
  - `an_xy_overlap_on_non_adjacent_layers_is_open`: same-net shapes on layer 0 and layer 2 overlapping in xy with no cut → 1 open.
- Acceptance: `signoff_fixtures` DRC baseline still 0; no dr report on any bench circuit carries an `unresolved congestion` V entry, and no `routing overuse` Θ entry remains. (Note: `RunStats::route_overuse` is not an overuse-only statistic; it sums the margins of every routing Θ entry, including `em underwidth`, `em cuts` and budget batches, lib.rs:691–695, so it is not the measure for this item.)
- Risks / notes: a crowded cell may leave few landing nodes; pins are reserved before blocking, so a pin's own stitch nodes stay usable. Sealing: sky130 rings draw no met1 band ("No met1 band: routing is met1-and-up", kernel/cells/src/post_cell.rs:202), so no sky130 ring blocks a routed layer; on gf180/ihp the ring's `li`-role band is metal1 (pdks/gf180mcu.json:28, pdks/ihp_sg13g2.json:25) and becomes a routed layer only with RTE-11, where metal2 still crosses it.
- Status on m0 (recorded by the M0 review panel, pending the owner decision the item review required): **not merged
  through review.** `ba49ae2` ports `e8ef0e9`/`b36ca9f`/`93fe6fb` onto m0 outside the review; the originals are kept on
  branch `m0-rte03-original`. **Acceptance 3 is not met:** 7 of dac4's 90 dr reports (none a winner) carry
  `unresolved congestion` plus `drawn short nets a/b` V (m0-report §1), so this plan's M0 row T2 is not met either. The
  owner either reverts `ba49ae2`, or keeps it with acceptance 3 recorded as not met and the residual congestion handed
  to RTE-08 (M1).

### RTE-04 (cut — duplicated by REL-03 and REL-04)

- Moved to C2. REL-03 adds `Routes::terms: Vec<Vec<Terminal { at, ua: Option<f32> }>>` (dr fills it next to `(routes.cell, routes.gates)`, dr:879), `routing::current::net_flow(stack, shapes, terms) -> Option<NetFlow { shape_ua, drop_uv }>` (Lienig eqs 3.5–3.7 on a least-R spanning tree; chords bounded by the tree path), the hard per-shape and per-cut-group `Electromigration` including the pin-access layer and cut (`em_limits` over `layers ∪ {pin_access}`), and the `em.rs` tests; REL-04 makes `IrDrop` terminal-resolved from the same flow.
- This plan's former nodal solve (`dc_solve`: conjugate gradient, per-terminal voltages) is dropped: every RTE consumer is served by `NetFlow::shape_ua`, `NetFlow::drop_uv` or the existing `Stack::terminal_resistance_ohm` (stack.rs:153–161) — see RTE-05, 14, 17, 21, 27.
- RTE's remaining EM work — dr sizing trunks, access jogs and access cuts until REL-03's rule passes — is RTE-14.

### RTE-05 Differential: terminal-resolved RC, via count, coupling asymmetry, `known`; no stub trim

Status: done in M1 (`7f8d92b`; routing merge `57e3cc8`). `trim_pair` and its test are deleted (RTE-20 recovers them from `7f8d92b^`).

- Priority: P0. Effort: M. Depends on: REL-03 (`Routes::terms`), RTE-02 (weights), RTE-09 (`RepairKind::Mirror`); EXT-09 moves the rule to the budget arm and deletes `Differential::extract`. REL-03, RTE-02 and RTE-09 landed in M0: `Routes::terminals(net)` (kernel/core/src/routes.rs:101), `CouplingBudget::aggressor_weight` (coupling.rs:47), `gr::symmetric_nets` already reads both arms (gr:277), while dr's trim loop still filters `reqs.hard` only (dr:876-880 on m0; `trim_pair` at dr:1016).
- Why: AR-12, AR-39, AR-44, AA-14, AT-20; BAL2-22 (per-aggressor balance, balasa_graeb_survey.txt:8905–8909, 9745–9753); NOTES-43; SURV-14 (exact per-layer matching, perf_driven_survey L86–89); SUB-44 (by analogy only: a 5 % mismatch of a receiver pair's *substrate* capacitances costs ≈ 40 dB of differential isolation at 1 GHz and ≈ 27 dB at 100 MHz, values read from Charbon Fig 8.10b per SUB-44; the source concerns substrate coupling, not routing coupling).
- Current: see §1.5; `trim_pair` adds a stub off the lighter net that equalises the summed metric while terminal R stays unequal (dr:944–990, "it is not on a terminal path").
- Change:
  1. Fields: add `pub aggressor_weight: Option<&'static [f32]>` (same table as RTE-02).
  2. `mismatch_pct` in stack mode = max of
     - ground C: `|C(pos) − C(neg)| / mean` (`Stack::ground_af`, existing);
     - terminal R: rects of each side = `r.terminals(net).iter().map(|t| t.at)` (REL-03 step adds `Routes::terminals(&self, NetId) -> &[Terminal]`, plan-06 REL-03), sorted by `(y, x)` of their centres; with equal counts, `R_a = stack.terminal_resistance_ohm(r.shapes(pos), rects_a)`, `R_b` likewise (stack.rs:153–161, a star model from the net's centre), and `max_i |R_a[i] − R_b[i]| / mean(R_a ∪ R_b) · 100`; a `None` entry (unreached terminal) makes the term 100 %; with unequal counts (an ABBA cell), fall back to the runs' R (differential.rs:63);
     - via count: per cut layer `|n_a − n_b|` summed over mean count (squares on cut layers);
     - coupling asymmetry (when `aggressor_weight` is set): `Σ_a w_a·|C(pos, a) − C(neg, a)| / mean(C_ground) · 100` over every other net `a`, where `C(x, a) = coupling::net_pair_af(stack, r.shapes(x), r.shapes(a))`: a new `pub fn net_pair_af(stack: Option<&Stack>, a: &[Shape], b: &[Shape]) -> f32` in coupling.rs = Σ over same-layer shape pairs of the existing `pair_coupling_af` (coupling.rs:24–29, deck `lateral_af` else `EPS_H_AF`). RTE-18 adds screening and the crossing term inside `net_pair_af`, and RTE-21 step 3 and RTE-20 reuse it, so all three measure coupling one way.
     The result is `max` of the four terms, in percent, compared with `max_len_delta_pct10/10` as today (differential.rs:92–95).
  3. `known`: both nets have at least one shape (AR-44).
  4. Arm and dispatch: EXT-09 moves the push from `r.hard` to `r.budget` (extract.rs:53–54) and makes `gr::symmetric_nets` (gr:275–281) and dr:811 read both arms; RTE-09 then selects Differential by `repair_kind() == RepairKind::Mirror` everywhere. No RTE edit to extract.rs.
  5. Delete the Differential trim loop (dr:808–817). `trim_pair` moves to RTE-20 as the capacitor-lead equaliser, where a dead-end stub is the correct fix (H08-22). Update the `ponytail:` note on the type (differential.rs:26–27) to the new metric.
- Tests: `kernel/analog/src/routing/differential.rs`
  - `a_stub_does_not_match_terminal_resistance`: pos 30 µm trunk to its far pin; neg 20 µm trunk plus a 10 µm dead-end stub (equal summed length) → mismatch > 5 %.
  - `an_exact_mirror_is_zero`: mirrored shapes and pins → 0.0.
  - `a_one_sided_aggressor_breaks_the_pair`: aggressor parallel to pos only, 10 µm at 280 nm, weight 1 → residual > 0; weight 0 → mismatch 0.
  - `an_unrouted_side_is_unknown`: `neg` has no shapes → `known == false`.
  - Test stack: a local `Box::leak`ed `Stack` with the values of the private stack.rs test helper (metal area 25 aF/µm², fringe 40 aF/µm, lateral 3.9·8.854·360, sheet 0.125 Ω/□, cut 4.5 Ω; stack.rs:410–414); terminals filled into `Routes::terms` with `ua: None`.
- Acceptance: the Differential residual never improves by adding metal that carries no terminal current (test above); on `ota` it reaches 0 once RTE-15 lands.
- Risks / notes: moving to the budget arm lowers its lexicographic weight; exact matching becomes a construction property of RTE-15 instead of a hard toleranced check (AR-12).

### RTE-06 Antenna repair on the drawn geometry

Status: not merged (rejected; commits on branch `m0-rte06-rejected`); carried to M1 with M0 exit criterion 2 (dac4 `erc/ar.met2.1:gate`). See the Status note at the end of this item and 00-MASTER-PLAN Status. M1: not needed; after FLOW-17 dac4 signs off ERC 0 at `feedback_iters = 1` (`fixtures_sign_off_within_baseline`), so the branch stays unmerged.

- Priority: P0. Effort: S. Depends on: REL-02 (the model: `GatePin { at, dev, nm2 }` per gate, gate area per piece, deck-only diode credit, `Antenna::known`); RTE-09 (`RepairKind::Antenna`). Gate-area-per-piece and stage-aware diode credit, formerly steps 1–3 of this item, are REL-02 steps 1–3 (C3).
- Why: AV-15 (dac4 `erc/ar.met2.1:gate` red at `feedback_iters = 1`, clean at 5, signoff_fixtures.rs:139); AT-23 (repair judged on `probe` without access jogs, dr:555); REL-02 risk note ("If dac4 still fails after this item, the remaining work is RTE's repair"); H05-26 (the node ratio counts "the gate oxide area beneath the poly regions belonging to the node", hastings.txt:13148–13157).
- Current: the Antenna repair arm (jumper per layer, lift, plain; dr:1486–1498, 1753–1788) is judged on `probe` = `build_routes` of the trees (dr:555). Access jogs, access cuts, short breaking, via arrays and fill (dr:607–877) add metal afterwards; the final `score` (dr:881) is the first measurement of the drawn antenna area and nothing repairs it.
- Change (`backend/dr/src/lib.rs` only):
  1. After fill (dr:877, before the frame shift is undone): `drawn = Routes { wires: <final frame-coordinate wires>, cell: cell_f.clone(), gates: <REL-02 GatePins, frame-shifted> }`; evaluate only the batches with `repair_kind() == RepairKind::Antenna` (hard and budget arms).
  2. If any is violated: run the Antenna arm of `repair_constraints` on the nets it names, with the trial judged on `probe_drawn(hot) = build_routes(hot) + add_pin_access(access)` (landing is fixed before PathFinder, so the same `access` list re-draws the jogs), then redo the geometry passes (short breaking, via arrays, fill) for all nets. At most `ANTENNA_ROUNDS = 2` (tuning default); a violation that survives stays in the report as V.
  3. The diode path at lib.rs:632–652 is not touched here (REL-02 step 5 gates it on deck credit; FLOW-08 step 4 fixes its double history).
- Tests (`backend/dr/src/lib.rs`, existing helpers at dr:2141–2147): `antenna_is_repaired_on_the_drawn_metal`. One placed macro with a gate pin named `d0:G` (170×170 on layer 0) and gate area 1 µm² through REL-02's table (`cfg.gate_nm2 = vec![vec![("d0:G".into(), 0, 1_000_000)]]`), `cfg.stack = Some(leaked stack)` (gate pins are collected only with a stack, dr:1728), and one driver pin 20 µm away. Run 1 without an `Antenna` rule; measure `A_tree` (the net's trunk metal on the jog layer `jog_layer(..)`, dr:1083–1091) and `A_all` (all of the net's metal on that layer, access included). Run 2 with an `Antenna` hard rule whose ratio on that layer is `(A_tree + A_all)/2 / 1 µm²` (a stack with `antenna_cumulative = false`, the stack.rs test helper at stack.rs:414). Assert: the test is set up so that `A_tree < limit < A_all` (checked), and run 2's report has 0 `Antenna` violations.
- Acceptance: `benchmarks/tests/signoff_fixtures.rs::fixtures_sign_off_within_baseline` green at `feedback_iters = 1` (dac4 ERC 0), with REL-02's `antenna_in_loop_never_passes_what_signoff_fails` green.
- Risks / notes: each round re-runs the geometry passes (RouteStats `us_geometry`, RTE-13). RTE-23 generalises final-geometry repair to every rule.
- Status on m0 (recorded by the M0 review panel): **rejected, not on m0.** Its acceptance was red on its own branch with
  the same `dac4: ERC rows ["erc/ar.met2.1:gate"]`. Its commits `b635606`/`63bc2a2`/`32efea4` are kept on branch
  `m0-rte06-rejected`. The owner either merges it recorded as "acceptance not met" and names the item that owns dac4's
  met2 cap-plate antenna (`routing_stack` gives dr only met1/met2 on sky130, or the cap pin layer changes), or
  re-scopes M0 exit criterion 2 in 00-MASTER-PLAN §5. The dac4 ERC baseline is not relaxed.

### RTE-07 Retire the coarse `GlobalRoute`; frame, corridors and group pricing

Status: done in M1 (`f61390f`; routing merge `57e3cc8`). `GlobalRoute`, the gcell grid (`gcell_capacity`) and `gr::price_group` are gone.

- Priority: P0. Effort: M. Depends on: none (RTE-25 provides congestion to PLC).
- Why: AT-08, AT-09, AT-32, AT-37 (reporting), AT-30; LAMP-36 (area routing for 10–20 nets, lampaert.txt:5511–5540); AF-06.
- Current: §1.3. gr costs one route per epoch; its report is discarded (lib.rs:617–618); its frame is wrong for negative coordinates; its only effect is corridors for 8 of 150 dr iterations.
- Change:
  1. Delete from `backend/gr/src/lib.rs`: `GlobalCfg`, `GlobalRoute`, `price_group`, `GroupPrice`, `group_bbox`, `gcell_terms`, `keep_apart`, `keepaway_partners`, `order_by_pin_count`, gr's `charge_rect`, `die_extent`, `seg_shape`, gr's `score`, `build_nets`, `GcellGrid`, `CORRIDOR_EPOCHS`, `RGraph::region`, `TrackGrid::set_regions` and `region_of`, `RouteCtx::corridors`, and the `corridor` parameter of `route_net`/`reroute`. `Tier` goes in RTE-08 with the `Negotiation` key change.
     Keep: `pub use pnr_core::place_macros` (gr:19; callers geometry.rs:11, 63, elaborate.rs:167, lib.rs:609, 612, 620, 721, 765, 809), `Negotiation`, `RGraph`, `TrackGrid`, `RouteHot`, `RouteCtx`, `Dij`, `Elec`, `route_net`, `run_pathfinder`, `bump_history`, `extract_geometry`, `to_shapes`, `Wire`, `Via`, `order_by_priority`, `symmetric_nets`, `analog_tiers`, `KEEPOUT_COST` (until RTE-16), `NONE`. `RGraph::region` goes with the corridors.
     Add `pub fn group_hpwl(macros: &[Macro]) -> i64` (the HPWL half of `price_group`, gr:113–121).
  2. New dr signature (the `global` argument goes):
     ```rust
     pub fn route(&self, pins: &[(NetId, Rect, LayerId)], placed: &[Macro], rings: &[Macro],
                  reqs: &Requirements<Routes>, layers: &[LayerId], cuts: &[Cut],
                  neg: &mut gr::Negotiation) -> (Routes, Report, RouteStats)
     ```
     The frame comes from pins, placed bboxes and rings; `n_nets = max(cfg.n_nets, max pin net + 1)` with a new `DetailedCfg::n_nets` set to the netlist's net count, so rules on unrouted nets see empty shapes. `RouteStats` is introduced here with every field RTE-08/13/22/25/27 fill, all zero or empty until those items land:
     ```rust
     #[derive(Default, Clone, Debug)]
     pub struct RouteStats {
         pub us_landing: u64, pub us_negotiate: u64, pub us_repair: u64, pub us_geometry: u64, pub us_fill: u64,
         pub expanded: u64, pub trials: u32, pub pf_iters: u32, pub overuse: f32,
         pub coarsened: bool,                   // RTE-13
         pub width_fallbacks: u32,              // RTE-22
         pub single_cut_vias: u32,              // RTE-27
         pub congestion: Vec<(Rect, f32)>,      // RTE-25, absolute nm
     }
     ```
     The library keeps the last epoch's `RouteStats` in `Epoch` next to its `Report`; FLOW-09 owns `RunStats` and copies what the bench prints.
  3. dr: delete corridor construction (dr:432–451), `ggrid` and `set_regions` (dr:261–263), `GCELLS_PER_SIDE` and its false comment (dr:30–31); fix the duplicated doc line (dr:1028).
  4. `cellgen::price` (cellgen.rs:354–375) and its caller `seed_assignment` (cellgen.rs:327–346, which builds `gr::GlobalCfg` at :332 and passes it at :340): `gr::group_hpwl(std::slice::from_ref(m))`; tuple `(usize /*DRC+ERC*/, i64 /*hpwl*/)`; the `cfg` parameter goes (a few-line edit in a CELL/FLOW file, coordinated).
  5. `lib.rs:614–652` (the gr call at 617–618, `global.debug_check("gr::route")` at 619, and both dr calls, 629–631 and 646) and `elaborate.rs:166–190` (gr call at 176, dr call at 177): drop the gr call; pass the new signature and bind the third tuple element. On m0 these are `lib.rs:761` (gr) and `:797` (dr), `elaborate.rs:179-180`. `route` still returns `(Routes, Report)` on m0; RTE-09 added the private `route_counted` (dr:224) that returns the repair-trial count as a third element (its doc says this item replaces the `u32` with `RouteStats`): fold it into `route` and the count into `RouteStats::trials`. The five `.route(` calls inside dr's tests take the new signature too. Fix the stale `gr::build_nets` mention in the message at geometry.rs:87.
- Tests: delete gr's `two_pin_net_routes_coarsely`, gr's `negotiation_persists_across_calls` (gr:1197; it drives `GlobalRoute`, the dr test of the same name at dr:2330 stays), gr's `budget_residual_reaches_theta`, `hard_batch_margin_is_measured_not_counted`, `overflow_theta_is_a_capacity_residual_not_a_count`, `price_group_measures_the_group_not_the_die`, `a_keepaway_rule_parts_two_nets_during_search` (its behaviour moves to RTE-18 in dr). Add dr `negative_coordinate_pins_route` (pins at (−20 000, −5 000) and (5 000, 3 000) nm → routed, V empty).
- Acceptance: `bench local` 10/10 circuits DRC 0 and LVS MATCH as before; per-circuit ms reported before/after (expected lower: one router run per epoch instead of two).
- Risks / notes: the deleted items' outside callers are exactly the ones steps 3–5 edit: `price_group` (cellgen.rs:360), `GlobalCfg` (cellgen.rs:332, 357), `GlobalRoute` (lib.rs:618, elaborate.rs:176), `GcellGrid` (dr:21, 261); `gr::Tier` is also used by dr (dr:506, 584) and goes with RTE-08's `Negotiation` change. PLC-15 names a `gr::OverflowMap` from the coarse router; after this item PLC reads `RouteStats::congestion` (RTE-25) instead (plan-04 §0.2 already lists it).

### RTE-08 Negotiation keys and PathFinder convergence

Status: done in M1 (`a192c02`; routing merge `57e3cc8`). RTE-03 acceptance 3: 0/45 dac4 score calls with congestion (routing branch).

Field report: FR-6 (a short between two nets of a 7-device PTAT core, "device count mismatch"). FR-6's netlist is not in the report; ptat_bias signs off CLEAN on m0. The same failure class reproduces on m0 on a FET-only circuit: the CLI run of pwm_driver (34 FETs, default `Config`, two runs, identical) ends with `lvs/extract: label short: labels ["vss", "ba_m"]`, and 4 of that run's 25 non-empty dr reports carry `drawn short nets a/b` (2/13 twice, 3/15, 2/3), with `open net` and `pin access sacrificed` rows beside them (instrumented copy, one `eprintln!` at the end of `dr::score`). Whether the winning epoch's own dr report flags the short was not measured. Added acceptance: a `benchmarks/fixtures/pwm_driver.spice` (field-report netlist) run through the CLI has no `lvs/extract: label short` row; if the winner's dr report reads 0 hard rows while signoff finds the short, the gap goes to RTE-03's short detection.

- Priority: P0. Effort: S. Depends on: RTE-07 (the Global tier's only writer is gone). The history **lifecycle** — reset before every cold epoch, kept across warm epochs, one accumulation per epoch when antenna diodes force a second dr run (`let snapshot = neg.clone()` … `*neg = snapshot`) — is FLOW-08 step 4 (C4). The key change to lattice indices waits for RTE-10.
- Why: AT-10 (keys), AT-11 (constant present-congestion factor; "Stop on stationary overflow" is the audit's fix direction, recorded there as external McMurchie–Ebeling knowledge, not in ref/), AT-34/AF-06 (lifecycle, FLOW's).
- Current: history keyed `(Tier, layer, x/500, y/500)` with `max` folding (gr:37–92, `HIST_QUANTUM_NM` gr:55); p_fac constant across iterations (gr:1019–1043); the run stops only at zero overflow, on an iteration where no dirty net could be rerouted, or at `MAX_ITERS = 150` (dr:29), so an unsolvable instance always spends 150 iterations.
- Change (`backend/gr/src/lib.rs`):
  1. ```rust
     #[derive(Default, Clone)]
     pub struct Negotiation { hist: BTreeMap<(u32 /*layer*/, i32 /*kx*/, i32 /*ky*/), f32> }
     impl Negotiation {
         pub fn seed(&self, hist: &mut [f32], pos: impl Fn(u32) -> (i32, i32, u32));
         pub fn accumulate(&mut self, hist: &[f32], pos: impl Fn(u32) -> (i32, i32, u32));
     }
     ```
     `Tier` and the tier argument are deleted (dr:506, 584 drop it). Key quantum: `HIST_QUANTUM_NM = 500` until RTE-10; after RTE-10 the quantum is `p0` (`kx = x.div_euclid(p0)`), and RTE-10's origin (a multiple of `p0·lcm(strides)`) gives a track the same key in every epoch. `pressure()` is kept (dr test at dr:2330).
  2. `run_pathfinder(hot, cold, p_fac, hist_inc, max_iters) -> (f32 /*overflow*/, u32 /*iterations*/)`: iteration `i` uses `p_i = min(p_fac·P_GROWTH^i, P_FAC_MAX)` with `P_GROWTH = 1.3`, `P_FAC_MAX = 1000.0` (tuning defaults; the cap keeps f32 costs finite). Stop additionally when the overflow has not decreased for `STALL_ITERS = 10` consecutive iterations (tuning default). The existing "no net could be rerouted" stop stays; the iteration count goes to `RouteStats::pf_iters` (RTE-07).
- Tests (`backend/gr/src/lib.rs`, `TrackGrid::with_layers` fixtures like gr:1323–1361):
  - `pathfinder_converges_on_a_solvable_crossing`: `TrackGrid::with_layers` 12×12×2 (capacity 1, layer 0 horizontal); three two-pin nets with pins on layer 0 at (0, 4)–(11, 6), (0, 5)–(11, 5), (0, 6)–(11, 4) (their shortest routes share row 5 nodes) → overflow 0.0 and `iterations < 150`.
  - `an_unsolvable_instance_stops_on_stall`: 2 two-pin nets whose only paths share one node (1 layer, a 1-node-wide corridor built with `reserved`) → overflow 1.0 and `iterations ≤ STALL_ITERS + 1`.
  - `history_keys_ignore_the_frame`: `accumulate` with frame origin (−1 000, 0) and `seed` with origin (−1 500, 0) for the same absolute node → the seeded value equals the accumulated one.
- Acceptance: `negotiation_persists_across_calls` (dr:2330) still passes with the tier removed; `bench local` DRC/LVS unchanged; `RouteStats::pf_iters` p50 on `ota` ≤ the M0 baseline. If `ba49ae2` (RTE-03) stays on m0, this item also owns RTE-03's unmet acceptance 3: 0 of dac4's dr reports carry `unresolved congestion` V (7 of 90 on m0, margins 3–16, each with `drawn short nets a/b`; m0-report §1), counted the way m0-report §1 did (one `eprintln!` of the hard rows at the end of `dr::score`, bench seed 1, FEEDBACK_ITERS 5, not committed).
- Risks / notes: a growing factor forces convergence at the price of detours on congested spots; the cap bounds it.

### RTE-09 Typed repair dispatch

Status: done in M0 (`e4915ea`). The acceptance grep must be word-bounded: `grep -rnE '\bkind\(\)' backend/gr backend/dr frontend/library/src/elaborate.rs` is empty on m0, while the plain `'kind()'` pattern matches every `repair_kind()` call. dr's repair trial count comes back through the private `route_counted` (backend/dr/src/lib.rs:224), which RTE-07 folds into `RouteStats::trials`.

- Priority: P0. Effort: S. Depends on: none (lands before or after EXT-09; with EXT-09 the `Mirror` filter reads both arms).
- Why: AT-31; AR-18 (kind strings). The stale `docs/CRATES.md` router text (AT-36: CRATES.md:58, 63, 97) is FLOW-15 step 5 (C5).
- Current: repair and ordering dispatch on `kind()` suffixes (gr:253, 277; dr:534, 811 with `ends_with`, dr:1444 with `rsplit("::")`; also `elaborate::antenna_diodes`, elaborate.rs:342).
- Change: `kernel/analog/src/rule.rs`
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum RepairKind { None, Reroute, Mirror, Balance, KeepAway, Antenna, Shield, Em, Ir, Budget }
  pub trait Rule { /* … */ const REPAIR: RepairKind = RepairKind::Reroute; }
  pub trait RuleBatch<On> { /* … */ fn repair_kind(&self) -> RepairKind { RepairKind::Reroute } }
  ```
  The blanket `impl RuleBatch<On> for Vec<R>` returns `R::REPAIR`. Overrides: Differential → `Mirror`, CommonNodes → `Balance`, CrosstalkExclusion and CouplingBudget → `KeepAway`, Antenna → `Antenna`, Shield → `Shield`, Electromigration → `Em`, IrDrop → `Ir`, ParasiticBudget and PerformanceBudget → `Budget` (CommonNodes and PerformanceBudget implement `RuleBatch` directly, common_node.rs:61, performance.rs:36, so they override `repair_kind()`; the others set `const REPAIR`). All kind-string matches in gr/dr and elaborate.rs:342 are replaced: `symmetric_nets` = nets touched by batches with `repair_kind() == Mirror` in `reqs.hard ∪ reqs.budget`; `order_by_priority`'s hard tier excludes `Mirror` batches; dr:534 selects `Ir`; dr:811 selects `Mirror`; dr:1444 matches on `repair_kind()`; elaborate.rs:342 selects `Antenna`. `repair_constraints` generates **no** trial for `RepairKind::Em` (a reroute cannot change width until RTE-12/14; REL-03 step 7 asks for exactly this, AT-24).
- Tests: `rule.rs` `every_routing_rule_declares_its_repair` (one assertion per type listed); `dr` `an_em_violation_spends_no_repair_trial` (a net with a violated `Electromigration` and nothing else → `RouteStats::trials == 0`).
- Acceptance: `grep -rn 'kind()' backend/gr backend/dr frontend/library/src/elaborate.rs` returns nothing. (A plain `ends_with("` grep cannot be the gate: dr:1737 legitimately tests the pin name `":G"`, and dr:1444 uses `rsplit`.)
- Risks / notes: none; a mechanical refactor. The price-key string problem of `gp::Prices` (AR-18) stays with PLC/FLOW.

### RTE-10 Per-layer lattice: base pitch with per-layer track stride, per-layer wire width, oriented via pads, along-track halos

- Priority: P1. Effort: L. Depends on: RTE-07.
- Why: AT-02; H15-20 (Eq 15.19 `P_VTV = W_v + 2E_mv + S_m`, E_mv = the smallest opposite-side overlap, hastings.txt:48196–48219); H15-18 (asymmetric enclosure, hastings.txt:48100–48146); H15-21 (grid consistency, hastings.txt:48285–48313); Hastings' practice of widening a lead to the via width (hastings.txt:48196–48199); NOTES-52 (landing templates); BAL2-03 (obstacles inflated by per-layer spacing, balasa_graeb_survey.txt:8596–8602); LAMP-36 (a uniform worst-case pitch wastes area, lampaert.txt:5543–5590).
- Current: one wire width = the first-via pad (290 on sky130) and one pitch for every layer (460 on sky130) (elaborate.rs:393–399, pdk.rs:423–433; derived); square pads at the larger enclosure on all sides (pdk.rs:354–358); `TrackGrid` uniform (gr:515–532); frame origin arbitrary (dr:213–218). Adding met3 (width/space 300, sky130.deck:360–362) under one pitch would force every layer to met3's pitch.
- Change:
  1. New type in `backend/gr/src/lib.rs`:
     ```rust
     pub const MAX_LAYERS: usize = 8;
     #[derive(Clone, Debug)]
     pub struct LayerSpec {
         pub id: LayerId,
         pub horizontal: bool,
         /// A track every `stride` base pitches across the layer's direction.
         pub stride: u32,
         /// One-track wire width, nm.
         pub wire: i32,
         /// Spacing a wire keeps (min spacing incl. EOL), nm, and the wide-metal steps (width threshold, spacing).
         pub space: i32,
         pub wide: Vec<(i32, i32)>,
         /// Via pad sides across / along the wire, nm (largest over the cuts that land here).
         pub pad_across: i32,
         pub pad_along: i32,
         /// Along-track nodes a foreign net keeps clear of a wire end / of a via end.
         pub halo_wire: u8,
         pub halo_via: u8,
     }
     ```
     `DetailedCfg` replaces `pitch`, `wire_width`, `spacing` with `base_pitch: i32` and `layers: Vec<LayerSpec>`; `min_width` stays for the pin-access layer.
  2. Computation in `elaborate::detailed_router`, for each routed metal `l` with its cuts `c` (below and above, including the pin-access cut for the first layer):
     - `E_across(c, l)`, `E_along(c, l)`: the deck's all-round enclosure of `c` by `l`, and the larger of that and the one-pair-of-opposite-sides enclosure. New `Pdk::cut_enclosure_pair(metal, cut) -> (i32, i32)` (pdk.rs, coordinated with FLOW; it reads the same `min_enclosure` (`limit`) and `asymmetric_enclosure` (`min_one_side`) rules as `max_enclosure_of`, pdk.rs:373–391, but filters on **both** layers, `layers_of(s) == [metal, cut]`: `max_enclosure_of` keys on the inner layer only, so it returns the larger of the metal-below and metal-above enclosure — met1 would wrongly get via's met2 value). Returns `(E_across, E_along)` = `(limit, max(limit, min_one_side))`, 0 for a missing rule.
     - `pad_across_l = max_c (size_c + 2·E_across)`; `pad_along_l = max_c max(size_c + 2·E_along, ⌈min_area_l / pad_across_l⌉)`, rounded up to the manufacturing grid.
     - `wire_l = max(min_width_l, pad_across_l)`; `S_l = route_spacing(l)` (pdk.rs:439–447).
     - `P_l = max(wire_l, pad_across_l) + S_l` (Eq 15.19).
     - `p0 = max(P_{l0}, P_{l1})` over the two lowest routed metals, rounded up to `2·grid`.
     - `stride_l = ⌈P_l / p0⌉`; `halo_wire_l = ⌈(wire_l + S_l)/p0⌉ − 1`; `halo_via_l = ⌈(pad_along_l/2 + max(pad_along_l, wire_l)/2 + S_l)/p0⌉ − 1`.
     - Direction: alternate from the first routed metal (horizontal), as today (gr:630–638).
  3. `TrackGrid::new(die: (i32, i32), p0: i32, specs: &[LayerSpec], via_cost: f32) -> Self`; node indexing unchanged (`n_layers × nx × ny`); `on_track(n)` = `(if horizontal { iy } else { ix }) % stride == 0`; dr marks off-track nodes `BLOCKED` (RTE-03) at construction, so via edges exist only where both layers have a track; `beside()` (gr:648–660) steps by the layer's stride.
  4. Frame origin (dr:218): `origin = ⌊(low − halo − margin) / L⌋ · L` with `L = p0 · lcm(stride_l)`. Track positions are then fixed in absolute coordinates across epochs (used by RTE-08 and RTE-15).
  5. Geometry: `extract_geometry` (gr:1085–1125) emits each run at its layer's width; `build_routes` (dr:1322–1355) draws each via pad as a `pad_along × pad_across` rectangle oriented along the wire of that layer (horizontal layer: `w = pad_along, h = pad_across`), and a wire that ends at a via extends `max(wire/2, pad_along/2)` past the node.
  6. Along-track halos enter PathFinder as footprint nodes (the k = 1 case of RTE-12): committing a tree adds usage on `halo_wire_l` nodes each side of every run end and `halo_via_l` nodes each side of every via end; the searching net's own coverage is subtracted.
  7. Delete `Pdk::routing_pitch` (pdk.rs:423–433) and its ponytail; `Pdk::routing_vias` returns `(cut, size, below: (i32, i32), above: (i32, i32))` (across, along) instead of square pads.
  8. `MAX_STRIDE = 4` (tuning default): used by RTE-11 to cap the stack.
- Derived sky130 values (sky130.deck lines in brackets):

  | Layer | Cuts landing (size; enclosure across/along) | pad_across | pad_along | wire | S | P_l | stride | halo_wire | halo_via |
  |---|---|---|---|---|---|---|---|---|---|
  | met1 (H) | mcon 170; 30/60 [354, 383–384]; via 150; 55/85 [385, 387–388]; area 0.083 µm² [372] | 260 | 320 | 260 | 140 [360–362] | 400 | 1 | 0 | 1 |
  | met2 (V) | via 150; 55/85 [389–390]; via2 200; 40/85 [391, 393–394]; area 0.0676 µm² [373] | 280 | 370 | 280 | 140 | 420 | 1 | 0 | 1 |
  | met3 (H) | via2 200; 65/65 [395]; via3 200; 60/90 [396, 398–399]; area 0.24 µm² [374] | 330 | 730 | 330 | 300 [360–362] | 630 | 2 | 1 | 2 |
  | met4 (V) | via3 200; 65/65 [400]; area 0.24 µm² [375] (via4 excluded, RTE-11) | 330 | 730 | 330 | 300 | 630 | 2 | 1 | 2 |

  `p0 = max(400, 420) = 420`. Tracks per µm across all routed layers: 2/0.42 + 2/0.84 = 7.14, against 2/0.46 = 4.35 today (+64 %, derived).
- Tests:
  - `frontend/library/src/elaborate.rs` (unit, sky130 sidecar): `sky130_layer_specs_match_the_hand_derivation` asserts every cell of the table and `p0 = 420`.
  - `gr`: `off_track_nodes_are_blocked_on_stride_layers`; `beside_steps_by_stride`.
  - `dr`: `adjacent_foreign_vias_keep_the_along_track_halo` (two nets forced to change layer at along-track neighbours on met1 → one moves; drawn pad-to-pad gap ≥ 140 nm); `origin_is_a_multiple_of_the_lattice_period`.
- Acceptance: routing-layer DRC 0 on the 7 `signoff_fixtures` circuits and the 3 slow OTAs; RTE-26 deck cases green; `RouteStats` reports the lattice (p0, strides).
- Risks / notes: the fixed ±2-row stacked-via exclusion at landing (gr:595–599) stays until a deck states stacking rules (H15-18); halos already keep adjacent foreign vias apart.

### RTE-11 Route on every useful metal

- Priority: P1. Effort: S. Depends on: RTE-10.
- Why: AT-01; AF-35 (`routing_stack` panics on bad decks; FLOW-11 needs a `Result` and a metal cap); Lampaert §5.2 requirement "n-layer routing" ("An analog routing algorithm has to take advantage of all available routing layers", lampaert.txt:5472–5476; summarised in ref-lampaert §5.1–5.2, not in LAMP-36, whose range is L5511–5590); H15-03 (routing factor falls with metal count, hastings.txt:46793–46834).
- Current: `take_while(min_width ≤ first-via pad)` and an unconditional bottom split (elaborate.rs:210–219): sky130 routes met1+met2 only; gf180 and ihp never route metal1 (derived, audit-05 §1.1). Note: gf180 and ihp map the cell role `li` to `metal1` (pdks/gf180mcu.json:28, pdks/ihp_sg13g2.json:25), so the role name cannot tell a resistive local interconnect from a real metal.
- Change:
  ```rust
  pub(crate) struct RoutingStack { pub layers: Vec<gr::LayerSpec>, pub cuts: Vec<Cut>, pub pin_access: Option<(LayerId, Cut)> }
  /// `top`: keep at most this many routed metals (FLOW-11 child solves); `None` = all that pass step 2.
  pub(crate) fn routing_stack(pdk: &Pdk, top: Option<usize>) -> Result<RoutingStack, String>;
  ```
  The three `assert!`s (elaborate.rs:221–238: no routable layer; a missing cut between adjacent layers; a cut wider than its pads) become `Err(..)` with the same messages; callers (lib.rs:279, elaborate.rs:166–190 and RTE-26's `router_for`) propagate them into FLOW's error path.
  1. Pin-access split: split off `routing_layers()[0]` iff its `pex` sheet resistance exceeds 10× that of `routing_layers()[1]` (tuning default 10×), looked up the way `elaborate::stack` reads `pex` sheets (elaborate.rs:266–291). Deck values at the pinned GPurify rev: sky130 li 12.8 Ω/□ vs met1 0.125 Ω/□ (sky130.deck:588, 590) → 102× → split; gf180 `metal1_con` 0.09 vs `metal2_con` 0.09 Ω/□ (gf180mcu.deck:575, 577) → 1× → metal1 routed; ihp `metal1` 0.11 vs `metal2` 0.088 Ω/□ (ihp_sg13g2.deck:684, 686) → 1.25× → metal1 routed; generic_finfet `M1` 1.2675 vs `M2` 0.534829 Ω/□ (generic_finfet.deck:343, 345) → 2.4× → M1 routed. A routed metal1 has its cell metal hard-blocked by RTE-03.
  2. Build `LayerSpec`s for all remaining metals (RTE-10), then drop from the top while `stride > MAX_STRIDE`, recomputing the layer below (its via to the dropped layer no longer lands). sky130: met5 `P = max(1600, 800 + 2·310) + 1600 = 3200` [378–379, 401, 404] → stride 8 → dropped; met4 is then recomputed without via4 → stride 2 (derived).
  3. `take_while` is deleted.
- Tests: `frontend/library/src/elaborate.rs`: `sky130_routes_met1_to_met4` (layers met1–met4, `pin_access` = li/mcon); `gf180_routes_metal1` and `ihp_routes_metal1` (first routed layer is the deck's metal1, `pin_access == None`); `a_top_cap_keeps_the_lowest_metals` (sky130, `top = Some(2)` → met1, met2); `a_deck_without_a_cut_is_an_error` (`Pdk::load(text, sidecar)`, pdk.rs:97, on the sky130 deck text with every `via` rule line removed → `Err`).
- Acceptance: `ota`, `tt_ota`, `ota_constrained` route with DRC 0 on met1–met4; `ota_cross_pdk` (sky130 and generic_finfet only, ota_cross_pdk.rs:338–358) reports its routing-layer DRC count; gf180/ihp routing is first exercised by RTE-26's `dr_deck_drc` (driven to 0 there).
- Risks / notes: supply straps on sky130 met5 (thick, 0.029 Ω/□) stay out; see open question 2.

### RTE-12 Footprint resource model: width and via arrays reserved during the search

- Priority: P1. Effort: L. Depends on: RTE-10.
- Why: AT-03, AT-12 (fattening loop ≈ 270 candidate widths per segment, each scanning all shapes), AT-38; LAMP-36/37 (width belongs to the segment; Eqs 5.1–5.3, lampaert.txt:5941–5963); SURV-29/31 (discrete widths realised as parallel routes, todaes22.txt:628–645); CC-30 (parallel wires for critical bits, cc_dac_constructive.txt:790–795); H15-24 (widths as integer track multiples, Eq 15.22, hastings.txt:48387–48400); H15-26 (fill the overlap with vias, hastings.txt:48411–48414); BAL2-36 (conditional rules: "the metal separation rule has a larger value if one of the metals becomes a wide metal", balasa_graeb_survey.txt:11819–11837; applying them in the search is this plan's step).
- Current: every net searches one track; widths are grown afterwards into leftover room (dr:637–715); a shortfall is Θ; via arrays fill whatever overlap resulted (dr:717–806).
- Change:
  1. A connection with `k` tracks on layer `l` occupies tracks `t, t+σ·s_l, …, t+(k−1)·σ·s_l` (σ = +1, or −1 for the mirrored partner on layers whose across axis the mirror flips, RTE-15). It is drawn merged, width `W_l(k) = wire_l + (k−1)·s_l·p0`, when `W_l(k)` is below the layer's first wide-metal threshold, else as `k` separate wires (TODAES parallel routes). A merged width at or over a threshold adds `⌈(wide_space_l(W) − (s_l·p0 − wire_l)) / (s_l·p0)⌉` blocked tracks each side.
  2. `TrackGrid::footprint(&self, n: u32, k: u8, side: i8, via: bool, out: &mut Vec<u32>)`: the `k` across-tracks at `n`; `halo_wire` along-track nodes each side; at a via end, the `k×k` block on both layers plus `halo_via` along-track.
  3. Types:
     ```rust
     pub struct Branch { pub nodes: Vec<u32>, pub k: [u8; MAX_LAYERS] }
     pub struct RouteHot { pub usage: Vec<u16>, pub hist: Vec<f32>, pub trees: Vec<Vec<Branch>>, pub side: Vec<i8>, /* weight, sens as today */ }
     ```
     `commit` adds/removes usage over footprints; `tree_nodes` becomes `tree_footprint(net) -> Vec<u32>` (with multiplicity, for own-coverage subtraction).
  4. `route_net` takes a query struct (replacing its 13 arguments, gr:882–896):
     ```rust
     pub struct NetSearch<'a> {
         pub net: u32,
         /// Terminals in connection order; terms[0] is the root.
         pub terms: &'a [u32],
         /// Tracks per layer for the branch that reaches terms[i] (k[0] unused).
         pub k: &'a [[u8; MAX_LAYERS]],
         pub side: i8,
         pub own: &'a [u32],       // the net's current footprint
         pub penalty: &'a [f32],
         pub elec: Elec<'a>,
         // RTE-15 adds `pub mirror: Option<Mirror<'a>>`.
     }
     pub fn route_net<G: RGraph>(g: &G, hot: &RouteHot, reserved: &[u32], q: &NetSearch, p_fac: f32, dij: &mut Dij) -> Option<Vec<Branch>>;
     ```
     Node cost = Σ over the footprint of `hist + p_fac·max(0, usage − own + 1 − cap)` + `penalty` + `Elec` of the anchor node; a node is legal only if every footprint node is on-track, not `BLOCKED`, and not reserved for another net.
  5. Via arrays: the existing overlap fill (dr:717–806) is kept and now fills the `W_l(k) × W_{l+1}(k)` corner overlap; the enclosure used per side is the deck's across/along pair (RTE-10), not the square pad.
  6. Deleted: fattening (dr:637–715), `fat_signal`/`fat_supply` (dr:61–63, elaborate.rs:429–435), `em_width` (dr:149–153), `widen` (dr:1028–1038). So that EM does not regress between this item and RTE-14, every branch of a net gets the same interim `k_l = 1 + ⌈max(0, I_max·1000/J_l − wire_l) / (s_l·p0)⌉` (capped at `K_MAX`), with `I_max = max(Σ_t max(ua_t, 0), Σ_t max(−ua_t, 0))` over the net's terminals (REL-01 currents; KCL bounds every segment by it) and `J_l` = the layer's `ua_per_um` (0 → `k_l = 1`). RTE-14 replaces this per-net bound by per-branch `k`. Supplies get no `fat_supply` width (dr:675–676); IR is repaired by RTE-21 (`k + 1` on IR-broken branches).
  7. `K_MAX = 8` tracks (tuning default).
- Tests: `dr`:
  - `a_two_track_net_reserves_both_tracks` (k = 2 on met1: a foreign net cannot use the adjacent track along the path).
  - `a_two_by_two_corner_gets_four_cuts` (sky130 values: `W_met1(2) = 680`, `W_met2(2) = 700`; cuts per axis `⌊(700 − 2·85 − 150)/(150 + 170)⌋ + 1 = 2` and `⌊(680 − 2·55 − 150)/320⌋ + 1 = 2` → 4 via cuts; derived from via.1a/via.2/via.4a/via.5a and m2.4/m2.5, sky130.deck:385–390; assumes the along-wire 85 nm pair is taken on the 700 nm axis and 55 nm on the 680 nm axis).
  - `a_wide_bundle_keeps_wide_spacing` (synthetic layer with a 1 µm threshold).
- Acceptance: dr p50 time on `ota` at least 2× lower than before this item (`RouteStats` timers, same protocol as T13); no DRC regression; REL-03's `Electromigration` V on `ota` not higher than before this item.
- Risks / notes: footprints raise congestion; the extra met3/met4 capacity of RTE-11 absorbs it on sky130. A net that cannot route at `k` retries at `k − 1` and reports the shortfall (RTE-14, RTE-22).

### RTE-13 Search speed, reuse and per-stage measurement

- Priority: P1. Effort: M. Depends on: RTE-07.
- Why: AT-12, AT-37; LAMP-38 (A* with an admissible predictor, lampaert.txt:6181–6196); BAL2-02 (Eqs 4.8–4.9, balasa_graeb_survey.txt:9481–9587); SURV-24 (budget the expensive stages).
- Current: plain Dijkstra (gr:966–992); `Dij::new` per repair trial and per shield (dr:1535, 1605); `first_short` O(S²) inside a delete loop (dr:621, 2075–2088); access and fill scan every shape (dr:1213–1221, 859–865); the lattice is coarsened silently above `max(die)/1200` (gr:528); only whole-flow time is measured (bench.rs:425–432).
- Change:
  1. A*: per target, heap key `(g + h).to_bits()` (the existing `Reverse((bits, node))` order is valid for non-negative f32), `h(n) = |Δix| + |Δiy| + via_cost·|Δl|` to the target (every planar step costs ≥ 1, every layer change ≥ `via_cost`, every added term ≥ 0 → admissible and consistent). New `RGraph::lower_bound(&self, a: u32, b: u32) -> f32` with default `0.0` (plain Dijkstra for any other graph); `TrackGrid` implements the formula with its `via_cost` field (gr:527). `dist` stays `g`; only the heap key adds `h`.
  2. One `Dij` per dr call, passed `&mut` to `trial`, `add_shield` and `repair_constraints`.
  3. `ShapeIndex` in dr: per layer, buckets of side `4·p0` holding shape ids; `fn near(&self, layer: LayerId, r: Rect, halo: i32) -> impl Iterator<Item = u32>`. Used for shorts, access clearance, fill foreign lists and via-array cut clearance.
  4. Fill `RouteStats` (defined in RTE-07): `std::time::Instant` around landing (dr:360–451), PathFinder (dr:525), repair (dr:531–596), geometry (dr:597–817) and fill (dr:819–877); `expanded` += heap pops in `route_net`; `trials` += trials in `repair_constraints`; `coarsened = true` when `TrackGrid` raised the pitch (gr:528; AT-37: reported, not silent). FLOW-09 copies the dr total into `RunStats.stage_ms`; the bench prints the per-stage split (PERF-20).
- Tests: `gr`: `astar_matches_dijkstra_cost` (random 30×30×2 lattices with random `hist` in [0, 2], 50 seeds, source and target random on layer 0: equal path cost to 1e-4); `astar_expands_fewer_nodes` (open 200×200×2 lattice, source (100, 100, 0), target (150, 100, 0): A* heap pops ≤ ½ of Dijkstra's).
- Acceptance: after RTE-12 + RTE-13, dr p50 per call on `ota` ≤ ⅓ of the M0 baseline; stage times printed by the bench.
- Risks / notes: A* is exact only while every added cost stays non-negative; the parasitic and penalty terms are all ≥ 0 today (gr:842–849 doc) and must stay so (a debug assertion on `node_cost ≥ 0`).

### RTE-14 Current-driven topology, EM-correct widths, EM-correct pin access, wide nets first

- Priority: P1. Effort: L. Depends on: RTE-12; REL-01 (per-pin currents), REL-03 (`Routes::terms`, `net_flow`, the EM rule including the pin-access layer and cut); optional: REL-12 (front-row cuts), REL-17 (ESD floors), EXT-24 (`em_critical`).
- Why: AT-06, AT-15 (supplies routed as free nets, last), AT-16; EM-09/10/19/20/21 (lienig_em.txt:3975–4141, 4540–4574, 4687–4699); LAMP-37; H15-25 ("begin with any signals that require wider-than-minimum leads", hastings.txt:48387–48409); H15-37/38; H05-12.
- Current: targets sorted by Manhattan distance from the root (gr:943–948); widths after search; the "narrow" access jog `min(wire, pin.w, pin.h)` (dr:384, 1209); one access cut (dr:743–749: the pin-access cut is not in `cuts` and is pushed unchanged; a stack-layer access cut gets `cuts(0)` = 1); `order_by_priority` (gr:250–271) gives supplies no priority: in the flow every ≥ 2-terminal net carries a hard `Electromigration` (lib.rs:1016–1027), so supplies share the hard tier with every other routed net and are ordered by weight (0 unless budgeted) and pin count.
- Change:
  1. EM-critical filter (EM-20): `Intent.nets[n].em_critical` when EXT-24 supplies `Some`; otherwise net `n` is critical iff `I_max > I_min`, with `I_max = max(Σ_t max(ua_t, 0), Σ_t max(−ua_t, 0))` over its terminals (`ua_t` = REL-01 table value × `pin_shares`; any `None` → not critical, EM stays unknown per REL-03) and `I_min = min_l J_l·wire_l / 1000` µA. sky130 at 90 °C: met1 2 800 µA/µm × 0.26 µm = 728 µA (sky130.json:99; derived). Non-critical nets: `k = 1` (plus RTE-22's performance `k`); this replaces RTE-12's interim per-net `k`.
  2. Planned topology for critical nets (EM-10, Lienig Fig 3.10): Prim MST over terminal centres (Manhattan), rooted at the feed (a port or supply pin, when the net has one) or else the terminal with the largest |ua|; edge current `I_e = |Σ ua over the child subtree|` (KCL; Lienig eq 3.6 with lower = upper bound, lienig_em.txt:4017–4035).
  3. Per edge and layer: `k_e(l) = 1 + ⌈max(0, I_e·1000/J_l − wire_l) / (s_l·p0)⌉`, then `max` with the REL-17 ESD floor (`⌈(esd_width − wire_l)/(s_l·p0)⌉ + 1` when set) and RTE-22's `k`, capped at `K_MAX`.
  4. Routing: `NetSearch.terms` in the planned Prim order and `NetSearch.k[i] = k_{e(i)}` for the branch that reaches terminal `i`.
  5. Verify and repair: after geometry, evaluate REL-03's `Electromigration` for the net on the drawn shapes (it runs `net_flow`); `build_routes` records, per net, a provenance vector parallel to the net's shapes, `origin: Vec<u16>` (branch index, `u16::MAX` for access shapes); each violating trunk shape → its branch gets `k += 1` on that layer, each violating via group → `k += 1` on both layers of that corner; reroute the net; at most `EM_ROUNDS = 3` (tuning default); what remains is REL-03's V entry. With REL-12 on, a `k×k` corner satisfies a need of `n` cuts only if its front row (the row across the current) holds ≥ `n` cuts, else `k += 1`.
  6. Access (dr:1174–1317): jog width = `max(W_l(k_access), I_access·1000/J_l)`; the `narrow` option is only allowed when the pin carries no current. Access cuts: `⌈I_access / I_cut⌉` cuts placed on the cut pitch inside `pin ∩ pad`; when they do not fit, try the terminal's other regions (they share the current); if none fits → V `em access net {n} pin {name}` (EM-19: the cell must widen the strap; reported to CELL).
  7. Order key in `order_by_priority` (gr:250–271): 0 = matched pairs, 1 = nets with any `k > 1`, 2 = other hard, 3 = budget, 4 = free, 5 = shield references. Within a tier: decreasing `F_k` once RTE-21 step 4 lands; until then decreasing `net_weight`, then pin count, as today (gr:247–248). Tests: `gr` `wide_nets_route_before_other_hard_nets` (three hard nets, one with `k = 2` → it is first in the order after the matched pair).
- Tests: `dr`:
  - `a_trunk_carrying_two_branches_is_wider`: sky130 met1 values (`wire` 260, `p0` 420, J = 2 800 µA/µm); a feed pin at `ua = −2 000` and two leaf pins at `+1 000` each, placed so the MST is feed → Steiner → two leaves. Trunk need 2 000·1 000/2 800 = 714.3 nm → `k = 1 + ⌈(714.3 − 260)/420⌉ = 3` (W = 1 100 nm); each leaf need 357.1 nm → `k = 2` (W = 680 nm) (derived).
  - `access_jog_never_necks_below_em`: 170×170 li pin drawing 1 000 µA → the met1 access jog is ≥ 357.1 nm wide (derived: 1 000·1 000/2 800).
  - `access_cuts_follow_current`: 800 µA through mcon at 360 µA/cut (sky130.deck:491, via REL-03 step 6) → ⌈800/360⌉ = 3 cuts drawn inside `pin ∩ pad`.
- Acceptance: Electromigration V = 0 and IrDrop Θ = 0 on every bench circuit with an operating point; no `narrow` access jog on a net carrying current.
- Risks / notes: the MST can differ from the routed tree; step 5 closes the gap on the drawn geometry, which is what REL-03 and signoff check. `branch_currents` (dr:897–934) and the `em underwidth`/`em cuts` Θ entries (dr:1852–1854, 882–884) are deleted here: REL-03's rule is the only EM measure.

### RTE-15 Joint mirrored routing of matched net pairs

- Priority: P1. Effort: L. Depends on: RTE-05, RTE-10, RTE-12; PLC-28 (axis contract, step 3); EXT-09/EXT-24 (the `Differential` pairs).
- Why: AT-04, AT-25, AT-33 (mirror gates, outputs, cascodes), AT-38; LAMP-40 ("a new cell is first checked … on the side that is actually routed … mirrored … and checked again", lampaert.txt:6199–6228); BAL2-07 (ANAGRAM II evolves both halves, ROAD mirrors obstacles, balasa_graeb_survey.txt:8827–8900); SURV-13/14/15; TODAES ("two nets with a symmetry constraint are always assigned with the same width", todaes22.txt:212–215).
- Current: first-come landing (dr:196–204); lattice not aligned to `Layout::axis` (dr:218, gr:641–644); `copy_tree` needs exact bin symmetry (dr:1643–1691); `mirror_guide` is a 1.0/node soft field about the mean terminal x (dr:1697–1718); pairs route blind to parasitics (`plain`, dr:469–472); each half fattened to its own need (dr:673–674).
- Change:
  1. Input `DetailedCfg::pairs: Vec<PairSpec>`:
     ```rust
     pub struct PairSpec { pub a: NetId, pub b: NetId, pub map: SymMap }
     pub enum SymMap { MirrorX { axis2: i32 /* 2·axis x, absolute nm */ }, Shift { dx: i32, dy: i32 } }
     ```
     The library builds one `PairSpec` per `Differential` rule in `reqs.hard ∪ reqs.budget` (EXT-09/EXT-24 emit them): `a = pos`, `b = neg`; `axis2 = 2·Layout::axis[id]` of the `AxisId` of the devices whose pins the pair's nets reach; `map = MirrorX` when every pin rect of `a` has a `b` pin equal to its mirror about `axis2` within one grid step, else `Shift` when one `(dx, dy)` (the difference of the two pin-set centroids) maps every pin (PLC-21's Perfect mode), else no `PairSpec` (step 7).
  2. Lattice map (`gr`): `pub enum LatticeMap { MirrorX { k: i32 }, Shift { dx: i32, dy: i32 } }`, `TrackGrid::map(&self, m, n) -> Option<u32>`. MirrorX: `(ix, iy, l) → (k − ix, iy, l)` with `k = (axis2 − 2·origin_x − p0) / p0`; valid iff integral and `k ≡ 0 (mod stride)` for every vertical layer. Shift: `dx, dy` multiples of `p0` and, on stride layers, of `stride·p0` across their tracks. An invalid map → fallback (step 7).
  3. Contract with PLC (provided through `LatticeSpec`, RTE-25): every vertical symmetry axis satisfies `x_axis ≡ p0/2 (mod p0·max(1, s_max/2))` in absolute nm, i.e. it lies on a base-track centreline. With the origin a multiple of `p0·lcm(strides)` (RTE-10), `k` is then even and stride-2 layers map tracks to tracks (derived). sky130: axes at `210 + m·420` nm.
  4. Joint landing, before every other net: for each pin of `a` (sorted by `(y, x)`), its partner is the `b` pin whose rect equals `map(rect)` within one grid step; none → fallback. A landing node `n` for `a` must satisfy clean_a(n), clean_b(map(n)), `map(n) ≠ n`, and the mirrored jog legs must be clean for `b`. Both are landed together.
  5. Joint search: `NetSearch` gains `pub mirror: Option<Mirror<'a>>` with `pub struct Mirror<'a> { pub map: LatticeMap, pub partner: u32, pub partner_own: &'a [u32], pub partner_penalty: &'a [f32] }`; node legal iff legal for `a` at `n` and for `b` at `map(n)` and `map(n) ≠ n`; node cost = cost_a(footprint_a(n)) + cost_b(footprint_b(map(n))). Widths are tied: `k = max(k_a, k_b)` per branch. Commit `a`'s tree and `map(tree)` for `b` with `side_b = −side_a` on layers whose across axis the mirror flips.
  6. `RouteCtx::plain` and its uses (gr:738–742, 794–795; dr:469–472) are deleted: the joint cost is symmetric by construction.
  7. Fallback (no exact pin map, invalid lattice map): route `a`, then `b` with the existing `mirror_guide` field; RTE-05 reports the true mismatch; no stub.
- Tests: `dr`:
  - `mirrored_pins_route_as_exact_mirrors`: `p0 = 420`, axis `x = 210 + 24·420 = 10 290`; pins mirrored; an obstacle on `a`'s side only → both nets detour (Lampaert Fig 5.11); per-layer lengths, areas and cut counts equal; `Differential` mismatch 0.0.
  - `the_axis_track_is_never_used_by_a_pair`: same set-up without the obstacle → no node of either tree has `ix = k/2` (the axis column) on any layer.
  - `unmatched_pins_fall_back_to_the_guide`: `b`'s pins shifted by one grid step off the mirror → both nets routed, no `PairSpec`, and every shape of both nets lies on a path between two of its terminals (no stub).
- Acceptance: on `ota`, `ota_constrained`, `tt_ota`, every matched pair meeting the PLC-28 contract has Differential mismatch 0.0 % (the bench prints "pairs exact n/m", PERF-20); pairs that fall back are listed with their reason (`no pin map`, `axis off lattice`).
- Risks / notes: exactness needs PLC-28's snapping and EXT's pairs; without them quality equals today's.

### RTE-16 Keep metal off matched gates and precision resistor bodies, by match class

- Priority: P1. Effort: M. Depends on: RTE-03; CELL-01 (`Macro::keepouts`, fallback below); MAT-07/FLOW-06 (`pnr_core::MatchClass` on `Unitization.class`, default `Moderate`).
- Why: AT-14; H13-51 (Hastings §13.3 rule 17: moderately and exceptionally matched transistors "must not have metal leads crossing their active areas", hastings.txt:42603–42613); H13-36 (metal coverage causes up to 20 % drain-current mismatch "regardless of what metal layers are involved", hastings.txt:41786–41798); H08-27/28 (leads over resistors by class and sheet R, hastings.txt:24840–24864, 25392–25398); H06-14 (lightly doped resistors are modulated; cross over heads, not bodies, hastings.txt:16920–16928, 17481–17487); H14-36 (hastings.txt:44148–44153); SURV-16 (no routing over active, perf_driven_survey L83–84); AV-11 (no metal-over-gate check on the GDS).
- Current: soft `KEEPOUT_COST = 2.0` per node over the whole bbox of a matched cell, foreign nets only (gr:757, dr:476–498); resistors unprotected; nothing measures the result.
- Change:
  1. `frontend/library/src/elaborate.rs` (named `Blockage` because CELL-01 adds `pnr_core::Keepout`):
     ```rust
     pub struct Blockage { pub rect: Rect /*absolute*/, pub layers: LayerMask /*routed layers it blocks*/, pub hard: bool,
                           pub own_exempt: bool, pub only_aggressors: bool, pub cell: u32 }
     pub(crate) fn blockages(placed: &[Macro], pdk: &Pdk, class_of: impl Fn(usize) -> Option<pnr_core::MatchClass>) -> Vec<Blockage>;
     ```
     `LayerMask = u16` over the routed-layer index. `class_of(c)` = the `class` of the `Unitization` whose members the cell draws (MAT-07; `Moderate` when the field is absent); `None` = the cell is in no matched set → no blockage. Source rects: `placed[c].keepouts` (CELL-01; `place_macro` already moved them); until CELL-01 lands, gates = intersections of the cell's `poly` and `diff` role shapes and resistor bodies = `rpoly` role shapes (sky130 `poly_rs`, pdks/sky130.json:13; ihp `res`) shortened by `res_head` (sky130 2 160 nm, pdks/sky130.json:42) at both ends. Every rect is grown by `S` of the lowest routed metal. Policy by `KeepWhy`:
     - `Gate`: `Moderate`/`Exceptional` → hard on all routed layers, `own_exempt = false` (H13-51 rule 17; H13-36 "regardless of what metal layers are involved"). `Minimal` → soft (`KEEPOUT_COST`, today's value) for foreign nets, `own_exempt = true`.
     - `ResistorBody` (heads stay crossable, H06-14): hard on all routed layers for foreign nets when the class is ≥ `Moderate`, or the body's sheet R > 500 Ω/□ (H08-28: the MIN allowance holds only for ≤ 500 Ω/□; H06-14 names > 1 kΩ/□ as modulated), or the foreign net is an aggressor class (`NetClass::Clock` today, EXT-18's Noisy/DigitalSwitching later; H08-28 "never switching digital or HF analog"); otherwise soft (`KEEPOUT_COST`). The sky130 body sheet value is disputed (AV-16, FLOW-05); with `Moderate` as the default the threshold does not decide any sky130 fixture.
     - `CapPlate`: class ≥ `Moderate` → hard for foreign nets on the routed layers below the plate's top conductor (H08-24: "nothing may be routed beneath them"); aggressor-class nets → hard on all routed layers over the plate grown by 3 000 nm (H08-23: "route them ≥3–4 µm around the shielded area", lower end); other foreign nets over the plate → soft (`KEEPOUT_COST`).
  2. dr: a hard blockage marks every node of its `layers` whose footprint intersects it `BLOCKED` (for the cell's own nets too unless `own_exempt`; for aggressor-class nets only when `only_aggressors`: those nodes go into a sorted per-net list `RouteCtx::blocked_for: Vec<Vec<u32>>`, which `route_net` checks like `reserved`); `jog_clean` (dr:1098–1103) also rejects access legs crossing a hard blockage; soft blockages replace `RouteCtx::keepout`/`own_cells` (gr:748–757, dr:476–498) with the same cost.
  3. New rule `kernel/analog/src/routing/metal_over_gate.rs`: `MetalOverGate { rect: Rect, cell: u32 }` (hard arm): violated when any routed shape of any net overlaps `rect`; `residual` = overlap area in µm². The library registers one per hard gate keep-out.
- Tests: `dr`: `no_route_crosses_a_moderately_matched_gate` (foreign two-pin net whose straight route crosses a four-gate matched cell → `MetalOverGate` overlap 0 µm², the route goes around); `own_drain_does_not_cross_its_gates_at_mod` (the cell's own drain net, pins on both sides of a gate → overlap 0); `a_min_class_pair_allows_crossing_at_a_cost` (same as the first with `Minimal` and the detour blocked → the route crosses, overlap > 0); `a_clock_never_crosses_a_resistor_body` (`Minimal` resistor, 300 Ω/□, a `Clock` net and a Signal net both needing to cross → the Signal crosses the body, the Clock crosses a head).
- Acceptance: on the final GDS of `ota` and `tt_ota`, `MetalOverGate` V = 0 (0 µm²) with no new unrouted nets.
- Risks / notes: pins lie on S/D strips between gates. On sky130 the strip is `730 − 150 = 580` nm (AC-04 pitch at L = 150); minus `2·140` of halo leaves 300 nm ≥ one 260 nm met1 jog (derived). The 5–10 µm EXC "similar surroundings" halo (H13-36) is left to MAT's scoring.

### RTE-17 Split-net topologies: star nodes, Kelvin force/sense, separate ring returns

- Priority: P1. Effort: L. Depends on: RTE-12, RTE-15, REL-03 (`net_flow`, for the Kelvin check); EXT-24 (`CommonNodeReq`, `StarReq`, `KelvinReq`); MAT-06 (`max_delta_ohm`).
- Why: AT-15; H15-33 (star node, 10 squares × 30 mΩ/□ × 1 mA = 0.3 mV between matched emitters, hastings.txt:48509–48538); H15-34 (Kelvin force/sense, match the sense-lead R, hastings.txt:48540–48549; "signals which should Kelvin at a bondpad actually do so", checklist item 12, hastings.txt:48940–48944); MM-33 (8.3 mV/mm IR drop masquerades as a gradient); BAL2-04/06 (net → subnet → group; net splitting, balasa_graeb_survey.txt:8666–8734, 8806–8823); BAL2-46; H08-21; H14-10 and SUB-30 (separate injector/victim ring returns); NOTES-46.
- Current: `CommonNode { net, a, b, feeds, max_delta_ohm }` measures ΔR only (common_node.rs:20–53); dr's repair is a Manhattan balance field (dr:1792–1805); the library builds common nodes only for 2-device DiffPair/CurrentMirror/Load leaves (lib.rs:780–785); nothing enforces a star.
- Change:
  1. ```rust
     pub struct CommonNode {
         pub net: NetId,
         /// Terminal groups; each group's branch is its own subtree from the root.
         pub groups: Vec<Vec<Rect>>,
         /// The root: where current enters (a tail drain, a rail port) or the Kelvin tap.
         pub feeds: Vec<Rect>,
         /// Allowed max |R_i − R_j| between groups, Ω; ≤ 0 = no balance check.
         pub max_delta_ohm: f32,
         /// Branches may meet only inside the root halo.
         pub star: bool,
     }
     ```
     Star check: drop the net's shapes intersecting the root halo (feeds grown by `p0`); components by the joins relation (RTE-03); violated iff a component holds terminals of two groups. `violations` counts star breaks plus ΔR overshoots; `residual` as today for ΔR.
  2. dr subnets: each group of a star node is routed as its own compact pseudo-net (terminals = the group's pins + one root node inside the feed pin's halo; distinct root nodes per group). If the root cannot host a node per group → V `star root too small net {n}`. Two groups that EXT marks as matched route jointly (RTE-15), giving ΔR = 0. The pseudo-nets' shapes are appended to the parent net in `build_routes`.
  3. Kelvin: `groups = [force pins, sense pins]`, `feeds = [the device terminal rect]`, `star = true`, `max_delta_ohm = 0`; the sense branch starts at the terminal itself.
  4. `Flow::common_nodes` (lib.rs:763–801): one `CommonNode` per EXT-24 `CommonNodeReq` (its two sides → two groups, `star = false`). Allowance: `max_delta_ohm = min(MAT-06 remaining_mv, EXT-24 max_delta_mv) / I_ua · 1e3` Ω (ΔV = I·ΔR; mV/µA·1e3 = Ω, the unit conversion lib.rs:795 already uses), with `I_ua` = |I_D| of the side's first member (`self.id_ua`, lib.rs:793); either allowance absent → the other; `I_ua` unknown or no allowance → `0.0` (= no balance check, as today). MAT-06 edits the same function first (plan-02 MAT-06 step 2); RTE-17 changes only the constructor to `groups`; a `StarReq` on the same net **supersedes** that net's `CommonNodeReq`s (the decision EXT-24 leaves to RTE): one `CommonNode { groups = the star's branches, feeds = [root], star = true, max_delta_ohm = the smallest MAT-06 value among the superseded requests }`; each `KelvinReq` → step 3. Until EXT-24 lands, today's 2-device leaves map to two groups. `balance_field` (dr:1792–1805) is deleted once matched groups route jointly.
  Separate injector/victim ring returns (H14-10, SUB-30), formerly step 4, are deferred (C7): no plan emits a ring-return request.
- Tests: `common_node.rs`: `two_groups_match_the_old_a_b_measure` (the existing ΔR tests rewritten with `groups = vec![a, b]`, `star = false`: same residuals as before); `a_shared_segment_breaks_the_star` (two groups whose branches share a 5 µm segment outside the root halo → 1 violation); `branches_meeting_at_the_root_are_a_star` (the same branches meeting inside the feed rect grown by `p0` → 0). `dr`: `a_kelvin_sense_branch_starts_at_the_tap` (a met1 test net: force pins draw +1 000 µA from a `−1 000` µA feed, one sense pin at `0` µA; REL-03 `net_flow` gives `shape_ua` = 0.0 on every shape between the sense pin and the tap, and the star check passes).
- Acceptance: on `ota`, every common node meets its ΔR allowance; every EXT-requested star/Kelvin passes the topology check; sense branches carry 0 current.
- Risks / notes: a small feed pin may not host one root node per group; the V entry names it so CELL/EXT can enlarge the root (a strap or a pad).

### RTE-18 Crosstalk: separation during the search, crossing capacitance, aggressor weighting and screening

- Priority: P1. Effort: M. Depends on: RTE-02 (weights, `exclude`), RTE-10; EXT-24 (`CrosstalkExclusion` by class), EXT-18 (aggressor classes).
- Why: AT-13, AR-14 (no screening, no inter-layer C, overlapping shapes double-counted); H15-29 (ΔV = C·R·dV/dt, Eq 15.23, hastings.txt:48574–48595); H15-30 (no noisy lead beside or on an adjacent layer to a sensitive one; cross at right angles, hastings.txt:48625–48629, 48683–48687); H14-07 (Eq 14.1: one M1×M2 crossing ≈ 0.05 fF, hastings.txt:43023–43042); H06-18; BAL2-09 (C_1D = A·C_α + S·C_β, C_2D, Eqs 4.2–4.3, balasa_graeb_survey.txt:8912–8966); BAL2-13; NOTES-44; LAMP-39 (½·S_Cki·C_ki counts both nets, lampaert.txt:6140–6160).
- Current: router coupling = the two adjacent same-layer tracks (gr:648–660, 916–924); the rule sums every same-layer pair at any gap (coupling.rs:49–68); nothing prices inter-layer overlap; `CrosstalkExclusion` is repaired by an 8-neighbour field that cannot reach spacings above one track (dr:1807–1822).
- Change:
  1. Crossing capacitance per area between adjacent routed layers from the deck's existing `pex` keys: `c_x(l) = ε0·k_l / (h_{l+1} − (h_l + t_l))` with `k_l` the **lower** metal's `dielectric` value, ε0 = 8.854 aF/µm — the same parallel-plate term GPurify extracts ("The dielectric between them is the one above the lower plate", `k = dielectric_k[lower]`, gpurify crates/extract/src/analytical.rs:237–242 at the pinned 8df8c09). sky130 (sky130.deck:590–596, derived): met1–met2 8.854·4.5/0.270 = 147.6 aF/µm²; met2–met3 8.854·4.2/0.420 = 88.5 aF/µm²; met3–met4 8.854·4.1/0.390 = 93.1 aF/µm². New `Pdk::overlap_af_um2(lo, hi) -> Option<f32>` (pdk.rs, coordinated with FLOW).
  2. Router: `RGraph::stacked(&self, n: u32, out: &mut [u32; 2]) -> usize` (nodes directly above and below; default impl returns 0, `TrackGrid` implements it); `Elec` gains `cross_c: &[f32]` (per layer pair, `c_x·wire_l·wire_{l+1}` over the cheapest step's ground C) and `a_self: f32`; `RouteHot` gains `agg: Vec<f32>` (Σ aggressor weight of the nets on a node, kept by `commit` like `sens`). The coupling term for a neighbour `j` (beside or stacked) becomes `coef·(w_self·agg[j] + a_self·sens[j])`, replacing `(weight + their sens)` (gr:916–924).
  3. Separation during the search (hard):
     ```rust
     pub struct Separation { pub a: NetId, pub b: NetId, pub lateral_nm: i32, pub no_cross: bool }
     ```
     from every `CrosstalkExclusion` (`lateral_nm = min_spacing_nm`, `no_cross = false`) and from RTE-20's plate sets (`no_cross = true`); EXT emits no no-cross pair today (open question 8); per layer `tracks = ⌈max(0, lateral_nm − (s_l·p0 − wire_l)) / (s_l·p0)⌉`. A node is illegal for `a` if a node within `tracks` across on the same layer, or (with `no_cross`) a stacked node, is used by `b`. The first-routed net keeps its route; the second avoids it. `keep_away` (dr:1807–1822) is deleted.
  4. `coupling::net_pair_af` (added by RTE-05) and `CouplingBudget::total_af` (coupling.rs:49–68, which becomes `Σ_other weight(other)·net_pair_af(..)` with the victim's runs merged once): (a) the victim's same-layer shapes are merged per layer into maximal runs first, so a pad over a wire end counts once (AR-14); (b) screening: per victim run and side, foreign same-layer parallel shapes sorted by gap, each counted only over the run length no nearer shape (of any net, the victim's shield included) already covers — `net_pair_af` gains a fourth parameter `screens: &[Shape]` (the shapes of every net other than the two measured; callers that want no screening pass `&[]`), and `total_af`, `Differential` and `PlateRatio` pass `r.wires` minus the two nets; (c) crossings: shapes on adjacent routed layers overlapping in xy add `A_overlap·c_x` (the perimeter term C_β is not in the deck: not given, omitted); (d) weights and `exclude` (RTE-02).
- Tests: `coupling.rs`: `a_wire_behind_a_nearer_wire_is_screened` (X at 280 nm over the full run, Y at 1 000 nm behind X → Y adds 0); `a_crossing_couples_by_overlap_area` (260-wide met1 crossed by a 280-wide met2 → 0.0728 µm² × 147.6 = 10.7 ± 0.1 aF). `dr`: `separated_nets_never_share_adjacent_tracks`; `a_no_cross_pair_never_stacks`.
- Acceptance: all `CrosstalkExclusion` rules satisfied on every fixture; on `ota`, for each net with a `CouplingBudget`, `total_af` computed with every weight 1, no `exclude`, and shapes `r.shapes(n) ∪ r.cell_metal(n)` for every net lies within 20 % of the sum over other nets of GPurify's extracted victim–net coupling on the same final GDS (printed by the bench as a calibration row; PERF-16 owns PEX fidelity).
- Risks / notes: hard separation is first-come, so order matters; RTE-14's order (pairs, wide nets, hard, budget by `F_k`) routes the victims before most aggressors. The inter-layer term omits fringe (C_β), so it under-estimates crossings; the 20 % PEX check bounds the error.

### RTE-19 Shields generated during the search

- Priority: P1. Effort: M. Depends on: RTE-02, RTE-12, RTE-18; EXT-24 step 2 (`shield_ref`; fallback ground).
- Why: AT-22; H15-32 (shield tied to the victim's own reference, hastings.txt:48638–48667); BAL2-10 (space first, shield last; a shield is tied to a DC potential); NOTES-45; SUB-30.
- Current: `add_shield` claims free stretches after repair and keeps them all-or-nothing (dr:585–596, 1556–1621); the reference is always ground and only when a clock exists (extract.rs:87–104).
- Change:
  1. A victim with a `Shield` rule routes with a `(k + 2)`-track footprint centred on its `k` tracks (`side = 0`, RTE-12): the two outer tracks are reserved for the reference net (`reserved[n] = reference`) instead of counting as the victim's usage; when `max_gap_nm` (shield.rs:26) is below the outer tracks' edge gap `s_l·p0 − (W_l(k) + wire_l)/2`, the rule cannot be met on the lattice and is reported, not attempted.
  2. After the victim's tree is committed, the existing tie-in (dr:1597–1620) reroutes the reference with one node of each reserved stretch as an extra terminal; stretches are already reserved, so the tie-in only fails when the reference cannot reach them.
  3. Reference: EXT's per-victim reference; fallback ground as today.
  4. Plate shields between layers are not added: on this lattice adjacent layers are perpendicular, so a victim can only be crossed, and crossings are priced or forbidden by RTE-18.
- Tests: `dr`: `a_shielded_victim_gets_both_side_tracks` (a 20 µm two-pin victim with a `Shield { min_coverage_pct: 80 }` → shield.rs coverage ≥ 0.95 after RTE-02's intersection fix); `a_shield_does_not_count_as_an_aggressor` (the same routes: the victim's `CouplingBudget` with `exclude = Some(reference)` reads the same total with and without the reference's shield tracks).
- Acceptance: every requested shield reaches its `min_coverage_pct` (80, extract.rs:98) on the unit fixtures and on any bench circuit with a clock (PERF adds a comparator fixture, SURV-40).
- Risks / notes: three tracks per shielded run triple the victim's track use; shields are only requested against a real aggressor (extract.rs:87–92 policy, kept).

### RTE-20 Capacitor-set plate routing: plate roles, top/bottom separation, per-unit lead capacitance

- Priority: P1. Effort: M. Depends on: RTE-18 (`Separation`, crossing C); FLOW-07 (`c_af`, the schematic capacitance, for `c_unit_af`). Plate R/C weighting (formerly step 3) moves to RTE-21 step 1, which it needs.
- Why: AT-33; CC-14 (C_TS gives gain error, C_TB code-dependent nonlinearity, C_BS only power, cc_dac_constructive.txt:142–173); CC-28 ("nonoverlapped routing that separates the wires that route the top-plate and bottom-plate", cc_dac_constructive.txt:792–794); CC-44 (routing wirelength proportional to capacitance keeps ratios, cc_review.txt:349–353); H08-22 (equal lead C per unit; rule C10: "insert jogs or dead-end branches into its lead until the two capacitances match", hastings.txt:22640–22670, 25584–25595); AV-06 / AV-32 (dac4 top–bit couplings 0.42/0.85/1.67/3.36 fF, audit-06 L310–311; the numbers sit in audit-06's narrative and in AV-32, not in the AV-06 row).
- Current: inside the array the generator keeps C_TB structurally absent (cap_array.rs:12–17); outside it dr has no plate policy; `trim_pair`, which held the stub machinery, was deleted by RTE-05 (M1, `7f8d92b`); its last version is in `7f8d92b^:backend/dr/src/lib.rs`.
- Change:
  1. `DetailedCfg::plates: Vec<PlateSet>`: `pub struct PlateSet { pub top: NetId, pub bits: Vec<(NetId, u32 /*units*/)>, pub c_unit_af: f32 }`. The library builds it from `cellgen::dac_banks` (cellgen.rs:760, made `pub(crate)`; one `Vec<DeviceId>` per bank, members sorted by `m`, slot 0 the electrical dummy): `top` = the `P` terminal net of any member; `bits` = `(N net, m)` of every member whose `N` net is not a rail (the dummy's bottom plate is a bulk rail, cellgen.rs:779–782); `c_unit_af` = FLOW-07's `c_af` of a member with `m = 1`, `f32::NAN` when absent. EXT-19's capacitor sets replace the source later; the struct stays.
  2. Separation: for every bit net, `Separation { a: top, b: bit, lateral_nm: S_l of the first routed metal, no_cross: true }` (RTE-18).
  3. New rule `kernel/analog/src/routing/plate_ratio.rs`:
     ```rust
     pub struct PlateRatio { pub set: &'static PlateSet, pub array: Rect, pub tol_pct10: i32, pub stack: &'static Stack }
     ```
     `array` = the placed cap-array macro's bbox. Lead C of bit `i`: `C_i = stack.ground_af(S_i) + Σ_{other nets m} coupling::net_pair_af(Some(stack), S_i, r.shapes(m))` with `S_i` = bit `i`'s routed shapes not contained in `array` (RTE-18's screened measure, weights 1). `spread_pct = max_i |C_i/n_i − mean_j(C_j/n_j)| / c_unit_af · 100`; `violations = (spread_pct > tol) as u32`, `residual = over(spread_pct − tol, tol)` (rule.rs) with `tol = tol_pct10/10`, default 1 % (tuning default; H08-22 gives the check form, not a number); `known` = `c_unit_af` finite and every bit net has shapes. Budget arm.
  4. Repair `equalize_leads` (restored from `trim_pair`, `7f8d92b^`): `target = max_j C_j/n_j`; for each bit with `C_i/n_i < target`, add a same-layer dead-end stub continuing one of its runs outside `array`, length `L = (target − C_i/n_i)·n_i / c_per_nm(l)` nm, with `c_per_nm(l) = (area_af_um2(l)·wire_l/1000 + 2·fringe_af_um(l))/1000` aF/nm (Stack terms, stack.rs:55–64); keep it only if `spread_pct` drops and the stub clears foreign metal by `S_l`.
- Tests: `dr`: `bottom_plates_never_cross_the_top_plate` (synthetic 4-bit bank: one macro with pins `P` and four `N_i`; after routing, no bit-net shape overlaps a top-net shape in xy on adjacent layers and every same-layer gap ≥ `S`); `plate_ratio.rs`: `lead_capacitance_is_equalised_per_unit` (bits of 1/2/4/8 units, equal 10 µm met1 leads, `c_unit_af = 1 000` → before: spread > 1 %; after `equalize_leads`: spread ≤ 1 %, i.e. every `C_i/n_i` within 10 aF of the mean).
- Acceptance: `dac4`: no top-plate/bit-plate crossing outside the array; `PlateRatio` residual 0.
- Risks / notes: the array-internal parasitics are CELL's (CC-26/27/29); this item only governs what leaves the macro.

### RTE-21 Performance-driven measurement, pricing, ordering and repair

- Priority: P1. Effort: L. Depends on: RTE-09, RTE-18, RTE-32; step 3 has no PERF dependency and lands first (it only reads fields EXT-25/PERF fill, empty until then). Consumes: EXT-25 (row fields `r_nets`/`r_weights`, `diff_pairs`/`diff_weights`), PERF-06 step 2 (`limit`), PERF-12 step 2 (`perf::router_weights` fills `cfg.r_weight`/`cfg.pair_weight`) and step 3 (`coupling`); optional EXT-24 step 8 (`RcClass`).
- Ownership note (review): plan-07 lists "`PerformanceBudget` ground-C/R measurement (RTE-21 step 3)" and "budget repair (RTE-21 step 5)" as RTE's (plan-07 §0.4, PERF-12 header, its Cut table), and EXT-25 step 3 says "Until RTE measures R and ΔC, `used()` ignores the new fields". The previous draft of this item had moved the measurement to PERF-12 (C6), so no plan owned it. It is restored here as step 3, and the step numbers match plan-07's references.
- Why: AT-24, AT-26, AT-17 (R part); AR-22 (PerformanceBudget uses length × one C/nm, performance.rs:31–33); LAMP-39 (Eq 5.10 `ΔP_j = S_Ci·C_i + S_Ri·R_i + Σ ½·S_Cki·C_ki`, lampaert.txt:6099–6178); LAMP-42 (impact factor `F_k = (Σ_j ΔP_j^k/ΔP_j)/N_s`, Eqs 5.12–5.14, lampaert.txt:6282–6348); BAL2-16 (adverse-direction split, Eqs 4.6–4.7, balasa_graeb_survey.txt:9334–9360); BAL2-18/19 (weight ∝ sensitivity/allowed variation; ROAD raises the weights of violated parasitics, 9431–9449); SURV-19 (R-sensitive nets avoid resistive layers and via stacks).
- Current: router weight = normalised `gp::net_weights`, zero unless budgeted (lib.rs:312–316); R priced only after an IR budget broke (dr:531–550); order within a tier by the net's own weight (gr:247–248 ponytail); blind `_` repair (dr:1499–1505); `PerformanceBudget::used` = Σ w·length·`af_per_nm` (performance.rs:31–33).
- Change:
  1. `DetailedCfg`: keep `net_weight`; add `r_weight: Vec<f32>` (per net, max 1) and `pair_weight: Vec<(NetId, NetId, f32)>` (per aF, same normalisation as `net_weight`), `Default` empty. PERF-12 step 2 (`perf::router_weights`) fills them after `elaborate::detailed_router` (lib.rs:297–316); RTE does not build them from rows. Without PERF data the library applies the `RcClass` fallback next to `net_weight` (lib.rs:312–316): `r_weight[n] = 1` for nets whose EXT-24 `RcClass` is `R` or `Rc` (DAC bottom plates, CC-15), `net_weight[n] = max(net_weight[n], 1)` for class `C` or `Rc` (DAC top plate); no class → empty = today.
  2. `Elec`: series-R term `(current + r_weight)·layer_r` per step and `·via_r` per via; the coupling coefficient between two nets uses `pair_weight` when present, else RTE-18's self terms.
  3. `PerformanceBudget` measurement (`kernel/analog/src/routing/performance.rs`). The struct becomes the union of the EXT-25 and PERF contracts:
     ```rust
     pub struct PerformanceBudget {
         pub metric: String,
         pub nets: Vec<NetId>, pub weights: Vec<f32>,             // 1/aF, ground C (today)
         pub af_per_nm: f32,                                       // fallback C per nm when `stack` is None
         pub r_nets: Vec<NetId>, pub r_weights: Vec<f32>,          // 1/Ω (EXT-25 step 3)
         pub diff_pairs: Vec<(NetId, NetId)>, pub diff_weights: Vec<f32>, // 1/aF on |C_a − C_b| (EXT-25 step 3)
         pub coupling: Vec<(NetId, NetId, f32)>,                   // 1/aF on C_ab (PERF-12 step 3)
         pub limit: f32,                                           // 1.0 unless PERF-06 step 2 sets 0.0
         pub stack: Option<&'static Stack>,
     }
     ```
     `used(r) = Σ w_n·C(n) + Σ wr_n·R(n) + Σ wd·|C(a) − C(b)| + Σ w_ab·coupling::net_pair_af(stack, r.shapes(a), r.shapes(b))` with `C(n) = stack.ground_af(r.shapes(n))` (RTE-32's union) or `r.length(n)·af_per_nm` without a stack, and `R(n) = stack.path_resistance_ohm(r.shapes(n))` (stack.rs:94–146, the worst-path bound; REL-04's terminal-resolved drop replaces it only for IrDrop). `violations = u32::from(used > limit)`, `residual = (used − limit).max(0)`, `criticality = (used / limit.max(1e-6)).clamp(0, 1)` (PERF-06 step 2's definitions); `unknown(r) = 1` when `stack` is None and any of `r_nets`, `diff_pairs`, `coupling` is non-empty (those terms then contribute 0). `touched` also lists `r_nets`, both nets of every `diff_pairs`/`coupling` entry with a positive weight. Constructors: perf.rs:254 and the test at performance.rs:83 gain `..` fields via a `PerformanceBudget::ground_c(metric, nets, weights, af_per_nm)` helper that sets the new fields empty, `limit = 1.0`, `stack = None`; the library sets `stack: Some(self.stack)`.
  4. `F_k` ordering: a pre-route pass routes every net alone on the empty lattice (A*, no congestion, footprints at `k = 1`); from its geometry `ΔP_j^k = Σ S·p` (the per-net terms of step 3's `used`); `F_k` as above; within a tier, decreasing `F_k`; the PathFinder present cost of net `k` is scaled by `1/(0.1 + F_k)` (LAMP-42 recipe; 0.1 is a tuning default).
  5. Repair by kind (RTE-09): `Budget` → multiply the named nets' `net_weight`/`r_weight` by `(1 + 0.5^round·residual)` (round = 0, 1, …; BAL2-19's damping), capped at 4× the initial value, and reroute; `Em` → no trial (RTE-09); `Ir` → `k + 1` on every branch on the path from the feed to the terminal with the largest `Stack::fed_resistance_ohm` (stack.rs:167–177), then reroute. This replaces the IR repricing block (dr:531–550) and the blind `_` arm (dr:1499–1505).
- Tests:
  - `performance.rs` (local leaked stack with the stack.rs helper values: area 25 aF/µm², fringe 40 aF/µm, sheet 0.125 Ω/□, lateral 3.9·8.854·360; after RTE-32): net a = one met1 shape 10 µm × 0.5 µm, net b = 5 µm × 0.5 µm. `c_and_r_terms_add`: `weights [1e-3]` on a, `r_weights [0.01]` on a → C(a) = 25·5 + 40·21 = 965 aF, R(a) = 0.125·20 = 2.5 Ω, `used = 0.965 + 0.025 = 0.990 ± 1e-4`, violations 0; with `limit = 0.5` → violations 1, residual 0.490 ± 1e-4. `a_pair_term_reads_the_difference`: `diff_pairs [(a, b)]`, `diff_weights [1e-3]` → C(b) = 25·2.5 + 40·11 = 502.5, `used = 0.4625 ± 1e-4`. `a_coupling_term_uses_the_lateral_measure`: a and a second 10 µm × 0.5 µm met1 net at 280 nm edge gap, `coupling [(a, c, 1e-3)]` → `used = 12 431.0·10/280·1e-3 = 0.444 ± 1e-3`. `r_terms_without_a_stack_are_unknown`: `stack None`, `r_nets` non-empty → `unknown == 1`, R contributes 0.
  - `gr`: `an_r_weighted_signal_avoids_vias` (like `a_current_carrying_net_avoids_via_resistance`, gr:1366–1379, driven by `r_weight = 1` with `current = 0`).
  - `dr`: `budget_repair_escalates_and_moves_the_net` (a `PerformanceBudget` row with a `coupling` term on one net whose first route runs beside the aggressor at `used = 1.3`; after repair `used ≤ 1`); `ir_repair_widens_the_worst_branch` (an `IrDrop` budget broken by 20 % on a 3-terminal supply → after repair the branch to the farthest terminal has `k = 2` and the budget holds).
- Acceptance: step 3: the four `performance.rs` tests; metadata prints each row's `used` split into C, R, pair and coupling parts. Whole item: with PERF's OTA testbench, every `PerformanceBudget` row has `used ≤ limit` and the post-layout spec miss is not worse than the M1 baseline.
- Risks / notes: sensitivities are linear around the schematic point; PERF owns re-deriving them at the layout point (SURV-26, NOTES-13). Without PERF inputs the router behaves as today. `path_resistance_ohm` is O(k²) in a net's shapes (stack.rs:91–93 ponytail); it runs once per row per evaluation, not per search node.

### RTE-22 Discrete performance-driven wire widths (1×/2×/3×)

- Priority: P1. Effort: M. Depends on: RTE-12, RTE-21; PERF-11/12 (rows); PERF-28 consumes `width_candidates`.
- Why: AT-17; SURV-27 (7-nm OTA: diff-pair source nets 1× → 3× raise UGF 688.5 → 972.0 MHz, todaes22.txt:43–60); SURV-28 (designer R/RC annotation loses: TS-OTA2 FOM 0.92 vs 0.74, todaes22.txt:895–920); SURV-29/31/32 (per-net discrete widths, parallel routes, symmetric nets equal, ≈ ⅓ of nets at each width, todaes22.txt:147–157, 212–215, 628–645, 807–812); SURV-33 (relax, round, simulate the top η = 10, todaes22.txt:486–498); NOTES-47.
- Current: sensitive signals are held at their EM width, unweighted signals go toward `fat_signal` = 2× (dr:667–679).
- Change:
  1. Candidates `k ∈ {1, 2, 3}` per net (todaes22.txt:628–635).
  2. From the `F_k` pre-route (RTE-21 step 4): `pub struct PreRoute { pub run_nm: Vec<[i64; MAX_LAYERS]> /* per net, per routed layer */, pub terms: [LayerTerms; MAX_LAYERS] }` with `pub struct LayerTerms { pub area_af_um2: f32, pub fringe_af_um: f32, pub sheet_ohm: f32, pub wire_nm: i32, pub step_nm: i32 /* s_l·p0 */, pub wide_nm: i32 /* first wide-metal threshold */ }`. With `L = run_nm/1000` µm: merged (`W_l(k) < wide_nm`): `C_n(k) = Σ_l L·(area·W_l(k)/1000 + 2·fringe)` aF, `R_n(k) = Σ_l sheet·L·1000/W_l(k)` Ω; separate wires (RTE-12 step 1): `C_n(k) = Σ_l k·L·(area·wire/1000 + 2·fringe)`, `R_n(k) = Σ_l sheet·L·1000/(k·wire)` (Stack terms, stack.rs:55–81).
  3. Linear model per `PerformanceBudget` row `j` (PERF-12; weights already `∂f/∂x / headroom`, adverse only): `used_j(k) = used_j(1) + Σ_n [wC_{j,n}·(C_n(k_n) − C_n(1)) + wR_{j,n}·(R_n(k_n) − R_n(1))]`, with `wC` from `row.nets/weights` and `wR` from `row.r_nets/r_weights` (RTE-21 step 3's struct).
  4. Coordinate descent from all-1: for each net in decreasing `F_k`, pick the `k_n` that minimises `max_j used_j`; repeat until no change, at most 5 sweeps; tie matched pairs to the larger `k`.
  5. Output `DetailedCfg::width_mult: Vec<u8>`; the branch `k = max(k_perf, k_EM, k_floor)` (RTE-14).
  6. PERF hook: `pub fn width_candidates(pre: &PreRoute, rows: &[analog::routing::PerformanceBudget], eta: usize) -> Vec<Vec<u8>>` returns the `eta` best vectors by the linear model (single- and two-net changes around the optimum); PERF simulates them and passes the winner back as `width_mult`.
  7. A net that fails to route at its `k` retries at `k − 1` and counts the fallback in `RouteStats::width_fallbacks` (99 % routable in TODAES, todaes22.txt:628–635).
- Tests: `dr` (pure functions over a hand-built `PreRoute` of one 100 µm met1 run per net, sky130 terms): `an_r_sensitive_source_net_widens` (`wR = 0.01 /Ω`, `wC = 0` → `k = 3`); `a_c_sensitive_high_impedance_net_stays_narrow` (`wC = 1e-3 /aF`, `wR = 0` → `k = 1`); `matched_partners_share_width` (two nets of one `PairSpec`, only one R-sensitive → both `k = 3`).
- Acceptance (PERF testbenches on `ota`, `tt_ota`): post-layout FOM ≥ max(uniform 1×, uniform 2×) and no spec worse than uniform 1×.
- Risks / notes: the linear model can mis-rank widths far from the schematic point; PERF's top-η simulation (SURV-33) is the judge. Widening increases C on high-impedance nets, which the S_C term prices.

### RTE-23 Repair judged on the geometry that ships

- Priority: P1. Effort: M. Depends on: RTE-06, RTE-12.
- Why: AT-23, AT-24; NOTES-07 (evidence refers to the final geometry).
- Current: `probe` = trees without access or fill (dr:555); access, short breaking, via arrays, stubs and fill all run after repair (dr:607–877) and nothing re-checks them; the repair key rebuilds everything and rescores every rule per trial (dr:1420–1425).
- Change:
  1. `probe(hot)` = `build_routes` + `add_pin_access` (landing is fixed before PathFinder, so access is a function of landing and trees).
  2. Incremental key: cache each batch's `(violations, residual)`; after a trial, recompute only batches whose `touched` ids intersect the rerouted nets. DC batches (Electromigration, IrDrop) are excluded from the key; RTE-14/21 own them.
  3. Fill for a matched pair: fillers are computed for `a` and mirrored to `b`; a mirrored filler that comes within `S` of `b`'s foreign metal drops both.
  4. After fill: rescore the routing batches; any hard batch newly violated → one more repair round (at most 2) on the nets it names, then redraw.
- Tests: `dr`: `the_report_is_the_measurement_of_the_drawn_routes` (three synthetic cases with a `CouplingBudget`, a `Shield` and a `Differential`: re-evaluating every batch on the returned `Routes` gives exactly the report's `(violations, residual)`); `fill_on_a_pair_is_mirrored` (a mirrored pair whose `a` side needs one notch filler → `b` gets its mirror image, or neither gets one). The antenna case is RTE-06's test.
- Acceptance: on every fixture, the routing V/Θ that dr reports equals what `metadata::build` measures on the final routes.
- Risks / notes: dropping mirrored fillers can leave a sliver DRC on one side; the fill is then re-run with that pair frozen, and any leftover sliver appears as a DRC finding rather than a silent asymmetry.

### RTE-24 Pin access: priority landing order, land once per strapped piece, ampacity filter

- Priority: P1. Effort: M. Depends on: RTE-14, RTE-15; REL-01 (pin currents), REL-03 (`net_flow`).
- Why: AT-04 (first-come landing), AT-06, AT-29; EM-19 (terminal regions below the wire's current are not connection points, lienig_em.txt:4687–4699); audit-05 §2.2 edge case "multi-finger pins are all routed by the router, with no notion that the cell may already strap them".
- Current: pins land in first-seen order (dr:196–204, 360–431); every pin of every terminal is landed even when the cell straps them together.
- Change:
  1. Landing order: matched pairs (RTE-15) → nets in `order_by_priority` order → within a net, pins by decreasing access current.
  2. Strapped pieces: per placed macro, `Stack::connected` pieces (as `cell_metal`, dr:1726–1750); pins of one net inside one piece are landed once (the pin nearest the net's pin centroid) when the net's known total current ≤ the piece's access ampacity (`cuts that fit × I_cut` of the access cut; li has no EM limit on sky130, pdks/sky130.json:99); unknown current → land every pin.
  3. A landing candidate is rejected when the access cut array it needs (RTE-14 step 6) does not fit in `pin ∩ pad`.
  4. Coordinated one-line edit in REL-03's `Electromigration::check` (em.rs): pass `r.shapes(net) ∪ r.cell_metal(net)` to `net_flow`, so the terminals reached only through the cell strap stay reached (otherwise `net_flow` returns `None` and EM reads unknown); cell metal on a layer without a deck limit (li on sky130) carries current but is not checked.
- Tests: `dr`: `a_strapped_terminal_lands_once` (four `d0:S` pins joined by one li strap, total current 0 → one access jog); `a_high_current_terminal_lands_every_region` (the same cell with 2 000 µA total and mcon at 360 µA/cut, the chosen pin fitting 2 cuts = 720 µA < 2 000 → all four pins landed).
- Acceptance: on `ota`, fewer access jogs (count printed) with DRC 0 and no `em access` V.
- Risks / notes: landing once per strapped piece moves current into the cell strap; the strap's EM is CELL's (EM-19 flags it), so collapsing is limited to nets whose current is known and within the access cut ampacity.

### RTE-25 Lattice and congestion export for placement

- Priority: P1. Effort: S. Depends on: RTE-07, RTE-10, RTE-13.
- Why: AP-17; NOTES-41; LAMP-31/47 (per-edge routing space; the 10–15 % area gap LAYLA attributes to separated P&R, lampaert.txt:3944–4030, 7094–7114); H15-09 (choke points, hastings.txt:47241–47261); AF-05.
- Current: gr's report is discarded (lib.rs:617–618); placement receives no routing feedback (lib.rs:575).
- Change:
  1. `RouteStats::congestion`: after PathFinder, the frame split into 16×16 regions; per region `demand = Σ_nodes (usage + hist/p_fac) / on-track node count` over the region's nodes on every routed layer, returned as `(Rect absolute, f32)`. This replaces the `gr::OverflowMap` that PLC-15 step 2 names (the coarse router is retired by RTE-07); plan-04 §0.2 already lists `RouteStats::congestion` as its input.
  2. `pub fn lattice_spec(cfg: &DetailedCfg) -> LatticeSpec` with `pub struct LatticeSpec { pub p0: i32, pub strides: Vec<u32>, pub origin_multiple: i32 /* p0·lcm(strides) */ }` (PLC-28 snaps axes with it, RTE-15). sky130 after RTE-10/11: `{ p0: 420, strides: [1, 1, 2, 2], origin_multiple: 840 }`.
- Tests: `dr`: `congestion_marks_the_crowded_region` (10 two-pin nets forced through one region-wide channel, one net elsewhere → the channel's region has the largest demand); `congestion_without_history_counts_usage` (history zeroed: `Σ_regions demand·on_track_nodes = Σ usage`).
- Acceptance: PLC's consumer tests (theirs); RTE: the two tests above.
- Risks / notes: the map reflects this epoch's placement; PLC must smooth it across epochs (its decision).

### RTE-26 DRC-clean detailed routing on every shipped deck

Field report: FR-4 (routing half). On m0 the CLI run of pwm_driver (34 FETs) ends with `drc/m1.2.notch:met1` 25, `drc/m2.2.notch:met2` 115, `drc/m1.7:met1` 169, `drc/m2.7:met2` 342 (`m1.7`/`m2.7` = `hole_area` ≥ 0.14 µm², sky130.deck:378–379), all on routed layers; add notch and enclosed-hole cases to the sky130 deck cases.

- Priority: P1. Effort: M. Depends on: RTE-03, RTE-10, RTE-11, RTE-12 (cases a, b, c, e: milestone M1, sky130 first); RTE-15 (case d) and the other three decks: milestone M3.
- Why: AT-27; AV-15; audit-05 §2.2 (all dr unit tests use synthetic layers and a 430 nm pitch, dr:2141–2147).
- Current: routing-layer DRC is gated only on 7 small sky130 fixtures (signoff_fixtures.rs:15–41); `ota_cross_pdk` excludes routing layers as "dr debt" (ota_cross_pdk.rs:196–224).
- Change:
  1. `elaborate` exposes `pub fn router_for(pdk: &Pdk) -> Result<(dr::DetailedRoute, RoutingStack), String>` (re-exported by `library`; `RoutingStack` from RTE-11).
  2. New `frontend/library/tests/dr_deck_drc.rs`, for each of sky130, gf180mcu, ihp_sg13g2, generic_finfet: (a) a 20 µm channel crossed by 8 nets; (b) 16 two-pin nets with forced layer changes at adjacent tracks; (c) `k = 2` and `k = 3` bundles with corners; (d) a mirrored pair about a PLC-28 contract axis; (e) access to pins of the deck's minimum width at the deck's minimum pitch on the pin layer (170×170 li on sky130). Each case runs `verify::drc(shapes, &[], pdk)` on the routes plus the pin shapes and asserts zero findings whose rule belongs to a routed layer, a routing cut or the pin-access cut.
  3. `ota_cross_pdk.rs`: the `OWNED` filter is replaced by `drc.is_empty()` once (1) is green.
- Tests: the five cases per deck in `frontend/library/tests/dr_deck_drc.rs` (`channel_crossing_is_drc_clean`, `adjacent_layer_changes_are_drc_clean`, `bundles_are_drc_clean`, `mirrored_pair_is_drc_clean`, `min_pitch_pin_access_is_drc_clean`), each parameterised over the four decks.
- Acceptance: M1: cases a, b, c, e green on sky130. M3 (T1): 0 routing-layer DRC on 4 decks × 5 cases, all `signoff_fixtures` (including the slow set) and `ota_cross_pdk`.
- Risks / notes: generic_finfet (ASAP7) may need deck rules the lattice does not model (e.g. discrete widths); failures there become deck-specific items rather than weakened assertions.

### RTE-27 Redundant vias and current-aware via-array geometry

- Priority: P2. Effort: S. Depends on: RTE-12, RTE-13 (`ShapeIndex`); REL-03 (cut-group check), REL-12 (front-row count, which replaces this item's former g(H)/current-spreading step, C8).
- Why: AT-18, AT-19; H15-19 (fabs demand via pairs against stress voiding, hastings.txt:48148–48153); H15-26 (hastings.txt:48411–48414); H15-39 (arrays at a 90° corner load the inside via; put arrays in straight segments, hastings.txt:48818–48824); H05-14 (only the faces toward the current conduct, hastings.txt:12452–12485); EM-36/37 (lienig_em.txt:6116–6182, 6234–6279); EM-14 (g(H) > 1 for inhomogeneous flow, lienig_em.txt:4626–4660).
- Current: minimum-width trunks always get one cut (dr:762–780); arrays assume equal sharing (dr:721–722).
- Change:
  1. Post-pass after via arrays: every single-cut via of a net that is not one half of a matched pair gets a second cut along the lower layer's wire direction, at the deck cut pitch (cut size + cut spacing), when both extended pads (`pad_along + cut pitch`) clear foreign metal by `S_l` (`ShapeIndex`); matched pairs double both vias or neither.
  2. `RouteStats::single_cut_vias` counts what remains.
- Tests: `dr`: `every_free_via_is_doubled` (3 two-pin nets with one layer change each on an empty lattice → 3 vias × 2 cuts, `single_cut_vias == 0`); `pair_vias_double_symmetrically` (a mirrored pair with one via each, an obstacle blocking the second cut on `a`'s side only → neither via doubled).
- Acceptance: single-cut vias ≤ 10 % of all vias on `ota` (tuning target), DRC 0.
- Risks / notes: a second cut enlarges the pad along the wire and can block a neighbour's via halo; the post-pass never moves other nets, it only skips vias that do not fit.

### RTE-28 Wrong-way jogs at a price

- Priority: P2. Effort: S. Depends on: RTE-10.
- Why: AT-02 (a one-track jog on the wrong direction costs two vias); Lienig §4.6.5 (arrays in line with the current need a wire routed "around the corner"; "This can only be done, however, if the routing layout is not forced to follow preferred directions", lienig_em.txt:6355–6359).
- Current: no wrong-way edges (gr:630–638).
- Change: `TrackGrid::neighbors` adds the two non-preferred steps on stride-1 layers with base cost `WRONG_WAY_COST = 2.5` (tuning default, below the `2·VIA_COST = 8` of a via detour); a wrong-way step is legal only when both nodes' footprints are free (a node pitch `p0 ≥ wire + S` keeps it DRC-clean on stride-1 layers, derived from the RTE-10 table).
- Tests: `gr`: `a_one_track_jog_stays_on_its_layer`.
- Acceptance: via count on `ota` reduced (printed), DRC 0.
- Risks / notes: wrong-way steps on stride-2 layers are not offered (the adjacent row is off-track).

### RTE-29 Crossover connector and further symmetry transforms

- Priority: P2. Effort: M. Depends on: RTE-15; PLC (horizontal axes, if emitted).
- Why: AT-21; BAL2-08 (a symmetric "connector" when paired terminals sit on opposite sides and the paths must cross, balasa_graeb_survey.txt:8901–8909).
- Current: only translation and vertical-axis mirror (dr:1637–1662).
- Change: `SymMap` gains `MirrorY { axis2 }` and `Rot180 { cx2, cy2 }`, used only when PLC emits such axes; for a pair whose mapped terminals require crossing the axis (cross-coupled latches), the joint search is split at the axis track: `a` and `b` each change layer once on either side of the axis at mirrored nodes (`k/2 − 1` and `k/2 + 1`), so both nets see one via and equal length on each layer.
- Tests: `dr`: `a_cross_coupled_pair_routes_with_equal_signatures`.
- Acceptance: on a latch fixture (PERF adds one), `Differential` mismatch 0.0 %.
- Risks / notes: the crossing adds one via per net; it is used only when no uncrossed mirror route exists.

### RTE-30 Spend leftover margin on critical-area yield

- Priority: P2. Effort: M. Depends on: RTE-21, RTE-23.
- Why: AT-35; LAMP-43/44 (manufacturability phase; short `λ = L·x0²·D/(2s)`, open `λ = L·x0²·D/(2w)`, Eqs 5.27–5.31, lampaert.txt:6351–6616); BAL2-29; NOTES-56.
- Current: no critical-area term.
- Change: after a result with V = 0 and every budget within limit, compute the relative short exposure `λ_rel = Σ_pairs L_run·x0²/(2s)` with `D = 1` and `x0 = min spacing` (defect density and `x0` are not given by the sources for these decks); reroute nets in decreasing `λ_rel` with an added cost `μ·Δλ_rel` (μ tuning default 1.0), rejecting any node that pushes a `PerformanceBudget` above 1 or raises any budget residual; keep a reroute only if `Σλ_rel` drops.
- Tests: `dr`: `spreading_never_raises_a_budget` (a net with slack is moved one track away from a neighbour; every `PerformanceBudget.used` unchanged or lower, `Σλ_rel` lower).
- Acceptance: `Σλ_rel` on `ota` reduced by ≥ 10 % with V and Θ unchanged.
- Risks / notes: without foundry defect data the metric is relative; it can rank layouts but not predict yield.

### RTE-31 Hard-bound pruning of partial paths (conditional)

- Priority: P2. Effort: M. Depends on: RTE-21. Only if `ParasiticBudget` residuals remain after M3.
- Why: BAL2-20 (ANAGRAM III prunes parasitically non-viable partial paths, balasa_graeb_survey.txt:9464–9478).
- Change: the search carries accumulated ground C per label and discards a label whose C exceeds the net's remaining budget (budget − already-routed share); single resource, one label per node.
- Tests: `gr`: `a_label_over_budget_is_pruned` (two paths, the cheaper one exceeds the C budget → the search returns the other).
- Acceptance: the remaining `ParasiticBudget` residuals reach 0 on the fixtures where they persisted.
- Risks / notes: one label per node makes the pruning a heuristic (not an exact resource-constrained shortest path); acceptable for a conditional P2 item.

### RTE-32 Router-side capacitance on the per-layer union; `ParasiticBudget` cost in its own unit; one touch predicate (added in review)

Status: done in M0 (`e5abfc8`); one `Rect::touches` in `pnr_core::geom`.

- Priority: P0. Effort: S. Depends on: none. Lands before RTE-05 (ground-C term), RTE-20 (lead C) and RTE-21 step 3, which all read `Stack::ground_af`.
- Why: AR-23 ("ground C double-counts overlapping shapes … parasitic cost is length² even when the budget is C", audit-02, parasitic.rs:30–36, 46–49, stack.rs:52–64) and AR-20 (four copies of the rect-touch predicate, stack.rs:106, 185, 374–378, routes.rs:61, plus dr:1731) had no owner in any plan. Signoff extraction computes a polygon's ground C as `area × area_coeff + perimeter × fringe_coeff` over the merged polygon (GPurify crates/extract/src/analytical.rs:55–67, 348–407 at the pinned 8df8c09); the router sums rect by rect and counts fringe on the two long sides only (stack.rs:55–64, "a via pad on a wire counts twice"). plan-07 V11 reports unmerged ground C at 1.13–1.73× the unioned value on the fixtures (audit-06 G.4(b)); PERF-16 fixes what enters GPurify, not the router's own estimate. Benefit: RTE-05's pair C, RTE-20's per-unit lead C and RTE-21's `used` measure what signoff extracts.
- Change:
  1. `kernel/core/src/geom.rs`: `impl Rect { pub fn touches(&self, o: &Rect) -> bool }` (closed intervals: edge contact counts, the semantics of every current copy). Replace the copies at stack.rs:106, 185, the free fn `touches` (stack.rs:376–378) and routes.rs:61, and dr:1731 (RTE file). mosfet.rs:700 is CELL's and is left.
  2. `Stack::ground_af(&self, shapes) -> f32`: group shapes by stack layer; per layer, the union's area `A` (µm²) and perimeter `P` (µm) by coordinate compression: sorted distinct xs and ys of the layer's rect edges, a 2-D difference array marks covered cells (O(n + X·Y)); `A` = Σ covered cell areas; `P` = Σ lengths of cell edges between a covered and an uncovered (or outside) cell. `C = Σ_layers (area_af_um2·A + fringe_af_um·P)`; cut layers (area and fringe 0 in every deck, e.g. sky130.deck:587, 589) add 0 as today. Delete the ponytail note at stack.rs:52–53.
  3. `ParasiticBudget::cost` (parasitic.rs:46–49) = `(spent / budget.max(1.0))²` from `spent()` (C when `stack` and `max_c_af > 0`, else length), replacing `len²·1e-6`; its ponytail note (parasitic.rs:43–45) goes. With `gr`'s score retired by RTE-07, the only reader is dr's `Report.cost` (dr:1858), a tie-break scalar.
- Tests (`kernel/analog/src/routing/stack.rs`, helper values area 25 aF/µm², fringe 40 aF/µm):
  - `ground_c_is_area_plus_fringe_per_layer` (existing, stack.rs:419–423): expected value changes from 925 to 965 aF (10 × 0.5 µm: 25·5 + 40·21); the unknown-layer shape still adds 0.
  - `overlapping_shapes_count_once`: [0, 10] × [0, 0.5] µm and [5, 15] × [0, 0.5] µm on m1 → union 15 × 0.5 → 25·7.5 + 40·31 = 1 427.5 ± 1e-2 aF (the summed value would be 1 930).
  - `a_pad_inside_a_wire_adds_nothing`: 10 × 0.5 µm wire plus a 0.5 × 0.5 µm pad inside it → 965 aF.
  - `parasitic.rs`: `cost_is_the_squared_budget_fraction` (length mode, 500 µm routed of a 1 mm budget → cost 0.25).
  - `kernel/core/src/geom.rs`: `edge_contact_touches_and_a_gap_does_not` ((0,0,10,10) vs (10,0,5,5) → true; vs (11,0,5,5) → false).
- Acceptance: the tests above; `cargo test -p analog` green with only the one expected-value update; `bench local` DRC/LVS verdicts unchanged (the change only alters measurements).
- Risks / notes: the fringe convention changes (full perimeter instead of long sides), so every stack-mode C rises by `2·fringe·w` per isolated wire (≈ 21 aF for a 0.26 µm met1 wire, 40.57 aF/µm, sky130.deck:590) and falls wherever shapes overlap; budgets calibrated against the old sum may shift slightly. It matches signoff, which is the point.

---

## 4. Milestones

| Milestone | Items | Exit criteria |
|---|---|---|
| M0 — Honest router (P0) (master M0 took RTE-32, 02, 09, 03, 06: 32, 02 and 09 merged; 03 is on m0 outside review; 06 was rejected and is carried to master M1) | RTE-32 → 02 → 09 → 03 → 07 → 08; then 05, 06 once REL M1 (REL-01, 02, 03) has landed. RTE-01 and RTE-04 are cut (C1, C2) | `signoff_fixtures` green including dac4 at `feedback_iters = 1` (T11; needs REL-02 + RTE-06); bench 10/10 DRC 0 and LVS MATCH; no `drawn short`/`unresolved congestion` V and no `routing overuse` Θ on any bench circuit (T2); gr's coarse route gone (`GlobalRoute` absent from the workspace); `grep -rn 'kind()' backend/gr backend/dr frontend/library/src/elaborate.rs` empty; EM/IR per shape and per terminal are REL-03/04's measurements and are only reported here (T7, T8 not yet 0) |
| M1 — Routing substrate | RTE-10 → 11 → 12 → 13, 25, 26 (sky130 cases a, b, c, e) | sky130 routes met1–met4 at p0 = 420 with strides [1, 1, 2, 2] (T3, T4; `sky130_layer_specs_match_the_hand_derivation`, `sky130_routes_met1_to_met4`); fattening code deleted; dr p50 per call on `ota` ≤ ⅓ of the M0 baseline (T13); `dr_deck_drc` sky130 cases 0 findings; `lattice_spec` = `{420, [1, 1, 2, 2], 840}` available to PLC-28 (RTE-25 moved here from M2 in review: PLC-28 needs `LatticeSpec` before RTE-15 can be exact) |
| M2 — Analog-quality routing | RTE-14, 16, 18 → 15 (after PLC-28), 17, 19, 20, 24 | EM V = 0 and IR Θ = 0 on circuits with an operating point (T7, T8); every contract-compliant matched pair exact (T5); 0 µm² metal over MOD/EXC gates (T6); star/Kelvin/ΔR checks pass (T9); every `CrosstalkExclusion` satisfied and the coupling estimate within 20 % of PEX on `ota` (T10); dac4 `PlateRatio` residual 0 |
| M3 — Performance-driven routing | RTE-21 (step 3 may land any time after RTE-32), 22, 23, 26 (all decks, case d) | PERF OTA testbenches: all `PerformanceBudget` rows `used ≤ limit`, FOM ≥ max(uniform 1×, 2×) (T12); dr's V/Θ equal the metadata measurement on final routes (RTE-23); T1: 0 routing-layer DRC on 4 decks × 5 cases and `ota_cross_pdk` gated on all layers |
| M4 — Polish | RTE-27, 28, 29, 30, 31 | single-cut vias ≤ 10 %; wrong-way jogs cut via count; latch pairs exact; `Σλ_rel` −10 %; conditional pruning only if needed |

---

## 5. Open questions

1. **Match class before MAT delivers it.** Proposed default: MOD for every matched cell, so gates get hard keep-outs (Hastings rule 17). If routability suffers on a fixture, drop that fixture's cells to MIN and report it.
2. **sky130 met5.** The stride cap (MAX_STRIDE = 4) drops met5 (derived stride 8). Proposed default: exclude it; revisit only if a supply net cannot meet IR on met1–met4, then add met5 as pre-drawn supply straps (a separate, deck-driven feature).
3. **History decay factor.** Resolved in review: FLOW-08 step 4 resets `Negotiation` before every cold epoch and keeps it across warm epochs, with no decay factor (plan-08). RTE-08 only changes the keys and the per-iteration factor.
4. **Differential tolerance.** Proposed default 5 % (today's value) until MAT supplies an offset-derived allowance; exact mirrors make it moot for contract-compliant pairs.
5. **Unknown currents.** When the operating point is missing, EM/IR are `unknown` (metadata) rather than V. Proposed default: keep that, and let FLOW's certificate refuse certification while any mandatory EM rule is unknown (NOTES-02).
6. **Merged vs parallel wires for k > 1.** Proposed default: merged below the layer's first wide-metal threshold, separate wires at or above it and on decks whose layers declare discrete widths (FinFET).
7. **PLC axis snapping granularity.** The contract puts axes on base-track centrelines (`p0/2 + m·p0`, 420 nm steps on sky130). If PLC cannot hold it for a group, that group's pairs use the fallback (guided route, honest metric).
8. **Noisy/sensitive crossings.** Proposed default: priced (RTE-18 crossing C); hard `no_cross` only for pairs EXT flags because `C·R·dV/dt` of one crossing would exceed the victim's allowance (H15-29).
9. **Pin-access split criterion.** Resolved in review: the pinned decks give sky130 li_c 12.8 vs met1 0.125 Ω/□ (sky130.deck:588, 590; 102×, split), gf180 metal1_con 0.09 vs metal2_con 0.09 (gf180mcu.deck:575, 577; 1×), ihp metal1 0.11 vs metal2 0.088 (ihp_sg13g2.deck:684, 686; 1.25×), generic_finfet M1 1.2675 vs M2 0.534829 (generic_finfet.deck:343, 345; 2.4×). The 10× threshold splits only sky130, as RTE-11 step 1 states.

---

## Verification log

Fact-check run 2026-09-28 against the working tree (HEAD `dad330c` + uncommitted changes), the pinned GPurify checkout `8df8c09`, the study docs in `docs/plans/`, and the reftext extracts. No cargo command was run.

**References checked: 573.**
- 291 code `file:line` citations (223 distinct), including 22 `sky130.deck` line citations and the JSON sidecar lines: every cited line opened and the stated behaviour compared with the code.
- 82 reftext line-range citations (72 distinct) in hastings, lampaert, lienig_em, balasa_graeb_survey, todaes22, perf_driven_survey, cc_dac_constructive, cc_review: every quoted phrase compared verbatim; Hastings Eq 15.19 read from the PDF (p. 812) because the text extract drops the symbols.
- 200 study-doc IDs (ranges such as `H08-20..29`, `EM-19/20/21` expanded): all 200 exist in an `audit-*.md` or `ref-*.md` file; each "Why" entry was read against the claim it supports.
- Derived numbers re-computed: the whole RTE-10 sky130 table (pads, wires, pitches, strides, halos, p0 = 420, +64 % tracks), met5 stride 8, the 460 nm pitch today, EM widths/cut counts (357, 714 nm; 2 and 3 cuts; 728 µA), IR 4.808 mV, coupling 857.1 aF, the 2×2 via array, crossing C, the 580/300 nm strip, the star-node 0.3 mV, the proposed new names (no collision with existing items; `Separation` exists only as a test-local struct in backend/dp/src/tests.rs:238).

**Corrections (before → after).**
1. §1.5 and RTE-05: Differential stack mode "differential.rs:79–90" / "differential.rs:88" → differential.rs:55–65 (runs' R at :63). Lines 79–90 are the geometric signature.
2. §0.2 and RTE-05 step 6: `Differential::extract` "differential.rs:141–158" (test code) → differential.rs:116–133.
3. T1: "excluded as dr debt on gf180/ihp (ota_cross_pdk.rs:196–224)" → `ota_cross_pdk` runs only sky130 and generic_finfet (ota_cross_pdk.rs:338–358); nothing gates gf180/ihp routing DRC.
4. RTE-11 acceptance: "`ota_cross_pdk` on gf180/ihp reports …" → it runs sky130/generic_finfet only; gf180/ihp are first exercised by RTE-26.
5. RTE-18 crossing capacitance: `ε0·k_{l+1}/gap` with "the same term GPurify extracts" → `ε0·k_l/gap` with the lower metal's dielectric, as GPurify does (crates/extract/src/analytical.rs:237–242). Values 137.7 / 86.4 / 90.8 aF/µm² → 147.6 / 88.5 / 93.1 aF/µm²; test `a_crossing_couples_by_overlap_area` 10.0 ± 0.1 aF → 10.7 ± 0.1 aF.
6. RTE-07 step 4: `cellgen::price` "cellgen.rs:354–377", "3-line edit" → cellgen.rs:354–375 plus `seed_assignment` (cellgen.rs:327–346), which also uses `gr::GlobalCfg` (:332, :357).
7. RTE-07 step 5: "lib.rs:614–631" → lib.rs:614–652 (the second dr call at :646 also takes the new signature).
8. RTE-07 tests: added gr's `negotiation_persists_across_calls` (gr:1197), which drives `GlobalRoute` and would not compile after the deletion.
9. RTE-07 risks: "none of the deleted code has another caller (`grep price_group` finds only cellgen.rs:361)" → the outside callers are cellgen.rs:360/332/357, lib.rs:618, elaborate.rs:176, dr:21/261 (`GcellGrid`) and dr:506/584 (`gr::Tier`).
10. §1.3: "cellgen.rs:361, 376" → cellgen.rs:360, 374 (plus the `GlobalCfg` at :332).
11. RTE-09 current: added the `rsplit("::")` form at dr:1444 and the sixth dispatch site elaborate.rs:342 (`antenna_diodes`); `net_current_ua` clarified (the `DetailedCfg` field is gone, the function oppoint.rs:89 remains; CRATES.md:58, 63, 97).
12. RTE-09 acceptance and M0 exit: `grep -n 'ends_with("' backend/gr backend/dr` returns nothing → `grep -rn 'kind()' backend/gr backend/dr frontend/library/src/elaborate.rs`. The old grep would always fail on dr:1737 (pin name `":G"`) and would miss dr:1444.
13. RTE-09 change: noted that CommonNodes and PerformanceBudget implement `RuleBatch` directly (common_node.rs:61, performance.rs:36) and so override `repair_kind()`.
14. RTE-14 current: "`order_by_priority` puts supplies in the free tier (gr:250–271)" → in the flow every ≥ 2-terminal net carries a hard `Electromigration` (lib.rs:1016–1027), so supplies sit in the hard tier with every other routed net and get no priority.
15. RTE-14 current and §1.4: access-cut cite "dr:748–749" → dr:743–745 (the pin-access cut is not in `cuts` and is pushed unchanged) and dr:748–749 (a stack-layer access cut gets `cuts(0)` = 1).
16. RTE-03 acceptance: "the `route_overuse` statistic is 0" → no `unresolved congestion` V / `routing overuse` Θ entry; `RunStats::route_overuse` sums every routing Θ margin (lib.rs:691–695), so it is not an overuse measure.
17. RTE-01 step 4: "member 0 of injected macros" → member 0 of any cell (cellgen.rs:933–947).
18. RTE-06 step 3: rank 0 "below every stack layer" → the lowest stack layer's rank, present at every stage (stack.rs:295).
19. RTE-17 current: `CommonNode { a, b, feeds, max_delta_ohm }` → `{ net, a, b, feeds, max_delta_ohm }`.
20. §1.2: `fat_supply` "just under the widest wide-metal threshold" → 2·grid under the smallest, over routed layers, of each layer's largest threshold (elaborate.rs:429–435).
21. RTE-12 test: deck cite "sky130.deck:385–388" → 385–390 (met2's m2.4/m2.5 are used for the 700 nm axis); the enclosure-pair assumption stated.
22. RTE-02 current: the ground `Shield` is emitted only when the design has a clock (extract.rs:90–92).
23. Quotes made verbatim (paraphrases had been placed in quotation marks):
    - T5 "exact route matching" → "Exact routing": "the pair of wire lengths be the same for every layer" (perf_driven_survey L86–89).
    - T7, RTE-04 "EM at every point" / "EM must hold at every point" → "a lead must meet electromigration rules at every point along its length" (hastings.txt:48741).
    - RTE-06 "under the node's poly" → "the gate oxide area beneath the poly regions belonging to the node" (hastings.txt:13154–13155).
    - RTE-14 "route wide leads first" → "begin with any signals that require wider-than-minimum leads" (hastings.txt:48387).
    - RTE-15 TODAES "two nets under a symmetry constraint always get the same width" → "two nets with a symmetry constraint are always assigned with the same width" (todaes22.txt:215).
    - RTE-17 "Kelvin at the bondpad" → "signals which should Kelvin at a bondpad actually do so" (hastings.txt:48940–48944).
    - RTE-20 CC-28 → "nonoverlapped routing that separates the wires that route the top-plate and bottom-plate" (cc_dac_constructive.txt:792–794); H08-22 → "insert jogs or dead-end branches into its lead until the two capacitances match" (hastings.txt:25595).
    - RTE-28 Lienig "possible only without strict preferred directions" → "This can only be done, however, if the routing layout is not forced to follow preferred directions" (lienig_em.txt:6358–6359).
24. Attribution fixes:
    - RTE-11 cited LAMP-36 for lampaert.txt:5472–5476 → the quote is Lampaert §5.2's "n-layer routing" requirement, outside LAMP-36's range (L5511–5590).
    - RTE-05 SUB-44 "a 5 % coupling asymmetry costs ≈ 40 dB" → a 5 % *substrate*-capacitance mismatch costs ≈ 40 dB at 1 GHz, ≈ 27 dB at 100 MHz (read from Charbon Fig 8.10b per SUB-44); used by analogy only.
    - RTE-02 BAL2-10 "must not count as noise" → the source says the shield must be tied to a DC potential "not to worsen the effects of crosstalk"; not counting it as an aggressor is the plan's inference.
    - RTE-12 BAL2-36 "width-dependent spacing in the search" → the source's conditional wide-metal spacing rule (balasa_graeb_survey.txt:11819–11837); applying it in the search is the plan's step.
    - RTE-20 AV-06 → AV-06 / AV-32 (the dac4 coupling numbers are in audit-06 L310–311 and AV-32, not in the AV-06 row).

**Confirmed without change (selection).** All gr/dr/analog/library line cites in §1.1–§1.5 except those above; the RTE-10 sky130 table and p0 = 420; the 460 nm pitch today; the met5 stride 8; the EM-limit values at sky130.deck:491–492; the IR, coupling, EM and via-count arithmetic; Hastings Eq 15.19 (PDF p. 812); Lampaert eqs 5.1–5.3, 5.10, 5.12–5.14, 5.27–5.31; Lienig eqs 3.5–3.7, 3.25; TODAES UGF 688.5 → 972.0 MHz and FOM 0.92 vs 0.74; the audit's "~270 candidate widths" ((2990 − 290)/10 = 270 on sky130); `bench local` = 10 fixtures (benchmarks/fixtures/*.spice); every proposed new type/fn/const name is free.

**Unresolved.**
- §1.6 item 6 / RTE-06 / T11: the dac4 `erc/ar.met2.1` failure at `feedback_iters = 1` is a runtime result from audit-06 (AV-15, G.3); marked [UNVERIFIED] inline because no test was run.
- Measured values carried from audits (dac4 top–bit couplings, audit-06 L310–311; AT-12 runtime observations) are cited correctly but were not re-measured.
- RTE-12 `a_two_by_two_corner_gets_four_cuts`: the 2 × 2 result depends on which opposite-side pair each metal uses for its 85 nm enclosure and on how far the met1 wire overruns the corner (RTE-10 step 5); the arithmetic is right for the stated assumption, but the final geometry decides it.
- RTE-11 step 1: the 10× sheet-R split criterion for gf180/ihp/generic_finfet was not checked against those decks' `pex` lines (the plan already defers it to implementation).

---

## Cut or deferred items

Items referenced as C1–C8 in §3 had no appendix in the previous draft; it is written here. "Owner" is the item that delivers the work instead.

| # | Item | Status | Owner / reason |
|---|---|---|---|
| C1 | RTE-01 per-pin current split (AT-07) | Cut: duplicate | REL-01 (plan-06): exact per-finger share `pnr_core::macro::pin_shares`, `DetailedCfg::pin_ua: Vec<Vec<(String, Option<i32>)>>`, test `a_multi_finger_terminal_is_not_counted_per_pin`. RTE-14/24 read `table value × pin_shares`. |
| C2 | RTE-04 per-shape EM rule, `Routes::terms`, nodal DC solve, terminal-resolved IR (AR-05, AR-21, AT-06 check side, AF-24) | Cut: duplicate | REL-03 (`Routes::terms`, `routing::current::net_flow`, hard per-shape/per-cut-group `Electromigration` incl. the pin-access layer and cut) and REL-04 (`IrDrop` from `NetFlow::drop_uv`). The CG nodal solve is dropped; no RTE consumer needs node voltages beyond `NetFlow`. RTE-14 does the sizing. |
| C3 | RTE-06 former steps 1–3: gate area per antenna piece, stage-aware diode credit, `Antenna::known` (AR-13) | Cut: duplicate | REL-02 steps 1–3 (`GatePin { at, dev, nm2 }`, `Stack::antenna(shapes, cell, gates, fallback_nm2)`). RTE-06 keeps only repair on the drawn geometry. |
| C4 | PathFinder history lifecycle: reset per cold epoch, snapshot around the antenna-diode second dr run (AT-10 lifecycle, AT-34, AF-06) | Cut: duplicate | FLOW-08 step 4. No decay factor (open question 3 resolved). RTE-08 keeps key and convergence changes. |
| C5 | Stale router text in `docs/CRATES.md` (AT-36: CRATES.md:58, 63, 97) | Cut: duplicate | FLOW-15 step 5 (FLOW owns CRATES.md). |
| C6 | `PerformanceBudget` v2 (stack C, series R, coupling, `limit`) | Split | Row construction and weights: EXT-25 (plan-01). Sensitivity export and `r_weight`/`pair_weight` values: PERF-12 step 2. `limit`: PERF-06 step 2. **Measurement: restored to RTE-21 step 3** in review, because plan-07 and EXT-25 both name RTE as the measurer and no plan owned it. |
| C7 | Separate injector/victim ring returns (H14-10, SUB-30; former RTE-17 step 4) | Deferred | No plan emits a ring-return request (EXT-24 has `StarReq`/`KelvinReq` only; REL-07 draws rings by role). Re-enter when REL or EXT emits `ReturnReq { ring, rail }`; the implementation is then a `CommonNode { star: true }` with the ring pins as one group. |
| C8 | Via-array current-spreading factor g(H) and array orientation at turns (AT-19; EM-14, H15-39) | Cut: duplicate | REL-12 (effective front-row cuts). RTE-14 step 5 consumes it when present. |
| C9 | Learned routing guidance (GeniusRoute-style VAE on manual layouts; audit-05 §3 gap 12, perf_driven_survey.txt:150–158) | Deferred (added in review) | Needs a corpus of hand layouts that Philis does not have and a training pipeline; no measurable benefit can be stated before RTE-15/18/21 exist. Hook if revisited: the `penalty` slice of `NetSearch` (RTE-12). |
| C10 | Template / generator-embedded routes (LAYGEN/BAG; audit-05 §3 gap 14) | Deferred (added in review) | Cell-internal routing is CELL's; a block-level template library has no source-backed benefit over RTE-15's exact joint routing for the fixture set. |
| C11 | Windowed or hierarchical lattice instead of the silent coarsening above `max(die)/1200` (AT-37 fix direction) | Deferred (added in review) | RTE-13 makes the coarsening reported (`RouteStats::coarsened`); no bench circuit reaches it (> ~550 µm die on sky130, audit-05 AT-37, derived). Re-enter when a fixture reports `coarsened = true`. |
| C12 | Plate shields on adjacent layers and bulk shields (AT-22 second half; Hastings Fig 15.28A, hastings.txt:48639–48690) | Deferred | RTE-19 step 4: on a preferred-direction lattice a victim is only crossed, and RTE-18 prices or forbids crossings (`no_cross`). Re-enter if the RTE-18 PEX calibration row shows crossing coupling above a victim's budget with `no_cross` unusable (routability). |
| C13 | Pre-drawn supply straps on sky130 met5 and gridless area routing (audit-05 §5 Q1, AT-15 "pre-route supply trunks") | Deferred | Open question 2: met5 is dropped by `MAX_STRIDE`; revisit only if an `IrDrop` budget cannot be met on met1–met4 after RTE-14/21. Gridless routing is rejected in favour of the lattice (audit-05 §5 Q1: "The lattice is simpler to keep DRC-clean"), with widths as track multiples (RTE-12, TODAES parallel routes). |

## Implementer review log

Review 2026-09-28 by the implementer of this plan, against the working tree, plans 01–08 and the pinned GPurify checkout. No cargo command was run. Items reviewed: 32 (RTE-01 … RTE-32, including the two cut stubs and the new RTE-32).

**Missing work added.**
1. **RTE-32 (new, P0, S):** `Stack::ground_af` on the per-layer union with full-perimeter fringe (GPurify analytical.rs:55–67, 348–407), `ParasiticBudget::cost` as the squared budget fraction, and one `Rect::touches`. AR-23 and AR-20 were not owned by any plan; RTE-05, RTE-20 and RTE-21 read `ground_af`. Exact union algorithm, the one expected-value change (925 → 965 aF) and numeric tests are given.
2. **RTE-21 step 3 (restored):** the `PerformanceBudget` measurement (ground C, series R, pair ΔC, coupling, `limit`, `unknown`) had no owner. The previous draft sent it to PERF-12. PERF-12 sends it back to "RTE-21 step 3" (plan-07 §0.4, PERF-12 header and Cut table), and EXT-25 step 3 waits for RTE to measure the new fields. The struct now unions the EXT-25 fields (`r_nets/r_weights`, `diff_pairs/diff_weights`) with PERF's (`coupling`, `limit`) and adds `stack`. Four numeric tests were added. Steps were renumbered so budget repair is step 5, as plan-07 cites it.
3. Cut/deferred appendix: C1–C8 were referenced but the appendix did not exist. It is now written, with C9–C13 added for audit-05 gaps the plan dropped without saying why (learned guidance, templates, windowed lattice, plate shields, met5 straps and gridless routing).

**Corrections for exactness.**
4. §0.2 PERF row: the `PerformanceBudget` shape (`series`, `stack`, …) matched no plan. It now uses EXT-25/PERF-06/PERF-12's actual fields and states that PERF-12 step 2 fills `r_weight`/`pair_weight`. RTE-21 step 1 no longer rebuilds them from rows (that duplicated PERF-12 step 2). RTE-22's `wR` source changed from `row.series` to `row.r_nets/r_weights`, and its pre-route reference changed to RTE-21 step 4. §0.1, §0.3 and §0.4 ownership text were aligned.
5. RTE-03: the exact `space(..)` call, ownership taken from `cell_metal` via a `(LayerId, Rect) → net` map, the no-stack behaviour, and the existing V label `drawn short nets {a}/{b}` (dr:1850). The cell-metal short test became a direct `cross_net_shorts` call with coordinates. A resolver test was added.
6. RTE-05: `Routes::terminals(net)` returns `&[Terminal]` (use `.at`). Coupling now goes through one new function, `coupling::net_pair_af(stack, a, b, screens)`, shared by Differential, CouplingBudget, PlateRatio and PerformanceBudget. Added the test `an_unrouted_side_is_unknown` and the test-stack values.
7. RTE-06 test: the gate area enters through REL-02's `cfg.gate_nm2` table on a `d0:G` pin, and the stack is required because `cell_metal` collects gates only with one (dr:1728).
8. RTE-07: `pub use pnr_core::place_macros` (gr:19, nine callers) and `KEEPOUT_COST` were missing from the keep list, and deleting gr wholesale would have broken the library. Added `global.debug_check` (lib.rs:619), elaborate.rs:176–177, dr's five test call sites, and the geometry.rs:87 message.
9. RTE-08: the vague "cross pairwise" convergence test now has exact pin coordinates. (The old instance was not congested: horizontal and vertical nets sat on different layers.)
10. RTE-10: `cut_enclosure_pair` must filter on `[metal, cut]`. `max_enclosure_of` keys on the cut only (pdk.rs:370–391), so it returns the larger of the enclosures below and above.
11. RTE-13: A* is specified through `RGraph::lower_bound` (default 0) and the existing bit-ordered heap key.
12. RTE-14 step 7: within-tier order falls back to `net_weight` until RTE-21 step 4 lands, because RTE-14 (M2) must not need RTE-21 (M3). Added one ordering test.
13. RTE-17 step 4: conversion `max_delta_ohm = min(MAT-06 remaining_mv, EXT-24 max_delta_mv)/I_ua·1e3`, with the fallbacks and the edit order against MAT-06. Added a regression test for the `a/b → groups` rewrite.
14. RTE-18/20: screening needs third-party shapes, so `net_pair_af` takes `screens`. PlateRatio's lead C is written as an exact formula.

**Order and milestones.**
15. M0 listed the cut RTE-01/04 and promised EM/IR measurements that are REL's. It now lists RTE-32 → 02 → 09 → 03 → 07 → 08, then 05 and 06 after REL M1, with measurable exits.
16. RTE-25 moved from M2 to M1. PLC-28 needs `LatticeSpec` before RTE-15 can be exact, and RTE-25 depends only on RTE-07/10/13. The dependency graph is acyclic: RTE-32 → {05, 20, 21}; 02 → {05, 18}; 09 → {05, 06, 21}; 07 → {08, 10, 13, 25}; 10 → {11, 12, 15, 18, 25, 28}; 12 → {14, 15, 17, 19, 22, 23, 27}; 25 → PLC-28 → 15 → {17, 24, 29}; 18 → {19, 20, 21}; 21 → {22, 30, 31}.

**Open questions resolved.**
17. Q3 (history decay) was resolved by FLOW-08: reset per cold epoch, no decay. Q9 (pin-access split) was checked against the four decks' `pex` lines (102×, 1×, 1.25×, 2.4×), which closes the last "Unresolved" entry of the verification log except the runtime-only dac4 result.
