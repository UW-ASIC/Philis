# M1 matching batch: implementation cards (MAT-01 … MAT-06)

Branch `m1a-matching`, worktree `philis-m1a/matching`, base `ca3e509` (`git merge m1a`: already up to date). Spec:
`plan-02-matching-and-common-centroid.md` §3. Line numbers below are this base's; where the plan's differ, the card
wins. Order = dependency order: **01 → 02, 03 → 04 → 05, 06** (05 only needs 01, but it emits next to 04's rule).

Every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first. Crate names: `analog`
(kernel/analog), `pnr_core` (kernel/core), `cells`, `annotator`, `library`, `benchmark` (bin `bench`).

Batch-wide decisions (deviations from plan-02, each for a stated reason):

- D1. `matching/mod.rs` declares only modules that exist (`moments` in 01, `pattern` in 02, `mismatch` in 04). No
  empty `class`/`dac` stubs; MAT-07/18 add them.
- D2. PLC-03 is **not** on this branch (`grep -rn matched_pairs` is empty), so MAT-04/05 implement no
  `matched_pairs` hook. PLC-03, when it lands, adds the overrides on `MatchedSet` and `OrientationSet` (C28 rule).
- D3. `Ledger.order` (cancelled order) is dropped: only MAT-13 (M3) reports it, and computing it allocates in dp's hot
  path. MAT-13 adds it.
- D4. `MatchedSet` gets `pub mos: bool` (coincidence feasibility differs for MOS rows, MAT-03); MAT-07 replaces it
  with `family`.
- D5. `OrientCheck::PhiZero` deferred to MAT-12 (its only emitter). `OrientationSet{Phi}` is emitted into **budget
  only**: Φ is discrete (a flip), so a cost copy has no gradient to give; Θ prices it.
- D6. MAT-06's `flow_smoke` test is replaced by a library unit test of the lookup: `Solution` does not expose
  `CommonNode.max_delta_ohm`, and the budget identity holds by construction.
- D7. MAT-03 `deal`: `Q` is the key weight still in play, not the fixed `Σ_i k(i)`: it starts at the row total,
  drops by every finger pre-placed in `seq` and by every quad dealt (Drain deals with the reduced counts `c`). With a
  fixed `Q` the card's own vectors are not reproduced (`equal_counts_reproduce_the_old_orders` fails).
- D8. MAT-04 step 3: coincidence no longer requires both members in one cell (c41f71b). Gated on one cell, a
  CC-feasible pair split into two cells escaped the check and Θ ranked the split candidate first (ota 1729.8 →
  2057.0 µm²). Consequence: a CC-feasible pair drawn as two cells is a permanent `MatchedSet` violation (two cells'
  centroids never coincide) that placement cannot fix; with merged pairs drawn AABB the bench reads `MatchedSet`
  0/6 satisfied. FLOW-16 step 4 and the M1 exit criterion are written against this rule. Risk (review, FLOW-16/PLC-03):
  such a pair's `coincidence` (Δm/tol, ~13 600 at 34 µm) dominates `residual` and is a `violating_residuals`
  target; on this branch only the report reads it (library metadata.rs:190), but a dp/gp repair consuming
  budget-arm violating ids would churn on it. FLOW-16/PLC-03 must offer the merged variant (MAT-07) or skip it.

Out of scope, reported: `CentroidGroup::kind()` is `"CommonCentroid"` (cc.rs:176-178). MAT-04 deletes it, but
FLOW-16 step 4 (plan-08 ~L1108) and the M1 exit criterion "ota, ota_constrained, tt_ota `CommonCentroid` rows 3/3"
are written against it. After MAT-04 the same check is the `MatchedSet` coincidence of XM1/XM2. Whichever of
MAT-04/FLOW-16 lands second rewrites FLOW-16 step 4 ("in one `MatchedSet`") and the criterion ("`MatchedSet` rows of
the OTA input pair satisfied").

---

## MAT-01 Moment and orientation kernel — class: mechanical

Facts: first moments in `kernel/analog/src/placement/cc.rs:55-64`; Φ test is private `currents_run_alike`,
`frontend/library/src/cellgen.rs:261-272` (called at :140, test :1564). Members with 0 units are skipped there; a macro
without units passes. `pnr_core::{Unit, PlacedUnit}` (kernel/core/src/units.rs:16-33, 55-64); `analog` already
depends on `pnr_core`. `kernel/analog/src/lib.rs:13-19` lists modules.

Edits:

1. `kernel/analog/src/lib.rs`: add `pub mod matching;` (alphabetical, after `pub mod constraints;`) and a doc line
   `//! - [`matching`]: unit moments, CC patterns and the mismatch ledger of matched sets.`
2. New `kernel/analog/src/matching/mod.rs`: `//! Matched-set math: …` + `pub mod moments;`.
3. New `kernel/analog/src/matching/moments.rs`:
   ```rust
   #[derive(Clone, Copy, Debug, PartialEq)]
   pub struct Pt { pub x: f64, pub y: f64, pub w: f64, pub phi: (i8, i8) }
   impl From<pnr_core::PlacedUnit> for Pt  // x,y as f64, w = weight as f64, phi
   impl From<pnr_core::Unit> for Pt        // same, local frame
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub enum Axis { #[default] None, H, V, Mixed }
   #[derive(Clone, Copy, Debug, Default, PartialEq)]
   pub struct Sums { pub w: f64, pub x: f64, pub y: f64, pub xx: f64, pub xy: f64, pub yy: f64,
                     pub phi_x: i32, pub phi_y: i32, pub n: u32, pub axis: Axis }
   pub fn sums(pts: impl IntoIterator<Item = Pt>) -> Sums;
   impl Sums {
       pub fn centroid(&self) -> Option<(f64, f64)>;        // None when w <= 0
       pub fn second(&self, c: (f64, f64)) -> [f64; 3];      // [xx, xy, yy] per unit weight about c
       pub fn phi(&self) -> (f64, f64);                      // (phi_x/n, phi_y/n); (0,0) when n == 0
   }
   pub fn phi_equal(a: &Sums, b: &Sums) -> bool;
   pub fn phi_equal_all(units: &[pnr_core::Unit], members: usize) -> bool;
   pub fn mirror_allowed(members: &[Sums]) -> bool;
   pub fn cancelled_order(devs: &[&[Pt]], nmax: u8, tol: f64) -> (u8, [f64; 5]);
   ```
   - `sums`: one pass; `w += p.w; x += p.w*p.x; xx += p.w*p.x*p.x; xy += p.w*p.x*p.y; …; phi_x += p.phi.0; phi_y +=
     p.phi.1; n += 1`. Axis: `any_x |= p.phi.0 != 0; any_y |= p.phi.1 != 0`; at the end `(false,false)→None,
     (true,false)→H, (false,true)→V, (true,true)→Mixed` (zero-φ units ignored; this is MAT-05's definition).
   - `second`: `[xx/w − 2cx·x/w + cx², xy/w − cx·y/w − cy·x/w + cx·cy, yy/w − 2cy·y/w + cy²]`; `[0;3]` when `w <= 0`.
   - `phi_equal`: `i64(a.phi_x)*i64(b.n) == i64(b.phi_x)*i64(a.n)` and the same for y.
   - `phi_equal_all`: per owner `d < members`, `sums(units.iter().filter(|u| usize::from(u.owner) == d).map(|&u|
     Pt::from(u)))`; keep those with `n > 0`; `live.windows(2).all(|w| phi_equal(w[0], w[1]))` (true if < 2 live).
   - `mirror_allowed`: `members.iter().all(|s| s.phi_x == 0)` (Mx180 negates φ_x).
   - `cancelled_order` (`debug_assert!(nmax <= 4)`; index `r[n]` = residue of order n, `r[0] = 0`):
     1. c = weighted centroid of all points of all devices; L = max `hypot(p − c)`. Total w ≤ 0 or L == 0 →
        `(nmax, [0.0; 5])`.
     2. Per device with Σw > 0 (others skipped): `M_d(p,q) = Σ w·((x−cx)/L)^p·((y−cy)/L)^q / Σw`.
     3. For n in 1..=nmax: `r[n]` = max over device pairs and p in 0..=n of `|M_a(p,n−p) − M_b(p,n−p)|`.
     4. order = number of leading n = 1, 2, … with `r[n] <= tol`.
4. `frontend/library/src/cellgen.rs:261-272`: keep the fn and its doc; body becomes
   `analog::matching::moments::phi_equal_all(&m.units, n_members)`.

Tests (`moments.rs` `#[cfg(test)] mod tests`, pitch 1000 nm, w = 1; expected values computed by script):

- `abba_cancels_first_order_only`: row A{0,3} B{1,2} → `order == 1`, `r[2] > 0.5` (0.889).
- `abba_baab_cancels_second_order`: rows ABBA (y 0) / BAAB (y 1000) → `order == 2`, `r[3] > 0.2` (0.253).
- `nth_fig3_4x4_cancels_third_order`: ABBA/BAAB/BAAB/ABBA → `order >= 3`.
- `radial_equal_is_not_second_order`: A (±2000,0), B (0,±2000) → `order == 1`; `sums(A).second((0,0))[0] == 4e6`,
  B's `== 0.0`.
- `hastings_phi_example`: φx {+,+,+,−} vs 9 `+` 3 `−` → `phi_equal`; {+,+,+,−} vs {−,−,−,+} → not.
- `ratio_weights_make_one_to_two_exact`: A one unit at 0, B at ±1000 → `order >= 1`.
- `axis_ignores_zero_phi_units`: units φ (1,0),(0,0) → `Axis::H`; (0,0) only → `None`; (1,0),(0,1) → `Mixed`.

Run: `cargo test -p analog matching:: && cargo test -p library --lib cellgen` (cellgen tests unchanged and green).

## MAT-02 Point-symmetric grid assignment; BJT array defect — class: judgment

(The algorithm is fully determined below; it is "judgment" because the acceptance on sky130 DRC with empty cells in
sparse BJT grids, e.g. the [1,4] "+" cross, is unmeasured; if DRC fails, the fix is CELL-09's dummy/emitter
geometry, not this order.)

Facts: `kernel/cells/src/bjt.rs` — `Bjt { columns }` :21-24; `enumerate` :27-48 (matched set → square only, :40-42);
`draw` :50-68 (clamp :56-57); `Unit::draw` :145-202 (`let e = at(self.emitter)` :154), **no `b.unit(` anywhere in
bjt.rs**; `unit_order` :205-231; test :271-281. `kernel/cells/src/cap_array.rs` — `general_assign` :140-192 (callers
:413 and test :745), `Grid` + impl :200-307 (methods `refl, free_pair, put_pair, rect, rc, chessboard, spread,
corridor`; also used by `CapArray::assign` :314-346), `GENERAL` :727, centroid test :741-756. `Builder::unit`
builder.rs:34. `cells` depends on `analog` (Cargo.toml:8). `kernel/cells/tests/cell_selfcheck.rs:293` prints
`v.columns` (keep the field name).

Edits:

1. New `kernel/analog/src/matching/pattern.rs`; `pub mod pattern;` in `matching/mod.rs`.
   - Move `struct Grid` and its whole `impl` from cap_array.rs:200-307 verbatim (inherent methods cannot be split
     across crates): `pub struct Grid { pub cols: usize, pub slot: Vec<Option<u8>> }`, every method `pub`, plus
     `use std::cmp::Reverse;`. cap_array.rs: delete them, `use analog::matching::pattern::{self, Fill, Grid};`, keep
     `use std::cmp::Reverse` only if still used (it is not after `general_assign` moves; remove it).
   - Add:
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum Fill { Compact, Dispersed, Balanced }   // docs: plan-02 MAT-02 step 2
     pub fn grids(counts: &[u16], max_aspect: f64) -> Vec<(usize, usize)>;
     pub fn centro_assign(counts: &[u16], rows: usize, cols: usize, fill: Fill) -> (Vec<Option<u8>>, bool);
     pub fn cc_feasible(counts: &[u16]) -> bool;   // counts.iter().filter(|&&c| c % 2 == 1).count() <= 1
     ```
   - `grids`: `T = Σcounts` (usize; T == 0 → `vec![]`), `odd = #odd counts`. For `r in 1..=T.isqrt() + 2`: if
     `odd == 1 && r % 2 == 0` skip; `c = T.div_ceil(r)`, if `odd == 1 && c % 2 == 0 { c += 1 }`; keep if
     `max(r,c) as f64 / min(r,c) as f64 <= max_aspect`. Sort by aspect compared exactly
     (`max_a*min_b` vs `max_b*min_a`), then empties `r*c − T`, then `r`; dedup; push `(1, T)` if absent.
     Expected: [2,2] → [(2,2),(3,2),(1,4)]; [1,4] → [(3,3),(1,5)]; [1,8] → [(3,3),(5,3),(1,9)];
     [4,4,4] → [(3,4),(4,3),(5,3),(2,6),(1,12)]; [1,2,2] → [(3,3),(1,5)]; [3,5] → [(3,3),(2,4),(4,2),(1,8)].
   - `centro_assign` = the body of `general_assign` (cap_array.rs:144-190) with `rows, cols` given
     (`debug_assert!(rows*cols >= T)`): same `key`/`order`, same odd prelude (:151-171). Then by fill:
     `Dispersed` = the `Pattern::Chessboard` arm (:176-179), `Compact` = the `_` arm (:180-187), with the same
     `by_size` loop; `Balanced`:
     1. `reps`: walk `order`; take cell i when `g.free_pair(i)` and i is not the mirror of a cell already taken;
        stop at `P = Σleft / 2` reps (the first cell met of each pair has the smaller angle).
     2. `r2(i) = dr²+dc²` (doubled offsets as in `key`). `R: i64 = Σ r2` over cells already owned + `2·Σ r2(reps)`.
        `S[d]: i64 = Σ r2` of cells d already owns (the prelude's).
     3. Deal `reps` sorted by `(r2 desc, angle asc)`: `d = argmax over {d : left[d] >= 2} of
        (counts[d] as i64 * R − T as i64 * S[d])`, ties → lower d; `g.put_pair(i, d)`, `left[d] −= 2`,
        `S[d] += 2·r2(i)`.
     Return `(g.slot, (0..N).all(|i| g.slot[i] == g.slot[N − 1 − i]))`.
     Expected (Balanced): [2,2] on 2×2 → `[0,1,1,0]`, exact; [1,4] on 3×3 → `[_,1,_,1,0,1,_,1,_]`;
     [1,6] → `[1,1,_,1,0,1,_,1,1]`; [1,8] → all 1 but centre 0; [4,4,4] on 3×4 → `[0,2,0,1,2,1,1,2,1,0,2,0]`;
     [1,1,2] on 2×2 → exact false.
2. `cap_array.rs:413`: `let (rows, cols) = pattern::grids(&s.dev_nf, 3.0)[0]; let fill = if self.pattern ==
   Pattern::Chessboard { Fill::Dispersed } else { Fill::Compact }; let slots = pattern::centro_assign(&s.dev_nf,
   rows, cols, fill).0;` and delete `general_assign`. Test :745 becomes the same three lines (assertions unchanged).
   GENERAL grids are unchanged ([8,1] 3×3, [2,2] 2×2, [3,5] 3×3, [4,4,4] 3×4), so slots are identical.
3. `bjt.rs`: `pub struct Bjt { pub rows: u16, pub columns: u16 }` (doc: order from `pattern::centro_assign(Balanced)`).
   - `enumerate`: `let counts: Vec<u16> = s.dev_nf.iter().map(|&u| u.max(1)).collect(); let n = counts.iter().sum();`
     matched set (`group.devices.len() > 1`) → `pattern::grids(&counts, 3.0)`; single device → cols in
     sorted-deduped `[1, n, ceil(√n)]`, rows = `n.div_ceil(cols)`. Map to `Bjt { rows, columns }`.
   - `draw`: `let (owners, _) = pattern::centro_assign(&counts, rows, cols, Fill::Balanced);`, delete :55-57, place
     slot i at `((i % cols)·px, (i / cols)·py)` for `Some(d)` → `u.draw(&mut b, process, usize::from(d), ox, oy)`.
   - `Unit::draw`, after :154: `b.unit(pnr_core::Unit { owner: di as u8, x: e.x + e.w / 2, y: e.y + e.h / 2,
     weight: i64::from(e.w) * i64::from(e.h), phi: (0, 0), sa: 0, sb: 0 });`
   - Delete `unit_order` and `a_one_to_eight_pair_centres_the_single_unit` (replaced below).

Tests:

- `pattern.rs`: `every_single_odd_ratio_is_exact` (a in 1..=4, b in 1..=16, ≤ 1 odd; every grid of `grids(&[a,b],
  3.0)`; Balanced → exact, per-member Σ doubled offsets == (0,0), counts match, `slot[i].is_none() ==
  slot[N−1−i].is_none()`; verified by script: 0 failures); `equal_pair_is_a_diagonal_quad`; `four_to_one_is_a_cross`
  (grids and owners above); `two_odd_members_are_reported_inexact` ([1,1,2], 2×2); `grid_order_is_deterministic`
  ([4,4,4] and [1,8] lists above).
- `bjt.rs`: `every_bjt_ratio_is_common_centroid` (sky130, `testkit::pdk()` else skip; PNP;
  `testkit::group_of(Pnp, 2, 1, 1000, 1000)` with `dev_nf = [a, b]`, a in 1..=2, b in 1..=16, ≤ 1 odd; every
  `Bjt::enumerate` variant drawn: per-owner weight-weighted unit centroid, `|Δx| <= 1 && |Δy| <= 1` nm, and
  `m.units.len() == a + b`). Extend `every_variant_is_drc_and_erc_clean` with `[2,2]` and `[1,4]` groups like the
  1:8 block (:263-267).
- Unchanged and green: cap_array :741-756, `cell_selfcheck`.

Run: `cargo test -p analog matching::pattern && cargo test -p cells --lib -- bjt cap_array && cargo test -p cells --test
cell_selfcheck`.

## MAT-03 Diffusion-legal CC rows for ratioed MOS — class: mechanical

(Algorithm fixed by plan-02 steps 2–3; every expected vector below was reproduced by a reference script.) No mosfet.rs
edits: CELL-10 wires them.

Facts: row parity `kernel/cells/src/mosfet.rs:324-328` (multi-device rows: region 0 = D, S iff odd; mirror pins: S
iff even); `centroid_sequence` :730-750; `finger_sequence` :784-796; tests :861-888 (plan said 855-880);
`greedy_centroid` builder.rs:284-305 (plan said 269/271-292).

Edits (`kernel/analog/src/matching/pattern.rs`):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outer { Drain, Source }
pub fn diffusion_cc_row(counts: &[u16], outer: Outer) -> Option<Vec<usize>>;
pub fn diffusion_legal(s: &[usize], outer: Outer) -> bool;
```

- `diffusion_legal`: `s.windows(2).enumerate().all(|(i, w)| w[0] == w[1] || ((i + 1) % 2 == 1) == (outer ==
  Outer::Drain))` (boundary between fingers i, i+1 is region i+1).
- Shared deal `fn deal(counts, n, seq: &mut [Option<usize>], p: usize, h: &mut [usize], off: usize)`: token t
  occupies fingers `off+2t, off+2t+1`; key `k(i) = (2i − n + 1)²` (i64), `Q = Σ_i k(i)`, `S[d] = Σ k(i)` over fingers
  already in `seq`; for t in 0..p/2: fingers of t and of `p−1−t`; `d = argmax over {h[d] > 0} of
  (counts[d]·Q − n·S[d])`, ties → lower d; write d into the 4 fingers, `S[d] += Σk`, `h[d] −= 1` (`Q` shrinks as weight
  is committed, D7).
- `Drain`: n = Σcounts; n == 0, n odd or any count odd → `None`. `P = (n−2)/2`. For e ascending with `counts[e] >= 2`:
  `c = counts, c[e] −= 2`; `pairs = c/2`; `odd = members with pairs odd`; require `odd.len() == P % 2`; `seq[0] =
  seq[n−1] = e`; if P odd: the odd member at fingers `1 + 2(P/2)`, `2 + 2(P/2)`; `deal(.., P, h = pairs/2, off = 1)`.
  Score `r2 = moments::cancelled_order(&per-member Pt rows (x = finger index, w 1), 2, 0.0).1[2]`; keep the first
  e, replace only when `r2 < best − 1e-9`.
- `Source`: any count odd → `None`; `P = n/2`, `pairs = counts/2`, require `#odd pairs == P % 2`; odd member at
  fingers `2(P/2)`, `2(P/2)+1`; `deal(.., P, pairs/2, off = 0)`.

Tests (`pattern.rs`; A=0, B=1, …):

- `diffusion_rows_never_join_two_devices_on_a_drain`: both `Outer`s, `[a,b]` a,b in 1..=16 and `[a,b,c]` in 1..=8:
  `Some(s)` ⇒ counts match, `s` palindrome, `diffusion_legal(&s, outer)`; any odd count ⇒ `None`.
- `equal_counts_reproduce_the_old_orders` (Drain): [2,2] ABBA; [4,4] ABBAABBA; [6,6] ABBAABBAABBA; [8,8] ABBA×4;
  [4,4,4] ABBCCAACCBBA; [4,4,4,4] ABBCCDDAADDCCBBA (literal vectors). [8,8,8] gives ABBCCCCAABBAABBAACCCCBBA ≠ old
  ABBCCAACCBBAABBCCAACCBBA: assert `r2(new) <= r2(old)` (0.0832 vs 0.0907).
- `two_to_four_is_a_bbbb_a` ([2,4] Drain → `[0,1,1,1,1,0]`); `two_to_four_source_is_bb_aa_bb` (→ `[1,1,0,0,1,1]`);
  `source_needs_even_pair_parity` ([2,2] Source → None; [4,4] Source → AABBBBAA); `one_to_two_has_no_row` ([1,2]
  both → None); `align_example_is_exact` ([2,2,4,8,8] Drain == ADDEECCEEDDBBDDEECCEEDDA, every member
  `Σ(2i − (n−1)) == 0`); [4,8] Drain == ABBBBAABBBBA.

Run: `cargo test -p analog matching::pattern`.

## MAT-04 `MatchedSet`: one ledger, one allowance per pair — class: judgment

(Cross-crate L-sized change; Θ now reads the live thermal field; acceptance T8 = HPWL/footprint ±5 % on the bench is
unmeasured; cost scales are carried over, not derived.)

Facts (plan line numbers that moved are corrected here):
- `emit.rs` (backend/annotator/src): constants :40-52; `systematic_allowance_mv` :58-65; `Pelgrom` :67-103;
  `by_polarity`/`avt` :105-118; emission loop :138-239 (MatchingPair :159-171, ThermalGradient :176-183, sides
  :196-197, CentroidGroup :216-238); test `pelgrom_numbers_come_from_the_deck_and_the_budget` :298-322.
- `ProcessNumbers` is `backend/annotator/src/netrole.rs:142-171` (plan: 79-108), `#[derive(Default)]`; the only full
  literal is `frontend/library/src/lib.rs:633-648` (plan: 499-514).
- `missing` names "MatchingPair" at annotator lib.rs:57, :60 **and :139** ("device W/L", added by EXT-11; plan does
  not list it), asserted in `backend/annotator/src/size.rs:124`.
- `rule::over` rule.rs:368-373 (plan: 313-318); `RuleBatch` :165-252.
- `CentroidGroup::kind()` = `"CommonCentroid"` (cc.rs:176-178). Tests naming the old rules:
  annotator tests.rs:139-157, 192-202, 410, 507-523; symmetry.rs:294-310 (MatchingPair), :326-340 (CentroidGroup);
  core thermal.rs tests :131-170 use `delta_temp_mc`/`live_delta_temp_mc`, whose only other caller is
  placement/thermal.rs. No dp/gp/bench code names these types (bench.rs:539 is a string literal in a `short_kind`
  test; leave it).

Edits:

1. `kernel/analog/src/matching/mismatch.rs` (`pub mod mismatch;`):
   ```rust
   pub fn sigma_pair(a_pair: f32, a1_um2: f32, a2_um2: f32) -> f32;  // A·√((1/a1+1/a2)/2); 0 if a1<=0||a2<=0
   #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum MatchKind { Voltage, Current, Ratio }
   #[derive(Clone, Copy, Debug)] pub enum Budget { Eta(f32), Sigma1Mv(f32), Allowance(f32) }
   impl Budget { pub fn allowance(self, sigma_rand: f32) -> f32 }  // η·σ; √max(0,b²−σ²); a
   #[derive(Clone, Copy, Debug, Default)]
   pub struct Coeffs { pub avt_mv_um: Option<f32>, pub svt_uv_per_um: Option<f32>,
                       pub kvth0_mv_um: Option<f32>, pub tc_uv_per_k: Option<f32> }
   #[derive(Clone, Copy, Debug, Default, PartialEq)]
   pub struct Ledger { pub sigma_rand: f32, pub sigma_grad: f32, pub mu_thermal: f32, pub mu_lod: f32,
                       pub allowance: f32, pub coincidence: Option<f32>, pub second_order_nm: f32,
                       pub delta_m_nm: f32, pub known: bool }
   impl Ledger {
       pub fn spent(&self) -> f32;    // mu_thermal + mu_lod + sigma_grad
       pub fn usage(&self) -> f32;    // max(spent / allowance.max(f32::EPSILON), coincidence.unwrap_or(0))
       pub fn residual(&self) -> f32; // max(rule::over(spent − allowance, allowance), coincidence.map_or(0, |c| c − 1), 0)
   }
   ```
   Doc comments carry plan-02 MAT-04 step 1's formula sources (one line each).
2. `kernel/core/src/thermal.rs`: rename the loop `rise_at_mc` (:30-49) to `fn rise_at(l, power_uw, x: i32, y: i32)
   -> f32` with `(dx, dy) = (x − l.x[j], y − l.y[j])` and `r = hypot.max(r_floor)` for every j (identical at a
   device's own centre: distance 0 floors to `r_floor`, the old `i == j` branch); `rises_mc` calls it with
   `(l.x[i], l.y[i])`; add `impl Layout { /// Rise at (x, y) from every powered source, mK; O(n).
   pub fn rise_at_point_mc(&self, x: i32, y: i32) -> f32 { rise_at(self, &self.power_uw, x, y) } }`. Delete `delta_temp_mc`, `live_delta_temp_mc` (:57-90)
   and the now-unused `use crate::ids::Target`. Rewrite their three tests on the surviving API:
   `no_power_means_no_gradient` drops its `delta_temp_mc` assert (the all-zero `temp_mc` assert stays);
   `moving_a_pair_onto_one_isotherm_kills_its_delta` uses `(l.temp_mc[1] - l.temp_mc[2]).abs()`;
   `the_live_field_follows_a_move_before_any_refresh` uses `live = |l| (l.rise_at_point_mc(l.x[1], l.y[1]) −
   l.rise_at_point_mc(l.x[2], l.y[2])).abs()` against `|temp_mc[1] − temp_mc[2]|` (same three assertions).
3. New `kernel/analog/src/placement/matched_set.rs`:
   ```rust
   #[derive(Clone)]
   pub struct MatchedSet {
       pub members: Vec<DeviceId>,   // slot 0 = reference; pairs (0, i)
       pub kind: MatchKind,
       pub mos: bool,                // D4
       pub coeffs: Coeffs,
       pub budget: Budget,
       pub gate_um2: Vec<f32>,       // netlist W·L·m per member, read only without units
       pub tol_nm: f32,              // half the cut lattice
       pub cell_of: Vec<u16>,        // set by retarget; empty = ids are cells
   }
   impl MatchedSet { pub fn ledger(&self, l: &Layout, i: usize) -> Ledger }
   ```
   `ledger(l, i)`, a = members[0], b = members[i]:
   1. `sa, sb = sums(l.units.of_device(l, d).map(Pt::from))`; `units = sa.w > 0 && sb.w > 0`.
   2. `cell(d) = cell_of.get(d).copied().unwrap_or(d.0) as usize`; centroid = `s.centroid()` else `(l.x[c], l.y[c])`
      of `cell(d)` (if `c < l.x.len()`, else (0,0)). `delta_m_nm = hypot(ca − cb)`.
   3. coincidence = `Some(delta_m_nm / tol_nm)` iff `units` and feasible (one-cell condition dropped, D8): `mos` →
      `diffusion_cc_row(&[na, nb], Drain).is_some() || (.., Source).is_some()`,
      else `cc_feasible(&[na, nb])` (n = `Sums.n` as u16).
   4. second_order_nm (only with units): c = combined centroid `((sa.x+sb.x)/(sa.w+sb.w), …)`; `d = sa.second(c) −
      sb.second(c)`; `F = √(d0² + 2·d1² + d2²)`; L = max unit distance from c over both members; `F / L` (0 if L == 0).
   5. areas µm² = `s.w / 1e6` with units, else `gate_um2[k]`; `sigma_rand = coeffs.avt.map_or(0, |A|
      sigma_pair(A, a1, a2))`; `budgeted = sigma_rand > 0 || matches!(budget, Budget::Allowance(_))`.
   6. Only when `budgeted` (else 0): `sigma_grad = svt·delta_m_nm·1e-6`; `mu_lod` (units only) = `kvth0·|lodA − lodB|`,
      lod = Σw·lod / Σw over units with finite `lod` (none → term 0); `mu_thermal` (any `l.power_uw != 0`) =
      `tc·|rise(ca) − rise(cb)|·1e-6`, rise = `l.rise_at_point_mc(x.round() as i32, y.round() as i32)`.
   7. `allowance = budget.allowance(sigma_rand)`; `known = units && (budgeted || coincidence.is_some())`.

   `impl RuleBatch<Layout> for MatchedSet` (pairs i in `1..members.len()`):
   `cost` = Σ `1e-3·(Δm² + so²) + if allowance > 0 { 3e5·(mu_thermal/allowance)² } else { 0 }` (the scales of
   matching_pair.rs:42-48 and thermal.rs:25; ponytail comment: kept until PLC-18 normalises costs);
   `violations` = #known with `usage() > 1`; `residual` = Σ known `residual()` (f64); `unknown` = #!known;
   `worst_usage` = max usage over known (None if none); `kind` = `"MatchedSet"`; `count` = `members.len() − 1`;
   `retarget` = `self.cell_of = cell_of.to_vec()`; `touched` pushes `cell(m)` for every member; `violating_ids` /
   `violating_residuals` push `cell(a)`, `cell(b)` (with the pair's residual) per violated pair.
   Add `#[cfg(test)] pub(crate) fn pair(a: u16, b: u16) -> MatchedSet` (Current, mos, sky130 nfet coeffs 9.5 / 1.63 /
   9.8 / 765, `Eta(0.3)`, gate 20.0 each, tol 5.0) for the tests here and in symmetry.rs.
4. `placement/mod.rs`: delete `pub mod cc;`, `pub mod thermal;`, `pub use matching_pair::MatchingPair;`,
   `pub use thermal::ThermalGradient;`; add `pub mod matched_set;` and `pub use matched_set::MatchedSet;`. Delete
   files `placement/cc.rs`, `placement/thermal.rs`. `matching_pair.rs`: keep :8-15 constants, `is_fet`,
   `is_diff_pair`, `is_supply`; delete `MatchingPair`, its impls, `gradient_over_random`, the tests and now-unused
   imports; module doc → "Shared FET recognisers and terminal positions."
5. `netrole.rs`: `ProcessNumbers` gains `/// Cut lattice, nm (coincidence tolerance is half of it); 0 = unknown.
   pub lattice_nm: i32`. `frontend/library/src/lib.rs:633` literal: `lattice_nm: cells::builder::cut_lattice(pdk),`.
6. `emit.rs`:
   - Imports: drop `cc::CentroidGroup`, `MatchingPair`, `ThermalGradient`; add `analog::placement::MatchedSet`,
     `analog::matching::mismatch::{Budget, Coeffs, MatchKind}`.
   - Delete `THERMAL_MAX_DELTA_MC`, `THERMAL_MARGIN_PCT`, `Pelgrom` (+ `thermal_limit_mc`) and its test. Rewrite
     `systematic_allowance_mv` standalone (MAT-06 deletes it): `let s = avt.filter(|&a| a > 0.0 && gate_um2 >
     0.0).map(|a| a / gate_um2.sqrt())?; Some(offset_sigma_mv.map_or(GRADIENT_SHARE * s, |b| (b * b − s *
     s).max(0.0).sqrt()))` (identical value: η·σ = √(b²−σ²)).
   - In the leaf loop replace :159-183 by one `MatchedSet { members: vec![a, b], kind: DiffPair → Voltage else
     Current, mos: matches!(nl.devices[a].kind, Nmos | Pmos), coeffs: Coeffs { avt_mv_um: avt(nl, p, a),
     svt_uv_per_um: p.svt_uv_per_um, kvth0_mv_um: by_polarity(nl, a, p.lod_kvth0_mv_um), tc_uv_per_k:
     by_polarity(nl, a, p.vt_tc_uv_per_k) }, budget: offset_sigma_mv.map_or(Budget::Eta(GRADIENT_SHARE),
     Budget::Sigma1Mv), gate_um2: vec![gate_um2(nl, a), gate_um2(nl, b)], tol_nm: p.lattice_nm.max(1) as f32 / 2.0,
     cell_of: Vec::new() }` pushed (clone) into `r.budget` and `r.cost`; keep the CurrentMirror Proximity push.
     Delete `a_side`/`b_side` (:145, :196-197) and the CentroidGroup block (:216-238).
   - Module doc table (:8-10) and arms paragraph (:19-24): MatchedSet replaces MatchingPair/ThermalGradient/sides.
7. annotator `lib.rs:57,:60` → `("MatchedSet", "deck svt_uv_per_um — distance term unknown")`, `("MatchedSet",
   "deck avt_n_mv_um/avt_p_mv_um")`; `:139` → `("MatchedSet", "device W/L")`; `size.rs:124` the same string.
8. Tests that named deleted rules, rewritten to the new contract (never weakened):
   - tests.rs :139-157 `contains("MatchingPair")` → `contains("MatchedSet")` (comment updated).
   - :192-202: `ends_with("MatchedSet")` in budget, not in hard.
   - :410: `!kinds.iter().any(|k| k.ends_with("MatchedSet"))` (a Stack emits none).
   - :507-523 renamed `matched_sets_are_budgeted_and_missing_deck_terms_are_listed`: bare deck → `MatchedSet` in
     `budget` and `missing` has a `"MatchedSet"` entry; with `avt` and `svt` set → in `budget` and no `"MatchedSet"`
     entry.
   - symmetry.rs :301-309 and :326-340: build `matched_set::pair(0, 1)` / `matched_set::pair(0, 2)`, retarget, assert members unchanged and `cell_of == cell_of.to_vec()`.
   - `frontend/library/src/metadata.rs:19,184` doc examples: `ThermalGradient` → `MatchedSet`, `CentroidGroup on its
     bbox proxy` → "a batch that counts both".

Tests (new):

- `mismatch.rs`: `unequal_areas_use_the_per_device_variance` (`sigma_pair(10, 4, 16)` = 3.953 ± 0.001);
  `one_allowance_is_spent_once` (`Ledger{mu_thermal 0.4, mu_lod 0.4, allowance 0.637}` → usage 1.256 ± 0.001,
  residual 0.256 ± 0.001); `sigma1_budget_leaves_the_quadrature_share` (`Sigma1Mv(3·√1.09).allowance(3.0)` = 0.9 ±
  1e-3; `Sigma1Mv(2.0).allowance(3.0) == 0`); `zero_allowance_is_a_violation_not_nan` (`allowance 0, mu_lod 0.01`
  → residual 1.0, `usage().is_finite()`; all zero → residual 0; `Sigma1Mv(1.0).allowance(2.124) == 0`).
- `matched_set.rs` (layouts and `UnitLib::build` as cc.rs:197-213, 289-297):
  - `merged_pair_reads_its_units_not_its_cell`: one cell, 4 units weight 10 at x 100/300/500/700; ABBA →
    `coincidence == Some(0.0)`, known, violations 0; growing `hw` leaves `cost` unchanged; AABB → usage > 1,
    violations 1.
  - `separate_cells_a_millimetre_apart_spend_the_distance_term`: two one-unit cells 1 000 000 nm apart, svt 1.63 →
    `sigma_grad` 1.63 ± 0.01.
  - `stage_pooling_cannot_cancel`: sets {0,1} at x 0/d and {2,3} at x d/0 (d = 10 000), → each `delta_m_nm == d`.
  - `unknown_is_not_charged`: no units, cells 5 µm apart → `cost > 0`, `residual == 0`, `unknown == 1`; after
    `retarget(&[0, 0])` (one cell) → `cost == 0`, `residual == 0`, `unknown == 1`.
  - `thermal_reads_unit_centroids_of_a_merged_pair`: cell 0 heater at (0,0), hw = hh = 1000, power 10 000 µW; cell 1
    bbox (0,0,20 000,1000) at x 10 000, y 0, hw 10 000, hh 500; AABB units at local x 2500/7500 (A) and 12 500/17 500
    (B), y 500 → `mu_thermal` 1.097 ± 0.01; ABBA (A 2500/17 500) → < 0.01.
  - `lod_is_weighted_by_unit_area`: one cell; A units (sa 500, sb 1500, w 10) and (1000, 1000, w 30), B (1000, 1000,
    w 40), kvth0 9.8 → `mu_lod` 1.633 ± 1e-3 (the unweighted mean would give 3.267).
  - `second_moment_separates_abba_from_abba_baab`: 1-row ABBA (pitch 1000) → `second_order_nm > 1000` (1333);
    ABBA/BAAB → `< 1.0`.
  - `retarget_keeps_schematic_members`, `violating_ids_are_cells` (AABB in cell 1 of 2, ids ⊂ {1}).
- core `thermal.rs`: `point_rise_equals_device_rise_at_its_centre` (bench(): `|rise_at_point_mc(x_i, y_i) −
  rises_mc(..)[i]| <= 0.5` for i in 0..3).

Run: `cargo test -p analog -p pnr_core -p annotator -p cells && cargo test -p library` (the second is long: run it in
the background; no new failures against the base, where `hier_elaborate` is already red).

Acceptance (T5, T8; measured, recorded in the merge commit): on the commit **before** MAT-04 and after it, same
machine and env, `cargo build --release -p benchmark && PNR_BENCH_SEED=1 ./target/release/bench local`: DRC 0 and
the LVS column unchanged on all 10 fixtures; a `MatchedSet` row with 0 unknown on every fixture whose cells carry
units; per fixture HPWL and footprint (the `place` columns) within ±5 % of the before run. If ±5 % fails, report it
with the numbers; do not retune the cost scales to pass.

## MAT-06 Remaining allowance to `CommonNodes` — class: mechanical (after MAT-04)

Facts: `common_nodes` frontend/library/src/lib.rs:900-941 (doc :900-903; σ/η at :929-937); `Flow` fields
`avt_mv_um`/`offset_sigma_mv` declared :617-620, initialised :446-447, read only at :931, :934;
`systematic_allowance_mv`'s only caller is :934. `self.problem.placement.budget` holds each `MatchedSet` once (the
cost copy is the other arm). `RuleBatch` default hooks end at rule.rs:251.

Edits:

1. `kernel/analog/src/rule.rs`, after `shield_pairs`: 
   ```rust
   /// `(device_a, device_b, mV)`: the 1σ systematic allowance each matched pair has left after
   /// placement's own spend — what a routing rule may use. Default none.
   fn offset_allowances(&self, state: &On, out: &mut Vec<(u32, u32, f32)>) { let _ = (state, out); }
   ```
   `MatchedSet` overrides: per i, `let g = self.ledger(l, i); if g.known { out.push((u32::from(members[0].0),
   u32::from(members[i].0), (g.allowance − g.spent()).max(0.0))) }`.
2. `lib.rs`: new private fn next to `common_nodes`:
   ```rust
   /// A common node's ΔR budget, Ω: the pair's remaining allowance (mV, either order) over I_D (µA); 0 = unknown.
   fn common_node_ohm(left: &[(u32, u32, f32)], a: DeviceId, b: DeviceId, i_ua: Option<f32>) -> f32
   ```
   (`find` the entry with `(x, y)` equal to `(a, b)` or `(b, a)`; `(Some(mv), Some(i)) → mv / i * 1e3`, else 0.0).
   In `common_nodes`: before the leaf loop `let mut left = Vec::new(); for b in &self.problem.placement.budget {
   b.offset_allowances(layout, &mut left); }`; replace :929-937 by `let max_delta_ohm = common_node_ohm(&left, a, b,
   i_ua);` (keep the `i_ua` line); doc :900-903 → "budgeted `ΔR ≤ (allowance − placement spend) / I_D` from the
   pair's `MatchedSet` ledger". Delete the two `Flow` fields and their initialisers, and
   `annotator::emit::systematic_allowance_mv`.

Tests: `matched_set.rs::remaining_allowance_subtracts_placement_spend` (`Budget::Allowance(0.637)`, svt 1.0, two
one-unit cells 400 000 nm apart → `σ_grad` 0.4 → `offset_allowances` == `[(0, 1, 0.237 ± 1e-3)]`; with
`Allowance(0.3)` → 0.0); library `lib.rs` `#[cfg(test)] mod tests::common_node_budget_is_the_remaining_allowance`
(`left = [(0, 1, 0.237)]`, I = 100 µA: `(a,b)` and `(b,a)` → 2.37 ± 1e-4 Ω; absent pair → 0; `i_ua None` → 0).

Run: `cargo test -p analog matched_set && cargo test -p library --lib common_node`.
Acceptance (T5): holds by construction (ΔR·I·1e-3 = max(0, allowance − spent)); bench CommonNode rows reported
before/after in the merge commit. Recorded in review (d12b34b/850560f carry no body): `PNR_BENCH_SEED=1 bench
local`, c41f71b and d12b34b both `CommonNode budget 6 6 0 0 0 100% 0.000 tt_ota 0.000`; the whole report table is
identical (only wall times differ), so the remaining allowance changed no bench verdict.

## MAT-05 `OrientationSet`: axes parallel, Φ equal — class: judgment

(Small code, but a new **hard** placement rule: whether dp/gp ever leave a separately drawn partner quarter-turned
on the 10 fixtures is unmeasured, and acceptance is "0 violations, 0 unknown" there.)

Facts: `UnitLib::placed` turns φ with the cell (units.rs:120, 131-135; `Mx180` maps (1,0) → (−1,0), `R90` (1,0) →
(0,1)); no rule reads `PlacedUnit.phi` today; `Orient` variants geom.rs:31-42.

Edits:

1. New `kernel/analog/src/placement/orientation.rs` (`pub mod orientation; pub use orientation::{OrientCheck,
   OrientationSet};` in placement/mod.rs):
   ```rust
   #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum OrientCheck { Axis, Phi }   // PhiZero: MAT-12 (D5)
   #[derive(Clone)] pub struct OrientationSet { pub members: Vec<DeviceId>, pub check: OrientCheck, pub cell_of: Vec<u16> }
   impl OrientationSet { fn residual_of(&self, l: &Layout) -> Option<f32> }  // None = unknown
   ```
   `residual_of`: `s_d = sums(l.units.of_device(l, d).map(Pt::from))`; any member `n == 0` → None. Axis: 1.0 if any
   `Axis::Mixed` or both `H` and `V` occur (ignore `None`), else 0.0. Phi: 0.0 if `phi_equal(s_0, s_d)` for every d,
   else max over d of `|Φx_0 − Φx_d| + |Φy_0 − Φy_d|` (`Sums::phi`).
   `RuleBatch`: `cost` = residual (f32); `violations` = `u32::from(residual > 0)`; `residual` = that; `unknown` =
   `u32::from(None)`; `kind` = `"Orientation"`; `count` = 1; `retarget` stores `cell_of`; `touched` /
   `violating_ids` push each member's cell (`cell_of.get(d).unwrap_or(d)`). Doc cites Hastings rule 7 and eq. 13.61
   (plan-02 MAT-05 lines).
2. `emit.rs`, after the `MatchedSet` push: `let orient = |check| OrientationSet { members: vec![a, b], check,
   cell_of: Vec::new() }; r.hard.push(Box::new(orient(OrientCheck::Axis))); r.budget.push(Box::new(orient(OrientCheck::Phi)));`
   Doc table: add "Orientation (Axis hard, Φ budget)" to the three matched kinds.

Tests (`orientation.rs`): `a_quarter_turned_partner_is_illegal` (two one-unit cells φ (1,0), cell 1 `Orient::R90` →
Axis violations 1; both R0 → 0); `a_mirrored_odd_finger_partner_breaks_phi` (each cell units φx +,−,+; cell 1
`Mx180` → Phi violations 1, residual 2/3 ± 1e-6; with +,− per cell → 0 and `mirror_allowed` true);
`capacitor_units_never_violate` (φ (0,0), one cell R90 → Axis and Phi 0); `no_units_is_unknown` (unknown 1,
violations 0). Annotator tests.rs: `matched_pairs_get_orientation_rules` (`ota()`: `"Orientation"` in `hard` and in
`budget`, not in `cost`).

Run: `cargo test -p analog orientation && cargo test -p annotator`.
Acceptance (T6): bench local (same command as MAT-04): `Orientation` hard row 0 violations, 0 unknown on all 10
fixtures; DRC/LVS columns unchanged. If a fixture shows a violation, report it (layout, cell orients); do not demote
the rule to budget.
