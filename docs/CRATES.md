# Crates: data in, data out

Flow: `library::run(spice, pdk, macros, cfg)` → parse → `annotator` → `cells`
→ `gp` → `dp` → `gr` → `dr` → `verify::signoff`. Each epoch places, routes and
signs off; the best epoch (lexicographic |V|, Θ, PEX) wins.

## kernel/core (`pnr_core`) — no deps
- **In:** `Netlist` (from `library::parse`, `macroMaster`); `Layout` columns
  written by gp/dp (x, y, hw, hh, orient, variant, branch, groups, power_uw);
  `Routes.wires` from gr/dr; `Process` impl (`verify::Pdk`).
- **Out:** id types, `Macro`/`Shape`/`Pin`/`Orient`/`Dir`; `BipartiteHypergraph`,
  `UnionFind`; `Layout` helpers (bbox, centre, edge_gap, axis_x);
  `thermal::rises_mc`; `place_macro(s)`; `Report`/`Violation`/`lex`;
  `Routes::shapes`/`length`.

## kernel/analog — deps: core
- **In:** hypergraph + union-find (from annotator's `extract` calls); `Layout`
  or `Routes` to score against; `cell_of` maps from library.
- **Out:** rule types — placement: `MatchingPair`, `Symmetry(Group)`,
  `CommonCentroid`/`CentroidGroup`, `DtiBand`, `Isolation`, `Proximity`,
  `ThermalGradient`; routing: `Antenna`, `CouplingBudget`,
  `CrosstalkExclusion`, `Differential`, `ParasiticBudget`. `RuleBatch` scoring
  (cost, violations, residual, project, retarget, branches). `Requirements`
  (hard / budget / cost arms). `Constraints` + `cell::*` (unitization, guard
  rings) for cells. `metadata::NetClassification`.

## backend/annotator — deps: core, analog
- **In:** `Netlist`; `AnnotationConfig` (do-not-identify, extra supply/ground/clock names).
- **Out:** `Problem`: recognised `blocks`, `groups` (→ `Layout::groups`),
  `abutment` (diffusion sharing), `placement` rules (→ gp/dp), `routing` rules
  (→ gr/dr), `net_classes`, cell `constraints` (→ cells, `library::cellgen`).
  Only 2-device primitives emit pair constraints; stacks get Proximity only.

## kernel/cells — deps: core, analog
- **In:** device params (w, l, nf, kind), `Unitization`, guard-ring reqs, `Pdk` cell section.
- **Out:** every legal drawn variant per device/matched group (`enumerate`/`draw`),
  guard-ring macros. Geometry must be DRC-clean against the real sky130 tech (magic).

## backend/gp — deps: core, analog
- **In:** macros, `VariantSpace`s, placement `Requirements` (cell-indexed), `Prices`.
- **Out:** coarse `Layout` + `Report`. Shares `Prices`, `VariantSpace`,
  `CLEARANCE_NM`, `mechanics` (nets, HPWL, encroachment, objective) with dp.

## backend/dp — deps: core, analog, gp
- **In:** coarse `Layout` (groups, axis, power filled by library), macros,
  variants, `Requirements`, fixed cells, prices, seed.
- **Out:** legal `Layout` (≥ clearance apart, hard symmetry projected, variant/
  orient/DTI branch chosen) + `Report`. Every move goes through one `Sa::trial`;
  hard equalities are re-projected so mirror partners follow.
- **PEX is dimensionless (PLC-18):** `HPWL/L_ref + Σ criticality·cost + priced
  budgets`, `L_ref = sqrt(Σ cell area)`; every cost is a squared ratio to its own
  length (rule distance, DTI band, or `L_ref`). T0 = `t0_scale·mean |ΔPEX|`, no floor.
  Term shares of Σ|Δterm| over the 128 T0 probes, `metrics_are_populated_on_ota`
  (release, measured once): first (cold) dp call — HPWL/L_ref 75.4 %, Proximity
  18.9 %, Symmetry 5.7 %, MatchedSet 0 %, priced budgets 0 % (pass: none > 80 %);
  the other cold calls peak at HPWL 68 %. The run's last dp call is 100 % priced
  budgets (mean |ΔE| 7.3e4 vs ~1 elsewhere): once T6 prices have stepped, λ·residual
  swamps the rest — a prices/energy-weight question for PLC-09, not a unit one.

## backend/gr — deps: core, analog
- **In:** placed `Layout` + `Macro`s, guard rings, routing `Requirements`,
  layer stack, shared `Negotiation`.
- **Out:** coarse `Routes` (gcell segments per net) + `Report`; `GroupPrice` to cellgen.

## backend/dr — deps: core, analog, gr
- **In:** coarse `Routes`, placed pins/macros, rings, `Requirements`, metals,
  cuts, `Negotiation`, `DetailedCfg` (pitch, width, pin access, `net_current_ua`,
  `supply_nets`).
- **Out:** final `Routes` on PDK layers (wires, cuts, pads, access jogs) +
  `Report` (opens, unlanded pins, shorts hard; overuse, EM under-width, analog
  budgets as budget). Differential pairs rerouted as mirror images; crosstalk
  victims priced off aggressor tracks; EM width from the deck's per-layer limit (`DetailedCfg::em_limit`).

## backend/verify — deps: core, analog, GPurify
- **In:** shapes, `LabeledPin`s, `RefInput` (schematic reference), deck JSON.
- **Out:** `Pdk` (layers, rules, grid, metals, vias, `layer_gds`) to everyone;
  `signoff_checked()` → `Signoff { report, warnings, coverage, caps }`: error
  rows `{drc|erc|lvs|engine}/rule:layer` + total C, deck warnings apart, and
  `Coverage` (LVS-unverified devices, rules not run, with reasons);
  `drc()`/`erc()` located findings; `extract_spice`; `Checker`.

## kernel/macroMaster — deps: core, analog, cells
- **In:** user `Composition`/`DeviceGen` code, a `Process` (sky130 deck), cells geometry for `Mos`/`MatchedPair`/`Res`.
- **Out:** `build_composition`/`build_with` → `BuiltComp` (placed macros, net names, ports, netlist) for `library::elaborate`/`emit` (manual path, no gp/dp); `Macros` override registry checked first by cellgen.

## kernel/visualizer
- **In:** GDS bytes, `(name, (gds layer, datatype))` pairs (`layer_names`), `Shape`/`Macro` with a `Pdk::layer_gds` table.
- **Out:** `parse_gds`, `export_svg`, `show_macro`, `run_viewer`.

## frontend/library — the orchestrator
- **In:** SPICE text, `Pdk`, user `Macros`, `Config`; optional ngspice op point.
- **Out:** `Solution` (layout, routes, macros incl. rings, netlist, `RunStats`,
  budget report); `signoff`/`signoff_inputs`; `gds::emit`; `elaborate`; `emit`
  (solution → PDK-agnostic generator source).

## frontend/cli (`philis`)
- `philis [run|emit] <netlist.sp> <deck.json> [out.rs] [-o DIR] [--seed N]
  [--max-iters N] [--starts N]` → run → optional emit → signoff verdict as exit
  code. `-o DIR` writes `<top>.gds` (the `.subckt` ports as labels on the deck's
  text layers), `<top>_ref.spice` (the LVS reference, dummies included),
  `signoff.txt`, `signoff.json`. `--version` names the commit (`PHILIS_GIT_REV`
  when packaged without `.git`). `--interface` is accepted and ignored with a
  warning: no fixed die or boundary pins yet.

## benchmarks
- `cargo run --release -p benchmark --bin bench local` — per-circuit DRC/LVS/
  ERC/C/WL/area table + constraint satisfaction. Artifacts in `target/bench_debug/`.
- `xcheck*.py` cross-check DRC/LVS/PEX against KLayout. Real-sky130 DRC via magic:
  `magic -dnull -noconsole -T ~/.volare/.../sky130A.tech script.tcl`.

## Stage contracts

- **Epoch key (FLOW-02).** `LexKey = (|V|, spec miss, Θ, C tier, footprint)`
  (`frontend/library/src/lib.rs`, `type LexKey`), compared by `key_lt`: |V|
  counts violated hard rules per rule plus signoff errors (deck warnings and
  `lvs-coverage/` rows stay out), Θ is the metadata residual in milli-budgets,
  C within `C_TIE` is a tie broken by footprint, NaN reads as +∞.
- **Prices (FLOW-03).** `gp::Prices` (`backend/gp/src/lib.rs`) keys each λ by
  batch identity (`PriceKey`, GAP-10), takes one dual step per epoch
  (`settle`), relaxes on slack, and reports `drift` and `saturated`; a run
  converges when feasible with drift under `PRICE_STATIONARY` and nothing
  saturated.
- **Schedule (FLOW-08, not merged).** Today `library::search` runs every
  epoch with `dp::Schedule::cold()`, stops a middle loop after `PATIENCE`
  non-improving epochs, and escalates the variant assignment
  (`cellgen::escalate`, an odometer over the alternatives `seed_assignment`
  keeps) when not converged. FLOW-08 replaces this with warm/cold epochs,
  blame-driven escalation and `RunStats.stop`.

## Open issues (triage, plan-08 §1.6, re-checked on the M2 tree)

| # | Issue | Status | Owner |
|---|---|---|---|
| 1 | bjt_mirror magic violations; `ntap`/`ptap` split | Open: `pdks/decks/sky130.deck` now recognises BJTs and CELL-09 (M1) draws fixed-geometry emitters; the magic count is unmeasured | PERF-19 (foundry signoff) |
| 2 | Routing EM not wired | Closed (REL-01, M0): `supply_nets`, `pin_ua`, `em` set in `library::topology` | — |
| 3 | Placement net weights | Closed: `gp::net_weights` | — |
| 4 | Placer grid 10 nm | Closed (PLC-02, M1): cut-lattice-aligned boxes | — |
| 5 | LU.2 latch-up | Open: no latch-up rule in `pdks/*.json` | CELL-13 (tap reach), REL (in-flow check) |
| 6 | xhrpoly contacts, `pex.rbody`, multi-segment | Partly closed: 190×2000 slot contacts are the deck's construction (`pdks/sky130.json`, resistor note); multi-segment magic check open | CELL-14, MAT-12 |
| 7 | Diode anode tap ≥ 410 nm | Closed (CELL-07, M1) | — |
| 8 | Centroid diff pairs discarded | Closed (stale) | — |
| 9a | `REQUIRED_RULES` obsolete `bjt_*` | Closed (FLOW-04, M0): `verify::sidecar::KEYS`, checked both ways by `kernel/cells/tests/deck_keys.rs` | — |
| 9b | `asymmetric_enclosure` stricter than magic | Open | PERF-19 (foundry cross-check decides) |
| 9c | LVS device-count / parametric | Closed (PERF-01, M0): range checks are NotInDeck; MOS W/L compared as `lvs.parameter_mismatch`, non-MOS values listed as skipped in `Coverage` | — |
| 9d | `ir_drop` needs intent | Closed: `elaborate::intent` | — |
| 10a | dp always runs 220 iterations | Open | PLC-09 |
| 10b | `mosfet.rs` links to `VariantSpace::lock` | Closed (stale) | — |
| 10c | Unused `Unitization` / `GuardRingRequirement` fields | Closed for `Unitization` (FLOW-15: `same_variant_required`, `SeriesParallel::RepeatedStage` deleted); guard rings: GAP-05 (M1) | REL-07 (ring fields it wires) |
| 10d | Only `frontend/cli` rustfmt-clean; CI `fmt` advisory | Open | FLOW-14 follow-up (one formatting commit) |
| 10e | `xcheck_lvs.py` MOS only, not re-run | Open | PERF-19 |
