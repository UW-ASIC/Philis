# Audit 03: device generators (`kernel/cells`)

Auditor scope: `kernel/cells/src/**` and `kernel/cells/tests/**`, read completely, working tree as of 2026-09-28 (uncommitted edits in every `src` file except `cap_array.rs` and `finfet.rs`). ID prefix: **AC-NN**. No build or test was run. Every number marked *computed* is my arithmetic from deck values; nothing here was measured.

Reference text paths are relative to the scratchpad `reftext/` directory. "PDF p" means the PDF page counter (1 + form feeds). Where the extracted text dropped a value, I read the PDF page image and give that page.

---

## 0. Scope

| File | Lines | Read |
|---|---:|---|
| kernel/cells/Cargo.toml | 14 | all |
| kernel/cells/src/lib.rs | 166 | all |
| kernel/cells/src/builder.rs | 320 | all |
| kernel/cells/src/mosfet.rs | 1087 | all |
| kernel/cells/src/cap_array.rs | 796 | all |
| kernel/cells/src/capacitor.rs | 494 | all |
| kernel/cells/src/resistor.rs | 361 | all |
| kernel/cells/src/post_cell.rs | 641 | all |
| kernel/cells/src/bjt.rs | 282 | all |
| kernel/cells/src/diode.rs | 149 | all |
| kernel/cells/src/finfet.rs | 245 | all |
| kernel/cells/src/inductor.rs | 92 | all |
| kernel/cells/tests/cell_selfcheck.rs | 434 | all |
| **Total** | **5081** | |

Context read partially, to trace how the generators are called and checked:

- `frontend/library/src/cellgen.rs`: lines 40–340 and 440–936. Covers the merge, the filters, folds, `draw_variants`, `draw_all` and the LVS reference.
- `frontend/library/src/lib.rs`: lines 200–262, 598–616, 860–880 and 1190–1202.
- `backend/annotator/src/constraints.rs`: lines 1–100.
- `backend/verify/src/pdk.rs`: lines 76–90, 938–961, 1030–1071 and 1155–1172.
- `kernel/core/src/units.rs`: lines 17–62. `kernel/core/src/geom.rs`: lines 22–33.
- The `cell` sections of `pdks/{sky130,gf180mcu,ihp_sg13g2,generic_finfet}.json`.
- The GPurify deck the build pins, `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/pdks/sky130.deck` (Cargo.lock:678, rev 8df8c09), lines 43–670. `backend/verify/build.rs:3-12` lets `GPURIFY_DIR` override this path.
- `docs/CRATES.md` lines 30–40 and 79–117, and `docs/LAYOUT-FUNDAMENTALS.md` (searched).
- `benchmarks/src/fixtures.rs` lines 538–575, and the header of `frontend/library/src/fill.rs`.

---

## 1. Architecture and data flow

### 1.1 Contract

- **Trait `Cell`** (`lib.rs:45-52`):
  - `enumerate(group, constraints, process) -> Vec<Self>` returns every feasible variant in a deterministic order. The variants are never ranked; the placer picks one.
  - `draw(&self, …) -> Macro` is pure.
- **Input:**
  - `DeviceGroup`: device ids.
  - `analog::Constraints`. Only `unitization` is read. `post_cell` also reads `guard_rings`.
  - `&dyn Process`: the deck (`verify::Pdk`), or a resistor recipe `Overlay`.
- **Sizing** (`builder.rs:233-267`):
  - The covering `Unitization` is found by subset match. It supplies `unit_w`, `unit_l`, per-member `dev_nf`, `device_type`, `series_parallel`, `dummy_required` and `route_matching_required`.
  - Each generator has its own default sizes, read from the deck's sidecar keys.
- **Output `Macro`:**
  - `shapes`: rectangles only.
  - `pins`: named `d{i}:{T}`. The nets are synthetic placeholders (`builder.rs:216-221`) that cellgen rebinds by name (`cellgen.rs:937`).
  - `bbox`: rounded to two grid steps (`builder.rs:119-135`).
  - `units`: one per active finger, segment or cap. Each carries `owner`, centre, `weight`, signed S→D direction `phi`, and `sa`/`sb` (= BSIM4 SA+L/2 and SB+L/2, per `core/units.rs:28-32`).
  - `dummies`: dummy-gate LVS cards.
- **Builder** (`builder.rs:9-136`):
  - Snaps every rectangle to the grid and never lets a width or height go below one grid step.
  - Snaps cuts down to a lattice of two grid steps (`cut_lattice`, `snap_cut`).
  - Covers poly cuts with the `npc` role (`cover_poly_cuts`).
  - `finish` removes exact duplicate shapes.
- **Rule access:**
  - `dim()` (`builder.rs:195-212`) = max(sidecar key, deck-derived rule). The sidecar can only raise a value.
  - `req()` panics if a mandatory role is missing.
  - Everything else goes through `process.width`, `space`, `enclosure`, `endcap`, `extension`, `area` and `space_between`.
  - `process.rule(name)` also resolves names that `verify` synthesises from the deck for every layer and role: `<name>_min_width`, `<name>_min_spacing` and `<role>_min_area` (`pdk.rs:131-172`). Example: `poly_min_spacing` = poly.2 (210) and `nwell_min_spacing` = nwell.2a (1270) on sky130, even though no sidecar lists them. Explicit `cell.*` keys win.

### 1.2 Consumers

- **`cellgen::draw_variants`** (`cellgen.rs:792-811`) chooses the generator by device kind:
  - fin deck → `FinFet`
  - planar MOS → `Mosfet`
  - resistor → `Resistor` through the model's recipe overlay
  - capacitor → `CapArray`, or `Capacitor` when `CapArray` returns no variants
  - diode → `Diode`; bipolar → `Bjt`; inductor → `Inductor`
- **`draw_all`** (`cellgen.rs:815-833`) returns one **empty macro** when `enumerate` returns nothing.
- **Merge filters** on multi-device groups (`cellgen.rs:138-141`):
  - `shared_pads_carry_one_net`: no two nets on one pad.
  - `gate_straps_stay_private`: poly islands, checked with `mosfet::gate_islands`.
  - `currents_run_alike`: Σφ per member.
  - If every variant fails, the members are drawn as separate cells.
- **Folding** (`cellgen.rs:615-706`): `folds` picks `k` for each (kind, W, L) class and redraws it as `k×nf` fingers of width `W/k`. The row aspect uses `cells::mosfet::sd_and_pitch`. The generators never refold.
- **Post-placement** (`lib.rs:606-613`): `post_cell::guard_rings`, then `well_bridges`, then `implant_bridges`. `ring_halo` inflates the requester's variant bboxes before placement (`lib.rs:1196-1200`).

### 1.3 Algorithms as implemented

| Generator | Construction | Variant axes |
|---|---|---|
| `Mosfet` | One diffusion row with vertical gates at a uniform pitch `2·sd_w+L` (`mosfet.rs:657-669`). Each gate has its own contacted pad (bottom end, or top end for split device 1). A per-device poly strap sits under an li strap. S/D columns hold cuts at the licon pitch. End dummies are bulk-tied through a poly skirt and li risers to one top tap strip. There is an optional LOD "moat" of bare diffusion, and the PMOS nwell carries a WPE halo on matched groups. | style ∈ {Single, Cc1d, Chain}; split_gates; mirror_pins; rows ∈ {1,2}; double_gate (`mosfet.rs:69-133`) |
| Finger order | `centroid_sequence` (`mosfet.rs:730-750`): cyclic `ABBA` for pairs, and `A BB CC…AA…CC BB A` for n>2 when nf%4==0. `mirror_sequence` (`mosfet.rs:760-779`): exhaustive search over the 2^(nf/2) half-orders `H ++ swap(rev H)` for the minimum centroid offset, falling back to alternation above nf=32. `greedy_centroid` (`builder.rs:271-292`): outside-in mirror fill by most-remaining fingers, used whenever `dev_nf` differs between members. Chain: members in series order, each starting on D. | — |
| Two rows | Row B is mirrored about row A's tap strip. For a pair, B carries A's labels swapped; otherwise B is reversed (`mosfet.rs:152-222`). | — |
| `CapArray` | A `2^N`-unit grid (binary bank `[1,1,2,…]`) or a near-square grid (general set). Point-symmetric assignment: Spiral (ring order), Chessboard (greedy farthest-point pairs), or BlockChessboard (DACP Algorithm 1 corridors). A dummy ring surrounds the array. Bottom plates are met1, top plates met2 (inset), with per-column met3 top straps joined above the array. Bottom plates are routed on per-column channel tracks (met1) down to per-bit met2 buses below. `metrics()` gives the analytic centroid, INL/DNL and route spread (`cap_array.rs:357-397`). | pattern × tall |
| `Capacitor` | One merged plate per device: a single-metal comb, a two-metal plate, or a three-metal sandwich with a side strap column. | units_x (divisors, aspect only) × kind, truncated to 16 |
| `Resistor` | Vertical poly segments with contacted heads (slot cuts per recipe), joined in series by li (adjacent columns) or met1 tracks (non-adjacent). Order is Single (blocked) or `Interdig` (greedy_centroid). Plain-poly end dummies are tied to GND. The recipe layers are rpm, npc, implant and salicide block. | segments (1 or even, eight squarest) × pattern |
| `Bjt` | Concentric emitter, base ring and collector ring per unit, arrayed centre-out with the smallest member first. An NPN adds an nwell ring and a dnwell per unit. | columns |
| `Diode` | An n+ diffusion cathode plus a p+ substrate-tap anode per device under `diom`, or a p+/nwell well form under `diode_mk`. | pattern {Single, Interdig=flip} × columns |
| `FinFet` | Fins at the fin pitch, gates at the gate pitch, trench/LISD per S/D, LIG gate pads, and M1/M2 straps. Members sit side by side on separate actives, with one tap strip. | none (one variant) |
| `Inductor` | Nested met1 rectangles plus an li strip at the centre. | turns = Σdev_nf |
| `post_cell` | Guard rings: union-find over shareable, same-class requesters within 2 µm, split greedily where the hull would hit a foreign cell. Each ring is a four-band tap under implant and li, with 1–4 rows of cuts sized against `max_ring_resistance_mohm`. `well_bridges` fills facing same-bulk nwell gaps up to twice the well spacing; `implant_bridges` fills same-type implant gaps. | — |

---

## 2. Module-by-module findings

### 2.1 `lib.rs` (166)

- **`Pattern`** (`lib.rs:31-42`): `Interdig` is documented as ABAB but means three things:
  - `Mosfet` never offers it (`mosfet.rs:87-88`).
  - `Resistor` uses it for greedy centroid (`resistor.rs:302-307`).
  - `Diode` uses it for "flip every other device" (`diode.rs:92-94`).

  See AC-26.
- **`testkit`** (`lib.rs:57-166`):
  - `port_name` labels every member's `G`, `S` and `B` as one net (`lib.rs:159-165`). The in-file DRC/ERC sweeps therefore cannot see a short between two members' gates or sources. Gate privacy is tested separately (`mosfet.rs:1007-1030`).
  - `findings` drops `*_density`, `floating_gate` and `soft_connection*` (`lib.rs:113`).
  - `dirty_group` asserts that at least one variant exists (`lib.rs:121`). The integration tests do not (§2.13).

### 2.2 `builder.rs` (320)

- **Grid:** `snap` rounds half away from zero and never goes below one grid step (`builder.rs:46-54`). `finish` rounds the bbox out to two grid steps (`builder.rs:124-133`), which keeps cut lattices intact when the placer centres a cell.
- **`cover_poly_cuts`** (`builder.rs:63-112`): each cut row gets one npc strip, and strips closer than the npc spacing are hulled. The ceiling is noted in-code: a row cannot skip a foreign contact.
- **`dim`** (`builder.rs:195-212`): the sidecar value can only raise the deck value. Example: sky130 sidecar `m1_enc` = 65 against the deck's m1.5 of 60 nm, so every MOS pitch floor that uses `m1_pitch` grows by 10 nm.
- **`greedy_centroid`** (`builder.rs:271-292`): balances centroids but ignores diffusion parity. Its test (`builder.rs:307-314`) only counts fingers. This is the root cause of AC-05.
- **Nits:** the doc comment at `builder.rs:30-31` belongs to `pin` but sits on `unit`, and `hull` is defined after the test module (`builder.rs:317`). Tests: `snap_rounds_half_away_from_zero` and `greedy_centroid_uses_every_finger`.

### 2.3 `mosfet.rs` (1087)

**What it does.** See §1.3. The generator is sound in these respects:

- Every S/D region gets a full column of cuts at the licon pitch (`mosfet.rs:303-309`, `mosfet.rs:458-472`).
- Dummies sit on the diffusion at the finger pitch and are tied to bulk (`mosfet.rs:279-284`, `mosfet.rs:511-565`). They are listed as LVS cards.
- Units carry `phi` and SA/SB (`mosfet.rs:382-390`).
- Split gates keep an ABBA pair's gates private (`mosfet.rs:364-367`), with a test at `mosfet.rs:1007-1030`.
- The mirror-pin order is swap-reverse symmetric (`mosfet.rs:752-779`).
- Two rows produce a cross-coupled quad (`mosfet.rs:161-222`, test at `mosfet.rs:990-1002`).
- The two-ended gate is offered only where the finger's poly R exceeds a contact's R (`mosfet.rs:58-66`).

**Correctness and quality problems**

1. **Contacted pitch is about 1.7× hand layout (AC-04).**
   - `sd_and_pitch` returns `pitch = max(2·sd_w + L, m1_pitch)` (`mosfet.rs:668`). Every floor inside `sd_w` (`mosfet.rs:664-667`) is written for one S/D gap, e.g. `m1_pitch − L` and `li pad + li space − L`, so pitch = sd_w + L would already satisfy them.
   - *Computed* for sky130 at L = 150 nm, using ct = 170, li.5 = 80, li.3 = 170, m1_enc = 65, m1.2 = 140 and licon.5a = 40: sd_w = max(250, 290, 270, 250) = 290, so the pitch is **730 nm** and the S/D gap is **580 nm**.
   - The minimum contacted pitch is licon.1 (170) + 2 × licon.11 (55) + L (150) = **430 nm**, with a 280 nm gap.
   - The doubling hides the real constraint. Every finger has its own isolated pad, `pad_w = ct + 2·80 + lat = 340` (`mosfet.rs:344`, `mosfet.rs:393-402`). Neighbouring pads above the strap need poly.2 or poly.2.notch = 210, so the pitch must be at least 550.
   - Result: about 1.7× row length and about 2× S/D junction area per finger, so higher Cdb and Csb. `folds` inherits the inflated pitch through its aspect computation (`cellgen.rs:661`).
2. **Ratioed groups short their drains (AC-05).**
   - When `dev_nf` differs between members, `finger_sequence` uses `greedy_centroid` (`mosfet.rs:785-788`). Multi-device rows make every even region a drain (`mosfet.rs:327-328`), so two different members meet across a drain.
   - Example: `[2,4]` gives `BBAABB`, whose region 2 carries both A's and B's drain.
   - cellgen rejects each such variant (`cellgen.rs:139`, `cellgen.rs:276-282`). Because every variant of a ratioed group uses this order, the group is never merged.
   - A legal common-centroid order does exist: `A BBBB A` puts A's drains at the two row ends and pairs B's drains.
   - `enumerate` also keys everything on `dev_nf[0]` (`mosfet.rs:78`).
3. **Single, one-sided tap strip (AC-07).**
   - The tap sits only above the row (`mosfet.rs:567-572`). With fingers up to `max_finger_width` = 10 µm (sky130.json), the bottom of the diffusion is more than 10 µm from any tap.
   - The deck's `tie_max_dist_nm` = 3000 (sky130.json:85) is never read by a generator. `verify` lists it as required (`pdk.rs:1068`).
   - The pinned deck has no latch-up rule ("Latch-up: esd_latchup needs a guard-ring width and tap distance that [S1] does not state", GPurify sky130.deck, note block near line 668), so no check catches this. This is the same issue as CRATES #5.
4. **Split gates break equal poly overhang (AC-11).** Device 0's fingers go down to the bottom pads and device 1's go up (`mosfet.rs:364-380`). Dummy gates have a skirt at the top and only `poly_ext` at the bottom (`mosfet.rs:516`, `mosfet.rs:537-552`). Hastings MOS r21 asks for 1–2 µm beyond the rule, for dummies too, and "no gates … extend further than their counterparts" (hastings.txt 42638-42643; value read from PDF p716).
5. **A poly strap is drawn under the li strap** (`mosfet.rs:428-436`). Hastings MOS r22 says to connect gate fingers with metal rather than polysilicon (hastings.txt 42644-42647). See AC-12.
6. **LOD and WPE come in one tier.**
   - `moat = lod_moat_ext_moderate` applies only to multi-device rows (`mosfet.rs:293`). Every deck carries the same value, 3000.
   - The halo is `wpe_clearance_moderate` for matched PMOS only (`mosfet.rs:629`). The sky130 sidecar also has tiers `wpe_clearance_nm` [2000, 3000, 5000] and `lod_moat_ext_nm` [3000, 5000], which are unused.
   - Hastings r12: in STI, the moat extends ≥ 5 µm past the last active gate for moderate matching and ≥ 10 µm for exceptional (PDF p715). The moat here is 3 µm past the outer dummy's S/D.
   - `dummies_per_end` defaults to 1 because no deck defines `dummy_gates_per_end` (`mosfet.rs:54-56`).
   - NMOS gets no WPE margin in-cell. `lib.rs:871` scores it at placement instead.

   See AC-13.
7. **No per-variant figures** are exported for the placer: centroid offset, junction C, gate R and strap R. The `Cell` trait has only `enumerate` and `draw` (`lib.rs:45-52`), and the variant seed is an isolated routing price (`cellgen.rs:327`). See AC-21.
8. **Chain needs every member odd** (`mosfet.rs:81-85`). A centroid order for n > 2 needs nf % 4 == 0 (`mosfet.rs:737-739`). See AC-24.
9. **No in-cell S/D routing.** Each region exposes one li pin (`mosfet.rs:473-481`). The router joins the drains of an ABBA device, so matched interconnect R is not built into the cell, except in the `mirror_pins` variants (AC-20).
10. **Literal and comments:** `skirt_y = finger_w + poly_ext - 10` hard-codes 10 nm (`mosfet.rs:537`). The comment at `mosfet.rs:560` says "40 nm" while the code uses `lat`. The doc at `mosfet.rs:44-47` cites Hastings rule 22 for gate resistance, but r22 is about etch uniformity (AC-25, AC-28).

**Tests** (sky130 only; each skips if the deck is absent):

- `every_variant_is_drc_and_erc_clean` (`mosfet.rs:803-824`): NMOS and PMOS at n,nf ∈ {1,1},{2,1},{2,2},{4,4}, W = 420 to 10 000.
- `every_variant_extracts_its_fingers_and_dummies` (`mosfet.rs:828-851`): count of M cards = units + dummies.
- Order properties: `mosfet.rs:855-880`.
- Units, phi and moments: `mosfet.rs:885-905`.
- Chain junction sharing: `mosfet.rs:910-946`.
- Two-ended extraction: `mosfet.rs:951-965`.
- Uniform dummy pitch: `mosfet.rs:969-986`.
- Two rows: `mosfet.rs:990-1002`.
- Split gates: `mosfet.rs:1006-1030`.
- Mirror order and pins: `mosfet.rs:1034-1086`.

No test covers unequal `dev_nf`, tap distance or area and pitch.

### 2.4 `cap_array.rs` (796)

**What it does.** A binary bank or general matched set is drawn as a unit array after Karmokar et al. (DACP). The patterns are exact and point-symmetric; tests at `cap_array.rs:641-658` and `cap_array.rs:741-756` cover them. Top metal never crosses a bottom-plate route (`cap_array.rs:12-17`, construction at `cap_array.rs:562-581`).

**Problems**

1. **Wrong device for the deck (AC-01).**
   - The stack is BOT = met1 and TOP = met2 (`cap_array.rs:417`, `cap_array.rs:513-514`).
   - The pinned sky130 deck recognises a MIM, `device capacitor capm model "sky130_fd_pr__cap_mim_m3_1" terminals [met4, met3]` (GPurify sky130.deck:565). No generator draws `capm`.
   - *Computed* parallel-plate density from the deck's pex stack: met1 top at 1736.1 nm, met2 bottom at 2006.1 nm, so t = 270 nm. With k = 4.2–4.5, ε0·k/t ≈ **0.14–0.15 fF/µm²**, fringe excluded.
   - The benchmarks size capacitors from the sidecar's `cap_density_ff_um2` = 2.0 (`fixtures.rs:544-556`), so a netlisted C is drawn roughly **14× too small**.
   - LVS skips every capacitor (`cellgen.rs:844-848`, `cellgen.rs:870`). The comment there says sky130 has no capacitor recogniser, which the pinned deck contradicts. The performance deck keeps the schematic C (LAYOUT-FUNDAMENTALS row #53). No check sees the error.
2. **Binary-weighted and general sets only.** There is no split DAC with a bridge capacitor and no non-unit capacitor sized for equal perimeter-to-area ratio (DACP §II-B and §III-D, cc_dac_constructive.txt 177-180 and 606-625, PDF p3 and p6; Hastings 8.3.2 r1, hastings.txt 25470-25484, PDF p423). There is no electrostatic shield (Hastings 8.3.2 r8, hastings.txt 25562-25570, PDF p424). See AC-16.
3. **Tracks are one width** (`cap_array.rs:448`). There are no parallel wires for MSB bottom plates, which DACP uses to lower R (cc_dac_constructive.txt 141-144). There is no 3-dB-frequency figure (DACP 600-603), and C_TS and C_BB are not measured (DACP 143-146).
4. **`metrics()` is never called by the flow.** The only callers are tests (search: `cap_array.rs` and `cellgen.rs` only). The placer's variant choice is the isolated `seed_assignment` price (`cellgen.rs:327`).
5. **Loose tolerance:** extracted per-unit C must agree only within 6 % (`cap_array.rs:792`).

**Tests:**

- `only_binary_banks_are_recognised` (`cap_array.rs:632-637`).
- Exactness and symmetry: `cap_array.rs:641-658`.
- Spiral and chessboard shape: `cap_array.rs:662-671`.
- Block core: `cap_array.rs:674-680`.
- `metrics_rank_the_families` on a fake deck: `cap_array.rs:684-704`.
- DRC/ERC for N = 4, 5 and the general sets: `cap_array.rs:707-723`.
- General-set centroids: `cap_array.rs:741-756`.
- Extracted ratios within 6 %: `cap_array.rs:761-795`.

### 2.5 `capacitor.rs` (494)

- **Merged plate** (`capacitor.rs:178-186`): a device's units become one plate whose area includes the `unit_gap` between units. C is therefore not n × C_unit, and no pattern or dummy is drawn. This path is only reached when `CapArray` returns no variants (`cellgen.rs:803-806`), which on normal decks means single capacitors (AC-17).
- **Keep-out not exported:** "Nothing may be drawn between stacked plates … the router is not told this" (`capacitor.rs:23-26`). A router via through the plate stack shorts the capacitor, and no blockage is exported (AC-17).
- **Naming:** `Kind::VerticalInOneLayer` is the lateral comb. The comb is single-layer MOM; there are no stacked MOM layers joined by vias.
- **Tests:**
  - DRC/ERC for n = 1, 2 at 4 units of 2 µm: `capacitor.rs:332-345`.
  - Comb never shorts, via flood fill on a fake deck: `capacitor.rs:414-449`.
  - Distinct stacks: `capacitor.rs:453-469`.
  - Bounded enumeration: `capacitor.rs:472-481`.
  - Single-metal deck: `capacitor.rs:485-493`.

### 2.6 `resistor.rs` (361)

**What it does.** Vertical poly segments with contacted heads: the recipe's slot cut, e.g. sky130 high_po 190×2000 and `res_head` 2160 (sky130.json `resistors.recipes.high_po`). Series jumpers are li for adjacent columns and met1 tracks for non-adjacent ones. Current direction alternates per segment (`resistor.rs:109-128`), which satisfies Hastings 8.3.1 r11 (hastings.txt 25282-25290). End dummies are plain poly (`resistor.rs:162-192`). rpm/npc, the implant and the salicide block cover the array (`resistor.rs:194-230`).

**Problems**

1. **`dev_nf` is ignored (AC-02).** Every member is drawn with `segments` segments of `unit_l / segments` (`resistor.rs:81-82`, `resistor.rs:101`, `resistor.rs:304-310`). A resistor with `m = 2` or `nf = 2` is drawn as one resistor, i.e. twice the intended R. The LVS reference emits one card with no parameters for every resistor (`cellgen.rs:887-891`), so LVS passes while R is wrong.
2. **Multi-segment LVS is unverified.** The deck cuts each body out of poly, so each segment extracts as its own resistor. The reference has one card (`cellgen.rs:891`). CRATES #6 says this has not been checked.
3. **Dummy pitch differs from segment pitch (AC-14).** The dummy pitch is `body_w + max(seg_gap, space_between(rpoly, poly))` (`resistor.rs:170`); the segment pitch is `body_w + seg_gap` (`resistor.rs:83`). On sky130 the comment cites poly.9, which is 480 nm (sky130.deck `poly.9`), against `res_seg_gap` 400. Hastings 8.3.1 r9: "The spacings between all segments, dummy or active, must exactly match" (hastings.txt 25263-25273, PDF p420).
4. **No real common centroid (AC-15).** The only interleave is 1-D `greedy_centroid` with equal counts. A multi-segment `Interdig` is dropped when devices outnumber met1 tracks (`resistor.rs:41`). There is no 2-D array and no ratio by segment count. The annotator unitizes by (kind, W, L) (`constraints.rs:30-44`), so resistors whose ratio is set by L never share a group.
5. **Fixed orientation.** Segments are always vertical. That meets Hastings 8.3.1 r6 inside a cell; equal orientation across cells is left to placement.
6. **Test:** `every_variant_is_drc_and_erc_clean` (`resistor.rs:344-360`) at w = 500, l ∈ {10 µm, 40 µm}, n ∈ {1, 2, 3}. No test covers LVS or R value.

### 2.7 `post_cell.rs` (641)

**What it does.** See §1.3. It is sound in these respects:

- Rings are placed on reserved ground (halo), and splits keep a ring from swallowing a foreign cell.
- Bands are thin (the deck minimum), following Charbon ch.8 §6: "use minimum width rings and ensure a very good ground connection" (charbon_substrate.txt 3696-3704, PDF p147).

**Problems**

1. **Ring polarity is inverted against the requested type (AC-06).**
   - The annotator requests `Hcgr` (hole-collecting) for PMOS and `Ecgr` (electron-collecting) for NMOS, tied to the bulk (`constraints.rs:66-84`).
   - `implant_name` and `in_nwell` draw Hcgr/Hbgr as n+ in the nwell and Ecgr/Ebgr as p+ in the substrate (`post_cell.rs:310-323`). Those are majority-carrier bulk-tap rings.
   - Hastings §5.4.4 defines them differently (hastings.txt 15420-15446, 15514-15520, 15622-15627 and 15697-15700; PDF pp.264–269):
     - an ECGR "consists of an N-type region that collects minority-carrier electrons";
     - an HCGR "consists of a P-type region that collects minority-carrier holes … adjacent N-type region such as an N-well". In CMOS this is PSD inside a retrograde well.
     - "Substrate, tank, and well contacts cannot stop the flow of minority carriers" (hastings.txt 15422-15424).
   - Collecting rings, and double rings (a bulk ring plus a collector ring), are therefore never drawn. The test `well_follows_implant_polarity` (`post_cell.rs:539-546`) enshrines the mapping.
2. **Merged rings share one net.** Merged clusters (`post_cell.rs:79-137`) tie several victims to one ring and one net. Charbon §8.8 grounds separate rings independently; tied together, "the two guard rings appear to act as one large ring" (charbon_substrate.txt 3787-3811, PDF p150). See AC-23.
3. **Unused requirement fields and simplifications.**
   - `tap_pitch_nm` (the annotator sets 2000) and `enclosure_complete` are never read. `cut_pitch` always uses the densest pitch (`post_cell.rs:337-344`).
   - `ring_gap` ignores its `width` argument (`post_cell.rs:299-302`).
   - Ring R counts parallel cuts only, and rows are capped at 4 (`post_cell.rs:352-367`).
   - `RING_MERGE_GAP_NM` = 2000 is a policy default that the deck can override (`post_cell.rs:17`).
4. **`well_bridges`** bridges only wells with an identical span on the facing axis, at a gap of at most 2 × `nwell_min_spacing` (`post_cell.rs:424-430`; the value is synthesised from nwell.2a, 1270 on sky130). Matched PMOS cells of different heights, or offset by any amount, never share a well. Hastings r19 wants the well edge far from matched gates, and a common well is the usual way to get that.
5. **Tests:**
   - Same-bulk wells: `post_cell.rs:447-500`. Its line 483 filter `!…ends_with(":S") || true` is a no-op.
   - Polarity: `post_cell.rs:539-546`.
   - Clustering table tests: `post_cell.rs:548-598`.

### 2.8 `bjt.rs` (282)

1. **Not verifiable on sky130 (AC-08).** The pinned deck has no bipolar recogniser: "the bipolars (npn, pnp) … still wait" (GPurify sky130.deck, device notes near line 664), yet the flow emits one BJT card per unit (`cellgen.rs:884-886`). This matches CRATES #1.
2. **Equal pairs are not common-centroid.** `unit_order` orders slots by distance and then angle, and gives the smallest member the centre (`bjt.rs:208-231`). *Computed* examples:
   - `[2,2]` in a 2×2 grid gives rows AA/BB, not the diagonal quad.
   - `[1,4]` in a 3×2 grid leaves one cell empty and the centroids apart.

   Only `[1,8]` on a 3×3 is exact (test `bjt.rs:273-281`). Hastings 10.3.1 r6 recommends even ratios from 4:1 to 16:1 (hastings.txt 31327-31333, PDF p525), and Table 8.4 requires coincidence.
3. **No units, no dummies, one contact array.** There are no `Unit` records (`bjt.rs:50-68`), so the unit-based centroid and orientation checks fall back to nothing (`cellgen.rs:259-260`, `core/units.rs:38`). There is no dummy unit ring. The emitter contact array matches the emitter, which satisfies Hastings 10.3.1 r14 (hastings.txt 31404-31411).
4. **NPN isolation per unit.** Each NPN unit gets its own dnwell with dnwell.3 spacing of 6.3 µm (`bjt.rs:127-141`), which inflates area.
5. **Other items.**
   - NPN is offered only when `dnwell` exists and `npn_isolation` ≠ 0 (`bjt.rs:34`), which only sky130 sets. Elsewhere `draw_all` then returns an empty macro (AC-10).
   - `_constraints` and `_process` are named as unused but are used (`bjt.rs:27-37`).
   - `verify` still requires the obsolete `bjt_*` keys (`pdk.rs:1039-1044`; CRATES #9).
6. **Tests:** DRC/ERC for NPN and PNP at n = 1, 2 and 1:8 (`bjt.rs:248-269`); the 1:8 order (`bjt.rs:273-281`).

### 2.9 `diode.rs` (149)

- **`dev_nf` ignored (AC-02).** Every member is one diode of `unit_w × unit_l` (`diode.rs:89-119`). A diode with `m = 8` is drawn at 1/8 of its area. The reference has one card with no area (`cellgen.rs:887-891`).
- **One cut per terminal** regardless of junction size (`diode.rs:99-108`). Hastings 11.3.4 r5–r6 asks for square geometries with arrays of contacts and sizes 2–10× the minimum (hastings.txt 33935-33956, PDF p569–570). See AC-18.
- **No matching construct.** `Interdig` flips every other device; it is not a centroid. No `Unit` records are emitted.
- **gf180 well form unusable.** The file itself says so (`diode.rs:15-18`).
- **CRATES #7** asks for an anode tap ≥ 410 nm (licon.7 "opposite" = 120 nm on both sides). `tap_side = ct + diff_enc + tap_cap` (`diode.rs:67`) is 170 + 40 + 120 = 330 in x (*computed*, assuming endcap(tap, licon) = 120). y is enclosed ≥ 120 on both sides, so whether the deck passes depends on its "one direction" reading (CRATES #9 asymmetric_enclosure).
- **Test:** DRC/ERC for n = 1, 2 (`diode.rs:135-148`).

### 2.10 `finfet.rs` (245)

- **One variant** (`finfet.rs:88-94`). Members sit side by side on separate actives (`finfet.rs:139-194`), with no diffusion sharing, no CC, no dummies or diffusion-edge dummy gates, and one tap strip.
- **Units:** `sa = sb = 0` (`finfet.rs:186`), so LOD is invisible on fin decks. `phi` alternates by finger index.
- **Silent W quantisation:** `nfin = unit_w / fin_p` floors (`finfet.rs:114`).
- **Nit:** `act_s.max(2*sel_enc - 0)` (`finfet.rs:193`).
- **Reach and tests:** on a fin deck every MOS goes through this generator (`cellgen.rs:795`). There are no in-file tests. It is covered only by the release-only `the_mosfet_is_clean_on_every_deck` (`cell_selfcheck.rs:283-309`). See AC-09.

### 2.11 `inductor.rs` (92): placeholder that measures nothing

- **Turns short together (AC-03).** Every turn is drawn from origin (0, 0): `Rect { x: 0, y: 0, w: ring_outer, … }` and `{ x: 0, y: trace_w, … }` (`inductor.rs:46-48`). Only `ring_outer` shrinks, so all turns overlap on the bottom and left edges into one conductor. The spiral path does not exist.
- **N pin is isolated.** The centre strip is li (`inductor.rs:63`) with no mcon to met1, so N does not connect to the coil.
- **Poor layer choice.** The coil is met1, the thinnest and most resistive metal, with no ground shield.
- **Meaningless turn count.** `turns = Σ dev_nf` (`inductor.rs:23`).
- **Nothing catches it.** There is no recogniser and LVS skips inductors (`cellgen.rs:870`). The only test is DRC/ERC (`inductor.rs:80-91`).

### 2.12 `Cargo.toml` (14)

`verify` is a dev-dependency only, so there is no dependency cycle.

### 2.13 `tests/cell_selfcheck.rs` (434)

Tests:

- Axis coverage: `cell_selfcheck.rs:119-133`.
- Implant covers the channel: `cell_selfcheck.rs:135-178`.
- Extraction does not abort: `cell_selfcheck.rs:180-204`.
- PMOS inside a well: `cell_selfcheck.rs:206-228`.
- Bbox contains every shape: `cell_selfcheck.rs:230-251`.
- Bipolar well construction: `cell_selfcheck.rs:256-274`.
- Release only: MOS on four decks (`cell_selfcheck.rs:283-309`) and the other generators on three decks (`cell_selfcheck.rs:314-345`).

Gaps (AC-19):

- The two cross-deck tests are `#[cfg(not(debug_assertions))]`, so the default `cargo test` skips them.
- `check_on` (`cell_selfcheck.rs:362-408`) does not assert that variants exist, so an empty enumeration counts as clean. Example: NPN on gf180 and ihp.
- Diode, inductor and FinFET LVS are absent from the cross-deck sweep.
- No test covers `dev_nf` > 1 for a resistor or diode, ratioed MOS, tap distance or capacitance value.

---

## 3. Gap analysis against a constraint-aware analog P&R meant to beat hand layout

**Where Philis sits.**

- Philis is a procedural module-generator library whose variants a placer chooses among. That is Lampaert's "Simultaneous Placement and Module Optimization" pattern: stack-module alternatives generated first, then swapped by annealing (lampaert.txt 2833-2853, PDF p70).
- Like the "fixed library" approach (lampaert.txt 2763-2791, PDF p68–69), it loses every geometry-sharing opportunity outside its built-in shapes.
- BAG and LAYGO fix this with designer templates; MAGICAL and ALIGN with automated primitive generation (perf_driven_survey.txt 48-57, PDF p1).

What an expert, or ALIGN/MAGICAL-class tooling, does that these generators do not:

1. **Common centroid for ratioed banks, with diffusion sharing.**
   - ALIGN builds a half-diffusion graph `Mhalf` for a mirror bank with M = [2,2,4,8,8], finds an Euler path and reflects it about the CC point. It handles odd counts with an edge cell and orders for dispersion and LOD (cc_review.txt 430-452, PDF p6).
   - Philis's `greedy_centroid` ignores diffusion parity, and every ratioed MOS group falls back to separate cells (AC-05).
   - The general Euler-trail stacking of Bas 96, "constraint-driven … linear time" (lampaert.txt 2854-2915, PDF p70–71), is also missing. Only same-source rows and simple series paths merge (`cellgen.rs:95-128`).
2. **LDE-aware unit cells.**
   - Identical unit cells cancel every layout-dependent effect except LOD and WPE, and matched devices "must have same values of SA and SB" (cc_review.txt 100-131, PDF p2).
   - Philis records SA/SB, which is good, but draws no OSE or uniform-OD-spacing envelope. It has one moat tier (3 µm) where Hastings gives 5 µm moderate and 10 µm exceptional (PDF p715).
   - WPE is PMOS-only and single-tier, where Hastings r19 gives 3 µm moderate, 2 µm minimal and 5–10 µm exceptional (hastings.txt 42625-42631; values from PDF p716).
   - A hand layout also routes the gates of ABBA members out identically (r21).
3. **Density at minimum contacted pitch.** A hand layout merges gate contacts into a continuous strap and runs at the contacted pitch of 430 nm on sky130. Philis runs at 730 nm (AC-04).
4. **In-primitive matched routing.** ALIGN routes inside the primitive with an EM/IR-aware "fishbone" whose wire widths are optimised (cc_review.txt 449-452). CC routing mismatch is a known cost that two swapped rows remove (cc_review.txt 438-455). Philis leaves drain and source joins to the global router, except in the `mirror_pins` variant.
5. **Real passive devices.**
   - Capacitors: the process MIM (and multi-layer MOM), unit arrays for any matched set, split DAC with bridge, 3-dB and RC-aware routing with parallel wires (DACP 141-146, 177-180 and 600-625), and shields (Hastings 8.3.2 r8).
   - Resistors: 2-D CC arrays of identical segments, ratio by segment count, dummies at the exact pitch (Hastings 8.3.1 r5, r8, r9; hastings.txt 25218-25273).
   - Higher-order gradient cancellation: nth-order central-symmetric patterns (nth_order.txt 7-12, PDF p1; the abstract as extracted reads "2n unit cells for each device", exponent formatting unverified). Only first-order point symmetry and chessboard dispersion exist here.
6. **Bipolar and diode arrays.** A cross-coupled or 3×3/5×5 centred unit array with a dummy ring, identical square emitters and contact arrays (Hastings 10.3.1 r1, r3, r6, r14; 11.3.4 r4–r6). Philis gets only 1:8 right, and cannot be verified on sky130.
7. **Substrate protection.** Majority rings around victims, minority-carrier collecting rings around injectors and double rings (Hastings §5.4.4), rings grounded separately (Charbon §8.8), and taps within the latch-up distance. Philis draws only bulk-type single rings and one tap strip.
8. **Matching grade as an input.** Hastings keys dummies, moat, WPE, extension, poly and metal keep-outs to minimal, moderate or exceptional (hastings.txt 42321-42348 and r12–r23). `Unitization` carries only booleans (`dummy_required` and the like), so no generator can scale to the grade (AC-22).
9. **Verification closure.** A shipped generator should be LVS-closed: device type, value and multiplicity. Philis skips capacitors and inductors, does not compare resistor or diode values, and has no BJT recogniser on sky130. Cells therefore can be wrong silently (AC-01, AC-02, AC-03, AC-08).
10. **Per-variant figures of merit for the placer.** `CapArray::metrics` exists but is unused. The MOS variants report no figure: centroid offset, junction C, gate R and strap R are not given to the cost function.

---

## 4. Ranked findings

| ID | Sev | file:line | Finding | Fix direction |
|---|---|---|---|---|
| AC-01 | critical | cap_array.rs:417,513-514; capacitor.rs:13-26; cellgen.rs:844-848 | Capacitors are drawn as met1/met2 plates (≈0.14–0.15 fF/µm², *computed*) while benchmarks size W×L at 2.0 fF/µm² (`fixtures.rs:556`). The deck's MIM (`capm`, sky130.deck:565) is never drawn, and LVS skips capacitors. The capacitor is ~14× too small and nothing checks it. | Add a per-deck capacitor recipe table (like `resistors.recipes`): MIM for sky130 with capm/met3/met4/via3, and a multi-layer MOM elsewhere. Draw the unit array in the recipe's layers. Emit LVS capacitor cards with the area parameter. |
| AC-02 | critical | resistor.rs:81-82,101,304-310; diode.rs:89-119; cellgen.rs:887-891 | Resistor and diode ignore `dev_nf` (m or nf). R is m× too large and diode area is 1/m, and LVS passes because it emits one card with no parameters. | Draw `dev_nf` parallel units per member (resistors: parallel segment strings of identical segments). Emit one reference card per unit with w/l. Add a test with m = 2. |
| AC-03 | critical | inductor.rs:41-65 | Every turn is anchored at (0, 0), so all turns merge into one met1 conductor. The li centre tap has no via, so N floats. `turns = Σ dev_nf`. There is no LVS. | Rewrite as a true spiral on the top thick metal with an underpass through vias. Or remove inductors from the supported set and reject them at parse time. |
| AC-04 | high | mosfet.rs:344,393-402,657-669 | Isolated per-finger gate pads force pitch = 2·sd_w + L: 730 nm against 430 nm hand-layout pitch (sky130, L = 150, *computed*). Row length is ~1.7× and S/D junction area ~2×. | Draw one continuous contacted poly bar per device (strap height = pad height, cuts at the licon pitch), and set pitch = max(contacted pitch, m1 pitch). |
| AC-05 | high | mosfet.rs:78,781-788; builder.rs:271-292; cellgen.rs:138-143 | For unequal `dev_nf`, `greedy_centroid` puts two members across a drain. cellgen rejects every such variant, so ratioed mirrors are never merged or centroided (e.g. [2,4] has the legal order A BBBB A). | Replace the order generator with a diffusion-parity-aware search: ALIGN's Mhalf Euler path, or exhaustive and DP search on small counts with pair tokens. Keep `greedy_centroid` only for resistors. |
| AC-06 | high | post_cell.rs:310-323; annotator constraints.rs:66-84; post_cell.rs:539-546 | Collecting ring types (Hcgr/Ecgr) are drawn as bulk-polarity majority rings. There are no minority-carrier collecting rings and no double rings (Hastings §5.4.4). | Split the ring kinds. Keep the bulk tap ring (the majority/blocking ring). Add an ECGR (nwell + n+ ring tied to VDD) around NMOS injectors and an HCGR (p+ ring, grounded) for PMOS injectors. The annotator chooses by aggressor or victim role. |
| AC-07 | high | mosfet.rs:567-572; sky130.json:85; pdk.rs:1068 | There is one tap strip, on one side. `tie_max_dist_nm` = 3000 is never read, yet 10 µm fingers leave diffusion more than 10 µm from a tap. No LU rule exists in the deck. | Add a tap at the bottom (or in both rows) whenever `finger_w + gap > tie_max_dist_nm`. Add a cell-level assertion over the whole diffusion. |
| AC-08 | high | bjt.rs:50-68,127-141,208-231 | There is no BJT recogniser in the pinned sky130 deck. Equal pairs are not cross-coupled ([2,2] gives AA/BB). No Unit records or dummy units exist, and each NPN unit gets its own dnwell. | Emit Units. Use a CC unit-array algorithm with a dummy ring; share the dnwell per matched group where collectors allow. Gate BJT support on a deck recogniser. |
| AC-09 | high | finfet.rs:88-194 | FinFET has one variant: no dummies or edge dummy gates, no diffusion sharing, no CC, `sa = sb = 0`, silent W floor, no tests. Every MOS on a fin deck uses it. | Port the planar row logic (Cc1d, mirror, two rows, dummies) onto the fin grid. Record SA/SB and report the W error. |
| AC-10 | high | cellgen.rs:815-833; bjt.rs:34; resistor.rs:23 | An empty enumeration becomes an empty macro, so the device silently disappears from the layout (NPN on gf180 or ihp, a resistor on a deck without rpoly). | Make "no variant" an error with the device name, or a documented fallback. The cells should return a reason. |
| AC-11 | medium | mosfet.rs:364-380,516,537-552 | Split gates give A and B fingers unequal poly overhang, and dummy poly differs from active poly (Hastings r21). | Route both devices' gates out both ends identically, e.g. split pads per end on alternate devices plus metal straps. Extend dummies the same distance. |
| AC-12 | medium | mosfet.rs:428-436 | A poly strap is kept under the li strap (Hastings r22 asks for metal straps). | Drop the poly strap where the li strap and per-finger cuts exist, at least for matched classes. |
| AC-13 | medium | mosfet.rs:54-56,293,629 | Moat and WPE have one tier (3000 nm in every deck), under Hastings' 5 µm (moderate) and 10 µm (exceptional). The moat applies only to multi-device rows. No NMOS WPE in-cell. One dummy per end is fixed. | Choose moat, halo and dummy count by matching grade from the tiered deck keys (`lod_moat_ext_nm`, `wpe_clearance_nm`). |
| AC-14 | medium | resistor.rs:83,170 | Dummy pitch ≠ segment pitch whenever the resistor-to-poly spacing exceeds the segment gap (sky130 480 vs 400). Hastings 8.3.1 r9 requires them equal. | Set `seg_gap = max(res_seg_gap, poly.9)`, or make the dummies of the same material so the same gap is legal. |
| AC-15 | medium | resistor.rs:27-41,302-310; constraints.rs:30-44 | Resistors get only a 1-D greedy interleave with equal segments. No 2-D CC, no ratio by segment count, and L-ratioed resistors are never grouped. Multi-segment LVS is unverified. | Unitize resistors by (material, width, segment length) with counts per member, then build a CC segment array. Emit reference cards per segment, or merge series resistors in LVS. |
| AC-16 | medium | cap_array.rs:77-116,448,792 | No split DAC or bridge capacitor, no non-unit capacitor sizing, no shield, single-width tracks, `metrics()` unused, 6 % ratio tolerance. | Add split-DAC support (DACP §II-B, III-D), a shield plate option and MSB wire widening. Feed `metrics` into the variant price. Tighten the test to ≤ 1 %. |
| AC-17 | medium | capacitor.rs:23-26,178-186 | The merged plate includes unit gaps, so C ≠ n·C_unit. The no-via zone between plates is not exported to the router. | Route all multi-unit capacitors through the array. Emit a blockage shape per cut layer inside the plate stack. |
| AC-18 | medium | diode.rs:15-18,67,92-119 | One cut per terminal, Interdig = flip, no Units. The gf180 well form is unusable, and the anode tap is 330 nm in x (CRATES #7). | Use contact arrays scaled to area and a CC unit array for matched diodes. Resolve licon.7 against magic. |
| AC-19 | medium | cell_selfcheck.rs:283-345,362-408; lib.rs:159-165 | Cross-deck tests are release-only and pass vacuously on empty enumerations. testkit labels G, S and B common. No value or multiplicity tests. | Assert variants are non-empty, run the cross-deck tests in CI (release), add LVS-with-parameters tests per generator, and label per member in DRC sweeps. |
| AC-20 | medium | mosfet.rs:458-482 | No in-cell S/D or gate routing (fishbone), so matched interconnect R depends on the global router. | Add optional in-cell drain and source straps (a comb on the first routing metal) with widths from the current budget. Mirror them per member. |
| AC-21 | medium | lib.rs:45-52; cap_array.rs:357-397; cellgen.rs:327 | Generators export no per-variant figures of merit: centroid offset, junction C, gate R, strap R. `CapArray::metrics` exists but is unused. The placer sees only an isolated routing price. | Add a `figures()` method to `Cell` (or fields on `Macro`) and feed it into the variant price and the constraint residual. |
| AC-22 | medium | `analog::cell::Unitization` (as used in `builder.rs:233-267`) | There is no matching-grade input, so dummies, moat, halo, extension and keep-outs cannot scale per Hastings. | Add `grade: Minimal|Moderate|Exceptional` to Unitization, set by the annotator. |
| AC-23 | low | post_cell.rs:17,79-137,299-302,337-367 | Merged rings share one net (Charbon's dual rings are grounded separately). `tap_pitch_nm` and `enclosure_complete` are unused. Ring R counts cuts only. | Use per-victim nets or a separate ring option. Honour `tap_pitch_nm`, or delete it. |
| AC-24 | low | mosfet.rs:81-85,737-739 | A chain needs all-odd nf. CC for n > 2 needs nf % 4 == 0. | Allow an even first or last chain member. Admit n > 2 orders found by search (with AC-05). |
| AC-25 | low | mosfet.rs:537,560; finfet.rs:193; bjt.rs:27-37; builder.rs:30-31; post_cell.rs:483 | Literal `-10` nm; stale comment; `- 0`; misleading `_` parameter names; misplaced doc; no-op filter. | Replace the literal with `lat`; clean up the rest. |
| AC-26 | low | lib.rs:36-37; resistor.rs:302-307; diode.rs:92-94 | `Pattern::Interdig` has three meanings. | Give each generator its own variant enum, or rename. |
| AC-27 | low | LAYOUT-FUNDAMENTALS.md:62,68,78,88,99; CRATES.md #8,#10; cellgen.rs:844-848; pdk.rs:1038-1071 | Docs are stale against the code: contacts, chain, resistor dummies, BJT order, two rows, and the claim that sky130 has no capacitor recogniser. `REQUIRED_RULES` demands keys no generator reads (`bjt_*`, `tie_max_dist_nm`). | Refresh the docs. Align `REQUIRED_RULES` with what the code reads. |
| AC-28 | low | mosfet.rs:44-47 | The doc cites Hastings r22 for gate R, but r22 is about etch uniformity. | Fix the citation (gate R belongs to folding, `cellgen.rs:609-612`). |
| AC-29 | low | builder.rs:9-28 (Rect-only) | Rectangles only: no 45° or octagonal shapes, so there are no Hastings MOS r24 diagonal PMOS, circular or octagonal emitters, or octagonal spirals. | Add polygon support if RF or bipolar precision becomes a goal. |

---

## 5. Questions the planners must resolve

1. **Capacitor device per deck.** Should cells draw the process MIM (sky130 `capm`, recognised by the pinned deck at line 565) and emit LVS capacitor cards with area? Or is MOM the target, in which case the sidecar's `cap_density_ff_um2` and the benchmark sizing must change to the MOM density?
2. **Multiplicity semantics.** For resistors and diodes, does `m`/`nf` mean parallel identical units (Hastings-style unitization)? Should LVS compare value and area parameters, or only device count?
3. **Resistor segmentation in LVS.** Should each segment be a reference card (`segments × m` cards), or should the extractor series-merge them? This decides AC-15 and CRATES #6.
4. **BJT scope on sky130.** Is the target sky130 PNP a fixed-geometry model? The sources in `docs/ref/` do not say; confirm against the PDK documentation. If it is, the generator should emit only the fixed cell. When will GPurify recognise bipolars?
5. **Guard ring taxonomy.** Rename the current rings to base/blocking rings and add collecting rings? Which nets bias an ECGR (VDD) and an HCGR (ground)? Who decides aggressor versus victim: annotator `Isolation` or `NetClass`?
6. **Matching grade.** Where does minimal, moderate or exceptional come from: annotator block type, user annotation, or a performance budget? Which deck keys carry the tiers (`lod_moat_ext_nm`, `wpe_clearance_nm` exist only in sky130 and generic_finfet)?
7. **Pitch fix.** Changing to a continuous gate bar changes every MOS macro and the cellgen fold aspect. Is a benchmark re-baseline acceptable? Does any deck forbid a contacted poly bar next to the channel (npc.4 90 nm to gate on sky130)?
8. **Latch-up.** Is `tie_max_dist_nm` = 3000 authoritative for sky130 (its source is not recorded in the sidecar)? Should the fix be a second tap row or a guard ring?
9. **In-cell routing ownership.** Should cells own drain, source and gate straps (ALIGN-style fishbone), or should the router receive matched-route constraints per member?
10. **FinFET.** Is `generic_finfet` a delivery target? If so, which deck rules define edge dummies and diffusion breaks? None are in the sidecar.
11. **Tests.** Can CI run the release-only cross-deck tests (`cell_selfcheck.rs:283`, `cell_selfcheck.rs:314`)? Should the generator tests also run on gf180 and ihp in debug?
12. **Inductor.** Fix it (top metal, underpass, shield) or drop inductor support until a recogniser exists?
13. **Empty-enumeration policy.** Should a device with no legal variant be a hard error? Today it is a silent empty macro (AC-10).
