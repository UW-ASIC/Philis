# M2+ placement, segment 9 (M3): PLC-21, PLC-14

HEAD 69e8bd4 (m2 fast-forwarded in). Deps merged: PLC-03 (`ce78a7d`), MAT-01/MAT-05 (`fd88a21`), EXT-14
(`00aecbc`), MAT-07 (`c068dff`), RTE-15 (`c063c04`), PLC-06 code in (M1 `c29b238`; its owner decision does not touch these items).

## PLC-21 Mirror versus perfect symmetry (class: do)

Facts (plan corrections marked **stale**):
- `kernel/core/src/geom.rs:31-75` `Orient` (D4) with `apply`, `apply_rect`, `swaps_axes`; no `then`/`inverse`. The 8-member
  list `ALL` exists only in the test module (`geom.rs:~108`); `the_eight_transforms_are_distinct` proves `(1, 2)` separates all 8.
- `kernel/analog/src/placement/symmetry.rs:20` `Symmetry { a, b, axis }` (Copy, no mode). `satisfied` (`:35`) = mirror error 0
  && `same_shape` (`:99`), which requires `orient[a] == orient[b]`. Axis is always vertical (`error` uses x only).
- **stale** "decided in `solve()`": Symmetry rules are emitted by the annotator, `backend/annotator/src/emit.rs:162-169`, from
  `intent.compounds`; `Compound.kind: SymKind {Mirror, Perfect}` is set at `backend/annotator/src/lib.rs:324`. Unit geometry
  (φ) is only known after `CellSpace::new` in `frontend/library/src/lib.rs:733` (`fn topology`, there is no `solve`).
  So: annotator sets the mode from `kind`; `topology` demotes after `CellSpace::new`.
- `kernel/analog/src/matching/moments.rs:139` `mirror_allowed(&[Sums]) -> bool` (every `s.phi_x == 0`); `sums`, `Pt: From<Unit>`
  at `:22,:57`. `Macro.units: Vec<Unit>` with `owner`, `phi`.
- dp: `backend/dp/src/lib.rs:584-597` `quarter_turn`, `:602-618` `try_rotate` turns every member of `locks.members(c, false)`
  by the same quarter; called at `:366-369`. **stale** "flat `try_rotate_set`": does not exist; dp's `try_rotate` is the only rotate.
- `backend/dp/src/locks.rs:27-54` orient sets = union of `matched_pairs` (Symmetry `matched_pair` is every distinct a≠b pair).
- `RuleBatch` hooks: `kernel/analog/src/rule.rs` (defaults ~`:260-300`, `Tagged` forwarding `:313-392`, `Vec<R>` `:394-480`);
  `SymmetryGroup` impl `symmetry.rs:138`.
- Symmetry struct literals to update (+`mode: SymMode::Perfect` unless stated): `emit.rs:164-165`, `backend/gp/src/lib.rs:908`,
  `kernel/analog/src/placement/dti.rs:195`, `symmetry.rs` tests (`:231,:307-308,:341-342,:377,:403`), `backend/dp/src/tests.rs:50,437,484,539,626`,
  `backend/dp/src/locks.rs:112`, `frontend/library/src/geometry.rs:390`. (`cargo check --workspace --tests` finds any missed.)

Edits:
1. `geom.rs`: move `ALL` out of the test module as `impl Orient { pub const ALL: [Orient; 8] }` (tests use `Orient::ALL`) and add
   ```rust
   /// `self` then `next`: `a.then(b).apply(p) == b.apply(a.apply(p))`. D4 members are told apart by
   /// the image of (1, 2) (`the_eight_transforms_are_distinct`), so the product is found by search.
   pub fn then(self, next: Orient) -> Orient   // find o in ALL with o.apply(1,2) == next.apply(self.apply(1,2)) (unwrap: D4 is closed)
   pub fn inverse(self) -> Orient              // find o in ALL with self.then(o) == R0
   ```
   (`// ponytail:` 8-way search, not the plan's `const [[Orient;8];8]` table; a table only if a profile shows it.)
2. `symmetry.rs`: `#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] pub enum SymMode { #[default] Perfect, Mirror }`;
   `Symmetry { a, b, axis, pub mode: SymMode }`. `same_shape`: orient check becomes
   `want = match mode { Perfect => orient[ia], Mirror => orient[ia].then(Orient::Mx180) }; orient[ib] == want` (column too
   short still reads equal; `ia == ib` passes). Re-export `SymMode` in `placement/mod.rs`.
3. `rule.rs` `RuleBatch`, two hooks (default no-op, forwarded by `Tagged`):
   `fn mirrored_pairs(&self, out: &mut Vec<(u32, u32)>)` (distinct Mirror pairs) and
   `fn demote_mirrors(&mut self, keep: &dyn Fn(u32, u32) -> bool)` (Mirror → Perfect where `!keep(a, b)`). Implemented on
   `SymmetryGroup` only (`Vec<Symmetry>` is never a placement batch; ponytail note).
4. `moments.rs`: `pub fn mirror_allowed_units(units: &[pnr_core::Unit]) -> bool` = per owner `sums(..)`, then `mirror_allowed`
   (same grouping as `phi_equal_all` `:127`).
5. `emit.rs:164`: `mode: if c.kind == SymKind::Mirror { SymMode::Mirror } else { SymMode::Perfect }`; selfs (`:165`) Perfect.
6. `frontend/library/src/lib.rs` `topology`, right after `CellSpace::new` (`:733`), before `locks` (`:739`):
   `keep = |a, b| a != b && [a, b].all(|c| cells.variants[c].alternatives.iter().all(|m| mirror_allowed_units(&m.units)))`
   over `problem.placement.{hard,budget,cost}`. Correction to the plan: **every** variant, not variant 0, since dp reshapes.
7. `locks.rs`: `Locks.rel: Vec<Orient>` (length n, `R0` default) = each cell's orient relative to its set's first member.
   Collect `mirrored_pairs` alongside `matched_pairs`; after `sets`, walk each set from `set[0]` (BFS over the pair edges):
   `rel[b] = rel[a].then(Mx180)` across a mirrored edge, `rel[a]` otherwise; first assignment wins (`// ponytail:` an
   inconsistent cycle is left to `Symmetry::satisfied` to report). `pub fn align(&self, orient: &mut [Orient])`:
   `orient[m] = orient[set[0]].then(rel[m])` for every set member.
8. dp `place` (`lib.rs:261` after `refresh_temps`): `if l.orient.len() == n { locks.align(&mut l.orient) }`.
   `try_rotate(.., set, rel: &[Orient], ..)`: `let o0 = quarter_turn(l.orient[set[0]]);` each member gets `o0.then(rel[c])`
   (hw/hh swap unchanged: `Mx180` preserves `swaps_axes`). Call site passes `&locks.rel`. Update the doc comment on
   `try_rotate` ("No move introduces a mirror": now only seeding does, for Mirror pairs).

Tests:
- `kernel/core/src/geom.rs` `then_matches_apply_on_all_64_products`: for all a, b in `ALL`:
  `a.then(b).apply(1, 2) == b.apply(a.apply(1, 2))` and `a.then(a.inverse()) == Orient::R0`.
- `kernel/analog/src/matching/moments.rs` `mirror_is_refused_for_an_odd_finger_pair`: one owner, φ `(1,0),(-1,0),(1,0)` →
  `!mirror_allowed_units`; `(1,0),(-1,0)` → allowed; φ `(0,1)` ×3 → allowed.
- `symmetry.rs` `mirror_pair_satisfies_only_with_mx180_partner`: pair mirrored about the axis, `mode: Mirror`: orients
  `(R0, R0)` → `!satisfied`; `(R0, Mx180)` → satisfied; `(R90, R90.then(Mx180))` → satisfied; same layout with `Perfect` and
  `(R0, Mx180)` → `!satisfied`. Plus `demote_mirrors_keeps_only_allowed_pairs`: `SymmetryGroup` with two Mirror pairs,
  `keep` true for one → `mirrored_pairs == [that one]`.
- `backend/dp/src/locks.rs` `mirror_partner_is_related_by_mx180`: Mirror sym(0,1) in `hard` → `rel == [R0, Mx180]`;
  `align` on `[R90, R0]` → `[R90, R90.then(Mx180)]`.
- `backend/dp/src/tests.rs` `mirror_pair_keeps_its_reflection_through_rotation`: like the test at `:84` but Mirror mode; seeds
  1..=10: `l.orient[1] == l.orient[0].then(Orient::Mx180)` and at least one seed turned (`l.orient[0] != R0`). Existing
  `:84` (`orient[0] == orient[1]`, Perfect) must stay green.
- Command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk; cargo test -p pnr-core -p analog -p dp -p annotator`
  (check crate names in each `Cargo.toml`), then `cargo test -p library --release` for the flow.
- Acceptance: release `signoff_fixtures`/bench on `ota`, `ota_constrained`, `tt_ota`: no `OrientationSet{Phi}` violation, and
  dr `pairs_exact` count (`backend/dr/src/lib.rs:278`) ≥ the same run with `demote_mirrors(&|_, _| false)`. Report both counts;
  if Mirror fails it, say so (no threshold change).

## PLC-14 Thermal-aware placement: power separation per milliwatt (class: do)

Facts:
- No `heat.rs`, `HeatSeparation` or `heat_source_uw` in the tree.
- Template: `kernel/analog/src/placement/isolation.rs:12-46` `Isolation { a, b, min_distance_nm }` has exactly the plan's
  cost/satisfied/residual/touches (`edge_gap` `kernel/core/src/layout.rs:181`; `rule::over` for residual). A separate type
  keeps its own report row (`Vec<R>::kind`).
- Class: `pnr_core::MatchClass {Minimal, Moderate, Exceptional}` (`kernel/core/src/process.rs:107`); per set in
  `problem.intent.sets[i].class` (`kernel/analog/src/intent.rs:98-117`, `members[].device`). **stale** "victims = PLC-03 orient
  sets": orient sets (`dp::locks`) carry no class; use `intent.sets` mapped to cells through `cells.units.cell_of`
  (`frontend/library/src/lib.rs:2081,2186`).
- Per-cell power: `CellSpace.power` (µW, `lib.rs:2079`). dp refreshes the thermal field per epoch (`dp/lib.rs:376`), so no dp change.
- `Config` at `frontend/library/src/lib.rs:80`, defaults at `:208` (next to `min_utilization: 0.6`).

Edits:
1. New `kernel/analog/src/placement/heat.rs` (+ `pub mod heat; pub use heat::HeatSeparation;` in `mod.rs`):
   ```rust
   /// Hastings rule 14 (L42569–42589): ≥ 1 µm per mW between a power source and a matched set.
   #[derive(Clone, Copy)] pub struct HeatSeparation { pub victim: Target, pub source: Target, pub min_gap_nm: i32 }
   impl Rule for HeatSeparation  // cost ((min−gap)⁺/min)², satisfied gap ≥ min, residual rule::over(min−gap, min), touches both
   /// One rule per (victim cell, source cell) with `power[source] ≥ source_uw`, source ∉ set, class ≠ Minimal;
   /// `min_gap_nm = power_uw` (k = 1 µm/mW = 1 nm/µW for Moderate and Exceptional; Exceptional is policy).
   pub fn separations(sets: &[(MatchClass, Vec<u16>)], power_uw: &[i32], source_uw: i32) -> Vec<HeatSeparation>
   ```
   (dedup victim cells per set; a set merged into one cell is one victim.)
2. `Config.heat_source_uw: i32` (doc: policy, Hastings rule 14 "small power devices"), default `1000`.
3. `topology`, after `CellSpace::new` (next to `live_environment`, `lib.rs:735`): build `sets` from `problem.intent.sets`
   (cells via `cells.units.cell_of`, sorted+dedup), `let heat = heat::separations(&sets, &cells.power, cfg.heat_source_uw)`;
   if non-empty push `Box::new(heat.clone())` to `problem.placement.budget` and `Box::new(heat)` to `.cost`.

Tests:
- `heat.rs` `heat_separation_scales_with_power`: power `[0, 0, 2000]`, set `(Moderate, [0, 1])`, `source_uw 1000` → 2 rules,
  each `min_gap_nm == 2000`, source cell 2; `(Minimal, ..)` → empty; power 999 → empty; source inside the set → empty.
- `heat.rs` `satisfied_iff_gap_reaches_min`: edge gap 1999 → `!satisfied`, residual > 0; 2000 → satisfied, residual 0.
- `backend/dp/src/tests.rs` `dp_pushes_a_matched_pair_away_from_a_hot_cell`: 3 cells (pair 0,1 + hot 2) seeded adjacent,
  `HeatSeparation` 0→2 and 1→2 (`min_gap_nm 2000`) in budget and cost; seeds 1..=10: both residuals `== 0.0` at exit.
- Command: `cargo test -p analog heat`, `cargo test -p dp dp_pushes_a_matched_pair_away_from_a_hot_cell`.
- Acceptance (release, seeds 1–5, `Config::op` fixtures per `frontend/library/tests/perf_postlayout.rs`): list fixtures with a
  cell ≥ 1000 µW; on those all `HeatSeparation` residuals 0, mean matched-pair `mu_thermal` ≤ the run with
  `heat_source_uw = i32::MAX`, area ≤ +5 %. Fixtures without such a cell must show zero `HeatSeparation` rows. Report numbers;
  if none of the fixtures reaches 1000 µW, say so (the unit tests carry the behaviour).
