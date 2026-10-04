# M2+ cells, segment 3 (M3): CELL-17, CELL-18

Worktree `philis-m2/cells`, branch `m2-cells`, merged with `m2` at `4cb9e89` (one conflict in
`backend/verify/src/sidecar.rs`: kept HEAD's `tie_max_dist_nm` reader note from CELL-13 and m2's `vbe_tc_uv_per_k`
reader note from EXT-20).

| Item | Class | Why |
|---|---|---|
| CELL-17 | do | No hard deps (GAP-05 done in M1; REL-07 landed in m2 with the `ecgr_drawable` gate). Flow acceptance waits on GAP-03 injector tags; the drawing is tested directly. |
| CELL-18 | do | Uses `Problem.net_classes` (Supply/Ground) until EXT-18 refines the rank. |

Commands: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`, then `cargo test -p cells post_cell`,
`cargo test -p library --test drawn_cards`, `cargo test -p library cellgen`, `cargo test -p annotator rings`.

---

## CELL-17 Electron-collecting ring: an n-well band, not a filled well. Class: do

### Code facts (the plan is stale in places)
- `kernel/cells/src/post_cell.rs:216` `tap_ring(b, process, implant, in_well: bool, inner, gap, (ring_width, rows), pin) -> Rect`.
  Filled well at 277-283: one nwell rect `ox0-enc … ox1+enc` with `enc = dim("nwell_diff_enc")` (difftap.10 180).
  Other callers: `kernel/cells/src/bjt.rs:204` (`self.pnp`) and `:208` (`false`).
- `draw_ring` (204-210) passes `in_nwell(r.ring_type)`. `in_nwell` (326) is true only for `Tap { in_well: true }`, so
  today an `Ecgr` draws an n+ band (`implant_name` 315 → `nsdm`) with no well at all.
- `drawable` (334-342): `Ecgr => false` behind a `ponytail:` naming this item.
- `ring_gap` (300-304): the `Ecgr` arm is `max(base, space_between("nwell","diff"))` (340). **Plan is wrong that this
  keeps the band well clear**: it measures to the tap band, but the band's well grows `g` (≥ 180) inward of the band,
  so the well's inner edge sits only `gap − g` from the NMOS diffusion. sky130: base gap = 380 + 125 = 505,
  `505 − 210 = 295 < 340` → difftap.9 violation. The `width` parameter is already there (`let _ = width;`), unused.
- The well also has a hole: its inner width is `inner.w + 2(gap − g)`. For a small device that is under nwell spacing
  (`nwell_min_spacing` 1270 on sky130, nwell.2a), which is a notch error. The plan does not mention it.
- `ring_halo` (291-294) = `width + ring_gap + outer_clear + ring_implant_enc`. `outer_clear` (347-353) gives an
  `Ecgr` the well-less `ring_clear` (380), but the band well grows `g` outward and owes neighbouring wells 1270.
- Consumer: `frontend/library/src/lib.rs:1052` passes `drawable(Ecgr, pdk)` to REL-07 (`backend/annotator/src/rings.rs:77`),
  which requests an `Ecgr` only for `injector == Some(Electrons)`. `backend/annotator/src/lib.rs:389` passes
  `injector: &vec![None; n]` (GAP-03 not wired), so turning `drawable` on changes no fixture.
- REL-08 is cut (D20). Its "injector fixtures" acceptance cannot run until GAP-03 feeds `injector`. The unit DRC/ERC
  test stands in for it.

### Edits (`kernel/cells/src/post_cell.rs` unless stated)
1. Add the well shape next to `tap_ring`:
   ```rust
   /// The n-well a tap ring sits in: none (p+ in substrate), one rect over the ring and its interior (a PMOS's
   /// bulk tap, the PMOS shares it), or a band around the tap band only (an ECGR, whose interior is an NMOS).
   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   pub(crate) enum WellShape { None, Filled, Band }
   ```
   and `fn well_shape(t: GuardRingType) -> WellShape`: `Tap { in_well: true }` → `Filled`, `Tap { in_well: false }` | `Hcgr` → `None`, `Ecgr` → `Band`.
   Keep `in_nwell` (its tests and `outer_clear` read it). Add one more helper:
   ```rust
   /// How far an ECGR's band well grows past its tap band of `width`: the deck's well-over-diff/tap enclosure, and
   /// enough that the band well is `width("nwell")` wide (sky130 840 > 420 + 2·180).
   fn band_well_ext(process: &dyn Process, width: i32) -> i32 {
       dim(process, "nwell_diff_enc").max(process.enclosure("nwell", "tap").unwrap_or(0)).max((dim(process, "nwell_min_width") - width + 1) / 2)
   }
   ```
2. `tap_ring(b, process, implant, well: WellShape, inner, gap, (ring_width, rows), pin) -> Rect`. Replace 277-283:
   - `Filled`: today's rect, unchanged.
   - `Band`: `let g = band_well_ext(process, ring_width);` four nwell rects, full-length along x for bottom/top and
     full-height along y for left/right, each band grown by `g` on all sides:
     bottom `(ox0-g, oy0-g, ox1-ox0+2g, ring_width+2g)`, top `(ox0-g, iy1-g, …)`, left `(ox0-g, oy0-g, ring_width+2g, oy1-oy0+2g)`,
     right `(ix1-g, oy0-g, …)`. They overlap at the corners, so the union is one ring.
   - `None`: nothing.
   - `bjt.rs:204`: `if self.pnp { WellShape::Filled } else { WellShape::None }`; `bjt.rs:208`: `WellShape::None`. Its GDS stays the same.
   - `draw_ring` (209): `well_shape(r.ring_type)`.
3. `ring_gap`: drop `let _ = width;`. The `Ecgr` arm becomes
   `band_well_ext(process, width) + space_between("nwell","diff").unwrap_or(0).max((nwell_space(process) + 1) / 2)`,
   with `nwell_space = rule("nwell_min_spacing", 0).max(space("nwell").unwrap_or(0))` (the expression already in
   `outer_clear`; hoist it into a `fn nwell_space(process) -> i32` and call it from both). Half the well spacing per side
   means the hole is ≥ the well spacing even around a point device. `ponytail:` comment: this is conservative for large
   devices (sky130 gap 845 vs 550). Size the hole from `inner` if the area matters.
   Doc: the gap is measured to the band well's inner edge, not to the tap band.
4. `outer_clear(ring_type, process, width)`: `Ecgr` → `nwell_space(process) + band_well_ext(process, width)`. `ring_halo`
   passes the `width` it already has. The halo stays conservative because `width + 2·band_well_ext(width)` = `max(width+2e, 840)`
   does not decrease with width, and the halo uses the widest (point-device) band.
5. `drawable`: `GuardRingType::Ecgr => p.layer("nwell").is_some() && p.layer("nsdm").is_some()`. Delete the `ponytail:`.
   Update the doc comments on `implant_name` (313-314) and `in_nwell` (324-325): "drawn by CELL-17" → "drawn as a band ([`WellShape::Band`])".

### Tests (`kernel/cells/src/post_cell.rs` `mod tests`, sky130 via `testkit::pdk()`)
- Shared helper in the test module: `fn ecgr_around_nmos(pdk) -> (Macro /*cell*/, Macro /*ring*/, Rect /*inner*/)`. It builds
  `testkit::group_of(DeviceKind::Nmos, 1, 2, 1000, 150)` (about 2 µm × 2 µm), takes the cell from
  `mosfet::Mosfet::enumerate(..)[0].draw(..)`, sets `inner = cell.bbox`, and draws the ring with a `Builder` and
  `draw_ring(&mut b, &pdk, &req(0, 9, GuardRingType::Ecgr, false), inner, 15.0)`.
- `an_ecgr_well_never_covers_its_interior`:
  - Let `s = pdk.space_between("nwell","diff").unwrap()` (340). No ring nwell rect intersects `inner` grown by `s`.
  - Every ring nwell rect has `min(w, h) >= dim(&pdk, "nwell_min_width")` (840).
  - `testkit::findings(cell ∪ ring shapes, ports_with(cell, &["G","S","B"]) + the ring pins labelled "VDD", &pdk)` is
    empty, i.e. DRC and ERC are clean.
- `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos` (the GAP-05 test M1 left for this item):
  - Every ring `tap` rect is inside some ring `nsdm` rect and inside some ring `nwell` rect (containment, not intersection).
  - `drawable(Ecgr, &pdk)` is true.
  - No ring `psdm` exists.
- `hcgr_is_not_drawable_on_sky130` (571-577): the `assert!(!drawable(Ecgr, &pdk))` line becomes `assert!(drawable(Ecgr, &pdk))`.
  This is the spec'd behaviour change, not a loosened assertion. Keep `!drawable(Hcgr)`.
- `ecgr_ring_gap_clears_the_well` (581-617): on `Narrow` the `Ecgr` value changes from 400 to 450 (`g` 50 + max(400, 50)).
  The expected value goes up because the old arm measured to the wrong edge. Add a sky130 case:
  `ring_gap(&pdk, &req(..Ecgr..), 420) - band_well_ext(&pdk, 420) >= 340`.
- `tap_rings_draw_as_before` stays green as it is. Run `cargo test -p cells` for all of it, including `bjt` and
  `cell_selfcheck`, which cover the BJT rings through `tap_ring`, and `cargo test -p annotator rings`.
- Acceptance shortfall to report: no flow fixture carries an `Ecgr` until GAP-03 feeds `injector` (annotator `lib.rs:389`).

---

## CELL-18 Capacitor plate orientation by net impedance. Class: do

### Code facts
- Pins: `P` = top plate, `N` = bottom plate (`kernel/cells/src/capacitor.rs:37`, pins at 83-84). The plan's
  "capacitor.rs:37 SPICE order" is right about the binding, which `cellgen::bind_pins` (`frontend/library/src/cellgen.rs:966`)
  does by terminal name.
- **Stale in the plan:** a single sky130 MIM is drawn by `CapArray` as "a set of one" (`kernel/cells/src/cap_array.rs:105-110`;
  `cellgen.rs:754-760` tries `CapArray` first), not by `Capacitor`. Only a MOM single uses `Capacitor`. Both keep
  `P` = top and `N` = bottom (cap_array drawn nodes `[Pin("P"), Pin("N")]`, `cap_array.rs:319`), so the flip is
  generator-agnostic.
- A single device takes the path at `cellgen.rs:176-184` (`unit_of[i] == None`, no user macro). Merged unitizations
  (DAC banks, ≥ 2 members) bind at 137-139 and stay untouched.
- `Drawn.nodes` are `Node::Pin("P"|"N")` (`kernel/core/src/macro.rs:53-56`). They resolve through pin nets in
  `drawn_cards` (`cellgen.rs:897`), so swapping the pin nets also moves the LVS reference card. Note that LVS then
  checks geometry against the drawn card, not against the schematic's terminal order. A two-terminal C is symmetric,
  so this is correct, but LVS cannot catch a wrong flip. The pin-net assertion below is what catches it.
- `enumerate_folded` (59-67) takes `ground: Option<NetId>`; its only flow caller computes that from
  `problem.net_classes` (`frontend/library/src/lib.rs:1830-1833`); `enumerate` (53) passes `None`.

### Edits (`frontend/library/src/cellgen.rs`, plus one call site in `lib.rs`)
1. `enumerate_folded(…, fold, net_classes: &[analog::metadata::NetClassification]) -> Cells` in place of `ground`.
   Inside: `let ground = net_classes.iter().find(|c| c.class == NetClass::Ground).map(|c| c.net);` and
   `let rails: Vec<NetId> = net_classes.iter().filter(|c| matches!(c.class, NetClass::Supply | NetClass::Ground)).map(|c| c.net).collect();`.
   `enumerate` passes `&[]`. `lib.rs:1830` passes `&problem.net_classes` and drops its inline ground block.
2. Add:
   ```rust
   /// A plate net's impedance rank (H06-40, H08-25): 0 a rail (Supply/Ground), 2 a net that only reaches MOS gates
   /// besides capacitors (high-Z), 1 anything else. ponytail: class-only rank; EXT-18's net classes refine it.
   fn plate_rank(netlist: &Netlist, rails: &[NetId], net: NetId) -> u8
   ```
   For the gate-only test, every `(device, terminal)` on `net` satisfies `(kind ∈ {Nmos, Pmos} && t == "G") || kind == Capacitor`.
3. Add:
   ```rust
   /// Put the bottom (`N`, high-parasitic) plate on the lower-impedance net: swap the bound nets of every `P` and `N` pin.
   fn swap_plates(m: &mut Macro)
   ```
   Take the `P` net and the `N` net from the pins, whose names end in `:P`/`:N` or are bare `P`/`N`, then reassign each.
4. In the single-device branch (after the `bind_pins` loop at 181-183):
   `if d.kind == DeviceKind::Capacitor { if let (Some(p), Some(n)) = (terminal(d,"P"), terminal(d,"N")) { if plate_rank(netlist,&rails,p) < plate_rank(netlist,&rails,n) { alternatives.iter_mut().for_each(swap_plates) } } }`.
   Doc line on `enumerate_folded`: single capacitors bind their bottom plate to the lower-impedance net (CELL-18).

### Tests
- `frontend/library/tests/drawn_cards.rs::a_decoupling_cap_puts_its_bottom_plate_on_the_rail`:
  - Netlist:
    `.subckt t VDD VSS g\nXM1 VSS g VSS VSS sky130_fd_pr__nfet_01v8 w=1u l=0.15u\nXC1 VDD g sky130_fd_pr__cap_mim_m3_1 W=5u L=5u m=1\n.ends t\n`
    run with `library::run` and `Config { feedback_iters: 1, ..Default::default() }`.
  - Find the cell whose `devices_of` holds XC1's id; it is the macro with a pin ending `:N`.
  - Assert: that pin's net is `VDD` (by name, through `sol.netlist.nets`), and the `:P` pin's net is `g`.
  - Assert: `lvs_rows(&sol, &pdk)` is empty.
- Unit test `cellgen.rs` `mod tests::plate_rank_orders_rail_signal_gate`: a netlist with a rail, a drain net and a
  gate-only net gives ranks `0, 1, 2`. A cap between two signal nets is not swapped (equal rank).
- Regression: `cargo test -p library --test drawn_cards` (`a_mim_dac_signs_off_with_its_capacitors` stays green, since
  merged banks are untouched) and `cargo test -p library cellgen`.
- Acceptance: every single capacitor with exactly one rail terminal has `N` on the rail. The test covers the MIM case.
  A MOM single goes through the same branch.
