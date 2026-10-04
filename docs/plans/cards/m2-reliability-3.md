# M2+ reliability, segment 3 (M6): REL-16, REL-12, REL-13, REL-15, REL-17, GAP-13

Worktree `philis-m2/reliability`, branch `m2-reliability`, merged with `m2` at `7073e38` (fast-forward). Line numbers
are from that commit. All six are **do**. Run `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first.

Stale plan facts fixed here:
- `well_bridges` is at `kernel/cells/src/post_cell.rs:468`, not `:387`. Its library call sites are
  `frontend/library/src/lib.rs:1276` (`Flow::epoch`) and `:2903` (test `drawn`), not `:609`.
- `elaborate::stack` is at `frontend/library/src/elaborate.rs:281-305`, not `:275-287`.
- sky130 `pex met1 thickness 360nm` is at `pdks/decks/sky130.deck:639`, not `GP/pdks/sky130.deck:590`. `rbody_po` is
  `:650` (height 326.2 nm). `rbody_high_po` (`:653`, 317.39 Ω/□) and `rbody_xhigh_po` (`:656`, 2000 Ω/□) now carry a
  sheet R, so REL-13's note that high/xhigh have none (FLOW-05 OQ-3) is stale.
- `Isolation::cost` (`kernel/analog/src/placement/isolation.rs:23-27`) is dimensionless since PLC-18:
  `(shortfall/min)²`. REL-15's `4e-3·Δd²` in nm² is not "the same scale", so the card normalises it (below).
- EXT-23 merged without a `substrate::isolation`. Emission is still `emit::isolation` (`backend/annotator/src/emit.rs:256-288`),
  called at `backend/annotator/src/lib.rs:399`.
- EXT-23 tags exist. They are `intent.aggressors: Vec<Aggressor { device, inject: Inject }>` and `intent.victims`
  (`kernel/analog/src/intent.rs:229-251, 292-293`). `Inject::{MinorityElectron, MinorityHole}` is the minority-injector
  tag. `substrate::victim_mask` (`backend/annotator/src/substrate.rs:145-151`) already names REL-16 as a reader.

---

## REL-16 Shared-well merge legality — do

Facts: `well_bridges(placed, rings, process)` (`post_cell.rs:468-532`) bridges any facing same-bulk n-wells. No
`CellFlags` or `may_share_well` exists. Only the predicate is REL-16's. CELL-22 owns the `well_bridges` signature
change and the library wiring.

Edit `kernel/cells/src/post_cell.rs`. Put this above `well_bridges`:
```rust
/// Substrate tags of one placed cell, OR over its members (EXT-23): `injector` = a
/// `MinorityElectron`/`MinorityHole` aggressor, `noisy` = any aggressor, `sensitive` = a victim.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellFlags { pub injector: bool, pub noisy: bool, pub sensitive: bool }

/// Hastings §14.1.5 (L43595–43612): two cells may share a well only if both inject or neither does,
/// and neither is noisy beside a sensitive one. §14.1.4: output (injecting) devices get their own.
#[must_use]
pub fn may_share_well(a: CellFlags, b: CellFlags) -> bool {
    a.injector == b.injector && !((a.noisy && b.sensitive) || (b.noisy && a.sensitive))
}
```
Test: `post_cell.rs` `may_share_well_table`. Let `f(i,n,s)` be `CellFlags{injector:i,noisy:n,sensitive:s}`. Assert:
- `!may(f(1,0,0), f(0,0,0))`
- `!may(f(0,1,0), f(0,0,1))` and the mirrored order
- `may(f(0,1,0), f(0,1,0))`
- `may(f(1,0,0), f(1,0,0))`
- `may(f(0,0,0), f(0,0,0))`

Command: `cargo test -p cells may_share_well`. Acceptance (0 bridges joining an injector cell to a non-injector) is
measured by CELL-22's `a_forbidden_pair_is_not_bridged` once CELL-22 wires the predicate. REL-16 reports it as
CELL-22's.

## REL-12 Effective via cuts by current direction — do

Facts: `Electromigration { net, limits, stack }` (`kernel/analog/src/routing/em.rs:101-108`). In `walk`
(`em.rs:145-203`), cuts union into groups when they share a landing shape below and above (`:180-187`). Each member is
then reported with `have = group size` (`:189-201`). Constructors:
- `backend/dr/src/lib.rs:1017` (EM repair loop) and `:2578` (test `em_rule`)
- `frontend/library/src/lib.rs:1736` (`em_rules`, `:1700`)
- `em.rs:265` (test `em()`)

Edits:
1. `em.rs`: add `pub front_row: bool` to `Electromigration`, with a doc comment covering `em_front_row_cuts`,
   Hastings §5.1.3 and the default `true`. In `walk`, when `self.front_row`, replace the group's `have` with:
   - Common metals: the landing shapes on both sides (indices into `all`) that touch **every** cut of the group. dr
     draws a pad per cut, and a pad that touches one cut says nothing about the array.
   - `n_front(m)` for a common metal with `w ≠ h`: the largest number of the group's cuts with the same centre
     coordinate along `m`'s long axis. Use the centre x when `w > h`, else the centre y.
   - For a square `m`: the group size.
   - `have = min n_front` over the common metals. With no common metal, `have` = the group size.

   Keep `ratio = ua/(ua_per_cut·have)`. Add a helper `fn front_row(cuts: &[&Rect], m: &Rect) -> u32` (a sort or count
   over the centre coordinate).
2. `em.rs` tests: `em()` sets `front_row: true`. The two group-count tests (`a_via_group_needs_ceil_i_over_i_cut` and
   `array_cuts_on_separate_pads_over_one_trunk_are_one_group`) build `Electromigration { front_row: false, ..em() }`.
   They test REL-03's grouping, and their cuts lie along the current. Their assertions stay as they are. Each also
   asserts the `front_row: true` result:
   - `a_via_group_needs_ceil_i_over_i_cut`: `have` 1 → residual (3−1)/3.
   - `array_cuts_on_separate_pads_over_one_trunk_are_one_group`: residual 2/3.
3. New test `a_deep_array_counts_its_front_row`. Wires:
   - met1 `shape(1, 0, 0, 10_000, 800)`
   - met2 `shape(3, -100, -100, 1_500, 1_500)`
   - six cuts `shape(2, x, y, 200, 200)` for x ∈ {0, 500, 1000}, y ∈ {0, 500}. The cut at y 500 spans 500..700 < 800,
     so it lands on met1.

   Terminals: +700 µA at the met1 far end, −700 µA on the met2 pad. Assert `usage` = 700/(290·2), and that `residual`
   is within 1e-4 of 1/3 (need 3, have 2). With `front_row: false` it is satisfied (6 ≥ 3).
4. Config: `dr::DetailedCfg` gains `pub em_front_row: bool` (`Default` true), read at `dr/lib.rs:1017` and in test
   `em_rule` `:2578`. The library sets it, and `em_rules` gets a `front_row: bool` argument, from
   `pdk.cell.get("em_front_row_cuts").and_then(serde_json::Value::as_bool).unwrap_or(true)`. Change the call at
   `lib.rs:708` and the test calls at `:2521, :2527`.
5. `backend/verify/src/sidecar.rs` `KEYS` (alphabetical): `k("em_front_row_cuts", Bool, false, true,
   "frontend/library/src/lib.rs em_rules; backend/dr DetailedCfg (REL-12)")`.

Tests: `cargo test -p analog em::` then `cargo test -p dr` and `cargo test -p library --lib`.
- If a dr test now fails because dr's arrays lie along the current, that is RTE-27's job (orient arrays). Report it.
  Do not set `front_row: false` in a test whose subject is the via count.
- Acceptance (bench `em cuts` 0, arrays across the current) is RTE-27's. REL-12 reports the bench `em cuts` count
  before and after (expected to rise until RTE-27).

## REL-13 Resistor self-heating width floor — do

Facts: `kernel/cells/src/resistor.rs` has no current input (`ResModel` `:17-80`, `value_tol_ppm` `:83`). `Process`
(`kernel/core/src/process.rs`) has `sheet_ohm(role)`. `Pdk::pex_f32_named(name, "height_nm")` (`verify/pdk.rs:879`)
gives `rbody_po` 326.2 nm. REL-13 owns the formula and the keys. CELL-23 applies it and reports per-resistor ΔT.

Edits in `kernel/cells/src/resistor.rs`:
```rust
/// κ_ox, W/(m·K) (Hastings's 0.011 W/cm/°C).
const K_OX: f64 = 1.1;
/// Hastings eq. 5.8 (L12044–12073): W_min = I·√(t_ox·R_s/(κ_ox·ΔT)), nm, rounded up; 0 when any input ≤ 0.
#[must_use]
pub fn self_heating_min_width_nm(i_ua: f32, sheet_ohm: f32, t_ox_nm: f32, dt_k: f32) -> i32
/// Eq. 5.8 inverted, the reported figure: ΔT = t_ox·R_s·(I/W)²/κ_ox, K; 0 when any input ≤ 0.
#[must_use]
pub fn self_heating_rise_k(i_ua: f32, sheet_ohm: f32, t_ox_nm: f32, w_nm: i32) -> f32
```
Compute in f64. `W = i·1e-6·sqrt(t·1e-9·R/(1.1·dt))·1e9`, then `.ceil()`.

Register two keys in `sidecar.rs` `KEYS`:
- `k("res_self_heat_dt_k", Real, false, true, "kernel/cells/src/resistor.rs self_heating_min_width_nm (CELL-23; default 5 K, Hastings eq. 5.8)")`
- `k("res_tox_nm", Real, false, true, "kernel/cells/src/resistor.rs self_heating_min_width_nm (CELL-23; else pex height_nm of the body layer)")`

Neither is added to any shipped deck. The `t_ox` order (sidecar, else body-layer `height_nm`, else skip) is CELL-23's
call site.

Test `resistor.rs` `hastings_eq_5_8`:
- `self_heating_min_width_nm(100.0, 2000.0, 326.2, 5.0)` is within 5 of 1089.
- `(1000.0, 2000.0, 326.2, 5.0)` is within 5 of 10 891.
- Any argument ≤ 0 gives 0.
- `self_heating_rise_k(100.0, 2000.0, 326.2, 1089)` is within 0.02 of 5.0.

Command: `cargo test -p cells hastings_eq_5_8`. Acceptance (every resistor ΔT ≤ 5 K, reported) needs CELL-23's call
site. REL-13 states this and does not claim it.

## REL-15 Substrate balance of differential pairs — do

Facts: `Isolation` (`isolation.rs:11-47`) is dimensionless (PLC-18). `Layout::centre(t)`
(`kernel/core/src/layout.rs:100`) returns `(i32, i32)`. Emission is `emit::isolation` (`emit.rs:256`). `emit::leaves`
gives DiffPair leaves (`emit.rs:213` uses `l.kind == BlockKind::DiffPair && l.devices.len() == 2`). `block_of` is built
at `annotator/src/lib.rs:384-387`, just before line 399.

Correction to the plan's cost: `4e-3·Δd²` nm² mixes units with the dimensionless costs. Use
`cost = ((d_ca − d_cb)/d_ab)²`, where `d_ab` is the a–b centre distance. By the triangle inequality the cost is in
[0, 1]. It is 1 exactly on the pair axis, Charbon's worst case (§8.3.1), and 0 on the bisector. When `d_ab == 0` the
cost is 0.

Edits:
1. `kernel/analog/src/placement/isolation.rs`: add the rule.
   ```rust
   /// Charbon §8.3.1 (L3382–3399): an injector equidistant from both halves of a differential pair
   /// couples a common-mode disturbance only. Cost-only (no threshold in the source).
   #[derive(Clone, Copy)]
   pub struct SubstrateBalance { pub aggressor: Target, pub a: Target, pub b: Target }
   ```
   `impl Rule`:
   - `type On = Layout`.
   - `cost` as above, with f64 distances from `l.centre`.
   - `satisfied` true. `residual` 0. Use the trait defaults where they already give that.
   - `touches` pushes the device ids of all three.
   - `retarget` maps all three.

   Export it next to `Isolation` (check `placement/mod.rs`).
2. `backend/annotator/src/emit.rs`: add
   `pub fn substrate_balance(aggressor: &[bool], blocks: &[Block], block_of: &[usize], r: &mut Requirements<Layout>)`.
   For every DiffPair leaf with two devices and every aggressor `g` with `block_of[g] != block_of[pair dev 0]`, push one
   `SubstrateBalance`. Push the batch to `r.cost` only. Call it at `annotator/src/lib.rs:402`, after `emit::isolation`.
   It keeps the same `Isolation` origin tag path, or the net-class fallback at `:404-407`. Check the origin lookup
   handles the new type through `touches`.

Tests:
- `isolation.rs` `an_aggressor_on_the_bisector_costs_nothing`: a at (−5000, 0), b at (5000, 0), aggressor at
  (0, 10 000) → cost 0 (|·| < 1e-6).
- `off_axis_costs`: aggressor at (−20 000, 0) → d_ca 15 000, d_cb 25 000, d_ab 10 000, cost 1.0 ± 1e-6. Add an
  aggressor at (−5000, 10 000) → d_ca 10 000, d_cb √(2e8) = 14 142.1, cost 0.1716 ± 1e-3.

  Build a 3-device `Layout` the way other placement tests do (grep `Layout {` in `kernel/analog/src/placement`).
- `emit.rs` test `substrate_balance_skips_the_aggressors_own_block`, on the existing `strongarm_like()` fixture: the
  stage's `mn0` (clk) emits none for `mn1/mn2`, and the lone `XS` emits one per pair.

Commands: `cargo test -p analog isolation`, `cargo test -p annotator substrate_balance`, `cargo test -p annotator`.

Acceptance (final |d_ca − d_cb| per pair decreases vs. the rule off): measure once on the strongarm/comparator fixture
by placing with and without the emission (a local, uncommitted toggle). Report both numbers. If the fixture has no
aggressor outside the pair's stage, report that the acceptance is vacuous there and do not invent a fixture.

## REL-17 ESD-path adiabatic width floor — do

Facts:
- `stack::Layer` (`kernel/analog/src/routing/stack.rs:10-30`) has no thickness. `elaborate::stack`
  (`elaborate.rs:281-305`) fills it from `pex_f32`.
- `Electromigration` is built only for nets with ≥ 2 device terminals (`lib.rs:1731-1737`). Its `walk` returns `None`
  when the DC flow is unknown (`em.rs:149`). A pad net's current is usually unknown, so an ESD floor folded into
  `Electromigration` would read unknown.

Correction: a separate hard rule in `em.rs` instead of an `Electromigration` field. The width floor does not depend on
DC current.

Edits:
1. `stack.rs` `Layer`: add `pub thickness_nm: f32` (0 = unknown). `elaborate::stack` sets
   `thickness_nm: pdk.pex_f32(l, "thickness_nm").unwrap_or(0.0)`.
2. `em.rs`:
   ```rust
   /// Hastings eq. 14.6 (L45870–45932): A = √(ρ·τ·I_pk²/(2·C_V·ΔT)), τ 225 ns, ΔT 50 K, I_pk = hbm_v/1500 Ω; µm².
   #[must_use]
   pub fn esd_area_um2(hbm_v: f32, rho_uohm_cm: f32, cv_j_per_k_cm3: f32) -> f32
   /// Table 14.3 (ρ µΩ·cm, C_V J/K/cm³) for `metal_family`: "al" (2.7, 2.42), "cu" (1.7, 3.45); absent or other → Al,
   /// the larger area.
   #[must_use]
   pub fn metal_family(key: Option<&str>) -> (f32, f32)
   /// Every routed metal shape of an ESD pad net has a cross-section ≥ `area_um2`:
   /// short side ≥ area·1e6/thickness_nm. A layer of thickness 0 is unknown for it.
   #[derive(Clone, Copy)]
   pub struct EsdWidth { pub net: NetId, pub area_um2: f32, pub stack: &'static Stack }
   ```
   `impl Rule for EsdWidth` (`On = Routes`, `REPAIR = RepairKind::Em`):
   - `known`: at least one routed shape on a metal (`!cut`) with `thickness_nm > 0`.
   - `residual`: worst `over(need − have, need)` over those shapes.
   - `satisfied`: residual ≤ 0.
   - `cost`: residual.
   - `usage`: worst need/have.
   - `touches`: the net.

   Compute in f64: `ρ[Ω·cm] = rho·1e-6`, then `sqrt(ρ·225e-9·(hbm/1500)²/(2·cv·50))·1e8`.
3. `frontend/library/src/lib.rs` `Config` (`:80`, `Default` at `:195`) gains `pub esd: Option<EsdSpec>`, default
   `None`, with `pub struct EsdSpec { pub hbm_v: f32, pub nets: Vec<String> }`. Next to `em_rules` (`:708`): when
   `cfg.esd` is set and a stack is present, find each named net by `netlist.nets[k].name`. A name that is not found
   goes to `problem.missing`. Push `EsdWidth { net, area_um2: esd_area_um2(hbm, ρ, C_V), stack }` into
   `problem.routing.hard`. Get ρ and C_V from `metal_family(pdk.cell_str("metal_family"))`.
4. `sidecar.rs` `KEYS`: `k("metal_family", Text, false, true, "kernel/analog/src/routing/em.rs metal_family (REL-17 ESD width; \"al\" | \"cu\")")`.

Tests in `em.rs`:
- `hbm_2kv_on_sky130_met1_needs_18_6um`:
  - `esd_area_um2(2000., 2.7, 2.42)` is within 0.01 of 6.68.
  - `esd_area_um2(2000., 1.7, 3.45)` is within 0.01 of 4.44.
  - Need at t = 360: `6.68e6/360` is within 30 of 18 557.
- `an_esd_net_narrower_than_its_floor_fails`: a stack with met1 `thickness_nm: 360`, one met1 shape 10 000 × 10 000
  → satisfied. 10 000 × 18 000 → satisfied. 10 000 × 9 000 → residual within 1e-3 of (18 557 − 9 000)/18 557.
  The same at `thickness_nm: 0` → `!known`.

Commands: `cargo test -p analog esd` and `cargo test -p library --lib`.

Acceptance (0 ESD-net width violations when `esd` is set): it is real once RTE sizes to the floor (RTE-14 reads it,
optional). Report the count on one fixture with `esd` set on its input net. A non-zero count is the expected honest
result until RTE consumes it.

## GAP-13 Latent antenna — do

Facts: `Stack::antenna(&self, shapes, cell, gates, fallback_nm2)` (`stack.rs:308-344`) builds pieces per stage from the
net's shapes only. Callers:
- `Antenna::worst` (`kernel/analog/src/routing/antenna.rs:66-73`)
- dr test `backend/dr/src/lib.rs:2918, 2921`
- stack tests `stack.rs:594-659`

`Routes.wires` and `Routes.cell` hold every net's shapes (`kernel/core/src/routes.rs:63-91`). `Pdk::min_spacing(layer)`
is at `verify/pdk.rs:937`.

Edits:
1. `stack.rs` `Layer`: add `pub latent_merge_nm: i32` (0 = no latent merge). `elaborate::stack` sets it from sidecar
   `pdk.cell_f32("latent_merge_nm")` when present, else `pdk.min_spacing(l.0).unwrap_or(0)`. For sky130 met1 that is
   140.
2. `Stack::antenna` gains `others: &[Shape]` (after `cell`). At stage `s`, a piece's area also includes every foreign
   shape on layer `s` whose Euclidean edge gap to one of the piece's layer-`s` rects is ≤ `layer.latent_merge_nm`.
   Use `dx = max(0, b.x − a.right, a.x − b.right)`, the same for `dy`, and compare `dx² + dy² ≤ g²` in i64. The ratio
   is charged to the piece's own reached gates. Foreign gate area is not credited: the stack sees only this net's
   pins, which is pessimistic, so it is only stricter than before. Hop count is 1.

   Doc it: `// ponytail: one hop, foreign gates uncredited; a chain of min-spaced nets is charged only its direct
   neighbours`. Pass `&[]` at the dr and stack-test call sites.
3. `antenna.rs` `Antenna::worst`: build `others` from `r.wires` and `r.cell` of every net `≠ self.net`
   (`.iter().enumerate().filter(|(k, _)| *k != self.net.0 as usize).flat_map(|(_, v)| v)`). The cost is O(all
   shapes) per evaluation. Note the ceiling, and the upgrade path: a per-layer spatial index if routing time shows it.
4. `sidecar.rs` `KEYS`: `k("latent_merge_nm", Nm, false, true, "frontend/library/src/elaborate.rs stack (GAP-13; default the layer's min spacing)")`.

Tests in `stack.rs`, using the `stack(100.0, 400.0, false)` helper with met1 `latent_merge_nm: 140` and gate 1 µm²:
- `two_min_spaced_pieces_share_one_antenna`: net shape `shape(1, 0, 0, 60_000, 1_000)` (ratio 60 alone, under 100).
  The foreign shape is `shape(1, 0, 1_140, 60_000, 1_000)`, gap 140. `antenna(&mine, &[], &foreign, &[], gate)` →
  `Some((120.0, 100.0))`, violated. With `others = &[]` → `(60.0, 100.0)`.
- `a_wider_gap_is_independent`: the foreign shape at y 1 141 (gap 141) → `(60.0, 100.0)`.

Commands:
- `cargo test -p analog stack::`
- `cargo test -p dr`
- `cargo test -p library --lib`
- the REL T2 guard `benchmarks/tests/signoff_fixtures.rs:210` `antenna_in_loop_never_passes_what_signoff_fails`:
  `cargo test --release -p benchmarks --test signoff_fixtures antenna_in_loop` in the background. Acceptance: it still
  passes. The in-loop model only adds area.

---

Order: REL-16, REL-12, REL-13, REL-15, REL-17, GAP-13. REL-12, REL-17 and GAP-13 all touch `em.rs`/`stack.rs`/`sidecar.rs`:
commit each item separately (`M2+ REL-xx: …`). Keep `KEYS` alphabetical.
