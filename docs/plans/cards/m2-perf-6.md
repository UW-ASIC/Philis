# M2+ perf segment 6 (M4): implementation card — PERF-26

Spec: plan-07 `### PERF-26`. DAG: module perf, M4, hard dep CELL-19 (landed: `kernel/core/src/macro.rs:96–104`
`Figures { sd, gate_ohm }`, filled by `kernel/cells/src/mosfet.rs:200–208, 298–303, 871`); PERF-08 done.
Env: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`; every cargo command via
`/home/omare/Documents/Projects/Rust/Philis/tools/qcargo` from the worktree (enter `nix develop` from the repo's
`flake.nix` first if a tool is missing from PATH; ngspice is on PATH today).

| Item | Class |
|---|---|
| PERF-26 | do (steps 1–5; step 4's "PERF-15 post-fill evaluation" half lands with PERF-15, which does not exist yet) |

## PERF-26 The post-layout deck carries the drawn devices — class: do

### Current code facts (plan corrections marked **stale**)

- `frontend/library/src/perf.rs:14–31` `Parasitics { caps, series, lod_inv_um, extracted, gate_offset_v }`,
  `#[derive(Clone, Debug, Default)]`. `deck` perf.rs:185–241 (private `fn`): `branch` closure :206 reads
  `par.series[di]`; a terminal with branch R gets node `{net}__{di}_{t}` and `Rpex_{di}_{t}` (:211–214); gate offset
  chains after it (:215–218, :226–228); the FET `extra(di)` closure :229–238 writes
  `" sa={:.4e} sb={:.4e}", s * 1e-6` (**plan said :137 / :115–127: stale**).
- `frontend/library/src/oppoint.rs:417–475` `flat_circuit_with`: FET card `X… D G S B model W= L= nf= m={sz.m}{extra(di)}`
  (:443–452), W/L in µm; diode card `D{inst} P N {model} area={w·l µm²}` (:465–468). `extra` is called for FETs only.
- `frontend/library/src/lib.rs`: `Epoch.extra` :1481–1483; `epoch()` already builds `netlist ∪ extra` as a `Cow`
  for signoff (:1642–1647, **plan omits this**); winner append :1142–1144 (**plan said 442–444: stale**);
  `parasitics()` :1716–1756, `n = self.netlist.devices.len()` :1720, placed macros
  `gr::place_macros(&cellgen::realize(..))` :1721 (cells zip `self.cells.devices_of`), `lod_inv_um` :1749–1754,
  struct literal :1755; `score_perf` :1837–1856 calls `perf::evaluate(self.netlist, &self.parasitics(epoch), ..)`
  :1845 (**plan said 878–893 / 719–757: stale**). Owner index convention = `member_pin` :1710–1714
  (`devices_of[c][owner]`). PERF-15 has not landed: no post-fill evaluation exists anywhere.
- `Figures.gate_ohm` (mosfet.rs:200–208) is the owner's whole gate: one finger's R over **all** the owner's drawn
  fingers in parallel (all `m` copies) — so it goes in series with the card's gate node as is, **not** divided by m.
  `Figures.sd` areas/perimeters are owner totals over all drawn copies — divide by the card's `m`.
- **New bug (out of the plan, blocks step 4):** `elaborate.rs:493–497` inserts the antenna diode with
  `model: String::new()`; its deck card is `DXDANT<n> <gnd> <net>  area=…`, which ngspice rejects
  ("could not find a valid modelname", measured below) → once diodes are simulated every such epoch reads
  "cannot simulate".

### Step 5 pinned now (ngspice batch, sky130 tt via `sky130.lib.spice`, `XM1 d g 0 0 sky130_fd_pr__nfet_01v8 W=1 L=0.15`, Vd=Vg=0.9, `show m.xm1.msky130_fd_pr__nfet_01v8 : capbd capbs id`)

| card params | capbd | capbs | id |
|---|---|---|---|
| none | 1.401e-16 | 2.015e-16 | 33.77 µA |
| `ad=100 pd=4` | 9.420e-14 | — | 33.77 µA |
| `ad=1e-10 pd=4e-6` (metres) | 1.401e-16 (no effect) | — | 33.77 µA |
| `as=2 ad=1.5 ps=6 pd=5` | 1.696e-15 | 3.095e-15 | 33.77 µA |
| `sa=100 sb=100` | — | — | 38.09 µA |
| `sa=0.5 sb=0.5` | — | — | 29.93 µA |
| `sa=5e-7 sb=5e-7` | — | — | 2.50 µA |
| `sa=1e-3 sb=1e-3` | — | — | 2.63 µA |

- `.option scale=1.0u` applies to AS/AD/PS/PD **and** SA/SB: both are read in µm(²). The OP variable that moves
  is `capbd`/`capbs`; `cbd` stays 2.61e-21 for every `ad` (**plan's `show m : cbd` is wrong**).
- So perf.rs:237's `s * 1e-6` hands BSIM4 an SA of ~1e-12 µm: every drawn LOD term today is saturated extreme stress
  (Id ÷ 13). Fix in the same commit (step 5 says so).
- Diode: `DXDANT5 0 n sky130_fd_pr__diode_pw2nd_05v5 area=0.2025` simulates (cd 1.90e-16 F; `area=1e3` → 9.41e-13 F,
  so diode area is µm² under scale too); the same card with no model aborts the run.

### Edits

1. `perf.rs` `Parasitics` gains (doc comments dense, units stated):
   ```rust
   /// Per device, `[AS, AD, PS, PD]` per SPICE instance: µm², µm² , µm, µm (the library's `.option scale=1.0u`
   /// scales them, measured: `ad=100` moves capbd 670×, `ad=1e-10` not at all); `None` = not drawn-derived.
   pub junction: Vec<Option<[f64; 4]>>,
   /// Per device, the drawn gate's poly + cut R, Ω, in series with the card's gate; `None`/0 = none.
   pub gate_ohm: Vec<Option<f64>>,
   ```
   Update the struct literal at lib.rs:1755 (all other constructors use `..Parasitics::default()`).
2. `perf.rs` `deck`:
   - `branch` (:206): for `t == "G"` add `par.gate_ohm[di]` (when `Some(r)`, `r > 0`) to the routed G branch R —
     both are in series between net and gate, so one `Rpex_{di}_G` of their sum, node `{net}__{di}_G`; the gate offset
     chain (:215–228) is unchanged.
   - `extra(di)` (:229–238): SA/SB written in µm: `format!(" sa={s:.4e} sb={s:.4e}")` (drop `* 1e-6`); then append
     `format!(" as={} ad={} ps={} pd={}", j[0], j[1], j[2], j[3])` when `par.junction.get(di).copied().flatten()` is
     `Some(j)` (`{}` Display so 2.0 prints `2`, matching the test). Fix the doc of `lod_inv_um`'s use if it mentions metres.
   - Make `deck` `pub(crate)` (lib.rs test below calls it).
3. `lib.rs`: new free fn next to `parasitics`:
   ```rust
   /// Per device `[AS, AD, PS, PD]` per SPICE instance from the placed cells' `Figures.sd` (CELL-19):
   /// owner `o` of cell `c` is `devices_of[c][o]`; S/D entries summed per device, ÷ the device's `m`
   /// (the card has `m` copies), nm² → µm² ÷ 1e6, nm → µm ÷ 1e3. `None` for a device no figure names.
   fn junctions(placed: &[Macro], devices_of: &[Vec<DeviceId>], netlist: &pnr_core::Netlist) -> Vec<Option<[f64; 4]>>
   ```
   (m from `dev.mos_size().map_or(1, |s| s.m)`; a terminal with no entry contributes 0.) And
   `fn gate_ohms(placed, devices_of, n) -> Vec<Option<f64>>` the same way from `Figures.gate_ohm` (no ÷ m).
   `parasitics()` takes `extra: usize` implicitly: size every per-device vector to
   `self.netlist.devices.len() + epoch.extra.len()` (`lod_inv_um`, `series`, `junction`, `gate_ohm`; inserted diodes
   get `None`/empty) and fills `junction`/`gate_ohm` from the `placed` it already builds (:1721).
4. `lib.rs` `score_perf` (:1845): simulate `netlist ∪ epoch.extra`: extract the Cow at :1642–1647 into
   ```rust
   /// `netlist` with dr's inserted devices appended (borrowed when there are none).
   fn with_extra<'a>(netlist: &'a pnr_core::Netlist, extra: &[pnr_core::Device]) -> std::borrow::Cow<'a, pnr_core::Netlist>
   ```
   and use it in `epoch()` (:1642) and `score_perf`. `solve` (:1142–1144) keeps its owned extend. PERF-15, when it
   lands, calls the same pair (`with_extra`, `parasitics`) for its post-fill evaluation — note it in PERF-15's card.
5. Diode model (needed for step 4 to simulate at all): `elaborate.rs:495` sets
   `model: crate::model_table(pdk).into_iter().find(|(_, k)| *k == pnr_core::DeviceKind::Diode).map(|(m, _)| m).unwrap_or_default()`
   — the deck's first diode row, the same default `verify::Pdk::reference_spice` (pdk.rs:402) gives LVS, so LVS is
   unchanged (sky130: `sky130_fd_pr__diode_pw2nd_05v5`). Check `model_table` returns deck row order; if not, read
   `pdk.deck.devices` in order exactly as `reference_spice`'s `first` does. The `antenna_diode.rs` test must stay green.
6. Report: when any FET's `junction` is `None` at the winner, `eprintln!("[perf] junction geometry: not drawn-derived
   for {k} of {n} FETs")` once in `solve` (no metadata field fits; skipped a new one).

### Tests

- `perf.rs` `junction_params_reach_the_card`: one-FET netlist (copy of the fixture in
  `the_deck_carries_branch_resistance_and_layout_stress` :760), `junction: vec![Some([2.0, 1.5, 6.0, 5.0])]`,
  `gate_ohm: vec![Some(305.8)]` → `d.contains(" as=2 ad=1.5 ps=6 pd=5")`, `d.contains("Rpex_0_G in__0_G in 305.8000")`,
  `d.contains("XM1 out in__0_G")`. With `series: vec![vec![("G".into(), 100.0)]]` also set →
  `Rpex_0_G in__0_G in 405.8000` (sum, one resistor).
- `perf.rs` existing `the_deck_carries_branch_resistance_and_layout_stress`: add
  `assert!(d.contains(&format!(" sa={s:.4e}")), "{d}")` (µm, fails with the old `* 1e-6`).
- `tests/perf_postlayout.rs` `scale_applies_as_the_deck_assumes` (`models()` skip): one nfet
  `sky130_fd_pr__nfet_01v8` W=1000 L=150 nm, D=G=`d`… use nets d,g,0; testbench
  `Vd d 0 0.9\nVg g 0 0.9\n.control\nop\nlet cbd = @m.xm1.msky130_fd_pr__nfet_01v8[capbd]\nlet idd = @m.xm1.msky130_fd_pr__nfet_01v8[id]\nprint cbd idd\n.endc`,
  specs `cbd`, `idd` (no bounds); via `evaluate(&nl, &par, &cfg, &[0])`:
  - `junction = [0, 100, 0, 40]` → `cbd > 100 × cbd(None)` (measured 1.4e-16 → ~1e-13);
  - `lod_inv_um = Some(0.02)` (S ≈ 100 µm, relaxed) → `idd > 0.8 × idd(None)` (measured 38.1 vs 33.8 µA; the
    current metres card gives ~2.5 µA, so this fails before the fix).
- `tests/perf_postlayout.rs` existing `branch_resistance_and_stress_reach_the_simulation`: assertion unchanged
  (`> 0.01` dB); re-measure and rewrite the "0.53 dB" / "0.0099 dB" comment numbers after the SA fix.
- `tests/perf_postlayout.rs` `junction_geometry_moves_the_ota_hf_gain` (acceptance, reported): ota netlist, bench =
  `BENCH` plus `meas ac ghf find vdb(vout2) at=1e8`; junction per FET `[w·0.29, w·0.29, 2(w+0.29), 2(w+0.29)]`
  (w µm per finger·nf, sky130 SD extension 0.29 µm) vs none; `eprintln!` both; assert `|Δghf| > 0.01` dB. If the
  measured Δ is below that, report the numbers — do not lower the threshold.
- `lib.rs` `an_inserted_diode_is_simulated`: one-FET netlist + `extra = [diode XDANT1 P=vss N=in, model from step 5
  on sky130.json]` → `perf::deck(&with_extra(&nl, &extra), &Parasitics::default(), &cfg, "", &cfg.scenarios()[0])`
  contains `"DXDANT1 vss in sky130_fd_pr__diode_pw2nd_05v5"`; `with_extra(&nl, &[])` is `Cow::Borrowed`.
- `lib.rs` `junctions_come_from_the_drawn_figures`: reuse the fixture loop at :3118–3128 for `ota`; place
  `cells.variants[c].alternatives[0]` macros; `junctions(..)` is `Some` for every FET, and for each FET
  `AD·m·1e6 == Σ Figures.sd D area of its owner` (exact within 1e-6), AS likewise; a device with `m = 4`
  (ota XM5) gets a quarter of its owner total.
- Command: `qcargo test -p library --lib perf` , `qcargo test -p library --lib junctions an_inserted`,
  `qcargo test -p library --test perf_postlayout`, `qcargo test -p library --test antenna_diode`,
  then `qcargo test -p library`.

### Acceptance

ota post-layout HF gain with vs without junction params reported (test above); CELL-19's "PERF's post-layout deck
carries AS/AD/PS/PD from `Figures`" holds via `junctions_come_from_the_drawn_figures` + `junction_params_reach_the_card`.
Out of scope, reported: PERF-15 post-fill evaluation (PERF-15 not landed); resistor dual-π (98-gap H06-17).
