# M2+ cells, segment 13 (M6): CELL-21, CELL-20

Base: `m2-cells` after `git merge m2` (fast-forward to `4ae9deb`). Line numbers are at that commit. Commands: `export
PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`, then `Q=/home/omare/Documents/Projects/Rust/Philis/tools/qcargo`
from the worktree. The owner asked for the nix shell, so run them as `nix develop -c $Q ...` (the repo's `flake.nix` dev
shell). PLC-06 is not needed by either item.

## CELL-21 Split DAC and non-unit capacitors (drawing only): class **do**

Hard deps are on `m2`: MAT-18 (`analog::matching::dac::{attenuation_cap, nonunit_dims, split_dac_assign}`,
kernel/analog/src/matching/dac.rs:81, 90, 107) and EXT-19 (`annotator::passive::capacitor_sets` emits one `"split_dac"`
set with `PassiveSet::bridge`, passive.rs:118-170; the `splitdac` corpus netlist is inline in
backend/annotator/tests/corpus.rs:62-63, not a fixture file). The gate holds.

### Current code (corrects the plan)
- `bits()` is cap_array.rs:155-160 (plan: 123-130). `enumerate` 99-145, `build` 324-604, `mim_unit` 295-321. Every `d{i}:P`
  pin sits on one TOP lead (cap_array.rs:600-602), so the array has one top net.
- **The split set never reaches cap_array today.** `annotator::constraints::assemble` (constraints.rs:59-63) skips a set
  whose members differ in finger W/L; the bridge `CA` (3u×4u) differs from the 3u×3u units, so the set falls to the block
  fallback (constraints.rs:96-123): C0..C4 become one class unitization, CA another. cap_array then draws C0..C4 as a general
  set with one top lead; C0..C2 (`tl`) and C3..C4 (`tm`) share that pad, so `shared_pads_carry_one_net` (cellgen.rs:345)
  drops every variant and each capacitor is drawn alone.
- `MatchSpec` carries no bridge (intent.rs:98-117); member order comes from the grouping (sets.rs:293-321), not from
  `PassiveSet::devices`, so "bridge is last" cannot be relied on. Identify it structurally: the one member whose `P` and `N`
  nets are both other members' `P` nets.
- **Plan correction (equal edge sensitivity is impossible in this array).** Every cell has the unit's pitch, so a C_A half
  plate fits inside the unit plate box: W ≤ W_u and L ≤ L_u give 1/W + 1/L ≥ 1/W_u + 1/L_u, with equality only at the unit
  itself. `nonunit_dims` keeps k only for A ≥ 4/k_u², which is the unit area for a square unit (AM-GM), and such a plate is
  longer than the cell. For `splitdac`, C_A = 4/3 C_u, so a half is 2/3 C_u and `nonunit_dims(6, 3, 3)` = `None`. The
  plan's test `the_bridge_keeps_the_unit_edge_sensitivity` cannot pass for any C_A ≠ 2 C_u. As built, the half plate is
  the square of the half's capacitance, which is the least k any rectangle of that area can have. That k stays above k_u.
  The test asserts the square and the C, and the residual Δk is reported as a figure. Owner decision (escalate): a C_A
  that keeps k_u must be drawn outside the unit grid. That is a later item; `nonunit_dims` stays unused here.
- **Plan correction (top routing).** The deck's capm card takes `met4` as its top terminal (sky130.deck: `device capacitor
  capm ... terminals [met4, met3]`). The module doc says an uncontacted capm under no met4 is no device. So a met4 strap of
  one bank over a plate of the other bank would short or ambiguously bind that plate, and per-column straps cannot carry
  two top nets. Bank top lines run in the row gaps instead (edit 4).

### Edits
0. Coordinated edit in the annotator's file (the cells owner makes it, and it is reported to the annotator owner).
   backend/annotator/src/constraints.rs `assemble`, in the per-set loop (59-94): when `s.origin == Origin::PassiveSet {
   rule: "split_dac" }`, let `bridge` be the member whose `P` and `N` terminals are both `P` terminals of other members
   (from `netlist`). `same` and `d0` then read only the non-bridge members. The bridge keeps its `nf` (1). Nothing else
   changes. Test: in corpus.rs `split_dac_bridge_value`, also assert that one `c.unitization` entry holds all six `splitdac`
   devices.
1. cap_array.rs `Pattern`: add `Split { lsb: u8, bridge_top_lsb: bool }`. Doc: members ordered LSB bank `[1, 1, 2, …]`
   (termination first), then MSB `[1, 2, …]`, then the bridge; owner ids as in `split_dac_assign`; MSB bits =
   `members − lsb − 2`.
2. New `impl CapArray { #[must_use] pub fn split(group: &DeviceGroup, c: &Constraints, process: &dyn Process, lsb: u8,
   bridge_top_lsb: bool) -> Option<Self> }`. It is `None` unless all of these hold: the plate stack is MIM
   (`plate_stack(..).plate.is_some()`), the dummy ring is drawn (`unitization(..).dummy_required`), `bits(&dev_nf[..=lsb])
   == Some(lsb)`, the MSB counts are `[1, 2, …, 2^(M-1)]` with M ≥ 1, and the half side from step 3 is ≥
   `width("plate")` and ≤ `min(snapped unit W, L)`. `Cell::enumerate` never returns `Split`, because it cannot see nets.
3. Private `fn ca_half_side(process, unit_w, unit_l, lsb, msb) -> Option<i32>`: `ca = dac::attenuation_cap(1 << lsb,
   (1 << msb) - 1)`; `target = ca · c_u_af(process, unit_w, unit_l)? / 2` aF. For a square of side t µm, `u = t + dw`
   (`c_dw_nm`·1e-3) and `C = c_area·u² + 4·c_perim·u`. Take the positive root `u = (−4c_p + √(16c_p² + 4c_a·C))/(2c_a)`,
   so `t = u − dw`, then snap up to `cut_lattice`. `None` without `c_area_af_um2`. Mark it `// ponytail: square half; an
   off-grid C_A (keeps k_u) is the upgrade`.
4. `build`. With `Pattern::Split`:
   - slots come from `dac::split_dac_assign(lsb, msb, rows, cols)`, where `(rows, cols) = pattern::grids(&counts, 3.0)[0]`
     and `counts` = the banks' `dev_nf` plus `[2]`. Here `n = members − 1` and `general = false`, so ring dummies join slot
     0 (C0, the termination).
   - `mim_unit` takes `plate: Option<i32>`, the side of an override square. Owner `n` (the bridge) draws a capm of that
     side centred in its cell. The bottom plate stays the full cell, and the `Drawn` and the keep-out use the real plate.
   - The top is no column strap. Each unit has a met4 pad over its own capm (inset `encp`) with its via3 array, and a met4
     stub that leaves the pad through the cell edge to its bank's line. Gap line `k` (between interior rows `k−1` and `k`,
     `k = 0..=rows`) belongs to the LSB when `k` is even and to the MSB otherwise. A row has one line of each bank, so a unit
     goes up or down to its own bank's line. The bridge goes to the LSB line when `bridge_top_lsb`.
   - Line width is `jw`. In Split, `gap_y` grows to `≥ jw + 2·(space(st.top) − encp)` so that a line clears the pads it
     does not join.
   - The LSB lines join a met4 spine left of column 0 and the MSB lines join one right of the last channel. Each spine
     drops through `island` to its own met2/via1 lead and pin, like the existing TOP stack (cap_array.rs:580-602).
   - Pins: `d{i}:P` for i in `0..=lsb` on the LSB lead and for `lsb+1..=lsb+msb` on the MSB lead. The bridge's `d{n}:P` goes
     on the LSB lead when `bridge_top_lsb`, else on the MSB lead. `d{n}:N` keeps its own bus, and the router joins it to the
     other bank's top.
5. frontend/library/src/cellgen.rs. New `fn split_dac(netlist: &Netlist, rails: &[NetId], members: &[DeviceId], dev_nf:
   impl Fn(DeviceId) -> u16) -> Option<(Vec<DeviceId>, u8, bool)>`. It finds the structural bridge (as in edit 0) and
   takes the LSB as the bank on one bridge plate that holds a unit whose `N` is a rail (passive.rs's `terminated`). Each
   bank is ordered by `(count, !terminated, id)`. It returns `(lsb ++ msb ++ [bridge], L, P(bridge) == LSB top)`. In the
   merged loop (cellgen.rs:125-141), for `kind == Capacitor` with `Some(..)`, reorder `members` and draw
   `CapArray::split(..)` on the recipe overlay (as `draw_variants`, cellgen.rs:899-909). If that gives `None`, fall through
   to `draw_variants`. `bind_pins` (1114) works unchanged, because owner = member index.

### Tests
- cap_array.rs `a_split_bank_is_point_symmetric`: (L, M) ∈ {(3, 3), (2, 2)}. Counts are LSB ++ MSB ++ `[1]`, built with
  `mim_set(.., 3000)` and `dummy_required = true`, and drawn with `CapArray::split(..).expect("offered")`. Assert:
  - every owner draws exactly its count of `m.units`, and the bridge draws 2;
  - for every owner except 0, 1 and L+1, the set of unit centres is closed under 180° rotation about the units' bbox centre;
  - `testkit::findings(&m.shapes, &ports, pdk)` is empty, where `ports` from `ports_with(&m, &[])` relabels each `d{i}_P` to
    `TL`/`TM` by bank;
  - `verify::extract_spice` gives `2^L + 2^M + 1` `C` lines with exactly two distinct top nodes.
- cap_array.rs `the_bridge_is_the_attenuation_cap`: for (2, 2) and (3, 3), `2·c_u_af(half, half)` is within
  `2·(c_u_af(half+lat, half+lat) − c_u_af(half, half))` of `attenuation_cap(2^L, 2^M−1)·C_u`. Also assert `(W+L)/(W·L) ≥
  (W_u+L_u)/(W_u·L_u)` (Δk reported, see the correction), and that a (4, 1) bank gives `split(..) == None` (half > unit).
- corpus.rs `split_dac_bridge_value`: add the edit-0 assertion.
- frontend/library/tests/drawn_cards.rs `a_split_dac_signs_off`: the `splitdac` netlist as a sky130
  `XC.. sky130_fd_pr__cap_mim_m3_1 W=3u L=3u` subckt (CA `W=3u L=4u`), run through `library::run`. Assert `lvs_rows` is
  empty and that the cap cell's `m.drawn` has 9 capacitor cards (LSB 4 + MSB 3 + 2 bridge halves; derive it from the counts in
  the test). Risk: `a_mim_dac_signs_off_with_its_capacitors` fails at base with `lvs.floating_net`
  (card 12); if this one fails the same way, report it with that evidence and do not loosen it.
- Run: `nix develop -c $Q test -p cells cap_array`, `nix develop -c $Q test -p annotator --test corpus`,
  `nix develop -c $Q test --release -p library --test drawn_cards`.

### Acceptance
LVS clean, and C_A within one lattice step of MAT-18's value, are met by the tests above if they pass. The point-symmetry
test proves exact CC for every even-count cap; the MAT ledger reading is MAT's. "Equal edge sensitivity" is not met; it is
escalated with the proof above.

## CELL-20 Butted source–tap variant: class **do**

Hard deps are on `m2`: CELL-13 (`mosfet::{tap_reach, taps_in_reach}`, mosfet.rs:908-944, applied in `draw_variants`,
cellgen.rs:892) and CELL-19 (junction figures, mosfet.rs:637-680).

### Current code (corrects the plan)
- The bulk strip is mosfet.rs:765-822 (plan: 567-572): a tap row above the diffusion at `tap_y0`, its implant, cuts, li
  rail and one `d{i}:B` pin per member. The PMOS nwell (847-866) encloses the strip, and `stack_on_tap` (216) shares it
  between rows.
- S/D parity: `s0` is at 502, `term` at 507-516. Ends: region 0 is `term(0, false)` and region n is
  `term(n_fingers−1, true)`. With dummies (`nd > 0`, 449) the outer regions are already bulk-tied through the risers
  (703-760).
- `shared_pads_carry_one_net` is cellgen.rs:345 (plan: 276). It runs on merged cells only (152-155); singletons
  (190-201) get no filter, and they are the main source-on-bulk case.
- Deck: `licon.6` is sky130.deck:663 (plan: 605). Butted diff/tap is legal (deck 275), and `nsd.5a`/`psd.5a` exempt butted
  shapes (deck 181-184, 329-332).

### Scope cuts (ponytail)
- `Butted` is offered only for `rows == 1`, no dummies (`dummies_per_edge == 0` and `row_env(..)` `None`, so `nd == 0`),
  not `Chain`, and both end regions S. That covers singletons with even nf and Single-block rows of even counts.
- The plan's combined "Strip and Butted" variant is not built: a row past the reach is already dropped by
  `taps_in_reach` (cellgen.rs:892), and `Strip` stays offered.

### Edits
1. mosfet.rs: `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum TapStyle { Strip, Butted }`, and `pub tap: TapStyle`
   on `Mosfet` (47-58). Every `Mosfet { .. }` literal (11, all in mosfet.rs) gets `tap: TapStyle::Strip`.
2. `enumerate` (89-183): after the existing list, append one `Butted` copy of each `rows == 1`, `!double_gate` variant when
   `dummies == 0 && row_env(unitization(..), process).is_none()` and both ends of `row_orders(..)[0]` are S, by the same
   `s0`/`term` rule as `draw_row`. Lift `term`'s parity into `fn ends_are_sources(seq: &[usize], mirror_pins: bool) ->
   bool`. Appending last keeps the existing variant indices stable.
3. `draw_row`, when `self.tap == Butted`, skips the strip block (765-822) and draws instead:
   - a `tap` rect `w_t = snap(2·tap_enc + ct)` wide and `finger_w` tall, abutting the diff at `x = diff_x_start − w_t` and
     at `x = diff_x_end`;
   - its tap implant over `[tap.x − imp_enc, diff edge]`, with the channel implant ending at the diff edge on that side;
   - licon cuts in the tap at the S column's y positions, `tap_enc` inside;
   - one li rect joining the tap cuts to the region-0 (or region-n) S li pad;
   - each `d{i}:B` pin on a tap cut.
   The PMOS nwell takes the x span `[tap_l.x, tap_r.x+w_t]` and the y span `[0, finger_w]`, each grown by `nw_enc` (and
   then the existing min-width growth). Nothing that reads `tap_y0` runs, because there are no risers when `nd == 0`.
4. cellgen.rs: `fn butted_taps_share_the_source(m: &Macro, pdk: &Pdk) -> bool`, next to `shared_pads_carry_one_net`. If a
   `tap` shape touches a `diff` shape (zero gap, the abut), it requires `net(d{k}:S) == net(d{k}:B)` for every member k
   that has both pins. Otherwise it is `true`. Apply it in the merged retain (152) and on the singleton path after
   `bind_pins` (196-199).

### Tests
- mosfet.rs `a_butted_row_is_clean`: Nmos and Pmos, `testkit::group_of(kind, 1, 2, 1680, 150)` and `(kind, 2, 2, ..)`
  with dummies off. Filter `enumerate` to `tap == Butted` and assert it is non-empty. Each variant gives empty `findings`
  with `ports_with(&m, &["G", "S", "B"])`, the `B` labels renamed to `S` (one conductor by construction).
- mosfet.rs `butted_is_shorter_than_strip`: for the same group, the `Butted` variant's `bbox.h` < the matching `Strip`
  variant's `bbox.h`, and `tap_reach(&m, pdk).is_some()`.
- mosfet.rs `an_odd_row_is_not_butted`: nf = 1 and nf = 3 singletons offer no `Butted`.
- cellgen.rs `butted_needs_source_on_bulk`: `two_devices()` (S == B, nf = 2), group `[M1]`. `draw_variants` + `bind_pins`
  give at least one butted macro that passes the filter. With M1's `B` moved to net `vdd`, every butted macro fails it.
- Run: `nix develop -c $Q test -p cells mosfet`, `nix develop -c $Q test -p library --lib cellgen`, and
  `every_variant_is_drc_and_erc_clean` (it now covers the Butted variants of its nf = 2 singletons).

### Acceptance
Run `nix develop -c $Q run --release -p benchmarks --bin bench` before and after on the local fixtures. Report per circuit
the area and the DRC/LVS counts. Area is the search's choice, so the claim "falls for source-on-bulk devices" is measured,
not asserted. Any DRC or LVS regression is a failure to fix, not to waive.
