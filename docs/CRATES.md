# Crates: data in, data out

Flow: `library::run(spice, pdk, macros, cfg)` → parse → `annotator` → `cells`
→ `gp` → `dp` → `gr` → `dr` → `verify::signoff`. Each epoch places, routes and
signs off; the best epoch (lexicographic |V|, Θ, PEX) wins.

## kernel/core (`pnr_core`) — no deps
- **In:** `Netlist` (from `library::parse`, `macroMaster`); `Layout` columns
  written by gp/dp (x, y, hw, hh, orient, variant, branch, groups, power_uw);
  `Routes.wires` from gr/dr; `Process` impls (`verify::Pdk`, `macroMaster::GenericPdk`).
- **Out:** id types, `Macro`/`Shape`/`Pin`/`Orient`/`Dir`; `BipartiteHypergraph`,
  `UnionFind`; `Layout` helpers (bbox, centre, edge_gap, axis_x);
  `thermal::rises_mc`; `place_macro(s)`; `Report`/`Violation`/`lex`;
  `Routes::shapes`/`length`.

## kernel/analog — deps: core
- **In:** hypergraph + union-find (from annotator's `extract` calls); `Layout`
  or `Routes` to score against; `cell_of` maps from library.
- **Out:** rule types — placement: `MatchingPair`, `Symmetry(Group)`,
  `CommonCentroid`/`CentroidGroup`, `DtiBand`, `Isolation`, `Proximity`,
  `ThermalGradient`; routing: `StraightNet`, `Antenna`, `CouplingBudget`,
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
  victims priced off aggressor tracks; EM width = I / 1 mA/µm.

## backend/verify — deps: core, analog, GPurify
- **In:** shapes, `LabeledPin`s, `RefInput` (schematic reference), deck JSON.
- **Out:** `Pdk` (layers, rules, grid, metals, vias, `layer_gds`) to everyone;
  `signoff()` → `Report` rows `{drc|erc|lvs|engine}/rule:layer` + total C;
  `drc()`/`erc()` located findings; `extract_spice`; `Checker`.

## kernel/macroMaster — deps: core, analog, cells (+verify, visualizer optional)
- **In:** user compositions (`place`/`place_by` calls), `GenericPdk`.
- **Out:** drawn hierarchical macros for `library::elaborate` (manual path, no gp/dp).

## kernel/visualizer
- **In:** GDS bytes, deck JSON, `Shape`/`Macro`.
- **Out:** `parse_gds`, `export_svg`, `show_macro`, `run_viewer`.

## frontend/library — the orchestrator
- **In:** SPICE text, `Pdk`, user `Macros`, `Config`; optional ngspice op point.
- **Out:** `Solution` (layout, routes, macros incl. rings, netlist, `RunStats`,
  budget report); `signoff`/`signoff_inputs`; `gds::emit`; `elaborate`; `emit`
  (solution → PDK-agnostic generator source).

## frontend/cli (`philis`)
- `philis [emit] <netlist.sp> <deck.json> [out.rs]` → run → optional emit →
  signoff verdict as exit code.

## benchmarks
- `cargo run --release -p benchmark --bin bench local` — per-circuit DRC/LVS/
  ERC/C/WL/area table + constraint satisfaction. Artifacts in `target/bench_debug/`.
- `xcheck*.py` cross-check DRC/LVS/PEX against KLayout. Real-sky130 DRC via magic:
  `magic -dnull -noconsole -T ~/.volare/.../sky130A.tech script.tcl`.
