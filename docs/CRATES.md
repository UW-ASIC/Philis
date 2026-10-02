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

## kernel/macroMaster — deps: core, analog, cells
- **In:** user `Composition`/`DeviceGen` code, a `Process` (sky130 deck), cells geometry for `Mos`/`MatchedPair`/`Res`.
- **Out:** `build_composition`/`build_with` → `BuiltComp` (placed macros, net names, ports, netlist) for `library::elaborate`/`emit` (manual path, no gp/dp); `Macros` override registry checked first by cellgen.

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

## Open issues (next session)
1. **bjt_mirror: 15 magic violations, all from the BJT cell.** The deck recogniser `[poly, sd, sd]` forces a toy BJT. A real concentric PNP/NPN was magic-clean on its own. It needs the deck `tap` split into `ntap`/`ptap` with terminals `[ntap, sd, ptap]`, but that split broke LVS everywhere. Suspects: `floating_well` and `soft_connection` still name `tap`.
2. **Routing EM is not wired.** Fill `dr::DetailedCfg.supply_nets` from Supply/Ground net classes. Restore the drain current `id` in `oppoint.rs` and set `net_current_ua[n] = max(max|id|, Σ|id|/2)`.
3. **Placement net weights.** Pass a per-net `&[f32]` into `gp::place`/`dp::place` HPWL, derived from net class, `c_budget_af` and `shielding_required`. Delete those fields if the weights don't help.
4. **Placer grid.** Cell origins should snap to 10 nm. At an odd multiple of 5 nm, cut overlaps fail magic.
5. **LU.2 latch-up.** Wide NMOS fingers may need a second tap row. The deck can't express LU.x.
6. **xhrpoly resistor.**
   - Needs real 0.19×2.0 µm contact slots (`licon_max_width` is 170 today).
   - `pex.rbody` should be about 319 Ω/sq, not 48.2.
   - Multi-segment resistors have not been magic-checked in the flow.
7. **Diode.** The anode tap needs to be at least 410 nm wide (licon.7).
8. **Centroid diff pairs are always discarded.** Interleaved gate straps cross each other; staggering the strap depth per device would fix it.
9. **Deck and verify.**
   - ~~`verify` `REQUIRED_RULES` still demands obsolete `bjt_*` cell keys.~~ Closed (FLOW-04): `verify::sidecar::KEYS` requires every key a generator reads with a compiled default, except keys that only raise a deck-derived value and the `npn_isolation` flag (kernel/cells/tests/deck_keys.rs checks both directions, and that every other name a generator reads resolves from the loaded PDK; `max_finger_width`, read by the library, is required by name; post_cell.rs `guard_ring_merge_gap_nm` is not reached by that run).
   - `asymmetric_enclosure` is stricter than magic's "one direction".
   - Settle whether LVS checks device count and params: signoff marks `lvs.device_count_*` / `lvs.parametric` NotInDeck, while verify says the reference comparison already covers them.
   - `ir_drop` needs design intent (supply nets + currents).
10. **Smaller items.**
    - `dp` always runs all 220 iterations; stop it when the result stops improving.
    - `kernel/cells/src/mosfet.rs:719` links to the removed `VariantSpace::lock`.
    - Unused `Unitization`/`GuardRingRequirement` fields can now go.
    - Only `frontend/cli` is rustfmt-clean (`cargo fmt --all --check` reports diffs in the other 15 workspace crates, not only gr/dr); CI's `fmt` job stays advisory until one formatting commit.
    - `xcheck_lvs.py` covers MOS only and was not re-run.
