# M1 annotator batch: implementation cards

Branch `m1a-ext-misc` (the task called it `m1a-annotator`), worktree `philis-m1a/ext-misc`, after `git merge m1a` (EXT-02 and EXT-11 merged; clean
merge). Line numbers are of this tree. Every command needs
`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

Order (dependencies first): GAP-04 → EXT-03 → EXT-04 → EXT-05 → EXT-07 → EXT-09 → EXT-06 → EXT-10 → GAP-10.
EXT-06 moves after EXT-05 and EXT-09: its tie key reads EXT-05's roles, and T8 (≤ 2.0 s at 12,500 devices) also
needs EXT-05's deletion of the per-composite re-search and EXT-09's deletion of the two O(N²) `extract` scans.

Corpus rule for every item: `tests/corpus.rs::EXPECTED` characterises today. An item that changes a row edits that
row in the same commit, with a one-line `//` comment naming the edit that moved it. A row that moves for no reason
the item names is a stop-and-report, never a silent edit.

| Item | Class |
|---|---|
| GAP-04 | mechanical (done, verified) |
| EXT-03 | mechanical |
| EXT-04 | mechanical |
| EXT-05 | judgment |
| EXT-07 | mechanical |
| EXT-09 | mechanical |
| EXT-06 | judgment |
| EXT-10 | judgment |
| GAP-10 | mechanical |

---

## GAP-04 Kind-aware isolation; no rule inside a block; substrate key (steps 1–2): done, verified

Committed in `c3d4668` and `69b515a`. Both first-review findings are resolved; nothing to fix:

1. `backend/annotator/src/emit.rs:438-453` `bulk_is_cost_only_and_unknown` asserts
   `SubstrateKind::from_key(Some("epi_on_pplus")) == EpiOnLowRes` (:440) and
   `p.missing.contains(&("Isolation", "bulk substrate: no plateau distance"))` (:447), and `"substrate kind unknown"`
   for `Unknown` (:451).
2. `frontend/library/src/lib.rs:1690-1699` `start_tests::substrate_kind_is_read_from_the_deck`: sky130 → `Bulk`;
   gf180mcu, ihp_sg13g2, generic_finfet → `Unknown`, via `crate::annotation(&Pdk::builtin(name)?, ..)`.

Measured on this tree (debug): `cargo test -p annotator` 42 + 4 passed, 0 failed; `no_emitted_conflicts_strongarm`
is un-ignored and green; `cargo test -p library --lib substrate_kind` 1 passed. The compound/`MatchSpec` part of
step 2 waits for EXT-14/15 (M2); step 3 waits for REL-15.

---

## EXT-03 Terminal roles by name, not by position — mechanical

Current code:
- `backend/annotator/src/classify.rs:11-14` `const G, D, S, B = 0..3`; the loop `classify.rs:62-85` marks
  `touches_gate` on terminal 0 of every non-capacitor device (a BJT's collector, a resistor's first pin).
- `backend/annotator/src/extract.rs:35-40` charges gate area to `nets[0]`.
- The corpus `permute` (tests/common/mod.rs:102) shuffles each device's terminal list, so positional reads also break
  `permutation_invariance`. `pattern.rs`, `size.rs`, `netrole.rs` already read pins by name.

Edits:
1. New `backend/annotator/src/terms.rs` (`pub mod terms;` in `lib.rs`):
   ```rust
   //! What a terminal is, by its name and its device's kind (AA-16): never by position.
   use pnr_core::DeviceKind;

   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   pub enum TermRole { FetGate, Channel, Body, BjtBase, Plate, Passive }

   /// FET `G`/`D`,`S`/`B`; bipolar `B` = base, `C`/`E` = channel; capacitor pins are plates;
   /// anything else (resistor, diode, inductor, unknown pin names) is a passive DC path.
   #[must_use]
   pub fn term_role(kind: DeviceKind, term: &str) -> TermRole {
       use DeviceKind::{Capacitor, Nmos, Npn, Pmos, Pnp};
       match (kind, term) {
           (Nmos | Pmos, "G") => TermRole::FetGate,
           (Nmos | Pmos, "D" | "S") => TermRole::Channel,
           (Nmos | Pmos, "B") => TermRole::Body,
           (Npn | Pnp, "B") => TermRole::BjtBase,
           (Npn | Pnp, _) => TermRole::Channel,
           (Capacitor, _) => TermRole::Plate,
           _ => TermRole::Passive,
       }
   }
   ```
2. `classify.rs`: delete the four consts (:11-14) and their doc line. Replace the loop body (:62-85) with
   ```rust
   for (d, nets) in hg.device_nets.iter().enumerate() {
       for (t, n) in hg.terminals[d].iter().zip(nets) {
           let i = n.0 as usize;
           match term_role(hg.kinds[d], t) {
               TermRole::FetGate => {
                   touches_gate[i] = true;
                   load_um2[i] += gate_um2[d];
                   gate_of_sensitive[i] |= sensitive_devices[d];
               }
               TermRole::Channel | TermRole::BjtBase | TermRole::Passive => touches_channel[i] = true,
               TermRole::Body => touches_bulk[i] = true,
               TermRole::Plate => on_plate[i] = true,
           }
       }
   }
   ```
3. `extract.rs:35-40`: find the gate by role:
   `if let Some(t) = hg.terminals[d].iter().position(|t| term_role(hg.kinds[d], t) == TermRole::FetGate) { gate_nm2[nets[t].0 as usize] += … }`
   (keep the `gate_um2[d] > 0.0` guard; delete the "Terminal 0 is G" comment).

Tests (`classify.rs` `mod tests`, build the graph with `pnr_core::BipartiteHypergraph::from_netlist`, roles with
`crate::netrole::classify_nets(&hg, &AnnotationConfig::default())`, then `classify(&hg, &roles, &[false; n], &gates, None)`):
- `bjt_terminals_are_not_gates`: `XQ1` Npn `[("C",outn),("B",in),("E",VSS)]`, `XQ2` Pnp `[("C",outp),("B",in),("E",VDD)]`,
  nets `outn, in, VSS, outp, VDD`, `params: vec![]`, gates `[0.0; 2]` → class of `outn`, `outp`, `in` all `Signal`
  (today `outn`/`outp` are `Sensitive`).
- `resistor_ends_are_channels`: `R1` Resistor `[("P",a),("N",b)]` + `fet("M1", Nmos, g=a, d=dn, s=VSS, b=VSS, 1_000, 1_000)`
  (`crate::tests::fet`), nets `a, b, dn, VSS` → `a` is `Signal`. Control in the same test: without `R1`, `a` is `Sensitive`.
- Existing `tests.rs::antenna_gate_area_lands_on_the_gate_net_not_the_drain` stays green.

Command: `cargo test -p annotator`. Acceptance: T2 on `bjt_mirror` (M1 exit "bjt_mirror collector nets Signal").

---

## EXT-04 Catalog corrections and catalog validation — mechanical

Current code (`backend/annotator/src/catalog.rs`, anchors still match the plan): `PinRel { Same, Diff }`
(`pattern.rs:12-16`); `links_ok(links, hg, assigned)` (`pattern.rs:115-126`); `DIFF_PAIR` :118-137, `DIFF_SWITCH`
:2228-2247, `WILSON_MIRROR` :492-506, `GILBERT_CELL` :1252-1280, `PATTERNS` :2572-2687 (103 patterns).

Plan corrections:
- **Gilbert sizes.** Slots 2–5 are `same0_exact()` (`ExactAs(0)`), so the LO quad must be sized like the RF pair; the
  corpus `gilbert` (RF 8 µm, quad 4 µm) can never match it, and EXT-05's 3-DiffPair expectation needs it to. The quad
  matches itself, not the RF pair: slot 2 `same0()`, slots 3–5 `same_exact(2)`. (Simulated: with this and the link
  fix, `gilbert_cell` matches the corpus gilbert as `[M1,M2,M3,M4,M5,M6]`.)
- **No pattern is unsatisfiable today** (a union-find simulation of every pattern's links finds no contradiction);
  the listed dead patterns are dead by shadowing or duplication (`STARTUP_MIRROR` ≡ `CURRENT_MIRROR_SINGLE_ENDED`).
  So `every_pattern_matches_its_own_minimal_netlist` is expected to pass after the deletions, and the duplicate check
  is what catches a re-added twin.
- `recognize_all` does not exist until EXT-06; this item adds the per-pattern matcher it will reuse (step 0).

Edits:
0. `pattern.rs`: extract the per-pattern search into
   `pub(crate) fn matches(pat: &'static Pattern, hg: &BipartiteHypergraph, drawn: &[Drawn], roles: &[NetRole], allowed: &[bool]) -> Vec<PatternMatch>`
   (body = today's `Search { .. }.run(..)` with a fresh `seen`); `recognize` calls it per pattern. Make `Search::pat`
   `&'static Pattern` (iterating `PATTERNS` already yields `&'static`).
1. `PinRel::SameSignal` (doc: "same net, and that net's role is `Signal`"). `links_ok(links, hg, assigned, roles)`:
   `PinRel::SameSignal => na.is_some() && na == nb && roles[na.unwrap().0 as usize] == NetRole::Signal` (write it with
   `na.is_some_and(..)`). Update the one caller (`Search::run`). In `catalog.rs` add
   `const fn eq_sig(a, pa, b, pb) -> PinLink` next to `eq`.
2. `DIFF_PAIR` :131 and `DIFF_SWITCH` :2241: `eq(0, "S", 1, "S")` → `eq_sig(0, "S", 1, "S")`.
3. Delete the consts and their `PATTERNS` entries: `DIFF_PAIR_SPLIT_SOURCE` (143), `MIRROR_PAIR_SPLIT_SOURCE` (206),
   `MILLER_COMP_FETS` (2043), `TRIODE_LOAD_PAIR` (2208), `CASCODE_MIRROR_OTA` (1955), `CROSS_COUPLED_COMPLEMENTARY`
   (929), `CURRENT_MIRROR_SINGLE_ENDED` (1419), `STARTUP_MIRROR` (1861), `ACTIVE_LOAD` (299), `DIODE_CASCODE` (1398),
   `DUMMY_PAIR` (2252), each with its doc comment. Nothing outside `catalog.rs` names them.
4. `WILSON_MIRROR` links → `eq(0,"G",1,"G"), eq(0,"S",1,"S"), eq(1,"D",2,"G"), eq(0,"D",2,"S")`; doc: slot 0 = diode
   reference, slot 1 = input, slot 2 = output.
5. `GILBERT_CELL`: slots `[RF+ (unchanged), RF- (unchanged), same0(), same_exact(2), same_exact(2), same_exact(2)]`;
   links `eq(0,S,1,S), ne(0,G,1,G), eq(0,D,2,S), eq(0,D,3,S), eq(1,D,4,S), eq(1,D,5,S), eq(2,G,5,G), eq(3,G,4,G),
   ne(2,G,3,G), eq(2,D,4,D), eq(3,D,5,D)`; delete the "(actually differ …)" comments.
6. Delete doc debris at 1911-1917, 2294-2299, 2341-2345 (the comment blocks above `CASCODE_MIRROR_OTA`, above
   `CURRENT_MIRROR_1_TO_2`'s neighbour, and "actually let's do a useful one"). Rewrite the `DIFF_SWITCH` doc
   (2223-2227) last sentence: "`gate_is_signal: false` means the gate is not checked, so this also matches
   signal-gated pairs; `diff_pair` (priority 10) wins those."

Tests (`catalog.rs` `#[cfg(test)] mod tests`):
- `catalog_is_well_formed`: for every `p` in `PATTERNS`, slot `k`: `SameTypeAs(r)`/`ComplementOf(r)`/`ExactAs(r)`/
  `SameLAs(r)` have `r < k`; every link has `a, b < slots.len()` and pins in `["G","D","S","B"]` (every slot is a
  FET: `slot_ok` rejects others); names unique; no two patterns with equal
  `(format!("{:?}", slots), sorted Vec of format!("{:?}", link))`.
- `every_pattern_matches_its_own_minimal_netlist`: per pattern build a `Netlist`: union-find over the pins
  `(slot, pin)` joined by `Same`/`SameSignal` links and `(k,"G")~(k,"D")` for `DiodeReq::Required`; one net per
  class named `n{i}`; any `B` pin not in a link goes to `VSS` (NMOS) or `VDD` (PMOS). Kinds: slot 0 Nmos;
  `SameTypeAs(r)` = kind of r; `ComplementOf(r)` = the other; `AnyFet` = Nmos. Every device `w=1000, l=1000`,
  model `""`. Assert `pattern::matches(p, ..)` (all devices allowed, roles from `netrole::classify_nets`) contains a
  match whose sorted `instances == 0..slots.len()`. Message names the pattern.
- `tests/corpus.rs::negative_corpus_sc_switches_and_equal_fets`: delete `#[ignore]`; must pass (simulated: after
  steps 2–3 no 2-slot pattern matches either circuit).
- Update `corpus_expectations` rows that move (expected: `gilbert`, possibly `three_stage`, `dac4`, `strongarm`).
  `three_stage`'s `Group [M6, M8, M9]` (`push_pull_with_bias`) stays (plan; AA §2.13-2).

Command: `cargo test -p annotator && cargo test -p annotator --test corpus`.
Acceptance: T3 on `sc_switches`, `equal_fets`.

---

## EXT-05 Declared slot roles in patterns — judgment

Current code: `lib.rs:89-106` builds each composite's children by re-running `recognize` on its instances with
`max_slots = 2` (the AA-03 latch→inverters bug, and at 12,500 devices the T8 killer: N composites × ~90 patterns ×
N devices); `BlockKind::from_template` `block.rs:42-56`; `emit.rs:199-211` puts every stage member outside a pair on
the axis; `Block` literals at `lib.rs:108`, `size.rs:112`; exhaustive `match kind` at `emit.rs:149`.

Why judgment: two plan gaps need a decision, and the emitted constraints change on most corpus circuits.
- **Shared reference (decided here).** `current_mirror_3/4/1_to_2` declare `(0,k)` for every output, so the
  reference is in 2–3 pairs; today's emit gives each pair a hard `Symmetry` about the stage axis, which puts the
  reference in several symmetry entries (T6 failure in `no_emitted_conflicts` on `mirror6`, simulated: its top block
  becomes `current_mirror_4`). Rule: within a stage, a pair whose device is already in an earlier pair's `Symmetry`
  gets no `Symmetry` and no CentroidGroup side; it keeps `MatchingPair`, `ThermalGradient`, `Proximity`, `DtiBand`.
  `// ponytail: pairwise emission; MAT-04's MatchedSet replaces it for multi-output mirrors.`
- **Missing declaration.** `complementary_diff_pair` (priority 23) wins `rail2rail` over both `diff_pair_with_tail`s
  (simulated), and with no entry it yields no DiffPair leaf, so EXT-06's "rail2rail yields both DiffPair leaves"
  cannot hold. Add `complementary_diff_pair: pairs (0,1,DiffPair),(2,3,DiffPair)` (slots 0,1 = one polarity's pair,
  2,3 = the complement's, `catalog.rs` `COMPLEMENTARY_DIFF_PAIR`).

Edits:
1. `block.rs`: `BlockKind::CascodePair` (doc: "symmetric pair of cascodes: matched for symmetry, not a gate
   reference"); `Block` gains `pub template: &'static str` (`"glue"` for glue) and `pub selfs: Vec<DeviceId>`. Update
   both literals (`lib.rs:108`, `size.rs:112`, template `"glue"`/`"test"`).
2. `pattern.rs`: `PatternMatch` gains `pub pattern: &'static Pattern` (set in `Search::run`).
3. `catalog.rs`: `pub struct Roles { pairs: &'static [(u8, u8, BlockKind)], selfs: &'static [u8], prox: &'static [(u8, u8)] }`
   (`#[derive(Clone, Copy, Debug, Default)]`), `pub const ROLES: &[(&str, Roles)]` with exactly the plan's list
   (plan-01 EXT-05 step 2) plus `complementary_diff_pair` above and `cmos_inverter: prox (0,1)`; and
   ```rust
   /// `ROLES` entry; else for a 2-slot pattern today's `BlockKind::from_template` mapping
   /// (DiffPair/CurrentMirror/Load → pairs (0,1,k); Stack → prox (0,1)); else none.
   pub fn roles_of(p: &Pattern) -> Roles
   ```
   For the 2-slot fallback return `&'static` slices via `match kind { DiffPair => &[(0,1,DiffPair)], … }`. Keep
   `from_template` (now only the 2-slot fallback).
4. `Block::from_match(m)`: `kind` = for 2 devices `from_template`, else `Group`; `template = m.template`;
   `sub_blocks` = one 2-device `Block` per `roles.pairs` (its kind) then per `roles.prox` (`Stack`), devices in slot
   order, `template = m.template`, empty `selfs`; but a 2-device match gets no children (it is the leaf);
   `selfs = roles.selfs` mapped to devices.
5. `lib.rs`: delete the re-search (:98-103); `let mut b = Block::from_match(&m)` only.
6. `emit.rs`: `match kind` arm `DiffPair | CurrentMirror | Load | CascodePair => {}`. Shared-reference rule above:
   keep `let mut in_sym: Vec<DeviceId>`; for a pair with `in_sym.contains(&a) || in_sym.contains(&b)` skip the
   `syms.push` and `a_side/b_side` pushes. Replace :203-211 with
   `for &d in &stage.selfs { syms.push(Symmetry { a: td(d), b: td(d), axis }); …tail Proximity as today… }`
   under the same `if let Some(dp)` guard. Update the module doc table (add CascodePair row; tail sentence → "each
   declared self (tail, shared bias)").

Tests:
- `catalog::tests::roles_are_well_formed`: every `ROLES` name is in `PATTERNS`; names unique; every slot index
  `< slots.len()`; no slot in two `pairs` **except** a pattern whose pairs all share slot 0 (`current_mirror_*`).
- `tests/corpus.rs::corpus_expectations`: `latch` leaves exactly `{(DiffPair,[MN1,MN2]),(DiffPair,[MP1,MP2]),(Stack,[MN1,MP1]),(Stack,[MN2,MP2])}`;
  `gilbert` contains `(DiffPair,[M1,M2])`, `(DiffPair,[M3,M6])`, `(DiffPair,[M4,M5])` (M0 leaves the stage);
  `folded` contains `(DiffPair,[M1,M2])`, `(CascodePair,[M5,M6])`, `(Load,[M3,M4])`. Other rows: update as measured
  under the corpus rule.
- `tests.rs::a_differential_stage_is_symmetric_about_one_axis` unchanged and green (3 Symmetry).
- New `tests.rs::telescopic_slot7_is_not_self_symmetric`: an 8-FET `telescopic_ota_full` netlist (nets per the
  pattern's links, slot 7 a PMOS with S on slot 4's S) → slot 7's device is in no `Symmetry` with `a == b`.
- `tests/corpus.rs::no_emitted_conflicts` stays green (mirror6 is the case the shared-reference rule exists for).

Command: `cargo test -p annotator && cargo test -p annotator --test corpus && cargo test -p library`.
Acceptance: AA-03 (latch pairs constrained), AA-23 (only declared selfs on the axis); T6 = 0 on the corpus.

---

## EXT-07 Sensitivity marked from matching only — mechanical (after EXT-05)

Current: `block.rs:60-62` `is_sensitive` includes `Stack`; read by `lib.rs:126-132` (marks `sensitive` devices →
`classify` `gate_of_sensitive`, `classify.rs:79-81` before EXT-03) and `emit::isolation` victims (`emit.rs:290-295`).

Edit: `is_sensitive` = `matches!(self, DiffPair | CurrentMirror | Load)`; doc "Devices whose gate nets are Sensitive and
which are isolation victims; Stack and CascodePair are adjacent/symmetric, not a gate reference."

Tests (`tests.rs::a_cascode_stack_is_adjacent_not_matched`, extend):
- add a clocked switch `fet("XS", Nmos, clk, sw, VSS-id, VSS-id, 1_000, 150)` (push nets `clk`, `sw`) → no
  `Isolation` batch touches device 0 or 1 in `p.placement.cost` (today both are victims);
- variant: add `fet("MD", Nmos, g=0 (vin), d=3 (vcas), s=2, b=2, …)` so `vcas` also touches a channel → class of
  `vcas` is `Signal`;
- base netlist: `vcas` (gates only) still `Sensitive`; a `chain4` netlist (as `a_shared_gate_chain_is_a_series_stack…`)
  keeps `g` `Sensitive` (EXT-18 changes both).

Command: `cargo test -p annotator`. Acceptance: T2 on `chain4`, `folded` (unit tests above).

---

## EXT-09 Differential and crosstalk from recognised pairs; `Differential` to the budget arm — mechanical

Current code:
- `extract.rs:54-55` pushes `Differential::extract` into `r.hard`; `:57-62` `CrosstalkExclusion::extract`. Both scan
  all device pairs with `is_diff_pair` (`kernel/analog/src/placement/matching_pair.rs:83-96`, positional G/D/S, its
  own `is_supply` :100-107 that allocates per call and does not know `0`).
- Plan corrections: `gr::symmetric_nets` (`backend/gr/src/lib.rs:273-280`) **already** chains hard+budget: no edit.
  The dr trim loop is at `backend/dr/src/lib.rs:876` (plan said 811); dr's repair loop (:1499-1513) already reads the
  budget arm. gr's test is at `gr/src/lib.rs:1387`.

Edits:
1. `extract::routing` gains `pairs: &[(DeviceId, DeviceId)]`; `annotate` passes the DiffPair leaves with 2 devices
   (`block::leaves(&blocks)`, `kind == DiffPair`). Make `pattern::pin_net` `pub(crate)` and use it for `D`/`G` by name.
   For each `(a, b)` with `D(a), D(b)` both `Some` and different: push
   `Differential { pos: D(a), neg: D(b), max_len_delta_pct10: 50, same_layer_required: true, stack: process.stack }`
   into one `Vec` pushed to **`r.budget`** (before the crosstalk batch); and for `g in [G(a), G(b)]`,
   `d in [D(a), D(b)]`, `g != d`: `CrosstalkExclusion { a: g, b: d, min_spacing_nm: route_space_nm * max multiple, margin_pct: 25 }`
   (today's semantics). Delete `uf`/`UnionFind`. Update the module doc table (Differential → budget, both "from the
   DiffPair leaves").
2. `kernel/analog`: delete `Differential::extract` (`differential.rs:116-135` with doc) and its imports :5-6;
   `CrosstalkExclusion::extract` (`crosstalk.rs:96-120` with doc) and imports :6-7; in `matching_pair.rs` delete
   `G`, `D`, `S` (:9-11), `is_fet` (:13), `is_diff_pair`, `is_supply` (:82-107) and imports left unused.
3. `backend/dr/src/lib.rs:876`: `reqs.hard.iter().chain(&reqs.budget).filter(|b| b.repair_kind() == RepairKind::Mirror)`.
4. `gr/src/lib.rs:1387` test: `reqs.hard.push(..Differential..)` → `reqs.budget.push(..)`; assertion unchanged
   (`[3, 4, 5, 1, 0, 2]`: `tier` tests `sym` first). `dr/src/lib.rs:2834` same move (production emits budget only).

Tests:
- `tests/corpus.rs::differential_comes_from_recognized_pairs` (corpus parser lives there): `dac4` with
  `src("dac4").replace("VSS", "0")` → 0 `Differential` rules in any routing arm (today 10); `ota5t` → exactly 1, in
  `routing.budget`, 0 in `routing.hard`.
- `tests.rs::budget_rules_land_in_exactly_one_partition`: add `"Differential"` to the kind list (must be `["budget"]`).
- `canon.net_pairs` rows in `corpus_expectations` may change (now leaf-derived): update under the corpus rule.

Command: `cargo test -p annotator -p analog -p gr -p dr && cargo test -p annotator --test corpus`.
Acceptance: no hard `Differential` anywhere (M1 EXT exit "0 Differential in hard").

---

## EXT-06 Overlapping recognition, canonical order, a faster matcher — judgment

Current code: `pattern::recognize` (`pattern.rs:171-208`) greedily keeps a disjoint set, ties by sorted device ids
(AA-07); `Search::run` (:139-163) scans every device for every slot and dedupes with `Vec::contains` (O(M²));
`pin_net` (:77-80) is a linear name search. `Drawn` (EXT-11) now carries model and bulk, so labels can use the model.

Why judgment: T8 and T4 are uncertain until measured. Estimate for T8 with only the plan's step 1: 27 patterns join
slot 1 to slot 0 only by `0S = 1S` (simulated list: every diff-pair composite, `wilson_mirror_4`, `diode_load_pair`,
…); in the T8 fixture slot 0 is often a rail-sourced device (5,000 PMOS on VDD), so slot 1 iterates the 5,000-device
rail: ~4·10⁷ checks per pattern, ~10⁹ total, far over 2.0 s. Step 1b fixes that without changing any pattern's meaning.
For T4, a device-id tie-break is not permutation-invariant on automorphic ties (`mirror6`: 3 identical 2 µm outputs
give tied `current_mirror_4` instances); instance **names** are, and are what `canon` compares.

Edits (`pattern.rs`):
1. `pins: Vec<[Option<NetId>; 8]>` built once (index `fn pin_index(&str) -> Option<usize>`: G,D,S,B,C,E,P,N = 0..7);
   `pin_net`, `is_diode`, `slot_ok`, `links_ok` read it.
1b. Static slot order per pattern (`fn slot_order(p: &Pattern) -> Vec<usize>`): start `[0]`; repeatedly take the
   smallest unplaced slot whose `SlotKind`/`SizeMatch` reference is placed and that has a `Same`/`SameSignal` link to
   a placed slot whose pin pair is not both in {S, B}; else one with any such link; else the smallest whose
   references are placed. `Search` assigns in that order into `assigned: Vec<u32>` indexed by slot (`u32::MAX` =
   empty); `links_ok` checks links with both ends assigned; the emitted `instances` stay in slot order.
2. Candidates for the next slot `k`: over links joining `k`'s pin `pk` to an assigned slot's pin, `rel` Same or
   SameSignal, take the net `n` with the fewest `hg.net_devices[n]`; a `SameSignal` link on a non-Signal net, or a
   missing pin, gives no candidates. Iterate `hg.net_devices[n]` skipping a repeat of the previous id, keeping
   devices with `pins[c][pk] == Some(n)`. No such link: scan all devices.
3. `seen: HashSet<Vec<u32>>` per pattern.
4. New API:
   ```rust
   /// Every match of every allowed pattern; one per (template, device set).
   #[must_use] pub fn recognize_all(hg: &BipartiteHypergraph, drawn: &[Drawn], roles: &[NetRole], cfg: &AnnotationConfig) -> Vec<PatternMatch>;
   /// Permutation-invariant labels: 3 rounds of WL refinement on the device–net bipartite graph.
   #[must_use] pub fn canonical_labels(hg: &BipartiteHypergraph, drawn: &[Drawn], models: &[String], roles: &[NetRole]) -> Vec<u64>;
   /// Disjoint subset for `Problem::blocks` (interim, until EXT-13), in selection order.
   #[must_use] pub fn select_disjoint(all: &[PatternMatch], canon: &[u64], names: &[&str]) -> Vec<PatternMatch>;
   ```
   Labels: `std::collections::hash_map::DefaultHasher::new()` (fixed keys, deterministic). Round 0 device label =
   hash(kind as u8, `w_finger_nm`, `l_nm`, `fingers`, `models[drawn.model]`, sorted Vec of (pin name, role as u8,
   `net_devices[n].len()`)). Each round: net label = hash(role, sorted labels of its devices); device label =
   hash(own label, sorted Vec of (pin name, net label)). Do not hash `NetId`/`DeviceId`/model index (order-dependent).
   `select_disjoint` sorts by `(Reverse(priority), roles_of(pattern).pairs.is_empty(), sorted canon labels, sorted
   names)`, then keeps greedily. `// ponytail: names break exact label ties (automorphic instances); ids would not be
   permutation-invariant.`
5. Delete `recognize`. `annotate`: `recognize_all` → `canonical_labels` (pass the `models` table `size::drawn`
   fills) → `select_disjoint` (names from `netlist.devices`) → blocks in that order. Do **not** store `all` in
   `Problem` (no reader until EXT-13; EXT-13 adds the field).

Tests:
- `pattern::tests::recognize_all_is_a_superset_of_select_disjoint` (OTA from `crate::tests::fet`): every selected
  match is in `recognize_all` (same template, same instances); selected sets pairwise disjoint;
  `recognize_all(..).len() > select_disjoint(..).len()`.
- `tests/corpus.rs::permutation_invariance`: delete `#[ignore]`; green for every circuit, seeds 1..=20.
- `corpus_expectations` `rail2rail` row: leaves include `(DiffPair,[MN1,MN2])` and `(DiffPair,[MP1,MP2])`.
- `tests/scale.rs::twelve_thousand_devices`: delete `#[ignore]` and the "fails today" module doc; green in release.

Commands: `cargo test -p annotator --test corpus`; `cargo test --release -p annotator --test scale -- --nocapture`
(record the printed wall time in the commit message). Acceptance: T4 on the interim canonical form; T8 ≤ 2.0 s.
If T8 still misses after 1–3 and 1b, report the measured time and the profile; do not relax the bound.

---

## EXT-10 Stable IDs, provenance, coverage, relevant `missing`, one policy table — judgment

Current code: `RuleBatch<On>` (`kernel/analog/src/rule.rs:165-252`) has **20** methods (plan said 18); prices keyed by
`(kind, ordinal)` (`requirements.rs:16-18`, `gp/src/lib.rs:151-158`); `missing(p)` unconditional (`lib.rs:54-72`);
literals: `PROXIMITY_NM` (`emit.rs:45`), `margin_pct`/`spacing_multiple` (`extract.rs:123-141`), shield 80 and
`2 * route_space` (`extract.rs:110-112`), antenna margin 20 (`extract.rs:48`), diff 50 (EXT-09), IR shares
(`ir.rs:18-27`, called by `frontend/library/src/lib.rs:392`). `MetadataReport` already has a `coverage` field
(`verify::Coverage`, signoff coverage), so recognition coverage gets other names.

Why judgment: the origin attribution rule and the id order are design; the plan's `missing` relevance test for
budgets is circular (see step 3).

Edits:
1. New `kernel/analog/src/intent.rs` (`pub mod intent;`, no root re-export):
   ```rust
   #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)] pub struct ConstraintId(pub u32);
   /// Where a batch came from (EXT-12 widens this).
   #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Origin { Pattern { template: &'static str }, NetClass }
   #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct BatchMeta { pub id: ConstraintId, pub origin: Origin }
   ```
   `rule.rs`: `fn meta(&self) -> Option<&crate::intent::BatchMeta> { None }` on `RuleBatch`, and
   `pub struct Tagged<On> { pub meta: BatchMeta, pub inner: Box<dyn RuleBatch<On>> }` with
   `impl<On> RuleBatch<On> for Tagged<On>` delegating all 20 methods to `inner` and `meta() -> Some(&self.meta)`.
   (A boxed `inner` lets `annotate` tag after emission without touching each push site.)
2. `annotate`, after all emission: `let mut next = 0u32;` wrap every batch of `placement.hard`, `.budget`, `.cost`,
   then `routing.hard`, `.budget`, `.cost`, in that order, as `Tagged { meta: BatchMeta { id: ConstraintId(next), origin }, inner }`
   (`next += 1`; take each arm with `std::mem::take`). Origin: placement batch whose kind does not end with
   `::Isolation` and whose first `touched` id is a device in a non-glue block → `Pattern { template: blocks[bi].template }`;
   everything else → `NetClass`. Ids are permutation-invariant because emission order is (EXT-06 block order).
3. `Needs { matched, gate_nets, budgeted_nets }`; `missing(p, needs)`: the two `MatchingPair` entries only if
   `matched` (any leaf of kind DiffPair, CurrentMirror, Load or CascodePair); `Antenna` only if `gate_nets` (any
   Nmos/Pmos); `ParasiticBudget`/`CouplingBudget` only if `budgeted_nets`. Plan correction: "some net got a budget" is
   always false when the deck lacks `gate_af_per_um2` (that is why budgets are `None`), so it would hide exactly the
   entry it should report; use `budgeted_nets = gates.iter().any(|&g| g > 0.0)` (classify's own precondition,
   `classify.rs` `smallest`).
4. `Problem.coverage: Vec<(DeviceId, Coverage)>`, `pub enum Coverage { Constrained, Grouped(&'static str), Unconstrained(&'static str) }`
   (`#[derive(Clone, Copy, Debug, PartialEq, Eq)]`, in `lib.rs`). Per device: touched by any placement batch →
   `Constrained`; else in a non-glue block → `Grouped(template)`; else `cfg.do_not_identify` → `Unconstrained("do_not_identify")`;
   else `size::unknown_size` → `Unconstrained("unknown size")`; else `Unconstrained("no pattern")`. Replace the
   `ponytail` note in `size.rs:122-123` test (`unknown_size_never_matches`) with an assertion on `Unconstrained("unknown size")`.
5. `backend/annotator/src/policy.rs`: `#[derive(Clone, Debug)] pub struct Policy` + `impl Default` with today's
   values, one doc line each saying "Philis policy" (or its source): `proximity_nm: i32 = 5_000`,
   `spacing_multiple: [i32; 4] = [8, 7, 3, 1]` (Sensitive, Clock, Signal, other), `margin_pct: [u8; 4] = [35, 30, 25, 20]`
   (Sensitive, Clock, Supply|Ground, other), `shield_coverage_pct: i32 = 80`, `shield_gap_spaces: i32 = 2`,
   `antenna_margin_pct: i32 = 20`, `diff_pct10: i32 = 50`, `ir_headroom_share: f64 = 0.1`, `ir_rail_share: f64 = 0.01`,
   `ir_high_current_share: f64 = 0.1`. `AnnotationConfig.policy: Policy`. `emit::placement(.., policy: &Policy)`,
   `extract::routing(.., policy: &Policy)`, `ir::budgets(.., policy: &Policy)` (library passes `&ann.policy`).
   Delete the replaced consts. Skipped (YAGNI): `pn_max_degree`, `beta_target`, `max_eta` — EXT-13/21 add them with
   their readers.
6. Library: `MetadataReport` gains `pub recognition: Vec<(&'static str, usize)>` (template, count of non-glue blocks)
   and `pub unconstrained: Vec<(String, &'static str)>` (device name, reason); `solve` fills both after
   `metadata::build` (`lib.rs:548`, beside `binding`) from `flow.problem.blocks`/`.coverage` and `netlist`. `Display`
   prints, after the `missing` lines, `  RECOGNITION: five_transistor_ota ×1, …` and `  UNCONSTRAINED: R1 (no pattern), …`
   when non-empty.

Tests:
- `tests/corpus.rs::ids_survive_permutation`: `ota5t`; the `placement.cost` batch whose kind ends `::MatchingPair`
  and whose touched device names are `{XM1, XM2}` has the same `meta().unwrap().id` in the base and `permute(&nl, s)`
  for `s in 1..=5`.
- `tests/corpus.rs::coverage_is_total` (write the body, drop `#[ignore]`): for every circuit in `all()`,
  `coverage.len() == devices.len()`, ids are exactly `0..n`, every `Unconstrained` reason is one of the three; `ota5t`
  all `Constrained`; `rdiv` all `Unconstrained("no pattern")`.
- `tests.rs::missing_is_relevant`: the two-resistor netlist of `glue_only_netlist_emits_no_placement` → no entry with
  `.0` in `{"MatchingPair", "Antenna"}`; `ota()` with default config → contains `("MatchingPair", "deck svt_uv_per_um")`
  and `("ParasiticBudget", "deck gate_cap_af_um2")`.
- `tests.rs::every_batch_is_tagged`: `ota()` plus the clocked `XS` of `clocked_devices_are_kept_away_from_matched_ones`
  → every batch of all six arms has `meta().is_some()`, ids dense `0..total`.
- `metadata.rs` test `recognition_is_printed`: report with one `recognition` and one `unconstrained` row → output
  contains both lines.

Commands: `cargo test -p analog -p annotator -p library -p gp`; `cargo test -p annotator --test corpus`.
Acceptance: T7; every annotator batch tagged.

---

## GAP-10 Prices keyed by stable constraint ID (FLOW-03 step 5) — mechanical (after EXT-10)

Current code: `backend/gp/src/lib.rs:27-28` `priced: BTreeMap<(&'static str, u32), Price>`; `fn keys` :151-158;
`bind` :76-85; `settle` :93-127 pushes `key.0` into `saturated`; contract comments at `gp/src/lib.rs:24-26` and
`kernel/analog/src/requirements.rs:16-18`.

Edits (`backend/gp/src/lib.rs`):
1. `#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)] enum PriceKey { Id(analog::intent::ConstraintId), Ord(&'static str, u32) }`;
   `priced: BTreeMap<PriceKey, Price>`.
2. `keys(reqs) -> Vec<PriceKey>`: `Some(m) = reqs.budget[bi].meta()` → `Id(m.id)`; else `Ord(kind, n)` with `n` =
   earlier **untagged** budget batches of the same kind.
3. `settle`: saturation records `reqs.budget[bi].kind()` (contains-check on it), not `key.0`.
4. Docs: `Prices` — "keyed by `BatchMeta::id` when the batch carries one, else by (kind, ordinal among untagged
   same-kind batches)"; `requirements.rs:16-18` — "order matters only for untagged batches".

Tests (gp `mod tests`, using `bench()`'s layout and the `Budget` rule; `Tagged { meta: BatchMeta { id, origin: Origin::NetClass }, inner: Box::new(..) }`):
- `reordered_tagged_batches_keep_their_prices`: budget `[A = Tagged(id 1, vec![Budget]), B = Tagged(id 2, vec![Budget, Budget])]`,
  `l.x[0] = 1_000`, one `settle`; `wa = weight_of(0)`, `wb = weight_of(1)`, `assert!(wa > 0.0 && wa != wb)`; `bind`
  `[B, A]` → `weight_of(0) == wb`, `weight_of(1) == wa`.
- `untagged_batches_keep_todays_behaviour`: same with untagged `vec![Budget]`, `vec![Budget, Budget]` → after the
  swap `weight_of(0) == wa` (position keys, today's contract). `price_survives_an_appended_batch` stays green.

Commands: `cargo test -p gp -p dp`; bench before and after this commit:
`cargo run --release -p benchmark --bin bench local` — every row identical (no batch reorders, so ids map one-to-one
onto today's keys).
Acceptance: EXT-10's `ids_survive_permutation` plus these two tests; bench rows unchanged.
