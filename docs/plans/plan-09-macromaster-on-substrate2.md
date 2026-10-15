# Plan 09 — macroMaster on Substrate2

## Decisions

- **Substrate2 is the base.** macroMaster stops being its own framework. It becomes a set of Philis plugin crates on top of [ucb-substrate/substrate2](https://github.com/ucb-substrate/substrate2), consumed as a git dependency pinned to a rev (surveyed at `01d9600`, 2026-10-05, `substrate` 0.10.3).
- **Substrate's design choices are the correct ones.** Where Philis and Substrate disagree (runtime `Process` vs type-level schemas, string pins vs typed `Io` bundles, layout-only vs schematic + layout + sim), Substrate wins. Philis contributes features Substrate lacks. It does not replace anything Substrate already has.
- **Compile-time.** PDKs are types (`scir::schema::Schema` + `layout::schema::Schema` with a layer enum). Ports are `#[derive(Io)]` bundles. Rules are associated consts or trait methods on the PDK type, not `rule("name", default)` string lookups.
- Upstream is BSD-3 (Philis is MIT): keep their copyright notice in any file derived from theirs. Upstream pins Rust 1.91.1 stable.

## What Substrate2 already has (we use it, don't rebuild it)

| Area | Upstream | Size |
|---|---|---|
| Block, typed Io, derive macros | `substrate` (`block`, `types`), `codegen` | 11.8k + 1.4k |
| Schematic IR, SPICE in/out | `libs/scir`, `libs/spice` | 7.4k |
| Layout IR, geometry, GDS, LEF/DEF | `libs/layir`, `geometry`, `gds`, `gdsconv`, `lefdef` | 12.5k |
| Tiled layout, grid routing, straps | `libs/atoll` (`Router`, `GreedyRouter`, `GreedyStrapper`) | 5.5k |
| Caching, executors | `libs/cache`, `substrate::execute` | 5.3k |
| Simulation, typed testbenches, waveforms | `substrate::simulation`, `tools/ngspice`, `tools/spectre` | 5.8k |
| PEX-backed schematics | `substrate::schematic::pex`, `tools/quantus` | |
| External signoff | `tools/pegasus`, `magic`, `netgen`, `magic_netgen` | 2k |
| PDK | `pdks/sky130` only | 4k |

## What Philis adds

| Philis feature | Today | Lands as | Substrate hook |
|---|---|---|---|
| PDKs gf180mcu, ihp_sg13g2, asap7 (+ Philis rule data for sky130) | JSON decks behind `pnr_core::Process` | `mm_pdk_*` crates; layer enum + rules generated from the decks by `build.rs` | `scir::schema::Schema`, `layout::schema::Schema`, a `PhilisRules` trait for spacing/sheet-R/cut-R |
| Device generators (MOS, matched pair, R, C, diode, common-centroid, dummies, rings) | `kernel/cells` via `cells::Builder` | `mm_cells`: one `Block + Schematic + Layout` per generator, generic over `PhilisRules` | `Layout`, `Schematic`, atoll `Tile` for tileable ones |
| In-process signoff (DRC/LVS/ERC/PEX via GPurify) | `backend/verify` | `mm_verify` tool crate shaped like `tools/pegasus` (`DrcParams`/`DrcData`, `LvsParams`/`LvsData`), but in-process | Consumes `export_layir` + `export_scir`; PEX output feeds `schematic::pex` for post-layout sim |
| Reliability (EM, antenna, latch-up, derating) | `frontend/library/reliability.rs`, `analog` rules | Checks in `mm_verify`, typed results | Same inputs as DRC, plus sim currents from upstream ngspice |
| Constraint inference (91 topologies, net classes, symmetry) | `backend/annotator` | `mm_constraints`: runs over a block's exported SCIR and returns typed constraints as `NestedData` | `Context::export_scir` |
| Matching and symmetry validation | `analog` placement rules | Layout check pass: given constraints + layout, report met/violated | `export_layir` + constraint data |
| Automatic placement | `backend/gp`, `backend/dp` | `mm_place`: computes transforms for instances already in a `CellBuilder` | Instance transforms in `layout::CellBuilder` |
| Automatic routing | `backend/gr`, `backend/dr` | `mm_route`: (a) an `atoll::route::Router` impl; (b) a post-layout router over a finished cell's LayIR | `atoll::route::Router`; LayIR |
| Netlist → GDS CLI flow | `frontend/library::elaborate`, `frontend/cli` | Stays, rebuilt on the crates above (see open question 1) | `libs/spice` parser → SCIR |

## Phases

Each phase ends green on `cargo test` with the old macroMaster API still working. The old API is deleted only in P7.

0. **Pin and prove.** Add substrate2 git deps at a fixed rev. Build upstream's `sky130_inverter`, export LayIR → GDS and SCIR → SPICE, and run them through Philis's existing `verify` from a test. Exit: DRC and LVS clean on that inverter.
1. **`signoff::run`** (done). GPurify DRC/LVS/ERC with typed results. Exit: the P0 test calls it and no longer goes through `backend/verify` glue.
2. **Typed PDKs** (done, MOS only). `build.rs` codegen from the gf180mcu/ihp/asap7 decks: layer enum, MOS models, schema. sky130 reuses upstream's `Sky130` type and implements `PhilisPdk` on it. Exit: a MOS signs off on all four PDKs. R/C/diode primitives come with P3.
3. **`mm_cells`.** Port each `cells` generator to a `Schematic + Layout` block. Exit: per generator and PDK, the new GDS equals the old `cells` output geometrically (shape-set diff in tests).
4. **`mm_place` + `mm_route`.** Exit: a sizing-generic inverter signs off clean on all four PDKs (moved from P2), must-join pins are joined, and the three-stage op-amp example, rebuilt as a Substrate block, is auto-placed and auto-routed and signs off clean.
5. **`mm_constraints` + matching validation.** Exit: the OTA's inferred constraints match the current annotator's output; the layout check reports them met.
6. **Sim and PEX.** Replace `oppoint.rs`/`perf.rs` with typed testbenches on upstream `tools/ngspice`, and feed GPurify PEX into `schematic::pex`. Exit: a post-layout OTA testbench runs on ngspice.
7. **Migrate and delete.** Move `library::elaborate`, `emit.rs` (which generates macroMaster source and must now generate Substrate source), the tests and the examples. Delete the old `macro_master` API. Exit: no `macro_master::` imports outside the new crates.

## Risks

- **API churn.** Substrate is 0.x and moves fast. Pin a rev, bump on purpose, and send fixes upstream rather than forking.
- **Atoll router access.** `RoutingState` fields are `pub(crate)` (`libs/atoll/src/grid.rs:639`), and Atoll routes on its own abstract grid. A Philis `Router` impl likely needs an upstream PR to expose state. Fallback: route (b), on LayIR after layout.
- **sky130 layer and model naming.** Upstream's `Sky130` names layers and models in its own way, which may not match Philis's deck. Map once in `PhilisRules`; don't fork the PDK crate.
- **Dependency weight.** `substrate` pulls in caching, tera, and other crates. Keep it out of the viewer/GPU build, as the workspace already does for `gdsverify`.

## P0 status (done)

- `macro_master` re-exports substrate2 at its root (pinned `01d9600`). The old API lives in `macro_master::legacy` until P7.
- `macro_master::signoff::check` runs GPurify DRC/ERC/LVS/PEX on a Substrate block: flattened LayIR → deck layers through the PDK's GDS map, SCIR (Spice schema) → LVS reference, device kinds from the deck (`verify::Pdk::model_kind`).
- `kernel/macroMaster/tests/sky130_inverter.rs`: upstream's sky130 inverter signs off clean, and a resized schematic fails LVS (negative control).
- Build needs: `.cargo/config.toml` defines the `substrate` registry (without it cargo skips every substrate2 package as malformed), and `protoc` (substrate2's `cache` build script). The flake's devShell and package now include protobuf.
- Substrate's derives resolve the crate by the name `substrate`, so a crate using `#[derive(Io, Block)]` depends on `substrate` directly as well as `macro_master`. Upstream PR candidate: a crate-path override.
- Unverified: the substrate2 flake `outputHashes` entry. `nix build` currently stops earlier, on a stale GPurify hash from `234d9da` (expected `sha256-CF7k80QDIDnVXl7vwn+KFGnVP8BAoVKvTuiS6sdVVU4=`).

## P1 + P2 status (done)

Philis features live as modules of `macro_master`, not separate `mm_*` crates: the crate is substrate2 re-exposed, plus these.

- **P1, `macro_master::signoff::run(ctx, block)`**: signs off any block whose layout and schematic share a `PhilisPdk`, and returns a typed `Report` (`drc`/`erc`/`lvs`/`other`, `warnings`, `coverage`, `caps`; `is_clean`, `lvs_matches`). `check` remains the step underneath, for libraries already exported. Every shape of every port is labelled.
- **P2, `macro_master::pdk`**:
  - `PhilisPdk` (deck, GDS map, MOS primitive, SPICE export) and `MosModel`.
  - `build.rs` generates `gf180mcu`, `ihp_sg13g2` and `asap7` from their decks through `verify`: a `Layer` enum (55/75/45 drawn layers), a `Model` enum of MOS models with polarity, the schema type, and its `FromSchema` to `Spice`.
  - `PhilisPdk` is implemented on upstream `sky130::Sky130`, which stays upstream's.
  - `Mos<M>` is a `Block + Schematic + Layout` on any of the four, drawn by `cells` (FinFET on a fin deck, flavour markers through `verify::Pdk::mos_overlay`, moved there from `cellgen`). `to_gds::<P>` writes GDS with port labels.
- **Tests** (`tests/pdks.rs`): NMOS and PMOS at nf 1 and 2 on all four PDKs. asap7 LVT/SLVT extract as themselves; an RVT layout against an LVT schematic fails LVS.

Findings:

- **Must-join pins.** The planar generator leaves same-terminal pins for the router: two outer sources at nf ≥ 2, and sky130's two body taps. Signed off alone, they read as `label_conflict`, unpaired nets/devices, or `soft_connection`. The tests allow exactly those rules for split devices and require DRC clean. P4 joins them.
- **Minimum widths are dirty.** At the deck minimum finger width (what `retarget` draws for a MOS with no `w`), gf180mcu and ihp fail contact enclosure (CO.4, Cnt.c) and asap7 fails M1.S.2. The generators' own clean tests draw 1680 nm. A `cells` fix.
- **gf180mcu 6 V models.** `min_channel` gives them the 3.3 V length (PL.2_5V_n), and the `dualgate` overlay misses DV.6/7/8 and DF.2a. A `cells`/`verify` fix.
- P2's original exit (an inverter on all four PDKs) needs routing between devices, so it moves to P4.

## Settled questions

1. **Runtime PDKs.** The CLI keeps a runtime `DynPdk` path for JSON-sidecar processes. Generators target typed PDKs.
2. **Where features live.** In Philis: `macro_master` re-exposes substrate2 with Philis's features added, rather than sending them upstream.
