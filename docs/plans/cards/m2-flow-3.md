# M2+ flow, segment 3 (M2): FLOW-06, GAP-07, FLOW-09, FLOW-12, FLOW-13

Branch `m2-flow` after `git merge m2` (clean, tip `0f84944`). Every cargo command needs
`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`. dag: no hard deps on any of the five. PLC-06
(M1, not done) is touched by none.

All five were built in segment 1 (card m2-flow-1.md): `a2d4e9a` FLOW-06, `e2afff1` GAP-07, `223d711` + `bd66670`
FLOW-09, `2f7f304` FLOW-12, `cac3892` FLOW-13, `ea009a9` review fixes. This card is the re-check after the merge
of m2 and the residue each item still owes.

Post-merge test run (release): `verify --lib` 57/57; `macro_master` 13/13; `library --lib` gds/emit/hoisting/
same_seed/stage_times 11/11; `philis` (cli) 2+1 pass; **`library --test emit_roundtrip` 2/3: `run_emit_elaborate_signs_off`
red** (see FLOW-13). The same test is green at the pre-merge tip `bfa620d` (checked in a detached scratch worktree):
the merge broke it.

| Item | Class | Residue |
|---|---|---|
| FLOW-06 | do | one word: scan `bjts` in `recipe_layers` |
| GAP-07 | do | none; verify only |
| FLOW-09 | do | none; verify only |
| FLOW-12 | do | constraint diagnostics never reach `report.txt` |
| FLOW-13 | do | T11 round trip red after the merge: find and fix |

---

## FLOW-06 Sidecar plumbing — class: do (residue only)

Code facts (the plan is stale on all three steps):
- Step 1 is done by the owners (C23): `backend/verify/src/sidecar.rs:63-153` registers every key of the plan's table
  under MAT's names (`abeta_n_pct_um`, `abeta_p_pct_um`, `bjt_ka_pct_um`, `vbe_tc_uv_per_k`, the six `Tier` rows,
  `substrate_kind`, `epi_thickness_nm`, `capacitors`, `bjts`). The REL-05 scalars `em_ref_temp_c`/`em_activation_ev`/
  `em_current_exponent` do not exist: GAP-06's `em_derating` table (sidecar.rs:87) replaced them. The deprecated
  `*_moderate` scalars are gone (MAT-07). No `mosfets` row (C8, GAP-07). `Kind::Tier` validation is at
  sidecar.rs:184 (3 entries, `u64 | null`).
- Step 2: `recipe_layers` (`backend/verify/src/pdk.rs:1328-1340`) scans `["resistors", "capacitors"]`. sky130's
  `bjts` recipes (pdks/sky130.json:162) carry no `layers` object today, so adding `"bjts"` is a no-op now and keeps
  the spec's rule for when CELL-09 adds one.
- Step 3 done: pdks/sky130.json:13 `cap_density_ff_um2_source` = "= camimc …"; test `cap_density_is_sourced`
  (pdk.rs:1714).
- Tests in place: `tier_arrays_are_validated` (pdk.rs:1698), `capacitor_recipes_do_not_move_routing_rules`
  (pdk.rs:1649, asserts `Overlay::enclosure("bottom", "plate") == Some(140)`, capm.3; the plan's
  `recipe_layers_cover_capacitor_tables`). `every_registered_sourced_key_present_has_a_source` is covered by
  `every_builtin_pdk_loads` (pdk.rs:1624): `Pdk::load` runs `sidecar::validate`, which rejects a sourced value
  without `<key>_source` (sidecar.rs:201).

Edit: pdk.rs:1330 `for kind in ["resistors", "capacitors"]` → `["resistors", "capacitors", "bjts"]`.
Test: none new (no-op on every built-in sidecar); `cargo test --release -p verify --lib` stays 57/57.

## GAP-07 `Pdk::model_markers` — class: do (verify only)

Done at `e2afff1`: `pdk.rs:1357` `pub fn model_markers(&self, model: &str) -> Option<(Vec<LayerId>, Vec<LayerId>)>`,
walking hidden (`#`/`@`) sub-layers of the device marker; leftmost required leaf = gate, dropped. Tests
`lvt_needs_lvtn_and_forbids_hvi`, `hvt_pmos_needs_hvtp`, `core_nfet_has_no_required_marker`,
`unknown_model_has_no_markers` (pdk.rs:1771-1791), green post-merge. No `mosfets` registry row exists (C8 satisfied).
Acceptance (CELL-16's test) belongs to cells, M3.

## FLOW-09 Hoist, stage timers, seed — class: do (verify only)

Done (`223d711`, `bd66670`): `topology`/`search`/`finish` split (`finish` at frontend/library/src/lib.rs:994),
`RunStats.stage_ms: [f64; 9]` (f64, not the plan's u64: sub-ms stages), `library::STAGES` names, dp seed xor.
Tests `hoisting_prices_each_alternative_once` (lib.rs:2560), `same_seed_same_gds_bytes` (2578),
`stage_times_are_reported` (2588): green in release post-merge. Known: `same_seed_same_gds_bytes` trips the
`gp::Prices::bind` debug_assert in debug builds (m2-flow-2.md "Out of scope"; owner gp/annotator), so run it
`--release`. Acceptance "GDS bytes identical to the pre-change binary" was checked at `223d711`; not re-run.

## FLOW-12 CLI and deliverables — class: do

Code facts: flags at frontend/cli/src/main.rs:47-97; `--constraints` now reads the file into
`Config.constraints` (main.rs:92) and `run` parses it with EXT-26's `AnnotationConfig::from_json`
(frontend/library/src/lib.rs:407-420; EXT-26 landed, `d21ca1a`, so the plan's "exit 2 until EXT-26" is stale).
`--max-wall` / `--hierarchy ≠ flat` exit 2 naming FLOW-08 / FLOW-11 (main.rs:93-97). `report.txt` is written at
main.rs:270-284 (metadata, signoff hard rows, run stats, `stage_ms`). Labelled GDS tests: `top_name_and_texts_are_written`
(gds.rs:259, the plan's `text_records_are_well_formed`), `unmapped_layer_is_an_error` (gds.rs:274); CLI tests
`run_writes_a_labelled_gds_and_a_report`, `constraints_flag_reads_the_file` (frontend/cli/tests/cli.rs:31, 55).

Gap: the spec says "each returned `Diagnostic` is one line of `report.txt` (unknown constraint names are reported,
never dropped)". The diagnostics go `ann.sidecar_diags` → `intent.diagnostics` (backend/annotator/src/lib.rs:229)
and stop there: nothing in `frontend/library` reads `problem.intent.diagnostics`, `Solution` has no field for them
(`Solution.intent` at lib.rs:243 is `verify::Intent`, a different type), so they are dropped.

Edits:
1. `frontend/library/src/lib.rs` `Solution` (lib.rs:228): add
   ```rust
   /// Annotator findings (EXT-26 sidecar entries it could not apply, ambiguous symmetry, conflicts), one per
   /// finding, in annotator order; the CLI writes one line each to `report.txt`.
   pub diagnostics: Vec<analog::intent::Diagnostic>,
   ```
   In `finish` (lib.rs:994), take them before `flow.problem.placement` moves:
   `let diagnostics = std::mem::take(&mut flow.problem.intent.diagnostics);` (bind `flow` `mut`), and set the field
   in the `Solution { … }` literal (lib.rs:~1102). Every other `Solution { … }` constructor (grep
   `Solution {` in frontend, benchmarks) gets `diagnostics: Vec::new()`.
2. `frontend/cli/src/main.rs` report (main.rs:272): append `"# diagnostics ({n})\n"` then one line per
   `d` in `sol.diagnostics`: `format!("{}\t{}\n", d.kind, d.message)`.

Test (`frontend/cli/tests/cli.rs`, new fn `constraint_diagnostics_reach_the_report`): write
`[{"constraint":"GroundPorts","ports":["NOPE"]}]` to a temp file (not the planned
`Align`/`XNOPE`: `Align` has no sidecar reader and yields `sidecar_unsupported`); run `philis pair.spice --pdk sky130
--starts 1 --iters 2 --outer 1 --constraints F -o DIR`; assert exit ∈ {0, 1}; `report.txt` contains a line starting
`sidecar_unknown_name\t` (backend/annotator/src/sidecar.rs:47). Fails today (no such line). Command:
`cargo test --release -p philis --test cli`.

## FLOW-13 emit and macroMaster round trip — class: do

Code facts: steps 1-5 done at `cac3892` (`Solution.devices_of`, `emit_solution`, `IrInst.orient`, port idents,
macroMaster `place_copy` / duplicate / unknown-terminal / odd-finger checks; tests at kernel/macroMaster/src/tests.rs:245-283,
emit.rs:548, green). Round trip: frontend/library/tests/emit_roundtrip.rs:224 `run_emit_elaborate_signs_off`
(chain4 is skipped only with `Unsupported("… no quad variant")` and asserts so; ota gets the full check).

Regression: post-merge, ota fails at emit_roundtrip.rs:241 with many `lvs/lvs.unpaired_device` rows; at `bfa620d`
(pre-merge) the test passes. `git bisect run` (scratch worktree, `bfa620d` good, `0f84944` bad, test
`run_emit_elaborate_signs_off`) names `6fe83dc` "Merge branch 'm2' into m2-routing" (parents `0f0f26a` routing
review fixes 3, green; `240a99b` annotator 4 GAP-09, green): neither side alone breaks it, their combination does.

Edits: none decided before the culprit is known. Steps:
1. Diff `6fe83dc` against each parent on the round-trip path (`git diff 0f0f26a 6fe83dc` and `240a99b 6fe83dc` over
   frontend/library/src/{emit,elaborate,lib}.rs, backend/dr, backend/annotator/src/emit.rs). Rerun the test with `--nocapture`, dump `sol.layout.variant` and the
   emitted `GenIr` for ota: check whether the winner now picks a variant `emit_solution` maps wrongly (new
   variants since the merge: CELL-15 four-row MOS, `e9128a0`) or `elaborate_ir` draws differently from the flow.
2. Fix in `emit.rs` (refuse with `Unsupported` an alternative it cannot model, as it does for ratioed groups at
   emit.rs:118, or map it), never by loosening the assertion. If the cause is in another module's code, report it
   to that owner with the commit and the failing rows.
Test: `cargo test --release -p library --test emit_roundtrip` 3/3; the assertion at emit_roundtrip.rs:241 unchanged.
T11 stays open for >2-member groups (chain4, dac4*, mirror_ratio) and non-MOS/R kinds (card 1 review notes).

## Notes

- The worktree carries uncommitted edits from segment 2 (docs/LAYOUT-FUNDAMENTALS.md line refs,
  docs/plans/cards/m2-flow-2.md CLI/bench-name notes); not part of this commit.
- FLOW-13 outcome (resume run, after merging m2 at `e241ff3`): two causes. (1) `elaborate::route_built` never set
  `dr::DetailedCfg::stack`, so every route landing on cell metal read as `drawn short net N to cell metal`; fixed
  (set from `annotation(..).process.stack`, as `Flow` does). (2) Still red: annotator EXT-24 (`0e5edfa`,
  backend/annotator/src/extract.rs:121-130) emits a `CrosstalkExclusion` between each Voltage set's gate and drain
  nets (ota: vinp/vinm × vout1/vout2), and routing RTE-18 (`407ce25`, backend/dr/src/lib.rs:1050-1084) enforces it as
  a hard separation in the search with no pin exemption. In an interdigitated pair those pins sit one finger apart,
  so dr routes nothing on vout1/vout2 (`open net 1 7`, `open net 4 7`) and LVS reports unpaired devices/nets. The
  flow's own winner shows it too (`sol.route`: `open net 1`, `open net 5`). With the crosstalk batch dropped (debug
  only, not committed) the round trip is 3/3. Owners: routing (exempt pin approach / cell footprints from `sep`) or
  annotator (no gate-drain exclusion inside one matched cell). Assertion at emit_roundtrip.rs:241 unchanged.
- Resume check (release, PDK_ROOT set): `library --lib` 176/176, `philis` all green, `verify --lib` 57/57. Red on
  `library::run` paths this commit does not touch: `antenna_diode`, `extra_devices::an_adopted_antenna_diode_keeps_lvs_matched`,
  `drawn_cards::a_mim_dac_signs_off_with_its_capacitors`, `perf_postlayout::ota_probe_regions` (LVS unpaired rows,
  consistent with the same crosstalk-separation cause); not investigated further here.
