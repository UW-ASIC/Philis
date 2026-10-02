# Plan 06 — Reliability and substrate (REL): EM, IR, self-heating, antenna, latch-up, guard rings, ESD, substrate noise

Plan ID prefix: **REL-NN**. Scope owner: the physics models and checks for electromigration (EM), IR drop, self-heating and thermal coupling, antenna ratios, latch-up, minority-carrier guard rings, ESD path rules and substrate noise/isolation. REL defines the deck keys and the rule/verify checks; RTE, PLC and CELL consume them.

Conventions. Code citations are `path:line` on the working tree of 2026-09-28 (HEAD `dad330c` plus uncommitted changes). `GP/…` = GPurify at the pinned rev, `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/`. `VOL/…` = `~/.volare/sky130A/`. Reference citations give the book/paper, section/equation and the reftext line range (`scratchpad/reftext/<file>.txt`), with the PDF page where the study docs recovered a value from the page image. Numbers marked *computed* are my arithmetic from cited inputs; nothing here was built or run. Study docs cited: `audit-02` (AR-), `audit-03` (AC-), `audit-05` (AT-), `audit-06` (AV-), `audit-01` (AA-), `audit-07` (AF-), `ref-00` (NOTES-), `ref-lienig-electromigration` (EM-), `ref-hastings-01/05/12/14/15` (H01-/H05-/H12-/H14-/H15-), `ref-charbon-substrate-noise` (SUB-), `ref-balasa-graeb-survey-part2` (BAL2-).

---

## 0. Scope and boundaries

### 0.1 What REL owns
- **EM model and check.** Terminal current inputs and their apportionment to pins. The per-segment and per-via check on final route geometry. Deck limits, temperature derating (the Ea/n fallback policy), Blech domain, and the via current-direction rule.
- **IR drop model.** A terminal-resolved drop on the routed conductor tree.
- **Temperature for reliability.** Junction temperature from θJA·P, the device self-heating term (Hastings eq. 5.6), a placement-independent self-heating bound, and the resistor self-heating width function.
- **Antenna model.** Per etch stage, per connected piece, with gate area per piece and diode credit exactly as the deck grants it.
- **Guard-ring policy.** Which device gets which ring type, role, tie net and merge key (REL-07). The ring enum is CELL-05's, the drawing is CELL-05/CELL-17's, and the aggressor/victim/injector tags are EXT-23's.
- **Operating-point voltage checks.** VGS/VDS ratings per model, reverse bias of bulk junctions, and a matched-pair aging screen (ΔV_DS, ΔV_GS).
- **Substrate balance.** The differential substrate-balance cost (REL-15). The kind-aware Isolation emission is EXT-23's; the substrate deck key is FLOW-06 step 7's.
- **Well-merge predicate** (REL-16) that CELL-22's `well_bridges` calls.
- **ESD.** The adiabatic width floor on user-declared ESD nets.

Not REL's any more (review of 2026-09-28, see "Cut or deferred items"): the LU deck rules and `tie_max_dist_nm` value (FLOW-05), tap reach (CELL-13), injector classification (EXT-23), the substrate-kind key and Isolation emission (FLOW-06, EXT-23), EM waveform classes and the substrate macromodel (deferred, no input data).

### 0.2 What REL consumes
| From | What | Used by |
|---|---|---|
| FLOW-05 | sky130 deck fixes: `ar.met3.1` sidewall 845 nm, LU.2/LU.2.1/LU.3 rules, `tie_max_dist_nm` 15 000, `pex rbody_po … height 326.2nm`, `Pdk::pex_f32_named` | REL-02 (T1), REL-13 |
| FLOW-06 step 3 | Sidecar table `em_derating {t_ref_k, ea_ev, n}` read by `Pdk::em_limit` | REL-05 |
| FLOW-06 step 7 | Sidecar table `substrate {kind, epi_um, rho_ohm_cm}` | REL-07 |
| FLOW-07 | `.subckt` ports kept in `pnr_core::Netlist` (AF-02) | EXT-23 → REL-07 |
| PERF-09 | `OpPoint.vgs_v`, `vds_v`, `vbs_v: Vec<Option<f64>>` (V, signed) and `Solution.op: Option<OpPoint>` | REL-10 |
| PERF-08 | Non-FET DC currents in `OpPoint::terminal_ua` (AF-03). Until it lands, REL-01 step 4 makes them `None` | REL-01, REL-13 |
| EXT-23 | Aggressor, victim and minority-injector tags (with carrier) per device | REL-07, REL-15, REL-16 |
| CELL-05 | `GuardRingType { Tap { in_well }, Ecgr, Hcgr }`; CELL-17 `post_cell::drawable(t, process)` | REL-07 |
| CELL | `Macro.units` with correct `owner`, `x/y` and `phi` for every MOS finger (already emitted, `kernel/cells/src/mosfet.rs:382-390`) | REL-01 |

### 0.3 What REL provides
| To | What |
|---|---|
| RTE | `Routes.terms` (per-net terminal rects with DC current) and the hard per-segment EM check (REL-03). `Limit::width_nm/cuts` and per-pin current shares (REL-01). Effective-cut rule (REL-12). ESD width floor (REL-17). RTE owns width reservation in search (AT-03), access-jog sizing (AT-06, RTE-14), the pin ampacity filter (EM-19, RTE-24), via-array orientation, chamfers (EM-25), redundant vias (H15-19), antenna repair (RTE-06) and skipping EM in the blind repair arm (AT-24). |
| CELL | Ring requests with role (REL-07), drawn by CELL-05/CELL-17. `resistor::self_heating_min_width_nm` (REL-13), called by CELL-23. `post_cell::may_share_well` (REL-16), passed to CELL-22's `well_bridges`. |
| EXT | `rings::plan` (REL-07), which replaces EXT-23's `substrate::rings`. |
| PLC | `SubstrateBalance` cost rule (REL-15). Thermal self term (REL-14). |
| MAT | The thermal field API (`kernel/core/src/thermal.rs`). REL-14 changes the self term that ThermalGradient reads. MAT owns ThermalGradient budgets (H15-58), weighted group temperature (AR-43) and the curvature term (MAT-14). |
| PERF | Report data: EM min margin and unknown counts, antenna agreement, ring count and area, voltage findings. PERF owns the bench columns and the certificate format (NOTES-03). |

Out of scope (owned elsewhere or YAGNI at block level): pad ring, seal ring, bondwires (H05-07, H05-32, H15-04); voltage-aware spacing and parasitic-channel checks (H01-02, H05-39, H15-36 — no HV devices are drawn, and sky130.deck omits hv.*, `GP/pdks/sky130.deck:654`); TDDB area scaling (H05-18/19); star/Kelvin topology (H15-33/34 → RTE); crossing-coupling budgets (H14-07, H15-29/30 → RTE/EXT); mission profiles (EM-15/16, see §5).

---

## 1. Audit digest (current state, verified in source)

### 1.1 EM
- **Weak hard rule.** `Electromigration` is hard (`frontend/library/src/lib.rs:1027`). It passes if the one segment with the most headroom (`kernel/analog/src/routing/em.rs:109-120`, `satisfied` `:128-130`) is wide enough for the largest single-terminal current (`lib.rs:1003-1015`). A trunk carrying ΣI, or a narrow neck, passes as `known` (AR-05).
- **The real per-segment check sits in Θ.** dr sizes each segment from tree branch currents (`backend/dr/src/lib.rs:897-934`, `:653-663`). Its shortfall goes to Θ (`em underwidth`, `:1852-1854`; `em cuts`, `:882-884`). The tiers are inverted.
- **Every pin gets the whole terminal current (AT-07).** In dr's `pin_ua` closure (`backend/dr/src/lib.rs:227-232`), each pin of terminal `T` gets the whole current. MOS cells emit one `d{k}:T` pin per diffusion region (`kernel/cells/src/mosfet.rs:458-481`), so an N-region terminal injects about N×I.
- **Currents are all-or-nothing.** `pin_currents` (`lib.rs:1035-1055`) returns empty if any device is unresolved, which switches EM sizing off everywhere (AF-25).
- **Unknown reads as zero.** `OpPoint::terminal_ua` returns `Some(vec![])` for every non-FET (`frontend/library/src/oppoint.rs:45-47`), so resistor, diode and BJT currents read as known zero (AF-03, NOTES-02).
- **Access geometry has no EM limit.** `em_limits` covers only `layers`/`cuts` (`frontend/library/src/elaborate.rs:248-262`, called at `lib.rs:280`). The pin-access layer and its cut (li/mcon on sky130) are split off by `routing_stack` (`elaborate.rs:219`), so the access cut is never checked. sky130 li has no deck EM limit (`pdks/sky130.json:99` source text); mcon has 0.36 mA/cut (`GP/pdks/sky130.deck:491`).
- **Too few rule slots.** `MAX_METALS = 8` (`em.rs:74`). Today `em_rules` fills slots with metals only (`lib.rs:999`, filter `ua_per_um > 0`), but REL-03's metals-plus-cuts rule needs 10 on sky130: met1–met5 and mcon–via4 carry deck limits (`GP/pdks/sky130.deck:491-495`); li has none.
- **Blech domain.** It is the net's whole same-layer run (`em.rs:115`; `dr/lib.rs:662`). This is conservative. No shipped deck or sidecar supplies `blech_limit` (grep of `GP/pdks/*.deck` and `pdks/*.json`; audit-02 L300), so the Blech path is dormant; generic_finfet states that it has no EM limits at all (`GP/pdks/generic_finfet.deck:434-436`).
- **DC only.** `em.rs:90`.
- **Derating math is correct but inert.**
  - `derate` implements Hastings eq. 15.25 and passes its 0.58 example (`em.rs:16-21, 216-229`). The cut factor is correctly divided (`em.rs:60-63`, per Lienig eq. 3.25–3.26, L4626–4660).
  - No shipped deck carries T_ref/Ea/n, so `EmLimit.derating` is `None` (`backend/verify/src/pdk.rs:542`; `GP/pdks/sky130.deck:491-495`; `GP/pdks/ihp_sg13g2.deck:789-790`).
  - One die temperature is used, equal to the simulation temperature (`elaborate.rs:245-247`; `OpConfig::temp_c` default 27 °C, `oppoint.rs:133`).
- **Signoff EM covers supplies only.** It gets one DC number per Supply/Ground net (`elaborate.rs:297-319`), which GPurify spreads uniformly over terminals (AV-09). The GPurify intent schema has no per-net voltages (`GP/crates/ingest/src/intent.rs:149-155`).

### 1.2 IR drop
- `IrDrop` charges the net's whole current across its worst BFS path (`kernel/analog/src/routing/ir.rs:32-36`, `stack.rs:94-146`). This over-reports when branches carry less current, and on meshes the bound holds only within ×2 (AR-21).
- `fed_resistance_ohm` (`stack.rs:163-177`) is used only by `CommonNodes` (`kernel/analog/src/routing/common_node.rs:43-47`); `IrDrop` does not use it. (AR-21 calls it unused; that is stale.)
- Budgets are 10 % of saturation headroom, or 1 % of the rail (`backend/annotator/src/ir.rs:14-27`).

### 1.3 Thermal / self-heating
- **Point-source model.** Semi-infinite point sources with k = 148 W/(m·K) fixed (`kernel/core/src/thermal.rs:14`). The self term is a proxy, `P/(2πk·max(hw,hh))` (`thermal.rs:38-45`); it is not Hastings eq. 5.6 (H05-04).
- **No link to EM.** No junction temperature (θJA·P) and no self-heating feed into EM derating (H05-01, EM-04).
- **Resistor width ignores current.** Resistor width is `max(unit_w, res_min_width)` (`kernel/cells/src/resistor.rs:81`) (H05-05).

### 1.4 Antenna
- **Per-stage connectivity is correct.**
  - `Stack::antenna` (`kernel/analog/src/routing/stack.rs:270-317`) handles connectivity per stage, and the jumper and lift repair works (H05-26, H05-27).
  - Sidewall thickness comes from the deck (`pdk.rs:586-613`).
- **Each piece is charged the whole net's gate area.** `gate_nm2` is the net total (`stack.rs:310`; producer `backend/annotator/src/extract.rs:33-46`). A piece that reaches only some gates reads too low, which is non-conservative (AR-13).
- **Diode credit is infinite and unconditional.** Any diode-marker shape on the net returns ratio 0 at every stage (`stack.rs:286-291`).
- **The deck gives no diode credit.** The sky130 deck's antenna rules are plain `antenna(...)` with no diode credit (`GP/pdks/sky130.deck:451-463`). Signoff therefore never credits the diodes that `elaborate::antenna_diodes` (`elaborate.rs:330-373`) inserts at `lib.rs:632-652`.
- **Result.** In the loop, a diode reads "fixed" while signoff still fails. This mechanism is consistent with the red test: dac4 fails `erc/ar.met2.1` at `feedback_iters=1` (AV-15; `benchmarks/tests/signoff_fixtures.rs:139`).
- **No `known`.** An unrouted net reads as satisfied (AR-44).
- **Deck error.** `ar.met3.1 … sidewall: 2000nm` (`GP/pdks/sky130.deck:459`) contradicts the deck's own `pex met3 thickness 845nm` (`:594`) (AV-17). It is conservative for the loop and wrong for signoff.

### 1.5 Latch-up and guard rings
- **sky130 has no latch-up rule.** The deck states latch-up is waiting (`GP/pdks/sky130.deck:669-670`). The foundry magic tech file states LU.2/LU.2.1/LU.3 < 15.0 µm (`VOL/libs.tech/magic/sky130A.tech:4165-4170`). gf180 has `tap_distance` DF.13/DF.14, 20 µm at 3.3 V (`GP/pdks/gf180mcu.deck:303-306`). ihp has `missing_tie` LU.a/LU.b, 20 µm (`GP/pdks/ihp_sg13g2.deck:595-596`).
- **Sidecar value is unsourced and unused.** `tie_max_dist_nm` is 3000 in sky130 (`pdks/sky130.json:85`), 20000 in gf180/ihp. It is only listed as required (`backend/verify/src/pdk.rs:1068`) and never read by a generator (AC-07, H01-07).
- **MOS cells have one tap strip.** It sits above the row (`kernel/cells/src/mosfet.rs:567-572`). Its farthest diffusion point is about `max_finger_width` = 10 µm away (`pdks/sky130.json:40`): inside the foundry's 15 µm, outside the sidecar's 3 µm.
- **Every FET in a non-glue block gets a ring.** The annotator requests a ring for each such FET, typed by polarity and tied to its bulk, with literal pitch, width and resistance (`backend/annotator/src/constraints.rs:66-85`).
- **Ring types are named for minority carriers but drawn as majority taps.** `post_cell` draws `Ecgr` as a p+ substrate tap and `Hcgr` as an n+ tap in n-well (`kernel/cells/src/post_cell.rs:315-323`). These are majority-carrier tap rings (Hastings §5.4.4 L15421–15446; §14.2.3 L43795–43799). The enum docs say the opposite (`kernel/analog/src/cell.rs:25-35`) (AC-06, AA-18, H05-54, H14-27).
- **Rings have no role.** No ring is placed around injectors. Rings merge on (net, type) only (`post_cell.rs:79-93`).
- **Dead fields.** `tap_pitch_nm` and `enclosure_complete` are never read (`cell.rs:14,20`; AR-28).
- **No injector or pin role.** Net roles are Signal/Supply/Ground/Clock (`backend/annotator/src/netrole.rs:11-16`), and the parser drops ports (AF-02) (H05-47, H14-01).
- **Correct today.** Ring bands are minimum width with contact rows sized against R (`post_cell.rs:355-367`), which matches Charbon §8.6.2 (SUB-29).

### 1.6 Substrate isolation
- **Fixed 4×epi rule.**
  - `emit::isolation` (`backend/annotator/src/emit.rs:250-288`) keeps each Clock-net device (`:271-273`) `4·t_epi` from each sensitive device.
  - It uses a nominal 2.5 µm epi when the deck has none (`:256`), which gives a cost-only 10 µm pull.
  - No deck carries `epi_thickness_nm` (`lib.rs:509`); every deck's `p_epi_thickness` = 3000 is a ring-depth default read by nothing but `REQUIRED_RULES` (`pdk.rs:1059`).
- **Wrong substrate model for sky130.** sky130 is bulk, where Charbon's plateau does not hold (the note at `emit.rs:252-255`; Charbon §8.2.2 L3320–3351) (SUB-32, AR-34).
- **Contradictory rules.** Isolation is emitted against the clocked tail's own pair, which also gets Proximity ≤ 5 µm (AA-13).
- **No substrate network, no ring credit, no differential balance** (SUB-03, SUB-43).

### 1.7 Operating-point voltages
- `OpPoint` carries `power_uw`, `id_ua`, `headroom_mv`, `gm_us` (plus `provenance` and `resolved`) only (`frontend/library/src/oppoint.rs:13-28`). There are no node voltages and no VGS/VBS.
- The deck's `vgs.*`/`vds.*` rules (`GP/pdks/sky130.deck:482-486`) skip for lack of intent (AV-29).
- No bulk-junction reverse-bias check exists (H01-01), nor any HCI/BTI screen for matched pairs (H05-33, H05-35).

---

## 2. Target: what "better than hand layout" means here

A hand-layout reliability review:
- inspects power plots of supply nets for pinch points and too few vias (Hastings §15.5 item 4, L48879–48886, PDF 823);
- relies on DRC for antenna and latch-up;
- runs latch-up and ESD checklists by hand (items 7–8, L48898–48918, PDF 823–824).

Philis must check every net that carries current, every segment and every via, every gate net at every etch stage and every device at its operating point. It must report numeric margins, and it must never report an unknown as a pass (NOTES-02).

| # | Metric | Target | Where measured |
|---|---|---|---|
| T1 | Signoff `erc/ar.*` rows per sky130 fixture at `feedback_iters = 1` | 0 on all 7 default fixtures (dac4 today: 1) | `benchmarks/tests/signoff_fixtures.rs` |
| T2 | Nets where signoff flags an antenna violation that the in-loop model passes | 0 | new agreement test (REL-02) |
| T3 | Per-segment EM width and per-via-group cut violations on final geometry, access jogs and pin cuts included, over every net with known terminal currents | 0 on bench fixtures; min(w/need) per net ≥ 1.0 reported | REL-03 rule in the route report (V) |
| T4 | Share of multi-terminal nets whose currents are known | reported per fixture; certification requires 100 % in signoff mode | metadata `missing`/unknown counts |
| T5 | IR drop, terminal-resolved | every `IrDrop` budget met; max drop (µV) reported | REL-04 |
| T6 | Latch-up tap distance: sky130 LU.2/LU.2.1/LU.3, gf180 DF.13/14, ihp LU.a/b | 0 on all fixtures (a guard, not REL work: FLOW-05 adds the rules, CELL-13 the tap reach; REL-07 must not raise it when it removes rings) | existing DRC baseline 0 in `benchmarks/tests/signoff_fixtures.rs:17-40` (LU rows are DRC rows) |
| T7 | Injector ring coverage | 100 % of EXT-23 minority-injector devices get a `Ring` request of role Injector of the type in the REL-07 table | `backend/annotator/src/tests.rs` (REL-07) |
| T8 | Rings without a role | 0; the OTA-class fixtures (no clock, no ports, no quiet ring net) draw 0 rings and bench area drops, with T6 still 0 | bench `area µm²`, ring count |
| T9 | Operating-point voltage findings (VGS/VDS rating, forward-biased bulk) | 0 on fixtures biased by a testbench; ΔV_DS/ΔV_GS per matched pair reported | REL-10 |
| T10 | Isolation rules between members of one block | 0 (owned and tested by EXT-23; REL only relies on it) | EXT-23 tests |
| T11 | Every REL check reports `known`/`unknown` counts | unknown count listed in the report and never counted as pass | metadata |

---

## 3. Work items

### REL-01 EM current inputs: per-pin shares, and unknown is not zero

Status: done in M0 (`e7db737`); it also does PERF-09 step 2 (C17): `pin_currents` keeps resolved members (`one_unresolved_device_keeps_the_others_known`).

- Priority: **P0**. Effort: **M**. Depends on: none. PERF-08 step 5 (non-FET cards and currents) later turns the unknowns created here into known values.
- **Supersedes PERF-09 step 2.** PERF-09 step 2 makes an unresolved member "contribute no pins". With dr's lookup, a pin missing from the table reads `0.0` (`backend/dr/src/lib.rs:229-231`, `.unwrap_or(0.0)`), so that version turns unknown into a known zero on exactly the nets it touches (NOTES-02). Land this item's step 3 instead; PERF-09's test `an_unresolved_device_leaves_other_pins_sized` still holds under it.
- **Why:** AT-07, AF-25 (second half: one unresolved device disables EM everywhere), AF-03 (non-FET currents read as known zero), NOTES-02, EM-07. Lienig §3.3.2 requires per-terminal current bounds (lower/upper), not one equivalent value per terminal, for correct segment currents (L3790–3836, PDF 83–84).
- **Current:**
  - `dr/lib.rs:227-232` looks up a pin's current by pin name and returns the whole terminal current for every pin of that name.
  - `lib.rs:1035-1055` returns `Vec::new()` when any device is unresolved.
  - `oppoint.rs:45-47` returns `Some(Vec::new())` for every non-FET.
  - `elaborate.rs:297-319` sums per net with `d.iter().flatten()`, so an unknown device reads as 0.
- **Change:**
  1. `kernel/core/src/macro.rs`: add
     ```rust
     /// Fraction of its member terminal's DC current each pin carries, parallel
     /// to `m.pins`. MOS S/D pins: fingers adjacent to the pin's region over the
     /// member's fingers, from `m.units` (owner, centre, S→D `phi`). Any other
     /// pin: `min(1, 2/n)` over the `n` pins sharing its name — an upper bound,
     /// since each finger touches one region of each terminal (n ≤ F) and a
     /// region touches at most two fingers (share ≤ 2/F ≤ 2/n).
     #[must_use]
     pub fn pin_shares(m: &Macro) -> Vec<f32>
     ```
     Algorithm:
     1. For each pin `i` whose name is `d{k}:S` or `d{k}:D`, set `w[i] = 0`.
     2. For each unit `u` with owner `k` and `u.phi != (0, 0)`, and for `(T, dir)` in `[("D", u.phi), ("S", (−u.phi.0, −u.phi.1))]` (`phi: (i8, i8)` is the S→D unit vector, `kernel/core/src/units.rs:26-27`): find the pin `j` named `d{k}:T` whose centre offset `Δ = c_j − (u.x, u.y)` has `Δx·dir.0 + Δy·dir.1 > 0` and the smallest `|Δx| + |Δy|`. Set `w[j] += 1`. No such pin: skip.
     3. For each group of pins with the same name and `Σw > 0`, set `share[i] = w[i] / Σw`. For every other pin, set `share[i] = min(1, 2 / n_name)`, where `n_name` counts the pins with that name. The bare-name pins `T` of member 0 (`frontend/library/src/lib.rs:1047-1049`) are their own group and get `min(1, 2/n)` too.
  2. `backend/dr/src/lib.rs:55`: change to `pub pin_ua: Vec<Vec<(String, Option<i32>)>>` and add, next to it, `pub pin_share: Vec<Vec<f32>>` (per placed cell, parallel to that cell's `pins`; empty inner vec = share 1.0 for every pin). In the closure at `:227-232`, find the pin index `p` with `position` instead of `find`, and return `Option<f32>` = `table value × pin_share[cell][p]`. The shares must come from the **unplaced** cell macro: `place_macro` turns and shifts pins but leaves `units` in the local frame (`kernel/core/src/macro.rs:56-57`), so `pin_shares` on a `placed` macro would mix frames. Pin order is preserved by `place_macro` (`macro.rs:57`), so compute `pin_shares` once per cell in the library (next to `pin_currents`, where `cfg.pin_ua` is filled at `lib.rs:309-310`) and fill `pin_share` there. A pin with no table entry (ring, antenna diode) is `Some(0.0)`; a `None` value stays `None`. At `:269` and `:421` (`node_ua`), skip a net whose terminals include any `None`. Such a net gets no EM sizing (the same behaviour as the empty table today).
  3. `frontend/library/src/lib.rs:1035`: change the signature to `fn pin_currents(netlist: &Netlist, devices_of: &[Vec<DeviceId>], draws: &[Option<Vec<(String, f64)>>]) -> Vec<Vec<(String, Option<i32>)>>`. For an unresolved device, emit `(d{k}:T, None)` (and `(T, None)` for member 0) for every terminal name in `netlist.devices[d].terminals`. Delete the all-or-nothing early return.
  4. `frontend/library/src/oppoint.rs:45-47`: a `Capacitor` returns `Some` with every terminal at `0.0`, because a capacitor carries no DC current. `Resistor`, `Diode`, `Npn`/`Pnp` and `Inductor` return `None` (unknown until PERF-08 step 5 simulates them, AF-03).
  5. `elaborate.rs:297-319` (`intent`): if any device on a Supply/Ground net has `None` draws, omit that net's current. GPurify then reports its EM rule as skipped instead of checking a wrong number.
  - Data flow: `OpPoint::terminal_ua` → `pin_currents` → `DetailedCfg.pin_ua` → dr `node_ua` and `Routes.terms` (REL-03).
- **Tests:**
  - `kernel/core/src/macro.rs` `pin_shares_follow_finger_adjacency`: member 0 has 4 units at x = 1000, 3000, 5000, 7000 nm, y = 0, with `phi` = (1,0), (−1,0), (1,0), (−1,0). Pins `d0:S` sit at x = 0, 4000, 8000 and `d0:D` at x = 2000, 6000 (all y = 0). Assert D shares `[0.5, 0.5]` and S shares `[0.25, 0.5, 0.25]` (exact).
  - `pin_shares_without_units_bound_by_two_over_n`: three pins named `d0:S` and no units. Assert each share = 2/3.
  - `backend/dr/src/lib.rs` `a_multi_finger_terminal_is_not_counted_per_pin`: a cell with two `d0:D` pins (4 fingers, as above) and a table value of +400 µA; a sink pin elsewhere at −400 µA. Assert the trunk branch current is 400 µA (today 800).
  - `frontend/library/src/oppoint.rs` `a_resistor_current_is_unknown_and_a_capacitor_is_zero`.
  - `frontend/library/src/lib.rs` `one_unresolved_device_keeps_the_others_known`.
- **Acceptance:**
  - For every MOS cell with units, Σ over a terminal's pins of the assigned currents equals the terminal current within 1 µA.
  - For other cells it is at most 2× (proved bound).
  - On rc_filter and dac4, nets that touch no resistor keep EM sizing (today switched off whenever one device is unresolved).
- **Risks / notes:**
  - Two-row MOS cells: the pin nearest in the S/D direction is in the same row, because the other row is farther in y. The cross-quad test at `mosfet.rs:990-1002` must still pass.
  - Merged cells where two members share one region carry two pins at one rect (`mosfet.rs:474-481`), and each member's share is computed separately. That is correct.

### REL-02 Antenna: in-loop model agrees with signoff

Status: done in M0 (`dac139f`). As built: `pnr_core::GatePin { at, dev, nm2 }` per net in `Routes::gates` (kernel/core/src/routes.rs:54, 72); diode credit is the deck's `antenna_electrical` `diode_bonus` only, on pieces touching a diode shape. Deferred to M1 (review finding 20): `antenna_in_loop_never_passes_what_signoff_fails` compares per fixture, not per net, because a signoff ERC row carries no net (`verify::Finding` is a point on the `gate` layer); the per-net form lands here once GPurify reports the net of an antenna row (00-MASTER-PLAN §6.4 Q7).

- Priority: **P0**. Effort: **M**. Depends on: none for steps 1–5 and 7. T1 on met3 also needs FLOW-05 step 2 (`ar.met3.1` sidewall 2000 → 845 nm in the vendored deck); T1 on dac4 may also need RTE-06 (repair on the drawn geometry).
- **Why:** AR-13, AR-44, AV-15, AV-17, H05-26, H05-28. Hastings defines the node ratio as metal of the node over the gate oxide of poly belonging to that node, per stage (§5.1.6, L13148–13157, PDF 228). Junction bleed is credited only as the process states it (L13215–13230, PDF 229). GPurify's `antenna_electrical` computes `ratio − credit·A_diode − bonus` and refuses `credit ≠ 0` with a diode layer (`GP/crates/check/src/erc/rules/antenna.rs:405-440`).
- **Current:** see §1.4. The test `benchmarks/tests/signoff_fixtures.rs::fixtures_sign_off_within_baseline` fails on dac4 (`erc/ar.met2.1:gate`) at `feedback_iters=1` (AV-15).
- **Change:**
  1. `kernel/core/src/routes.rs`: add
     ```rust
     /// A gate pin, the device it gates and that device's gate-oxide area.
     #[derive(Clone, Copy, Debug, PartialEq, Eq)]
     pub struct GatePin { pub at: Rect, pub dev: u32, pub nm2: i64 }
     ```
     Change `pub gates: Vec<Vec<GatePin>>` (`routes.rs:15`) and `gate_pins(&self, net) -> &[GatePin]` (`:35`).
  2. `backend/dr/src/lib.rs`:
     - `DetailedCfg` gets `pub gate_nm2: Vec<Vec<(String, u32, i64)>>`: per placed cell, `(pin name "d{k}:G", device id, W·L·fingers nm²)`.
     - `cell_metal` (`:1726-1750`) pushes `GatePin { at, dev, nm2 }` from that table. A pin missing from the table gets `nm2 = 0` and `dev = u32::MAX`.
     - The library fills the table next to `pin_ua` (`lib.rs:309-310`), using `annotator::gate_um2` × 1e6 (`backend/annotator/src/lib.rs:155-161`). This matches the annotator's convention (AR-46 stays FLOW's).
  3. `kernel/analog/src/routing/stack.rs`:
     - Replace `pub diode_layer: Option<u16>` (`:40`) with `pub diode: Option<DiodeCredit>`, where `#[derive(Clone, Copy, Debug, PartialEq)] pub struct DiodeCredit { pub layer: u16, pub bonus: f32 }`.
     - Change the signature to `pub fn antenna(&self, shapes: &[Shape], cell: &[Shape], gates: &[GatePin], fallback_nm2: i64) -> Option<(f32, f32)>`.
     - Per stage and piece:
       1. `reached` = the distinct `dev` of gates whose rect touches a piece rect.
       2. Skip the piece if `gates` is non-empty and `reached` is empty (today's behaviour).
       3. `gate = Σ nm2 over reached` (or `fallback_nm2` when `gates` is empty).
       4. `ratio = area / gate`.
       5. If `self.diode` is `Some(d)` and any net shape on `d.layer` overlaps a piece rect, then `ratio = (ratio − d.bonus).max(0)`.
     - Delete the early `return` at `:289-291`.
  4. `backend/verify/src/pdk.rs`: add `pub fn antenna_diode_credit(&self) -> Option<(LayerId, f32)>`. It returns the diode layer and `diode_bonus` of an `antenna_electrical` rule with `diode_credit == 0`, else `None`. `elaborate::stack` (`elaborate.rs:266-291`) sets `diode` from it; do not use `pdk.diode_marker()` for credit.
  5. `elaborate::antenna_diodes` (`elaborate.rs:330-340`) returns empty unless `pdk.antenna_diode_credit().is_some()`. A diode the deck does not credit fixes nothing at signoff and adds an LVS device.
  6. (Moved: the `ar.met3.1` sidewall fix, AV-17, is FLOW-05 step 2.)
  7. `kernel/analog/src/routing/antenna.rs`: add `fn known(self, r: &Routes) -> bool { !r.shapes(self.net).is_empty() }` (AR-44).
  8. `elaborate.rs:289`: `diode_layer: pdk.diode_marker().map(|l| l.0)` becomes `diode: pdk.antenna_diode_credit().map(|(l, bonus)| DiodeCredit { layer: l.0, bonus })`. Update the two test constructors that name `diode_layer` (`stack.rs:416`, `:504`).
- **Tests:**
  - `kernel/analog/src/routing/stack.rs` `a_piece_is_charged_only_the_gates_it_reaches`: two gates (1 µm² and 9 µm²) and a met2 piece of area A touching only the 1 µm² gate. Assert ratio = A/1 µm² (today A/10).
  - `diode_credit_is_the_decks_bonus_only`: `diode = Some({bonus: 50})`, ratio 120 → 70. `diode = None` with a diode shape present → 120.
  - `frontend/library/tests/antenna_diode.rs`: its deck transform must also rewrite each `antenna(gate, X; max_ratio: 0.001, …)` row as `antenna_electrical(gate, X; max_ratio: 0.001, diode: <diom layer>, diode_credit: 0, diode_bonus: 1000000)`, so the diode path is still exercised. The existing assertions stay.
  - New `benchmarks/tests/signoff_fixtures.rs::antenna_in_loop_never_passes_what_signoff_fails`: for each `BASELINE` fixture, run as `check` does (`signoff_fixtures.rs:71-84`, `feedback_iters = 1`). If `report.hard_violations` has any rule starting `erc/ar.`, assert `sol.metadata.routing.iter().any(|b| b.kind.ends_with("Antenna") && b.satisfied + b.unknown < b.total)` (`BudgetStatus`, `frontend/library/src/metadata.rs:18-38`; hard batches are reported there with `arm == Arm::Hard`, `metadata.rs:192`).
- **Acceptance:** T1 and T2. `fixtures_sign_off_within_baseline` is green on dac4.
- **Risks / notes:**
  - The in-loop sidewall area sums per-rect perimeters (`stack.rs:299-301`), while GPurify uses the merged perimeter (`antenna.rs:30-32` in GP). The loop therefore stays pessimistic, and T2 is deliberately one-directional.
  - GPurify subtracts `bonus` from the ratio of **every** gate net of an `antenna_electrical` row, whether or not the net reaches a diode (`GP/crates/check/src/erc/rules/antenna.rs:357-360, 438`; the diode area only multiplies `credit`). Step 3.5 grants the bonus only to pieces touching a diode shape, so the loop is stricter than signoff here too (safe for T2). With the test deck's `diode_bonus: 1000000`, signoff passes every gate net; `antenna_diode.rs` asserts only diode insertion and LVS (`frontend/library/tests/antenna_diode.rs:39-55`), so it is unaffected.
  - Without diode credit, sky130 relies on dr's jumper and lift repair (`dr/lib.rs:1486-1498, 1753-1788`). If dac4 still fails after this item, the remaining work is RTE's repair, and the failure is now visible as V.

### REL-03 EM: per-segment and per-via check on final geometry (hard)

Status: done in M0 (`76e7232`, `ab947a2`); the via grouping is transitive (amendment at step 4.3).

- Priority: **P0**. Effort: **M**. Depends on: REL-01.
- **Why:** AR-05, AT-06 (access jogs and pin cuts unchecked), AV-09 (signal nets never EM-checked), EM-09, EM-20, EM-29, EM-30, H05-12, H15-37, NOTES-48.
  - Lienig: each segment carries the sum of one side's terminal currents (eqs. 3.5–3.7, L3975–4052, PDF 87–88); w ≥ I/(J·h) (eq. 3.21, L4587–4617); n_via = ⌈I/I_cut⌉ (eq. 3.25, L4626–4660); a segment is the conductor between vias or branches (§4.3.1, L5370–5377).
  - Hastings: "a lead must meet electromigration rules at every point along its length"; one cannot neck down a lead (§15.4.4, L48738–48747, PDF 821).
- **Current:** §1.1.
- **Change:**
  1. `kernel/core/src/routes.rs`: add
     ```rust
     /// A routed terminal and the DC current it draws from its net, µA
     /// (+ into the device); `None` = unknown.
     #[derive(Clone, Copy, Debug, PartialEq)]
     pub struct Terminal { pub at: Rect, pub ua: Option<f32> }
     ```
     Add `pub terms: Vec<Vec<Terminal>>` and `pub fn terminals(&self, net: NetId) -> &[Terminal]`. dr fills `routes.terms` in absolute coordinates from `all_pins` and the REL-01 `pin_ua` (next to `(routes.cell, routes.gates)` at `dr/lib.rs:879`).
  2. `kernel/analog/src/routing/stack.rs`:
     - Make `port_graph` (`:183`) and `PortGraph` (`:323-326`) `pub(crate)`. Each link records its shape: `adj: Vec<Vec<(usize, f32, u32)>>`, where the `u32` is the shape index, or `u32::MAX` for a 0-Ω junction or terminal link. Set this in the four `link` calls: cut half-R `:214`, 0-Ω shape junction `:220`, terminal merge `:237`, along-shape `:247`.
     - Add `fn tree(&self, root: usize) -> Vec<Option<(usize, f32, u32)>>`: the parent link of every reached node in a least-R Dijkstra tree, reusing `from` (`:334-355`) and recording parents.
  3. New `kernel/analog/src/routing/current.rs`:
     ```rust
     /// DC current distribution of one routed net.
     pub struct NetFlow {
         /// Worst-case current through each of the net's `shapes`, µA.
         pub shape_ua: Vec<f32>,
         /// Largest DC drop between any two points of the net, µV (REL-04).
         pub drop_uv: f32,
     }
     /// `None` when a terminal's current is unknown, no terminal touches the
     /// routes, or a terminal is unreached (open).
     pub fn net_flow(stack: &Stack, shapes: &[Shape], terms: &[Terminal]) -> Option<NetFlow>
     ```
     Algorithm (Lienig eqs. 3.5–3.7; a DC point, so lo = hi):
     1. Return `None` if any `ua` is `None`. Build the port graph over `(shapes, term rects)`.
     2. Root = the node of terminal 0. Build the least-R spanning tree. Return `None` if a terminal is unreached.
     3. Inject `ua_t` at each terminal node. `total = Σ ua_t`. Set `bal = |total| ≤ 1e-3·max|ua_t|` (a balanced net; else a port whose position is unknown supplies `−total`).
     4. Walk the tree post-order to get subtree sums `S(n)`. The tree edge into `n` carries `I = |S(n)|` if `bal`, else `max(|S(n)|, |total − S(n)|)` (the rule dr uses, `dr/lib.rs:891-893`, valid for any port position).
     5. For a chord (a non-tree link `a–b`), `I` = the maximum tree-edge current on the tree path `a…b` (a parallel branch carries at most what the path carries).
        - ponytail: a general-mesh bound; a nodal solve replaces it if a test ever disproves it.
     6. `shape_ua[i]` = max `I` over links whose shape index is `i`.
     7. `drop_uv` = the weighted diameter of the tree with edge weight `R_e·I_e` (Ω·µA = µV), by the double sweep already used in `path_resistance_ohm` (`stack.rs:110-145`).
  4. `kernel/analog/src/routing/em.rs`:
     - Change `pub const MAX_LAYERS: usize = 16;` (rename from `MAX_METALS`, `:74`).
     - The struct becomes
       ```rust
       #[derive(Clone, Copy)]
       pub struct Electromigration {
           pub net: NetId,
           /// `(layer, limit)` per metal **and cut** the deck limits (pin-access
           /// layer and cut included); unused slots `u16::MAX`.
           pub limits: [(u16, Limit); MAX_LAYERS],
           pub stack: Option<&'static Stack>,
       }
       ```
     - Delete `current_ua` and `best()`. Add `fn check(self, r: &Routes) -> Option<f32>` = the worst residual:
       1. `all = [r.shapes(net), r.cell_metal(net)].concat()`; `flow = net_flow(stack, &all, r.terminals(net))?`. The cell metal is passed so that terminals joined only through a cell's own strap are reached (else `net_flow` returns `None` and the net reads unknown); this is the edit RTE-24 step 4 asks for, done here once. Only indices `0..r.shapes(net).len()` (the routed shapes) are checked below; cell-internal straps are CELL's (EM-19, RTE-24).
       2. For every metal shape `i` whose layer has `ua_per_um > 0`: `need = lim.width_nm(flow.shape_ua[i], long_side_nm)`. The Blech domain is the shape's own long side (EM-30). [UNVERIFIED: this bounds the Lienig segment only if no two same-net, same-layer shapes abut collinearly; such a pair is one segment (§4.3.1, L5370–5377: a segment ends only at vias or branches) longer than either rect, so the shape side can under-bound it. Dormant today: no shipped deck supplies `blech_limit`.] `have = short_side_nm`. If `have < need`, the residual is `(need − have)/need`.
       3. Group cut shapes on layers with `ua_per_cut > 0` by (cut layer, the set of metal shapes on rank ± 1 they overlap). Group current = max `shape_ua` of its members. `need = lim.cuts(I)`, `have = group size`. If `have < need`, the residual is `(need − have)/need`.
          **Amended (M0, recorded by the review panel):** as built (kernel/analog/src/routing/em.rs `check`), two cuts of
          one layer join a group when they share **any** landing shape below and any above, transitively (union-find),
          not when their landing sets are equal: `dr`'s array cuts spread past the original cut's pads, so each lands on
          its own pad plus the common trunk, and equal sets would split one array into single cuts. Known false pass:
          cuts under two unjoined same-layer, same-net shapes merge into one group (`n` = all of them, `I` = the
          largest member's), so a group that is short of cuts can pass. It needs same-net shapes closer than a cut
          is wide, which dr does not draw today; split groups per (below, above) landing pair if it ever does.
     - `known = stack.is_some() && check().is_some()`. `satisfied = !known || residual ≤ 0`. `residual = check().unwrap_or(0)`. `headroom = 1 − max(need/have)`.
  5. `frontend/library/src/lib.rs:991-1028` (`em_rules`):
     - Fill `limits` from `em` including cuts, and set `stack: ann.process.stack`.
     - Delete the worst-terminal computation (`:1003-1016`).
     - Push `("Electromigration", "a routed or pin-access layer has no deck EM limit (unchecked)")` once to `problem.missing` when any stack or pin-access layer lacks a limit (sky130 li). `missing` holds `&'static str` pairs (`metadata.rs:81`), so the layer name is not interpolated; the bench can list the layers from `em`.
  6. `elaborate.rs:248` and `lib.rs:280`: call `em_limits(pdk, &layers ∪ {pin_access.0}, &cuts ∪ {pin_access.1}, …)`, so that sky130's mcon gets its 0.36 mA/cut limit (`GP/pdks/sky130.deck:491`).
  7. dr keeps its own `em underwidth` and `em cuts` Θ as search pressure (`dr/lib.rs:1852-1854, 882-884`). The V comes from this rule in `problem.routing.hard`. RTE must exclude `Electromigration` from the blind repair arm (AT-24), because width is not a search resource yet (AT-03).
- **Tests** in `kernel/analog/src/routing/em.rs` use a stack with met1 (sheet 0.125 Ω/□), via (4.5 Ω) and met2, and limits met1 2800 µA/µm, via 290 µA/cut, met2 2800 µA/µm (the sky130 values, `GP/pdks/sky130.deck:491-492`):
  - `a_narrow_access_jog_fails_even_if_the_trunk_is_wide`: A(+500 µA) and B(−500 µA). A met1 trunk 1000 nm wide, plus a 140 nm × 400 nm jog to B. need = 500·1000/2800 = 178.6 nm (*computed*). Assert violated, residual ≈ (178.6−140)/178.6 = 0.216. The old rule passes this case.
  - `a_trunk_carries_the_sum_of_its_branches`: terminals +300, +200 on two branches and −500 at the root. Assert trunk need 178.6 nm and branch needs 107.1 / 71.4 nm.
  - `a_via_group_needs_ceil_i_over_i_cut`: 700 µA through a group of 2 cuts at 290 µA/cut. Assert need 3, residual 1/3.
  - `an_unknown_terminal_current_is_unknown`: one `ua: None` → `!known`.
  - `an_unbalanced_net_charges_the_larger_side`: +300 and +200 and no sink. Assert the edge between them carries 300 (max(|300|, |500−300|)).
  - `parallel_paths_are_bounded_by_the_tree_path`: a loop of two equal met1 wires between A(+1000) and B(−1000). Assert both shapes are checked at 1000 µA.
- **Acceptance:** T3 and T4 on `cargo run --release -p benchmark --bin bench local`. Every EM violation is listed per net in `violations.txt`. The bench constraint table's `Electromigration` row counts known and unknown nets.
- **Risks / notes:**
  - This check turns today's hidden Θ shortfalls (including AT-06 necks) into V. Until RTE reserves width (AT-03) and sizes access jogs (AT-06), fixtures may show V > 0. That is a real reliability defect surfaced, not a regression; the epoch ranking still prefers fewer V.
  - Cost: `port_graph` is O(k²) per net, the same class as `path_resistance_ohm`, which already runs per repair trial (AT-12).

### REL-04 IR drop, terminal-resolved on the same current tree
- Priority: **P1**. Effort: **S**. Depends on: REL-03.
- **Why:** AR-21, NOTES-49, BAL2-46. ΔV = Σ I_e·R_e along a path is Ohm's law per branch; neither source states it as one equation. R_e comes from Lampaert eq. 2.33 (R_i = ρ_□·l_i/w_i per rectangle, lampaert.txt L2003; cited in `ir.rs:9`) and I_e from Lienig eqs. 3.5–3.7 (L3975–4052). The worst path × total current over-reports when branches carry less current.
- **Current:** `ir.rs:32-36`: `current_ua × path_resistance_ohm(shapes)`.
- **Change:** in `kernel/analog/src/routing/ir.rs`, `drop_uv` becomes:
  - if `r.terminals(net)` is non-empty: `net_flow(st, shapes, terms).map(|f| f.drop_uv)`;
  - else: today's bound, kept for `gr`'s shape-only routes and existing tests.

  Keep the `current_ua` field: it feeds the fallback bound, and dr's IR repair picks the nets to reprice by this rule's residual (`dr/lib.rs:531-538`), which reads it. The price weight itself comes from dr's own `node_ua` (`:539-549`), not from `current_ua`.
- **Tests** in `ir.rs`:
  - `a_dead_branch_adds_no_drop`: A(+1000 µA) and B(−1000 µA) joined by 20 □ of met1 at 0.125 Ω/□ (2.5 Ω), plus a 100 □ branch to C (0 µA) from the midpoint. Assert the new drop = 2500 µV. The old bound = 1000 × (1.25 + 12.5) Ω = 13 750 µV (*computed*) when the trunk is drawn as two 10 □ rects meeting at the branch; one 20 □ rect gives 1000 × (2.5 + 12.5) = 15 000 µV, because `path_resistance_ohm` charges whole shapes (`stack.rs:90-91`).
  - The existing test `drop_is_current_times_route_resistance` must still pass (no terminals → old path).
- **Acceptance:** T5. Bench IR residuals fall on rails with tapered branch currents. Nets that touch no resistor keep known status.
- **Risks / notes:** an unbalanced supply net uses the any-port bound from REL-03 step 4, which over-reports only when the port position is unknown. FLOW's port positions (interface.json) would tighten it later.

### REL-05 EM temperature: junction temperature, self-heating bound, rating data
- Priority: **P1**. Effort: **S**. Depends on: FLOW-06 step 3 (the `em_derating` key and its reader) for steps 4–5; steps 1–3 and 6 have none. REL-14 shares its formula.
- **Why:** H05-01, H05-04, H15-38, EM-03, EM-04, EM-42.
  - Hastings eq. 5.1 T_J = T_A + θ_JA·P_D (L11904–11932, PDF 212).
  - Hastings eq. 5.6 ΔT = ln(4L/W)/(π·κ·L)·P, κ_Si ≈ 1.3 W/cm/°C (L12021–12038, PDF 213).
  - Derating D = exp[(Ea/(n·k))(1/T_j − 1/T_0)] (Hastings eq. 15.25, L48776–48791, PDF 821; Lienig eq. 3.11, L4299–4304, PDF 94).
  - Fallback Black parameters: Cu Ea 0.9 eV, n 1.1–1.3; Al Ea 0.7 eV, n 2 (Lienig L1241–1245, PDF 31).
- **Current:**
  - `lib.rs:280` passes `temp_c + 273.15`.
  - `pdk.rs:542` derating is `None` on every deck.
  - `thermal.rs:38-45` self term is a proxy.
- **Change:**
  1. `kernel/core/src/thermal.rs`: add
     ```rust
     /// Hastings eq. 5.6: rise of a W×L (L ≥ W) source over its own area, mK.
     pub fn self_rise_mc(p_uw: i32, w_nm: i32, l_nm: i32, k_w_per_m_k: f32) -> f32
     /// Σ_j self_rise_mc: no device rises more, whatever the placement.
     pub fn rise_bound_mc(p_uw: &[i32], footprint_nm: &[(i32, i32)], k_w_per_m_k: f32) -> f32
     ```
     - Formula: `ln(4L/W)·P_µW·1e6 / (π·k·L_nm)` mK, with L = max side and W = min side.
     - Proof of the bound: each mutual term `P_j/(2πk·r)` with `r ≥ max(hw_j, hh_j) = L_j/2` is ≤ `P_j/(πk·L_j)` < `ln(4)·P_j/(πk·L_j)` ≤ eq. 5.6's self term (since L ≥ W gives ln(4L/W) ≥ ln 4).
  2. `frontend/library/src/oppoint.rs:103-121`: add `pub theta_ja_c_per_w: Option<f64>` to `OpConfig` (default `None` = 0 K rise; no package θ_JA is given anywhere in ref/ or the decks). `oppoint.rs` is PERF's file (PERF-09); this one field is added here for REL, next to `temp_c`.
  3. `lib.rs:280`: compute
     `t_em_k = temp_c + 273.15 + θ_JA·(total_power_uw·1e-6) + rise_bound_mc(cell power, smallest-variant bbox per cell, 148.0)/1e3`,
     using `CellSpace` built at `lib.rs:275`. Pass it to `em_limits`.
     - [Correction] The bound's proof holds for the footprint the layout actually uses. "Smallest-variant bbox" by area does not guarantee the largest `ln(4L/W)/L` or the smallest `L` (e.g. a 1 × 100 µm variant gives ln(400)/100 = 0.060 µm⁻¹, a 10 × 12 µm one ln(4.8)/12 = 0.131 µm⁻¹, *computed*). Use, per cell, the variant that maximises `self_rise_mc` and minimises `max(w, h)`, or take the max over variants.
  4. `backend/verify/src/pdk.rs:525-559`, on top of FLOW-06 step 3 (which reads the sidecar table `em_derating {t_ref_k, ea_ev, n}` only when all three are non-null): when a deck rule has no `(reference_temperature, activation_energy_ev, current_exponent)` and the table has `t_ref_k` but a null `ea_ev` or `n`, use `derating = Some((t_ref_k, ea_ev.unwrap_or(0.9), n.unwrap_or(1.1)))` and set `derating_assumed = true`.
     - The fallback is Lienig's Cu values at the most conservative stated n. It gives the largest Ea/n of the stated families, so the strongest derating above T_ref. (Hastings gives Cu ≈ 0.8 eV, n = 1, H05-10, L12316–12347: Ea/n 0.8 < 0.9/1.1 = 0.82, so the Lienig pair is the stronger of the two.)
     - Add `pub derating_assumed: bool` to `EmLimit` (`false` when the deck or a complete sidecar table supplies all three).
  5. Sidecar values for FLOW-06's `em_derating.t_ref_k` (REL supplies the numbers, FLOW-06 the key; `ea_ev` and `n` stay `null` on every deck, §5 Q1):

     | deck | `t_ref_k` | source |
     |---|---|---|
     | sky130 | 363.15 (90 °C) | `VOL/libs.ref/sky130_fd_sc_hd/techlef/sky130_fd_sc_hd__nom.tlef:92,119` ("Iavg_max at Tj = 90oC"); FLOW-06 step 3 already sets it |
     | gf180mcu | 358.15 (85 °C) | `GP/pdks/gf180mcu.deck:478` ("Electromigration at 85 C") |
     | ihp_sg13g2 | 378.15 (105 °C) | `GP/pdks/ihp_sg13g2.deck:599` ("11 years at 105 C") |

  6. `frontend/library/src/metadata.rs:115-125` `BiasSummary`: add `pub em_temp_k: f32` and `pub em_derate: &'static str` (`"deck"`, `"sidecar+fallback Ea/n"` or `"none"`).
- **Tests:**
  - `kernel/core/src/thermal.rs` `hastings_self_heating_example`: `self_rise_mc(100_000, 25_000, 25_000, 130.0)` ≈ 13 600 mK ± 100. Hastings quotes 14 °C for 25 µm at 100 mW (L12021–12038).
  - `rise_bound_holds_for_any_placement`: two 1000 µW devices of 10 × 10 µm. Assert bound = 596 mK ± 2 (*computed*: 2·ln4·1e-3 W/(π·148·10e-6 m)). For 50 random placements, `rises_mc` ≤ bound.
  - `kernel/analog/src/routing/em.rs` `fallback_derating_at_125c`: `derate(398.15, 363.15, 0.9, 1.1)` ≈ 0.100 ± 0.002 (*computed*: exp[(0.9/(1.1·8.617e-5))·(1/398.15 − 1/363.15)]).
  - `backend/verify/src/pdk.rs` `sky130_em_rating_is_90c_with_fallback_parameters`: `em_limit(met1).derating == Some((363.15, 0.9, 1.1))` and `derating_assumed`.
- **Acceptance:**
  - With `temp_c = 125`, sky130 met1's limit is derated to 2800 × 0.100 ≈ 281 µA/µm (*computed*).
  - With `temp_c = 27` and µW-level power, the factor is 1 (never credited cooler, `em.rs:17`).
  - `BiasSummary` reports `em_temp_k` and its provenance.
- **Risks / notes:** the fallback over-derates an Al process by up to 3.7× at 125 °C (Al 0.7/2 gives 0.374 vs 0.100, *computed*). §5 Q1 asks for the metal family.

### REL-06 (cut — duplicated by FLOW-05 and CELL-13)
- Moved to "Cut or deferred items", C1. The ID is kept so cross-references in plan-03 and plan-08 stay valid.
- What REL still requires: T6 = 0 on every fixture after REL-07 removes rings. No extra test is needed: LU findings are DRC rows, and every sky130 fixture's DRC baseline is 0 (`benchmarks/tests/signoff_fixtures.rs:17-40`, checked at `:133-138`), so `fixtures_sign_off_within_baseline` fails on any LU hit once FLOW-05 adds the rules.

### REL-07 Guard-ring policy by role (injector, aggressor, victim)
- Priority: **P1**. Effort: **M**. Depends on: CELL-05 (enum `Tap { in_well }`, `Ecgr`, `Hcgr`; deletes `tap_pitch_nm`, `enclosure_complete`, `Hbgr`, `Ebgr`), EXT-23 (per-device aggressor, victim and minority-injector tags; `SubstrateKind`). Without EXT-23 the policy still runs with aggressor = "touches a Clock-class net" and no injectors, which already removes today's blanket rings (T8). CELL-17 draws `Ecgr`/`Hcgr`; until it lands, those arms draw nothing (CELL-05 step "return no ring plus a `debug_assert!`"), so the annotator must not request them before CELL-17 (gate: `RingInputs.ecgr_drawable`, `hcgr_drawable`).
- **Why:**
  - Findings: AC-06, AA-18, AR-28, AC-23, H05-54, H14-27, H14-09, H12-39, SUB-28, SUB-30, SUB-34, SUB-39.
  - Hastings on majority vs minority rings:
    - "Substrate, tank, and well contacts cannot stop the flow of minority carriers" (L15421–15446, PDF 264–265).
    - Rings of substrate contacts around NMOS are majority-carrier rings (§14.2.3, L43795–43799).
    - Rings go on injectors: "there are likely to be many more potentially vulnerable devices than there are injectors" (§14.2, L43641–43645).
    - Guard rings "can be placed only around a few devices — usually those that potentially inject" (§12.2.9, L36896–36899).
    - A useful HCGR needs a well bottom ≥ 10× its middle doping (§14.2.4, L43842–43846); CMOS designers ring PMOS with NMoat backgate contacts instead (L43871–43874).
  - Charbon on ring placement and nets:
    - A ring on the internal ground with large L_gr "is worse than no ring" (§8.6.1, L3583–3598).
    - A ring tied to the shared substrate-bias pad increases noise (App. D.3, L4473–4482).
    - Epi substrates: rings give only 7–10 dB (§8.5.1, L3494–3527).
    - A ring around the injector is ~20 dB better against ring-net noise (§8.6.2, L3709–3720).
  - Measurable benefit: T7, T8 (today every FET of every non-glue block gets a ring, `backend/annotator/src/constraints.rs:66-85`).
- **Conflict resolved here (EXT-23).** EXT-23's `substrate::rings` gives every FET/BJT member of a Moderate+ set a victim `MajorityTap` ring, falling back to the only ground with a `shared_ring_return` diagnostic, and expects 4 rings on `ota5t`. That contradicts SUB-30 (a ring on a shared return can be worse than none) and T8. REL-07's `rings::plan` replaces EXT-23's `rings()`; EXT-23 keeps `tag` and `isolation`. EXT-23's ota5t ring assertion must change to 0 (flagged in §5 Q12; plan-01 is not edited here).
- **Current:** §1.5.
- **Change:**
  1. `kernel/analog/src/cell.rs` (after CELL-05): add to `GuardRingRequirement`
     ```rust
     /// Why the ring exists; rings of different roles never merge (post_cell).
     pub role: RingRole,
     ```
     and
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum RingRole { Injector, Aggressor, Victim }
     ```
  2. `kernel/cells/src/post_cell.rs:86` (`clusters`): `same_class` becomes `a.connection_net == b.connection_net && a.ring_type == b.ring_type && a.role == b.role`, so a victim ring never shares an aggressor's return (H14-10, SUB-35). Rename nothing else; the drawing is CELL-05/CELL-17's.
  3. New `backend/annotator/src/rings.rs`:
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum Carrier { Electrons, Holes }
     /// Plain per-device data, so the policy is table-testable without ports or a PDK.
     pub struct RingInputs<'a> {
         pub netlist: &'a Netlist,
         pub aggressor: &'a [bool],             // EXT-23 Switching/Capacitive/ImpactIonization; fallback: touches a Clock-class net
         pub victim: &'a [bool],                // EXT-23 victims (Moderate+ set members, Reference-gated devices)
         pub injector: &'a [Option<Carrier>],   // EXT-23 MinorityElectron/MinorityHole; all None without ports
         pub substrate: SubstrateKind,          // EXT-23 enum, from FLOW-06 step 7 `substrate.kind`
         pub quiet_ring_net: Option<NetId>,     // step 4
         pub highest_supply: Option<NetId>,     // Supply-class net with the highest OpConfig rail voltage; ties → lowest NetId
         pub ground: Option<NetId>,             // the Ground-class net of lowest NetId
         pub min_ring_width_nm: i32,            // sidecar `min_guard_ring_width` (sky130 420, pdks/sky130.json:64)
         pub ecgr_min_width_nm: Option<i32>,    // sidecar `ecgr_min_width_nm` (CELL-17 key); absent on every shipped deck
         pub ecgr_drawable: bool,               // cells::post_cell::drawable(Ecgr, process) (CELL-17), passed by the library
         pub hcgr_drawable: bool,               // cells::post_cell::drawable(Hcgr, process); false on sky130 (retrograde well not declared)
     }
     pub fn plan(i: &RingInputs) -> (Vec<GuardRingRequirement>, Vec<(&'static str, &'static str)>)
     ```
     Policy, first matching row wins, at most one ring per device, `bulk` = the device's `B` terminal net (no `B` → no ring):

     | # | Device | `ring_type`, `role` | `connection_net` | `min_width_nm` | `shareable` | Source |
     |---|---|---|---|---|---|---|
     | 1 | `injector == Some(Electrons)`, `ecgr_drawable`, `highest_supply` is Some | `Ecgr`, Injector | `highest_supply` | `max(min_ring_width_nm, ecgr_min_width_nm.unwrap_or(0))` | false | Hastings §14.2.3 L43826–43827 (N-well ECGR to a supply, not ground), §14.2.1 L43654–43655; H14-34 (large-current rings not merged) |
     | 2 | `injector == Some(Holes)`, `hcgr_drawable`, `ground` is Some | `Hcgr`, Injector | `ground` | `min_ring_width_nm` | false | §14.2.4 L43842–43846 |
     | 3 | `injector == Some(Holes)` otherwise (PMOS) | `Tap { in_well: true }`, Injector | bulk | `min_ring_width_nm` | false | §14.2.4 L43871–43874 |
     | 4 | `injector == Some(Electrons)` otherwise | `Tap { in_well: false }`, Injector | bulk | `min_ring_width_nm` | false | majority ring as the fallback; see missing note |
     | 5 | `aggressor && !victim`, `substrate != EpiOnLowRes` | `Tap { in_well: pmos }`, Aggressor | bulk | `min_ring_width_nm` | true | H14-09 (L43072–43086), SUB-34 |
     | 6 | `victim`, some device matches rows 1–5, `substrate != EpiOnLowRes`, `quiet_ring_net` is Some | `Tap { in_well: pmos }`, Victim | `quiet_ring_net` | `min_ring_width_nm` | true (merges per matched pair, SUB-39) | SUB-30, SUB-28 |
     | 7 | anything else | none | — | — | — | H12-39; cells carry their own taps (CELL-13) |

     - `max_ring_resistance_mohm` stays 100 000 (today's literal, `constraints.rs:81`); sizing from injector current waits for a test-current input (§5 Q7).
     - `missing` entries: row 1 or 2 skipped for lack of a net → `("GuardRing", "injector ring: no supply/ground net")`; row 1 with `ecgr_min_width_nm == None` → `("GuardRing", "ECGR width rule not given: collection efficiency unknown")`; row 4 used → `("GuardRing", "ECGR not drawable: electron injector has a majority ring only")`; any victim without a ring because `quiet_ring_net` is None while an aggressor/injector exists → `("GuardRing", "victim rings: no quiet ring return (SUB-30)")`.
  4. `quiet_ring_net` resolution (in `annotate`): `AnnotationConfig` (`backend/annotator/src/netrole.rs:59-75`) gains `pub quiet_ring_net: Option<String>`, resolved by case-insensitive name. If `None`: the Ground-class net whose lowercase name contains `avss`, `vssa` or `agnd` (EXT-23's analog-root rule) **and** on which no aggressor device has a terminal; else `None`.
  5. Wiring: delete the ring loop at `constraints.rs:66-85` (`assemble` then emits no rings) and call `rings::plan` from `annotate` (`backend/annotator/src/lib.rs`, after net classes and EXT-23 tags exist, next to `constraints::assemble` at `:144`), appending its requirements to `constraints.guard_rings` and its notes to `missing`. `ProcessNumbers` (`netrole.rs:79-108`) gains `min_ring_width_nm: i32`, `ecgr_min_width_nm: Option<i32>`, `ecgr_drawable: bool`, `hcgr_drawable: bool`; `library::annotation` fills them from the sidecar and `cells::post_cell::drawable`.
- **Tests** (`backend/annotator/src/tests.rs`, building `RingInputs` directly unless noted):
  - `an_ota_without_clocks_or_ports_gets_no_rings`: annotate `benchmarks/fixtures/ota.spice` with default config → `constraints.guard_rings.is_empty()`.
  - `clocked_devices_get_aggressor_tap_rings`: a netlist with NMOS `M1` gated by `clk`, `aggressor[M1] = true`, no quiet net → exactly one ring, `Tap { in_well: false }`, Aggressor, on M1's bulk; a victim device gets none, and `missing` has the SUB-30 note.
  - `a_quiet_net_turns_on_victim_rings`: same plus `quiet_ring_net = Some(vssq)` and two victims → two Victim rings on `vssq`, `shareable == true`.
  - `an_electron_injector_gets_a_supply_tied_ecgr`: NMOS with `injector = Some(Electrons)`, `highest_supply = Some(VDD)`, `ecgr_drawable = true`, `min_ring_width_nm = 420`, `ecgr_min_width_nm = None` → `Ecgr`, Injector, net VDD, `min_width_nm == 420`, `shareable == false`, and the "ECGR width rule not given" note.
  - `a_hole_injector_falls_back_to_a_well_tap`: PMOS, `Some(Holes)`, `hcgr_drawable = false` → `Tap { in_well: true }`, Injector, on bulk.
  - `epi_substrate_draws_no_aggressor_or_victim_rings`: `substrate = EpiOnLowRes`, aggressor + victim + quiet net → no rings.
  - `kernel/cells/src/post_cell.rs` `rings_of_different_roles_never_merge`: two adjacent shareable requests, same net and type, roles Aggressor and Victim → two clusters.
- **Acceptance:** T7 and T8. OTA-class fixtures (`ota`, `ota_constrained`, `tt_ota`) draw 0 rings; bench `area µm²` for each decreases (report the delta); DRC 0, LVS MATCH, T6 = 0 (`large_fixtures_sign_off_within_baseline` green).
- **Risks / notes:**
  - Removing default victim rings drops per-FET substrate pinning. The cells' own tap strips (CELL-13) keep latch-up covered, and Charbon/Hastings do not support rings without an aggressor.
  - Neither EXT nor CELL may re-emit rings from `constraints.rs`.

### REL-08 (cut — duplicated by EXT-23)
- Moved to C2. Injector classification (pin-connected diffusions, and diffusions reached through less than 50 kΩ, Hastings §14.2 L43629–43638 and §15.5 item 8 L48898–48918) is EXT-23 tag 1 (`MinorityElectron`/`MinorityHole`, `policy.inj_series_ohm` = 50 kΩ). REL-07 consumes it as `RingInputs.injector`. The ID is kept for cross-references.

### REL-09 (cut — duplicated by EXT-23, FLOW-06 step 7 and FLOW-04)
- Moved to C3. The kind-aware, same-block-skipping `Isolation` emission is EXT-23's `substrate::isolation`; the `substrate` sidecar key is FLOW-06 step 7; `p_epi_thickness` becoming non-required is FLOW-04 step 5. T10 is measured by EXT-23's tests. The ID is kept for cross-references.

### REL-10 Operating-point voltage reliability checks
- Priority: **P1**. Effort: **M**. Depends on: PERF-09 step 5 (`OpPoint.vgs_v`, `vds_v`, `vbs_v: Vec<Option<f64>>`, V, signed; `Solution.op: Option<OpPoint>`). Step 1 and its test have no dependency.
- **Why:** H05-17, H01-01, H01-15, H05-33, H05-35, H14-23, AV-29.
  - Isolation junctions must stay reverse-biased, and a PMOS backgate ≥ its source (Hastings §2.6.1, L5202–5215, PDF 94; §4.2.3, L10110–10113).
  - Operating vs blocking ratings (Table 4.4, L10006–10026, PDF 179).
  - Matched devices at unequal V_DS (HCI) or unequal PMOS V_GS (NBTI) drift apart (§5.3.1, L13736–13761; §5.3.4, L14176–14181).
  - The sky130 deck already states the ratings: `vgs.01v8 … max: 1.95V`, `vds.01v8 … 1.95V`, `vgs.g5v0 … 5.5V`, `vds.d10v5 … 11V` (`GP/pdks/sky130.deck:482-486`). GPurify cannot receive per-net voltages (`GP/crates/ingest/src/intent.rs:149-155`).
- **Current:** §1.7.
- **Change:**
  1. `backend/verify/src/pdk.rs`: add
     ```rust
     pub struct FetLimit { pub model: String, pub vgs_max_mv: Option<f32>, pub vds_max_mv: Option<f32> }
     /// Every `gate_oxide`/`drain_source` rule: its models and max voltage.
     pub fn fet_voltage_limits(&self) -> Vec<FetLimit>
     ```
     One `FetLimit` per model name per rule, merged by model (a model named by both a `gate_oxide` and a `drain_source` rule gets both fields). The deck's `let` lists (`GP/pdks/sky130.deck:479-481`) arrive expanded to full model names such as `sky130_fd_pr__nfet_01v8`. Read the repeated `("model", ParamValue::Model)` params and `max_voltage` (engine names, `GP/crates/ingest/src/deck/kinds.rs:570-587`; parsed as `ParamSrc::Model` at `parse.rs:1433-1449`, interned to `ParamValue::Model` at `GP/crates/ingest/src/deck/mod.rs:447`).
  2. New `frontend/library/src/reliability.rs`:
     ```rust
     pub struct PairAging { pub a: DeviceId, pub b: DeviceId, pub dvds_mv: f64, pub dvgs_mv: f64 }
     pub fn voltage_findings(netlist: &Netlist, op: &OpPoint, limits: &[FetLimit],
                             pairs: &[(DeviceId, DeviceId)]) -> (Vec<Violation>, Vec<PairAging>, usize /* FETs with unknown voltages or no limit */)
     ```
     Rules per FET with known voltages:
     1. `|vgs|` above the model's `vgs_max` → `pnr_core::report::Violation { rule: format!("rel/vgs:{name}"), margin: (over_mv).ceil() as i64 }` (`margin: i64`, `kernel/core/src/report.rs:32-37`). Same for `vds`. The limit is looked up by exact `Device::model` string; a FET whose model has no `FetLimit` is counted as unknown, never as a pass (T11).
     2. Bulk forward bias, tolerance 1 mV (numerical only):
        - NMOS: `vbs > 1e-3` or `vbs − vds > 1e-3` → `rel/bulk_forward:{name}`.
        - PMOS: `vbs < −1e-3` or `vbs − vds < −1e-3`.
     3. For each 2-device leaf of kind DiffPair, CurrentMirror or Load (`backend/annotator/src/block.rs:23-36`), emit `PairAging` with `|Δvds|` and `|Δvgs|` in mV. This is report-only: the sources give no threshold (H05-33, H05-35).
  3. `frontend/library/src/lib.rs:1289` (`signoff`): when `sol.op` is `Some`, append (1) and (2) to `report.hard_violations` (after the undrawable loop at `:1295-1300`). `Solution` (`lib.rs:82-101`) gains `pub pairs: Vec<(DeviceId, DeviceId)>`, filled in `run` from the annotator's 2-device leaves of kind DiffPair, CurrentMirror or Load. `MetadataReport` gains `pub aging: Vec<(String, String, f64, f64)>` (device names, ΔV_DS mV, ΔV_GS mV) and `pub voltage_unknown: usize`.
     - These checks are layout-independent, so they run once per signoff and not per epoch; ranking is unaffected.
- **Tests:**
  - `backend/verify/src/pdk.rs` `sky130_fet_ratings_come_from_the_deck`: `sky130_fd_pr__nfet_01v8` vgs 1950 mV and vds 1950 mV; `sky130_fd_pr__nfet_g5v0d10v5` vgs 5500 mV and vds 11 000 mV; `sky130_fd_pr__esd_nfet_g5v0d10v5` vgs 5000 mV (`sky130.deck:485`).
  - `frontend/library/src/reliability.rs` `a_gate_over_its_rating_is_a_violation`: `sky130_fd_pr__nfet_01v8` at vgs 2.5 V → one `rel/vgs:` violation, margin 550.
  - `a_forward_biased_nmos_bulk_is_a_violation`: NMOS vbs +0.2 V, vds 0.9 V → one `rel/bulk_forward:`; the same NMOS at vbs 0 → none; a model with no limit → `voltage_unknown == 1`.
  - `pair_aging_reports_the_vds_difference`: 0.9 V vs 0.6 V → 300 mV.
- **Acceptance:** T9 on fixtures with a testbench bias. On a probe bias (AF-04) the findings carry the probe provenance and never certify.
- **Risks / notes:** GPurify's engine unit for `Dim::Voltage` is mV (`GP/crates/ingest/src/deck/kinds.rs:68-69`: `mV` shift 0, `V` shift 3; applied in `parse.rs:1596-1634`), so `max: 1.95V` reads as 1950. The pdk test pins it.

### REL-11 (deferred — no input data)
- Moved to C4 with its full specification. No plan produces per-terminal avg/RMS/peak currents (grep of plans 01–08 for RMS/transient terminal currents: none), and with DC currents the RMS term cannot bind (sky130 RMS limits exceed the DC ones). The ID is kept.

### REL-12 Effective via cuts by current direction
- Priority: **P2**. Effort: **S**. Depends on: REL-03. RTE consumes it (array orientation).
- **Why:** H05-14, EM-37, H15-39, AT-19.
  - Hastings: only faces toward the current conduct; of two vias at a lead end only the front face of the front via conducts (§5.1.3, L12452–12485, PDF 219).
  - Hastings: a via array at a 90° corner loads the inside via most (§15.4.4, L48818–48824).
  - Lienig: vias in series along the wire carry unequal current (§4.6.4, L6234–6279).
- **Current:** a group's cut count is its size (`em.rs:65-70`; `dr/lib.rs:760-803` fills the whole overlap).
- **Change:** in REL-03 step 4.3, `have = min over the group's two metal shapes of n_front(metal)`, where `n_front` = the largest number of group cuts sharing one coordinate along that metal's long axis (a row perpendicular to its current). Toggle via sidecar/config `em_front_row_cuts: bool`, default true.
  - A metal shape with `w == h` has no long axis: its `n_front` is the group size.
- **Tests:** `em.rs` `a_deep_array_counts_its_front_row`: six via cuts at x ∈ {0, 500, 1000} nm × y ∈ {0, 500} nm under a horizontal met1 lead (w 10 000 × h 800 nm) and a square met2 pad (1500 × 1500 nm). `n_front(met1)` = 2 (two cuts per x column), `n_front(met2)` = 6. Assert have = 2 and, at 700 µA and 290 µA/cut, need 3 → violated, residual 1/3.
- **Acceptance:** RTE's arrays are oriented across the current, and bench `em cuts` is 0.
- **Risks / notes:** this is conservative, and square arrays at L-turns need more cuts. RTE may extend one wire past the corner to make the overlap collinear (Lienig Fig. 4.29, L6355–6359).

### REL-13 Resistor self-heating width floor
- Priority: **P2**. Effort: **S**. Depends on: PERF-08 step 5 (resistor currents), FLOW-05 steps 3–4 (`pex rbody_po` row, `Pdk::pex_f32_named`). CELL-23 calls the function (its `W_heat` term).
- **Why:** H05-05, H15-45, H01-20. Hastings eq. 5.8: W_min = I_max·√(t_ox·R_s/(κ_ox·ΔT)), with κ_ox ≈ 0.011 W/cm/°C, design ΔT ≈ 5 °C and never more than 50 °C (L12044–12073, PDF 214).
- **Current:** `kernel/cells/src/resistor.rs:81` ignores current.
- **Change:**
  - `kernel/cells/src/resistor.rs`: `pub fn self_heating_min_width_nm(i_ua: f32, sheet_ohm: f32, t_ox_nm: f32, dt_k: f32) -> i32`, with κ_ox = 1.1 W/(m·K) (Hastings's 0.011 W/cm/°C).
    Formula: `W[nm] = i_ua·1e-6 · sqrt(t_ox_nm·1e-9 · sheet_ohm / (1.1 · dt_k)) · 1e9`, rounded up; `0` when any input is ≤ 0.
  - `t_ox_nm` (the oxide under the body, the heat path to the substrate), in this order: sidecar `res_tox_nm` (CELL-23's key; absent on every shipped deck), else `pdk.pex_f32_named(<body layer>, "height_nm")` for the resistor's body layer (sky130 after FLOW-05 step 3: `rbody_po` 326.2 nm, whose geometry columns FLOW-05 copies from `poly_c`, `GP/pdks/sky130.deck:582`), else `None` and CELL-23 skips the term. `poly_c` itself excludes the body (`poly not poly_rs`, `:106`), so today's deck has no body row.
  - `dt_k` from sidecar `res_self_heat_dt_k`, default 5 (Hastings's design value).
  - CELL-23 applies `W' = max(…, W_heat)` with the value-preserving `L'`; the per-resistor current comes from `OpPoint::terminal_ua` (`P` terminal magnitude), passed through `cellgen` as the power is (`lib.rs:275`).
- **Tests:** `resistor.rs` `hastings_eq_5_8`: I = 100 µA, R_s = 2000 Ω/□, t = 326.2 nm, ΔT = 5 K → 1089 nm ± 5 (*computed*); I = 1 mA → 10 891 nm.
- **Acceptance:** every resistor's modeled ΔT ≤ 5 K, reported.
- **Risks / notes:** `sheet_ohm` must be the body's own sheet R (FLOW-05 step 4 `device_sheet_ohm(model)`); `res_high_po`/`res_xhigh_po` have none there (FLOW-05 OQ-3), so their floor is skipped, not guessed. The 326.2 nm height is the poly level's, used as a proxy for the body's oxide.

### REL-14 Thermal field: Hastings eq. 5.6 self term
- Priority: **P2**. Effort: **S**. Depends on: REL-05 (shared function). MAT is told the ThermalGradient numbers change.
- **Why:** H05-04, AR-24 (the self-term part only). Eq. 5.6 is the book's closed form for a device's own rise (L12021–12038).
- **Current:** `thermal.rs:38-45` uses `r = max(hw, hh)`.
- **Change:** in `rise_at_mc`, when `i == j`, replace the self term `P/(2πk·r_floor)` with `self_rise_mc(p, 2·min(hw,hh), 2·max(hw,hh), K_SI_W_PER_M_K)` (adding it would double-count, and the acceptance value below is eq. 5.6 alone). Mutual terms are unchanged.
  - Finite die thickness and image sources (AR-24) are not planned: derived, not in ref/.
- **Tests:** `thermal.rs` `the_self_term_is_eq_5_6`. The existing isotherm tests (`thermal.rs:144-170`) must still pass.
- **Acceptance:** a powered 25 µm square device reads ≈ 11.9 K at 100 mW with k = 148 (*computed*).
- **Risks / notes:** ThermalGradient budgets are MAT's; only the self-heating contribution of a powered matched device changes.

### REL-15 Substrate balance of differential pairs
- Priority: **P2**. Effort: **S**. Depends on: EXT-23 (aggressor tags; until it lands, aggressor = a device touching a Clock-class net, as `emit.rs:271-273` does today). PLC-19 (1) was cut in favour of this item.
- **Why:** SUB-43, SUB-44. An injector on the pair's symmetry axis keeps it perfectly balanced; on the pair axis is the worst case (Charbon §8.3.1, L3382–3399, PDF 131–132). A 5 % mismatch of receiver substrate C collapses differential isolation (§8.3.2, L3410–3427).
- **Current:** no such term (`kernel/analog/src/placement/symmetry.rs:7-14` mirrors positions only).
- **Change:**
  - `kernel/analog/src/placement/isolation.rs`: add `#[derive(Clone, Copy)] pub struct SubstrateBalance { pub aggressor: Target, pub a: Target, pub b: Target }`, cost-only.
  - `cost = 4e-3·(d_ca − d_cb)²`, distances in nm, where `d_c·` are Euclidean distances between `Layout::centre` (`kernel/core/src/layout.rs:89`) of the aggressor and of `a` / `b`. This is the same scale as `Isolation::cost` (`isolation.rs:23-26`).
  - `satisfied = true` (no threshold is given). Implement `retarget`.
  - Emitted next to `Isolation`, in the placement cost arm, for every aggressor × 2-device DiffPair leaf not in the aggressor's block: in EXT-23's `substrate::isolation` once it exists, in `emit::isolation` (`backend/annotator/src/emit.rs:263-288`) before that.
- **Tests:** `isolation.rs` `an_aggressor_on_the_bisector_costs_nothing`: a at (−5000, 0), b at (5000, 0), aggressor at (0, 10 000) → cost 0. `off_axis_costs`: aggressor at (−20 000, 0) → d_ca 15 000, d_cb 25 000, cost 4e-3·1e8 = 4e5 (*computed*).
- **Acceptance:** on a clocked comparator fixture, final |d_ca − d_cb| is reported per pair and decreases vs. the rule off.
- **Risks / notes:** cost-only; PLC's move set decides how far it can act.

### REL-16 Shared-well merge legality
- Priority: **P2**. Effort: **S**. Depends on: EXT-23 (injector and aggressor tags). CELL-22 owns the `well_bridges` signature and calls the predicate.
- **Why:** H14-03, H14-13, H14-22, H01-05.
  - Hastings's three questions before merging devices into one well or tank: does anything inject, does anything debias the region, does it mix noisy with sensitive (§14.1.5, L43595–43612).
  - Output transistors get their own backgate region (§14.1.4, L43526–43547).
- **Current:** `well_bridges` merges any facing same-bulk nwells (`post_cell.rs:387-438`).
- **Change:**
  - `kernel/cells/src/post_cell.rs`: add `pub struct CellFlags { pub injector: bool, pub noisy: bool, pub sensitive: bool }` and `pub fn may_share_well(a: CellFlags, b: CellFlags) -> bool { a.injector == b.injector && !((a.noisy && b.sensitive) || (b.noisy && a.sensitive)) }`.
  - CELL-22 changes `well_bridges` (`post_cell.rs:387`) to take `can_share: &dyn Fn(usize, usize) -> bool`. The library (call site `frontend/library/src/lib.rs:609`) builds `flags: Vec<CellFlags>` per placed cell (OR over the cell's members: `injector` = EXT-23 minority-injector tag, `noisy` = EXT-23 aggressor tag, `sensitive` = EXT-23 victim tag) and passes `&|i, j| may_share_well(flags[i], flags[j])`.
- **Tests:** `post_cell.rs` `may_share_well_table`: (inj, –) vs (–, –) → false; (–, noisy) vs (–, sensitive) → false; (–, noisy) vs (–, noisy) → true; (inj) vs (inj) → true. The bridging behaviour itself is CELL-22's `a_forbidden_pair_is_not_bridged`.
- **Acceptance:** 0 bridges joining an injector cell to a non-injector.
- **Risks / notes:** placement-time well sharing (H01-05) is PLC's.

### REL-17 ESD-path adiabatic width floor
- Priority: **P2**. Effort: **S**. Depends on: REL-03.
- **Why:** H14-56, H05-06, H15-46, H05-24.
  - Hastings eq. 14.6: A = √(ρ·τ·I_pk²/(2·C_V·ΔT)), with τ = 225 ns, ΔT typically 50 °C, and I_pk ≈ V_HBM/1.5 kΩ (the HBM network's 1.5 kΩ, Fig. 5.5, L61905; the book rounds 2 kV to 1.3 A, L13065–13066 and PDF 772; the code uses `hbm_v / 1500` exactly, 1.333 A).
  - Table 14.3: Al ρ 2.7 µΩ·cm and C_V 2.42 J/°C/cm³; Cu 1.7 and 3.45. This gives 6.5 µm² for Al and 4.3 µm² for Cu (L45870–45932, PDF 772–773).
- **Current:** there is no ESD net class. The deck has topological `esd.pad` only (`GP/pdks/sky130.deck:475-476`).
- **Change:**
  - `library::Config` gains `pub esd: Option<EsdSpec>`, with `pub struct EsdSpec { pub hbm_v: f32, pub nets: Vec<String> }` (user-declared pad nets).
  - Sidecar `metal_family: "al" | "cu"` selects Table 14.3 values (§5 Q1).
  - `analog::routing::stack::Layer` gains `pub thickness_nm: f32`, filled in `elaborate::stack` (`elaborate.rs:275-287`) from `pdk.pex_f32(l, "thickness_nm").unwrap_or(0.0)`.
  - `pub fn esd_area_um2(hbm_v: f32, rho_uohm_cm: f32, cv_j_per_k_cm3: f32) -> f32` in `em.rs` = `sqrt(ρ[Ω·cm]·225e-9·(hbm_v/1500)²/(2·C_V·50))·1e8` (cm² → µm²).
  - `Electromigration` gains `esd_area_um2: f32` (0 = none), set only for nets named in `EsdSpec.nets`. In `check`, every routed metal shape of the net with `thickness_nm > 0` needs `short_side_nm ≥ esd_area_um2·1e6 / thickness_nm`; residual `(need − have)/need` like step 4.2 of REL-03. A metal with thickness 0 is unknown for this term.
- **Tests:** `em.rs` `hbm_2kv_on_sky130_met1_needs_18_6um`: Al (ρ 2.7 µΩ·cm, C_V 2.42), 2000 V → `esd_area_um2` = 6.68 ± 0.01 µm²; met1 t = 360 nm (`GP/pdks/sky130.deck:590`) → need 18 557 nm ± 30 (*computed*; at the book's rounded 1.3 A the same formula gives 6.51 µm², matching Hastings's 6.5 µm²). Cu (1.7, 3.45) → 4.44 ± 0.01 µm² (*computed*; book 4.3 at 1.3 A).
- **Acceptance:** 0 ESD-net width violations when `esd` is set.
- **Risks / notes:**
  - Pad-to-clamp path resistance ≤ 2 Ω (Hastings §5.1.5, L13065–13069) and CDM clamp proximity (H14-52) need pads and clamp recognition, which a block has not got (§5 Q8).
  - The inputs are Al/Cu metal values; a silicide cross-section uses its own row of Table 14.3.

### REL-18 (deferred — no deck data)
- Moved to C5 with its full specification. No shipped deck or sidecar gives a substrate ρ stack or package inductance (§5 Q3), so the fit cannot run; EXT-23's cost-only pull stands meanwhile. The ID is kept.

## 4. Milestones

| Milestone | Items | External prerequisites | Exit criteria |
|---|---|---|---|
| M1 Correct reliability evidence | REL-01, REL-02, REL-03 | none for the code; FLOW-05 step 2 (met3 845 nm) and possibly RTE-06 for T1 on dac4 | every test named in REL-01–03 green under `cargo test --workspace`; `antenna_in_loop_never_passes_what_signoff_fails` green (T2); `fixtures_sign_off_within_baseline` green once FLOW-05 is in (T1); `Electromigration` is in `problem.routing.hard` with metals and cuts, and `cargo run --release -p benchmark --bin bench local` prints its known/unknown/violated counts per fixture (T3, T4) |
| M2 Temperature and IR | REL-04, REL-05, REL-14 | FLOW-06 step 3 for REL-05 steps 4–5 | REL-04/05/14 tests green (T5); `BiasSummary.em_temp_k` and `em_derate` printed on every op run; existing `thermal.rs` isotherm tests still green |
| M3 Rings by role, voltage checks | REL-07, REL-10 | CELL-05; EXT-23 (tags); CELL-17 (for `Ecgr` requests); PERF-09 (for REL-10 steps 2–3) | T8: 0 rings on `ota`, `ota_constrained`, `tt_ota` with `large_fixtures_sign_off_within_baseline` green (DRC 0 incl. LU, LVS MATCH); T7 on the REL-07 injector tests; T9 on a testbench-biased fixture |
| M4 Refinements | REL-12, REL-13, REL-15, REL-16, REL-17 | PERF-08 (REL-13), FLOW-05 steps 3–4 (REL-13), CELL-22/23 (consumers), EXT-23 (REL-15/16) | each item's tests green; M1–M3 exit criteria still hold |

Dependency order inside REL (acyclic): REL-01 → REL-03 → {REL-04, REL-12, REL-17}; REL-05 → REL-14; REL-02, REL-07, REL-10, REL-13, REL-15, REL-16 depend on no other REL item. Bug fixes first: REL-01 (current over-count, unknown read as zero), REL-02 (red dac4 test, non-conservative gate area) and REL-03 (a hard rule that passes necked leads) are M1.

---

## 5. Open questions (each with a proposed default)

1. **Metal family and Black's parameters per deck.** Neither the decks nor ref/ state Ea and n for sky130, gf180 or ihp (`GP/pdks/ihp_sg13g2.deck:789-790`). *Default:* the conservative fallback of Lienig Cu, 0.9 eV and n 1.1 (REL-05), flagged `derating_assumed`. If the owner confirms aluminium (for example, "sky130 BEOL is Al-Cu"), set FLOW-06's `em_derating.ea_ev` = 0.7 and `n` = 2 (Lienig L1241–1245) and `metal_family: "al"` (REL-17).
2. **Dedicated quiet ring return.** Charbon says a ring on a shared node can be worse than none (SUB-30). *Default:* victim rings only on a user-declared `quiet_ring_net` or an analog-root ground no aggressor touches (REL-07 step 4). Philis does not invent a new port.
3. **Substrate kind and ρ stack.** sky130 "bulk" rests on the code note at `emit.rs:252-255`, not on a ref/ document. gf180 and ihp are unknown. *Default:* FLOW-06 step 7 sets sky130 = bulk; others null (EXT-23 treats them as `Unknown`, cost-only); REL-18 stays deferred.
4. **Deck edits.** Owned by FLOW-04/FLOW-05 (vendored decks under `pdks/decks/`). REL has no deck edit left.
5. **Antenna diode credit on sky130.** The magic tech file carries diode PWL terms (AV-17, `VOL/libs.tech/magic/sky130A.tech:4912`), but GPurify refuses `diode_credit ≠ 0`. *Default:* no credit and no diode insertion on sky130 (REL-02). Revisit when the deck states a credit GPurify accepts.
6. **Hard EM before width planning.** REL-03 may produce V > 0 on fixtures until RTE reserves width (AT-03) and sizes access jogs (AT-06, RTE-14). *Default:* keep it hard; report per net; do not hide the check in Θ.
7. **Ring resistance from injected current.** H14-34 and H05-49 want rings sized for 100–200 mA; the 0.3 V debias target is H05-50's (100 mA / 0.3 V → R_V ≤ 3 Ω) and H14-34's recipe. *Default:* keep 100 Ω (`constraints.rs:81`) until a config field `latchup_test_ma` exists (Hastings §5.4.2: "a typical test applies currents of 100 mA for 50 ms", L14975–14981); not planned this round.
8. **ESD at block level.** Pad-to-clamp R ≤ 2 Ω and CDM clamp placement need pads and clamp recognition. *Default:* only REL-17's width floor on user-declared nets.
9. **HCI/BTI thresholds.** None are given in the sources. *Default:* report ΔV_DS/ΔV_GS per matched pair (REL-10), with no pass/fail.
10. **ECGR width.** Hastings ties the NMoat ECGR width to P-epi thickness or retrograde P-well depth, and gives no width rule for an N-well ECGR (§14.2.3 L43806–43809, L43826). The earlier proxy `n_well_depth` (2000 nm, unsourced, `pdks/sky130.json:68`) is dropped. *Default:* CELL-17's `ecgr_min_width_nm` key, absent on every deck → deck minimum ring width plus a `missing` note (REL-07).
11. **Latch-up loop-gain margin (Hastings eqs. 12.32–12.33, L36838–36899).** It needs parasitic β and ring efficiency η, which are not given. *Default:* not planned; tap distance (FLOW-05, CELL-13) and injector rings (REL-07) stand in.
12. **EXT-23 ring policy conflict.** EXT-23 (plan-01) emits victim `MajorityTap` rings on every Moderate+ set and expects 4 rings on `ota5t`; REL-07 emits none without an aggressor and a quiet return (SUB-30, H12-39). *Default:* REL-07's `rings::plan` replaces EXT-23's `rings()`, and EXT-23's ota5t ring assertion becomes 0. Needs the plan-01 owner's sign-off.
13. **PERF-09 step 2 conflict.** PERF-09 step 2 drops an unresolved member's pins, which dr reads as 0 µA. *Default:* REL-01 step 3 (emit `None`) replaces it.
14. **BJT emitter–base reverse bias (H05-37).** Needs V_EBO per model; neither the decks nor the sky130 PNP model file (`VOL/libs.tech/ngspice/sky130_fd_pr__model__pnp.model.spice`, no `bv`/`vebo` parameter) give it. *Default:* deferred (C7).

---

## Cut or deferred items

| # | Item | Status | Reason and new owner |
|---|---|---|---|
| C1 | REL-06 latch-up deck rules, sky130 `tie_max_dist_nm` 15 000 with source, verify test, CELL tap contract | Cut (duplicate) | FLOW-05 steps 1 and 6 add the same rules and key (and also LU.2.1, with well-aware layers `lu_ndiff = (nsd not nwell) not dnwell`, `lu_pdiff = psd and nwell`, which REL-06's `tap_distance(nsd, ptap)` lacked) plus the test `latch_up_rules_find_a_far_tap`; CELL-13 owns tap reach and the second strip. T6 needs no new test (DRC baseline 0). |
| C2 | REL-08 injector classification (`Carrier`, `Injector`, `injectors()`, 50 kΩ Dijkstra over resistor edges) | Cut (duplicate) | EXT-23 tag 1 (`MinorityElectron`/`MinorityHole`, `policy.inj_series_ohm` 50 kΩ). Hand-over to EXT-23: REL-08's corpus test is still the right one — `.subckt drv in out VDD VSS` with `M1 out in VSS VSS nfet`; `M2 x in VSS VSS nfet` + `R1 x out 10k`; `M3 y in VSS VSS nfet` + `R2 y out 100k` → M1, M2 electron injectors, M3 not (Hastings §14.2 L43629–43638, §15.5 item 8 L48898–48918). BJTs unclassified → a `missing` note. |
| C3 | REL-09 substrate kind in `ProcessNumbers`, kind-aware `Isolation`, same-block skip, `p_epi_thickness` deletion | Cut (duplicate) | EXT-23 `substrate::isolation` (skip within a compound/set, `EpiOnLowRes` → multiple × epi in the budget arm, bulk/unknown cost-only with `known = false`); FLOW-06 step 7 (`substrate` table); FLOW-04 step 5 (`p_epi_thickness` not required). REL's tests `a_clocked_tail_is_not_isolated_from_its_own_pair` and `epi_decks_get_a_plateau_budget` (Epi 10 µm → 40 000 nm at Su's 4×, Charbon §8.2.1 L3289–3296) are handed to EXT-23. |
| C4 | REL-11 EM waveform classes | Deferred | No producer of per-terminal avg/RMS/peak currents in any plan; with DC only the RMS term cannot bind. Full specification kept below. |
| C5 | REL-18 substrate two-port macromodel | Deferred | No deck ρ stack or package inductance; blocked on data. Full specification kept below. |
| C6 | REL-07's former `post_cell` drawing changes (`PTap`/`NTap` enum, `implant_name`/`in_nwell` mapping, `ring_gap` for `Ecgr`, test `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos`) | Cut (duplicate) | CELL-05 (enum `Tap { in_well }`, rename test) and CELL-17 (`Ecgr` band well, clearances difftap.9/.11, nesting, `drawable`). REL-07 keeps only the `role` field and the merge key. |
| C7 | BJT emitter–base reverse-bias check (H05-37) | Deferred | Needs V_EBO per model; not given in any shipped deck or the sky130 PNP model file (§5 Q14). Revisit with a sidecar `vebo_v` per model. |
| C8 | Effective EM width with process margins (EM-13, Lienig eq. 3.24: h_nom/h_min, Δw, w_etch) | Deferred | No deck carries the four inputs; with the book's own defaults (ratio 1, margins 0) the term is the identity. |

### C4 — REL-11 specification (deferred)
- Priority: **P2**. Effort: **M**. Depends on: FLOW/PERF (per-terminal transient currents per operating phase), REL-03.
- **Why:** EM-06, EM-07, EM-12, EM-22, H05-16, NOTES-48.
  - Lienig: RMS applies to analog DC nets (it includes Joule self-heating), average to signal nets, peak to single events. Size to the maximum of the three (§3.3.1, L3745–3786, PDF 82–83; eq. 3.21–3.23, L4587–4617).
  - Lienig: use at least two limits, DC/f < 10 kHz vs AC (§4.7.2, L6450–6478).
  - Hastings: AC above ~10 kHz seldom fails; pulsed life is t_DC/D or t_DC/D² (eq. 5.13, L12504–12519, PDF 220).
  - The sky130 techlef states RMS limits: met1/met2 6.1, met3/met4 14.9, met5 22.34 mA/µm at Tj 90 °C (`VOL/libs.ref/sky130_fd_sc_hd/techlef/sky130_fd_sc_hd__nom.tlef:120,162,204,246,288`). No peak limit is given.
- **Current:** DC only (`em.rs:90`). With DC, RMS = |I_dc| and the RMS limit exceeds the DC one, so the RMS term cannot bind until waveforms exist. This is why the item is P2.
- **Change:**
  - `pnr_core::routes::Terminal.ua` becomes `pub i: Option<TermCurrent>`, where `pub struct TermCurrent { pub avg: f32, pub rms: f32, pub peak: f32 }`, all magnitudes in µA; DC fills all three with |I|.
  - `Limit` gains `ua_per_um_rms` and `ua_per_um_peak` (0 = none). Sidecar gets `em_rms_ua_per_um: {met1: 6100, met2: 6100, met3: 14900, met4: 14900, met5: 22340}` (sky130, source above).
  - `net_flow` runs once per class.
  - Width need = max over classes with a limit (Lienig eq. 3.21–3.23).
  - Nets classified AC by EXT (switching, f > 10 kHz) skip the avg term but keep RMS (H05-16).
- **Tests:**
  - `em.rs` `rms_governs_a_pulsed_net`: 10 mA at 10 % duty → avg 1 mA, RMS 3.16 mA, peak 10 mA (ref-00 topic-electromigration L48–54). Met1 avg 2800 and RMS 6100 µA/µm. Assert need = max(357, 518) = 518 nm (*computed*).
- **Acceptance:** RMS/peak coverage is reported per net; no regression on DC-only runs.
- **Risks / notes:** mission profiles (EM-15/16) are not planned (§5 Q1).

### C5 — REL-18 specification (deferred)
- Priority: **P2**. Effort: **L**. Depends on: deck substrate data (none exists), REL-09.
- **Why:** SUB-02, SUB-03, SUB-04, SUB-33, SUB-53, SUB-54, SUB-57, NOTES-54, AR-34.
  - The substrate is resistive below ~2–5 GHz (Charbon §3.2, L980–984).
  - Three-element two-port per pair (Fig. 3.3) with Z′12 = R10 + R20 − jR10R20/(ωL_gnd) (eq. 8.1, L3302–3306, PDF 127).
  - Guard-ring model, eq. 8.2 (L3636–3650, PDF 143).
  - Constants are fitted to two-contact sweeps (App. D, L4234–4341).
- **Current:** distance only (`isolation.rs:22-37`).
- **Change** (specified; built only once the data exists):
  1. Deck `substrate: {kind, layers: [{t_um, rho_ohm_cm}], backplate: "grounded"|"floating", l_gnd_nh, l_ring_nh}` (SUB-01).
  2. Calibrate per deck by a finite-difference resistive solve over two-contact sweeps (sizes 10–1000 µm, distances 3–1000 µm, SUB-57 recipe) [UNVERIFIED: the sweep ranges are the SUB-57 recipe's; the Charbon text does not state them, and the figure axes were not read]. Fit `R_i0(A)` and `R12(d)`; accept the fit within 15 % for voltage ratios > 0.5. Charbon reports < 15 % agreement only for ratios > 0.5, "still good" with no number for ratios > 0.1, and degraded below 0.1 (App. D.1, L4293–4297), so report ratios 0.1–0.5 without a pass/fail.
  3. Replace the `Isolation` distance floor with a per-pair evaluation of the fitted three-node circuit, and add ring nodes (eq. 8.2) when REL-07 rings exist.
  4. Add the per-pair isolation (dB) to the signoff report (SUB-56).
- **Tests:** an FD solver unit test against eq. 5.26 of Hastings for two contacts in a uniform thick substrate (L15159–15241, PDF 261), `R_SP = ρ/(2r)·[1 − (2/π)·sin⁻¹(r/d)]`. Assert the formula, not the book's quoted numbers: the book says 10 µm-diameter contacts on 10 Ω·cm give 1 kΩ far apart and 840 Ω at 20 µm c-c, but the formula with ρ = 10 Ω·cm, r = 5 µm gives 10 kΩ and 8.39 kΩ (*computed*); the quoted values match ρ = 1 Ω·cm. The ratio 0.839 at d = 4r is consistent either way.
- **Acceptance:** per-pair isolation reported; the fit meets the 15 % criterion for voltage ratios > 0.5.
- **Risks / notes:** ρ stacks and package inductance are "not given" in ref/ for any shipped deck (§5 Q3). Until then REL-09's cost-only pull stands.

---

---

## Verification log

Adversarial fact-check of this plan against the working tree of 2026-09-28, the pinned GPurify checkout (`8df8c09`, matches `Cargo.lock:678`), `~/.volare/sky130A` (open_pdks `fa87f8f4`), the study docs in `docs/plans/`, and the reftext files. Nothing was built or run.

**References checked: 399.** 160 code references (136 `path:line`, 24 bare `:line`), 72 reftext line ranges with their sections, equations, constants and PDF pages, 136 study-doc IDs (every cited AR/AC/AT/AV/AA/AF/NOTES/EM/H01/H05/H12/H14/H15/SUB/BAL2 entry, including the slash forms EM-16, H05-19, H15-30 and H15-34), and 31 proposed new names, checked for collisions (none collide; `NetClass::Substrate` is an enum variant, not a type). PDF pages were opened where the text extraction drops the number: Hastings PDF 213–214 (eqs. 5.6, 5.8), 261 (eq. 5.26), 621 (25–250 µm), 772 (eq. 14.6), 824 (50 kΩ).

**Corrections (before → after):**
1. §0.2: "AV-06 question 2" → "audit-06 §5 question 2". AV-06 is the `Report.cost` finding; deck ownership is §5 Q2.
2. §1.1 rule slots: "sky130 has 6 metals and 5 cuts that need limits" → the limited layers are met1–met5 and mcon–via4 (10, `sky130.deck:491-495`), and today `em_rules` fills slots with metals only (`lib.rs:999`).
3. §1.1 Blech: "only the generic_finfet deck has a Blech value" → no deck or sidecar supplies `blech_limit`, and generic_finfet has no EM rows (`generic_finfet.deck:434-436`). Also `dr/lib.rs:661` → `:662`.
4. §1.2: "`fed_resistance_ohm` exists but is unused (`stack.rs:167-181`)" → used by `CommonNodes` (`common_node.rs:43-47`), at `stack.rs:163-177`. AR-21 is stale on this point.
5. §1.4: `stack.rs:284-321` → `:270-317`. Lines 318–321 are outside `antenna`.
6. §1.7: the `OpPoint` field list now includes `provenance` and `resolved`.
7. §2: "item 8, L48898–48918" → "items 7–8, …, PDF 823–824". Item 7 is ESD and item 8 is latch-up.
8. T6: "GPurify DRC" → DRC `tap_distance` on sky130/gf180 and ERC `missing_tie` on ihp (`kinds.rs:529-534`).
9. REL-01 Why: "terminal currents that are correct per terminal" → per-terminal current bounds, not one equivalent value (Lienig §3.3.2).
10. REL-01 step 2: `pin_shares(m)` on the placed macro → it must run on the unplaced macro. `place_macro` keeps `units` local while it moves the pins (`macro.rs:56-57`).
11. REL-02: GP `antenna.rs:31-33` → `:30-32`. Added a verified fact: GP subtracts `diode_bonus` from every gate net of the row (`antenna.rs:357-360, 438`).
12. REL-03 Why: the paraphrase "must hold at every point" was in quotation marks → replaced with Hastings's own words (L48741).
13. REL-03 step 2: "three `link` calls at `:205-247`" → four (`:214`, `:220`, `:237`, `:247`). `from` `:335-360` → `:334-355`. `PortGraph` (`:323`) must also become `pub(crate)`. `path_resistance_ohm` `:112-146` → `:110-145`.
14. REL-03 step 4.2: "shape long side bounds the segment" → marked UNVERIFIED (collinear abutting rects form one segment).
15. REL-04 Why: "ΔV = Σ I·R (Lampaert eq. 2.33; Lienig eqs. 3.1–3.7)" → Lampaert 2.33 is R = ρ_□·l/w per rectangle (lampaert.txt L2003) and Lienig 3.5–3.7 gives the branch currents; ΔV = Σ I·R itself is Ohm's law. (Lienig eqs. 3.1–3.4 are terminal-current models.)
16. REL-04: "dr prices R with `current_ua` (`dr/lib.rs:531-550`)" → dr selects over-budget nets by the rule's residual (`:531-538`) and weights them by its own `node_ua` (`:539-549`).
17. REL-04 test: the old bound of 13 750 µV holds only for a two-rect trunk; a one-rect trunk gives 15 000 µV (whole-shape charging, `stack.rs:90-91`).
18. REL-05: eq. 15.25 "PDF 822" → "PDF 821" (L48776 is on PDF 821 by form-feed count).
19. REL-05 step 3: the "smallest-variant bbox" does not guarantee the bound → take the variant that maximises the self rise (counter-example given).
20. REL-06: `overlay.rs:410-425` → `:413-425` (`max_distance_to_tap`, first-layer polygons).
21. REL-06 step 2: `gf180mcu.deck:303-304` "DF.13/14 3.3 V" → `:303,305` (line 304 is DF.13_5V). Also noted that generic_finfet's 3000 contradicts its deck's 30 µm.
22. REL-07 Why: the paraphrase "which are fewer than the potential victims" was in quotation marks → replaced with the actual words (L43642–43643).
23. REL-07 step 1: "HBGR/EBGR need NBL or deep-N+" → HBGR needs a deep-N+ sinker or trench (pure CMOS has neither, L43849–43851). EBGR needs a PBL plus a P+ sinker (L15614–15618). sky130/gf180 have only `dnwell`.
24. REL-07 table, Ecgr row: the width `max(min_ring_width, n_well_depth)` cited §14.2.3 as its source → marked UNVERIFIED. Hastings ties NMoat ECGR width to P-epi thickness or retrograde P-well depth (L43806–43809); the "≥ well depth" rule is for the PMoat HCGR (L43855). The net source now adds L43826–43827, the CMOS N-well ECGR tie to a supply. PMOS row: the parenthetical source → L43842–43846 and L43871–43874.
25. REL-08 Why: the paraphrase "Below about 50 kΩ" was in quotation marks → replaced with the actual words (§14.2).
26. REL-10 step 1: added `kinds.rs:570-587` and `mod.rs:447`, where the text is converted to `ParamValue::Model`. The risk note "voltage unit unknown" is resolved: GP's engine unit is mV (`kinds.rs:68-69`), so `max: 1.95V` reads as 1950.
27. REL-13: "`poly_c` is the resistor body's conductor" → `poly_c` excludes the body (`sky130.deck:106`), and the body `poly_res` (`:144`) has no pex row; the poly-level height is a proxy.
28. REL-14: "add `self_rise_mc`" → "replace the self term". Adding it would double-count and contradicts the 11.9 K acceptance value.
29. REL-17: the source of I_pk ≈ V/1.5 kΩ is now given (HBM network, L61905; 1.3 A at L13065–13066 and PDF 772).
30. REL-18 step 2 and M5: "15 % for coupling ratios > 0.1" → 15 % holds only for voltage ratios > 0.5 (Charbon App. D.1, L4293–4297). The sweep ranges are marked UNVERIFIED.
31. REL-18 test: the "840 Ω at 20 µm" target → assert eq. 5.26 itself. With ρ = 10 Ω·cm and r = 5 µm the formula gives 10 kΩ and 8.39 kΩ; the book's quoted 1 kΩ and 840 Ω match 1 Ω·cm.
32. §5 Q7: "Hastings JESD78 100 mA, L14975–14981" → the §5.4.2 quote "100 mA for 50 ms". The 0.3 V debias is attributed to H05-50 and the H14-34 recipe.
33. §5 Q10: added that Hastings ties no ECGR width to N-well depth.

**Verified as stated (spot list):** the em.rs rule, the derating and cuts (`em.rs:16-21, 60-70, 74, 90, 109-130`); `pin_ua` (`dr/lib.rs:55, 227-232, 269, 421`); `branch_currents` (`:897-934`); Θ `em underwidth`/`em cuts` (`:1852-1854`, `:882-884`); `pin_currents` all-or-nothing (`lib.rs:1035-1055`); non-FET `Some(vec![])` (`oppoint.rs:45-47`); `intent` flattening (`elaborate.rs:297-319`); `em_limits`/`routing_stack` (`elaborate.rs:219, 248-262`); antenna early return (`stack.rs:289-291`) and net-total gate area (`:310`); deck lines sky130 451–463/459/482–486/491–495/582/594/654/669–670/102–105; gf180 303–306/478; ihp 595–596/599/789–790; VOL tech 4165–4170/4890/4912 and tlef 92/119/120/162/204/246/288; the ring code (`cell.rs:6-35`, `post_cell.rs:79-93, 299, 315-323, 355-367, 387-438`, `constraints.rs:66-85`); isolation (`emit.rs:250-288`); `REQUIRED_RULES` (`pdk.rs:1059, 1068`); every computed number (178.6 nm, 0.216, 0.100, 0.374, 281 µA/µm, 13 578 mK, 596 mK, 11.9 K, 1089/10 891 nm, 6.51/4.33 µm², 18.1 µm, 357/518 nm).

**Unresolved:**
- REL-03 Blech domain per shape (UNVERIFIED; dormant until a deck carries `blech_limit`).
- REL-07/REL-08 Ecgr width = `n_well_depth` has no source in ref/ (UNVERIFIED; §5 Q10).
- REL-18 calibration sweep ranges (UNVERIFIED; from the SUB-57 recipe).
- Hastings's eq. 5.26 example disagrees with its own formula by 10× (book-internal; the test now asserts the formula).
- dac4's red `erc/ar.met2.1` (AV-15) and all runtime claims are taken from audit-06, not re-run (no cargo in this task).
- Values recovered by study docs from PDF images and not re-opened here: Hastings §5.4.1 10 kΩ / 50–100 kΩ (PDF 256–257, H05-47) and §14.2 50 kΩ (PDF 735, H14-01).

---

## Implementer review log

Review of 2026-09-29 by the implementing engineer, against the working tree, plans 01–08, the audits and the ref docs. Only this file was edited. Nothing was built or run.

**Items reviewed: 18** (REL-01 … REL-18). Kept and tightened: REL-01, 02, 03, 04, 05, 07, 10, 12, 13, 14, 15, 16, 17. Cut as duplicates: REL-06 (FLOW-05, CELL-13), REL-08 (EXT-23), REL-09 (EXT-23, FLOW-06, FLOW-04). Deferred for lack of input data: REL-11, REL-18. IDs are kept as stubs so other plans' references stay valid.

**Changes:**
1. §0 scope/consumes/provides rewritten to name the owning items in other plans (FLOW-05/06/07, PERF-08/09, EXT-23, CELL-05/13/17/22/23, RTE-06/14/24) instead of "FLOW" generically. REL-10's voltage source is PERF-09, not FLOW.
2. REL-01: added that it supersedes PERF-09 step 2, whose "no pins for an unresolved member" turns unknown into 0 µA through dr's `.unwrap_or(0.0)` (`dr/lib.rs:229-231`). `phi` is a `(i8, i8)` vector (`units.rs:26-27`): the adjacency test is now a dot product and the test fixture gives vectors and nm coordinates. Added the explicit `DetailedCfg.pin_share` field and the fill site (`lib.rs:309`). Bare-name member-0 pins get the `2/n` bound.
3. REL-02: the met3 sidewall deck edit moved to FLOW-05 step 2 (dependency stated). Added step 8 (the `elaborate.rs:289` constructor and the two test constructors at `stack.rs:416, 504`). The agreement test now says how to read in-loop antenna failures (`sol.metadata.routing`, `BudgetStatus`, hard arm at `metadata.rs:192`).
4. REL-03: `net_flow` now receives routed shapes plus cell metal (the RTE-24 step 4 edit, folded in), with only routed indices checked; the `missing` note is a static string because `missing` is `&'static str`. `port_graph` line cited (`stack.rs:183`).
5. REL-05: sidecar keys aligned with FLOW-06 step 3's `em_derating {t_ref_k, ea_ev, n}` table; REL supplies the gf180 (358.15 K) and ihp (378.15 K) `t_ref_k` values and the Ea/n fallback, and the reason 0.9/1.1 beats Hastings's Cu 0.8/1 (H05-10). θ_JA default stated (none given anywhere).
6. REL-07 rewritten: uses CELL-05's enum instead of a third enum; drawing left to CELL-05/17; keeps role, merge key (`post_cell.rs:86`) and a table-testable `rings::plan` with a first-match policy table, `missing` notes, and exact tests. Resolved the policy conflict with EXT-23 (victim rings only with a quiet return; §5 Q12). Replaced the unsourced `n_well_depth` ECGR width with CELL-17's `ecgr_min_width_nm` (absent → deck minimum + note). PMOS injectors get `Hcgr` only when drawable, else a well tap (Hastings L43871–43874).
7. REL-10: deck model names are the full `sky130_fd_pr__…` strings from the deck's `let` lists (`sky130.deck:479-486`); `Violation.margin` is `i64`; FETs without a limit count as unknown; `Solution.pairs` added as the pair source; added the esd_nfet 5 V assertion and a negative bulk case.
8. REL-12: square metal has no long axis (defined); the test fixes both metals' geometry and the resulting need/residual.
9. REL-13: `t_ox` now comes from `res_tox_nm`, else FLOW-05's new `rbody_po` pex row, else skip; formula with units written out; CELL-23 is the caller; stale AV-16 risk replaced.
10. REL-15: dependency moved from REL-09 to EXT-23; distance defined with `Layout::centre`; numeric test values added.
11. REL-16: reduced to the predicate; `well_bridges`' signature is CELL-22's closure form; flags built from EXT-23 tags at `lib.rs:609`; table test.
12. REL-17: corrected an internal inconsistency — the stated `I_pk = V/1.5 kΩ` gives 1.333 A at 2 kV, so A = 6.68 µm² and W = 18.56 µm on met1, not 6.51/18.1 (those assume the book's rounded 1.3 A). Added `esd_area_um2` signature, `Layer.thickness_nm` fill site and the check.
13. Milestones rebuilt with external prerequisites per milestone, measurable exit criteria and the internal dependency order; M5 dropped.
14. §5: questions updated for the new owners; added Q12 (EXT-23 ring conflict), Q13 (PERF-09 step 2 conflict), Q14 (V_EBO not given).
15. New appendix "Cut or deferred items" (C1–C8), carrying REL-11 and REL-18 in full, REL-08/REL-09 tests handed to EXT-23, and two ref findings judged not buildable now: H05-37 (no V_EBO) and EM-13 (no process-margin inputs).

**Left open:** the EXT-23 and PERF-09 conflicts need the owners of plans 01 and 07 to accept REL's version; T1 on dac4 may still need RTE-06 after REL-02.
