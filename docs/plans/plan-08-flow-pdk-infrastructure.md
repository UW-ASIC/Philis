# FLOW — Orchestration, PDK deck, hierarchy, macroMaster/emit, CLI and infrastructure

Plan ID prefix: `FLOW-NN`. Code citations are `path:line` on the working tree of 2026-09-28 (uncommitted changes
included). Reference citations are `<source> §/eq, reftext <file>.txt L<a>–<b>` in the reftext directory
`/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/`,
with the PDF page where the text extraction dropped the number. Study-document IDs (`AF-`, `AV-`, `AA-`, `AR-`, `AC-`,
`AP-`, `AT-`, `H01-`, `H13-`, `H15-`, `NOTES-`, `SURV-`, `BAL1-`, `BAL2-`, `MM-`, `SUB-`, `EM-`, `LAMP-`) refer to the
documents in `docs/plans/`. `GP/` = `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/` (the rev `Cargo.lock:676–678`
pins). `VOL/` = `~/.volare/sky130A/libs.tech/` (installed foundry PDK, volare fa87f8f4).

Numbers tagged **[policy]** are Philis choices with a stated rationale, not source values. Numbers tagged **[measure]**
must be measured on the bench before they gate anything. Every other number is quoted from code, a deck, an installed
PDK file, or a reference, with its citation.

---

## 0. Scope & boundaries

### 0.1 Owned here

1. **The epoch loop** in `frontend/library/src/lib.rs`: cold/warm schedule, PathFinder history lifecycle, variant
   escalation, stop reasons, the dual (price) step, the |V| and Θ tiers of the epoch key and their NaN safety.
2. **Stage contracts**: what each stage's `Report` means to the key (`kernel/core/src/report.rs`, the batch rows of
   `backend/gp/src/mechanics.rs` and `backend/gr/src/lib.rs`).
3. **Input**: the SPICE front end (`frontend/library/src/parse.rs`), the one device-size convention (`W_total`, `nf`,
   `m`) and the netlist data model (`kernel/core/src/netlist.rs`: ports, instance tree, source cards).
4. **Hierarchy**: sub-circuits solved bottom-up and placed/routed as block cells in the parent.
5. **PDK deck and sidecar**: where decks live and how they load (`backend/verify/src/pdk.rs`, `build.rs`), the sidecar
   key registry (every key, whoever owns its value), the deck-data fixes FLOW itself needs (latch-up, antenna
   thickness, resistor-body `pex` rows, `density_cmp` reader, `pex_row`/cut resistance). Key *values* for tiers,
   EM derating, substrate, mismatch coefficients and device recipes are owned by MAT, REL and CELL (§0.2).
6. **Fill** as part of the certified result (`frontend/library/src/fill.rs`, `lib.rs:410–420`).
7. **Deliverables**: CLI (`frontend/cli`), GDS writer (`frontend/library/src/gds.rs`), `emit`/`to_rust`
   (`frontend/library/src/emit.rs`) and the macroMaster authoring surface (`kernel/macroMaster/src/lib.rs`).
8. **Runtime and determinism**: pure-work hoisting, per-stage timers, seed derivation.
9. **Test infrastructure and CI**, and triage/closure of every `docs/CRATES.md` open issue not owned elsewhere.

### 0.2 Consumed from other plans

| Needed | From | Used by |
|---|---|---|
| `dp::place` accepting a start layout and a schedule (`Schedule::cold()`/`warm()`), or the equivalent for its new representation (AP-01, AP-09) | PLC-10 (step 0 lands the flat-path `Schedule` in PLC's M0) | FLOW-08 |
| User constraint file: `annotator::sidecar::parse(json: &str, nl: &Netlist) -> Result<(AnnotationConfig, Vec<Diagnostic>), String>` (EXT-26, plan-01) and net roles incl. node `0`, `vee`, `VPB/VNB` (AA-11) | EXT | FLOW-12, FLOW-07 tests |
| `MatchClass` (`kernel/analog/src/matching/class.rs`), `Process::tier(name, idx: usize) -> Option<i32>` and the tier arrays `lod_moat_ext_nm`, `wpe_clearance_nm`, `dummy_reach_nm`, `gate_ext_extra_nm` (MAT-07, plan-02) | MAT | FLOW-06 (registry rows only) |
| EM derating keys `em_ref_temp_c`, `em_activation_ev`, `em_current_exponent` and the `Pdk::em_limit` fallback (REL-05); substrate keys `substrate_kind`, `epi_thickness_nm` and deletion of `p_epi_thickness` (REL-09) | REL | FLOW-06 (registry rows only) |
| Mismatch coefficient keys `abeta_pct_um` (MAT-09), per-recipe `ka_pct_um`, `tc_ppm_per_k`, `sd_pct_per_mm`, cell `cap_ka_pct_um`, `bjt_ka_pct_um`, `vbe_tc_uv_per_k` (MAT-10) | MAT | FLOW-06 (registry rows only) |
| Recipe tables `capacitors` (CELL-08), `bjts` (CELL-09), `mosfets` (CELL-16), resistor value-model keys (CELL-06) | CELL | FLOW-06 step 2 (`recipe_layers` scans them), registry rows |
| `OpPoint` per-FET `vgs/vds/vbs` (PERF-09); EXT's further request for `vth`, `gmb`, `gds` and per-net DC voltage (plan-01 §0.2) goes to PERF-09, owner of `oppoint.rs` | PERF | not FLOW |
| Stable semantic constraint IDs for price keys (NOTES-04) | EXT | FLOW-03 (keys stay `(kind, ordinal)` until then) |
| Pre-search conflict detection (AA-13, BAL2-48, NOTES-06) | EXT | FLOW-03 (saturation report is the loop-side symptom only) |
| Identical-instance (SameTemplate) symmetry between block cells (AA-08, H15-07) | EXT | FLOW-11 |
| `verify::Signoff { report, warnings, coverage, caps }` (AV-10, AV-01) | PERF | FLOW-02 (interim: `warn/` prefix, FLOW-02 step 4) |
| Spec tier, C tier, Pareto report, post-fill re-simulation, limiting-spec net-weight feedback | PERF | FLOW-08 (plumbing only), FLOW-10 |
| MOS bulk / wells as nets / BJT recognisers as deck content (AV-05, AV-01) | PERF (edits the vendored decks of FLOW-04) | FLOW-05 boundary |
| In-flow latch-up tap check and tap rows (AC-07, H12-43), antenna model agreeing with signoff (AV-15) | REL, CELL | FLOW-05 boundary |
| Consumers of tiers / markers / capacitor recipes (AC-01, AC-13, H13-47, H01-16) | CELL, MAT | FLOW-06 |
| Routing stack returning `Result` and a per-run metal range (AT-01, AF-35) | RTE | FLOW-11 |

### 0.3 Provided to other plans

| Provided | To |
|---|---|
| `pnr_core::MosSize` + `Device::mos_size()` / `Device::gate_area_um2()` (FLOW-01) | EXT, CELL, MAT, PERF |
| `Netlist.ports`, `Netlist.insts`, `Netlist.device_inst`, `Netlist.sources`; R/C/L values `r_mohm`/`c_af`/`ind_ph` (FLOW-07) | EXT (roles from ports/sources, SameTemplate), PERF (bench without preprocessing, V/I cards), CELL (resistor/capacitor values) |
| `sidecar::KEYS` registry rows for every key another plan adds, `recipe_layers` over every recipe table (FLOW-04, FLOW-06) | MAT, CELL, REL, PLC |
| `Pdk::provenance()`, `Pdk::device_sheet_ohm()`, `Process::cut_ohm(cut, onto)`, `Pdk::pex_f32_named()`, `Pdk::density_rules()` for `density` and `density_cmp` (FLOW-04/05) | CELL, REL, PERF |
| Vendored, embedded decks with `# PHILIS:` edit markers (FLOW-04) | PERF (deck content edits), REL |
| `LexKey` contract: |V| per rule, Θ from one source (FLOW-02) | all plans' acceptance |
| Loop-owned per-epoch `Weights { place, route }` (FLOW-08) | PERF (net-weight feedback) |
| Labelled GDS, CLI output directory (FLOW-12) | PERF (foundry cross-check needs labels), users |
| `Hierarchy::BottomUp` block cells (FLOW-11) | PLC (block cells as placeable units), EXT |

### 0.4 Explicitly not here

Placement internals and representation (PLC), router internals and the routing stack rewrite (RTE), rule models
(MAT/REL), generators (CELL), sensitivities / spec tier / Pareto / Monte Carlo / verify signoff semantics / bench
(PERF), recognition and net roles (EXT).

---

## 1. Audit digest — current state

### 1.1 Epoch loop, key and multipliers

| # | Fact | Where |
|---|---|---|
| L1 | Every epoch calls `gp::place` from a fresh random pile; the incumbent layout never seeds the next epoch. | lib.rs:352, 575; backend/gp/src/mechanics.rs:361–371 (AF-06, AP-09) |
| L2 | `gr::Negotiation` is created once per `solve` and threaded through every epoch; history is keyed by absolute 500 nm buckets, folded with `max`, never reset. PLAN.md:178 grounds convergence on PathFinder's history term (a repeatedly contested node becomes monotonically more expensive); that argument presumes the same nodes recur between iterations, which re-placement breaks (the fixed-graph premise is this plan's inference, not text in PLAN.md). | lib.rs:341–342, 618, 630; backend/gr/src/lib.rs:36–92 (AF-06, AT-10) |
| L3 | dr runs twice in an epoch when antenna diodes are inserted, accumulating history twice. | lib.rs:629–646 (AT-34) |
| L4 | The dual step `prices.settle` runs at the end of `gp::place` on gp's overlapping pile and again at the end of `dp::place` on the legal layout: two steps per epoch on two different layouts; the ρ "did not shrink" test compares gp's residual with dp's. | backend/gp/src/lib.rs:83–107, 325; backend/dp/src/lib.rs:364 (AF-07, AP-06) |
| L5 | λ is one-sided: `c = residual.max(0)`, so a price never relaxes; saturation at `LAMBDA_MAX = 64` reads as settled in `drift`. PLAN.md:124 requires "decrease pressure on those with slack". | backend/gp/src/lib.rs:44–49, 86–100 |
| L6 | Termination = feasible and `prices.drift() < 1e-3`; drift is 0 whenever the last dp layout meets every placement budget. | lib.rs:137–139, 377–383 |
| L7 | Escalation fires whenever the incumbent is not feasible-and-stationary, including feasible-but-not-stationary; the odometer advances the highest-pin-spread cell, not a blamed one; `outer_iters = 4` gives ≤ 3 successors. | lib.rs:386–393; frontend/library/src/cellgen.rs:401–416 (AF-14) |
| L8 | Θ = `pt + rt + budgets.theta()`: the stage reports restate the same placement/routing batch residuals that metadata sums, so annotator budgets count twice while `CommonNodes`/`Environment` (added only via `add_routing`) and dr's EM-underwidth/overuse count once. The doc comment calls this ranking-neutral. | lib.rs:949–954; frontend/library/src/metadata.rs:88–101, 206–208 (AF-12) |
| L9 | |V| adds one row per violated *batch* (gp/gr) to one row per signoff *finding*; a batch with 40 broken pairs weighs one DRC notch. | backend/gp/src/mechanics.rs:291–299; backend/gr/src/lib.rs:380–386; lib.rs:952 (AF-12) |
| L10 | Signoff warnings (`tie_high_low`, `density_cmp`) are hard violations in |V|. | backend/verify/src/lib.rs:150–163; GP/pdks/sky130.deck:471, 500–503 (AV-10) |
| L11 | `key_lt` compares f64/f32 with `<`: a NaN tier makes both directions false, so a NaN incumbent at equal |V| is never displaced. | lib.rs:929–939 (AF-29) |
| L12 | The comment "dp consults the live DRC oracle" is false; no oracle type exists. | lib.rs:336–339 (AF-05, AF-28) |
| L13 | No per-stage timing, no stop reason, no wall-time budget in `RunStats`. | lib.rs:104–124 |

### 1.2 Input and device size

| # | Fact | Where |
|---|---|---|
| I1 | Layout and LVS draw `max(nf, m)` fingers of the written `w`; the op/perf card passes `W={w} L={l} nf={nf}` (BSIM4 reads W as the instance total [UNVERIFIED in ref/: no BSIM4 manual among the sources; standard BSIM4 NF semantics, as AF-01 states]) and drops `m`. ota.spice XM1 (`W=10u nf=2`) is drawn with 20 µm of channel and simulated with 10 µm; XM5 (`W=40u m=4`) is drawn with 160 µm and simulated with 40 µm. | backend/annotator/src/constraints.rs:21–24; backend/annotator/src/lib.rs:155–161; frontend/library/src/cellgen.rs:565, 712, 879–884; frontend/library/src/oppoint.rs:270–286; frontend/library/src/lib.rs:792; benchmarks/fixtures/ota.spice:3–7 (AF-01, AR-46) |
| I2 | The parser skips every `.`-directive (so `.subckt` ports are lost and several sub-circuits merge into one namespace), keeps only `w l nf nfin stack m multi`, reads `.param` names and expressions as 0, drops R/C/L values, rejects V/I/E/F/G/H/B/K cards, treats the title line as a card, and interns net names case-sensitively. | frontend/library/src/parse.rs:35–39, 61–62, 95–104, 17–31, 253 (AF-02, AF-17) |
| I3 | `X` calls are classified by model-name substrings: `reset_logic` → resistor, `window_cmp` → inductor, gf180 `ppolyf_u` and IHP `rsil`/`rppd` → NMOS; an unknown user sub-circuit becomes an NMOS. | parse.rs:157–161, 189–206; pdks/gf180mcu.json:86–89; pdks/ihp_sg13g2.json:81–97 (AF-10) |
| I4 | A 4-node BJT silently drops its substrate node. | parse.rs:88–92, 211–218 |
| I5 | `Netlist` is `{devices, nets}`: no ports, no hierarchy, no sources; `NetId`/`DeviceId` are `u16`. | kernel/core/src/netlist.rs:46–50; kernel/core/src/ids.rs:5–9 |
| I6 | The benchmark works around I2 outside the library (`preprocess_spice`: `.param`, backslash joins, bare R/C → model + W/L, `nfin`→W, `l=0.15u` appended to every `m`/`x` card that states no `l`); `library::run` and the CLI get none of it. | benchmarks/src/fixtures.rs:566–610, 596, 611–613 (AF-02, AV-23) |

### 1.3 Deck and sidecar

| # | Fact | Where |
|---|---|---|
| D1 | The deck is read at run time from GPurify's `pdks/`, whose absolute path is baked in at compile time from the cargo git cache; a moved binary, a cleaned cache or the flake package (which installs only `pdks/*.json`) cannot load any PDK. | backend/verify/src/pdk.rs:81–91; backend/verify/build.rs:7–23; flake.nix `postInstall` (AV-21) |
| D2 | The GPurify deck language rejects a duplicate rule id, so a Philis-side override cannot replace a deck rule; `connect conductors` is additive. | GP/crates/ingest/src/deck/parse.rs:1018, 864–866 |
| D3 | `parse_roles` keeps only integer/boolean `cell.*` scalars in `rules`; floats and arrays are reachable only through `cell_f32`/raw JSON; the tier arrays `wpe_clearance_nm [2000,3000,5000]` and `lod_moat_ext_nm [3000,5000]` are read by no code (the code reads `*_moderate` scalars). | backend/verify/src/pdk.rs:1225–1269, 515–517; pdks/sky130.json:59–63, 70–73; kernel/cells/src/mosfet.rs:293, 629; lib.rs:871–872 (AV-22, H13-42) |
| D4 | `REQUIRED_RULES` demands keys no generator reads (`bjt_base_frac_permille`, `bjt_collector_frac_permille`, `bjt_max_emitter_stripe`, `bjt_stripe_gap`, `tie_max_dist_nm`, `n_well_depth`, `p_well_depth`, `p_epi_thickness`, `retrograde_pwell`; grep finds each only in `REQUIRED_RULES`); process numbers such as `tie_max_dist_nm 3000`, `n_well_depth 2000`, `p_epi_thickness 3000`, `lod_moat_ext_moderate 3000` carry no `_source`. | backend/verify/src/pdk.rs:1039–1071; pdks/sky130.json:66–86 (CRATES #9, AV-22, AC-27) |
| D5 | sky130 has no latch-up rule; the installed foundry tech file states LU.2 / LU.2.1 / LU.3 "< 15.0 µm"; gf180 encodes `tap_distance` 20 µm (3.3 V) / 15 µm (5 V); IHP encodes `missing_tie … max_distance: 20um`. | GP/pdks/sky130.deck:669–670; VOL/magic/sky130A.tech:4163–4170; GP/pdks/gf180mcu.deck:303–306; GP/pdks/ihp_sg13g2.deck:595–596 (AV-08, CRATES #5) |
| D6 | `ar.met3.1` states `sidewall: 2000nm`; the same deck's `pex met3 thickness 845nm` and magic's `height allm3 … 0.845` disagree (2.37×). met1/met2 antenna rules state 350 nm against pex 360 nm (techlef 0.35 vs magic 0.36). | GP/pdks/sky130.deck:455, 457, 459, 590, 594; VOL/magic/sky130A.tech:4890 (AV-17) |
| D7 | `pex_row` answers through subtracted operands: `sheet_ohm("rpoly")` returns `poly_c`'s 48.2 Ω/□ (poly *not* poly_rs); for `licon` five derived cut rows qualify (`licon_nd/pd/nt/pt/po`, 152–585 Ω/cut, GP/pdks/sky130.deck:583–587) and `max_by_key` (pdk.rs:672) returns the last of equals by layer id — `licon_po` (152 Ω, a poly contact) if ids follow declaration order, not a tap cut. The deck has no `pex` row for any `rbody_*` layer. The kept-operand walk exists (`reaches`). | backend/verify/src/pdk.rs:661–674, 758–770; GP/pdks/sky130.deck:106, 203–209, 582–587 (AV-16, CRATES #6) |
| D8 | `density_rules` reads only kind `density`; sky130 states density as `density_cmp` (params `window_x/_y`, `min_density`, `max_density`), so fill and every other reader see no density rule on sky130. | backend/verify/src/pdk.rs:619–637; GP/pdks/sky130.deck:500–503; GP/crates/ingest/src/deck/kinds.rs:457–468 (AV-18) |
| D9 | EM derating reads `(reference_temperature, activation_energy_ev, current_exponent)` only from the EM rule; no deck carries them, so derating is silently off. | backend/verify/src/pdk.rs:525–547; GP/pdks/sky130.deck:491–495; ihp_sg13g2.deck:789–790 (H15-38, EM-42) |
| D10 | `cap_density_ff_um2 = 2.0` equals the sky130 MIM `camimc = 2.00e-15 F/µm²` (typical; 1.778/2.231 in the low/high corners; `cpmimc = 0.19e-15 F/µm`), but the generator draws met1/met2 MOM, extracted at 0.105 fF/µm². | pdks/sky130.json:8; VOL/ngspice/r+c/res_typical__cap_typical__lin.spice:7–8, res_low__cap_low__lin.spice:7, res_high__cap_high__lin.spice:7; audit-06 G.4(a) (AC-01, AV-32) |
| D11 | Which FET flavour needs which marker is already in the deck's recogniser expressions (`nfet_01v8_lvt = (ngate and lvtn) not hvi`, `nfet_g5v0d10v5 = ngate and hvi`, `pfet_01v8_hvt = (pgate and hvtp) not hvi`); Philis draws no `hvi`/`lvtn`/`hvtp`. | GP/pdks/sky130.deck:192–202 (H01-16) |

### 1.4 Outputs, CLI, emit, macroMaster

| # | Fact | Where |
|---|---|---|
| O1 | The CLI hard-wires `Config::default()`, writes no GDS or report, and cannot set op, performance, seed, starts, iterations or annotation. | frontend/cli/src/main.rs:23–63 (AF-11) |
| O2 | The GDS writer emits one flat `TOP` structure of BOUNDARY records: no TEXT labels, no pins; an unmapped layer silently becomes `(id, 0)`. | frontend/library/src/gds.rs:36–73 (AF-18, H01-30) |
| O3 | `emit` re-annotates and re-enumerates cells with default folds and the merged topology, then indexes the caller's `Layout` by its own cell order: a `run()` result from the `apart` topology or a gm-driven fold table mis-lifts or panics; `philis emit` does exactly this. | frontend/library/src/emit.rs:98–99, 190–194; lib.rs:174–182, 274; cli/src/main.rs:47 (AF-08) |
| O4 | `to_rust` exports every net as a port and uses net names as Rust identifiers unsanitised (`in`, `0`, `a<1>`, `vout-` do not compile); emit ignores `m` and defaults `w=420`, `l=150`. | emit.rs:282, 382–389, 147–149 (AF-21, AF-33) |
| O5 | macroMaster `Instance` has no orientation; `place_mirrored` reflects shapes and pins but not `units`/`dummies`, so an odd-finger mirrored partner conducts the other way (Hastings Table 13.2 rule 5); duplicate instance names merge nets; `connect` typos mint singleton nets; align offsets are not grid-checked. | kernel/macroMaster/src/lib.rs:152–165, 184–192, 342–349, 353–362, 435–441, 450–451 (AF-19) |
| O6 | The undrawable-cell message names `netlist.devices[cell index]`, wrong after a group collapse; injected macros' bare `G/D/S/B` pins get no series R and no common-node pins. | lib.rs:1295–1297, 726, 771 (AF-31, AF-32) |

### 1.5 Runtime, determinism, tests

| # | Fact | Where |
|---|---|---|
| R1 | Each start re-annotates, re-draws every variant and re-prices every alternative (one DRC+ERC per alternative), up to 2·`starts` times per run on identical inputs; `annotation()` leaks the `Stack` once per call. | lib.rs:174–189, 262–275, 340, 511–513; cellgen.rs:327–375 (AF-22) |
| R2 | gp and dp in one epoch receive the same seed, so dp's RNG stream starts identical to gp's. | lib.rs:575, 596 (AP-19) |
| R3 | Ground truth: `cargo test --release --workspace` 331 passed, 1 failed (dac4 `erc/ar.met2.1` at `feedback_iters = 1`), 2 ignored; `bench local` 539.5 s. | audit-06 G.2–G.3 |
| R4 | There is no CI (no `.github/`); cross-deck generator tests are `#[cfg(not(debug_assertions))]`; ngspice tests pass when the tool is missing (`eprintln!` + `return`, frontend/library/tests/perf_postlayout.rs:52–53, 79–80, output captured by the harness); `flow_smoke` accepts `Err`; `hier_elaborate` prints LVS without asserting; `tmp_deck_drc.rs` is an `#[ignore]`d diagnostic. | repository root listing; kernel/cells/tests/cell_selfcheck.rs:283–345; frontend/library/tests/flow_smoke.rs:24–30; tests/hier_elaborate.rs:206–220; tests/tmp_deck_drc.rs (AF-26, AC-19) |

### 1.6 `docs/CRATES.md` open issues (docs/CRATES.md:95–117) — triage

| # | Issue | Status 2026-09-28 | Owner |
|---|---|---|---|
| 1 | bjt_mirror magic violations; tap split | Open; sky130 deck already derives `ntap`/`ptap` (GP/pdks/sky130.deck:104–105) but has no BJT recogniser (:663–665) | PERF (recogniser content in the vendored deck), CELL (BJT cell, AC-08) |
| 2 | Routing EM not wired | Stale — wired (lib.rs:304–311) | FLOW-15 (doc) |
| 3 | Placement net weights | Stale — done (backend/gp/src/lib.rs:137) | FLOW-15 (doc) |
| 4 | Placer grid 10 nm | Open (AP-05: origins off-lattice by 5 nm) | PLC |
| 5 | LU.2 latch-up | Open | FLOW-05 (deck rule + key), CELL (tap rows, AC-07), REL (in-flow check) |
| 6 | xhrpoly contacts, `pex.rbody`, multi-segment | Open | FLOW-05 (`pex rbody_po`, `device_sheet_ohm`), CELL (slot contacts, segments, AC-02/AC-15) |
| 7 | Diode anode tap ≥ 410 nm | Open | CELL (AC-18) |
| 8 | Centroid diff pairs discarded | Stale — fixed (cellgen.rs:1466–1501) | FLOW-15 (doc) |
| 9a | `REQUIRED_RULES` obsolete `bjt_*` | Open | FLOW-04 |
| 9b | `asymmetric_enclosure` stricter than magic | Open | PERF (foundry cross-check decides, AV-02) |
| 9c | LVS device-count/parametric skip list | Open | PERF (AV-20) |
| 9d | `ir_drop` needs intent | Stale — done (elaborate.rs:297–319) | FLOW-15 (doc) |
| 10a | dp always 220 iterations | Open | PLC (AP-08) |
| 10b | mosfet.rs link to `VariantSpace::lock` | Stale — fixed | FLOW-15 (doc) |
| 10c | Unused `Unitization`/`GuardRingRequirement` fields | Open (AR-28) | FLOW-15 (delete what no plan wires by M4) |
| 10d | gr/dr not rustfmt-clean | Open | FLOW-14 |
| 10e | `xcheck_lvs.py` MOS-only | Open | PERF |

---

## 2. Target — what "better than hand layout" means here, measurably

Hand layout's advantage in this area is that the designer's intent, the drawn circuit and the simulated circuit are the
same thing, and the final GDS is what was checked. The tool beats it when every number it reports is about the shipped
geometry, every input feature is honoured or refused loudly, and the search spends its budget where it matters.

| ID | Metric | Definition | Target |
|---|---|---|---|
| T1 | Size fidelity | Devices where Σ drawn channel width (from `Macro.units`) ≠ simulated `W_total·m` (±`fingers·grid` nm) | 0 on all 10 local fixtures |
| T2 | Input fidelity | Local fixtures and competition netlists (ALIGN, MAGICAL) that `library::parse` reads without `preprocess_spice` rewriting anything but model names | 10/10 local; every ALIGN/MAGICAL failure is an `unknown model` error, none a parser-feature error |
| T3 | Deck honesty | sidecar process numbers without `_source`; required keys unread by any code; decks not embedded | 0 / 0 / 0 |
| T4 | sky130 rule coverage | LU.2/LU.2.1/LU.3 present; every antenna sidewall thickness within 5 % [policy] of the deck's `pex` thickness | LU rules present; today only met3 fails (2000 vs 845 nm) → none fails (met1/met2 differ by 2.8 %: techlef vs magic sources) |
| T5 | Key integrity | |V| counts violated rules and signoff errors (not warnings); Θ counts each budget residual once | unit tests FLOW-02 |
| T6 | Multiplier integrity | dual steps per epoch; prices on slack budgets fall; saturated batches listed | exactly 1; strictly decreasing while slack; listed in `MetadataReport.binding` |
| T7 | Certificate freshness | `library::signoff(sol)` hard count vs the winner's key |V| (signoff part) after fill | equal on every fixture |
| T8 | Determinism | Same seed, same inputs → byte-identical GDS and identical report text (timings excluded) | 3 runs × 10 fixtures |
| T9 | Redundant work | `cellgen::price` calls per run | Σ alternatives × topologies (≤ 2), not × starts |
| T10 | Search quality from the loop | Median final `LexKey` over seeds 1–5 with warm epochs vs the cold-only baseline at equal epochs | no worse on ≥ 8/10 fixtures [policy]; epochs-to-best reported [measure] |
| T11 | Deliverables | CLI writes labelled GDS + report; `run → emit → elaborate_ir → signoff` | GDS TEXT per port net; LVS clean on chain4 and ota |
| T12 | Scale | Two-instance OTA fixture in `BottomUp` mode | block solved once; LVS 0, DRC 0 |
| T13 | CI | `cargo test --workspace` on every push; release generator tests; nightly tool-dependent tests fail loudly when a tool is missing | green |

---

## 3. Work items

### FLOW-01 One device-size convention (SPICE `W_total`, `nf`, `m`) across parse, cells, LVS and simulation

Status: done in M0 (`f90c5e7`). As built: `parse::ParseOptions { size }` is crate-private (not re-exported), `Config.size_convention` feeds `run` only and `parse()` is always `Spice`. Measured consequence: the OTA `CommonCentroid` rows went 3/3 → 0/3 (XM1/XM2 W=10u nf=2 now fold to two 5 µm fingers each, drawn AABB; `cellgen::folds` ignores CommonCentroid), carried to M1 under FLOW-16.

- Priority: **P0**. Effort: **M**. Depends on: none.
- Why: AF-01 (critical), AR-46, AA-21 (size semantics part), PERF §0.2 row 1. The glossary of the prior handbook records
  that "W/nf/m semantics are PDK-specific" (ref-00 §2.4, notes glossary L13–29), so the parser must take the
  convention as input and normalise once. Hastings §4.2.3 defines drawn W as the poly perimeter touching the source per
  device (hastings.txt L9998–10002, PDF p.179; H01-13), i.e. one physical total per device that layout, LVS and
  simulation must agree on.
- Current: see §1.2 I1. Layout, LVS reference and annotator count `max(nf, m)` fingers of width `w`; the simulator
  receives `W=w nf=nf` without `m`.
- Change:
  1. `kernel/core/src/netlist.rs` — add:
     ```rust
     /// A MOS instance's size in SPICE/BSIM4 semantics: `w_total_nm` is one
     /// instance's gate width, split over `nf` fingers; `m` instances in parallel.
     #[derive(Clone, Copy, Debug, PartialEq, Eq)]
     pub struct MosSize { pub w_total_nm: i64, pub l_nm: i64, pub nf: u32, pub m: u32 }
     impl MosSize {
         /// Per-finger width, nm (floor; the < nf nm residue is inside LVS's 2 % tolerance).
         pub fn w_finger_nm(self) -> i64 { self.w_total_nm / i64::from(self.nf) }
         /// Drawn fingers = nf·m.
         pub fn fingers(self) -> u32 { self.nf.saturating_mul(self.m) }
         /// W_total·L·m, µm².
         pub fn gate_area_um2(self) -> f64 { (self.w_total_nm * self.l_nm) as f64 * 1e-6 * f64::from(self.m) }
     }
     impl Device {
         /// `None` for a non-MOS or when `w`/`l` is missing or ≤ 0 (never a silent 0 µm²).
         pub fn mos_size(&self) -> Option<MosSize>;   // reads params "w","l","nf"(≥1),"m"(≥1); "multi" folds into m
         pub fn gate_area_um2(&self) -> f64 { self.mos_size().map_or(0.0, MosSize::gate_area_um2) }
     }
     ```
     Canonical storage stays `params["w"]` = **W_total** in nm (so the simulator card is the netlist as written).
  2. `frontend/library/src/parse.rs` — add `pub enum SizeConvention { Spice, PerFinger }` and
     `pub struct ParseOptions { pub size: SizeConvention, … }` (FLOW-07 extends it). `spice(text)` keeps
     `SizeConvention::Spice`. With `PerFinger`, a MOS card's `w` is multiplied by `nf` before storing (W_total = w·nf).
     `Config.size_convention: SizeConvention` (default `Spice`) feeds `library::run`.
  3. Consumers (each a 1–3 line change, all reading `Device::mos_size()`):
     - `backend/annotator/src/constraints.rs:21–24` `fingers(dev)` → `dev.mos_size().map_or(1, |s| s.fingers().min(u32::from(u16::MAX)) as u16)`;
       `class_of` (constraints.rs:30–33) keys on `w_finger_nm` instead of `w`.
     - `backend/annotator/src/lib.rs:155–161` `gate_um2` → `dev.gate_area_um2() as f32`.
     - `frontend/library/src/cellgen.rs:565–567` `dev_nf = fingers()`, `unit_w = w_finger_nm()`; `cellgen.rs:712, 753`
       use `fingers()`/`m`; `reference` (cellgen.rs:879–884) emits `fingers()·k` cards with `w = w_finger_nm` (or the
       fold width `fw`), `l = l_nm`.
     - `frontend/library/src/cellgen.rs:703–706` private `fn fingers(d)` (a second copy of `max(nf, m)`) → deleted;
       its callers use `d.mos_size().map_or(1, |s| s.fingers())`.
     - `frontend/library/src/cellgen.rs:615–699` `folds`: the width it splits is today `nm(d, "w")` (cellgen.rs:624,
       640, 643, 650), read as a per-finger width; it becomes `s.w_finger_nm()` (so `fw = snap(w_finger / k)` and the gate-R
       floor `n = √(5·gm·R□·W_f/(3L))` at cellgen.rs:669 uses the per-finger width, µS·Ω·nm/nm → dimensionless).
       Class equality (cellgen.rs:650) compares `w_finger_nm`.
     - `frontend/library/src/lib.rs:789–792` `common_nodes` gate area → `dev.gate_area_um2() as f32` (also closes
       AA-32's third copy).
     - `frontend/library/src/oppoint.rs:270–286` card → `W={w_total_um} L={l_um} nf={nf} m={m}` (PERF-owned file;
       this one line belongs to the convention).
     - `frontend/library/src/emit.rs:139–150` → `mos_size()`; a missing size is `EmitError::Unsupported`, not
       `w=420, l=150` (AF-33).
     - `kernel/macroMaster/src/lib.rs:585` `variants::Mos` device params → `("w", w·nf)`, `("nf", nf)`.
  4. `benchmarks/src/fixtures.rs` suites: ALIGN and MAGICAL runs pass `SizeConvention::PerFinger` only if
     OQ-1 confirms their convention (PERF owns the bench; this item supplies the switch).
- Tests:
  - `kernel/core/src/netlist.rs` `mos_size_follows_spice_semantics`: `w=10000 l=1000 nf=2` → `w_finger 5000`,
    `fingers 2`, area 10.0 µm²; with `m=4` → `fingers 8`, area 40.0 µm²; `w=0` → `None`.
  - `frontend/library/src/parse.rs` `per_finger_convention_stores_total_width`: `M1 d g s b nfet_01v8 W=1u L=0.15u nf=4`
    → `w = 4000` under `PerFinger`, `1000` under `Spice`.
  - `frontend/library/src/lib.rs` `size_tests::drawn_width_equals_simulated_width` (sky130, benchmarks/fixtures/ota.spice):
    for every MOS device, Σ over its units in variant 0 of `unit.weight / l_nm` (`Unit.weight` = W·L nm²,
    kernel/core/src/units.rs:23–25; the device's units are those whose `owner` equals its member index in
    `CellSpace.devices_of[cell]`) equals `w_total_nm·m` within `fingers·pdk.grid` nm; and the `oppoint::flat_circuit` line for `XM5` contains `W=40 L=2 nf=1 m=4`.
  - Existing `frontend/library/tests/ota_cross_pdk.rs` LVS-clean assertion (ota_cross_pdk.rs:335) stays green.
- Acceptance: T1 = 0 on all 10 fixtures; `bench local` LVS verdicts unchanged; bench re-baselined (XM1 fingers become
  5 µm, not 10 µm).
- Risks / notes: every fixture whose author meant per-finger W changes size; the three local fixtures that state `nf`
  (ota, ota_constrained, tt_ota) must be audited (OQ-1). `nfin` stays a separate key for FinFET decks (CELL, AC-09).

### FLOW-02 Epoch-key contract: |V| per rule, Θ from one source, warnings out of V, NaN-safe

Status: done in M0 (`65f5f8c`); amended by the review panel: `lex_key` leaves the `lvs-coverage/` rows out of |V| (plan-07 PERF-02 step 3, master §6.1).

- Priority: **P0**. Effort: **S**. Depends on: PERF `verify::Signoff.warnings` (optional; interim step 4).
- Why: AF-12, AF-29, AV-10, AF-28; PLAN.md:115 (termination over V=0, Θ=0), D17 in docs/API-WISH.md
  ("normalised residual"). Hastings §15.5 checklist items 1–3 separate findings that can never be waived (LVS
  topological errors) from ones reviewed and waived (dubious DRC diagnostics, LVS parametric errors) (hastings.txt
  L48856–48878, PDF p.823; H15-42).
- Current: §1.1 L8–L11.
- Change:
  1. `kernel/core/src/report.rs` — add the one marker for rows that restate a rule batch:
     ```rust
     impl Violation {
         /// Prefix of a stage row that restates a whole rule batch; the epoch key
         /// counts those per rule from `metadata`, not per row.
         pub const BATCH: &'static str = "batch:";
         #[must_use] pub fn is_batch_row(&self) -> bool { self.rule.starts_with(Self::BATCH) }
     }
     ```
     `backend/gp/src/mechanics.rs:297, 307` and `backend/gr/src/lib.rs:385, 393` format their rows as
     `format!("{}analog hard {bi} ({kind})", Violation::BATCH)` etc. Built-in rows keep their names: gp/dp
     "device overlap" (mechanics.rs:312); dr "open net", "pin access sacrificed", "drawn short", "em underwidth",
     "routing overuse", "em cuts net" (backend/dr/src/lib.rs:1841–1856, 883); gr "routing overflow" (gr/lib.rs:404).
  2. `frontend/library/src/metadata.rs` — add
     `pub fn hard_violated(&self) -> usize { Σ over placement ∪ routing rows with arm == Arm::Hard of (total − satisfied) }`
     and fix the `theta()` doc (metadata.rs:88–92): it is the only Θ source.
  3. `frontend/library/src/lib.rs:941–958` `lex_key` becomes:
     ```rust
     let own = |r: &Report| r.hard_violations.iter().filter(|v| !v.is_batch_row()).count();
     let errors = signoff.hard_violations.iter().filter(|v| !v.rule.starts_with("warn/")).count();
     let v = budgets.hard_violated() + own(place) + own(route) + errors;
     let theta = budgets.theta()
         + route.budget_violations.iter().filter(|x| !x.is_batch_row()).map(|x| x.margin as f64).sum::<f64>();
     (v, perf.map_or(0.0, |p| p.residual), theta, signoff.cost, footprint_nm2)
     ```
     (placement budget rows are all batch rows; dr's `em underwidth`/`routing overuse` stay, in milli-budgets like
     `theta()`.)
  4. Warnings, interim until PERF's `verify::Signoff` lands: `backend/verify/src/lib.rs:150–163` `harvest` prefixes a
     row whose `severity` is `Severity::Warning` (field of GPurify's `Violation`, GP/crates/check/src/report/violation.rs:11–24)
     with `warn/`. `RunStats.warnings: u32` counts them.
  5. `key_lt` (lib.rs:929–939): every f64/f32 tier passes through `nan_last = |x| if x.is_nan() { INFINITY } else { x }`
     before comparison.
  6. Delete the false oracle sentence at lib.rs:336–339 and fix the `LexKey` doc (lib.rs:910–915).
- Tests (`frontend/library/src/lib.rs` `start_tests`):
  - `v_counts_rules_not_batches`: metadata with one hard row `total 40, satisfied 0` and a place report whose only
    row is a batch row → `lex_key(..).0 == 40`.
  - `theta_counts_each_budget_once`: a place report restating a budget batch (margin 500) and metadata `theta() = 500`
    → Θ = 500, not 1000.
  - `warnings_are_not_violations`: signoff with rows `warn/erc/tie_high_low:li` and `drc/m1.1:met1` → V = 1.
  - `nan_loses`: `(0, NaN, 0, 1, 1)` vs `(0, 5.0, 0, 1, 1)` → the finite key wins; a NaN incumbent is displaced.
- Acceptance: T5; the fixtures with V = 0 today keep V = 0 (8 of 10: bgr_core and bjt_mirror each carry one
  `erc/supply_short:met1` error row, audit-06 G.3, which stays in V because it is not a deck warning).
- Risks / notes: changes epoch selection wherever a batch had >1 violated rule (intended). Batch row renaming touches
  report text only; no consumer parses those names (grep for `"analog hard batch"`/`"routing hard batch"`: emitters only).

### FLOW-03 One dual step per epoch; prices that relax on slack; saturation reported

Status: done in M0 (`de04d31`); `RunStats::dual_steps` equals `iterations` (T6). GAP-10 (prices keyed by ID) is done in M1 (`809c844`); before it, on m0, the key was `fn keys` → `(&'static str, u32)` (backend/gp/src/lib.rs:151) into `Prices.priced: BTreeMap` (:27-28).

- Priority: **P0**. Effort: **S**. Depends on: FLOW-02.
- Why: AF-07, AP-06, PL-9/PL-10 (audit-07 §2.14.1). PLAN.md:92 (λ ← λ − ρ·c on true residuals) and PLAN.md:124
  ("increase ρ for constraints whose residual is not shrinking, decrease pressure on those with slack").
- Current: §1.1 L4–L6.
- Change:
  1. Delete `prices.settle(reqs, &l)` at backend/gp/src/lib.rs:325 and backend/dp/src/lib.rs:364 (both keep
     `prices.bind(reqs)`, gp/lib.rs:196, dp/lib.rs:241). One line each in PLC-owned crates; the semantics are FLOW's.
  2. `Flow::epoch` (lib.rs:561–709) calls `prices.settle(placement, &layout)` once, after `dp::place`, on the layout
     the epoch is scored on.
  3. `backend/gp/src/lib.rs:83–102` `settle` becomes a projected dual step on a signed constraint value `g`
     (dimensionless, fraction of the batch's own budget). Existing fields of `Price` (gp/lib.rs:36–42: `lambda ≤ 0`,
     `rho`, `residual`) are kept; `residual` now stores the last `g`. `Prices` gains `steps: u32` and
     `saturated: Vec<&'static str>`.
     ```rust
     pub fn settle(&mut self, reqs: &Requirements<Layout>, l: &Layout) {
         let mut drift_sq = 0.0f64;
         self.saturated.clear();
         for (bi, key) in keys(reqs).into_iter().enumerate() {
             let b = &reqs.budget[bi];
             let r = b.residual(l).max(0.0) as f32;
             // Slack only where the batch can measure it: spent fraction (`worst_usage`,
             // rule.rs:164) else headroom-derived criticality (rule.rs:159, rule_criticality
             // rule.rs:281–289). A batch that reports neither has criticality 1 → g = 0 → price held.
             let g = if r > 0.0 { r } else {
                 b.worst_usage(l).map_or(-(1.0 - b.criticality(l)), |u| (u - 1.0).min(0.0))
             };
             let p = self.priced.entry(key).or_insert(Price { lambda: 0.0, rho: RHO_FLOOR, residual: f32::INFINITY });
             if g > 0.0 && g >= p.residual { p.rho = (p.rho * RHO_GAIN).min(RHO_MAX); }        // stuck → harder
             else if g <= 0.0 { p.rho = (p.rho / RHO_GAIN).max(RHO_FLOOR); }                   // slack → softer
             p.residual = g;
             let next = (p.lambda - p.rho * g).clamp(-LAMBDA_MAX, 0.0);   // price −λ relaxes when g < 0
             drift_sq += f64::from(next - p.lambda).powi(2);
             p.lambda = next;
             if next <= -LAMBDA_MAX && r > 0.0 { self.saturated.push(key.0); }
         }
         self.drift = drift_sq.sqrt();
         self.steps += 1;
         self.bind(reqs);
     }
     pub fn saturated(&self) -> &[&'static str] { &self.saturated }
     pub fn steps(&self) -> u32 { self.steps }
     ```
     The only behavioural change for a batch with `r > 0` is the upper clamp at 0 (unchanged: it was never positive);
     for `r = 0` the price now falls by `ρ·(1 − usage)` or `ρ·(1 − criticality)` per epoch instead of staying.
     `RunStats` gains `dual_steps: u32` = `prices.steps()` at the end of `solve`.
  4. `lib.rs:377–383` convergence = `feasible && prices.drift() < PRICE_STATIONARY && prices.saturated().is_empty()`.
     `MetadataReport.binding: Vec<String>` (new field) lists saturated kinds at the end of the run.
- Tests (`backend/gp/src/lib.rs` `price_tests`):
  - `a_slack_budget_relaxes_its_price`: a second test rule `Spent` (same as `Budget`, gp/lib.rs:347–356, plus
    `fn usage(self, l) -> Option<f32> { Some(1.0 + l.x[0] as f32 / 1000.0) }`, unclamped, so `x[0] = -500` reads
    usage 0.5 with residual 0); `x[0] = 500` for 3 settles, then `x[0] = -500` → `weight_of(0)` strictly decreases
    each settle and reaches exactly 0.0 (clamped), then stays 0.0.
  - `place_does_not_settle`: fresh `Prices`; `gp::place` and `dp::place` on the dp test fixture leave
    `drift() == INFINITY` and `steps() == 0`.
  - `a_saturated_price_is_reported`: `Budget` with `x[0] = 1000` (residual 1.0) for 30 settles → `saturated()`
    contains the batch kind (λ reaches −64 after the ρ ramp 0.25, 0.5, …, 64: Σ < 30 steps).
  - `frontend/library/src/lib.rs` `one_dual_step_per_epoch` (sky130, pair.spice, `feedback_iters: 3, outer_iters: 1,
    starts: 1`): `sol.stats.dual_steps == sol.stats.iterations`.
- Acceptance: T6. `rho_escalates_for_a_residual_that_does_not_shrink` (gp/lib.rs:386–413) and
  `drift_decreases_as_prices_settle` (gp/lib.rs:418–440) pass unchanged: the test rule `Budget` reports no usage and
  default criticality 1.0 (headroom 0.0, rule.rs:24–26, 281–289), so at residual 0 `g = 0` and drift is exactly 0.
- Risks / notes: relaxing prices can slow `converged`; `RunStats.stop` (FLOW-08) makes the reason visible.

### FLOW-04 Deck ownership and loading: vendored embedded decks, a sidecar key registry, required = read

Status: done in M0 (`5ebffd8`). As built: `REQUIRED_RULES` is gone; the registry is `verify::sidecar::KEYS` (backend/verify/src/sidecar.rs:62, `Key { name, kind, required, sourced, reader }`); `Pdk::builtin(name)` (backend/verify/src/pdk.rs:82) loads a compiled-in sidecar and the CLI takes a built-in name or a sidecar path; `dummy_gates_per_end` and `max_finger_width` are registered required keys.

Field report: FR-8 (only sky130.json and generic_finfet.json shipped; gf180/ihp in another schema). Since this item all four sidecars and decks are compiled in (`Pdk::builtin`, backend/verify/src/decks.rs) and the CLI takes `gf180mcu`/`ihp_sg13g2` by name; the package itself is FLOW-17's `--version` plus a rebuild outside this repository.

- Priority: **P0**. Effort: **M**. Depends on: none.
- Why: AV-21, AV-22, D2, CRATES #9a, AC-27; PERF §0.2 row 4 (needs a local deck-edit mechanism for MOS bulk / wells /
  BJT recognisers, which are *non-additive* edits that D2 rules out as an overlay). NOTES-02 (unknown never passes) and
  NOTES-60 (provenance survives review). AR-29 (`Process::rule(name, default)`, kernel/core/src/process.rs:11–12,
  makes a silent default the normal path): the registry closes the sidecar half — a `required` key missing from a
  sidecar fails `Pdk::load`, so the compiled default is never silently used on a shipped deck; changing the trait
  signature to `Option` is not done here (≈ every generator call site; no measured defect beyond the missing-key case).
- Current: §1.3 D1–D4.
- Change:
  1. Vendor `GP/pdks/{sky130,gf180mcu,ihp_sg13g2,generic_finfet}.deck` into `pdks/decks/` unchanged except a new
     first line `# vendored from GPurify 8df8c09af1dfd849d5271abb4f6f03e595cbf390 pdks/<file>; Philis edits are marked "# PHILIS:"`.
     Every later edit in any plan adds a `# PHILIS: <reason, source>` comment on the line above.
  2. New `backend/verify/src/decks.rs`:
     ```rust
     /// Decks and sidecars compiled into the binary: a moved or packaged `philis` loads its PDKs.
     pub(crate) const DECKS: &[(&str, &str)] = &[
         ("sky130.deck", include_str!("../../../pdks/decks/sky130.deck")),
         ("gf180mcu.deck", include_str!("../../../pdks/decks/gf180mcu.deck")),
         ("ihp_sg13g2.deck", include_str!("../../../pdks/decks/ihp_sg13g2.deck")),
         ("generic_finfet.deck", include_str!("../../../pdks/decks/generic_finfet.deck")),
     ];
     pub(crate) const SIDECARS: &[(&str, &str)] = &[
         ("sky130", include_str!("../../../pdks/sky130.json")), ("gf180mcu", include_str!("../../../pdks/gf180mcu.json")),
         ("ihp_sg13g2", include_str!("../../../pdks/ihp_sg13g2.json")), ("generic_finfet", include_str!("../../../pdks/generic_finfet.json")),
     ];
     ```
     `Pdk::deck_text` (pdk.rs:81–91) resolves the sidecar's `deck` name in `DECKS` first, then as an absolute path;
     the `GPURIFY_PDKS` fallback stays for decks not vendored. New `pub fn builtin(name: &str) -> Result<Pdk, String>`
     loads from `SIDECARS`. `build.rs` adds `cargo:rerun-if-changed=../../pdks`.
  3. New `backend/verify/src/sidecar.rs` — the registry of every `cell.*` key:
     ```rust
     pub enum Kind { Nm, Count, Bool, Real, Tier, Text, List, Table, Layers }
     pub struct Key { pub name: &'static str, pub kind: Kind, pub required: bool, pub sourced: bool, pub reader: &'static str }
     pub const KEYS: &[Key] = &[ /* one row per key in §3 FLOW-04 table below */ ];
     /// Every problem at once: unknown key (a typo), wrong kind, a required key
     /// missing, a sourced key with a non-null value and no non-empty `<name>_source`.
     pub fn validate(cell: &serde_json::Value) -> Vec<String>;
     ```
     `Pdk::validate` (pdk.rs:180–304) appends `sidecar::validate(&self.cell)`; `REQUIRED_RULES` (pdk.rs:1039–1071)
     is deleted in favour of `required: true` rows. A `_source` text that starts with `UNVERIFIED` is accepted and the
     key is listed in `Pdk::unverified() -> Vec<&str>`, which `metadata::build` copies into a new informational
     `MetadataReport.assumed: Vec<String>` (reported, not blocking the certificate).
  4. Accessors on `Pdk`: `pub fn provenance(&self, key: &str) -> Option<&str>` (reads `<key>_source`);
     `cell_f32` stays; the tier accessor is MAT-07's `Process::tier` (FLOW-06 registers the keys).
  5. Required set = read set: `required: true` exactly for the keys generators read through `dim()`/`rule()` with a
     compiled default (kernel/cells/src/builder.rs:195–212). Today's unread demands (`bjt_base_frac_permille`,
     `bjt_collector_frac_permille`, `bjt_max_emitter_stripe`, `bjt_stripe_gap`, `tie_max_dist_nm`, `n_well_depth`,
     `p_well_depth`, `p_epi_thickness`, `retrograde_pwell`) become `required: false`; `bjt_base_frac`/`bjt_collector_frac` floats and their `_permille`
     duplicates are deleted from the sidecars if the recording test finds no reader.
  - Registry rows for keys that are *process data* (sourced: true): `avt_n_mv_um`, `avt_p_mv_um`, `svt_uv_per_um`,
    `vt_tc_uv_per_k`, `vt_tc_uv_per_k_p`, `lod_kvth0_n_mv_um`, `lod_kvth0_p_mv_um`, `gate_cap_af_um2`,
    `cap_density_ff_um2`, `tie_max_dist_nm`, `n_well_depth`, `p_well_depth`, `p_epi_thickness`, `retrograde_pwell`,
    and every FLOW-06 key. Generator construction dimensions (`sd_width`, `device_gap`, `res_seg_gap`, …) are
    `sourced: false` (they are Philis choices bounded below by deck rules, builder.rs:195–212).
- Tests:
  - `backend/verify/src/pdk.rs` `every_builtin_pdk_loads` (`Pdk::builtin` for the four names, no filesystem read).
  - `vendored_decks_name_their_origin`: line 1 of each `DECKS` entry starts with `# vendored from GPurify 8df8c09`.
  - `sidecar_rejects_a_misspelt_key`: sky130 sidecar with `wpe_clearence_nm` added → `Err` naming it.
  - `a_process_number_needs_a_source`: sky130 sidecar with `avt_n_mv_um_source` removed → `Err` naming `avt_n_mv_um`.
  - New `kernel/cells/tests/deck_keys.rs` `generators_read_exactly_the_required_keys`: a `Recording<'a>(&'a Pdk,
    RefCell<BTreeSet<String>>)` wrapper implementing `Process` (delegating every method, recording `rule()` names)
    runs `enumerate` + `draw` of every generator on sky130 over groups built as in kernel/cells/tests/cell_selfcheck.rs:42–95
    (`group_of`, `all_variants`); the in-crate `testkit` (kernel/cells/src/lib.rs:56–166) is `#[cfg(test)] pub(crate)`
    and not reachable from `tests/`;
    assert `{required keys} ⊆ recorded` and `recorded ∩ sidecar keys ⊆ {registry keys}`.
- Acceptance: T3 (0 / 0 / 0); the flake package (`nix build`) runs `philis --pdk sky130` from outside the repo.
- Risks / notes: vendored decks drift from GPurify upstream; a GPurify bump must re-copy the decks and re-apply the
  `# PHILIS:` edits (OQ-2). Rules without a marker are upstream text by construction.

### FLOW-05 Deck data fixes (sky130 first) — latch-up, antenna thickness, resistor body sheet, density_cmp

Status: done in M0 (`bb556b4`); `tie_max_dist_nm` is sourced on all four sidecars (sky130 15 000, gf180mcu 20 000, ihp_sg13g2 20 000, generic_finfet 30 000) and still read by no code (CELL-13).

- Priority: **P0**. Effort: **S**. Depends on: FLOW-04.
- Ownership: steps 1 and 6 are the same edit as REL-06 steps 1–2 (plan-06) and step 2 is REL-02 step 6; FLOW lands
  them here (deck file owner, M0) and REL-06 keeps its CELL contract (step 3) and its `LU.` fixture tally. FLOW's LU
  layers are qualified by well (`not nwell`, `not dnwell`) where REL-06 used bare `nsd`/`psd`: an N-diff inside an
  n-well (a varactor, an NMOS-in-nwell structure) is not the "N-diff" that LU.2 measures to a P-tap.
- Why: AV-08, AV-17 (deck half), AV-16, AV-18 (reader half), CRATES #5/#6. Hastings §12.2.9: maximum distance to a
  backgate contact "usually … between 25 and 250 µm" (hastings.txt L37002–37008, "250 µm" blank in the text, read on
  PDF p.621; H12-43) — the foundry number governs. Hastings §5.1.6: antenna limits are per layer (hastings.txt L13128–13160, PDF p.228).
- Current: §1.3 D5–D8.
- Change:
  1. `pdks/decks/sky130.deck` — at the end of the rule section, before `# --- Connectivity, bottom up ---`
     (upstream line 505):
     ```text
     # PHILIS: latch-up, VOL/magic/sky130A.tech:4163-4170 (LU.2, LU.2.1, LU.3: "< 15.0um")
     layer lu_ndiff = (nsd not nwell) not dnwell
     layer lu_ndiff_dnw = (nsd not nwell) and dnwell
     layer lu_pdiff = psd and nwell
     rule LU.2 tap_distance(lu_ndiff, ptap) <= 15um
     rule LU.2.1 tap_distance(lu_ndiff_dnw, ptap) <= 15um
     rule LU.3 tap_distance(lu_pdiff, ntap) <= 15um
     ```
     (`tap_distance` is GPurify's `max_distance_to_tap`, GP/crates/ingest/src/deck/kinds.rs:326; syntax as
     GP/pdks/gf180mcu.deck:303–306. The operand layers follow magic's own definitions: `ptap_missing` = `*ndiff`
     `and-not dnwell` (VOL/magic/sky130A.tech:1514–1516), `dptap_missing` = `*ndiff and dnwell` (:1608–1610),
     `ntap_missing` = `*pdiff` (:1547–1549); magic's `ndiff`/`pdiff` types are the diffusions outside/inside n-well.)
  2. `pdks/decks/sky130.deck` upstream line 459: `ar.met3.1 … sidewall: 2000nm` → `845nm`, marked
     `# PHILIS: = pex met3 thickness (this deck) and VOL/magic/sky130A.tech:4890 'height allm3 … 0.845'`.
  3. `pdks/decks/sky130.deck` — after the `pex` block (upstream line 598):
     `pex rbody_po thickness 180nm height 326.2nm sheet 48.2ohm dielectric 3.9 area_cap 106.13aF/um2 fringe_cap 55.27aF/um`,
     marked `# PHILIS: sheet = rsh of sky130_fd_pr__res_generic_po (rp1 = 48.2, VOL/ngspice/r+c/res_typical__cap_typical.spice:21
     and VOL/ngspice/sky130_fd_pr__model__r+c.model.spice:134); geometry columns = poly_c`. Two more rows, same
     geometry columns as `poly_c` (GP/pdks/sky130.deck:582):
     `pex rbody_high_po … sheet 317.3885ohm …` (`rsheet = 317.3885`,
     VOL/../libs.ref/sky130_fd_pr/spice/sky130_fd_pr__res_high_po.model.spice:27) and
     `pex rbody_xhigh_po … sheet 2000ohm …` (`rsheet = 2000.0`, …/sky130_fd_pr__res_xhigh_po.model.spice:27), each
     marked `# PHILIS:` with that citation. These are the models' nominal body sheets; head resistance (`rhead_ps`,
     res_high_po.model.spice:29) and width bias (`rbody_dw`, :28) stay in the model card, not the deck row.
  4. `backend/verify/src/pdk.rs:661–674` `pex_row` (today: direct operands only, `max_by_key(conductor)` picks the
     last of equals):
     ```rust
     fn pex_row(&self, layer: LayerId) -> Option<usize> {
         let st = &self.deck.stack;
         let declared = |i: usize| st.thickness_nm.get(i).is_some_and(|&t| t > 0.0) || st.dielectric_k.get(i).is_some_and(|&k| k > 0.0);
         if declared(layer.0 as usize) { return Some(layer.0 as usize); }
         let cands: Vec<GvLayerId> = (0..self.deck.layers.len()).map(|i| GvLayerId(i as u16))
             .filter(|&d| d.0 != layer.0 && declared(d.0 as usize) && self.reaches(d, layer.0)).collect();
         // A declared conductor beats a cut/derived part of it (sky130 `poly`: poly_c over licon_po).
         let conductors: Vec<GvLayerId> = cands.iter().copied().filter(|d| self.deck.connectivity.conductors.contains(d)).collect();
         let pool = if conductors.is_empty() { cands } else { conductors };
         (pool.len() == 1).then(|| pool[0].0 as usize)   // several → None: ambiguous, never an arbitrary pick
     }
     ```
     sky130 outcomes (GP/pdks/sky130.deck:102–114, 506, 582–598): `poly` → `poly_c` 48.2 Ω/□ (candidates `poly_c`,
     `licon_po`; `poly_c` is the only conductor); `licon` → `None` (five cut rows, none a conductor); `poly_rs` (the
     `rpoly` role, pdks/sky130.json:13) → `None` (`poly_c = poly not poly_rs` keeps only `poly`; today it wrongly
     answers 48.2).
     New, all in pdk.rs:
     - `pub fn pex_f32_named(&self, name: &str, key: &str) -> Option<f32>` (the named deck layer's own row).
     - `pub fn device_sheet_ohm(&self, model: &str) -> Option<f32>`: the deck device row whose model is
       `deck_model(model)` (pdk.rs:1116–1123) → its marker layer (`deck.devices.marker[r]`) → that layer's own `pex`
       sheet. `Overlay::sheet_ohm` (pdk.rs:1168–1170): for a `RESISTOR_ROLES` role (pdk.rs:1088) →
       `device_sheet_ohm(&self.recipe.model)`, else `pdk.sheet_ohm(role)`.
     - `kernel/core/src/process.rs` trait `Process`: `fn cut_ohm(&self, cut: &str, onto: &str) -> Option<f32> { None }`
       (Ω per cut of role `cut` landing on role `onto`). `Pdk` implements it as: the cut layer's own `pex` row if
       declared, else the max `sheet_res_ohm_sq` over declared derived layers `d` with `reaches(d, cut)` and
       `reaches(d, onto)` (max = conservative); `Overlay` delegates. sky130: `cut_ohm("licon", "poly") = 152`
       (`licon_po` only, since `licon_po = licon and poly_c` keeps both operands); `cut_ohm("licon", "tap") = 585`
       (`licon_nt` 185, `licon_pt` 585).
     - Callers: `kernel/cells/src/mosfet.rs:62` `two_ended_gate_pays` reads `process.cut_ohm("licon", "poly")` instead
       of `sheet_ohm("licon")` (which becomes `None` and would silently disable two-ended gates);
       `frontend/library/src/lib.rs:521–524` `ring_cut_ohm` → `pdk.cut_ohm("licon", "tap").unwrap_or(0.0)`;
       `frontend/library/src/cellgen.rs:638–639` `folds` reads one R□: `poly_sq` from `pdk.sheet_ohm("poly")` for both
       the p2p cap and the gate-R floor (today two reads, `sheet_ohm` and `pex_f32`, AF-15 "two sources for poly R□").
     This is also the defect plan-07 hands over (PERF-05 cut entry): without the conductor preference, `poly` would
     answer `None` and disable the p2p finger limit (cellgen.rs:638) and two-ended gates (mosfet.rs:61–66).
  5. `pdk.rs:619–637` `density_rules` also reads kind `density_cmp`: `window_x` (Length), `min_density` and
     `max_density` (Ratio, each optional) → one `(layer, window, limit, is_max)` row per present bound.
  6. Sidecars: sky130 `tie_max_dist_nm: 3000 → 15000` (pdks/sky130.json:85) with `_source` = the tech-file lines above;
     gf180mcu and ihp_sg13g2 already carry `20000` (pdks/gf180mcu.json:81, pdks/ihp_sg13g2.json:76) and gain only
     `_source` = gf180mcu.deck:303, 305 (`DF.13_3.3V`, `DF.14_3.3V`; 15 µm for 5 V devices at :304, :306) and
     ihp_sg13g2.deck:595–596 respectively. generic_finfet: `tie_max_dist_nm: 3000 → 30000` (pdks/generic_finfet.json:91)
     with `_source` = GP/pdks/generic_finfet.deck:165–166 (`ACTIVE.LUP.1.*`, `missing_tie … max_distance: 30um`).
     Delete the stale sky130 `antenna_source` sentence
     "kept out of deck rules because gpurify refuses sidewall antenna rules" and the dead `cell.antenna_sidewall` table
     (pdks/sky130.json:100–126; used only when the deck has no rule, pdk.rs:586–592).
- Tests (`backend/verify/src/pdk.rs` tests module unless noted):
  - `latch_up_rules_find_a_far_tap` (sky130): one NMOS diffusion 20 µm from the nearest ptap → a DRC finding whose rule
    is `LU.2`; the same at 10 µm → none.
  - `antenna_sidewall_thickness_matches_pex` (sky130): for each routing metal with an antenna sidewall rule,
    |sidewall − pex thickness| / pex thickness ≤ 0.05 [policy].
  - `resistor_body_sheet_is_the_body`: `device_sheet_ohm("sky130_fd_pr__res_generic_po") == Some(48.2)`;
    `device_sheet_ohm("sky130_fd_pr__res_high_po")` within 1e-3 of 317.3885; `sheet_ohm("licon") == None` (ambiguous);
    `sheet_ohm("rpoly") == None`.
  - `poly_answers_with_its_conductor`: sky130 `sheet_ohm("poly") == Some(48.2)`;
    `cut_ohm("licon", "poly") == Some(152.0)`; `cut_ohm("licon", "tap") == Some(585.0)`.
  - `routing_layers_keep_their_pex_rows`: on sky130, gf180mcu and ihp_sg13g2, `pex_f32(l, "thickness_nm").is_some()`
    for every `routing_metals` and `routing_cuts` entry (run once on the unchanged code first; it must already pass).
  - `kernel/cells/src/mosfet.rs` tests: `two_ended_gate_still_pays_on_sky130`: `two_ended_gate_pays(&pdk, 10_000, 150)`
    is `true` (48.2·10000/(3·150) = 1071 Ω > 152 Ω).
  - `density_cmp_is_read`: sky130 `density_rules()` contains `(met1, 700_000, 0.7, true)`.
- Acceptance: T4; `bench local` DRC counts reported with LU rules on (true findings go to CELL/REL, not waived).
- Risks / notes: LU.2 may flag long-finger cells (AC-07: one-sided tap strip, fingers to 10 µm); that is the purpose.
  The in-loop latch-up model and the tap-row fix are REL/CELL.

### FLOW-06 Sidecar plumbing for keys other plans own: registry rows, recipe layers, provenance
- Priority: **P1**. Effort: **S**. Depends on: FLOW-04 (registry). Lands with or before the first consumer item.
- Why: AV-22 (unsourced / unread keys), H13-42 (class model; tiers today unread, §1.3 D3), AC-01 / AV-32
  (`cap_density_ff_um2` = MIM value, generator draws MOM, §1.3 D10), NOTES-60 (provenance). Every new sidecar key in
  plans 02, 03, 06 must enter FLOW-04's registry, or `sidecar::validate` rejects it as a typo. The key *content* is
  owned by the consuming plan (see §0.2); this item owns only the schema, the loader and the provenance rule.
- Current: §1.3 D3, D10; `recipe_layers` (backend/verify/src/pdk.rs:1103–1114) scans only `["resistors"]`, so layers
  that a `capacitors`/`bjts`/`mosfets` recipe draws (e.g. sky130 `capm`) are not `philis_drawn`, and every two-layer
  lookup through `widest_on` on them returns `None` (plan-03 CELL-08 precondition: capm inset read as 0, not 140).
- Change:
  1. `backend/verify/src/sidecar.rs` `KEYS` rows (kind, required, sourced) for the keys other plans add; every row
     `required: false`:
     | key | kind | sourced | owner item |
     |---|---|---|---|
     | `lod_moat_ext_nm`, `wpe_clearance_nm`, `dummy_reach_nm`, `gate_ext_extra_nm` | `Tier` (JSON array of 3, nm or `null`, index = `MatchClass as usize`) | yes | MAT-07 |
     | `abeta_pct_um` (cell), `cap_ka_pct_um`, `bjt_ka_pct_um`, `vbe_tc_uv_per_k`; per resistor recipe `ka_pct_um`, `tc_ppm_per_k`, `sd_pct_per_mm` | `Real` | yes | MAT-09, MAT-10 |
     | `em_ref_temp_c`, `em_activation_ev`, `em_current_exponent` | `Real` | yes | REL-05 |
     | `substrate_kind` (`"bulk"`/`"epi"`), `epi_thickness_nm` | `Text`, `Nm` | yes | REL-09 |
     | `capacitors`, `bjts`, `mosfets` | `Table` (same shape as `resistors`: `default` + `recipes{model, aliases, layers, …}`) | per-recipe `*_source` | CELL-08, CELL-09, CELL-16 |
     `Kind::Tier` validation: an array of exactly 3 entries, each a non-negative integer or `null`. The deprecated
     scalars `wpe_clearance_moderate`, `lod_moat_ext_moderate` stay registered (`required: false`) until MAT-07 deletes
     them.
  2. `pdk.rs:1105` `for kind in ["resistors"]` → `for kind in ["resistors", "capacitors", "bjts", "mosfets"]`
     (one line; a table that is absent is skipped by the existing `continue`).
  3. `cap_density_ff_um2` (pdks/sky130.json:8, read only by the benchmark preprocessor, benchmarks/src/fixtures.rs:556)
     gains `_source`: "= camimc of sky130_fd_pr__cap_mim_m3_1 (MIM), VOL/ngspice/r+c/res_typical__cap_typical__lin.spice:7;
     the MOM that cap_array draws today is not this value (AV-32)". It is deleted once CELL-08's `capacitors` table
     carries `c_area_af_um2` and PERF's preprocessor reads it.
- Tests (`backend/verify/src/sidecar.rs` / `pdk.rs` tests):
  - `tier_arrays_are_validated`: sky130 sidecar with `"wpe_clearance_nm": [2000, 3000]` → `Err` naming the key
    (2 entries); `[2000, 3000, null]` → `Ok`.
  - `every_registered_sourced_key_present_has_a_source`: for all four built-in sidecars, each present sourced key has a
    non-empty `<key>_source` (the FLOW-04 rule, run over the new rows).
  - `recipe_layers_cover_capacitor_tables`: a sky130 sidecar with CELL-08's `capacitors` table (plan-03 CELL-08 step
    1 JSON) → `Overlay::enclosure("cap_bot", "cap_ins")` for the `mim_m3` recipe returns `Some(140)`
    (capm.3, GP/pdks/sky130.deck:408), not `None`.
- Acceptance: T3 stays 0 / 0 / 0 after MAT-07, MAT-09/10, REL-05, REL-09, CELL-06/08/09/16 add their keys.
- Risks / notes: the tier *values* and their Hastings citations are MAT-07's table (plan-02); FLOW does not duplicate
  them. Cross-plan naming: plan-05 (RTE-16) and plan-07 (PERF-23) name the accessor `pnr_core::MatchClass` /
  `Pdk::tier_nm(key, MatchClass)`; the implemented names are MAT-07's `analog::matching::MatchClass` and
  `Process::tier(name, class as usize)` (kernel/core cannot name an `analog` type).

### FLOW-07 SPICE front end: hierarchy, ports, parameters, values, deck-classified models
- Status: done in M1 (`40755ad`, `295bef3`). Deviation: step 8's `opts.top` wins over file-level cards (amended
  above). Acceptance gap: `preprocess_spice` is not reduced to model mapping; carried to CELL-06/CELL-08, which did not
  reduce it either (it still appends `l=0.15u` and rewrites bare R/C at the M1 close); carried to M2 (FLOW T2).
- Priority: **P0**. Effort: **L**. Depends on: FLOW-01, FLOW-04.
- Why: AF-02 (critical), AF-10, AF-17, AF-33 (parser part), I4, AA-11/G8 (ports and sources are role evidence), AV-04
  (true ports vs labels), PERF §0.2 row 2. LAYLA's inputs are a netlist, a spec file and a technology file
  (Lampaert §1.6, lampaert.txt L1151–1170, PDF pp.31–32); the survey's Fig. 1 flow starts "Given a netlist and design
  specifications" (perf_driven_survey.txt L59–60).
- Current: §1.2 I2–I6.
- Change: rewrite `frontend/library/src/parse.rs` (keeping `pub fn spice(text) -> Result<Netlist, String>` as
  `spice_with(text, &ParseOptions::default())`; both exist since FLOW-01, with `ParseOptions { size: SizeConvention }`
  crate-private and `run` the only caller passing `Config.size_convention`, lib.rs:240 on m0):
  1. Data model, `kernel/core/src/netlist.rs` (`Netlist` gains `#[derive(Default)]`; every `Netlist { … }` literal —
     on m0 backend/annotator/src/tests.rs ×9, backend/annotator/tests/common/mod.rs ×2 (EXT-01), frontend/library/src/cellgen.rs ×4,
     oppoint.rs ×4, perf.rs ×2, elaborate.rs ×1, lib.rs ×1 (tests), parse.rs ×1, kernel/macroMaster/src/lib.rs ×1;
     re-count with `grep -rn 'Netlist {'` — gets `..Default::default()`):
     ```rust
     pub struct Netlist {
         pub devices: Vec<Device>, pub nets: Vec<Net>,
         /// Top sub-circuit ports in declaration order (empty: no .subckt around the top).
         pub ports: Vec<NetId>,
         /// Every flattened sub-circuit instance, parents before children.
         pub insts: Vec<SubcktInst>,
         /// Innermost instance of each device; `None` = top level. Parallel to `devices`.
         pub device_inst: Vec<Option<u32>>,
         /// Independent/controlled sources and couplings: evidence, not devices.
         pub sources: Vec<SourceCard>,
     }
     pub struct SubcktInst { pub path: String /* "X1/X3" */, pub subckt: String, pub parent: Option<u32>, pub ports: Vec<NetId> /* actuals, formal order */ }
     pub struct SourceCard { pub name: String, pub kind: char /* V I E F G H B K */, pub nodes: Vec<NetId>, pub dc: Option<f64>, pub waveform: bool /* PULSE/PWL/SIN/EXP present */ }
     ```
  2. FLOW-01's `ParseOptions { size }` gains `title_line: bool /* default false */, top: Option<String>, models: Vec<(String, DeviceKind)>` and becomes `pub` (re-exported) so `run` can pass `model_table(pdk)`.
     `library::model_table(pdk) -> Vec<(String, DeviceKind)>`: every deck device row (`pdk.deck.devices`: mos /
     bjt / resistor / capacitor / diode kinds — GPurify `DeviceKind` has `Bjt` too, GP/crates/ingest/src/deck/mod.rs:335–341,
     and gf180mcu/ihp_sg13g2 carry 2 `device bjt` rows each; BJT polarity from the model token `npn`/`pnp`) with MOS
     polarity = the S/D terminal layer reaches the `psdm` role (`Pdk::reaches`, private today, pdk.rs:758, so it becomes
     `pub`), plus every sidecar recipe `model`/`aliases` (pdk.rs:1129–1144).
  3. Lexing: `+` continuation and trailing-`\` joins; `*` comment lines; inline `;` and ` $ ` comments stripped;
     `.control … .endc` skipped; case-insensitive keywords; net identity key = lowercase, display = first spelling
     (AF-17); line 1 skipped iff `title_line`.
  4. Directives: `.subckt NAME p1 … [params: k=v …]` / `.ends` (nested definitions → error "nested .subckt is not
     supported"); `.param k=v …` (file scope, in order); `.global n …` (never prefixed); `.include`, `.lib`, `.model`,
     `.option(s)`, `.temp`, `.end`, analysis and `.meas` cards → `ParseReport.ignored` (not errors).
  5. Values: number with SI suffix (`meg t g k m u n p f a`), `{expr}`, `'expr'`, or a bare identifier. Expression
     grammar: `expr := term (('+'|'-') term)*`, `term := factor (('*'|'/') factor)*`, `factor := unary (('**'|'^') factor)?`,
     `unary := '-' unary | primary`, `primary := number | ident | ident '(' args ')' | '(' expr ')'`; functions
     `sqrt abs min max`; scope = instance overrides → `.subckt` defaults → file `.param`. An unresolved identifier is an
     error naming it (never 0, parse.rs:253).
  6. Cards: `M` (D G S [B]); `X` (last positional = sub-circuit or model); `R/C/L/D` (2 nodes, optional 3rd = `B`);
     `Q` (C B E [S], the 4th kept as terminal `S`, fixing I4); `V I E F G H B K` → `sources`. For `R/C/L` letter cards
     the first numeric positional after the nodes is the value, stored as `r_mohm` / `c_af` / `ind_ph` (i64); a
     remaining identifier is the model.
  7. Classification of an `X` target, in order: (a) a `.subckt` in the file → instantiate (arity must equal its port
     count); (b) `opts.models` (exact or vendor-prefix match as `Pdk::deck_model`, pdk.rs:1116–1123); (c) token rule on
     `_`-separated tokens of the lowercase model: `npn`/`pnp` → BJT, `res` → resistor, `cap`/`mim` → capacitor,
     `diode` → diode, `ind` → inductor (so `reset_logic`, `window_cmp` never match); (d) otherwise
     `Err("device {name}: unknown model {model}: not a .subckt here and not a deck model")` — no NMOS default.
  8. Flattening: device name = `path + "/" + name` (e.g. `X1/XM2`); internal net = `path + "/" + net`; formal ports
     map to the actuals; `.global` nets and `0` are never prefixed; instance `m=k` multiplies every child device's `m`.
     Top = `opts.top` when set (file-level cards and sources are then dropped: a simulator deck's testbench around
     its DUT); else the file-level cards if any; else the unique uninstantiated `.subckt` (several → error listing
     them, naming `ParseOptions::top`); `Netlist.ports` = the top sub-circuit's formals.
  9. MOS sizes normalised per FLOW-01; `w`/`l` ≤ 0 after evaluation is an error for a MOS.
  10. `oppoint::instance_name` (oppoint.rs:296–302, PERF-owned) maps `/` to `__` so `parse_show`'s `m.<name>.` split
      (oppoint.rs:457–461) keeps working; collisions after mapping are a parse error.
  11. The benchmark's `preprocess_spice` keeps only PDK model-name mapping; `.param`, joins and value→W/L sizing move
      here or to CELL's value-based sizing (PERF owns the bench file).
- Tests (`frontend/library/src/parse.rs` unit tests + new `frontend/library/tests/parse_real.rs`):
  - `subckt_ports_and_hierarchy`: two-level netlist → devices `X1/XM1`, nets `X1/n1`, `ports` in declaration order,
    `insts[0].path == "X1"`, `device_inst` set.
  - `params_and_expressions`: `.param wu=0.5u` and `W={2*wu}` → `w == 1000`; `W=wx` with `wx` undefined → `Err` naming `wx`.
  - `rc_values`: `R1 a b 10k` → `r_mohm == 10_000_000`; `C1 a b 1p` → `c_af == 1_000_000`.
  - `nets_are_case_insensitive`: `VDD` and `vdd` → one net displayed `VDD`.
  - `sources_are_evidence_not_devices`: `V1 vdd 0 1.8` → `sources[0].dc == Some(1.8)`, device count unchanged;
    `Vclk clk 0 PULSE(0 1.8 0 1n 1n 5n 10n)` → `waveform == true`.
  - `unknown_model_is_an_error`: `X1 a b c myamp` without `.subckt myamp` → `Err` containing `unknown model`.
  - `token_rule_has_no_substring_false_positives`: models `reset_logic`, `window_cmp` → not resistor/inductor.
  - `recipe_aliases_classify_gf180_and_ihp_resistors`: with `model_table(gf180mcu)`, `XR1 a b ppolyf_u …` → Resistor;
    with `model_table(ihp_sg13g2)`, `rsil`/`rppd` → Resistor.
  - `bjt_substrate_node_is_kept`: `Q1 c b e s pnp_05v5` → terminal `S` present.
  - `local_fixtures_parse_as_before`: all 10 `benchmarks/fixtures/*.spice` parse; device and net counts equal today's
    bench (e.g. ota 5 devices / 9 nets, audit-06 G.3); ota `ports` = `[vinp, vinm, vout1, vout2, VDD, VSS]`.
- Acceptance: T2. `bench local` runs with `preprocess_spice` reduced to model mapping and reproduces DRC/LVS verdicts.
- Risks / notes: hierarchical names change device names in reports and in the op deck (step 10). `NetId`/`DeviceId`
  remain `u16`: > 65 535 nets or devices is a parse error, not a silent wrap (AA-35).

### FLOW-08 Epoch loop: warm and cold epochs, history lifecycle, blame-driven escalation, stop reasons

Field report: FR-2 (a 100-iteration run never stopped early at DRC 0 and LVS match). On m0 the loop stops once `converged` (FLOW-03; bench "conv."); this item's stop reasons make the reason visible per run.

- Priority: **P1**. Effort: **M**. Depends on: FLOW-02, FLOW-03, FLOW-09 step 1 (`search` is where the loop lives), PLC-10 step 0 (`dp::Schedule`, flat path; done in M1, `2eb81b2`); PLC-10 step 1 (`dp::Start::Warm`) only once `DpMode::Sp` is the default.
- Why: AF-06, AF-14, AP-09, AT-10, AT-34 (history part), NOTES-08 (termination reason), PL-19/PL-21 (audit-07
  §2.14.1). PLAN.md:52 carries "the best feasible incumbent" and the representation between iterations; PLAN.md:115
  terminates on feasibility + stationary prices + no improving move; PLAN.md:178 rests convergence on PathFinder's
  history term, which presumes the same resources recur between iterations (this plan's inference; PLAN.md does not
  state the fixed-graph premise). Lampaert's schedule starts hot to escape and cools to refine (Lampaert §4.11 eqs
  4.37–4.40, lampaert.txt L4913–5019, PDF pp.117–119).
- Current: §1.1 L1–L3, L6–L7, L13.
- Change (`frontend/library/src/lib.rs`):
  1. `enum Start<'a> { Cold, Warm(&'a Layout) }` (library-local; PLC's plan defines its own
     `dp::Start { Cold(&Layout), Constructive, Warm { tree, variant, orient } }`, plan-04-placement.md:325, which this maps
     onto); `Layout` gains `#[derive(Clone)]` (kernel/core/src/layout.rs:11; every field is `Vec` or `Arc`; AR-26).
  2. Schedule [policy, measure]: epoch `k` is **cold** iff there is no incumbent from the current assignment
     (none yet, or every epoch since the last escalation lost to an older incumbent), or `k % COLD_EVERY == 0`
     (`COLD_EVERY = 4`); otherwise **warm** from the incumbent's layout
     and variants. Ablation over `COLD_EVERY ∈ {2, 4, 8, ∞}` on the bench decides the default.
  3. `Flow::epoch(&self, assignment, reshape, start: Start, w: &Weights, prices, neg, seed)`:
     - Cold: today's path (`gp::place` → `dp::place(…, dp::Schedule::cold())`).
     - Warm: no gp; `coarse = incumbent.clone()`; `dp::place(&coarse, …, dp::Schedule::warm())`. PLC-10 step 0 (M1) built
       `dp::Schedule { range0, max_temps, t0_scale }` (backend/dp/src/lib.rs): `cold()` = `{ 0.4, 220, 0.02 }`, today's
       constants, which the flow passes now; `warm()` = `{ 0.05, 60, 0.002 }` [policy, measure], unused until this item. The flat path takes the incumbent as its `coarse` input, so it needs
       no `dp::Start`).
  4. History: `neg = gr::Negotiation::new()` before every cold epoch; kept across warm epochs (same placement
     neighbourhood). `Negotiation` is already `Clone` (backend/gr/src/lib.rs, RTE-08 in M1; a `BTreeMap` keyed on quantised `(layer, x, y)`); dr's second
     run after diode insertion (lib.rs:646) routes with a clone of `neg` taken before the first run and that clone
     replaces `neg` afterwards, so one epoch accumulates history once (AT-34).
  5. Weights: `struct Weights { place: Vec<f32>, route: Vec<f32> }` moves out of `Flow` (lib.rs:294–296, 312–316)
     into the loop; `fn route_weights(place: &[f32], classes, sens) -> Vec<f32>` factors lib.rs:312–316. PERF's
     feedback updates `Weights` between epochs; this item only plumbs it.
  6. Actions after a middle loop (replacing lib.rs:376–393):
     ```text
     feasible  = best.key.0 == 0 && best.key.1 <= 0 && best.key.2 <= 0
     stationary = prices.drift() < PRICE_STATIONARY && prices.saturated().is_empty()
     if feasible && stationary         → stop(Converged)
     if feasible                        → stop(FeasibleNotStationary)       # never escalate a feasible incumbent
     if outer + 1 == n_outer            → stop(OuterBudget)
     next = escalate_blamed(variants, assignment, blame(best), &tried) else stop(EscalationExhausted)
     ```
  7. `cellgen.rs` adds
     ```rust
     /// Next assignment on an infeasible stall: the most-blamed cell with an untried
     /// alternative moves first (ties: larger pin spread, then lower index); falls back to
     /// `escalate` order; never returns an assignment in `tried`.
     pub fn escalate_blamed(variants: &[gp::VariantSpace], current: &[u16], blame: &[u32], tried: &BTreeSet<Vec<u16>>) -> Option<Vec<u16>>
     ```
     Order: cells sorted by `(Reverse(blame[i]), Reverse(pin_spread(&variants[i])), i)` (`pin_spread`,
     cellgen.rs:420); the first cell `i` with an alternative `a > current[i]` such that `current` with `[i] = a` is not
     in `tried` wins; if none, fall back to `escalate` (cellgen.rs:401–416) repeatedly until an untried assignment or
     `None`. When PLC-03's `Flow.locks` exists (plan-04 PLC-03 step 3), cells that are non-first members of a lock set
     are skipped and the result passes through `locks.unify` (plan-04 notes this to FLOW).
     `fn blame(&self, e: &Epoch) -> Vec<u32>` (lib.rs): +1 per cell id in `violating_ids` of every placement hard or
     budget batch with residual > 0 on `e.layout`; +1 per `verify::drc(shapes, &[], pdk)` finding whose `(x, y)` lies in
     the cell's placed bbox (one DRC call per escalation, not per epoch).
  8. `RunStats` gains `stop: StopReason` (`Converged | FeasibleNotStationary | OuterBudget | EscalationExhausted |
     WallBudget | Patience`) and `Config.max_wall: Option<std::time::Duration>` checked between epochs.
- Tests (`frontend/library/src/lib.rs`, `frontend/library/src/cellgen.rs`):
  - `schedule_is_cold_first_then_periodic` (pure fn `epoch_kind(k, current_incumbent, cold_every) -> Kind`).
  - `feasible_incumbent_is_never_escalated` (pure fn `next_action(feasible, stationary, last_outer) -> Action`).
  - `escalate_blamed_moves_the_most_blamed_cell_first` and `escalate_blamed_never_repeats`.
  - `wall_budget_stops_with_its_reason`: `max_wall = Some(0 s)` → exactly one epoch, `stop == WallBudget`.
  - `warm_epoch_starts_from_the_incumbent`: with `COLD_EVERY = ∞` and a stub dp schedule of 0 moves, the warm epoch's
    layout equals the incumbent's (x, y, orient, variant).
- Acceptance: T10 on `bench local` over seeds 1–5 [measure]; `RunStats.stop` printed per circuit.
- Risks / notes: warm epochs reduce diversity; the multi-start threads (lib.rs:185–188) and periodic cold epochs keep
  it. Replica exchange (PL-20) is not planned (no measurement asks for it yet).

### FLOW-09 Runtime and determinism: hoist pure work, per-stage timers, seed derivation

Field report: FR-2 (45–60 min per iteration in "extracting feedback", no early stop). That was the old packaged build's engine (`backend/engine/src/block.rs:322` at `e8bc59e`, absent from m0). On m0 (CLI, default `Config`: `feedback_iters` 200, `outer_iters` 4, `starts` 3) the runs stop on convergence: strongarm 2 min 32 s, ptat_bias 6 min 46 s, pwm_driver 6 min 26 s, wta 5 min 13 s, rescale 2 min 04 s. This item's per-stage timers are what turns the next runtime report into a number per stage.

- Priority: **P1**. Effort: **M**. Depends on: FLOW-02.
- Why: AF-22, AP-19, AT-12 ("no per-stage timing"), SURV-24 (simulation and PEX dominate cost; count them). D13 in
  docs/API-WISH.md (annotator once per run).
- Current: §1.5 R1–R2; `RunStats` has no timing (lib.rs:104–124).
- Change:
  1. Split `solve` (lib.rs:249–458) in three, so everything a start does not change is built once:
     ```rust
     /// Fixed for the run: annotate + folds + `CellSpace::new` + router config + `seed_assignment` (lib.rs:262–338).
     struct Topology<'a> { flow: Flow<'a>, assignment0: Vec<u16>, distinct: bool }
     fn topology<'a>(nl: &'a Netlist, injected: &Macros, pdk: &'a Pdk, cfg: &'a Config, bias: &Bias,
                     perf_rows: &[analog::routing::PerformanceBudget], merge_distinct_gates: bool,
                     stack: &'static Stack) -> Topology<'a>;
     /// One start: the search loop (lib.rs:340–394) on a shared `&Topology`; owns its `Prices` and `Negotiation`.
     fn search(t: &Topology, cfg: &Config, seed: u64) -> (Epoch, RunStats);
     /// The winner only: realize, rings, fill, metadata, `Solution` (lib.rs:396–457). Consumes the topology, so
     /// `flow.problem.placement` moves into `Solution.placement` as today (lib.rs:440) with no `Clone` of
     /// `Requirements` (not `Clone`: boxed trait objects, docs/API-WISH.md D13).
     fn finish(t: Topology, best: Epoch, stats: RunStats, bias: &Bias) -> Solution;
     ```
     `run` (lib.rs:156–190): build `merged = topology(.., true, ..)`; build `apart` only if `merged.distinct`; spawn
     `cfg.starts` threads in `std::thread::scope`, each calling `search` on every built topology (`Flow` is read-only in
     `epoch`; `RuleBatch` values are `Send + Sync`, kernel/analog/src/rule.rs:135 and the `impl<R: Rule + Send + Sync>`
     at rule.rs:210); reduce all `(topology index, Epoch, RunStats)` by `key_lt` (ties to the earliest start, as today);
     `finish` only the winner's topology. Side effect: fill, metadata and `realize` run once per run instead of once
     per start (today `solve` runs them per start, lib.rs:396–457).
  2. `run` annotates once up front — `let ann = annotation(pdk, &cfg.annotation, stack); let base = annotate(&netlist, &ann);`
     — before `bias` (lib.rs:165); `performance_rows` (lib.rs:196–208, today its own `annotation` + `annotate`) and
     PERF-09's rail classification read `&base.net_classes`. `topology` still annotates its own `Problem` (it is
     mutated by `CellSpace::new`, lib.rs:275, and `Problem` is not `Clone`), so a run annotates 1 + (1 or 2) times
     instead of 1 + 2·`starts` (R1). This is the "annotate once" step plan-07 PERF-09 defers to.
  3. `annotation()` leaks the `Stack` once per `run`, not per call (lib.rs:511–513): compute `elaborate::stack(pdk)`
     in `run` and pass the `&'static` in.
  4. `RunStats.stage_ms: [u64; 9]` for `gp, dp, rings, gr, dr, diodes, signoff, metadata, perf`, accumulated with
     `Instant` around each call in `Flow::epoch`/`score_perf`.
  5. dp seed = `seed ^ 0xD1B5_4A32_D192_ED03` [policy: any fixed odd constant] so gp and dp streams differ (AP-19).
- Tests: `hoisting_prices_each_alternative_once` (a `#[cfg(test)]` `AtomicU32` counter in `cellgen::price`: calls ==
  Σ alternatives × topologies with `starts = 3`, i.e. the same count as with `starts = 1`); `same_seed_same_gds_bytes` (sky130, chain4 and ota, two runs →
  identical `gds::emit` bytes); `stage_times_are_reported` (Σ `stage_ms` > 0).
- Acceptance: T8, T9; after steps 1–3 (pure refactor) GDS bytes are identical to the pre-change binary for seeds 1–3 on
  all fixtures; step 5 changes results and is re-baselined separately.
- Risks / notes: bounding ngspice concurrency is PERF's (AF-16). `Flow` must be `Sync`: if a field is not (check
  `DetailedRouter` and the `Box<dyn RuleBatch>` vectors at compile time), the fallback is one `topology` per thread
  (still hoisting the per-start `annotation()` leak and `performance_rows` re-annotation).

### FLOW-10 Fill inside the certificate: DRC-gated fill, post-fill signoff
- Priority: **P1**. Effort: **S**. Depends on: FLOW-09 step 1 (`finish` runs fill once, on the winner only).
- Why: AF-13, NOTES-07 (evidence freshness), H01-40 (every generated shape, chip art included, must follow the design
  rules, hastings.txt L8222–8224, PDF p.147; CMP dishing cure is automatically generated dummy fill, L5783–5792,
  PDF p.105). The certificate (`MetadataReport`, `RunStats.drc_hard`, the winner's `LexKey`) must describe the
  geometry that ships; today it describes the pre-fill geometry.
- Current: fill runs once per `solve` on its winner (lib.rs:410–420), gates each layer on ERC only (fill.rs:58, 66),
  and the reported metadata and key are pre-fill (lib.rs:421–439). On today's decks and fixtures fill is a no-op
  (sky130 states only a 700 µm-window maximum, `density_cmp`, GP/pdks/sky130.deck:500–503; gf180mcu states metal
  minima only as `global_density`, GP/pdks/gf180mcu.deck:395, 419, which `density_rules` does not read, pdk.rs:619–637;
  ihp_sg13g2's windowed minima use 800 µm windows, GP/pdks/ihp_sg13g2.deck:269, 395, wider than every local block,
  largest 2326.9 µm², audit-06 G.3). So this item is insurance for the first deck/block where fill is non-empty, and
  is kept small.
- Change:
  1. `frontend/library/src/fill.rs:58–71`: `base = (verify::drc(drawn, &[], pdk).len(), verify::erc(drawn, &[], pdk).len())`;
     a layer's tiles are dropped when either count on `drawn + out + tiles` exceeds its base (today ERC only).
  2. `finish` (FLOW-09 step 1), after fill (this is the step plan-07 PERF-22 calls "FLOW-10 step 4"): when
     `fill(..)` returned `Some`, recompute `(signoff, caps) = signoff_shapes(..)` (lib.rs:1345–1346, the call
     `Flow::epoch` makes at lib.rs:670) on the final shapes (the `geometry::collect` + rings + fill set that
     `library::signoff` reads, lib.rs:1289) and replace the winner's `RunStats.drc_hard`; otherwise keep the winning
     epoch's `signoff` and `caps` (lib.rs:547). `Solution` gains `pub caps: verify::CapMatrix` (the final matrix, read
     by PERF-22's post-fill re-simulation) and `MetadataReport` gains `post_fill: bool` (true when fill was kept).
     Budgets (`metadata::build`) are not recomputed: fill adds no route and moves no cell, so the rows are unchanged.
- Tests:
  - `frontend/library/src/fill.rs` `fill_that_adds_a_drc_finding_is_dropped`: a synthetic `density` floor rule
    (0.30, 50 µm window) on the gf180 test block fill.rs already uses, plus a met1 wire at exactly the met1 spacing
    from a free tile column → no fill kept on met1 when a tile would violate spacing; `verify::drc` count unchanged.
  - `frontend/library/src/lib.rs` `winner_signoff_matches_its_certificate`: pair.spice on gf180mcu →
    `library::signoff(&sol, &pdk).hard_violations.len() == sol.stats.drc_hard`.
- Acceptance: T7 on all fixtures (holds trivially today; the test guards the first non-empty fill).
- Risks / notes: one extra signoff on the winner only when fill is non-empty.

### FLOW-11 Hierarchical bottom-up flow: sub-circuits solved once, placed and routed as block cells
- Priority: **P1**. Effort: **L**. Depends on: FLOW-07, FLOW-09, FLOW-02; RTE (per-run metal range, optional); EXT
  (SameTemplate symmetry between instances, optional).
- Why: AF-20, AA-08 (hierarchy part), G13 (audit-01), H15-05 (matched arrays in one cell; about 50–100 components per
  cell, hastings.txt L46414–46431, PDF p.783), H15-07 (identical blocks as mirror images of one cell,
  hastings.txt L47136–47145, PDF p.794). Analog circuits "seldom contain more than 100 cells per hierarchical
  level" (balasa_graeb_survey.txt L1604–1605), while dp's cost per call is O(n·(P + R + n)) (audit-04 §1.10). The
  survey also notes that modern placers optimise hierarchical sub-circuits simultaneously instead of bottom-up
  "because the optimal placement of a subcircuit may not lead to the globally optimal placement"
  (balasa_graeb_survey.txt L3068–3075, p.77): so bottom-up is an opt-in scale tool, compared against flat on the same
  fixture, not the default.
- Current: `Netlist` is flat (§1.2 I5); `Problem::blocks` are used only for axes and pair measurements (lib.rs:580–583,
  779, 849); macroMaster hierarchy (`CompBuilder::instantiate_comp`, kernel/macroMaster/src/lib.rs:328–338) never
  reaches the searched flow.
- Change:
  1. `Config.hierarchy: Hierarchy` with `pub enum Hierarchy { Flat, BottomUp { min_devices: usize }, Auto }` —
     default `Flat`; `Auto` = `BottomUp { min_devices: 2 }` when the flattened top has > 100 cells [policy: the
     L1604–1605 bound], else `Flat`.
  2. New `frontend/library/src/hier.rs`:
     ```rust
     pub(crate) struct Block {
         pub subckt: String,
         pub mac: Macro,                    // origin at its bbox corner; pins named by formal port
         pub ports: Vec<String>,            // formal order
         pub ref_cards: Vec<verify::reference::RefDeviceIn>,   // child net names
         pub metadata: metadata::MetadataReport,
         pub stats: RunStats,
     }
     /// Post-order over sub-circuit definitions; one solve per eligible definition.
     pub(crate) fn solve_blocks(nl: &Netlist, pdk: &Pdk, cfg: &Config, bias: &Bias) -> Vec<Block>;
     ```
  3. Algorithm:
     1. Eligible definitions: sub-circuits (not the top) whose flattened device count ≥ `min_devices`.
     2. For each eligible definition `S` in post-order: pick its first instance `i0` in `nl.insts`; local netlist
        `N_S` = devices with `i0` on their `device_inst` chain, names with the `path/` prefix stripped, formal ports as
        `N_S.ports`; nested eligible instances inside `S` are already `Block`s and enter `N_S`'s solve as block cells
        (step 5).
     3. Bias for `N_S`: the top operating point restricted to those devices (power, terminal currents, headroom, gm,
        by device index); `performance: None` (specs are top-level).
     4. Solve `N_S` with the ordinary `solve` path (flat at that level) → `Solution`. `Block.mac`: all shapes of
        `sol.geometry()` (cells, rings, routes; no fill) translated to the bbox origin; one pin per formal port = the
        widest routed rect of that net on the highest routing layer it uses, else its first placed device pin;
        `units`/`dummies` empty. `ref_cards` = `signoff_inputs(&sol, pdk).2.devices` (lib.rs:1318–1331, which calls the
        private `labels_and_reference`, lib.rs:1351–1370; dummy cards included).
     5. In the parent: each instance of `S` becomes one cell with a single-alternative `VariantSpace`, `fixed =
        false`, power = Σ member power; `cell_of` maps every member device to it; unitizations and guard-ring requests
        touching member devices are removed (drawn in the child). Rules between devices of two different block cells
        survive retargeting as cell-level rules (identical-instance symmetry from EXT, H15-07); rules inside one block
        collapse to self-pairs, as merged cells do today.
     6. Parent LVS reference = `cellgen::reference(non-block devices)` + each block's `ref_cards` with net names mapped:
        formal port → the instance's actual parent net name; internal `n` → `path/n` (the FLOW-07 flattening name).
     7. Parent routing sees block metal as cell metal. When RTE exposes a metal range, child solves route on
        `metals[..k]` and the parent on all (OQ-6).
     8. `Solution.blocks: Vec<(String, metadata::MetadataReport)>`; `RunStats.block_solves: u32`.
  4. New fixture `benchmarks/fixtures/two_ota.spice`: `.subckt ota …` (ota.spice body) + top with `X1`, `X2` sharing
     VDD/VSS and a bias net.
- Tests (`frontend/library/tests/hier_flow.rs`):
  - `bottom_up_solves_each_definition_once_and_signs_off_clean` (sky130, two_ota, `BottomUp { min_devices: 5 }`):
    `block_solves == 1`; no `lvs/` row; no `drc/` row.
  - `flat_and_bottom_up_both_run`: record both `LexKey`s in the test output (no assertion on which is better).
- Acceptance: T12; wall time of `BottomUp` vs `Flat` on two_ota reported [measure].
- Risks / notes: block pins on the child's top routing layer may be unreachable if RTE's landing stays on layers 0–1
  (backend/gr/src/lib.rs:586) and the child used higher metals; with today's 2-layer sky130 stack (AT-01) both levels
  use met1–met2. Mirrored placement of identical blocks depends on PLC's orientation policy.

### FLOW-12 CLI and deliverables: flags, built-in PDKs, output directory, labelled GDS

Field report: FR-7 (export half) and FR-8. The m0 CLI accepts the four built-in PDKs (FLOW-04) but writes no file; `fix-export` (`73087b6`) has `philis run -o`, the labelled GDS and `--version`, landed by FLOW-17. The simulatable post-layout netlist is PERF-30.

- Priority: **P1**. Effort: **M**. Depends on: FLOW-04 (`Pdk::builtin`), FLOW-07 (ports); EXT-26 (`annotator::sidecar::parse`) only for `--constraints`.
- Why: AF-11, AF-18, H01-30 (a layout pin is shape + layer + name; GDSII carries names as text, hastings.txt
  L6880–6885, PDF p.126, and L7036–7039, PDF p.129), H01-31 (GDSII limits: structure names ≤ 32 chars, L7036–7073), AV-12
  (interface contracts ignored, benchmarks/src/bench.rs:13), PERF §0.2 row 3.
- Current: §1.4 O1–O2.
- Change:
  1. `frontend/library/src/gds.rs`:
     ```rust
     pub struct Label { pub text: String, pub layer: (u16, u16), pub x: i32, pub y: i32 }
     /// Flat `TOP` of BOUNDARY rects plus one TEXT per label. `Err` names every shape
     /// layer with no stream number (a derived layer maps to (0, 0), pdk.rs:321–330).
     pub fn emit_labeled(shapes: &[Shape], labels: &[Label], layer_gds: &[(u16, u16)]) -> Result<Vec<u8>, String>;
     pub fn emit(shapes: &[Shape], layer_gds: &[(u16, u16)]) -> Result<Vec<u8>, String>;   // = emit_labeled(.., &[], ..)
     ```
     TEXT element: `TEXT 0x0C00`, `LAYER 0x0D02`, `TEXTTYPE 0x1602`, `XY 0x1003` (one point), `STRING 0x1906`
     (even-padded), `ENDEL 0x1100`. Callers (bench, examples, tests) handle the `Result`.
  2. `frontend/library/src/lib.rs`: `pub fn gds(sol: &Solution, pdk: &Pdk) -> Result<Vec<u8>, String>` = shapes of
     `sol.geometry()` + one `Label` per `labeled_pins` entry (lib.rs:1376–1410) on the label layer
     `verify::geom::label_layer(&pdk.deck, pin.layer)` mapped through `pdk.layer_gds()`.
  3. `frontend/library/src/lib.rs`: `pub struct Interface { pub die_nm: Option<(i32, i32)>, pub pins: Vec<IoPin> }`,
     `pub struct IoPin { pub net: String, pub side: Side, pub frac: f32, pub width_nm: i32, pub layer: String }`,
     `pub fn Interface::from_json(text: &str) -> Result<Interface, String>` (format of
     benchmarks/fixtures/ota_constrained.interface.json, whose keys are `die.w`/`die.h` and per pin `net`, `side`, `frac`,
     `width`, `layer`: the parser maps `die` → `die_nm`, `width` → `width_nm`); `Config.interface: Option<Interface>`; every `IoPin.net` must
     be a netlist port (FLOW-07) else `FlowError::Interface`. Consumers: PLC (die outline, boundary), RTE (pin shapes).
  4. `frontend/cli/src/main.rs` (hand-rolled parser, no new dependency; `serde_json` from the lockfile for JSON inputs):
     ```text
     philis [run] <netlist.sp> (--pdk <sky130|gf180mcu|ihp_sg13g2|generic_finfet|path.json> | <deck.json>)
            [--out DIR] [--seed N] [--starts N] [--iters N] [--outer N] [--max-wall SECS]
            [--size spice|per-finger] [--top NAME] [--hierarchy flat|auto|bottom-up:N]
            [--op-lib PATH [--corner tt] [--vdd V] [--temp C] [--testbench FILE]]
            [--perf SPECS.json] [--constraints FILE.json] [--interface FILE.json]
     philis emit <netlist.sp> --pdk … --out-rs FILE.rs
     ```
     `--constraints F` → `annotator::sidecar::parse(&text, &netlist)` (EXT-26) → `cfg.annotation`; each returned
     `Diagnostic` is one line of `report.txt` (unknown constraint names are reported, never dropped). Until EXT-26
     lands the flag exits 2 with "--constraints needs EXT-26".
     Default `--out` = `./philis_out/<netlist stem>/`. Writes `layout.gds` (labelled) and `report.txt` =
     `MetadataReport` `Display` + one line per signoff row + `RunStats` (stop reason, stage times, warnings). Exit code
     0 clean, 1 hard violations, 2 input/flow error.
- Tests:
  - `frontend/library/src/gds.rs` `text_records_are_well_formed`: emit one rect + one label; walk records; the TEXT
    element's STRING equals the label and its XY is the point.
  - `unmapped_layer_is_an_error`: a shape on a layer id past the table → `Err`.
  - New `frontend/cli/tests/cli.rs` `run_writes_a_labelled_gds_and_a_report`: `env!("CARGO_BIN_EXE_philis")` on
    `benchmarks/fixtures/pair.spice --pdk sky130 --iters 2 --starts 1 --out <tmp>` → exit code ∈ {0, 1}; both files
    exist; the GDS has a TEXT record for every port net of pair.spice.
- Acceptance: T11 (deliverables half); PERF's foundry cross-check consumes the labelled GDS.
- Risks / notes: hierarchical SREF output (H01-29) is not planned: flat labelled GDS satisfies magic/netgen/KLayout.

### FLOW-13 emit and macroMaster round trip

Status: step 0 done in M1 (`add9e94`; input merge `5a21f2b`). As built: the cause was detailed routing, not macroMaster; the fix is the card's step 2 (access jogs prefer unowned nodes; PathFinder history on foreign nodes a jog touches), because step 1 regressed dac4. Steps 1+ stay in M2.

Carried from M0 (step 0, M1): `frontend/library/tests/hier_elaborate.rs::hierarchical_composition_elaborates_with_a_correct_schematic` is red on m0 at hier_elaborate.rs:216 with 8 × `lvs/lvs.unpaired_device` and 17 × `lvs/lvs.unpaired_net` (FLOW-14 replaced its assertion-free check). It drives `CompBuilder::instantiate_comp` through `library::elaborate`, the macroMaster path this item owns, and no M0 item fixed it. Step 0: find why the elaborated hierarchical layout does not pair with `BuiltComp::netlist` and fix it with the test unchanged.
- Priority: **P1**. Effort: **M**. Depends on: FLOW-01, FLOW-07.
- Why: AF-08, AF-19, AF-21, SUBSTRATE3 B P2/P3 (docs/SUBSTRATE3.md). Hastings Table 13.2 rule 5: matched MOS need equal
  orientations (hastings.txt L42191–42210, PDF p.709); the signed orientation Φ is the check (§13.2.6 Eq 13.61,
  L42162–42187; orientation mechanisms §13.2.1, L40966–41017; H13-25). Hastings §3.1.4: use only the eight D4 transforms (L6798–6823).
- Current: §1.4 O3–O5.
- Change:
  1. `Solution` gains `pub devices_of: Vec<Vec<DeviceId>>` (from `CellSpace.devices_of`, lib.rs:1115–1116).
  2. `emit.rs`: `pub fn emit_solution(sol: &Solution, pdk: &Pdk) -> Result<GenIr, EmitError>` uses
     `sol.devices_of` (no re-enumeration), `sol.folds` and `Device::mos_size()` (per-device fingers incl. `m`), and
     `sol.layout.orient[i]` → new `IrInst.orient: Orient`. The old `emit(netlist, layout, pdk, cfg)` stays for hand
     layouts and returns `Err(Unsupported("layout does not match the cell table"))` when
     `layout.x.len() != cells.devices_of.len()` (no panic). `philis emit` calls `emit_solution`.
  3. `GenIr.ports` = `netlist.ports` names (FLOW-07); legacy netlists without ports keep today's every-net behaviour.
  4. `to_rust` identifiers: `fn ident(name: &str) -> String`: lowercase; every char outside `[a-z0-9_]` → `_`; prefix
     `n_` if empty or starting with a digit; suffix `_` if a Rust keyword (2021 strict and reserved keywords, plus `gen`,
     which is reserved only from edition 2024 — the workspace crates are edition 2021 — kept for forward compatibility:
     `as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move
     mut pub ref return self Self static struct super trait true type unsafe use where while abstract become box do
     final macro override priv try typeof unsized virtual yield gen`); collisions get `_2`, `_3`, …; the original string
     stays in `.port("…")`.
  5. `kernel/macroMaster/src/lib.rs`:
     - `Instance.transform` also maps `mac.units` (x, y, and `phi` sign for the flipped axis) and leaves `dummies`
       (owner-relative) unchanged.
     - `place_mirrored` returns `Err(GenError::Orientation(String))` when both instances carry units and the mirrored
       Σφ of `inst` differs from the reference's Σφ (the odd-finger flip of O5).
     - New `pub fn place_copy(&mut self, inst, reference: &Instance, gap: i32) -> Result<Instance, GenError>`:
       translation only (`ToTheRight` by `gap`), for matched partners ("perfect" symmetry).
     - `place` rejects a duplicate instance name (`GenError::DuplicateName(String)`) and an off-grid bbox
       (`GenError::OffGrid`); `build_with` rejects an edge endpoint that is neither an io port nor `inst.pin` of a
       placed instance (`GenError::UnknownTerminal(String)`). `GenError` stays `PartialEq + Eq`.
     - `frontend/library/tests/ota_cross_pdk.rs` switches its `nf = 1` partners from `place_mirrored` to `place_copy`.
- Tests:
  - `kernel/macroMaster/src/tests.rs`: `duplicate_names_are_rejected`, `an_unknown_terminal_is_rejected`,
    `mirroring_an_odd_finger_device_is_refused`, `copy_keeps_orientation`.
  - `frontend/library/tests/emit_roundtrip.rs` `run_emit_elaborate_signs_off` (sky130, chain4 and ota via `library::run`
    with `feedback_iters: 2`): `emit_solution` → `elaborate_ir` → `Elaborated::signoff` has no `lvs/` row.
  - `emit.rs` `identifiers_compile_shaped`: nets `in`, `0`, `a<1>`, `vout-`, `VDD`, `vdd` → identifiers
    `in_`, `n_0`, `a_1_`, `vout_`, `vdd`, `vdd_2`, each matching `^[a-z_][a-z0-9_]*$` and not a keyword.
- Acceptance: T11 (round-trip half).
- Risks / notes: variants are still re-derived as the smallest in `elaborate_ir` (documented); route replay (SUBSTRATE3
  P2) is not planned.

### FLOW-14 Test infrastructure and CI

Status: done in M0 (`89a1740`). Deviation: the tool gates live once in `library::tools::{present_or_skip, tool_or_skip, sky130_models}` (`#[doc(hidden)]`, frontend/library/src/lib.rs:33), not per test file; `hier_elaborate` now asserts and is red (8 unpaired devices, 17 unpaired nets), carried to M1 under FLOW-13 (00-MASTER-PLAN Status).

- Priority: **P0** (the red test and CI gate), **P1** (the rest). Effort: **S**. Depends on: none.
- Why: AF-26, AC-19, AV-15 (red test visibility), CRATES #10d, T13.
- Current: §1.5 R3–R4.
- Change:
  1. `.github/workflows/ci.yml`:
     - `test`: ubuntu-latest, stable Rust, `cargo build --workspace --locked`, `cargo test --workspace --locked`.
     - `release`: `cargo test --release --locked -p cells --test cell_selfcheck` and
       `cargo test --release --locked -p library --test ota_cross_pdk`.
     - `fmt`: `cargo fmt --all --check`, after one dedicated formatting commit of `backend/gr` and `backend/dr` (the
       only crates CRATES #10d names). That commit is made only after the owner has committed the current large
       uncommitted working-tree changes (git status of 2026-09-28), so formatting never mixes with their diff; until
       then the `fmt` job is `continue-on-error: true`.
     - `nightly` (schedule): installs `ngspice` (apt) and sky130A via volare at the flake's hash
       `1341f54f5ce0c4955326297f235e4ace1eb6d419` (flake.nix:112, shellHook) and runs
       `PHILIS_REQUIRE_TOOLS=1 cargo test --release --workspace -- --include-ignored`.
       **Amended (M0 review panel):** the job runs `cargo test --release --workspace` (the tool tests are not
       `#[ignore]`d; they read `PHILIS_REQUIRE_TOOLS`) plus `-p benchmark --test signoff_fixtures -- --ignored` by
       name, with `timeout-minutes: 90`. A workspace-wide `--include-ignored` also ran EXT-01's placeholders that are
       `#[ignore]`d until later EXT items (`coverage_is_total` is `unimplemented!`; `twelve_thousand_devices` did not
       finish in 13.5 min), so the job was red or hung whatever the tools did, and a missing-tool panic could not be
       told apart from them.
  2. Loud skips: `fn tool_or_skip(bin: &str) -> bool` in each tool-dependent test file (frontend/library/tests/
     perf_postlayout.rs and any ngspice test): if `PHILIS_REQUIRE_TOOLS=1` and the binary is absent → `panic!`, else
     `eprintln!` + return.
  3. `frontend/library/tests/flow_smoke.rs:24–30`: `run` must return `Ok`.
  4. `frontend/library/tests/hier_elaborate.rs:206–220`: assert no `lvs` row.
  5. Delete `frontend/library/tests/tmp_deck_drc.rs` and the stray `frontend/library/bsim4v5.out` plus
     `./bsim4v5.out` (repo root listing) once PERF's scoped temp dirs land.
  6. The dac4 red test (the ERC baseline assertion at `benchmarks/tests/signoff_fixtures.rs:139–143`, reached from
     `fixtures_sign_off_within_baseline`, :150–155) is not waived: REL's antenna fix closes it; until
     then CI is red on that one test, by design.
- Tests: the workflow itself; `tool_or_skip` has a unit test with a nonexistent binary name.
- Acceptance: T13.
- Risks / notes: GPurify is a public git dependency (Cargo.toml:23); CI needs network for the first build. The flake
  pins volare `1341f54f…` while every `VOL/` citation in this plan and the sidecar `_source` texts (e.g.
  pdks/sky130.json:99–100) were read from the installed `fa87f8f4…`; the nightly job installs the version the
  `_source` texts cite, or those citations are re-verified against `1341f54f…` first.

### FLOW-15 Housekeeping, stale comments, docs and dead fields
- Priority: **P2**. Effort: **S**. Depends on: the items whose results the docs describe.
- Why: AF-28, AF-31, AF-32, AF-34, AV-27, AT-36, AA-30 (doc part), AC-27 (doc part), AR-28, CRATES open-issue table (§1.6).
- Change:
  1. lib.rs:1295–1297: name the undrawable cell's members through `sol.devices_of` (AF-31).
  2. lib.rs:726, 771: accept a bare `T` pin as member 0's terminal, as `pin_currents` does (lib.rs:1047–1049) (AF-32).
  3. Stale comments: lib.rs:48–49 vs 1071/1076 (fallback power), lib.rs:960–962 (`round_up` doc), emit.rs:10–14
     (scope), emit.rs:240 ("x-overlap" filter), examples/three_stage_opamp/src/main.rs:46, 80 (`Generator` →
     `DeviceGen`/`Composition`).
  4. `kernel/visualizer/src/lib.rs:53–66`: `parse_layer_names(json)` → `layer_names(pairs: &[(String, (u16, u16))])`,
     built by callers from `pdk.layers` + `pdk.layer_gds()` (the sidecar has no top-level `layers`, AV-27);
     `polys_from_shapes` (kernel/visualizer/src/lib.rs:1188–1201) takes the same table so `Probe` colours match deck names (AF-34).
  5. `docs/CRATES.md`: replace "Open issues" with the §1.6 triage (owner per issue); add a "Stage contracts" section
     (FLOW-02 key, FLOW-03 prices, FLOW-08 schedule); fix the dr line "EM width = I / 1 mA/µm" (docs/CRATES.md:63,
     AT-36). `docs/API-WISH.md` D4: mark "not implemented; superseded by FLOW-08 blame escalation". 
     `docs/LAYOUT-FUNDAMENTALS.md` rows listed stale in audit-07 §2.14.3 (§1g #65, §1e #53, §1a #5, §1c #39, LF-10) and
     `docs/SUBSTRATE3.md` M3/M4 status updated.
  6. At M4: delete `Unitization.same_variant_required`, `SeriesParallel::RepeatedStage` (kernel/analog/src/cell.rs;
     `tap_pitch_nm` and `enclosure_complete` were deleted by GAP-05 in M1) unless a CELL/PLC item has wired them;
     the deprecated `*_moderate` scalars are MAT-07's to delete (plan-02 MAT-07 step 4); FLOW-15 only drops their
     registry rows afterwards.
- Tests: `common_node_sees_injected_macro_pins` (lib.rs tests, the injected-macro fixture of
  frontend/library/tests/injected_macro.rs) — the CommonNode for the injected pair lists its bare `S` pins.
- Acceptance: grep for `Generator`, `variant_signoff`, `EM_UA_PER_UM` in docs → 0; CRATES.md open issues each have an owner.

### FLOW-16 Fold classes per cell, not per netlist-wide size class
- Status: done in M1 (`baeda1c`; cells merge `726bc13`): MatchedSet 2/2 on each OTA fixture (was 0/2). **Regression:** its fold classes add routing overuse on the OTA trio (472 alone, 1458 merged, CrosstalkExclusion 12/12 → 0/12, WL +21.8 % vs M0; m1-report §4); carried to M2 item 0.
- Priority: **P1** (moved to M1 by the M0 close-out for step 4). Effort: **S**. Depends on: FLOW-01 (`w_finger_nm`), FLOW-05 step 4 (one R□ read in `folds`), both done in M0; MAT-03 (`diffusion_cc_row`) and CELL-10 (the merged row is drawn) for step 4.
- Why: AF-15 (audit-07; owned by no other plan: grep of plans 01–07 finds no AF-15 entry). `folds` groups every MOS of
  equal `(kind, W, L)` in the whole netlist into one fold class (cellgen.rs:647–652) and scores aspect and ABBA/chain
  parity on the class's summed row (`row = Σ fingers`, cellgen.rs:654; parity :678–682; aspect :684), so two unrelated same-size devices
  (a tail source and a bias device) are folded to suit a row that never exists. Hastings' folding and aspect rules
  (§13.3 rule 9, hastings.txt L42504–42516, per MAT-07's citation) are per matched array, i.e. per drawn cell.
- Current: `pub fn folds(netlist, pdk, gm_us) -> Vec<(u16, i32)>` (cellgen.rs:615; cellgen.rs:630 on m0); callers lib.rs:274
  (lib.rs:376 on m0, passing `bias.gm_us`), cellgen.rs:52 (`enumerate`), tests cellgen.rs:1176, 1599–1600 (on m0 1259,
  1683–1684, 1789, 1802). Since FLOW-01 draws the simulated size, the OTA's XM1/XM2 (`W=10u nf=2`) fold to k = 1, two
  5 µm fingers each, drawn AABB, and the bench `CommonCentroid` rows of ota, ota_constrained and tt_ota read 0/3
  (max use 2.195, Θ 3.584; they were 3/3 when 4×5 µm per device interleaved); `folds` chooses k from parity and aspect
  only (FLOW-01 merge record `5297b98`, m0-report §5).
- Change (`frontend/library/src/cellgen.rs`):
  1. Signature `pub fn folds(netlist: &Netlist, pdk: &Pdk, gm_us: &[Option<f64>], cells: &[Vec<DeviceId>]) -> Vec<(u16, i32)>`.
     The class of device `i` = the members of the first `cells` entry containing `i` that are MOS with equal
     `(kind, w_finger_nm, l_nm)` to `i`; a device in no entry is its own class. Everything inside the loop (parity,
     `k_gate`, aspect score, `P2P_SHARE` cap) is unchanged; only the class filter at cellgen.rs:647–652 changes.
  2. Callers: lib.rs:274 passes `&problem.constraints.unitization.iter().map(|u| u.devices.clone()).collect::<Vec<_>>()`
     (the entries `CellSpace::new` merges into cells, kernel/analog/src/constraints.rs:8–11, cell.rs:41–42);
     `enumerate` (cellgen.rs:52) passes the same from its `constraints` argument; the two tests pass `&[]`.
  3. Matched devices still fold alike: they share one unitization entry by construction (the annotator emits one
     `Unitization` per recognised non-glue block, backend/annotator/src/constraints.rs:45–54).
  4. (M0 follow-up, measured above.) Within a class whose devices are in one `CentroidGroup`, keep only k whose
     per-member finger counts admit a common-centroid row (`pattern::diffusion_cc_row(.., Outer::Drain).is_some()`,
     MAT-03; for XM1/XM2 [2,2] → A BB A), before the aspect score; if no k qualifies, today's choice stands and the
     `CommonCentroid` row stays violated (reported, not hidden).
- Tests (`cellgen.rs` tests, sky130):
  - `unrelated_same_size_devices_fold_independently`: netlist `XA`, `XB` (`nfet_01v8 W=2u L=0.5u`, a mirror sharing
    gate and source) and `XC` (`W=2u L=0.5u nf=16`, unrelated nets); `cells = [[A, B]]` →
    `folds(..)[A] == folds(nl_ab, .., &[[A, B]])[A]` where `nl_ab` has only `XA`, `XB`, and
    `folds(..)[C] == folds(nl_c, .., &[])[0]` with `nl_c` = `XC` alone.
  - The existing fold tests (cellgen.rs:1176, 1599–1600) pass with `&[]`.
- Acceptance: `bench local` LVS verdicts unchanged; per-fixture footprint reported before/after [measure]; the
  `CommonCentroid` rows of ota, ota_constrained and tt_ota read 3/3 satisfied again (0/3 on m0).
- Risks / notes: `P2P_SHARE = 0.55` measured on one deck (cellgen.rs:593–597) is not revisited here (AF-15's third
  point; no second deck measurement exists).

### FLOW-17 Land `fix-export` on m0: GPurify past `bde681c`, the re-vendored sky130 deck, labelled GDS and `philis run` (added at the M0 close-out)

Status: done in M1: `fix-export` (GPurify `6341f18`, re-vendored decks whose LVS MOS references carry a 4th bulk terminal, labelled GDS, `philis run`) is on `m1a` via `2a09c30`. dac4 signs off ERC 0 at `feedback_iters = 1`. pwm_driver through the CLI signs off CLEAN; **strongarm `lvs.parameter_mismatch` is unmeasured** (no strongarm layout fixture); carried to M2 item 0.

Field report: FR-3 (false LVS), FR-8 (packaging), and the export half of FR-7.

- Priority: P0. Effort: M. Depends on: FLOW-04 (vendored decks), FLOW-05 (the `# PHILIS:` edits). Overlaps FLOW-12
  (`--out`, labelled GDS, `--version`), whose remaining steps (`--interface`, `--constraints`, `report.txt`) stay
  there.
- Why:
  - FR-3 and FR-02 (field report 02 on `main`, `c3a1303`) came from the packaged build: Philis `e8bc59e` with GPurify
    `e3c8eb2`. The "device class N has a in layout vs b in reference" text exists only in GPurify `e3c8eb2`
    (`crates/lvs/src/gpu_compare.rs:397`) and "extracting feedback" only in Philis `e8bc59e`
    (`backend/engine/src/block.rs:322`). Neither string is in m0 or its GPurify `4ef439d`.
  - On m0 the CLI still reports LVS parameter mismatches on FET-only field-report circuits (measured below), and
    GPurify fixes after `4ef439d` are not on m0: the local GPurify history has `19e08b5` (capm.4/cap2m.4 judge only
    vias on the plate), `ef2004f` (X cards calling a deck model, bare W/L in µm), `a429763` (Euclidean min width at
    concave corners), `fd5672e` (n-wells and isolated p-wells are nets; MOS has a bulk), `bde681c` (antenna: a gate
    joins the topmost conductor it overlaps).
  - Branch `fix-export` (`73087b6`, based on `eba9954`, not merged into m0) already bumps GPurify to `bde681c` and adds
    `gds::emit` with a named top cell and TEXT labels, `Pdk::label_gds`, `Pdk::reference_spice`,
    `library::export_gds`/`reference_spice`, and `philis run <sp> <deck> -o DIR --seed --max-iters --starts` with a
    `--version` that carries the git rev. The 2026-10-01 handoff note on `main` records for it: full suite green with
    dac4's ERC fixed by GPurify, strongarm klayout 0 / magic 0 / netgen MATCH, and a pending bump to GPurify `6341f18`
    ("LVS pairs symmetric devices by params"). `6341f18` is not in the local cargo checkout, so that claim is
    unverified here.
- Current (m0, measured with the m0 CLI patched to print each hard row, `sky130`, default `Config`; not committed):
  - strongarm (9 FETs): 4 × `lvs/lvs.parameter_mismatch` (margins 751, 923, 3000, 11977), 2 min 32 s.
  - pwm_driver (34 FETs): the bench (seed 1) reads LVS MISMATCH with 16 × `lvs/lvs.parameter_mismatch` and DRC 0.
  - ptat_bias (FR-3's circuit): CLEAN (DRC 0, ERC 0, LVS clean), 6 min 46 s. FR-3 itself does not reproduce.
  - `Cargo.lock` pins GPurify `4ef439d`; `pdks/decks/*.deck` line 1 names it and `vendored_decks_name_their_origin`
    (backend/verify/src/pdk.rs:1387) compares it with `GPURIFY_REV` from `backend/verify/build.rs`. Between `4ef439d`
    and `bde681c` only `pdks/sky130.deck` changes (65 diff lines); the other three decks are identical.
  - The m0 CLI (frontend/cli/src/main.rs) writes nothing to disk and has no `run`, `-o`, `--seed`, `--max-iters` or
    `--version`; it does accept the four built-in PDK names (FLOW-04).
- Change:
  1. Merge `fix-export` into m0. Expected conflicts: `frontend/cli/src/main.rs` (m0: built-in PDK names, CLEAN only
     with no LVS-unverified device, the coverage print; keep both), `benchmarks/src/bench.rs`, `backend/verify/src/lib.rs`,
     `backend/verify/src/pdk.rs`, `frontend/library/src/lib.rs`, `Cargo.lock`.
  2. Bump GPurify to the newest rev that contains both `bde681c` and `6341f18` (`cargo update -p gpurify --precise
     <rev>`), and record the rev in the merge commit.
  3. Re-vendor `pdks/decks/sky130.deck` from that rev's `pdks/sky130.deck`, line 1 `# vendored from GPurify <full
     hash> …`, and re-apply the five `# PHILIS:` edits of m0 (sky130.deck:461 met3 antenna thickness, :511 LU.2/LU.2.1/
     LU.3, :615/:618/:621 the `pex rbody_*` sheets). Re-vendor the other three decks (line 1 only, unless their text
     changed).
  4. MOS references are already 4-terminal (`cellgen::reference` emits D G S B, frontend/library/src/cellgen.rs:888–889);
     the `backend/verify/src/lib.rs` unit tests that build 3-terminal `RefDeviceIn` take the bulk as `fix-export` does.
- Tests:
  - `vendored_decks_name_their_origin` and `generators_read_exactly_the_required_keys` pass on the new rev.
  - `signoff_fixtures`: every fixture at its M0 baseline or better, named row by row; if dac4's
    `erc/ar.met2.1:gate` clears, M0 exit criterion 2 is met by this item and RTE-06's carried acceptance is re-read
    against it.
  - New `benchmarks/tests/signoff_fixtures.rs` case on a 9-FET strongarm fixture (the ALIGN StrongARM netlist EXT-01
    already carries as `STRONGARM`, backend/annotator/tests/common/mod.rs): 0 `lvs/` rows.
  - FLOW-12's `run_writes_a_labelled_gds_and_a_report` (its `--out` half) passes against `philis run`.
- Acceptance: on m0 + this item, strongarm and pwm_driver show 0 `lvs/lvs.parameter_mismatch` rows, the suite is no
  worse than M0's 439/2/7 (row by row), and `philis --version` prints the git rev.
- Risks / notes: the deck's 4-terminal MOS recognisers and well nets change what ERC sees (`nwell.4` floating wells,
  `well_bias`); a new ERC row on a fixture is a finding, not a baseline edit. Packaging (EDA-Packaged's nix
  derivation, FR-8) lives outside this repository; this item gives it a `--version` to check.

---

## 4. Milestones

| Milestone | Items | Exit criteria |
|---|---|---|
| M0 — Honest numbers (done: all six merged, see each item's Status) | FLOW-01, FLOW-02, FLOW-03, FLOW-04, FLOW-05, FLOW-14 | CI running (only the REL-owned dac4 test red); T1 = 0; T3 = 0/0/0; T4; T5; T6; `bench local` re-baselined and verdicts unchanged except for intended size changes |
| M1 — Real inputs, real outputs | FLOW-17 (M0 close-out), FLOW-07, FLOW-12, FLOW-13 (step 0 first: `hier_elaborate`), FLOW-06, FLOW-16 | T2; labelled GDS from the CLI; emit round trip LVS-clean on chain4 and ota; T3 still 0/0/0 after the first MAT/CELL/REL keys land; `recipe_layers_cover_capacitor_tables` and `unrelated_same_size_devices_fold_independently` green |
| M2 — Loop quality and cost | FLOW-09, then FLOW-08 and FLOW-10 | T7, T8, T9; warm-start ablation table (T10) published in the bench output; `RunStats.stop` and `stage_ms` printed per circuit |
| M3 — Scale | FLOW-11 | T12; flat vs bottom-up comparison recorded |
| M4 — Close-out | FLOW-15 | every CRATES.md issue owned or closed; dead fields and deprecated keys removed |

Dependency graph (acyclic; `→` = must land before): FLOW-04 → {05, 06, 07, 12}; FLOW-01 → {07, 13, 16}; FLOW-05 → 16;
FLOW-02 → {03, 08, 09, 11}; FLOW-03 → 08; FLOW-09 → {08, 10, 11}; FLOW-07 → {11, 12, 13}; FLOW-14 and FLOW-15 have no
inbound FLOW edge (FLOW-15's doc steps trail the items they describe). External: PLC-10 step 0 → FLOW-08;
EXT-26 → FLOW-12 `--constraints` only; MAT-07 / MAT-09 / MAT-10 / REL-05 / REL-09 / CELL-08 → their FLOW-06 registry rows
(landed together).

---

## 5. Open questions (each with a proposed default)

1. **Input size convention of the competition suites.** ALIGN and MAGICAL netlists may state `w` per finger (the
   `five_transistor_ota` test deck, parse.rs:274–279, has `w=10.5e-7 nf=10`, i.e. 105 nm per finger under SPICE
   semantics, below sky130's 420 nm minimum — which suggests a per-finger source). Not given in ref/. **Default:**
   `SizeConvention::Spice` for local fixtures and user netlists; `PerFinger` for the ALIGN/MAGICAL suites after one
   manual check of each suite's convention.
2. **Deck ownership.** Vendoring decouples Philis from GPurify releases (same org, UW-ASIC). **Default:** vendor
   (FLOW-04); every `# PHILIS:` edit is also filed upstream; a GPurify bump re-copies decks and re-applies markers.
3. **sky130 high-poly body sheet.** Resolved in review: the generic model cards state `rsheet = 317.3885` (res_high_po)
   and `2000.0` (res_xhigh_po) (VOL/../libs.ref/sky130_fd_pr/spice/*.model.spice:27); FLOW-05 step 3 adds both rows.
   CRATES #6's "≈ 319 Ω/□" is superseded by the cited 317.3885.
4. **WPE/LOD tiers: Hastings generic vs model-card parameters.** Owned by MAT-07 (plan-02), which holds the tier table;
   FLOW-06 only registers the keys.
5. **Warm-start constants and `COLD_EVERY`.** **Default:** `COLD_EVERY = 4`, warm schedule `{0.05, 60, 0.002}`;
   replaced by the ablation winner (T10).
6. **Metal split between block and parent in `BottomUp`.** Depends on RTE's AT-01 stack. **Default:** no split
   (both levels see the whole stack) until RTE exposes a range; then children use metals up to the second-highest
   routing metal.
7. **Hierarchy default.** **Default:** `Flat`; `Auto` switches to bottom-up only above 100 cells per level
   (balasa_graeb_survey.txt L1604–1605) after T12 shows bottom-up is LVS/DRC clean and no worse on two_ota.
8. **EM activation energy and exponent for sky130 metals.** Owned by REL-05 (plan-06: sidecar `em_activation_ev`,
   `em_current_exponent` absent, code fallback 0.9 eV / 1.1, `EmLimit.derating_assumed`). FLOW-04's `assumed` list
   reports it.
9. **CI tool installation (magic, netgen).** PERF's foundry cross-check needs them; the flake shell has KLayout and
   ngspice only (flake.nix `devShells`). **Default:** nightly job installs via nix; PR jobs do not run foundry checks.

---

## Verification log

Adversarial fact-check of this plan against the working tree of 2026-09-28, GP/ (GPurify 8df8c09), VOL/ (volare
fa87f8f4), the reftext files and the `docs/plans/` audit and reference documents. No cargo command was run.

**References checked: 378** (approximate breakdown): about 190 code citations (`path:line`, types, fns, fields,
constants, test names, including GP/ and VOL/ lines); 115 study IDs (every `AF/AV/AA/AR/AC/AP/AT/H01/H06/H08/H12/H13/
H15/NOTES/SURV/BAL2/MM/SUB/EM/PL/LF/G` ID cited exists as a defined entry in its audit or ref document); 30 reftext line
ranges, with the PDF page recomputed from form-feed counts; 5 PDF pages opened for numbers blank in the text (Hastings
PDF pp.621, 714, 715, 716: the values 250 µm; 5 µm / 10 µm moat; 10 µm dummy reach; 5–10 µm dummy-metal block; 5–10 µm
or 2× well depth; WPE 3 µm / 2 µm all confirmed); 38 new names proposed by the plan, grepped for collisions (none, except
`annotator::block::Block`, which lives in another crate and module from the proposed `library::hier::Block`).

Confirmed without change (sample): the §1.1 loop facts L1, L3–L13 and their lines; `RunStats` (lib.rs:104–124); `LexKey`
and `key_lt` NaN behaviour (lib.rs:916, 929–939); `Prices` constants (gp/lib.rs:44–49) and `settle` (83–102); batch row
formats (gp/mechanics.rs:297, 307; gr/lib.rs:385, 393) and that nothing parses them; `harvest` (verify/lib.rs:150–163) and
GPurify `Severity` (violation.rs:11–24); deck lines for ar.met*.1, pex rows, density_cmp, EM rules, the FET marker
recognisers, capm terminal order, LU rules in magic's tech file, gf180 DF.13/DF.14 and IHP LU.a/LU.b; camimc/cpmimc
values in all three corner files; `REQUIRED_RULES` (pdk.rs:1039–1071); `pex_row`/`reaches` (pdk.rs:661–674, 758–770);
parser behaviour (parse.rs); CLI (cli/src/main.rs:23–63); GDS writer and the TEXT record codes; emit lines; macroMaster
lines; CRATES.md open-issue text and the stale/fixed triage; ota.spice sizes, ports, 5 devices / 9 nets; audit-06 G.2–G.3
figures (331/1/2, 539.5 s).

**Corrections (before → after):**
1. §1.1 L2: "PLAN.md:178 grounds PathFinder convergence on a fixed resource graph" → PLAN.md:178 states only the
   history-term argument; the fixed-graph premise is marked as this plan's inference (same fix in FLOW-08 Why).
2. §1.2 I1: "BSIM4 reads W as the instance total" → kept, tagged [UNVERIFIED in ref/].
3. §1.2 I6: "`l=0.15u` appended to every `m`/`x` card" → "…that states no `l`" (benchmarks/src/fixtures.rs:611–613).
4. §1.3 D3: `pdk.rs:1225–1276` → `pdk.rs:1225–1269` (`parse_roles` ends at 1269; 1271+ is `deck_grid`).
5. §1.3 D4 and FLOW-04 step 5: unread-demand lists omitted `p_epi_thickness` (and step 5 omitted `tie_max_dist_nm`) →
   added; grep finds both only in `REQUIRED_RULES`.
6. §1.3 D7: "an arbitrary one of five cut rows (185–585 Ω)" → the last of equals by layer id (`max_by_key`,
   pdk.rs:672), range 152–585 Ω/cut (`licon_po` = 152 Ω, sky130.deck:587).
7. §1.5 R4: "ngspice tests skip silently" → they pass after `eprintln!` + `return` (perf_postlayout.rs:52–53, 79–80).
8. FLOW-01 step 3: `class_of (constraints.rs:31–34)` → `constraints.rs:30–33`.
9. FLOW-01 risks: "the three ALIGN-style `nf` fixtures" → "the three local fixtures that state `nf` (ota,
   ota_constrained, tt_ota)"; nothing marks them ALIGN-style.
10. FLOW-02 Why: "§15.5 items 1–3 separate DRC findings that must be fixed from reviewable ones" → items 1–3 separate
    never-waivable LVS topological errors from reviewed-and-waived DRC diagnostics and LVS parametric errors
    (hastings.txt L48856–48878).
11. FLOW-02 acceptance: "the 10 fixtures that are DRC/LVS clean today keep V = 0" → 8 of 10; bgr_core and bjt_mirror
    carry one `erc/supply_short:met1` error each (audit-06 G.3).
12. FLOW-03 acceptance: added that `drift_decreases_as_prices_settle` (gp/lib.rs:418–440) asserts `drift == 0` at
    residual 0, which step 3's slack relaxation breaks.
13. FLOW-04 test: "over the testkit groups (kernel/cells/src/lib.rs:57–166)" → `testkit` is `#[cfg(test)] pub(crate)`
    and unreachable from `kernel/cells/tests/`; use the helpers of cell_selfcheck.rs:42–95.
14. FLOW-05 Why: the 250 µm bound is blank in hastings.txt → noted as read on PDF p.621 (verified).
15. FLOW-05 step 3: rp1 source → `res_typical__cap_typical.spice:21` (line added).
16. FLOW-05 step 6: "gf180mcu `20000` / ihp_sg13g2 `20000`" as new values → both already carry 20000
    (pdks/gf180mcu.json:81, pdks/ihp_sg13g2.json:76) and gain only `_source`; sky130 is 3000 → 15000.
17. FLOW-06 step 2: exceptional `wpe_clearance_nm` 5000 and `fill_block_halo_nm` 5000 were untagged picks from
    Hastings' "5–10 µm" → tagged [policy: lower end].
18. FLOW-06 step 3: Eq 15.25 "PDF pp.821–822" → "PDF p.821" (L48776–48791 lie between the 819 and 820 footers).
19. FLOW-06 step 4: `pdk.rs:1078–1147` → `Recipe` pdk.rs:1077–1085, `Pdk::recipe` pdk.rs:1129–1144.
20. FLOW-06 step 7: "not given in ref/" → no deck-specific value; Charbon gives generic classes (SUB-01).
21. FLOW-07 Why: quote "Netlist + Spec" (perf_driven_survey.txt L56–72; not found verbatim) → "Given a netlist and
    design specifications" (L59–60).
22. FLOW-07 step 1: "~20 struct literals in tests" → 20 `Netlist { … }` literals, in tests and in library code
    (parse.rs:118, kernel/macroMaster/src/lib.rs), listed by file.
23. FLOW-07 step 2: `model_table` listed mos/resistor/capacitor/diode only → added `bjt` rows (GPurify `DeviceKind::Bjt`,
    GP/crates/ingest/src/deck/mod.rs:335–341; 2 rows each in gf180mcu/ihp_sg13g2); noted `Pdk::reaches` is private
    (pdk.rs:758); `pdk.rs:1129–1147` → `1129–1144`.
24. FLOW-07 step 7: `Pdk::deck_model, pdk.rs:1120–1127` → `pdk.rs:1116–1123`.
25. FLOW-08 step 1: added the cross-plan name note (PLC's `dp::Start`, plan-04-placement.md:325).
26. FLOW-09 risks: added that `solve` moves `flow.problem.placement` into `Solution.placement` (lib.rs:440) and
    `Requirements` is not `Clone` (docs/API-WISH.md D13), so a `&Topology` shared across starts needs a change there.
27. FLOW-10 Why: "dummy fill that must obey the rules, hastings.txt L5783–5792" (no rule statement in those lines) →
    the rule statement cited separately, L8222–8224, PDF p.147.
28. FLOW-10 acceptance: "gf180/ihp exercise it" → false today: gf180 states metal minima only as `global_density`
    (gf180mcu.deck:395, 419), which `density_rules` ignores; IHP uses 800 µm windows (ihp_sg13g2.deck:269, 395), wider
    than any local block; fill is a no-op on every fixture and deck.
29. FLOW-11 Why: H15-05 "PDF pp.782–783" → p.783; H15-07 "pp.794–795" → p.794; balasa "L3062–3072" → L3068–3075, and
    "recommends simultaneous hierarchical placement" → "notes that modern placers optimise sub-circuits simultaneously
    because…" (quoted).
30. FLOW-11 current: "macroMaster hierarchy (lib.rs:328–338)" (read as frontend/library/src/lib.rs, where those lines are
    `Flow` fields) → `kernel/macroMaster/src/lib.rs:328–338` (`instantiate_comp`), as AF-20 cites.
31. FLOW-11 step 3.4: "`labels_and_reference(sol)`" (private fn taking shapes/placed/nets/schematic/fold/pdk, not a
    `Solution`) → `signoff_inputs(&sol, pdk).2.devices` (lib.rs:1318–1331).
32. FLOW-12 Why: H01-30 "PDF pp.126–128" → PDF p.126 (L6880–6885) and p.129 (L7036–7039).
33. FLOW-12 step 3: added the fixture's JSON key names (`die.w`/`die.h`, `width`) next to the proposed field names.
34. FLOW-13 Why: signed orientation Φ at "§13.2.1, L40966–41017" → §13.2.6 Eq 13.61, L42162–42187 (§13.2.1 kept for the
    orientation mechanisms).
35. FLOW-13 step 4: "2021 strict and reserved list … gen" → `gen` is reserved only from edition 2024; workspace crates
    are edition 2021.
36. FLOW-14: volare hash cited as flake.nix:112; added that the flake pins `1341f54f…` while every VOL/ citation here and
    in the sidecar `_source` texts was read from `fa87f8f4…`.
37. FLOW-15 step 4: `polys_from_shapes (lib.rs:1188–1201)` → `kernel/visualizer/src/lib.rs:1188–1201`.

**Unresolved (left as stated or tagged inline):**
- BSIM4 "W is the instance total, NF splits it": no BSIM4 manual in `ref/`; tagged [UNVERIFIED in ref/] in §1.2 I1.
- PLAN.md:178's fixed-resource-graph premise: an inference, marked as such in L2 and FLOW-08.
- D7's "`licon_po` if layer ids follow declaration order": GPurify's id assignment order was not traced.
- OQ-1 (ALIGN/MAGICAL size convention): the competition submodules' netlists were not read; the plan already says
  "not given in ref/".
- §1.6 #4 (AP-05, origins off-lattice by 5 nm) and R3 (331/1/2 tests, 539.5 s bench) are taken from audit-04/audit-06;
  not re-run (no cargo, per task rules).
- Proposed constants tagged [policy]/[measure] (COLD_EVERY, warm schedule, dp seed constant, 5 % antenna tolerance)
  are choices, not verifiable facts.

---

## Cut or deferred items

| Item (previous text) | Decision | Reason |
|---|---|---|
| FLOW-06 steps 1–2: `pnr_core::MatchClass`, `Process::tier(key, MatchClass)`, `Pdk::tier_nm`, tier *objects* `{minimal, moderate, exceptional}`, keys `dummy_poly_reach_nm`, `lod_moat_ext_nm.minimal = –` | Cut → MAT-07 (plan-02) | Duplicate with a conflicting schema: MAT-07 defines `MatchClass` in `kernel/analog/src/matching/class.rs`, `Process::tier(name, idx)` over 3-element arrays, keys `dummy_reach_nm`/`gate_ext_extra_nm`, and a sourced Minimal moat of 3000 nm (§13.2.2). `kernel/core` cannot name an `analog` type. FLOW-06 keeps only the registry rows. |
| FLOW-06 step 3: `em_derating {t_ref_k, ea_ev, n}` table | Cut → REL-05 (plan-06) | REL-05 defines `em_ref_temp_c`, `em_activation_ev`, `em_current_exponent` and the `em_limit` fallback; two key sets for one datum would disagree. |
| FLOW-06 step 4: `capacitors` table with `bottom/plate/top_contact/top` roles, `Pdk::capacitor_recipe`, `CapRecipe` | Cut → CELL-08 (plan-03) | CELL-08 defines the table with `cap_*` roles read by the existing `Pdk::recipe("capacitor", model)` (pdk.rs:1129–1144); a second reader is dead weight. FLOW keeps the `recipe_layers` fix CELL-08 lists as its precondition (FLOW-06 step 2). |
| FLOW-06 step 5: `Pdk::model_markers(model)` | Deferred | No consumer: CELL-16 names the marker layers in explicit `mosfets` recipes, and LVS extraction already rejects a wrong marker. Add as a `Pdk::validate` cross-check (recipe markers ⊇ recogniser's required operands) when a second deck's `mosfets` recipes are written. |
| FLOW-06 step 6: mismatch-coefficient slots `abeta_n_pct_um`, `abeta_p_pct_um`, per-recipe `k_a_pct_um` | Cut → MAT-09, MAT-10 | Same data under different names (`abeta_pct_um`, `ka_pct_um`, `cap_ka_pct_um`, …); MAT owns producer and consumer. |
| FLOW-06 step 7: `substrate {kind, epi_um, rho_ohm_cm}` table | Cut → REL-09 | REL-09 defines `substrate_kind`, `epi_thickness_nm` and deletes `p_epi_thickness`. |
| FLOW-10 step 1: per-window fill targets with exact clipped coverage | Deferred | Fill is a no-op on every shipped deck × local fixture (FLOW-10 Current); a per-window algorithm would be untestable on real data. Trigger: a `global_density` reader for gf180mcu, or a block wider than IHP's 800 µm window. |
| FLOW-10 step 2: `fill_block_halo_nm` keep-out around Exceptional cells (H13-52) | Deferred with the above | No fill exists to keep out; also needs MAT/EXT's per-cell class. |
| FLOW-09 step 2 (previous): `performance_rows` reuses the first topology's classes | Replaced | Circular: `perf_rows` are an input of the topology (lib.rs:267–269). Replaced by one up-front `annotate` in `run`. |
| FLOW-03 acceptance: rewrite `drift_decreases_as_prices_settle` | Dropped | With slack measured as `worst_usage` else `1 − criticality`, the existing test rule has `g = 0` at residual 0; the test passes unchanged. |
| AR-29: `Process::rule(name) -> Option<i32>` | Deferred | Touches every generator call site; FLOW-04's `required` rows already make a missing key a load error on shipped decks. Revisit when a deck without a compiled default is added. |
| AF-15 third point: re-measure `P2P_SHARE` (cellgen.rs:593–597) on a second deck | Deferred | Needs a measurement on gf180/ihp p2p rules; no data in the repo or ref/. |
| REL-06 steps 1–2 (LU rules, `tie_max_dist_nm`) and REL-02 step 6 (`ar.met3.1` 845 nm) | Landed here, not duplicated | Same deck/sidecar edits as FLOW-05 steps 1, 2, 6; FLOW owns the deck files (M0). REL-06 keeps its CELL contract and fixture tally. |

---

## Implementer review log

Reviewed as the implementer, against the code (frontend/library, backend/verify, backend/gp, kernel/analog, kernel/cells,
kernel/core), GP/ (8df8c09), VOL/ (fa87f8f4) and plans 01–07. 16 work items checked (FLOW-01…15 plus the new FLOW-16).
No cargo command run.

Changes:
1. §0.2 / §0.3 rewritten with exact producer item IDs: EXT-26 `annotator::sidecar::parse` (not `AnnotationConfig::from_json`),
   MAT-07 tiers, MAT-09/10 coefficients, REL-05 EM keys, REL-09 substrate keys, CELL-06/08/09/16 recipe tables, PERF-09
   op-point fields; `MatchClass`/`tier_nm`/`capacitor_recipe`/`model_markers` removed from what FLOW provides.
2. FLOW-01: added the two missed consumers — `cellgen::folds` (cellgen.rs:615–699) reads `w` as per-finger width at
   cellgen.rs:624, 640, 643, 650 and must read `w_finger_nm`; the private duplicate `fingers` (cellgen.rs:703–706) is
   deleted. Made the drawn-width test's unit-to-device mapping exact (`Unit.owner`, `Unit.weight` = W·L).
3. FLOW-02: completed the list of non-batch stage rows (`em cuts net`, dr lib.rs:883; `routing overflow`, gr lib.rs:404).
4. FLOW-03: replaced pseudocode with compilable Rust on the existing `Price` fields; the slack signal was
   `worst_usage`, which most batches do not report (`usage` implemented only by thermal, antenna, parasitic, ir,
   crosstalk rules) — now `worst_usage` else `1 − criticality`, and a batch that measures neither keeps its price.
   Consequence: `drift_decreases_as_prices_settle` passes unchanged (previous plan said it must be rewritten).
   Added `RunStats.dual_steps` so `one_dual_step_per_epoch` can observe the count (prices are internal to `solve`);
   made the relax/saturate tests concrete (test rule with an unclamped `usage`; saturation reached in 9 steps).
5. FLOW-04: linked AR-29 and stated what the registry does and does not fix.
6. FLOW-05: (a) fixed the defect plan-07 handed over — the new `pex_row` makes `poly` ambiguous (`poly_c` and
   `licon_po` both reach it); now prefers declared conductors, with full Rust; (b) `sheet_ohm("licon")` becoming `None`
   would silently disable two-ended gates (mosfet.rs:62): added `Process::cut_ohm(cut, onto)` (sky130 licon→poly 152 Ω,
   licon→tap 585 Ω) and switched mosfet.rs:62 and `ring_cut_ohm`; (c) one R□ read in `folds` (AF-15 part);
   (d) LU layer definitions cited to magic's CIF templayers (sky130A.tech:1514–1516, 1547–1549, 1608–1610) and the
   ownership overlap with REL-06/REL-02 recorded; (e) generic_finfet `tie_max_dist_nm` 3000 → 30000 (its deck states
   30 µm, generic_finfet.deck:165–166); (f) OQ-3 resolved: `pex rbody_high_po` 317.3885 Ω/□ and `rbody_xhigh_po`
   2000 Ω/□ from the installed model cards (…res_high_po.model.spice:27, …res_xhigh_po.model.spice:27);
   (g) tests added: `poly_answers_with_its_conductor`, `routing_layers_keep_their_pex_rows`,
   `two_ended_gate_still_pays_on_sky130` (1071 Ω > 152 Ω).
7. FLOW-06: reduced from a 7-step schema/population item (M) to registry rows + `recipe_layers` over all recipe tables
   (pdk.rs:1105; CELL-08's stated precondition, capm enclosure 140 nm, sky130.deck:408) + `cap_density_ff_um2`
   provenance (S). Six steps moved to the appendix as duplicates of MAT-07/09/10, REL-05/09, CELL-08 or as unconsumed.
8. FLOW-08: dependency pinned to PLC-10 step 0 (`dp::Schedule`) — the flat warm path passes the incumbent as dp's
   `coarse` input and needs no `dp::Start`; `Schedule` field renamed `max_iters` → `max_temps` to match plan-04;
   `escalate_blamed` ordering made exact and the PLC-03 lock-set rule added.
9. FLOW-09: resolved the open risk (Requirements not `Clone`) with an exact split `topology` / `search` / `finish`
   (winner finalised once, consuming its topology); replaced the circular "reuse the first topology's classes" with one
   up-front `annotate` in `run` (the step PERF-09 defers to). Effort S → M.
10. FLOW-10: cut the speculative per-window fill and halo (fill is a no-op on all decks and fixtures); kept the ERC+DRC
    gate and the post-fill certificate, now also returning `Solution.caps` (PERF-22 reads it as "FLOW-10 step 4").
    Effort M → S; depends on FLOW-09 instead of FLOW-05/06.
11. FLOW-12: `--constraints` wired to EXT-26's parser with diagnostics in the report; exits 2 until EXT-26 lands.
12. FLOW-14: red-test citation made exact (signoff_fixtures.rs:139–143 via `fixtures_sign_off_within_baseline`);
    the one-time `cargo fmt` commit is gated on the owner committing the current uncommitted tree.
13. New FLOW-16 (AF-15, unowned by any plan): fold classes per unitization entry instead of per netlist-wide
    `(kind, W, L)`, with an exact test.
14. §0.1 scope item 5, FLOW-15 step 6, OQ-3/4/8, milestones M1/M2 and a FLOW dependency graph updated to match.

Unresolved for other plans (not editable here): plan-05 (RTE-16) and plan-07 (PERF-23) still name
`pnr_core::MatchClass` / `Pdk::tier_nm`; the implemented names are MAT-07's `analog::matching::MatchClass` and
`Process::tier(name, class as usize)`. plan-07 line 49 cites "FLOW-10 step 4"; it is now FLOW-10 step 2.
