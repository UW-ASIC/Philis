# M1 placement batch: implementation cards (PLC-02, PLC-04, PLC-03, PLC-06, PLC-10 step 0)

**Status (2026-10-04, after `git merge m1a` at `1547db2`, clean, no conflicts):** PLC-02 (`1d7a243`), PLC-04
(`721d775`), PLC-03 (`505644c`) and PLC-06 (`c29b238`) are already committed on this branch; their cards below are
kept as the record and their line numbers are the old base's. The merged tree passes `cargo check --workspace
--tests` and `cargo test -p dp` (33 passed). **Only PLC-10 step 0 remains**; its card is refreshed to today's lines.

Branch `m1a-placement`, worktree `philis-m1a/placement`, base `850560f` (`git merge m1a` fast-forwarded: no
conflicts). Spec: `plan-04-placement.md` §3. Line numbers are this base's; where the plan's differ, the card wins.
Order = dependency order: **02 → 04 → 03 → 06 → 10.0**. 06 needs 03 (dp stops reading `Layout::groups` before 06
deletes the abutment patch); 10.0 depends on nothing but its characterization literals must be captured on the tree
right before its own refactor, so it goes last.

Every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first. Crates: `pnr_core`
(kernel/core), `analog`, `gp`, `dp`, `library`, `benchmark` (bin `bench`).

**Bench protocol** (B), used by several acceptances, background it (≈ 5–15 min per seed):
```sh
cargo build --release -p benchmark --bin bench
for s in 1 2 3 4 5; do PNR_BENCH_SEED=$s ./target/release/bench local; done > /tmp/…/bench-<item>.txt 2>&1
```
Columns read: `lattice off`, `matched mismatch`, `matched incompat`, `route hard`, `DRC`, `ERC`, `key tier`, `area`.
"Lex key" below = per fixture, the median over seeds of `(route hard + DRC + ERC, key tier, area um2)` compared
lexicographically.

Batch-wide decisions (deviations from plan-04, each with its reason):

- P1. MAT-04/05 (merged) deleted `MatchingPair`, `CentroidGroup` (`cc.rs`) and `ThermalGradient` (`thermal.rs`).
  `matching_pair.rs` now holds only FET recognisers. PLC-03's hook overrides go on `Symmetry`, `SymmetryGroup`,
  `MatchedSet` and `OrientationSet` instead (matching card D2 hands this to PLC-03); every test the plan wrote on
  the deleted types is rewritten on the new ones (§6 C28).
- P2. PLC-04 drops `area_ref`. Once encroachment is its own lexicographic slot, dividing it by a constant does not
  change any comparison; PLC-18 adds the normalisation when an objective sums it.
- P3. PLC-06 drops the per-iteration and per-probe `refresh_temps`. The only power reader is `MatchedSet`, and it
  uses `Layout::rise_at_point_mc`, which is live (`kernel/core/src/thermal.rs:52-58`). No product rule reads
  `temp_mc`. gp still calls `refresh_temps()` once before it returns, so the cache stays consistent.
- P4. PLC-03 step 5 (`rotate_90_allowed` → half turns) is deferred to PLC-07 (M2), which reads the other deck
  placement keys (§6 table: "`rotate_90_allowed`, `placement_space` | PLC-03, PLC-07"). No shipped deck sets the key
  and no M1 exit criterion reads it.
- P5. `dp::place` takes the locks from its caller (`locks: &dp::locks::Locks`). It does not compute them itself
  because even epochs pass `variants = &[]` (`lib.rs:508`, `iter % 2 == 1`). Computed inside dp, the locks would
  call every pair compatible and report `matched_incompatible = 0` on those epochs.
- P6. PLC-10 step 0 ships only the flat-path fields `{range0, max_temps, t0_scale}`. The SP fields (`p0`, `alpha`,
  `moves_per_kid`) are PLC-09's, and nothing reads them in M1.

Coordination: CELL-03 (cells batch, not merged here) edits the same halo line `frontend/library/src/lib.rs:1480`
to the same text. Whichever batch merges second resolves the conflict by keeping one copy. For FLOW-08:
`escalate_blamed` should skip non-first members of a shape set (`Locks::shape_of`).

---

## PLC-02 Cut-lattice-aligned placement boxes — class: mechanical

Current facts (base `850560f`):
- `Builder::finish` rounds the corner and extents to `step = 2·grid` = `cut_lattice` = 10 nm on sky130
  (`kernel/cells/src/builder.rs:137-146`, `cut_lattice` at `:154-156`). Extents are therefore multiples of 10 nm
  only, and `hw = w/2` (`backend/gp/src/mechanics.rs:322-325`) can be an odd multiple of 5 nm.
- `place_macro` stamps the bbox corner at `x − hw` (`kernel/core/src/macro.rs:104-126`; `anchor` cancels). dp
  snaps `x`, `y` to `grid = cut_lattice` (`backend/dp/src/lib.rs:378-381`), and the legalizer and projection stay
  on `g`. So the corner is off the lattice whenever `hw ≢ 0 (mod lattice)`. chain4 is one cell with
  `lattice_off = 1` (baseline-plc-01.md).
- The ring halo is rounded to `pdk.grid` (5 nm) at `frontend/library/src/lib.rs:1480` (in `CellSpace::new`,
  `:1403`). The halo loop is `:1478-1489`; the `per_cell` frames and `UnitLib::build` follow at `:1490-1497`.
- PLC-01's metrics are computed after dp at `lib.rs:782-783` (`let place = geometry::placement_metrics(..)`).
- CELL-03 is not on this branch.

Edits:
1. `kernel/core/src/macro.rs`, in a new `impl Macro` block after the struct:
   ```rust
   impl Macro {
       /// Grow `bbox` so its lower-left corner is on `lattice` and both extents are
       /// multiples of `2·lattice`: then `hw`, `hh` and every placed corner are
       /// lattice multiples (`place_macro` stamps the corner at `x − hw`). Shapes and
       /// pins are unchanged; idempotent.
       pub fn align_bbox(&mut self, lattice: i32) {
           let l = lattice.max(1);
           let (x0, y0) = (self.bbox.x.div_euclid(l) * l, self.bbox.y.div_euclid(l) * l);
           let (x1, y1) = (self.bbox.x + self.bbox.w, self.bbox.y + self.bbox.h);
           let up = |d: i32| (d + 2 * l - 1).div_euclid(2 * l) * 2 * l;
           self.bbox = Rect { x: x0, y: y0, w: up(x1 - x0), h: up(y1 - y0) };
       }
   }
   ```
2. `frontend/library/src/lib.rs` `CellSpace::new`:
   - Bind `let lattice = cells::builder::cut_lattice(pdk);` before the halo loop.
   - At `:1480`, `round_up(.., pdk.grid.max(1))` becomes `round_up(.., lattice)`.
   - Right after the halo loop (after `:1489`) and before the `per_cell` comment at `:1490`, insert:
     ```rust
     // Origins on the cut lattice need extents on twice it (PLC-02); generated
     // and injected cells alike, before unit frames are taken from the bbox.
     for m in cells.variants.iter_mut().flat_map(|s| s.alternatives.iter_mut()) {
         m.align_bbox(lattice);
     }
     ```
   No other edit is needed: `cellgen::realize` reads `cells.variants`, so gp, dp, rings, routing and GDS all see
   the aligned boxes.
3. `Flow::epoch`: right after `let place = geometry::placement_metrics(..)` (`:783`) add
   `debug_assert_eq!(place.lattice_off, 0, "dp::place: cell origin off the cut lattice");`.

Tests:
- `kernel/core/src/macro.rs` `mod tests`:
  - `align_bbox_puts_corner_on_lattice_and_even_extents`: `Macro { bbox: Rect{x:5,y:3,w:95,h:41}, ..Default::default() }`,
    `align_bbox(10)` → `Rect{x:0,y:0,w:100,h:60}`. `Rect{x:10,y:20,w:30,h:30}` → `Rect{x:10,y:20,w:40,h:40}`.
  - `align_bbox_is_idempotent`: two calls on `(5,3,95,41)` give `(0,0,100,60)` both times.
- `frontend/library/tests/placement_metrics.rs` `every_origin_is_on_the_cut_lattice`. Generalise `run_ota` into
  `run_fixture(name: &str, cfg: &Config)` (same body, `benchmarks/fixtures/{name}.spice`; `run_ota` calls it). For
  `["ota", "dac4", "rc_filter", "chain4", "quad"]` × seeds 1..=3, run
  `Config { starts: 1, feedback_iters: 2, outer_iters: 1, seed, ..Default::default() }` (`Config::seed`,
  `lib.rs:79`) and assert `sol.stats.place.lattice_off == 0` with the fixture and seed in the message.
- Commands: `cargo test -p pnr_core align_bbox`, then
  `cargo test --release -p library --test placement_metrics` (dac4 in debug takes tens of minutes).
- Acceptance: run (B). `lattice off` must be 0 on every row. DRC per fixture must be ≤ baseline-plc-01.md (or ≤ the
  pre-item bench on this base if a fixture's baseline row has changed since M0; record both).

Risk: the guard ring is drawn inside the reservation by shrinking the placed bbox by the unrounded `ring_halo`
(`kernel/cells/src/post_cell.rs`). Padding on the right and top therefore only widens the ring's inner clearance.
If the bench DRC rises on a ringed fixture, stop and report; do not retune.

---

## PLC-04 Placement report and gate correctness — class: mechanical

Current facts:
- The gate tuple is `(phi.0, phi.1 + ov, theta)`, built at `backend/dp/src/lib.rs:197-198`. `accept` is at
  `:423-429` and takes `(usize, f64, f64)`. The module doc at `:4-8` describes the old tuple.
- `mechanics::report` (`backend/gp/src/mechanics.rs:291-316`) pushes only a "device overlap" row (`:310-313`). Its
  callers are `gp/lib.rs:246, :368` and `dp/lib.rs:267, :387`. The legalizer's return value at `dp/lib.rs:383` is
  ignored.
- `Symmetry` has no `residual` override (`kernel/analog/src/placement/symmetry.rs:17-60`; doc `:7-9` says
  "residual stays at the default 0/1"). `SymmetryGroup` (`:103-150`) does not forward `residual`, so it gets the
  `RuleBatch` default = violation count.
- `mirror_about` (`:82-90`): `half = snap_to((snap_to(xb) − snap_to(xa)) / 2, g)` truncates (25 → 12 → 10).
- `DtiBand::satisfied` uses strict ends (`kernel/analog/src/placement/dti.rs:55-57`; doc `:7-8`).
- `layout.debug_check_placed("dp::place")` is at `frontend/library/src/lib.rs:772`.

Edits:
1. `backend/dp/src/lib.rs`: `fn accept(before: (usize, f64, f64, f64), after: (usize, f64, f64, f64), d_pex: f64,
   temp: f64, rng: &mut SplitMix64) -> bool`, body unchanged. In `trial`, use
   `let before = (phi0.0, phi0.1, ov0, theta0);` and
   `let after = (phi1.0, phi1.1, ov1, analog_theta(self.reqs, l));`. In the module doc, replace "`(violating hard
   batches, Φ margin + clearance encroachment, Θ)`" with "`(violating hard batches, Σ hard residual, clearance
   encroachment nm², Θ)`". Do not add `area_ref` (P2).
2. `symmetry.rs`: in `impl Rule for Symmetry` add
   ```rust
   /// Mirror error `|ex| + |ey|` in µm: a hard margin that is a length, not a count.
   fn residual(self, l: &Layout) -> f32 {
       let (ex, ey) = self.error(l);
       (ex.abs() + ey.abs()) as f32 / 1000.0
   }
   ```
   In `impl RuleBatch<Layout> for SymmetryGroup` add `fn residual(&self, l: &Layout) -> f64 { self.0.residual(l) }`
   (the blanket `Vec` impl applies `FAIL_FLOOR`). Doc `:7-9`: replace the residual sentence with "`residual` is the
   mirror error in µm".
3. `mechanics::report(nets, reqs, l, prices, clearance: i32)`: after the overlap row add
   ```rust
   let residue = encroachment(l, clearance) - ov;
   if residue > 0.5 {
       hard_violations.push(Violation { rule: "clearance encroachment".into(), margin: residue.ceil() as i64 });
   }
   ```
   Update the doc ("plus residual device overlap and clearance-only encroachment"). Callers pass `rules.clearance`
   (gp, both sites) and `clearance` (dp, both sites; it is already bound at `:225`).
4. `frontend/library/src/lib.rs:772`: `layout.debug_check_placed("dp::place")` → `layout.debug_check("dp::place")`.
   `debug_check_placed` stays (legalize test `:171`).
5. `symmetry.rs::mirror_about`: replace the `half` line with
   ```rust
   let d = snap_to(l.x[ib], g) - snap_to(l.x[ia], g);
   // Ceiling of |d|/2 on the grid, sign kept: a projected pair never moves closer.
   let half = d.signum() * ((d.abs() + 2 * g - 1) / (2 * g)) * g;
   ```
6. `dti.rs:56`: `gap <= self.s_max_nm as f32 || gap >= self.d_dti_nm as f32`. In the doc `:7-8`, `< s_max` becomes
   `≤ s_max` and `> d_dti` becomes `≥ d_dti`.

Tests:
- `backend/dp/src/tests.rs`:
  - `gate_never_trades_a_broken_pair_for_overlap`: `let e = 1_000.0;` For 100 draws of `SplitMix64::new(1)`,
    `accept((1, 0.002, e, 0.0), (1, 0.003, e - 2.0, 0.0), -1.0, 1e12, &mut rng)` is `false`.
  - `report_counts_clearance_only_residue`: `layout(&[(0, 0, 500, 500), (1_100, 0, 500, 500)])`,
    `report(&Nets::from_macros(&[]), &Requirements::default(), &l, &gp::Prices::new(), 270)`. Expect exactly one
    row with `rule == "clearance encroachment"` and `margin == 215_900` (170 × 1270), and no "device overlap" row.
- `symmetry.rs` tests:
  - `symmetry_residual_is_the_mirror_error_in_um`: `layout(-1_000, 0, 1_500, 0)` with axis 0, then
    `SymmetryGroup(vec![rule()]).residual(&l) == 0.5`.
  - `odd_parity_projection_never_shrinks_the_pair`: `layout(0, 0, 25, 0)`, `rule().project(&mut l, 5)` →
    `l.x[1] - l.x[0] == 30` and `rule().satisfied(&l)`.
- `dti.rs` `a_pair_exactly_at_either_end_is_legal`: `rule().satisfied(&bench(200, false))` and
  `rule().satisfied(&bench(2_000, true))`.
- Commands: `cargo test -p dp`, `cargo test -p analog symmetry`, `cargo test -p analog dti`,
  `cargo test -p library --test flow_smoke --test injected_macro --test placement_metrics` (debug: no placement
  panic).
- Acceptance: the tests above pass, and no debug flow test panics in placement. Run (B) and record lex keys next to
  PLC-02's. Watch one cross-module effect: gp's Φ gate (`gp/lib.rs:340-355`) compares `analog_phi` tuples, so it
  now also rejects steps that raise the summed mirror error in µm. Before this item, each violated `Symmetry` rule
  added 1.0, so the gate saw only the count. If a fixture's lex key regresses, stop and report it with the bench
  rows. Do not change gp's gate inside this item.

---

## PLC-03 Matched-set orientation and shape locks — class: judgment

Judgment because it spans analog, dp, library and metrics, and redefines `matched_geometry_mismatch`. The exit
criterion (`== 0` on every fixture and seed) holds by construction only if no matched pair is index-incompatible
and also a `Symmetry` pair (see the risk at the end).

Current facts:
- `Rule::mirror_pair` is at `kernel/analog/src/rule.rs:122-127`; `RuleBatch::mirror_pairs` at `:240-243`; the
  blanket impl at `:328-330`. No `matched_pair(s)` hook exists.
- Matched rules on this base (after MAT-04/05; P1):
  - `Symmetry` (device ids after retarget).
  - `SymmetryGroup` (`symmetry.rs:101-150`).
  - `MatchedSet` (`matched_set.rs:22-41`): members are schematic devices; `cell(d)` (`:52-54`, private) maps them
    through `cell_of`. Emitted into `budget` and `cost`.
  - `OrientationSet` (`orientation.rs:30-41`; `cell(d) -> u32`): `Axis` in `hard`, `Phi` in `budget`
    (`backend/annotator/src/emit.rs:117-139`).
- dp:
  - `rotatable` (`backend/dp/src/lib.rs:577-582`) reads `Layout::groups`, which the library sets to
    `cells.abutment` (`lib.rs:753`).
  - `try_rotate(c)` is at `:584-600`. The move loop calls it at `:361`.
  - `try_reshape(c)` is at `:602-642`; the loop calls it at `:359`; the refusal path restores pins at `:636-640`.
- `Symmetry::satisfied` (`symmetry.rs:25-27`) checks positions only.
- `placement_metrics` (`frontend/library/src/geometry.rs:41-69`) counts hard mirror pairs whose
  `(variant, orient, hw, hh)` differ. Its test `placement_metrics_counts_each_defect` is at `geometry.rs:361-388`.
- `PlaceStats::matched_incompatible` is `None` (`dp/lib.rs:47-48`); the bench prints it (`bench.rs:307`).
- Callers:
  - `dp::place`: `lib.rs:761` and the dp test helper `run` (`tests.rs:34-43`), plus `tests.rs:218`.
  - `try_reshape` is called directly in `tests.rs:186, :191`.

Edits:
1. `kernel/analog/src/rule.rs`:
   - In `Rule`, after `mirror_pair`:
     ```rust
     /// `(a, b)` cells that must be drawn alike (same orientation, same variant when
     /// their variant spaces agree): what dp's locks preserve. Default none.
     #[inline]
     fn matched_pair(self) -> Option<(u32, u32)> { None }
     ```
   - In `RuleBatch`, after `mirror_pairs`:
     `/// Append every matched pair (see [`Rule::matched_pair`]).` followed by
     `fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) { let _ = out; }`.
   - In the blanket impl:
     `fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) { out.extend(self.iter().filter_map(|r| r.matched_pair())); }`.
2. Overrides:
   - `symmetry.rs`, `impl Rule for Symmetry`:
     `fn matched_pair(self) -> Option<(u32, u32)> { match (self.a, self.b) { (Target::Device(a), Target::Device(b)) if a != b => Some((u32::from(a.0), u32::from(b.0))), _ => None } }`.
   - `impl RuleBatch for SymmetryGroup`: `fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) { self.0.matched_pairs(out); }`.
   - `matched_set.rs`, `impl RuleBatch for MatchedSet`:
     `fn matched_pairs(&self, out) { let c0 = self.cell(self.members[0]) as u32; out.extend(self.members[1..].iter().map(|&m| (c0, self.cell(m) as u32))); }`.
     Guard `members.is_empty()` first.
   - `orientation.rs`, `impl RuleBatch for OrientationSet`: the same shape, using its `cell()` (already `u32`).
   - Pairs whose two cells coincide are emitted as-is; `locks` drops them.
3. New `backend/dp/src/locks.rs`; add `pub mod locks;` in `dp/lib.rs`:
   ```rust
   //! Matched cells move as one (PLC-03): a set rotates together and, when the
   //! members' variant spaces agree index by index, reshapes together.
   #[derive(Clone, Debug, Default)]
   pub struct Locks {
       /// Cells that rotate together. Singleton sets omitted; each set sorted, sets sorted by first member.
       pub orient: Vec<Vec<u16>>,
       /// Cells that reshape together (same variant index). Same conventions.
       pub shape: Vec<Vec<u16>>,
       /// Cell → index into `orient` / `shape`; `None` = unlocked. Length `n`.
       pub orient_of: Vec<Option<u16>>,
       pub shape_of: Vec<Option<u16>>,
       /// Distinct matched pairs whose variant spaces differ (not shape-locked).
       pub incompatible: u32,
   }
   pub fn locks(reqs: &Requirements<Layout>, n: usize, variants: &[gp::VariantSpace]) -> Locks;
   impl Locks {
       /// Cells sharing `c`'s orient (`shape = false`) or shape set; `[c]` when unlocked.
       pub fn members(&self, c: usize, shape: bool) -> Vec<usize>;
       /// Set every shape set's members to its first member's variant.
       pub fn unify(&self, assignment: &mut [u16]);
   }
   ```
   Algorithm for `locks`:
   - (a) Collect `matched_pairs` from `reqs.hard`, `reqs.budget` and `reqs.cost`. Drop pairs with `a == b` or an
     index `>= n`. Normalise each pair to `(min, max)`, then `sort_unstable` and `dedup`.
   - (b) `pnr_core::UnionFind::new(n)` over every pair gives the orient sets.
   - (c) A second `UnionFind` over the compatible pairs gives the shape sets. A pair is compatible when
     `dims(a) == dims(b)`, with
     `dims = |i| variants.get(i).map(|s| s.alternatives.iter().map(|m| (m.bbox.w, m.bbox.h)).collect::<Vec<_>>())`
     (`None == None`, so with `variants` empty every pair is compatible). Count each incompatible pair in
     `incompatible`.
   - (d) Take each union-find's `groups()`, keep groups with `len() >= 2`, convert to `u16`, sort each group and
     the list, then fill the `_of` maps. `unify` sets `assignment[m] = assignment[set[0]]` for every shape set,
     skipping indices `>= assignment.len()`.
4. `backend/dp/src/lib.rs`:
   - `place(...)` gains `locks: &locks::Locks` after `fixed`. Update the doc line.
   - Set `sa.stats.matched_incompatible = Some(locks.incompatible)` right after `Sa::new`. The `n == 0` return
     becomes `PlaceStats { matched_incompatible: Some(locks.incompatible), ..Default::default() }`.
   - Delete `rotatable` (`:577-582`).
   - `try_rotate(sa, l, rng, temp, set: &[usize], clamp_x, clamp_y)` applies the existing body to every
     `m in set` inside one `sa.trial`. Doc: "Turn a whole orient set a quarter, in one trial".
   - The move loop's last branch becomes
     `{ let set = locks.members(c, false); can_rotate && !set.iter().any(|&m| sa.is_fixed(m)) && try_rotate(&mut sa, &mut l, &mut rng, temp, &set, &clamp_x, &clamp_y) }`.
   - `try_reshape(sa, l, rng, temp, set: &[usize], variants, clamp_x, clamp_y)`:
     - Let `c = set[0]`. Use `depth` and `cur` of `c`. Return `false` if `depth < 2` or any member is fixed or has
       `alternatives.len() != depth`.
     - Draw `next` once. Inside one trial, apply the existing per-cell body (variant, extents, clamp,
       `nets.reshape_cell`) to every member.
     - On refusal, restore every member's pins (the loop over the `:636-640` body).
     - The loop calls it with `&locks.members(c, true)`.
   - `Layout::groups` is no longer read by dp. `place` still copies it through (`:236`).
5. `frontend/library/src/lib.rs`:
   - After `let cells = CellSpace::new(..)` (`:417`):
     `let locks = dp::locks::locks(&problem.placement, cells.variants.len(), &cells.variants);`.
   - `Flow` (`:631`) gains `locks: dp::locks::Locks` (doc: "Matched-cell orient/shape locks (PLC-03), over all
     variants"). Set it in the constructor at `:444`.
   - First lines of `Flow::epoch`:
     `let unified = { let mut a = assignment.to_vec(); self.locks.unify(&mut a); a }; let assignment = &unified[..];`.
   - Pass `&self.locks` to `dp::place` after `&cells.fixed`.
6. `symmetry.rs`:
   - `Symmetry::satisfied` becomes `self.error(l) == (0, 0) && self.same_shape(l)`, with
     ```rust
     /// Perfect symmetry (PLC-03; Mirror mode is PLC-21): distinct partners drawn
     /// alike. A column too short to hold either index reads as equal.
     fn same_shape(self, l: &Layout) -> bool {
         let Some((ia, ib)) = self.indices(l) else { return true };
         let eq = |v: &[_]| v.get(ia).zip(v.get(ib)).is_none_or(|(a, b)| a == b);
         ia == ib || (l.hw[ia] == l.hw[ib] && l.hh[ia] == l.hh[ib] && eq(&l.variant) && eq(&l.orient))
     }
     ```
     If the closure does not infer over two element types, write two closures: `eq_u16` for `variant` and
     `eq_orient` for `orient`.
   - Update the struct doc: "partners also share `hw`, `hh`, `variant`, `orient`".
7. `frontend/library/src/geometry.rs` `placement_metrics(.., reqs, locks: &dp::locks::Locks)`:
   - Collect `matched_pairs` from `hard`, `budget` and `cost`.
   - `matched_geometry_mismatch` = number of distinct pairs `a ≠ b`, both `< n`, where
     `l.orient.get(a) != l.orient.get(b)`, or where `locks.shape_of[a]` is `Some` and equals `shape_of[b]` while
     `(variant, hw, hh)` differ.
   - Field doc: "Matched pairs (`RuleBatch::matched_pairs`) drawn at different orient, or at different shape
     inside one shape set".
   - Caller at `lib.rs:783` passes `&self.locks`.

Tests (`backend/dp/src/tests.rs` unless noted). The `run` helper builds
`let locks = locks::locks(reqs, coarse.x.len(), variants);` and passes it. `tests.rs:218` does the same.
- `mirror_partners_reshape_together`:
  - Setup: `variants` = 2 × `VariantSpace { alternatives: vec![alt(1_000, 4_000), alt(2_000, 2_000), alt(4_000, 1_000)] }`;
    `coarse = layout(&[(-20_000, 0, 500, 2_000), (20_000, 0, 500, 2_000)])`; `reqs.hard` =
    `SymmetryGroup(vec![Symmetry{a: Dev 0, b: Dev 1, axis: AxisId(0)}])`.
  - For seeds 1..=20: `l.variant[0] == l.variant[1]`.
  - For at least one seed: `l.variant[0] != 0`.
- `mixed_polarity_composite_halves_turn_together`:
  - Setup: `rotate_bench()` (its `groups` are now ignored), with `reqs.budget` =
    `OrientationSet { members: vec![DeviceId(0), DeviceId(1)], check: OrientCheck::Phi, cell_of: vec![] }`.
    This is not a hard rule and there are no units, so the lock can only come from `matched_pairs`.
  - For seeds 1..=20: `l.orient[0] == l.orient[1]`.
  - For at least one seed: `l.orient[0] != Orient::R0`, so the test cannot pass on a lock that never turns.
- `dp/src/locks.rs` `#[cfg(test)]` `locks_join_only_compatible_shapes`:
  - Setup: `reqs.cost` = `vec![sym(0,1), sym(0,2)]` (`Symmetry`); spaces: cells 0 and 1 `[alt(1_000,4_000)]`,
    cell 2 `[alt(2_000,2_000)]` (build `Macro` with only `bbox`), `n = 3`.
  - Expect `orient == [[0,1,2]]`, `shape == [[0,1]]`, `incompatible == 1`, `shape_of[2] == None`.
  - Then `unify` on `[2, 0, 1]` → `[2, 2, 1]`.
- `matched_cells_share_one_orient_set` (replaces `matched_devices_never_turn`, `:54-59`): a `Symmetry` pair (0,1)
  in `hard`, `n = 3` → `orient_of[0] == orient_of[1]`, `orient_of[0].is_some()`, `orient_of[2] == None`.
- `extents_stay_in_lockstep_with_orientation` (`:61-71`): requirements become `hard: [SymmetryGroup([sym(0,1)])]`;
  replace the `(R0, R0)` assertion with `l.orient[0] == l.orient[1]`.
- Rename `cells_reshape_independently` → `unmatched_cells_reshape_independently`. Body unchanged (no pair rules).
- `reshape_is_priced_on_where_the_pins_land`: the calls become `try_reshape(.., &[0], &variants, ..)`.
- `symmetry.rs` `unequal_variants_are_not_symmetric`: `layout(-500, 0, 500, 0)` with axis 0 is satisfied. With
  `l.variant = vec![0, 1]` it is `!satisfied`.
- `geometry.rs` `placement_metrics_counts_each_defect`: pass `&dp::locks::locks(&reqs, 3, &[])`. With no variants
  every pair is shape-locked, so the soft pair (1,2) is now matched and differs in variant. Replace the comment
  "A soft pair is not a hard mirror pair" with "matched pairs come from every tier" and expect
  `matched_geometry_mismatch == 2`. Add a fourth case to keep the orient branch covered: `l.orient[1] = Orient::R90`
  with `hw`/`hh` unchanged gives 3. This redefinition is specified by the plan, not a looser threshold.
- Commands: `cargo test -p analog`, `cargo test -p dp`, `cargo test -p library --lib geometry`,
  `cargo test --release -p library --test placement_metrics`.
- Acceptance: run (B). `matched mismatch` must be 0 on every row, and `matched incompat` must print a number (not
  `n/a`) on every row. Record it per fixture. Report every fixture where it is > 0 together with its `place_hard`.

Risk: a matched pair that is also a `Symmetry` pair but has differing variant spaces is now a permanent hard
`Symmetry` violation (an EXT error, AA-05). One way this can happen is a guard-ring halo on only one partner. The
`matched incompat` column makes it visible. Do not exempt it.

---

## PLC-06 gp sees block axes, power, units; fixed means shape-frozen — class: judgment

Judgment because it changes what gp optimises: per-block axes and the live thermal term that `MatchedSet` now
supplies. Its acceptance is a measured no-regression, and `gp_sees_power` is statistical.

Current facts:
- `gp::place(macros, variants, assignment, reqs, prices, rules, net_weight, seed, iterate)` is at
  `backend/gp/src/lib.rs:223-370`. `save_x` and the momentum step at `:340-347`; Φ gate `:349-355`.
- `initial_layout` (`mechanics.rs:362-386`) sets `axis = vec![c; n]` (one per cell, not per block).
- The library patches gp's output at `lib.rs:751-759`: `groups = abutment`, axis resize to `blocks.len()`,
  `power_uw`, `units`, `refresh_temps`.
- gp never calls `refresh_temps`.
- `ThermalGradient` is deleted. The thermal term is in `MatchedSet::ledger_with`, which reads power live through
  `rise_at_point_mc` (`matched_set.rs:118-122`). See P3.
- dp `fixed`:
  - Position restore after projection at `dp/lib.rs:180-183`.
  - `project_hard(.., fixed, ..)` at `:393-419` (restore `:405-410`), called at `:366`.
  - `sym_groups(.., fixed)` at `:464-489` (filter `:487`), called at `:261`.
  - Per-move skip at `:329-331`; swap-branch guard `!sa.is_fixed(o)` at `:354`; legalize call at `:383`.
- `legalize::separate_overlaps(.., fixed, ..)` (`legalize.rs:17-90`): `movable` at `:27`, restore loop `:70-73`.
  Its test `a_pinned_macro_never_moves` is at `:126-133`.
- The doc for `fixed` is at `dp/lib.rs:213` and the field doc at `lib.rs:1382`. Other gp callers:
  `dp/tests.rs:217`.

Edits:
1. `backend/gp/src/lib.rs`:
   ```rust
   /// Everything `place` reads.
   pub struct GpInput<'a> {
       pub macros: &'a [Macro],
       pub variants: &'a [VariantSpace],
       pub assignment: &'a [u16],
       pub reqs: &'a Requirements<Layout>,
       pub rules: Rules,
       pub net_weight: &'a [f32],
       /// Symmetry axes, one per block (`Problem::blocks`).
       pub n_axes: usize,
       /// Per cell, µW; empty = unpowered.
       pub power_uw: &'a [i32],
       pub units: std::sync::Arc<pnr_core::UnitLib>,
       /// `false`: return the initial pile (`GpMode::Pile`).
       pub iterate: bool,
   }
   pub fn place(inp: &GpInput, prices: &mut Prices, seed: u64) -> (Layout, Report)
   ```
   Body changes, in order:
   - Destructure `inp` at the top.
   - `initial_layout(&drawn, variant, side, inp.n_axes, &mut rng)`. In `mechanics.rs` the signature gains
     `n_axes: usize` before `rng`, and the body uses `axis: vec![c; n_axes.max(1)]`.
   - Then: `if inp.power_uw.len() == n { l.power_uw.copy_from_slice(inp.power_uw); } l.units = inp.units.clone();`.
   - Collect the mirror pairs once:
     `let mut pairs = Vec::new(); for b in &reqs.hard { b.mirror_pairs(&mut pairs); }`.
   - In the loop, before `save_x.copy_from_slice` add `save_axis.clone_from(&l.axis)` (declared next to `save_x`).
   - After the momentum `for` (ends `:347`) and before the Φ check, recompute every axis:
     ```rust
     // Each block's axis follows its pairs (mean midpoint), so two stages are not pinned to one line.
     for id in 0..l.axis.len() {
         let (s, k) = pairs.iter().filter(|p| usize::from(p.2) == id && (p.0 as usize) < n && (p.1 as usize) < n)
             .fold((0i64, 0i64), |(s, k), p| (s + i64::from(l.x[p.0 as usize]) + i64::from(l.x[p.1 as usize]), k + 2));
         if k > 0 { l.axis[id] = snap((s / k) as i32, rules.grid); }
     }
     ```
     Import `snap` from `mechanics`.
   - In the rejection branch also `std::mem::swap(&mut l.axis, &mut save_axis);`.
   - Call `l.refresh_temps();` before both `report(..)` returns. Do not add probe refreshes (P3).
2. `frontend/library/src/lib.rs` `Flow::epoch`:
   - Replace the `gp::place(..)` call (`:749`) with a `gp::GpInput` built with these fields:
     - `macros: &macros`, `variants: &cells.variants`, `assignment`, `reqs: placement`,
     - `rules: place_rules(self.pdk)`, `net_weight: &self.net_weight`,
     - `n_axes: self.problem.blocks.len()`, `power_uw: &cells.power`, `units: cells.units.clone()`,
     - `iterate: self.gp_mode == GpMode::Analytic`.
     Then call `gp::place(&inp, prices, seed)`.
   - Delete `:751-759` (the groups/axis/power/units/refresh patch); keep `coarse.debug_check("gp::place")`.
   - Delete `CellSpace::abutment` (field `:1386`, init `:1469`): it has no reader left, and leaving it causes a
     dead-field warning. `problem.abutment` stays because the annotator owns it.
3. `backend/dp/src/lib.rs`, where `fixed` now means drawn as given (no reshape, no rotation) while the position is
   placed like any cell:
   - Delete `:180-183`.
   - `project_hard(reqs, l, grid)`: drop the `fixed` parameter and `:405-410`. Fix its doc ("restore pinned cells").
   - `sym_groups(reqs, n)`: drop `fixed` and `:487`. Doc: delete "Groups with a fixed member are left to
     projection."
   - Delete `if sa.is_fixed(c) { continue; }` (`:329-331`); the rotate and reshape branches already refuse fixed
     members (PLC-03).
   - Swap branch: `o != c && try_swap(..)`.
   - Legalize call: drop `fixed`.
   - Doc `:213`: "`fixed[i]` draws cell `i` as given (no reshape, no rotation); its position is placed like any
     cell's."
   - Extract `:244-260` into
     `fn seed_branches(reqs: &Requirements<Layout>, branch: &mut Vec<bool>) -> Vec<BranchId>` (returns
     `branch_ids`) and call it as `let branch_ids = seed_branches(reqs, &mut l.branch);`.
4. `backend/dp/src/legalize.rs`:
   - Drop the `fixed` parameter, `movable`, the `push` match (always `(whole / 2 + whole % 2, whole / 2)`), and the
     restore loop `:70-73`.
   - Doc: delete "Pinned devices never move".
   - Update the six test calls (drop the `&[..]` argument) and delete `a_pinned_macro_never_moves`.
5. `lib.rs:1382` doc: "Injected (user-macro) cells, drawn as given: dp never reshapes or rotates them."

Tests:
- `backend/gp/src/lib.rs` new `#[cfg(test)] mod place_tests`:
  - Helper: `fn input<'a>(macros: &'a [Macro], reqs: &'a Requirements<Layout>, n_axes: usize, power: &'a [i32]) -> GpInput<'a>`
    with `rules: Rules { grid: 10, clearance: 270 }`, `iterate: true`, `units: Default::default()` and empty slices
    for everything else.
  - `gp_keeps_one_axis_per_block`:
    - Setup: 6 cells `Macro { bbox: Rect{x:0,y:0,w:1_000,h:1_000}, ..Default::default() }`. `hard` and `cost` each
      hold `SymmetryGroup([sym(0,1, axis 0)])` and `SymmetryGroup([sym(2,3, axis 1)])`. `n_axes = 2`.
    - For seeds 1..=5: `l.axis.len() == 2`.
    - For at least one seed: `l.axis[0] != l.axis[1]`.
  - `gp_sees_power` (C28 rewrite of the `ThermalGradient` test):
    - Setup: 3 cells as above; `reqs.cost` =
      `MatchedSet { members: [Dev 0, Dev 1], kind: MatchKind::Voltage, mos: true, coeffs: Coeffs { tc_uv_per_k: Some(1_000.0), ..Default::default() }, budget: Budget::Allowance(1.0), gate_um2: vec![], tol_nm: 5.0, cell_of: vec![] }`.
    - For seeds 1..=10, run with `power = [0, 0, 10_000]` and with `[0, 0, 0]`. Score each output under the true
      field: `l.power_uw = vec![0, 0, 10_000]`, then
      `(l.rise_at_point_mc(l.x[0], l.y[0]) - l.rise_at_point_mc(l.x[1], l.y[1])).abs()`.
    - Assert the powered mean is strictly lower.
    - If it fails, report it with the per-seed numbers. Do not raise `tc` or change the seeds to make it pass.
- `backend/dp/src/tests.rs`:
  - Update the `run` helper and `tests.rs:217` (`GpInput`) for the new signatures.
  - `fixed_cells_separate_but_keep_shape`: `variants = spaces()`,
    `coarse = layout(&[(0, 0, 1_000, 8_000), (0, 0, 1_000, 8_000)])`, `fixed = [true, true]`. For seeds 0..4:
    `encroachment(&l, 0) == 0.0`, `l.variant == [0, 0]`, `l.orient == [R0, R0]`,
    `(l.hw, l.hh) == (coarse.hw, coarse.hh)`.
  - `projection_never_moves_pinned_cells` → `projection_moves_fixed_cells_but_not_their_shape`: `sym_bench()`,
    `fixed = [true, false]`, seeds 0..4 → `analog_violations(&reqs, &l) == 0`,
    `(l.hw[0], l.hh[0], l.orient[0]) == (coarse.hw[0], coarse.hh[0], R0)`.
  - `seeds_are_written_and_table_resized`: test `seed_branches` directly. `let mut b = Vec::new();` with
    `band(3, true)` gives `b == [false, false, false, true]`. Starting from `vec![false; 6]` gives
    `[false, false, false, true, false, false]`.
  - `a_fixed_cell_never_reshapes` stays unchanged.
- Commands: `cargo test -p gp`, `cargo test -p dp`, `cargo test -p library --test injected_macro --test flow_smoke`.
- Acceptance:
  - `injected_macro.rs` passes.
  - Run (B) on the commit before PLC-06 (after PLC-03) and after it. No fixture's lex key regresses (median, seeds
    1–5). Record both tables. A regression is reported, not tuned away.
  - The ablation note in baseline-plc-01.md (Pile vs Analytic) is now measured on this tree; leave it to PLC-11.

---

## PLC-10 step 0 `dp::Schedule` (flat path) — class: mechanical

Current facts (tree `1547db2`, after PLC-03/06 and the matching merge; the plan's `:21-27`, `:276-289` are stale):
- `const MAX_ITERS: u32 = 220;` at `backend/dp/src/lib.rs:23`; `const RANGE0: f32 = 0.4;` at `:28` (doc `:27`).
  `ALPHA`, `MOVES_PER_CELL`, `RANGE_DECAY`, `REGION_FILL` stay constants (not in step 0's scope).
- `pub fn place(coarse, macros, variants, reqs, fixed, locks, prices, rules, net_weight, seed)` at `:216-227`
  (doc `:205-214`, `#[allow(clippy::too_many_arguments)]` at `:215`).
- Probe: comment `:286`, `let mut range = RANGE0;` `:287`, 128 probe moves `:290-298`,
  `let mut temp = (sum / 128.0).max(1.0) * 0.02;` `:299`. Loop `for _ in 0..MAX_ITERS {` `:311`.
- Callers of `dp::place`: `frontend/library/src/lib.rs:785` (epoch), the `run` helper `backend/dp/src/tests.rs:34-44`
  (call `:43`), and `place_does_not_settle` `tests.rs:257` (call `:287`). No other caller in the workspace.
- `rotate_bench()` is at `tests.rs:53-57`; its `groups` are ignored since PLC-03, it is still a valid 4-cell input.
- FLOW-08 (plan-08 L727-731) calls `dp::place(…, dp::Schedule::cold()|warm())` with the schedule trailing and names
  only `range0`, `max_temps`, `t0_scale` (`..` for the rest). P6: the SP fields are not added.

Edits:
1. **First**, on today's tree, add to `backend/dp/src/tests.rs`:
   ```rust
   /// PLC-10 step 0: `Schedule::cold()` is exactly the constants it replaced.
   #[test]
   fn cold_schedule_reproduces_todays_layout() {
       let l = run(&rotate_bench(), &[], &[], &Requirements::default(), &[false; 4], 7);
       assert_eq!(l.x, vec![/* literals */]);
       assert_eq!(l.y, vec![/* literals */]);
       assert_eq!(l.orient, vec![/* literals */]);
   }
   ```
   Fill the literals by temporarily adding `eprintln!("{:?} {:?} {:?}", l.x, l.y, l.orient);`, running
   `cargo test -p dp cold_schedule -- --nocapture`, pasting, removing the print. Commit it green before step 2 (same
   item commit is fine if step 2 is applied after the literals are captured and the test is re-run unchanged).
2. `backend/dp/src/lib.rs`:
   - Delete `MAX_ITERS` (`:23`) and `RANGE0` with its doc (`:27-28`).
   - After the constants, add:
     ```rust
     /// The flat anneal's schedule (PLC-10 step 0): initial move window `range0` (fraction of the
     /// die span), `max_temps` temperature steps, `t0 = t0_scale · mean|ΔPEX|` over 128 probe moves.
     #[derive(Clone, Copy, Debug, PartialEq)]
     pub struct Schedule {
         pub range0: f32,
         pub max_temps: u32,
         pub t0_scale: f64,
     }
     impl Schedule {
         /// Today's constants: refine gp, don't randomise it.
         pub fn cold() -> Self { Self { range0: 0.4, max_temps: 220, t0_scale: 0.02 } }
         /// FLOW-08 step 3's flat warm start from an incumbent [policy, measure].
         pub fn warm() -> Self { Self { range0: 0.05, max_temps: 60, t0_scale: 0.002 } }
     }
     ```
   - `place` gains a trailing `schedule: Schedule` after `seed`. Doc: add "`schedule` sets the anneal's window,
     length and starting temperature ([`Schedule::cold`] is the gp-refining default)."
   - `:286` comment → `// t0 = t0_scale · mean |ΔPEX| over probe moves.`; `:287` → `let mut range = schedule.range0;`;
     `:299` → `... * schedule.t0_scale;`; `:311` → `for _ in 0..schedule.max_temps {`.
3. Callers append `Schedule::cold()` (`dp::Schedule::cold()` in library): `lib.rs:785`, `tests.rs:43`, `tests.rs:287`.
   No caller passes `warm()` in M1 (FLOW-08 is its first reader).

Tests: `cold_schedule_reproduces_todays_layout` (step 1) must stay green unchanged after step 2; it is the
bit-identity proof (`temp` is `f64`, `range` `f32`, so moving the literals into fields cannot change arithmetic).
Commands: `cargo test -p dp`; `cargo check --workspace --tests`; `cargo test --release -p library --test
placement_metrics` (must still pass). Acceptance: all green, no bench needed (no behaviour change).
