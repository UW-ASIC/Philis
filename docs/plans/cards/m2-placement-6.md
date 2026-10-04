# M2+ placement, segment 6 (M4): PLC-17

Worktree `philis-m2/placement`, branch `m2-placement`, HEAD `84e1d52`. `git merge m2`: nothing to merge (`m2` =
`be34be4`, already merged at `3230772`). Spec: `plan-04-placement.md` `### PLC-17` (l.565-571). DAG entry: module
placement, M4, no hard deps, no step-level notes. Not blocked by PLC-06. The per-epoch C feedback this item once
carried is GAP-08's (PERF owns `Weights.place`, master plan C9); PLC-17 is only the static supply weighting (AP-18), so
the two do not fight: GAP-08 multiplies whatever `net_weights` returns.

## PLC-17 Current-weighted supply nets (class: do)

### Current code facts (plan line refs are stale; corrected here)

- `gp::net_weights(classes, sens) -> Vec<f32>`: `backend/gp/src/lib.rs:196-212` (plan said `:136-153`), doc
  `:180-195`. Index = position in `classes` (callers keep `classes[i].net == NetId(i)`). Budgeted / sensed nets are
  normalised to mean 1 over `known`; every other net, rails included, gets `1.0` (`:211`).
- Only caller: `frontend/library/src/lib.rs:700` (`gp::net_weights(&problem.net_classes, &sens)`, once per run;
  plan said `:296`). Tests: `gp/lib.rs:788`, `:794` (`mod weight_tests` `:777-798`). dp only consumes the vector.
- Net currents: `oppoint::net_current_ua(netlist, draws) -> Vec<Option<i32>>` (`frontend/library/src/oppoint.rs:112`)
  returns **`i32` µA**, not `f64` as the plan says. It is computed only inside the IR block,
  `let i = oppoint::net_current_ua(netlist, c);` at `lib.rs:685`, under `if let (Some(c), Some(h)) =
  (&bias.currents, &bias.net_headroom_mv)` (`:683`), so it is not in scope at `:700`.
- Routing mask: `lib.rs:724-727` zeroes nets with no `c_budget_af` and no `sens` row, then scales to max 1. A rail with
  a `c_budget_af` would pass the mask, so its routing weight does change (normalised by `top`); unbudgeted rails stay
  0 as the plan says.
- `NetClass::{Supply, Ground}`: `kernel/analog/src/metadata.rs:18-26`.

### Edits

1. `backend/gp/src/lib.rs::net_weights`, new signature
   ```rust
   pub fn net_weights(classes: &[analog::metadata::NetClassification], sens: &[(pnr_core::NetId, f32)], current_ua: &[Option<i32>]) -> Vec<f32>
   ```
   Steps: compute `raw` as today; then `let rail = |i: usize| matches!(classes[i].class, NetClass::Supply |
   NetClass::Ground);`. Rails are removed from the normalisation (`known` skips `rail(i)`), so their budgets no longer
   move the signal mean (AP-18, "normalise each source separately"). `let i_ref = (0..classes.len()).filter(|&i|
   rail(i)).filter_map(|i| current_ua.get(i).copied().flatten()).max().unwrap_or(0).max(1) as f32;`. Output per
   net: rail with `Some(i)` -> `(i as f32 / i_ref).clamp(0.1, 1.0)`; rail with `None` / index past `current_ua`
   (no op point) -> `0.25`; else today's `r.map_or(1.0, |w| w / mean)`.
   Constants: `const RAIL_UNKNOWN: f32 = 0.25;` `const RAIL_MIN: f32 = 0.1;` with a one-line doc each, labelled
   **policy** (a rail's HPWL matters for IR/EM, not for C; EM-11 / lienig_em L3457-3462).
   Doc (`:180-195`): replace "rails (unbudgeted) keep `1`" by the rail rule, `current_ua` indexed by `NetId`, µA.
2. `frontend/library/src/lib.rs` `:683-700`: hoist the current vector above the IR block:
   `let net_ua = bias.currents.as_ref().map_or_else(Vec::new, |c| oppoint::net_current_ua(netlist, c));`
   the IR block drops its `let i = ...` and passes `&net_ua` to `annotator::ir::budgets`; `:700` becomes
   `gp::net_weights(&problem.net_classes, &sens, &net_ua)`. Routing mask `:724-727` unchanged.
3. `gp/lib.rs:788`, `:794`: add `&[]` as third argument.

### Tests (`backend/gp/src/lib.rs` `mod weight_tests`)

Add a helper `fn rail(net: u16, class: NetClass, c_budget_af: Option<i64>) -> NetClassification`.

- `a_tighter_budget_pulls_harder_and_sensitivities_win` (existing; plan's `signal_weights_are_unchanged`): only the
  extra `&[]` argument; message at `:791` becomes "an unbudgeted signal net keeps unit weight" (net 2 is
  `NetClass::Signal`, so `w[2] == 1.0` still holds). All assertions unchanged.
- `supply_weight_scales_with_current`: classes `[Supply 0, Ground 1, Supply 2, Signal 3 (budget 1_000)]`,
  `current_ua = [Some(100), Some(1_000), None, None]`. Assert `(w[0] - 0.1).abs() < 1e-6`, `w[1] == 1.0`,
  `w[2] == 0.25`, `w[3] == 1.0` (sole budgeted net, mean 1). Second call with `current_ua = &[]`: every rail `== 0.25`.
  Third: `[Some(10), Some(1_000)]` -> `w[0] == 0.1` (clamped, 0.01 raw).
- `rails_do_not_move_the_signal_mean`: classes `[Signal 0 (1_000), Signal 1 (4_000), Supply 2 (budget 10)]`, `&[]`,
  `&[None, None, Some(50)]`: `assert!((w[0] + w[1] - 2.0).abs() < 1e-5)` and `w[2] == 1.0` (only rail, so
  `I_ref` = its own current). Fails if rails stay in `known`.
- Command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk; cargo test -p gp weight_tests`, then
  `cargo test -p library --release --test flow_smoke --test placement_metrics` and
  `cargo test -p library --release --test perf_postlayout op_runs_carry_ir_limits` (op path compiles and runs).

### Acceptance (measured once, not committed as code)

Temporary `#[test]` in `frontend/library/tests/perf_postlayout.rs` (reuse `models()`, `op_cfg`, the `ota` and
`rc_filter` fixtures; `Config { op: Some(op_cfg(lib)), seed, ..Default::default() }`, seeds 1-5), run once
before the edits (baseline, HEAD `84e1d52`) and once after. Per run record: IR Θ (the `IrDrop` routing batch: `sol.metadata.routing` row
`kind == "IrDrop"` violations, and `library::signoff(&sol, &pdk)` `ir_drop` findings' worst drop/limit) and the median
extracted C over budgeted nets (`library::signoff_c_tier`, `lib.rs:1631`; confirm its per-net output when writing the
test). Pass iff both are no worse than baseline on every fixture (median over seeds). Record the table in
`docs/CRATES.md` under `## backend/gp`; delete the temporary test. If either is worse, report the numbers; do not tune
0.1/0.25 to force a pass without saying so (they are policy).
