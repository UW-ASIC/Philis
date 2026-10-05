# M2+ analog-matching, segment 5 (M6): implementation card (MAT-18)

Base: `m2-analog-matching` fast-forwarded to `m2` (0f84944). Line numbers are this base's; the card wins over
`plan-02` §MAT-18. DAG: module analog-matching, M6, hard dep MAT-11 (done, segment 2); step-level: CELL-21 (cells)
consumes these functions for drawing, so MAT-18 lands first.

## MAT-18 Split-DAC slot assignment, attenuation-capacitor check and sizing — class: do

Facts (plan stale on four points):

- **Gate is met, not by a bench fixture**: EXT-19 added the `splitdac` netlist to the annotator corpus
  (`backend/annotator/tests/corpus.rs:62-63`: LSB bank C0,C1,C2 = 1+1+2 units on `tl`, MSB bank C3,C4 = 1+2 on `tm`,
  bridge `CA` 3×4 µm on a 3×3 µm unit). `benchmarks/fixtures` still has no split DAC.
- **The C_A ±1 % check already exists** in EXT-19: `capacitor_sets` (`backend/annotator/src/passive.rs:117-170`)
  computes `want = C_T^LSB/C_T^MSB · area(unit)` inline (:158-159) and emits `bridge_cap_value` (:160-166);
  tested by `corpus.rs::split_dac_bridge_value` (:566-574). MAT-18 does not touch it (annotator module). Follow-up
  for the annotator owner: replace :158's ratio with `analog::matching::dac::attenuation_cap(lsb, msb)`.
- `bits()` is `kernel/cells/src/cap_array.rs:153-160` (not :123-130) and stays single-bank; `Pattern` is
  `cap_array.rs:40-50` with no `Split` variant. Both are CELL-21's, not MAT-18's.
- Spiral order is a closure local to `centro_assign` (`kernel/analog/src/matching/pattern.rs:70-79`, `key` + sort);
  `Grid` (`pattern.rs:172-200`: `refl`, `free_pair`, `put_pair`) is public and reusable.

Edits:

1. `kernel/analog/src/matching/pattern.rs`: extract the order from `centro_assign` (:74-79) into
   ```rust
   /// Cells of a `rows × cols` grid nearest the centre first (doubled offsets, r²), ties by angle `atan2(dr, dc)`.
   #[must_use]
   pub fn spiral(rows: usize, cols: usize) -> Vec<usize>;
   ```
   and call it from `centro_assign` (`let order = spiral(rows, cols);`). `key` stays for the Balanced r² use
   (:111-125); behaviour unchanged (existing `pattern.rs` tests are the check).
2. `kernel/analog/src/matching/dac.rs` (module doc: add one line naming MAT-18, DACP eq. 1, §III-D, §IV-C):
   ```rust
   /// C_A in units of C_u: `C_T^LSB / C_T^MSB` (DACP eq. 1); the LSB total counts the termination unit.
   #[must_use]
   pub fn attenuation_cap(ct_lsb_units: u32, ct_msb_units: u32) -> f64;

   /// Non-unit side lengths `(H, l)`, `H ≥ l`, of area `A` with the unit's perimeter-to-area ratio
   /// `k = (H_u + l_u)/(H_u·l_u)`: roots of `z² − kA·z + A = 0` (DACP §III-D eqs 37–45). `None` when
   /// `k²A² < 4A` (no rectangle of that area keeps the unit's edge sensitivity).
   #[must_use]
   pub fn nonunit_dims(area_um2: f64, unit_h_um: f64, unit_l_um: f64) -> Option<(f64, f64)>;

   /// Owner per cell (row-major, `None` = dummy) of a split DAC: ids `0..=L` the LSB bank `[1, 1, 2, …, 2^(L-1)]`,
   /// `L+1..=L+M` the MSB bank `[1, 2, …, 2^(M-1)]`, `L+M+1` the two C_A halves. DACP §IV-C order along
   /// [`pattern::spiral`]: C_A on the most central reflected pair; the odd caps (C0, C1, C_{L+1}) next, the
   /// centre cell first on an odd grid, else split across one pair as `centro_assign` does; then the
   /// remaining pairs, LSB and MSB tokens alternating (each bank largest cap first), each at the next free pair.
   #[must_use]
   pub fn split_dac_assign(l_bits: u8, m_bits: u8, rows: usize, cols: usize) -> Vec<Option<u8>>;
   ```
   Steps for `split_dac_assign`: counts as above; `debug_assert!(rows*cols >= 2^L + 2^M - 1 + 2)`;
   `g = Grid { cols, slot: vec![None; n] }`; C_A: `g.put_pair(first free_pair in order, L+M+1)`; odd caps
   `[0, 1, L+1]`: if `n` odd, `slot[n/2] = Some(0)`, rest chunked in twos onto the next free pair (`i` and
   `refl(i)`, a lone one leaves its reflection `None`); token lists `lsb`/`msb` = `left[d]/2` copies of each `d`,
   largest `d` first, interleaved `lsb[0], msb[0], lsb[1], …` then the tail; each token `put_pair` at the next
   free pair in `order`. No new types.

Tests (`kernel/analog/src/matching/dac.rs` `mod tests`):

- `attenuation_cap_formula`: `(attenuation_cap(8, 7) - 8.0/7.0).abs() < 1e-12` (L = M = 3);
  `(attenuation_cap(4, 3) - 4.0/3.0).abs() < 1e-12` (the corpus `splitdac`: 12 µm² bridge on a 9 µm² unit).
- `nonunit_dims_keep_the_unit_edge_sensitivity`: `nonunit_dims(12.0, 3.0, 3.0)` is `(6.0, 2.0)` within 1e-9;
  for `(A, Hu, lu)` in `[(12.0, 3.0, 3.0), (10.0, 2.0, 3.0), (37.5, 4.0, 2.5)]`: `H ≥ l`, `|H·l − A| < 1e-9`,
  `|(H+l)/(H·l) − (Hu+lu)/(Hu·lu)| < 1e-9`; `nonunit_dims(4.0, 3.0, 3.0) == None`.
- `split_assign_is_exact_for_even_count_caps`: for `(L, M, rows, cols)` in
  `[(2, 2, 3, 3), (3, 3, 3, 6), (3, 3, 4, 5), (4, 3, 5, 5)]`: per-id cell counts equal the counts above
  (C_A = 2); every id with an even count has `slot[i] == slot[n-1-i]` on all its cells; C_A's two cells have the
  minimum doubled-offset r² among cells other than the odd-grid centre.
- Command: `PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk cargo test -p analog matching::`
  (covers the unchanged `pattern.rs` tests after step 1).

Acceptance: the three tests green. The plan's "a split-DAC fixture draws with exact CC and reports C_A error" is
CELL-21's (it adds `Pattern::Split`, draws the non-unit plate from `nonunit_dims`) plus a bench fixture; MAT-18's
share is that `split_dac_assign` is exact for every even-count cap and C_A error is already reported by EXT-19.
