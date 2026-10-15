# Plan 09 — macroMaster on Substrate2

## Decisions

- **macroMaster is Substrate2, re-exposed with more features.** `macro_master` re-exports [ucb-substrate/substrate2](https://github.com/ucb-substrate/substrate2) at its root, consumed as a git dependency pinned to `01d9600` (2026-10-05, `substrate` 0.10.3). The added features are modules of the same crate.
- **Substrate's design choices are the correct ones.** Where they and Philis's disagree, Substrate wins: type-level schemas over the runtime `Process`, typed `Io` bundles over string pins, schematic primitives kept apart from layout tiles. Features are added; nothing Substrate already has is replaced (sky130 stays upstream's `Sky130` and its tiles).
- **Compile-time.** PDKs are types (`scir::schema::Schema` + `layout::schema::Schema` with a layer enum), ports are `#[derive(Io)]` bundles.
- **Philis is a separate issue.** This plan does not port Philis's flow (generators, placer, router, `library`) onto macroMaster. Philis keeps using `macro_master::legacy`.
- Upstream is BSD-3 (Philis is MIT): keep their copyright notice in any file derived from theirs.

## Substrate2 provides (used, not rebuilt)

| Area | Upstream |
|---|---|
| Block, typed Io, derive macros | `substrate` (`block`, `types`), `codegen` |
| Schematic IR, SPICE in/out | `libs/scir`, `libs/spice` |
| Layout IR, geometry, GDS, LEF/DEF | `libs/layir`, `geometry`, `gds`, `gdsconv`, `lefdef` |
| Tiled layout, grid routing, straps | `libs/atoll` |
| Caching, executors | `libs/cache`, `substrate::execute` |
| Simulation, testbenches, waveforms | `substrate::simulation`, `tools/ngspice`, `tools/spectre` |
| PEX-backed schematics | `substrate::schematic::pex`, `tools/quantus` |
| External signoff | `tools/pegasus`, `magic`, `netgen`, `magic_netgen` |
| PDK | `pdks/sky130` |

## macroMaster adds

- **`signoff`**: in-process GPurify signoff.
  - `run(ctx, block)` covers DRC, ERC, LVS and PEX for a block with a layout and a schematic on one `PhilisPdk`.
  - `run_layout(ctx, block)` covers DRC and ERC for a layout-only block (a tile). LVS is recorded as not run, never as a pass.
  - Both return a typed `Report` (`drc`/`erc`/`lvs`/`other`, `warnings`, `coverage`, `caps`; `is_clean`, `layout_clean`, `lvs_matches`).
  - `check` is the step underneath, for libraries already exported. Device kinds come from the deck (`verify::Pdk::model_kind`).
  - Every shape of every port is labelled. A layer the deck does not draw is a coverage row, never dropped silently.
- **`pdk`**: typed PDKs for every deck Philis carries.
  - `build.rs` generates `gf180mcu`, `ihp_sg13g2` and `asap7` from their decks through `verify`: a `Layer` enum (55/75/45 drawn layers), a `Model` enum of MOS models with polarity, the schema type, and its `FromSchema` to `Spice`.
  - `PhilisPdk` is implemented on upstream `sky130::Sky130`.
  - Devices follow Substrate's split. `Mos<M>` is the schematic primitive, on any of the four PDKs, like upstream's `Nfet01v8`. `MosTile<M>` is the layout on a generated PDK (`DrawnPdk`), like upstream's `NmosTile`.
  - Each `MosTile` terminal is an array with one entry per region the drawing leaves unjoined (two sources on a two-finger planar device; one on a FinFET, which straps them). The composing block draws the joins.
  - `to_gds::<P>` writes GDS with port labels.

## Status

- **P0 (done)**: pin substrate2, re-export it, and sign off upstream's sky130 inverter through GPurify. A resized schematic fails LVS (negative control).
- **P1 (done)**: typed `signoff::run` / `run_layout` / `Report`. The inverter test uses `run`.
- **P2 (done, MOS)**: typed PDKs, with `Mos` + `MosTile`. In `tests/pdks.rs`:
  - a one-finger device (primitive + tile, composed) signs off clean on gf180mcu, ihp_sg13g2 and asap7;
  - a two-finger tile is DRC/ERC clean;
  - the planar tile exposes two source regions and the FinFET tile one;
  - asap7 LVT/SLVT extract as themselves, and an RVT layout against an LVT schematic fails LVS.
  - The sky130 inverter builds its schematic on `Mos` with upstream's tiles.

## Next (candidates)

- **R/C/diode primitives** on the generated PDKs, with their SPICE cards.
- **Simulation on the generated PDKs**: the SPICE card form each simulator's model files want (`X` subckt vs `M`, `nfin`), on upstream `tools/ngspice`.
- **PEX into `schematic::pex`**: GPurify's extracted network as a post-layout schematic.

## Build notes

- **Registry:** `.cargo/config.toml` defines the `substrate` registry. Without it cargo skips every substrate2 package as malformed.
- **Nix vendoring:** the flake hands `importCargoLock` a cargo with `CARGO_REGISTRIES_SUBSTRATE_INDEX` set, because vendoring runs `cargo metadata` inside the substrate2 checkout. Vendoring and an offline `cargo metadata --locked` against the vendored sources both succeed; a full `nix build` has not been run since.
- **protoc:** substrate2's `cache` build script runs it. The flake's devShell and package include protobuf.
- **Derive crate name:** Substrate's derives resolve the crate by the name `substrate`, so a crate using `#[derive(Io, Block)]` depends on `substrate` directly as well as `macro_master`. Upstream PR candidate: a crate-path override.

## Risks

- **API churn.** Substrate is 0.x. Pin a rev, bump on purpose, and send fixes upstream rather than forking.
- **Dependency weight.** `substrate` pulls in caching, tera, tonic. Keep it out of builds that do not need it.

## Philis-side findings (separate issue)

Found while drawing `MosTile` through Philis's `cells`; left for Philis:

- **Minimum widths are dirty:** at the deck minimum finger width (what `retarget` draws for a MOS with no `w`), gf180mcu and ihp fail contact enclosure (CO.4, Cnt.c), and asap7 fails M1.S.2. The generators' own clean tests draw 1680 nm.
- **gf180mcu 6 V models:** `min_channel` gives them the 3.3 V length (PL.2_5V_n), and the `dualgate` overlay misses DV.6/7/8 and DF.2a.
- **Failing tests:**
  - `gp::Prices::bind`'s `debug_assert!` fires in debug builds of every `library::run`, failing the flow-based `library` tests.
  - `ota5t_clean_on_asap7` (transistor count) fails.
  - `cells`' `cap_array::extracted_bit_ratios_follow_the_weights` fails.

## Settled questions

1. **Runtime PDKs.** Philis's CLI keeps its runtime, JSON-sidecar `Process` path. macroMaster's PDKs are types.
2. **Where features live.** In `macro_master`, which re-exposes substrate2 with the features added, rather than upstream.
