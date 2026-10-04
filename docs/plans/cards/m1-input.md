# M1 input batch: implementation cards (FLOW-07, PERF-03 step 3, PERF-08, PERF-09, PERF-30, FLOW-13 step 0)

Branch `m1a-input`, worktree `philis-m1a/input`, base `b29c786` (`git merge m1a`: clean, brings EXT-02 + EXT-11 onto
FLOW-07's `40755ad`/`295bef3`). Line numbers are this base's; where a plan differs, the card wins. Specs:
plan-08 `### FLOW-07`, `### FLOW-13`; plan-07 `### PERF-03`, `### PERF-08`, `### PERF-09`, `### PERF-30`.

Every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first. ngspice tests gate on
`library::tools::sky130_models()` (lib.rs:55); with `PDK_ROOT` set they run. Crates: `library` (frontend/library),
`verify`, `benchmark` (bin `bench`), `dr`, `pnr_core`.

**Order (dependencies first): FLOW-07 → PERF-03 → PERF-08 → PERF-09 → PERF-30 → FLOW-13.** PERF-08 and PERF-09 both
edit `oppoint.rs` (`parse_show`, `extract`, `OpPoint`); PERF-09 goes second. PERF-30 needs only FLOW-07 (ports) and
FLOW-17 (merged on main). FLOW-13 step 0 is independent; it is last because it is a router change.

Sky130 model-library facts measured for this card (ngspice 2026 build in PATH, `$PDK_ROOT/sky130A/libs.tech/ngspice`;
they correct plan-07 PERF-08 step 3, which assumed every non-FET is a `.subckt`):

- `sky130_fd_pr__res_generic_po` is `.model … r` (sky130_fd_pr__model__r+c.model.spice:134, included by
  r+c/res_typical__cap_typical.spice:46 from `.lib tt`). An `R` element works:
  `RXR1 a 0 sky130_fd_pr__res_generic_po w=0.5 l=2 m=1` at 1 V draws 5.20059 mA, R = 192.8 Ω (4 squares).
- `sky130_fd_pr__res_high_po` is a `.subckt r0 r1 b` only in libs.ref, and **no file reachable from
  `sky130.lib.spice tt` includes it**: neither an `X` nor an `R` card simulates it with this library.
- `sky130_fd_pr__npn_05v5_W1p00L1p00` (`.subckt c b e s`, all.spice:49) reads `dkisnpn1x1`/`dkbfnpn1x1`, defined only in
  parameters/montecarlo.spice:348,353; under `.lib … tt` ngspice exits 1 ("Undefined parameter [dkisnpn1x1]"). With
  `.param dkisnpn1x1=0.87913 dkbfnpn1x1=0.98501` (their value at `sky130_fd_pr__npn_05v5_all = 0`, the nominal per
  parameters/critical.spice:62) it simulates. `sky130_fd_pr__pnp_05v5_W3p40L3p40` (`Collector Base Emitter`) needs
  nothing (corners/tt/nonfet.spice:39).
- The sky130 deck (`pdks/decks/sky130.deck:571-603`) has **no BJT rows**, so `deck_models` never adds the
  `sky130_fd_pr__` prefix to a BJT: a fixture must name the full model to be simulated.
- `show r : i,p` prints `device rxr1 / i / p` (top-level R element, no `r.` prefix); `show q : ic,ib,ie,p` prints
  `device q.xq1.qsky130_fd_pr__`; currents are into the terminal (NPN ic > 0, PNP ic < 0). `show q` with no BJT prints
  "No matching instances or models" and the run continues (exit 0).
- ngspice accepts `:` in node names (`n0:1` simulates), so extracted PEX node names need no rewrite.

---

## FLOW-07 SPICE front end — class: mechanical (residual: plan text only)

Implemented `40755ad`, review fixes `295bef3`; re-reviewed here against the 7 findings and the stale EXT-02 comment.
Tests green on `b29c786`: `cargo test --release -p library --lib parse` 21 passed; `--test parse_real` 2 passed;
`-p benchmark --bins mos_channels` 1 passed.

| # | Finding | State on b29c786 | Evidence |
|---|---|---|---|
| 1 | bench/fixtures parse without the model table | resolved | benchmarks/src/bench.rs:169-170 and fixtures.rs:777-779 call `library::spice_with(&text, &ParseOptions { models: library::model_table(..), ..Default::default() })`; `mos_channels_below_the_deck_minimum_are_raised` adds `XM4 … fet33p` classified only by the table |
| 2 | recipe-alias test not load-bearing | resolved | tests/parse_real.rs:42-47 asserts `model_table(&sky)` contains `("res_high_po", Resistor)` and `("res_generic_po", Resistor)`; the deck rows name only `sky130_fd_pr__…` (sky130.deck:584-585), so only the recipe loop (lib.rs:262-266) supplies them |
| 3 | `opts.top` vs file-level cards vs docs | code and rustdoc agree (parse.rs:30-33, :70-72, :133-136; test `several_tops_need_a_choice` :942-952); **plan step 8 still says the opposite** | see edit below |
| 4 | 65 536 devices/nets accepted | resolved | parse.rs:171 rejects `len > u16::MAX`; :225 interns ids `< u16::MAX` (65 535 nets max); test `more_than_u16_max_devices_or_nets_is_an_error` :895-905 pins both edges |
| 5 | E/G node count | resolved | parse.rs:409-416: 2 nodes when a `value`/`vol`/`cur` key is present or the 3rd positional starts `poly(`, else 4; test :876-884 covers all four forms and the too-few error. `POLY (1)` written with a space is read as 4 nodes (tokens split there); no fixture or spec form uses it — not fixed |
| 6 | diode positional area taken as model | resolved | parse.rs:276-279 drops a trailing value when ≥ 2 tokens follow the nodes; test `diode_area_is_not_a_model` :886-893 |
| 7 | `preprocess_spice` not reduced to model mapping | **open, deferred** | fixtures.rs:563-570 still joins `\`, resolves `.param` R/C values, sizes bare R/C from sheet R/cap density, synthesises `nfin`→W, raises channels. The value→W/L sizing belongs to CELL-06 (resistor exact value) / CELL-08 (MIM); the join/`.param` parts are now redundant with FLOW-07 but harmless. FLOW-07 acceptance "bench local runs with `preprocess_spice` reduced to model mapping" is **not met**; it moves to CELL-06/CELL-08 (M1 item 4) |
| – | stale `c_tier` ponytail ("node 0 reads Signal until EXT-02") | resolved by the m1a merge | lib.rs:1191 now reads `ponytail: rails are what the name classifier says (annotator::netrole).` (from ca3e509) |

Edits (docs only; no code):

1. `docs/plans/plan-08-flow-pdk-infrastructure.md`, FLOW-07 step 8: replace "Top = the file-level cards if any, else the
   unique uninstantiated `.subckt` (several → error listing them unless `opts.top`)" with "Top = `opts.top` when set
   (file-level cards and sources are then dropped: a simulator deck's testbench around its DUT); else the file-level
   cards if any; else the unique uninstantiated `.subckt` (several → error listing them, naming `ParseOptions::top`)".
   Reason: the override is explicit, nothing calls it yet (`grep -rn 'top: Some'` = parse.rs tests only), and the
   tested behaviour is the useful one for a testbench deck. This is a recorded deviation, not a code change.
2. Same file, under the FLOW-07 heading, add:
   `Status: done in M1 (\`40755ad\`, \`295bef3\`). Deviation: step 8's \`opts.top\` wins over file-level cards (amended
   above). Acceptance gap: \`preprocess_spice\` is not reduced to model mapping; carried to CELL-06/CELL-08.`

Tests: none new. Command: `cargo test --release -p library --lib parse && cargo test --release -p library --test parse_real`.

---

## PERF-03 step 3 (+ step 4) Floating-gate exemption only for declared ports — class: mechanical

Facts: steps 1–2 on base (`RefInput.external_ports`, backend/verify/src/reference.rs:62; `Checker.external` set in
`set_reference`, checker.rs:96; `drop_port_floating` checker.rs:183; skipped-rule note checker.rs:277; verify test with
`external_ports` Some/None at verify/src/lib.rs:437-446). `cellgen::reference` returns `external_ports: None`
(frontend/library/src/cellgen.rs:935). `labels_and_reference` is lib.rs:1661-1686; it is the one place every signoff
path builds the reference (`signoff_shapes` :1646, `signoff_inputs` :1604 → `library::signoff`, CLI, bench). Every
fixture has a `.subckt` header (`bgr_core e1 e2 VSS`, `bjt_mirror in outn outp VDD VSS`, `chain4 a e g VSS`, `dac4
top cmp d0 d1 d2 d3 VDD VSS`, `ota*`/`tt_ota vinp vinm vout1 vout2 VDD VSS`, `pair d g VSS`, `quad n g VSS`,
`rc_filter vin vout VDD VSS`), so after this step every fixture is checked with its port list. `macro_master` builds
schematics with `ports` empty (macroMaster lib.rs:463) → `None`, old behaviour, as the spec wants.
`*.interface.json` are ignored by the flow (bench.rs:13).

Edits:

1. `frontend/library/src/lib.rs` `labels_and_reference`, after `reference.ports = …` (:1680):
   ```rust
   // PERF-03: only the declared `.subckt` ports leave the cell; with none
   // (no `.subckt` around the top) every labelled net stays exempt.
   reference.external_ports = (!schematic.ports.is_empty())
       .then(|| schematic.ports.iter().map(|n| schematic.nets[n.0 as usize].name.clone()).collect());
   ```
2. Step 4: `benchmarks/fixtures/{ota,ota_constrained,tt_ota}.spice` `.subckt` line: append ` vbias vbn` (ota.spice:2,
   ota_constrained.spice:2, tt_ota.spice:4).
3. `frontend/library/tests/parse_real.rs:76`: ota ports become
   `["vinp", "vinm", "vout1", "vout2", "VDD", "VSS", "vbias", "vbn"]` (the fixture changed; net count stays 9).

Tests:

- `benchmarks/tests/signoff_fixtures.rs`, new
  `#[test] fn ota_old_header_flags_its_undriven_bias_gates()`: read `benchmarks/fixtures/ota.spice`, replace the header
  line with `.subckt ota vinp vinm vout1 vout2 VDD VSS` (`text.replacen(" vbias vbn", "", 1)`, then
  `assert!(old != text)` so the test fails if the fixture loses the ports), run `library::run` with
  `Config { feedback_iters: 1, starts: 1, ..Default::default() }`, `library::signoff`, and
  `let fg = erc_rows(..).iter().filter(|r| r.starts_with("erc/floating_gate")).count(); assert_eq!(fg, 2, …)` (M1
  exit criterion: vbias, vbn). The new-header side is the existing SLOW `ota` row (ERC `&[]`). If the measured count
  is not 2 (GPurify may report per gate, not per net), stop and report the rows; do not change the 2.
- Existing `fixtures_sign_off_within_baseline` (all BASELINE rows ERC `&[]`) must stay green: a new `floating_gate`
  row on any of pair/quad/rc_filter/bjt_mirror/bgr_core/chain4/dac4 is a real undriven internal gate — report it with
  its fixture, do not add it to the baseline.
- `cargo test --release -p benchmark --test signoff_fixtures` and
  `cargo test --release -p benchmark --test signoff_fixtures -- --ignored large_fixtures_sign_off_within_baseline`
  (the three ota rows); `cargo test --release -p library --test parse_real`.
- Bench (`cargo run --release -p benchmark --bin bench local`): `floating_gate` leaves the skipped list on all 10 rows.

---

## PERF-08 The simulated circuit is the drawn circuit — class: judgment

Judgment because the sky130 library contradicts the plan's card forms (facts above) and two fixes need an owner-visible
choice: the NPN nominal parameters injected into the deck, and the full vendor BJT names in `bjt_mirror.spice`.

Facts (frontend/library/src/oppoint.rs unless noted): `OpPoint` :13-29 (no `Default`/`Clone`); `terminal_ua` :41-61
(R/D/Q/L → `None`, C → 0); `OpConfig` :103-122 + `Default` :126-139; `model_for` :143-150 (FETs only); `extract`
:158-199 (deck to `temp_dir()/philis_op_{pid}`, empty table → `Err`); `DevOp` :202-208; `build_deck` :220-247
(`show m : id,vds,vdsat,gm`); `flat_circuit` :250-252; `flat_circuit_with` :257-292 (skips non-FETs at :265-267, emits
`W={w_um} L={l_um} nf= m=`); `instance_name` :299-306; `param_um` :323-334 (invents 0.15 µm / 1 µm); `parse_show`
:414-458; `instance_device` :461-466 (`m.` prefix only). perf.rs: `deck` :99-149 (calls `flat_circuit_with` :128,
`l_um` fallback 0.15 at :135), `evaluate` :168-188 (ignores exit status), `Parasitics` :18-27 (literals at lib.rs:934,
perf.rs:325). `bias()` lib.rs:1341-1374 (`BiasSummary.resolved = o.resolved`). `deck_models` lib.rs:226-232.
`benchmarks/fixtures/bjt_mirror.spice:3-4` uses `npn_05v5_1x1` (no sky130 model) and bare `pnp_05v5_W3p40L3p40`.
Bench `op_config` benchmarks/src/bench.rs:142-150.

Edits:

1. `oppoint.rs` `OpPoint`: `#[derive(Default)]`; doc of `id_ua` → "FET: drain current; resistor: current `P`→`N`;
   `None` otherwise or unresolved"; add
   ```rust
   /// BJT `[ic, ib, ie]`, µA, each into its terminal as ngspice reports it; `None` for any other device or unresolved.
   pub bjt_ua: Vec<Option<[f64; 3]>>,
   ```
   Fix the two test literals (:609, :640) with `..Default::default()`.
2. `OpConfig` gains
   ```rust
   /// `.param` lines emitted right after the model library: values a library reads but does not define
   /// in the chosen corner (sky130's NPN, [`SKY130_NPN_NOMINAL`]).
   pub params: Vec<(String, f64)>,
   ```
   (`Default`: empty) and add
   ```rust
   /// sky130 `npn_05v5_W1p00L1p00` reads these, which only the `mc` corner defines
   /// (libs.tech/ngspice/parameters/montecarlo.spice:348, :353); here at the statistical
   /// variable's nominal 0 (parameters/critical.spice:62).
   /// ponytail: two parameters of one model; a deck-side table if another library needs more.
   pub const SKY130_NPN_NOMINAL: [(&str, f64); 2] = [("dkisnpn1x1", 0.87913), ("dkbfnpn1x1", 0.98501)];
   ```
   Emit in `build_deck` and `perf::deck` after `{lib}`:
   `cfg.params.iter().map(|(k, v)| format!(".param {k}={v}\n")).collect::<String>()` (perf: `cfg.sim.params`).
3. Delete `model_for` and `param_um`. New signature
   `pub(crate) fn flat_circuit_with(netlist, cfg, node, extra, emit_caps: bool) -> Result<String, String>`;
   `flat_circuit(netlist, cfg) -> Result<String, String>` passes `emit_caps = false` (**correction**: a DC operating
   point sees a capacitor as open, so dropping it is exact; emitting dac4's `cap_generic_m1m2`, which no library
   defines, would turn its bias into "cannot simulate"). Per device, with `inst = instance_name(dev)`, `n(t)` the
   existing `net` closure (missing terminal → `"0"`), `m = multiplier` (`params["m"]`, default 1), `um(k) =
   params[k] as f64 / 1000.0`:
   - Nmos/Pmos: model = `cfg.nmos_model`/`cfg.pmos_model` if non-empty else `dev.model`;
     `let s = dev.mos_size().ok_or_else(|| format!("{inst}: no W/L"))?;` card unchanged in form:
     `{inst} {D} {G} {S} {B} {model} W={w_total_nm/1000} L={l_nm/1000} nf={s.nf} m={s.m}{extra}`.
   - Resistor with a model: `R{inst} {P} {N} {model} w={um(w)} l={um(l)} m={m}` (an `R` element: sky130's
     `res_generic_*` are `.model … r`; a `B` terminal is not emitted, an `R` element has two nodes); `w` or `l`
     missing → `Err("{inst}: no W/L")`. Without a model: `R{inst} {P} {N} {r_mohm / 1000}`; neither model nor
     `r_mohm` → `Err("{inst}: resistor has no model or value")`. Add `// ponytail: an R element for every modelled
     resistor; a library shipping one as a .subckt (sky130 res_high_po, libs.ref only) fails in ngspice and reads
     "cannot simulate".`
   - Diode: `D{inst} {P} {N} {model}` + ` area={um(w)*um(l)}` when both exist.
   - Npn: `{inst} {C} {B} {E} {S} {model} m={m}` (`S` missing → `"0"` via `n`); Pnp: `{inst} {C} {B} {E} {model} m={m}`.
   - Capacitor: only when `emit_caps`: with a model `{inst} {P} {N} {model} w={um(w)} l={um(l)} m={m}`, else
     `C{inst} {P} {N} {c_af}a`; neither → `Err("{inst}: capacitor has no model or value")`.
   - Inductor: `Err(format!("{inst}: inductors are not simulated"))`.
   `extra(di)` is appended to FET cards only.
4. `build_deck -> Result<(String, String), String>`; `extract` maps its `Err(e)` to `Err(format!("cannot simulate: {e}"))`
   and, after running ngspice, returns `Err(format!("cannot simulate: ngspice exit {status}: {last 400 chars of
   stderr}"))` when `!out.status.success()` (an unknown subckt or a pin-count mismatch is fatal in batch mode, exit 1;
   measured). Control block: after `show m : id,vds,vdsat,gm` add `show q : ic,ib,ie,p` and `show r : i,p`.
5. `parse_show`: `DevOp` gains `ic, ib, ie, i, p: f64`; the value arm matches
   `"id" | "vds" | "vdsat" | "gm" | "ic" | "ib" | "ie" | "i" | "p"`. `instance_device(col)`: `m.`/`q.` prefix → the
   text up to the next `.` (as today); else a column starting `r` (a top-level `R` element, `rxr1`) → `col[1..]`;
   else `None`. Keep `filter_map` (PERF-09 changes it).
6. `extract` mapping, per device found in the table: FET as today; Resistor: `id_ua = Some(op.i * 1e6)`,
   `power_uw = (op.p.abs() * 1e6).round()`; Npn/Pnp: `bjt_ua = Some([op.ic, op.ib, op.ie].map(|a| a * 1e6))`, power
   from `p`; `resolved += 1` for each. Diode/capacitor: not looked up (unresolved; a capacitor's terminals are already
   a known 0 in `terminal_ua`).
7. `terminal_ua`: Resistor → `id_ua[i]?` → `P: +i, N: −i`, other terminals 0; Npn/Pnp → `bjt_ua[i]?` → `C: ic, B: ib,
   E: ie`, `S` 0. Update the doc comment (resistor and BJT now known when resolved).
8. `perf.rs`: `Parasitics` gains `/// Built from a drawn layout (its capacitor plates are in `caps`): schematic
   capacitor cards are left out so they are not counted twice. pub extracted: bool,`; lib.rs:934 sets
   `extracted: true`; perf.rs:325 literal gets `extracted: false`. `deck -> Result<String, String>` passes
   `emit_caps = !par.extracted` and its `extra` closure returns `String::new()` unless `dev.mos_size()` is `Some`
   (delete the `map_or(0.15, …)` at :135; use `s.l_nm as f64 / 1e3`, `s.nf`). `evaluate`: `deck(..)?` mapped to
   `format!("cannot simulate: {e}")`, and `!out.status.success()` → `Err` as in step 4.
9. `benchmarks/fixtures/bjt_mirror.spice`: XQ1 model → `sky130_fd_pr__npn_05v5_W1p00L1p00`, XQ2 →
   `sky130_fd_pr__pnp_05v5_W3p40L3p40` (**correction** of plan step 6: the deck has no BJT rows, so a bare name is never
   prefixed; FLOW-07's token rule still reads `npn`/`pnp`). `parse_real` counts (2 devices, 5 nets) are unchanged.
10. `benchmarks/src/bench.rs` `op_config`: add
    `params: library::oppoint::SKY130_NPN_NOMINAL.iter().map(|&(k, v)| (k.to_string(), v)).collect()`.

Tests (`oppoint.rs` unit tests unless noted):

- update `flat_circuit_emits_devices_against_library_models` (`.unwrap()`); perf.rs tests calling `deck` (`.unwrap()`).
- `a_resistor_gets_an_r_card_with_its_model`: parse `XR1 a b sky130_fd_pr__res_generic_po w=0.5u l=2u\nXM1 a g 0 0
  nfet_01v8 W=1u L=1u\n` → `flat_circuit` contains the line `RXR1 a b sky130_fd_pr__res_generic_po w=0.5 l=2 m=1`;
  `R1 a b 10k` → line `RXR1 a b 10000` (instance_name prefixes `X`).
- `a_bjt_card_is_collector_first`: the new `bjt_mirror.spice` text → lines
  `XQ1 outn in 0 0 sky130_fd_pr__npn_05v5_W1p00L1p00 m=1` and `XQ2 outp in vdd sky130_fd_pr__pnp_05v5_W3p40L3p40 m=1`
  (`node_name` lowercases and maps VSS → 0; the plan's `VDD` is stale).
- `an_inductor_refuses_to_simulate`: `L1 a b 1n` → `Err` containing `inductors are not simulated`.
- `a_missing_length_is_an_error`: `XM1 d g 0 0 nfet_01v8 W=1u` → `Err` containing `no W/L`.
- `parse_show_reads_bjt_and_resistor_columns`: the canned block (copy verbatim from the measured run):
  ```text
  @@PHILIS_OP
   BJT: Bipolar Junction Transistor
       device q.xq2.qsky130_fd_pr__ q.xq1.qsky130_fd_pr__
        model xq2:sky130_fd_pr__pnp xq1:sky130_fd_pr__npn
           ic          -7.76701e-05           5.14061e-07
           ib          -6.20301e-06           1.42865e-08
           ie           8.38731e-05          -5.28349e-07
            p           0.000144768           5.24063e-07
   Resistor: Simple linear resistor
       device                  rxr1
        model sky130_fd_pr__res_gen
            i            0.00520059
            p            0.00520059
  @@PHILIS_END
  ```
  → `t["xq2"].ic == -7.76701e-05`, `t["xq1"].ib == 1.42865e-08`, `t["xr1"].i == 0.00520059` (exact f64 parse).
- `a_resolved_resistor_draws_its_current`: OpPoint with `id_ua[0] = Some(5.0)` on `R1 x vss` → `terminal_ua[0] ==
  Some([("P", 5.0), ("N", -5.0)])` and `net_current_ua` of `x` = `Some(5)`. Keep
  `a_resistor_current_is_unknown_and_a_capacitor_is_zero` (unresolved resistor), fix its doc comment.
- `frontend/library/tests/perf_postlayout.rs` (gated `let Some(lib) = models() else { return };`), helper
  `fn fixture(name) -> Netlist` = `spice_with(text, &ParseOptions { models: model_table(&sky130), ..Default::default() })`
  then `deck_models`; `fn op_cfg(lib) -> OpConfig { model_lib: Some(lib), params: SKY130_NPN_NOMINAL…, ..Default }`:
  - `fixtures_resolve_every_device`: for `rc_filter`, `bjt_mirror`: `extract(&nl, &op_cfg(lib.clone())).unwrap()
    .resolved == nl.devices.len()` (the M1 PERF criterion; `BiasSummary.resolved` is `o.resolved`, lib.rs:1367).
  - `rc_filter_resistor_carries_current`: `OpConfig { testbench: Some("Vdd vdd 0 1.8\nVin vin 0 0.9\nRload vout 0
    10k\n".into()), ..op_cfg(lib) }` (the probe leaves `vout` unloaded, so the resistor would carry 0); `t =
    op.terminal_ua(&nl)`; `XR1`'s `P` current `abs() > 1.0` µA; Σ of every terminal current on `vmid` `abs() < 1e-3`
    µA (1 nA).
  - `an_inductor_netlist_cannot_simulate`: `extract` on `XM1 d g 0 0 nfet_01v8 W=1u L=1u\nL1 d 0 1n\n` → `Err`
    containing `cannot simulate`.
- Commands: `cargo test --release -p library --lib oppoint perf`; `cargo test --release -p library --test perf_postlayout`;
  `cargo test --release -p library --test parse_real`; bench `local`: rc_filter EM unknown 1 → 0; bjt_mirror `bias`
  none → a probe value; `bgr_core` stays `none` (bare `pnp_05v5_W3p40L3p40` names no ngspice model; renaming it is
  out of scope — report).

Acceptance: rc_filter and bjt_mirror `resolved == devices`; an inductor netlist reports "cannot simulate".

---

## PERF-09 Operating-point robustness — class: mechanical

Facts (after PERF-08): `parse_show` `"device"` arm `cols = rest.iter().filter_map(instance_device)` (oppoint.rs:438-439:
an unparseable column shifts every later one); deck dirs `temp_dir()/philis_op_{pid}` (oppoint.rs:160) and
`philis_perf_{pid}` (perf.rs:170), never removed, ngspice run in the caller's cwd (so `bsim4v5.out` lands there;
`frontend/library/bsim4v5.out` exists, gitignored); `node_name` :309-320 and `probe_bench` :350-394 use
`is_ground`/`is_supply` :396-402 (name lists); EXT-02's `annotator::rail_of` (backend/annotator/src/netrole.rs:36,
re-exported lib.rs:24) is on the base; `library` depends on `annotator`. `bias()` returns a 5-tuple (lib.rs:1341);
`Bias` lib.rs:373-383; `Solution` literal lib.rs:613; `BiasSummary` metadata.rs:158-169 (literal only at lib.rs:1365);
`certified` metadata.rs:149-154; `MetadataReport: Default`.

Deviation (ladder): plan step 4's `OpConfig.ground_nets` is not added; `oppoint` calls `annotator::rail_of` directly
(one classifier, no plumbing). Bulk-inferred rails (EXT-02 `classify_nets`) are not grounded — same ceiling as today's
lists; say so in a `ponytail:` comment on `node_name`.

Edits:

1. `parse_show`: `let mut cols: Vec<Option<String>>`; `"device" => cols = rest.iter().map(|c| instance_device(c)).collect()`;
   value arm: `let Some(Some(dev)) = cols.get(ci) else { continue };`. `OpPoint` gains `/// Devices the simulation
   did not report, by netlist name. pub unresolved: Vec<String>,` filled in `extract` for every device with no table
   row (all kinds).
2. `oppoint.rs`: `pub(crate) fn scratch_dir(tag: &str) -> std::io::Result<PathBuf>`: `static N: AtomicU64`;
   `let d = temp_dir().join(format!("philis_{tag}_{}_{}", std::process::id(), N.fetch_add(1, Relaxed))); create_dir_all(&d)?; Ok(d)`.
   `extract`: `let dir = scratch_dir("op").map_err(..)?`, write `op.spice` there, `Command::new(..).current_dir(&dir).arg("-b").arg("op.spice")`;
   after reading output: `if std::env::var_os("PHILIS_KEEP_DECKS").is_none() { let _ = std::fs::remove_dir_all(&dir); }`
   (also on the `Err` paths after the run). `perf::evaluate`: same with tag `"perf"`, file `perf.spice`; delete its
   `RUN` counter (the dir is unique now).
3. `OpConfig` gains `/// Explicit DC voltage per net (case-insensitive name) for the probe bench; wins over the supply
   default. pub rails_v: Vec<(String, f64)>,`. `node_name`: `if annotator::rail_of(&raw) == Some(NetRole::Ground) { "0" }`.
   `probe_bench`: per net, `rail_of(name)`: Ground → skip; else `rails_v` match → that V; `Supply` → `cfg.vdd`;
   gate-only → `cfg.vdd / 2`; else skip. Delete `is_ground`, `is_supply`.
4. Control block `show m : id,vgs,vds,vbs,vdsat,gm`; `DevOp` gains `vgs, vbs`; parse arm adds `"vgs" | "vbs"`.
   `OpPoint`: `#[derive(Clone, Default)]` (Default from PERF-08) and `pub vgs_v: Vec<Option<f64>>, pub vds_v: …,
   pub vbs_v: …` (V, signed as ngspice prints) set for FETs in `extract`.
5. `lib.rs`: `fn bias(netlist, cfg) -> Bias` (return the struct; the tuple goes); `Bias` gains `op:
   Option<oppoint::OpPoint>`; `run` :284-285 becomes `let bias = bias(&netlist, cfg);`; test lib.rs:1832
   `crate::bias(&nl, &cfg).currents.expect("operating point")`. `Solution` gains `/// The operating point the run was
   biased with; \`None\` without one. pub op: Option<oppoint::OpPoint>,` set `op: bias.op.clone()` at :613.
6. `metadata.rs` `BiasSummary` gains `/// Synthesised mid-rail probe, not a testbench: never a sign-off bias. pub probe:
   bool,` (set in `bias()` as `oc.testbench.is_none()`); `certified()` adds `&& self.bias.as_ref().map_or(true, |b| !b.probe)`.
   The bench's rows are all probes, so none certifies afterwards — intended (plan-07 §0).

Tests:

- `oppoint.rs`: `a_truncated_header_does_not_shift_later_devices`: the existing `SHOW` block with its middle device
  column replaced by `xm4:truncated` (no `m.` prefix) → `t["xm3"].id == 4.10968e-06` and `!t.contains_key("xm4")`.
  `avdd_is_driven_when_classified_supply`: `probe_bench` on a netlist with nets `avdd`, `vss`, gate-only `vin` →
  contains `Vavdd avdd 0 1.8`, no `V…vss` line, contains `Vvin vin 0 0.9`; with `rails_v = [("AVDD", 3.3)]` →
  `Vavdd avdd 0 3.3`. `node_zero_is_ground`: `node_name` of nets `0`, `VSS`, `vssa`, `gnd!` → `"0"`; `vout` → `"vout"`.
  `show_reads_vgs_and_vbs`: `SHOW` plus a `vbs` row → `t["xm5"].vgs == 0.8`, `t["xm4"].vbs ==` the printed value.
  `concurrent_extracts_use_distinct_decks` (gated on `crate::tools::sky130_models()`): two threads extract a 1-NMOS
  netlist with `W=1u` and one with `W=4u` (same bench) → both `resolved == 1`, the second `id_ua` > the first.
  `extract_leaves_nothing_in_the_cwd` (gated): remove `bsim4v5.out` from `std::env::current_dir()` if present, run one
  `extract`, assert it does not exist.
- `metadata.rs`: `a_probe_bias_never_certifies`: `MetadataReport { bias: Some(BiasSummary { probe: true, .. }),
  ..Default::default() }.certified() == false`, same with `probe: false` → `true`.
- Commands: `cargo test --release -p library --lib oppoint metadata`; `cargo test --release --workspace` then
  `find . -name bsim4v5.out -newer Cargo.lock -not -path './target/*'` empty (M1 criterion "no bsim4v5.out in the cwd").

---

## PERF-30 A simulatable post-layout netlist — class: judgment

Judgment because the rewrite works around GPurify's writer output (upstream §6.4 Q7) and the resistor W/L must be
recovered from the schematic: the extractor reports area only.

Facts (measured: `library::run` on rc_filter, `Config { feedback_iters: 1, starts: 1 }`, then
`verify::extract_spice(&sol.geometry(), &[], &pdk, Detail::WithParasitics)`; throwaway, not committed):

- `extract_spice` (backend/verify/src/netlist.rs:25) prints `* lengths below are in database units`, `.subckt TOP`
  (with labels: `.subckt TOP vin:0 vss:0 …`, seen through `Elaborated::netlist`, elaborate.rs:62-83), `.ends TOP`.
- MOS cards **now have 4 nodes** (plan's "three nodes" is stale since GPurify 6341f18):
  `M1 n1:0 n0:0 n3:0 n3:0 sky130_fd_pr__nfet_01v8 w=500 l=150 area=75000` (nm). Dummy gates extract as tied devices
  (`M0 n3:0 n3:0 n3:0 n3:0 …`).
- Resistor: `R6 n4:0 n1:0 sky130_fd_pr__res_generic_po area=1000000` — no W/L, no value.
- Parasitics: `Cp<k> a b <v>f` (559 lines) and `Rp<k> a b <ohm>` (199 lines) on split nodes `net:k`.
- `signoff_inputs(sol, pdk) -> (shapes, pins, RefInput)` lib.rs:1604-1617 gives the signoff labels. CLI writes outputs in
  `write_outputs` frontend/cli/src/main.rs:144-187 (`<top>.gds`, `<top>_ref.spice`, …); bench debug dir
  benchmarks/src/bench.rs:339.

Edits:

1. `frontend/library/src/lib.rs`, next to `reference_spice`:
   ```rust
   /// The routed layout extracted with parasitics as `.subckt {top}` over the schematic's ports
   /// ([`pnr_core::Netlist::ports`], declaration order), simulatable against the PDK's ngspice library (FR-7):
   /// MOS cards become `X` cards in µm (the library's `.option scale=1.0u`, as [`oppoint`] assumes), a
   /// modelled resistor an `R` element with the schematic's `w`/`l`, parasitic `Cp`/`Rp` kept.
   ///
   /// # Errors
   /// No ports, a port with no label, a MOS card without a bulk node, a resistor matching no single schematic
   /// resistor, any other device card (not rewritten yet), or the extractor's own error.
   pub fn post_layout_spice(sol: &Solution, pdk: &Pdk, top: &str) -> Result<String, String>
   ```
   (`top` is a parameter: `Netlist` does not keep the `.subckt` name.) Body:
   - `let (shapes, pins, _) = signoff_inputs(sol, pdk); let raw = verify::extract_spice(&shapes, &pins, pdk, verify::Detail::WithParasitics)?;`
   - `ports`: names of `sol.netlist.ports`; empty → `Err("no .subckt ports")`; each must be some `pins[i].name`
     (case-insensitive) else `Err(format!("port {p} has no label"))`.
   - Per line of `raw` (tokens by whitespace; every token equal, case-insensitively, to `{port}:0` becomes `{port}`):
     `.subckt …` → `.subckt {top} {ports joined by ' '}`; `.ends …` → `.ends {top}`; the `database units` comment →
     `* lengths in um (the model library's .option scale=1.0u)`; other `*` lines kept;
     `M…` → fewer than 5 node/model tokens before `k=v` (i.e. no bulk) → `Err(format!("{name}: MOS card without a bulk
     node"))`; else `X{name} d g s b {model} w={w/1000} l={l/1000}` (drop `area=`);
     `R…`/`C…` whose 4th token parses as a number (with optional SI/`f` suffix: use `t[3].trim_end_matches(char::is_alphabetic).parse::<f64>().is_ok()`) → parasitic, kept;
     `R…` device (4th token a model) → the schematic resistors with `deck` model equal to it (case-insensitive) whose
     `{P, N}` net names equal the card's two nodes with any `:k` suffix stripped (as an unordered pair); exactly one →
     `R{name} a b {model} w={w_nm/1000} l={l_nm/1000}`, else `Err(format!("{name}: {n} schematic resistors match"))`
     (`// ponytail: a resistor drawn as several segments (CELL-06) extracts several cards and errs here`);
     any other device card (`C`/`D`/`Q`/`X` with a model) → `Err(format!("{name}: card not rewritten (PERF-30)"))`.
2. CLI `write_outputs` (main.rs:157-159): after `_ref.spice`, `match library::post_layout_spice(sol, pdk, top) {
   Ok(s) => write(format!("{top}_pex.spice"), s.as_bytes())?, Err(e) => eprintln!("{top}_pex.spice not written: {e}") }`;
   add `{top}_pex.spice` to the `wrote …` line (main.rs:119).
3. Bench, after the GDS write in `run_circuit` (bench.rs ~:339): the same match, file `debug_dir.join(format!("{}_pex.spice", c.name))`.

Tests (`frontend/library/tests/perf_postlayout.rs`, gated on `models()`; helper `fn rc_filter_pex() -> (Solution, String)`
running rc_filter with `Config { feedback_iters: 1, starts: 1, ..Default::default() }` and `post_layout_spice(&sol, &pdk, "rc_filter").unwrap()`):

- `post_layout_rc_filter_simulates`: write to `std::env::temp_dir().join(format!("philis_pex_{}", std::process::id()))`
  a deck `* pex\n.lib {lib} tt\n{pex}\nXdut vin vout VDD VSS rc_filter\nVdd VDD 0 1.8\nVss VSS 0 0\nVin vin 0 0\n.control\nop\nprint v(vout)\n.endc\n.end\n`
  and the same deck with the schematic `.subckt rc_filter vin vout VDD VSS` / `XM1 vmid vin VDD VDD sky130_fd_pr__pfet_01v8 W=1 L=0.15 nf=1 m=1` /
  `XM2 vmid vin VSS VSS sky130_fd_pr__nfet_01v8 W=0.5 L=0.15 nf=1 m=1` / `RXR1 vmid vout sky130_fd_pr__res_generic_po w=0.5 l=2` / `.ends`;
  run `ngspice -b` with `current_dir` = that dir; assert exit success, no stdout/stderr line starting `Error` or
  `ERROR`, and `|v_pex − v_sch| ≤ 0.01·|v_sch|` with `v` parsed from the `v(vout) = ` line. `vin = 0` (not the plan's
  mid-rail): the output then sits at VDD through the PMOS in triode, so the comparison does not ride the inverter's
  gain (at 0.9 V a mV of IR drop moves `vout` by tens of mV). Remove the dir at the end.
- `post_layout_ports_are_the_schematic_ports`: the `.subckt` line of `rc_filter_pex().1` equals
  `.subckt rc_filter vin vout VDD VSS` (rc_filter instead of the plan's ota: same check, a quarter of the run time).
- `every_mos_card_has_four_nodes`: every line starting `X` whose 6th token contains `fet_` has exactly 4 node tokens,
  and its bulk (5th token, `:k` stripped) is `VSS` for `nfet` and `VDD` for `pfet` (FR-7's "NMOS bulks on vdd").
- Commands: `cargo test --release -p library --test perf_postlayout`; `cargo run --release -p philis -- run
  benchmarks/fixtures/rc_filter.spice -o /tmp/x` (check the CLI crate name in frontend/cli/Cargo.toml) writes
  `rc_filter_pex.spice`.

Acceptance: the three tests green (M1 criterion `post_layout_rc_filter_simulates`). If GPurify's header lists the
labelled pieces under another suffix than `:0`, the port rename misses and the test fails: report the header line.

---

## FLOW-13 step 0 `hier_elaborate` green — class: judgment

Root cause found (throwaway diagnostic test + a temporary `eprintln!` in `dr::break_shorts`, both reverted; not
committed). It is **a detailed-routing failure, not a macroMaster or schematic bug**:

- Schematic and pins are right: assertions 1–4 of the test pass (4 devices `x1.m1 … x2.m2`, 8 nets
  `inp vss net2 vb outp inn net6 outn`, shared `vb`/`vss`, private mid nodes `net2`/`net6`).
- The routed result has `open net 2` and `pin access sacrificed on net 2` (dr report); the extracted layout splits x1's
  mid node into two nets (`M0 n3:0 inp …` vs `M1 outp vb n2:0 …`) while x2's mid node (`n7`) is whole. The flat
  single-leg elaboration (`elaborate(&Leg, …)`, the same pin geometry) routes net 2 and is LVS-clean.
- One open net makes GPurify pair nothing: 8 unpaired devices = 4 layout + 4 reference; 17 unpaired nets = 9 layout
  (inp, vss, vb, outp, inn, outn, n2, n3, n7) + 8 reference. Exactly the test's numbers.
- `break_shorts` (backend/dr/src/lib.rs:2160-2190) logged two cross-net pairs, both net 1 (`vss`) vs net 2 on
  `LayerId(20)`: vss shape `(3765, 2845, 750×290)` against net-2 shapes `(2950, 2890, 1105×290)` and
  `(3765, 1925, 290×1255)` (routing frame) → net 2's access shapes deleted. Net 2's pin `x1.m2.s` sits at (130, 2880),
  in the column of x1's vss pins (120–130, y 120/1110/3870); in the hierarchical case vss has 6 terminals spanning x1
  and x2 and its route passes over that pin's access, which only the twin's wider vss net does.
- Not epochs: `ElabConfig { epochs: 1 | 2 | 8 | 16 }` all give `open net 2` (deterministic structural conflict).

Edits (the decision is the router's; candidate, in order of laziness):

1. In dr, before PathFinder, reserve for each pin's net the lattice nodes its L-shaped access jog will occupy (landed
   node → pin, the path `draw` later fills), using the existing `reserved` owner mechanism (lib.rs:37-40, claims kept
   at :1184) so no other net's trunk takes them; then `break_shorts` never needs to delete an access shape. Check where
   the jog's path is computed (grep `access` / `jog` near lib.rs:1305) and reserve the same nodes.
2. If (1) over-constrains dense cells (bench DRC/unrouted regress), make those nodes a high history cost for other
   nets instead of a hard reservation.

Do not change the test, `ElabConfig::default()` (4 epochs), the `Twin`/`Leg` geometry or `device_gap`.

Tests: `cargo test --release -p library --test hier_elaborate` green with no edit to the file;
`cargo test --release -p dr`; `cargo test --release -p library --test ota_cross_pdk --test elaborate`; bench `local`
10/10 DRC 0, unrouted 0, LVS column identical to m1a-report §4 (route hard may drop, must not rise).
If no router change achieves it inside M1, record FLOW-13 step 0 as not met with the evidence above and hand it to
RTE-08 (negotiation keys, M1 item 6), which owns access/trunk contention.
