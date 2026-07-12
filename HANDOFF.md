# HANDOFF — PNR_UPGRADED

Branch `experimental`, large uncommitted working tree. **Working tree is the only good state — never stash/reset; HEAD alone does not compile.**

Updated 2026-07-11.

## Current state

All 7 align bench circuits: **DRC 0 blocking, LVS MATCH, 0 unrouted, 0 overuse, contracts ≈449/449**. 97/97 workspace tests.

| circuit | DRC | LVS | util |
|---|---|---|---|
| block_spacing_bug | 0 | MATCH | ~24% |
| buffer | 0 | MATCH | ~16% |
| cascode_current_mirror_ota | 0 | MATCH | ~10-11% |
| five_transistor_ota | 0 | MATCH | ~10-11% |
| five_transistor_ota_Bulk | 0 | MATCH | ~12-14% |
| five_transistor_ota_high_frequency | 0 | MATCH | ~5-8% |
| telescopic_ota | 0 | MATCH | ~12-13% |

Verify loop:
```bash
cargo test --workspace
cargo build --release -p pnr-benchmark
./target/release/bench align buffer five_transistor_ota cascode_current_mirror_ota   # smoke
./target/release/bench all                                                            # full
```
Debug env vars: `PNR_DEBUG_LVS`, `PNR_DEBUG_LVS_RAW` (pre-reduction devices), `PNR_DEBUG_LANDINGS`, `PNR_DEBUG_NOTCH`.

## What landed recently (2026-07-10/11) — do not undo

- **LVS series merge deleted** (`verify/src/lvs/extract.rs reduce_netlist`): generator never builds series stacks; ref side never series-reduced; it fused devices through shared ports.
- **Single-hpin nets routed** (`backend/routing/src/lib.rs build_nets`): a 1-hpin net expands to many finger pads needing strapping; skipping them caused finger-island LVS mismatches.
- **terminal_only landing nodes** (`backend/engine/src/routing.rs TrackGrid`): landing claims punch met1 holes in blocked cell area; lateral met1 between them crosses foreign pads → via-only access.
- **Grid snap at builder boundary** (`frontend/substrate3 CellBuilder::rect/pin/polygon` via deck `snap()`/`grid()`): odd finger_w divisions leaked off-grid coords.
- **5-metal routing stack, deck-derived**: both PDKs (`pdks/sky130.json`, `pdks/generic_finfet.json`) carry met1–met5 + via1–via4. Orchestrator counts met(N+1)+viaN pairs → `n_layers` (TrackGrid clamp 2..6). Layer parity = direction (even horizontal). `Via.layer` field selects cut. `merge_block_geometry` uses `mets[]`/`cuts[]` tables.
- **Best-iteration score** (`backend/engine/src/block.rs`): LVS > DRC+hard_violations > clean > parasitic > **die area**. Hard contract violations gate convergence too.
- **Per-iteration SA seed** (orchestrator place closure): fixed seed made the engine loop cycle identical states.
- **Plateau stop** (`stall_iters=12`, only once best is fully clean): 10× runtime cut on converged circuits.
- **DTI compaction policy** (`backend/engine/src/placement.rs compact_placement`): weld near-abutting pairs only when iso allows abutment and no transitive cross-cluster pair falls under its required gap; far pairs get d_dti+50nm floor.
- **Same-net notch fill** (`gdsverify::same_shape_gap_fills`, called at end of `merge_block_geometry`): sub-min gaps within one merged shape (pad-row corner slivers) filled with metal inflated by min/2.
- **EMA feedback prior wired** (was passed `None` — dead code).
- **DOD perf pass (2026-07-11, result-neutral — bitwise-identical outputs, 2× wall on smoke: 97.8s→49s).** Callgrind-driven. Key: lazy cell→constraint CSRs in `HardGaps`/`SaCost` (OnceCell per stage instance) replace per-move full scans of pushes/dti/pulls/cc/aligns — deleted `touches()` (was 10% Ir). Also: `propose` group-move `collect()` → buffer reuse; `[u32;4]` in cc_pattern_cost; `Dij` old-node stamp array replaces per-neighbor `binary_search` (route_net destructures Dij for split borrows); `tree_nodes_into` + `PfScratch.nodes_buf` (propose/commit alloc-free); `TrackGrid.region_of` table (region() = 1 load); `GeometryStore.layer_index` per-layer buckets (`polys_on_layer` O(k)); `run_drc_no_density` for in-loop signoff; `reference_netlist` hoisted out of loop; LVS connectivity union-find now sweep-line `node_candidate_pairs` (was all-pairs O(m²), GPU path no longer materializes m²/2 index pairs — union order differs but partition + net-id assignment identical); same-bucket skip in `OverlapDensity::delta` (sort keeps candidate order → identical f64 sums); merge_rows single-row fast path. CSR rows sorted-unique by construction — merged rows reproduce flat-scan order exactly (f64 accumulation bitwise stable). Remaining hot (five_transistor profile): `new_gap` 19%, `new_pos` 13% — mv.idx scans, cheap per call; further Ir cuts did NOT move wall time (cache-bound). Next real lever is algorithmic (AsfTree seed / plateau), not micro-opts.

Rejected approaches (do not retry blindly): met2 node reservations near landings (starved 2-layer capacity — worth REVISITING now that n_layers ≥ 4); via-at-node met2 landings; global compaction slack growth (WL 2.4×); single met1 term per pin (breaks LVS — fingers are strapped by pad rows).

See `docs/feedback-loop-research.md` for the routing-feedback roadmap (persist PathFinder hist, per-pin landing report, pin-aware placement, orientation moves, hotspot inflation).

---

## Roadmap: backend/cells — comprehensive device support

`backend/cells/src/generators/` today: mosfet (mature — 504 lines: nf folding × interdigitation styles × dummies, unitization, LOD moat, WPE halo), resistor (348), capacitor (223), bjt (197), inductor (180 — **single variant, rectangular spiral approximation**), diode (144), guard_ring (148).

Gaps to close, roughly in value order:

1. **Variant richness parity.** Only `MosfetSpec::enumerate` produces a real variant space. Resistor/capacitor/bjt/diode/inductor return 1–few specs → the engine's variant-selection feedback (cell_hints) has nothing to choose from. Each spec needs an aspect/folding axis at minimum: resistor serpentine fold count, capacitor rows×cols unit tiling, BJT emitter-finger count, inductor turns/width/spacing.
2. **MOS flavors.** `device_recognition.mos` in decks supports flavors/well_layer/device_class, but generators emit generic nmos/pmos. Missing: HV/LV/native flavors (sky130 hvtp/lvtn markers exist in the deck layer table, unused by generators), DMOS/LDMOS (device_class plumbing exists in LVS compare — `class_hash` — but no generator emits marker layers).
3. **Passives correctness.** MiM/MoM cap distinction (deck `cap_rules` has top/bottom/marker layers — generator draws parallel plates only); poly vs diffusion vs high-R resistor (rpm marker in sky130 table, unused); varactor (Ncap/Pcap DeviceType exists in enum, check generator coverage).
4. **Matched-structure generation.** Common-centroid is only intra-cell (multi-device ABBA in mosfet.rs `finger_sequence`). Cross-cell matched pairs (mn2/mn3 in every OTA) are separate cells the placer must arrange — the single biggest area lever is generating an **interdigitated merged cell** for matched pairs (one diffusion row, shared dummies, internal strapping). Hypergraph already supports `grouped_devices`.
5. **Per-finger pin metadata.** Cells expose per-finger li pads; routing straps them. A generator-drawn met1 strap rail per S/D net (top/bottom rail rows) would delete most strapping wirelength and the pad-row hazards that motivated terminal_only. Requires moving S/D pads to two rows (D above, S below) instead of one row at cy.
6. **Guard ring as variant axis**, not just constraint response — `dummies_per_edge`-style `ring: bool` on specs so the engine can trade area vs isolation.

## Roadmap: backend/placement

`backend/placement/src/{lib,model}.rs` (driver + cold-model build) over `backend/engine/src/placement.rs` (SA core, compaction, checks — ~2600 lines).

1. **AsfTree seed.** `AsfTree` with contour `pack()` exists (~engine/placement.rs:1786) and is **dead code**. Wire as SA initial state (ASF-B*-tree packing = 1–2.5% dead space in literature). Biggest packing lever.
2. **Pin-aware HPWL.** `pin_dx/pin_dy/variant_pins` fields exist in the cold model but `build_cold` never populates them — net weights pull cell centers, not pins. Orchestrator already computes pin positions per variant. (Research doc §3.)
3. **Orientation moves.** `PlaceMv::orient_changes` is committed by the SA core but no move generator proposes it. Add an orient-flip branch to `SymSaCore::propose`; mirror symmetry support already landed in the B*-tree work (commit 56b8a50).
4. **Compaction completeness.** The x-then-y longest-path sweep only constrains sweep-adjacent clusters — non-adjacent pairs can end under their required gap (observed: buffer iso pairs before seed diversity masked it). Either add all-pairs edges where other-axis projection doesn't clear, or a post-compaction legality repair pass.
5. **Hotspot-targeted inflation.** Replace wires-near-cell-center inflation with per-gcell overuse locations exported from routing (research doc #3).
6. **Region/row constraints.** No notion of placement rows or P/N region bands; every OTA wants a P row + N row(s). A row-assignment prior (even soft) would cut SA search space dramatically.
7. **DTI/iso joint reasoning.** Current weld/floor policy is per-pair greedy at compaction time. A small pair-state ILP (abut vs far per DTI pair, consistent with iso) would pick the globally cheaper branch set — today buffer pays d_dti floors that a different weld set avoids.

## Roadmap: backend/routing

`backend/routing/src/lib.rs` (driver, nets/feedback) over `backend/engine/src/routing.rs` (grids, A*/pathfinder, geometry extraction).

1. **Persist congestion memory.** `RouteHot::new` per iteration discards PathFinder `hist`; carry decayed hist across engine iterations via the existing `rcfg` RefCell channel (like `net_priority_overrides`). ~15 lines, top item in research doc.
2. **Per-pin landing report.** `build_track_grid` knows which pin failed to claim, its displacement, and the blocker — all discarded (only a per-net `missing` count survives). Surface as `RoutingResult.landing_report`; emit `HintReason::PinAccessFailed`; gate the destructive util-halving on true NoPath instead of landing failures.
3. **Track-pitch layers.** One pitch (430) for all 5 layers. Upper metals can run coarser pitch/wider wires (matches real stacks, met4/met5 pex already reflect lower R); per-layer pitch in TrackGrid is the enabling change for power routing and long straps.
4. **Post-route DRC repair loop.** The same-net notch fill is the first instance of "repair by construction". Generalize: run in-loop DRC, map violations to nodes, rip-up + block + re-route affected nets (EOL repair at routing/lib.rs ~line 890 already does exactly this pattern for EOL — extend to spacing/notch classes).
5. **Net topology quality.** Trees come from sequential A* with pathfinder negotiation; no Steiner refinement, no trunk-style strapping for many-pad single-hpin nets (32-finger nets route as chains of stubs). A trunk+taps template for pad-row nets would cut WL and capacitance materially.
6. **Symmetric-net routing** exists (`mirror_symmetric_routes`) but is post-hoc mirroring; failures are silent. Report mismatches into `hard_violations` so score sees them.
7. **Revisit met2 landing reservations** behind `n_layers >= 4` — the 2-layer starvation objection no longer holds; measure with the landing report from item 2.

## Known residuals

- Constraint satisfaction occasionally shows 1–2 boundary-rounding violations (0.0–0.1µm depth) depending on run; root cause is compaction adjacency (placement roadmap item 4).
- `cascode_current_mirror_ota` never hits the convergence threshold (max_weight stays ≥1.2) → runs full budget (~90s). Plateau stop doesn't trigger because area keeps improving slightly.
- Magical suite discovers `Reference_buffer_core` twice (benchmark/src fixtures bug).
- VCO_type2_65 constraint-emission saturation (see docs/ constraints knowledge base) — untouched.
