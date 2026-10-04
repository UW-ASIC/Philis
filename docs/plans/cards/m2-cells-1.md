# M2 cells — cards, segment 1

Base: `m2` at `c2940c6` (merge was a no-op). `dag-m2-m6.json`: CELL-11 has no hard deps (CELL-02, CELL-10, MAT-03 done).
Line numbers are at `c2940c6`.

## CELL-11 MOS: continuous gate bars at the deck's minimum contacted pitch — class: do

Spec: plan-03 `### CELL-11`. Step 4 (class gating) moves to CELL-12, see "Scope" below.

### Current code facts (plan corrections in **bold**)

- `sd_and_pitch` is **mosfet.rs:672-684** (plan: 657-669). It returns `(sd_w, max(2·sd_w + L, m1_pitch))`, where
  `m1_pitch = dim("mcon_size") + 2·dim("m1_enc") + dim("met1_space")` (:677). `dim("m1_enc")` = max(sidecar 65,
  deck max(m1.4 30, m1.5 60)) (builder.rs:213). sky130 at L = 150: sd_w = max(250, 440−150, 170+0+80+170−150, 170+2·40)
  = 290, pitch = 730. An interior S/D region is `pitch − L` = 580 wide.
- draw_row has a second copy of `m1_pitch` (**:274**), used only by the gate stub (`stub = 2·(m1_pitch − W/2 − poly_ext)`, :328).
- `sd_end = sd_w.max(ct + 2·(gate_space + lat/2))` (:279); `lat = cut_lattice = 2·grid` = 10 on sky130 (builder.rs:154).
- `pad_over` (:287) is the per-finger pad overhang, used only in the dummy clearance `sd_edge` (:292-296).
- Per-finger pads: `pad_w = max(L, ct + 2·licon_poly_side + lat)` (**:359**) = 340; drawn with cut and li in the loop at **:413-417**.
  The pad rows are `pad_y` (bottom) and `top_pad_y` (top, :373-376). `up(di) = split_gates && di == 1` (:379), with
  `top_end` / `bottom_end` at :381-382.
- Per-device poly strap of `poly_min_width` plus an li strap (**:435-452**). The unsplit mirror-pins row merges spans
  into one strap (:436-439). For `double_gate`, the second end gets an li strap and a second pin (:455-458).
- Gate pin: the first finger of each device, or the last finger of device 1 under mirror pins (:420-426).
- `cover_poly_cuts` (:663; builder.rs:63-112) draws one npc strip per cut row, so a bar row is covered automatically.
- Consumer: `cellgen.rs:677` `folds` reads the pitch (**plan said :661**). `gate_straps_stay_private` is at cellgen.rs:287 (**plan: 290**).
  It rejects variants whose drawn poly joins distinct gates. When enumerate returns no variant, cellgen `continue`s (cellgen.rs:149-151),
  and the members are drawn as separate cells.
- Deck values (sky130.deck): licon.1 170 (**:339**), licon.11 55 (**:349**), licon.8 50 / licon.8a 80 (:345-346), licon.14 190 (:352),
  poly.2 210 (:284). The m1 loop (m1.2 = 140) is at **:363-369**, m1.4 30 / m1.5 60 at **:391-392**. Sidecar `licon_to_gate_spacing` 55 (sky130.json:97).
- The DRC, ERC and extraction sweeps are sky130-only (`testkit::pdk`, lib.rs:63). Other decks are covered only through their own suites.

### Bar geometry (derived from the deck, sky130 values)

- The bar is horizontal. The 80 nm "opposite" poly enclosure (licon.8a) goes on y: `pad_h = ct + licon_poly_enc + licon_poly_side` = 300.
  The x enclosure is `licon_poly_enc` = 50.
- Interior gate gap at 430: 280 ≥ poly.2 210.
- A cut on a gate overhangs it by ≤ 60 (cut 170 centred on a 150 gate, +50). When both boundary fingers of two devices sharing a row have a cut,
  the two bars are 280 − 2·60 = 160 apart, which is less than 210. That is why one boundary finger drops its cut.
  With one cut dropped, the gap is 280 − 60 = 220 ≥ 210 (the cut snapped to the lattice adds at most `lat`).

### Edits (kernel/cells/src/mosfet.rs unless named)

1. New `fn m1_land(process: &dyn Process) -> i32`. It reads deck values only:
   `process.width("mcon") + 2·max(process.enclosure("met1","mcon"), process.endcap("met1","mcon")) + process.space("met1")`,
   each `unwrap_or(0)` (sky130 170 + 120 + 140 = 430). Doc: Hastings eq. 13.9.
2. `pub fn sd_and_pitch(process, gate_l) -> (i32, i32)` (keep the signature, :672). Update its doc comment:
   `sd_w = max(r("sd_width"), ct + 2·gate_space, ct + 2·diff_enc, li_col + li_space − gate_l, m1_land − gate_l)`, with
   `li_col = ct + li_enc + li_side` and `gate_space = max(r("licon_to_gate_spacing"), space_between("licon","poly"))`, as at :265.
   Return `(sd_w, sd_w + gate_l)`. sky130 at L = 150: (280, 430).
3. draw_row :274: `let m1_pitch = m1_land(process);` (the stub keeps its formula). Do not touch `dim("m1_enc")` in resistor.rs or the sidecar key (CELL-29).
4. Gate cuts. New pure function:
   `fn bar_cuts(seq: &[usize], same_row: impl Fn(usize, usize) -> bool, shared: bool) -> Option<Vec<bool>>`.
   - It returns one flag per finger: does this finger get a gate cut?
   - All `true` when `shared` (one bar for the row).
   - Otherwise, at each index `i` where `seq[i-1] != seq[i]`, `same_row(seq[i-1], seq[i])`, and the two devices' finger spans do not overlap
     (blocks or a chain; an interleave's bars merge into one poly island, so it keeps every cut and cellgen's filter decides):
     - clear `i` if `seq[i]`'s device keeps another cut;
     - else clear `i-1` if `seq[i-1]`'s device keeps another cut;
     - else return `None`.
   - The plan drops only the right-hand finger. The left-hand fallback is added so that `[2,1]`-shaped blocks still draw.
5. Split rows, generalised: `up(di) = self.split_gates && di % 2 == 1` (:379; same as today for pairs). A split Single or Chain row then
   alternates devices between the top and bottom rows, so neighbours never share a row.
6. enumerate (:69-136):
   - Factor `fn row_orders(&self, dev_nf: &[u16], n_dev: usize) -> Vec<Vec<usize>>` out of `draw` (:143-161: first row, plus the
     relabelled or reversed second row). `draw` calls it.
   - Keep a variant only if `bar_cuts` is `Some` for every row order, with `same_row = |a,b| !v.split_gates || a % 2 == b % 2` and
     `shared = v.mirror_pins && !v.split_gates`.
   - Single, any `n_dev ≥ 2`: also push `(Pattern::Single, true, false)`. The generator cannot see gate nets, so it offers both;
     `gate_straps_stay_private` and the search choose. The plan's "blocked distinct-gate pair only split" cannot be decided here.
   - Series Chain (:82-87): return the unsplit chain if it passes `bar_cuts`, else the split chain (`split_gates: true`).
     This keeps nf=1 cascode stacks merged instead of dropping them.
7. Gate drawing in draw_row (replaces :359-360, :391-417 pads and :435-452 straps):
   - Compute `cut = bar_cuts(sequence, same_row, shared).expect("enumerate filters")`.
   - In the finger loop, keep the gate and stub rects. Draw licon only where `cut[idx]`, at the existing `cut_x` / `cut_y`. Drop the
     per-finger pad and li rects.
   - Record per (device, row) the x span of gates and of cuts.
   - After the loop, for each (device, row), or one entry per row when `shared`:
     - poly bar `Rect { x: min(gx0, first_cut_x − licon_poly_enc), y: row_pad_y, w: …to max(gx1, last_cut_x + ct + licon_poly_enc), h: pad_h }`;
     - one li rect over its cuts, using the existing `strap` geometry (:442).
   - Rows: bottom `pad_y` for `bottom_end(di)`, top `top_pad_y` for `top_end(di)`.
   - `double_gate` therefore gets a top bar too. Delete the "a second poly strap would close the fingers into a ring" comment and keep
     the second pin on its last cut. If the extraction test then fails (poly loop), stop offering `double_gate` and report it; do not special-case.
   - Delete `poly_w` and `pad_w`.
   - Gate pin: the first finger of the device that has a cut, or for mirror-pins device 1 the last finger that has a cut.
8. Dummy clearance (:287, :292-296): replace `pad_over` with `bar_over = ((ct + 2·licon_poly_enc + lat − gate_l + 1) / 2).max(0)`,
   the bar's worst-case overhang past an end gate. Keep `skirt_over` as is.
9. Doc comments: update the struct doc (`split_gates` "device 1's gates" → odd-indexed devices) and the `sd_and_pitch` doc.
   Cite Hastings eqs 13.7–13.9.

### Scope

Step 4 (class gating) is CELL-12's: plan-03 CELL-12 change 2 "Gate connection by class" owns it. `MatchClass` does not exist yet
(grep: none; FLOW-06/MAT-07/EXT-12 add it). Until CELL-12 lands, every class gets the minimum bar. EXC isolated pads and MOD
bar ≥ 1 µm are re-added there. Their 550 nm pitch is T5's. Hand-off for CELL-12: make the bar's y offset one local
(`bar_gap`, 0 here) so that CELL-12 only sets it.

### Tests (mod tests in mosfet.rs; sky130 via `testkit::pdk()` / `verify::Pdk::builtin("sky130")`)

- `the_contacted_pitch_is_the_deck_minimum`: `assert_eq!(sd_and_pitch(&pdk, 150), (280, 430))`.
- `a_gate_bar_has_no_notch`. Fixtures: Nmos pair nf 2 and 8, single nf 4, triple nf 2 (`testkit::group_of`), every enumerated variant.
  For each gate-cut row (licon rects on poly with `y + ct ≤ 0` or `y ≥ W`), take the poly rects crossing the line `y = cut.y + ct/2`
  and merge them into x-runs. Assert:
  - each run is covered by one rect (`rects.any(|p| p.x <= run.0 && p.x + p.w >= run.1)`);
  - each run holds ≥ 1 cut;
  - runs per row == number of devices whose gates leave by that row, or 1 when `mirror_pins && !split_gates`.
- `a_row_is_shorter`: `group_of(Nmos, 2, 8, 1680, 150)`, `dummy_required = false`, first variant with `style == Cc1d && rows == 1`.
  Take the poly x where `h >= 1680`, sorted and deduplicated; every step `== 430`.
- `bar_cuts_drops_one_boundary_cut`: pure fn.
  - `bar_cuts(&[0,0,1,1], |_,_| true, false) == Some(vec![true,true,false,true])`;
  - `&[0,0,1]` → `Some([true,false,true])`;
  - `&[0,1]` → `None`;
  - `&[0,1,1,0]` (interleave) → all `true`;
  - `shared = true` on `[0,1]` → all `true`.
- `a_one_finger_pair_draws_split`: `group_of(Nmos, 2, 1, 1680, 150)`. The enumerate result is non-empty and every variant has `split_gates`.
  `testkit::dirty_group::<Mosfet>` is empty.
- Existing tests stay green, unedited: DRC/ERC sweep (:825), extraction (:863), `dummies_keep_the_finger_pitch`, two rows, split-gate
  privacy (:1171, :1201), mirror pins, `a_chain_shares_each_junction_and_is_clean`, `a_two_ended_gate_extracts_every_finger`, CELL-30 dummy tests.
- Commands:
  - `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`
  - `cargo test --release -p cells --lib mosfet && cargo test -p cells && cargo test --release -p library`
  - signoff: `cargo test --release -p benchmark --test signoff_fixtures`. Its BASELINE DRC/ERC/LVS rows must stay met; do not loosen any row.

### Acceptance and report

- T5: `sd_and_pitch(sky130, 150) == (280, 430)`. Interior S/D region 280 (was 580).
- Bench (`cargo run --release -p benchmark --bin bench`, background): an area and active % table per fixture, before (`c2940c6`) and after, in the item report.
- Report any fixture whose cell count rises, i.e. a group no longer merged because `bar_cuts` returned `None` on every variant.
