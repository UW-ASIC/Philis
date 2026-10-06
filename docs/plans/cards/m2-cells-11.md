# M2+ cells, segment 11 (M2): CELL-11, CELL-12, CELL-13, CELL-19

Base: `m2-cells` at `b062863`. `git merge m2` was a no-op (already up to date). Line numbers are at `b062863`
(`git show HEAD:kernel/cells/src/mosfet.rs`). The worktree also holds **uncommitted GAP-19 WIP** (segment 10:
`frontend/library/src/cellgen.rs`, `frontend/library/src/lib.rs`, `kernel/cells/src/mosfet.rs`). This card does
not touch or commit it, and its working-tree line numbers differ from the ones below.

## CELL-11, CELL-13, CELL-19: already merged, no work

| item | commits (all ancestors of `m2` and HEAD) | card |
|---|---|---|
| CELL-11 | `b8f6628`, report `33ea606` | m2-cells-1.md. Status was **escalate**: the router's met1 pin access at 430 nm S/D columns (m1.2 on dac4, dac4_mim, res_m2, tq_chain). That is a routing item, not cells |
| CELL-13 | `c60b19b` (`taps_in_reach`, `max_finger_for_taps`, `folds` clamp) | m2-cells-2.md |
| CELL-19 | `ed9da17` (`Figures.sd`, `Figures.gate_ohm`, mosfet.rs:191-198, 597-622) | m2-cells-2.md |

Checked facts: `sd_and_pitch` (mosfet.rs:826) and its test `the_contacted_pitch_is_the_deck_minimum` (:1643),
`every_diffusion_point_reaches_its_tap` (:1774), `sd_figures_add_up` / `a_shared_drain_halves_the_area` /
`gate_ohm_matches_the_formula` (:1818-1878). The only thing left is verification:
`cargo test --release -p cells` (with `PDK_ROOT` exported).

## CELL-12 Matching class drives dummies, LOD moat, well clearance, gate overhang and gate straps (class: do)

Spec: plan-03 `### CELL-12`, plus D17 (`mos_env` comes from GAP-01, not MAT-07) and C16 (`Option` class; readers
use `unwrap_or(Moderate)` only for matched cells). DAG: hard deps CELL-11, GAP-01 and EXT-12 are all in. EXT-16 is
step-level only, so the class tests are synthetic.

### Current code facts (corrections to the plan)

- GAP-01 has landed: `analog::matching::class::{mos_env, MosEnv, GateStrap}` (`kernel/analog/src/matching/class.rs:37-77`).
  `MosEnv { dummy_reach_nm, moat_nm, wpe_nm, gate_ext_extra_nm, gate_strap }`. `GateStrap::{PolyBar (MIN), PolyBarFar (MOD), MetalIsolated (EXC)}`.
  The tiers come from `pdks/sky130.json:17-24`:
  - `wpe_clearance_nm` = [2000, 3000, 5000]
  - `lod_moat_ext_nm` = [3000, 5000, 10000]
  - `dummy_reach_nm` = [0, 3000, 10000]
  - `gate_ext_extra_nm` = [0, 1000, 1000]

  The plan's "MAT-07 applies max(5000, 2·n_well_depth)" is stale: the EXC well value is the tier's 5000, and
  `n_well_depth` is read by no code.
- EXT-12 has landed: `Unitization.class: Option<MatchClass>` (`kernel/analog/src/cell.rs:69`). The annotator sets
  `Some(s.class)` on every matched set (`backend/annotator/src/constraints.rs:90`) and `None` on block groups (`:125`,
  `dummy_required: true, route_matching_required: true`). `testkit::group_of` (`kernel/cells/src/lib.rs:70-87`) uses
  `None`, both flags `false`.
- `draw_row` (mosfet.rs:330) already does the following:
  - `matched = u.devices.len() > 1` (:343).
  - `nd = self.dummies_per_edge` (:385). `dummies_per_edge` is set in `enumerate` from `dummy_required` (:83) and
    recorded per drawn dummy via `b.dummy` (:676). cellgen's LVS reference reads `Macro.dummies`, never the field.
  - Moat: `if n_dev > 1 && nd > 0 && !Chain { tier("lod_moat_ext_nm", Minimal) }` (:425), with ponytail note :424.
  - The PMOS halo: `if matched { tier("wpe_clearance_nm", Moderate) }` (:782), with ponytail note :781.
  - The gate rect is `y = -poly_ext`, `h = finger_w + 2·poly_ext` (:516). The dummy rect is the same (:668). The skirt
    is at `skirt_y = finger_w + poly_ext - 10` (:689). The dummy cut row `licon_y` floor is
    `finger_w + max(polycon_gap, poly_ext - lat + licon_poly_side)` (:645).
  - The bottom bar is at `pad_y = snap_cut(-(poly_ext + stub + bar_gap), lat)` with `bar_gap = 0` (:456-457, the CELL-11
    hand-off). `pad_h` is defined later (:483). The top bar mirrors the bottom one (`top_cut_y`, :497). The split-row
    stub term `ct + 2·lps + poly_min_spacing` (:452) puts the pad a poly space below the neighbour's free gate end.
  - Gate cuts: `cut = bar_cuts(..)` (:509) drops the boundary cut. There is one bar per (row, device) (:571) and one
    li strap per bar (:574).
- `cellgen` reads `sd_and_pitch` for the fold aspect. CELL-12 leaves `sd_and_pitch` unchanged, so EXC's wider pitch
  is local to `draw_row` and the `folds` aspect does not see it. That is a known ceiling, noted in code.

### Exact edits (kernel/cells/src/mosfet.rs only)

1. Imports: add `use analog::matching::class::{mos_env, GateStrap, MosEnv};`.
2. Add a new free fn near `dummies_per_end`:
   ```rust
   /// The environment `u`'s matched set asks of a MOS row (Hastings §13.3 rules 12, 19, 21, 22 via
   /// [`mos_env`]): a multi-device, non-series unitization that wants dummies or matched routing, at its
   /// class, `Moderate` when unset (C16). `None` = a singleton, a series stack or a plain parallel group:
   /// today's unmatched row.
   fn row_env(u: Option<&analog::cell::Unitization>, process: &dyn Process) -> Option<MosEnv>
   ```
   Body: `u.filter(|u| u.devices.len() > 1 && u.series_parallel != SeriesParallel::Series && (u.route_matching_required || u.dummy_required)).map(|u| mos_env(u.class.unwrap_or(MatchClass::Moderate), process))`.
3. `draw_row`:
   - Add `let env = row_env(u, process); let e = env.unwrap_or_default();`. Replace `matched` (:343) with
     `env.is_some()` where it is used.
   - EXC pitch (`e.gate_strap == GateStrap::MetalIsolated`): after :374, compute
     `pad_w = ct + 2·licon_poly_enc + lat` (worst case for a lattice-snapped cut) and
     `pitch = lattice_up(pitch.max(pad_w + poly_space))`, where
     `poly_space = r("poly_min_spacing",0).max(process.space("poly").unwrap_or(0))`. Then set `sd_w = pitch - gate_l`.
     Derived for sky130: (170+100+10)+210 = 490. This needs the `licon_poly_enc` binding before :374, so move it up
     if needed.
   - `nd` (:385):
     - After `sd_edge` and `d_step` are known (:417), set the final `nd` as the smallest value that satisfies two
       conditions:
       - `nd ≥ dummies_per_edge`;
       - `nd ≥ 1` if `e.moat_nm > 0 || e.dummy_reach_nm > 0` (the moat sits past a bulk-tied dummy, so no signal
         junction grows).

       It must also satisfy `sd_edge - sd_end + nd·d_step ≥ e.dummy_reach_nm`, i.e.
       `nd = max(that, ceil((reach - sd_edge + sd_end) / d_step))`. This is the outer dummy poly edge to the
       outermost active gate edge, PERF-23's r12 measure.
     - `sd_edge` depends only on `nd > 0`, so compute `let dummied = nd0 > 0 || e.moat_nm > 0 || e.dummy_reach_nm > 0;`
       first, use it in `sd_edge`, then compute `nd`. Change `self.dummies_per_edge` uses inside `draw_row` to that
       local `nd` (the dummy loop :662 already uses `nd`).
   - Moat (:425): `let moat = if env.is_some() && self.style != Pattern::Chain { (e.moat_nm - (sd_edge + nd * d_step)).max(0) } else { 0 };`.
     Delete the ponytail line :424. Comment: diffusion end ≥ `moat_nm` past the outermost active gate edge (rule 12;
     §13.2.2 for MIN).
   - Gate overhang (rule 21), with `ext = poly_ext + e.gate_ext_extra_nm`:
     - Active gate rect (:516): `y: -ext, h: finger_w + 2·ext`.
     - Dummy rect (:668): the same with `dummy_l`.
     - `skirt_y` (:689): `finger_w + ext - 10`.
     - `licon_y` floor (:645): replace `poly_ext` with `ext`.
   - Bar distance. Move `pad_h` (:483) above `pad_y`, delete `bar_gap`, and set
     `need = ext + if self.split_gates { poly_space } else { 0 }`, then
     `.max(if e.gate_strap == GateStrap::PolyBarFar { 1000 } else { 0 })`.
     The 1000 is Hastings §13.2.2: poly ≥ 1–2 µm from moat, hastings.txt L41125–41132. Then
     `pad_y = snap_cut(-(poly_ext + stub).max(need + pad_h), lat)`. The bar's near edge (`pad_y + pad_h`) then sits
     ≥ `need` from the diffusion, and the top bar mirrors it.
   - EXC gate straps (rule 22, `MetalIsolated`):
     - `cut` (:509) = `vec![true; sequence.len()]`, so every finger is contacted.
     - In the bar loop (:567-578), draw one poly pad per finger instead of one bar per (row, device). Each pad is
       `x0 = min(gx, cut_x - licon_poly_enc)`, `x1 = max(gx + gate_l, cut_x + ct + licon_poly_enc)`, y as the bar,
       `h = pad_h`. Record per-finger `(x0, x1)` in a `Vec` beside `bars`.
     - Keep the li strap per (row, device) unchanged. It joins the device's pads in metal: a gap of
       `pitch - ct - li_side - li_enc` ≥ li space to the next device's strap, 240 on sky130 at 490 (li space 170).
   - PMOS halo (:782): `let wpe_halo = e.wpe_nm;`. Delete the ponytail line :781.
4. `enumerate`: unchanged. `bars_fit` still filters EXC variants that would only fit as isolated pads. Leave a
   `// ponytail:` note naming that ceiling and the upgrade path: skip `bars_fit` when EXC.
5. Comments: replace "§13.3 r9" with "§13.3 rule 12" at :394 and :422.

### Tests (mosfet.rs `mod tests`)

The helper builds sky130 `testkit::group_of(kind, 2, 2, 1680, 150)` with `dummy_required = true` and
`class = Some(c)`, for `kind ∈ {Nmos, Pmos}` and `c ∈ [Minimal, Moderate, Exceptional]`. Expected values come from
`mos_env(c, &pdk)` and are never hard-coded, except that the sky130 sanity case below asserts the table.

- `class_scales_the_moat_and_dummies`:
  - For every enumerated variant, `diff.x_end - max(active channel x_end)` ≥ `env.moat_nm`, and the same on the left.
    The channel comes from `m.units` (`x ± gate_l/2`), and diff is the union of the `diff` layer.
  - For EXC, `outermost dummy poly outer edge - nearest active gate edge` ≥ `env.dummy_reach_nm`.
  - The sky130 sanity case asserts `[moat_nm] == [3000, 5000, 10000]`.
- `every_gate_overhangs_equally`: MOD and EXC, all variants including split. For each poly rect with `h ≥ finger_w`
  (gates and dummies):
  - `rect.y ≤ -(poly_ext + gate_ext_extra_nm)`;
  - `rect.y + rect.h ≥ finger_w + poly_ext + gate_ext_extra_nm`;
  - no other poly rect intersects the band `y ∈ (-(poly_ext+extra), 0)` or `(finger_w, finger_w+poly_ext+extra)`
    within that gate's x span (straight, not padded).
- `the_pmos_well_clears_by_class`: PMOS, each class. The min over channel rects of the distance from the channel to
  the nwell rect's nearest edge is ≥ `env.wpe_nm`.
- `a_singleton_has_no_environment`: one NMOS, `class = Some(Moderate)`, `dummy_required = true`. Its bbox equals the
  bbox with `class = None` and with the env forced off (`row_env` returns None for `devices.len() == 1`). Assert
  `row_env(Some(&u), &pdk).is_none()`, plus equal bboxes.
- `exceptional_gates_are_isolated_pads`: EXC pair. On each gate-cut row, the poly runs (as in `a_gate_bar_has_no_notch`)
  number one per finger. Each holds exactly one cut. Gate-to-gate pitch == 490 (derived for sky130).
- `moderate_bar_sits_a_micron_off`: MOD pair. Every gate-row poly bar's near edge is ≥ 1000 from the diffusion.
- Sweep: extend `every_variant_is_drc_and_erc_clean` and `every_variant_extracts_its_fingers_and_dummies` with
  `(Nmos|Pmos, n=2, nf=2, W=1680, L=150)` at each `class = Some(c)`, `dummy_required = true`. 0 DRC/ERC findings, and
  extracted count = units + `m.dummies.len()`.
- `every_diffusion_point_reaches_its_tap` must still pass with the larger EXC rows. The strip spans the moat, and
  reach is vertical.

Commands (background, with `PDK_ROOT` exported): `cargo test --release -p cells`, `cargo test --release -p library`.
Bench: `cargo run --release -p benchmark --bin bench`. Report area and active % before/after. Expect growth: every
annotator-matched set is `Moderate` by default, which means 7 dummies per end at reach 3000, a 5 µm moat and a
+1 µm gate overhang. Block groups (class None, both flags true) get `Moderate` too under C16.

### Risks

- Bench re-baseline (audit Q7). The EXC area is about 23 dummies per end at sky130 L = 150.
- Two-row and four-row stacks (`stack_on_tap` / `stack_pad_to_pad`) use the macros' extents, so a taller row moves
  the mirror line. Their tests are in the sweep.
- If the owner reads C16 as "block groups are not matched cells", `row_env` should gate on `u.class.is_some()`
  instead. That is a one-line change, and its test is `a_singleton_has_no_environment`, plus a block-group case.
