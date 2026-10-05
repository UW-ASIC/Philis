# M2+ cells, segment 9 (M6): GAP-11, GAP-14

State on entry: both items are already implemented on `m2-cells` (commits `d5cee37` GAP-11, `260b155`
GAP-14); this card records what exists, corrects the plan, and lists what is left. Merging `m2` (`f5c8139`)
broke one initializer: `frontend/library/src/elaborate.rs:731` (a test from RTE-23, routing 4) built
`Unitization` without GAP-11's `kind` field; fixed with `kind: None` in this commit. `cargo check --workspace
--tests` is clean after it.

## GAP-11 Aspect-ratio limits for matched variants — class: do (done; verify only)

Deps (dag): GAP-01, EXT-16, both merged.

Code facts (plan's `cellgen.rs:600-613` is stale):
- `kernel/analog/src/cell.rs:70-71` `Unitization.kind: Option<intent::MatchKind>` (None = no limit).
- `frontend/library/src/cellgen.rs:806` `fn aspect_limit(class: Option<MatchClass>, kind: Option<MatchKind>) -> Option<f64>`:
  Current 10/3/1.5, Voltage 3/1.5/1.5, `Ratio` and unknown kind → None, class `unwrap_or(Moderate)`.
- `cellgen.rs:819` `fn member_aspect(m: &Macro) -> f64`: worst member subarray aspect on the unit grid.
- `cellgen.rs:~840` `fn keep_compact(alts: &mut Vec<Macro>, limit: f64) -> bool`: filters; if none fits keeps
  the squarest and returns false.
- `cellgen.rs:159-163` call site in `draw_variants`; misses counted in `aspect_missed` (`cellgen.rs:31`, 215),
  surfaced as `("MatchClass", "no variant meets the aspect limit")` through `frontend/library/src/lib.rs`.
- Folding tie-break (`cellgen.rs:758-767`) still prefers |log aspect| among parity-equal shapes.

Tests (exist, pass): `frontend/library/src/cellgen.rs:1147 current_mod_filters_4_to_1` (asserts
`member_aspect` 4.0 / 2.0, Moderate+Current limit 3, 4:1 dropped, 2:1 kept);
`cellgen.rs:1162 last_variant_is_kept_and_reported` (Exceptional+Voltage keeps one, returns false; Ratio → None;
None class → 3.0). Command: `PDK_ROOT=... cargo test -p library --lib -- current_mod last_variant` (2 passed).

Left: acceptance "PERF-23 ALRC compactness row 0 failures" is checked when PERF-23 runs (owned by verify);
no product edit pending.

## GAP-14 User-declared deep-n-well isolated tubs for NMOS — class: do (done; verify only)

Deps (dag): EXT-26, CELL-17 merged; step-level PERF-18 (LVS p-well binding) merged in M3, so the
`lvs-coverage/unverified` fallback in the plan's step 3 is not needed: the isolated bulk extracts as its own net
and binds.

Code facts (plan's `bjt.rs` "only for NPN" is stale):
- Sidecar: `backend/annotator/src/sidecar.rs:19,200-218` parses `{"constraint":"IsolatedTub","instances":[..],"tie":".."}`;
  refuses non-NMOS / mixed-bulk members (`sidecar_unsupported`). Tests `an_isolated_tub_parses`,
  `a_tub_with_a_pmos_is_refused` (`sidecar.rs:267,275`).
- `kernel/analog/src/cell.rs` `GuardRingType::Tub { id: u16 }` (id keeps distinct tubs from merging).
- `backend/annotator/src/rings.rs:82-90`: tub members get a `Tub{id}` victim ring tied to `tie`; without
  `tub_drawable` → plain ring plus `("IsolatedTub", "deck has no deep n-well: tub drawn as an ordinary ring")`
  (tests `backend/annotator/src/tests.rs:893,902`).
- `frontend/library/src/lib.rs`: `ann.tubs.extend(side.tubs)`, `tub_drawable: cells::post_cell::drawable(Tub{id:0}, pdk)`.
- `kernel/cells/src/post_cell.rs:212` draws `dnwell` around the n-well band; `:340 tub_well_ext`, `:432` drawable
  needs `dnwell`/`nwell`/`nsdm` + `enclosure("dnwell","nwell")`; `:446` halo owes foreign DNW `space("dnwell")`.
  Plan correction: no separate inner p-well tap is drawn; the device cell's own bulk tap ties the isolated p-well.

Tests (exist, pass): `kernel/cells/src/post_cell.rs:931 a_tub_is_drc_clean_on_sky130` (one dnwell enclosing the
cell, 4 n-well band rects clear of diff by `space_between("nwell","diff")`, klayout findings empty);
`:958 the_tub_bulk_is_not_the_substrate` (extracted nfet bulks are `["B","SUB"]`);
`:978 a_tub_halo_clears_dnwell_spacing`. Command: `cargo test -p cells --lib tub` (3 passed).
Acceptance: `frontend/library/tests/extra_devices.rs a_declared_tub_signs_off_clean` (no drc*/lvs* hard rows,
exactly one dnwell shape). Command: `PDK_ROOT=... cargo test --release -p library --test extra_devices a_declared_tub`.

Left: nothing in scope.
