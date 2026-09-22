# Substrate3 — PDK-agnostic generator authoring + Solution decompilation

Goal: any circuit's layout generatable PDK-agnostically; PDK swappable on the
fly (`elaborate(block, pdk)`); and the P&R flow's `Solution` decompilable into
generator *source* targeting the same API. Emitted code freezes search
*decisions* (variants, relative order, symmetry, net topology), never *numbers*
(nm). Elaboration recomputes numbers from the target `Process`.

## Gap audit (2026-09-14, macroMaster/src/lib.rs)

| # | Gap | Status |
|---|-----|--------|
| 1 | Pins dropped: `CompBuilder::commit` copies shapes only; `variants::Mos/Res::layout` replay shapes only → composed Macro unroutable | M1 |
| 2 | No routing: `connect` records strings, no gr/dr call | M2 |
| 3 | No instance-qualified nets: `connect("a","a")` flat strings — no `m1.d → vout`; structural LVS degenerate | M1 |
| 4 | No hierarchy: Composition instantiates DeviceGen only | M4 |
| 5 | No orientation/symmetry: translate only; `pnr_core::Orient` unused here | M3 |
| 6 | Library thin: Mos/Res only; no Pattern/dummies, no Cap/BJT/Diode | M5 |
| 7 | No `elaborate → Solution/GDS` entry | M2 |

Non-gap: rule-named spacing — `place_by(.., c.process().rule("x", d))`
already PDK-agnostic; emit prints exactly that.

## Track A — finish substrate3 (macroMaster)

- **M1** Pins retained through commit (qualified `inst.pin`); instance names at
  `instantiate(name, dev)`; connect over qualified terminals; union-find net
  resolution (class named by member io port, else synthesized); pins bound to
  resolved `NetId`s. Device tier: `DeviceBuilder::pin`, variants forward
  cells pins (`d0:G` → `g`; resistor `d0:P/N` → `a/b`).
- **M2** `elaborate(comp, pdk, cfg) → Solution`: build → route (gr+dr as
  libraries; extract reusable `route()` from `library::run` internals, incl.
  layer-stack derivation) → signoff (verify) → gds bytes. Lives in
  frontend/library (dep direction: kernel crates must not depend on frontend).
- **M3** `Instance` orient (D4 `Orient`, GDS SREF-compatible); mirror keeps
  on-grid (turn about bbox corner, axis grid-aligned); `place_mirrored`;
  centroid helper.
- **M4** Hierarchy: instantiate any layout-bearing Block; child ports bind to
  parent nets; netlists merge.
- **M5** Library: Mos{pattern,dummies}, Res{segments,pattern}, Cap (metal-stack
  constructions), Bjt, Diode — all via adapter from `cells`, checked vs
  GenericPdk.
- **M6** Acceptance: hand-written 5T OTA Composition elaborated on sky130 AND
  generic deck → both signoff clean. Zero literal nm outside device params.

## Track B — emit (Solution → GenIr → source)

IR-first: `emit(sol, cells, problem) → GenIr`; `elaborate_ir(ir, pdk) →
Solution` (interpreter — round-trip tests need no rustc); codegen pretty-prints
IR as a macroMaster Composition.

GenIr: instances (cell family + params from `Cells::spaces` +
`Layout::variant`), abutment chains/seq-pair, gap→rule-name candidates (max at
elaboration; unattributed slack compacts away unless budget-driven → explicit
keepout), symmetry (`Layout::axis`/`groups`), branch commitments, nets,
route_cfg (frozen seed + net order).

Phases: P0 round-trip harness (5T OTA, sky130: DRC-clean + LVS-true + instance
bboxes match) → P1 placement lift → P2 route replay (needs M2) → P3 codegen +
`philis emit` CLI → P4 emitted generator elaborated on second deck, clean.

Risks: variant infeasible on target → re-enumerate nearest (params+pattern
kept); ambiguous gap attribution → max of candidates (never under-spaces);
route determinism → frozen seed/order in IR.

Order: M1 → M2 → M3 → M6 → B(P0–P4); M4/M5 fill as needed.

## Status (2026-09-14)

Done: M1, M2, M3, M6, B v1 (P0–P4).
- `library::elaborate(&comp, &pdk, cfg)` — build → route → `signoff_drc`.
- `ota_cross_pdk.rs`: one `Ota5T` source, sky130 + generic_finfet, device
  layers DRC-clean, all nets routed, zero literal nm.
- `library::emit`: `emit(netlist, layout, pdk, cfg) → GenIr`,
  `elaborate_ir(ir, pdk, cfg)` (interpreter), `to_rust(ir)` (source printer),
  `philis emit a.sp deck.json out.rs` CLI. `emit_roundtrip.rs` proves
  round-trip + retarget to second deck; gaps attribute to `device_gap`.
- verify→GPurify rewrite port: done. Everything builds against the local
  `rewrite/definition-phase` checkout (`../GPurify`); the pinned worktree is
  gone from every Cargo.toml.

Open debt (tasks): M4 hierarchy; M5 patterned library (unlocks emitting
merged matched groups + symmetry lift to `place_mirrored`); dr li pin-access
spacing (then tighten M6 gate to `drc.is_empty()`); run() debug open-net
panic (then flow→emit integration test via `emit_solution`).
