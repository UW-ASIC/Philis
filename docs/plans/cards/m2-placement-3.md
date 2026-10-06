# M2+ placement, segment 3 (M3): PLC-18

Worktree `philis-m2/placement`, branch `m2-placement`, after merging `m2` (`3230772`; the one conflict in
`frontend/library/src/lib.rs` kept both sides: `self.rules.spacing.fallback` from placement plus `lap(5)` from m2, and
m2's `round_up` doc line). DAG entry: module placement, M3, no hard deps, no step-level notes. Item not blocked by
PLC-06.

## PLC-18 Dimensionless placement objective (class: do)

### Current code facts (plan line refs are stale; corrected here)

- PEX = `hpwl(nm) + analog_cost`: `backend/dp/src/lib.rs:154-157` (`Sa::pex`, plan said `:120-122`);
  `analog_cost` is at `backend/gp/src/mechanics.rs:244-257` (`Σ criticality·cost` over `reqs.cost` + `Σ price·residual`
  over `reqs.budget`); gp's stage report `cost = hpwl + analog_cost` at `mechanics.rs:320`.
- T0 probe: `dp/lib.rs:303-316` (plan said `:276-289`). `temp = (sum/128).max(1.0)·t0_scale`: the `.max(1.0)` floor
  is in PEX units, so it would bite once PEX is O(1) and break scale invariance. It is removed below.
- gp sums three gradients with no common unit (`backend/gp/src/lib.rs:380-448`): (a) HPWL subgradient `w` per pin
  (nm/nm), (b) analog cost by central difference, probe 64 nm (`:400-418`), (c) density push `λ·over`. The step is
  normalised by `gmax` (`:453-454`), so only the ratio of (a), (b), (c) matters.
- Cost bodies (all `kernel/analog/src/placement/`):
  - `proximity.rs:31-34` `ex²·1e-3`, `ex = gap − max_distance_nm`; residual `over(gap − m, m)` (`:38-41`) stays.
  - `isolation.rs:23-26` `shortfall²·4e-3`.
  - `dti.rs:42-49` linear nm miss to the committed side; `residual` (`:62-63`) is `over(self.cost(l), band)`, so it
    reads `cost` and must be decoupled before `cost` changes. Band = `d_dti_nm − s_max_nm`.
  - `symmetry.rs:29-33` `(ex² + ey²)·1e-3`.
  - `matched_set.rs:256-264` `1e-3·(Δm² + so²) + 3e5·(μ_thermal/allowance)²`. Plan says "MAT converts its own rules"
    (`MatchingPair`, `CentroidGroup`, `ThermalGradient`), but MAT-04/05 deleted those and folded them into
    `MatchedSet`, whose doc and the M1 matching card (`cards/m1-matching.md:321`) say the legacy scales are
    "kept until PLC-18 normalises costs"; no MAT item in M2-M6 owns it (dag-m2-m6.md). **PLC-18 converts it**
    (one closure in a matching-owned file; small merge surface).
  - Already dimensionless, unchanged: `Utilization` (fraction), `OrientationSet` (Φ residual), `Environment`
    (usage), priced budget residuals.
  - `HeatSeparation`, `SymmetryIsland` do not exist (grep: none). The items that add them (PLC-09/11) must use the
    convention below; nothing to do here.
  - Routing-tier costs (`routing/parasitic.rs` `len²·1e-6`, `antenna.rs` ×100) are not in the placement objective:
    out of scope.
- `Layout` (`kernel/core/src/layout.rs:11-41`) has `footprint_nm2` (`:44-57`) but no cell-area sum.

### Edits

1. `kernel/core/src/layout.rs`, after `footprint_nm2`:
   ```rust
   /// Normalising length, nm: `sqrt(Σ cell area)` over every cell's current
   /// `hw`/`hh` (PLC-18); `1` with no cells, so callers never divide by 0.
   #[must_use]
   pub fn l_ref(&self) -> f32 {
       let a: f64 = self.hw.iter().zip(&self.hh).map(|(&w, &h)| 4.0 * f64::from(w) * f64::from(h)).sum();
       a.sqrt().max(1.0) as f32
   }
   ```
   `// ponytail: O(n) per call, read once per rule eval; cache on Layout if PLC-09's T7 profile shows it.`
   Live (not cached) so dp's reshape moves stay exact.
2. `proximity.rs::cost`: `let m = self.max_distance_nm.max(1) as f32; let e = (self.gap(l) - m).max(0.0) / m; e * e`.
   Doc: `(excess / max_distance)²`.
3. `isolation.rs::cost`: `let m = self.min_distance_nm.max(1) as f32; let s = (m - l.edge_gap(self.a, self.b)).max(0.0) / m; s * s`.
   Doc: `(shortfall / min_distance)²`.
4. `dti.rs`: move the current `cost` body into `fn miss(self, l: &Layout) -> f32` (inherent, private, doc "nm to
   the committed side's interval"); `cost` = `(miss / band)²` with `band = (d_dti_nm − s_max_nm).max(1) as f32`;
   `residual` = `over(self.miss(l), band)` (unchanged values). Re-read the comment at `dp/lib.rs:~575` that names
   `DtiBand::cost` and keep it accurate.
5. `symmetry.rs::cost`: `let (ex, ey) = self.error(l); let e = (ex.abs() + ey.abs()) as f32 / l.l_ref(); e * e`.
   Doc: `((|ex| + |ey|) / L_ref)²`.
6. `matched_set.rs::cost` (`:256-264`): `let r2 = l.l_ref().powi(2);` then per pair
   `(g.delta_m_nm² + g.second_order_nm²) / r2 + if g.allowance > 0 { (g.mu_thermal / g.allowance)² } else { 0 }`.
   Update the struct/impl doc that says "kept until PLC-18".
7. `backend/gp/src/mechanics.rs`: add
   ```rust
   /// PEX-tier energy, dimensionless (PLC-18): `HPWL / L_ref + analog_cost`.
   #[must_use]
   pub fn pex(nets: &Nets, reqs: &Requirements<Layout>, l: &Layout, prices: &Prices) -> f64 {
       hpwl(nets, l) / f64::from(l.l_ref()) + f64::from(analog_cost(reqs, l, prices))
   }
   ```
   `report` (`:320`) uses `pex(..) as f32`; fix its doc ("cost = HPWL/L_ref + analog cost").
8. `dp/lib.rs`: `Sa::pex` returns `pex(&self.nets, self.reqs, l, self.prices)` (import `pex`). T0 (`:316`):
   `let mean = sum / 128.0; let mut temp = if mean > 0.0 { mean } else { 1.0 } * schedule.t0_scale;`
   (the fallback only keeps today's behaviour for a layout where no probe changes PEX).
9. `gp/lib.rs`: gp minimises `L_ref·E`, a constant multiple of E (hw/hh are fixed inside gp), so (a) and (c) keep
   their present balance: compute `let l_ref = l.l_ref();` once after `initial_layout`, and in (b) use
   `let inv = l_ref / (2.0 * ANALOG_PROBE as f32);`. Comment: one line saying why.

### Tests

- `backend/dp/src/tests.rs::energy_is_invariant_to_scaling_all_lengths`: helper `fn scene(k: i32) -> (Nets, Layout, Requirements<Layout>)`:
  two cells `layout(&[(0, 0, 5_000·k, 5_000·k), (40_000·k, 3_000·k, 5_000·k, 5_000·k)])`, `axis = vec![10_000·k; 2]`,
  nets from two `Macro`s like `pin_alt` with `bbox`/pin rect scaled by k (net 0 on both); `reqs.cost` = one batch each
  of `vec![Proximity { a: d0, b: d1, max_distance_nm: 10_000·k }]`, `vec![Isolation { .., min_distance_nm: 60_000·k }]`,
  `vec![DtiBand { s_max_nm: 200·k, d_dti_nm: 50_000·k, branch: BranchId(0), seed_isolate: true, .. }]` with
  `l.branch = vec![true]`, `SymmetryGroup`/`vec![Symmetry { a: d0, b: d1, axis: AxisId(0) }]` (whatever wrapper
  `reqs.cost` accepts; reuse the file's existing constructors). Assert, for k = 1: every batch's `cost(&l) > 0.0`
  (each term is live, so a forgotten unit scale fails the test), and `hpwl > 0`. Then with `Sa::new(nets, 2, &reqs,
  &gp::Prices::new(), &[], &rules(0))`: `let (e1, e2) = (sa1.pex(&l1), sa2.pex(&l2));`
  `assert!((e2 - e1).abs() <= 1e-6 * e1.abs(), "{e1} vs {e2}")`. Breaks if any converted cost keeps nm units or HPWL
  loses `/ L_ref`.
- `kernel/core/src/layout.rs` (existing test module, or new `#[cfg(test)] mod tests`): `l_ref_is_root_of_cell_area`:
  cells hw/hh (3, 4) and (6, 2) → `l_ref() == (4·12 + 4·12) as f32` sqrt `= 96f32.sqrt()`; empty layout → `1.0`.
- Update value assertions whose expected numbers the spec changes (not loosened, recomputed):
  `dti.rs:122-123` 900 → `0.25` (`(900/1800)²`, tolerance `1e-6`); `dti.rs:198` 900 → `0.25`;
  `matched_set.rs:624` `heat / (3e5·mu·mu)` → `heat / (mu·mu)` (allowance 1.0), comment `:619` likewise. Ordering
  assertions (`dti.rs:130-137`, `matched_set.rs:563-566`) stay as they are.
- Commands: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk;`
  `cargo test -p pnr_core -p analog -p gp -p dp` (crate names as in each Cargo.toml), then
  `cargo test -p library --release --test placement_metrics --test flow_smoke`.

### Acceptance (policy, measured once, not committed as code)

Temporary patch in the T0 loop (`dp/lib.rs:303-316`): per probe, accumulate `|Δ|` separately for `hpwl/L_ref`, each
`reqs.cost` batch by `kind()`, and the priced budget sum; `eprintln!` the shares. Run
`cargo test -p library --release --test placement_metrics metrics_are_populated_on_ota -- --nocapture`. Pass iff no
term > 80 % of `mean |ΔE|` (first dp call of the run, i.e. the cold schedule). Record the table (and the before/after
lex key for `ota`, `dac4`, `rc_filter` at seed 1, informational) in `docs/CRATES.md` under `## backend/dp`; revert
the patch. If some term exceeds 80 %, report it with the numbers; do not add per-term weights to force it (out of
scope, PLC-09 owns the energy weights).
