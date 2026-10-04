# M2+ analog-matching, segment 4 (M6): implementation card (GAP-18)

Base: `m2-analog-matching` after merging `m2` (beda4a6). Line numbers are this base's; the card wins over
`98-gap-critic.md` §GAP-18. DAG: module analog-matching, M6, hard dep MAT-11 (done, segment 2), no step-level notes.

## GAP-18 Capacitor-array variant choice by metric — class: do (spec class source waits on EXT-16)

Facts (spec is stale on three points):

- `ArrayMetrics` `kernel/cells/src/cap_array.rs:64-91` already carries MAT-11's `msys`, `inl_lsb`, `area_um2`;
  `CapArray::metrics(&self, group, c, process, g)` is **:238-274** (not :357), callers only tests (:708, :885).
- **`Unitization` has no `class` field** (`kernel/analog/src/cell.rs:49-63`). `MatchClass` lives on
  `placement::MatchedSet.class` (`matched_set.rs:34`) only; nothing in the flow produces `Exceptional` (no EXT-16
  spec-to-class assignment, no user override path in `intent`). An annotated `Unitization` reaches
  `CapArray::enumerate` unchanged: `cellgen::with_per_device_sizing` clones it (`cellgen.rs:468-474`).
- `CapArray::enumerate` (`cap_array.rs:95-131`): binary bank (`bits(&dev_nf)` Some) → Spiral, Chessboard,
  BlockChessboard{k,bs}, × tall for odd N, deduped by assignment; general set → `[Spiral, Chessboard]` early return.
- **Reordering alone does not make the odometer try the best variant first.** `cellgen::seed_assignment`
  (`cellgen.rs:326-337`) seeds each cell at `min (DRC+ERC, pin HPWL)` (ties → index); `keep` (:347) keeps all
  alternatives of a matched cell; `escalate` (:412-431) moves a digit from the seed to the next higher allowed index,
  then carries to `allowed[0]`. So a ranked cell must also seed at its lowest-index DRC-minimal alternative.
  Caller: `lib.rs:750-751` (`matched` from `flow.cells.devices_of`).
- `dac4` (`benchmarks/fixtures/dac4.spice`): XC0..XC4 `cap_generic_m1m2` W=L=2u, m = 1,1,2,4,8 on plate `top` — a
  4-bit bank found by `cellgen::dac_banks` (:807) when no annotated unitization covers it. "Reports its chosen
  variant's M_sys": no report row exists (MAT-11 step 4 / PERF-20 own the bench row); the acceptance is a test.

Edits:

1. `kernel/analog/src/cell.rs` `Unitization`: add
   ```rust
   /// Match class of the set (EXT-16 from a spec, or the user); `None` = not given (Moderate behaviour).
   /// Read by `cells::cap_array` (GAP-18): an Exceptional binary bank lists its variants best-matching first.
   pub class: Option<pnr_core::MatchClass>,
   ```
   Add `class: None,` to every struct literal without `..`: `frontend/library/tests/drawn_cards.rs:27`,
   `examples/three_stage_opamp/src/main.rs:96`, `frontend/library/src/cellgen.rs:481,509,530,551,575,1427`,
   `kernel/cells/tests/deck_keys.rs:76`, `backend/annotator/src/constraints.rs:51`, `kernel/cells/src/lib.rs:73`,
   `kernel/cells/tests/cell_selfcheck.rs:45,416`, `kernel/cells/src/capacitor.rs:386`, `kernel/cells/src/cap_array.rs:607`,
   `kernel/macroMaster/src/adapter.rs:29` (`cellgen.rs:120` uses `..u.clone()`, keeps the class). Re-grep
   `Unitization {` after the merge; `cargo build --workspace --all-targets` finds any missed site.
2. `kernel/cells/src/cap_array.rs`:
   ```rust
   /// t0/t gradient at which an Exceptional bank ranks its variants, 1/µm: DACP's γ = 100 ppm read per µm (GAP-18).
   pub const RANK_G_PER_UM: f64 = 1e-4;
   ```
   In `enumerate`, binary branch only (the general branch's INL has no binary code ladder): after `out` is built,
   if `crate::builder::unitization(group, constraints).and_then(|u| u.class) == Some(MatchClass::Exceptional)`,
   stable-sort the variants by `(msys, inl_lsb, area_um2)` from `v.metrics(group, constraints, process,
   RANK_G_PER_UM)`, compared with `f64::total_cmp` lexicographically (compute the keys once, `sort_by_cached_key`
   is unavailable for f64: zip keys and sort pairs). No spec INL limit is assumed. Doc line on `enumerate`: "An
   Exceptional binary bank lists its variants by (M_sys, INL, area) at `RANK_G_PER_UM` (GAP-18, CC-24)". Cost: one
   extra `build` per variant, Exceptional only.
3. `frontend/library/src/cellgen.rs` (the one-line hook, plus its helper):
   ```rust
   /// Seed index from `(DRC+ERC, HPWL)` prices: the cheapest, ties → index; a `ranked` cell (its generator lists
   /// alternatives best-first, GAP-18) takes its lowest index among the DRC+ERC-minimal ones, HPWL ignored.
   fn seed_of(prices: &[(usize, i64)], ranked: bool) -> usize;
   pub fn seed_assignment(variants: &[gp::VariantSpace], matched: &[bool], ranked: &[bool], pdk: &Pdk) -> (Vec<u16>, Vec<Vec<u16>>);
   ```
   `seed_assignment` calls `seed_of(&prices, ranked.get(i).copied().unwrap_or(false))`; `keep` unchanged (a ranked
   cell is a merged group, so `matched` keeps all and `escalate` then walks them in rank order from 0). Update the
   doc of `seed_assignment` and `escalate` ("ascending = best-matching first for a ranked cell").
4. `frontend/library/src/lib.rs:750-751`: next to `matched`,
   `let ranked: Vec<bool> = flow.cells.devices_of.iter().map(|m| flow.problem.constraints.unitization.iter().any(|u| u.class == Some(pnr_core::MatchClass::Exceptional) && m.iter().all(|d| u.devices.contains(d)))).collect();`
   (use the field names at that site; `problem.constraints` is what `enumerate_folded` received at :1815), pass
   `&ranked`. Update any other `seed_assignment` caller (cellgen tests).

Tests:

- `kernel/cells/src/cap_array.rs::tests::exceptional_banks_try_the_lowest_msys_first` (no PDK, the `Flat` process of
  `metrics_rank_the_families` — hoist it to a module-level test helper): `(g, mut c) = bank(n)`;
  `plain = CapArray::enumerate(&g, &c, &Flat)`; set `c.unitization[0].class = Some(Exceptional)`;
  `ranked = CapArray::enumerate(&g, &c, &Flat)`; key `k(v) = (msys, inl_lsb, area_um2)` at `RANK_G_PER_UM`. Assert:
  same length and every `plain` variant appears in `ranked` (compare `(pattern, tall)`); `ranked` keys nondecreasing
  lexicographically; `k(ranked[0]).msys <= k(v).msys` for every `v`; and **`plain` keys are not nondecreasing**
  (the test is vacuous otherwise: pick `n` among 4..=6 where this holds and say so in the doc line).
  Also `class = Some(Moderate)` yields exactly `plain`'s order.
- `frontend/library/src/cellgen.rs::tests::ranked_cells_seed_at_their_first_clean_alternative` (pure):
  `seed_of(&[(1, 5), (0, 90), (0, 10)], false) == 2`, `seed_of(&[(1, 5), (0, 90), (0, 10)], true) == 1`,
  `seed_of(&[(0, 9), (0, 1)], true) == 0`.
- Acceptance `frontend/library/src/cellgen.rs::tests::exceptional_dac4_seeds_the_lowest_msys` (sky130): parse
  `benchmarks/fixtures/dac4.spice` (`crate::parse`, `crate::deck_models(&mut nl, &pdk)`, as `lib.rs:2422-2424`);
  `c` = one `Unitization` over XC0..XC4 (ids by name) with `dev_nf`/`target_ratio` `[1,1,2,4,8]`, the `unit_w`/
  `unit_l`/`series_parallel`/flags `with_per_device_sizing` gives a dac bank (:509ff), `class: Some(Exceptional)`.
  `cells = enumerate(&nl, &Macros::default(), &c, &pdk, false)`; `ci = cells.cell_of[XC0]`. Re-derive the variants
  as `draw_variants` does (recipe `Overlay` for `cap_generic_m1m2`, `CapArray::enumerate(&group, &c, p)`), `ms[k]` =
  `metrics(..., RANK_G_PER_UM).msys`. Assert: `ms.iter().any(|&m| m > ms[0])` (non-vacuous), `ms[0] <= ms[k]` ∀k,
  `cells.spaces[ci].alternatives[0].shapes == variants[0].draw(&group, &c, p).shapes`, and
  `seed_assignment(&cells.spaces, &vec![true; n], &ranked, &pdk).0[ci] == 0` with `ranked[ci] = true`.
- Commands (`export PDK_ROOT=...`): `cargo test -p cells --lib cap_array`, `cargo test -p library --lib cellgen`,
  `cargo build --workspace --all-targets` (the new field), then `cargo test -p library` for regressions.

Not done here: no producer sets `Exceptional` until EXT-16 (spec → class) or a user override lands; until then every
flow run has `class: None` and behaves exactly as today. Wiring EXT-16's class into `Unitization.class` (annotator
`constraints.rs:51`) is EXT-16's step, recorded for its owner.
