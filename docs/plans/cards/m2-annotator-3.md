# M2 annotator, segment 3 (M3): EXT-17, EXT-18, EXT-23, GAP-03, EXT-24, EXT-26

Branch `m2-annotator`, fast-forwarded to `m2` at `a60fd9d` (segment 2, EXT-20, is in). Specs: plan-01 `### EXT-17/18/23/24/26`, 98-gap-critic `### GAP-03`, and master §6 C4, C19 and §6.2 (`Inject` gains the minority kinds). The dag
(`dag-m2-m6.json`) lists these hard deps: EXT-12/14/16/20 (done), and inside the segment EXT-17 → EXT-18 → EXT-23 → GAP-03 → EXT-24, then EXT-26. None of the six needs PLC-06. PERF-12 is a non-blocking producer: `Sensitivities` is defined here and stays unfilled until M4. Line numbers are at `a60fd9d`. Plan citations into `lib.rs`, `emit.rs`, `extract.rs`, `classify.rs` and `frontend/library/src/lib.rs` are stale; the corrected ones are below.

Every commit must pass `PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk cargo test -p annotator -p analog -p library`. EXT-26 also needs `-p cli`. Use one commit per item, `M2+ <ID>: <title>`.

## Shared code facts (all items)

- `annotate(netlist, cfg)` is at `backend/annotator/src/lib.rs:116-420`. Its order is: `netrole::classify_nets` `:123` → recognition `:126-147` → `sensitive` from leaf kinds `:164-171` → `classify::classify` `:173` → **`extract::routing` `:191-198` (before any intent exists)** → compounds `:212` → sets/class/unit `:215-294` → `emit::placement` `:304` → `emit::isolation(.., &sensitive, &same_block, ..)` `:312` → batch tagging `:316-349` → coverage → **REL-07 rings `:368-404`**. REL-07 is already merged. Its fallback is aggressor = touches a Clock net `:375`, `victim` all false `:388` and `injector` all `None` `:389`. The ponytail at `:392-394` waits for rail voltages.
- `analog::intent` (`kernel/analog/src/intent.rs`) already has `Region`, `DeviceRole`, `DeviceFacts`, `RcClass`, `EvidenceLevel`, `NetFacts {evidence, port, dc_mv, rc, z_ohm, shield_ref}`, `Inject {Switching, Capacitive}` `:229-233`, `Aggressor`, `Victim`, `CommonNodeReq`, `StarReq`, `KelvinReq` and the `Intent` vectors `nets, devices, aggressors, victims, common_nodes, stars, kelvins` `:286-292`. Today all of them stay empty.
- `NetClass` has 6 variants (`kernel/analog/src/metadata.rs:17-26`).
- `OpPoint` (`frontend/library/src/oppoint.rs:15-48`) has `power_uw, id_ua, bjt_ua, headroom_mv, gm_us, vgs_v, vds_v, vbs_v, net_v, provenance`. It has **no `vdsat`, `vth`, `gmb` or `gds`**: vdsat is folded into `headroom_mv`. `bias()` (`frontend/library/src/lib.rs:1759-1794`) runs **after** `let base = annotate(..)` (`:386-390`). `Bias.op` holds the `OpPoint`, and `Bias.summary.probe` is `testbench.is_none()`.
- `Netlist.ports` exists (FLOW-07, `kernel/core/src/netlist.rs:117`). A letter-card R value is param `r_mohm` (read with `crate::param`, `lib.rs:429`). `DeviceKind::Diode` exists.
- The CLI rejects `--constraints` (`frontend/cli/src/main.rs:91`), and `frontend/cli/tests/cli.rs:52 constraints_flag_exits_2` pins that rejection "until EXT-26".

---

## EXT-17: evidence input, op point and sensitivities (class: do)

### Plan corrections
- `Evidence.ports` is dropped because `nl.ports` already carries the ports (FLOW-07). Keeping both would duplicate the data.
- `DeviceOp` carries `headroom_mv` (what `OpPoint` has) instead of `vds_mv`/`vdsat_mv`. It adds `vbs_mv: Option<f64>`, which GAP-03 step 5 needs and the plan's struct omits. `vth_mv`, `gmb_us` and `gds_us` stay `Option`, and are `None` from the library because PERF-09 does not produce them.
- The role rules need EXT-18's `Bias`/`Reference`/`Digital*` classes. Step 1 of EXT-18 (the enum variants plus the exhaustive budget/margin/spacing arms) therefore lands **in this commit**. That step changes no behaviour, because no net gets a new class until EXT-18 step 3. The plan's acceptance (ota5t: XM5 is CurrentSource) needs `vbn` = Bias, so its test lands with EXT-18.
- `Cascode` is checked **before** `CurrentSource`. Both require Saturation and a Bias gate, so in the plan's order every cascode would be classed as a CurrentSource.
- The `annotate_without_evidence_is_unchanged` test is a tautology once `annotate` calls `annotate_with`. It is replaced by `no_evidence_no_regions`.

### Exact edits
1. `kernel/analog/src/metadata.rs`: add `Bias, Reference, DigitalStatic, DigitalSwitching, Noisy` to `NetClass`, each with a one-line doc. Then update the class arms:
   - `classify.rs:20-27 budgets`: `Sensitive | Bias | Reference => frac(1,1)`; `Signal => frac(2,2)`. For `Clock | DigitalSwitching | Noisy | DigitalStatic` use `(Some(2c), None)`: no coupling budget, because aggressors are not victims. This changes Clock's coupling budget, as plan EXT-18 step 1 specifies. Rails and Substrate stay `(None, None)`.
   - `extract.rs:35-50`: margin index `Sensitive|Bias|Reference → 0`, `Clock|DigitalSwitching|Noisy → 1`; spacing index `Sensitive|Bias|Reference → 0`, `Clock|DigitalSwitching|Noisy → 1`, `Signal|DigitalStatic → 2`.
2. New file `backend/annotator/src/evidence.rs` (`pub mod evidence;` in `lib.rs`, plus `pub use evidence::{annotate_with, Evidence};`):
   ```rust
   #[derive(Clone, Debug, Default)]
   pub struct Evidence { pub op: Option<OpFacts>, pub sens: Option<Sensitivities>,
       /// Nets driven by PULSE/PWL/SIN testbench sources.
       pub switching_nets: Vec<NetId>,
       /// Nets held by DC testbench sources, mV.
       pub dc_sources: Vec<(NetId, f64)>,
       /// The op point came from the synthesised probe bench (not a sign-off bias).
       pub probe_bias: bool }
   #[derive(Clone, Debug)] pub struct OpFacts { pub dev: Vec<Option<DeviceOp>>, pub net_mv: Vec<Option<f64>> }
   #[derive(Clone, Copy, Debug)] pub struct DeviceOp { pub id_ua: f64, pub headroom_mv: f64, pub gm_us: f64, pub power_uw: f64,
       pub vgs_mv: Option<f64>, pub vbs_mv: Option<f64>, pub vth_mv: Option<f64>, pub gmb_us: Option<f64>, pub gds_us: Option<f64> }
   // Sensitivities / SpecSens exactly as plan-01 EXT-17 (f64 fields, d_c, d_r, d_vt, d_t, d_cc).
   pub fn region(op: &DeviceOp, max_id_ua: f64) -> Region;
   pub fn device_facts(nl: &Netlist, op: Option<&OpFacts>, classes: &[NetClassification],
                       shared_bias: &[Vec<DeviceId>], load_leaf: &[bool]) -> Vec<DeviceFacts>;
   ```
   - `region` rules, first match wins:
     1. `|id| < 1e-3·max_id` gives `Off`.
     2. `Subthreshold` when `vgs`/`vth` are both known and `|vgs| < |vth|`, or else when `gm_us/|id_ua| ≥ 20` (gm·1V/|Id|, with gm in µS and Id in µA).
     3. `headroom < 0` gives `Triode`.
     4. Otherwise `Saturation`.

     `max_id` is taken over the FETs that have a `DeviceOp`.
   - `device_facts` returns one entry per device. A non-FET gets `Passive` (R/C/L) or `Unknown`. A FET gets `region` (`Unknown` without an op). Its role is the first match below; a FET that matches none gets `Unknown`:
     1. `Switch`: Triode and gate class ∈ {Clock, DigitalSwitching, DigitalStatic}.
     2. `Diode`: D == G.
     3. `Cascode`: Saturation, S on a non-rail net, and gate class Bias.
     4. `CurrentSource`: Saturation, and gate class ∈ {Bias, Reference} or the device is in a `shared_bias` group.
     5. `Amplifier`: Saturation and gate class ∈ {Signal, Sensitive}.
     6. `Load`: the device is in a `BlockKind::Load` leaf.

     Without an op, only `Diode`, `Load` and `Passive` are assigned.
   - `annotate_with(nl, cfg, ev)` holds today's `annotate` body. `annotate(nl, cfg)` becomes `annotate_with(nl, cfg, &Evidence::default())`. The body changes in two places:
     - **Testbench rails and clocks enter the pre-pass.** Clone `cfg` and extend `supply_nets`/`ground_nets` with the names of the `dc_sources` nets at the max/min voltage (only nets that `rail_of` gives no role, with ≥ 2 distinct DC levels present), and `clock_nets` with `switching_nets` names. This reuses `classify_nets` unchanged.
     - After `sets::set_pairs` (`lib.rs:303`), set `intent.devices = device_facts(..)`. When `ev.op.is_some() && ev.probe_bias`, push `Diagnostic { kind: "probe_bias", devices: vec![], message: "device regions from a synthesised probe bench".into() }`.
3. `frontend/library/src/oppoint.rs`:
   - `impl OpPoint { pub fn evidence(&self, netlist: &Netlist, probe: bool) -> annotator::Evidence }` maps V to mV, and maps a device with `id_ua`, `headroom_mv` and `gm_us` all `Some` to a `DeviceOp`.
   - `pub fn testbench_sources(netlist: &Netlist, tb: &str) -> (Vec<NetId>, Vec<(NetId, f64)>)` scans the `V…` cards. `n+`/`n-` are resolved case-insensitively; a node `0` or a ground-role net is the reference. When the card contains `pulse(`/`pwl(`/`sin(` (case-insensitive), the non-reference node is switching. Otherwise the first numeric value after an optional `dc` is the DC level of that node, in V×1000.
4. `frontend/library/src/lib.rs` `run` (`:384-401`):
   - Move `let bias = bias(&netlist, cfg);` above `let base = …`. Then build `let ev = bias.op.as_ref().map_or_else(Default::default, |o| { let mut e = o.evidence(&netlist, bias.summary.as_ref().is_some_and(|s| s.probe)); if let Some(tb) = cfg.op.as_ref().and_then(|c| c.testbench.as_deref()) { (e.switching_nets, e.dc_sources) = oppoint::testbench_sources(&netlist, tb); } e });`.
   - Change `base` to `annotate_with(&netlist, &ann, &ev)`.
   - Add the parameter `ev: &annotator::Evidence` to `topology` (`:603-623`; call sites `:400`, `:401`, test `:2731` passes `&Default::default()`). Its `annotate(netlist, ann)` at `:623` becomes `annotator::annotate_with(netlist, ann, ev)`.
   - Unchanged, and reported: `emit.rs:101`, `elaborate.rs:170` and `benchmarks/src/bench.rs:236` re-annotate without evidence. That is the same gap plan EXT-26 names for bench.

### Tests
- `evidence::tests::regions_from_headroom_and_gm`: four `DeviceOp`s with `max_id=100`:
  - `id=0.05` gives `Off`.
  - `id=10, gm=250` (ratio 25) gives `Subthreshold`.
  - `id=10, gm=50, headroom=-20` gives `Triode`.
  - `id=10, gm=50, headroom=+100` gives `Saturation`.
  - Plus `vgs=300, vth=400, gm=50` gives `Subthreshold`.
- `evidence::tests::switch_needs_digital_gate`: on `sc_switches` with Triode ops, with `cfg.clock_nets=["p1","p2"]` assert MS1/MS2 are `Switch`. With no clock config (gates Signal → Sensitive), assert the role ≠ `Switch`.
- `evidence::tests::cascode_before_current_source`: on `folded` with all-Saturation ops, assert M5/M6 are `Cascode` and M0 is `CurrentSource`. This test is green only after EXT-18 (it needs Bias classes), so land it in the EXT-18 commit.
- `tests/corpus.rs::no_evidence_no_regions`: for every corpus circuit, `p.intent.devices.len() == nl.devices.len()` and every region is `Region::Unknown`.
- Command: `cargo test -p annotator -p analog -p library`.

---

## EXT-18: net classes with evidence levels (class: do)

### Plan corrections and decisions
- **Two passes.** `symmetry::analyze`, `sets::shared_bias_groups` (`sets.rs:64` reads `class != Signal`), `passive::*` and `graph::requirements` (`graph.rs:82` excludes Clock) all read classes **before** sets exist. Meanwhile the new rules need sets (Voltage gates, DAC banks). So `classify::classify` stays the pre-pass, unchanged, and a new `classify::refine` runs after `sets::set_pairs`. Readers before the sets see exactly today's classes, so recognition is unchanged.
- **`extract::routing` moves** from `lib.rs:191` to after `refine`. Placement does not read it; only batch-id order does, and the routing arm is tagged after placement anyway (`:342`).
- **Rule order** (corrects the plan). Sensitive-by-Voltage-gate goes before DigitalStatic, Noisy and Bias. Under the plan's order, strongarm's `vin_o`/`vip_o` (inputs of the `mp11/mn13` and `mn3/mp5` inverter matches) would become DigitalStatic, and `three_stage`'s gates-only `vin_p` would become Bias. Both contradict the plan's own tests and EXT-24's strongarm shields.
- **`_n` suffix.** The suffix gives Noisy only when no net named `<stem>_p` exists. `vin_n`/`out_n` is the ordinary differential name; Hastings's suffix convention (H15-06) is honoured only where the name cannot be a differential half.
- **Never a silent demotion.** A pre-pass Sensitive net that matches no new rule stays Sensitive. Examples are a mirror gate with a DC path, such as `three_stage` `n1` and `folded` `o1`.
- **`z_ohm`.** `gds` is `None` from the library (PERF-09 has none), so the 100 kΩ rule is inert in the flow. It is implemented and tested with a synthetic `Evidence`.
- **Evidence levels.** User = `cfg` names. Name = `rail_of`, the clock name rule and the suffixes. Testbench = `dc_sources`/`switching_nets`. Structure = matches, sets and gates-only. OpPoint = the `z_ohm` rule. Default = Signal.

### Exact edits
1. Enum and arms: done in EXT-17.
2. `classify.rs`: factor the per-net load out of `classify` (`:48-91`) as `pub(crate) fn net_load_af(hg, gate_um2: &[f32], gate_af_per_um2: Option<f32>) -> Vec<Option<f32>>`. It keeps the `smallest` and `on_plate` logic. `classify` calls it, with no change in behaviour.
3. `classify.rs`: add
   ```rust
   pub struct RefineCtx<'a> { pub hg: &'a BipartiteHypergraph, pub names: &'a [String], pub matches: &'a [PatternMatch],
       pub sets: &'a [MatchSpec], pub passive: &'a [PassiveSet], pub cfg_roles: &'a [NetRole], pub user: &'a [bool] /*net named in cfg*/,
       pub ev: &'a Evidence, pub ports: &'a [NetId], pub load_af: &'a [Option<f32>] }
   /// EXT-18 step 3: refine pre-pass classes, recompute budgets, return per-net facts.
   pub fn refine(classes: &mut [NetClassification], cx: &RefineCtx) -> Vec<NetFacts>;
   ```
   Rails (Supply/Ground), Substrate and Clock keep their pre-pass class; their evidence level is User, Name or Testbench according to how they were set. For every other net, the first rule that applies wins:
   1. **DigitalSwitching**: the D net of a `cmos_inverter`/`nand_gate`/`nor_gate` match in `matches` (the overlapping `all`, `lib.rs:128`) when any G net of that match is Clock or DigitalSwitching. Iterate to a fixpoint, with at most `#nets` rounds.
   2. **Sensitive**: G nets of members of a `MatchKind::Voltage` set.
   3. **DigitalStatic**: any G or D net of a `cmos_inverter | nand_gate | nor_gate | transmission_gate` match (for a tgate, S too), excluding rails.
   4. **Noisy**: the slot-0 D net of a `charge_pump_cell` match, or a name ending `_n` with no `<stem>_p` net.
   5. **Reference**:
      - The shared G/B net of a `SetRole::BandgapCore` set or passive set, when it is not a rail. This is **Philis policy**: the plan's "output net of a BandgapCore set" names no terminal, and the shared base is the core's VBE node.
      - The slot-1 D net of a `cascoded_reference` match.
      - The `N` net of a DacBank's `reference` capacitor, when it is not a rail.
   6. **Sensitive**: the net every member of a DacBank set shares (the top plate), a name ending `_s`, or, with `op` and known `gds`, a net that is Signal at this point with `z_ohm ≥ 100_000`.
   7. **Bias**: a net touching only FET gates and the D=G of diode-connected FETs.
   8. Otherwise the pre-pass class (Sensitive stays Sensitive; Signal stays Signal).

   After the class is set, `(c_budget_af, max_coupling_af) = budgets(class, load_af[n] (· cox already applied))`. `NetFacts { evidence, port: ports.contains(n), dc_mv: op.net_mv[n].map(|v| (v as i32, v as i32)), rc: Unknown, z_ohm, shield_ref: None }`. Here `z_ohm = 1/(Σ gds of FETs with D on n + Σ gm of diode FETs on n)`, in Ω from µS, and only when every such gds is known.
4. `lib.rs` (`annotate_with`), after `sets::set_pairs`:
   - `intent.nets = classify::refine(&mut net_classes, &cx)`, with `net_classes` made `let mut`.
   - Then `device_facts` (EXT-17).
   - Then one Bias pass: the G net of a `CurrentSource` device that is Signal/Sensitive becomes Bias, with budgets recomputed. This is the plan's "gate net of every CurrentSource"; Cascode/CurrentSource-by-class gates are already Bias, so one pass is a fixpoint.
   - Then `extract::routing(..)`, moved here from `:191`.
5. Readers whose meaning changes:
   - `frontend/library/src/lib.rs:497` and `:1614` filter `Signal|Sensitive|Clock` (sensitivity rows). Make them `!matches!(c.class, Supply|Ground|Substrate)`, so every net that was a row before stays one.
   - `frontend/library/src/lib.rs:882-883` (fill keep-outs) takes `wires_of(Sensitive)`. Take the union of `Sensitive`, `Bias` and `Reference` (`wires_of` gets a `&[NetClass]` argument).
   - `extract.rs:129-131` shields `class == Sensitive` → `matches!(Sensitive|Bias|Reference)`, and `has_clock` → any routed net of class Clock|DigitalSwitching|Noisy. This keeps today's shields for nets that move Sensitive → Bias. EXT-24 then rewrites the block.
   - `emit.rs:259` (`clocked`) is replaced in EXT-23. `graph.rs:82` and `sets.rs:64` read pre-pass classes and are unchanged.

### Tests (`classify::tests` and `tests/corpus.rs::net_classes`)
- `dac4`: `d0..d3`, `b0..b3` DigitalStatic; `top` Sensitive; `VDD` Supply; `VSS` Ground.
- `three_stage`: `vbias` Bias; `vin_p`, `vin_n` Sensitive; `n3` Signal.
- `folded`: `vbp1`, `vbp2`, `vbn`, `vbn2` Bias.
- `bjt_mirror`: `in`, `outn`, `outp` Signal.
- `ota5t`: `vbn`, `vbias` Bias; `vinp`, `vinm` Sensitive. `intent.nets.len() == nl.nets.len()`, and `vbn`'s evidence is `Structure`.
- `sc_switches` with `cfg.clock_nets = ["p1","p2"]` and synthetic Triode `Evidence`: `p1`, `p2` Clock with evidence `User`; MS1/MS2 `Switch`.
- `noisy_suffix_needs_no_p_twin`: a net `x_n` alone is Noisy; `x_n` together with `x_p` is not.
- `z_ohm_marks_high_impedance_sensitive`: one FET with `gds_us = 5.0` on a Signal net gives `z_ohm == 200_000`, and the net becomes Sensitive. With `gds_us = 20.0` (50 kΩ) it stays Signal.
- `budgets`: `budgets(Clock, c).1 == None`, and `budgets(Bias, c) == budgets(Sensitive, c)`.
- `perf_postlayout.rs::ota_probe_regions` (EXT-17 acceptance): `fixture("ota")` + `op_cfg`, then `o.evidence(&nl, true)` and `annotate_with`. Assert XM1..XM5 are `Saturation` and XM5 is `CurrentSource`. If the probe bench puts a device in triode, the test stays red, and the evidence (headroom per device) goes in the report. Do not change the bench to make it pass.
- `evidence::tests::cascode_before_current_source` (from EXT-17).
- Existing tests that encode the old rule: `tests.rs:541` (`vcas` gates-only is `Sensitive`) becomes `Bias`, a spec-mandated rename of the class; the assertion strength is unchanged. Grep `NetClass::Sensitive` in `src/tests.rs` and `tests/` and apply the same treatment.
- Command: `cargo test -p annotator -p analog -p library`, plus `cargo test -p library --release --test perf_postlayout ota_probe_regions`.

---

## EXT-23: substrate aggressor and victim tags (class: do)

### Plan corrections and decisions
- The strongarm victim list in the plan is stale. At `a60fd9d` the Moderate sets are `{mn1,mn2}`, `{mn3,mn4}`, `{mp7,mp8,mp9,mp10}` (Current, all clocked) and `{mp5,mp6,mp11,mp12}` (`corpus.rs:168`).
- **A device tagged as an aggressor is not a victim.** This matches today's `clocked(a) && !sensitive[a]` (`emit.rs:270`), and REL-07 row 5 rings it as an aggressor (`rings.rs:99-104`).
- Strongarm victims are therefore `mn1, mn2, mn3, mn4, mp5, mp6, mp11, mp12`.
- **One tag per device**, in the order Switching > Capacitive.
- `weight`: when any set holding the device has `MatchSpec.weight`, use the largest such weight. Otherwise use the largest rank over its sets with class ≥ Moderate (Moderate 3, Exceptional 10). A Bias/Reference-gated device in no such set gets 1. Rank 1 (Minimal) is never used: a Minimal set member is not a victim. These ranks are **Philis policy**.
- REL-07 is merged, so this item also feeds `RingInputs`: `aggressor` from the tags, `victim` = `victim_mask`, and `highest_supply` = the Supply net with the largest `intent.nets[n].dc_mv` when known. That resolves the ponytail at `lib.rs:392-394`.

### Exact edits
1. New file `backend/annotator/src/substrate.rs` (`pub mod substrate;`):
   ```rust
   /// EXT-23 steps 1–3 (GAP-03 adds the minority kinds): aggressors, then victims (non-aggressors).
   pub fn tag(nl: &Netlist, classes: &[NetClassification], sets: &[MatchSpec]) -> (Vec<Aggressor>, Vec<Victim>);
   /// `victim[d]`: what REL-07's `RingInputs.victim` and REL-16's `CellFlags.sensitive` read.
   pub fn victim_mask(n_devices: usize, v: &[Victim]) -> Vec<bool>;
   ```
   - An aggressor-class net is Clock, DigitalSwitching or Noisy.
   - `Switching` (reason `"terminal on a switching net"`): any terminal is on an aggressor-class net.
   - `Capacitive` (reason `"capacitor-coupled to a switching net"`): a non-Switching device with a terminal on net `x`, where some `Capacitor` has one plate on `x` and the other on an aggressor-class net.
   - Victims (reason `"matched set"` or `"bias/reference gate"`): members of sets with `class >= Moderate`, and FETs whose G net is Bias or Reference, minus the aggressors. Output is in device-id order.
2. `lib.rs`:
   - Set `(intent.aggressors, intent.victims) = substrate::tag(..)` after `refine`.
   - Build `let victim = substrate::victim_mask(n, &intent.victims)` and `aggressor: Vec<bool>` from `intent.aggressors`.
   - Delete the `sensitive` vector at `:164-171`, but keep it as the `classify` pre-pass input. **Only** its isolation use goes.
3. `emit::isolation` (`emit.rs:248-283`):
   - Replace `classes`/`clocked` with `aggressor: &[bool]` and `victim: &[bool]`. The rule set becomes `(a, v)` with `aggressor[a] && victim[v] && !related(a, v)`.
   - In `lib.rs`, `related` = `same_block(a, v)` (existing) **or** a and v are members of one `intent.sets` entry, **or** both are in one compound (`pairs` ∪ `selfs`). That last clause is plan step 4, which covers strongarm `mn0` in the input pair's compound.
4. `lib.rs` REL-07 block (`:368-404`):
   - `aggressor` = the tag mask (Switching|Capacitive), `victim: &victim`.
   - `touched_by_aggressor` uses the tag mask.
   - `highest_supply` = the Supply net with max `intent.nets[n].dc_mv.0`, falling back to the lowest id. Update the ponytail comment.

### Tests (`substrate::tests`, plus existing tests)
- `strongarm_tags` with `strongarm_cfg()` (clk Clock):
  - Switching = `{mn0, mp7, mp8, mp9, mp10}`; there are no other aggressors.
  - Victims = `{mn1, mn2, mn3, mn4, mp5, mp6, mp11, mp12}`; `mn13`/`mn14` (Minimal) are not victims.
- `ota5t_tags`: 0 aggressors; victims = `{XM1, XM2, XM3, XM4, XM5}`, with XM5's reason `"bias/reference gate"`; `p.constraints.guard_rings.is_empty()` (C19, T8).
- `dac4_logic_is_no_victim`: no `XMN*`/`XMP*` device is a victim.
- `capacitor_couples_an_aggressor`: `C1 clk x`, `M1 x g VSS VSS`, `M2 y x VSS VSS`, with `clk` Clock. C1 is Switching; M1 and M2 are Capacitive. M2 qualifies because a G on `x` is a terminal.
- `emit.rs a_clocked_tail_is_not_isolated_from_its_own_pair`: stays as is.
- `tests.rs:751 clocked_devices_are_kept_away_from_matched_ones`: the count goes 4 → **5**, because XS is now isolated from XM5 too. XM5 is a Bias-gated victim per EXT-23's own test. The rule set grows, so nothing is loosened; update the message to "the four matched devices and the bias-gated tail".
- Corpus `no_emitted_conflicts` and `no_emitted_conflicts_strongarm` must stay green.
- Command: `cargo test -p annotator -p analog -p library`.

---

## GAP-03: minority-carrier injector classification (class: do)

### Code facts
- REL-07's `rings::Carrier {Electrons, Holes}` and `RingInputs.injector` exist (`rings.rs:11-26`). `lib.rs:389` passes all-`None`.
- `Policy` lives in `backend/annotator/src/policy.rs`.
- FLOW-07 is done: there are ports and `r_mohm`.

### Exact edits
1. `kernel/analog/src/intent.rs:229-233`: add `Inject::{MinorityElectron, MinorityHole}`, with doc `n+ diffusion in p-sub on a pin (forward-biased below ground)` and `p+ in n-well (above supply)`.
2. `policy.rs`: add `pub inj_series_ohm: f64` with default `50_000.0`. Its doc cites Hastings §14.2 L43629–43638 and H05-47 (10 kΩ usual, 50–100 kΩ conservative).
3. `substrate.rs`:
   ```rust
   /// GAP-03: devices with a diffusion within `inj_series_ohm` of a pin, plus forward-biased bulks.
   pub fn injectors(nl: &Netlist, classes: &[NetClassification], op: Option<&OpFacts>, inj_series_ohm: f64,
                    missing: &mut Vec<(&'static str, &'static str)>) -> Vec<Aggressor>;
   ```
   - Pin nets = `nl.ports` with a class not in {Supply, Ground, Substrate}. When `nl.ports` is empty, push `("GuardRing", "no port list: injectors unknown")` and return no port-based tags; step 5 still runs.
   - Multi-source Dijkstra (`BinaryHeap<Reverse<(u64 mΩ, net)>>`) uses Resistor devices as edges, with weight `param(d, "r_mohm", 0)`. A missing value gives weight 0, and the far net carries reason `"resistance unknown"`. A net is reached when its distance is below `inj_series_ohm·1000` mΩ.
   - On a reached net:
     - NMOS with D or S → `MinorityElectron`; PMOS with D or S → `MinorityHole`.
     - Diode: N on the net → Electron, P → Hole.
     - Reason `"pin diffusion"`, or `"resistance unknown"` when the path used an unknown R.
     - Npn/Pnp: no tag; push `("GuardRing", "bipolar injectors unclassified")` once.
   - With `op`: an NMOS with `vbs_mv > 0` or a PMOS with `vbs_mv < 0` gets the same tag with reason `"forward-biased bulk"`.
   - One tag per device; the first reason wins.
4. `lib.rs`:
   - `intent.aggressors.extend(substrate::injectors(..))`. A device may carry a Switching tag and a minority tag, since they are different kinds.
   - `RingInputs.injector[d]` = `Some(Electrons)`/`Some(Holes)` from the minority tags.
   - `aggressor[d]` stays Switching|Capacitive only.

### Tests (`substrate::tests`)
The fixture is `drv` with `Netlist.ports = [in, out, VDD, VSS]`:
- `M1 out in VSS VSS nfet`
- `M2 x in VSS VSS nfet` with `R1 x out` and `r_mohm = 10_000_000`
- `M3 y in VSS VSS nfet` with `R2 y out` and `r_mohm = 100_000_000`

The tests:
- `pin_diffusions_are_injectors`: M1 and M2 are `MinorityElectron`; M3 is untagged.
- `a_pmos_on_a_pin_injects_holes`: adding `M4 out in VDD VDD pfet` gives `MinorityHole`.
- `unknown_resistance_is_conservative`: with R2's `r_mohm` removed, M3 is tagged with reason `"resistance unknown"`.
- `no_ports_no_minority_tags`: `ports` empty gives 0 minority tags, and `missing` contains `("GuardRing", "no port list: injectors unknown")`.
- `forward_biased_bulk`: no ports, an op with `vbs_mv = +200` on an NMOS gives `MinorityElectron` and `"forward-biased bulk"`.
- `drv_injectors_get_rings` (REL T7 acceptance): `annotate(drv)` gives one `RingRole::Injector` ring per minority-tagged device (100 %).
- `ota5t` has 0 minority tags, because its ports are gates or rails (REL T8). Fold this into `ota5t_tags`. Note that `ota5t` in the corpus has no ports; use `fixture("ota")` ports in the library test, or set `ports` by hand.
- Command: `cargo test -p annotator -p analog -p library`.

---

## EXT-24: routing constraints from structure (class: do)

### Code facts and corrections
- `extract::routing(hg, classes, gate_um2, process, pairs, policy)` (`extract.rs:27-161`):
  - `Differential` and `CrosstalkExclusion` come from DiffPair drains (`:93-107`).
  - Shields use `ground` = the first routed Ground net (`:126-131`), which is AA-31.
  - `CommonNodes` are built in the library, not in `extract`: `Flow::common_nodes` at `frontend/library/src/lib.rs:1355-1388` (the plan's `:763-801` is stale) scans 2-device DiffPair/CurrentMirror/Load leaves sharing `S`.
- **Kelvin count** (corrects the plan): `KelvinReq` has one `term`, so a sense resistor between two DP gates yields **2** requests: `(RS, P, [(M1,G)])` and `(RS, N, [(M2,G)])`, each terminal paired with the gate on its net. The plan says "1 KelvinReq".
- Sidecar `Kelvin` entries come with EXT-26.
- This item changes the routing rules of every circuit. For example, ota5t now has 2 `Differential` (`(vinm,vinp)`, `(vout1,vout2)`) instead of 1, so `corpus.rs:316 differential_comes_from_recognized_pairs` changes. Its ota5t expectations become the compound net pairs, a spec-mandated change. Before merging, run `bench local` on the OTA rows and report any HPWL/overflow change; the OTA-class regression is already open per master Status.

### Exact edits
1. `extract::routing(hg, classes, gate_um2, process, intent: &mut Intent, net_names: &[String], policy)`. The `pairs` parameter is gone. Pass `&mut intent` because the function writes `nets[].shield_ref` and `rc`.
   - **Differential**: for each `intent.compounds[*].net_pairs` pair where neither net is a rail, deduplicated with `fresh(.., 0)`.
   - **CrosstalkExclusion (a)**: for each `MatchKind::Voltage` set, each member's G net × each member's D net.
   - **CrosstalkExclusion (b)**: every (victim ∈ {Sensitive, Reference, Bias}, aggressor ∈ {Clock, DigitalSwitching, Noisy}) routed net pair, at `route_space·max(mult)`, `margin_pct: 25`.
2. `pub fn quiet_ground(classes: &[NetClassification], net_names: &[String], aggressors: &[Aggressor], hg) -> Option<(NetId, bool /*shared*/)>`:
   - First choice: a Ground net whose name contains `avss`, `vssa` or `agnd`, lowest id.
   - Else: the lowest-id Ground net on which no `Switching` aggressor has a terminal.
   - Else: the lowest-id Ground net with `shared = true`. The caller then pushes `Diagnostic { kind: "shared_shield_return", .. }` (SUB-30).
   - No Ground net gives `None`.

   Shields go to routed victims (Sensitive|Bias|Reference) only when an aggressor-class routed net exists. Set `intent.nets[v].shield_ref = Some(r)`; `CouplingBudget.exclude` reads the same `r`.
3. `intent.common_nodes`: for each set `si` whose FET members all share the S net (BJT: E), rails included:
   - With `reference = Some(k)`, one `CommonNodeReq { net, set: si, a: vec![ref], b: vec![m], term }` per non-reference member.
   - Else, `a` = half-A members and `b` = half-B members (`Member.half`). When halves are `None`, use the first ⌈n/2⌉ by id as A.
4. `intent.stars`: for each common node net, look at the non-member devices with an S/D/E terminal on it. Excluding the single feed (a non-member D/C terminal or a port), if any remain, push `StarReq { net, root: feed, branches: members' (device, S|E) }`. With an op, it also qualifies when `Σ|I_nonmember| > 0.01·Σ|I_member|` (**Philis threshold**). Deduplicate by net, keeping the first set.
5. `intent.kelvins`: for a Resistor whose P and N nets are both G nets of one Voltage set, one `KelvinReq` per terminal, sensed by the member gates on that net.
6. DAC plates: for a DacBank set, `intent.nets[shared plate].rc = RcClass::C`, and each member's other plate, when not a rail, gets `RcClass::R`.
7. `lib.rs`: call `extract::routing(.., &mut intent, &hg.net_names, ..)` after `substrate::tag`.
8. `frontend/library/src/lib.rs:1373-1386` `common_nodes`:
   - Iterate `self.problem.intent.common_nodes`.
   - `pins` over `req.a`/`req.b` with `req.term` name.
   - `feeds` = pins on the net whose device is not in `a ∪ b`.
   - `max_delta_ohm = common_node_ohm(&left, req.a[0], req.b[0], i_ua of req.a[0])`.
   - Remove the leaf scan and the `use annotator::BlockKind` there if it becomes unused (`environment` at `:1394` keeps its own).

### Tests (`extract::tests` and corpus)
- `folded`: Differential net pairs = exactly `{(vinp,vinn), (x1,x2), (o1,out), (y1,y2)}`, unordered.
- `strongarm`: shield victims = exactly `{vin, vip, vin_o, vip_o}`, reference `vss`. A `shared_shield_return` diagnostic is present, because `mn0` (an aggressor) has S on `vss`. With `clock_nets` empty there are 0 shields.
- `ota5t`: exactly 2 `CommonNodeReq`: `vtail` with `a={XM1}`, `b={XM2}` and `VDD` with `{XM3}`/`{XM4}`. 0 `StarReq`.
- `mirror6`: 5 `CommonNodeReq` on `VSS`, each `a=[MR]`, and 0 `StarReq`. With `MX z g VSS VSS nfet w=2u l=1u` added: 1 `StarReq` on `VSS` with 6 branches.
- `kelvin_from_sense_resistor`: ota5t with `RS vinp vinm r=1k` added gives 2 `KelvinReq` on RS (terms P and N).
- `dac4`: `top` `RcClass::C`; `b0..b3` `RcClass::R`.
- `quiet_ground_prefers_analog_root`: Ground nets `{VSS, AVSS}` give `AVSS`, unshared.
- Update `corpus.rs:316` ota5t: 2 Differential in budget, and the CrosstalkExclusion set recomputed from rule (a): G×D of the Voltage set `{XM1,XM2}` gives 4 pairs. That is the same 4 as today, so the assertion stays.
- Command: `cargo test -p annotator -p analog -p library`; then `bench local` on OTA rows (release), with numbers in the report.

---

## EXT-26: user constraint sidecar (class: do)

### Code facts and corrections
- `AnnotationConfig` is at `netrole.rs:122-144`. `ClassCtx.user` is hard-wired `None` (`lib.rs:271`). `bench.rs:236` (not `:269`) re-annotates with `default()`.
- `serde_json` is not an annotator dependency; `backend/verify` has it, so it is in the lockfile.
- The ALIGN gold is vendored at `benchmarks/competition/ALIGN/examples/high_speed_comparator/high_speed_comparator.const.json`. Copy it into the test as a `const` string so the test never depends on the vendored tree.
- **YAGNI**: only entries with a reader now get config fields. `Load` (EXT-25, M4) and `Order` (EXT-28, M6) parse into a `Diagnostic { kind: "sidecar_unconsumed" }` naming the item that will read them. ALIGN `Align`, `HorizontalDistance`, `VerticalDistance`, `CompactPlacement`, `ConfigureCompiler` and any other unknown entry give `kind: "sidecar_unsupported"`.
- `bench.rs:236` has no constraint source, so it stays on `default()`. Note this in the report.

### Exact edits
1. `backend/annotator/Cargo.toml`: add `serde_json = "1"`.
2. `AnnotationConfig` gains:
   - `pub seeds: Vec<symmetry::Seed>` (ids assigned at parse: `ConstraintId(u32::MAX - entry_index)`, so they never collide with leaf-index seeds)
   - `pub groups: Vec<Vec<DeviceId>>`
   - `pub classes: Vec<(Vec<DeviceId>, MatchClass, Option<MatchKind>)>`
   - `pub net_classes: Vec<(NetId, NetClass)>`
   - `pub offset_budgets: Vec<(Vec<DeviceId>, f32)>`
   - `pub kelvins: Vec<KelvinReq>`

   All are `Default`-empty.
3. New file `backend/annotator/src/sidecar.rs`:
   ```rust
   pub fn parse(json: &str, nl: &Netlist) -> Result<(AnnotationConfig, Vec<Diagnostic>), String>;
   impl AnnotationConfig { pub fn from_json(json: &str, nl: &Netlist) -> Result<(Self, Vec<Diagnostic>), String> { sidecar::parse(json, nl) } }
   ```
   - `Err` only for non-JSON input or a top level that is not an array. Names resolve case-insensitively.
   - An unknown instance or net gives `Diagnostic { kind: "sidecar_unknown_name" }`, and that entry is skipped.
   - `PowerPorts`/`GroundPorts`/`ClockPorts` fill `supply_nets`/`ground_nets`/`clock_nets`.
   - `DoNotIdentify` fills `do_not_identify`.
   - `GroupBlocks` fills `groups` and registers the alias `instance_name → members`.
   - `SymmetricBlocks` (direction V/H; the direction is recorded, and a single-axis compound uses it) handles each entry:
     - 2 device names → `Seed::Devices`.
     - 2 aliases → members paired in order.
     - 1 device → `Seed::SelfDevice`.
     - A 1-name alias of 2 members → `Seed::Devices`.
   - `SymmetricNets` gives `Seed::Nets`.
   - `Match` fills `classes` (`kind` is optional: when given it overrides `class::kind_of`).
   - `NetClass` fills `net_classes`. `"digital"` maps to DigitalSwitching (conservative: it is treated as an aggressor) and `"digital_static"` to DigitalStatic.
   - `OffsetBudget` fills `offset_budgets`. `Kelvin` fills `kelvins` (`"M1/G"` → `(M1, Term::G)`).
4. Readers in `lib.rs`:
   - User seeds are prepended to `seeds` at `:204-211`.
   - `groups` feed `graph::requirements` as ProxBlock couples, by adding a `user_groups` slice argument next to `passive`. `emit::placement` also emits one `Proximity` batch per group (first member to each other, `policy.proximity_nm`), reusing the `prox` closure (`emit.rs:136`).
   - `ClassCtx.user` = the class of a `classes` entry whose devices ⊇ the set's members. `spec_6sigma` = `6·sigma_mv` from an `offset_budgets` entry covering the set, else today's value.
   - In `refine`, `net_classes` overrides go first, with evidence `User`.
   - `kelvins` are appended to `intent.kelvins` in EXT-24's step.
   - `problem.intent.diagnostics` gets the parse diagnostics: `Config` carries them, via a field `pub sidecar_diags: Vec<Diagnostic>` set by `from_json`'s caller.
5. Library and CLI (coordinated FLOW-12 edit):
   - `library::Config` gains `pub constraints: Option<String>` (the JSON text). `run` applies `AnnotationConfig::from_json(text, &netlist)` after parsing and merges it over `cfg.annotation`: lists extend, scalars from base. A parse error is `FlowError::Interface(msg)`.
   - `frontend/cli/src/main.rs:91` reads the file into `cfg.constraints`; an unreadable file exits 2.
   - Replace `cli.rs:52 constraints_flag_exits_2` with `constraints_flag_reads_the_file`: a missing file → exit 2 with "constraints", and the gold JSON on the `strongarm` fixture → exit 0. Its doc comment says the old test pinned "until EXT-26".

### Tests (`sidecar::tests`, plus `tests/align_gold.rs`)
- `align_power_ground_clock`: the gold's first 3 entries on STRONGARM give `vcc` Supply, `vss` Ground and `clk` Clock in `annotate(..).net_classes`.
- `symmetric_blocks_become_seeds` (in `align_gold.rs`):
  - Feeding the full gold through `from_json` (with no `strongarm_cfg` names) gives `canon(p).pairs == GOLD_PAIRS` exactly, `mn0` in `selfs`, and `axes == 1`.
  - The diagnostics have exactly 4 `sidecar_unsupported` (Horizontal/VerticalDistance and 2×Align) and 1 `sidecar_unconsumed` (`Order`).
- `unknown_constraint_is_diagnosed`: `[{"constraint":"Align","instances":["mn1"]}]` → `Ok`, with one `sidecar_unsupported` diagnostic.
- `match_class_overrides_role`: `[{"constraint":"Match","instances":["XM1","XM2"],"class":"exceptional"}]` on ota5t → the Voltage set has `class == Exceptional`, `class_source == ClassSource::User`, and its compound is `SymKind::Perfect`.
- `unknown_name_is_diagnosed`: `PowerPorts ["nope"]` → `sidecar_unknown_name`, and `supply_nets` is empty.
- `not_json_is_an_error`: `"{"` → `Err`.
- Command: `cargo test -p annotator -p analog -p library -p cli`.

---

## Out-of-scope notes (report, do not fix)
- `emit.rs:101` and `elaborate.rs:170` (library) re-annotate without evidence or sidecar, so their `Problem` can differ from `run`'s in testbench-derived rails/clocks and user seeds.
- `PERF-09` provides no `gds`/`vth`/`gmb`, so EXT-18's `z_ohm` and the vth-based subthreshold rule are inert in the flow until PERF produces them.
