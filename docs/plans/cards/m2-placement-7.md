# M2+ placement, segment 7 (M2): PLC-07, PLC-13, PLC-24, PLC-29

Worktree `philis-m2/placement`, branch `m2-placement`. `git merge m2` fast-forwarded to `0f84944`, so there were no
conflicts. DAG entries: PLC-07 has no hard deps. PLC-13 needs PLC-07, GAP-01 and MAT-07, with EXT-16 at step level
(class defaults to Moderate). PLC-24 needs PLC-07. PLC-29 needs GAP-01 and carries its step 5. All hard deps are on
`m2`: PLC-07 is `3c12878`, GAP-01 is `3650948`, and MAT-07 is `c068dff`/`8cfd016`. PLC-06 is not done (it waits on
the owner), but none of these four items reads anything PLC-06 left open.

## PLC-07 Per-pair spacing from cell edge profiles (class: do — acceptance measurement only)

### Current code facts
- The code landed in `3c12878`; card 1 is `m2-placement-1.md`. `backend/gp/src/spacing.rs` exists, but it differs
  from the plan in these ways:
  - `ROLES` has 28 entries, not 20 (`:10`). `MARKER` adds pnp and npn (`:35`).
  - `Edge` carries a `present` bitmask (`:63`).
  - `SpacingTable::halo` (`:224`) does the region sizing. `SpacingTable::uniform` (`:175`) and `max_gap` (`:181`)
    were also added.
- `gp::PlaceRules { grid, spacing, profiles, far }` is at `gp/lib.rs:239-262`, `gaps` at `:273`, and `encroach`
  after it.
- `place_rules(pdk, cells)` is at `frontend/library/src/lib.rs:1229`. It is built once per topology (`:740`).
- The sidecar reader `placement_space` is at `lib.rs:1268+`.
- The tests are `spacing.rs:339-396` (the plan's 7), plus `start_tests::shipped_cells_draw_no_unmapped_layer`
  (`lib.rs:3087`) and `table_gaps_are_drc_clean_on_sky130` (`lib.rs:3163`). They live in `lib.rs`, not in
  `tests/placement_spacing.rs` as the plan says.
- **Missing:** the acceptance. `docs/plans/m2-placement-report.md` holds only PLC-29's table. Nothing records T2 or
  signoff DRC/LVS before and after PLC-07.

### Edits
None to product code. Add a section `## PLC-07 acceptance` to `docs/plans/m2-placement-report.md`.

### Tests / measurement
1. Re-run the existing tests on the merged tree:
   - `cargo test -p gp spacing`
   - `cargo test --release -p library shipped_cells_draw_no_unmapped_layer table_gaps_are_drc_clean_on_sky130 -- --nocapture`
2. Bench, same protocol as the PLC-29 table:
   - Trees: `git archive` the base `c235de8` (the parent of `3c12878`) and HEAD.
   - Probe (uncommitted): `library::run` per fixture × seeds 1–5, release, sky130, `feedback_iters = 5`.
   - Record the T2 median per fixture, where T2 = `footprint_nm2 / Σ cell bbox area` from `PlaceStats`. Also record
     the signoff DRC and LVS counts.
3. Pass: T2 median ≥ 20 % better than base, and DRC/LVS no worse on any fixture.
   - If it falls short, report the number with the per-fixture rows. Do not tune.
   - HEAD also includes PLC-29, PLC-18 and PLC-17. Note this in the report, or base at `3c12878`/`c235de8` exactly to
     isolate PLC-07 (preferred).

## PLC-13 WPE and extraneous-poly keep-outs as pairwise spacing (class: do)

### Current code facts (plan corrections)
- `MatchClass` lives at `pnr_core::MatchClass` (`kernel/core/src/process.rs:107`): `Minimal=0 | Moderate=1 |
  Exceptional=2`. It derives `Copy, Eq, Ord, Default=Moderate`. The plan's `analog::matching::class::MatchClass` path
  is stale. gp already depends on pnr_core, so gp needs no analog dependency.
- `mos_env(c, p).wpe_nm` is at `kernel/analog/src/matching/class.rs:63`. It reads the `wpe_clearance_nm` tier, which
  is `[2000, 3000, 5000]` (`pdks/sky130.json:17`, not `:59-63`).
- No `foreign_poly` value exists anywhere.
- Today `Profile` is `{ edge: [Edge; 4], well_net }` (`spacing.rs:84-89`). `oriented` (`:281`) copies only
  `well_net`.
- Orient sets come from `dp::locks::locks(...).orient_of: Vec<Option<u16>>` (`backend/dp/src/locks.rs:15`). `locks`
  is computed at `lib.rs:739`, before `place_rules` at `:740`.
- The class is real, not constant: annotator intent sets carry `MatchClass` (`backend/annotator/src/lib.rs:309-320`).
  `MatchedSet.class` (`kernel/analog/src/placement/matched_set.rs:34`) holds it. `RuleBatch` exposes
  `matched_pairs` (`kernel/analog/src/rule.rs:272`) but no class.
- The "matched cell" test already exists at `lib.rs:867`: `devices_of[c].len() > 1 || any device in a 2-device
  DiffPair/CurrentMirror/Load/CascodePair leaf`.
- WPE scope correction: apply the WPE keep-out to `diff_out` only (NMOS, diff outside the cell's own well), not to
  `diff_in`.
  - A matched PMOS's relevant well edge is its own well, which CELL-12 inflates.
  - A foreign same-bulk well that abuts it merges into that well and moves its edge outward.
  - Applying WPE to `diff_in` would forbid that legal merge for no WPE gain.
  - The foreign-poly keep-out applies to both `diff_in` and `diff_out`.

### Edits
1. `kernel/analog/src/rule.rs`, in trait `RuleBatch`, next to `matched_pairs`:
   `fn matched_class(&self) -> Option<pnr_core::MatchClass> { None }`. Doc: "The class of the pairs from
   [`Self::matched_pairs`], if the batch has one."
   - `MatchedSet` (`matched_set.rs`, its `impl RuleBatch`) overrides it with `Some(self.class)`.
   - This is a cross-module one-liner in analog-matching's file. Tell that owner.
2. `backend/gp/src/spacing.rs`:
   - `Profile` gains `pub matched: Option<pnr_core::MatchClass>` and `pub set: Option<u16>`. Both are still
     `Copy + Eq + Default`.
   - `oriented` copies both, by changing `Profile { well_net: p.well_net, ..Default }` to `Profile { edge: [..], ..*p }`.
   - Add `pub const FOREIGN_POLY_NM: [i32; 3] = [0, 3000, 5000];`. Its doc is Hastings rule 23, lower ends, H13-55,
     L42648–42654, read from the PDF. It is **policy** and the same on every deck.
   - `SpacingTable` gains `pub wpe: [i32; 3]` and `pub foreign_poly: [i32; 3]`. Both are zero in `new` and `uniform`,
     so behaviour without matched profiles is unchanged.
   - `gap(a, fa, b)`: after the existing loop, call `keep(a, ea, b, eb)` and `keep(b, eb, a, ea)`, where
     ```rust
     // m matched (class c), o not in m's orient set: o's nwell ≥ wpe[c] from m's diff_out,
     // o's poly ≥ foreign_poly[c] from m's diff_in|diff_out. Hard (never mergeable).
     let exempt = m.set.is_some() && m.set == o.set;
     ```
     - Each `need = k − em.inset[d] − eo.inset[r]`, over present roles with `k > 0`, raises both `g_all` and `g_hard`.
     - Write it as a small closure over the bitmask, reusing the `present` / `inset` reads.
     - Doc the `gap` addition in one line.
3. `frontend/library/src/lib.rs`:
   - `place_rules(pdk, cells, locks: &dp::locks::Locks, class: &[Option<MatchClass>])`.
     - Set `table.wpe = [0, 1, 2].map(|i| mos_env(CLASS[i], pdk).wpe_nm)` and
       `table.foreign_poly = gp::spacing::FOREIGN_POLY_NM`.
     - In the profile map, set `p.matched = class[c]` and `p.set = locks.orient_of.get(c).copied().flatten()` before
       `Profiles::orients`.
   - New `fn match_class(problem: &Problem, cells: &CellSpace) -> Vec<Option<MatchClass>>`.
     - Start from the `:867` matched test, which gives `Some(Moderate)` per cell. Extract that test into a helper
       shared by `:867` and this function.
     - Then, for every `problem.placement` hard/budget/cost batch with `Some(k) = b.matched_class()`, set each cell in
       its `matched_pairs` to `max(current, k)`.
     - Call it at `:740`.
   - Update both test call sites (`:3170`, `:3181`). They pass `&Locks::default()` and `&[]`, so they stay
     unmatched and their gaps are unchanged.

### Tests (`backend/gp/src/spacing.rs` tests module, reuse `table`/`face` helpers; `wpe = [2000, 3000, 5000]`, rule[diff_out][nwell] = 340)
- `matched_nmos_keeps_wpe_distance_from_foreign_well`: a has `diff_out` inset 200 on R, `matched = Some(Moderate)`,
  `set = None`. b has `nwell` inset 0 on L. Assert `gap(a, R, b) == Gap { abut: false, min: 2800 }`, and
  `gap(b, L, a)` gives the same result (symmetric).
- `unmatched_nmos_uses_the_deck_rule`: same as above with `matched = None`. Assert `min == 140`.
- `same_set_partners_are_exempt`: both have `set = Some(0)` and a is matched. Assert `min == 140`.
- `matched_diff_keeps_foreign_poly_away`: a has `diff_in` inset 100, Exceptional. b has `poly` inset 0. Assert
  `min == 4900`.
- Command: `cargo test -p gp spacing`. Then run
  `cargo test --release -p library table_gaps_are_drc_clean_on_sky130 shipped_cells` (it must stay green) and
  `cargo test -p analog matched_set`.

### Acceptance
- T9 nearness = 0 on every fixture × seeds 1–5. Read it from the final report's routing `Environment` row, as in the
  PLC-29 table.
- T2 within +5 % of HEAD before this item, using the same bench as the PLC-29 table.
- If T2 grows by more than 5 %, report the per-fixture numbers. Do not change `wpe`.

## PLC-24 A reachable utilization floor (class: do)

### Current code facts
- `Config::min_utilization` is at `lib.rs:107`, with default `0.6` at `:208`.
- It is pushed before cells exist at `lib.rs:720-722`, inside `topology()`.
- `CellSpace::new` is at `:733`, and `rules = place_rules(...)` is at `:740` (the plan's `:76`, `:264`, `:275` are
  stale).
- `Utilization { u_min }` is in `kernel/analog/src/placement/utilization.rs:13-15` and has no ids, so moving the push
  past the retarget is safe.

### Edits (`frontend/library/src/lib.rs`)
1. Delete the push at `:720-722`. Re-add it right after `let rules = place_rules(...)`:
   ```rust
   if cfg.min_utilization > 0.0 {
       let dims: Vec<(i32, i32)> = cells.variants.iter().map(|v| v.alternatives.first().map_or((0, 0), |m| (m.bbox.w, m.bbox.h))).collect();
       let u_min = u_eff(cfg.min_utilization, &dims, median_x_gap(&rules));
       problem.placement.budget.push(Box::new(analog::placement::utilization::Utilization { u_min }));
   }
   ```
2. `fn u_eff(u_min: f32, cells: &[(i32, i32)], g: i32) -> f32`:
   - Compute `min(u_min, 0.9 · ΣA / Σ(w+g)(h+g))` in f64.
   - If there are no cells, or the sum is 0, return `u_min`.
   - The 0.9 is **policy** (room for routing halos). Doc it.
3. `fn median_x_gap(r: &gp::PlaceRules) -> i32`:
   - Take the median over ordered pairs a ≠ b of `r.spacing.gap(&of[a][0][0], Face::R, &of[b][0][0]).min`, with
     `of = &r.profiles.of`.
   - With fewer than 2 cells, or empty profiles, return `r.spacing.fallback`.
   - Use `select_nth_unstable`. The cost is O(n²) once per topology, about 10⁴ for 100 cells.
   - The pushed index of the Utilization budget moves later. Nothing indexes budgets by position before `solve`, so
     confirm the existing flow tests stay green.

### Tests (`lib.rs` `start_tests`)
- `utilization_floor_is_reachable_for_tiny_cells`, with four `(2000, 2000)` cells:
  - `(u_eff(0.6, &c, 270) - 0.6).abs() < 1e-6`. The cap is 0.6986, so 0.6 binds.
  - `(u_eff(0.6, &c, 1270) - 0.3367).abs() < 1e-3`, from `0.9·16e6/(4·3270²)`.
  - `u_eff(0.6, &[], 0) == 0.6`.
- Command: `cargo test -p library utilization_floor`. Then run `cargo test -p library start_tests` in the background.

### Acceptance
- Per fixture, seed 1, release: run the bench with the default floor and again with `min_utilization = 0`.
- Where the final `Utilization` residual is > 0, the `= 0` footprint must exceed `Σ A / u_eff`. Otherwise the floor
  alone is binding, which counts as a failure.
- Record the result in `m2-placement-report.md`.

## PLC-29 Matched-pair environment evaluated live (class: do — done; verify only)

### Current code facts
- Landed in `913b52a` (review fixes `e948120`).
- Code locations:
  - `LiveEnvironment` / `EnvGeo`: `kernel/analog/src/placement/environment.rs:92-186`.
  - `place_rect`: `kernel/core/src/macro.rs`.
  - Builder `live_environment`: `lib.rs:2039`, pushed at `:735-738`.
- GAP-01 step 5 (C20) is already carried: `wpe_min_nm = mos_env(Moderate).wpe_nm` (`lib.rs:2045`).
- The acceptance is recorded in `docs/plans/m2-placement-report.md` §PLC-29: T9 = 0 on every fixture × seed.
- Not done, and correctly so: the per-pair class. EXT-16 / step-level work keeps Moderate.

### Edits
None.

### Tests
On the merged tree, run `cargo test -p analog environment` and `cargo test -p library environment_tests`. Both must
pass. If one fails, fix the regression the merge introduced. Do not change the test.
