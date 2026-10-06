# Audit 06 — Signoff, decks and benchmarks (backend/verify, pdks, benchmarks) + ground-truth run

Scope: physical verification (DRC/LVS/ERC/PEX) as Philis wires it to GPurify, the PDK sidecars and the
GPurify decks they name, the benchmark harness and its fixtures, and the cross-check scripts. Finding IDs:
`AV-NN`. Code citations are `file:line` against the working tree of 2026-09-28 (large uncommitted changes
included; `backend/verify/build.rs`, `benchmarks/characterize_mismatch.py`, `fixtures/bgr_core.spice`,
`fixtures/dac4.spice` are untracked). GPurify is cited as `GP/…` =
`~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/`, the rev `Cargo.lock:676–678` pins
(`8df8c09af1dfd849d5271abb4f6f03e595cbf390`). The installed foundry PDK is cited as `VOL/…` =
`~/.volare/sky130A/libs.tech/` (volare build fa87f8f4). Reference texts are cited as
`<file>.txt L<a>–<b>` in the extracted reftext directory, with the PDF page.

---

## 0. Scope — files read

Read completely, every line:

| File | Lines |
|---|---|
| backend/verify/Cargo.toml | 10 |
| backend/verify/build.rs | 23 |
| backend/verify/src/lib.rs | 531 |
| backend/verify/src/checker.rs | 355 |
| backend/verify/src/pdk.rs | 1428 |
| backend/verify/src/reference.rs | 174 |
| backend/verify/src/geom.rs | 71 |
| backend/verify/src/netlist.rs (in `src/**`, not in the task list) | 53 |
| pdks/sky130.json | 177 |
| GP/pdks/sky130.deck (the deck `sky130.json:2` names; every rule, layer, device and PEX value) | 670 |
| benchmarks/Cargo.toml | 25 |
| benchmarks/README.md | 109 |
| benchmarks/src/bench.rs | 507 |
| benchmarks/src/fixtures.rs | 909 |
| benchmarks/src/gds2svg.rs | 19 |
| benchmarks/tests/signoff_fixtures.rs | 195 |
| benchmarks/xcheck.py | 333 |
| benchmarks/xcheck_lvs.py | 165 |
| benchmarks/xcheck_pex.py | 101 |
| benchmarks/xcheck_selftest.py | 184 |
| benchmarks/characterize_mismatch.py | 146 |
| benchmarks/examples/drc_gds.rs, net_dump.rs | 109, 76 |
| benchmarks/fixtures/*.spice (10) and *.interface.json (2) | 151 total |

Skimmed for structure (top-level keys, `cell.layers` role map, scalar key set, deck name) with a JSON dump:
pdks/gf180mcu.json (111), pdks/ihp_sg13g2.json (111), pdks/generic_finfet.json (95). Rule-kind census of
`GP/pdks/sky130.deck` by grep; headers of the other three decks (`GP/pdks/{gf180mcu,ihp_sg13g2,generic_finfet}.deck`,
655/810/436 lines).

Read in part, to trace behaviour verify depends on (ranges given where cited): `GP/crates/extract/src/analytical.rs`
1–430 of 725 (the whole capacitance/resistance model); `GP/crates/extract/src/network.rs` 75–132;
`GP/crates/ingest/src/deck/kinds.rs` 200–614 (deck keyword → rule kind table); `GP/crates/check/src/lvs/compare.rs`
1–47; `GP/crates/check/src/erc/ruleset.rs` 20–45; `GP/crates/check/src/drc/ruleset.rs` 915;
`GP/crates/check/src/erc/rules/antenna.rs` 584–790 (density windows); `GP/crates/check/src/erc/power.rs` 700–840
(EM/IR current distribution). Callers of verify: `frontend/library/src/lib.rs` 918–957 (epoch key), 1283–1405
(`signoff`, `signoff_inputs`, `labels_and_reference`, `labeled_pins`); `frontend/library/src/cellgen.rs` 836–933
(`reference`, `dummy_cards`); `frontend/library/src/elaborate.rs` 255–319 (`stack`, `intent`);
`frontend/library/src/perf.rs` 1–40; `kernel/analog/src/routing/antenna.rs` 1–67;
`kernel/visualizer/src/lib.rs` 56–67. Local PDK files: `VOL/magic/sky130A.tech` 4160–4170, 4876–4916;
directory listings of `VOL/klayout/{drc,lvs}` and `VOL/netgen`.

`benchmarks/competition/` (git submodules, ALIGN-public @ e392ae4, MAGICAL-CIRCUITS @ b2d7ed1) is third-party;
inventoried, not read line by line: ALIGN has 41 example dirs, 38 `*.const.json` files, 5
`*.scaled_placement_verilog.json` (ALIGN's own placements), 3 GDS, and `regression_summaries/*.csv` with columns
`name, failed before pnr, failed during pnr, open, short, minlen, terminal, other, w, h, area, aspect` (units not
given). Constraint census over the 38 const files: PowerPorts 28, GroundPorts 27, GroupBlocks 8,
ConfigureCompiler 7, Order 6, ClockPorts 4, HorizontalDistance 4, VerticalDistance 4, SymmetricNets 3,
CompactPlacement 2, GroupCaps 2, AspectRatio 2, Align 2, DoNotUseLib 2, SymmetricBlocks 1, ChargeFlow 1,
GuardRing 1 — i.e. most files declare only supply ports. MAGICAL has ADC, Comparator, OTA, Vcm_Buffer,
Vrefp_buffer; each directory mixes circuit netlists with `*_test.sp` testbenches and `*_test.ocn` scripts.

Tools on this machine: `ngspice` (nix profile) and `nix` 2.34.7; volare sky130A/B (fa87f8f4) and gf180mcuA–D.
**Absent: `klayout`, `magic`, `netgen`.** The foundry decks those tools need are installed:
`VOL/klayout/drc/sky130A_mr.drc`, `VOL/klayout/lvs/sky130.lvs`, `VOL/netgen/sky130A_setup.tcl`,
`VOL/magic/sky130A.tech`.

---

## 1. Architecture & data flow

### 1.1 What `verify` is

`verify` is an adapter over GPurify (crate `gpurify`, imported as `gdsverify`). It implements no checking
algorithm. It does four things:

1. **Process schema (`Pdk`, pdk.rs).** `Pdk::from_json(sidecar)` reads `{"deck": "<name>.deck", "cell": {…}}`
   (pdk.rs:73–89) and loads the deck text from GPurify's `pdks/` directory, whose absolute path is baked in at
   compile time (`option_env!("GPURIFY_PDKS")`, pdk.rs:86; set by build.rs:7–23 from `$GPURIFY_DIR` or the cargo
   git checkout for the locked rev). It parses the deck on a 1 nm grid (pdk.rs:16–18, 99) and builds the
   generator-facing `rules: Vec<(String, i32)>` in three layers, last wins: every rule's `limit`/`min_one_side`
   length (pdk.rs:1015–1029), synthesised `<layer>_min_width/_min_spacing` and role aliases incl.
   `<role>_min_area` as a square side (pdk.rs:148–171), then integer/boolean `cell.*` scalars (pdk.rs:172,
   1259–1267). `validate()` (pdk.rs:180–304) rejects a deck missing a mandatory role (`REQUIRED_ROLES`
   pdk.rs:1032–1034), a `REQUIRED_RULES` key (pdk.rs:1039–1071), stack continuity, or min width/spacing on a
   routing layer. `impl Process for Pdk` (pdk.rs:928–1008) answers generator queries (`space`, `width`,
   `enclosure`, `endcap`, `extension`, `eol_space`, `space_between`, `sheet_ohm`, `area`) from deck rules through
   `widest_on` (pdk.rs:798–817), which only accepts rule layers derivable from Philis-drawn layers
   (`drawn_from`/`philis_drawn`/`reaches`, pdk.rs:752–790). Extra readers: `em_limit` (525–559), `p2p_max_ohm`
   (564–570), `antenna_rule`/`antenna_max_ratio` (575–614), `density_rules` (619–637), `diode_marker` (643–656),
   `pex_f32` (680–693), `wire_af_per_um` (698–704), `lateral_af_per_um` (715–719), `min_channel` (831–857),
   `recipe`/`Overlay` (1129–1201).
2. **Geometry load (geom.rs).** Each `pnr_core::Shape` rectangle is pushed as one GPurify polygon, not merged
   (geom.rs:39–42); pin labels go on the deck label layer bound to the drawn conductor (geom.rs:43–47,
   `label_layer` 61–71); derived layers are computed and labels resolved, failing closed on a label over no
   conductor (geom.rs:48–53).
3. **Schematic reference (reference.rs).** `RefInput` → GPurify SoA `Netlist` (reference.rs:61–133). A device
   whose kind/polarity has no deck recogniser is **skipped and only counted** (reference.rs:92–97); terminals are
   truncated to the recogniser's arity (reference.rs:98–114), so a 4-terminal MOS card loses its bulk.
4. **Session (`Checker`) and signoff (lib.rs).** `Checker::new` re-parses the deck into its own string table,
   optionally drops kind `density` (checker.rs:43–47), removes sidecar waivers and `global_density` rules as
   deferred (checker.rs:50–67). `run` = load geometry → `extract` (nets, devices, ports) → `run_checks` with
   `CompareOptions::default()` (`param_tolerance 0.02`, GP/crates/check/src/lvs/compare.rs:26–33) and
   `quasistatic_inductance: false` (checker.rs:163–168) → drop `unconnected_pin`/`floating_gate` rows on any
   labelled net (checker.rs:176–200). `signoff_with_intent` (lib.rs:83–136) runs `Checks::ALL`, turns **every**
   row, error or warning, into a hard `Violation{rule:"{domain}/{rule}:{layer}", margin: nm shortfall}`
   (lib.rs:150–163), adds `engine/{stage}` rows for a skipped/refused stage (lib.rs:164–175), and sets
   `Report.cost = total_cap_ff()` (lib.rs:182). `drc`/`erc` (lib.rs:210–251) are standalone probes returning
   located `Finding`s. `extract_spice` (netlist.rs:25–53) writes the extracted circuit, optionally with PEX R/C.

How the library calls it (`frontend/library/src/lib.rs:1289–1370`): all drawn shapes; one label per net that has
a placed pin on a label-capable layer (`labeled_pins`, lib.rs:1372–1405); every label is also declared an LVS
port (lib.rs:1368); the reference comes from `cellgen::reference`, which **drops every capacitor and inductor
before verify sees them** (cellgen.rs:870) and passes W/L only for MOS (cellgen.rs:879–887; BJTs get unit cards
without params, cellgen.rs:888–890; R and D get nothing); dummy-gate cards are appended
(`dummy_cards`, cellgen.rs:912–933); `sol.intent` holds supplies plus one DC current per **Supply/Ground-class
net only** (elaborate.rs:297–319), empty without an operating point (elaborate.rs:304).

The epoch key that consumes signoff is `(|V_place|+|V_route|+|signoff.hard_violations|, perf residual, Θ, C,
footprint)` with C within 2 % treated as a tie broken by footprint (`key_lt`/`lex_key`, lib.rs:918–957).
`signoff.cost` is therefore the fourth ranking key.

### 1.2 What the engine computes, as configured

- **DRC.** GPurify has 35 DRC kinds (GP/crates/check/src/drc/ruleset.rs:915). The deck keyword maps to a kind in
  GP/crates/ingest/src/deck/kinds.rs:214–604 (e.g. `space`→`min_spacing`, `enclosure(inner, outer)`→
  `min_enclosure` with layers swapped to `[outer, inner]`, `tap_distance`→`max_distance_to_tap`). sky130.deck
  has 255 `rule` statements (267 after the 4-iteration metal loop at sky130.deck:360–366). Census by keyword:
  space 70, enclosure 50, width 26, forbidden 18, notch 16, antenna 13, area 11, size 7, hole_area 7,
  wide_space 5, em_current_density 5, density_cmp 4, inside 3, gate_oxide 3, extension 2, drain_source 2, and one
  each of length, off_grid, angle, contains, unconnected_pin, floating_well, supply_short, soft_connection,
  floating_gate, tie_high_low, ir_drop, multiple_drivers, esd_topological. Values come from the SkyWater
  periphery CSV and named secondary sources (sky130.deck:3–29).
- **ERC.** 24 kinds (GP/crates/check/src/erc/ruleset.rs:20–45). sky130 uses: 13 antenna rules, areal for cuts
  and sidewall for conductors, no diode credit (sky130.deck:451–463); 5 `em_current_density` (average DC at
  Tj 90 °C, sky130.deck:488–495); 4 `density_cmp` warnings, max 0.7 in 700 µm windows, `partial_windows: false`
  (sky130.deck:500–503); `gate_oxide`/`drain_source` (need per-net voltages, sky130.deck:482–486);
  `unconnected_pin` (x.22), `floating_well` (nwell.4, presence of a contacted tap only), `supply_short`,
  `soft_connection`, `floating_gate` with `labels_are_ports: false`, `tie_high_low` (warning), `ir_drop`,
  `multiple_drivers` (waived by the sidecar, sky130.json:173–175), `esd.pad` (sky130.deck:466–476).
  GPurify also offers `antenna_electrical` (diode credit), `esd_latchup`, `tap_distance` (DRC),
  `well_bias`, `reliability`, `p2p_resistance` (kinds.rs:326, 444–604); sky130 uses none of them
  (sky130.deck:663–670), while gf180 uses `tap_distance` (GP/pdks/gf180mcu.deck:303–306).
- **LVS.** Recogniser extraction (marker layer + terminal layers) and graph-refinement compare with
  series/parallel reduction, 2 % parameter tolerance, reference bulk dropped when the layout recogniser has none
  (GP/crates/check/src/lvs/compare.rs:1–47). sky130 MOS recognisers are 3-terminal `[poly_c, nsd, nsd]`
  (sky130.deck:542–550); resistor, diode (substrate anode), MiM and varactor recognisers exist
  (sky130.deck:551–570); **no BJT recogniser, no MOM/VPP recogniser**, n-well not a net, no MOS bulk
  (sky130.deck:663–668).
- **PEX** (GP/crates/extract/src/analytical.rs, "Closed-form extraction", line 1): per polygon, series
  `R = sheet·L/W` with L, W the sides of the rectangle of equal area and perimeter, polygons of a net chained
  node to node in PolyId order ("a comb's fingers come out in series; the net total is unchanged",
  analytical.rs:341–417); ground `C = area·area_cap + perimeter·fringe_cap` (analytical.rs:55–68); lateral
  coupling `C = ε0·k·t·L_facing/gap` between polygon **bounding boxes** that face each other within a halo of
  10× thickness (analytical.rs:22–23, 106–124, 181–222); inter-layer coupling `C = ε0·k/gap·overlap_area` for
  every overlapping pair of every conductor-layer pair, regardless of layers in between (analytical.rs:223–263);
  via `R = per_cut/count` (analytical.rs:266–340). The file states "Both coupling terms are parallel-plate lower
  bounds (no fringing)" (analytical.rs:84). Ground C is to substrate only and is not reduced by covering metal.
  Diffusion cap is zero by design (sky130.deck:572–581). `capacitance_per_net` adds every coupling element to
  both of its nets (GP/crates/extract/src/network.rs:83–106).

### 1.3 Benchmarks

`bench` (bench.rs) discovers fixtures (fixtures.rs:295–311), preprocesses SPICE (fixtures.rs:570–769:
backslash joins, `.param` resolution, bare R/C → PDK models sized by `cap_density_ff_um2` / recipe sheet R,
`nfin`→W, `l=0.15u` appended to every `m`/`x` card without `l`, MOS L/W raised to the deck minimum,
fixtures.rs:776–808), runs `library::run` with `feedback_iters = 5` and an op-point config when
`$PDK_ROOT|~/.volare/sky130A/libs.tech/ngspice/sky130.lib.spice` exists (bench.rs:142–151, 180), then
`library::signoff`, and prints one row per circuit: cells, nets, WL = Σ route length over all nets
(bench.rs:211–214), unrouted, overuse, DRC row count, LVS MATCH iff no `lvs/` row (bench.rs:196–203), ERC row
count, `C` = `Report.cost`, area = bbox of device half-extents (bench.rs:104–117), util, active diffusion %,
SA convergence, bias. A per-constraint table re-evaluates the run's own placement batches on the final layout
and a **fresh** default annotation's routing batches on the final routes (bench.rs:259–288, 321–365).
Artifacts: GDS, `signoff.txt`, `violations.txt`, `drc_located.txt` in `target/bench_debug/<name>/`, and an SVG in
`assets/` (gitignored) (bench.rs:290–314). `tests/signoff_fixtures.rs` asserts DRC/ERC ceilings and LVS clean on 7
fixtures with `feedback_iters = 1`; 3 OTA-class fixtures are `#[ignore]`d. The Python cross-checks
(`xcheck*.py`) compare GPurify with KLayout; see AV-02.

---

## Ground truth (run 2026-09-28 on this working tree, release profile)

All three commands were run by this auditor, in order, from the repository root. Output logs are in the session
scratchpad (`gt/build.log`, `gt/test.log`, `gt/bench.log`).

### G.1 `cargo build --release --workspace`

- Wall time **2.4 s**, exit 0, **0 warnings**, 0 errors. Incremental: dependencies and library crates were
  already built; only the `philis`, `op_demo`, `three_stage_opamp` binaries were recompiled. A cold build time
  was not measured.

### G.2 `cargo test --release --workspace --no-fail-fast`

- Wall time **157.1 s** (test binaries already compiled by an earlier session). Exit 101.
- **331 passed, 1 failed, 2 ignored** (sum over all `test result` lines).
- Per target (passed/failed/ignored): analog 61/0/0; annotator 32/0/0; bench bin 13/0/0; gds2svg 0;
  **benchmark `signoff_fixtures` 12/1/1** (10 of the 12 are `fixtures.rs` unit tests pulled in by `#[path]`);
  cells 37/0/0; cells `cell_selfcheck` 8/0/0; dp 27/0/0; dr 25/0/0; gp 4/0/0; gr 11/0/0; library 48/0/0;
  library integration: antenna_diode 1, elaborate 1, emit_roundtrip 2, extra_devices 1, flow_smoke 1,
  hier_elaborate 1, injected_macro 1, ota_cross_pdk 3, perf_postlayout 2 (109.7 s; runs ngspice),
  tmp_deck_drc 0/0/1; macro_master 8/0/0; pnr_core 11/0/0; verify 19/0/0; visualizer 2/0/0.
- **Failing test:** `benchmarks/tests/signoff_fixtures.rs::fixtures_sign_off_within_baseline`, panic at
  signoff_fixtures.rs:139: `dac4: ERC 1 exceeds baseline 0 … erc: erc/ar.met2.1:gate` (the met2 antenna rule,
  sky130.deck:457). The flow has an in-loop per-stage antenna model (elaborate.rs:266–291 →
  kernel/analog/src/routing/antenna.rs:58–66) and an antenna-diode insertion pass (elaborate.rs:321–333), yet
  signoff still finds a met2 antenna violation on dac4: the in-loop model and signoff disagree on this net.
  Per-fixture numbers printed by the test (DRC is `verify::drc` with no labels; ERC/LVS from `library::signoff`):

  | fixture | DRC (routing / cells) | ERC | PEX Σ C | LVS verdict | what LVS actually compared |
  |---|---|---|---|---|---|
  | pair | 0 (0/0) | 0 | 4.8 fF | clean | 2 MOS |
  | quad | 0 (0/0) | 0 | 10.6 fF | clean | 4 MOS |
  | rc_filter | 0 (0/0) | 0 | 27.8 fF | clean | 2 MOS + 1 resistor (no value) |
  | bjt_mirror | 0 (0/0) | 1 | 33.7 fF | "clean" | **nothing**: stderr `2 schematic device(s) have no deck recogniser; left out of LVS` |
  | bgr_core | 0 (0/0) | 1 | 81.6 fF | "clean" | **nothing**: `9 schematic device(s) have no deck recogniser` |
  | chain4 | 0 (0/0) | 0 | 24.1 fF | clean | 4 MOS |
  | dac4 | 0 (0/0) | **1** (`ar.met2.1`) | 326.0 fF | clean | 9 MOS; the 15 unit caps are removed by `cellgen::reference` **with no message at all** |

- Ignored: `large_fixtures_sign_off_within_baseline` (ota, ota_constrained, tt_ota) and `tmp_deck_drc::dump`. The
  OTA-class fixtures are never signed off by a default `cargo test`.

### G.3 `cargo run --release -p benchmark --bin bench local`

Invocation per benchmarks/README.md ("Run"). Wall time **539.5 s**, exit 0, seed 1, deck sky130 for every
circuit, `FEEDBACK_ITERS = 5`. Side effect: SVGs rewritten in `assets/` (gitignored, `.gitignore` "bench exports
SVGs here"). **10/10 circuits placed and routed.**

| circuit | ms | devices / nets | WL nm | unrouted | overuse | DRC | LVS | ERC | C (Σ, fF) | area µm² | util % | active % | SA best/iters | outer, esc | bias |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 16 113 | 2 / 3 | 260 320 | 0 | 0 | 0 | MATCH¹ | 1² | 77.2 | 135.0 | 100.0 | 1.2 | 4/5 budget | 1, 0 | none³ |
| bjt_mirror | 12 300 | 2 / 5 | 74 085 | 0 | 0 | 0 | MATCH¹ | 1² | 33.7 | 64.7 | 79.4 | 1.8 | 3/5 budget | 1, 0 | none³ |
| chain4 | 11 647 | 4 / 7 | 44 675 | 0 | 0 | 0 | MATCH | 0 | 22.6 | 62.6 | 100.0 | 19.4 | 1/5 conv. | 1, 0 | 0 µW probe |
| dac4 | 287 520 | 15⁴ / 12 | 580 410 | 0 | 18 | 0 | MATCH⁵ | 0 | 275.8 | 2326.9 | 60.0 | 0.8 | 5/15 budget | 3, 2 | 112 µW probe |
| ota | 52 197 | 5 / 9 | 444 720 | 0 | 0 | 0 | MATCH | 0 | 616.0 | 1865.1 | 85.9 | 43.1 | 1/20 conv. | 4, 3 | 2 µW probe |
| ota_constrained | 59 006 | 5 / 9 | 444 720 | 0 | 0 | 0 | MATCH | 0 | 616.0 | 1865.1 | 85.9 | 43.1 | 1/20 conv. | 4, 3 | 2 µW probe |
| pair | 11 468 | 2 / 3 | 17 700 | 0 | 0 | 0 | MATCH | 0 | 4.6 | 8.6 | 100.0 | 48.4 | 4/5 conv. | 1, 0 | 0 µW probe |
| quad | 16 803 | 4 / 3 | 37 945 | 0 | 0 | 0 | MATCH | 0 | 9.3 | 15.9 | 100.0 | 51.2 | 5/5 conv. | 1, 0 | 0 µW probe |
| rc_filter | 13 445 | 3 / 5 | 86 835 | 0 | 0 | 0 | MATCH | 0 | 24.4 | 131.7 | 76.9 | 3.5 | 5/5 conv. | 1, 0 | 28 µW probe |
| tt_ota | 57 522 | 5 / 9 | 444 720 | 0 | 0 | 0 | MATCH | 0 | 616.0 | 1865.1 | 85.9 | 43.1 | 1/20 conv. | 4, 3 | 2 µW probe |

¹ Nothing compared: stderr prints `2 schematic device(s) have no deck recogniser` (bjt_mirror) and `9 …`
(bgr_core) 16 times each over the run. ² Both ERC rows are `erc/supply_short:met1`
(`target/bench_debug/*/violations.txt`), the GPurify false positive the test baseline documents
(signoff_fixtures.rs:22–29). ³ `[op] operating point unavailable (no device operating points: )` for both BJT
circuits: EM/IR never armed. ⁴ 14 schematic devices plus flow-inserted antenna diode(s) (2 `areaid_diode`
shapes in `dac4.gds`); with 5 feedback iterations dac4 is antenna-clean, with 1 (the test) it is not (G.2).
⁵ The 15 unit capacitors are not in the reference. ota, ota_constrained and tt_ota are **bit-identical**
(same netlist, same seed, interface contracts ignored; the tt_ota GDS has no met4/met5 shape although its
contract puts pins on met4/met5, `tt_ota.interface.json:4–10`).

Skip log (printed once per process, for the first circuit, bgr_core): `ir_drop`, `esd.pad`, `vgs.01v8`,
`vds.01v8`, `vgs.g5v0`, `vgs.esd_nfet`, `vds.d10v5`, `EM.met1_mcon` … `EM.met5_via4` → `Skipped(NoDesignIntent)`;
`lvs.device_count_mos`, `lvs.device_count_bjt`, `lvs.parametric` → `Skipped(NotInDeck)`; `multiple_drivers` →
`Waived(…)`. The four `m*.density` rules are **not** in the list: they "ran" over zero whole windows (AV-18).
Whether EM ran for the circuits that had a probe bias is not printed anywhere (AV-26).

Constraint satisfaction table (self-graded, bench.rs:259–288), verbatim totals:

| type | arm | total | sat | viol | unk | max use (at) | Θ |
|---|---|---|---|---|---|---|---|
| Antenna | hard | 21 | 21 | 0 | 0 | 0.143 (dac4) | 0 |
| CommonCentroid | budget | 3 | 3 | 0 | 0 | 0.000 (tt_ota) | 0 |
| CommonNode | budget | 6 | 6 | 0 | 0 | 0.000 | 0 |
| CouplingBudget | budget | 39 | 39 | 0 | 0 | 0.535 (dac4) | 0 |
| CrosstalkExclusion | budget | 12 | 12 | 0 | 0 | 0.235 (tt_ota) | 0 |
| Differential | hard | 3 | 3 | 0 | 0 | – | 0 |
| Electromigration | hard | 46 | 44 | 0 | 2 | – | 0 |
| Environment | budget | 6 | 6 | 0 | 0 | 0.320 (tt_ota) | 0 |
| IrDrop | budget | 24 | 24 | 0 | 0 | 0.136 (dac4) | 0 |
| MatchingPair | budget | 6 | 6 | 0 | 0 | 0.000 | 0 |
| ParasiticBudget | budget | 39 | 38 | 1 | 0 | 1.016 (dac4) | 0.016 |
| Proximity | budget | 11 | 11 | 0 | 0 | 0.255 (tt_ota) | 0 |
| Symmetry | hard | 9 | 9 | 0 | 0 | – | 0 |
| ThermalGradient | budget | 6 | 6 | 0 | 0 | 0.000 | 0 |
| Utilization | budget | 10 | 9 | 1 | 0 | 1.001 (dac4) | 0.001 |
| **OVERALL** | | 241 | 237 | 2 | 2 | | rate 99 % |

Every matching/symmetry/centroid row comes from the three identical OTAs (tt_ota is named as the worst only
because it is last); the constraint coverage of the suite is effectively one 5-transistor OTA. The dac4
common-centroid cap array is not a graded constraint row at all.

### G.4 Supplementary measurements (scratch probes outside the repository)

Two read-only probes, run by this auditor from the session scratchpad (no repository file created or changed;
the Rust probe was a separate crate with path dependencies on `library`/`verify`, built into `target/` with
a copy of the repository's `Cargo.lock`, same GPurify pin).

**(a) What `Report.cost` is made of** (`library::run` with bench's config, then `verify::signoff_with_intent`
and its `CapMatrix`; totals reproduce the bench `C` exactly):

| circuit | Report.cost | ground, signal nets | ground, supply nets | coupling signal–signal (once) | coupling touching a supply (once) | largest single term |
|---|---|---|---|---|---|---|
| pair | 4.6 | 2.7 | 1.2 | 0.0 | 0.3 | g ground 1.68 |
| ota | 616.0 | 116.5 | 200.4 | 5.4 | 144.1 | VSS–vbn coupling 89.38 (counted twice: 178.8 = 29 % of the total) |
| dac4 | 275.8 | 70.8 | 115.4 | 16.1 | 28.7 | VSS ground 60.04 |

On ota, supply-related terms are 200.4 + 2×144.1 = 488.6 fF of 616.0 (79 %); the performance-critical nets
carry vinp 5.26/6.18, vinm 5.25/6.23, vout1 4.61/6.11, vout2 4.57/5.62 fF (ground/coupling). The quantity a
designer checks — output-pair coupling mismatch, 6.11 vs 5.62 fF (0.49 fF, 8 %) — is not reported anywhere,
while the bias-to-rail coupling vbn–VSS (a decoupling capacitance a designer would keep) dominates the ranking
key. On dac4 the extracted top-plate couplings are top–b0 0.42, top–b1 0.85, top–b2 1.67, top–b3 3.36 fF
(ratio 1 : 2.02 : 3.98 : 8.0 — the binary ratio holds within 2 %), top–VSS 1.46 fF (C0 plus routing), while the
top plate's ground capacitance is 12.94 fF, about twice the whole extracted array. 0.42 fF per 2 × 2 µm unit is
0.105 fF/µm²; the sidecar's `cap_density_ff_um2 = 2.0` (sky130.json:8), which the bench preprocessor uses to
size capacitors (fixtures.rs:556, 720–722), is 19× that. Either the MOM generator or the fringe-free lateral
PEX (or both) is off by an order of magnitude; nothing in the repository detects it (AV-32).

**(b) Unmerged-rectangle bookkeeping** (Python over `target/bench_debug/*/*.gds`, met1–met5 only, sky130.deck:590–598
area/fringe coefficients; "unmerged" = Σ per rectangle as GPurify stores them, "merged" = union per layer):

| circuit | rects | ground C unmerged fF | ground C merged fF | ratio |
|---|---|---|---|---|
| bgr_core | 226 | 28.02 | 17.37 | 1.61 |
| bjt_mirror | 77 | 8.09 | 5.47 | 1.48 |
| chain4 | 34 | 4.79 | 3.35 | 1.43 |
| dac4 | 450 | 122.94 | 108.88 | 1.13 |
| ota (= ota_constrained = tt_ota) | 243 | 44.30 | 33.74 | 1.31 |
| pair | 25 | 2.00 | 1.16 | 1.73 |
| quad | 45 | 4.17 | 2.45 | 1.70 |
| rc_filter | 75 | 9.36 | 5.66 | 1.66 |

The metal ground term is overstated by 13–73 % from bookkeeping alone, before any modelling error; the error
is largest on small cells, i.e. it is not a constant factor that ranking can ignore (AV-07).

---

## 2. Module-by-module findings

### 2.1 backend/verify/src/lib.rs (531 lines)

What it does: public API (`signoff`, `signoff_with_caps`, `signoff_with_intent`, `drc`, `erc`, `shortfall_nm`,
`Intent`, `CapMatrix`, `Finding`) and 10 unit tests.

- **Warnings are hard violations** (lib.rs:39–41, `harvest` 150–163 ignores `Severity`). sky130 declares
  `tie_high_low` and the four `density_cmp` rules as warnings (sky130.deck:471, 500–503). The epoch key counts
  `signoff.hard_violations.len()` first (frontend/library/src/lib.rs:952), so a warning weighs the same as a
  short. (AV-10)
- **`Report.cost` is Σ over all nets of ground + coupling C** (`total_cap_ff`, lib.rs:182 → checker.rs:317–320
  → GP network.rs:83–106), each coupling element counted on both nets, supply rails included. It is the fourth
  epoch key (frontend/library/src/lib.rs:955). It rewards removing metal anywhere, including shields and rail
  width, and says nothing about which node carries the C. (AV-06)
- **Label-short fallback** (lib.rs:118–126) re-runs label-free and harvests everything: LVS is then compared
  without port names and its rows are added after the `lvs/extract: label short…` row, and `caps` stays empty,
  so a downstream perf evaluation gets no capacitance on exactly those layouts. (AV-25)
- **Skip log once per process** (lib.rs:176–181, `static Once`). In a bench run the skip list printed belongs to
  the first circuit only (G.3: it lists EM/IR/vgs/vds as `Skipped(NoDesignIntent)` for `bgr_core`, which had no
  operating point); whether EM ran on later circuits is not visible anywhere. (AV-26)
- The reference-skip count is only `eprintln!`ed (lib.rs:111–116), never a violation or a report field. (AV-01)
- `defer_chip_level` (lib.rs:140–148) uses the shapes' union bbox — correct, but it only acts on kind `density`
  (see 2.2).
- `shortfall_nm` (lib.rs:29–34) returns 1 for every non-length measurement (ratios, counts): an antenna overshoot
  of 2× and of 1.01× both carry margin 1, so margins are not comparable across rule kinds.

Tests (what they assert): `unconnected_metal_is_floating_unless_it_is_a_port` (271–283, one met1 plate → one
`x.22`; labelled → 0); `shortfall_is_nm_for_lengths_and_boolean_otherwise` (287–294);
`checker_reports_a_min_width_shortfall_in_nm` (298–316, 100 nm li → `li.1`, margin 70, domain drc);
`intent_arms_the_em_rules` (321–341, ihp EM rules stop skipping once intent is set — asserts neither pass nor
fail); `density_wider_than_the_block_is_chip_level` (347–357, ihp AFil.g2); `checker_runs_clean_on_legal_geometry_and_proves_it_ran`
(362–372); `reference_builder_compiles_two_devices` (380–424, CSR layout; NPN skipped → count 1);
`a_wrong_reference_width_is_a_parameter_mismatch_and_the_right_one_matches` (432–475, parametric LVS works);
`device_count_sees_one_diode_under_a_diom_marker` (481–510); `device_count_sees_one_mos_in_a_minimal_stack`
(516–528). Missing: any test that a skipped reference device changes the verdict; warning vs error handling;
the label-short path; any PEX number against an external reference; LVS with a wrong body tie (undetectable
today, AV-05).

### 2.2 backend/verify/src/checker.rs (355 lines)

- **Floating-gate / unconnected-pin exemption is global in practice (AV-04).** `drop_port_floating`
  (checker.rs:176–200) drops every `unconnected_pin`/`floating_gate` row on a net with a port name. The library
  names (and declares as a port) every net with a placed pin on a label layer (frontend/library/src/lib.rs:1368,
  1372–1405), so internal nets are exempt too. The deck explicitly configures the opposite for this rule:
  `rule floating_gate floating_gate(; labels_are_ports: false)` (sky130.deck:470). Concrete case: `ota.spice:5–7`
  gates M3/M4 on `vbias` and M5 on `vbn`, which no device drives and no `.subckt` port names (ota.spice:2); the
  flagship OTA benchmark reports ERC 0 (G.3).
- `set_intent` (checker.rs:101–133): a ground net is put in the highest power domain (checker.rs:108–112);
  currents are one number per net (`budget_current_ua`, checker.rs:123–128). GPurify spreads that number
  uniformly over the rail's device terminals and solves the supply grid anchored at one pad
  (GP/crates/check/src/erc/power.rs:721–727: "a hot spot reads cooler than it is … over-reports drop"). Only
  declared supplies get grid nodes (power.rs:721–722), and the library only declares Supply/Ground nets
  (elaborate.rs:306), so **signal nets are never EM-checked**. (AV-09)
- `run` hard-codes `CompareOptions::default()` (2 %) and `quasistatic_inductance: false` (checker.rs:163–168); no
  caller can tighten parameter tolerance or ask for inductance.
- `defer_density_wider_than` (checker.rs:262–281) and the `strip_density` branch (checker.rs:43–47) match kind
  `density` only. sky130 states density as `density_cmp` (an ERC kind, sky130.deck:500–503), which is neither
  stripped nor deferred. GPurify then runs it with `partial_windows: false` on a block far smaller than one
  700 µm window, examines zero windows, and records `Outcome::Ran`
  (GP/crates/check/src/erc/rules/antenna.rs:698–704, 790): a vacuous pass reported as "ran", which the skipped-rule
  list (checker.rs:248–256) cannot show. (AV-18)
- `cap_matrix` (checker.rs:288–313) drops every element touching an unlabelled net and every coupling whose ends
  share a name; with near-total labelling this loses little, but an unlabelled net's C silently vanishes from
  the perf model.
- `domain_of` (checker.rs:340–354) is a linear scan per violation; negligible at current sizes.
Tests: exercised only through lib.rs tests.

### 2.3 backend/verify/src/reference.rs (174 lines)

- **Skip-and-count semantics (reference.rs:11–14, 92–97) are the root of AV-01.** A device the deck cannot
  extract disappears from both sides and signoff reports a match. On sky130 that is every BJT
  (sky130.deck:663–665), and — by the caller's choice (cellgen.rs:870) — every capacitor and inductor, which
  never even reach the counter. The docstring's reason (MOM combs are many polygons, reference.rs:13–14) is a
  recogniser limitation, not a verification result.
- `recogniser_for` (reference.rs:138–174): for a kind with no model hint it returns the first row of that kind
  (reference.rs:162). For `RefKind::Capacitor` on sky130 that is `capm` (sky130.deck:565), so a MOM cap card
  would be compared as a MiM if the caller stopped filtering capacitors. Polarity is read off the marker's first
  letter (reference.rs:155–157), a naming convention, not deck data. The fallback (reference.rs:165–171) accepts
  any row whose marker starts with `n`/`p` when the hint names no deck model — a wrong model silently matches
  another model's recogniser.
- Params are whatever the caller passes; the library passes W/L for MOS only (cellgen.rs:879–887): resistor
  value/W/L, diode area and BJT area are never compared. (AV-19)
- Ports = all labelled nets, which is what disables the floating ERC (AV-04).
Tests: lib.rs:380–424.

### 2.4 backend/verify/src/geom.rs (71 lines)

- **No union of same-layer shapes** (geom.rs:39–42; GPurify's store builder does not merge). met1–met5 are
  directly conductors (sky130.deck:506), so their ground C double-counts every overlap (wire over via pad, wire
  over wire, cell metal under a route) and counts internal abutting edges as fringe perimeter; lateral coupling
  is computed per rectangle pair by bbox (analytical.rs:106–124). `li`/`poly` go through a boolean
  (`li_c = li not li_rs`, sky130.deck:106–107), so the size of the effect differs by layer. G.4(b) measures the metal part: 1.13–1.73× on the fixtures.
  (AV-07)
- `label_layer` (geom.rs:61–71) accepts a label row whose conductor has `drawn` among its `operands()`, which
  includes the subtracted operand of a `not` (`li_c = li not li_rs`): a pin on `li_rs` would resolve to the
  `li_c` label. Harmless today (no generator labels `li_rs`).
Tests: indirect.

### 2.5 backend/verify/src/pdk.rs (1428 lines)

- **Deck path baked at compile time** (pdk.rs:86; build.rs:7–23). A moved `philis` binary, or one run after
  `~/.cargo/git/checkouts` is cleaned, cannot load any PDK; the override `GPURIFY_DIR` is read at build time
  only. (AV-21)
- **`pex_row` follows subtracted operands (AV-16).** pdk.rs:661–674 returns the row of any declared layer whose
  `operands()` contain the queried layer. For role `rpoly` → `poly_rs` (sky130.json:12) the only declared row
  with `poly_rs` as operand is `poly_c = poly not poly_rs` (sky130.deck:106, 582), so `sheet_ohm("rpoly")`
  returns 48.2 Ω/□, the **complement** of the resistor body. `reaches` (pdk.rs:758–770) already implements the
  kept-operand rule; `pex_row` does not use it. For `licon`, five declared derived cuts qualify
  (`licon_nd/pd/nt/pt/po`, sky130.deck:583–587, 185–585 Ω/cut) and `max_by_key` (pdk.rs:672) returns the last of
  equals — an arbitrary pick. The deck has no `pex` row for any `rbody_*` layer, so no correct value exists to
  return. CRATES.md open issue 6 (docs/CRATES.md:101–104) states the high-po body "should be about 319 Ω/sq, not
  48.2" (source not given; no text in `docs/ref/` states it).
- **Antenna.** `antenna_max_ratio` (pdk.rs:575–577) takes `min` over routing metals of `antenna_rule(l).0`,
  mixing areal and sidewall ratios and dropping the thickness; on sky130 that yields 75 (li sidewall,
  sky130.deck:453). It is only the fallback of `routing::Antenna` when no per-stage stack is present
  (kernel/analog/src/routing/antenna.rs:58–66); the flow passes the stack (elaborate.rs:266–291), so impact is
  limited. **Deck error:** `ar.met3.1 … sidewall: 2000nm` (sky130.deck:459) contradicts the same deck's
  `pex met3 thickness 845nm` (sky130.deck:594) and magic's `height allm3 2.7861 0.845` (VOL/magic/sky130A.tech:4890);
  met3 sidewall antenna area is overstated 2.37×. The sidecar's `antenna_source` text (sky130.json:100) says
  sidewall rules are "kept out of deck rules because gpurify refuses sidewall antenna rules", which the pinned
  deck contradicts (sky130.deck:451–463); `cell.antenna_sidewall` (sky130.json:101–126) is dead whenever the
  deck has a rule (pdk.rs:587). The foundry antenna rows carry diode-credit terms (`antenna m1,m2,m3 sidewall
  400 2600 400`, VOL/magic/sky130A.tech:4912) that the deck drops ("Diode PWL terms not encoded",
  sky130.json:100), although GPurify has `antenna_electrical` with `diode_credit` (kinds.rs:444–455). (AV-17)
- **Density readers see kind `density` only** (pdk.rs:619–637); sky130's density rules are `density_cmp`.
  `density_rules` has no caller outside verify's own test (grep over backend/frontend/kernel). (AV-18)
- `parse_roles` keeps integer and boolean scalars only (pdk.rs:1263); float keys (`res_corner_squares 0.56`,
  `res_serpentine_aspect 10.0`, `bjt_base_frac 0.3`, `bjt_collector_frac 0.5`, `cap_density_ff_um2 2.0`,
  sky130.json:8, 47–48, 53–54) never enter `rules`; hence the `_permille` duplicates (sky130.json:76–77) that
  `REQUIRED_RULES` demands (pdk.rs:1040–1041). Array-valued keys (`wpe_clearance_nm`, `lod_moat_ext_nm`,
  sky130.json:59–63, 70–73) are also dropped. (AV-22)
- `REQUIRED_RULES` (pdk.rs:1039–1071) checks presence, not provenance: `lod_moat_ext_moderate 3000`,
  `wpe_clearance_moderate 3000`, `tie_max_dist_nm 3000`, `p_epi_thickness 3000`, `n_well_depth 2000`,
  `p_well_depth 1500`, `retrograde_pwell false` (sky130.json:66–86) carry no `_source`, unlike `avt_*`, `svt_*`,
  `vt_tc_*`, `em_*`, `antenna_*`, `density_*`, `lod_kvth0_*` (sky130.json:90–131). Hastings §13.3 rule 19
  (hastings.txt L42630–42636, PDF p716, printed p715; numbers read from the PDF page because the text extraction
  drops them) gives, in WPE-prone processes, ≥3 µm well-edge clearance for moderately and ≥2 µm for minimally
  matched devices, and 5–10 µm or twice the well junction depth for exceptionally matched ones; the sidecar's
  `[2000, 3000, 5000]` is consistent with that but uncited. `tie_max_dist_nm 3000` is 5× tighter than the
  foundry latch-up rule LU.2/LU.3 (< 15.0 µm, VOL/magic/sky130A.tech:4165–4170) and is never checked.
- `MAX_CELL_FEATURE_NM = 5000` (pdk.rs:1207) decides which wide-metal spacings `space()` applies to cells
  (pdk.rs:974). sky130's wide-metal threshold is 3 µm (sky130.deck:365), below the bound, so today it binds; the
  constant is a Philis design bound, not process data.
- `lateral_af_per_um` (pdk.rs:715–719) duplicates GPurify's fringe-free `ε0·k·t/gap` term for the router's
  crosstalk budget; consistent with signoff, and inherits its underestimate.
- `em_limit` (pdk.rs:525–559) separates per-µm and per-cut limits correctly; `derating` is `None` on sky130
  (no `reference_temperature`/`activation_energy` params, sky130.deck:491–495), so no Black's-law temperature
  scaling (Lienig §3.4.2 eq. 3.8, lienig_em.txt L4247–4262, PDF p93) is possible; li has no limit
  (sky130.json:99).
- Empty tables: `"sheet_tolerance": {}`, `"linewidth_control_nm": {}`, `"dti": null` (sky130.json:46, 74–75).
Tests (pdk.rs:1287–1427): `min_channel_reads_the_gate_width_rule`, `routing_stack_is_metal_only`,
`via_stack_matches_metal_stack`, `routing_pitch_clears_every_layer_of_its_stack`,
`an_unreachable_layer_does_not_set_the_pitch`, `a_diode_marker_only_where_lvs_can_extract_it`,
`role_min_area_is_the_decks_side`, `antenna_rules_are_read_per_stage` (sky met1 = (400, 350)),
`em_limits_are_read_per_layer_from_the_deck_rules`. Untested: `pex_row`/`sheet_ohm` per role, `density_rules`,
`wide_spacing`, `space_between`, `recipe`, `Overlay`.

### 2.6 backend/verify/src/netlist.rs (53 lines)

`extract_spice` writes the extraction as SPICE; with `Detail::WithParasitics` it splices GPurify's R/C, whose R is
a per-polygon chain in PolyId order (analytical.rs:341–345), not a physical RC tree — terminal-to-terminal
resistances in such a netlist are wrong. The library's perf loop does not use it; it builds per-terminal branch
R from routes plus the cap matrix and a LOD term (frontend/library/src/perf.rs:17–27). Callers:
frontend/library/src/elaborate.rs:82, kernel/cells/src/mosfet.rs:841, 963 (schematic detail only).

### 2.7 pdks/sky130.json (177 lines) and the GPurify decks

- The sidecar holds roles (sky130.json:10–38), generator dimensions, mismatch/tempco/LOD coefficients with
  sources (`avt_*` from `characterize_mismatch.py` Monte Carlo, sky130.json:93–96; `vt_tc_*` 127–130;
  `lod_kvth0_*` 90–92), EM/antenna/density provenance (99–131), resistor recipes (137–170) and waivers (173–175).
  `svt_uv_per_um 1.63` (sky130.json:97–98) is a proxy from Schaper & Linnenbank Table I
  (distance_vs_pair_mismatch.txt L181–197): rows 0.24/1.60 → 0.780, 0.24/12.8 → 0.941, 0.48/1.60 → 0.582,
  0.48/3.20 → 0.644 match the sidecar; the SD entry of the 0.12/1.60 row (the 1.63 the sidecar uses) is not
  legible in the text extraction (not verified).
- What the sky130 deck **does** express: widths/spaces/notches/enclosures/extensions, wide-metal spacing,
  min area/hole area, presence rules (forbidden/inside/contains), butting edges, grid, angles, per-stage antenna
  without diode credit, EM average DC at Tj 90 °C, max metal density (warning, whole windows only), supply
  short, floating well (tap presence), gate-oxide/drain-source (voltage intent), ESD topology on `pad`.
- What it **does not** express (sky130.deck:638–670, "Rules waiting on a check kind" / "Left out"):
  - **Latch-up.** "esd_latchup needs a guard-ring width and tap distance that [S1] does not state"
    (sky130.deck:669–670). The number is in the installed foundry tech file: LU.2 N-diff to P-tap < 15.0 µm,
    LU.2.1 in deep n-well, LU.3 P-diff to N-tap < 15.0 µm (VOL/magic/sky130A.tech:4165–4170); GPurify's
    `tap_distance` kind can state it (gf180mcu.deck:303–306). The gap is deck data, not engine capability.
    (AV-08)
  - **LOD / WPE / STI stress** have no check kind. Hastings §13.2.2 (hastings.txt L41265–41436, PDF p695–698)
    describes WPE and the LOD effect and their fixes (well edges away from gates, dummies, stretched moats).
    Philis models LOD in perf (`lod_inv_um`, perf.rs:23–26) but signoff never compares SA/SB or well distance
    across matched devices.
  - **Wells as nets, MOS bulk** (sky130.deck:663–668); `well_bias` not configured. (AV-05)
  - Channel-width rules difftap.2/2b, gate length in hvi, poly corner rules, slotted licon spacing, exact
    resistor widths, net-aware spacing nwell.7/8, capm.7, rpm terminal counts (sky130.deck:639–658).
  - **Minimum density** and poly/diff density: none ("No minimum metal density found", sky130.json:131).
- PEX stack (sky130.deck:572–598): per-layer area and fringe to substrate only, no per-pair or per-spacing
  tables, diffusion C left to device models, and verify has no AS/AD/PS/PD back-annotation path.
- gf180/ihp/generic_finfet sidecars share the schema; gf180 and ihp add `min_gate_l`, `li_encloses_licon*`,
  `via_spacing`. `generic_finfet.deck` is ASAP7 (the predictive 7 nm PDK, GP/pdks/generic_finfet.deck:1–3), which
  bench uses for ALIGN/MAGICAL by default (bench.rs:54–56); ALIGN's own examples target its FinFET mock PDK, so
  numbers on ASAP7 are not comparable to ALIGN's published ones.

### 2.8 benchmarks/src/bench.rs (507 lines)

- Row metrics (bench.rs:193–257): DRC and ERC are **row counts**, not unique sites; antenna is ERC.
  **LVS MATCH whenever no `lvs/` row exists** (bench.rs:196–203), including runs where the reference skipped
  devices (AV-01). WL sums every net incl. supplies (bench.rs:211–214). `area` is the bbox of device
  half-extents (bench.rs:104–117), not of the emitted GDS; `util` counts ring halos as device area
  (bench.rs:119–121). `C` = `report.cost` (AV-06). (AV-14)
- Constraint table (bench.rs:259–288) grades the run's own `placement.hard/budget/cost` batches and routing
  batches from a **fresh** `annotator::annotate(… AnnotationConfig::default())` (bench.rs:269), not necessarily
  what the run optimised. It is self-referential: no external constraint ground truth.
- No post-layout performance column although `library::perf` exists and is tested (perf_postlayout, G.2);
  bias is a synthesised mid-rail probe, "not a sign-off condition" (bench.rs:140–141).
- One seed per invocation (bench.rs:392–395); the README warns results are seed-noisy; no statistics.
- `MAX_CELLS = 800` (bench.rs:32), `FEEDBACK_ITERS = 5` (bench.rs:37) vs the test's 1
  (signoff_fixtures.rs:78).
- Filter doc says "name substrings" (bench.rs:379), code matches exactly (bench.rs:390); README says exact.
- SVG layer names come from `visualizer::parse_layer_names(&sidecar)` (bench.rs:420), which reads a top-level
  `layers` object (kernel/visualizer/src/lib.rs:59) the sidecar no longer has: every layer renders as `L68/20`.
Tests: `unknown_rules_are_not_counted_as_satisfied`, `short_kind_trims_path`, `footprint_single_device`.

### 2.9 benchmarks/src/fixtures.rs (909 lines)

- `discover_magical` (fixtures.rs:158–194) takes every `*.sp` in a category, including the `*_test.sp`
  testbenches (inventory in §0). (AV-13)
- `ref_gds_path` is found for TinyTapeout repos (`gds/tt_*.gds`, fixtures.rs:233–247) and is
  `#[allow(dead_code)]` (fixtures.rs:53–54): the hand layouts are never compared. (AV-03)
- `discover_tinytapeout`: the pattern table's second field is ignored (`_ext`) and `pattern` is built and
  discarded (fixtures.rs:212–230); `n.starts_with("TT")` after `to_ascii_lowercase` is dead (fixtures.rs:205–206).
- `Suite::from_str` maps any unknown word to `All` (fixtures.rs:68–75): `bench lcoal` clones 14 repos over the
  network.
- `preprocess_spice` (fixtures.rs:570–769): appends the sky130 literal `l=0.15u` to **every** `m`/`x` card
  without `l` (fixtures.rs:596, 611–613) — BJTs (`bgr_core.spice:4–5`), cap instances, subckt calls — on every
  deck; `raise_channels` then fixes only MOS (fixtures.rs:776–808). `nf` (fingers) is read as a fin count when W
  is absent (fixtures.rs:603–609). The comment "Neither deck defines a capacitor recogniser"
  (fixtures.rs:553) is false for sky130 (MiM, varactor at sky130.deck:565–570). The test comment "The deck
  states no resistor body sheet R" (fixtures.rs:842) is not why the card stays: `Overlay.width("rpoly")` finds no
  `min_width` on `poly_rs` (the rule is on `poly_res`, sky130.deck:278), so `res_w = 0` (fixtures.rs:551–552),
  while `sheet_ohm` returns the wrong 48.2 (AV-16). One cap density for every cap model (fixtures.rs:556).
  (AV-23)
- Repo pinning: 40-hex commits, asserted by `fixture_revisions_are_immutable_commit_ids` (fixtures.rs:901–908).
Tests: string-level only (parse_si, fmt_um, R/C rewrite, MOS raising, is_numeric, backslash join, params, suite
filter, pinning); none checks discovery against the real competition trees.

### 2.10 benchmarks/tests/signoff_fixtures.rs (195 lines)

- Baseline (signoff_fixtures.rs:17–32) asserts `LVS clean = true` for `bjt_mirror` and `bgr_core`, which compare
  zero devices (G.2): vacuous. (AV-01)
- ERC 1 accepted for the BJT fixtures because of a GPurify `supply_short(ntap, ptap)` false positive
  (signoff_fixtures.rs:22–29): the baseline encodes a known engine bug.
- `origin()` (signoff_fixtures.rs:57–69) attributes a finding to "routing" when its layer is in
  `routing_metals ∪ routing_cuts`; on sky130 that stack starts at `li`/`mcon` (sky130.json:22–36), which cell
  generators also draw, so li/mcon findings are misattributed. (AV-28)
- DRC uses `verify::drc(&shapes, &[], …)` (no labels; fine for DRC), ERC/LVS use `library::signoff`.
- `cap > 0` (signoff_fixtures.rs:103–107) is the only PEX assertion.
- `an_undrawable_device_is_one_finding` (182–195) checks ASAP7 reports BJT/resistor as `cell/undrawable`.
- Red on the current tree: dac4 antenna (G.2). (AV-15)

### 2.11 Cross-check scripts and examples

- **The KLayout DRC and PEX differentials and the self-test cannot run against the current sidecar (AV-02).**
  `pdks/sky130.json` has only `deck`, `_about`, `cell`. `xcheck.py` reads `deck["layers"]`, `deck["derived"]`,
  `deck["rules"]` (xcheck.py:93, 104, 111) → `KeyError`; its kind names (`min_width`, `min_spacing`…,
  xcheck.py:44–58) and layer name `_via1` (xcheck.py:207; the deck calls it `via`) belong to the old JSON deck.
  `xcheck_pex.py` reads `deck["layers"]`, `deck["pex"]` (xcheck_pex.py:53–54) → `KeyError`.
  `examples/drc_gds.rs:81` does `table["layers"].as_object().expect("layers")` → panic, which fails
  `xcheck_selftest.py:148–156`; its expected ids (`li_min_width`, `met1_min_area`, `floating_interconnect`,
  xcheck_selftest.py:29–48) are the old names (the deck uses `li.1`, `m1.6`, `x.22`).
  `xcheck_lvs.py` does not read the sidecar (it only imports `xcheck.klayout_bin`) and would run given KLayout,
  but it is topology-only by construction: `tolerance(... :relative => 1.0)` on W and L (xcheck_lvs.py:79–82);
  its docstring says the bulk is dropped (xcheck_lvs.py:94–99) while the regex keeps all nodes
  (xcheck_lvs.py:103–105) and the layout side extracts `mos4` with bulk (xcheck_lvs.py:58–59), so it checks bulk
  that GPurify does not. CRATES.md notes it "was not re-run" (docs/CRATES.md:117). Independently, KLayout,
  magic and netgen are not installed, so no independent check has run here at all.
- `xcheck_pex.py` recomputes ground C from GDS bboxes with the same unmerged bookkeeping ("polygons are NOT
  unioned, matching the store", xcheck_pex.py:2–5) and accepts `ground ≤ total ≤ 4×ground` (xcheck_pex.py:92). It
  can catch a unit bug, not an accuracy error.
- `characterize_mismatch.py`: sound method — mismatch corner, one AGAUSS draw per instance, 250 pairs per
  geometry in chunks with distinct seeds (characterize_mismatch.py:86–98), constant-current VT, σ(ΔVT) fit
  through the origin vs 1/√(WL) (138), tempco by least squares over 0/27/85/125 °C (105–120). It is the
  provenance of `avt_*` and `vt_tc_*`. Gradient `S_VT` is not measurable from models (sky130.json:94).
- `examples/net_dump.rs` (76 lines): diagnostic; runs the flow and maps routed wires to extracted nets by exact
  bbox (net_dump.rs:151–177). `gds2svg.rs` (19): renders with the same empty layer-name table.

### 2.12 Fixtures (benchmarks/fixtures)

10 local circuits: pair (2 nfet), quad (4), chain4 (4), rc_filter (2 MOS + 1 `res_generic_po`), bjt_mirror (NPN
+ PNP), bgr_core (PNP 1:8, `m=8`), dac4 (5 cap cards = 15 units + 9 MOS), ota, ota_constrained, tt_ota.
**ota, ota_constrained and tt_ota are the same netlist** (ota.spice:2–7, ota_constrained.spice:2–7,
tt_ota.spice:4–9); their `*.interface.json` die/pin contracts are ignored (bench.rs:13). The OTA netlist gates on
undriven `vbias`/`vbn` (ota.spice:5–7) (AV-30). Largest circuit: 14 devices before m/nf expansion. Fixture
comments state known optima — pair (pair.spice:2), quad (quad.spice:2), chain4 "Optimal HPWL = … 6*pitch"
(chain4.spice:2–3), bgr_core "one 3x3 array, Q1's unit at the centre" (bgr_core.spice:1–2) — which no code
checks.

---

## 3. Gap analysis vs "best constraint-aware analog P&R that beats hand layout"

What a senior layout engineer or a state-of-the-art flow does at signoff and benchmarking time that Philis does
not:

1. **Foundry-grade signoff with an independent engine.** A hand sky130 layout is signed off with magic
   `drc(full)`, the foundry KLayout DRC and netgen LVS. Philis's only engine is GPurify on a hand-transcribed
   deck, and the independent differential is dead (AV-02). The foundry decks are already on disk
   (`VOL/klayout/drc/sky130A_mr.drc`, `VOL/klayout/lvs/sky130.lvs`, `VOL/netgen/sky130A_setup.tcl`,
   `VOL/magic/sky130A.tech`); only the binaries are missing, and `nix` is installed. Until a real-PDK run gates CI,
   "DRC 0" means "DRC 0 against Philis's reading of the manual". CRATES.md open issue 1 (docs/CRATES.md:96)
   records 15 magic violations on bjt_mirror that GPurify does not report.
2. **Complete LVS.** Every device, every terminal: bulk/well connectivity (AV-05), BJTs, MiM and MOM caps,
   resistor value, diode/BJT area, dummy devices, m/nf accounting. Philis compares MOS D/G/S with W/L at 2 % and
   drops the rest without a verdict change (AV-01, AV-19).
3. **Analog layout-rule verification on the final GDS.** The checks a reviewer applies to matched structures —
   Hastings §13.3 rules 17–23 (hastings.txt L42607–42654, PDF p716): no metal across the gates of moderately or
   exceptionally matched transistors and minimal metal within 5–10 µm; dummy-metal block layers over matched
   arrays extending 5–10 µm; well edges ≥3 µm (moderate) / ≥2 µm (minimal) in WPE-prone processes; gate
   geometries extended 1–2 µm beyond the rule; gate fingers strapped in metal; no stray poly within 5–10 µm
   (exceptional) or 3–5 µm (moderate); plus LOD/WPE equality across matched devices (§13.2.2, L41265–41436),
   mirror-exact routing and equal parasitics on differential nets, common-centroid pattern validity. Philis grades
   these only with its own placement-rule models (bench.rs:259–288); no verifier reads the GDS and says "this pair
   is not symmetric". (AV-11)
4. **PEX good enough to rank layouts.** Lampaert §2.5.3.1 (lampaert.txt L1885–1981, PDF p49–51) uses three
   components per box: area coupling to overlapping conductors `C = ε/d·A` (eq. 2.29), lateral
   `C = F(d)·l` with `F(d) = C0 + C1/d + C2/d² + C3/d³ + C4/d⁴` fitted per technology (eq. 2.30–2.31), and
   inter-layer fringe with fitted `C_F0`, `x0` (eq. 2.32). GPurify implements the 1/d parallel-plate term only,
   no inter-layer fringe, no shielding (ground C not reduced under covering metal; coupling computed through
   intervening layers), unmerged rectangles, bbox facing, a fixed 10×t halo, and a non-physical R chain (AV-07).
5. **Performance, not proxies, as the metric.** The performance-driven survey §I (perf_driven_survey.txt L36–44)
   states that optimising "proxy objectives, such as wire length and area … fails to reveal the capability of the
   automation algorithms for important metrics such as performance and manufacturability"; its §III.C case study
   ranks layouts by post-layout offset voltage, CMRR, bandwidth and DC gain (L218–245, Fig. 3). Lampaert §2.4.1
   models layout-induced degradation as `ΔP_j = Σ_i S_ij·Δx_i` (eq. 2.12, lampaert.txt L1599–1613, PDF p42) and
   reports LAYLA against manual layout: a 5–7× productivity gain (§6.4, L7003–7008, PDF p169) and a 10–15 % area
   penalty (ch. 7, L7095–7098, PDF p171). TODAES'22 §1 (todaes22.txt L26–34) attributes the remaining
   manual/automatic gap to geometric or parasitic constraints that are "either inadequate or overly tight".
   Philis's bench reports DRC/LVS/ERC counts, Σ C, WL and area — no gain/offset/PM/noise, no ΔC between matched
   nets, no comparison with a manual layout or with ALIGN/MAGICAL output. (AV-03, AV-06, AV-12) For a
   charge-redistribution DAC the relevant layout metrics are named parasitics, not their sum: top-plate to
   substrate C_TS, top-to-bottom-plate C_TB, bottom-plate C_BS and plate-to-plate C_BB/C_TT; C_TS and C_TB alter
   V_OUT (linearity) while bottom-plate parasitics load V_REF and set the 3-dB frequency (cc_dac_constructive.txt
   §II-A, L140–166, PDF p2, Fig. 2); top-plate routing is built to minimise C_TS (§IV-B.3, L813–826, PDF p9). The
   dac4 row reports none of these; G.4(a) shows C_TS ≈ 2× the extracted array capacitance. (AV-32)
6. **Reliability signoff for analog nets.** Lienig §3.3.1 (lienig_em.txt L3745–3786, PDF p82): RMS currents for
   analog DC nets, average for AC, peak for ESD, all three considered; §3.3.3 (L3975–3995, PDF p87) derives
   per-segment worst-case currents from terminal currents; §3.4.2 (L4247–4262, PDF p93) Black's law with
   temperature. Philis passes one DC number per supply net, GPurify spreads it uniformly over terminals
   (power.rs:725–727), signal nets are never checked, sky130 limits are average-only at 90 °C (AV-09).
   Latch-up: Hastings §5.4.2 (hastings.txt L14962–14970, PDF p258) lists the four countermeasures (prevent
   injection, collect minority carriers, recombine them, reduce well/substrate resistance); sky130 has no rule
   (AV-08).
7. **Substrate coupling.** Charbon ch. 4 (charbon_substrate.txt L1329–1345, PDF p56): compact substrate models for
   substrate-aware optimisation. Philis's PEX has no substrate network (psub is one global net,
   sky130.deck:512).
8. **Benchmark hygiene.** Competitor suites should run on the competitor's own PDK or with the competitor also run
   on sky130, over several seeds, graded with the circuits' published constraints. Philis runs ALIGN/MAGICAL
   netlists on ASAP7 with a 0.1 µm/fin planar mapping (README "Caveat"), discovers MAGICAL testbenches as
   circuits (AV-13), ignores ALIGN's 5 reference placements, and its local suite is 10 circuits of ≤14 devices
   with three copies of one OTA (AV-12). ALIGN's const files are a weak ground truth for extraction: most declare
   only supply ports (§0 census).

---

## 4. Ranked findings (severity order; IDs are stable, not ranks)

| ID | Sev | file:line | Finding | Suggested fix direction |
|---|---|---|---|---|
| AV-01 | critical | backend/verify/src/reference.rs:92-97; frontend/library/src/cellgen.rs:870; backend/verify/src/lib.rs:111-116; benchmarks/src/bench.rs:196-203; benchmarks/tests/signoff_fixtures.rs:28-31 | LVS silently drops every device the deck cannot recognise (all sky130 BJTs) and the library removes every C and L before LVS without even a message. bench prints "LVS MATCH" and the test baseline asserts LVS clean for bjt_mirror (2/2 devices skipped) and bgr_core (9/9 skipped); dac4's 15 caps are never compared (G.2, G.3). | `Checker::set_reference` returns the skipped list; `signoff` emits one `lvs/unverified:<kind>` hard row per skipped kind (or a third verdict PARTIAL that bench prints); stop filtering C in `cellgen::reference` and count them as unverified until the deck extracts them; add BJT and MOM recognisers to the deck. |
| AV-02 | critical | benchmarks/xcheck.py:93-111,207; benchmarks/xcheck_pex.py:53-54; benchmarks/examples/drc_gds.rs:81; benchmarks/xcheck_selftest.py:29-48 | The only independent validation reads JSON keys the sidecar no longer has and old rule ids/kinds; DRC/PEX differentials and the self-test crash. KLayout/magic/netgen are not installed although the foundry decks are (VOL/klayout, VOL/netgen, VOL/magic). No independent check of GPurify runs. | Drop the self-generated KLayout deck; run the foundry `sky130A_mr.drc`, `sky130.lvs`/netgen and magic `drc(full)` from a nix shell over `target/bench_debug/*.gds` in a CI job; report per-fixture counts next to GPurify's; fail on any foundry error GPurify misses. Fix `drc_gds` to map layers via `pdk.layer_gds()`. |
| AV-03 | critical | benchmarks/README.md:6-8; benchmarks/src/fixtures.rs:51-54,233-247; benchmarks/fixtures/{pair,quad,chain4,bgr_core}.spice:1-3 | No comparison to hand layout or to ALIGN/MAGICAL results exists ("That reference comparison is not wired up yet"). TinyTapeout hand GDS paths are found and ignored; fixture "known optimum" comments are never checked; ALIGN's 5 placements and summary CSVs are unused. "Beats hand layout" is unmeasured. | Reference columns: extract the TinyTapeout hand GDS with the same verify/PEX and report area, per-net C, ΔC(matched), DRC by foundry deck, post-layout perf side by side; assert known-optimum HPWL for pair/quad/chain4; import ALIGN placements as baseline rows (PDK caveat stated). |
| AV-04 | high | backend/verify/src/checker.rs:145-150,176-200; frontend/library/src/lib.rs:1368,1372-1405; GP/pdks/sky130.deck:470 | `floating_gate`/`unconnected_pin` rows are dropped on every labelled net; the library labels and ports every net with a pin, overriding the deck's `labels_are_ports: false`. The OTA's undriven `vbias`/`vbn` pass ERC. | Pass the real `.subckt` port list to verify separately from labels; exempt only true ports; keep labels on internal nets for LVS naming. |
| AV-05 | high | GP/pdks/sky130.deck:542-550,663-668; GP/crates/check/src/lvs/compare.rs:35-40 | MOS bulk is not extracted and n-well is not a net: a wrong body tie, a PMOS in the wrong well, or a matched pair split across wells passes LVS; `floating_well` only checks a contacted tap exists. | Make nwell a conductor joined through `ntap`; 4-terminal MOS recognisers (bulk = nwell / psub); configure `well_bias`; add an LVS test with a deliberately wrong body tie. |
| AV-06 | high | backend/verify/src/lib.rs:182; backend/verify/src/checker.rs:317-320; GP/crates/extract/src/network.rs:83-106; frontend/library/src/lib.rs:924-956 | `Report.cost` (4th epoch key, bench `C`) = Σ over all nets of ground + coupling C, coupling counted twice, supplies included. Measured (G.4a): on ota 79 % of the 616 fF is supply-related and the single largest term is the vbn–VSS bias decoupling (29 %), while the output-pair coupling mismatch (0.49 fF, 8 %) is invisible. | Rank on per-net C from `cap_matrix` weighted by sensitivity (Lampaert eq. 2.12) or on `perf` margins; report signal-net C, supply C and max ΔC over matched net pairs as separate columns. |
| AV-07 | high | GP/crates/extract/src/analytical.rs:22-23,84,106-124,224-263,341-417; backend/verify/src/geom.rs:39-42; GP/pdks/sky130.deck:572-598 | PEX is a parallel-plate lower bound: lateral 1/d only, no inter-layer fringe, no shielding, unmerged rectangles double-count overlap area and internal edges, bbox facing, fixed 10×t halo, R per polygon chained in PolyId order, diffusion C 0 with no AS/AD/PS/PD. Measured (G.4b): unmerged metal ground C is 1.13–1.73× the merged value on the fixtures. | Merge same-net polygons per layer before extraction; subtract covered area from ground C and restrict inter-layer coupling to the nearest covering layer; fit F(d) per layer (Lampaert eq. 2.31) from GPurify's field solver (`GP/crates/extract/src/field/`); build R from the router's segment graph; emit AS/AD/PS/PD. Calibrate against magic `ext2spice` on the fixtures. |
| AV-08 | high | GP/pdks/sky130.deck:669-670; pdks/sky130.json:85; VOL/magic/sky130A.tech:4165-4170 | No latch-up rule on sky130 although GPurify has `tap_distance` (gf180mcu.deck:303-306) and the installed foundry tech file states LU.2/LU.2.1/LU.3 (< 15.0 µm diff-to-tap). `tie_max_dist_nm 3000` is unsourced and unchecked. | Add `LU.2 tap_distance(ndiff outside nwell, ptap) <= 15um`, `LU.3 tap_distance(pdiff in nwell, ntap) <= 15um` (and LU.2.1 in dnwell) to the deck; cite the tech file in the sidecar for `tie_max_dist_nm`. |
| AV-09 | high | frontend/library/src/elaborate.rs:297-319; backend/verify/src/checker.rs:123-128; GP/crates/check/src/erc/power.rs:721-727; GP/pdks/sky130.deck:488-495 | EM intent is one DC number per Supply/Ground net; GPurify spreads it uniformly over the rail's terminals (hot spots read cooler); signal nets are never EM-checked; sky130 limits are average-only at 90 °C; li has no limit; vgs/vds/ir_drop/esd.pad skip without intent (G.3 log). | Per-terminal currents from the op point into per-segment currents (Lienig §3.3.3) for every net carrying DC; RMS for analog DC nets (§3.3.1); per-net voltages into intent; print an "EM ran / skipped" column per circuit. |
| AV-11 | high | benchmarks/src/bench.rs:259-288 (self-grading); no verify module | No analog layout-rule check on the final GDS (symmetry, centroid, WPE/LOD equality, metal over matched gates, fill blocking, dummies, ΔC on differential nets). Constraint satisfaction is graded only by the flow's own rule models. | Add an independent `verify::analog` pass: shapes + annotator constraint list + cap_matrix → `alrc/*` rows for mirror-exact geometry per symmetry group, centroid coincidence per CC group, per-device SA/SB and well distance equality, metal coverage over matched gates (Hastings rules 17–23), ΔC between differential nets. |
| AV-10 | medium | backend/verify/src/lib.rs:39-41,150-163; frontend/library/src/lib.rs:952 | Warning rows (`tie_high_low`, `density_cmp`) are hard violations and count equally with shorts in epoch selection. | Carry severity into `Violation` or a separate `warnings` list; exclude warnings from the `|V|` key. |
| AV-12 | medium | benchmarks/fixtures/ota*.spice, tt_ota.spice; benchmarks/src/bench.rs:13,140-141,180,392-395 | Suite: 10 circuits ≤14 devices; 3 identical OTAs give bit-identical rows (G.3); interface contracts ignored (tt_ota has no met4/met5 pins); all matching/symmetry/centroid rows come from that one OTA; single seed; bias from a probe; no post-layout perf although `library::perf` works. | Dedupe; honour interface.json; N seeds with median/min/max; add perf columns via `library::perf` with per-fixture testbenches; add larger sky130 circuits (TinyTapeout OTA_FC, bandgap, SAR DAC). |
| AV-13 | medium | benchmarks/src/fixtures.rs:158-194; benchmarks/src/bench.rs:54-56 | MAGICAL discovery includes `*_test.sp` testbenches; ALIGN/MAGICAL run on ASAP7 with 0.1 µm/fin, not comparable to published results. | Skip `_test.sp`; tag rows "not comparable" unless the competitor ran on the same deck. |
| AV-14 | medium | benchmarks/src/bench.rs:104-121,211-214 | `area` is the device bbox, not the GDS bbox; WL sums supply nets; `util` counts halos. Metrics can improve while the block grows. | Area from `sol.geometry()` bbox; WL split signal/supply; drop util or define it on GDS area. |
| AV-15 | medium | benchmarks/tests/signoff_fixtures.rs:139,157-163 | Test suite is red: dac4 `erc/ar.met2.1:gate` at `feedback_iters = 1`, clean at 5 (G.3): the in-loop antenna model and signoff disagree and the outcome depends on iteration count. OTA fixtures are `#[ignore]`d. | Log the per-stage ratio the router computed next to GPurify's for that net; fix the model mismatch; run the SLOW set nightly. |
| AV-16 | medium | backend/verify/src/pdk.rs:661-674 | `pex_row` answers through subtracted operands: `sheet_ohm("rpoly")` = poly_c's 48.2 Ω/□; for licon the last of five cut rows. The deck has no rbody pex rows. | Use the kept-operand rule (`reaches`); add `pex rbody_*` rows with sourced sheet R; test `sheet_ohm` per role. |
| AV-17 | medium | GP/pdks/sky130.deck:459,594; VOL/magic/sky130A.tech:4890,4912; pdks/sky130.json:100-126; backend/verify/src/pdk.rs:575-577 | `ar.met3.1` sidewall thickness is 2000 nm vs 845 nm in the deck's own pex and the foundry tech file; diode-credit terms dropped although GPurify has `antenna_electrical`; stale sidecar antenna text/table; scalar `antenna_max_ratio` mixes areal and sidewall ratios. | Correct met3 to 845 nm; move to `antenna_electrical` with the tech file's diode terms; delete `cell.antenna_sidewall` and the stale text; expose `(ratio, thickness)` only. |
| AV-18 | medium | backend/verify/src/pdk.rs:619-637; backend/verify/src/checker.rs:43-47,262-281; GP/crates/check/src/erc/rules/antenna.rs:698-704,790 | Density handling knows kind `density` only; sky130's `density_cmp` is neither stripped nor deferred, examines zero whole windows on a block and is recorded as Ran: a vacuous pass. No min density, no fill-block concept. | Treat `density_cmp` like `density` (defer when window > block, reported as ChipLevel); delete or wire `density_rules`; add fill-block markers over matched arrays (Hastings rule 18). |
| AV-19 | medium | frontend/library/src/cellgen.rs:879-890; backend/verify/src/reference.rs:116-119 | Resistors, diodes, BJTs carry no parameters into LVS; resistor value/geometry errors are invisible. | Pass W/L for resistors and area for diodes/BJTs; have the deck recognisers emit those params. |
| AV-29 | medium | backend/verify/src/lib.rs:57-63 | `Intent` has no per-net voltages, so `vgs.*`/`vds.*` (sky130.deck:482-486) never run even with an op point. | Add `(net, V)` from the op point to `Intent` and map it to GPurify's intent format. |
| AV-32 | medium | pdks/sky130.json:8; benchmarks/src/fixtures.rs:556,720-722; frontend/library/src/cellgen.rs:870 | dac4's extracted unit cap is 0.42 fF per 2×2 µm (0.105 fF/µm²) while the sidecar density used to size caps is 2.0 fF/µm² (19×); caps are not in LVS, so neither their value nor their existence is verified; C_TS (12.94 fF) ≈ 2× the extracted array and is not reported (G.4a). | Calibrate MOM density per generator against a field solve or magic extraction and store it with a source; report per-bit C, ratio error, C_TS/C_total and C_TB for DAC fixtures; bring capacitors into LVS with a value tolerance. |
| AV-20 | low | bench stderr (G.3); docs/CRATES.md:110 | `lvs.parametric` and `lvs.device_count_*` log as `Skipped(NotInDeck)` while parametric compare runs (lib.rs:432-475): the skip list misreports what LVS checked. | Declare the rules in the deck or filter them from the skip list. |
| AV-21 | low | backend/verify/src/pdk.rs:86; backend/verify/build.rs:7-23 | Deck directory is a compile-time absolute path into the cargo git cache; binaries are not relocatable. | `include_str!` the four decks at build time, or copy them next to the sidecars. |
| AV-22 | low | backend/verify/src/pdk.rs:1039-1071,1263; pdks/sky130.json:47-86 | Float and array sidecar values silently dropped from `rules`; `REQUIRED_RULES` demands unsourced constants and obsolete `bjt_*` keys. | Parse floats into an `f32` table; require `<key>_source` for process numbers; delete obsolete keys. |
| AV-23 | low | benchmarks/src/fixtures.rs:553,596-617,603-609,205-230,842; fixtures.rs:68-75 | Preprocessor appends `l=0.15u` to every m/x card without `l`; maps `nf` to fins; dead `pattern`/`_ext`/`"TT"`; stale comments; unknown suite word → All (network clone). | Touch only cards `library::parse` calls MOS; drop the `nf` alias; delete dead code; reject unknown suites. |
| AV-24 | low | benchmarks/xcheck_lvs.py:79-82,94-108 | The KLayout LVS differential ignores W/L (100 % tolerance) and its docstring misstates bulk handling. | 2 % tolerance to match GPurify; fix the docstring; or replace by foundry LVS (AV-02). |
| AV-25 | low | backend/verify/src/lib.rs:118-126 | Label-short fallback harvests an unlabelled LVS run and returns no cap matrix. | Harvest DRC/ERC/PEX only on the fallback; still return caps. |
| AV-26 | low | backend/verify/src/lib.rs:176-181 | Skip log is once per process, so bench shows the first circuit's skip set only. | Log per `Checker` run, or return the skip set in the report. |
| AV-27 | low | benchmarks/src/bench.rs:379-391,420; kernel/visualizer/src/lib.rs:59 | Filter doc says substring, code is exact; SVG layer names are empty because the sidecar has no top-level `layers`. | Fix the doc; build the name map from `pdk.layers` + `layer_gds()`. |
| AV-28 | low | benchmarks/tests/signoff_fixtures.rs:22-29,57-69 | Baseline accepts a known GPurify `supply_short` false positive; `origin()` misattributes li/mcon findings. | Track the GPurify fix as a named xfail; attribute by shape provenance. |
| AV-30 | low | benchmarks/fixtures/ota.spice:5-7 | The flagship OTA fixture has undriven bias gates. | Make `vbias`/`vbn` ports (as the interface contracts imply). |
| AV-31 | low | backend/verify/src/lib.rs:29-34 | `shortfall_nm` returns 1 for every non-length measurement: an antenna ratio 2× over and 1.01× over carry the same margin. | Return a normalised shortfall (`(measured − limit)/limit`) for ratio kinds. |

---

## 5. Questions the planners must resolve

1. **Verdict semantics for partial LVS.** Should an unrecognisable device fail signoff, produce a distinct
   PARTIAL verdict that blocks "beats hand layout" claims, or be waivable per kind? (AV-01) This decides whether
   bjt_mirror, bgr_core and dac4 pass today.
2. **Who owns the sky130 deck?** It lives in GPurify, pinned by `Cargo.lock`. AV-05 (wells as nets, 4-terminal
   MOS), AV-08 (LU rules), AV-01 (BJT/MOM recognisers), AV-16/19 (resistor body sheet R), AV-17 (met3 thickness,
   diode credit) are edits in another repository. May Philis carry a local deck override?
3. **Reference signoff engine.** Is the target "GPurify clean" or "magic + netgen + foundry KLayout clean"? The
   latter needs a nix-provided klayout/magic/netgen in CI; AV-02's repair depends on it.
4. **WPE/LOD numbers for sky130.** Hastings gives generic clearances (rule 19: 2 / 3 / 5–10 µm, PDF p716) and LOD
   guidance whose distances are not in the text extraction; the sidecar's KVTH0 values come from the BSIM
   cards (sky130.json:90–92). Should the WPE clearance come from Hastings or from the models' WPE parameters
   (not read in this audit)?
5. **Ranking objective after violations.** Keep Σ C (AV-06), switch to sensitivity-weighted parasitics
   (Lampaert eq. 2.12, needs ngspice sensitivities), or to `perf` spec margins (needs a testbench per circuit)?
   Bench columns should follow the same choice.
6. **Hand-layout ground truth.** TinyTapeout `gds/tt_*.gds` (sky130, silicon-proven, cloned at pinned commits),
   ALIGN/MAGICAL published layouts (other PDKs), or in-house hand layouts of the local fixtures? Licence check
   before vendoring GDS.
7. **Warnings.** Do warnings (density, tie_high_low) take part in epoch selection? (AV-10)
8. **EM model.** GPurify distributes a rail's budget uniformly over its terminals (power.rs:725–727). Should
   Philis instead pass per-terminal currents (requires a GPurify intent extension), and should analog DC nets be
   checked on RMS (Lienig §3.3.1)?
9. **PEX fix location.** Upstream in GPurify (`analytical.rs`) or pre-processing in verify (merge polygons in
   `build_store`)? Either changes every historical C number.
10. **Competitor protocol.** Run ALIGN and MAGICAL on sky130 for the same netlists, or compare only against their
    published numbers on their own PDKs (not like-for-like)?
11. **MOM capacitance ground truth.** The bench preprocessor sizes bare `C` cards at 2.0 fF/µm² (sky130.json:8) while signoff
    extracts 0.105 fF/µm² for the drawn `cap_generic_m1m2` unit (G.4a). Which is authoritative, and what
    independent extraction (magic `ext2spice`, GPurify's field solver, foundry VPP data) calibrates it before any
    cap-array result is quoted?
