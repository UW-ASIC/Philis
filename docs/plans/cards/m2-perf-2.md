# M2 perf segment 2 (M3): implementation cards

Branch `m2-perf`, worktree `philis-m2/perf`. `git merge m2` fast-forwarded to `c73fccf` (no conflicts).
`docs/plans/dag-m2-m6.json` is still only in the main checkout and was read from there: GAP-17 has no hard deps
(PERF-09 done) and perf owns only step 1; PERF-18 has no hard deps (FLOW-04, PERF-02, PERF-08 done) and one
step-level note: its `nwell` merge-list step comes after PERF-16 step 1. Line numbers are for this tree. Find code
by the symbol named next to each one. Every command needs
`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

Order: GAP-17 → PERF-18. They share no code.

| Item | Class |
|---|---|
| GAP-17 | do (step 1 only; steps 2–3 are REL-10 steps 4–5, reliability module) |
| PERF-18 | do (steps 1, 2-BJT, 4; step 2's MOS/n-well part is already in the vendored deck; step 3 moves to PERF-16 step 1) |

Batch rules: no new warnings in `library`, `verify`, `benchmark`. If a test not named here goes red, stop and report
it. Do not edit its assertion.

---

## GAP-17 Node voltages in the op point (step 1)

### Current code (corrections to 98-gap-critic marked *)

- `frontend/library/src/oppoint.rs:16-44` `OpPoint`. \* It is no longer "power, Id, headroom, gm only". PERF-09
  added `bjt_ua`, `vgs_v`, `vds_v` and `vbs_v` (:24-37). There is still no per-net voltage.
- \* The control block is in `build_deck` (:320-346), not at :238. It reads
  `op / echo @@PHILIS_OP / show m : id,vgs,vds,vbs,vdsat,gm / show q : ic,ib,ie,p / show r : i,p / echo @@PHILIS_END`
  (:334-342).
- `extract` (:232-310) builds `OpPoint` as a struct literal (:245-257), so a new field must go there. The test
  literals (:757, :791, :868, :881, :903) use `..Default::default()` and need no edit.
- `node_name(netlist, id)` (:463-474) gives the exact ngspice node name: ground maps to `"0"`, everything else is
  lowercased with `/ . < >` replaced by `_`.
- `parse_show` (:546-600) stops at the first `@@PHILIS_END` line.
- \* The open question "does `print all` print every node" is now settled. A canned ngspice-45 run (scratch, a
  4-resistor divider) between echo markers printed one `name = value` line per node, all lowercase, plus
  `v1#branch = …` for the source current. Ground was not printed. The same run with sky130 `tt` and two
  `sky130_fd_pr__pnp_05v5_W3p40L3p40` instances printed `e1 = 6.795725e-01`.
- No consumers exist yet: `OpFacts.net_mv` (EXT-17, annotator, M3) and `PairAging.dvbs_mv` (REL-10) are not in the
  tree (`grep -rn net_mv` is empty). Filling `net_mv` belongs to EXT-17, not to this item.

### Edits (`frontend/library/src/oppoint.rs`)

1. `OpPoint` gains, after `vbs_v`:
   ```rust
   /// DC voltage per net, V, indexed by `NetId`; ground nets read 0.0. `None`
   /// for a net the simulated circuit does not reach (only capacitors, which
   /// the flat deck drops) or that ngspice did not print.
   pub net_v: Vec<Option<f64>>,
   ```
2. `build_deck`: after `echo @@PHILIS_END\n`, append `echo @@PHILIS_NV\n print all\n echo @@PHILIS_NV_END\n`
   (same `\n\` line style). Putting it after `@@PHILIS_END` leaves `parse_show` untouched, because it breaks at
   END. Update the doc line "a control block that dumps every MOSFET's operating point" to say it also dumps every
   node voltage.
3. New `fn parse_nodes(text: &str) -> std::collections::HashMap<String, f64>`: take the lines strictly between
   `@@PHILIS_NV` and `@@PHILIS_NV_END`. Keep lines that split into exactly `[name, "=", value]` where `value`
   parses as `f64` and `name` contains no `#` (branch currents). Key = `name.to_ascii_lowercase()`.
4. `extract`: `let nodes = parse_nodes(&stdout)`. Bind `String::from_utf8_lossy(&out.stdout)` once and use it for
   both parsers. Set `net_v: (0..netlist.nets.len()).map(|i| { let n = node_name(netlist, pnr_core::NetId(i as u16));
   if n == "0" { Some(0.0) } else { nodes.get(&n).copied() } }).collect()` in the literal.

### Tests

- `oppoint.rs` `print_all_reads_node_voltages`: canned text with an `@@PHILIS_OP … @@PHILIS_END` show block, then
  `@@PHILIS_NV`, `vdd = 1.800000e+00`, `x1_mid = 9.000000e-01`, `v1#branch = -9.00000e-04`, `@@PHILIS_NV_END`.
  Assert `parse_nodes` gives exactly two keys, `["vdd"] == 1.8`, `["x1_mid"] == 0.9`, and no key containing `#`.
  Also assert that `parse_show` on the same text is unchanged (its device map is the same with or without the NV
  block).
- `frontend/library/tests/perf_postlayout.rs` `ota_op_resolves_every_net_voltage` (gated on `models()` like its
  neighbours): `fixture("ota")` with `op_cfg(lib)`. Then:
  - `op.net_v.len() == nl.nets.len()`;
  - every entry is `Some`;
  - `VSS` reads `Some(0.0)`;
  - `|VDD − cfg.vdd| < 1e-6`;
  - `vbias` and `vbn` are within 1e-6 of `cfg.vdd / 2` (the probe drives gate-only nets);
  - `vtail` is strictly inside `(0, cfg.vdd)`.
- Command: `cargo test -p library --lib oppoint && cargo test -p library --test perf_postlayout`.

### Acceptance

ota with an op run has `net_v` resolved for every net (the test above). "EXT-17 regions unchanged" and "REL T9
reports V_SB per matched pair" belong to EXT-17 and REL-10 (other modules). Report them as not this item's.

---

## PERF-18 Complete LVS on sky130: MOS bulk, n-well net, BJT recognisers

### Current code (corrections to plan-07 marked *)

- \* The vendored deck `pdks/decks/sky130.deck` (GPurify `6341f18` + `# PHILIS:` edits) **already has the MOS bulk
  and the n-well net**. The plan's "Current" describes the older GP deck:
  - `psub` / `psub_tie` are at :119-120;
  - `pwell_iso = dnwell not nwell`, `nwell_tie = ntap and nwell` and `pwell_iso_tie` are at :124-126;
  - `connect conductors [nsd, psd, ntap, ptap, nwell, pwell_iso, …]` is at :531;
  - `connect global psub` and the three `connect via …_tie` lines are at :537-540;
  - every MOS row is 4-terminal (:571-584). nfet rows come in pairs, `psub` and `pwell_iso` bulk; pfet rows use
    `nwell`.

  The upstream names are kept (`nwell_tie`, not the plan's `ntap_tie`). So step 2's `ntap_tie`, conductor and
  4-terminal MOS edits are **already done**. Write no duplicate rows: two rows on one marker means the earlier
  wins, `device.rs:275-277`.
- The deck still has **no BJT row**. `layer pnp = gds(82, 44)` (:83) exists, and there is no `npn` layer. The
  `# Devices:` comment at :712-716 says "the bipolars (npn, pnp) … have no recogniser yet".
- `backend/verify/src/lib.rs:706` already asserts reference arity 4 for an nfet. Before this item no test shows
  that a wrong bulk is caught.
- `frontend/library/src/cellgen.rs:953` `pub(crate) const BJT_PINS: [&str; 3] = ["E", "B", "C"]`. It is used by
  `reference` (:892, :905-906). `kernel/cells/src/bjt.rs:169-177` draws `Drawn.nodes` as
  `[Pin("E"), Pin("B"), Pin("C")]` ("matches `cellgen::BJT_PINS`"), and `kernel/core/src/macro.rs:53-55` documents
  the shared order. GPurify `6341f18` reads reference cards collector-first: `(Bjt,0)=>Collector, (Bjt,1)=>Base,
  (Bjt,2)=>Emitter` (`crates/check/src/lvs/graph.rs:298-300`). Layout positions are `[Base, Emitter, Collector]`
  (`topology/device.rs:456-458`). The latent swap is confirmed.
- `kernel/cells/src/bjt.rs:221-224` draws the `pnp`/`npn` marker as the unit's collector-outer rect `co`, if
  `process.layer(..)` resolves. `"npn"` is a known role (`backend/verify/src/pdk.rs:1387`), so adding the deck
  layer makes NPN units carry a marker.
- `recogniser_for` (`backend/verify/src/reference.rs:154-197`) reads polarity from the marker's first letter, so
  `pnp` and `npn` both work. A hint naming no row falls back to the first row of that polarity, so a `pnp_0p68`
  card binds the W3p40 row and fails as a model mismatch. That is honest, not hidden.
- \* **Op point of bgr_core** (master §Status: "the PNP stays PERF-18"). The scratch ngspice run above simulated
  both bgr_core PNPs with the shipped `sky130.lib.spice tt` (ie = 1.000 µA). The model exists and needs no extra
  `.param` (the `dkispp5x` family is defined in `corners/tt/nonfet.spice:39`). The bench failure must therefore come
  from the netlist side. Step 0 below pins it.
- Tests that encode "no BJT recogniser" and must follow the new behaviour. This is the premise changing, not a
  loosened check:
  - `backend/verify/src/lib.rs:714-725` (`reference_builder_compiles_two_devices`, the "A bipolar has no
    recogniser" tail);
  - `:731-755` `a_skipped_reference_device_is_listed_in_coverage` (uses `Npn` as the unrecognised kind);
  - `benchmarks/tests/signoff_fixtures.rs:52-66` `unverified()` (`Npn | Pnp => m`);
  - `:233-245` `uncompared_devices_do_not_block_convergence` (bjt_mirror, `assert!(uncompared > 0)`).

### Edits

0. Op point. Add `"bgr_core"` to the fixture list of `fixtures_resolve_every_device`
   (`frontend/library/tests/perf_postlayout.rs:232`). If it fails, the error text names the cause. Fix it in
   `oppoint::flat_circuit_with` or `deck_models` (`frontend/library/src/lib.rs:228`), whichever leaves a model name
   ngspice lacks. Do not special-case the fixture. If it passes, the M1 note was stale: say so in the report.
1. BJT pin order:
   - `cellgen.rs:953` becomes `pub(crate) const BJT_PINS: [&str; 3] = ["C", "B", "E"];`, with the doc "LVS card
     order, collector first (GPurify `lvs/graph.rs` reads card position 0 as Collector)".
   - `kernel/cells/src/bjt.rs:174` `nodes` becomes `[Pin("C"), Pin("B"), Pin("E")]`.
   - Update the `macro.rs:53-55` doc to name C, B, E.
   - The oppoint BJT card (`oppoint.rs` `flat_circuit_with`, `n("C"), n("B"), n("E")`) is already
     collector-first. Leave it.
2. `pdks/decks/sky130.deck`, BJT rows only. Put them after the diode rows (after :597), each under a `# PHILIS:`
   line:
   ```text
   # PHILIS: vertical PNP as VOL/klayout/lvs/sky130.lvs:745 (pnpid 82/44), 1089-1091 (E = p+ diff in nwell
   #   inside pnpid, B = n-tap, C = p-tap). Position order Base, Emitter, Collector (GP device.rs:456-458).
   #   Only W3p40L3p40: two rows on one marker, the earlier wins (device.rs:275-277), no emitter-area select.
   device bjt pnp model "sky130_fd_pr__pnp_05v5_W3p40L3p40" terminals [ntap, psd, ptap]
   # PHILIS: vertical NPN as sky130.lvs:744 (npnid 82/20), 1063-1065 (E = n+ diff in dnwell, B = p-tap,
   #   C = n-tap on the n-well ring). Only W1p00L1p00, same reason.
   device bjt npn model "sky130_fd_pr__npn_05v5_W1p00L1p00" terminals [ptap, nsd, ntap]
   ```
   - Add `layer npn = gds(82, 20)` next to `layer pnp` (:83), under `# PHILIS: npn marker (sky130.lvs:744)`.
   - Rewrite the :712-716 comment: drop "the bipolars (npn, pnp)" from "no recogniser yet" and add that only the
     W3p40 PNP and the W1p00L1p00 NPN are recognised.
3. (Moved) `nwell` joins the PERF-16 step 1 merge list (`build_store`'s `merge` = routing metals ∪ li ∪ `nwell`).
   It is done there, as the DAG's step-level note says. Today's fixtures pass LVS with the 4-terminal pfet rows
   already, so nothing here waits on it. Write it into the PERF-16 card.
4. No code change. `reference` already emits `D G S B` (`cellgen.rs:901-902`), and `reference::build` keeps the
   arity-4 terminals (`reference.rs:109-122`).
5. Update the tests whose premise changed:
   - `verify` `reference_builder_compiles_two_devices` tail: an `Npn` card with 3 terminals and a `Pnp` card with
     3 terminals give `set_reference(..) == []` (both recognised now). An `Inductor` card gives
     `[(RefKind::Inductor, None)]` (the "skipped, not mismatched" guarantee is kept on a kind that still has no
     row).
   - `a_skipped_reference_device_is_listed_in_coverage`: replace the `Npn` card with
     `RefKind::Capacitor, model: Some("sky130_fd_pr__cap_vpp_02p4x04p6_m1m2_noshield")` (no deck row), and assert
     `[(RefKind::Capacitor, _, 1)]`. Fix the comment "sky130 has no NPN recogniser".
   - `signoff_fixtures.rs` `unverified()`: drop the `Npn | Pnp` arm and update its doc ("no BJT").
   - `uncompared_devices_do_not_block_convergence`: run it on `dac4` (5 MOM caps, `m` = 1+1+2+4+8 = 16 uncompared
     units) instead of bjt_mirror and keep every assertion. If dac4 does not converge in 5 epochs, report it. Do
     not raise the epoch count.

### Tests (new)

- `backend/verify/src/lib.rs` `a_wrong_body_tie_is_an_lvs_mismatch`: use the minimal nfet stack from
  `a_wrong_reference_width…` (:829-836). The reference `[d, g, s, b]` with w = 2e-7, l = 1e-7 gives no `lvs.` rows.
  The reference `[d, g, s, s]` (body on the source net, which the layout does not draw) gives at least one `lvs.`
  row. Reuse that test's `lvs_rows` closure shape.
- `a_pmos_in_a_foreign_well_is_an_lvs_mismatch`: two minimal pfet stacks (`poly`, two `diff`, `psdm`), each in its
  own `nwell` rect 5 µm apart, so there are two well nets. Assert `device_count == Some(2)` first. Reference
  bulks `w1`, `w2` give no `lvs.` rows. Both bulks `w` (one well net) give at least one `lvs.` row. If the pfet
  marker needs a layer the minimal stack lacks, add the smallest shape that derives `pfet_01v8` (deck :206) and
  say so.
- `frontend/library/src/cellgen.rs` `a_bjt_reference_is_collector_first`:
  - parse bjt_mirror (`include_str!("../../../benchmarks/fixtures/bjt_mirror.spice")`, with `deck_models` as in
    `oppoint.rs:814`);
  - `reference(&nl, None, &[])`: the XQ1 card's `terminals == ["outn", "in", "VSS"]`;
  - draw XQ2 as `drawn_cards_carry_no_params_for_passives` (:1844-1860) does (with `DeviceKind::Pnp`) and assert
    that its drawn card's terminals are `["outp", "in", "VDD"]`.
- `benchmarks/tests/signoff_fixtures.rs` (existing `BASELINE`): bjt_mirror and bgr_core stay `LVS clean = true`.
  With `unverified()` no longer counting BJTs, their `lvs-coverage/` sum must be 0.
- Commands:
  - `cargo test -p verify --lib`
  - `cargo test -p library --lib cellgen`
  - `cargo test -p cells`
  - `cargo test -p library --test perf_postlayout`
  - `cargo test --release -p benchmark --test signoff_fixtures` (backgrounded)

### Acceptance

T3 = 0 on every sky130 local fixture except MOM capacitors (dac4: 16 units). Measure it with the bench LVS column:
bjt_mirror and bgr_core go from PARTIAL(2)/PARTIAL(9) to MATCH.

### Risks (report, do not work around)

- A marker that touches two polygons of one terminal layer is refused (`device.rs:240-249`). bgr_core's 3×3 array
  may hit this if a unit's `co` marker reaches a neighbour's ring that is a separate polygon. Then the BJT shows
  as an `lvs/` count mismatch. The fix is in the generator (CELL-09: abut or merge collector rings), not in the
  deck. Record the rows and stop.
- An NPN collector `ntap` on the n-well ring and the base `ptap` inside `pwell_iso` were not proven to bind before
  running. If bjt_mirror's XQ1 is refused, record the `lvs/` rows.
- With the swap fixed, a C/E mix-up anywhere else (oppoint is already C, B, E) shows up as an LVS mismatch. That
  is intended.
