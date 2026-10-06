# M2+ perf, segment 5 (M3): PERF-19, PERF-16

Order: PERF-19, then PERF-16 (hard dep: PERF-16's C_magic column comes from PERF-19's harness). Line numbers are for
this tree after merging `m2` (FLOW-12 and FLOW-14 are in). Spec: plan-07 `### PERF-19`, `### PERF-16`; dag entries:
PERF-19 hard = [FLOW-12] (done), PERF-16 hard = [PERF-19], step-level PERF-11 (rank by ground-C/sensitivity rows until M4).

| Item | Class |
|---|---|
| PERF-19 | do |
| PERF-16 | do (all four steps; step 3's promoted-epoch use is gated on its own measurement, as the plan says) |

## Tools: use the nix shell (owner request)

klayout/magic/netgen are not on PATH. Every tool-gated test and the harness run inside `nix-shell`, which keeps
the environment (PATH, PDK_ROOT) that qcargo hands its worker:

    cd /home/omare/Documents/Projects/Rust/philis-m2/perf
    nix-shell -p klayout magic-vlsi netgen-vlsi --run \
      'PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk PHILIS_REQUIRE_TOOLS=1 \
       /home/omare/Documents/Projects/Rust/Philis/tools/qcargo test -p benchmark --test xcheck_smoke'

(checked: resolves magic 8.3.629, netgen 1.5.320, klayout 0.30.8). `PHILIS_REQUIRE_TOOLS=1` makes a missing tool a
failure, not a skip, so a green run proves the tools ran. Fallback without nix-shell: prepend the store dirs from the
task prompt to PATH.

## PERF-19 Foundry signoff harness; delete the broken cross-checks

### Current code facts (plan corrections in bold)
- FLOW-12 is done: `library::export_gds(sol, pdk, top, ports)` (frontend/library/src/lib.rs:2386) writes labels
  (`gds::Text`, gds.rs:38–50; `gds::emit` already returns `Result<Vec<u8>, String>`, gds.rs:50); `ports` empty =
  every signoff label. Its doc (lib.rs:2381–2384) notes magic makes every top label a port. `library::reference_spice
  (sol, pdk, top, ports)` (lib.rs:2399) is the LVS reference with dummies and per-finger cards. **So step 3's
  `<name>.ref.spice` must come from `reference_spice`, not from the preprocessed schematic** (which lacks dummies).
- bench `run_circuit` (benchmarks/src/bench.rs:157) writes `<name>.gds` with `ports = &[]` at bench.rs:355–358,
  `<name>_pex.spice` (359–366), `signoff.txt`, `violations.txt`, `drc_located.txt`. `signoff.caps` is
  `verify::CapMatrix` = `Vec<(String, Option<String>, f64)>` fF (backend/verify/src/lib.rs:72, checker.rs:337).
- FLOW-14: `library::tools::tool_or_skip(bin)` (frontend/library/src/lib.rs:51) and `present_or_skip`; nightly CI job
  `.github/workflows/ci.yml:57–` sets `PHILIS_REQUIRE_TOOLS=1` but installs only ngspice (apt). **A new tool-gated
  test needs klayout + magic in that job or the nightly goes red.**
- flake.nix:85 has `pkgs.klayout`; magic/netgen absent. **Open question 4 resolved**: nixpkgs attributes are
  `magic-vlsi` and `netgen-vlsi` (`netgen` is the FEM mesher), checked against the flake's locked nixpkgs b6c98e9e.
- Foundry decks under `$PDK_ROOT/sky130A/libs.tech/` (volare 1341f54f): `klayout/drc/sky130A_mr.drc`,
  `klayout/lvs/sky130.lvs` + `run_lvs.py`, `magic/sky130A.tech`. sky130A_mr.drc: `$input`/`$top_cell` :27–28,
  `$report` :31–32, `FEOL = false`/`BEOL = false` :42–43, `$feol`/`$beol` :49–58, `$offgrid` :61, `$thr` :90–91,
  **m1.2 (0.14 µm) at :441** (plan said :439).
- **run_lvs.py cannot be used**: it imports `docopt` (:43; not installed, `python3 -c "import docopt"` fails), and
  passes its own `-rd target_netlist=extracted_netlist_….cir` (:100) after `--output_netlist`'s (:60–61). Call the
  deck directly: `klayout -b -r $VOL/klayout/lvs/sky130.lvs -rd input=<gds> -rd report=<f>.lvsdb -rd schematic=<f>.ref.spice
  -rd target_netlist=<f>.ext.cir -rd run_mode=deep -rd thr=<ncpu>`; every other `$var` in sky130.lvs is optional
  (`if $x … else default`). This also removes the PDK_ROOT trap (run_lvs.py:116–123).
- Files to delete exist: benchmarks/xcheck.py, xcheck_pex.py, xcheck_lvs.py, xcheck_selftest.py,
  benchmarks/examples/drc_gds.rs. Other references: benchmarks/README.md:80 `## Cross-checking GPurify (xcheck)`,
  docs/CRATES.md (open issue 1); ref/Notes/*.html|js are reference notes, leave them.

### Edits
1. `flake.nix` devShell `buildInputs` (after `pkgs.klayout`, :85): `pkgs.magic-vlsi` `pkgs.netgen-vlsi`.
2. `.github/workflows/ci.yml` nightly: the apt line becomes `sudo apt-get install -y ngspice klayout magic`
   [verify the Ubuntu package names in the job log; if klayout is not in the runner's apt, use the KLayout .deb].
3. `benchmarks/src/bench.rs` `run_circuit`, after the GDS write (bench.rs:358):
   - `let ports: Vec<String> = sol.netlist.ports.iter().map(|n| sol.netlist.nets[n.0 as usize].name.clone()).collect();`
   - `<name>.lvs.gds` = `library::export_gds(&sol, pdk, &c.name, &ports)` (only the schematic ports labelled, so
     magic/KLayout see the same pins as the `.subckt`); `<name>.ref.spice` = `library::reference_spice(&sol, pdk,
     &c.name, &ports)`. `<name>.gds` stays all-labelled (DRC, magic C per named net).
   - `caps.json` = `serde_json::to_string(&signoff.caps)` (rows `[net, other|null, fF]`; serde_json is already a
     dependency, benchmarks/Cargo.toml:13). A failed export removes the stale file, as `_pex.spice` does (bench.rs:362–365).
4. New `benchmarks/signoff_xcheck.py` (stdlib only), per `target/bench_debug/<f>/` or `--drc-only <gds>`:
   - `VOL = $PDK_ROOT/sky130A/libs.tech` (error if unset or missing).
   - `klayout_drc(gds, top) -> int`: the plan's command with `-rd feol=1 -rd beol=1 -rd offgrid=1`; the argv is built
     in one place and the function `assert`s both `feol=1` and `beol=1` are in it; count = `<item>` elements of the
     lyrdb (`xml.etree`). `seal`/`floating_met` off.
   - `klayout_lvs(f)`: the direct deck call above on `<f>.lvs.gds` vs `<f>.ref.spice` → verdict from klayout's
     exit/log ("Netlists match" / "don't match") [verify the exact log text on one run].
   - `magic(f)`: plan's stdin script (`-dnull -noconsole -T $VOL/magic/sky130A.tech`), cwd = the fixture dir, on
     `<f>.gds`; DRC count from `drc count total` output; ground C per net from `TOP.spice` (`C… net 0 <val>`;
     coupling rows summed per pair).
   - GPurify side: `signoff.txt`/`violations.txt` DRC count (`drc/` rows), LVS verdict (any `lvs/` row = mismatch),
     `caps.json`.
   - Writes `target/xcheck/summary.json` (per fixture: drc {foundry, magic, gpurify}, lvs {foundry, gpurify},
     caps [{net, gpurify, magic}]) and prints a table; findings attributed by layer like signoff_fixtures.rs `origin`.
     Exit 1 on any foundry DRC error, or a foundry LVS mismatch where GPurify said match.
   - `--drc-only <gds>`: prints `{"drc": n}` (for the smoke test).
5. Delete the four `xcheck*.py` and `examples/drc_gds.rs`; README "Cross-checking GPurify" rewritten to: run `bench
   local`, then `nix-shell -p klayout magic-vlsi --run 'python3 benchmarks/signoff_xcheck.py'`, what the summary
   holds, why feol/beol are mandatory. docs/CRATES.md open issue 1: point at the harness.

### Tests
- `benchmarks/tests/xcheck_smoke.rs`, `fn foundry_drc_sees_a_planted_met1_spacing_fault()`:
  `if !library::tools::tool_or_skip("klayout") { return; }`; two met1 `Shape`s 1000×1000 nm at x=0 and x=1100
  (100 nm gap < m1.2 140 nm), `library::gds::emit("TOP", &shapes, &pdk.layer_gds(), &[]).unwrap()` written to a
  `std::env::temp_dir()` subdir. Assert (a) `python3 benchmarks/signoff_xcheck.py --drc-only <gds>` exits 0 and
  prints JSON with `drc >= 1`; (b) the same GDS through `klayout -b -r $VOL/klayout/drc/sky130A_mr.drc -rd input=…
  -rd top_cell=TOP -rd report=…` (no feol/beol) gives 0 <item>`s — pins why the flags are mandatory. Needs `PDK_ROOT` (skip via `present_or_skip` when the deck file is absent).
- Command: the nix-shell line above. Also `qcargo check -p benchmark` and `qcargo run --release -p benchmark --bin
  bench local` then the harness on all local fixtures (acceptance: summary for every fixture; list every foundry
  finding; T1 gate stays off until owners fix them — report the counts, do not loosen anything).

## PERF-16 PEX fidelity: merged metal, field-solved nets, calibration

### Current code facts (plan corrections in bold)
- `verify::geom::build_store(shapes, pins, deck, strings)` backend/verify/src/geom.rs:30–56: one rect per `Shape`
  (:38–41). Only caller `Checker::load_geometry` checker.rs:149–155 (**not 135–137**), called from `Checker::run`
  checker.rs:171–191. `RunOptions { quasistatic_nets: Vec::new(), .. }` at **checker.rs:179–184** (not 163–168).
- `geometry::merge_rects` is at **frontend/library/src/geometry.rs:95**; the library merges the well layer only:
  `Solution::geometry` lib.rs:2290–2295, epoch geometry lib.rs:1519, test lib.rs:3169. Leave these (they also shape
  the GDS); build_store's union makes them redundant for PEX only.
- GPurify (Cargo.lock rev 6341f18): `gdsverify::geom::view::validate_layer_into(store, layer, out)` view.rs:168;
  `gdsverify::geom::derive::merge_into(a, out) -> Result<(), BooleanError>` derive.rs:24; `gdsverify::geom::rects::
  decompose_into(layer, rects, poly_start)` rects.rs:32; `GeometryStoreBuilder::finish(layer_count)` store.rs:197;
  `run_pex` engine/run.rs:338–410 returns `StageStatus::Refused` for an unknown net name or non-reciprocal solve.
- **met1 PEX row is the vendored deck pdks/decks/sky130.deck:639** (`area_cap 25.78aF/um2 fringe_cap 40.57aF/um`), not
  GP/pdks/sky130.deck:590. Test arithmetic unchanged: 1248.2 aF merged, 1382.8 aF unmerged.
- `signoff_checked` lib.rs:190 has 9 callers (7 tests, library lib.rs:2312, 2509); `Checker::run` ~10 test callers.
  **Deviation (smaller diff, same behaviour)**: options go through a Checker setter and a new
  `signoff_extract`, not a new parameter on `signoff_checked`/`Checker::run`.
- `standalone` lib.rs:375 backs `verify::drc`/`erc` (lib.rs:363–371). harvest lib.rs:290–330 turns a refused `pex`
  into a hard `engine/pex: refused: …` row (`denied`, lib.rs:333).
- Sensitivities: `perf::SensTable.rows: Vec<SensRow { param: Param::GroundC{net} | CouplingC{a,b} | …, d:
  Vec<Option<f64>> }>` (perf.rs:342–380); the run's tables are `plan.sens` (lib.rs:502). PERF-11 (M4) refines the
  ranking; until then rank by these GroundC rows (dag step-level).

### Edits
1. Merge (backend/verify/src/geom.rs): `pub fn build_store(shapes, pins, deck, strings, merge: &[u16])`. For each
   shape whose layer is in `merge`, collect per layer; per such layer: one-layer `GeometryStoreBuilder`, push rects
   as layer 0, `finish(1)`, `validate_layer_into(&tmp, GvLayerId(0), &mut raw)`, `merge_into(&raw, &mut merged)`
   (errors → `Err(format!("merge layer {l}: {e}"))`), then per merged polygon: no holes → push `outer()` ring coords;
   with holes → push each rect of `decompose_into` for that polygon (`poly_start` slice). Other layers as today.
   Checker: field `merge: Vec<u16>` set in `Checker::new` = `pdk.routing_metals` ∪ `pdk.layer("li")` ∪
   `pdk.layer("nwell")` (PERF-18 step 3 moved here), deduped; `load_geometry` passes it.
2. Options (backend/verify/src/lib.rs):
   `#[derive(Clone, Debug, Default)] pub struct ExtractOptions { pub field_solve: Vec<String>, pub unmerged: bool }`;
   `Checker::set_extract(&mut self, o: &ExtractOptions)` (stores `field_nets`; `unmerged` clears `merge`);
   `Checker::run` puts `field_nets.clone()` in `quasistatic_nets`.
   `pub fn signoff_extract(shapes, pins, reference, intent, opts: &ExtractOptions, pdk) -> Signoff`;
   `signoff_checked` = `signoff_extract(.., &ExtractOptions::default(), pdk)`. With `field_solve` non-empty:
   run as today with field nets cleared, harvest; then a timed `Checks { pex: true, others false }` run with the field
   nets → `Signoff.pex_field: Duration`, `caps`, `report.cost` from it. If its `summary.pex` is `Refused(why)`: keep the
   analytical caps from the first run and push `("pex.field", why)` to `coverage.skipped_rules` (no hard row).
   `standalone(.., checks, opts: &ExtractOptions)`; `pub fn drc_with(shapes, pins, pdk, opts) -> Vec<Finding>`;
   `drc` = `drc_with(.., &Default::default())`.
3. Net choice (frontend/library/src/lib.rs): `fn field_nets(sol) -> Vec<String>`: nets of the problem's
   `Differential` and `CommonNode` routing rows, plus the top 8 [policy] labelled nets by Σ_b |d_b|/scale_b over
   `GroundC{net}` rows of every `plan.sens` table (scale_b = |bound|, 1 for a zero bound, as `perf::miss`). `pub fn
   signoff_with(sol, pdk, opts) -> Signoff`; `library::signoff` = `signoff_with(sol, pdk, &ExtractOptions { field_solve:
   field_nets(sol), .. })` (final signoff: always). Promoted epochs: at the first promoted epoch run both; keep field
   solve for later epochs only if `pex_field ≤ 2 ×` the analytical pex-only time [measure]; print both times next to
   FLOW-09's `stage_ms` signoff slot.
4. `bench --pex-cal` (benchmarks/src/bench.rs `main`): per local fixture (dac4 incl. `top`, `b0..b3`) and signal
   net: C_unmerged (`unmerged: true`), C_merged (default), C_field (`field_solve` = all labelled signal nets) ground
   C → `target/bench/pex_cal.json`. signoff_xcheck.py (PERF-19) adds `c_magic` per row from magic's `TOP.spice` and
   reports |C_field − C_magic|/C_magic.

### Tests (backend/verify/src/lib.rs `mod tests`, existing `sky130()`/`rect` helpers)
- `partial_overlaps_extract_as_their_union`: A = rect met1 (0, 0, 10000, 500), B = rect met1 (9000, −250, 3000, 1000),
  pin `a` on A; `signoff_checked`; row `("a", None, c)`: `(c − 1.2482).abs() / 1.2482 <= 1e-3`. With `ExtractOptions
  { unmerged: true, .. }` via `signoff_extract`: `1.3828 ± 0.1 %` (proves the merge, not the deck, moved it).
- `a_ring_loop_keeps_its_area`: met1 loop outer 10 µm, inner 8 µm, drawn as left/right 1×10 µm and bottom/top
  10×1 µm (overlapping corners: drawn area 40 µm², true 36 µm²), pin `a`. Merged C(a) vs `unmerged: true` C(a):
  assert `c_unmerged − c_merged >= 4.0 · 25.78e-3 fF` (the 4 µm² double-counted area alone; merged fringe 80 µm ≤
  unmerged 88 µm can only widen the gap). Fringe not asserted exactly (slab edges inside the loop, plan's ceiling).
- `field_solve_changes_only_selected_nets`: three parallel met1 lines `a`,`b`,`c` (each labelled), field on `a`:
  C(a) differs from analytical by > 0.1 %, C(c) ground row equal within 1e-9.
- `a_refused_field_solve_falls_back_with_coverage`: `field_solve: vec!["nosuchnet"]` → no `engine/pex` hard row,
  `skipped_rules` has `("pex.field", _)` containing "no extracted net", caps non-empty.
- benchmarks/tests/signoff_fixtures.rs `check()`: after `raw`, `let unmerged = verify::drc_with(&shapes, &[], &pdk,
  &ExtractOptions { unmerged: true, .. })` errors; `assert!(raw.len() <= unmerged_len, "{name}: merging added DRC
  findings")` (`merging_does_not_add_drc_findings`, inside the existing fixture loop).
- Commands: `qcargo test -p verify`, `qcargo test --release -p benchmark --test signoff_fixtures` (DRC baselines must
  hold unchanged; a rise is reported, not absorbed), `qcargo run --release -p benchmark --bin bench -- --pex-cal`,
  then the nix-shell harness for C_magic. Acceptance ≤ 15 % per signal net ≥ 1 fF is reported with numbers; a miss is
  stated, never thresholded away. bench `C total` on ota drops (baseline moves once; record old/new).
