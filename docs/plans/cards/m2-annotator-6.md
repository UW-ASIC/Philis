# M2+ annotator segment 6 (M6): implementation cards

Branch `m2-annotator`, worktree `philis-m2/annotator`, after `git merge m2` (`ad40c5b`, clean).
Order: EXT-27, EXT-28, EXT-29. EXT-27 uses EXT-28's `Intent.order` for arrays, so EXT-28 step 1 (the `Order`
type) is done first inside EXT-27 and EXT-28 then fills it. Line numbers are of this tree; find code by symbol.
Every command needs `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

| Item | Class |
|---|---|
| EXT-27 | do (FLOW-07 hierarchy is in the tree: `Netlist.insts`, `device_inst`) |
| EXT-28 | do (defines the `Intent.order` contract PLC-25 reads; see "contract change") |
| EXT-29 | do (the metadata-report line is a two-line edit in `library`) |

DAG: hard deps done (EXT-13 `graph.rs`, EXT-14 `symmetry.rs`, EXT-16 `class.rs`, EXT-17 `evidence.rs`).
`dag-m2-m6.json` notes "FLOW-07 (instances) done" for EXT-27. No M1-pending item (PLC-06) is needed.

Batch rules: no new warnings in `annotator`, `analog`, `library`. A test not named here that goes red means stop and
report; do not edit its assertion. Tests whose expected value changes by design are listed per item.

---

## Shared: `Intent.order` (first commit of EXT-27)

`kernel/analog/src/intent.rs` (Intent at :285–298). Add after `GroupNode`:

```rust
/// A placement order (EXT-28, EXT-27 arrays, sidecar `Order`). `steps[0]` sits at the low
/// coordinate of `dir` (bottom for `V`, left for `H`); each step is a set of devices
/// (PLC-25 takes the bbox of a step's devices).
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub steps: Vec<Vec<DeviceId>>,
    pub dir: AxisDir,
    /// The sense along `dir` is placement's choice (extracted orders); `false` for a user order.
    pub reversible: bool,
    /// Path current / the largest extracted path current, 0..=1; 1.0 without an op point.
    pub weight: f32,
}
```

and `pub order: Vec<Order>,` in `Intent` (after `tree`). **Contract change\***: the plan's
`Intent.order: Vec<(Vec<u32 /*group ids*/>, AxisDir)>` (plan-01 EXT-28, plan-04 PLC-25 L624) cannot name a single
device (strongarm's `mn0` is not a tree node, render: `...,mn0}`) and gives PLC no sense freedom; device steps plus
`reversible`/`weight` cover PLC-25's `FlowOrder { chain, dir, weight }` directly. Tell the PLC-25 owner in the report.

---

## EXT-27 Hierarchy: identical instances and arrays

### Current code (plan corrections marked \*)

- \* Plan says "flat parser, ports and subcircuits dropped (`parse.rs:35-39`)" and consumes a FLOW
  `Netlist.instances: Vec<(String, String, Vec<DeviceId>)>`. Stale: FLOW-07 landed `Netlist.ports`,
  `insts: Vec<SubcktInst { path, subckt, parent, ports }>` and `device_inst: Vec<Option<u32>>` (innermost instance,
  parallel to `devices` or empty) at `kernel/core/src/netlist.rs:113–138`; the parser fills them
  (`frontend/library/src/parse.rs:327, 404`). Device names are `X1/X3/<name>` (netlist.rs:108).
- `graph::requirements` (graph.rs:34–100): ProxNet is a star per non-rail net (:88–98), crossing instances freely.
- Seeds: user seeds then leaf seeds (lib.rs:216–225), `symmetry::analyze` (lib.rs:226); a seed whose devices are taken
  or inconsistent is dropped with a GAP-09 conflict diagnostic (symmetry.rs:201–206).
- \* Plan signatures take `canon`; canon labels include outside connections, so two identical instances on different
  nets differ. Use `size::Drawn` signatures (`symmetry::sig` shape: kind, model, finger W, L, fingers).
- \* Plan "arrays become a `GroupNode` ... with an `ArrayStyle::Any` request": `GroupNode` (intent.rs) has no style and
  nothing reads one. Skipped; the array is a ProxBlock group plus an `Order`.

### Edits

New `backend/annotator/src/hier.rs` (`pub mod hier;` in lib.rs):

```rust
/// Device couples of instances `a`, `b` by name below the instance path (`X1/M3` ↔ `X2/M3`);
/// `None` when the device lists or any couple's (kind, model, finger W, L, fingers) differ.
pub fn corresponding(nl: &Netlist, drawn: &[Drawn], a: u32, b: u32) -> Option<Vec<(DeviceId, DeviceId)>>;
/// Instance pairs of one subckt under one parent, exactly two such instances, device-wise identical
/// (`corresponding` is `Some`); a subckt with ≥ 3 instances is an array candidate, never paired (Philis policy).
pub fn same_template(nl: &Netlist, drawn: &[Drawn]) -> Vec<(u32, u32)>;
/// ≥ 3 identical instances of one subckt under one parent with one port net of class `Bias`
/// in common; per Bias net the instances that carry it, largest set first, instance order.
pub fn arrays(nl: &Netlist, drawn: &[Drawn], classes: &[NetClassification]) -> Vec<Vec<u32>>;
/// Port nets of `a` and `b` pair up: per formal port k, equal nets (shared) or a couple, and no net in two couples.
fn ports_pair(nl: &Netlist, a: u32, b: u32) -> bool;
```

A device of instance i is one whose `device_inst` chain (via `insts[..].parent`) reaches i; relative name =
`name.strip_prefix(&format!("{}/", path))`. Empty `insts` → all three return empty (every existing test unchanged).

`lib.rs`:
1. After `drawn` (lib.rs:142) and before seeds: `let pairs = hier::same_template(netlist, &drawn);`.
2. Seeds (lib.rs:225): append, after the leaf seeds, `Seed::Devices(x, y, ConstraintId(u32::MAX / 2 + k))` for each
   couple of a pair with `ports_pair` true whose devices are in no leaf seed (leaf symmetry inside an instance wins).
3. `graph::requirements` gains `hier_pairs: &[(DeviceId, DeviceId)]` (every couple of every `same_template` pair:
   MatchBlock unless already MatchSym, like the match loop :68–74), `arrays: &[Vec<DeviceId>]` (all devices of one
   array: ProxBlock star from `g[0]`, like the sidecar loop :84–87) and `inst: &[Option<u32>]` (`netlist.device_inst`):
   in the ProxNet loop push `(ds[0], d)` only when `inst(ds[0]) == inst(d)` (`get(..).copied().flatten()`, so an empty
   slice keeps today's edges). Doc comment: ProxNet does not cross an instance boundary (**Philis policy**).
4. Arrays need classes, which exist before `requirements` (lib.rs:182): `let arrays = hier::arrays(netlist, &drawn, &net_classes);`
   and per array keep an `Order { steps: one step per instance (its devices, sorted by (canon, id)), dir: AxisDir::H,
   reversible: true, weight: 1.0 }` in `array_orders`, appended to `intent.order` after EXT-28's orders.

### Tests

`hier::tests` (build `Netlist` with `crate::tests::{fet, nets}` and explicit `insts`/`device_inst`, names `X1/MN`):
- `two_identical_otas_pair_up`: two 5-device OTA instances `X1`,`X2` of `ota` (DP MN1/MN2, loads MP3/MP4, tail MN5;
  ports inp inn out vdd vss vb, X2 on distinct inp/inn/out). `assert_eq!(same_template(&nl, &drawn), [(0, 1)])`;
  annotate: `reqs` (call `graph::requirements` as lib does, or check the tree) has a `MatchBlock` X1/MN5–X2/MN5;
  no `ProxNet` req joins an `X1/` with an `X2/` device; each OTA keeps its own DP pair in `intent.compounds`
  (X1/MN1↔X1/MN2 present, X1/MN1↔X2/MN1 absent).
- `identical_inverters_mirror`: `inv` X1 (a→b), X2 (c→d), shared vdd/vss; compounds contain (X1/MN, X2/MN) and
  (X1/MP, X2/MP).
- `bias_shared_array`: 4 instances of `cell` (MC: D=out_k G=vb S=vss; MK cascode: D=o_k G=vc S=out_k) on shared
  `vb`, `vc`, plus a diode MREF (D=G=vb) at top. `assert_eq!(arrays(..), [vec![0, 1, 2, 3]])`; `intent.order` has one
  `H`, reversible entry of 4 steps of 2 devices; a third test variant with 2 instances gives `arrays == []`.
- `flat_netlist_unchanged`: `same_template`/`arrays` on `crate::tests::three_stage()` are empty.

Acceptance (T2 on a parsed two-instance fully differential netlist): `frontend/library/tests/hier_annotate.rs`
`two_instance_fd_ota`: `library::parse::spice` on a `.subckt fd_ota` body (DP, cross loads, tail, CMFB-free) instanced
`X1`, `X2`; `annotate`; assert the leaves (`DiffPair` per instance), `same_template == [(0,1)]` and the X1/X2
MatchBlock as above. Commands: `cargo test -p annotator hier`, `cargo test -p library --test hier_annotate`,
then `cargo test -p annotator` (corpus/permutation tests unchanged).

---

## EXT-28 Signal-flow and current-flow ordering

### Current code

- No ordering data (`grep current_flow` empty). `sidecar.rs:208` diagnoses `Order` as `sidecar_unconsumed`;
  `tests/align_gold.rs:65` asserts `(4, 1, 5)` for (unsupported, unconsumed, total).
- Measured (probe annotate, this tree): three_stage classes `vin_p/vin_n/n1` Sensitive, `vbias` Bias, `tail/n2/n3/vout`
  Signal; strongarm `vin/vip/vin_o/vip_o` Sensitive, `clk` Clock, `vcom/vin_d/vip_d` Signal, `vop/von` DigitalStatic.
- \* Without an op point `evidence::device_facts` (evidence.rs:95–160) assigns only Diode/Load/Passive, so the plan's
  "structural chain ... of saturated-role devices" filters nothing; the rule below replaces it.
- \* Plan signatures return group ids from `tree`; changed to device steps (see Shared).

### Edits

New `backend/annotator/src/flow.rs`:

```rust
/// Signal stages: BFS from input nets over net edges control→drain (FET G→D, BJT B→C) and
/// source→drain (S→D, E→C), rails never traversed. Inputs: nets touched only by G/B terminals,
/// class Signal or Sensitive, ∩ `ports` when `ports` is non-empty. Step k = devices whose D/C net
/// is at level k ≥ 1, sorted by (canon, id).
pub fn stage_order(hg: &BipartiteHypergraph, classes: &[NetClassification], ports: &[NetId], canon: &[u64]) -> Vec<Vec<DeviceId>>;
/// Per supply-to-ground conduction chain, the steps from ground up and the chain current (µA, 0 without op).
pub fn current_paths(hg: &BipartiteHypergraph, op: Option<&OpFacts>, classes: &[NetClassification], canon: &[u64]) -> Vec<(Vec<Vec<DeviceId>>, f64)>;
```

`current_paths`:
1. Channel edges low→high: NMOS S→D, PMOS D→S, NPN E→C, PNP C→E; with an op point drop devices with
   `|Id| < 0.01·max|Id|` (**Philis threshold**, plan). Ground = class Ground, Supply = class Supply.
2. `depth(net)` = longest path from a Ground net (Kahn order over the net DAG; a net on a cycle gets none and its
   devices drop out); `height(net)` = longest path to a Supply net. Device depth = depth(low)+1, height = height(high)+1.
3. Components = union-find of non-rail nets joined by a device. Per component, L = max over devices with Supply high
   net of depth; step k (1..=L) = devices with depth k and depth+height−1 = L.
4. In a step holding a device whose gate is not class Clock, drop the Clock-gated ones (precharge switches parallel
   to a load, strongarm `mp9/mp10`); a step of only Clock-gated devices stays (a clocked tail, `mn0`). **Philis policy.**
5. Chain current = Σ|Id| of step 1 (op) else 0.

`lib.rs` after `intent.devices` (lib.rs:387): user orders first (`cfg.order`), then each `current_paths` chain as
`Order { dir: V, reversible: true, weight: I/I_max or 1.0 }` unless a user order has the same steps in either sense,
then `stage_order` (when ≥ 2 steps) as `Order { dir: H, reversible: true, weight: 1.0 }`, then EXT-27's `array_orders`.

Sidecar: `AnnotationConfig.order: Vec<analog::intent::Order>` (netrole.rs after `kelvins`); `"Order"` arm resolves
`instances` through `alias` or `device` (as `SymmetricBlocks`), `direction` `bottom_to_top`/`left_to_right` as given,
`top_to_bottom`/`right_to_left` reversed, `reversible: false`, weight 1.0; another direction → `sidecar_unsupported`.
Update the module doc (sidecar.rs:5) and the table.

### Tests

- `flow::tests::three_stage_stage_order`: names of `stage_order` steps on `crate::tests::three_stage()` (no ports),
  each sorted, `== [["M1","M2","M4","M5"], ["M6","M7"], ["M8","M9"]]` (M3, on unreached `tail`, absent).
- `flow::tests::op_drops_idle_branch`: three_stage with an op where M8/M9 carry 0.001 µA and the rest 10 µA: no chain
  contains M8 or M9.
- `tests/align_gold.rs::strongarm_order_matches_gold`: `annotate(net(STRONGARM), strongarm_cfg())`; the
  `intent.order` entry with `dir == V` containing `mn0` has step names `[["mn0"], ["mn1","mn2"], ["mn3","mn4"],
  ["mp5","mp6"]]` and `reversible`; chains `[["mn13"],["mp11"]]`, `[["mn14"],["mp12"]]` also present.
- `tests/align_gold.rs::gold_order_entry`: `from_json(GOLD_JSON)` gives `cfg.order == [Order { steps: [[mp5,mp6],
  [mn3,mn4],[mn1,mn2],[mn0]] (ids), dir: V, reversible: false, weight: 1.0 }]`; annotate with it: exactly one
  `intent.order` entry covers `mn0` and it is the user one (the extracted chain was its reverse: gold reproduced).
- Changed by design: `symmetric_blocks_become_seeds` (align_gold.rs:65) `(4, 1, 5)` → `(4, 0, 4)` (Order consumed).
- Commands: `cargo test -p annotator flow`, `cargo test -p annotator --test align_gold`, `cargo test -p annotator`.

---

## EXT-29 Structure and bias audit diagnostics

### Current code

- None. Inputs exist: `MatchSpec { members, reference, family, kind, class }` (intent.rs), `DeviceOp { id_ua, gm_us,
  vgs_mv, vth_mv, gds_us, .. }` and `OpFacts.net_mv` (evidence.rs:28–46; V_DS/V_CE from `net_mv` of D/S, C/E),
  `analog::matching::class::limit(Family, MatchKind, MatchClass) -> Option<ClassLimit::{Mv, Pct}>` (class.rs:145),
  cascode patterns `cascode_mirror`, `wide_swing_cascode_mirror`, `low_voltage_cascode_mirror` (catalog.rs:665–720,
  slots 0 bottom ref, 1 bottom out, 2 top ref, 3 top out) in `all` (lib.rs, `pattern::recognize_all`).
- \* Plan signature `audit(intent, nl, drawn, ev)`: `drawn` unused (W/L from `Device::mos_size`, netlist.rs:71) and the
  cascode check needs the matches. \* The metadata report has no diagnostics field (`MetadataReport`,
  `frontend/library/src/metadata.rs:83`); `library` never reads `intent.diagnostics`.

### Edits

New `backend/annotator/src/audit.rs`:

```rust
/// Every kind this module emits (the metadata report filters on it).
pub const KINDS: [&str; 7] = ["vgst_low", "clm_mismatch", "cascode_ratio", "cascode_bulk", "bjt_ratio", "vce_unequal", "audit_not_checked"];
/// H13-32/33/34, H09-04/23 checks; never alters constraints. `cascodes`: slot order 0..3.
pub fn audit(intent: &Intent, nl: &Netlist, cascodes: &[[DeviceId; 4]], op: Option<&OpFacts>) -> Vec<Diagnostic>;
```

1. `vgst_low`: `kind == Current`, `class >= Moderate`, FET family; per member with an op: `|vgs|−|vth|` when both
   known, else `2·|Id|/gm·1000` mV (gm > 0); one diagnostic per set listing the members below 100 mV.
2. `clm_mismatch`: same sets; ref = `members[reference.unwrap_or(0)]`; with `gds_ref`, `Id_ref ≠ 0` and V_DS of both
   known: `100·gds_us·|ΔV_DS mV|·1e-3/|Id_ref µA| > X/2`, X from `limit(family, Current, class)` `Pct` → one per (ref, i).
3. `cascode_ratio`: r(d) = W_total·m/L from `mos_size`; `|r0/r1 − r2/r3| > 0.01·r2/r3` → one; `cascode_bulk` per slot
   2/3 device whose `B` net ≠ `S` net.
4. `bjt_ratio`: `family == Bipolar`, `class >= Moderate`: units = parallel·series, N = units/min; any N > 16 or (N odd
   and N > 1) → one. With an op: `max−min` of V_CE over members > 10 mV (**Philis threshold**) → `vce_unequal`.
5. `op == None` and some set qualifies for 1, 2 or the V_CE check → one `audit_not_checked` naming the checks.

`lib.rs`: after `intent.devices` (lib.rs:387) `let cascodes = all.iter().filter(|m| matches!(m.template,
"cascode_mirror" | "wide_swing_cascode_mirror" | "low_voltage_cascode_mirror")).map(..).collect::<BTreeSet<_>>()`
(dedup), then `intent.diagnostics.extend(audit::audit(&intent, netlist, &cascodes, ev.op.as_ref()))`.

`library`: `MetadataReport.audit: Vec<String>` (doc: EXT-29 findings, `kind: message`); set after `metadata::build`
(lib.rs:1015–1026) from `flow.problem.intent.diagnostics` filtered on `annotator::audit::KINDS`; Display prints
`  AUDIT: ...` when non-empty, next to `AGING` (metadata.rs:476).

### Tests

`audit::tests` (hand-built `MatchSpec`s and `OpFacts`):
- `vgst_floor`: Current set, Moderate, 2 members, gm 20 µS, Id 0.6 µA → exactly one `vgst_low` naming both; gm 10 µS
  (120 mV) → none.
- `bjt_ratio_odd`: NPN set Moderate, parallel 1:7 → one `bjt_ratio`; 1:8 → none; 1:7 Minimal → none.
- `cascode_ratio_mismatch`: bottom 2u/1u : 4u/1u, top 2u/0.5u : 2u/0.5u → one `cascode_ratio`; all equal → none.
- `clm_from_vds`: Current Moderate (X = 3 %), gds 1 µS, Id 10 µA, ΔV_DS 200 mV → 2 % > 1.5 % → one; 100 mV → none.
- `no_op_not_checked`: same set, `op = None` → only `audit_not_checked`.
`tests/corpus.rs::audit_on_corpus`: every corpus circuit through `annotate_with` with an op on every device
(Id 10 µA, gm 100 µS → 200 mV): no panic, three_stage has no `vgst_low`; same with gm 400 µS (50 mV): three_stage has
a `vgst_low` on {M4, M5} (the Current Moderate load set). Commands: `cargo test -p annotator audit`,
`cargo test -p annotator --test corpus`, `cargo test -p library metadata`.
