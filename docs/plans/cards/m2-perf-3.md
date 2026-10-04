# M2+ perf segment 3 (M4): implementation cards

Branch `m2-perf`, worktree `philis-m2/perf`, after `git merge m2` (`23854da`, clean). DAG: PERF-11 hard-depends on
PERF-10 (done, segment 1); PERF-13 on PERF-11; MAT-04 optional, FLOW-01 done. Line numbers are of this tree; find
code by the symbol named next to each. Every command needs
`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

| Item | Class |
|---|---|
| PERF-11 | do |
| PERF-13 | do (MAT systematic/gradient terms passed empty; see scope) |

Batch rules: no new warnings in `library`. A test not named here that goes red means stop and report; do not edit
its assertion. Order: PERF-11 then PERF-13 (PERF-13 reads `SensTable`).

---

## PERF-11 Sensitivity engine v2

### Current code (plan-07 corrections marked \*)

- \* PERF-10 already landed: `Scenario` (perf.rs:62), `PerfConfig { sim, testbenches, specs, scenarios }` (:71),
  `BoundResult { spec, upper, value, scenario }` (:99), `PerfResult { metrics, bounds, miss, residual }` (:111),
  `score` (:146), `deck(netlist, par, cfg, tb, sc)` (:178), `evaluate(netlist, par, cfg, scenarios)` (:251, one
  `thread::scope` thread per (testbench, scenario) deck, marked `ponytail: … PERF-11's run_jobs bounds it`, :260).
- \* `Sensitivity { base, d_per_af: [bound][net] }` and `sensitivities(netlist, cfg, nets, delta_af, scenarios)` are at
  perf.rs:300–339 (plan says :192–223): base at `Parasitics::default()` + one thread per net, forward difference.
- `budget_rows(cfg, s: &Sensitivity, nets, af_per_nm)` (:346–367) is its only consumer; tests at :445–503 build
  `Sensitivity` by hand.
- \* `performance_rows` is lib.rs:322–404 (plan: 196–231); it calls `perf::evaluate(.., &all)` (start), derives
  `active` (nominal + each bound's worst scenario), then `perf::sensitivities(.., &names, 10_000.0, &active)` (:388).
  It returns `(rows, notes, active)`, consumed at lib.rs:287 and passed to both `solve` calls (:296, :300).
- `Parasitics { caps: CapMatrix = Vec<(String, Option<String>, f64 /* fF */)>, series: Vec<Vec<(String, f32)>>,
  lod_inv_um, extracted }` (perf.rs:16). `deck` drops cap entries `<= 0` (:192), so a negative perturbation must
  edit the existing entry, not add a negative card.
- Gate node: `flat_circuit_with(netlist, sim, node(di, t, net) -> String, extra, emit_caps)` (oppoint.rs:385); the
  FET card reads `n("G")` (:413). `deck` already renames branch-R terminals to `{net}__{di}_{t}` (:213–219).
- Terminal names are strings: FET `D G S B`, BJT `C B E (S)`, R/C/D `P N` (oppoint.rs:411–447).
- \* `gate_cap_af_um2` 8325 is pdks/sky130.json:10 (plan :9; its `_source` says UNVERIFIED); it reaches code as
  `annotation(pdk, ..).process.gate_af_per_um2` (lib.rs:745; plan says :501). \* `avt_*` are sky130.json:105–108.
- `RunStats` (lib.rs:167) has no simulation count; `sim_failures` (:204) is summed over starts in `run`
  (lib.rs:302, :315–317), the pattern `sims` follows.
- `crate::tools::tool_or_skip` (lib.rs:49) and `sky130_models()` (:55) gate ngspice tests.

### Decisions

- **`SeriesR.terminal` is an index**, `u8` into `netlist.devices[device].terminals`, not the plan's `[u8; 2]`
  name: keeps `Param: Copy`, no string conversion, and PERF-12 maps it to a net with `terminals[t].1` directly.
- **`StepPolicy` gains `gate_af_um2: f64`** (0 = unknown → every C step is `c_min_af`); the plan reads the deck
  number but gives the policy no field for it.
- **Coupling screening is normalised:** score = `max_j (|d_a[j]| + |d_b[j]|) / s_j`, `s_j = |base metric j|` (1 when
  0/None). Raw `max_j` would let the spec with the largest unit dominate every pair.
- **Linearity of a central row** costs no run: `s₊ = (f(+Δ) − f₀)/Δ`, `s₋ = (f₀ − f(−Δ))/Δ`, `d = (s₊+s₋)/2`,
  `linear` by the same `lin_tol` test on `(s₊, s₋)`. Forward rows: Δ and Δ/2, keep `s₂` (plan). Both 2 runs per row.
- **`budget_rows` stays** (EXT-25/PERF-12 replace it later) but reads tables: the bound's worst value from the start
  evaluation, `d` from the `GroundC` row of the table whose `scenario == bound.scenario`, `d[bound.spec]`;
  a `linear == false` row is left out like an unmeasured net. Semantics change: the derivative is now at the bound's
  active scenario, not "the worst case under the step".
- One pool level: `sensitivities` flattens (perturbation, testbench) into one `run_jobs` call. `evaluate` uses
  `run_jobs` too; neither calls the other inside a job, so threads ≤ `available_parallelism()`.
- `PerfPlan` (private struct in lib.rs) replaces `performance_rows`' 3-tuple, since PERF-13 needs the tables at the
  winner.

### Edits

1. `frontend/library/src/perf.rs`, delete `Sensitivity` and the old `sensitivities`; add:
   ```rust
   /// One perturbed quantity. Gate offset sign: a source in series with the gate, `δ = V_gate − V_net`; an NMOS
   /// threshold shift ΔV_T acts as `δ = −ΔV_T`, a PMOS one as `δ = +Δ|V_T|`.
   #[derive(Clone, Copy, Debug, PartialEq)]
   pub enum Param {
       GroundC { net: pnr_core::NetId },             // aF
       CouplingC { a: pnr_core::NetId, b: pnr_core::NetId }, // aF, a < b
       SeriesR { device: u16, terminal: u8 },        // Ω, index into the device's `terminals`
       GateOffset { device: u16 },                   // V
   }
   pub struct SensRow { pub param: Param, pub step: f64, pub d: Vec<Option<f64>>, pub linear: bool }
   pub struct SensTable { pub scenario: usize, pub at: Parasitics, pub base: PerfResult, pub rows: Vec<SensRow>, pub sims: u32 }
   pub struct StepPolicy { pub c_min_af: f64, pub c_frac: f64, pub gate_af_um2: f64, pub r_ohm: f64, pub lin_tol: f64 }
   // Default: 1000.0, 0.1, 0.0, 100.0, 0.2
   pub fn sensitivities(netlist: &Netlist, cfg: &PerfConfig, scenario: usize, params: &[Param],
                        sigma_v: &[Option<f64>], steps: &StepPolicy, at: &Parasitics) -> Result<SensTable, String>;
   /// Appends `CouplingC` rows for the `max` best-screened pairs to `t` (reuses `t.base`).
   pub fn add_coupling(t: &mut SensTable, netlist: &Netlist, cfg: &PerfConfig, nets: &[pnr_core::NetId],
                       steps: &StepPolicy, max: usize) -> Result<(), String>;
   /// Steps 1–3 of the selection: GroundC per `nets`; SeriesR for FET S/D, BJT E/C, resistor P/N terminals on nets
   /// with ≥ 2 pins; GateOffset per FET.
   pub fn default_params(netlist: &Netlist, nets: &[pnr_core::NetId]) -> Vec<Param>;
   /// Pairs `(a ∈ nets, b ≠ a any net)`, unordered, ranked by the normalised score (decision above), ties by
   /// `(a, b)`; at most `max`.
   pub fn coupling_params(t: &SensTable, netlist: &Netlist, nets: &[pnr_core::NetId], max: usize) -> Vec<Param>;
   pub(crate) fn run_jobs<J: Sync, T: Send>(jobs: &[J], f: impl Fn(&J) -> T + Sync) -> Vec<T>;
   ```
   - `run_jobs`: `available_parallelism()` (1 on error) workers in `thread::scope`, each `fetch_add` on an
     `AtomicUsize` index; results into `Mutex<Vec<Option<T>>>` by index; order preserved. Remove the `ponytail`
     line at :260 and the `thread::scope` in `evaluate`.
   - Split `evaluate`'s per-deck closure into `fn measure(netlist, par, cfg, tb, sc) -> Result<Vec<(String, f64)>, String>`
     and its per-scenario assembly loop (:276–290) into `fn assemble(cfg, per_tb: &[Vec<(String, f64)>]) -> Result<Vec<Option<f64>>, String>`;
     `sensitivities` reuses both.
   - `Parasitics` gains `pub gate_offset_v: Vec<f64>` (per device, missing/0 = none). In `deck`, the `node` closure
     passed to `flat_circuit_with` returns `{prev}__o{di}` for `t == "G"` with a non-zero offset (`prev` = the
     branch-R node or the net node) and `deck` adds `Vgo{di} {prev}__o{di} {prev} {v:.6e}` to the series-R section.
     Update the struct literal at lib.rs:1026 (`..Default` not used there).
   - `fn perturb(at: &Parasitics, p: Param, v: f64, n_dev: usize) -> Parasitics`: GroundC adds `v/1000` fF to the
     `(name, None)` entry (push if absent); CouplingC likewise to `(a, Some(b))` or `(b, Some(a))`; SeriesR adds `v`
     Ω to `series[device]`'s entry for the terminal name (resize `series` to `n_dev`); GateOffset adds `v` to
     `gate_offset_v[device]` (resize). `fn base_value(at, p) -> f64` reads the same entry (0 if absent).
   - Step: GroundC/CouplingC `Δ = max(c_min_af, c_frac·C_est)`, `C_est` = Σ `gate_area_um2()` of devices whose `G`
     is on the net (net `a` for coupling) × `gate_af_um2`; SeriesR `Δ = r_ohm`; GateOffset `Δ = sigma_v[d]` or
     1e-3 V when `None`. Central iff `base_value ≥ Δ` (always for GateOffset), else forward Δ, Δ/2.
   - `sensitivities`: `base = assemble(measure(at))` at `[scenario]`; jobs = every (row, ±/full/half, testbench);
     `d[j] = None` if either run misses metric j; `linear` = every spec with `Some` passes the test; `sims` = decks
     run. `base` is `score(&cfg.specs, &[row], &[scenario])`.
   - `budget_rows(cfg, start: &PerfResult, tables: &[SensTable], nets, af_per_nm)` per the decision; doc comment
     updated, logic otherwise unchanged.
2. `frontend/library/src/lib.rs`:
   - `RunStats` gains `/// ngspice decks run: sensitivities plus every scored epoch. pub sims: u32`. `score_perf`
     adds `(self.perf_active.len() * p.testbenches.len()) as u32` per `evaluate` call; `run` sums `sims` over both
     topologies and every start exactly as `sim_failures` (:302, :315) and adds `plan.sims`.
   - `struct PerfPlan { rows: Vec<PerformanceBudget>, notes: Vec<String>, active: Vec<usize>, tables: Vec<perf::SensTable>, sims: u32 }`;
     `performance_rows` returns it; `solve`'s `perf_rows, perf_active` params become `perf: &PerfPlan`.
   - In `performance_rows`, after `start` and `active`: per active scenario `s`,
     `t = sensitivities(nl, p, s, &default_params(nl, &nets), &vec![None; n_dev], &steps, &Parasitics::default())`,
     then `add_coupling(&mut t, nl, p, &nets, &steps, 64)`; `steps = StepPolicy { gate_af_um2: ann.process.gate_af_per_um2.map_or(0.0, f64::from), ..Default }`.
     Delete the "10 fF" comment. Notes added (and `eprintln!`ed): `"sens {scenario name}: {rows} rows, {sims} sims, {ms} ms"`
     and per nonlinear row `"sens {scenario name}: nonlinear {param:?}"`. The `Err` path keeps
     `"sensitivities unavailable: {e}"` (lib.rs:1964 asserts it).

### Tests

`perf.rs` unit tests (ngspice ones start `if !crate::tools::tool_or_skip("ngspice") { return; }`, models by
`.model` cards in the bench, `sim.model_lib = None`):
- `run_jobs_never_exceeds_the_pool`: 64 jobs each `inc` an `AtomicUsize`, update a max with `fetch_max`, sleep
  5 ms, `dec`; `assert!(max <= available_parallelism())` and results equal `jobs` order (`out == (0..64).map(|i| i*2)`).
- `the_deck_inserts_a_gate_offset_source`: the 1-FET netlist of `the_deck_sets_corner_temperature_and_params`,
  `gate_offset_v = [1e-3]` → deck contains `"Vgo0 in__o0 in 1.000000e-3"` and the `XM1` card's second node is
  `in__o0`.
- `rc_lowpass_f3db_derivative_matches_analytic`: netlist nets `in`, `out`, no devices; bench
  `Vin in 0 dc 0 ac 1\nR1 in out 1k\nC1 out 0 1p\n.ac dec 1000 1e6 1e10\n.measure ac f3db when vdb(out)=-3`;
  spec `f3db`; `sensitivities(.., 0, &[GroundC{out}], &[], &StepPolicy::default(), &Parasitics::default())`;
  `d = rows[0].d[0]`; `let want = -f0 / 1e6` (aF: C = 1e6 aF); `assert!((d / want - 1.0).abs() < 0.05)`;
  `rows[0].linear`; `t.sims == 3`.
- `gate_offset_moves_a_diff_pair_output_both_ways`: level-1 NMOS pair (`.model nch nmos level=1 vto=0.5 kp=200u`,
  `sim.nmos_model = "nch"`), `Rl1 vdd o1 10k`, `Rl2 vdd o2 10k`, tail `I1 t 0 100u`, both gates at 1 V,
  `.dc Vdd 1.8 1.8 0.1` + `.measure dc vod find v(o1,o2) at=1.8`; GateOffset rows for both FETs:
  `d1 * d2 < 0` and `(d1 + d2).abs() <= 0.1 * d1.abs().max(d2.abs())`; both rows `linear`.
- `coupling_rows_only_for_screened_pairs` (pure): a `SensTable` by hand with 20 nets of GroundC rows, `d` = net
  index; `coupling_params(&t, &nl, &nets, 64)` has `len() <= 64`, first is the pair of the two largest-`d` nets, all
  `CouplingC { a, b }` with `a < b` and no duplicates.
- Port `rows_turn_sensitivities_into_shares_of_the_headroom`, `one_net` and the three `one_net` tests to
  `SensTable` with `GroundC` rows (same numbers, same assertions); add to the first one a `linear: false` row on
  net 1 and assert net 1 is left out of row 0.

`frontend/library/tests/perf_postlayout.rs` (acceptance, `let Some(lib) = models() else { return };`):
- `ota_sensitivities_cover_every_parameter_class`: `nl = ota()`, `p = cfg(lib)`, nets = every net whose
  lowercase name is not `vdd`/`vss` (ota: `vinp vinm vout1 vout2 vbias vbn vtail`); `t = sensitivities(&nl, &p, 0, &default_params(..), &[], &Default, &Default)`,
  `add_coupling(.., 64)`. Assert each of the four `Param` kinds has ≥ 1 row; `CouplingC` rows ≤ 64; `t.sims as usize
  <= 1 + 2 * (n_nets + n_terminals + n_fets + 64)` (counts from `default_params` by kind); print rows, nonlinear
  rows and wall time with `eprintln!`. The `GroundC{vout2}` and `GateOffset{XM1}`/`{XM2}` rows have
  `d[0].is_some()`, and the two gate-offset derivatives have opposite signs (XM1/XM2 drive the two branches).

Command: `cargo test -p library --lib perf:: && cargo test -p library --release --test perf_postlayout`.

---

## PERF-13 Statistical layer: σ_f, β, statistical headroom, linearised yield

### Current code

- No σ/β/yield anywhere in `frontend/library` (plan P13 holds). `robust.rs` does not exist.
- \* A_VT: sky130.json:105–108 (plan: 93–96), read as `annotation(..).process.avt_mv_um: [Option<f32>; 2]`
  (lib.rs:749) — the exact type the plan's `device_sigma_v` takes.
- `Device::gate_area_um2()` (kernel/core/src/netlist.rs:89) = W_total·L·m, 0 for non-MOS.
- `gp::SplitMix64` (backend/gp/src/mechanics.rs:11, re-exported at gp lib.rs:15): `new(seed)`, `f64()` uniform
  `[0,1)`; `gp` is already a `library` dependency.
- MAT-04 exists as a placement ledger: `sigma_grad`, `mu_thermal` per matched pair at
  kernel/analog/src/placement/matched_set.rs:115–131, not exposed to the flow's winner.
- Winner metadata is filled at lib.rs:654–679 (`performance`, `performance_worst`); printed at metadata.rs:367.

### Scope

- `sys` and `grad` are parameters and tested, but the flow passes `&[]` for both: feeding MAT's per-pair ledger at
  the winner needs a layout-side export that MAT owns. Metadata labels every σ_f "V_T only" (plan risk line).
- `β_target` is `pub const BETA_TARGET: f64 = 3.0` in `robust.rs`, not a `PerfConfig` field (no consumer sets it;
  adding a field touches every `PerfConfig` literal).
- `SpecSens.sigma_f` is PERF-12's `to_evidence`; PERF-13's acceptance line on it is checked there.

### Edits

1. New `frontend/library/src/robust.rs` (`pub mod robust;` in lib.rs next to `pub mod perf`), signatures as the plan:
   `device_sigma_v`, `BoundStat`, `bound_stats`, `linear_joint_yield`, `phi`. `BoundStat` gains
   `pub headroom_stat: Option<f64>` = `margin − BETA_TARGET·σ_f` (margin = `f_post + Δf_sys − lo` / `hi − f_post −
   Δf_sys`); `shares` sorted descending, truncated to 3.
   - `device_sigma_v`: FET with `mos_size()` and `avt[polarity]` → `avt / (2·area).sqrt() / 1000`; else `None`.
   - `s_bi` = `d[spec]` of the `GateOffset { device: i }` row of the table with `scenario == post.bounds[b].scenario`
     (used whatever `linear` says: it is the step-σ secant). σ_f is `None` when that table is missing, `value` is
     `None`, or any FET has `sigma_v` `None` or no `Some` `d` (unknown, never an underestimate).
   - `phi(x) = 0.5·erfc(−x/√2)`, erfc by A–S 7.1.26 for `z ≥ 0` and `2 − erfc(−z)` below.
   - `linear_joint_yield`: Box–Muller from two `SplitMix64::f64()` (use `1 − u` for the log), one `t` vector per
     sample shared by every bound; `None` if any bound's σ_f is `None`.
2. `lib.rs`: `PerfPlan` gains `sigma_v: Vec<Option<f64>>` = `robust::device_sigma_v(nl, ann.process.avt_mv_um)`,
   passed to `sensitivities` instead of `vec![None; n]`. In the winner block (:654), with
   `stats = robust::bound_stats(&plan.tables, &plan.sigma_v, result, &cfg.specs, &[], &[])`, fill new
   `MetadataReport.robustness: Vec<String>` (metadata.rs next to `performance_worst`, printed as `"  robust {r}"`):
   `"{metric}:{min|max} σ_f {:.4e} β {:.2} Φ(β) {:.4} (V_T only) top {dev} {pct:.0}%, …"`, or
   `"{metric}:{side} UNKNOWN (no A_VT)"` when σ_f is `None`; plus one line `"joint yield (linear, 1e5) {:.4}"`.

### Tests

`robust.rs` unit tests (pure, no ngspice), exactly the plan's five with these assertions:
- `sigma_f_of_a_symmetric_pair`: GateOffset rows d = [+1000, −1000], σ = [1e-3, 1e-3], one floor bound →
  `(sigma_f - 2f64.sqrt()).abs() < 1e-12`; shares `[(0, 0.5), (1, 0.5)]` ± 1e-12.
- `beta_reproduces_graeb_table_14_gain`: one device, s·σ = 4.4, `value` 76, `min` 65 → `(beta - 2.5).abs() < 1e-12`;
  `headroom_stat == 11 − 3·4.4` ± 1e-12.
- `phi_matches_table_12`: Φ(−1, 0, 1, 2, 3) vs 0.159, 0.500, 0.841, 0.977, 0.999, each within 1e-3; plus
  `phi(-x) + phi(x) == 1` ± 1e-7 at x = 0.3.
- `device_sigma_of_a_sky130_nfet`: W 1000 nm, L 1000 nm, m 1, `[Some(9.5), Some(11.5)]` → `(s - 6.7175e-3).abs() < 1e-5`;
  a resistor → `None`; `[None, Some(11.5)]` on the NFET → `None`.
- `linear_yield_of_one_bound_is_phi_beta`: β = 1.5 setup, 100 000 samples, seed 1 → `|Ŷ − phi(1.5)| ≤ 0.005`.
- Added: `systematic_shift_moves_beta` (`sys = [1.1]` on the Graeb case → β 2.75) and `a_missing_sigma_is_unknown`
  (one `sigma_v` `None` → `sigma_f`, `beta`, `yield_part` all `None`).

`perf_postlayout.rs` acceptance `ota_reports_robustness_per_bound`: the `a_flow_reports_the_worst_scenario_per_bound`
flow setup with one scenario; `sol.metadata.robustness` has one line per bound starting `"gain:min σ_f "`, containing
`" β "`, `" Φ(β) "` and `" top "` with 3 device names, and a `"joint yield"` line.

Command: `cargo test -p library --lib robust:: && cargo test -p library --release --test perf_postlayout ota_`.
