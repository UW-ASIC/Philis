# CELL — Device generators (`kernel/cells`): DRC-clean by construction, value-exact, layout-effect aware

Plan ID prefix: **CELL-NN**. Working tree of 2026-09-28 (HEAD `dad330c` plus uncommitted changes). Every code citation below was re-read in this session: all of `kernel/cells/src/*.rs` and `kernel/cells/tests/cell_selfcheck.rs`, `kernel/core/src/{macro.rs, units.rs 1-80, process.rs}`, `kernel/analog/src/cell.rs`, `backend/annotator/src/constraints.rs`, `frontend/library/src/cellgen.rs` 45-195, 443-615, 706-940, `frontend/library/src/lib.rs` 596-640, 1150-1215, 1283-1370, `backend/verify/src/{pdk.rs 1030-1270, reference.rs 55-135, netlist.rs}`, `pdks/sky130.json`, the pinned GPurify deck (`GP/pdks/sky130.deck`, GP = `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/`) lines 224-411, 540-670, `GP/crates/check/src/lvs/{compare.rs 1-60, reduce.rs 1-60, 255-312}`, and the installed sky130 model cards under `~/.volare/sky130A/` (cited as `VOL/…`). Reference citations use `<file>.txt L<a>–<b>` in the scratchpad `reftext/` directory, normally through the study-doc entry that recovered the number from the PDF. No build or test was run.

---

## 0. Scope & boundaries

**CELL owns** the device generators and what they export:

- MOS rows (`mosfet.rs`): finger order, S/D sharing, dummies, gate contacting, LOD moat and WPE halo, tap strips, Vt/oxide markers, row stacking.
- FinFET rows (`finfet.rs`), resistors (`resistor.rs`), capacitors (`cap_array.rs`, `capacitor.rs`), BJTs (`bjt.rs`), diodes (`diode.rs`), the inductor (`inductor.rs`).
- Post-placement rings and bridges (`post_cell.rs`): the drawing only. The ring taxonomy, which device gets which ring and the merge predicate belong to REL (REL-07, REL-08, REL-16) and EXT (EXT-12, EXT-23). Also the drawing surface (`builder.rs`) and the generator tests (`lib.rs::testkit`, `tests/cell_selfcheck.rs`).
- What a `Macro` tells downstream about itself: the extracted-device cards it drew and its keep-out regions (new `Macro` fields, CELL-01).
- Geometry consequences of MAT's pattern functions. MAT owns the pure pattern algorithms in `kernel/analog/src/matching/pattern.rs` (MAT-02 `grids`/`centro_assign`, MAT-03 `diffusion_cc_row`, MAT-12 `segment_row`, MAT-15 `nth_order_rows`, MAT-18 split-DAC functions). CELL calls them and never re-implements an order generator.
- The CELL side of the open cell issues in `docs/CRATES.md` (#1, #4, #6, #7). #5 is FLOW-05/REL-06 plus CELL-13's guard. #8, #9 and #10 are FLOW-04/FLOW-15 (see §1.3).
- The sidecar keys only the generators read: the resistor value-model keys (CELL-06), the `bjts` recipe table (CELL-09), and the `c_dw_nm` key and MOM roles on FLOW-06's `capacitors` table (CELL-08). Every new key is registered in FLOW-04's `sidecar::KEYS` once that registry exists.

**CELL consumes:**

| From | What | Used by |
|---|---|---|
| EXT | `Unitization.class: Option<MatchClass>`, `.series: Vec<u16>`, `.style: ArrayStyle` (EXT-12); resistor/BJT/diode/capacitor sets and their unit (EXT-15, EXT-19); net classes for the rail list | CELL-06, 12, 14, 18 |
| MAT | `pattern::{grids, centro_assign, Fill}` (MAT-02), `pattern::diffusion_cc_row` (MAT-03), `pattern::segment_row` (MAT-12), `pattern::nth_order_rows` (MAT-15), `dac::{attenuation_cap, nonunit_dims, split_dac_assign}` (MAT-18), `class::{mos_env, resistor_env}` (MAT-07) | CELL-07, 09, 10, 12, 14, 15, 21, 23 |
| REL | `GuardRingType` taxonomy and ring requests (REL-07/08), `tie_max_dist_nm` = 15 000 on sky130 (REL-06), `post_cell::may_share_well` (REL-16), `resistor::self_heating_min_width_nm` (REL-13) | CELL-13, 17, 22, 23 |
| FLOW | size convention `Device::mos_size()` (FLOW-01), `capacitors` recipe table (FLOW-06 step 4), `Pdk::model_markers` (FLOW-06 step 5), LU deck rules and `tie_max_dist_nm` (FLOW-05), key registry (FLOW-04), `cellgen::folds` | CELL-06, 08, 13, 16 |
| PERF | `lvs-coverage/unverified` rows for devices no recogniser sees (PERF-02); BJT recognisers (PERF-18) | CELL-01, 08, 09 |

**CELL provides:**

| To | Interface |
|---|---|
| REL | the `Ecgr` well-band drawing REL-07's ring policy needs (CELL-17) |
| MAT | drawn geometry of MAT's orders; unit moments in `Macro.units` (already emitted for MOS, resistor and capacitor units; BJT units via MAT-02 step 5; diode units via CELL-07) |
| PLC | macro extents on twice the cut lattice (CELL-03) |
| RTE | `Macro.keepouts` (gate, resistor body, capacitor plate) (CELL-01) |
| PERF | `Macro.drawn` and `cellgen::drawn_cards` for the LVS reference (CELL-01) |
| FLOW | the `bjts` recipe table and the resistor value-model keys, for FLOW-04's registry |

**Out of scope** (IDs recorded, no items):

- ESD primitives and pad-ring rings (H14-42..H14-60, H05-22..25). Philis is block-level, with no pad or pin plan (FLOW). The antenna diode uses the `Diode` generator unchanged.
- HV, DEMOS, LDMOS, power-FET metallization (H01-18, H13-01..18).
- JFET and NVM (H12-45..51).
- Lateral PNP, Schottky, buried-layer rings (H09-29/30/44, H05-58, H14-33). No current deck has NBL, deep-N+ or a Schottky.
- Notched-moat unequal-width merges (H12-31), directional-halo rotation (H12-26, PLC), foreign-well spacing for NMOS (H13-53 placement half, PLC).

---

## 1. Audit digest (current state, verified)

### 1.1 Contract

- `Cell::enumerate/draw` (lib.rs:45-52). Sizing comes only from the covering `Unitization` (builder.rs:233-267). Its fields: `devices, device_type, dev_nf, target_ratio, unit_w, unit_l, series_parallel, same_variant_required, dummy_required, route_matching_required` (analog/src/cell.rs:40-56). It has no grade.
- `Macro` = shapes, pins, bbox, units, dummies (core/src/macro.rs:9-20). It has no field for what else was drawn.
- `Builder::finish` rounds the bbox corner and extents to 2·grid, 10 nm on sky130 (builder.rs:119-135).
- `dim()` lets a sidecar key only raise a deck value (builder.rs:195-212).
- `cellgen::draw_all` turns an empty enumeration into an empty macro (cellgen.rs:815-835). `library::signoff` reports each empty cell as `cell/undrawable` (lib.rs:1293-1300), but looks the name up with the cell index as a device index (lib.rs:1295-1297), so after merges it names the wrong device. FLOW-15 step 1 fixes the naming (AF-31); CELL-02 makes the empty enumeration visible in tests.

### 1.2 Per generator

| Generator | Sound today (keep) | Defects and gaps (file:line) | IDs |
|---|---|---|---|
| MOS `mosfet.rs` | Full S/D cut columns (303-309, 458-472). On-diffusion bulk-tied dummies at pitch, listed as LVS cards (279-284, 511-534). Units carry φ and SA/SB (382-390). Split gates keep ABBA gates private (364-367; test 1006-1030). Swap-reverse mirror orders (752-779). 2-row cross-coupled quad (152-222). Two-ended gate only where it pays (58-66). | (a) Pitch `max(2·sd_w+L, m1_pitch)` = 730 nm on sky130 at L=150 (657-669; audit computation). The minimum is licon.1 170 + 2·licon.11 55 + L = 430 (sky130.deck:331, 341). The cause is isolated 340 nm pads per finger (344, 391-402) [corrected: the pads alone force only 340 + poly.2 210 = 550 nm (*derived*, sky130.deck:276); the rest of the 730 comes from `sd_and_pitch` sizing both sides of every gate at `sd_w` = m1_pitch − L = 290 (mosfet.rs:664-668)]. (b) Unequal `dev_nf` goes to `greedy_centroid` (784-788 → builder.rs:271-292), which puts two members across a drain. cellgen rejects every such variant (cellgen.rs:138-141, 276), so ratioed mirrors never merge. (c) One tap strip, above only (567-572). `tie_max_dist_nm` is never read. (d) One LOD/WPE tier: `lod_moat_ext_moderate` added beyond the outer dummy (293-295), `wpe_clearance_moderate` (629). The tiered sidecar arrays are dropped by the parser (pdk.rs:1259-1267). (e) Split gates give A and B unequal far-end overhang, and dummy skirts are on top only (364-380, 516, 537-552). (f) A poly strap under the li strap (428-436). (g) No Vt/oxide markers. (h) No figures. (i) `skirt_y … - 10` literal (537), stale "40 nm" (560), wrong r22 and r9 citations (44-47, 278, 291). | AC-04, 05, 07, 11, 12, 13, 21, 25, 28; H13-47, H13-53, H13-54, H12-43, H01-16 |
| Resistor `resistor.rs` | Recipe layers and slot contacts (61-78, 194-231). Current direction alternates per segment, so Σφ = 0 for even counts (109-110, 125). Salicide block covers every body and dummy (216-230). | (a) `dev_nf` never read: every member is drawn once as `segments` × `unit_l/segments` (81-82, 101, 304-310). The cellgen singleton sets `dev_nf = max(nf,m)` with `Series` (cellgen.rs:563-572), which is wrong for SPICE `m`. (b) Segmenting preserves L, not R: every segment is its own recognised device with its own heads. (c) Dummy pitch `body_w + max(seg_gap, space_between(rpoly,poly))` (170) differs from the segment pitch `body_w + seg_gap` (83); on sky130 the gap term is 480 vs 400 (resistor.rs:257-260 gives 400; poly.9, sky130.deck:284, gives 480). (d) Mixed li and met1 jumpers (137-155). (e) Only a 1-D equal-count interleave (304-310). The LVS reference has one card and no params (cellgen.rs:889-890). GPurify reduces MOS series/parallel only (`GP/crates/check/src/lvs/reduce.rs:1`), so a multi-segment variant would mismatch. That goes unseen today because rc_filter's 2 µm body admits only n=1 (resistor.rs:318-319 with `res_min_segment` 10 µm). | AC-02, 14, 15; H06-01, H06-02, H08-10, H08-13, H08-20 |
| Cap array `cap_array.rs` | Exact point-symmetric Spiral / Chessboard / BlockChessboard (97-102, 317-351). Dummy ring. TOP never crosses bottom routes (12-17). `metrics()` (357-397). | Stack hard-coded met1/met2/met3 (417, 513-514). The deck's MIM `capm` (sky130.deck:565, rules 407-411) is never drawn. The unit extracts 0.105 fF/µm² (AV-32), while the sidecar and bench size at `cap_density_ff_um2` 2.0 (sky130.json:8). That value equals the MIM model's `camimc` 2.00 fF/µm² (VOL/libs.tech/ngspice/r+c/res_typical__cap_typical__lin.spice:7). `metrics()` is used only by tests. No split DAC, no shield. | AC-01, 16; AV-32; CC-24 |
| Capacitor `capacitor.rs` | Three MOM kinds, bounded enumeration (48-68). | The merged plate includes unit gaps (180-186), so C ≠ n·C_u. No keep-out export (23-26). P is always top (37). | AC-17; H06-40 |
| BJT `bjt.rs` | Concentric emitter, base and collector per unit, all deck-driven (86-202). Emitter contact array matches the emitter (156-176). | `unit_order` is exact only for 1:8 (208-231; test 273-281). [2,2] comes out as AA/BB and [1,4] off-centre (hand trace, H09-49). No `Unit` records (50-68). Emitter = `max(unit_w/l, bjt_min_emitter_side)` (91-92), while the sky130 BJTs are fixed-geometry models (VOL/libs.ref/sky130_fd_pr/spice: `pnp_05v5_W0p68L0p68`, `W3p40L3p40`, `npn_05v5_W1p00L1p00`, `W1p00L2p00`). No deck recogniser (sky130.deck:663-665). | AC-08, CRATES #1, H09-49 |
| Diode `diode.rs` | Substrate and well forms (78-119). | `dev_nf` ignored (89-119). One cut per terminal (99-108). Anode tap 330 nm wide (67) against licon.7 opposite 120 nm (sky130.deck:336), i.e. 170 + 2·120 = 410. "Interdig" = flip (93). No Units. | AC-02, 18, CRATES #7 |
| FinFET `finfet.rs` | Deck-driven fins, trench, LISD, LIG (51-86). | One variant (89-94). No dummies and no sharing across members. `sa = sb = 0` (186). Silent W floor (114). No tests. | AC-09 |
| Inductor `inductor.rs` | — | Every turn anchored at (0,0), so all turns merge into one met1 conductor (46-57). li centre tap with no via (63-65). turns = Σ`dev_nf` (23). | AC-03 |
| Rings `post_cell.rs` | Minimum-width bands with rows sized to `max_ring_resistance_mohm` (355-367; Charbon §8.6.2, charbon_substrate.txt L3695–3704). Cluster splitting around foreign cells (79-137). | `Ecgr`/`Hcgr` drawn as the bulk's own tap (310-323), i.e. majority-carrier rings. The enum docs say the opposite (analog/src/cell.rs:25-35). The annotator rings every matched FET, tied to its bulk (constraints.rs:66-84). `tap_pitch_nm` and `enclosure_complete` unused. `ring_gap` ignores `width` (299-302). `well_bridges` only for identical facing spans (424-430). | AC-06, 23; H14-27, H05-54 |
| Tests | Per-generator DRC/ERC on sky130. MOS extraction counts. Order properties. | Cross-deck sweeps are release-only (cell_selfcheck.rs:283, 314). `check_on` accepts an empty enumeration (362-408). testkit labels every member's G/S/B as one net (lib.rs:159-165). No value, multiplicity or tap-distance tests. | AC-19 |

### 1.3 CRATES.md open issues (cell side)

| # | Status | Plan |
|---|---|---|
| 1 | BJT cell violations and missing recogniser | CELL-09 (fixed-geometry emitters); MAT-02 (array order); PERF-18 (recogniser) |
| 4 | Placer grid 10 nm | CELL-03 (extents); PLC-02 (origin snap, injected macros) |
| 5 | LU.2 second tap row | FLOW-05 (deck rule, key 15 000); REL-06 (contract); CELL-13 (reach test; second strip deferred) |
| 6 | xhrpoly slots done (sky130.json:151-153); rbody pex 319 Ω/□ (deck, FLOW-05; model card gives 317.3885, VOL/libs.ref/sky130_fd_pr/spice/sky130_fd_pr__res_high_po.model.spice:27); multi-segment LVS open | CELL-01, CELL-06 |
| 7 | Diode anode tap < 410 nm | CELL-07 |
| 8 | "Centroid diff pairs discarded": resolved by split gates (mosfet.rs:17-21, test 1006-1030); doc stale | FLOW-15 step 5 (docs) |
| 9 | Obsolete `bjt_*` REQUIRED_RULES (pdk.rs:1040-1042, 1044; `bjt_min_emitter_side` at 1043 is read, bjt.rs:91, 234) | FLOW-04 step 5 |
| 10 | Unused ring/unitization fields | REL-07 (ring fields), FLOW-15 step 6 (unitization fields) |

---

## 2. Target: what "better than hand layout" means here

| # | Metric | Threshold | How measured |
|---|---|---|---|
| T1 | DRC/ERC of every enumerated variant | 0 findings on sky130, gf180mcu, ihp_sg13g2 (and generic_finfet for MOS); 0 magic `drc(full)` once PERF's cross-check runs (AV-02) | `cell_selfcheck.rs` sweeps; PERF xcheck |
| T2 | No silent devices | 0 empty macros without a `cell/undrawable` finding naming the right device; 0 shorted drawings (inductor) | `signoff_fixtures.rs`; CELL-02, 04 |
| T3 | LVS closure on sky130 | Every drawn MOS, resistor (all segment counts), MIM capacitor and diode is compared by device count and connectivity (0 skipped for these kinds); BJT reported `lvs-coverage/unverified` (PERF-02) until PERF-18 adds the recogniser. No R/C/D parameters are compared: GPurify measures `w`/`l` for MOS only and `area` for every device, and emits a measured param only when the reference interns its name (GP/crates/check/src/topology/device.rs:295-323, GP/crates/check/src/lvs/graph.rs:111-115), so values are T4's job | reference built from `Macro.drawn` (CELL-01) |
| T4 | Value fidelity of drawn passives | Resistor: Σ over a string's drawn segments of `ResModel::ohm(w, l)` within 0.5 % of the schematic device's model value, for every variant. MIM: per-unit model C from the drawn plate equals the model C of the schematic W×L (area + perimeter terms, `c_dw_nm` applied). Diode: Σ cathode diffusion area = `dev_nf`·W·L exactly (after grid snap) | CELL-06/07/08 tests computed from `Macro.drawn` |
| T5 | Density at hand-layout pitch | MOS contacted gate pitch = deck minimum (430 nm, sky130 L=150, derived) for unmatched, Minimal and Moderate rows; Exceptional rows 550 nm (isolated pads, CELL-12). CELL-11's `m1_land` uses deck values only, so the pitch is 430 whether or not the sidecar `m1_enc` 65 stays (CELL-11 deletes it). Drain junction area per interior finger 280·W vs 580·W today (−52 %, derived) | `sd_and_pitch` test; bench area/active % |
| T6 | Exact common centroid by construction | Centroid offset 0 (integer check) for every MOS count vector that MAT-03's `diffusion_cc_row` accepts, every BJT/diode set with ≤1 odd member (MAT-02), every resistor set with ≤1 odd member (MAT-12); 4-row MOS variants cancel gradient orders 1–3 when every member has ≥ 8 units (MAT-15) | MAT property tests plus CELL tests on the drawn units (CELL-07/10/14/15) |
| T7 | Class-scaled environment | For each class: dummy reach, LOD moat, own-well WPE clearance, gate overhang equality and gate-strap construction per Hastings §13.3 rules 12, 19, 21, 22, at the values `class::mos_env` returns (MAT-07) | CELL-12 tests; PERF-23 on the final GDS |
| T8 | Latch-up reach | Max diffusion-to-own-tap distance ≤ `tie_max_dist_nm` for 100 % of MOS variants on every shipped deck (sky130 15 000 nm after FLOW-05/REL-06) | CELL-13 test; LU.2/LU.3 deck rules (FLOW-05) |
| T9 | Known optima of fixtures | bgr_core: Q1's unit at the 3×3 centre, centroid residual 0 (bgr_core.spice:1-2); the 1:2:4 `mirror_ratio` fixture merges into one cell | bench, MAT-02/03, CELL-10 |

Why this beats a hand layout: the hand habit segments a resistor at constant L. On sky130 `res_high_po` that adds 523 Ω per extra segment (W = 0.69 µm: the 409.4 Ω head term plus 113.8 Ω from the model's +0.247 µm length offset), +5.4 % at n=2 for a 20 µm body (CELL-06, derived; corrected from "409 Ω", which omitted the length-offset term). Hand layouts rarely go past ABBA (CC-04) and apply class rules unevenly (H13-47). The generator solves every one of these exactly for every variant.

## 3. Work items

Order: correctness first (P0), then core capability (P1), then advantage (P2). Numbers marked *derived* are arithmetic on cited values.

### CELL-01 A macro records what it drew: extracted-device cards and keep-outs

Status: done in M0 (`409e74b`); keep-outs are plumbing only and `Drawn` is resistor-only (see the Status note under Acceptance).

- Priority: P0. Effort: M. Depends on: none. Unblocks CELL-06/07/08/09/14/19.
- Why: AC-02, AC-15 (multi-segment LVS unverified), AC-17 (no keep-out export), AV-19 (R/D/BJT carry no params), AV-01 (caps silently dropped from LVS). H06-02: the modelled per-segment value must be "in the macro metadata, so the LVS reference and perf read the same value" (the quote is the H06-02 automation recipe, not the book; the book's basis is per-segment heads and contacts, hastings.txt L16932–17018, L17729–17744). H08-28 and H13-51: routing needs body and gate rects (L24840–24898, L42607–42617).
- Current:
  - `Macro` has no field for devices other than MOS fingers and dummies (core/src/macro.rs:9-34).
  - `cellgen::reference` builds cards from the schematic only. It skips capacitors and inductors (cellgen.rs:870), passes W/L for MOS only (879-885), and emits one param-less card per resistor or diode and one param-less card per BJT unit (886-891).
  - The resistor's `segments` variant axis (resistor.rs:12-18) is invisible to it.
  - The capacitor plate stack is not exported (capacitor.rs:23-26).
- Change:
  1. `kernel/core/src/geom.rs:10`: add `Default` to the `Rect` derive.
  2. `kernel/core/src/macro.rs`:
     ```rust
     #[derive(Clone, PartialEq, Debug, Default)]
     pub struct Macro {
         pub shapes: Vec<Shape>, pub pins: Vec<Pin>, pub bbox: Rect,
         pub units: Vec<crate::units::Unit>, pub dummies: Vec<Dummy>,
         /// Devices this macro draws that extract differently from their schematic card
         /// (a resistor's series segments, a MIM's unit plates, a diode's or BJT's units).
         /// The LVS reference lists these for `device` instead of the schematic card.
         pub drawn: Vec<Drawn>,
         /// Regions where foreign metal changes the device; RTE decides the policy per `why`.
         pub keepouts: Vec<Keepout>,
         /// Per-variant figures (CELL-19).
         pub figures: Figures,
     }
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub struct Drawn {
         pub owner: u8,
         /// Filled by the caller when it binds pins (`cellgen::bind_pins`).
         pub device: Option<crate::ids::DeviceId>,
         pub kind: DrawnKind,
         /// Terminal nodes in the LVS reference's pin order: R/C/D `[P, N]`; BJT the order of
         /// `cellgen::reference`'s BJT pin list (today `["E","B","C"]`, cellgen.rs:868-869; PERF-18 step 1
         /// changes it to C, B, E). Both read one `pub(crate) const BJT_PINS: [&str; 3]` in cellgen.rs,
         /// so the two orders cannot diverge.
         pub nodes: [Node; 3],
         /// Drawn body, nm: resistor segment W×L, MIM plate W×L, diode junction W×L, emitter W×L.
         pub w: i32,
         pub l: i32,
     }
     #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum DrawnKind { Resistor, Capacitor, Diode, Npn, Pnp }
     /// A member pin by terminal name, or a node internal to the macro.
     #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Node { Unused, Pin(&'static str), Internal(u16) }
     #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct Keepout { pub rect: Rect, pub why: KeepWhy }
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum KeepWhy { Gate { owner: u8 }, ResistorBody { owner: u8 }, CapPlate { owner: u8 } }
     ```
     `Figures` is defined in CELL-19. Land it here as an empty `#[derive(Default)] struct Figures {}` so the type change happens once.
  3. `place_macro` (macro.rs:42-60): transform `keepouts[i].rect` with `shift` exactly like shapes. Copy `drawn` and `figures` unchanged.
  4. `Builder` (builder.rs): add fields and methods `pub fn drawn(&mut self, d: Drawn)`, `pub fn keepout(&mut self, r: Rect, why: KeepWhy)` (rect snapped like `rect`), and carry both into `finish`.
  5. Every `Macro { … }` struct literal gets `..Default::default()`. Sites found by `grep -rn "dummies:"`: builder.rs:134, core/macro.rs:58, cellgen.rs:831/1050/1136/1540, geometry.rs:187/271/317, fill.rs:73, gr/lib.rs:1161, dr/lib.rs:1019/2228/2252/2319/2371/2461/2529, dp/tests.rs:92/165, macroMaster/adapter.rs:18/49, library tests injected_macro.rs:28 and extra_devices.rs:33. Only the literals among these.
  6. `cellgen::bind_pins` (cellgen.rs:937): for every `m.drawn[i]`, set `device = Some(members[owner])`.
  7. New in cellgen.rs, next to `dummy_cards` (912):
     ```rust
     /// LVS cards for everything the placed cells drew as `Macro::drawn`, and the schematic devices they replace.
     pub fn drawn_cards(placed: &[Macro], nets: &[String], schematic: &Netlist, pdk: &Pdk)
         -> (Vec<RefDeviceIn>, Vec<DeviceId>)
     ```
     (`pdk` is used only for `pdk.recipe(kind, model)`.)
     - `Node::Pin(t)` resolves to the net of the placed macro's pin `d{owner}:{t}` (as `dummy_cards` does, cellgen.rs:916-919).
     - `Node::Internal(k)` resolves to `format!("~{cell}.{owner}.{k}")` with `cell` = the macro's index in `placed`; it is not a port.
     - Returned `Vec<DeviceId>` = the distinct `Drawn.device` values seen (sorted, deduplicated): the schematic devices whose own cards `reference` must skip.
     - `model` = `pdk.recipe(kind, &schematic.devices[id].model).map(|r| r.model)`, else the schematic model.
     - params: none. GPurify measures `w`/`l` for MOS markers only and `area` for every device (GP/crates/check/src/topology/device.rs:295-323), and a measured param reaches the compare only when the reference interns its name (GP/crates/check/src/lvs/graph.rs:108-122). A non-MOS card with `w`/`l` would be a one-sided param, which is an LVS mismatch (cellgen.rs:907-911). The earlier `Pdk::extracts_params(kind)` probe is dropped: the answer is fixed by that code, `true` only for `Nmos | Pmos`. PERF-02 and plan-07 §0.2 name the probe; they get this rule instead. `area` params are deferred (appendix): interning `"area"` makes every extracted MOS carry `area` too, so every MOS and dummy card would need one.
  8. `cellgen::reference(netlist, fold)` → `reference(netlist, fold, skip: &[DeviceId])`: drop the cards of devices in `skip`. `labels_and_reference` (lib.rs:1351-1370) calls `drawn_cards`, then `reference(…, &replaced)`, then `dummy_cards`. The `Capacitor | Inductor => continue` arm (cellgen.rs:870) stays for devices not replaced. MOM capacitors without a recogniser remain unverified, reported by PERF (AV-01).
- Tests:
  - `kernel/core/src/macro.rs::place_macro_moves_keepouts_and_keeps_drawn`:
    - a macro with bbox (0,0,400,200) and keepout (0,0,100,50), placed at x=1000, y=2000, R0 → keepout (800,1900,100,50) for hw=200, hh=100;
    - under R90 the keepout w/h swap;
    - `drawn` is unchanged.
  - `frontend/library/src/cellgen.rs::drawn_cards_carry_no_params_for_passives`: a Resistor macro with 2 `Drawn` segments and a bound pin set → 2 `RefDeviceIn` of kind Resistor, each with `params.is_empty()`, the internal node named `~0.0.1` on both cards (cell index 0, owner 0, internal 1).
  - New `frontend/library/tests/drawn_cards.rs`:
    - `a_segmented_resistor_signs_off_lvs_clean`: netlist `XR1 a b sky130_fd_pr__res_high_po w=0.69u l=40u`, the n=2 variant forced by variant index → `signoff_with_intent` has 0 rows starting `lvs/`; the reference holds 2 resistor cards.
    - `a_missing_segment_is_an_lvs_error`: remove one `drawn` entry before signoff → ≥ 1 `lvs/` row.
- Acceptance: every resistor variant (not only n=1) of the bench fixtures is LVS MATCH, and no signoff row names a segment.
- Status (M0, recorded by the review panel): `Macro::keepouts` and `Builder::keepout` are plumbing only. No generator
  calls `keepout` (the only reader, dr's `place_macro` copy, moves an empty list), so AC-17 ("no keep-out export")
  stays open until CELL-06/RTE-15 record gate, resistor-body and cap-plate rects. `Drawn` is emitted by the resistor
  alone; the MIM, diode and BJT units carry none yet, so their LVS cards still come from `cellgen::reference`.
- Risks:
  - GPurify's refinement may treat named internal nets as anchors. Then `~c.o.k` names break pairing, and the fallback is the empty name.
  - The struct change touches about 20 literals, but each touch is mechanical.

### CELL-02 Generator tests that cannot pass vacuously

Status: done in M0 (`85e05bb`).

- Priority: P0. Effort: S. Depends on: none.
- Why: AC-19. H01-29 ("a properly designed Pcell always passes DRC", hastings.txt L6919–6922).
- Current:
  - `testkit::port_name` names every member's G/S/B as one net (lib.rs:159-165), so gate shorts between members are invisible to ERC.
  - `check_on` never asserts that variants exist (cell_selfcheck.rs:362-408). (The in-crate `testkit::dirty_group` already asserts a non-empty enumeration, lib.rs:121; only the integration test's `check_on` does not.)
  - Both cross-deck sweeps are `#[cfg(not(debug_assertions))]` (cell_selfcheck.rs:283, 314).
- Change:
  1. `lib.rs` testkit (lib.rs:118-165): `pub fn dirty_group_with<G: crate::Cell>(group: &DeviceGroup, c: &analog::Constraints, pdk: &verify::Pdk, common: &[&str]) -> Vec<String>` is today's `dirty_group` body with `port_name(pin, shared)` (lib.rs:159-165) matching `t` against `common` instead of the literal `"G" | "S" | "B"`; the substrate terminal (`shared`, lib.rs:130-134) stays common as today. `dirty_group` becomes `dirty_group_with::<G>(group, c, pdk, &["G", "S", "B"])` (mirror semantics, unchanged).
  2. `mosfet.rs` test `split_gate_variants_keep_private_gates_under_erc`: for each `v` in `Mosfet::enumerate(..)` with `v.split_gates`, `testkit::findings(&m.shapes, &testkit::ports_with(&m, &["S","B"]), pdk)` is empty (`ports_with` = today's private `ports`, lib.rs:144-153, made `pub` with the `common` list).
  3. `cell_selfcheck.rs`:
     - The list, derived from the enumerate guards read in this review (bjt.rs:34 needs a `dnwell` layer and `npn_isolation` ≠ 0, set only in pdks/sky130.json; resistor.rs:23 needs an `rpoly` role, absent from pdks/generic_finfet.json; `Pnp` has no layer guard, bjt.rs:27-34, so it is not listed):
       ```rust
       const UNDRAWABLE: &[(&str, DeviceKind, &str)] = &[
           ("gf180mcu", DeviceKind::Npn, "no npn_isolation (bjt.rs:34)"),
           ("ihp_sg13g2", DeviceKind::Npn, "no npn_isolation (bjt.rs:34)"),
           ("generic_finfet", DeviceKind::Npn, "no npn_isolation (bjt.rs:34)"),
           ("generic_finfet", DeviceKind::Resistor, "no rpoly role (resistor.rs:23)"),
           ("*", DeviceKind::Inductor, "no recogniser (CELL-04)"),
       ];
       ```
     - `check_on` returns the finding `"{kind:?}: no variants"` when `G::enumerate` is empty and `(deck, kind)` is not listed (`"*"` matches any deck).
  4. New debug-build test `every_generator_enumerates_on_every_deck` (no DRC, fast). For each of the four decks, one single-device group per kind, sized like the DRC sweep: MOS through `Mosfet` on the three planar decks and `FinFet` on generic_finfet (cellgen.rs:795); Capacitor through `CapArray` then `Capacitor` (cellgen.rs:803-806, the flow's order); Resistor, Diode, Npn, Pnp, Inductor through their generators. Assert: enumeration non-empty ⇔ the pair is not in `UNDRAWABLE`. The ⇔ also fails a listed pair that starts drawing, so the list cannot go stale.
- Tests: the above. Value tests live in CELL-06/07/08/09.
- Acceptance: `cargo test -p cells` in debug runs the enumeration test. A generator returning nothing on an unlisted deck fails it.
- Risks: per-member gate labels may expose real shorts in some variants. That is the purpose.

### CELL-03 Macro extents on twice the cut lattice
- Status: done in M1 (`9b21b3a`; cells merge `726bc13`).
- Priority: P0. Effort: S. Depends on: none. Cross: PLC (origin snap and debug assertion, AP-05).
- Why: AP-05, CRATES #4. Grid discipline, H01-26 (hastings.txt L6565–6568, L6720–6726: centred objects need extents on twice the increment).
- Current:
  - `finish` rounds corner and extents to `step = 2·grid`, 10 nm on sky130 (builder.rs:125-133).
  - The placer stamps at `x − hw − anchor.x` (core/macro.rs:47-48), where `hw` is a 5 nm multiple, so cuts land off the 10 nm lattice.
  - The ring halo is rounded to the grid (lib.rs:1198). It widens `w` by 2·ext, which breaks the same invariant.
- Change:
  1. builder.rs `finish`: `let step = 2 * self.grid.max(1); let ext = 2 * step;`. Corner floors to `step`; `w`, `h` round up to `ext`. Update the doc (builder.rs:114-117).
  2. builder.rs: `pub fn cut_lattice(p: &dyn Process) -> i32 { 2 * p.grid().max(1) }` (10 nm on sky130); `finish` uses it for `step`.
  3. lib.rs:1198: `round_up(ring_halo(..), cells::builder::cut_lattice(pdk))`: a halo on 10 nm keeps the corner on 10 nm and `w + 2·halo` on 20 nm. PLC-02 edits the same line; whichever lands second rebases.
- Tests:
  - builder.rs `extents_are_twice_the_cut_lattice`: grid 5, shapes spanning x ∈ [3, 1238] → `bbox.x % 10 == 0`, `bbox.w % 20 == 0`, bbox covers the shapes.
  - `cell_selfcheck.rs::every_half_extent_is_on_the_cut_lattice`: every sky130 variant of every generator has `(bbox.w/2) % 10 == 0` and `(bbox.h/2) % 10 == 0`.
- Acceptance: PLC's origin assertion holds on every bench circuit, and no off-lattice cut is left for magic to flag.
- Risks: each macro grows by ≤ 10 nm per axis.

### CELL-04 Inductor: stop drawing a shorted coil

Status: done in M0 (`9dba018`, `3b06b02`); `Inductor::enumerate` is empty, every `L` device is one `cell/undrawable` row, and `ind_min_trace`/`ind_min_diameter` left `sidecar::KEYS` and the four sidecars.

- Priority: P0. Effort: S. Depends on: none.
- Why: AC-03. H06-49/50/51 (Hastings §7.2.2 guidelines, hastings.txt L21510–21567). The current drawing violates guidelines 2, 3, 6, 8 and 9 and is not even a coil.
- Current: all turns start at (0,0) and merge (inductor.rs:46-57). The li centre tap has no via (63-65). No recogniser; LVS skips inductors (cellgen.rs:870).
- Change:
  - `Inductor::enumerate` returns `vec![]`. The doc says no deck recognises an inductor and the rectangular met1 spiral was one conductor.
  - `draw` becomes `Builder::new(process.grid()).finish()`; delete the body.
  - Delete the existing `inductor.rs::every_variant_is_drc_and_erc_clean` (inductor.rs:80-91): it goes through `testkit::dirty_group`, which asserts a non-empty enumeration (lib.rs:121) and would panic once `enumerate` returns nothing.
  - Remove `ind_min_trace` and `ind_min_diameter` from `REQUIRED_RULES` (pdk.rs:1051-1052 region) and from the four sidecars.
  - Signoff then reports `cell/undrawable` (lib.rs:1293-1300).
  - A real spiral generator is deferred (appendix, former CELL-28).
- Tests: `inductor.rs::an_inductor_is_undrawable_without_a_recogniser` (sky130: enumerate empty). `benchmarks/tests/signoff_fixtures.rs` gets a netlist with one `L` device → exactly one `cell/undrawable` row.
- Acceptance: no inductor geometry in any output.

### CELL-05 Guard-ring kinds name what is drawn — moved to REL-07
- Status: reinstated as GAP-05 (master §6 C6) and done in M1 (`b1510f4`): `GuardRingType { Tap { in_well }, Ecgr, Hcgr }`, `post_cell::drawable`; `Hbgr`, `Ebgr`, `tap_pitch_nm`, `enclosure_complete` deleted. The rest of this line is superseded. Was: cut from this plan (see appendix). REL-07 step 1 replaces `GuardRingType` with `PTap | NTap | Ecgr`, deletes `Hcgr`, `Hbgr`, `Ebgr`, `tap_pitch_nm` and `enclosure_complete` (with construction sources for each deletion, H05-58/H14-33 plus §5.4.4 L15614–15618 for the EBGR), and step 2 rewrites `post_cell::implant_name`/`in_nwell` (post_cell.rs:315, 321) and the annotator request. The type change and the drawing change are one diff there; a separate CELL rename would be undone by it. CELL's remaining ring work is CELL-17 (the ECGR well shape REL-07 does not draw).

### CELL-06 Resistors: honour multiplicity and keep the value exact under segmentation
- Status: done in M1 (`bda5935`; cells merge `726bc13`).
- Priority: P0. Effort: M. Depends on: CELL-01. Cross: FLOW (bare `R` values and `sheet_ohm` fixes, AF-02 and AV-16), EXT (ratio sets, CELL-14).
- Why:
  - AC-02 (critical).
  - H06-01/H06-02: Hastings eq 6.22 R = R_s(L_d − 2L_b)/(W_d + W_b) + 2R_h(L_h + L_b)/(W_d + W_b) (hastings.txt L17729–17744). Splitting a body adds heads and contacts: "+9 %" example (L17014–17018).
  - H06-03: segments ≥ 10 µm (L16358–16360).
  - H08-10: equal segment length (L22207–22215).
  - CRATES #6.
- Current:
  - `body_w = unit_w.max(res_min_width)`, `seg_l = unit_l / n_segments` (resistor.rs:81-82). Every member is drawn once (101-160, sequence 304-310), and `dev_nf` is never read. Since CELL-01 each series segment already records one `Drawn` card (resistor.rs:163), so a multi-segment variant is LVS MATCH; the value is still not preserved and no keep-out is recorded.
  - `feasible_segments` keeps n | body_l, n ∈ {1, even}, eight squarest (313-337).
  - cellgen gives resistor singletons `dev_nf = max(nf, m)` with `Series` (cellgen.rs:563-572).
  - sky130 `res_high_po` model card (VOL/libs.ref/sky130_fd_pr/spice/sky130_fd_pr__res_high_po.model.spice:27-40; `rhead` at 39, `rbody` at 40): R(W,L) = 317.3885·(L+0.247)/weff + 345.8312/(weff+0.1558) Ω, with weff = W − 0.001 − 0.0672·max(0.69−W, 0) and W, L in µm. At W = 0.69, L = 40 µm: 18 949 Ω (*derived*).
  - Today's n=2 variant (two 20 µm bodies, each a recognised device with its own head term of 409.4 Ω) gives +2.76 %. n=4 gives +8.3 %. `m=2` drawn once gives +100 % (*derived*).
- Change:
  1. resistor.rs:
     ```rust
     /// A recipe's per-device resistance, integers from the sidecar:
     /// R = sheet·(L + dl)/weff + head/(weff + head_dw),  weff = W + dw − narrow·max(knee − W, 0).
     #[derive(Clone, Copy, Debug)]
     pub struct ResModel { pub sheet_mohm: i64, pub head_mohm_um: i64, pub dl_nm: i32, pub dw_nm: i32,
                           pub head_dw_nm: i32, pub narrow_permille: i32, pub knee_nm: i32 }
     impl ResModel {
         /// `None` when the recipe states no `res_sheet_mohm`.
         pub fn of(p: &dyn Process) -> Option<Self>;
         /// One device of drawn W×L, Ω.
         pub fn ohm(&self, w_nm: i32, l_nm: i32) -> f64;
         /// Body length so `n` series devices of width `w` total `target`: L = (target/n − head/(weff+head_dw))·weff/sheet − dl,
         /// snapped to `lat`; `None` below `min_nm` or when the snapped residual exceeds `tol_ppm`.
         pub fn seg_len(&self, w_nm: i32, target_ohm: f64, n: u32, min_nm: i32, lat: i32, tol_ppm: i32) -> Option<i32>;
     }
     ```
  2. Sidecar recipe keys:
     - `resistors.recipes.high_po` (sky130.json:140-154): `"res_sheet_mohm": 317389, "res_head_mohm_um": 345831, "res_dl_nm": 247, "res_dw_nm": -1, "res_head_dw_nm": 156, "res_narrow_permille": 67, "res_knee_nm": 690, "res_model_source": "sky130_fd_pr__res_high_po.model.spice (volare fa87f8f4) lines 27-40"`. (`res_narrow_permille` 67 and `res_head_dw_nm` 156 round the card's 0.0672 and 0.1558 µm; the rounding only matters below W = 0.69 µm and in the fourth significant digit of the head term.)
     - `generic_po`: `"res_sheet_mohm": 48200` (`rp1 = 48.2`, VOL/libs.tech/ngspice/r+c/res_typical__cap_typical.spice:21), head 0, dl 0, dw 0. The model's `dw = -tol_poly/2 - poly_dw/2` (VOL/libs.tech/ngspice/sky130_fd_pr__model__r+c.model.spice:134) evaluates to +0.028 µm at typical (`poly_dw = -0.056u`, same file :78; `tol_poly = 0.0`, r+c/res_typical__cap_typical.spice:6); how ngspice applies `dw` to W was not checked; see §5.
     - Top level: `"res_value_tol_ppm": 5000`.
     - `Pdk::recipe` keeps every integer recipe key and `Overlay::rule` reads them first (pdk.rs:1142, 1162).
  3. Target value: `target = model.ohm(unit_w, unit_l)`, one schematic device of the netlist's W×L. Without a model, only n = 1 is offered, so the value never changes silently.
  4. Multiplicity: when `series_parallel == Parallel`, member d draws `dev_nf[d]` identical strings side by side, each of `segments` series segments of `seg_len(W, target, segments, res_min_segment, lat, tol)`. Each string carries its own `d{i}:P` and `d{i}:N` pins. Repeated names are joined by the router, as MOS S/D pins are. cellgen.rs:569-572: resistor singletons become `Parallel` (SPICE `m` is parallel). `Series` is CELL-14's ratio construction.
  5. `feasible_segments` keeps n with `seg_len` `Some`. At W = 0.69, L = 40 µm: n=2 gives 19.432 µm (kept); n=4 gives 9.148 µm, under `res_min_segment` 10 µm, so it is dropped (*derived*).
  6. The per-segment `b.drawn(Drawn{owner, kind: Resistor, nodes: [a, b, Unused], w: body_w, l: seg_l, device: None})` exists since CELL-01 (resistor.rs:163); it now runs once per segment of every parallel string, and records `w: body_w, l: seg_l` of the value-preserving `seg_len`. The string runs Pin("P") → Internal(k) … → Pin("N"). Add `b.keepout(body rect, KeepWhy::ResistorBody{owner})` (no generator calls `keepout` on m0).
  7. Unit weight for m parallel strings: `body_w·seg_l / m²` (∂R/∂R_i, Hastings eqs 8.24–8.25, hastings.txt L23277–23290).
- Tests (resistor.rs; sky130 through `Overlay` of `high_po`):
  - `segments_preserve_the_model_value`: W=690, L=40 000, every variant: |Σ_drawn `ohm(w,l)` − 18 949.2| / 18 949.2 ≤ 0.005.
  - `short_segments_are_not_offered`: W=690, L=40 000 → no variant with 4 segments.
  - `a_multiplied_resistor_draws_parallel_strings`: `dev_nf=[2]`, Parallel → `drawn.len() == 2·segments`, two P and two N pins, each string within 0.5 %.
  - `every_variant_extracts_one_resistor_per_segment`: `verify::extract_spice` resistor card count == `drawn.len()`.
  - The existing DRC sweep adds `dev_nf=[2]`.
  - Fixture `benchmarks/fixtures/res_m2.spice`: `XR1 a b sky130_fd_pr__res_high_po w=0.69u l=40u m=2` plus an inverter driving `a`. Bench shows LVS MATCH with 2·n cards.
- Acceptance: T3 and T4 for resistors. rc_filter and res_m2 are LVS MATCH for every variant.
- Risks:
  - Coefficients come from one volare build. The source string records it.
  - `res_high_po` exact-width rules (rpm.1b–1f) are not in the deck (sky130.deck:653), §5.

### CELL-07 Diodes: multiplicity, contact arrays, and a licon.7-legal anode tap
- Status: done in M1 (`1d03998`; cells merge `726bc13`).
- Priority: P0. Effort: S. Depends on: CELL-01, MAT-02 (`pattern::grids`, `pattern::centro_assign`).
- Why:
  - AC-02, AC-18, CRATES #7.
  - H09-43: matched diodes identical and common-centroid (Hastings §11.3.4 r4, r7, r8; hastings.txt L33927–33968).
  - Hastings §11.3.4 r5–r6: sizes 2–10× minimum with contact arrays (L33935–33956).
- Current: one diode per member (diode.rs:89-119); one cut per terminal (99-108); `tap_side = ct + diff_enc + tap_cap` = 330 nm (67); licon.7 needs 120 nm on both sides of one axis (sky130.deck:336), so 410 nm.
- Change:
  1. Member d draws `dev_nf[d]` units. Variants = `pattern::grids(&dev_nf, 3.0)` (MAT-02); slot owners = `pattern::centro_assign(&dev_nf, rows, cols, Fill::Balanced).0`, slot i at column `i % cols`, row `i / cols`, empty slots left empty. A single device keeps today's 1×n, n×1 and square columns (diode.rs:36). `Diode { rows: u16, cols: u16 }` replaces `{ pattern, columns }`; delete the `Pattern` field (AC-26).
  2. Cathode: a contact array at pitch `ct + space(licon)` inside `max(enclosure, endcap)(diff, licon)`, with an li plate. Same construction as bjt.rs:162-176.
  3. Anode tap: `tap_side = ct + 2·max(tap_cap, diff_enc)`, which is 410 on sky130. Tap height = cathode height, with a cut column.
  4. `Unit{owner, x, y: cathode centre, weight: w·l, phi: (0,0), sa: 0, sb: 0}`. `Drawn{kind: Diode, nodes: [Pin("P"), Pin("N"), Unused], w, l}`.
- Tests (diode.rs):
  - `a_multiplied_diode_draws_its_area`: `dev_nf=[4]`, W=500, L=1000 → 4 units; Σ cathode diff area = 4·5·10⁵ nm².
  - `the_anode_tap_meets_licon_7`: sky130 tap rect width ≥ 410.
  - `matched_diodes_share_a_centroid`: `dev_nf=[2,2]` and `[1,4]`, every variant → for members A, B: n_B·Σ_{u∈A} u.x == n_A·Σ_{u∈B} u.x and the same in y (i64, exact because MAT-02 closes each member under 180° rotation about the grid centre).
  - DRC sweep for `[1]`, `[4]`, `[2,2]`.
- Acceptance: T4 for diodes. The flow-inserted antenna diode stays clean (`frontend/library/tests/antenna_diode.rs`).

### CELL-08 Capacitors drawn as the deck's device (MIM on sky130), unit-exact MOM elsewhere

Status: done in M1 (`c72a623`, `47a8231`, `6cdfc7a`; cells merge `726bc13`). Step 7 (`"cap"` alias) not taken: sky130 `mim_m3_1.aliases` is `["cap_mim_m3_1"]`. **Open:** `dac4_mim` and `tq_chain` have no `BASELINE` row; tq_chain through the CLI has `drc/m1.2.notch:met1`, and the bench (FI 5) ERC `erc/ar.met3.1:gate` (m1-report §2); carried to M2 item 0.

Field report: FR-1 (several `cap_mim_m3_1` hang; LVS device-class mismatch with one). The hang and the class message are the old packaged build's (Philis `e8bc59e`, GPurify `e3c8eb2`); on m0 there is no hang (async_ctrl `tq_chain`, 8 caps of 21.87 µm: 3 min 20 s, 37 MB) but sky130 has no `cell.capacitors` recipe, so the MIM is drawn as met1/met2 MOM plates on the only two routing layers. Measured with the m0 CLI and bench (seed 1): one cap → every one of 56 dr reports carries `drawn short net <tap1|vss> to cell metal`, signoff `lvs/extract: label short: labels ["vss", "tap1"]`; eight caps → label short (`tap3`, `tap2`, `vss`), 5 × `drc/m1.2:met1`, 1 × `drc/via.2:via`; wta (one 9.83 µm MIM) → label short (`vss`, `mrail`). Added acceptance: the fixture `benchmarks/fixtures/tq_chain.spice` (async_ctrl `tq_chain`, 8 × `sky130_fd_pr__cap_mim_m3_1 W=21.87 L=21.87`) signs off with DRC 0, no `lvs/` row, and no dr report on it carries `drawn short`.

- Priority: P0. Effort: L. Depends on: CELL-01. Cross: PERF (AV-01 verdict for MOM, AV-32 calibration), FLOW (fixture sizing, recipe schema), RTE (plate keep-outs).
- Why:
  - AC-01 (critical), AC-17, AV-01, AV-32.
  - H06-43: pick the construction from the deck.
  - H06-45: a MIM can sit over lower-level circuitry (hastings.txt L20649–20661).
  - H08-11: identical units, area/periphery equality (L22218–22272).
  - H08-14: dummy ring (L25550–25561).
- Current:
  - CapArray stack: BOT met1, TOP met2 inset, met3 straps (cap_array.rs:12-17, 417, 513-514).
  - capacitor.rs merges units with gaps (180-186).
  - sky130 recognises `capm` with terminals [met4, met3] (sky130.deck:565). Its rules: capm.1 width ≥ 1 µm, capm.2a space ≥ 840 nm, capm.3 met3 encloses capm by ≥ 140 nm, capm.4 capm encloses via3 by ≥ 140 nm, capm.11 space(met3 not on capm, capm) ≥ 500 nm (sky130.deck:407-411).
  - Model: C = camimc·wc·lc + cpmimc·2(wc+lc), with typical camimc 2.00 fF/µm² and cpmimc 0.19 fF/µm (VOL/libs.tech/ngspice/r+c/res_typical__cap_typical__lin.spice:7-8; VOL/libs.ref/sky130_fd_pr/spice/sky130_fd_pr__cap_mim_m3_1.model.spice:23-24). [corrected: the card uses wc = W + m3_dw + tol_m3 and lc = L + m3_dw + tol_m3 (same file :17-18), not the drawn W, L; m3_dw = −0.025 µm (VOL/libs.tech/ngspice/sky130_fd_pr__model__r+c.model.spice:82) and tol_m3 = 0 at typical (r+c/res_typical__cap_typical.spice:10).]
  - Since PERF-02, `cellgen::reference` sends every capacitor to LVS (one card per unit, `max(nf, m)`), and `recogniser_for` has no model fallback for R/C/D, so a MOM card is never compared as `capm`: each is an `lvs-coverage/unverified:Capacitor:<model>` row (dac4: 16).
- Change:
  1. Recipe table: FLOW-06 step 4 owns `cell.capacitors` (schema, `Pdk::capacitor_recipe`, the sky130 `mim_m3_1` and `mom_m1m2` rows with roles `bottom`, `plate`, `top_contact`, `top`). CELL adds only what the generator reads, in the same change or right after it:
     - `mim_m3_1`: `"c_dw_nm": -25` (m3_dw = −0.025 µm, VOL/libs.tech/ngspice/sky130_fd_pr__model__r+c.model.spice:82; tol_m3 = 0 at typical, r+c/res_typical__cap_typical.spice:10), and `"cap"` appended to `aliases`, so the bench preprocessor's `cap` model (fixtures.rs:556-559, sized at 2.0 fF/µm²) resolves to the MIM.
     - `mom_m1m2`: roles `"top_contact": "via2"`, `"strap": "met3"`, `"bottom_contact": "via1"` (today's construction, cap_array.rs:417, 513-514).
     - FLOW's `default` stays `mom_m1m2`, so a model no recipe names keeps today's drawing.
     - The generator reaches the table through `Overlay { pdk, recipe: pdk.recipe("capacitor", model)? }` (pdk.rs:1129 reads `{kind}s`); roles resolve only through the recipe (`Overlay::layer`, pdk.rs:1155-1161), because the base deck has none of them. Add `const CAPACITOR_ROLES: &[&str] = &["bottom", "plate", "top_contact", "top", "strap", "bottom_contact"]` next to `RESISTOR_ROLES` (pdk.rs:1087) and match it in `Overlay::layer`, so an unset role is `None`, not a base-deck lookup.
     - Integer keys (`c_area_af_um2` 2000, `c_perim_af_um` 190, `c_dw_nm`) reach the generator through `Recipe.rules` (pdk.rs:1142 keeps integer values) and `Overlay::rule`.
     - `mom_m1m2` has no `c_area_af_um2`: its C is unknown (AV-32) and PERF reports it.
     - Precondition, fixed here: `Overlay::enclosure`/`space_between` answer through `Pdk::widest_on` (pdk.rs:798-817), which keeps only rules whose layers `philis_drawn` accepts (pdk.rs:775-790). `capm` is neither in `DRAWN_ROLES` (pdk.rs:1211-1212) nor a routing layer, and `recipe_layers` scans only `resistors` (pdk.rs:1103-1114), so every two-layer capm lookup returns `None` today (inset 0, not 140). Change `for kind in ["resistors"]` to `["resistors", "capacitors"]` (pdk.rs:1105). Guard test `backend/verify/src/pdk.rs::capacitor_recipes_do_not_move_routing_rules`: on sky130, `space("met3")`, `width("met3")`, `space("met4")` and `enclosure("met3","via3")` are equal before and after (computed once with the table removed from a cloned sidecar), and `Overlay(mim_m3_1).enclosure("bottom","plate") == Some(140)` (capm.3).
  2. cap_array.rs:
     ```rust
     struct PlateStack { bot: LayerId, ins: Option<LayerId>, top: LayerId, top_cut: LayerId, top_cuts_array: bool,
                         strap: LayerId, bot_cut: LayerId, down: Vec<(LayerId, LayerId)> }
     /// From the recipe roles: bot = `bottom`, ins = `plate`, top_cut = `top_contact`, top = `top`, strap = `strap`
     /// (MIM: `top`), bot_cut = `bottom_contact` (MIM: `via2`, the met3 stub's cut); `None` without `bottom`/`top`.
     fn plate_stack(p: &dyn Process) -> Option<PlateStack>
     ```
     - MIM: bot = met3 plate. `ins` = capm inset by `enclosure("bottom", "plate")` (capm.3, 140). A via3 array on capm inset by `enclosure("plate", "top_contact")` (capm.4, 140) at via3 spacing. Strap = met4 per column. `bot_cut` = via2 (met3 stub down to the met2 branch). `down` = [(via3, met3 island off capm, ≥ 500 from capm per capm.11), (via2, met2), (via1, met1)] to the met1 pins.
     - MOM: today's layers (bot met1, top met2 inset, centre via2, met3 strap, bot_cut via1, down [(via2, met2), (via1, met1)]).
     - Unit plate for MIM = the netlist W×L applied to capm; plate = capm + 2·140. Unit pitch ≥ capm.2a 840 between capm edges.
     - `C_u = c_area·(W+dw)(L+dw) + c_perim·2(W+L+2dw)`, dw = `c_dw_nm`: at W = L = 5 µm, 53.28 fF (*derived*; corrected from 53.80 fF, which dropped the card's m3_dw = −0.025 µm).
  3. Single capacitors with a MIM recipe go through CapArray. `enumerate` accepts `dev_nf.len() == 1` when `ins` is Some (cap_array.rs:89). The dummy ring is drawn only when `dummy_required`.
  4. capacitor.rs (MOM singles, one-metal decks): the merged plate area is exactly n·W·L. `grid_w = cols·unit_w`, `grid_h = rows·unit_l` (delete the `unit_gap` terms, capacitor.rs:180-186).
  5. Per MIM unit `Drawn{kind: Capacitor, nodes: [Pin("P"), Pin("N"), Unused], w, l}` (P = top/met4, N = bottom/met3, the recogniser's order), and `keepout(plate rect, CapPlate{owner})`.
  6. cellgen draw_variants (cellgen.rs:803-806): Capacitor → the capacitor recipe overlay into CapArray and Capacitor. The dac_banks key (cellgen.rs:766) becomes (P net, model, w, l), so a bank never mixes recipes (H08-56, hastings.txt L25164–25181).
- Tests:
  - cap_array.rs (sky130, `mim_m3_1` overlay, `[1,1,2,4]`, W=L=5000):
    - `a_mim_bank_is_drc_and_erc_clean`.
    - `a_mim_bank_extracts_one_capacitor_per_unit`: extracted capacitor card count == 8.
    - `the_mim_unit_is_the_model_s`: every `Drawn` of kind Capacitor has w = l = 5000, and `c_u(overlay, w, l)` (the step-2 formula as `pub fn c_u_af(p: &dyn Process, w_nm: i32, l_nm: i32) -> Option<f64>`) = 53 282 aF ± 10; per member Σ = n·C_u.
  - capacitor.rs `a_merged_plate_is_the_units_area`: 4 units of 2000² → plate area 16·10⁶ nm².
  - Fixture `benchmarks/fixtures/dac4_mim.spice`: dac4 with `sky130_fd_pr__cap_mim_m3_1 W=5u L=5u`. `frontend/library/tests/drawn_cards.rs::a_mim_dac_signs_off_with_its_capacitors` → 0 `lvs/` rows, 16 capacitor cards.
- Acceptance: T3 and T4 for capacitors. dac4_mim compares all 16 units.
- Risks:
  - sky130 routes on met1+met2 only (AT-01), so the in-cell down-stack costs area.
  - The MIM bottom plate's parasitic belongs to PEX.
  - GPurify compares no capacitor value (CELL-01 step 7); the value guarantee is this item's `drawn` test, and PERF-16 calibrates PEX.

### CELL-09 BJT arrays: fixed-geometry emitters and drawn cards
- Status: done in M1 (`2a1ff0f`; cells merge `726bc13`).
- Priority: P0. Effort: S. Depends on: CELL-01, MAT-02. Cross: PERF-18 (sky130 BJT recogniser; sky130.deck:663-665 says "the bipolars … still wait"), EXT-19 (BJT sets).
- Why:
  - AC-08, CRATES #1, AV-03 (bgr_core's stated optimum), AV-23 (the bench appends `l=0.15u` to BJT cards).
  - The sky130 BJTs are fixed-geometry models: `pnp_05v5_W0p68L0p68`, `pnp_05v5_W3p40L3p40`, `npn_05v5_W1p00L1p00`, `npn_05v5_W1p00L2p00` (VOL/libs.ref/sky130_fd_pr/spice). A drawn emitter of any other size is a different device than the one simulated.
- Scope split: MAT-02 step 5 owns the array order (`pattern::grids`/`centro_assign(Fill::Balanced)` replace `unit_order`, bjt.rs:205-231), the `Bjt { rows, columns }` variant and the per-emitter `pnr_core::Unit` records, with the tests `every_single_odd_ratio_is_exact`, `equal_pair_is_a_diagonal_quad` and `every_bjt_ratio_is_common_centroid`. This item adds only the emitter size and the drawn cards. The earlier CELL-local `sym_order` is dropped as a duplicate of MAT-02.
- Current: emitter = `max(unit_w/l, bjt_min_emitter_side)` = max(w, 420) (bjt.rs:91-92), so bgr_core's `pnp_05v5_W3p40L3p40` gets a 420 nm emitter under the bench's `l=0.15u`.
- Change:
  1. Sidecar `cell.bjts` (new table, read by `pdk.recipe("bjt", model)`, pdk.rs:1129):
     ```json
     "bjts": { "recipes": {
       "pnp_3p40": { "model": "sky130_fd_pr__pnp_05v5_W3p40L3p40", "aliases": ["pnp_05v5_W3p40L3p40"], "bjt_emitter_w": 3400, "bjt_emitter_l": 3400 },
       "pnp_0p68": { "model": "sky130_fd_pr__pnp_05v5_W0p68L0p68", "aliases": ["pnp_05v5_W0p68L0p68"], "bjt_emitter_w": 680, "bjt_emitter_l": 680 },
       "npn_1x1":  { "model": "sky130_fd_pr__npn_05v5_W1p00L1p00", "aliases": ["npn_05v5_W1p00L1p00"], "bjt_emitter_w": 1000, "bjt_emitter_l": 1000 },
       "npn_1x2":  { "model": "sky130_fd_pr__npn_05v5_W1p00L2p00", "aliases": ["npn_05v5_W1p00L2p00"], "bjt_emitter_w": 1000, "bjt_emitter_l": 2000 } } }
     ```
     No `default`: a model no recipe names keeps today's sizing. The sizes are the model names' W and L. Register the four keys in FLOW-04's registry.
  2. bjt.rs:91-92: `let ew = match process.rule("bjt_emitter_w", 0) { 0 => s.unit_w.max(min_side), v => v };` and the same for `l`. A recipe size overrides the netlist w/l.
  3. cellgen draw_variants BJT arm (cellgen.rs:808): pass `Overlay { pdk, recipe }` when `pdk.recipe("bjt", model)` is Some, as the resistor arm does (cellgen.rs:797-800).
  4. In `Unit::draw` (bjt.rs:145-202), after MAT-02's `b.unit(…)`: `b.drawn(Drawn { owner, kind: if pnp { Pnp } else { Npn }, nodes: BJT_PINS.map(Node::Pin), w: ew, l: el, device: None })` (CELL-01 step 2 for the pin order).
  5. NPN keeps its per-unit dnwell (the foundry NPN is a fixed cell).
- Tests (bjt.rs, sky130 through the overlay):
  - `a_fixed_geometry_model_sets_the_emitter`: `pnp_05v5_W3p40L3p40` with netlist `w=0.15u l=0.15u` → the emitter diff rect of every unit is 3400 × 3400.
  - `every_emitter_is_a_drawn_card`: `dev_nf = [1, 8]` → `drawn.len() == 9`, all kind `Pnp`, w = l = 3400.
  - The existing `every_variant_is_drc_and_erc_clean` runs with the `pnp_3p40` overlay.
- Acceptance: T9 (bgr_core's Q1 unit at the 3×3 centre, from MAT-02). BJT LVS stays `lvs-coverage/unverified` until PERF-18 lands a recogniser; then the reference has 9 BJT cards for bgr_core and the compare runs.
- Risks: whether magic or GPurify extracts the generated construction as the foundry `pnp_05v5_W3p40L3p40` is not established (§5 Q6). The drawn size matches the model either way; the construction may still differ from the foundry cell.

### CELL-10 MOS: legal region parity for every multi-device row; ratioed mirrors merge
- Status: done in M1 (`878e222`; cells merge `726bc13`); cellgen declines a merge with no drawn variant. **Acceptance not met on `m1a`:** the bench draws mirror_ratio as 3 cells for 3 devices (expected devices − 2 = 1; m1-report §2); carried to M2 item 0.
- Priority: P0. Effort: S. Depends on: MAT-03 (`pattern::diffusion_cc_row`, which replaces `centroid_sequence` and the `greedy_centroid` branch of `finger_sequence`). Cross: EXT-15 (unit width, amendment below), FLOW-01 (W is SPICE total width).
- Why:
  - AC-05. H13-22: ratioed devices from identical units (hastings.txt L40850–40864, L42360–42368). H12-29: minimise drain (L36550–36564).
  - After MAT-03, `Cc1d` rows are diffusion-legal, but the region-parity rule itself is still implicit: multi-device rows start on D (mosfet.rs:324-328), so a `Single` (blocked) row of even counts puts every device boundary on a drain. Example: `[2,2]` Single → regions D S D | D S D, boundary region 2 is D, A's and B's drains share it; `shared_pads_carry_one_net` rejects it (cellgen.rs:138-141, 276) unless the drains share a net. No test states the rule.
- Scope split: the order algorithm (`diffusion_cc_row`), the `Cc1d` enumeration and the two-row half check are MAT-03's. The earlier CELL-local `pair_order`, `cc_exact_feasible` (MAT-02 has `cc_feasible`) and the source-ended mirror-pins order for unequal counts are dropped: mirror pins are defined by reflection swapping two equal devices (mosfet.rs:752-779), which unequal counts cannot satisfy.
- Change (mosfet.rs):
  1. ```rust
     /// Every inter-device boundary of `seq` (device index per finger) lies on a source region, with region 0
     /// a source iff `s0`. Region i sits left of finger i; region `seq.len()` is the right end.
     pub(crate) fn legal_row(seq: &[usize], s0: bool) -> bool {
         (1..seq.len()).all(|i| seq[i] == seq[i - 1] || ((i % 2 == 0) == s0))
     }
     ```
  2. `draw_row`: replace `multi = n_dev > 1 && !self.mirror_pins` (mosfet.rs:327) by `s0`: mirror pins → `true` (unchanged); `Chain` → unchanged (it restarts per member); otherwise `s0 = legal_row(seq, true)` (true for a `Single` row of even counts; `Cc1d` rows from MAT-03 give `false`, today's value). `is_s = |r| (r % 2 == 0) == s0`. `debug_assert!(chain || legal_row(seq, s0))`.
  3. `enumerate`: offer multi-device `Single` only when `legal_row(blocks, true) || legal_row(blocks, false)`; all-even counts pass with `s0 = true`, two all-odd devices with `false`; mixed parity is not offered.
  4. Request to EXT-15 (their step 1): when the set's class ≥ Moderate and `pattern::diffusion_cc_row(&parallel).is_none()`, halve `W_u` while `W_u/2 ≥ W_floor` and the counts stay integer. Without it `mirror_ratio` gives counts [1,2,4] (EXT-15's own `mirror_ratio_by_width` test), n = 7 is odd and no `Cc1d` row exists; with it W_u = 1000 (the Moderate floor) and counts [2,4,8].
- Tests (mosfet.rs):
  - `legal_row_matches_the_region_rule`: `[0,1,1,0]` true for `s0 = false`, false for `true`; `[0,0,1,1]` true for `true`, false for `false`; `[0,1]` true for `false`.
  - `a_blocked_even_row_puts_boundaries_on_sources`: NMOS `dev_nf = [2,2]` Single, distinct drain nets → the enumeration contains it, and no pad location carries two members' `D` pins.
  - `a_ratioed_mirror_merges`: NMOS `dev_nf = [2,4]`: the `Cc1d` variant (MAT-03: A BBBB A) has no pad location carrying two members' `D` pins, and `testkit::dirty_group::<Mosfet>` (G, S, B common: a mirror) is empty.
  - DRC/ERC and extraction sweeps add `[2,4]`, `[2,6]`, `[4,8]` and `[2,2]` Single.
  - Fixture `benchmarks/fixtures/mirror_ratio.spice` (format of `pair.spice`; SPICE total width, FLOW-01):
    ```
    * mirror_ratio — 1:2:4 NMOS mirror. Known optimum: one merged cell, centroid residual 0.
    .subckt mirror_ratio d1 d2 d3 VSS
    XM1 d1 d1 VSS VSS nfet_01v8 W=2u L=1u nf=2
    XM2 d2 d1 VSS VSS nfet_01v8 W=4u L=1u nf=4
    XM3 d3 d1 VSS VSS nfet_01v8 W=8u L=1u nf=8
    .ends mirror_ratio
    ``` Every finger is 1 µm wide, so the ratios are 1:2:4 in simulation and in layout under FLOW-01 (the earlier per-finger reading is gone).
- Acceptance: T6 for MOS. mirror_ratio draws the three devices in one cell (bench cells == devices − 2), CentroidGroup residual 0, LVS MATCH.
- Risks: groups with an odd count still draw as separate cells. That is deliberate: no Φ-balanced merged order exists for them (MAT-03 step 2.1).

### CELL-11 MOS: continuous gate bars at the deck's minimum contacted pitch
- Priority: P1. Effort: L. Depends on: CELL-02 (per-member gate labels), CELL-10 and MAT-03 (row orders). Cross: FLOW (`cellgen::folds` takes its aspect from `sd_and_pitch`, cellgen.rs:661, and benefits automatically), RTE (via landing at 430 nm).
- Why:
  - AC-04.
  - H13-03: pitch bounds P ≥ L_d + W_C + 2S_PC and P ≥ W_C + 2E_MC + S_M1 (Hastings eqs 13.7–13.9, hastings.txt L38423–38477).
  - LAMP-19: junction C is reduced by folding and sharing (lampaert.txt L2011–2062).
  - CRATES #8.
- Current:
  - `sd_and_pitch` returns pitch = max(sd_w + L + sd_w, m1_pitch) (mosfet.rs:657-669). On sky130 at L = 150, sd_w = max(250, 440 − 150, 170 + 0 + 80 + 170 − 150, 250) = 290, so pitch = 730 (AC-04; the 440 m1 term uses the sidecar `m1_enc` 65, sky130.json:5).
  - Each finger has its own pad `pad_w = gate_l.max(ct + 2·licon_poly_side + lat)` = 340 (mosfet.rs:344), pads at 391-402, plus a separate per-device poly strap of `poly_min_width` (428-431) and an li strap (427-436).
  - Deck minimum contacted pitch: licon.1 170 + 2·licon.11 55 + L = 430 (sky130.deck:331, 341).
- Change:
  1. `sd_and_pitch(process, gate_l) -> (i32, i32)`:
     - `sd_w = max(sd_width, ct + 2·gate_space, ct + 2·diff_enc, li_col + li_space − gate_l, m1_land − gate_l)`;
     - `li_col = ct + li_enc + li_side` (sky130 250);
     - `m1_land = mcon + 2·max(enclosure(met1,mcon), endcap(met1,mcon)) + space(met1)`, deck values only: 170 + 2·60 + 140 = 430 on sky130 (m1.2 from the metal loop at sky130.deck:360-366; m1.4 and m1.5 at 383-384);
     - `pitch = sd_w + gate_l`.
     
     sky130, L = 150: sd_w = max(250, 280, 250, 270, 280) = 280, pitch = 430 (*derived*). CELL-29 removes the sidecar `m1_enc` 65, which would otherwise raise `dim("m1_enc")` elsewhere.
  2. Gate connection, per device and pad row:
     - One poly bar of height `pad_h = ct + licon_poly_enc + licon_poly_side` spans the device's gates in that row. The bar is the strap; delete the separate `poly_w` strap (mosfet.rs:428-431).
     - Cuts go at every finger centre on the bar, with one exception: at a boundary between two devices that share a pad row, the right-hand boundary finger gets no cut. A variant in which some device then has no cut in a row is not enumerated.
     - One li rect covers the device's cuts in the row; delete the per-pad li rects.
     - An unsplit mirror-pins row (shared gate, mosfet.rs:421-424) gets one bar across the row.
  3. Two devices with distinct gates use split pad rows, top and bottom (existing `split_gates`). A blocked distinct-gate pair is enumerated only split. With n_dev > 2 and distinct gates, only `Single`, as now (`gate_straps_stay_private` rejects the rest, cellgen.rs:290).
  4. Grade gating (CELL-12):
     - `Exceptional` keeps today's isolated pads with an li strap. Hastings rule 22: metal gate straps, not a poly comb (hastings.txt L42644–42647). [corrected: rule 22 applies to moderately *and* exceptionally matched transistors, not only the most accurate class; PDF p.716 (printed 715) confirms.]
     - `Moderate` puts the bar ≥ 1000 nm from the diffusion. This follows the looser §13.2.2 text, not rule 22: a poly comb is acceptable "for all but the most accurately matched transistors" if the interconnecting poly lies ≥ 1–2 µm from the moat (L41125–41132; PDF p.693, printed 692, gives 1–2 µm). Rule 22 read strictly would give `Moderate` metal straps too.
  5. The `sd_edge` dummy clearance (mosfet.rs:279-284) keeps `poly_space + pad_over` using the bar overhang.
- Tests (mosfet.rs):
  - `the_contacted_pitch_is_the_deck_minimum`: `sd_and_pitch(&sky130, 150) == (280, 430)`.
  - `a_gate_bar_has_no_notch`: for each device and pad row, the poly shapes of that row union to one rectangle.
  - `a_row_is_shorter`: pair `nf=8`, Cc1d: gate-to-gate pitch == 430.
  - All existing tests pass: every variant DRC/ERC clean, extraction count = units + dummies, split gates private, dummies at pitch, two rows, mirror pins.
- Acceptance: T5. Bench area and active % reported before and after. Interior drain area per finger falls from 580·W to 280·W (*derived*).
- Risks:
  - Every MOS macro changes, so the bench is re-baselined (audit Q7).
  - npc strips per bar row come from `cover_poly_cuts` (builder.rs:63-112).
  - licon.14 keeps bar cuts ≥ 190 nm from diffusion through the existing stub rule (mosfet.rs:315-320).

### CELL-12 Matching class drives dummies, LOD moat, well clearance, gate overhang and gate straps
- Priority: P1. Effort: M. Depends on: MAT-07 (`MatchClass`, `class::mos_env`, the tier keys and the `lib.rs:871-872` switch), EXT-15/EXT-16 (the class on each emitted `Unitization`), CELL-11 (bar gating).
- Why:
  - AC-11, AC-12, AC-13, AC-22.
  - H13-42: classes MIN/MOD/EXC, hastings.txt L42333–42350.
  - H13-47: rule 12, L42532–42553.
  - H13-53: rule 19, L42625–42631.
  - H13-54: rules 21–22, L42638–42647.
  - H13-26: dummy equivalence, L41077–41093.
  - H13-29: moat; less accurate matching "two or three microns will suffice" (L41411–41422).
- Scope split: MAT-07 owns the class type, the Hastings table, `Process::tier`, the sidecar tier keys (`lod_moat_ext_nm`, `wpe_clearance_nm`, `dummy_reach_nm`, `gate_ext_extra_nm`), the deletion of `lod_moat_ext_moderate`/`wpe_clearance_moderate` and the `frontend/library/src/lib.rs:871-872` reader. FLOW-06 step 1-2 propose the same keys as per-class JSON objects; which encoding wins is FLOW/MAT's call, and CELL reads neither directly: it calls `class::mos_env`. The earlier CELL-local `MatchGrade`, `parse_roles` array change and key table are dropped as duplicates.
- Current:
  - `dummies_per_end` = `dummy_gates_per_end`, default 1 (mosfet.rs:54-56), drawn only when `dummy_required` (74).
  - The moat `lod_moat_ext_moderate` (3000) is added beyond the outer dummy's S/D, multi-device rows only (293-295).
  - The PMOS well halo `wpe_clearance_moderate` (3000) applies to matched groups (626-630).
  - Split gates: device 0's far end and device 1's near end carry only `poly_ext` (mosfet.rs:364-380). Dummy skirts are at the top only (516, 537-552).
- Change (mosfet.rs):
  1. Which rows get an environment: `env = u.class.map(|c| class::mos_env(c, kind, process))` where `u.class` is EXT-12's `Option<MatchClass>` (None for singletons and for parallel identical groups, EXT-15). MAT-07 step 5 instead makes the field non-optional with default `Moderate` at every construction site; under that encoding every singleton would get a 5 µm moat. Until the two plans agree, CELL treats `u.devices.len() == 1 || !u.route_matching_required && !u.dummy_required` as "no environment" (EXT-15 emits parallel groups with `route_matching_required = false`, and `dummy_required = class ≥ Moderate`). `kind` = the set's `MatchKind`, `Voltage` when absent.
  2. `draw_row`, with `e = env.unwrap_or_default()` (all zeros = today's unmatched behaviour):
     - `nd = max(if dummy_required { dummies_per_end } else { 0 }, ceil(e.dummy_reach_nm / pitch))`.
     - `moat = max(0, e.moat_nm − (sd_edge + nd·d_step))` added on both diffusion ends, so the diffusion end lies ≥ `moat_nm` past the outermost active gate edge. Replaces the `lod_moat_ext_moderate` read (mosfet.rs:293-295).
     - PMOS own-well halo = `e.wpe_nm` (MAT-07 already applies max(5000, 2·`n_well_depth`) for EXC; on sky130 2·2000 = 4000 < 5000, so 5000). Replaces mosfet.rs:626-630. The NMOS-to-foreign-well distance is PLC's (PLC-13).
     - Every active and dummy gate runs straight for `poly_ext + e.gate_ext_extra_nm` beyond the diffusion on both ends before any bar, pad or skirt begins, so split gates get equal overhang (fixes AC-11) and dummy skirts start past it.
     - Gate connection by class: EXC → isolated pads plus li strap (rule 22); MOD → bar ≥ 1000 nm from the diffusion (CELL-11 step 4); MIN and none → CELL-11's bar at minimum.
  3. `mosfet.rs:278, 291`: cite rule 12, not "§13.3 r9" (MAT-07 step 6 leaves these two to CELL).
- Tests (mosfet.rs; sky130 NMOS and PMOS pair, `nf=2`, L = 150, W = 1680, each class; expected values are MAT-07's table):
  - `class_scales_the_moat_and_dummies`: diffusion end − outermost active gate edge ≥ 3000 / 5000 / 10000 for MIN / MOD / EXC. For EXC, the outermost dummy's outer poly edge is ≥ 10000 from the nearest active gate.
  - `every_gate_overhangs_equally`: MOD and EXC, split-gate variants. Each gate and dummy poly rect has a straight extension ≥ `poly_ext + 1000` on both sides of the diffusion, equal across fingers within one lattice step.
  - `the_pmos_well_clears_by_class`: nwell edge to nearest active gate ≥ 2000 / 3000 / 5000.
  - `a_singleton_has_no_environment`: one NMOS, class field set to Moderate → bbox equal to today's (no moat, no halo).
  - DRC/ERC and extraction sweeps per class.
- Acceptance: T7. PERF-23's analog-rule check on the final GDS passes rules 12, 19 and 21 for every matched cell at its class; rule 22 is this item's gate-connection test.
- Risks: EXC is area-heavy: about 24 minimum-L dummies per end at 430 nm pitch, ≈ 10.3 µm (*derived*). Rule 12 prescribes the 10 µm reach, not the count, and names one full dummy with an 8–10 µm gate as a possibly better STI choice (hastings.txt L42544–42553, PDF p.715), which this plan does not draw (appendix).

### CELL-13 Every diffusion within reach of its own tap
- Priority: P1. Effort: S. Depends on: REL-06 (`tie_max_dist_nm` = 15 000 on sky130 with its source; LU.2/LU.3 deck rules). Cross: FLOW (`folds` finger-width clamp).
- Why:
  - AC-07, CRATES #5, AV-08.
  - H12-43: max distance to the nearest backgate contact 25–250 µm, per process (hastings.txt L37002–37008).
  - Foundry LU.2/LU.3: N-diff to P-tap and P-diff to N-tap < 15.0 µm (VOL/libs.tech/magic/sky130A.tech:4164-4170).
- Current:
  - One tap strip above the row (mosfet.rs:567-572).
  - `tie_max_dist_nm` = 3000 (sky130.json:85, no source) is required (pdk.rs:1068) and read by no generator.
  - Fingers go up to `max_finger_width` = 10 µm (sky130.json:40).
- Change:
  1. mosfet.rs: `fn tap_reach(p: &dyn Process, finger_w: i32, gate_l: i32) -> i32` = the distance from the diffusion's far (bottom) edge to the top strip's near edge, computed from the same offsets `draw_row` uses (`tap_y0`, mosfet.rs:567-572). The farthest diffusion point is a bottom corner, and the strip spans the full row, so this is the Euclidean maximum.
  2. `enumerate`: a variant whose `tap_reach(finger_w)` exceeds `process.rule("tie_max_dist_nm", i32::MAX)` is not offered (for two-row variants, each row to its own strip).
  3. `pub fn max_finger_for_taps(p: &dyn Process, gate_l: i32) -> i32` = `tie_max − (tap_reach(p, 0, gate_l))`, snapped down to the lattice; `tap_reach` is affine in `finger_w` with slope 1. FLOW's `folds` uses `min(max_finger_width, max_finger_for_taps)`.
  4. No second (bottom) strip: after REL-06 no shipped planar deck needs one (sky130 15 000, gf180mcu and ihp_sg13g2 20 000, against `max_finger_width` 10 000 plus the pad offsets). The bottom strip is in the appendix with its trigger.
- Tests (mosfet.rs):
  - `every_diffusion_point_reaches_its_tap`: every sky130 variant for W ∈ {420, 1680, 5000, 10000}, with the DRC sweep's n/nf sets. The max over diffusion-rect corners of the Euclidean distance to the nearest same-bulk tap rect is ≤ `tie_max_dist_nm`.
  - `max_finger_for_taps_is_tight`: with a test overlay setting `tie_max_dist_nm` = 3000, W = `max_finger_for_taps` has a variant and W + lattice has none.
- Acceptance: T8; REL-06's LU.2/LU.3 deck rules report 0 on every bench fixture.
- Risks: before REL-06 lands, the sidecar's 3000 would forbid fingers above about 3000 minus the pad offsets and force refolding; hence the dependency.

### CELL-14 Ratioed resistor arrays
- Priority: P1. Effort: M. Depends on: CELL-06 (`ResModel`), CELL-12 (class), MAT-12 (`pattern::segment_row`, which also deletes `greedy_centroid`), MAT-07 (`class::resistor_env`), EXT-15/EXT-19 (resistor sets, `Unitization.series`).
- Why:
  - AC-14, AC-15.
  - H08-10: equal segment length (hastings.txt L22207–22215).
  - H08-13: dummies by class, spacings equal to the active pitch (L22312–22347, L25263–25273).
  - H08-20: ratiometric jumpers (L22590–22632).
  - H08-33: electrical weights (L23277–23290).
  - H08-45: R11 thermoelectric cancellation (L25282–25290).
- Scope split: the order (`segment_row`, Table 8.5 golden tests) and the jumper metric (`jumper_spread`) are MAT-12's. The earlier CELL-local `palindrome` is dropped as a duplicate.
- Current:
  - The only interleave is `greedy_centroid` on equal counts (resistor.rs:304-310).
  - A multi-segment Interdig is dropped when devices outnumber met1 tracks (41).
  - Jumpers are li for adjacent columns and met1 with 2 mcon otherwise (137-155), so members get unequal jumper R.
  - Dummies sit at `d_pitch` ≠ segment pitch (170 vs 83); sky130 480 vs 400.
  - One dummy per end (162-192).
- Value rule (decides the earlier rdiv/T4 conflict): EXT-15 forms an L-ratio set from single schematic devices (`series_d = L_d / L_u`). Drawn as `series_d` identical units, member d has `series_d` head terms where the schematic device has one. On sky130 `res_high_po` at W = 0.69 µm: 2 × 10 µm units = 10 259.3 Ω against R(20 µm) = 9 736.2 Ω (+5.37 %), 4 × 10 µm = 20 518.6 Ω against R(40 µm) = 18 949.2 Ω (+8.28 %) (*derived* from the card, model.spice:27-40). So:
  - The series-unit array is offered only when every member passes `|series_d·ohm(W, L_u′) − ohm(W, L_d)| ≤ res_value_tol_ppm·ohm(W, L_d)`, with `L_u′ = seg_len(W, ohm(W, L_min)/series_min, 1, …)` (the unit sized so the smallest member is exact). With `ResModel` head = 0 and dl = 0 (sky130 `generic_po`) every member passes exactly.
  - Otherwise the group is drawn as blocks: each member one CELL-06 string at its own exact value, members side by side, no interleave. The value is right and the pattern is not common-centroid; MAT's ledger reports the residual.
  - Common-centroid `high_po` arrays at exact value need schematic unit chains (EXT-19 (a) chains of identical devices as one member); deferred (appendix).
- Change (resistor.rs):
  1. `series_parallel == Series` and the value rule passes: member d = `series_d` units of `unit_w × L_u′` in series; the `segments` axis is fixed to 1.
  2. Order: `pattern::segment_row(&series)` (MAT-12). Traversal alternates up/down per member, so Σφ_y = 0 for even counts (already per segment, resistor.rs:109-110).
  3. Jumpers: if any series join in the array is non-adjacent, every join of every member uses the met1-track construction with 2 mcon; one track per member, up to `jumper_tracks(process).len()`; more members than tracks → the variant is not offered. MAT-12's `jumper_spread` over the drawn joins is then 0.
  4. `seg_gap(process) = snap(max(res_seg_gap, space(li), space_between(rpoly, poly)))` → 480 on sky130 (poly.9, sky130.deck:284). Dummies sit exactly at the segment pitch (Hastings §8.3.1 r9, hastings.txt L25263–25273).
  5. Dummies per end from `class::resistor_env(class)`: `min_dummies` and `dummy_span_nm` (MAT-07: MIN 1 min-width, MOD 1 full-width, EXC span ≥ 10 000 nm, i.e. `ceil(10000/seg_pitch)` dummies). Tied through the `GND` pin (bound by `cellgen::bind_pins`, cellgen.rs:937).
  6. `Drawn` per unit as in CELL-06 step 6; unit weights = body area (all equal).
- Tests (resistor.rs):
  - `ratio_arrays_are_common_centroid` (sky130 `generic_po` overlay, head 0): counts [1,2], [2,3], [1,4], [3,2], [4,5], [9,4]. With ≤ 1 odd member, per member n_B·Σ_{u∈A} u.x == n_A·Σ_{u∈B} u.x (i64).
  - `high_po_ratios_fall_back_to_exact_blocks`: `high_po`, W = 690, members L = 10 µm and 40 µm with series [1,4] → no series-unit variant; the blocks variant's Σ ohm per member within 0.5 % of 5 129.7 Ω and 18 949.2 Ω.
  - `dummies_sit_at_the_segment_pitch`: sorted body and dummy poly x positions have one pitch.
  - `every_join_has_the_same_via_count`: 2 mcon per join, for all joins.
  - `each_member_s_current_directions_cancel`: Σφ_y == 0 per member with an even count.
  - Fixture `benchmarks/fixtures/rdiv.spice` (EXT-15's `resistor_divider` corpus case, {RA:1, RB:4}, unit L 10 µm):
    ```
    * rdiv — 1:4 poly divider. Known optimum: RB RB RA RB RB, centroid residual 0.
    .subckt rdiv a m b
    XRA a m res_generic_po W=0.69u L=10u
    XRB m b res_generic_po W=0.69u L=40u
    .ends rdiv
    ```
- Acceptance: T6 and T4 for resistor sets; rdiv drawn as one array with MAT's residual 0 and LVS MATCH (5 resistor cards from `Macro.drawn`).
- Risks: H08-07's mixed series-parallel small member (a 9:1 divider whose small resistor is 2p×2s, interdigitated as a 9:4 segment ratio, hastings.txt L23716–23721) needs a per-member topology field EXT does not emit; not planned.

### CELL-15 MOS rows from any legal order grid
- Priority: P2. Effort: M. Depends on: CELL-10, MAT-03 (base row), MAT-15 (`pattern::nth_order_rows`).
- Why:
  - CC-04: 2ⁿ units per device cancel gradients to order n; Table I 3rd order gives 0 / 0 / 0 / 0.01 / 0.068 % (nth_order.txt L90–119, L294–310).
  - CC-37 (cc_review.txt L445–464).
  - H13-41: 2-D arrays (hastings.txt L42255–42315).
  - NOTES-21.
- Current: rows ∈ {1, 2} (mosfet.rs:108-120). The second row is the first relabelled (pair) or reversed (152-156), mirrored about the tap strip (163-222). Orders are recomputed inside `draw` from `style` (141-147).
- Change:
  1. `Mosfet` gains `grid: Vec<Vec<u8>>` (rows × fingers of device indices), fixed in `enumerate`. `draw` renders it. `style` and `rows` stay as labels.
  2. `legal_row` (CELL-10) holds on every row with that row's `s0`, and Σφ per device over the whole grid is 0. Illegal grids from any source are dropped.
  3. Stacking: rows pair up on shared tap strips (`stack_on_tap`). Row pairs stack pad row to pad row, at the largest deck spacing among the layers on both facing edges (poly.2, li.3, npc.2, implants). Wells merge into one rect, as `stack_on_tap` already does (171-205).
  4. rows ∈ {1, 2, 4}. The 4-row grid is `pattern::nth_order_rows(3, &row)` with `row = pattern::diffusion_cc_row(&[c/4, c/4])` (MAT-15, MAT-03), offered for two-member sets when c (each member's count) is divisible by 8: P_n needs 2ⁿ units per device (CC-04, nth_order.txt L90–119), and the base row must hold an even count per member to be diffusion-legal. Pair `nf = 8` gives rows ABBA / BAAB / BAAB / ABBA (MAT-15's `order_three_is_nth_fig_3b`). MAT-15 also allows 8 rows; not offered (no fixture has 16 units per member).
- Tests (mosfet.rs):
  - `four_rows_cancel_to_third_order`: pair `nf=8` per device, 4-row grid. Integer unit moments M(a,b) = Σ x^a y^b with a + b ≤ 3 are equal between members.
  - `an_illegal_grid_is_not_drawn`: a row with two devices across a drain yields no variant.
  - The DRC/extraction sweep adds rows = 4.
- Acceptance: T6 order ≥ 3 for pairs with nf ≥ 8; MAT's metric reports order 3.

### CELL-16 MOS flavours: Vt markers from the model's recogniser
- Priority: P1. Effort: S. Depends on: FLOW-06 step 5 (`Pdk::model_markers`). Cross: FLOW-07 (parser model classification, AF-10), PERF (LVS model names).
- Why: H01-16 (LVMOS marker derivation, hastings.txt L11447–11458), H12-21 (NatVt layer around each gate, L35930–35936; example rules, Ex 12.11, L38125–38142), H01-34. An lvt device drawn without its marker extracts as `nfet_01v8`: the layout is a different device from the schematic, and LVS says so.
- Current:
  - The deck recognises `nfet_01v8_lvt` and `pfet_01v8_hvt` through the lvtn and hvtp layers (sky130.deck:542-550; rules lvtn.* 244-252, hvtp.* 234-240).
  - `DRAWN_ROLES` lists hvtp and lvtn (pdk.rs:1211-1212), so their two-layer rules already resolve through `widest_on`.
  - Mosfet draws no marker.
- Change:
  1. cellgen `draw_variants` MOS arm (cellgen.rs:796): `let marks = pdk.model_markers(&model).map_or(Vec::new(), |(req, _)| req)`, minus the layers the planar generator already draws (the layers of roles `diff, tap, poly, licon, li, mcon, met1, nwell, nsdm, psdm, npc`). If `marks` is non-empty, pass `Overlay { pdk, recipe: Recipe { model, layers: marks.enumerate().map(|(i, l)| (format!("gate_marker{i}"), l)), rules: vec![] } }`. The earlier sidecar `mosfets` recipe table is dropped: the deck's recogniser already names the markers.
  2. `draw_row`: for `i in 0..` while `process.layer(&format!("gate_marker{i}"))` is Some, draw one rect covering every gate of the row, dummies included, grown on each side by `max(enclosure(m, "poly"), enclosure(m, "diff"))` (lvtn.4b, hvtp.3: 180 nm), then raised to the marker's `width(m)` (380) and `area(m)` (0.265 µm²). `Overlay::enclosure` resolves `gate_marker{i}` through the recipe (pdk.rs:1180-1182); the base `Pdk::layer` returns `None` for that role.
  3. The marker's spacing to a different marker in a neighbouring cell is a placement constraint (PLC-07 edge profiles read drawn shapes); not handled here.
- Tests:
  - mosfet.rs `a_marker_covers_every_gate`: sky130 `nfet_01v8_lvt` overlay, pair `nf=4` with dummies → one lvtn rect per row containing every poly∩diff rect grown by 180.
  - `frontend/library/tests/flavours.rs::an_lvt_and_an_hvt_device_sign_off_with_their_models`: netlist with one `nfet_01v8_lvt` and one `pfet_01v8_hvt` → 0 `lvs/` rows and 0 `drc/` rows.
  - DRC/ERC sweep for lvt NMOS and hvt PMOS.
- Acceptance: the flavour fixture is LVS MATCH with the right models.
- Risks: 5 V thick-oxide devices (`*_g5v0d10v5`, marker `hvi`) are deferred (appendix): `hvi` is not a drawn role, and making it one through `recipe_layers` would make `diff and hvi`-style rules count for every cell (pdk.rs:775-790), which can widen every diffusion dimension.

### CELL-17 Electron-collecting ring: an n-well band, not a filled well
- Priority: P1. Effort: S. Depends on: GAP-05 (done in M1, `b1510f4`: `Tap { in_well } | Ecgr | Hcgr`, `drawable`, `in_nwell(Ecgr) == false`, `ring_gap` for `Ecgr` ≥ `space_between("nwell","diff")`), REL-07, REL-08 (which devices get an `Ecgr`).
- Why:
  - AC-06. H14-28: the CMOS ECGR is an N-well ring with n+ contact, around the NMOS injector (hastings.txt L43803–43837). H14-30: tied to a supply (L43650–43658).
  - Defect REL-07 does not fix: `tap_ring` draws, for any in-well ring, one nwell rect over the ring's full outer extent (post_cell.rs:276-282: `Rect { x: ox0 - enc, …, w: (ox1 - ox0) + 2*enc }`). For a `Tap { in_well: true }` around a PMOS that is right. Since GAP-05 (M1) `in_nwell(Ecgr)` is false, so an `Ecgr` draws n+ taps with no well at all, and `drawable(Ecgr)` returns false so nothing requests one. GAP-05's test `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos` was not written in M1; this item writes it.
- Scope split: REL owns which device gets which ring, its net and width, and the merge predicate. GAP-05 (M1) kept `Hcgr` behind `drawable(Hcgr) = retrograde_pwell` (false on sky130; no deck states a retrograde well, §14.2.4 L43842–43855). This item also sets `drawable(Ecgr)` to `p.layer("nwell").is_some() && p.layer("nsdm").is_some()` (GAP-05's spec; the M1 `ponytail:` in `drawable` names it). Concentric ring nesting stays dropped: one ring per device.
- Change (post_cell.rs):
  1. `tap_ring(…, well: WellShape)` with `enum WellShape { None, Filled, Band }` in place of the `in_well: bool` test at 276-282: `Tap { in_well: true }` → `Filled` (today's rect), `Tap { in_well: false }` → `None`, `Ecgr` → `Band`.
  2. `Band`: four nwell rects, each covering one side of the tap band grown by `e = max(dim("nwell_diff_enc"), enclosure("nwell","tap"))` (difftap.10, 180 nm) and at least `width("nwell")` wide (nwell.1, 840 nm on sky130; the band alone is narrower). Corners overlap so the four rects form one ring. The ring's inner nwell edge is the band's inner edge minus `e`; REL-07's `ring_gap` for `Ecgr` keeps that edge ≥ `space_between("nwell","diff")` (340) from the NMOS diffusion.
  3. `ring_halo` for `Ecgr` uses the band well's outer edge (`max(e, (width(nwell) − band)/2)` past the band) plus `outer_clear`.
- Tests (post_cell.rs, sky130):
  - `an_ecgr_well_never_covers_its_interior`: an `Ecgr` around a 2 µm × 2 µm NMOS cell → no nwell rect intersects the rect `inner` (the cell bbox grown by the NMOS's own spacing), every nwell rect is ≥ 840 wide, and `testkit` DRC/ERC is clean.
  - REL-07's `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos` passes.
- Acceptance: REL-08's injector fixtures carry `Ecgr` rings, DRC 0, ERC 0.

### CELL-18 Capacitor plate orientation by net impedance
- Priority: P1. Effort: S. Depends on: CELL-01, CELL-08. Uses EXT-18 net classes for the rank (today `Problem.net_classes`).
- Why:
  - H06-40: the high-parasitic plate goes to the low-impedance node (hastings.txt L20427–20433, L20725–20737).
  - H08-25: bottom plate to low-Z (L25540–25549).
  - SUB-47 (charbon_substrate.txt L2924–2981); H01-23.
- Current: P = top, N = bottom, by SPICE order (capacitor.rs:37). In the MIM the bottom (met3) plate carries the substrate parasitic.
- Change:
  - In `cellgen::bind_pins`, for single capacitors only (DAC banks already put the common high-Z top plate on top, cap_array.rs:12-17): `flip = rank(net(P)) < rank(net(N))`.
  - rank: Supply/Ground = 0 (from `Problem.net_classes`), other nets = 1, gate-only nets = 2. EXT may refine it.
  - A flipped device binds pin `d{k}:P` to the schematic N net and `d{k}:N` to P. The generator keeps "P = top/shielded, N = bottom/high-parasitic".
  - `Drawn` nodes resolve through pins, so LVS follows automatically.
- Tests: `frontend/library/tests/drawn_cards.rs::a_decoupling_cap_puts_its_bottom_plate_on_the_rail`: `C1 VDD g …` with g gate-only → pin `d0:N` (met3) on VDD after binding, and LVS clean.
- Acceptance: every capacitor with exactly one rail terminal has its bottom plate on the rail.

### CELL-19 Per-variant junction and gate-resistance figures for the post-layout deck
- Priority: P1. Effort: M. Depends on: CELL-01. Consumer: PERF-26 (reads `Figures.sd` and `Figures.gate_ohm` by these names, plan-07 §0.2).
- Why:
  - AC-21, AF-09 (the post-layout deck has no AS/AD/PS/PD: the sky130 FET subckt defaults them to 0, PERF-26 Current), BAL2-43.
  - LAMP-19: junction C from area and perimeter; folding and sharing reduce it (lampaert.txt L2011–2062).
  - H06-39: gate R lumps to 1/3 (one end) or 1/12 (both ends) of the finger's poly R (hastings.txt L20262–20275).
  - Diffusion sharing (CELL-10/11) is where the generator beats a hand layout on every device, and without these figures the simulator cannot see it.
- Current: `Cell` has only `enumerate` and `draw` (lib.rs:45-52). No per-terminal diffusion geometry leaves the generator.
- Change: fill `Figures` (declared empty in CELL-01):
  ```rust
  #[derive(Clone, PartialEq, Debug, Default)]
  pub struct Figures {
      /// (owner, terminal "S"/"D", area nm², edge perimeter nm): BSIM AS/AD, PS/PD. A region shared by two
      /// fingers counts half to each; gate-side edges are excluded; dummy-only moat is not counted.
      pub sd: Vec<(u8, &'static str, i64, i64)>,
      /// (owner, Ω): (R□_poly·W_f/(k·L) + R_cut)/N_f, k = 3 one-ended, 12 two-ended (mosfet.rs:44-47, 58-66).
      pub gate_ohm: Vec<(u8, f32)>,
  }
  ```
  - `draw_row` knows every region's width and owner (mosfet.rs:458-482); it pushes one `sd` entry per (owner, terminal), summed over the row.
  - For sky130, `R□_poly` = 48.2 Ω/□ (pex `poly_c`, sky130.deck:582) and `R_cut` = 152 Ω (pex `licon_po`, sky130.deck:587), both through `Process::sheet_ohm` (mosfet.rs:62).
  - Other generators leave `Figures` empty.
  - Dropped from the earlier draft (appendix): `value` (tests compute values from `Macro.drawn` with `ResModel`/`c_u_af`), `max_tap_dist_nm` (REL-06's deck rules check the final GDS), `centroid_offset_nm` (MAT-01 computes moments from `Macro.units`), and the cap-array `ArrayMetrics` (MAT-11 owns it; `pnr_core` cannot name a `cells` type anyway).
- Tests (mosfet.rs):
  - `sd_figures_add_up`: single device nf=1, W = 1680 → AS = AD = `sd_edge`·1680. Pair Cc1d nf=2 → Σ AS + AD over members == total active S/D diffusion area (Σ of the region rects' areas, dummies excluded).
  - `a_shared_drain_halves_the_area`: single device nf=2 → AD = one interior region's area, AS = 2 × one edge region's area.
  - `gate_ohm_matches_the_formula`: nf=4, W = 10000, L = 150, one-ended → (48.2·10000/(3·150) + 152)/4 = 305.8 Ω ± 1 % (*derived*).
- Acceptance: PERF-26's post-layout deck carries AS/AD/PS/PD from `Figures`.

### CELL-20 Butted source–tap variant
- Priority: P2. Effort: M. Depends on: CELL-13, CELL-19.
- Why: H01-09 (abut the PSD substrate contact to an NSD source only at the same potential, hastings.txt L9926–9929). H12-30 (butting backgate contacts when backgate = source; odd sections one end, even sections with minimised drain both ends, L36568–36574). H14-24.
- Current: bulk is only reachable through the separate strip (mosfet.rs:567-572). No butted form exists.
- Change:
  - Variant field `tap: TapStyle { Strip, Butted }`. `Butted` is offered when every row's outer regions are sources: `Source` convention (even counts), or a single device with even nf.
  - The outer S diffusion continues into a tap of the bulk implant. The implants split at the shared edge.
  - Cut columns stay separate: licon.6 forbids a cut straddling tap and diff (sky130.deck:605).
  - `Butted` is enumerated only if CELL-13's reach holds, i.e. the row length ≤ 2·`tie_max_dist_nm`. Otherwise `Strip` and `Butted` are combined.
  - New cellgen filter `butted_taps_share_the_source(m)` next to `shared_pads_carry_one_net` (cellgen.rs:276): keep `Butted` only when the bound nets of `d{k}:S` and `d{k}:B` are equal.
- Tests:
  - `a_butted_row_is_clean` (DRC/ERC).
  - `butted_needs_source_on_bulk`: the filter rejects S ≠ B.
  - Butted bbox height < Strip bbox height.
- Acceptance: bench area falls for source-on-bulk devices, with no LVS or DRC regression.

### CELL-21 Split DAC and non-unit capacitors (drawing only)
- Priority: P2. Effort: M. Depends on: CELL-08, MAT-18 (`dac::{attenuation_cap, nonunit_dims, split_dac_assign}`), EXT-19 (split-DAC set with `bridge`; corpus case `splitdac`). Gate: EXT-19's `splitdac` corpus netlist exists and EXT emits its set.
- Why: CC-16 (C_A = (C_T^LSB/C_T^MSB)·C_u, cc_dac_constructive.txt L60–76), CC-17 (non-unit sizing with equal edge sensitivity, L534–616), CC-23 (split placement, L848–878), H08-11 (non-integer caps with equal A/P, hastings.txt L22218–22272), AC-16, NOTES-31.
- Current: `bits()` accepts only the single-bank form [1,1,2,…] (cap_array.rs:123-130). No non-unit plate.
- Scope split: slot order and C_A sizing are MAT-18's; this item draws them. The earlier CELL-local quadratic is dropped (`nonunit_dims` is the same formula).
- Change (cap_array.rs):
  - `Pattern::Split { lsb: u8 }`, enumerated when the group carries EXT-19's bridge member.
  - Slots from `dac::split_dac_assign(l_bits, m_bits, rows, cols)`; the two C_A half-slots are drawn with the non-unit plate from `dac::nonunit_dims(area, unit_w, unit_l)`, snapped up to the lattice.
  - For the MIM recipe the target area comes from C: solve `c_area·(W+dw)(L+dw) + c_perim·2(W+L+2dw) = C_A` with W/L fixed at the `nonunit_dims` aspect (one quadratic in the scale factor).
  - LSB and MSB top plates stay separate nets; C_A's plates bind to them.
- Tests: `a_split_bank_is_point_symmetric` (L = M = 3, every even-count cap closed under 180° rotation); `the_bridge_keeps_the_unit_edge_sensitivity` (|(W+L)/(W·L) − (W_u+L_u)/(W_u·L_u)| ≤ 2·lat/(W·L) after snapping); DRC/ERC clean on sky130 with `mim_m3_1`.
- Acceptance: `splitdac` signs off LVS clean with C_A within one lattice step of MAT-18's value, and MAT's ledger reports exact CC for every even-count cap.

### CELL-22 Well bridges for unequal spans
- Priority: P2. Effort: S. Depends on: REL-16 (`CellFlags`, `may_share_well`, and the `flags: &[CellFlags]` parameter it adds to `well_bridges`).
- Why: H01-05 (same-bulk PMOS may share one well, hastings.txt L10090–10094). A gap between two same-net wells narrower than the well spacing rule is either a DRC error or wasted area; merging it is what a hand layout does. AC §2.7 records the identical-span limit.
- Current: a bridge is drawn only when the facing well spans are identical (post_cell.rs:424-430).
- Change (post_cell.rs, inside REL-16's predicate-filtered loop):
  - Bridge the overlap interval of the facing spans (non-empty overlap required).
  - Grow the bridge to the rectangle hull of the two facing edges when the hull meets no blocker (the existing `hits`, 406-410); otherwise keep the overlap bridge.
- Tests: `offset_wells_bridge_over_their_overlap` (two PMOS cells, facing spans [0, 4000] and [1000, 6000], gap 600 nm → one bridge rect y ∈ [1000, 4000] without a blocker; hull [0, 6000] when no blocker is present, and DRC clean either way); REL-16's `an_injector_well_is_not_bridged_to_a_victim` still passes.
- Acceptance: the count of nwell figures per bench circuit falls or stays equal, with no DRC change.

### CELL-23 Resistor width floor by class, value-preserving
- Priority: P2. Effort: S. Depends on: CELL-06, CELL-12, MAT-07 (`class::resistor_env(c).width_floor_x`). Cross: REL-13 (self-heating width floor; REL owns the formula and the current input).
- Why:
  - H08-41: width ≥ 150 / 200 / 400 % of minimum for MIN/MOD/EXC; length 3× / 5× / 10× (hastings.txt L25205–25214, L25274–25281).
  - H06-07 (L16598–16609).
  - REL-13 sets `unit_w = max(unit_w, res_min_width, w_sh)` in cellgen, which changes R unless L follows. CELL-06's `seg_len` makes both widenings value-preserving.
- Current: `body_w = unit_w.max(res_min_width)` only (resistor.rs:81).
- Change (resistor.rs):
  - `W' = snap_up(max(W, ceil(width_floor_x · res_min_width)), widths)`, where `widths` is the recipe's allowed widths when given, else the lattice.
  - sky130 `high_po` widths: add `"res_widths_nm": [350, 690, 1410, 2850, 5730]` to the recipe, sourced from the foundry model files `sky130_fd_pr__res_high_po_{0p35,0p69,1p41,2p85,5p73}.model.spice` (VOL/libs.ref/sky130_fd_pr/spice). `Pdk::recipe` keeps only integer values (pdk.rs:1142), so it also flattens an integer array `k` into `k[0]`, `k[1]`, …; the generator reads `rule("res_widths_nm[i]", 0)` until 0.
  - `L' = seg_len(W', model.ohm(W, L), segments, …)` (CELL-06), so R is unchanged within `res_value_tol_ppm`. The length floor (3×/5×/10×) then holds automatically for the widths the table gives; the generator asserts it and drops the variant otherwise.
  - Applied to members of a matched set only (the CELL-12 environment rule); singletons keep today's width.
  - The drawn cards carry W' and L'.
- Tests: `a_moderate_resistor_widens_at_the_same_value`: `high_po`, W = 690, L = 20 µm, class MOD → W' = 1410 (2 × 500 = 1000, snapped up to the next foundry width), Σ ohm within 0.5 % of R(0.69, 20) = 9 736.2 Ω.
- Acceptance: CELL-12's class sweep plus this test; T4 unchanged.
- Risks: the rpm.1b–1f exact-width rules are not in the deck (sky130.deck:653), so the DRC does not enforce the width set; the model files are the only source (§5 Q4).

### CELL-24 Shield plates over matched capacitor and resistor arrays — deferred (see appendix)

### CELL-25 FinFET generator parity
- Priority: P2. Effort: L. Depends on: CELL-10, MAT-03. Gate: PERF keeps ASAP7 competitor benchmarks (AV-13); every MOS on generic_finfet goes through this generator (cellgen.rs:795), so T1 for fin decks depends on it.
- Why: AC-09; CC-42 (FinFET orientation balancing, cc_review.txt L408–423); CC-35 (SA/SB equality, L103–117).
- Current: one variant (finfet.rs:89-94); members on separate actives (139-194); `sa = sb = 0` (186); `nfin = unit_w/fin_p` floors silently (114); no tests.
- Change:
  - One diffusion-edge dummy gate per end.
  - SA/SB from each gate to its active's ends.
  - `nfin = round(unit_w / fin_p)`, at least 1 (was a silent floor, finfet.rs:114); the drawn width `nfin·fin_p` is what `Unit.weight` and the LVS card carry.
  - `Cc1d` members share one active using `pattern::diffusion_cc_row` (MAT-03) where the deck allows diffusion sharing between gates.
  - finfet.rs tests: DRC/ERC on generic_finfet (release; engine debug assertion noted at cell_selfcheck.rs:278-282), SA/SB non-zero, extraction count.
- Acceptance: T1 on generic_finfet for pairs; T6 for pairs.

### CELL-26 MOS-capacitor generator (gf180) — deferred (see appendix)

### CELL-27 In-cell drain straps for matched pairs — deferred (see appendix)

### CELL-28 Inductor spiral generator — deferred (see appendix)

### CELL-29 Hygiene: literals, citations, dead generator keys
- Priority: P2. Effort: S. Depends on: CELL-04, CELL-12, FLOW-04 step 5 (the `REQUIRED_RULES` → registry change).
- Why: AC-25, AC-26, AC-28, AV-22, H06-05 (dead keys).
- Change:
  - Code:
    - `skirt_y = finger_w + poly_ext - 10` → `- lat` (mosfet.rs:537).
    - Fix the "40 nm" comment (mosfet.rs:560).
    - `2 * r.sel_enc - 0` → `2 * r.sel_enc` (finfet.rs:193).
    - Rename `_constraints`/`_process` in `Bjt::enumerate` (bjt.rs:27-37) to `constraints`/`process` (both are used, bjt.rs:34-36).
    - Move the `pin` doc off `unit` (builder.rs:30-31).
    - Delete the no-op `|| true` filter (post_cell.rs:483).
    - `ring_gap` drops its unused `width` (post_cell.rs:299-302), unless REL-07's rewrite of `ring_gap` has already removed it.
    - resistor.rs `Pattern::Interdig` (AC-26): after CELL-14 the resistor variant is `{ segments, series_array: bool }`; delete the shared `Pattern` use there (the diode's goes in CELL-07).
  - Citation: mosfet.rs:44-47 cites Hastings rule 22 for gate R; rule 22 is metal gate straps (hastings.txt L42644–42647); cite H06-39 (L20262–20275) instead. (mosfet.rs:278 and 291 are fixed in CELL-12.)
  - Sidecar keys the generators own: delete `m1_enc` (sky130.json:5): it has no source, and `dim` takes `max(rule, deck)` (builder.rs:195-211), so 65 overrides the deck's m1.5 = 60 and raises today's MOS pitch from 710 to 730 (*derived*). Delete the unread `res_corner_squares`, `res_serpentine_aspect`, `bjt_max_emitter_stripe`, `bjt_base_frac`, `bjt_collector_frac`, `bjt_stripe_gap`, `bjt_base_frac_permille`, `bjt_collector_frac_permille` (sky130.json:47-48, 51, 53-55, 76-77) from all four sidecars once FLOW-04 step 5 has made them `required: false`. Keep `bjt_min_emitter_side` (sky130.json:52), which is read (bjt.rs:91, 234).
  - cellgen.rs:846-848 comment ("sky130 has none") → capm exists (sky130.deck:565), drawn since CELL-08.
  - Not here (FLOW-15 owns them): the `cell/undrawable` naming (lib.rs:1295-1297, FLOW-15 step 1), docs/CRATES.md and LAYOUT-FUNDAMENTALS edits (FLOW-15 step 5), the dead `Unitization` fields (FLOW-15 step 6).
- Tests: none new; the whole suite stays green. `sd_and_pitch(&sky130, 150)` still returns (280, 430) after `m1_enc` goes (CELL-11's `m1_land` is deck-only).
- Acceptance: `grep -rn` for each removed key over `*.rs` and `pdks/*.json` returns nothing.

### CELL-30 MOS dummy gates capped at the microloading reach; long-L and narrow devices in the generator sweeps (added at the M0 close-out)

Status: done in M1 (`79f1a6a`; cells merge `726bc13`).

Field report: FR-5 (and the generator half of FR-4).

- Priority: P0 (a matched long-L device does not fit the die it was given). Effort: S. Depends on: none. CELL-12 later
  sets the dummy *count* by class; this item sets the dummy *length*.
- Why:
  - FR-5: "A 0.42/64.8 µm PFET becomes a ~196 µm cell … width looks like ~3·L" (field-report-01).
  - H13-26 (ref-hastings-13 §3, from hastings.txt L41077–41104, PDF p.692): poly microloading reaches 3–5 µm, so a
    dummy gate may be capped at 3–5 µm long unless the active L is shorter (then dummy L = active L); dummy width,
    poly extension and dummy-to-active spacing equal the active ones.
  - FR-4 (generator half): the MOS sweeps draw one size only. `cell_selfcheck`'s `group_of` uses `unit_w: 1680,
    unit_l: 150` (kernel/cells/tests/cell_selfcheck.rs), so no test draws a long-L or a 0.42 µm-wide device.
- Current (measured on m0 with a throwaway example: one sky130 PMOS through `Mosfet::enumerate` and `draw`, `nf = 1`,
  `Single`, DRC by `verify::drc`; not committed):
  - Every dummy gate is drawn at the active `gate_l`: `b.rect(poly, Rect { x: dx, .., w: gate_l, .. })`
    (kernel/cells/src/mosfet.rs:516), listed as `Dummy { w: finger_w, l: gate_l }` (:525–531), stepped by
    `d_step = gate_l + sd_end` (:285), left-edge positions `-(k + 1) * gate_l - k * sd_end` (:513, :547).
  - With `dummy_required` (set by the annotator for matched sets) and `dummy_gates_per_end` = 1, bbox widths are:

    | W / L (µm) | no dummies (nm) | with dummies (nm) |
    |---|---|---|
    | 0.42 / 64.8 | 65 740 | 196 340 |
    | 0.42 / 16.2 | 17 140 | 50 540 |
    | 2.16 / 9.6 | 10 540 | 30 740 |
    | 0.42 / 1.0 | 1 940 | 4 940 |

    Each dummy adds `gate_l + 500` nm. The 196 340 nm row is FR-5's cell.
  - All of these variants are DRC-clean on sky130 (no `poly` min-extension finding, no `li.3`). FR-4's generator rows
    came from the old packaged build and do not reproduce on m0 (rescale's four 2.16/9.6 µm PFETs and ptat_bias sign
    off CLEAN through the m0 CLI).
- Change:
  1. Sidecar key `dummy_max_l_nm`: `Nm`, `required: false`, sourced, reader `kernel/cells/src/mosfet.rs`, registered in
     `verify::sidecar::KEYS` (FLOW-04). Values: sky130, gf180mcu and ihp_sg13g2 3000, with `_source` "microloading
     reach 3–5 µm, hastings.txt L41097–41104 (H13-26); Philis policy: the lower end"; generic_finfet null.
  2. `mosfet.rs` draw: `let dummy_l = match process.rule("dummy_max_l_nm", 0) { cap if cap > 0 && gate_l > cap =>
     cap, _ => gate_l };`. The dummy poly rect (:516), the dummy gate cut centre `cx` (:517), `Dummy.l` (:530),
     `d_step` (:285) and the dummy left edges (:513, :547) use `dummy_l`. The spacing from the outer active gate stays
     `sd_edge` (dummy-to-active = active-to-active, H13-26). `pad_over`/`skirt_over` (:274–275) are evaluated with
     `dummy_l` for the dummy gates.
  3. LVS needs no change: `cellgen::dummy_cards` writes each dummy card's `l` from `Dummy.l`
     (frontend/library/src/cellgen.rs:1007).
- Tests:
  - `mosfet.rs` `a_long_gate_keeps_short_dummies`: sky130 PMOS, W = 420, L = 64 800, `dummy_required` →
    `bbox.w ≤ 65 740 + 2·(3 000 + 500) = 72 740` nm (*derived* from the measured per-dummy step), every
    `Dummy.l == 3000`, DRC and ERC clean, extracted MOS count = fingers + dummies.
  - `mosfet.rs` `a_short_gate_keeps_full_dummies`: L = 150 → every `Dummy.l == 150`, bbox unchanged from today.
  - `every_variant_is_drc_and_erc_clean` and `every_variant_extracts_its_fingers_and_dummies` (mosfet.rs:812, :837)
    also sweep (W, L) ∈ {(420, 150), (420, 1 000), (420, 16 200), (420, 64 800), (2 160, 9 600)} for NMOS and PMOS,
    with and without dummies (the FR-4 guard).
- Acceptance: FR-5's device drawn matched is ≤ 73 µm wide; bench GDS unchanged on the 10 fixtures (every fixture
  L ≤ 2 µm < 3 µm).
- Risks / notes: H13-26 also allows half dummies (one S/D termination) for long L; not drawn. A deck whose poly
  min-area or end-cap rules need a longer dummy is caught by the widened sweep.

---

## 4. Milestones

| Milestone | Items (external prerequisites) | Exit criteria (all measurable) |
|---|---|---|
| M0 Honest foundations | CELL-01, 02, 03, 04 (none) | `Macro` carries `drawn`/`keepouts`/`figures`; `every_generator_enumerates_on_every_deck` green in debug on 4 decks; `every_half_extent_is_on_the_cut_lattice` green; 0 inductor shapes in any output and one `cell/undrawable` row for an `L` device; bench GDS unchanged except bbox rounding (≤ 10 nm per axis per macro) |
| M1 Value closure | CELL-06, 07, 08, 09, 10, CELL-30 (M0 close-out, FR-5) (MAT-02, MAT-03, FLOW-06 step 4) | rc_filter, res_m2 (every segment count), dac4_mim (16 capacitor cards) LVS MATCH; `segments_preserve_the_model_value` (≤ 0.5 %); MIM unit 53 282 aF ± 10; diode Σ area = n·W·L; bgr_core Q1 at the 3×3 centre with 9 drawn BJT cards; mirror_ratio in one cell, CentroidGroup residual 0, LVS MATCH |
| M2 Density, class, latch-up | CELL-11, 12, 13, 19 (MAT-07, EXT-15/16 class, REL-06, PERF-26 consumer) | `sd_and_pitch(sky130,150) == (280,430)`; class tests (moat 3000/5000/10000, WPE 2000/3000/5000, overhang +1000) green; `every_diffusion_point_reaches_its_tap` green; `gate_ohm` 305.8 Ω ± 1 %; bench area and active % before/after table published |
| M3 Passive arrays, flavours, rings | CELL-14, 16, 17, 18 (MAT-12, FLOW-06 step 5, REL-07, REL-08) | rdiv drawn RB RB RA RB RB with MAT residual 0 and LVS MATCH; `high_po_ratios_fall_back_to_exact_blocks` green; lvt + hvt fixture LVS MATCH, DRC 0; `an_ecgr_well_never_covers_its_interior` and REL-07's ECGR test green; every single capacitor with one rail terminal has its bottom plate on the rail |
| M4 Extensions | CELL-15, 20, 21, 22, 23, 25, 29 (MAT-15, MAT-18 + EXT-19 `splitdac`, REL-16) | each item's named tests green: `four_rows_cancel_to_third_order`; butted bbox height < strip bbox height; `splitdac` LVS MATCH; nwell figure count per bench circuit ≤ before; `a_moderate_resistor_widens_at_the_same_value`; generic_finfet pair DRC 0; removed-key grep empty |
---

## 5. Open questions

1. **Value of `tie_max_dist_nm` on sky130.** Resolved: REL-06 sets 15 000 with the LU.2/LU.3 source (VOL/libs.tech/magic/sky130A.tech:4164-4170); CELL-13 depends on REL-06 and reads the key.
2. **Do GPurify's resistor, MIM and diode recognisers extract W/L params?** Resolved: no. W/L are measured for MOS only; every device gets `area` (GP/crates/check/src/topology/device.rs:295-323). Non-MOS drawn cards carry no params (CELL-01 step 7); value fidelity is CELL-06/08's generator tests. Comparing `area` needs `area` on every MOS and dummy card too (appendix).
3. **Named internal nets in the LVS reference.** GPurify refuses series merges across named nodes (`GP/crates/check/src/lvs/reduce.rs:271-273`). Whether refinement anchors on names is not established. Default: `~c.o.k` names; if `a_segmented_resistor_signs_off_lvs_clean` fails, emit unnamed internal nets or request that from GPurify.
4. **sky130 `res_high_po` allowed widths.** rpm.1b–1f "exact resistor widths" are not in the deck (sky130.deck:653). The foundry ships per-width models `res_high_po_{0p35,0p69,1p41,2p85,5p73}` (VOL/libs.ref/sky130_fd_pr/spice), so the width set is {350, 690, 1410, 2850, 5730} nm; CELL-23 snaps widened resistors onto it. Netlist widths are drawn as given (the designer picked the model).
5. **generic_po `dw`.** The model's `dw = -tol_poly/2 - poly_dw/2` evaluates to +0.028 µm at typical (VOL/libs.tech/ngspice/sky130_fd_pr__model__r+c.model.spice:78, :134; tol_poly = 0 at r+c/res_typical__cap_typical.spice:6). [UNVERIFIED: how ngspice's resistor model applies `dw` to the drawn W (W − dw or W − 2·dw) was not checked.] Default: 0, so the value model is exact only for high_po until FLOW reads the corner file.
6. **sky130 BJTs.** The deck has no recogniser (sky130.deck:663-665), and whether magic extracts the generated construction as `pnp_05v5_W3p40L3p40` is not established. Default: draw the fixed emitter sizes (CELL-09) and have PERF report BJT LVS as unverified. The alternative, importing the foundry cells through `macros`, belongs to FLOW.
7. **Owner and encoding of the class.** Three plans define `MatchClass`: EXT-12 (`analog::intent`, `Unitization.class: Option<MatchClass>`), MAT-07 (`analog::matching::class`, non-optional, default `Moderate`, tiers as JSON arrays read by `Process::tier(name, idx)`) and FLOW-06 (`pnr_core::process`, tiers as per-class JSON objects read by `Process::tier(key, class)`). CELL no longer defines one and reads only `class::mos_env`/`resistor_env` plus the Option (CELL-12 step 1 gives the interim rule). The three owners must pick one; CELL's code is unaffected by which.
8. **Drop the sidecar `m1_enc` 65?** Default: yes (CELL-29). The deck's m1.5 = 60 governs. [corrected: with CELL-11's deck-only `m1_land` the pitch is 430 either way; keeping the key gives 440 only if `m1_land` read `dim("m1_enc")`. Today (mosfet.rs:662-668) the key raises the pitch from 710 to 730 (*derived*: m1_pitch 430 vs 440).]
9. **EXC area.** Rule 12's 10 µm reach at 430 nm pitch means about 24 minimum-L dummies per end (rule 12 also allows one full dummy with an 8–10 µm gate, hastings.txt L42544–42553). Default: accept; EXC is assigned only by EXT or the user.
10. **ECGR width for sky130's bulk substrate.** REL-07 owns it (`max(min_ring_width, n_well_depth_nm)`, marked [UNVERIFIED] there); CELL-17 draws the requested width.
11. **MOM density for `cap_generic_m1m2`.** 0.105 fF/µm² is what GPurify's PEX extracts (AV-32), and that PEX is a parallel-plate lower bound without inter-layer fringe (AV-07), so it is not a calibrated value. Default: no `c_area_af_um2` on the MOM recipe, so C is unknown and reported by PERF until calibrated.


---

## Cut or deferred items

| Item | Status | Reason | Re-entry condition |
|---|---|---|---|
| CELL-05 Guard-ring rename (`Tap{in_well}`, `Ecgr`, `Hcgr`) | Cut (duplicate) | REL-07 step 1-2 replaces the same enum with `PTap | NTap | Ecgr`, deletes `Hcgr/Hbgr/Ebgr` and the two dead fields, and rewrites `implant_name`/`in_nwell` in the same diff | — |
| CELL-09 `sym_order` point-symmetric assigner and its BJT order tests | Cut (duplicate) | MAT-02 `pattern::grids`/`centro_assign(Fill::Balanced)` and MAT-02 step 5 (bjt.rs order and `Unit` records) | — |
| CELL-10 `pair_order`, `cc_exact_feasible`, source-ended mirror-pins order for unequal counts | Cut (duplicate / ill-posed) | MAT-03 `diffusion_cc_row` covers the drain-ended order; MAT-02 has `cc_feasible`; mirror pins are defined for two equal devices only (mosfet.rs:752-779) | — |
| CELL-12 `MatchGrade`, `parse_roles` array flattening, tier key table, `lib.rs:871-872` switch | Cut (duplicate) | MAT-07 steps 1-6 (and FLOW-06 steps 1-2) | — |
| CELL-13 second (bottom) tap strip | Deferred | After REL-06 no shipped planar deck needs it: sky130 15 000 nm, gf180mcu and ihp_sg13g2 20 000 nm, against `max_finger_width` 10 000 nm plus pad offsets | A deck with `tie_max_dist_nm` < `max_finger_width` + the pad offset, or a `max_finger_width` increase; `max_finger_for_taps_is_tight` shows the need |
| CELL-14 `palindrome` interleave | Cut (duplicate) | MAT-12 `segment_row` (with Table 8.5 golden tests) | — |
| CELL-14 common-centroid `high_po` arrays at exact value | Deferred | Series units add one head term per unit (+5.37 % for 2 × 10 µm vs 20 µm, *derived*); exact values need members that are schematic chains of identical devices, which EXT-15's `Unitization` (one device per member) cannot express | EXT emits chain members (EXT-19 (a) chains) as one Unitization member |
| CELL-15 8-row grids | Deferred | No fixture has 16 units per member | A fixture or user constraint asks for order ≥ 4 |
| CELL-16 5 V thick-oxide devices (`hvi` marker) | Deferred | `hvi` is not a drawn role; adding it through `recipe_layers` would make `X and hvi` rules count for every cell (pdk.rs:775-790) | `widest_on` takes the overlay's recipe layers instead of the global set, so a marker counts only inside its own recipe |
| CELL-17 `Hcgr`, `drawable()`, concentric ring nesting | Cut | REL-07 deletes `Hcgr` (no deck states a retrograde well, Hastings §14.2.4 L43842–43855) and allows one ring per device | REL asks for a second ring kind per device |
| CELL-19 `Figures.value`, `max_tap_dist_nm`, `centroid_offset_nm`, `ArrayMetrics` on `Figures` | Cut (no consumer / duplicate) | Tests compute values from `Macro.drawn`; LU is checked on the final GDS by REL-06's deck rules; MAT-01 computes moments from `Macro.units`; MAT-11 owns `ArrayMetrics`, and `pnr_core` cannot name a `cells` type | A consumer outside tests appears |
| CELL-21 local non-unit quadratic | Cut (duplicate) | MAT-18 `nonunit_dims` | — |
| CELL-23 self-heating width `W_heat` | Cut (duplicate) | REL-13 owns `self_heating_min_width_nm` and the current input; CELL-23 keeps the value-preserving length | — |
| CELL-24 Shield plates over matched arrays (H08-23, H08-29) | Deferred | No quiet reference net is emitted (EXT) and no coupling measurement exists to show a gain (PERF-16 PEX is a parallel-plate lower bound, AV-07); RTE-19 already generates routing shields | EXT names a quiet low-Z net per matched array and PERF-16 reports array-to-aggressor coupling |
| CELL-26 MOS-capacitor generator (gf180 `cap_nmos_*`/`cap_pmos_*`, gf180mcu.deck:540-543) | Deferred | No fixture or user circuit uses a MOS capacitor; every capacitor is drawn as MIM/MOM | A gf180 fixture with a MOS-cap model lands |
| CELL-27 In-cell drain straps for matched pairs (AC-20, CC-39/41) | Deferred | RTE-15 routes matched net pairs jointly and mirrored; no measurement shows a residual drain-R mismatch after it | PERF-16 reports a matched pair's drain R difference > one lattice step after RTE-15 |
| CELL-28 Inductor spiral generator (H06-47..52) | Deferred | No deck declares an inductor recogniser and no RF fixture exists (CELL-04 removed the shorted drawing) | A deck recognises an inductor, or an RF fixture lands |
| LVS `area` params for R/C/D drawn cards | Deferred | Interning `"area"` in the reference makes GPurify emit `area` on every extracted device (GP/crates/check/src/lvs/graph.rs:108-122), so every MOS and dummy card would need an `area` param too (a one-sided param is a mismatch, cellgen.rs:907-911) | A value-level LVS comparison is required beyond CELL-06/08's generator tests |
| Chain rows with an even-nf member (AC-24, low) | Deferred | A series-stack member with even nf ends on the wrong terminal for the next member (mosfet.rs:81-85); no fixture has one | A cascode fixture with even-nf members fails to merge |
| EXC full dummy with an 8–10 µm gate (rule 12, hastings.txt L42544–42553) | Deferred | CELL-12 meets rule 12's 10 µm reach with minimum-L dummies; the long-gate alternative is an area optimisation | Bench shows EXC rows dominate area |

## Verification log

Adversarial fact-check of this plan, 2026-09-28, working tree at HEAD `dad330c` plus uncommitted changes. GPurify pinned at `8df8c09` (Cargo.lock:676-678). Read-only: no build or test was run.

**References checked: 617 citation occurrences.** That is 148 `*.rs:line` citations, 93 bare line ranges inside a file's context, 44 deck/sidecar/model-card/tech-file lines, 105 `reftext` line ranges, and 227 audit/ref entry-ID uses. Every ID resolves to an existing entry in `docs/plans/audit-0*.md` or `docs/plans/ref-*.md`, and each "Why" matches its entry's topic. Numeric constants were recomputed from the model cards and the deck: R_high_po(0.69 µm, 40 µm) = 18 949.2 Ω; +2.76 % (n=2) and +8.28 % (n=4) today; seg_len 19.432 / 9.148 µm; pitch 730 → 430 nm; sd_w 290 → 280 nm; licon.7 tap 410 nm; capm rules; lvtn/hvtp 180/380/0.265; Table I of NTH; Hastings rules 12, 19, 21 and 22 (PDF pp. 693, 715-716 opened to recover the garbled numbers). Proposed new names (`Drawn`, `Node`, `Keepout`, `KeepWhy`, `Figures`, `MatchGrade`, `Outer`, `pair_order`, `legal_row`, `sym_order`, `palindrome`, `ResModel`, `PlateStack`, `TapStyle`, `WellShape`, `drawn_cards`, `extracts_params`, `max_finger_for_taps`, `cc_exact_feasible`, `drawable`, `tier`) collide with nothing in `kernel/`, `backend/`, `frontend/`, `benchmarks/` or `examples/`.

**Corrections (before → after):**

1. §1.2 MOS (a): "the cause is isolated 340 nm pads" → pads alone force 550 nm (340 + poly.2 210); the rest of the 730 nm comes from `sd_and_pitch` doubling `sd_w` (mosfet.rs:664-668).
2. §1.2 resistor (c): "480 vs 400" → the gap term is 480 vs 400 (seg_gap, resistor.rs:257-260; poly.9). (e): `cellgen.rs:884-891` → `889-890`.
3. T5: "440 while the sidecar `m1_enc` 65 stays" → 430 either way under CELL-11's deck-only `m1_land`.
4. §2: "adds 409 Ω per extra segment" → 523 Ω (409.4 Ω head + 113.8 Ω from the +0.247 µm length offset). The +5.4 % figure was already right.
5. CELL-01 Why: the H06-02 quote was attributed to Hastings's lines → it is the study entry's automation recipe.
6. CELL-01 Current: MOS params `874-880` → `879-885`; resistor/diode cards `884-891` → `886-891`, BJT unit cards added.
7. CELL-02 Current: added that `testkit::dirty_group` already asserts non-empty (lib.rs:121); only `check_on` does not.
8. CELL-04: added deletion of `inductor.rs::every_variant_is_drc_and_erc_clean` (inductor.rs:80-91), which would panic at lib.rs:121 once enumeration is empty.
9. CELL-05: `Ebgr` deletion reason marked [UNVERIFIED]. The cited entries cover only the HBGR.
10. CELL-06: model card `:27-39` → `:27-40` (rbody is line 40), in the text and in `res_model_source`. Added a note on rounding 0.0672 and 0.1558.
11. CELL-06 and §5 Q5: generic_po `dw` "not evaluated" → +0.028 µm at typical (r+c.model.spice:78, :134; tol_poly = 0). How ngspice applies it is [UNVERIFIED].
12. CELL-08 Current: MIM model `C = camimc·W·L + cpmimc·2(W+L)` (model.spice:23-25) → uses wc = W + m3_dw and lc = L + m3_dw (model.spice:17-18, 23-24), with m3_dw = −0.025 µm (sky130_fd_pr__model__r+c.model.spice:82).
13. CELL-08 recipe JSON: added `"c_dw_nm": -25` and its source.
14. CELL-08: C_u at 5×5 µm, 53.80 fF → 53.28 fF, in step 2 and in test `the_mim_unit_is_the_model_s`.
15. CELL-08: `fixtures.rs:556` → `556-559` (density at 556, the `cap` model at 559).
16. CELL-08: added a precondition. `widest_on` filters rule layers through `philis_drawn`, and `recipe_layers` scans only `resistors` (pdk.rs:752-753, 775-790, 798-817, 1103-1114). Two-layer capm lookups therefore return `None` today.
17. CELL-09: `Unit{…}` → `pnr_core::Unit`, because bjt.rs already has a private `struct Unit` (bjt.rs:73-84).
18. CELL-10 fixture: the 1:2/1:4 ratios hold only under Philis's per-finger W reading (cellgen.rs:710-713, 879-885; lib.rs:792). The model-card convention is [UNVERIFIED].
19. CELL-11 step 4: "rule 22 … for the most accurate class" → rule 22 covers MOD and EXC. The MOD poly bar follows the looser §13.2.2 comb text (PDF p.693, 1–2 µm).
20. CELL-12: added the third reader of `lod_moat_ext_moderate`/`wpe_clearance_moderate`, `frontend/library/src/lib.rs:871-872`. Added the key lines in the other three sidecars.
21. CELL-12: `n_well_depth` used in the EXC term is unsourced (AV-22), marked [UNVERIFIED].
22. CELL-12 Risks and §5 Q9: "24 dummies … as rule 12 prescribes" → rule 12 prescribes the 10 µm reach. It names one full dummy with an 8–10 µm gate as a possibly better STI choice (L42544-42553).
23. CELL-13 and §5 Q1: `sky130A.tech:4163-4169` → `4164-4170`.
24. CELL-14 rdiv fixture: flagged a conflict with T4. Two 10 µm high_po units give 10 259 Ω against the schematic's 9 736 Ω (+5.4 %).
25. CELL-14 Risks: "a 9:4 divider as 2p×2s" → a 9:1 divider whose small resistor is 2p×2s, interdigitated as a 9:4 segment ratio (hastings.txt L23716-23721).
26. CELL-15 step 4: order 3 on counts divisible by 4 → order 3 needs ≥ 8 units per member (CC-04: P_n uses 2ⁿ per device).
27. CELL-16 step 2: added a precondition. `hvi` is not in `DRAWN_ROLES`, so two-layer hvi rules (hvi.5) are invisible to `widest_on` until `recipe_layers` scans `mosfets`.
28. CELL-19: `Option<ArrayMetrics>` on `Figures` is a crate-direction conflict. `pnr_core` cannot name a `cells` type. Move `ArrayMetrics` or flatten it.
29. CELL-19 Why: CC-24 "Tables I–III" → Tables II–III and VI (L934-945, L998-1007, L1031-1043).
30. CELL-26 Why: gf180 MOS caps "[poly2_con, ntap]" → n-type [poly2_con, ntap], p-type [poly2_con, ptap] (gf180mcu.deck:540-543).
31. §5 Q8: "keeping the key gives 440" → 430 either way under CELL-11. Today the key raises the pitch from 710 to 730.
32. §5 Q11: "GPurify's fringe-free lower bound (AV-32)" → the extracted value (AV-32), from a PEX that AV-07 describes as a parallel-plate lower bound.

**Checked and found correct (no edit):** every mosfet.rs, builder.rs, resistor.rs, cap_array.rs, capacitor.rs, bjt.rs, diode.rs, finfet.rs, inductor.rs, post_cell.rs, lib.rs and cell_selfcheck.rs line cited in §1 and in the items' "Current" bullets. Also correct:

- All sky130.json keys and line numbers.
- REQUIRED_RULES lines (pdk.rs:1039-1071), `parse_roles` (1259-1267), `Pdk::recipe` (1129-1144), `Overlay::layer`/`rule` (1155-1164), `DRAWN_ROLES` (1211-1212).
- cellgen.rs filters (138-141, 276, 290), singletons (563-572), `draw_all` (815-835), `reference` (858-905), `dummy_cards` (912), `bind_pins` (937).
- library lib.rs:1198, 1293-1300, 1351-1370.
- Deck lines 276-284, 331-345, 360-366, 383-384, 407-411, 424-427, 542-570, 582, 587, 605, 648-653, 663-665.
- GPurify reduce.rs:1 and 270-274.
- The res_high_po card, camimc/cpmimc, rp1.
- The 23 `Macro` literal sites.
- Every Hastings, CC, SUB, LAMP, NOTES and BAL2 range spot-read against its entry or the reftext.

**Unresolved:**

- EBGR construction requirement (CELL-05): no cited source.
- sky130 BSIM W-with-nf convention for the `mirror_ratio` fixture (CELL-10).
- How ngspice applies the generic_po `dw` (CELL-06, Q5).
- `n_well_depth` 2000 nm has no source (CELL-12).
- rdiv versus T4 (CELL-14) needs a design decision: unit sizing by `seg_len` per member, or T4 restated against count·R_unit.
- The `UNDRAWABLE` tuples in CELL-02 still have to be checked against today's enumeration, as the plan already says.
- The plan's own open questions stay open: whether GPurify extracts W/L on resistor/MIM/diode (Q2), whether named internal nets anchor refinement (Q3), and whether magic extracts the fixed-geometry BJT as the foundry device (Q6).

---

## Implementer review log

Review of 2026-09-29 by the engineer who implements this plan next week. All 29 work items were read against the code (`kernel/cells/src/{mosfet,bjt,resistor,diode,cap_array,post_cell,lib}.rs`, `frontend/library/src/cellgen.rs:855-940`, `backend/verify/src/pdk.rs:740-817, 960-1010, 1070-1270`), the sidecar (`pdks/sky130.json`), GPurify (`topology/device.rs:290-325`, `lvs/graph.rs:62-70, 105-122`, `lvs/compare.rs:22-30`), the fixtures directory, and plans 01, 02, 05, 06, 07 and 08 for overlap. No build or test was run.

Duplicates removed (another plan already owns the same code change):
1. CELL-05 → cut; REL-07 steps 1-2 own the ring enum and `post_cell` implant/well mapping.
2. CELL-09 `sym_order` → cut; MAT-02 owns the BJT order and `Unit` records. CELL-09 is now fixed-geometry emitters (`cell.bjts` recipes) plus `Drawn` cards; effort M → S.
3. CELL-10 `pair_order`/`cc_exact_feasible`/source-ended mirror order → cut; MAT-03 owns `diffusion_cc_row`. CELL-10 is now the explicit region-parity rule (`legal_row`, `s0`), which also fixes blocked (`Single`) rows of even counts that put device boundaries on drains; effort M → S.
4. CELL-12 `MatchGrade`, tier parsing and key table → cut; the item now consumes MAT-07's `class::mos_env` and keeps only the `draw_row` geometry and tests. Expected values follow MAT-07's table (MIN moat 3000, not 2000).
5. CELL-14 `palindrome` → MAT-12 `segment_row`; dummies from MAT-07 `resistor_env`.
6. CELL-17 → reduced to the ECGR n-well band. REL-07 deletes `Hcgr` and allows one ring per device, so `drawable()` and concentric nesting had no caller. The band fixes a defect REL-07 would otherwise ship: `tap_ring` fills the n-well over the ring interior (post_cell.rs:276-282), which buries the NMOS.
7. CELL-19 → `Figures` reduced to `sd` and `gate_ohm`, the two fields PERF-26 reads; the `ArrayMetrics` crate-direction conflict is gone (MAT-11 owns it).
8. CELL-21, CELL-23 → consume MAT-18 `nonunit_dims`/`split_dac_assign` and REL-13's self-heating width instead of re-deriving them.
9. CELL-29 → the `cell/undrawable` naming, CRATES.md/LAYOUT-FUNDAMENTALS docs and `REQUIRED_RULES` edits moved out (FLOW-15 steps 1, 5 and FLOW-04 step 5).

Decisions that close questions the fact-check left open:
10. CELL-01: `Pdk::extracts_params` probe dropped. GPurify measures `w`/`l` only for MOS and `area` for every device (device.rs:295-323), so non-MOS cards carry no params; §5 Q2 resolved. `area` params deferred with the reason (interning `"area"` would require it on every MOS and dummy card).
11. CELL-01: BJT `Drawn.nodes` follow one shared `BJT_PINS` constant, fixing the E,B,C vs C,B,E disagreement with PERF-18.
12. CELL-14 rdiv/T4 conflict: series units are offered only when the model value holds per member within `res_value_tol_ppm`; otherwise members draw as exact CELL-06 strings in blocks. The rdiv fixture uses `res_generic_po` (head 0), with EXT-15's {RA:1, RB:4}, and is written out in full. `high_po` numbers recomputed: 2 × 10 µm = 10 259.3 Ω vs 9 736.2 Ω (+5.37 %), 4 × 10 µm = 20 518.6 Ω vs 18 949.2 Ω (+8.28 %).
13. CELL-10 fixture: rewritten under FLOW-01's SPICE total-width convention (W = 2u/4u/8u, nf = 2/4/8, 1 µm fingers), which removes the [UNVERIFIED] per-finger question. Added the EXT-15 amendment it needs (halve `W_u` when the counts admit no diffusion-legal row), because EXT-15's own `mirror_ratio_by_width` test yields odd counts [1,2,4].
14. CELL-13: depends on REL-06 (15 000 nm); §5 Q1 resolved. The bottom tap strip is deferred: no shipped planar deck needs it after REL-06.
15. CELL-16: markers come from FLOW-06's `Pdk::model_markers` through an overlay role `gate_marker{i}`; no `mosfets` sidecar table and no trait change. 5 V `hvi` devices deferred: making `hvi` a drawn layer globally would widen every diffusion rule.
16. CELL-08: recipe rows now extend FLOW-06's `capacitors` table (its role names `bottom/plate/top_contact/top`, its `mim_m3_1` key) instead of a conflicting second schema; added `CAPACITOR_ROLES`, the `recipe_layers` change, and a guard test that routing-layer rules do not move. C_u restated in aF (53 282 ± 10) and tested from `Macro.drawn`.
17. CELL-23: the sky130 `high_po` width set {350, 690, 1410, 2850, 5730} nm, sourced from the foundry per-width model files; §5 Q4 partly resolved.
18. CELL-02: `UNDRAWABLE` derived from the enumerate guards (bjt.rs:34, resistor.rs:23). `generic_finfet` Pnp removed (no guard), Inductor added; the test is now an equivalence, so the list cannot go stale. testkit signature made exact (`dirty_group_with`, `ports_with`).
19. CELL-03: `cut_lattice` defined; the ring-halo rounding kept at one lattice step (it already keeps extents on 20 nm).
20. Test values made numeric where they were not: `gate_ohm` 305.8 Ω, diode and resistor centroid checks as integer cross-products, CELL-22 span example, CELL-23 W' = 1410.

Priority, order and milestones:
21. CELL-15 P1 → P2 (MAT-15 is P2; order-3 cancellation pays only under curved gradients).
22. Milestones rebuilt with each milestone's external prerequisites and numeric exit criteria; CELL-10 moved to M1 with the other P0 items; M4's "per item" exit replaced by the named tests.
23. Deferred to the appendix with reasons and re-entry conditions: CELL-24 (shields), CELL-26 (gf180 MOS cap), CELL-27 (drain straps; RTE-15 covers matched routing), CELL-28 (inductor spiral), the chain even-nf case (AC-24) and the EXC long-gate dummy.

Still open (unchanged by this review): whether named internal nets anchor GPurify's refinement (§5 Q3); how ngspice applies the generic_po `dw` (Q5; ratios are unaffected because head and dl are 0); whether the generated BJT extracts as the foundry device (Q6); the three-way `MatchClass` encoding conflict between EXT-12, MAT-07 and FLOW-06 (Q7), which CELL-12 step 1 works around.
