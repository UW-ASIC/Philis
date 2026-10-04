# M2 annotator, segment 1: EXT-12, EXT-13, EXT-14, EXT-15, EXT-16, EXT-19

Branch `m2-annotator`, merged with `m2` at `c867abb` (GAP-01 in: `pnr_core::MatchClass`, `analog::matching::class`).
Specs: plan-01 `### EXT-1x`. Overrides applied: §6.1 C1, C4, C14, C16, C20; §6.2 (`BatchMeta { id, origin }`, no
`family`); dag-m2-m6 (EXT-12 and EXT-16 hard dep GAP-01, done; EXT-13 and EXT-14 step-level edges). PLC-06 is not done
in M1, but its `n_axes` line is in the code (`frontend/library/src/lib.rs:838`), which is all EXT-12 needs. Line numbers
below are at `c867abb`.

Order: commit each item on its own, `cargo test -p annotator -p analog` green before each commit, with
`PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`. All six items are **do**.

## Decisions that differ from plan-01 (apply them, cite this card)

- **D-a. `Problem.blocks` is not rebuilt from the HSMPG tree in EXT-13.** It stays the disjoint selection
  (`pattern::select_disjoint`, `pattern.rs:430-451`) until EXT-20 replaces the per-block emit loop. `emit.rs:94` uses
  `AxisId(block index)` and the leaf loop reads `blocks`. Rebuilding `blocks` first would change emission twice and
  rewrite every corpus row for nothing. The consumer, PLC-08/09 (plan-04 L394), reads `Intent.tree`, not `blocks`.
- **D-b. `Problem.axis_count = blocks.len()` until EXT-20.** EXT-14 step 6 says `compounds.len()`, but emitted
  `Symmetry` still uses `AxisId(block index)`. With fewer axes than blocks, `Layout.axis[bi]` would be out of range.
  EXT-20 switches it to `compounds.len()` in the same commit that emits per-compound axes.
- **D-c. EXT-14 does not change emission**, so `Canon` (emitted hard `Symmetry`, `tests/common/mod.rs:161`) cannot
  check it. Compounds get their own view, `canon_intent`, and their own test. `align_gold` reads `canon_intent` until
  EXT-20.
- **D-d. Compound `kind = Perfect` (EXT-14 step 9) lands in EXT-16.** It needs set classes. Until then every compound
  is `Mirror`.
- **D-e. The EXT-14 `Sig` struct is replaced by a private tuple** built from `size::Drawn` and `hg.kinds`.
  `analyze` takes `kinds`/`drawn` instead of `sig`, so there is no new public type.
- **D-f. EXT-15 keeps `constraints::assemble` for block devices that no set covers.** Without it, Stack/Group
  members such as `chain4` lose their Unitization and draw as singletons. Only set members move to per-set
  Unitizations.
- **D-g. `dummy_required` by class applies to MOS, R and C only.** Bipolar sets keep `false`, as cellgen's
  `bjt_groups` does today (`cellgen.rs:507`), so `bgr_core` draws the same (EXT-19 T9). Hastings MOS rule 12 is a MOS
  rule.
- **D-h. `cells::cap_array::bits` is not reachable** from `annotator`, which has no `cells` dependency. EXT-19 checks
  binary counts inline (sorted counts equal `[1, 1, 2, …, 2^(N-1)]`).
- **D-i. EXT-16's spec rule applies only where `limit(...)` is `Mv`.** `limit` returns `Option<ClassLimit>`
  (`class.rs:145`), not `ClassLimit`, and `offset_sigma_mv` is in mV. Current/Ratio sets use Role until EXT-21 gives
  % allowances.
- **D-j. `NetClass` has no `DigitalSwitching` value** (`metadata.rs:18-27`), so the EXT-13 `ProxNet` exclusion is
  Supply, Ground, Substrate and Clock.
- **D-k. No "largest group size" metadata field.** It can be computed from `intent.tree`; add one when a reader
  needs it.
- **D-l. EXT-15 step 3 is not drawn as written (review fixes 1).** The per-set Unitization uses schematic fingers
  (`dev_nf = nf·m`, `unit_w` = finger W, `series` all 1), not `Member::parallel` and `UnitGeom`, and a set whose
  members differ in finger W/L gets none (`constraints.rs`, `if !same { continue; }`). Reason: the LVS reference
  expands `nf·m` fingers of W_f, so drawing `W_u ≠ W_f` units gave `lvs.unpaired_device` on `mirror_ratio` and `ota`.
  Consequence: EXT-16's three_stage result does not hold. Its bias set {M3,M7,M9} (finger W differs) gets no set
  Unitization: M3 and M9 keep D-f per-block Unitizations with dummies, and M7 has none. Adopting the inferred unit
  (netlist rewrite or LVS reference in units) is a **CELL/FLOW follow-up**.

---

## EXT-12: `analog::intent` contract (class: do)

**Current code.** `kernel/analog/src/intent.rs:1-25` already exists from EXT-10 (M1). It has `ConstraintId`, a `Copy`
`Origin { Pattern { template }, NetClass }` and `BatchMeta { id, origin }`, and it re-exports `MatchClass`.
`pub mod intent` is already in `kernel/analog/src/lib.rs:16`. GAP-01 landed:

- `pnr_core::MatchClass` (`kernel/core/src/process.rs:100`, already `PartialOrd, Ord, Default`).
- `Family` (`kernel/analog/src/matching/class.rs:11-33`, with `Family::of(DeviceKind)`).
- `MatchKind` (`kernel/analog/src/matching/mismatch.rs:20`, re-exported by `class.rs:5`).

So the plan's fallback that creates `matching/class.rs` does not apply. There is no `AxisDir` anywhere (C14 wants
`analog::placement::symmetry::AxisDir { V, H }`). `Unitization` is at `kernel/analog/src/cell.rs:37-53` with
`#[derive(Clone)]` only. `Problem` is at `backend/annotator/src/lib.rs:32-55`. `n_axes: self.problem.blocks.len()` is
at `frontend/library/src/lib.rs:838`, and the debug probe `axis: vec![0; problem.blocks.len().max(1)]` is at
`lib.rs:1609`.

**Edits.**

1. `kernel/analog/src/placement/symmetry.rs`: add
   `#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] pub enum AxisDir { #[default] V, H }` (C14).
2. `kernel/analog/src/intent.rs`: extend in place and keep the EXT-10 doc comments.
   - `pub use crate::matching::class::{Family, MatchClass, MatchKind}; pub use crate::placement::symmetry::AxisDir;`
     This replaces the current `pub use pnr_core::MatchClass`.
   - Keep `Origin` `Copy`, because `rule::Tagged` copies `BatchMeta`, and keep its `NetClass` variant (it is used in
     `annotate`). Add `Symmetry { seed: ConstraintId }, SharedBias, PassiveSet { rule: &'static str }, Sensitivity,
     User { index: u32 }, Derived { parent: ConstraintId }`.
   - `BatchMeta` stays `{ id, origin }` (§6.2).
   - Add, exactly as plan-01 EXT-12 lists them: `ClassSource`, `Half`, `Term`, `ArrayStyle`, `Member`, `UnitGeom`,
     `MatchSpec`, `SymKind`, `Compound` (field `dir: AxisDir`), `ReqType` (derive `PartialOrd, Ord`, importance order
     `MatchSym < MatchBlock < ProxBlock < Sym < ProxNet`), `GroupKind`, `GroupNode`, `Region`, `DeviceRole`,
     `DeviceFacts`, `RcClass`, `EvidenceLevel`, `NetFacts`, `Inject { Switching, Capacitive }` (GAP-03 adds
     `MinorityElectron`, `MinorityHole` per C4), `Aggressor`, `Victim`, `CommonNodeReq`, `StarReq`, `KelvinReq`,
     `Diagnostic`, and `Intent` (`#[derive(Clone, Debug, Default)]`).
   - `Compound.axis` is `pnr_core::ids::AxisId`. Use `pnr_core::ids::{AxisId, DeviceId, NetId}`.
   - Do not add a root-level `pub use` in `lib.rs` (`verify::Intent` clash).
3. `kernel/analog/src/cell.rs` `Unitization`: append `/// None = not inferred; matched-cell readers use
   unwrap_or(Moderate) (C16). pub class: Option<MatchClass>`, `/// Per member; empty = all 1. pub series: Vec<u16>`
   and `/// None = today's choice. pub style: Option<crate::intent::ArrayStyle>`. Add `class: None, series:
   Vec::new(), style: None` to every full literal:
   - `backend/annotator/src/constraints.rs:51`
   - `frontend/library/src/cellgen.rs:450,478,500,522,547,1359` (`:120` uses `..u.clone()`, no edit)
   - `frontend/library/tests/drawn_cards.rs:27`
   - `kernel/cells/src/lib.rs:73`, `kernel/cells/src/capacitor.rs:386`, `kernel/cells/src/cap_array.rs:610`
   - `kernel/cells/tests/cell_selfcheck.rs:45,417`, `kernel/cells/tests/deck_keys.rs:76`
   - `kernel/macroMaster/src/adapter.rs:29`
   - `examples/three_stage_opamp/src/main.rs:96`

   Re-grep `Unitization {` after merging `m2` again, because other modules add sites.
4. `backend/annotator/src/lib.rs` `Problem`: add `/// Extraction's contract (EXT-12); filled from EXT-13 on. pub
   intent: analog::intent::Intent` and `/// Symmetry axes the placement emits: one per block until EXT-20 (card
   D-b). pub axis_count: usize`. In `annotate`, set `intent: Intent::default()` and `axis_count: blocks.len()`
   (computed before `blocks` moves into `Problem`).
5. `frontend/library/src/lib.rs:838`: change to `n_axes: self.problem.axis_count`. At `:1609`, change to
   `vec![0; problem.axis_count.max(1)]`. Leave `lib.rs:95,477`'s `verify::Intent` alone, and reach the new type by its
   path.

**Tests.**

- `kernel/analog/src/intent.rs` `#[cfg(test)] mod tests`: `match_class_orders`, with
  `assert!(MatchClass::Minimal < MatchClass::Moderate && MatchClass::Moderate < MatchClass::Exceptional)` and
  `assert_eq!(MatchClass::default(), MatchClass::Moderate)`; `req_type_importance`, with
  `assert!(ReqType::MatchSym < ReqType::MatchBlock && ReqType::MatchBlock < ReqType::ProxBlock && ReqType::ProxBlock
  < ReqType::Sym && ReqType::Sym < ReqType::ProxNet)`.
- `backend/annotator/src/tests.rs` `intent_empty_axes_per_block`: on `ota()`, `p.intent.sets.is_empty() &&
  p.intent.compounds.is_empty()` and `p.axis_count == p.blocks.len()`.
- Build: `cargo build --workspace --all-targets`. Every literal must compile.
- Commands: `cargo test -p analog intent && cargo test -p annotator && cargo test -p library --no-run`.

**Acceptance.** The workspace builds. Corpus rows are unchanged (`cargo test -p annotator --test corpus`).

---

## EXT-13: requirement graph and HSMPG tree (class: do)

**Current code.** `annotate` (`lib.rs:106-259`) builds `all = recognize_all(...)` (`pattern.rs:366`) and then
`select_disjoint` (`pattern.rs:430`). `catalog::roles_of(p) -> Roles { pairs, selfs, prox }` (`catalog.rs:2405-2460`)
gives declared couples by slot. `net_classes: Vec<NetClassification>` uses `NetClass` (`metadata.rs:18`).
`canonical_labels -> Vec<u64>` comes from `pattern.rs:393`. `Policy` (`policy.rs:6`, `Default` at `:37`) has no
degree cap.

**Edits.**

1. `policy.rs`: add `/// ProxNet star cap (EXT-13, Philis policy: the survey does not say whether rails are excluded;
   BAL1-49). pub pn_max_degree: usize` with default `8`.
2. New `backend/annotator/src/graph.rs` (`pub mod graph;` in `lib.rs`):
   ```rust
   pub struct Req { pub a: DeviceId, pub b: DeviceId, pub ty: ReqType, pub source: ConstraintId }
   pub fn requirements(matches: &[PatternMatch], compounds: &[Compound], shared_bias: &[Vec<DeviceId>],
                       passive: &[Vec<DeviceId>], hg: &BipartiteHypergraph, classes: &[NetClassification],
                       canon: &[u64], policy: &Policy) -> Vec<Req>;
   pub fn hsmpg(n_devices: usize, reqs: &[Req], canon: &[u64]) -> Vec<GroupNode>;
   ```
   - Edges are as plan-01 states. `MatchBlock` comes from `roles_of(m.pattern).pairs` of every match in `matches` (all
     of them, not only the disjoint ones: AA-01), skipping pairs already `MatchSym`. `ProxBlock` comes from
     `roles.prox` and from `roles.selfs[i]` to `pairs[0].0`. `ProxNet` follows D-j. Store each edge with
     `canon[a] <= canon[b]`.
   - `source` is `ConstraintId(index of the match/compound/group in its input slice)`, which is enough for M2.
   - `hsmpg` takes `canon` as well: the sort `(ty, canon[a], canon[b])` needs it.
   - Union-find is a local `Vec<u32>` with path halving (no crate).
   - Node order: nodes are created in importance order. Children within a node are sorted by
     `(min canon of the subtree, size)`, and the `Root` node is last. Node `children` index into the returned `Vec`.
     Leaves are device nodes: `GroupNode { kind, devices: [d], children: [] }` is **not** used. A child that is a
     device is `devices` and a child that is a group is `children`. Each node's `devices` lists **only** its direct
     device members (state this in the `GroupNode` doc).
3. `annotate`: after `net_classes`, run
   `let reqs = graph::requirements(&all, &[], &[], &[], &hg, &net_classes, &canon, &cfg.policy)` and
   `intent.tree = graph::hsmpg(n, &reqs, &canon)`. EXT-14, EXT-15 and EXT-19 replace the `&[]` arguments when they
   land.
4. `Problem.blocks` is unchanged (D-a).

**Tests** (`graph::tests`, exact). `requirements` is pure, so the compounds and shared groups are built by hand, and
neither test waits on EXT-14 or EXT-15.

- `ota5t_tree`: `crate::tests::ota()`, with a hand `Compound` holding pairs `[(XM1,XM2),(XM3,XM4)]` and selfs
  `[XM5]`. Expected nodes, by name sets: `Matching{XM1,XM2}`, `Matching{XM3,XM4}`, `Proximity{Matching{XM1,XM2},
  XM5}`, `Symmetry{that Proximity, Matching{XM3,XM4}}`, then `Root` holding the Symmetry node only.
- `tail_in_two_requirements`: three_stage netlist (corpus text; add a `pub(crate) fn three_stage()` to `tests.rs`).
  Use a hand compound with pairs `[(M1,M2),(M4,M5)]` and selfs `[M3]`, plus `shared_bias = [[M3,M7,M9]]`. Assert
  that `reqs` holds a `MatchBlock` between M3 and M7 and one between M3 and M9, and a `Sym` edge touching M3. Assert
  also that the tree has one Symmetry node whose subtree contains M3, M7 and M9 (plan risk note).
- `tree_is_permutation_invariant` (T4): for every `CIRCUITS` entry of the corpus, the tree mapped to names (a nested
  `BTreeSet`) is equal for `permute(seed 1..=3)`. This goes in `tests/corpus.rs`, next to `permutation_invariance`,
  using `p.intent.tree`.

Command: `cargo test -p annotator graph:: && cargo test -p annotator --test corpus`.

**Acceptance.** AA-01 is checked on `mirror6` and `three_stage` once EXT-15 supplies `shared_bias` (that
`corpus.rs` assertion lands in EXT-15's commit: `mirror6` has one `Matching` node with all 6 devices). T4 holds.

---

## EXT-14: circuit-level symmetry, compounds (class: do; step 8 lands with EXT-15, per the dag)

**Current code.**

- Symmetry exists only through emit: per block, `AxisId(bi)` (`emit.rs:94,120,174`).
- `pattern::pins(hg) -> Vec<[Option<NetId>; 8]>` (`pattern.rs:98`) uses columns `G D S B C E P N`
  (`pattern.rs:92-94`). `pin_net` is at `pattern.rs:86`.
- Terminal names are in `hg.terminals[i]` and nets in `hg.device_nets[i]`.
- `size::Drawn { w_finger_nm, l_nm, fingers, model, bulk }` (`size.rs:14-22`). `fingers` is `nf·m` for MOS and `m`
  otherwise.
- `align_gold.rs:27` is `#[ignore = "passes after EXT-14"]`.
- `Canon` (`tests/common/mod.rs:138-152`) reads emitted hard `Symmetry`.

**Edits.**

1. New `backend/annotator/src/symmetry.rs` (`pub mod symmetry;`):
   ```rust
   pub enum Seed { Devices(DeviceId, DeviceId, ConstraintId), Nets(NetId, NetId, ConstraintId), SelfDevice(DeviceId, ConstraintId) }
   pub fn analyze(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification], seeds: &[Seed],
                  canon: &[u64]) -> (Vec<Compound>, Vec<Diagnostic>);
   ```
   - Signature equality is `fn sig(k: DeviceKind, s: &Drawn) -> (DeviceKind, u16, Option<i64>, Option<i64>, u32)`
     (D-e). `bulk` is left out, because a PMOS pair's bulk may be its source.
   - `policy` is dropped from the signature (unused).
   - Steps 1-7 are exactly plan-01. Rails are nets whose `classes[n].class` is `Supply`, `Ground` or `Substrate`.
   - Two-terminal passives match in either orientation.
   - Ambiguity gives `Diagnostic { kind: "ambiguous_symmetry", devices, message }`. A seed that conflicts with an
     existing pairing gives `kind: "conflicting_seed"`.
   - `Compound.dir = AxisDir::V` and `kind = SymKind::Mirror` (D-d).
   - `axis = AxisId(index)`, with compounds sorted by the canonical order of each compound's smallest pair:
     `min (canon[a], canon[b])`, with names never used.
   - `net_pairs` are stored A-first, and `self_nets` lists the self-symmetric non-rail nets.
2. Seeds in `annotate` come from `block::leaves(&blocks)` of kind `DiffPair`, `Load` and `CascodePair` (the disjoint
   leaves, so seeds never contradict). Each is `Seed::Devices(d0, d1, ConstraintId(leaf index))`. Seeds are sorted by
   `(min canon, max canon)` of the pair, with `names` breaking exact ties as `select_disjoint` does.
   `Nets`/`SelfDevice` seeds come from the sidecar (EXT-26, M3). The enum carries them now, and nothing builds them
   yet.
3. `annotate`: run `(intent.compounds, d) = symmetry::analyze(...)`, append `d` to `intent.diagnostics`, and pass
   `&intent.compounds` to `graph::requirements` (EXT-13's `&[]`). `axis_count` is unchanged (D-b).
4. Step 8 (`set_pairs`) is built in EXT-15. Step 9's `Perfect` rule is in EXT-16 (D-d).

**Tests.**

- `tests/common/mod.rs`: add `pub fn canon_intent(p, nl) -> Canon`. It fills `pairs` (names sorted), `selfs`,
  `net_pairs` and `axes = p.intent.compounds.len()` from `p.intent.compounds`. Put the doc note "replaced by `canon`
  once EXT-20 emits per compound" on it.
- `tests/corpus.rs` `compound_expectations`, with exact rows on `canon_intent`, per plan-01 EXT-14:
  - `ota5t`: pairs {(XM1,XM2),(XM3,XM4)}, selfs {XM5}, net_pairs {(vinm,vinp),(vout1,vout2)}, axes 1.
  - `folded`: 5 pairs, self M0, net_pairs {(vinn,vinp),(x1,x2),(o1,out),(y1,y2)}, axes 1.
  - `gilbert`: pairs {(M1,M2),(M3,M6),(M4,M5)}, self M0, net_pairs {(rfn,rfp),(x1,x2),(outn,outp)}, axes 1, and
    `self_nets ⊇ {tail, lop, lon}` (a separate assert on the compound).
  - `rail2rail`: pairs {(MN1,MN2),(MP1,MP2)}, selfs {MN0,MP0}, axes 1.
  - `latch`: pairs {(MN1,MN2),(MP1,MP2)}, net_pairs {(q,qb)}, axes 1.
  - `three_stage`: pairs {(M1,M2),(M4,M5)}, selfs {M3}, axes 1, and M6, M7, M8, M9 in no pair.
  - Every `NEGATIVE` circuit and `bjt_mirror` get 0 compounds (T3, in `negative_corpus`'s helper).

  Names are sorted inside a pair, as `sorted()` does. Hand-check `folded`'s (o1,out) and `gilbert`'s (lop,lon)
  orientation against step 7 before fixing the row.
- `tests/align_gold.rs`: switch to `canon_intent` and remove the `#[ignore]`. If strongarm does not reach all 7 gold
  pairs, keep the ignore and report the missing pairs with evidence. Do not edit `GOLD_*`.
- `permutation_invariance` also compares `canon_intent` (T4).
- `corpus_expectations` must stay unchanged (emission is untouched).

Command: `cargo test -p annotator --test corpus --test align_gold && cargo test -p annotator`.

**Acceptance.** T1 (align_gold), T2 symmetry part (above), T4.

---

## EXT-15: matched sets, shared-bias groups, ratio inference, unitization (class: do)

**Current code.**

- `backend/annotator/src/constraints.rs:27-88` `assemble` emits one Unitization per block and per
  `(kind, model, bulk, W_f, L)` class, with `target_ratio = dev_nf` (`:51-63`) and `dummy_required` and
  `route_matching_required` always true.
- cellgen's side recognizers: loops at `frontend/library/src/cellgen.rs:471-535` (`dac_banks`, `bjt_groups`,
  `parallel_groups`) and functions at `:738-810`. The singleton fallback is at `:537-575`. All of them skip devices
  the annotator's Unitizations already cover (`covered`, `:465-470`).
- Deck keys: `min_finger_width` at `pdks/sky130.json:6`, `max_finger_width` at `:59` (not `:40`), and
  `res_min_segment` at `:68`. `ProcessNumbers` is `Copy + Default` (`netrole.rs:144`) and is filled at
  `frontend/library/src/lib.rs:743-760`.

**Edits.**

1. New `backend/annotator/src/sets.rs` (`pub mod sets;`):
   ```rust
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub struct UnitDeck { pub grid_nm: i64, pub min_w_nm: i64, pub max_w_nm: i64, pub min_l_nm: i64, pub res_min_segment_nm: i64 }
   pub fn shared_bias_groups(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification]) -> Vec<Vec<DeviceId>>;
   pub fn unitize(members: &[DeviceId], drawn: &[Drawn], kind: DeviceKind, class: MatchClass, deck: &UnitDeck)
       -> Result<(UnitGeom, Vec<(u16, u16)>), Diagnostic>;
   pub fn matched_sets(reqs: &[Req], compounds: &[Compound], shared: &[Vec<DeviceId>], kinds: &[DeviceKind],
                       drawn: &[Drawn], hg: &BipartiteHypergraph, deck: &UnitDeck, diags: &mut Vec<Diagnostic>) -> Vec<MatchSpec>;
   ```
   - `tree` and `passive` are dropped from `matched_sets`. Sets are components of the `MatchSym ∪ MatchBlock` edges,
     and passive groups arrive as `MatchBlock` stars through `reqs` (EXT-13), so neither is needed.
   - A `0` in `UnitDeck` means the key is missing, which gives `Err("unit_deck_incomplete")` and `unit: None`.
   - Rules, gcd floor, series/parallel and resistor/capacitor/BJT rules are exactly plan-01 steps 1-3. Use
     `class >= MatchClass::Moderate` for the 1000 nm floors.
   - `MatchSpec` fields:
     - `id = ConstraintId(set index)`.
     - `origin = SharedBias` if the set came from a shared group, else `Pattern { template }` of its first leaf.
     - `members` in canonical order. `half` comes from the compound pair (A = first). `parallel` and `series` come
       from `unitize`, or `(units, 1)` when it fails.
     - `reference` is the single diode-connected member (FET D == G), otherwise `None`.
     - `family = Family::of(kind)`.
     - `kind` is interim: `Ratio` for R/C, `Current` otherwise. EXT-16 replaces it.
     - `class = Moderate`, `class_source = Role`, `allowance = None`, `weight = None`, `style = Any`.
     - `compound` is the compound index holding any member.

     A mixed-kind component is split by `(kind, model)` and pushes `Diagnostic "mixed_kind_set"`.
   - EXT-14 step 8 goes here: `compound.set_pairs` gets `(i, j)` when `pair_of` maps set i's members bijectively onto
     set j's. `analyze` therefore also returns `pair_of: Vec<Option<DeviceId>>`, or exposes it on `Compound` through
     `pairs`. Build the map from `compound.pairs` (no signature change).
2. `netrole.rs` `ProcessNumbers`: add `/// Unitization bounds (EXT-15); 0 = deck key missing. pub unit:
   crate::sets::UnitDeck`. `frontend/library/src/lib.rs` `annotation()` fills it:
   - `grid_nm: i64::from(pdk.grid())`
   - `min_w_nm: pdk.rule("min_finger_width", 0)`
   - `max_w_nm: pdk.rule("max_finger_width", 0)`
   - `res_min_segment_nm: pdk.rule("res_min_segment", 0)`, read the way `kernel/cells/src/resistor.rs:405` does
   - `min_l_nm` = poly min width through `pdk.min_width` (the same call shape as `width` at `lib.rs:740`)

   Verify each value against sky130: 5, 420, 10000, 150, 10000.
3. `constraints::assemble(netlist, drawn, blocks, sets: &[MatchSpec])` changes as follows:
   - Emit one Unitization per set: `devices` = members, `dev_nf` = parallel, and `target_ratio = parallel /
     gcd(parallel)`.
   - `unit_w`/`unit_l` come from `UnitGeom`, falling back to today's class values when `unit: None`.
   - `dummy_required = class >= Moderate` for Mos/Resistor/Capacitor and `false` for Bipolar (D-g).
   - `route_matching_required = kind != Ratio || class >= Moderate`, `class: None` (EXT-16 sets it),
     `series` per member, `style: None`.
   - Then emit today's per-block classes over block devices in no set (D-f).
   - Parallel identical-terminal MOS (all four nets equal, same `sig`) that are not in any set get a Unitization with
     `route_matching_required = false`, `dummy_required = false` (cellgen's `parallel_groups` rule, `:515-535`), and
     no `MatchSpec`.
4. `annotate`: `shared = sets::shared_bias_groups(...)` goes before `graph::requirements` (replacing `&[]`). Then
   `intent.sets = sets::matched_sets(...)` and `constraints::assemble(..., &intent.sets)`.
5. cellgen deletion is EXT-19 step 4, not here.

**Tests.**

- `sets::tests` (deck `UnitDeck { grid_nm: 5, min_w_nm: 420, max_w_nm: 10_000, min_l_nm: 150, res_min_segment_nm:
  10_000 }`, class Moderate unless stated):
  - `mirror_ratio_by_width`: W 2/4/8 µm, L 1 µm gives `unit.w_nm == 2000` and parallel `[1,2,4]`.
  - `three_stage_bias_group`: `shared_bias_groups` on three_stage gives exactly `[[M3,M7,M9]]` (sorted by name), and
    unitize gives W_u 2000 and parallel `[4,3,10]`.
  - `hastings_unit_example`: W_u 10000, parallel `[10,20]`.
  - `series_parallel_20_to_1`: L_u 1000, W_u 10000, series `[4,1]`, parallel `[1,5]`.
  - `resistor_divider`: unitize RA/RB of `rdiv` gives `unit.l_nm == 10000` and series `[1,4]`.
  - `missing_deck_key`: `UnitDeck { max_w_nm: 0, .. }` gives `Err` with kind `"unit_deck_incomplete"`.
  - `non_integer`: W 2 µm and 3 µm, L 1 µm and 1.5 µm gives `Err("non_integer_ratio")`.
- `tests/common/mod.rs` `canon` fills `sets` from `p.intent.sets`, as `(members (name, parallel, series) sorted,
  format!("{:?}", kind), format!("{:?}", class))`. `corpus.rs` `Row` gains a `sets` column. Every row that changes is
  edited in this commit with a one-line comment, as the corpus file asks.

  Known rows:
  - `mirror6`: one set `[(MO1,1,1),(MO2,2,1),(MO3,1,1),(MO4,4,1),(MO5,1,1),(MR,1,1)]`, and `reference` = MR (a
    separate assert).
  - `three_stage`: the bias set `[(M3,4,1),(M7,3,1),(M9,10,1)]`.

  Verify every other row's sets against the spec before fixing it. The EXT-13 AA-01 assertion (one 6-device
  `Matching` node on `mirror6`) is added here.
- `benchmarks/fixtures/mirror_ratio.spice` (M1 Status: "check whether the annotator splits the mirror"): an annotator
  test asserts that its mirror devices are one `MatchSpec` and one Unitization.
- `bench local` on `dac4`, `bgr_core`, `pair`, `quad`, `ota`, `mirror_ratio` (release): DRC 0, LVS as `m2`. Report
  any drawn change.

Command: `cargo test -p annotator && cargo test -p library --release`, then `bench local` as the M1 report runs it.

**Acceptance.** T2 for `mirror6` and `three_stage`. `rdiv` and `bgr_core` (units [1,8]) need the passive and bipolar
requirement edges, so their T2 is EXT-19's (plan correction).

---

## EXT-16: precision class and match kind per set (class: do)

**Current code.**

- `limit(f, k, c) -> Option<ClassLimit>` is at `kernel/analog/src/matching/class.rs:145`, with tables 10/3/1 mV for
  Mos Voltage.
- `AnnotationConfig.offset_sigma_mv: Option<f32>` is at `netrole.rs:137`.
- The `frontend/library/src/lib.rs:871-872` consumer edit is dropped: C20 gives it to PLC-29 and GAP-01 step 5.
- `SetRole`'s passive roles come from EXT-19.

**Edits.**

1. New `backend/annotator/src/class.rs` (`pub mod class;`):
   ```rust
   pub enum SetRole { InputPair, LoadOfPair, BiasMirror, BandgapCore, DacBank, FeedbackRatio, Other }
   pub struct ClassCtx<'a> { pub user: Option<MatchClass>, pub spec_6sigma: Option<f32>, pub role: SetRole, pub diags: &'a mut Vec<Diagnostic> }
   pub fn kind_of(set: &MatchSpec, leaf_kinds: &[BlockKind], dk: DeviceKind) -> MatchKind;
   pub fn class_of(set: &MatchSpec, ctx: &mut ClassCtx) -> (MatchClass, ClassSource);
   pub fn style_of(class: MatchClass, kind: MatchKind, shares_source: bool) -> ArrayStyle;
   ```
   - `ClassCtx.family` is dropped, because `set.family` carries it.
   - The rules are plan-01's, except that the Spec rule applies only when `limit(set.family, set.kind, _)` is
     `Some(Mv(_))` (D-i). The rule order is User, then Spec, then Role.
   - `user` is always `None` until EXT-26 (M3).
   - `spec_6sigma = cfg.offset_sigma_mv.map(|s| 6.0 * s)`.
   - Role order (first that holds): InputPair if any member pair is a `DiffPair` leaf; LoadOfPair if a `Load` or
     `CascodePair` leaf whose compound also holds an InputPair set; BiasMirror if a `CurrentMirror` leaf or origin
     `SharedBias`; the passive role from EXT-19's output once it exists; Other otherwise. Three_stage's `{M4,M5}` is
     therefore LoadOfPair (Moderate), not BiasMirror.
   - `leaf_kinds` are the kinds of `block::leaves(&blocks)` whose both devices are in the set.
2. `annotate`, after `matched_sets`, for each set: `kind = kind_of`, `(class, class_source) = class_of`,
   `style = style_of`. Re-run `sets::unitize` with the inferred class, because the floors depend on it. Then:
   - Unitization `class: Some(set.class)`, `style: Some(set.style)`, and `dummy_required` from the class (D-g).
   - Compound `kind = Perfect` if it holds a set with `kind == Voltage && class == Exceptional`, else `Mirror`
     (EXT-14 step 9, D-d).

**Tests** (`class::tests`, exact).

- `spec_maps_to_class`: Mos Voltage, X = 12 gives Minimal, 5 gives Moderate, 2 gives Exceptional, and 0.5 gives
  Exceptional plus one `beyond_exceptional_trim` diagnostic. All with `ClassSource::Spec`.
- `user_wins_over_spec_and_role`: `user: Some(Minimal)`, spec 2 mV, role InputPair gives `(Minimal, User)`.
- `current_kind_ignores_mv_spec` (D-i): a Mos Current set with `spec_6sigma: Some(2.0)` and role BiasMirror gives
  `(Minimal, Role)`.
- `style_by_class`: Minimal gives Adjacent; Moderate with a shared source gives Interdigitated; Moderate without one
  gives CommonCentroid1d; Exceptional gives CommonCentroid2d.

Corpus `sets` rows are updated in this commit:

- `ota5t`: DP `Voltage Moderate`, load `Current Moderate`.
- `three_stage`: bias `{M3,M7,M9}` `Current Minimal`.

`dac4` (Exceptional Ratio) and `bgr_core` (Moderate Voltage) are EXT-19's rows. Also add a test that every set has a
`class_source` (T10: `intent.sets.iter().all(...)` is trivially typed, so T10 is the corpus rows).

Run `bench local` because dummies change with class (`three_stage`'s bias set becomes Minimal, so it has no dummies).
DRC 0 and LVS unchanged, with any drawn change reported.

Command: `cargo test -p annotator`.

---

## EXT-19: passive, bipolar and diode recognition; cellgen recognizers moved (class: do)

**Current code.**

- `SlotKind { AnyFet, SameTypeAs, ComplementOf }` is at `pattern.rs:26-30`.
- `unary_ok` rejects every non-FET at `pattern.rs:158`.
- `is_diode` is FET D == G only, at `pattern.rs:143-146`.
- `slot_ok`'s kind match is at `:167-171`.
- The pin table already has `C E P N` columns (`:92-94`).
- The cellgen recognizers (`frontend/library/src/cellgen.rs:471-535`, `:738-810`; plan's `486-549, 717-786` are
  stale) have no callers outside cellgen, and RTE-20 has not landed (no `dac_banks` reader). The step-4 RTE-20 switch
  is therefore moot.
- Corpus fixtures `bgr_core`, `bjt_mirror`, `brokaw`, `rdiv`, `dac4` and `splitdac` exist (`corpus.rs:15-62`).
  `bjt_mirror` is in `NEGATIVE`'s T3 set.

**Edits.**

1. `pattern.rs`:
   - Add `SlotKind::Kind(DeviceKind)` and `SlotKind::SameKindAs(u8)`.
   - `unary_ok` does its FET check only for `AnyFet`, `SameTypeAs` and `ComplementOf`.
   - `is_diode` becomes FET `D == G` or BJT `C == B`.
   - `gate_is_signal` reads `B` for BJTs.
2. `catalog.rs`: add `bjt_ratioed_pair`, `bjt_diff_pair` and `bjt_mirror` per plan-01 step 2, with `ROLES` entries.
   `bjt_ratioed_pair` gets `pairs: [(0, 1, CurrentMirror)]` so that it yields a `MatchBlock` edge. Its kind becomes
   Voltage in `kind_of` (BJT ratioed pair gives ΔV_BE). Equal emitter W/L and model use `SizeMatch::ExactAs`, which
   `size::exact_as` must accept for BJTs. Check `size.rs:exact_as` and extend it to compare `w`, `l` and model when
   the kind is bipolar.
3. New `backend/annotator/src/passive.rs` with plan-01's four functions plus
   `diode_sets(nl, drawn) -> Vec<Vec<DeviceId>>`.
   - The `dac_bank` binary test is inline (D-h).
   - Each set reaches EXT-13 as a `MatchBlock` star through `graph::requirements`' `passive` argument (replacing
     `&[]`).
   - Each set's role reaches `class::SetRole`, and `origin = Origin::PassiveSet { rule }` with `rule` as plan-01
     lists.
   - `capacitor_sets` returns the bridge. Store it as a member of the split set, with `reference` unchanged, and push
     `Diagnostic "bridge_cap_value"` when `|C_A − (C_T^LSB/C_T^MSB)·C_u| > 1 % · C_A`, using areas `w·l`.
4. Delete cellgen's `dac_banks`, `bjt_groups` and `parallel_groups` and their loops (`cellgen.rs:471-535,
   738-810`), but only once the annotator's Unitizations cover the same devices with the same `dev_nf`, `unit_w`,
   `unit_l`, `dummy_required` and `route_matching_required`. The loops are then dead (`covered`). Check this with a
   test that runs `annotate` on `dac4`, `bgr_core`, `pair` and `quad` (the bench fixtures) and asserts that every
   device of those groups is in some `p.constraints.unitization`.

   The gate is `bench local` drawn output unchanged on `dac4`, `bgr_core`, `pair` and `quad` (T9: the same cell
   count, area and DRC/LVS as `m2`). If any differs, keep the cellgen code and report the diff. Coordinate with the
   FLOW and CELL owners: cellgen is in `frontend/library`, so tell them in the report.
5. The degenerated diff pair (M1 regression note): add `diff_pair_with_degen` evidence through `degeneration()`. A
   DiffPair whose sources reach one node through one resistor each is recognised. Add the corpus circuit
   `degen_pair`: `M1 o1 inp s1 VSS nfet w=4u l=0.5u | M2 o2 inn s2 VSS nfet w=4u l=0.5u | R1 s1 tail rpoly w=2u
   l=10u | R2 s2 tail rpoly w=2u l=10u | M0 tail vb VSS VSS nfet w=8u l=0.5u`. Expected: one compound with the pair
   (M1,M2), and one `degeneration` Ratio set {R1:1, R2:1}.

**Tests** (corpus, exact; `Row.sets` and `compound_expectations` rows edited in this commit):

- `bgr_core`: set {XQ1:1, XQ2:8}, Voltage Moderate (BandgapCore).
- `brokaw`: three sets:
  - {Q1:1, Q2:8}, Voltage Moderate.
  - {R1:4, R2:1}, Ratio Moderate, `unit.l_nm == 20000`. The units are series units, so the canon member tuples
    are `(R1,1,4)` and `(R2,1,1)`.
  - {MP1, MP2}, Current.
- `rdiv`: `[(RA,1,1),(RB,1,4)]`, Ratio.
- `dac4`: {XC0..XC4} parallel `[1,1,2,4,8]`, DacBank, Exceptional Ratio.
- `splitdac`: one set C0..C4 plus CA, units `[1,1,2,1,2]` on the banks, bridge CA, and no `bridge_cap_value`. A
  second test edits CA to `l=5u` and asserts that the diagnostic is present.
- `bjt_mirror`: no set (it stays in T3).
- `negative_corpus` stays empty for all.
- `passive::tests`: `divider_chain_is_one_set` (rdiv) and `degeneration_inverse_ratio` (pair units [1,2] gives
  resistors with `target_ratio` [2,1]).

Commands: `cargo test -p annotator && cargo test -p library --release`, then `bench local` on dac4, bgr_core, pair,
quad and res_m2.

**Acceptance.** AA-04 closed. T2 on the passive and bipolar circuits, plus EXT-15's `rdiv` and `bgr_core` rows. T9
unchanged on the four bench fixtures. Otherwise step 4 stays undone, with evidence.
