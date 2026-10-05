# M2+ placement, segment 8 (M3): PLC-18

## PLC-18 Dimensionless placement objective (class: do — already done; verify only)

Facts (HEAD 72bf639, m2 merged, "Already up to date"):
- Implemented in `967798a` "M2+ PLC-18: dimensionless placement objective" and `f8f9da0` "record PEX term
  shares", both ancestors of `m2` and HEAD (card `m2-placement-3.md` was the spec). Segment 8 repeats it.
- `kernel/core/src/layout.rs:60` `Layout::l_ref` (sqrt Σ cell area from `hw`/`hh`, 1 with no cells).
- `backend/gp/src/mechanics.rs:259` PEX = `HPWL / L_ref + analog_cost`; `backend/gp/src/lib.rs:372` gp minimises `L_ref·E`.
- `backend/dp/src/lib.rs:316` T0 has no PEX-unit floor.
- Costs squared ratios: `proximity.rs:30` `(excess/max_distance)²`, `isolation.rs:22` `(shortfall/min_distance)²`,
  `dti.rs:57` `(miss/band)²`, `symmetry.rs:29` `/L_ref`, `matched_set.rs:253` `/L_ref` (MAT rules, incl. the
  old `1e-3`/`3e5` weights, now in MatchedSet). No raw `1e-3`/`4e-3` cost weights remain in
  `kernel/analog/src/placement/*.rs` (only test tolerances).
- Plan stale: `HeatSeparation` and `SymmetryIsland` do not exist in the tree (`grep -rl` empty); nothing to convert.
- Test exists: `backend/dp/src/tests.rs:635` `energy_is_invariant_to_scaling_all_lengths`.
- Acceptance recorded: `docs/CRATES.md:60-68` term shares on `ota` cold dp call, HPWL/L_ref 75.4 % max (< 80 %, pass).
  Last dp call is 100 % priced budgets after T6 price steps: PLC-09 question, out of scope.

Edits: none.

Verify:
`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk; cargo test -p dp energy_is_invariant_to_scaling_all_lengths`
(and `cargo test -p analog placement::` for the cost units). If green, close the item with no code change.
