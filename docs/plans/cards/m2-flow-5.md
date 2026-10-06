# M2+ flow, segment 5 (M6): FLOW-11

Branch `m2-flow` after `git merge m2` (conflict in frontend/library/src/lib.rs `mod start_tests`: FLOW-08/10 tests
vs PLC-24's `utilization_floor_is_reachable_for_tiny_cells`; both kept). dag: FLOW-11 hard-depends on FLOW-09 only
(done, `223d711`/`bd66670`); FLOW-02 and FLOW-07 done; RTE (metal range) and EXT (SameTemplate symmetry) optional.
PLC-06 (not done) is not touched.

Shell: `flake.nix`, no `shell.nix`, so "the nix-shell" is `nix develop`. Every command below runs as
`cd /home/omare/Documents/Projects/Rust/philis-m2/flow && nix develop -c env
PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk /home/omare/Documents/Projects/Rust/Philis/tools/qcargo <args>`
(written `qcargo <args>`).

| Item | Class | Size |
|---|---|---|
| FLOW-11 | do | L: new `hier.rs`, `run` split into `run` + `solve`, block cells through `cellgen`/`CellSpace`, LVS block cards, CLI `--hierarchy` |

---

## FLOW-11 Hierarchical bottom-up flow — class: do

### Code facts (plan corrections)
- No `solve` function exists (plan step 3.4 "the ordinary `solve` path"). The flow is one `pub fn run(spice, pdk,
  injected, cfg)` (frontend/library/src/lib.rs:468) that parses (:470), checks the interface (:473), checks injected
  macros (:481), sets up annotation (:485–504), solves the bias (:508, `fn bias` :2312), annotates, builds topologies
  (:527–528) and searches/finishes (:529–589). It must be split so a child can be solved from a `Netlist` + `Bias`.
- `Netlist` (kernel/core/src/netlist.rs:113): `insts: Vec<SubcktInst>` (parents first), `device_inst:
  Vec<Option<u32>>` (innermost instance). `SubcktInst` (:129) has `path`, `subckt`, `parent`, `ports` (actual
  NetIds in formal order) — **no formal port names** (plan's "pins named by formal port" / "formal port → actual"
  cannot use names). Internal nets are `path/<net>`, devices `path/<name>`.
- `annotator::hier::devices(nl, i)` (backend/annotator/src/hier.rs:17) already lists instance `i`'s devices
  (own + nested): reuse it, do not re-walk `device_inst`. EXT-27 (`hier::same_template`, :64) already turns identical
  instances into matched couples on member devices; once retargeted through `cell_of` they become cell-level rules
  between block cells (the H15-07 "optional EXT" dependency is satisfied by what exists).
- `Problem::blocks` claim is stale: the injected-macro mechanism (`Macros` keyed by device name, cellgen.rs:180–189,
  `CellSpace.fixed` lib.rs:2449) is one device per macro (`check_injected` lib.rs:2162 refuses anything else), so a
  multi-device block needs its own path in `cellgen::enumerate_folded` (cellgen.rs:63).
- Cell emission: `enumerate_folded` phase 1 merges unitizations (cellgen.rs:79–172), phase 2 emits one cell per
  device in order of first member (:175–212); callers: `enumerate` (cellgen.rs:55) and `CellSpace::new`
  (lib.rs:2417). `CellSpace::new` callers: `topology` (lib.rs:816) and tests lib.rs:3355, 3493, 3742.
  `topology` callers: `run` (:527–528), tests lib.rs:2914, 3429.
- LVS reference: `labels_and_reference` (lib.rs:2790) = `cellgen::reference(schematic, fold, &replaced)` + dummy
  cards + drawn cards. Called from `signoff_shapes` (lib.rs:2775; callers: epoch signoff lib.rs:1761,
  `elaborate.rs:90`) and `signoff_inputs` (lib.rs:2641–2652; used by `drawn_signoff`, `export_gds`,
  `reference_spice`).
- `Shape` carries no net (kernel/core/src/geom.rs:106): block shapes need no net remap; only `Pin.net` does.
- Metal range (plan step 7 / OQ-6) already exists: `elaborate::routing_stack(pdk, top: Option<usize>)`
  (elaborate.rs:230) returns `Result`; `topology` calls it with `None` and panics on `Err` (lib.rs:861, comment
  "FLOW-11 propagates the `Err`", the RTE card m2-routing-1.md:224–228 hand-off).
- Fill: `finish` appends the fill macro last when `metadata.post_fill` (lib.rs:1168–1170); `Solution::geometry`
  (lib.rs:2559) includes it. The block macro must drop it (plan "no fill").
- `Config` is not `Clone` (lib.rs:80); `OpConfig` and `AnnotationConfig` are.
- `LexKey` is a private type alias (lib.rs:2022): the test cannot "record both `LexKey`s"; it records the public
  projection in `RunStats`.
- CLI: `--hierarchy` other than `flat` exits 2 "needs FLOW-11" (frontend/cli/src/main.rs:96–101, doc :21–22).
- Fixture `two_ota.spice` does not exist; `ota.spice` is 5 devices (`.subckt ota vinp vinm vout1 vout2 VDD VSS vbias
  vbn`). Full-flow clean-signoff fixtures run as `#[ignore]` under `--release -- --ignored`
  (benchmarks/tests/signoff_fixtures.rs:35–42); follow that convention.

### Edits

1. `frontend/library/src/lib.rs` — config, stats, errors:
   ```rust
   /// Bottom-up hierarchy (FLOW-11). Opt-in: bottom-up is not globally optimal (Balasa–Graeb L3068–3075).
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub enum Hierarchy { #[default] Flat, BottomUp { min_devices: usize }, Auto }
   pub struct Config { …, pub hierarchy: Hierarchy }          // default Flat
   pub struct RunStats { …, pub block_solves: u32 }           // definitions solved as blocks (0 when flat)
   pub struct Solution { …, pub blocks: Vec<(String, metadata::MetadataReport)>,   // per solved definition
                            pub(crate) block_ref: hier::BlockRef }
   pub enum FlowError { …, /// The deck's routing stack is unusable (`elaborate::routing_stack`).
                            Deck(String) }
   ```
   `Auto` = `BottomUp { min_devices: 2 }` when the flattened top has > 100 **devices** (plan says cells; cells exist
   only after cellgen — policy bound balasa L1604–1605), else `Flat`. Add a `fn hierarchy(cfg, nl) -> Option<usize>`
   (resolved `min_devices`, `None` = flat).
2. Split `run` (lib.rs:468):
   - `run` keeps steps parse → `deck_models` → interface check → `check_injected` → `let bias = bias(&netlist, cfg)`;
     then `let (blocks, placed) = match hierarchy(cfg, &netlist) { Some(k) => { let b = hier::solve_blocks(&netlist,
     pdk, cfg, &bias, k)?; let p = hier::instantiate(&netlist, &b); (b, p) } None => Default };` then
     `solve(&netlist, pdk, injected, cfg, bias, &placed)` and fill `sol.blocks`, `sol.stats.block_solves`.
   - New `fn solve(netlist: &pnr_core::Netlist, pdk: &Pdk, injected: &Macros, cfg: &Config, bias: Bias, blocks:
     &hier::Placed) -> Result<Solution, FlowError>` = today's lines :483–589 unchanged except `topology(…, blocks,
     …)?` and `sol.block_ref = blocks.refs.clone()`. Bias moves before annotation setup (it does not read `ann`).
3. `topology` (lib.rs:787): add `blocks: &hier::Placed`, return `Result<Topology<'a>, FlowError>`; lib.rs:861 becomes
   `elaborate::routing_stack(pdk, None).map_err(FlowError::Deck)?` (delete the ponytail comment).
   `CellSpace::new` (lib.rs:2403) gains `blocks: &[(Vec<DeviceId>, Macro)]`, passed to `enumerate_folded`; guard-ring
   requests whose device is a block member are dropped before the dedup loop (:2437). Update test callers to `&[]` /
   `&Default::default()` and `.unwrap()`.
4. `cellgen::enumerate_folded` (cellgen.rs:63): add `blocks: &[(Vec<DeviceId>, Macro)]` (`enumerate` passes `&[]`).
   Phase 1: `continue` a unitization with any member in a block (same as the injected check :101). Phase 2: a
   device in block `b` emits `(b.0.clone(), vec![b.1.clone()])` at its lowest member, later members are no-ops
   (mirror the `unit_of`/`merged[ui].take()` pattern). `CellSpace.fixed` stays `false` for block cells (dp may
   mirror/rotate; one alternative so no reshape). Doc comment: block cells are drawn in the child, never here.
5. `labels_and_reference` (lib.rs:2790) gains `blocks: &hier::BlockRef`: `skip` = `replaced` ∪ `blocks.members`;
   after drawn/dummy cards, `reference.devices.extend(blocks.cards.iter().cloned())`. `signoff_shapes` gains the same
   param (`Flow` gets `block_ref: hier::BlockRef`, passed at lib.rs:1761; `elaborate.rs:90` passes
   `&Default::default()`); `signoff_inputs` passes `&sol.block_ref`.
6. New `frontend/library/src/hier.rs` (`mod hier;` in lib.rs):
   ```rust
   /// One sub-circuit definition solved once (FLOW-11).
   pub(crate) struct Block {
       pub subckt: String,
       /// Child geometry (cells, rings, routes; no fill) translated so its bbox corner is the origin; one pin
       /// `p{k}` per formal port k; `units`, `dummies`, `drawn` empty; child `keepouts` translated.
       pub mac: Macro,
       /// `signoff_inputs(&child, pdk).2.devices`: child net names (= the first instance's parent names).
       pub ref_cards: Vec<verify::RefDeviceIn>,
       /// First instance's path and actual port names: the name map's source side.
       pub i0_path: String,
       pub i0_ports: Vec<String>,
       pub metadata: metadata::MetadataReport,
       pub stats: RunStats,
   }
   /// The LVS side of placed blocks: cards renamed per instance, and the schematic devices they replace.
   #[derive(Clone, Default)]
   pub(crate) struct BlockRef { pub cards: Vec<verify::RefDeviceIn>, pub members: Vec<DeviceId> }
   /// What a parent solve consumes: one cell per outermost block instance, pins bound to its actual nets.
   #[derive(Default)]
   pub(crate) struct Placed { pub cells: Vec<(Vec<DeviceId>, Macro)>, pub refs: BlockRef }

   /// Definitions to solve, post-order (children first). Eligible: not the top, ≥ `min_devices` devices
   /// (`annotator::hier::devices`), and every instance has pairwise-distinct port nets and devices touching only
   /// its ports or `path/` nets (a shorted port or a global would make one layout wrong for another instance).
   pub(crate) fn eligible(nl: &Netlist, min_devices: usize) -> Vec<String>;
   /// Instance `i`'s devices as their own netlist, keeping parent names; ports = `insts[i].ports`; nested insts
   /// re-parented (i's children → `None`). Returns (netlist, parent device per child device, parent net per child net).
   pub(crate) fn local(nl: &Netlist, i: u32) -> (Netlist, Vec<DeviceId>, Vec<NetId>);
   /// Outermost instances (no ancestor instance also a block) of every definition in `blocks`.
   pub(crate) fn instantiate(nl: &Netlist, blocks: &[Block]) -> Placed;
   /// Instance `j`'s name for child net `name`: port k → j's actual k; `i0_path/x` → `j.path/x`; else `j.path/name`
   /// (synthetic `~c.o.k` nets of drawn cards stay unique per instance).
   fn rename(name: &str, b: &Block, j_path: &str, j_ports: &[String]) -> String;
   pub(crate) fn solve_blocks(nl: &Netlist, pdk: &Pdk, cfg: &Config, bias: &Bias, min_devices: usize)
       -> Result<Vec<Block>, FlowError>;
   ```
   `solve_blocks` per eligible `S` in order: `i0` = first inst with `subckt == S`; `(n_s, devs, nets) = local(nl,
   i0)`; `placed = instantiate(&n_s, &blocks_so_far)` (nested blocks); child bias = `bias.restrict(&devs, &nets)`
   (new `impl Bias`: `power`, `currents`, `gm_us` by device; `net_headroom_mv` by `nets`; `summary`/`op` `None`);
   child config = `child_config(cfg)` (new fn: `Config { seed, feedback_iters, outer_iters, starts,
   min_utilization, heat_source_uw, size_convention, gp_mode, cold_every, warm, max_wall` copied, `annotation` and
   `op` cloned, `performance`/`interface`/`constraints`/`esd`/`top` `None`, `hierarchy: Flat`); `child =
   crate::solve(&n_s, pdk, &Macros::default(), &cc, child_bias, &placed)?`. Block macro: drop the fill macro
   (`child.macros.truncate(len - usize::from(child.metadata.post_fill))`), `shapes = child.geometry()`, bbox = union,
   translate shapes/keepouts by −corner; pin k = widest rect of `child.routes.wires[port k]` on the highest
   `pdk.routing_layers()` layer it uses, else the first placed cell pin on that net; `ref_cards` from
   `signoff_inputs(&child, pdk).2.devices` taken **before** truncating fill (fill adds no card; order does not
   matter, but the child's `Solution` is otherwise unchanged).
   `instantiate`: per block instance `j`: `members = annotator::hier::devices(nl, j)` ids; macro = `b.mac.clone()`
   with pin `p{k}`.net = `j.ports[k]`; cards = `b.ref_cards` with every terminal `rename`d; `refs.members` += members.
7. CLI (frontend/cli/src/main.rs:96–101): `flat` → `Flat`, `auto` → `Auto`, `bottom-up:N` → `BottomUp {
   min_devices: N }`, else exit 2 naming the forms; drop "needs FLOW-11" from the doc (:21–22) and USAGE.
8. Fixture `benchmarks/fixtures/two_ota.spice`: `ota.spice`'s `.subckt ota … .ends ota` plus
   ```
   .subckt two_ota inp1 inm1 inp2 inm2 o1 o2 o3 o4 VDD VSS vbias vbn
   X1 inp1 inm1 o1 o2 VDD VSS vbias vbn ota
   X2 inp2 inm2 o3 o4 VDD VSS vbias vbn ota
   .ends two_ota
   ```
9. Not done (report): per-level metal range (`routing_stack`'s `top` exists, but sky130's stack is 2 routed layers,
   AT-01, so a split has nothing to separate — add a `Config` field when a deck needs it); `elaborate.rs:159`'s panic
   (substrate path, not FLOW's `run`).

### Tests
- `frontend/library/src/hier.rs` `mod tests` (fast, parse only, `crate::parse`):
  - `eligible_is_post_order_and_skips_shorted_ports`: nested `.subckt inner` (2 devices) inside `.subckt outer`
    (inner ×1 + 1 device) inside top: `eligible(&nl, 2) == ["inner", "outer"]`; a second top instance of `inner`
    with two ports on one net makes `inner` absent; `eligible(&nl, 4)` lacks `inner`.
  - `rename_maps_ports_internals_and_synthetics`: block with `i0_path "X1"`, `i0_ports ["a","b"]`:
    `rename("a", …, "X2", ["c","d"]) == "c"`, `rename("X1/n", …) == "X2/n"`, `rename("~3.0.1", …) == "X2/~3.0.1"`.
  - `local_keeps_parent_names_and_ports`: for two_ota's `X1`: 5 devices, `ports` names == X1's actuals,
    an internal net named `X1/vtail`, and `nets[k]` maps back to the parent id.
- `frontend/library/tests/hier_flow.rs` (sky130 via `Pdk::builtin("sky130")`, fixture two_ota, `top:
  Some("two_ota")`), both `#[ignore = "full flow: --release -- --ignored"]` per signoff_fixtures.rs:35:
  - `bottom_up_solves_each_definition_once_and_signs_off_clean` (`hierarchy: BottomUp { min_devices: 5 }`):
    `sol.stats.block_solves == 1`; `sol.blocks.len() == 1 && sol.blocks[0].0 == "ota"`; exactly two cells
    hold 5 devices each (`sol.devices_of.iter().filter(|m| m.len() == 5).count() == 2`: the block cells); `library::signoff(&sol, &pdk)` has no
    `lvs/` and no `drc/` row (print the rows on failure).
  - `flat_and_bottom_up_both_run`: both runs `Ok`; `println!` per mode `(place_hard, route_hard, drc_hard,
    route_overuse, c_tier)` from `RunStats` and the wall time (`Instant`) — no assertion on which is better.
- Existing: every `CellSpace::new`/`topology` test caller still passes with `&[]`.
- Commands: `qcargo check -p library -p cli`; `qcargo test -p library --lib`; `qcargo test -p library --test
  hier_flow --release -- --ignored --nocapture`; `qcargo test -p library` (regression).

### Acceptance
T12 = the bottom-up test clean; flat vs bottom-up wall time on two_ota recorded in the report [measure]. If the
parent cannot land on block port pins (risk: RTE pin landing on layers 0–1, backend/gr/src/lib.rs:586) or routing
rules on block-internal nets raise rows in the parent (nets with no pin there), report the rows with evidence; do not
loosen the test.

### Baseline after the merge
`qcargo test -p library --lib` (debug): 180 pass, 4 fail (`start_tests::same_seed_same_gds_bytes`,
`start_tests::winner_signoff_matches_its_certificate`, `environment_tests::environment_is_in_the_placement_arm_once`,
`environment_tests::sidecar_loads_reach_the_annotator`), all on ota.spice with the known `debug_assert` at
backend/gp/src/lib.rs:89 ("a batch kind is registered in both `hard` and `budget`"; MAT-07 annotator emit.rs:145–148,
m2-placement-report.md:44–48). Pre-existing, not from this merge; FLOW-11 must not add to it.
