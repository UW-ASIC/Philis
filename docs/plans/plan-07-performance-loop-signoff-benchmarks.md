# PERF — Performance-driven loop, signoff completeness, and proof against hand layout

Plan ID prefix: `PERF-NN`. Code citations are `path:line` on the working tree of 2026-09-28 (uncommitted changes
included). Reference citations are `<source> §/eq, reftext <file>.txt L<a>–<b>`; the reftext directory is
`/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/`.
Study-document IDs (`AV-`, `AF-`, `AR-`, `AT-`, `AA-`, `AC-`, `AP-`, `LAMP-`, `GRAEB-`, `SURV-`, `MM-`, `CC-`, `BAL2-`,
`NOTES-`, `H13-`) refer to the documents in this directory. `GP/` = `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/`
(the rev `Cargo.lock` pins). `VOL/` = `~/.volare/sky130A/libs.tech/`.

Numbers marked **[policy]** are Philis choices with the stated rationale, not source values. Numbers marked
**[measure]** must be measured before they gate anything.

---

## 0. Scope & boundaries

### 0.1 Owned here

1. **Sensitivity engine** (`frontend/library/src/perf.rs`): finite-difference sensitivities of every spec to ground C,
   coupling C, series R and per-device threshold offset, at every active scenario, with step control and bounded
   concurrency.
2. **Statistical layer** (new `frontend/library/src/robust.rs`): σ of every spec from device mismatch, signed
   worst-case distance β per spec bound, yield partitions, the statistical reserve taken out of layout headroom, and
   the per-device mismatch allowance exported to the matching rules.
3. **The performance loop** in `frontend/library/src/lib.rs`: which epochs are simulated, the spec and C tiers of the
   epoch key, sensitivity refresh, post-fill re-evaluation, the Pareto report, and limiting-spec feedback into net
   weights.
4. **Simulation fidelity**: the operating-point and performance decks (`frontend/library/src/oppoint.rs`, `perf.rs`)
   must simulate the drawn circuit (size convention on the card, every device kind, correct rails, scenarios).
5. **Post-layout Monte Carlo** of finalists and the yield report.
6. **Signoff completeness** (`backend/verify/src/*`): no silent skips, warnings separate from errors, honest coverage,
   PEX fidelity (merged metal, field-solved critical nets), design intent for IR, and the *content* of the deck rules
   signoff needs (MOS bulk, wells as nets, BJT recognisers).
7. **Independent signoff** against foundry decks (KLayout DRC/LVS, magic DRC and extraction).
8. **Benchmark suite and metrics** (`benchmarks/*`), including the **hand-layout comparison harness**.

### 0.2 Consumed from other plans

| Needed | From (item) | Used by |
|---|---|---|
| One device-size convention: `pnr_core::MosSize`, `Device::mos_size()`, `Device::gate_area_um2()`; the FET card `W={w_total_um} L={l_um} nf={nf} m={m}` in `oppoint::flat_circuit_with` (AF-01, AR-46) | FLOW-01 | PERF-08, PERF-13 |
| Parser: `Netlist.ports`, `insts`, `sources`, R/C/L values `r_mohm`/`c_af`/`ind_ph`, `Q` substrate terminal, `/`→`__` instance names (AF-02, AF-10) | FLOW-07 | PERF-03, PERF-08, PERF-21 |
| Epoch-key contract: `lex_key` counts rules not batches, `warn/` interim, NaN-last `key_lt` (AF-12, AF-29, AV-10) | FLOW-02 | PERF-01, PERF-06, PERF-07, PERF-14 |
| Vendored, embedded decks with `# PHILIS:` edit markers (AV-21, audit-06 §5 Q2) | FLOW-04 | PERF-18 |
| `pex_row` kept-operand rule, `pex rbody_po`, `device_sheet_ohm`, `density_rules` reading `density_cmp` (AV-16, AV-18 reader half) | FLOW-05 | PERF-04 (checker half stays here) |
| Match-class tiers `Pdk::tier_nm(key, MatchClass)`: `wpe_clearance_nm`, `lod_moat_ext_nm`, `dummy_poly_reach_nm` | FLOW-06 | PERF-23 |
| Loop-owned `Weights { place, route }` and `route_weights(place, classes, sens)` | FLOW-08 | PERF-15 |
| `RunStats.stage_ms: [u64; 9]` (gp, dp, rings, gr, dr, diodes, signoff, metadata, perf); `performance_rows` reusing the first topology's classes | FLOW-09 | PERF-20, PERF-09 |
| Post-fill signoff on the final geometry and its `CapMatrix` (`MetadataReport.post_fill`) | FLOW-10 | PERF-15, PERF-22 |
| Labelled GDS `library::gds(sol, pdk) -> Result<Vec<u8>, String>`; `Config.interface` (AF-18) | FLOW-12 | PERF-19, PERF-20, PERF-21 |
| CI workflow with a nightly tool job; `library::tools::{tool_or_skip, sky130_models}` / `PHILIS_REQUIRE_TOOLS=1` (landed in M0) | FLOW-14 | every ngspice/KLayout/magic test here |
| One rail classifier `annotator::netrole::rail_of` (node `0`, `gnd!`, `vee`, VPB/VNB) (AA-11) | EXT-02 | PERF-07, PERF-09 |
| `annotator::evidence::{Evidence, Sensitivities, SpecSens}` input type and `annotate_with` | EXT-17 | PERF-12 |
| Sensitivity-driven allocation per matched set (`allocate`) and budget rows (`annotator::budget::rows`) | EXT-21, EXT-25 | PERF-12 (PERF supplies their inputs) |
| `MatchedSet` ledger with systematic terms (`mu_thermal`, `mu_lod`) and `LedgerRow` (MM-30) | MAT-04, MAT-13 | PERF-13, PERF-23 |
| `Macro.drawn` cards and `cellgen::drawn_cards`; `Pdk::extracts_params(kind)` probe (AV-19) | CELL-01 | PERF-02 |
| `Macro.figures`: `Figures.sd` (AS/AD/PS/PD per owner), `Figures.gate_ohm` | CELL-19 | PERF-26 |
| MIM capacitors drawn as the deck's `capm` device; MOM calibration consumer (AC-01, AV-32) | CELL-08 | PERF-02, PERF-16 |
| `DetailedCfg.r_weight`, `DetailedCfg.pair_weight`, `PerformanceBudget` stack C and R measurement | RTE-21 | PERF-12 |
| `dr::width_candidates(pre, rows, eta)` and `DetailedCfg::width_mult` | RTE-22 | PERF-28 |
| Antenna model agreeing with signoff (AV-15, AV-17), latch-up rules (AV-08), per-segment EM in the hard tier (AR-05, AF-24), op-voltage checks (AV-29) | REL-02, REL-06, REL-03, REL-10 | milestone gates, PERF-17 |

### 0.3 Provided to other plans

| Provided | To |
|---|---|
| `perf::SensTable` per scenario (∂f/∂C_ground, ∂f/∂C_coupling, ∂f/∂R, ∂f/∂V_T per spec) and its export `perf::to_evidence(..) -> annotator::evidence::Sensitivities` (PERF-12) | EXT-17/21/25 (allocation, budget rows, GRAEB-20 pair discovery), PLC-16 (rows via EXT-25) |
| `DetailedCfg.r_weight` and `pair_weight` filled from the table (PERF-12) | RTE-21 |
| `PerformanceBudget.coupling` term and `limit` field (change request on the RTE-owned struct, PERF-06/PERF-12) | RTE-21, EXT-25 |
| σ_f, β, Φ(β) per bound (PERF-13) | EXT-17 (`SpecSens.sigma_f`), EXT-21 |
| `OpPoint.vgs_v/vds_v/vbs_v` per FET and `Solution.op` (PERF-09) | REL-10 |
| Per-set SPICE Monte Carlo σ(ΔV_GS) runner `perf::pair_sigma_mc` (PERF-27) | MAT-21 |
| Top-η width-vector simulation (PERF-28) | RTE-22 |
| Analog layout-rule check on the final geometry, Hastings §13.3 rules 12/16/17/19/21/23 (PERF-23) | CELL-12 acceptance, MAT, RTE-16 |
| `verify::Signoff` (report, warnings, coverage, caps) | FLOW (CLI output), everyone (tests) |
| Bench `results.json`, MAT-13 ledger table, REL report columns, the hand-comparison table | all plans' acceptance criteria |

### 0.4 Explicitly not here

Router pricing and wire sizing (RTE), placement-tier performance cost and MST estimators (PLC, LAMP-04/07/08),
matching estimators and the single offset budget per set (MAT, MM-30/31), antenna/EM/latch-up/substrate models (REL),
fill inside the loop and Θ weighting (FLOW, AF-12/AF-13), generator correctness (CELL). Also not here, because another
plan's item already does it: `pex_row` and the resistor-body sheet (FLOW-05), the NaN-safe `key_lt` (FLOW-02 step 5),
per-stage timers (FLOW-09 step 4), post-fill signoff (FLOW-10 step 4), budget-row construction and per-set allowance
allocation (EXT-25, EXT-21), `PerformanceBudget` ground-C/R measurement (RTE-21 step 3), per-epoch net-weight feedback
(PLC-17, RTE-21 step 5), LVS resistor/diode parameters (CELL-01 step 7). See "Cut or deferred items".

---

## 1. Audit digest — current state

### 1.1 Ground truth (audit-06 §G, run 2026-09-28)

- `cargo build --release`: 2.4 s, 0 warnings. `cargo test --release --workspace`: 331 passed, **1 failed**
  (`signoff_fixtures::fixtures_sign_off_within_baseline`, dac4 `erc/ar.met2.1` at `feedback_iters = 1`,
  benchmarks/tests/signoff_fixtures.rs:139), 2 ignored.
- `bench local`: 539.5 s, 10/10 circuits DRC 0 and "LVS MATCH". Three of those MATCH verdicts compared nothing or
  omitted devices: bjt_mirror (2/2 devices skipped), bgr_core (9/9), dac4 (all five capacitor cards removed
  silently: Σm = 1+1+2+4+8 = 16 units per dac4.spice:8–12; audit-06 G.2 counts 15, omitting the dummy C0).
- ota, ota_constrained and tt_ota produce bit-identical rows; the constraint table's matching rows all come from that
  one OTA.

### 1.2 Performance loop

| # | Fact | Where |
|---|---|---|
| P1 | Post-layout scoring is opt-in; default `None`; bench and CLI never set it. | frontend/library/src/lib.rs:53, 74; benchmarks/src/bench.rs:180 |
| P2 | Sensitivities: one baseline + one run per Signal/Sensitive/Clock net with +10 fF ground C, forward difference, computed once at the schematic point. | lib.rs:196–231 (step lib.rs:215–216); perf.rs:200–223 |
| P3 | One OS thread and one ngspice process per net, all spawned at once. | perf.rs:202–211 |
| P4 | A spec with both bounds keeps only the floor row (`(Some(lo), _)` arm). | perf.rs:241–245 |
| P5 | A spec without schematic headroom gets no row (only an `eprintln!`). | perf.rs:246–248; lib.rs:219–222 |
| P6 | Budget rows price `length × af_per_nm` of the first routing layer at minimum width, not the stack's C; no coupling, no R. | kernel/analog/src/routing/performance.rs:31–33; lib.rs:204, 218 |
| P7 | Spec tier = Σ normalised miss, `0` once every bound is met: no robustness beyond the bound. | perf.rs:88–95, 185 |
| P8 | A failed simulation is a full miss per spec with only an `eprintln!`. | lib.rs:878–893 |
| P9 | `key_lt` compares f64 with `<`; a NaN incumbent at equal `\|V\|` is never displaced (fixed by FLOW-02 step 5). | lib.rs:929–939 |
| P10 | C tier = signoff `Report.cost` = Σ over all nets of ground + coupling C, coupling counted on both nets, supplies included. On ota 79 % of 616 fF is supply-related; the vbn–VSS coupling alone is 29 %. | lib.rs:955; backend/verify/src/lib.rs:182; backend/verify/src/checker.rs:317–320; audit-06 G.4(a) |
| P11 | Fill is added after winner selection; the reported `metadata.performance` is pre-fill. | lib.rs:410–420, 432–438 |
| P12 | `certified()` ignores the bias provenance (a synthesised probe certifies). | frontend/library/src/metadata.rs:106–110 |
| P13 | No σ, β, yield, Monte Carlo, corner or temperature list anywhere in the flow; Monte Carlo exists only in the deck-characterisation script. | benchmarks/characterize_mismatch.py:74–103 |
| P14 | No per-stage timing or simulation count in `RunStats`. | lib.rs:104–124 |

### 1.3 Simulation fidelity

| # | Fact | Where |
|---|---|---|
| S1 | FET card is `W={w} L={l} nf={nf}`; `m` is dropped. Layout and LVS draw `max(nf,m)` fingers each `w` wide. XM1 of ota.spice is simulated at half its drawn width, XM5 at a quarter. | oppoint.rs:270–286; cellgen.rs:879–884; audit-07 AF-01 |
| S2 | Every non-FET is skipped (`model_for` returns `None`), contrary to the comment; non-FETs report zero terminal current. | oppoint.rs:140–147, 262–264, 45–47 |
| S3 | `parse_show` drops a column whose truncated header lost its `.` and shifts later values onto the wrong devices; one unresolved FET empties `pin_currents` for the whole circuit. | oppoint.rs:435; lib.rs:1035–1038 |
| S4 | Rails by fixed name lists that differ from the annotator's; node `0` is ground here but not there; one `vdd` for every supply; gate-only nets at `vdd/2`. | oppoint.rs:371–376, 392–398; annotator netrole.rs:18–19 (audit-07 AF-04) |
| S5 | Deck path `temp_dir/philis_op_{pid}/op.spice` per process (races between concurrent runs); decks never removed; ngspice writes `bsim4v5.out` into the caller's cwd. | oppoint.rs:157–160; perf.rs:170–174 |
| S6 | Perf deck = schematic FETs + one `Cpex` per matrix entry + per-terminal `Rpex` + equivalent `sa/sb`; no AD/AS/PD/PS, one corner, one temperature. | perf.rs:99–149; oppoint.rs:103–121 |
| S7 | `verify::extract_spice` exists but GPurify's R is a per-polygon chain in PolyId order (non-physical terminal R). Keeping the schematic-plus-overlay deck with Philis's terminal-resolved R is therefore the right model; it must be completed, not replaced. | backend/verify/src/netlist.rs:25–53; GP/crates/extract/src/analytical.rs:341–417; audit-06 §2.6 |

### 1.4 Signoff

| # | Fact | Where |
|---|---|---|
| V1 | A reference device with no deck recogniser is skipped and counted; the count only reaches `eprintln!`. | backend/verify/src/reference.rs:92–97; backend/verify/src/lib.rs:108–116 |
| V2 | The library drops every capacitor and inductor before LVS; only MOS carry params (W/L). | cellgen.rs:870, 879–891 |
| V3 | For kinds without polarity (R, C, D) an unknown model falls back to the first recogniser of the kind (a MOM card would be compared as `capm`). | reference.rs:148–173 |
| V4 | `floating_gate`/`unconnected_pin` rows are dropped on every labelled net; the library labels every net with a placed pin and declares it a port; the deck says `labels_are_ports: false`. ota's undriven `vbias`/`vbn` pass ERC. | checker.rs:176–200; lib.rs:1368, 1376–1410; GP/pdks/sky130.deck:470; benchmarks/fixtures/ota.spice:5–7 |
| V5 | Warnings (`tie_high_low`, `density_cmp`) become hard violations. | backend/verify/src/lib.rs:150–163 |
| V6 | `density_cmp` is neither stripped nor deferred; it examines zero windows and reports `Ran`. | checker.rs:43–47, 262–281; audit-06 AV-18 |
| V7 | The skipped-rule list is printed once per process (the first circuit's list). | backend/verify/src/lib.rs:176–181 |
| V8 | The label-short fallback returns no capacitance matrix. | backend/verify/src/lib.rs:118–126 |
| V9 | `pex_row` answers through subtracted operands: `sheet_ohm("rpoly")` returns `poly_c`'s 48.2 Ω/□ (the complement of the resistor body); for `licon` the highest-id of five equal-rank cut rows (152–585 Ω/cut): `licon_po`, 152 Ω, declared last (derived ids follow declaration order). Fixed by FLOW-05 step 4 (see Cut or deferred items for a defect in that rule). | backend/verify/src/pdk.rs:661–674 (the kept-operand rule exists at pdk.rs:758–770); GP/pdks/sky130.deck:110–114, 583–587; GP/crates/ingest/src/deck/mod.rs:354–355 ("derived layers follow in declaration order") |
| V10 | `Intent` = supplies + one DC current per supply net; `ir_drop`, `vgs.*`, `vds.*`, `esd.pad` skip. GPurify's intent accepts `max_drop_mv` per net, unused. | backend/verify/src/lib.rs:57–63; checker.rs:101–133; GP/crates/ingest/src/intent.rs:149–154 |
| V11 | Shapes enter GPurify one rectangle each; unmerged metal ground C is 1.13–1.73× the unioned value on the fixtures. Lateral and inter-layer coupling are fringe-free parallel plate; ground C is area plus perimeter fringe from the deck `pex` row. | backend/verify/src/geom.rs:39–42; audit-06 G.4(b); GP/crates/extract/src/analytical.rs:84 (coupling), 55–67 (ground) |
| V12 | GPurify has a BEM field solver for named nets (`RunOptions.quasistatic_nets`, overlaid on the analytical network); Philis always passes an empty list. | checker.rs:163–168; GP/src/engine/run.rs:338–399 |
| V13 | sky130 deck: MOS recognisers are 3-terminal, the n-well is not a net, no BJT recogniser, no latch-up rule. | GP/pdks/sky130.deck:542–550, 663–670 |
| V14 | The KLayout/PEX cross-checks crash on the current sidecar format; the self-test and `drc_gds` example panic; the LVS cross-check ignores W/L (100 % tolerance). KLayout is in the flake dev shell; magic and netgen are not installed; the foundry decks are. | benchmarks/xcheck.py:93–111; xcheck_pex.py:53–54; examples/drc_gds.rs:81; xcheck_lvs.py:79–82; flake.nix:84–85; VOL/klayout/{drc/sky130A_mr.drc,lvs/sky130.lvs}, VOL/magic/sky130A.tech |

### 1.5 Benchmarks

| # | Fact | Where |
|---|---|---|
| B1 | 10 local fixtures ≤ 14 devices; three copies of one OTA; `*.interface.json` ignored. | benchmarks/fixtures/; bench.rs:13 |
| B2 | One seed, no performance columns, bias from the synthesised probe ("not a sign-off condition"). | bench.rs:135–151, 392–395 |
| B3 | Area is the bbox of device half-extents, not of the GDS; WL sums supply nets; "LVS MATCH" whenever no `lvs/` row exists. | bench.rs:104–117, 196–203, 211–214 |
| B4 | The TinyTapeout hand GDS paths are discovered and marked `dead_code`; MAGICAL `*_test.sp` testbenches are discovered as circuits; an unknown suite word becomes `All` (network clone). | benchmarks/src/fixtures.rs:51–54, 233–247, 158–194, 68–76 |
| B5 | Fixture comments state known optima (pair, quad, chain4, bgr_core) that no code checks. | benchmarks/fixtures/{pair,quad,chain4,bgr_core}.spice:1–3 |
| B6 | Constraint table grades the flow against its own rule models (self-referential). | bench.rs:259–288 |

---

## 2. Target — what "better than hand layout" means here, measurably

A result counts only under a **declared judge**: the same extraction, the same testbench, the same models and the
same scenarios for the Philis layout and the reference (ref-00 §2.9 topic-validation "Fair comparisons, seeds, cache
keys (L74–82)", ref-00-prior-notes-handbook.md:331 — no NOTES entry carries it; NOTES-57 is the discriminating
validation catalog; Graeb §4.8.1 eq. 114, graeb_centering.txt L3370–3376). Metric definitions:

| ID | Metric | Definition | Target |
|---|---|---|---|
| T1 | Foundry DRC | error count of VOL/klayout/drc/sky130A_mr.drc and magic `drc(full)` on the emitted GDS | 0 on every local fixture and every hand-comparison circuit |
| T2 | Foundry LVS | VOL/klayout/lvs/run_lvs.py against the schematic | match on every circuit whose devices the foundry deck extracts |
| T3 | GPurify coverage | unverified devices + vacuous rules in `verify::Coverage` | 0 on sky130 fixtures after PERF-18; reported (never hidden) before |
| T4 | Spec compliance | every spec bound met at every declared scenario, post-layout | 100 % of bounds |
| T5 | Robustness | min over bounds of β (Graeb eq. 300/301, graeb_centering.txt L6079–6141), β = 3 is "three-sigma design" (L4266–4281) | min β ≥ 3 where the schematic itself has β_schem ≥ 3 + ε; otherwise the loss β_schem − β_post ≤ hand's loss; a bound counts only if its worst-case simulation passes (PERF-29, `wc_pass`) |
| T6 | Degradation vs schematic | per metric, post-layout / schematic value ([WS] §6 specs are the schematic values, todaes22.txt L594–610) | ≥ the hand layout's ratio on every metric of every hand-comparison circuit |
| T7 | Offset | systematic \|μ_os\| from the nominal extracted simulation; σ_os from post-layout Monte Carlo | \|μ_os\| ≤ hand's; σ_os ≤ hand's × (1 + 1/√(2(N−1))) (statistical tolerance of a σ estimate from N samples, characterize_mismatch.py:142) |
| T8 | Matched-net parasitics | max over matched net pairs of \|ΔC\|/C (ground + per-aggressor coupling) | ≤ hand's |
| T9 | Area | bbox of the emitted block cell | ≤ 1.00 × hand's (LAYLA reported a 10–15 % penalty against manual layout, Lampaert §7, lampaert.txt L7094–7114) |
| T10 | Yield agreement | abs(Φ(β)-model yield − Monte Carlo yield) | ≤ 0.03 + 3σ_Ŷ (Graeb §5.5.1 1–3 % absolute, graeb_centering.txt L4634–4685; σ_Ŷ eq. 226, L4966–5035) |
| T11 | Constraint-awareness gain | A/B: performance mechanism on vs off, 5 seeds | on is never worse in median on any spec (Lampaert comparator 6.9 → 3.7 mV offset, 5.4 → 2.8 ns delay, Table 4.1 lampaert.txt L5006–5012; §4.12.1 text L5020–5082) |
| T12 | Runtime | wall time per circuit and per stage (`RunStats`) | reported; no regression > 20 % **[policy]** against the stored `results.json` baseline |

---

## 3. Work items

### PERF-01 Signoff outcome: errors, warnings and coverage kept apart

Status: done in M0 (`5478230`). As built: `verify::Signoff { report, warnings, coverage, caps, elapsed }` (backend/verify/src/lib.rs:61); `verify::Finding` gains `warning` and a margin unit (nm | permille); the CLI is not CLEAN while any device is LVS-unverified.

- Priority: P0. Effort: M. Depends on: — (FLOW-02 may land first with its interim `warn/` prefix, FLOW-02 step 4;
  this item replaces that prefix).
- Why: AV-10 (warnings are hard), AV-26 (skip log once per process), AV-25 (label-short fallback loses caps),
  AV-20 (skip list misreports), AV-31 (non-length margins all read 1), NOTES-03 (structured coverage certificate),
  NOTES-02 (unknown never passes).
- Current: every violation row, error or warning, becomes a hard `Violation` (backend/verify/src/lib.rs:150–163);
  skipped rules go to `eprintln!` once per process (lib.rs:176–181); the reference-skip count goes to `eprintln!`
  (lib.rs:108–116); the label-short branch harvests without setting `caps` (lib.rs:118–126). The epoch key counts
  `signoff.hard_violations.len()` (frontend/library/src/lib.rs:952).
- Change:
  1. `backend/verify/src/lib.rs` — new public types and entry point; `signoff_with_intent` becomes a wrapper
     returning `(s.report, s.elapsed, s.caps)` so `frontend/library/src/lib.rs:1291, 1345` and, through
     `signoff_with_caps` (backend/verify/src/lib.rs:71–78), `kernel/cells/src/cap_array.rs:780` keep compiling:
     ```rust
     /// Everything one signoff concluded.
     pub struct Signoff {
         /// Errors only: DRC/ERC/LVS rows of `Severity::Error`, `engine/…`, `lvs-coverage/…` (PERF-02).
         pub report: Report,
         /// Rows the deck states as warnings. Never in `report.hard_violations`.
         pub warnings: Vec<Violation>,
         pub coverage: Coverage,
         pub caps: CapMatrix,
         pub elapsed: Duration,
     }
     #[derive(Clone, Debug, Default)]
     pub struct Coverage {
         /// Schematic devices LVS did not compare: `(kind, model hint, count)`.
         pub unverified: Vec<(RefKind, Option<String>, usize)>,
         /// Rules this run did not execute, `(rule, reason)`: deck skips, waivers, chip-level deferrals.
         pub skipped_rules: Vec<(String, String)>,
     }
     pub fn signoff_checked(shapes: &[Shape], pins: &[LabeledPin], reference: &RefInput,
                            intent: &Intent, pdk: &Pdk) -> Signoff;
     /// Pure split used by `harvest` (testable without a deck).
     fn split_by_severity(rows: impl Iterator<Item = (Violation, Severity)>) -> (Vec<Violation>, Vec<Violation>);
     ```
  2. `harvest` reads `out.violations.severity[i]` (the column `checker.rs:194` already reads) and routes
     `Severity::Error` to `report.hard_violations`, anything else to `warnings`. Delete the `static SKIPPED: Once`
     block; `coverage.skipped_rules = checker.skipped_rules()` per call (checker.rs:248–256).
  3. `reference::build` returns `(Netlist, Vec<usize>)` (indices of skipped reference devices) instead of a count
     (reference.rs:61–133); `Checker::set_reference` returns `Vec<(RefKind, Option<String>)>`; `signoff_checked`
     aggregates them into `coverage.unverified`. Delete the `eprintln!` at lib.rs:111–116.
  4. Label-short branch: after the label-free re-run, `caps = checker.cap_matrix()` as in the normal branch.
  5. `frontend/library/src/lib.rs`: `signoff_shapes` (lib.rs:1335–1347) returns `verify::Signoff`; the epoch key
     counts `report.hard_violations` only (if FLOW-02 landed, delete its `!v.rule.starts_with("warn/")` filter and
     the `warn/` prefixing in `harvest`: the split makes both dead); `RunStats` gains `warnings: u32` (FLOW-02 step 4
     declares the same field and type; whichever lands second reuses it; `RunStats` stays `Copy`); `library::signoff` returns
     `verify::Signoff` (callers: `benchmarks/src/bench.rs:189`, `benchmarks/tests/signoff_fixtures.rs:82, 191`,
     `benchmarks/examples/net_dump.rs:27`, `examples/op_demo/src/main.rs:56`, `frontend/cli/src/main.rs:54`,
     `frontend/library/tests/{flow_smoke.rs:42, antenna_diode.rs:53, extra_devices.rs:13}`). `MetadataReport` gains `pub coverage: verify::Coverage` and `certified()` requires
     `coverage.unverified.is_empty()`; `Display` prints the skipped-rule list once per report.
  6. AV-20 is closed by documentation, not by filtering: `lvs.device_count_mos`, `lvs.device_count_bjt` and
     `lvs.parametric` are GPurify's layout-only range checks, which it records as `Skipped(NotInDeck)` because the deck
     carries no limits (GP/crates/check/src/lvs/checks.rs:72–80); the reference parameter comparison runs under a
     different id, `lvs.parameter_mismatch` (checks.rs:24, 97–98). They stay in `coverage.skipped_rules`; `Coverage`'s
     `Display` appends "(range limits; reference parameters are compared as lvs.parameter_mismatch)" to those three.
  7. AV-31: `shortfall_nm` (lib.rs:27–34; callers lib.rs:161, 239) becomes `pub fn shortfall(limit: Measurement,
     measured: Measurement) -> i64`. `(Length(l), Length(m))` stays `(l − m).max(0)` nm: the existing test pins
     "a graze cannot go negative" (lib.rs:291), and the direction of a maximum rule is the rule's `LimitSense`, which a
     violation row does not carry (GP/crates/check/src/report/measure.rs:22–24). Any other same-variant numeric pair
     (`Area`, `Ratio`, `Count`, `Voltage`, `Current`, `Resistance`) → `ceil(1000·|m − l| / max(|l|, 1e-12))` (‰ of the
     limit) instead of 1; mismatched variants → 1. The doc comment states both units; the `Count` assertion at
     lib.rs:292 becomes 1000. Only reports read signoff margins (the key counts rows), so ranking is unchanged.
- Tests (`backend/verify/src/lib.rs` tests module):
  - `split_by_severity_keeps_warnings_out_of_the_hard_tier`: three rows (Error, Warning, Error) → 2 hard, 1 warning.
  - `a_skipped_reference_device_is_listed_in_coverage`: the existing two-device reference with an NPN
    (lib.rs:380–424) → `coverage.unverified == [(RefKind::Npn, _, 1)]`.
  - `skipped_rules_are_reported_on_every_run`: two consecutive `signoff_checked` calls on different geometry both
    return a non-empty `coverage.skipped_rules` on sky130 (ir_drop has no intent).
  - `the_label_short_fallback_still_returns_caps`: two labels on one shorted met1 plate → `lvs/extract: label short`
    row and `caps` non-empty.
  - `shortfall_reads_ratio_overshoot_in_permille`: `shortfall(Ratio(400.0), Ratio(800.0)) == 1000`,
    `shortfall(Ratio(400.0), Ratio(404.0)) == 10`, `shortfall(Count(1), Count(0)) == 1000`; the three existing
    `Length` assertions (lib.rs:290–291, 314) unchanged.
- Acceptance: `cargo test -p verify` green; bench prints warnings and skipped rules per circuit; no warning counts
  toward `|V|`.
- Risks: none on the split itself: GPurify's `Severity` has exactly two variants, `Warning` and `Error`
  (GP/crates/check/src/report/violation.rs:11–16), and checker.rs:194–197 already matches `Error` vs `_`.

### PERF-02 LVS reference completeness: every device either compared or declared unverified

Status: done in M0 (`6b28fca`); amended (`d682829`, step 3 below): `lvs-coverage/` rows are out of the epoch's |V| but block `certified()`.

- Priority: P0. Effort: M. Depends on: PERF-01. Coordinates with CELL-01 (which replaces schematic cards by
  `Macro.drawn` cards for resistor segments, MIM units, diode and BJT units, and owns LVS parameters for them).
- Why: AV-01 (critical: LVS silently skips BJTs, library drops caps), AV-28 (the baseline admits an ERC count, not a
  named finding), NOTES-03, topic-verification "recogniser omissions must be visible" (ref-00 §2.9).
- Current: `cellgen::reference` `continue`s on `Inductor | Capacitor` (frontend/library/src/cellgen.rs:870) and passes
  params for MOS only (cellgen.rs:879–891); `recogniser_for` falls back to the first recogniser of a polarity-less
  kind when the model hint names no deck model (backend/verify/src/reference.rs:165–171), so a MOM card would be
  compared as `capm`; bench prints `LVS MATCH` whenever no `lvs/` row exists (benchmarks/src/bench.rs:196–203);
  the test baseline asserts LVS clean for bjt_mirror and bgr_core (benchmarks/tests/signoff_fixtures.rs:28–31).
- Change:
  1. `cellgen::reference`: the `Inductor | Capacitor => continue` arm (cellgen.rs:870) — which CELL-01 step 8 keeps
     for devices its `drawn_cards` did not replace — emits cards instead of dropping them: capacitors
     `RefKind::Capacitor` (terminals `P N`, model = the schematic model, one card per unit: `fingers(dev)` cards,
     cellgen.rs:710–713, i.e. `max(nf, m)` — dac4's XC4 `m=8` gives 8), inductors a new `RefKind::Inductor` (terminals `P N`;
     backend/verify/src/reference.rs `RefKind` gains it; `recogniser_for` maps it to no `DeviceKind` and returns
     `None`). Devices CELL-01 replaced (MIM units) never reach this arm. Verify, not the library, decides what is
     unverified, so there is one place that counts.
  2. `recogniser_for`: for `polarity.is_none()` kinds (R, C, D, inductor) there is **no fallback**; a model hint
     that matches no row (exact or `__vendor` suffix, reference.rs:161) returns `None`. MOS/BJT keep today's fallback.
  3. `signoff_checked` (PERF-01): for each `(kind, model, n)` in `coverage.unverified` push
     `Violation { rule: format!("lvs-coverage/unverified:{kind:?}:{}", model.as_deref().unwrap_or("-")), margin: n as i64 }`
     into `report.hard_violations`. It is a constant offset across all epochs of a run, so ranking is unchanged; it
     blocks `certified()` and every "beats hand layout" claim (T3).
     **Amended (M0, `d682829`; recorded by the M0 review panel):** `lex_key` drops the `lvs-coverage/` rows from the
     epoch's |V| (an exception to PERF-01 step 5, "the epoch key counts `report.hard_violations` only"). Counted,
     bjt_mirror, bgr_core and dac4 could never read feasible or converged, and the winner's debug connectivity check
     (`key.0 == 0`) never ran on them. So `RunStats::converged` means "no layout-fixable hard row", not
     LVS-complete; only `certified()` claims LVS-complete. `uncompared_devices_do_not_block_convergence` pins it. An
     undrawable device (`cell/undrawable`, CELL-04) is not covered by this exception: the epoch counts it in |V|.
  4. (Resistor/diode/BJT LVS parameters are CELL-01 step 7's `Pdk::extracts_params(kind)`; not here.)
  5. Bench LVS column: `MATCH` | `MISMATCH` (any `lvs/` row) | `PARTIAL(n)` (only `lvs-coverage/` rows, n = Σ margins).
  6. AV-28: known-false ERC findings are named, not counted. `BASELINE` carries `expected_erc: &[&str]`, the exact
     rule strings the fixture is allowed to produce (bjt_mirror and bgr_core: `["erc/supply_short:…"]` as `harvest`
     formats it, with the comment at signoff_fixtures.rs:22–27 kept as the reason). Any ERC row not in the list, or a
     listed row that disappears, fails the test, so the GPurify fix is noticed the day it lands.
- Tests:
  - `benchmarks/tests/signoff_fixtures.rs`: `BASELINE` becomes `(name, max_drc, expected_erc, lvs_clean, unverified)`
    where `unverified` is computed by the test as Σ `fingers(dev)` over devices whose kind has no sky130 recogniser
    (BJTs, capacitors), not a literal; assert `report` has no `lvs/` row and the `lvs-coverage/` margins sum to it
    (bjt_mirror 2 = 1 + 1 per bjt_mirror.spice:3–4; bgr_core 9 = 1 + 8 per bgr_core.spice:4–5; dac4 = Σ `m` of the
    five capacitor cards, dac4.spice:8–12).
  - `backend/verify` `a_mom_card_is_unverified_not_compared_as_mim`: a `RefKind::Capacitor` with model
    `cap_generic_m1m2` on sky130 → unverified, no `lvs/` row.
  - `backend/verify` `an_inductor_is_unverified`: one `RefKind::Inductor` card → `coverage.unverified ==
    [(RefKind::Inductor, _, 1)]`, no `engine/` row.
  - `signoff_fixtures`: a synthetic extra ERC row (append `Violation { rule: "erc/floating_gate:poly", .. }` to a
    copy of the report) makes the baseline comparison fail (the named-list check is itself tested).
- Acceptance: G.3 rerun prints `PARTIAL(2)`, `PARTIAL(9)`, `PARTIAL(16)` (five `cap_generic_m1m2` MOM cards, Σm = 1+1+2+4+8, dac4.spice:8–12) for bjt_mirror, bgr_core, dac4 until
  PERF-18/CELL close them; every other fixture `MATCH`.
- Risks: dac4's unit cards may exceed what the generator draws if CELL changes unitization; the test computes the
  expected count from the netlist, so it tracks that.

### PERF-03 Floating-gate exemption only for declared ports

Status: steps 1–2 done in M0 (`2f37409`): `RefInput.external_ports` and the `Checker` filter exist, and with `None` the blanket exemption stays and is listed in `coverage.skipped_rules`. Steps 3–4 move to M1 after FLOW-07: on m0 `cellgen::reference` builds every `RefInput` with `external_ports: None` (frontend/library/src/cellgen.rs:932), and step 3's assignment goes in `labels_and_reference` (frontend/library/src/lib.rs:1600) after `reference.ports` is set.

- Priority: P0. Effort: S. Depends on: FLOW-07 (`Netlist.ports`, AF-02); PERF-01 (`coverage`).
- Why: AV-04 (every labelled net is exempt, overriding `labels_are_ports: false`), AV-30 (flagship OTA gates on
  undriven `vbias`/`vbn`).
- Current: `drop_port_floating` exempts any `unconnected_pin`/`floating_gate` row whose net has a port name
  (backend/verify/src/checker.rs:176–200); the library names and declares a port for every net with a placed pin
  (frontend/library/src/lib.rs:1368, 1376–1410); ota.spice:2 declares ports `vinp vinm vout1 vout2 VDD VSS` while
  M3/M4/M5 gate on `vbias`/`vbn` (ota.spice:5–7).
- Change:
  1. `RefInput` gains `pub external_ports: Option<Vec<String>>` (backend/verify/src/reference.rs:48–52).
  2. `Checker` stores `external: Option<Vec<StrId>>` set in `set_reference`; `drop_port_floating` exempts a row only
     if the net's label is in `external` (when `Some`); when `None`, today's behaviour stays and
     `coverage.skipped_rules` gains `("floating_gate", "exempt on every labelled net: no port list")`.
  3. Library: `labels_and_reference` (lib.rs:1351–1370) sets
     `external_ports: (!netlist.ports.is_empty()).then(|| netlist.ports.iter().map(|n| netlist.nets[n.0 as usize].name.clone()).collect())`
     — an empty `ports` (no `.subckt` around the top, FLOW-07 step 1) keeps today's behaviour, reported as skipped.
     The resolution in `set_reference` interns each name with `strings.intern` (the same table the labels use,
     reference.rs:58–68), so label and port ids compare equal.
  4. Fixtures: add `vbias vbn` to the `.subckt` line of `benchmarks/fixtures/{ota,ota_constrained,tt_ota}.spice`
     (bias supplied from outside the block, which is what the probe bench assumes, oppoint.rs:333–345).
- Tests: `backend/verify` `an_internal_gate_only_net_is_floating_even_when_labelled`: one MOS whose gate is labelled
  `g`, `external_ports = Some(vec![])` → one `floating_gate` row; `Some(vec!["g"])` → none. `signoff_fixtures`:
  ota with the old header (a string copy in the test) → ERC has `floating_gate` on `vbias` and `vbn`; with the new
  file → 0.
- Acceptance: ERC catches undriven internal gates; SLOW fixtures stay ERC 0 with the corrected headers.
- Risks: nets that only carry dummy gates tied to rails are ports-free internal nets; they are driven (tied), so not
  floating.

### PERF-04 `density_cmp` is deferred, never a vacuous "ran"

Status: done in M0 (`0e53eb9`).

- Priority: P0. Effort: S. Depends on: PERF-01.
- Why: AV-18.
- Current: `strip_density` and `defer_density_wider_than` match kind `density` only (checker.rs:43–47, 262–281);
  sky130 states density as `density_cmp` with 700 µm windows and `partial_windows: false`
  (GP/pdks/sky130.deck:500–503), so it examines zero windows on a block and reports `Ran`.
- Change: in both places match `kind ∈ {density, density_cmp}`. `density` stores one `window` param
  (`ParamValue::Length`); `density_cmp` stores `window: 700um x 700um` as two `ParamValue::Length` params named
  `window_x` and `window_y` (GP/crates/ingest/src/deck/kinds.rs:456–468, parse.rs:1402–1411; `ParamValue` has no
  2-D variant, mod.rs:252–265). Defer when either window side exceeds the block's side. Deferred rules go
  to `coverage.skipped_rules` as `ChipLevel(window … > block …)`. Boundary: FLOW-05 step 5 teaches
  `Pdk::density_rules` (pdk.rs:619–637, the fill pass's reader) the same kind; this item changes only the checker
  (`Checker::new`, `defer_density_wider_than`). Both read `window_x` as the window.
- Tests: `backend/verify` `density_cmp_wider_than_the_block_is_chip_level_on_sky130` (mirror of the ihp test at
  lib.rs:347–357): a 10 µm met1 block → `m1.density`…`m4.density` in `coverage.skipped_rules` with a `ChipLevel` reason
  and absent from the rules that ran.
- Acceptance: bench skip lists show the four sky130 density rules as chip-level for every local fixture.

### PERF-05 `pex_row` follows only kept operands — moved

Duplicated by FLOW-05 steps 3–4 (plan-08), which owns the deck and `pdk.rs` fix. Moved to "Cut or deferred items",
together with a defect in FLOW-05's rule that its implementer must fix (`poly` would answer `None`). The ID is kept so
cross-references stay valid.

### PERF-06 Spec rows for both bounds; a missed bound keeps a do-not-worsen row; failed simulations counted

Status: done in M0 (`0dabdb5`); `RunStats::sim_failures` counts failed simulations.

- Priority: P0. Effort: S. Depends on: — (the NaN-safe `key_lt` is FLOW-02 step 5, not repeated here).
- Why: LAMP-01 (two-sided margin, lampaert.txt L1305–1418), GRAEB-04 (one feature per bound, graeb_centering.txt
  L2433–2471, L3474–3515), GRAEB-11 (a spec already short must not get worse, L6921–6948), GRAEB-47, AF-16,
  AR-44-style "unknown never passes".
- Current: `budget_rows` keeps only the floor when both bounds exist (perf.rs:241–245) and drops specs without
  headroom (perf.rs:246–248); a failed simulation is an `eprintln!` only (lib.rs:887–890).
- Lifetime: this is the P0 fix inside today's `perf::budget_rows` (≈ 20 lines). EXT-25 (P1, after EXT-17/18/24) later
  moves row construction to `annotator::budget::rows` with the same one-row-per-bound rule; EXT-25 step 1 leaves the
  zero-limit row of step 2 "to PERF's policy", so EXT-25 must port step 2 and the two `perf.rs` tests below with it.
- Change:
  1. `perf::budget_rows` emits one row per finite bound: floor `(headroom = f0 − lo, sign = −1)`, ceiling
     `(headroom = hi − f0, sign = +1)`; row `metric` = `"{metric}:min"` / `"{metric}:max"`; weights stay today's
     `w_i = sign·d_i/headroom` (doc perf.rs:225–228, code perf.rs:252), `limit = 1.0`. The match at perf.rs:241–245 and the `filter_map` at perf.rs:239 become a loop
     over `[(spec.min, −1.0), (spec.max, +1.0)]` (`flat_map`).
  2. A bound with `headroom ≤ 0` gets a row with `limit = 0.0` and weights `sign·d/|bound|` (unit-free scale, as
     `miss` uses, perf.rs:93; `|bound| = 0` → divide by 1 as `miss` does): any adverse parasitic is a residual
     ("do not worsen"). `PerformanceBudget` (kernel/analog/src/routing/performance.rs:19–27, RTE-owned; one field,
     announced to RTE-21) gains `pub limit: f32` (1.0 for every other row); `residual = (used − limit).max(0)`,
     `violations = u32::from(used > limit)`, `criticality = (used / limit.max(1e-6)).clamp(0, 1)`.
  3. `score_perf` (lib.rs:878–893): on `Err`, `RunStats.sim_failures: u32 += 1`; metadata prints the count.
- Tests (`frontend/library/src/perf.rs`): `rows_cover_both_bounds_of_a_window_spec`: spec `[10, 20]`, `f0 = 15`,
  `d = [−1.0]` per aF → rows `:min` weight `+0.2`, `:max` weight `−0.2`. `a_missed_bound_keeps_a_do_not_worsen_row`:
  `f0 = 5`, `lo = 10`, `d = [−1]` → one row, `limit == 0.0`, weight `0.1`. (`kernel/analog/src/routing/performance.rs`)
  `a_zero_limit_row_violates_on_any_adverse_c`: `limit 0`, weight `0.1`, one net routed 1 nm at `af_per_nm = 1` →
  `violations == 1`, `residual == 0.1`.
- Acceptance: every declared bound appears in metadata with a row or an explicit reason.

### PERF-07 C tier of the epoch key: sensitivity-weighted signal capacitance

Status: done in M0 (`becffeb`). As built: `RunStats::c_sig` is gone; `RunStats::c_tier` is the winner's key tier and `library::signoff_c_tier` (frontend/library/src/lib.rs:1188) is shared by the epoch key and the bench.

- Priority: P0. Effort: S. Depends on: PERF-01.
- Why: AV-06 (total C, coupling double-counted, supplies included; ota: 79 % supply-related, vbn–VSS 29 %,
  output-pair ΔC unreported), Lampaert eq. 2.12 (ΔP = Σ S·x, lampaert.txt L1601–1620), BAL2-16 sign split
  (balasa_graeb_survey.txt L9334–9360, eqs. 4.6–4.7).
- Current: `lex_key` uses `signoff.cost` (frontend/library/src/lib.rs:955) = `total_cap_ff()` (checker.rs:317–320).
- Change: new fn in `frontend/library/src/lib.rs`:
  ```rust
  /// The epoch key's C tier. With sensitivity rows: Σ_n w⁺_n·C_n(ground) + Σ_(a,b) (w⁺_a + w⁺_b)·C_ab, aF-weighted,
  /// w⁺_n = Σ_rows max(w_jn, 0) — the share of spec headroom the extracted C spends (dimensionless).
  /// Without rows: signal-class ground C plus coupling counted once per signal end, fF; Supply/Ground nets 0.
  fn c_tier(caps: &verify::CapMatrix, names: &[String], classes: &[analog::metadata::NetClassification],
            rows: &[analog::routing::PerformanceBudget]) -> f32
  ```
  `CapMatrix` rows are `(net, None, fF)` ground and `(a, Some(b), fF)` coupling with `a < b`
  (backend/verify/src/lib.rs:65–67, checker.rs:282–315); names map to `NetId` through `Flow.net_names`; C is in fF, so
  the weighted form multiplies by 1000 (aF). A net with no class row, or class `Supply`/`Ground` (EXT-02's
  `rail_of` decides node `0`), contributes 0 in the no-rows form. `lex_key` (in the shape FLOW-02 step 3 gives it)
  takes it instead of `signoff.cost`. `Report.cost` keeps the total (bench column `C total`); bench adds
  `C sig` and `C tier`. `C_TIE` (lib.rs:922) applies to `c_tier`.
- Tests (`lib.rs` tests): `supply_decoupling_does_not_rank_layouts`: rows weight `vout1 = 0.01/aF`, `vbn = 0`;
  matrix A `{vbn–VSS 89.4 fF, vout1 4.6 fF}` has `c_tier = 46`, matrix B `{vbn–VSS 10 fF, vout1 5.0 fF}` has `50`;
  A ranks better although its total C is 94 vs 15 fF. `no_rows_counts_signal_nets_only`.
- Acceptance: the unit tests (M0). Once PERF-20's ota sidecar exists: the largest `c_tier` contributor is one of the
  three nets with the largest `Σ_j |∂f_j/∂C|`.

### PERF-08 The simulated circuit is the drawn circuit

- Priority: P0. Effort: M. Depends on: FLOW-01 (size convention and the FET card line itself, AF-01/AR-46), FLOW-07
  (R/C/L values `r_mohm`/`c_af`/`ind_ph`, raw params, `Q` terminal `S`, AF-02).
- Why: AF-03 (non-FETs dropped), AF-09 (overlay model incomplete), AF-33 (invented W/L fallbacks), SURV-01 (the
  simulation is the judge, perf_driven_survey.txt L15–37). The AF-01 card fix and its test
  (`size_tests::drawn_width_equals_simulated_width`, asserting `W=40 L=2 nf=1 m=4` for XM5) are FLOW-01 step 3; they
  are not repeated here.
- Current: non-FETs are skipped (`model_for` returns `None`, oppoint.rs:143; `continue` at oppoint.rs:262–264); since
  REL-01 their terminal currents read unknown, not zero (`terminal_ua`, oppoint.rs:41–60: R/D/Q/L → `None`, C → 0); `param_um` invents 0.15 µm / 1 µm (oppoint.rs:319–331); `perf.rs:135` repeats
  the 0.15 µm fallback.
- Change (`frontend/library/src/oppoint.rs`):
  1. `pub(crate) fn flat_circuit_with(netlist, cfg, node, extra, emit_caps: bool) -> Result<String, String>`;
     `flat_circuit` passes `emit_caps = true`.
  2. A FET without `w` or `l` is `Err("{inst}: no W/L")` (read through `Device::mos_size()`, FLOW-01; delete
     `param_um` and the `map_or(0.15, …)` at perf.rs:135).
  3. Non-FET cards. Model names are the deck's, as `deck_models` sets them (lib.rs:144–150). sky130 ships every one of
     these as a `.subckt` (terminal order checked in VOL/../libs.ref/sky130_fd_pr/spice): so each card is an `X` line
     (`instance_name`, oppoint.rs:296–302) with lengths in µm because the library sets `.option scale=1.0u`
     (VOL/ngspice/all.spice:2):
     - resistor with a model: `{inst} {P} {N} [{B}] {model} w={w_um} l={l_um} m={m}` — `{B}` only when the device
       has a `B` terminal (sky130 `res_high_po` is `r0 r1 b`, `res_generic_po` is `r0 r1`; a subckt/terminal-count
       mismatch is ngspice's error, surfaced by step 4); without a model (letter card): `R{name} {P} {N} {r_ohm}`
       with `r_ohm = r_mohm / 1000` (FLOW-07 step 6);
     - diode: `{inst} {P} {N} {model} area={w_um·l_um} m={m}` when `w`/`l` exist, else the raw params;
     - PNP: `{inst} {C} {B} {E} {model} m={m}` (`sky130_fd_pr__pnp_05v5_W3p40L3p40 Collector Base Emitter`,
       …pnp_05v5_W3p40L3p40.model.spice:26); NPN: `{inst} {C} {B} {E} {S} {model} m={m}` with `{S}` = the `S`
       terminal FLOW-07 keeps, else the ground node (`sky130_fd_pr__npn_05v5_W1p00L1p00 c b e s`,
       …npn_05v5_W1p00L1p00.model.spice:24);
     - capacitor `{inst} {P} {N} {model} w={w_um} l={l_um} m={m}` (letter card: `C{name} {P} {N} {c_af}a`) **only
       when `emit_caps`** (schematic-point runs); post-layout runs pass `emit_caps = false` because every capacitor
       is drawn as plates (cellgen.rs:844–848, AC-01; CELL-08) whose C the extracted matrix already carries — a card
       would double-count;
     - inductor: `Err("{inst}: inductors are not simulated")`.
  4. `extract` and `perf::evaluate` propagate `Err` as "cannot simulate: …" (metrics `None`, a full miss; bias
     provenance names the device) — refuse, never drop. `perf::evaluate` additionally scans ngspice stderr for
     `Error` lines (unknown subckt, wrong pin count) and returns them as `Err`.
  5. Currents of non-FETs: after `op`, add `print all` (node voltages, `v(node) = value` lines) and
     `show q : ic,ib,ie` to the control block. `parse_show(text, prefix: char, heads: &[&str])` generalises today's
     parser (oppoint.rs:410–470) to the `q.` prefix (ngspice names a BJT inside subckt `xq1` as
     `q.xq1.qsky130_fd_pr__…`, exactly as it names `m.xm1.msky130_fd_pr__…` for FETs, oppoint.rs:457–461). Resistor
     current `(V_P − V_N)/R` with `R` = `r_mohm/1000` or, for a model card, `R□·l/w` from FLOW-05's
     `device_sheet_ohm(model)` (unknown → the resistor's current is `None` and the device is unresolved, never 0).
     `terminal_ua` (oppoint.rs:39–58) fills `C`/`B`/`E` and `P`/`N`; a diode's current is `None` (reported unresolved;
     it only matters for EM once REL-01 asks for it).
  6. `benchmarks/fixtures/bjt_mirror.spice:3`: model `npn_05v5_1x1` → `npn_05v5_W1p00L1p00`. The old name matches no
     sky130 model (the shipped one is `sky130_fd_pr__npn_05v5_W1p00L1p00`, VOL/ngspice/all.spice:48), so neither this
     item's simulation, nor PERF-18's recogniser (model match), nor PERF-19's foundry LVS can resolve it.
     `parse.rs:316` tests the old string as a literal only and is unaffected.
- Tests: `oppoint.rs` `a_resistor_gets_a_card_with_its_model` (`XR1 a b sky130_fd_pr__res_high_po w=0.69u l=40u` with a
  `B` terminal → line contains `XR1 a b` and three nodes before the model), `a_bjt_card_is_collector_first`
  (bjt_mirror XQ2 → `XQ2 outp in VDD sky130_fd_pr__pnp_05v5_W3p40L3p40 m=1`), `an_inductor_refuses_to_simulate`,
  `a_missing_length_is_an_error`, `parse_show_reads_bjt_columns` (a canned `show q` block with two instances →
  both `ic` values on the right devices); the existing `frontend/library/tests/perf_postlayout.rs` (two tests today,
  :49–50 and :76–77; ngspice-dependent tests gate on `library::tools::sky130_models()`, FLOW-14): `rc_filter_resistor_carries_current` —
  `terminal_ua` of `XR1` non-zero and the currents on `vmid` sum to 0 within 1 nA.
- Acceptance: rc_filter and bjt_mirror resolve every device (`BiasSummary.resolved == devices`); a netlist with an
  inductor reports "cannot simulate" instead of a bias.
- Risks: the NPN subckt has four pins (`c b e s`); a netlist `Q`/`X` card with three nodes gets the ground node as
  `s`, which is right for a substrate NPN and wrong for an isolated one (then FLOW-07's 4th terminal must be given).

### PERF-09 Operating-point robustness: aligned parsing, isolated decks, classified rails, honest provenance

- Priority: P0. Effort: M. Depends on: EXT-02 (`rail_of`, node `0`, AA-11). Step 5 of the previous draft (annotate
  once) is FLOW-09 step 2 and is not repeated.
- Why: AF-25, AF-27, AF-04, AF-33, NOTES-02, S3–S5 in §1.3; REL-10 needs per-FET V_GS/V_DS/V_BS from the operating
  point ("FLOW (OpPoint `vgs_v`, `vds_v`, `vbs_v` kept on `Solution`)", plan-06 REL-10) and no other plan produces
  them (oppoint.rs is PERF-owned, plan-08 FLOW-01 step 3).
- Current: `parse_show` drops columns (oppoint.rs:435); `pin_currents` is all-or-nothing (lib.rs:1035–1038); one deck
  path per process and no cleanup (oppoint.rs:157–160, perf.rs:170–174); rails by name lists (oppoint.rs:392–398);
  one `vdd` for every supply (oppoint.rs:371–376); `certified()` ignores provenance (metadata.rs:106–110); `run`
  annotates once in `performance_rows` (lib.rs:203–208) and again in every `solve` (lib.rs:262–263), and `solve`
  runs once or twice per start (lib.rs:174–182) for `cfg.starts` starts (default 3, lib.rs:75).
- Change:
  1. `parse_show`: `cols: Vec<Option<String>> = rest.iter().map(|c| instance_device(c)).collect()`; a `None` column
     keeps its position and its values are skipped. `OpPoint` gains `pub unresolved: Vec<String>`.
  2. Deleted (C17): REL-01 landed it in M0 (`pin_currents`, frontend/library/src/lib.rs:1279, emits `None` for an
     unresolved member and keeps the others; test `one_unresolved_device_keeps_the_others_known`).
  3. `fn scratch_dir(tag: &str) -> std::io::Result<PathBuf>` = `temp_dir()/philis_{tag}_{pid}_{n}` with a
     process-wide `AtomicU64` `n`; ngspice runs with `.current_dir(&dir)` (no `bsim4v5.out` in the caller's cwd);
     `remove_dir_all(&dir)` after parsing unless `PHILIS_KEEP_DECKS` is set. Used by `oppoint::extract` and
     `perf::evaluate`.
  4. `OpConfig` gains `pub rails_v: Vec<(String, f64)>` (explicit per-net DC voltage) and
     `pub ground_nets: Vec<String>`; the library fills `ground_nets` with the nets EXT-02's
     `annotator::netrole::rail_of` (or the class table) calls ground, and Supply-class nets without an explicit
     voltage get `vdd`. `node_name(netlist, id, ground_nets)` (oppoint.rs:305) maps them to `0`; `is_ground`/`is_supply`
     (oppoint.rs:392–398) are deleted.
  5. Per-FET voltages for REL-10: the control block's `show m : id,vds,vdsat,gm` (oppoint.rs:238) becomes
     `show m : id,vgs,vds,vbs,vdsat,gm` (BSIM4 op quantities); `DevOp` (oppoint.rs:201–207) gains `vgs`, `vbs`;
     `OpPoint` gains `pub vgs_v, vds_v, vbs_v: Vec<Option<f64>>` (V, as ngspice reports, signed) and
     `#[derive(Clone)]` (every field is a `Vec` or `String`). `Bias` (lib.rs:233–245) keeps the `OpPoint`;
     `Solution` gains `pub op: Option<oppoint::OpPoint>`.
  6. `BiasSummary` gains `pub probe: bool`; `certified()` (metadata.rs:104–110) requires
     `self.bias.as_ref().map_or(true, |b| !b.probe)`.
- Tests (`oppoint.rs`): `a_truncated_header_does_not_shift_later_devices` (a `show` block whose middle column lacks the
  `m.x.` prefix: the third device gets its own `id`); `avdd_is_driven_when_classified_supply`;
  `node_zero_is_ground`; `concurrent_extracts_use_distinct_decks` (two threads, two netlists, both correct);
  `show_reads_vgs_and_vbs` (a canned six-column `show m` block → `vgs_v[i]`, `vbs_v[i]` equal the printed values).
  (`lib.rs`: REL-01's `one_unresolved_device_keeps_the_others_known` already covers step 2.) (`metadata.rs`) `a_probe_bias_never_certifies`.
- Acceptance: tests green; `frontend/library/bsim4v5.out` no longer produced (FLOW-14 step 5 then deletes the stray
  file); REL-10's `voltage_findings` receives a populated `OpPoint` on ota with an op testbench.

### PERF-10 Scenarios, several testbenches, corner-aware margins

- Priority: P1. Effort: M. Depends on: PERF-06, PERF-08.
- Why: LAMP-01 (margins against the process interval, eqs. 2.3–2.7, lampaert.txt L1305–1418), GRAEB-05 (spec holds for
  every range parameter, eq. 114, graeb_centering.txt L3370–3376), GRAEB-25/26 (range worst case by gradient sign,
  eq. 158/159 L4005–4032; keep the nominal-gradient corner and update it only when monitored gradient signs flip,
  L6013–6040 — the finalist-only re-check is GRAEB-26's automation recipe, not Graeb's text), NOTES-09, AF-09
  ("one corner").
- Current: `PerfConfig { sim, testbench: String, specs }` (perf.rs:59–68); one corner and one temperature
  (`OpConfig::corner`, `temp_c`, oppoint.rs:103–121); headroom at the nominal schematic point only (perf.rs:239–245).
  An offset measurement needs a feedback configuration a gain testbench cannot share (MAGICAL OTA README:
  "unity gain feedback configuration is used for the offset measurement").
- Change (`frontend/library/src/perf.rs`):
  ```rust
  #[derive(Clone, Debug)]
  pub struct Scenario { pub name: String, pub corner: String, pub temp_c: f64, pub params: Vec<(String, f64)> }
  #[derive(Clone, Debug)]
  pub struct PerfConfig {
      pub sim: OpConfig,
      /// Each is a complete bench (sources, analyses, `.measure`); a spec metric comes from exactly one.
      pub testbenches: Vec<String>,           // was `testbench: String`
      pub specs: Vec<Spec>,
      /// Empty = one scenario from `sim.corner` / `sim.temp_c`.
      pub scenarios: Vec<Scenario>,
      pub beta_target: f64,                   // 3.0 (PERF-13)
      pub mc: Option<McConfig>,               // PERF-22
  }
  pub struct BoundResult { pub spec: usize, pub upper: bool, pub value: Option<f64>, pub scenario: usize }
  pub struct PerfResult {
      pub metrics: Vec<(String, Option<f64>)>,  // nominal scenario, for display
      pub bounds: Vec<BoundResult>,              // worst simulated scenario per bound
      pub residual: f64,
  }
  pub fn evaluate(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig, scenarios: &[usize]) -> Result<PerfResult, String>
  ```
  Deck per (testbench, scenario): `.lib {lib} {corner}`, `.temp {T}`, one `.param k=v` per scenario param (the
  testbench references them, e.g. `Vdd vdd 0 {vdd}`), circuit, parasitics, testbench, `.end`. All (testbench,
  scenario) decks of one call run on the job pool of PERF-11.
  Algorithm in `lib.rs`:
  1. Run start: `evaluate(schematic, default, cfg, all)`. For each bound take the scenario with the smallest margin
     (`value − lo` or `hi − value`); `active` = distinct such scenarios (≤ 2·n_specs, usually 1–3). Also record
     `spread[j] = (min, max)` of spec j's metric over all scenarios at the schematic (EXT-17 `SpecSens.proc`, PERF-12).
  2. Promoted epochs evaluate `active` only; a bound's value = worst over them.
  3. Winner (after fill, PERF-15) evaluates all scenarios; if a different scenario is now worst for a bound, the
     report says so (GRAEB-26).
  4. Headroom per bound for rows = margin at its active scenario (LAMP-01's corner-safe margin).
- Tests: `the_deck_sets_corner_temperature_and_params`; `the_worst_scenario_is_chosen_per_bound` (synthetic results:
  floor worst at `ss`, ceiling worst at `ff`); `perf_postlayout.rs` `ota_gain_moves_with_temperature` (27 °C vs
  125 °C differ by > 0.1 dB).
- Acceptance: ota with scenarios `{tt 27 °C, ss 125 °C, ff −40 °C}` reports per bound the worst scenario and value;
  sims per promoted epoch ≤ |active| × |testbenches|.

### PERF-11 Sensitivity engine v2

- Priority: P1. Effort: L. Depends on: PERF-08, PERF-10.
- Why: LAMP-02/03 (every parasitic class; perturbation method eq. 2.15, lampaert.txt L1601–1742 — Lampaert's text
  gives the one-sided perturbation and warns a tiny Δh is unstable; central differences are LAMP-03's recipe),
  GRAEB-06 (σ_f needs ∂f/∂V_T per device, eq. 175 L4200–4211), GRAEB-23 (step "large enough to surmount numerical
  noise and small enough to compare to the gradient", eq. 83 L2682–2704), SURV-28 (per-net R sensitivity; automatic
  classification beat designer annotation, TS-OTA2 FOM 0.92 vs 0.74, todaes22.txt L895–920), BAL2-16/17 (critical
  parasitic screening, balasa_graeb_survey.txt L9305–9379), AF-16, AT-26.
- Current: `Sensitivity { base, d_per_af: [spec][net] }` from +10 fF ground C per net, forward difference, one thread
  per run (perf.rs:192–223; lib.rs:215–216).
- Change (`frontend/library/src/perf.rs`; the old `Sensitivity` is deleted):
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq)]
  pub enum Param {
      GroundC { net: NetId },                          // aF
      CouplingC { a: NetId, b: NetId },                // aF
      SeriesR { device: u16, terminal: TermName },     // Ω, TermName = [u8; 2] ("S", "D", "E", "C", "P", "N")
      GateOffset { device: u16 },                      // V, source in series with the gate: V_gate − V_net
  }
  /// `d[j]` = ∂(metric of spec j)/∂param, metric units per param unit. Per spec, not per bound: both bounds of a
  /// spec read the same metric; a bound picks the table of its own active scenario (`SensTable.scenario`).
  pub struct SensRow { pub param: Param, pub step: f64, pub d: Vec<Option<f64>>, pub linear: bool }
  pub struct SensTable { pub scenario: usize, pub at: Parasitics, pub base: PerfResult, pub rows: Vec<SensRow> }
  pub struct StepPolicy { pub c_min_af: f64, pub c_frac: f64, pub r_ohm: f64, pub lin_tol: f64 }
  impl Default for StepPolicy { fn default() -> Self { Self { c_min_af: 1000.0, c_frac: 0.1, r_ohm: 100.0, lin_tol: 0.2 } } }
  pub fn sensitivities(netlist: &Netlist, cfg: &PerfConfig, scenario: usize, params: &[Param],
                       sigma_v: &[Option<f64>], steps: &StepPolicy, at: &Parasitics) -> Result<SensTable, String>;
  /// Bounded pool: `available_parallelism()` workers over an `AtomicUsize` job index.
  pub(crate) fn run_jobs<J: Sync, T: Send>(jobs: &[J], f: impl Fn(&J) -> T + Sync) -> Vec<T>;
  ```
  `Parasitics` gains `pub gate_offset_v: Vec<f64>` (per device, 0 = none). In `deck`, a non-zero offset renames the
  gate node to `{prev}__o{di}` and adds `Vgo{di} {prev}__o{di} {prev} {v:.6e}` (`prev` = the net node or the series-R
  node). Sign: `δ_i = −ΔV_T,i` for NMOS, `+Δ|V_T,i|` for PMOS (documented on the enum).
  Step policy ([policy], each checked for linearity): ground/coupling C: `Δ = max(c_min_af, c_frac·C_est,n)` with
  `C_est,n` = Σ `Device::gate_area_um2()` (FLOW-01) of the gates on the net × `gate_cap_af_um2` (sky130: 8325 aF/µm²,
  pdks/sky130.json:9; read at lib.rs:501) — e.g. ota `vout1` carries no gate, so its step is `c_min_af` = 1 fF;
  ("10 % of node C" is GRAEB-23's
  automation recipe, a Philis choice — Graeb only requires Δx large enough to surmount noise, L2703–2704);
  series R: `Δ = r_ohm`; gate offset: `Δ = σ_i` (device random σ, PERF-13; sky130 nfet at W·L = 1 µm²:
  9.5/√2 = 6.7 mV from `avt_n_mv_um` 9.5, pdks/sky130.json:93; 1 mV when σ_i is unknown). Difference scheme: central
  `(f(+Δ) − f(−Δ))/2Δ` when the base value at `at` is ≥ Δ (always for gate offsets); otherwise forward at Δ and Δ/2,
  `linear = |s₁ − s₂| ≤ lin_tol·max(|s₁|,|s₂|)`, keep `s₂`.
  Parameter selection (`fn default_params(netlist, classes, ground_rows: Option<&SensTable>) -> Vec<Param>`):
  1. `GroundC` for every Signal/Sensitive/Clock net (as today).
  2. `SeriesR` for FET `S` and `D`, BJT `E`/`C`, resistor `P`/`N` terminals on nets with ≥ 2 pins.
  3. `GateOffset` for every FET.
  4. `CouplingC` for the 64 [policy] pairs of (budgeted net, any other net) with the largest
     `max_j(|∂f_j/∂C_a| + |∂f_j/∂C_b|)` from pass 1 (screening; two passes).
  `RunStats.sims: u32` counts every ngspice run. `perf::sensitivities` replaces the one-thread-per-net spawn
  (perf.rs:200–211) everywhere; `performance_rows` (lib.rs:196–231) calls it once per active scenario at
  `at = Parasitics::default()`.
- Tests (`perf.rs`, the ngspice ones use FLOW-14's `tool_or_skip`): `run_jobs_never_exceeds_the_pool` (an `AtomicUsize` high-water
  mark ≤ `available_parallelism()`); `rc_lowpass_f3db_derivative_matches_analytic` — testbench `R1 in out 1k`,
  `C1 out 0 1p`, `.measure ac f3db when vdb(out)=-3`; `∂f3dB/∂C_out` from the engine within 5 % of
  `−f3dB/C_total`; `gate_offset_moves_a_diff_pair_output_both_ways` (signs opposite for the two inputs);
  `coupling_rows_only_for_screened_pairs` (≤ 64 rows).
- Acceptance: ota at one scenario: every row reports `linear` or is listed nonlinear in metadata; sims ≤
  `1 + 2·(n_nets + n_terminals + n_fets + 64)`; wall time printed.
- Risks: negative steps are illegal at a zero base, hence the forward/half-step fallback; transient metrics are noisy
  (BAL2-16 notes perturbation is error-prone for transients, L9314–9333) — rows with `linear = false` get weight 0
  and are reported.

### PERF-12 Sensitivity export: EXT-17 evidence, RTE-21 router weights, and the coupling term

- Priority: P1. Effort: M. Depends on: PERF-11, PERF-13 (σ_f, statistical headroom), EXT-17 (the `Sensitivities`
  type), RTE-21 (the `DetailedCfg` fields). Rewritten in review: the previous draft redefined `PerformanceBudget` and
  `budget_rows`, which EXT-25 (row construction, sign split, β reserve) and RTE-21 step 3 (stack ground C, series R,
  `conservative`) already own — see "Cut or deferred items".
- Why: AR-22 (length × one constant, no coupling or R); LAMP-39 (eq. 5.10, `ΔP_j = S_Ci·C_i + S_Ri·R_i +
  Σ ½·S_Cki·C_ki`, lampaert.txt L6099–6178, as RTE-21 cites it); BAL2-16 (sign split, eqs. 4.6–4.7, L9334–9360);
  BAL2-22 (per-aggressor coupling asymmetry, L8905–8909); EXT-17's calling protocol ("PERF computes sensitivities over
  p0 nets and devices"); RTE-21 step 1 ("PERF fills them"). Without this item the other plans' performance-driven
  steps receive nothing.
- Current: the only export is `perf::budget_rows` → ground-C rows (perf.rs:230–257); `DetailedCfg.net_weight` is
  derived from them (lib.rs:292–316).
- Change:
  1. `frontend/library/src/perf.rs`:
     ```rust
     /// EXT-17's input: one `SpecSens` per spec, each read from the table of the spec's active scenario
     /// (`active[j]`, PERF-10 step 1). Rows with `linear == false` are left out (reported by PERF-11).
     pub fn to_evidence(cfg: &PerfConfig, tables: &[SensTable], active: &[usize], spread: &[(f64, f64)],
                        stats: &[robust::BoundStat], netlist: &Netlist) -> annotator::evidence::Sensitivities;
     ```
     Mapping for spec j (EXT-17 field ← PERF):
     - `metric`, `lo`, `hi` ← `cfg.specs[j]`; `f0` ← the table's base metric j;
     - `proc` ← `Some(spread[j])` when `cfg.scenarios.len() > 1`, else `None`;
     - `sigma_f` ← max over j's bounds of `BoundStat::sigma_f` (both bounds read one metric; `None` if any is `None`);
     - `d_c` (per aF) ← `GroundC { net }` rows' `d[j]`;
     - `d_r` (per Ω) ← `SeriesR` rows summed per net of the terminal (`netlist.devices[device].terminals`), a net-level
       figure because EXT-17's field is per net;
     - `d_vt` (per mV of |V_T|) ← `GateOffset { device }` rows: `−d[j]/1000` for NMOS (δ = −ΔV_T), `+d[j]/1000` for
       PMOS (δ = +Δ|V_T|), PERF-11's sign convention;
     - `d_t` ← empty (PERF computes no per-device temperature sensitivity; EXT-17 reads empty as unknown).
     The library then runs EXT-17's second `annotate_with(nl, cfg, &ev_with_sens)`; the rows EXT-25 builds from it
     replace today's `performance_rows` result (lib.rs:196–231).
  2. Router weights, RTE-21 step 1's definitions:
     ```rust
     /// `r_weight[n] = Σ_b |S_R,b,n| / h_b`, scaled so the maximum is 1; `pair_weight = (a, b, Σ_b ½·|S_Cc,b,ab| / h_b)`
     /// over the screened `CouplingC` rows. `h_b` = PERF-13's statistical headroom, or `|bound|` when it is ≤ 0
     /// (PERF-06's scale). `bounds[k] = (spec, h_b, scenario)`.
     pub fn router_weights(tables: &[SensTable], bounds: &[(usize, f64, usize)], netlist: &Netlist)
         -> (Vec<f32>, Vec<(pnr_core::NetId, pnr_core::NetId, f32)>);
     ```
     `solve` writes them into `flow.d_router.cfg.r_weight` / `pair_weight` after `elaborate::detailed_router`
     (lib.rs:297–316) builds the router.
  3. The coupling term (LAMP-39's `½·S_Cki·C_ki`) has no home yet: EXT-17's `SpecSens` carries no coupling field and
     RTE-21's `used` sums ground C and R only. Change requests, recorded here with their exact shape:
     `SpecSens.d_cc: Vec<(NetId, NetId, f64)>` (per aF; PERF fills it from `CouplingC` rows in step 1), and
     `PerformanceBudget.coupling: Vec<(NetId, NetId, f32)>` with `used += Σ w_ab · Σ_{p ∈ wires[a], q ∈ wires[b],
     same layer} stack.lateral_af(layer, p, q)` (kernel/analog/src/routing/stack.rs:256), filled by EXT-25 as it fills
     `weights`. Until both land, coupling reaches routing only as `pair_weight` (step 2).
- Tests (`frontend/library/src/perf.rs`):
  - `evidence_maps_units_and_signs`: table with `GroundC{vout1}` d = −0.002 dB/aF, `SeriesR{XM1,"S"}` and
    `SeriesR{XM2,"S"}` d = −0.01 dB/Ω each (both on `vtail`), `GateOffset{XM1}` d = +1000 dB/V (NMOS) →
    `d_c == [(vout1, −0.002)]`, `d_r == [(vtail, −0.02)]`, `d_vt == [(XM1, −1.0)]` (dB/mV).
  - `router_weights_scale_r_to_one`: two nets with `Σ|S_R|/h` = 0.3 and 0.6 → `r_weight` = `[0.5, 1.0]`; one coupling
    row with |S| = 0.4 /aF and h = 2 → `pair_weight == [(a, b, 0.1)]`.
  - `a_nonlinear_row_is_not_exported`.
- Acceptance: on ota with PERF-20's sidecar, `Sensitivities` has non-empty `d_c`, `d_r` (incl. `vtail`, the pair's
  source net — SURV-27's Gm-degeneration mechanism, todaes22.txt L46–53) and `d_vt` (XM1, XM2); `r_weight` and
  `pair_weight` are printed per net with the metadata.

### PERF-13 Statistical layer: σ_f, β, statistical headroom, linearised yield

- Priority: P1. Effort: M. Depends on: PERF-11, FLOW-01 (`gate_area_um2`), MAT-04 (optional systematic terms).
  The per-device/per-set allowance (GRAEB-38) and the sizing-infeasible diagnostic (LAMP-49) of the previous draft are
  EXT-21 (`allocate`, `spec_infeasible_at_schematic`) and EXT-25 (`no_layout_margin`); this item supplies their σ_f.
- Why: GRAEB-06/07/08 (σ_f = √(∇fᵀC∇f), eq. 175 L4200–4211; β = margin/σ_f, eq. 300/301 L6079–6141;
  Ȳ = Φ(β), eq. 302 L6143–6170), GRAEB-16 (worst case f0 ∓ β·σ_f, eq. 177/179 L4213–4224), GRAEB-02 (systematic shifts
  are mean shifts, L2334–2337), GRAEB-33 (linearised joint yield, L6247–6254), MM-02 (per-device σ = pair A/√(2·area),
  long_distance_mismatch.txt L144–148), MM-14.
- Current: no σ, β or yield in the flow (P13).
- Change: new `frontend/library/src/robust.rs`:
  ```rust
  /// Device random V_T σ, V: A_VT(pair)/√(2·A_gate) with A_gate = `Device::gate_area_um2()` (µm², W_total·L·m) and
  /// A_VT in mV·µm (`avt_n_mv_um` 9.5 / `avt_p_mv_um` 11.5 on sky130, pdks/sky130.json:93–96; these are pair
  /// coefficients, σ(ΔV_T) fitted against 1/√(WL), characterize_mismatch.py:118–124), result ÷ 1000.
  /// `None` for a non-FET or without the deck's A_VT for its polarity.
  pub fn device_sigma_v(netlist: &Netlist, avt_mv_um: [Option<f32>; 2]) -> Vec<Option<f64>>;
  pub struct BoundStat {
      pub bound: usize,                 // index into PerfResult::bounds
      pub sigma_f: Option<f64>,         // metric units
      pub beta: Option<f64>,
      pub yield_part: Option<f64>,      // Φ(β)
      pub shares: Vec<(u16, f64)>,      // device, s²σ²/σ_f²  (top contributors)
  }
  /// `sys[b]` = Δf_sys for bound b (MAT-04's `mu_thermal` through s; LOD excluded — simulated via sa/sb; empty
  /// slice = no systematic term); `grad` = MAT's matched sets `(p, q, σ_grad V)`: devices p, q and the σ of their
  /// gradient-induced ΔV_T (empty until MAT provides it). Bound b reads `tables` entry with
  /// `scenario == post.bounds[b].scenario`.
  pub fn bound_stats(tables: &[SensTable], sigma_v: &[Option<f64>], post: &PerfResult, specs: &[Spec],
                     sys: &[f64], grad: &[(u16, u16, f64)]) -> Vec<BoundStat>;
  pub fn linear_joint_yield(tables: &[SensTable], sigma_v: &[Option<f64>], post: &PerfResult, specs: &[Spec],
                            sys: &[f64], samples: usize, seed: u64) -> Option<f64>;
  /// Standard normal cdf via erfc (Abramowitz–Stegun 7.1.26); implementation choice, not from ref/.
  pub fn phi(x: f64) -> f64;
  ```
  Formulas (per bound b of spec j, s_bi = ∂f/∂δ_i from `GateOffset` rows):
  - σ_f,b² = Σ_i s_bi²·σ_i² + Σ_(p,q)∈grad ((s_bp − s_bq)/2)²·σ_grad,pq² (the gradient acts on the pair difference).
  - β_L = (f_post + Δf_sys − b_L)/σ_f; β_U = (b_U − f_post − Δf_sys)/σ_f; `f_post` = worst active scenario value.
  - headroom_stat,b = headroom_b − β_target·σ_f,b → `router_weights` (PERF-12 step 2) and, through `SpecSens.sigma_f`,
    EXT-25's rows (EXT-25 applies the same `β·σ_f` reserve; PERF must not subtract it a second time). β_target
    default 3.0 ("three-sigma design", graeb_centering.txt L4266–4281).
  - Linear joint yield: `gp::mechanics::SplitMix64` (reused) + Box–Muller, t_i ~ N(0,1),
    f_b = f_post,b + Δf_sys,b + Σ_i s_bi σ_i t_i; pass iff every bound holds; default 100 000 samples.
  - Missing A_VT → σ, β `None`; metadata `UNKNOWN (no A_VT)`; the key falls back (PERF-14).
- Tests (`robust.rs`): `sigma_f_of_a_symmetric_pair`: s = [+1000, −1000] dB/V, σ = [1 mV, 1 mV] → σ_f = √2 dB;
  `beta_reproduces_graeb_table_14_gain`: f = 76, bound 65, σ_f = 4.4 → β = 2.5 (Table 14 L6207–6216 gives β 2.5 for
  gain 76 dB vs 65 dB; σ_f = 11/2.5 is derived); `phi_matches_table_12`: Φ(−1, 0, 1, 2, 3) = 0.159, 0.500, 0.841,
  0.977, 0.999 within 1e-3 (L5583–5591); `device_sigma_of_a_sky130_nfet` (W = 1 µm, L = 1 µm, m = 1 →
  9.5/√2/1000 = 6.72e-3 V ± 1e-5); `linear_yield_of_one_bound_is_phi_beta` (|Ŷ − Φ(β)| ≤ 0.005 at 1e5 samples).
- Acceptance: ota metadata prints σ_f, β, Φ(β) and top-3 device shares per bound; `SpecSens.sigma_f` is set for
  every spec EXT-17 receives.
- Risks: V_T-only σ_f underestimates current-matched specs until MAT adds A_β (MM-28); labelled "V_T only".

### PERF-14 Epoch key on β; Pareto report

- Priority: P1. Effort: M. Depends on: FLOW-02 (key contract and its `nan_last`), PERF-07, PERF-13.
- Why: GRAEB-07/09 (WCDs make heterogeneous margins comparable: 11 dB vs 37 MHz cannot be judged, β 2.5 vs 7.7 can,
  L6196–6216), GRAEB-12 (l∞ for robustness, L7156–7172), GRAEB-36 (yield saturates above 99.9 %, rank by β,
  L3807–3877), GRAEB-42 / SURV-25 / NOTES-08 (Pareto: MOBO top-1 on 3–5 metrics vs weighted, perf_driven_survey.txt
  L217–252), AF-30.
- Current: `LexKey = (|V|, Σ miss, Θ, C, footprint)` (lib.rs:916); a met spec contributes 0, so the key is blind to
  robustness; a single incumbent (lib.rs:362–367).
- Change (`frontend/library/src/lib.rs`):
  ```rust
  /// (|V|, failed bounds, spec shortfall, Θ, C tier, footprint nm²)
  type LexKey = (usize, u32, f64, f64, f32, f64);
  ```
  failed = #bounds with β < 0 or value `None` (unknown never passes); shortfall = max_b max(0, β_target − β_b). When σ
  is unknown for the run (no A_VT), failed = #bounds with miss > 0 and shortfall = Σ normalised miss (today's
  semantics). `key_lt` compares the first four through FLOW-02's `nan_last`, then the C band (PERF-07), then
  footprint. The first tier is FLOW-02's `v` unchanged; FLOW-02's four `start_tests` that build 5-tuples
  (`v_counts_rules_not_batches`, `theta_counts_each_budget_once`, `warnings_are_not_violations`, `nan_loses`) are
  updated to the 6-tuple in the same commit.
  Pareto front: `MetadataReport::pareto: Vec<ParetoPoint>` with
  `ParetoPoint { outer: u32, iteration: u32, min_beta: Option<f64>, theta: f64, c_tier: f32, area_um2: f64 }`,
  maintained in `solve` over epochs with `key.0 == 0 && key.1 == 0`, non-dominated in
  (−min_beta, theta, c_tier, area), at most 16 [policy] (drop the largest-area point on overflow). The winner is still
  the lexicographic one (declared policy); the front is reported (metrics only: `Layout` is not `Clone`, AR-26).
- Tests: `robust_beats_barely_passing` (shortfall 0.5 < 2.0); `an_unknown_bound_counts_as_failed`;
  `pareto_keeps_only_nondominated` (5 synthetic points → 3).
- Acceptance: for 5 seeds on ota with `outer_iters = 1`, `feedback_iters ≤ PATIENCE` (lib.rs:137, so no stall exit)
  and refresh off, both keys choose among the same epochs (prices and routing history do not depend on `best`,
  lib.rs:341–367; `best` only gates promotion and stalls), and the new winner's min β ≥ the old key's winner's.

### PERF-15 Sensitivity refresh in a trust region; post-fill re-evaluation

- Priority: P1. Effort: M. Depends on: PERF-10, PERF-11, PERF-12; FLOW-08 (`Weights`), FLOW-10 (post-fill signoff).
- Why: GRAEB-39 (nested-loop simplifications: sensitivity analysis once per step or every µ-th step, L3316–3361;
  its automation recipe, a Philis choice, refreshes every µ = 4 epochs or on prediction error > 30 % of σ_f),
  NOTES-13 (trust region; its recipe refreshes on prediction error > 30 % of headroom), SURV-26 (a reused model's
  accuracy drops 3–22 % out of distribution, so reuse needs a validity check; perf_driven_survey.txt L219–258),
  NOTES-07/AF-13 (fill is applied after selection; reported metrics are pre-fill, lib.rs:410–438).
- Current: sensitivities once per run from the schematic (lib.rs:167, 196–231); `metadata.performance` from the
  pre-fill winner (lib.rs:432–438).
- Change (`frontend/library/src/lib.rs`):
  1. After a promoted epoch becomes incumbent: predicted `Δf_b = Σ_rows d·(p_epoch − p_at)` with p from its
     `Parasitics` (ground and coupling C from `epoch.caps`, series R from `parasitics().series`); error
     `e_b = |f_sim,b − (f_base,b + Δf_b)|`. If `max_b e_b / headroom_b > 0.3` [policy; NOTES-13's recipe uses 30 % of
     headroom, GRAEB-39's recipe 30 % of σ_f; neither number is in the source texts] and
     `refreshes < 3` [policy]: recompute `SensTable` at `at = incumbent parasitics` (central differences now legal),
     re-export (PERF-12 step 1) and let EXT-25 rebuild the rows, replace them in `flow.problem.routing.budget` (their
     index range is recorded when pushed at lib.rs:267–269), and recompute the loop's `Weights` with FLOW-08's
     `route_weights(place, classes, sens)` plus `gp::net_weights` for `place` (today inline at lib.rs:292–316);
     `DetailedCfg.r_weight`/`pair_weight` are rewritten by PERF-12 step 2. `RunStats.refreshes: u32 += 1`.
  2. After fill: FLOW-10 step 4 recomputes signoff on the final geometry (fill included) and returns its `CapMatrix`;
     this item runs `perf::evaluate` on **all** scenarios with that matrix; `metadata.performance` = post-fill,
     `metadata.performance_pre_fill` keeps the old one; a bound that flips to failing sets
     `RunStats.fill_regressed: bool = true` and is printed. (Before FLOW-10 lands: call `signoff_shapes` on
     `solution.geometry()` after lib.rs:420 for the caps — the same call FLOW-10 then owns.)
- Tests (`frontend/library/tests/perf_postlayout.rs`): `a_bad_prediction_triggers_a_refresh` (rows built with the
  wrong sign → refresh count ≥ 1); `reported_performance_is_post_fill` (`performance_pre_fill` present; the post-fill
  evaluation used a cap matrix containing the fill's ground net entries).
- Acceptance: ota prints refresh count, pre/post-fill metrics and the prediction error of the last refresh.

### PERF-16 PEX fidelity: merged metal, field-solved critical nets, calibration

- Priority: P1. Effort: M. Depends on: PERF-01, PERF-19 (magic reference); PERF-11 refines the net ranking (before it,
  rank by today's ground-C rows, perf.rs:192–223).
- Why: AV-07 (unmerged rectangles inflate metal ground C 1.13–1.73×; fringe-free parallel-plate coupling), AV-32 (dac4 MOM
  0.105 fF/µm² extracted vs 2.0 used to size, 19×), LAMP-18 (geometric models are only as good as their fit,
  lampaert.txt L1892–2008), NOTES-15.
- Current: one GPurify polygon per `Shape` (backend/verify/src/geom.rs:39–42); `geometry::merge_rects` exists and
  merges equal-span or contained rectangles but is applied to nwell only (frontend/library/src/geometry.rs:27–52;
  lib.rs:664–666, 1270–1276); `quasistatic_nets: Vec::new()` (checker.rs:163–168) although GPurify field-solves and
  overlays named nets (GP/src/engine/run.rs:338–399).
- Change:
  1. Merge in the one place every signoff path passes through (`verify::geom::build_store`,
     backend/verify/src/geom.rs:30–54; callers: `Checker::load_geometry`, hence `signoff`, `drc`, `erc`), with
     GPurify's exact KLayout-semantics union instead of the library's `geometry::merge_rects`. `merge_rects` is the
     wrong tool: O(n²) per pass with a restart after every join (geometry.rs:26, 44–55), and it joins only rects with an
     equal span on one axis or one inside the other (geometry.rs:33–36), so partial overlaps of unequal span stay
     double-counted, while audit-06 G.4(b)'s 1.13–1.73× was measured against a full per-layer union.
     `build_store(shapes, pins, deck, strings, merge: &[u16])` (its only caller is `Checker::load_geometry`,
     checker.rs:135–137; `Checker::new` stores `merge` = `pdk.routing_metals` ∪ `Process::layer(pdk, "li")`): for each
     layer id in `merge`, push that layer's rects into a one-layer
     `GeometryStoreBuilder`, `finish(1)`, `validate_layer_into(&tmp, LayerId(0), &mut raw)`,
     `gdsverify::geom::derive::merge_into(&raw, &mut merged)` (GP/crates/geom/src/derive.rs:23–26), then push each
     merged polygon into the main builder: a polygon without holes as its outer ring (`polygon.outer().coords()`);
     a polygon with holes (a metal loop around a ring) as the disjoint rectangles of
     `gdsverify::geom::rects::decompose_into` (GP/crates/geom/src/rects.rs:29–32) — exact area; the slab edges inside
     a loop still add fringe, a known ceiling limited to loops. Rects of other layers are pushed as today. Labels
     still land (the merged polygon covers every original rect).
  2. `backend/verify`: `pub struct ExtractOptions { pub field_solve: Vec<String> }`; `signoff_checked(…, opts:
     &ExtractOptions, pdk)`; `Checker::run(shapes, pins, checks, &opts)` sets `RunOptions.quasistatic_nets`
     (checker.rs:163–168). A `StageStatus::Refused` PEX with field nets re-runs PEX without them and records
     `("pex.field", reason)` in `coverage.skipped_rules` (a refused field solve is not a PEX failure).
  3. Net choice (library): matched routing nets (`Differential` and `CommonNode` rows) + the 8 [policy] labelled nets
     with the largest `Σ_b |∂f_b/∂C|`. Final signoff: always. Promoted epochs: only if the first promoted epoch's
     field-solve wall time ≤ 2 × its analytical PEX time [measure]; `Signoff` gains `pex_field: Duration` so the
     split is measurable next to FLOW-09's `stage_ms` signoff slot.
  4. `bench --pex-cal`: per fixture and signal net, C_unmerged (merge list empty), C_merged, C_field and C_magic
     (PERF-19) → `target/bench/pex_cal.json`; dac4 `top` and `b0..b3` included (evidence for CELL-08's MOM
     calibration: FLOW-06 leaves `mom_m1m2.c_area_af_um2: null` with "CELL/PERF calibrate", AV-32).
- Tests (`backend/verify/src/lib.rs` tests):
  - `partial_overlaps_extract_as_their_union` (sky130, one labelled met1 net): A = [0, 10]×[0, 0.5] µm and
    B = [9, 12]×[−0.25, 0.75] µm (unequal spans, so `merge_rects` would not join them). Union area 7.5 µm², perimeter
    26.0 µm; with met1 `area_cap 25.78aF/um2 fringe_cap 40.57aF/um` (GP/pdks/sky130.deck:590) the ground C is
    7.5·25.78 + 26.0·40.57 = 1248.2 aF (*computed*), against 1382.8 aF unmerged (A: 5.0·25.78 + 21·40.57; B:
    3.0·25.78 + 8·40.57). Assert the net's `(net, None, C)` row = 1.2482 fF ± 0.1 %.
  - `a_ring_loop_keeps_its_area`: a met1 square loop (outer 10 µm, inner 8 µm) drawn as four overlapping rects →
    ground-C area term = 36 µm² · 25.78 aF (fringe not asserted: slab edges).
  - `merging_does_not_add_drc_findings`: every local fixture's `verify::drc` count with merge ≤ without (run inside
    the existing `signoff_fixtures` loop).
  - `field_solve_changes_only_selected_nets` (three parallel met1 lines `a`, `b`, `c`: solving `a` changes C(a),
    leaves C(c)); `a_refused_field_solve_falls_back_with_coverage`.
- Acceptance: |C_field − C_magic|/C_magic ≤ 15 % [policy; open question 5] for every signal net ≥ 1 fF on the local
  fixtures; bench `C total` on ota drops by the unmerged excess (reported, the baseline moves once).
- Risks: BEM runtime unknown [measure]; that is why promotion-time use is gated on a measurement.

### PERF-17 Signoff intent arms IR drop; EM/IR coverage visible

- Priority: P1. Effort: S. Depends on: PERF-01.
- Why: AV-09 (only one DC number per supply net; `ir_drop` skips), AV-29 (no per-net intent beyond supplies),
  AV-26.
- Current: `Intent { supplies, currents }` (backend/verify/src/lib.rs:57–63); `set_intent` writes only
  `budget_current_ua` limits (checker.rs:123–128); GPurify's intent accepts `max_drop_mv`, `max_drop_fraction`,
  `max_overvoltage_mv`, `budget_current_ua` per net (GP/crates/ingest/src/intent.rs:149–154). The flow already
  computes IR budgets from op headroom (`annotator::ir::budgets`, lib.rs:283–290) for every current-carrying
  Supply/Ground net and every signal net carrying ≥ 10 % of the largest net current
  (backend/annotator/src/ir.rs:27–52). GPurify's `check_ir_drop` examines only nets declared as supplies
  (GP/crates/check/src/erc/rules/electrical.rs:176–201, "every grid node is on a declared supply"), so the
  signal-net triples arm nothing there; and a net stated twice in `limits` is refused (intent.rs:234–239), so the
  drop must go into the same entry as `budget_current_ua`.
- Change: `Intent` gains `pub max_drop_mv: Vec<(String, f64)>`; `set_intent` writes `"max_drop_mv"` into each net's
  `limits` entry; `elaborate::intent` (frontend/library/src/elaborate.rs:297–319) fills it from the same
  `annotator::ir::budgets` triples the in-loop `IrDrop` rules use (`max_drop_uv / 1000`). Bench column `EM/IR ran k/n`
  from `coverage.skipped_rules`.
- Tests: `backend/verify` `max_drop_arms_ir_drop` (sky130: with a limit, `ir_drop` is absent from
  `coverage.skipped_rules`); library `op_runs_carry_ir_limits` (ota with op: intent limits non-empty).
- Acceptance: the G.3 skip log no longer lists `ir_drop` for circuits with an operating point. Signal-net EM and
  `vgs.*`/`vds.*` remain skipped in GPurify (its intent has no per-net signal voltage; open question 6) and are
  reported so; the checks themselves are covered in Philis by REL-03 (per-segment EM, hard) and REL-10
  (`rel/vgs`, `rel/vds`, `rel/bulk_forward` from PERF-09's per-FET voltages).

### PERF-18 Complete LVS on sky130: MOS bulk, n-well net, BJT recognisers

- Priority: P1. Effort: M. Depends on: FLOW-04 (vendored `pdks/decks/sky130.deck` with `# PHILIS:` markers),
  PERF-02, PERF-16 step 1 (the merge list, extended here by `nwell`).
- Why: AV-05 (bulk not extracted, n-well not a net: a wrong body tie or a PMOS in a foreign well passes LVS), AV-01
  root cause (no BJT recogniser), CRATES open issue 1 (bjt_mirror, owned by PERF per plan-08 §1.6).
- Current: MOS recognisers `[poly_c, nsd, nsd]` / `[poly_c, psd, psd]` (GP/pdks/sky130.deck:542–550); the deck says
  "MOS bulk terminals are left off until the n-well is a net too" and bipolars wait on the same
  (sky130.deck:663–668); conductors exclude `nwell` (sky130.deck:506); `psub` is already a global net with a tie
  (`connect global psub`, `connect via psub_tie [ptap, psub]`, sky130.deck:118–119, 512–513). A latent bug waits for
  the first BJT recogniser: `cellgen::reference` lists BJT terminals `["E", "B", "C"]` (cellgen.rs:868–869) while
  GPurify reads reference cards collector-first, `(Bjt, 0) => Collector, (Bjt, 2) => Emitter`
  (GP/crates/check/src/lvs/graph.rs:289–302), and the parser names them `C B E` in card order (parse.rs:215). Every
  BJT would compare with emitter and collector swapped.
- How GPurify binds terminals (why no engine change is needed): a recogniser row binds, per terminal position, the
  polygons of that layer touching or overlapping the marker polygon (inclusive box prune, then exact intersection,
  GP/crates/check/src/topology/device.rs:172–173, 199–249); roles come from position, MOS `[Gate, Source, Drain, Bulk]`
  and BJT `[Base, Emitter, Collector]` (device.rs:449–459); a marker with more touching polygons of one layer than
  positions for it is refused (device.rs:240–249). Nothing is MOS-specific about a region terminal: the diode rows
  already bind the global `psub` (sky130.deck:560–563), and `compare` keeps reference `Bulk` whenever the layout
  extracts one (GP/crates/check/src/lvs/compare.rs:34–42).
- Change:
  1. `frontend/library/src/cellgen.rs:868–869`: BJT pins `["C", "B", "E"]` (card order); CELL-01's `drawn_cards` must
     emit BJT `Drawn.nodes` in the same C, B, E order (CELL-01 step 2 documents "E, B, C"; coordinate).
  2. `pdks/decks/sky130.deck` (FLOW-04's vendored copy), each edit under a `# PHILIS:` line:
     ```text
     # PHILIS: n-well as a net (AV-05), tied through the n-tap the way psub_tie ties the substrate (upstream 118-119)
     layer ntap_tie = ntap and nwell
     # PHILIS: replaces upstream line 506 (nwell appended)
     connect conductors [nsd, psd, ntap, ptap, poly_c, li_c, met1, met2, met3, met4, met5, nwell]
     connect via ntap_tie [ntap, nwell]
     # PHILIS: 4-terminal MOS; position 3 = Bulk (GP device.rs:449-454); same edit on nfet rows 543-545 (psub)
     #   and pfet rows 547-550 (nwell)
     device mos nfet_01v8 model "sky130_fd_pr__nfet_01v8" terminals [poly_c, nsd, nsd, psub]
     device mos pfet_01v8 model "sky130_fd_pr__pfet_01v8" terminals [poly_c, psd, psd, nwell]
     # PHILIS: vertical PNP as VOL/klayout/lvs/sky130.lvs:745 (pnpid = 82/44), 1089-1091 (E = p+ diff in nwell inside
     #   pnpid, B = n-tap inside pnpid, C = p-tap interacting pnpid). Order = Base, Emitter, Collector (device.rs:455-459).
     device bjt pnp model "sky130_fd_pr__pnp_05v5_W3p40L3p40" terminals [ntap, psd, ptap]
     # PHILIS: vertical NPN as sky130.lvs:744 (npnid = 82/20), 1063-1065 (E = n+ diff in dnwell inside npnid,
     #   B = p-tap inside dnwell, C = n-tap in the nwell ring)
     layer npn = gds(82, 20)
     device bjt npn model "sky130_fd_pr__npn_05v5_W1p00L1p00" terminals [ptap, nsd, ntap]
     ```
     The marker names start with `p`/`n`, which is how `recogniser_for` reads BJT polarity (reference.rs:152–158).
     `pnp` is already a drawn layer (sky130.deck:82) and the BJT generator draws the `pnp`/`npn` role marker over each
     unit (kernel/cells/src/bjt.rs:198–201); before this edit the deck had no `npn` layer, so NPN units carried no
     marker. Only the `W3p40L3p40` PNP gets a row: two rows on one marker would both bind and the earlier would win
     (device.rs:275–277), and GPurify cannot select by emitter area as the foundry deck does (sky130.lvs:1093–1100).
  3. The PERF-16 merge list gains the `nwell` role layer, so one drawn well is one polygon; otherwise a PMOS gate
     touching two overlapping well rects is refused (device.rs:240–249).
  4. `cellgen::reference` already emits `D G S B` (cellgen.rs:864–865); `reference::build` keeps 4 terminals once
     the arity is 4 (reference.rs:98–114).
  5. The bjt_mirror NPN model rename this row needs is PERF-08 step 6.
- Tests (`backend/verify`): `a_wrong_body_tie_is_an_lvs_mismatch` (minimal nfet whose body tap sits on net `X` while
  the reference says `VSS` → an `lvs/` row); `a_pmos_in_a_foreign_well_is_an_lvs_mismatch`;
  (`frontend/library/src/cellgen.rs`) `a_bjt_reference_is_collector_first` (bjt_mirror XQ1 → terminals
  `[outn, in, VSS]`); `signoff_fixtures`: bjt_mirror and bgr_core `unverified == 0` and LVS clean.
- Acceptance: T3 = 0 on every sky130 local fixture except MOM capacitors (open question 6).
- Risks: adjacent BJT unit markers or a shared base ring may give a marker two touching polygons of one layer, which
  GPurify refuses (device.rs:240–249); the bgr_core test shows it, and the fix is in the generator (CELL-09) or a
  merged marker. The `ntap_tie` edit makes every n-well a net, so a floating well now shows as an unconnected net
  in LVS instead of passing silently — intended.

### PERF-19 Foundry signoff harness; delete the broken cross-checks

- Priority: P1. Effort: M. Depends on: FLOW-12 (labelled GDS, AF-18; `gds::emit` returns `Result`), FLOW-14
  (the nightly job that has the tools). The harness itself can land before FLOW-12 for DRC; LVS needs the labels.
- Why: AV-02 (critical: no independent check runs), AV-24, audit-06 gap 1 ("DRC 0 means DRC 0 against Philis's
  reading of the manual"); docs/CRATES.md:96 open issue 1 (15 magic violations on bjt_mirror that GPurify does not report).
- Current: xcheck.py reads keys the sidecar lacks (benchmarks/xcheck.py:93–111), xcheck_pex.py likewise
  (xcheck_pex.py:53–54) and re-derives the same unmerged bookkeeping (xcheck_pex.py:2–5), `examples/drc_gds.rs:81`
  panics, the self-test uses obsolete rule ids (xcheck_selftest.py:29–48), xcheck_lvs.py tolerates 100 % W/L error
  (xcheck_lvs.py:79–82). KLayout is in the dev shell (flake.nix:84); magic is not installed; the foundry decks are
  (VOL/klayout/drc/sky130A_mr.drc, VOL/klayout/lvs/run_lvs.py + sky130.lvs, VOL/magic/sky130A.tech).
- Change:
  1. `flake.nix` devShell: add magic (nixpkgs attribute to confirm, open question 4).
  2. New `benchmarks/signoff_xcheck.py` (per `target/bench_debug/<f>/`):
     - KLayout DRC: `klayout -b -r VOL/klayout/drc/sky130A_mr.drc -rd input=<f>.gds -rd top_cell=TOP
       -rd report=<f>.drc.lyrdb -rd feol=1 -rd beol=1 -rd offgrid=1 -rd thr=<ncpu>` (variables `$input`,
       `$top_cell`, `$report`, `$feol`, `$beol`, `$offgrid`, `$thr` read at sky130A_mr.drc:25–33, 47–63, 88–92).
       `feol` and `beol` default to **false** (sky130A_mr.drc:41–57): a run without `-rd feol=1 -rd beol=1` checks
       nothing and reports 0 — the harness asserts both flags are passed. `seal` and `floating_met` stay off (a
       block has no seal ring). Error count = number of `<item>` elements in the lyrdb XML report.
     - KLayout LVS: `python3 VOL/klayout/lvs/run_lvs.py --design=<f>.gds --net=<f>.ref.spice --run_mode=deep
       --report=<f> --output_netlist=<f>.ext.cir` → match/mismatch. The script appends `.lvsdb` to `--report`
       itself and also passes its own `target_netlist=extracted_netlist_<net stem>.cir` (run_lvs.py:100). With
       `PDK_ROOT` set it loads `$PDK_ROOT/$PDK/sky130.lvs` (run_lvs.py:116–119), which for volare is not where the
       deck lives (VOL/klayout/lvs/sky130.lvs); unset `PDK_ROOT` for this call so it resolves the deck next to the
       script (run_lvs.py:120–123).
     - magic: `magic -dnull -noconsole -T VOL/magic/sky130A.tech` reading this script on stdin: `gds read <f>.gds; load TOP; select top cell;
       drc style drc(full); drc check; drc count total; extract all; ext2spice cthresh 0; ext2spice` (the tech file is
       passed directly because volare's `.magicrc` hard-codes its build path, benchmarks/README.md "Against the real
       PDK") → DRC count and `TOP.spice` with coupling/ground C.
     - Compare with GPurify: DRC counts, LVS verdicts, per-net C against `caps.json` → `target/xcheck/summary.json`
       and a table; exit 1 on any foundry DRC error or on a foundry LVS mismatch where GPurify said MATCH.
  3. `benchmarks/src/bench.rs` `run_circuit` writes, next to the GDS in `target/bench_debug/<name>/`,
     `<name>.ref.spice` (the preprocessed schematic it ran) and `caps.json` (the final `CapMatrix` from
     `verify::Signoff::caps`).
  4. Delete `benchmarks/xcheck.py`, `xcheck_pex.py`, `xcheck_lvs.py`, `xcheck_selftest.py`,
     `benchmarks/examples/drc_gds.rs`; rewrite README "Cross-checking GPurify".
- Tests: `benchmarks/tests/xcheck_smoke.rs` (FLOW-14's `tool_or_skip("klayout")`): writes two met1 rectangles
  1 µm × 1 µm, 100 nm apart (foundry m1.2 spacing 0.14 µm, VOL/klayout/drc/sky130A_mr.drc:439) with `library::gds::emit(..)?` and asserts the KLayout
  foundry DRC reports ≥ 1 met1 spacing error — the pair can see a planted fault; the same GDS with `feol`/`beol` unset
  reports 0, which pins why the flags are mandatory.
- Acceptance: summary for all local fixtures; every foundry finding is listed and attributed (cells/routing by layer
  as signoff_fixtures.rs:57–69 does); T1 gate enabled once the known findings are fixed by their owners.

### PERF-20 Bench v2: testbenches, seeds, faithful metrics, timing, JSON

- Priority: P1. Effort: L. Depends on: PERF-01, PERF-02, PERF-07, PERF-10, PERF-13; FLOW-09 (`stage_ms`), FLOW-12
  (`Interface::from_json`, `Config.interface`); columns from MAT-13 and REL appear when those land.
- Why: AV-03 (no hand/competitor comparison), AV-12 (tiny suite, one seed, no perf), AV-13 (MAGICAL testbenches as
  circuits; ASAP7 not comparable), AV-14 (area/WL definitions), AV-23, AV-27, SURV-03 (parity claims rest on
  post-layout vs schematic metrics, todaes22.txt L594–610), SURV-40 (OTA/comparator/VCO/SC family mix, L575–608),
  SURV-24 (label cost: 3–4 P&R iterations per PEX+sim, perf_driven_survey.txt L177–213), LAMP-46 (per-stage CPU
  split), AT-12 (no per-stage timing), NOTES-11 (make the perf tier the default in benchmarks).
- Current: see B1–B6 (§1.5).
- Change:
  1. Testbench sidecar `benchmarks/fixtures/<stem>.tb.json` (and `benchmarks/handcmp/<repo>.tb.json` for PERF-21):
     ```json
     { "op_testbench": "<DC bias bench, replaces the probe>",
       "testbenches": ["<ac bench with .measure>", "<offset bench: unity-gain feedback>"],
       "specs": [{ "metric": "gain_db", "better": "higher", "min": null, "max": null }],
       "spec_tol": 0.1,
       "scenarios": [{ "name": "tt27", "corner": "tt", "temp_c": 27, "params": { "vdd": 1.8 } }],
       "mc": { "corner": "tt_mm", "samples": null, "seed": 1 } }
     ```
     `spec_tol` is a declared **[policy]** degradation allowance (open question 2). `bench --calibrate <stem>` simulates
     the schematic at the first scenario and writes `min = v·(1 − tol)` for `higher`, `max = v·(1 + tol)` for `lower`.
     First sidecars: ota (gain_db, ugf, pm, offset_mv, power_uw — the gain/UGF bench is `examples/op_demo`'s,
     op_demo/src/main.rs:26–41), rc_filter (f3db). bench sets `cfg.op.testbench = op_testbench` and
     `cfg.performance`; fixtures without a sidecar run geometry-only and print `perf: none`.
  2. Seeds: `PNR_BENCH_SEEDS=1,2,3,4,5` (default `1`); every numeric column reports median, min, max.
  3. Columns: cells, nets, WL_sig, WL_sup, unrouted, DRC (GPurify), warnings, LVS {MATCH|MISMATCH|PARTIAL(n)}, ERC,
     C_total, C_sig, C_tier, area_gds (bbox of `sol.geometry()` in µm², replacing bench.rs:104–117), util, per spec
     {value, value/schematic, β}, min β, σ_os/μ_os (PERF-22), matched ΔC and ALRC failures (PERF-23), EM/IR ran,
     time total and per stage (FLOW-09's `stage_ms`), sims. Tables printed after the main one when the metadata
     carries them: "Matched sets" from MAT-13's `MetadataReport.matched` (σ_rand, σ_layout, μ_sys, allowance, usage,
     order, Φ — the columns MAT-13 step 4 names); REL's report fields (EM min margin and unknown count, LU count,
     antenna agreement, ring count/area, `rel/*` voltage findings — plan-06 §0.3 "PERF owns the bench columns").
  4. `RunStats` (lib.rs:104–124) timing is FLOW-09 step 4's `stage_ms: [u64; 9]`; this item adds only the
     performance counters `sims`, `sim_failures`, `refreshes: u32` and `fill_regressed: bool` (declared by PERF-06,
     PERF-11, PERF-15; listed here so the bench prints them) — `RunStats` stays `Copy`.
  5. `target/bench/results.json`: one object per (circuit, seed) with every column plus `git rev-parse HEAD`;
     `bench --compare <old.json>` prints per-metric deltas (T12).
  6. Artifacts per circuit in `target/bench_debug/<name>/` as PERF-19 step 3 defines, plus `perf.json` (per-bound
     values, β, scenario) for PERF-21.
  7. Hygiene: `discover_magical` skips `*_test.sp` (fixtures.rs:158–194); `Suite::from_str` →
     `Result<Suite, String>`, unknown words exit 1 (fixtures.rs:68–76); a fixture with `<stem>.interface.json` runs with
     `cfg.interface = Some(Interface::from_json(..)?)` (FLOW-12 step 3) — before FLOW-12 lands it is skipped with
     "interface contract unsupported" (either way the three bit-identical OTA rows disappear); ALIGN/MAGICAL rows on
     `generic_finfet` are tagged `not comparable (ASAP7, 0.1 µm/fin mapping)`; SVG layer names use FLOW-15 step 4's
     `layer_names(pdk.layers + pdk.layer_gds())` (bench.rs:420). The `preprocess_spice` `l=` append (AV-23) is removed
     by FLOW-07 step 11, not here.
  8. `benchmarks/tests/signoff_fixtures.rs` new test `known_optima_hold`: pair — the two cells' edge gap ≤ the place
     clearance (`place_rules`, lib.rs:529–538); chain4 — sorted along the row axis the cells are in chain order
     1-2-3-4 with gaps ≤ clearance (chain4.spice:2–3); quad — the 4-cell bbox is ≤ 2 cell widths × 2 cell heights
     (quad.spice:2); bgr_core — Q1's unit centroid equals Q2's within one grid step (bgr_core.spice:1–2). No
     `#[ignore]`: H09-49 finds `unit_order` correct for 1:8 on a 3-column grid (kernel/cells/src/bjt.rs:274–283), and
     a failure here is exactly the signal MAT-02/CELL-09 need; if the flow picks a non-3-column variant and fails, the
     test stays red and names the variant.
  9. New local fixtures from sized, hand-laid-out sources (PERF-21's repos) instead of invented sizings.
- Tests: `bench.rs` `results_json_roundtrips`, `seeds_parse_and_default_to_one`, `unknown_suite_is_an_error`;
  `fixtures.rs` `magical_testbenches_are_not_circuits` (temp dir with `a.sp` and `a_test.sp` → one circuit).
- Acceptance: `bench local` prints perf, β and timing for ota and rc_filter over 5 seeds and writes `results.json`;
  no duplicate OTA rows.

### PERF-21 Hand-layout comparison harness

- Priority: P1. Effort: L. Depends on: PERF-19, PERF-20, PERF-22, FLOW-07 (hierarchical xschem netlists, ports),
  FLOW-12 (labelled GDS).
- Why: AV-03 (critical: "beats hand layout" is unmeasured; TinyTapeout hand GDS found and ignored,
  fixtures.rs:51–54, 233–247), SURV-03 (claims rest on post-layout metrics against the same schematic), LAMP-47 (LAYLA
  lost 10–15 % area to manual layout, lampaert.txt L7094–7114), ref-00 §2.9 topic-validation "Fair comparisons,
  seeds, cache keys (L74–82)" (ref-00-prior-notes-handbook.md:331; not a NOTES entry — NOTES-57 is the
  discriminating validation catalog).
- Current: none. The pinned TinyTapeout repositories are sky130 tapeouts with hand GDS under `gds/tt_*.gds` and
  xschem netlists (fixtures.rs:25–40, 196–258). MAGICAL records one manual-layout number (OTA_4 CMRR 97 dB,
  benchmarks/competition/MAGICAL-CIRCUITS/benchmark_circuits/OTA/README.md) on a different PDK: not usable as a
  like-for-like reference.
- Change: `benchmarks/handcmp.py` + `benchmarks/handcmp/<repo>.tb.json`.
  1. Reference set (open question 3 confirms content and licence): tt10-OTA_FC, tt08-bgr, ttsky25a-2stageCMOSOpAmp,
     tt09-analog-opamp-3stage, tt08-analog-r2r-dac-3v3, and the comparator of tt06-sar if its subcell is separable.
  2. Philis side: `bench tinytapeout <repo>` with the tb sidecar → labelled GDS + `ref.spice` (the block's own
     netlist, so sizing is identical to the hand layout).
  3. Hand side: magic `gds read gds/tt_*.gds; load <cell>` where `<cell>` = the block's top `.subckt` name (or
     `--cell`); bbox area from the loaded cell.
  4. One judge for both layouts: magic `drc(full)` count; KLayout foundry LVS against `ref.spice`; magic
     `extract all; ext2spice cthresh 0` → `<side>.pex.spice`; ngspice runs every testbench × scenario with
     `.include <side>.pex.spice` and an instance of the cell, plus the schematic for ratios; Monte Carlo with the
     `mc.corner` library section and `.option seed=k` per sample (the pattern of characterize_mismatch.py:77, 91–92).
  5. Output `target/handcmp/<repo>.json` and a table: T1, T2, T4, T6 (per metric ratio to schematic), T7 (μ_os, σ_os),
     T8 (max |ΔC|/C of the tb's differential nets from the extracted netlists), T9 (area), Philis runtime. Verdict per
     metric and overall "beats hand" iff all of T1–T9 hold for Philis and are at least as good as the hand values.
- Tests: `python3 benchmarks/handcmp.py --self ota`: hand = Philis's own GDS → every metric ties (the judge is
  deterministic and symmetric).
- Acceptance: the table exists for ≥ 3 reference circuits; each lost metric opens an item in its owner's plan (area →
  PLC, parasitics → RTE, offset → MAT, DRC → CELL/RTE).
- Risks: TT GDS hierarchies may flatten the block into the tile (then `--cell` names the nearest enclosing cell and the
  area comparison is flagged); magic's extraction is C-only unless `extresist` is added (reported as "C-only" for both
  sides, so the comparison stays fair).

### PERF-22 Post-layout Monte Carlo and yield report for the winner

- Priority: P1. Effort: M. Depends on: PERF-10, PERF-11 (job pool), PERF-13, PERF-15 (and through it FLOW-10's
  post-fill caps).
- Why: GRAEB-30/31 (Ŷ = n_ok/n, σ²_Ŷ = Y(1−Y)/n, eq. 219/225 L4771–5035; n from eq. 232, L5097–5129: 85 % ± 1 % at
  99 % needs 8 461), GRAEB-32 (MC only for signoff, not in the loop, L5126–5138, L6257–6274), GRAEB-44 (WCD vs MC
  within 1–3 %, L4634–4685), MM-29 (SPICE-domain mismatch, drennan_mismatch.txt L21–29, L229–236), MM-36/39 (separate
  systematic mean from random σ, distance_vs_pair_mismatch.txt L86–111), NOTES-58.
- Current: Monte Carlo only in `benchmarks/characterize_mismatch.py:74–103`.
- Change (`frontend/library/src/perf.rs`):
  ```rust
  pub struct McConfig { pub corner: String /* sky130: "tt_mm" */, pub samples: Option<usize>, pub seed: u64 }
  pub struct McResult {
      pub n: usize,
      pub metrics: Vec<(String, Option<f64> /* mean */, Option<f64> /* σ */, usize /* measured */)>,
      pub yield_mc: Option<f64>, pub sigma_yield: Option<f64>,
      pub yield_linear: Option<f64>,        // PERF-13
      pub agrees: Option<bool>,             // T10
  }
  pub fn monte_carlo(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig, mc: &McConfig,
                     prior_yield: f64) -> Result<McResult, String>;
  ```
  Per sample k: the nominal scenario with its corner replaced by `mc.corner` and `.option seed={seed·1000+k}`; samples
  run on the PERF-11 pool. Default n = ⌈Y(1−Y)·k²/ΔY²⌉ with Y = clamp(prior, 0.5, 0.99), k = 1.960 (95 %), ΔY = 0.05
  [policy] (≤ 385 at Y = 0.5), at least 50. Yield is reported only when #pass > 4, #fail > 4, n ≥ 10
  (L5060–5071); otherwise the zero-failure bound 1 − 0.05^(1/n). `agrees = |yield_linear − yield_mc| ≤ 0.03 +
  3·σ_Ŷ`. Runs once, on the post-fill winner (`solve`, after PERF-15), when `cfg.performance.mc` is set; results in
  `metadata.monte_carlo`.
- Tests (`frontend/library/tests/perf_postlayout.rs`, `tool_or_skip("ngspice")` and a models check): `mc_offset_of_a_pair_follows_pelgrom`: a
  2-FET differential-pair offset bench, 200 samples, σ within ±25 % [policy: 5 % sample error + model deviation from
  1/√area, MM-03] of A_VT/√(WL) from the deck; `different_seeds_give_different_samples`.
- Acceptance: ota winner reports σ_os, μ_os, Ŷ and the agreement flag (T7, T10).

### PERF-23 Matched-structure signoff report and analog layout-rule check (ALRC) on the final geometry

- Priority: P1. Effort: M. Depends on: PERF-01, PERF-16; CELL-01 (`Keepout::Gate{owner}` rects), FLOW-06
  (`tier_nm`), MAT-07/EXT-16 (class per matched set; default `Moderate` when none, MAT open question 4), MAT-13
  (ledger printed beside).
- Why: AV-11 (no analog check on the final GDS: symmetry, centroid, metal over matched gates, ΔC on differential nets),
  AV-06 (output-pair ΔC 0.49 fF, 8 %, unreported), BAL2-22 (per-aggressor coupling asymmetry, balasa_graeb_survey.txt
  L8905–8909, L9745–9753), Hastings §13.3 rules 12, 16, 17, 19, 21, 23 (H13-47, H13-50, H13-51, H13-53, H13-54,
  H13-55; hastings.txt L42532–42654, PDF pp.714–716), AV-32 / cc_dac_constructive.txt §II-A L140–166 (C_TS, C_TB of
  DAC plates), NOTES-29/43. CELL-12's acceptance and CELL T7 name "PERF's analog-rule check on the final GDS (AV-11)"
  as their judge for rules 12, 19, 21 and 22 (plan-03 CELL-12, T7); without the environment rows below that judge
  does not exist.
- Current: bench grades constraints with the flow's own rule models (bench.rs:259–288); nothing reads extracted
  parasitics per matched net.
- Change: new `frontend/library/src/matched.rs`:
  ```rust
  pub struct MatchedNetRow { pub a: String, pub b: String, pub ground_ff: (f64, f64),
                             pub worst_aggressor: Option<(String, f64)>, /* |C(a,x) − C(b,x)| fF */ pub rel: f64 }
  pub struct GateCoverRow { pub members: Vec<DeviceId>, pub layer: String,
                            pub over_gate_um2: Vec<f64>, pub asymmetry: f64 }
  pub struct CapBankRow { pub top: String, pub bits: Vec<(String, f64)>, pub ratio_error: Vec<f64>, pub cts_over_ctotal: f64 }
  /// Hastings §13.3 environment of one matched set, measured on the placed geometry, nm / µm².
  pub struct EnvRow {
      pub members: Vec<DeviceId>, pub class: pnr_core::MatchClass,
      pub contact_over_gate_um2: f64,   // rule 16: (licon ∪ mcon) ∩ gate area
      pub dummy_reach_nm: [i32; 2],     // rule 12: per row end, outermost poly edge − outermost active gate edge
      pub well_edge_nm: Vec<i32>,       // rule 19: per member, min distance active gate → nearest nwell edge
      pub gate_ext_nm: (i32, i32),      // rule 21: (min, max) poly overhang past diffusion over the set's gates
      pub foreign_poly_nm: Option<i32>, // rule 23: min distance gate → poly of another cell (report only)
      pub fails: Vec<&'static str>,     // "r12", "r16", "r17", "r19", "r19eq", "r21"
  }
  pub struct MatchedReport { pub nets: Vec<MatchedNetRow>, pub gates: Vec<GateCoverRow>, pub banks: Vec<CapBankRow>,
                             pub env: Vec<EnvRow> }
  pub fn matched_report(sol: &Solution, caps: &verify::CapMatrix, pdk: &Pdk) -> MatchedReport;
  ```
  - Environment rows (one per matched set, gates = CELL-01's `Keepout { why: Gate { owner } }` rects of the placed
    macro, which `place_macro` transforms like shapes). Fail rules, only for `Moderate`/`Exceptional` sets (Hastings
    exempts `Minimal` for rules 16, 17, 21): r16 `contact_over_gate_um2 > 0`; r17 any `GateCoverRow.over_gate_um2 > 0`;
    r12 `min(dummy_reach_nm) < tier_nm("dummy_poly_reach_nm", class)` when the tier exists (sky130 EXC 10 000);
    r19 `min(well_edge_nm) < w` with `w = tier_nm("wpe_clearance_nm", class)` (2000 / 3000 / 5000), for EXC
    `max(w, 2·n_well_depth)` as FLOW-06 step 2 prescribes (sky130 `n_well_depth` 2000 nm, pdks/sky130.json:68 → EXC 5000 nm); r19eq `max − min` of `well_edge_nm` > one cut-lattice
    step (unequal WPE); r21 `gate_ext_nm.1 ≠ gate_ext_nm.0` (Hastings: "no gate extends further than its
    neighbours"). Rule 22 (metal gate straps) is a generator property CELL-12 tests itself. Rows are reported, not
    counted in `|V|` (the flow's own `MatchedSet`/`Environment` rules drive the search; this is the independent judge).
  - Net pairs: the `Differential` and `CommonNode` pairs in the run's routing requirements. `Solution` does not
    carry them today (its fields: frontend/library/src/lib.rs:82–101, placement rules only), so `solve` must store
    those pairs in `Solution` (or `matched_report` re-annotates, as bench does at benchmarks/src/bench.rs:269). `rel` =
    max(|ΔC_ground|, max_x |ΔC_x|)/mean ground C.
  - Gate cover: for every matched cell (units of more than one owner, the predicate the fill pass uses at
    lib.rs:417–418), gate rects =
    poly ∩ diff rects of the placed macro; metal area over them per member and per layer (li, met1 upward, routes and
    cell metal); `asymmetry = (max − min)/mean gate area`.
  - Cap banks: a net with ≥ 3 coupling partners that are bottom plates of one capacitor unitization: C_k from the
    top–b_k coupling, `ratio_error_k = C_k/(2^k·C_unit) − 1` with C_unit the smallest; `cts_over_ctotal` = top-plate
    ground C / Σ C_k.
  - MAT-13's `LedgerRow`s (the flow's own σ/μ model, `MetadataReport.matched`) are printed beside these rows; this
    report is stored as `MetadataReport.matched_signoff: MatchedReport` so it does not collide with MAT-13's
    `matched` field.
  - Bench columns max `rel`, max `asymmetry`, max |ratio_error|, ALRC fails (count per rule).
- Tests: `bank_ratio_from_a_synthetic_matrix` (couplings 0.42, 0.85, 1.67, 3.36 fF → ratio errors 0, +1.2 %,
  −0.6 %, 0 %, from audit-06 G.4(a)); `ota_output_pair_row_exists` (flow run: a (vout1, vout2) row with ground C of
  both sides); `gate_cover_is_zero_for_an_unrouted_pair`; `a_contact_on_a_gate_fails_r16` (synthetic placed macro:
  one licon square inside a `Gate` keepout of a `Moderate` set → `fails == ["r16"]`);
  `unequal_well_distance_fails_r19eq` (two members 3000 and 3500 nm from the n-well edge, lattice 10 nm → `r19eq`);
  `a_minimal_set_is_exempt_from_r16_r17_r21`.
- Acceptance: T8 measured on every fixture with matched pairs; dac4 reports its bit-ratio errors (≤ 2 % today per
  G.4(a)); every matched set on the local fixtures has an `EnvRow` (CELL-12's acceptance reads it).

### PERF-24 A/B protocol and degradation attribution

- Priority: P2. Effort: M. Depends on: PERF-13, PERF-20.
- Why: LAMP-33 (comparator A/B: offset 6.9 → 3.7 mV, delay 5.4 → 2.8 ns, Table 4.1 lampaert.txt L5006–5012;
  §4.12.1 L5020–5082), LAMP-35 (report the
  most important contributions to the degradation, L3466–3471; Tables 4.2 and 4.6), SURV-39 (offset from routing R is
  measurable, todaes22.txt L837–858).
- Current: no A/B; metadata stores per-spec value and miss only (lib.rs:432–438).
- Change: `Config.performance_drives: bool` (default true); false = EXT-17 receives `Evidence { sens: None, .. }`
  (so EXT-25 builds no performance rows and EXT-21 falls back to its `max_eta` cap), PERF-12 step 2 leaves
  `r_weight`/`pair_weight` empty, net weights come from classes only — but promoted epochs are still simulated (the
  judge stays). `PNR_BENCH_AB=1` runs each sidecar fixture both ways over
  the seeds and prints per-spec median deltas. Attribution for the winner per bound (sorted, top 5 in `Display`, all in
  JSON): ground C per net (`d·C`), coupling per pair, series R per terminal, device systematic (`s·δ_sys`), random
  share (`s²σ²/σ_f²`).
- Tests: `attribution_sums_to_the_linear_prediction` (Σ contributions = Σ rows d·Δp within 1e-9 relative).
- Acceptance: T11 on ota and rc_filter over 5 seeds.

### PERF-25 Limiting-spec feedback into net weights — deferred

A third controller on the weights PLC-17 (per-epoch extracted-C feedback, BAL2-19 damping, 4× cap) and RTE-21 step 5
(budget repair, same damping and cap) already adjust. Moved to "Cut or deferred items" with its re-entry condition.

### PERF-26 The post-layout deck carries the drawn devices: junction geometry, gate R, inserted diodes

- Priority: P1. Effort: M. Depends on: PERF-08 (diode cards), CELL-19 (`Figures.sd`, `Figures.gate_ohm`).
- Why: AF-09 (the post-layout deck "omits AD/AS/PD/PS, gate R, cell R, WPE, dummies, diodes and non-FETs … Folding
  and diffusion sharing are invisible to the performance tier", audit-07); S6 in §1.3; LAMP-19 (junction C from area
  and perimeter, lampaert.txt L2011–2062, cited by CELL-19); CELL-19's acceptance names this consumer ("PERF's
  post-layout deck carries AS/AD/PS/PD from `Figures`"). Diffusion sharing is one of the few levers where a generator
  beats a hand layout on every device (CELL-19/20), and today the judge cannot see it.
- Current: the sky130 FET subckt defaults `ad = as = pd = ps = 0` and `nrd = nrs = 0`
  (`~/.volare/sky130A/libs.ref/sky130_fd_pr/spice/sky130_fd_pr__nfet_01v8__ss.pm3.spice:30`: `.param l = 1 w = 1 nf = 1.0 ad = 0 as = 0 pd = 0 ps = 0 nrd = 0 nrs = 0 sa = 0 sb = 0 sd = 0 mult = 1`; the ff/tt files repeat it), and neither deck sets
  them, so no drain/source junction C is simulated at all; `score_perf` evaluates `self.netlist` (lib.rs:878–893),
  which does not contain the epoch's inserted antenna diodes (`Epoch.extra`, lib.rs:552–554; appended to the netlist
  only for the winner, lib.rs:442–444); `parasitics()` builds `lod_inv_um` for `self.netlist.devices.len()` devices
  only (lib.rs:719–757).
- Change (`frontend/library/src/lib.rs`, `perf.rs`):
  1. `perf::Parasitics` gains `pub junction: Vec<Option<[f64; 4]>>` (per device: AS, AD in µm², PS, PD in µm — the
     sky130 library sets `.option scale=1.0u`, VOL/ngspice/all.spice:2, so the card takes µm units like W/L) and
     `pub gate_ohm: Vec<Option<f64>>`.
  2. `Flow::parasitics` fills them from the placed macros' `figures`: for cell c with members `devices_of[c]`, sum
     `Figures.sd` entries `(owner, "S"|"D", area nm², perimeter nm)` per `(devices_of[c][owner], terminal)`; per SPICE
     instance divide by the device's `m` (the card has `m` copies, FLOW-01); nm² → µm² ÷ 1e6, nm → µm ÷ 1e3. `gate_ohm`
     = `Figures.gate_ohm` of the owner.
  3. `perf::deck`'s `extra(di)` appends ` as={} ad={} ps={} pd={}` when `junction[di]` is `Some`; a non-zero
     `gate_ohm` becomes a series resistor on the gate exactly as `series` branches are drawn (perf.rs:115–127), node
     `{net}__{di}_G`.
  4. `score_perf` and PERF-15's post-fill evaluation simulate `netlist ∪ epoch.extra` (a clone with `extra` appended,
     as `solve` does for the winner at lib.rs:442–444); the diode card is PERF-08's; `Parasitics` vectors are sized
     to the extended device count (the diode's `junction`/`gate_ohm` = `None`).
  5. SA/SB unit check: perf.rs:137 writes `sa`/`sb` in metres (`s * 1e-6`) while the library's `.option scale=1.0u`
     scales instance geometry; whether ngspice applies `scale` to BSIM4 `sa`/`sb` is not stated in the audited sources.
     The test below pins it before the AS/AD units are trusted, and the one that is wrong is fixed in the same commit.
- Tests:
  - (`perf.rs`) `junction_params_reach_the_card`: a one-FET netlist with `junction = [2.0, 1.5, 6.0, 5.0]` → the card
    contains `as=2 ad=1.5 ps=6 pd=5`.
  - (`perf_postlayout.rs`, `tool_or_skip`) `scale_applies_as_the_deck_assumes`: nfet `W=1 L=0.15`, op point with
    `ad=1` vs `ad=100` (µm² under scale) → the drain-bulk capacitance printed by `show m : cbd` differs by > 10×;
    and `sa=0.5` vs `sa=5e-7` identifies the unit `sa` is read in (the deck writes whichever moves Id).
  - (`lib.rs`) `an_inserted_diode_is_simulated`: an epoch with one `extra` diode → the evaluated deck contains its card.
- Acceptance: on ota, the post-layout gain/UGF with junction params differs from without (reported, sign not asserted);
  CELL-19's acceptance ("PERF's post-layout deck carries AS/AD/PS/PD from `Figures`") holds.
- Risks: until CELL-19 lands, `figures` is empty and `junction` is `None` everywhere (today's behaviour, reported as
  "junction geometry: not drawn-derived").

### PERF-27 Per-set SPICE Monte Carlo σ for MAT-21

- Priority: P2. Effort: S. Depends on: PERF-09 (`scratch_dir`), PERF-11 (`run_jobs`), PERF-22 (`McConfig`).
- Why: MAT-21 ("PERF produces it (one ngspice batch per unique geometry and bias, before the epoch loop)", plan-02);
  MM-29 (SPICE-domain mismatch, drennan_mismatch.txt L21–29); MM-20 (ratioed mirrors do not follow the area law,
  Drennan Table III, L286–308). The method already exists offline: diode-connected devices at constant current in the
  mismatch corner, one ngspice run holding many instances because each instance draws its own AGAUSS
  (benchmarks/characterize_mismatch.py:1–12, 60–97).
- Change (`frontend/library/src/perf.rs`):
  ```rust
  /// σ(ΔV_GS), mV, of two identical `model` devices (W_total, L µm, nf, m) forced at `id_ua` each, diode-connected
  /// (V_DS = V_GS), in `mc.corner`: `pairs` pairs in one ngspice run (chunked by 50, as characterize_mismatch.py:71),
  /// seed `mc.seed·1000 + chunk`. `Err` when ngspice or the corner is missing.
  pub fn pair_sigma_mc(sim: &OpConfig, mc: &McConfig, model: &str, pmos: bool, w_um: f64, l_um: f64,
                       nf: u32, m: u32, id_ua: f64, pairs: usize) -> Result<f64, String>;
  ```
  Card per device k: `I{k} 0 g{k} {id}u` (NMOS; PMOS `I{k} g{k} 0 {id}u`), `X{k} g{k} g{k} {rail} {rail} {model}
  W={w} L={l} nf={nf} m={m}` with `rail` = `0` for NMOS and a `Vdd` node for PMOS (characterize_mismatch.py:60–68
  uses 0 for both with a sign flip; this keeps the source at the rail). Control: `op` and `echo VG {k} $&v(g{k})`
  per device; σ = sample stdev of `v(g_{2i}) − v(g_{2i+1})` in mV. `pairs` default 250 (the offline default,
  characterize_mismatch.py:129). The library calls it once per unique `(model, W, L, nf, m, id)` among MAT's matched
  sets, with `id_ua` from `OpPoint.id_ua`, and hands the result to `MatchedSet.sigma_rand_override` (MAT-21).
- Tests (`perf_postlayout.rs`, `tool_or_skip`): `pair_sigma_matches_the_deck_avt`: sky130 nfet W = 4 µm, L = 1 µm at
  0.4 µA (= 0.1 µA·W/L, the offline bias) → σ within ±25 % [policy, as PERF-22] of `avt_n_mv_um`/√(W·L) = 9.5/2 =
  4.75 mV.
- Acceptance: MAT-21's ledger rows show the MC value for ota's pair and mirror.

### PERF-28 Top-η width-candidate simulation for RTE-22

- Priority: P2. Effort: M. Depends on: PERF-10 (scenarios), PERF-11 (pool), RTE-22 (`width_candidates`,
  `DetailedCfg::width_mult`).
- Why: SURV-33 (relax, round, then simulate the top η = 10 candidates, todaes22.txt L486–498); RTE-22 step 6 ("PERF
  simulates them and passes the winner back as `width_mult`") and its acceptance (post-layout FOM ≥ max(uniform 1×,
  2×), plan-05). Without a simulating caller, RTE-22's linear model is never checked against the judge it cites.
- Change (`frontend/library/src/lib.rs`, after the winner is chosen and before fill):
  1. `let cands = dr::width_candidates(&pre, &rows, 10)` (η = 10 as SURV-33); plus the two uniform vectors (all 1, all
     2) for the acceptance comparison.
  2. For each candidate (on `run_jobs`): re-run `dr` on the winner's placement and global routes with
     `cfg.width_mult = cand`, `signoff_shapes` → caps, `perf::evaluate` on the active scenarios.
  3. Keep the candidate with the best key (PERF-14) if it beats the incumbent; record every candidate's
     `(min β, Θ, c_tier, |V|)` in `MetadataReport.width_candidates`.
  Cost: 12 × (dr + signoff + |active|·|testbenches| sims) once per run, counted in `RunStats.sims`.
- Tests: (`lib.rs`) `a_worse_width_vector_never_replaces_the_winner` (stub candidates whose evaluation is forced
  worse → incumbent kept); (`perf_postlayout.rs`) on ota with a sidecar: the report lists 12 candidates.
- Acceptance: RTE-22's acceptance can be evaluated from `width_candidates` (uniform 1× and 2× rows present).

### PERF-29 Worst-case verification simulation per bound on the winner

- Priority: P1. Effort: S. Depends on: PERF-11 (`Parasitics.gate_offset_v`, `run_jobs`), PERF-13 (`BoundStat`,
  `device_sigma_v`), PERF-15 step 2 (post-fill parasitics of the winner).
- Why: GRAEB-17 (realistic worst-case parameter vector, Graeb §5.2.1 eqs. 171–174, graeb_centering.txt L4189–4204;
  worst-case values f0 ∓ β_W·σ_f, eqs. 176–179, L4214–4221; vectors of different specs may coincide or cluster,
  §4.7.1 L3268–3275). PERF-13's β is a linear extrapolation from one-σ-step derivatives; nothing in the plan checks it
  at the 3σ point it certifies. T5 ("robust") and every β column of the hand comparison rest on it. Not in any other
  plan (no plan cites GRAEB-17).
- Current: none (β does not exist yet, P13).
- Change:
  1. `frontend/library/src/robust.rs`:
     ```rust
     /// Graeb eq. 172/174 with C = diag(σ_i²) (independent device V_T): δ_i = ∓(β_target/σ_f,b)·σ_i²·s_bi, V;
     /// minus for a lower bound, plus for an upper. Its linear prediction is f ∓ β_target·σ_f,b (eq. 177/179).
     /// `None` when σ_f,b is `None` or 0. `s[i]` = ∂f/∂δ_i from the bound's `GateOffset` rows (PERF-11).
     pub fn worst_case_offsets(sigma_f: Option<f64>, s: &[Option<f64>], sigma_v: &[Option<f64>], upper: bool,
                               beta_target: f64) -> Option<Vec<f64>>;
     ```
     A device with `s` or `σ` `None` gets δ = 0. The gradient term of PERF-13 (`grad` pairs) is left out; the
     vector is the random-V_T worst case, labelled "V_T only" as PERF-13 is.
  2. `BoundStat` gains `pub wc_value: Option<f64>` (simulated metric at the vector, metric units),
     `pub wc_pass: Option<bool>` (bound met at the vector) and `pub lin_err: Option<f64>`
     = `|wc_value − (f_post ∓ β_target·σ_f)| / (β_target·σ_f)` (dimensionless).
  3. `frontend/library/src/lib.rs`, after PERF-15 step 2 and before PERF-22: for each bound with a vector, clone
     the winner's post-fill `Parasitics`, set `gate_offset_v = δ`, and `perf::evaluate(netlist, &par, cfg,
     &[bounds[b].scenario])`; all bounds on `run_jobs`. Vectors of bounds that share a scenario and have cosine
     similarity > 0.99 [policy; GRAEB-17's recipe, Graeb only notes clustering] share one simulation, whose result
     fills each of them. Sims counted in `RunStats.sims`; at most one per bound (≤ 2·n_specs).
  4. Metadata prints per bound `f_wc`, pass/fail and `lin_err`; `lin_err > 0.2` [policy, same tolerance as
     PERF-11's `lin_tol`] is flagged "β extrapolation unreliable". Report only: the epoch key is not changed (the
     simulation runs once on the winner).
- Tests:
  - (`robust.rs`) `worst_case_offsets_follow_graeb_172`: s = [+1000, −1000] dB/V, σ = [1 mV, 1 mV], β_target 3,
    σ_f = √2 dB, lower bound → δ = [−2.1213e-3, +2.1213e-3] V ± 1e-7 and Σ s_i·δ_i = −3·√2 = −4.2426 dB ± 1e-4;
    `upper` → both signs flip; `sigma_f = None` → `None`.
  - (`lib.rs`) `clustered_vectors_share_one_simulation`: two bounds on one scenario with s and 2·s → one job.
  - (`perf_postlayout.rs`, `tool_or_skip("ngspice")`) `wc_sim_of_a_pair_offset_is_linear`: PERF-22's 2-FET
    differential-pair offset bench, bound `offset_mv ≤ 5` → `lin_err ≤ 0.05` [policy: offset of a pair is linear in
    ΔV_T to first order].
- Acceptance: ota winner prints `wc_value`, `wc_pass`, `lin_err` for every bound with known σ_f; sims added
  ≤ 2·n_specs.

### PERF-30 A simulatable post-layout netlist as a deliverable (added at the M0 close-out)

Field report: FR-7.

- Priority: P0 (a post-layout netlist that cannot be simulated is not a deliverable). Effort: M. Depends on: FLOW-17
  (MOS bulk recognisers, `philis run -o`), FLOW-07 (ports). Upstream: the SPICE writer is GPurify's
  (`gdsverify::export::netlist::write_spice`); its fixes are filed upstream (§6.4 Q7 in 00-MASTER-PLAN).
- Why: FR-7: the reporter's `extracted_pex.spice` had no `.subckt`, generic `nmos`/`pmos` models, no R/C/diodes,
  parasitics only as comments, NMOS bulks on `vdd`, swapped D/S, merged parallel FETs, ports `n<id>` without
  `--interface`, so they rebuilt it with their own script. That file came from the old packaged build; the m0 writer
  differs (below) but is still not simulatable. SURV-01: the simulation is the judge (perf_driven_survey.txt L15–37).
- Current (m0; measured with a throwaway example: `library::run` on `benchmarks/fixtures/rc_filter.spice`, sky130,
  `starts = 1`, then `verify::extract_spice(&sol.geometry(), &[], &pdk, Detail::WithParasitics)`; not committed):
  - `verify::extract_spice` (backend/verify/src/netlist.rs:25) is called only by `Elaborated` (frontend/library/
    src/elaborate.rs:82) and two cell tests; `library::run`'s `Solution` and the CLI never write it.
  - Output: `.subckt TOP` with no port list when no pins are passed; deck model names
    (`sky130_fd_pr__nfet_01v8`); `w=500 l=150` in database units ("lengths below are in database units"); MOS cards
    with three nodes (`M1 n0:0 n1:0 n3:0 sky130_fd_pr__nfet_01v8 …`: no bulk, so ngspice reads the model name as the
    bulk node); the resistor as `R6 n0:0 n4:0 sky130_fd_pr__res_generic_po area=1000000` (no value, no W/L); parasitics
    are real `Cp…`/`Rp…` elements on split nodes `n0:k`.
  - sky130 ships its devices as `.subckt`s (PERF-08 step 3), so an `M`/`R` card cannot call them.
- Change:
  1. `frontend/library/src/lib.rs`: `pub fn post_layout_spice(sol: &Solution, pdk: &Pdk) -> Result<String, String>` =
     `verify::extract_spice(&sol.geometry(), &labeled_pins(..), pdk, Detail::WithParasitics)` with the same labels
     signoff uses (`labeled_pins`, lib.rs on m0 after `labels_and_reference`), so every labelled net keeps its name;
     the `.subckt` header lists the FLOW-07 ports (`Netlist.ports`) in declaration order, named as in the schematic.
  2. Card rewrite in that function, until the GPurify writer does it (each a filed upstream request): lengths in µm
     with `u` suffix (or one `.option scale=1e-9` line, whichever the writer supports); every device whose deck model
     is a sky130 `.subckt` becomes an `X` card with the terminal order of its model file (MOS `d g s b`, resistor
     `r0 r1 [b]`, as PERF-08 step 3 lists); a MOS card without a bulk node is an error, never a guess (FLOW-17 brings
     the 4-terminal recognisers); a resistor card carries `w`/`l` in µm; parallel FETs stay separate cards (`m` only
     when the writer merges, documented in the header comment).
  3. `philis run -o DIR` (FLOW-17/FLOW-12) writes `DIR/<top>_pex.spice` from step 1; the bench writes it to
     `target/bench_debug/<name>/`.
- Tests (`frontend/library/tests/perf_postlayout.rs`, gated on `library::tools::sky130_models()`):
  - `post_layout_rc_filter_simulates`: run rc_filter, write `post_layout_spice`, wrap it with the sky130 lib and
    supplies, ngspice `.op` → exit 0, no `Error` line on stderr, and `v(vout)` within 1 % of the schematic `.op` value
    (both inverter outputs settle at the same DC point; R/C do not move it).
  - `post_layout_ports_are_the_schematic_ports`: the `.subckt` line of ota's output equals the ota `.subckt` port list.
  - `every_mos_card_has_four_nodes`: parse the output; every `X…` card calling an `nfet`/`pfet` model has 4 nodes and
    its bulk is `VSS`/`VDD` as in the schematic (FR-7's "NMOS bulks on vdd").
- Acceptance: the three tests green with ngspice and the models present (loud skip otherwise); FR-7's reporter can
  drop their rebuild script.
- Risks / notes: D/S naming is symmetric for a MOS; ngspice does not care, LVS compares it as swappable.

---

## 4. Milestones

Order inside a milestone is the order of the items' "Depends on"; all P0 bug fixes are in M0/M1. External items each
milestone waits on are listed; nothing here depends on a later PERF milestone (acyclic).

| Milestone | Items | Waits on | Exit criteria (all measurable) |
|---|---|---|---|
| M0 Honest signoff (done in M0 except PERF-03 steps 3–4, see each Status) | PERF-01, 02, 03, 04, 06, 07 | FLOW-02 (optional), FLOW-07 (PERF-03 step 3), EXT-02 (PERF-07 classes) | `cargo test --workspace` green except the dac4 row of `signoff_fixtures` (REL-02 owns it); bench LVS column `PARTIAL(2)` bjt_mirror, `PARTIAL(9)` bgr_core, `PARTIAL(16)` dac4 (Σm = 1+1+2+4+8), `MATCH` for the other 7; `m1.density`…`m4.density` listed `ChipLevel` for every fixture; ota with the old `.subckt` header has 2 `floating_gate` rows (vbias, vbn), 0 with the new one; `supply_decoupling_does_not_rank_layouts` green; `warnings` printed per circuit and absent from `|V|`. |
| M1 Faithful simulation | PERF-08, 09, 10, 17, PERF-30 (M0 close-out, FR-7; after FLOW-17) | FLOW-01, FLOW-07, EXT-02, FLOW-09 step 2 | rc_filter and bjt_mirror: `BiasSummary.resolved == devices`; an inductor netlist reports "cannot simulate"; ota evaluated at `{tt 27, ss 125, ff −40}` with the worst scenario printed per bound; `ir_drop` absent from the skip list for fixtures with an op testbench; no `bsim4v5.out` created in the cwd by `cargo test`; `OpPoint.vgs_v/vds_v/vbs_v` populated on ota (REL-10's input). |
| M2 Independent judge | PERF-16, 18, 19 | FLOW-04, FLOW-12, FLOW-14 (nightly tools) | `target/xcheck/summary.json` for all local fixtures (KLayout DRC run with `feol=1 beol=1`, KLayout LVS, magic DRC); `pex_cal.json` with \|C_field − C_magic\|/C_magic ≤ 15 % for every signal net ≥ 1 fF; `partial_overlaps_extract_as_their_union` green; T3 = 0 on sky130 fixtures except dac4's MOM units; `a_bjt_reference_is_collector_first` green. |
| M3 Sensitivity & statistics | PERF-11, 12, 13, 14, 15 | EXT-17, RTE-21 (fields), FLOW-08, FLOW-10 | ota: a `SensTable` with `GroundC`, ≤ 64 `CouplingC`, `SeriesR` and `GateOffset` rows, each `linear` or listed; σ_f, β, Φ(β) per bound; EXT-17 receives `Sensitivities` with non-empty `d_c`, `d_r`, `d_vt`; over 5 seeds the β key's winner has min β ≥ the old key's (PERF-14 acceptance); refresh count, pre- and post-fill metrics printed. |
| M4 Proof | PERF-20, 21, 22, 23, 26, 29 | FLOW-06, CELL-01, CELL-19 (PERF-26 steps 1–3 only; steps 4–5 may land in M3) | `results.json` over seeds 1–5 with perf, β and per-stage time; hand-comparison table for ≥ 3 TinyTapeout circuits; ota winner reports σ_os, μ_os, Ŷ and `agrees` (T10); an `EnvRow` for every matched set on the local fixtures; post-layout FET cards carry `as/ad/ps/pd` once CELL-19 has landed; ota winner prints `wc_value`, `wc_pass`, `lin_err` for every bound with known σ_f, using ≤ 2·n_specs extra sims (PERF-29). |
| M5 Advantage | PERF-24, 27, 28 | MAT-21, RTE-22 | T11 holds on ota and rc_filter over 5 seeds; MAT-21 ledger rows show MC σ for ota's pair and mirror; `width_candidates` lists 12 rows incl. uniform 1× and 2×; every T1–T9 loss in the hand table has an owner item in another plan. |

---

## 5. Open questions

1. **Partial LVS verdict.** Is an unrecognised device a hard violation (proposed default: yes, `lvs-coverage/…`,
   constant across epochs, blocks certification and every "beats hand layout" claim) or a waivable warning?
2. **Bench spec tolerance.** Specs derived from schematic values need a declared degradation allowance. Default
   `spec_tol = 0.1` per metric, stored in each `tb.json`, reported next to every verdict; the hand comparison uses
   ratios, not this tolerance.
3. **Hand-layout references.** Which TinyTapeout repositories contain both the block netlist and a separable hand-drawn
   cell, and under which licence? Default: the five of PERF-21, cloned at the pinned commits (never vendored), content
   checked by the first `handcmp.py` run.
4. **magic / netgen in nix.** The nixpkgs attribute names are not verified here. Default: add magic to the dev shell;
   use the KLayout foundry LVS instead of netgen until netgen is packaged.
5. **PEX acceptance band.** ±15 % against magic is a proposed default; the right band depends on how the ranking
   reacts to C error. Default: gate on 15 %, report the full distribution, tighten once PERF-16 data exist.
6. **GPurify extensions.** Per-net signal voltages in intent (for `vgs.*`/`vds.*`; REL-10 checks them in Philis
   meanwhile), signal-net EM (REL-03 in Philis), MOM capacitor recognition (one polygon per terminal today,
   reference.rs:13–14). MOS region terminals need no extension (PERF-18 "How GPurify binds terminals"); resistor W/L
   in LVS is decided by CELL-01's `extracts_params` probe (GPurify measures W/L for MOS markers only,
   GP/crates/check/src/topology/device.rs:295–316; every kind gets only `Area`, :318–323). Default: request upstream; until then each stays in `coverage` as
   "not checked", never as passed.
7. **Resistor body sheet resistance.** Resolved outside this plan: FLOW-05 step 3 adds `pex rbody_po … sheet 48.2ohm`
   sourced to the `res_generic_po` model card; the high/xhigh-po bodies stay without a row (plan-08 open question 3),
   and `device_sheet_ohm` returns `None` for them.
8. **σ_f scope.** V_T-only σ_f underestimates current-matched specs. Default: label it "V_T only" until MAT adds A_β
   deck keys (MM-28; FLOW-06 step 6 reserves `abeta_n_pct_um`/`abeta_p_pct_um`) and the β term enters PERF-13's
   formula as σ_i,β with its own sensitivity rows (gain-factor perturbation via `m`, LAMP-03).
9. **Does an ALRC failure block `certified()`?** Hastings states rules 16/17/21 as requirements for moderate and
   exceptional matching. Default: reported only (PERF-23) until the local fixtures show zero false findings over one
   milestone; then `certified()` requires no ALRC fail on `Moderate`/`Exceptional` sets.

---

## Cut or deferred items

| Former content | Where it lives now | Reason / hand-off |
|---|---|---|
| PERF-05 `pex_row` kept-operand rule, `sheet_ohm("rpoly")`, `ring_cut_ohm` | FLOW-05 steps 3–4 (plan-08) | Same code, same audit entry (AV-16); FLOW owns `pdk.rs` deck readers and the `pex rbody_po` row. **Defect to hand to FLOW-05's implementer:** its rule "one candidate → it; several → `None`" makes `poly` ambiguous, because both `poly_c = poly not poly_rs` (GP/pdks/sky130.deck:106) and `licon_po = licon and poly_c` (sky130.deck:114) keep area of `poly` under `reaches` (pdk.rs:758–770). `sheet_ohm("poly")` would become `None`, which silently disables the p2p finger limit (cellgen.rs:638) and makes `two_ended_gate_pays` always false (kernel/cells/src/mosfet.rs:61–66). Fix: among the candidates prefer declared conductors (`deck.connectivity.conductors`, sky130.deck:506) before declaring ambiguity; add the test `pex_f32(poly, "sheet_res_ohm_sq") == Some(48.2)`. Separately, under FLOW-05 `sheet_ohm("licon") == None` (its own test), so mosfet.rs:62 must read the gate cut explicitly (`pex_f32_named("licon_po", …)` = 152 Ω, sky130.deck:587) or two-ended gates are never chosen. |
| PERF-06 step 3 NaN-safe `key_lt` and test `a_nan_key_never_wins` | FLOW-02 step 5, test `nan_loses` | Duplicate (AF-29). |
| PERF-09 step 5 "annotate once for classes" | FLOW-09 step 2 | Duplicate (AF-22). |
| PERF-02 step 4 sidecar `cell.lvs_params` and test `a_wrong_resistor_width_is_a_parameter_mismatch` | CELL-01 step 7 (`Pdk::extracts_params(kind)`) | Duplicate mechanism for AV-19; two switches for one fact would disagree. GPurify measures W/L for MOS markers only (device.rs:295–316), so the probe will answer "no" for resistors on sky130. |
| PERF-08 FET card `m=`/`W_total` and tests `the_card_carries_m_and_total_width`, `simulated_channel_width_equals_drawn` | FLOW-01 step 3 and its test `size_tests::drawn_width_equals_simulated_width` | Duplicate (AF-01). |
| PERF-12 (previous draft) `PerformanceBudget` v2 struct (`stack`, `series`, conservative sign) and `perf::budget_rows(cfg, table, headroom_stat, stack)` | EXT-25 (row construction, two rows per bound, β·σ_f reserve, `credit_helpful = false`), RTE-21 step 3 (stack ground C, R measurement, `conservative`) | Three plans defined three different structs. PERF keeps the data export and the two terms neither defines (coupling, `limit`) as change requests (PERF-12 step 3, PERF-06 step 2). |
| PERF-13 per-device allowance `AnnotationConfig::device_allowance_mv` and `MetadataReport::sizing_infeasible` | EXT-21 `allocate` (per matched set, equal consumption split, `spec_infeasible_at_schematic`), EXT-25 `no_layout_margin` | Duplicate (GRAEB-38, LAMP-49); PERF supplies σ_f and `d_vt` through PERF-12. |
| PERF-15 step 2 post-fill signoff | FLOW-10 step 4 | Duplicate (AF-13); PERF keeps the post-fill re-simulation. |
| PERF-20 step 4 per-stage timers `ms_place/ms_route/ms_signoff/ms_perf` | FLOW-09 step 4 `stage_ms: [u64; 9]` | Duplicate (AT-12); PERF keeps `sims`, `sim_failures`, `refreshes`, `fill_regressed`. |
| PERF-20 step 7 "`preprocess_spice` appends `l=` only to MOS cards" | FLOW-07 step 11 (preprocessing reduced to model mapping) | Duplicate (AV-23). |
| PERF-25 limiting-spec feedback into net weights | Deferred | PLC-17 (per-epoch extracted-C feedback, BAL2-19 damping, 4× cap) and RTE-21 step 5 (budget repair, same damping and cap) already adjust these weights; a third controller on the same vector risks the oscillation BAL2-19 warns about (balasa_graeb_survey.txt L9401–9405). Re-enter only if PERF-24's A/B shows the limiting spec's β unimproved with PLC-17 on, and then as an input to PLC-17's update (one controller), not a separate loop. |
| NOTES-12 multi-fidelity promotion with proven bounds (skip simulation when a cheap bound is decisive) | Deferred | Needs per-fidelity model-error data that do not exist yet; PERF-15 records the prediction error of the linear model per refresh. Re-enter when PERF-20's `sims` column shows simulation dominating `stage_ms` and PERF-15's error log bounds the cheap tier. |
| NOTES-14 correlated-parasitic reserve σ²_P = sᵀΣ_p s | Deferred | Σ_p would come from seed-to-seed extraction spread (NOTES-14 recipe), which PERF-20's 5-seed results only begin to provide; subtracting an unmeasured reserve from headroom is guessing. Re-enter with a measured Σ_p as a term beside EXT-25's β·σ_f reserve. |
| GRAEB-10 bottleneck (smallest-β) spec drives weights | With PERF-25 (deferred) | Same controller question as PERF-25; the limiting spec is already visible in PERF-13's per-bound β and PERF-24's attribution. |
| AV-28 second half: `origin()` attributes li/mcon findings by layer, not by shape provenance (signoff_fixtures.rs:57–69) | Deferred | Needs per-shape provenance through `sol.geometry()`; the attribution is a diagnostic message only (no verdict depends on it). The first half (named expected ERC rows) is PERF-02 step 6. |

---

## Verification log

Adversarial fact-check, 2026-09-28, against the working tree, GP/ at 8df8c09 (Cargo.lock:678), the study documents
in this directory, the reftext files, and Lampaert PDF pages 120–123 (searching for Table 4.1).

**References checked: 443 citation tokens** (grep count over the pre-check text: 206 code `file:line` tokens, 70
reftext line ranges, 167 study-document ID mentions covering every distinct ID, all of which exist in their
documents), plus the named types, functions, fields, constants and test names the plan claims exist (all found)
and every proposed new name (no collisions: `Signoff`, `Coverage`, `signoff_checked`, `Scenario`, `BoundResult`,
`Param`, `SensRow`, `SensTable`, `StepPolicy`, `run_jobs`, `robust.rs`, `matched.rs`, `McConfig`, `McResult`,
`ParetoPoint`, `ExtractOptions`, `LvsParams`, `RefKind::Inductor`, `device_allowance_mv`). Numeric checks
recomputed: PERF-06 row weights (+0.2/−0.2, 0.1), PERF-07 `c_tier` 46 vs 50, PERF-13 σ_f = √2 and β = 11/4.4 = 2.5,
Table 12 Φ values within 1e-3, PERF-22 n = 385 at Y = 0.5, PERF-23 ratio errors 0 / +1.2 % / −0.6 % / 0 %.

Corrections (before → after):

1. §0.2 row 4: "(AV-21, AV-22)" → adds audit-06 §5 Q2, where the local deck-override question is actually asked.
2. §1.1: "dac4 (15 unit caps removed silently)" → five capacitor cards, Σm = 16 units (dac4.spice:8–12); audit-06
   G.2's 15 omits the dummy C0.
3. §1.4 V9 / PERF-05: "`licon` … an arbitrary one of five rows" and "the last of five equal-rank rows (185–585
   Ω/cut)" → the highest-id (last-declared) row, `licon_po` at 152 Ω; range 152–585 Ω/cut (sky130.deck:583–587;
   declaration order per GP/crates/ingest/src/deck/mod.rs:354–355).
4. §1.4 V11 and PERF-16 Why: "PEX is fringe-free parallel plate" → coupling is fringe-free parallel plate
   (analytical.rs:84); ground C includes a perimeter fringe term (analytical.rs:55–67).
5. §2 intro and PERF-21 Why: "NOTES-57 'fair comparison'" → ref-00 §2.9 topic-validation "Fair comparisons, seeds,
   cache keys (L74–82)" (ref-00-prior-notes-handbook.md:331); NOTES-57 is the discriminating validation catalog and
   says nothing about fair comparison or "same budget".
6. T11 and PERF-24 Why: Table 4.1 cited at "lampaert.txt L5020–5082" → Table 4.1 is at L5006–5012 (values 3.7/6.9 mV,
   2.8/5.4 ns confirmed there); L5020–5082 is the §4.12.1 text and Table 4.2.
7. PERF-01 step 1: cap_array.rs:780 calls `signoff_with_caps`, not `signoff_with_intent` → stated as compiling
   through `signoff_with_caps` (verify lib.rs:71–78).
8. PERF-01 step 5: `library::signoff` caller list (3 entries) → all 9 call sites (cli, net_dump, three library
   integration tests, op_demo:56, bench:189, signoff_fixtures:82 and :191).
9. PERF-01 Risks: "`Severity` may have more than two variants" → it has exactly `Warning` and `Error`
   (GP/crates/check/src/report/violation.rs:11–16).
10. PERF-04: "read `window` whether a `Length` or a 2-D variant (inspect `ParamValue`)" → `density_cmp` stores
    `window_x`/`window_y` as two `ParamValue::Length` (kinds.rs:456–468, parse.rs:1402–1411); there is no 2-D
    variant.
11. PERF-05 rule: "a declared conductor that keeps area of it … largest value" → conductor-first, then any declared
    layer. As written the rule contradicted its own tests: `licon_*` rows are vias, so `licon` would get `None`,
    not 585; without the conductor preference `poly` would get `licon_po`'s 152, not 48.2 (sky130.deck:106, 114,
    506–514).
12. PERF-05 callers: "cellgen p2p at cellgen.rs:638, bench preprocess at fixtures.rs:551–552" → the only
    `sheet_ohm("rpoly")` caller is fixtures.rs:550 (through `Overlay::sheet_ohm`); cellgen.rs:638 and mosfet.rs:62
    query `poly` (unchanged at 48.2). Added: `ring_cut_ohm` (lib.rs:521–524) moves from 152 to 585 Ω.
13. PERF-09 Current: "`run` annotates twice" → once in `performance_rows` plus once per `solve`, with 1–2 solves per
    start and 3 starts by default (lib.rs:75, 174–182).
14. PERF-10 Why: "GRAEB-26 … re-check only finalists L6013–6040" → Graeb's text says update when monitored gradient
    signs flip; the finalist-only re-check is GRAEB-26's recipe.
15. PERF-11 Why: "LAMP-02/03 … perturbation with central differences" → Lampaert gives the one-sided perturbation
    (eq. 2.15); central differences are LAMP-03's recipe. Step policy: "GRAEB-23's '10 % of node C'" → labelled as
    GRAEB-23's recipe (Philis choice); Graeb states only the noise/gradient criterion (L2703–2704).
16. PERF-12: added that the per-net weight sum is `gp::net_weights` (backend/gp/src/lib.rs:137–149), which today
    adds `w.max(0.0)`, not `|w|`.
17. PERF-15 Why: "GRAEB-39 (… prediction error > 30 % of σ_f, L3316–3361)" → the 30 % and µ are GRAEB-39's recipe,
    not Graeb's text; "SURV-26 (linear surrogate fails far from the schematic)" → SURV-26 is about a reused ML
    model losing 3–22 % accuracy out of distribution. Change: "[policy, GRAEB-39's 30 %]" applied to headroom →
    NOTES-13's recipe is 30 % of headroom, GRAEB-39's is 30 % of σ_f.
18. PERF-16: `contained_rectangles_are_not_double_counted` placed in `backend/verify` → must live in
    frontend/library/src/geometry.rs (private module, lib.rs:9; verify cannot depend on library, reference.rs:1–2).
    Added `merge_rects`'s O(n²) ceiling (geometry.rs:26) and that it does not union partial overlaps
    (geometry.rs:33–36), unlike the union G.4(b) measured against.
19. PERF-17 Current: "IR budgets per supply net" → per current-carrying supply/ground net and per signal net with
    ≥ 10 % of the largest current (ir.rs:27–52). Added: GPurify's `check_ir_drop` only examines declared supplies
    (electrical.rs:176–201) and refuses a net listed twice in `limits` (intent.rs:234–239).
20. PERF-19 step 2: `--report=<f>.lvsdb` → `--report=<f>` (run_lvs.py:100 appends `.lvsdb`); added the `PDK_ROOT`
    path pitfall (run_lvs.py:116–123).
21. PERF-23: `matched_report(sol, …)` reads "the run's routing requirements" → `Solution` carries none
    (lib.rs:82–101); stated that `solve` must store the pairs or the report re-annotates (as bench.rs:269 does).
    "the test lib.rs:417–418 uses" → "the predicate the fill pass uses at lib.rs:417–418".
22. PERF-08 Tests: `perf_postlayout.rs` read as a new file → marked existing (tests at :49–50, :76–77).
23. PERF-18 Risks: added evidence that sky130 diode recognisers already bind `psub` (sky130.deck:560–563).

Confirmed without change (sample of the load-bearing ones): every §1.2–§1.5 row's file:line and behaviour claim
(P1–P14, S1–S7, V1–V8, V10, V12–V14, B1–B6); audit-06 ground truth (G.1–G.4) numbers; Graeb eq. 83, 114, 175,
177/179, 219/225/226, 232 (8,461), 300–302, Tables 12 and 14, §5.5.1 "1%–3%", "three-sigma design"; Lampaert
eq. 2.12, 4.22, §7 10–15 %; BAL2 eqs. 4.6–4.7; todaes22 FOM 0.92 vs 0.74, Gm degeneration L46–53; [PDS] "3-4 PNR";
Hastings §13.3 rules 16–18 at L42596–42624; cc_dac_constructive §II-A; MAGICAL OTA README (CMRR 97 dB, unity-gain
offset bench); volare foundry decks, `tt_mm`, run_lvs.py options; GPurify `quasistatic_nets`, `StageStatus::Refused`,
BEM solver, intent schema.

Unresolved:

- PERF-20 step 8: whether bgr_core's `#[ignore]` is needed — H09-49 reports `unit_order` correct for 1:8 on 3
  columns (bjt.rs:274–283); the flow's chosen column count was not established (marked inline).
- PERF-18: whether GPurify binds a region net (`psub`/`nwell`) as a MOS terminal; only diode rows do today (marked
  inline).
- §1.1 and §1.3 S5 runtime facts (build/test/bench times, `bsim4v5.out` creation) come from audit-06 G.1–G.3 and
  AF-27; no build or run was repeated here (task rule). The `bsim4v5.out` file is present
  (frontend/library/bsim4v5.out).
- PERF-08 step 5: that ngspice's `show q : ic,ib,ie` reports those columns for sky130 BJT subcircuit instances was
  not verified (the plan's own Risk covers it).

---

## Implementer review log

Review of 2026-09-28, read as the implementer, with no further research planned. Checked all 29 item headings
(PERF-01 to PERF-29; PERF-05 and PERF-25 are stubs that point to the cut table), the milestones and the cut table.
Spot-checked against the working tree: `budget_rows` (perf.rs:225–257), `LexKey`/`key_lt`/`lex_key`
(lib.rs:910–960), `RunStats` (lib.rs:104–124), `harvest`/`static SKIPPED`/`shortfall_nm`/`Intent`/`signoff_with_*`
(verify lib.rs:29, 58, 71, 83, 150, 177), `cellgen::reference` pins and `continue` (cellgen.rs:860–895, `fingers`
710–713), fixture netlists (dac4, bjt_mirror, bgr_core, ota), `Netlist` having no `ports` yet (kernel/core/src/netlist.rs:47,
so the FLOW-07 dependency of PERF-03 is real), sky130.json keys (`gate_cap_af_um2` :9, `n_well_depth` :68, `avt_*` :93–96),
the foundry DRC m1.2 0.14 µm (sky130A_mr.drc:439) and its `$feol`/`$beol` guards (:47, :53), and Graeb eqs. 171–179
(graeb_centering.txt L4180–4221). Recomputed the tests' numbers: PERF-06 weights, PERF-07 46 vs 50, PERF-13 √2 and
6.72e-3 V, PERF-22 n = 385, PERF-23 ratio errors, PERF-27 4.75 mV. All held.

Changes:

1. PERF-06 step 1 did not say what the weight of a row with positive headroom is. It now states today's
   `w_i = sign·d_i/headroom` (perf.rs:252), `limit = 1.0`, and that the `(Some(lo), _)` match and its `filter_map`
   become a `flat_map` over both bounds. The existing tests' +0.2/−0.2 follow from this.
2. PERF-02 acceptance: `PARTIAL(k)` for dac4 is now `PARTIAL(16)`, with the source: five `cap_generic_m1m2` MOM
   cards, Σm = 16, dac4.spice:8–12. This matches M0.
3. §4 Milestones: removed a stray three-column table header above the milestone text, which broke the table.
4. New item **PERF-29, worst-case verification simulation per bound** (GRAEB-17, Graeb eqs. 171–179). No plan
   cited GRAEB-17. Without it, PERF-13's linear β, which T5 and the hand comparison's β columns rely on, is never
   checked at the 3σ point it certifies. The item gives the signature, the formula with units, clustering by
   cosine > 0.99 [policy], ≤ 2·n_specs sims, and three tests with numeric assertions. It is added to M4 and its
   exit criterion, and T5 now requires `wc_pass`.
5. Cut/deferred table: added NOTES-12 (multi-fidelity promotion), NOTES-14 (correlated-parasitic reserve) and
   GRAEB-10 (bottleneck-spec weighting). The first two are in PERF scope but have no measured input yet, and each
   has a re-entry condition. GRAEB-10 goes with PERF-25's single-controller argument.

Kept as is after review: the order (all P0 fixes are in M0/M1; the dependencies of M0→M5 are acyclic; inside M3 the
order is PERF-11 → 13 → 12 → 14 → 15), the boundaries with FLOW/EXT/RTE/MAT/CELL/REL in §0.2–0.4, and every other
signature and test.

Stale entries in the "Verification log → Unresolved" list: the PERF-20 step 8 `#[ignore]` question and the PERF-18
region-terminal question are both already resolved inline (PERF-20 step 8 "No `#[ignore]`"; PERF-18 "How GPurify
binds terminals"). Two questions remain open: whether `show q : ic,ib,ie` reports columns for sky130 BJT subckt
instances (PERF-08 step 5; its first ngspice test answers it) and the SA/SB unit (PERF-26 step 5, pinned by
`scale_applies_as_the_deck_assumes`).
