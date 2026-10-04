# M2 flow, segment 1: implementation cards

Branch `m2-flow`, worktree `philis-m2/flow`, after `git merge m2` (already up to date at `c2940c6`). Line numbers are
of this tree. Every cargo command needs `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.
`docs/plans/dag-m2-m6.{json,md}` are not in this tree (untracked in the main checkout); read there, read-only.

Order: FLOW-06 → GAP-07 → FLOW-09 → FLOW-12 → FLOW-13. No hard cross-module deps (dag). FLOW-12 reads FLOW-09's
`stage_ms`; FLOW-13 step 1 edits `Solution`, which FLOW-09 moves into `finish`. PLC-06 (M1, not done) touches none.

| Item | Class |
|---|---|
| FLOW-06 | do |
| GAP-07 | do |
| FLOW-09 | do |
| FLOW-12 | do (`--constraints`, `--hierarchy` other than `flat`, `--max-wall` exit 2 until EXT-26 / FLOW-11 / FLOW-08) |
| FLOW-13 | do |

---

## FLOW-06 Sidecar schema: `Kind::Tier` shape, provenance

Scope after §6 C1/C8/C23 and dag D12: each owner registers its own keys (C23), GAP-01 (MAT) owns `Process::tier`
and the tier values, C8 deletes the `mosfets` row. FLOW-06 is the `Tier` shape check and the
`cap_density_ff_um2` source. Plan text that is stale:
- `recipe_layers` (`backend/verify/src/pdk.rs:1236-1248`, not 1103) already scans `["resistors", "capacitors"]`
  (CELL-08, M1), and `capacitor_recipes_do_not_move_routing_rules` (pdk.rs:1512) already asserts
  `enclosure("bottom", "plate") == Some(140)` (capm.3). Plan step 2 and test 3 are done. `bjts` recipes carry no
  `layers` (pdks/sky130.json:120-128), so adding `"bjts"` would change nothing. Skipped; add it when a bjts recipe
  draws layers. No `mosfets` key exists (C8).
- `capacitors`, `bjts`, `substrate_kind`, `epi_thickness_nm`, `wpe_clearance_nm`, `lod_moat_ext_nm` are already
  registered (`backend/verify/src/sidecar.rs:69-135`). The MAT/REL rows are their owners' (C23).
- Test 2 (`every_registered_sourced_key_present_has_a_source`) is what `sidecar::validate` (sidecar.rs:184)
  already enforces on every load, and `every_builtin_pdk_loads` (pdk.rs:1487) runs it over all four sidecars. No new
  test, because it would duplicate that one.
- `Tier` validation today: `v.as_array().is_some_and(|a| a.iter().all(Value::is_i64))` (sidecar.rs:167). That
  accepts any length and negative values, and rejects `null` entries.
- `lod_moat_ext_nm` has 2 entries on sky130 (pdks/sky130.json:78-81) and generic_finfet (C21). With an exact-3
  check, both would stop loading.

Edits:
1. `sidecar.rs:167`: `Tier => v.as_array().is_some_and(|a| a.len() == 3 && a.iter().all(|e| e.is_null() || e.is_u64()))`.
   Doc of `Kind::Tier` (sidecar.rs:30): "Array of exactly 3 (MIN, MOD, EXC; index = `MatchClass as usize`), each a
   non-negative integer or `null` (the process does not state that tier)".
2. `pdks/sky130.json:78-81` and `pdks/generic_finfet.json` `lod_moat_ext_nm`: `[3000, 5000]` → `[3000, 5000, null]`
   (EXC unstated; the key is read by nothing, `reader: "unread"`). GAP-01 step 2 sets `[3000, 5000, 10000]` with
   sources. On a merge conflict, keep GAP-01's line.
3. `pdks/sky130.json:9` `cap_density_ff_um2_source` → `"= camimc of sky130_fd_pr__cap_mim_m3_1 (MIM), 2.00e-15
   F/um2: VOL/libs.tech/ngspice/r+c/res_typical__cap_typical__lin.spice:7 (volare 1341f54f); the MOM that cap_array
   draws is not this value (AV-32). Deleted once the benchmark preprocessor reads capacitors.mim_m3_1.c_area_af_um2."`
   (verified: `+ camimc=  2.00e-15` at line 7 of that file under `$PDK_ROOT`). gf180/ihp stay UNVERIFIED.

Tests (`backend/verify/src/pdk.rs` `mod tests`, using `sky130_with`):
- `tier_arrays_are_validated`: `wpe_clearance_nm = [2000, 3000]` → `Err` containing `"cell.wpe_clearance_nm"`;
  `[2000, -1, 5000]` → `Err` containing it; `[2000, 3000, null]` → `Ok`.
- `cap_density_is_sourced`: `Pdk::builtin("sky130")`: `provenance("cap_density_ff_um2")` starts with `"= camimc"`, and
  `!unverified().contains(&"cap_density_ff_um2")`.
- Command: `cargo test -p verify` (all four sidecars load).

---

## GAP-07 `Pdk::model_markers` from the deck recogniser

Facts: `deck_model` is at pdk.rs:1253 (not 1116). Device rows: `self.deck.devices.{kind, model, marker}`
(pdk.rs:362-364). Derived layers: `deck.layers.op(x) -> Option<DerivedOp>` and `operands(x)` (gpurify 6341f18
`ingest/src/deck/mod.rs:123,131`). GPurify lowers a parenthesised sub-expression to a hidden layer named
`"{layer}#{n}"` (parse.rs:757; rule-inline ones are `"{ctx}@{n}"`, parse.rs:1216). Same-op chains may fold into one
row ("operands fold left", deck/mod.rs:289). sky130 (pdks/decks/sky130.deck:200-210):
`ngate = nfet not areaid_ed`, `nfet_01v8 = (ngate not hvi) not lvtn`, `nfet_01v8_lvt = (ngate and lvtn) not hvi`,
`nfet_g5v0d10v5 = ngate and hvi`, `pfet_01v8_hvt = (pgate and hvtp) not hvi`. The device rows' markers are those
named layers (sky130.deck:571-583). Philis `LayerId(n)` = `GvLayerId(n)` (pdk.rs:405).

Edit, in `backend/verify/src/pdk.rs` next to `deck_model`:
```rust
/// Layers `model`'s recogniser requires (`and`) and forbids (`not`) beyond its
/// gate layer, from the marker's derived-layer expression (sky130
/// `nfet_01v8_lvt = (ngate and lvtn) not hvi` → `([lvtn], [hvi])`). The walk
/// expands the expression's own hidden sub-layers (`name#n`) only, so a named
/// operand (`ngate`) is a leaf, and the leftmost required leaf is the gate.
/// `None` when the deck has no recogniser for `model`.
#[must_use]
pub fn model_markers(&self, model: &str) -> Option<(Vec<LayerId>, Vec<LayerId>)>;
```
Steps: `let m = self.deck_model(model)?;` and `r` = the first device row with `resolve(model[r]) == m`. Then
`walk(marker[r], true, &mut leaves)`, a private recursive fn: when `x` is the root or its name contains `'#'`/`'@'`,
then `And` → every operand with the same sign, and `Not` → operand 0 with the same sign and the rest with the sign
flipped. Any other op, a named layer, or a base layer → push `(x, sign)`. Required = the `+` leaves minus the first
`+` leaf (the gate). Forbidden = the `−` leaves. Both are deduplicated in order and mapped to `LayerId(x.0)`. A
base-layer marker → `Some((vec![], vec![]))`. C8: there is no `mosfets` row to delete.

Tests (`pdk.rs` `mod tests`, `load("sky130")`, `id(p, name)` helper at pdk.rs:1578):
- `lvt_needs_lvtn_and_forbids_hvi`: `model_markers("nfet_01v8_lvt") == Some((vec![id("lvtn")], vec![id("hvi")]))`.
- `hvt_pmos_needs_hvtp`: `pfet_01v8_hvt` → `(vec![id("hvtp")], vec![id("hvi")])`.
- `core_nfet_has_no_required_marker`: `nfet_01v8` → `required.is_empty()` and `forbidden` contains `id("hvi")`
  (the deck gives `[hvi, lvtn]`; assert that exact vector).
- `unknown_model_has_no_markers`: `model_markers("no_such_fet").is_none()`.
- Command: `cargo test -p verify model_markers`, plus the test names above.

Fallback (plan risk): a test may show the expression was compiled away (no `#` rows). In that case, report it and
stop. Do not hand-write a `markers` table without the owner's decision.

---

## FLOW-09 Hoist pure work, per-stage timers, seed derivation

Code facts (stale plan lines corrected): `RunStats` is at `frontend/library/src/lib.rs:167-215`. `run` is at
lib.rs:276-320: each start thread calls `solve` (lib.rs:391-642) for `merged`, then `apart` when `distinct`. So
per start it repeats annotate (lib.rs:404), `cellgen::folds`, `CellSpace::new` (lib.rs:418),
`seed_assignment` (lib.rs:483, which DRC-prices every alternative, `cellgen.rs:343`), routing config, then
realize/fill/metadata on the winner (lib.rs:542-628). `performance_rows` (lib.rs:327) annotates again
(lib.rs:356-360). `annotation` leaks one `Stack` per call (lib.rs:700). Its other callers: `elaborate.rs:168`,
`benchmarks/src/bench.rs:236`, and tests at lib.rs:1858/1990/2182. The epoch seed is `seed ^ iter ^ (outer << 32)`
(lib.rs:497), and gp and dp both get that same seed (lib.rs:784, 796). `RuleBatch: Send + Sync`
(`kernel/analog/src/rule.rs:172`). `prices.saturated()` and `prices.steps()` feed `metadata.binding` and
`stats.dual_steps` (lib.rs:537, 586), so `search` must return them.

Edits (`frontend/library/src/lib.rs`):
1. Split `solve`:
   ```rust
   /// Fixed for the run at one cell topology: annotate, folds, `CellSpace::new`, EM/IR rules, router config,
   /// `seed_assignment` (today lib.rs:401-484).
   struct Topology<'a> { flow: Flow<'a>, assignment0: Vec<u16>, distinct: bool, fold: Vec<(u16, i32)> }
   #[allow(clippy::too_many_arguments)]
   fn topology<'a>(nl: &'a pnr_core::Netlist, injected: &Macros, pdk: &'a Pdk, cfg: &'a Config, bias: &Bias,
                   perf_rows: &'a [analog::routing::PerformanceBudget], merge_distinct_gates: bool,
                   stack: &'static analog::routing::Stack) -> Topology<'a>;
   /// One start's search (lib.rs:486-539) on a shared topology; owns its `Prices` and `Negotiation`.
   struct Searched { best: Epoch, stats: RunStats, binding: Vec<String>, key: LexKey }
   fn search(t: &Topology, cfg: &Config, seed: u64) -> Searched;
   /// The winner only (lib.rs:541-641): realize, debug joins, rings, fill, metadata, `Solution`. Consumes `t`, so
   /// `flow.problem.placement` moves into `Solution.placement` (no `Clone` of `Requirements`).
   fn finish(t: Topology, s: Searched, bias: &Bias, pdk: &Pdk) -> Solution;
   ```
   `Flow.netlist` borrows `run`'s netlist. `finish` clones it once to append `best.extra` (today `solve` clones per
   start, lib.rs:401). `run`: build `merged`, then `apart` only if `merged.distinct`. Both are built on the calling
   thread, in that order. In `std::thread::scope`, start `j` (seed as today, lib.rs:300) runs `search` on each built
   topology. Selection must reproduce today's order, because `key_lt` is not transitive inside a C band: per start,
   `apart` wins only if `key_lt(apart, merged)`. Then reduce over starts in index order, and a later start wins only
   if `key_lt(r, best)`. `sim_failures` = Σ over every start and topology, as today. `finish` runs on the winning
   topology only. A compile-time `const _: fn() = || { fn s<T: Sync>() {} s::<Flow<'static>>(); };` asserts that
   `Flow: Sync`. If it is not, fall back to one `topology` per thread and keep steps 2-3, then say so in the report.
2. Annotate once in `run`: `let stack: &'static Stack = Box::leak(Box::new(elaborate::stack(pdk)));`
   `let ann = annotation_with(pdk, &cfg.annotation, stack); let base = annotate(&netlist, &ann);` before `bias`.
   `performance_rows(netlist, p, &ann, &base.net_classes)` reads them instead of re-annotating (lib.rs:356-360).
   `topology` still annotates its own `Problem`, because `CellSpace::new` mutates it and `Problem` is not `Clone`.
3. `fn annotation_with(pdk, base, stack: &'static Stack) -> AnnotationConfig` holds today's body. The public
   `annotation(pdk, base)` becomes `annotation_with(pdk, base, Box::leak(..))`, so its signature stays the same for
   `elaborate.rs:168` and `bench.rs:236`. Update the `ponytail:` note: once per `run` call.
4. `RunStats.stage_ms: [u64; 9]` with doc "CPU ms per stage `gp, dp, rings, gr+dr, dr (diode re-route), diodes,
   signoff, metadata, perf`, summed over every epoch of every start and topology (threads overlap: not wall time)".
   Use the order in `STAGES: [&str; 9] = ["gp", "dp", "rings", "route", "reroute", "diodes", "signoff",
   "metadata", "perf"]`, a pub const that FLOW-12's report prints. In `Flow::epoch`, wrap each call in
   `Instant::now()`/`elapsed().as_millis()` into a local `[u64; 9]` stored in the returned `Epoch.stats.stage_ms`.
   `score_perf` adds index 8 into `stats`. In `search`, `stats.stage_ms[i] += epoch.stats.stage_ms[i]` after every
   epoch. `RunStats::merge` keeps the run-wide `stage_ms` (`..run`). `run` sums the winners' totals over starts,
   like `sim_failures`. `RunStats` stays `Copy`.
5. Separate commit: dp seed = `seed ^ 0xD1B5_4A32_D192_ED03` at lib.rs:796 [policy: any fixed odd constant], so gp
   and dp streams differ (AP-19). This changes results: re-run `cargo test --release -p library` and the bench
   (`cargo bench`/benchmarks smoke the M1 report used), and record the deltas in the commit message. Do not edit
   any baseline to pass.

Tests (`lib.rs` `mod start_tests`):
- `hoisting_prices_each_alternative_once`: a `#[cfg(test)] thread_local!(static PRICE_CALLS: Cell<u32>)` incremented
  in `cellgen::price` (exposed `pub(crate) fn price_calls() -> u32`). Topologies are built on the calling thread, so
  a thread-local is immune to parallel tests. Run `run(pair-with-distinct-gates spice, starts: 1)` and
  `starts: 3` (`feedback_iters: 2, outer_iters: 1`), and read the counter delta of each. Assert `n1 > 0` and
  `n3 == n1`. This fails today, where pricing happens on the start threads and the caller sees 0.
- `same_seed_same_gds_bytes`: sky130, `benchmarks/fixtures/chain4.spice` and `ota.spice` (`include_str!`),
  `Config { seed: 1, feedback_iters: 2, outer_iters: 1, starts: 3, .. }`. Run twice, then
  `assert_eq!(export_gds(&a, ..), export_gds(&b, ..))`.
- `stage_times_are_reported`: one run (`starts: 1, feedback_iters: 2`): `stats.stage_ms.iter().sum::<u64>() > 0` and
  `stage_ms[6] > 0` (signoff runs every epoch).
- Pure-refactor check (steps 1-3, before step 5): build `philis` at the pre-change commit into the scratchpad. For
  every `benchmarks/fixtures/*.spice` and seeds 1-3, run `philis run <f> sky130 --seed s --max-iters 4 --starts 3 -o
  <dir>`, then `cmp` the `.gds` before and after. They must be identical. Record the output in the commit message.
- Command: `cargo test --release -p library start_tests` and `cargo test --release -p library`.

---

## FLOW-12 CLI flags, built-in PDKs, output directory, labelled GDS

Done by FLOW-17 (M1, `2a09c30`), so plan steps 1-2 are mostly stale: `philis run -o DIR` writes `<top>.gds` with
TEXT labels on the deck's text layer (`library::export_gds`, lib.rs:1654; `gds::emit(top, shapes, layer_gds,
texts)`, gds.rs:49, with TEXT/TEXTTYPE/STRING records and the test `top_name_and_texts_are_written`, gds.rs:250),
`<top>_ref.spice`, `<top>_pex.spice`, `signoff.txt`, `signoff.json`, and `--version`. Built-in PDK names and
`--seed/--max-iters/--starts` are positional. `--interface` warns "ignored" (`frontend/cli/src/main.rs:58`).
Remaining gaps: (a) `gds::emit` maps an out-of-table layer to `(id, 0)` and a derived layer to `(0, 0)` silently
(gds.rs:62-65; `layer_gds`, pdk.rs:401). (b) There are no `--pdk/--iters/--outer/--size/--top/--op-lib/--perf/
--interface/--constraints` flags. (c) An error exits 1, the same as "not clean" (main.rs:29). (d) Nothing is
written without `-o`. (e) There is no `report.txt` (metadata + RunStats). (f) There is no `Interface` type.
(g) There is no CLI test.

Edits:
1. `frontend/library/src/gds.rs:49`: `pub fn emit(top, shapes, layer_gds, texts) -> Result<Vec<u8>, String>`.
   `Err` names every distinct shape layer id that is outside `layer_gds` or maps to `(0, 0)`, as in
   `"no GDS stream number for layer ids [12, 40] (derived or unmapped)"`. Keep the name `emit`, because the
   existing callers keep it, so no `emit_labeled` is needed. `export_gds` → `Result<Vec<u8>, String>`. Callers:
   `frontend/cli/src/main.rs:158` (`?`), `benchmarks/src/bench.rs:345` (`.expect` with the fixture name, since a bench
   layout must map), `frontend/library/examples/xlvs_dump.rs:118`, and the gds.rs tests.
2. `frontend/library/src/lib.rs`: `pub enum Side { North, South, East, West }`;
   `pub struct IoPin { pub net: String, pub side: Side, pub frac: f32, pub width_nm: i32, pub layer: String }`;
   `pub struct Interface { pub die_nm: Option<(i32, i32)>, pub pins: Vec<IoPin> }`;
   `impl Interface { pub fn from_json(text: &str) -> Result<Interface, String> }` (format of
   `benchmarks/fixtures/ota_constrained.interface.json`: `die.{w,h}` → `die_nm`, per pin `net, side
   ("north"|…), frac ∈ [0,1], width → width_nm, layer`). Add `serde_json = "1"` (as backend/verify/Cargo.toml:10; already in the
   lockfile via verify) to `frontend/library/Cargo.toml`. Add `Config.interface: Option<Interface>` (default `None`),
   `Config.top: Option<String>` passed to `ParseOptions.top` (lib.rs:278), and `FlowError::Interface(String)`. In
   `run`, after parse, `Err(Interface)` names any `IoPin.net` that is not in `netlist.ports` (FLOW-07). Nothing
   else reads it yet. The doc says "PLC/RTE consume it".
3. `frontend/cli/src/main.rs` keeps the hand-rolled parser and today's positional `<pdk>` and `-o`/`--max-iters`
   (FLOW-17 users). It adds `--pdk NAME|FILE` (same resolution as main.rs:80), `--out` (alias), `--iters` (alias of
   `--max-iters`), `--outer N` → `outer_iters`, `--size spice|per-finger`, `--top NAME`,
   `--op-lib PATH [--corner C] [--vdd V] [--temp C] [--testbench FILE]` → `cfg.op = Some(OpConfig { model_lib,
   corner, vdd, temp_c, testbench: read(FILE), ..Default })`, `--perf SPECS.json` →
   `cfg.performance = Some(PerfConfig { sim: cfg.op or default, testbench: read(<json>.testbench path, relative to the
   JSON), specs })` with the format `{"testbench": "tb.spice", "specs": [{"metric": "gain_db", "min": 40, "max": null}]}`
   (PERF-10 may extend it), and `--interface FILE` → `Interface::from_json`. `--constraints FILE`, `--hierarchy`
   other than `flat`, and `--max-wall` exit 2 with "`--constraints` needs EXT-26" / "needs FLOW-11" / "needs FLOW-08".
   `philis emit` also accepts `--out-rs FILE`. Exit codes: `main` maps `Err` → `ExitCode::from(2)`, not clean → 1,
   clean → 0. The default `--out` is `./philis_out/<netlist stem>/`, so a `run` always writes. `write_outputs` adds
   `report.txt` = `MetadataReport` `Display` (metadata.rs:288) + one `rule\tmargin` line per signoff hard row +
   `RunStats` (`converged`, `iterations`, `outer_iterations`, `sim_failures`, `warnings`, and `stage_ms` named by
   FLOW-09's `STAGES`). `signoff.txt`/`signoff.json` stay. Update the module doc and `USAGE`.

Tests:
- `gds.rs` `unmapped_layer_is_an_error`: one shape on `LayerId(5)` with a 2-entry table → `Err` containing `"5"`.
  The same with table entry `(0, 0)` → `Err`. Existing `top_name_and_texts_are_written` covers TEXT well-formedness
  (plan's `text_records_are_well_formed`), so it needs only `.unwrap()`.
- `lib.rs` `interface_parses_the_fixture_and_rejects_a_non_port`: `from_json(ota_constrained.interface.json)` →
  `die_nm == Some((30000, 70000))`, 6 pins, `pins[0].side == South`, `width_nm == 800`. `run` on `pair.spice` with an
  interface naming net `"nope"` → `Err(FlowError::Interface(m))` and `m.contains("nope")`.
- New `frontend/cli/tests/cli.rs` `run_writes_a_labelled_gds_and_a_report`:
  `Command::new(env!("CARGO_BIN_EXE_philis"))` on `benchmarks/fixtures/pair.spice --pdk sky130 --iters 2 --starts 1
  --outer 1 --out <temp_dir()/philis-cli-<pid>>` → exit code ∈ {0, 1}. `pair.gds` and `report.txt` exist.
  Walk the GDS records (2-byte length, 2-byte type): the STRING (0x1906) payloads after each TEXT (0x0C00), with
  trailing NULs trimmed, contain `d`, `g`, and `VSS`. `report.txt` contains `"stage_ms"`.
- `constraints_flag_exits_2`: `--constraints x.json` → exit code 2, and stderr contains `"EXT-26"`.
- Command: `cargo test --release -p philis` and `cargo test -p library gds`.

---

## FLOW-13 emit and macroMaster round trip (steps 1+; step 0 done in M1)

Code facts: `Solution` (lib.rs:142-163) has no `devices_of`. It lives on `CellSpace.devices_of` (lib.rs:1420).
`emit::emit(netlist, layout, pdk, cfg)` (`frontend/library/src/emit.rs:92`) re-annotates and re-enumerates cells
(emit.rs:98-99), takes w/l/nf from unitization (not `sol.folds`), and indexes `layout.x[i]` by its own cell order
(emit.rs:193). A layout from `run` with a different cell count panics on the index. `IrInst` (emit.rs:26) has no
orient. `GenIr.ports` = every net (emit.rs:282). `to_rust` prints net names raw as identifiers (emit.rs:382-388).
macroMaster (`kernel/macroMaster/src/lib.rs`): `GenError { OffGrid, Overlap }` (lib.rs:121) derives `PartialEq,
Eq`. `Instance.transform` (lib.rs:184) maps bbox/shapes/pins, but not `mac.units` (`pnr_core::units::Unit { x, y, phi,
.. }`) and not dummies. `place_mirrored` (lib.rs:342) has no orientation check. `place` (lib.rs:353) checks overlap
only. `build_with` (lib.rs:404) accepts any edge endpoint. `Orient::apply_rect` exists (`kernel/core/src/geom.rs:69`).
`place_mirrored` users: `frontend/library/tests/ota_cross_pdk.rs:66,72`,
`frontend/library/examples/xlvs_dump.rs:59,63`, and `kernel/macroMaster/src/tests.rs:133`.

Edits:
1. `Solution.devices_of: Vec<Vec<DeviceId>>` (doc: "schematic devices per cell, indexed like `layout`"), set in
   FLOW-09's `finish` from `t.flow.cells.devices_of`.
2. `emit.rs`: move today's body after cell enumeration into
   `fn lift(netlist, layout, pdk, devices_of: &[Vec<DeviceId>], size: impl Fn(&[DeviceId]) -> Result<(i32, i32, u16), EmitError>) -> Result<GenIr, EmitError>`.
   `emit` = annotate/enumerate as today, then
   `if layout.x.len() != cells.devices_of.len() { return Err(Unsupported("layout does not match the cell table".into())) }`,
   then `lift` with the unitization sizes. Add
   `pub fn emit_solution(sol: &Solution, pdk: &Pdk) -> Result<GenIr, EmitError>` = `lift(&sol.netlist,
   &sol.layout, pdk, &sol.devices_of, |m| …)`, which sizes from `Device::mos_size()` of `m[0]`: unit w = total
   W / fingers, with fingers from `sol.folds` for that device when present, else `mos_size().nf·m`. Resistors use
   W/L as today. Add `IrInst.orient: pnr_core::Orient` = `layout.orient[i]`. `elaborate_ir` applies it with a new
   `macro_master::Instance::orient(&mut self, o: Orient)` (D4 via `apply_rect` on bbox/shapes/pins, then a translate
   so the bbox origin stays put; units' `(x, y)` and `phi` through `o.apply`). `to_rust` emits `.orient(Orient::…)`
   for non-`R0`. `frontend/cli/src/main.rs:87` calls `emit_solution`.
3. `GenIr.ports` = the names of `netlist.ports` when it is non-empty, else every net (legacy).
4. `fn ident(name: &str, taken: &mut HashSet<String>) -> String` exactly per the plan: lowercase; chars outside
   `[a-z0-9_]` → `_`; `n_` prefix if empty or digit-led; `_` suffix for the listed keywords (incl. `gen`);
   collisions `_2`, `_3`, …. `to_rust` uses `ident` for the field and keeps the raw name in `.port("…")`.
5. macroMaster: `GenError` gains `Orientation(String)`, `DuplicateName(String)`, and `UnknownTerminal(String)`, and
   stays `PartialEq + Eq`. `transform` also maps `mac.units` (x, y; `phi.0` negated when the map flips x, detected
   by `f` reversing a unit rect's width sign; simplest is that `place_mirrored` negates `phi.0` itself after its
   transform). Dummies are not transformed. `place_mirrored`: when both carry units and `Σ phi` of the mirrored
   `inst` ≠ the reference's `Σ phi` → `Err(Orientation(inst.name))`.
   `pub fn place_copy(&mut self, inst: Instance, reference: &Instance, gap: i32) -> Result<Instance, GenError>` =
   `place_by(inst, ToTheRight, reference, gap)` after a `Bottom` align (translation only). `place`: a duplicate
   name → `DuplicateName`, a bbox off `process.grid()` → `OffGrid`. `build_with`: an edge endpoint that is neither
   in `ports` nor `inst.pin` of a placed instance (pin or device-terminal name) → `UnknownTerminal(endpoint)`.
   Switch `ota_cross_pdk.rs`'s `nf = 1` partners and `xlvs_dump.rs` to `place_copy` where an odd-finger mirror
   would now error.

Tests:
- `kernel/macroMaster/src/tests.rs`: `duplicate_names_are_rejected` (place two `m1` → `Err(DuplicateName("m1"))`),
  `an_unknown_terminal_is_rejected` (`connect("m9.d", "vout")` → `build_with` `Err(UnknownTerminal("m9.d"))`),
  `mirroring_an_odd_finger_device_is_refused` (`Mos` nf=1 with units, `place_mirrored` → `Err(Orientation(_))`),
  `copy_keeps_orientation` (`place_copy`: Σphi equal, and `bbox.x == ref.x + ref.w + gap`).
- `frontend/library/tests/emit_roundtrip.rs` `run_emit_elaborate_signs_off`: sky130, `chain4.spice` and `ota.spice`
  via `library::run` (`feedback_iters: 2, outer_iters: 1, starts: 1`) → `emit_solution` → `elaborate_ir` →
  `Elaborated::signoff(&pdk)`. Assert no row whose rule starts with `"lvs/"`. A fixture with a >2-member matched
  group returns `Unsupported` (emit.rs:107). If ota hits that, the test asserts `Err(Unsupported(m))` naming
  "quad" for ota and keeps the full check on chain4, and the report says so. The acceptance T11 round-trip half is
  then open.
- `emit.rs` `identifiers_compile_shaped`: `in, 0, a<1>, vout-, VDD, vdd` → `in_, n_0, a_1_, vout_, vdd, vdd_2`, each
  matching `^[a-z_][a-z0-9_]*$` and not a keyword.
- Commands: `cargo test -p macro_master`, `cargo test --release -p library --test emit_roundtrip --test ota_cross_pdk
  --test hier_elaborate`.
