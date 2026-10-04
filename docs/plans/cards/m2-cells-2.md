# M2 cells — cards, segment 2

Base: `m2-cells` after merging `m2` at `be34be4` (merge commit `980d1d3`, no conflicts). `dag-m2-m6.json`: CELL-13 and
CELL-19 have no hard deps (CELL-13: D19, FLOW-05 done in M0; CELL-19: CELL-01 done). Line numbers are at `980d1d3`.

## CELL-13 Every diffusion within reach of its own tap — class: do

Spec: plan-03 `### CELL-13`, with 98-gap-critic D19 (depend on FLOW-05, not REL-06) and D36.

### Current code facts (plan corrections in **bold**)

- **`tie_max_dist_nm` is already 15 000 with its LU.2/LU.2.1/LU.3 source** (sky130.json:100-101). ihp_sg13g2 is 20 000
  (:106), generic_finfet 30 000. FLOW-05 did this, so the plan's "3000, no source" and its Risks no longer apply. The deck
  rules exist (`rule LU.2 tap_distance(lu_ndiff, ptap) <= 15um`, sky130.deck:528-530); **the bound is `<=`**.
  Registry: `k("tie_max_dist_nm", Nm, false, true, "unread")` (**sidecar.rs:137**; it is not required and not at pdk.rs:1068).
- One tap strip above the row: `tap_y0` is computed at mosfet.rs:565-571 and the strip is drawn at **:633-638**, spanning
  `diff_x_start..diff_x_end`, which is the diffusion's full x range, dummies and moat included. The farthest diffusion
  point is a bottom corner (y = 0), so the reach is `tap_y0`.
- **`tap_y0` is not affine in `finger_w`.** It is the max of four snapped terms built from about 20 `draw_row` locals
  (`licon_y` from `col_top`, the li-area growth, and the top pad rows when `split_gates || double_gate`). Plan steps 1-3
  (an analytic `tap_reach(p, finger_w, gate_l)` in `enumerate`) would duplicate those locals.
- Correction: every enumerated variant is drawn anyway. `draw_variants` → `draw_all` (**cellgen.rs:723-727, 753-758**)
  draws every `Mosfet::enumerate` result, and every MOS macro set goes through `draw_variants` (cellgen.rs:123, 132, 180).
  So the measurement is exact on the drawn macro and is filtered there. `stack_on_tap` (mosfet.rs:173-232) shares the
  strip between both rows, and a measurement on the drawn macro covers that case for free.
- `folds` reads `max_finger_width` as `w_max` (cellgen.rs:627).

### Edits

1. mosfet.rs, after `sd_and_pitch`:
   ```rust
   /// Worst diffusion-to-own-tap distance of a drawn MOS macro, nm: the max over
   /// every diffusion rect's corners (distance to a rect is convex, so a corner is
   /// the worst point) of the Euclidean distance to the nearest tap rect, where a
   /// tap rect is a `tap`-layer shape holding a `B` pin. `None` when no tap holds
   /// a `B` pin. LU.2/LU.3 measure the same thing on the final GDS.
   #[must_use]
   pub fn tap_reach(m: &Macro, process: &dyn Process) -> Option<i32>
   /// `tap_reach` ≤ `process.rule("tie_max_dist_nm", i32::MAX)`; a macro with no
   /// diffusion (the empty placeholder) passes.
   #[must_use]
   pub fn taps_in_reach(m: &Macro, process: &dyn Process) -> bool
   ```
   Diffusion rects are shapes on `req(process, "diff")` that are not tap rects, which handles decks that put diff and
   tap on one layer. Do the distance in i64 and use `f64::sqrt` once, rounded up.
2. cellgen.rs `draw_variants`, the planar MOS arm (:726):
   `{ let mut v = draw_all::<Mosfet>(group, c, pdk); v.retain(|m| cells::mosfet::taps_in_reach(m, pdk)); if v.is_empty() { v.push(Macro::default()) } v }`.
   The pushed default keeps `draw_all`'s "never empty" contract, because the placeholder is an empty macro. FinFet is
   out of scope: it has its own generator.
3. sidecar.rs:137: reader `"kernel/cells/src/mosfet.rs taps_in_reach, via frontend/library/src/cellgen.rs draw_variants (CELL-13)"`.
   In the `_source` strings of sky130/ihp_sg13g2/generic_finfet.json, replace "Read by no code yet (CELL-13 tap reach)"
   with "Read by cells::mosfet::taps_in_reach (CELL-13)." (sky130.json:101 and its siblings).
4. Plan step 3 (`max_finger_for_taps` and the `folds` clamp) is **not built**. The test below shows that every planar deck
   has `tap_reach(max_finger_width) ≤ tie_max`, so the clamp would never bind. If a future deck breaks that, the test
   fails, and step 2 reduces the device to the placeholder, which is loud. Add
   `// ponytail: no folds clamp; add max_finger_for_taps if a deck's tie_max < max_finger_width + pad offsets` at cellgen.rs:627.
   Plan step 4 (no second strip) stands.

### Tests

- mosfet.rs `tap_reach_is_the_far_corner`: `testkit::group_of(Nmos, 1, 1, 1680, 150)`, with dummies off. For each
  variant, `strip` is the tap-layer shape that holds the `B` pin. Assert `tap_reach(&m, &pdk) == Some(strip.y)`, since
  the diffusion is at y = 0 and the strip spans its x range. For `group_of(Nmos, 1, 4, 1680, 150)`, the `rows == 2`
  variant's reach equals the `rows == 1` variant's (the mirrored row has the same offset).
- mosfet.rs `every_diffusion_point_reaches_its_tap` (T8): for `Pdk::builtin` of sky130, gf180mcu and ihp_sg13g2, kinds
  N/P, W ∈ {420, 1680, 5000, `max_finger_width`}, L = the deck's `min_channel(pmos, "").0`, (n, nf) ∈ {(1,1), (2,1),
  (2,2), (4,4)}, and dummies {false, true}. Assert that every enumerated variant's drawn macro satisfies `taps_in_reach`,
  and collect failures into one message: deck, kind, n, nf, W, variant index, reach, tie_max. If a non-sky130 deck cannot
  draw some point, record that in the report. Do not drop the deck silently.
- cellgen.rs `a_tap_out_of_reach_drops_the_variant`: set up like the test at :1188-1194 with one nmos at W = 5000.
  `draw_variants` on sky130 gives all non-empty macros. With `pdk.rules.push(("tie_max_dist_nm".into(), 1000))` (Pdk
  `rule` reads `rules` in reverse, pdk.rs:1169-1172), the result is `[Macro::default()]`.
- Command: `cargo test -p cells mosfet && cargo test -p library cellgen`. Then `cargo test -p cells --test deck_keys`
  (registry reader).
- Acceptance: T8 by the test above. LU.2/LU.3 at 0 on bench fixtures is FLOW-05's deck rule (D36), and the bench run
  reports it.

## CELL-19 Per-variant junction and gate-resistance figures — class: do

Spec: plan-03 `### CELL-19`. Consumer: PERF-26 (`Figures.sd`, `Figures.gate_ohm`).

### Current code facts (plan corrections in **bold**)

- **`Figures` is at core/macro.rs:94-96** as `#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] pub struct Figures {}`.
  It is re-exported at core/lib.rs:26. `Macro` derives `Clone, PartialEq, Debug, Default` (no `Eq`, so f32 is fine).
  **Copy sites that must become `.clone()`**: core/macro.rs:137 (`place_macro`) and backend/dr/src/lib.rs:1276.
- `Builder::finish` (builder.rs:131-147) fills `figures` from `..Default::default()`. The plan's "lib.rs:45-52" for `Cell`
  is now lib.rs:45-53.
- `draw_row` (mosfet.rs:263-717) holds what the figures need. The region loop is at **:525-546**. Region 0 and region
  `n_fingers` are `sd_edge` wide (:339-349), inner ones `pitch − gate_l`, the diffusion is `finger_w` tall, and `term(idx,
  right_side)` (:398-408) names each side's terminal. Units at :459 carry `x = gx + gate_l/2` with `gx = sd_edge + idx·pitch`.
- `draw` (:155-169) builds both rows. `stack_on_tap` (:173-232) merges two macros through a fresh `Builder`, so it must
  carry figures over explicitly.
- sky130: `pex poly_c … sheet 48.2ohm` (**sky130.deck:631**) and `pex licon_po … sheet 152ohm` (**:636**). The plan's
  :582/:587 are stale. They are read by `sheet_ohm("poly")` and `cut_ohm("licon", "poly")` (pdk.rs:1180-1196), as
  `two_ended_gate_pays` already does (mosfet.rs:67-72).

### Edits

1. core/macro.rs:94-96:
   ```rust
   /// Per-variant figures (CELL-19); empty for generators that draw none.
   #[derive(Clone, PartialEq, Debug, Default)]
   pub struct Figures {
       /// (owner, "S"/"D", area nm², perimeter nm), sorted by (owner, terminal): BSIM AS/AD, PS/PD.
       /// A region whose two sides are different (owner, terminal) counts half to each; gate-side
       /// edges are excluded; regions beyond the outer dummy gates and the LOD moat are not counted.
       pub sd: Vec<(u8, &'static str, i64, i64)>,
       /// (owner, Ω): (R□_poly·W_f/(k·L) + R_cut)/N_f, k = 3 one-ended, 12 two-ended. Empty when the
       /// deck characterises poly or its gate cut not.
       pub gate_ohm: Vec<(u8, f32)>,
   }
   ```
   Then make macro.rs:137 `figures: m.figures.clone()` and dr/lib.rs:1276 `figures: cell.figures.clone()`.
2. mosfet.rs `draw_row`: before the region loop, `let mut sd: BTreeMap<(u8, &'static str), (i64, i64)>`. In the loop,
   region width `rw` = `sd_edge` at the ends and `pitch − gate_l` inside. Sides are `left = (sequence[region−1],
   term(region−1, true))` and `right = (sequence[region], term(region, false))`, each where present. If both sides are
   present and equal, the full figures go to that key; otherwise each side gets half. Area = `rw·finger_w`. Perimeter =
   `2·rw`, plus `finger_w` on an end region when `nd == 0` (its outer edge is the field, not a dummy gate). Halve both
   for a split region. At the end: `let mut m = b.finish(); m.figures.sd = sd.into_iter().map(|((o, t), (a, p))| (o, t, a, p)).collect(); m`.
3. `stack_on_tap`: after `out.finish()`, set `figures.sd` to the per-key sum of `a.figures.sd` and `b.figures.sd`
   (a BTreeMap again, so it stays sorted).
4. `Mosfet::draw`: after building `row0` or the stack, fill `gate_ohm`. When `(process.sheet_ohm("poly"),
   process.cut_ohm("licon", "poly"))` are both `Some`, then for each owner `di` with
   `n = orders.iter().flatten().filter(|&&d| d == di).count()`, push `(di as u8, (sq·W/(k·L) + cut)/n)`, where
   `k = if self.double_gate { 12.0 } else { 3.0 }` and `W, L = s.unit_w, s.unit_l`. Recursion through the split-gate
   fallback (:159-161) returns the inner draw's figures unchanged.
5. Other generators: no change (their `Figures` stays default).

### Tests (mosfet.rs; `e` = `min unit.x − L/2`, `pitch` = the gap between adjacent unit x's of one row)

- `sd_figures_add_up`:
  - `group_of(Nmos, 1, 1, 1680, 150)`, dummies off, the `Single` variant: `figures.sd == [(0,"D",e·1680, 2e+1680), (0,"S",e·1680, 2e+1680)]`.
  - `group_of(Nmos, 2, 2, 1680, 150)`, the `Cc1d` variant (unsplit, no mirror, 1 row), dummies on: Σ area over `sd` == `1680·(2e + 3·(pitch−150))`.
  - `group_of(Nmos, 1, 4, 1680, 150)`, the `rows == 2` variant: Σ area == `2·1680·(2e + (pitch−150))`.
- `a_shared_drain_halves_the_area`: `group_of(Nmos, 1, 2, 1680, 150)` `Single`, dummies off: AD = `(pitch−150)·1680`
  and PD = `2·(pitch−150)`; AS = `2·e·1680`.
- `gate_ohm_matches_the_formula`: `group_of(Nmos, 1, 4, 10_000, 150)` on sky130. The `double_gate == false` variant has
  `gate_ohm` `[(0, r)]` with `|r − 305.8| / 305.8 < 0.01` ((48.2·10000/450 + 152)/4, *derived*). The
  `double_gate == true` variant has `|r − 104.9| / 104.9 < 0.01` ((48.2·10000/1800 + 152)/4).
- Command: `cargo test -p pnr_core && cargo test -p cells mosfet && cargo test -p dr --no-run` (dr only for the
  `.clone()` change).
- Acceptance (PERF-26 carries AS/AD/PS/PD) belongs to PERF-26 (M4). CELL-19 delivers the figures under the names it
  reads.
