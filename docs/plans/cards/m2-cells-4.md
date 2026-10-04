# M2+ cells, segment 4 (M6): CELL-15

Worktree `philis-m2/cells`, branch `m2-cells`, fast-forwarded to `m2` at `ec141b3` (no conflicts).

| Item | Class | Why |
|---|---|---|
| CELL-15 | do | Hard dep MAT-15 is merged in m2 (`d960ba5`, `pattern::nth_order_rows`). CELL-10 and MAT-03 done. Uses no PLC-06. |

Command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk; cargo test -p cells mosfet`
(then `cargo test -p cells --test cell_selfcheck`, `cargo test -p analog matched_set`).

---

## CELL-15 MOS rows from any legal order grid. Class: do

### Code facts (plan line numbers are stale)
- `kernel/cells/src/mosfet.rs:37-55` `Mosfet { nf, style, dummies_per_edge, split_gates, mirror_pins, rows: u16, double_gate }`.
  Doc (37-40) says rows ∈ {1, 2}.
- `enumerate` (72-174): `two_rows` closure (147-159) gates rows = 2; rows loop at 165
  (`if two_rows(..) { &[1, 2] } else { &[1] }`); final `.filter(bars_fit)` (172).
- `draw` (176-199): `row_orders` → `draw_row` per order, `stack_on_tap(row0, row1)` for 2 rows (183-187); gate_ohm
  per owner counts fingers over `orders.iter().flatten()` (190-197), so it already scales to any row count.
- `stack_on_tap` (203-266): flips `b` about `a`'s topmost tap strip, skips `b`'s strip cuts, merges nwell into one
  rect, flips pins/units (`y2 - u.y`), sums `figures.sd`.
- `row_orders` (261-277 in `impl Mosfet`, starts ~259): `self.rows.clamp(1, 2)`; second row = relabelled (pair) or
  reversed. Plan's "orders recomputed inside draw from style" still true; this function is the single source.
- `legal_row(seq, s0)` (977); `pattern::nth_order_rows(order, row: &[u8]) -> Vec<Vec<u8>>`
  (`kernel/analog/src/matching/pattern.rs:310`, debug-asserts every row `diffusion_legal(.., Drain)` for even length);
  `pattern::diffusion_cc_row(counts, Outer::Drain) -> Option<Vec<usize>>` (411). Plan's
  `diffusion_cc_row(&[c/4, c/4])` lacks the `Outer` arg.
- MAT's metric: `matched_set.rs:330` `cancelled_order(&[&pa, &pb], 4, 1e-3).0` over `Macro.units`
  (`pnr_core::Unit { owner, x, y, weight, phi, .. }`, `kernel/core/src/units.rs:17`).
- Sweeps: `every_variant_is_drc_and_erc_clean` (1062, `(n, nf)` list at 1072),
  `every_variant_extracts_its_fingers_and_dummies` (1100, list at 1106); `every_diffusion_point_reaches_its_tap`
  (1662) and `split_gate_variants_keep_private_gates_under_erc` (1437) iterate all variants.

### Decision (deviation from plan step 1)
No `grid: Vec<Vec<u8>>` field. `row_orders` already derives the grid deterministically from the variant's fields;
a stored copy is duplicate state and touches every struct literal (8 test sites). `rows = 4` + `row_orders` is the grid.

### Edits (`kernel/cells/src/mosfet.rs` only)
1. `row_orders`: `let rows = self.rows.clamp(1, 4);` For `rows == 4` (only enumerated for an equal pair, `Cc1d`, no
   mirror pins): `let base = finger_sequence(Pattern::Cc1d, &[nf/4, nf/4])` as `u8`s, return
   `pattern::nth_order_rows(3, &base)` mapped back to `usize`. Rows 1/2 unchanged. `nf` per row = `self.nf / rows`.
2. `fn grid_legal(orders: &[Vec<usize>]) -> bool { orders.iter().all(|r| legal_row(r, true) || legal_row(r, false)) }`
   next to `legal_row`. `enumerate` adds `.filter(|v| grid_legal(&v.row_orders(&s.dev_nf, n_dev)))` beside
   `bars_fit` (illegal grids from any source dropped). Σφ per device: asserted in the test, not filtered (each
   drawn row alternates S/D, so it holds by construction).
3. `enumerate`: `let four_rows = |style, mirror| n_dev == 2 && s.dev_nf[0] == s.dev_nf[1] && nf % 8 == 0 && style == Pattern::Cc1d && !mirror;`
   (P_3 needs 2³ units per member, the base row an even count per member: c/4 even ⇔ c % 8 == 0).
   Rows slice built as `Vec<u16>`: `[1]`, `+2` if `two_rows`, `+4` if `four_rows` (appended last, so existing
   variant indices per style keep their relative order).
4. Stacking: split `stack_on_tap`'s merge body into
   `fn merge_stacked(a: &Macro, b: &Macro, place: impl Fn(Rect) -> Rect, skip_cuts_in: Option<Rect>, process: &dyn Process) -> Macro`
   (shapes, nwell union, pins, units via `place(Rect { x: u.x, y: u.y, w: 0, h: 0 }).y`, dummies, `figures.sd` sums).
   `stack_on_tap` = `merge_stacked(a, b, flip, Some(t), process)`. New
   `fn stack_pad_to_pad(lo: &Macro, hi: &Macro, process) -> Macro`: `gap` = max of `process.space(r)` over roles
   `["diff", "tap", "poly", "licon", "li", "mcon", "met1", "npc", "nsdm", "psdm"]` (poly.2, li.3, npc.2, implants);
   `dy = top(lo) + gap - bottom(hi)` with `top`/`bottom` over non-nwell shapes; `merge_stacked(lo, hi, |r| Rect { y: r.y + dy, ..r }, None, process)`.
   Translation, not mirroring: pair 1 ends in row 1's pad row facing up, pair 2 starts with row 2's pad row facing
   down, and equal pair offsets keep `y0 + y3 = y1 + y2` (needed for the x²y moment).
   ponytail: bbox-wide gap, the widest same-layer space; per-layer facing-edge gaps if area matters.
5. `draw`: 4 orders → `stack_pad_to_pad(&stack_on_tap(r0, r1), &stack_on_tap(r2, r3), process)`.
6. Doc on `rows` (37-40): rows ∈ {1, 2, 4}; 4 = MAT-15's order-3 grid, two tap-sharing pairs stacked pad to pad.

### Tests (`kernel/cells/src/mosfet.rs` tests module)
- `four_rows_cancel_to_third_order`: sky130 (`testkit::pdk()`, skip if absent), `testkit::group_of(Nmos, 2, 8, 1680, 150)`;
  for each enumerated variant with `rows == 4` (assert at least one exists, and none for `group_of(.., 2, 4, ..)`):
  `row_orders` == `[[0,1,1,0],[1,0,0,1],[1,0,0,1],[0,1,1,0]]`; drawn `m.units` 16 per owner; integer moments
  `Σ x^a y^b` (i128, a + b ≤ 3) equal between owners 0 and 1; `Σ phi.0` per owner == 0;
  `moments::cancelled_order(&[&pa, &pb], 4, 1e-3).0 >= 3` (MAT's metric). Same for Pmos.
- `an_illegal_grid_is_not_drawn`: `!grid_legal(&[vec![0, 1, 1, 0], vec![0, 1, 0, 0]])` (second row: boundaries on
  regions 1 and 2, one is a drain either way), `grid_legal(&nth rows)`; and every variant from
  `group_of(k, n, nf)` for n ∈ {1, 2, 3, 4}, nf ∈ 1..=16 satisfies `grid_legal(&v.row_orders(..))`.
- Sweeps: add `(2, 8)` to the `(n, nf)` lists of `every_variant_is_drc_and_erc_clean` and
  `every_variant_extracts_its_fingers_and_dummies` (rows = 4 then covered by DRC/ERC and extraction, Nmos and Pmos).

### Acceptance
Pair nf = 8 per device offers a 4-row variant whose units cancel to order ≥ 3 by `cancelled_order` (MAT's metric),
DRC/ERC clean and extracting its fingers. The bench-ledger T6/T4 claim ("order ≥ 3 for an ota pair") only shows if a
bench pair has 8k fingers per member and the search picks the 4-row variant; report what the ledger shows, do not
force the variant (MAT-15 Risks: variants only).
