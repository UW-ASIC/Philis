# Feedback-Loop Research: Richer Routing Feedback + Matching Cell/Placement DOF

Date: 2026-07-11. Scope: `run_block` iteration loop (cells → place → route → extract).
Problem: past ~N iterations the routing feedback repeats — the vocabulary is exhausted,
so identical signals reproduce near-identical layouts and the loop plateaus.

---

## 1. Current-state map: signal → consumer

| Signal (producer site) | Granularity | Consumer | Notes |
|---|---|---|---|
| `net_weights` (routing/lib.rs:275–311, detour ratio × parasitic boost) | per-net scalar | placement net weight overrides (model.rs:81) | Magnitude only — no *where* the detour is. **EMA prior is dead code**: orchestrator.rs:651 passes `prior: None`, so weights are memoryless. |
| `cell_inflation_x/y` (routing/lib.rs:313–347, wires-near-cell-center count) | per-cell scalar ×2 | placement virtual size (orchestrator.rs:544–552) | Crude: counts wire midpoints inside cell bbox; no edge/direction targeting beyond x/y split. |
| `net_order_priority` (routing/lib.rs:349–357) | per-net rank | next iteration's routing order (orchestrator.rs:656, `rcfg` RefCell) | Only cross-iteration routing state that persists. |
| `cell_hints` HighParasiticR/C, MatchedMismatchR/C (orchestrator.rs:660–717) | per-cell + reason | variant selection heuristic (orchestrator.rs:487–516) | `HintReason::Congested` (block.rs:83) is **declared and consumed but never emitted** — dead vocabulary. |
| `constraint_adjustments` (crosstalk violations, routing/lib.rs:590–602; device-layer DRC spacing, orchestrator.rs:754–773) | device-pair + gap nm | isolation constraint escalation (orchestrator.rs:555–575) | Generic, working channel: (name, name, gap) → placement push. Reusable for new pair signals. |
| `clean` / `overuse` (routing/lib.rs:1029–1035) | booleans / scalar sum | utilization halving on failure (orchestrator.rs:853–857); best-score | Overused **node locations** (`dhot.usage` vs cap) discarded. |
| `unrouted` (routing/lib.rs:1032–1035) | net names | none beyond `clean` | Cause lost: missing landing vs. no path vs. `max_len` pruning (engine/routing.rs:556–559) all collapse to a name. |
| `missing_pins` (engine/routing.rs:865/873, per-net count) | per-net count | folded into `unrouted` | Which **pin/cell** failed to claim a landing, and what blocked it, is known in `build_track_grid` and thrown away. |
| `landings` (PinLanding: pin, node, layer) | per-pin | geometry emission only (orchestrator.rs:1007) | Pin→node displacement = pin-access stress metric, computed (engine/routing.rs:1037–1051) and discarded. |
| `dhot.hist` (PathFinder history cost, engine/routing.rs:727–733) | per-track-node congestion memory | **discarded every iteration** — `RouteHot::new` per `run_routing_at` call | The densest congestion signal in the system dies at iteration boundary. |
| EOL/PRL repair sets (engine/routing.rs:1243, 1291) | blocked nodes + affected nets | one-shot in-loop repair only | Repeat offenders never reach placement/cell-gen. |
| per-net R/C, matched deltas (routing/lib.rs:165–239) | per-net / per-pair | budget boost, hints, parasitic_clean | Via count per net is computable (`Via.net`) but only total reported. |

### Consumer DOF today

- **Cell-gen** (`MosfetSpec::enumerate`, mosfet.rs:29–65): nf folding × pattern (Single/Cc1d/Cc2d) × dummies {1,2}, deduped by estimated dims, cap 16. Fixed: gate stub always on the −y side (mosfet.rs:144–153), S/D contact row at fixed cy, single row (multiplier not folded into rows), no guard-ring axis, no pin-side axis.
- **Placement**: net weights, per-cell inflation, isolation escalation, variant reshape move (placement.rs:1548–1566, 10% prob), per-iteration seed jitter. **Latent unused DOF**: (a) `PlaceMv::orient_changes` is committed (placement.rs:1618–1623) but **no proposer ever generates an orientation move**; (b) pin-aware HPWL (`pin_pos`, placement.rs:404–430; `pin_dx/pin_dy/variant_pins` in PlaceCold:300–316) exists in the engine but `build_cold` (model.rs:25–53) **never populates pin offsets** — cost is cell-center HPWL, so net weights pull centers, not pins.

### Why the loop repeats

Three compounding causes, all visible in code:
1. Feedback is memoryless (prior=None) and the layout→feedback map is deterministic, so a plateaued layout regenerates the same weights.
2. The signals are aggregates (scalars per net/cell); once each aggregate has pushed its consumer to its cap (weight_cap 5.0, inflation_cap 1.5), there is nothing new to say.
3. Consumers saturate: variant selection has only 3 heuristics over ≤16 variants; placement has no pin/orientation dimension for weights to act on.

---

## 2. Proposed new signals, consumers, and required new DOF

### S1. Persistent congestion memory (warm-start PathFinder `hist`)
- **Signal**: the full per-node `dhot.hist` array at end of detailed routing.
- **Consumer**: next iteration's routing (seed `RouteHot::hist` with `0.5 × prior_hist`, remapped by (x, y, layer) since die size may change). Same RefCell channel as `net_priority_overrides` (orchestrator.rs:419, 656).
- **New DOF needed**: none — pure routing config plumbing.
- **Also**: wire the existing EMA prior — pass last iteration's net weights as `prior` at orchestrator.rs:651 (store a `RefCell<HashMap<String,f64>>` beside `rcfg`). Two-line fix that un-deadens routing/lib.rs:306–311.
- **Effect**: routing itself explores different trees across iterations even when placement barely moves; resolved congestion relaxes instead of being recomputed identically. Directly attacks "vocabulary exhausted" at the source.

### S2. Per-pin landing report (pin-access feedback)
- **Signal**: extend `RoutingResult` with `landing_report: Vec<LandingIssue>` where
  `LandingIssue { net: u32, pin: (i32,i32), kind: Missed | Displaced { nm: i32 }, blocker: Option<(i32,i32)> }`.
  All inputs exist in `build_track_grid` (engine/routing.rs:916–927: the `else` branch knows the failed term; the claim closures know which foreign term/obstacle rejected candidates; displacement computed at 1037–1051).
- **Consumers**:
  - **Orchestrator → cell hints**: pin coords map back to (cell, pin) via the same `pin_pos` table the route closure built (orchestrator.rs:588–607). New `HintReason::PinAccessFailed { pin_name, displacement_nm }`.
  - **Cell-gen (new DOF)**: `MosfetSpec.gate_side: enum {Bottom, Top}` — flip the gate stub to +y (mosfet.rs:144–153 sign flip) so gate landings escape a crowded bottom edge; optionally `sd_row_bias` shifting the contact row `cy`. Doubles the variant space along an axis pin-access feedback can actually select on.
  - **Placement**: when two *different cells'* pins fight over the same landing area (blocker attribution), emit a `constraint_adjustments` entry for that device pair with gap = pitch — reuses the existing channel end-to-end, zero new placement code.
- **Effect**: today a landing failure is invisible until it becomes "net X unrouted", which triggers die-halving (util × 0.5, orchestrator.rs:854) — a sledgehammer. Per-pin attribution converts it into a 1-pair nudge or a 1-cell variant flip.

### S3. Overuse hotspot list + contending nets
- **Signal**: after detailed routing, for each node with `usage > cap`, emit
  `hotspots: Vec<(x, y, layer, overflow, Vec<net>)>` (nets found by scanning `hot.trees`; analog blocks are small, O(nets × tree) is fine). Aggregate to gcell resolution to keep it short.
- **Consumers**:
  - **Placement (new DOF)**: hotspot-targeted inflation — inflate only cells whose *edge nearest the hotspot* faces it, in the hotspot's direction (replaces the wire-midpoint-in-bbox heuristic at routing/lib.rs:313–347). Alternatively a soft "push away from point" term: `PlaceCold.pushes` already models pair pushes; add a `point_pushes: Vec<(cell, x, y, r)>` variant — small addition to `soft_terms`.
  - **Net weights**: boost precisely the contending nets instead of all detoured nets.
  - **Routing**: contending-net pairs → order the loser earlier next iteration (extends existing `net_priority_overrides`).
  - **Cell hints**: a cell adjacent to a persistent hotspot finally gets the never-emitted `HintReason::Congested`, activating the aspect-ratio variant selector already written at orchestrator.rs:505–513.
- **Effect**: converts the two scalar inflation vectors into a located, directional signal; gives placement moves a target.

### S4. Unrouted-cause taxonomy
- **Signal**: `unrouted: Vec<(String, UnroutedCause)>` with `MissingLanding` (from `missing_pins`), `MaxLenPruned` (route_net failed with `max_len` set but succeeds without — one extra probe call in the failure path), `NoPath` (else).
- **Consumers**: `MissingLanding` → S2 path; `MaxLenPruned` → strong net weight + `HighParasiticR` cell hint (widen variant) instead of util halving; `NoPath` → util halving stays as last resort. Gate the util × 0.5 fallback (orchestrator.rs:853–857) on `NoPath` only.
- **Effect**: stops the most destructive feedback (die halving) from firing on failures it cannot fix.

### S5. Per-net via count / layer occupancy
- **Signal**: `net_vias: Vec<u32>`, `net_layer_mask: Vec<u8>` (both already computed transiently in `reconcile_geometry`, routing/lib.rs:497–503).
- **Consumers**: matched-pair via-count asymmetry → `MatchedMismatchR` hint precision; high via count on an R-budgeted net → prefer routing that net on fewer layers (per-net via_cost override — small router extension).
- **Effect**: modest; cheap because the data exists.

### P1. Placement: pin offsets + orientation moves (DOF unlock, consumes S2/S3 and existing net weights)
- Populate `PlaceCold.pin_dx/pin_dy/pin_net/cell_pins` and `variant_pins` in `build_cold` from `CellOutput` pin bboxes (orchestrator already computes exactly these centers in the route closure, orchestrator.rs:594–606 — hoist per-variant pin centroids into `PlacementConfig`, e.g. `variant_pin_offsets: Vec<Vec<Vec<(String, i32, i32)>>>` keyed by net).
- Add an orientation-flip move to `SymSaCore::propose` (~10% for non-symmetry cells): push `(c, FN, cur)` into `mv.orient_changes`; delta cost already handled by pin-aware `cost_at_pins` once pins exist; commit already applies it.
- **Effect**: a weighted net can now be shortened by flipping a cell so its pin faces the net — a whole move class the router's feedback can finally pay into. Also makes `cell_inflation` semantically honest (pin side matters).

### P2. Cell-gen: multiplier row-folding + guard-ring variant axes
- `m`-row folding (2-row aspect variants) gives real aspect-ratio choices for `Congested` hints (today aspect only changes via nf, which also changes finger W). Larger effort (draw code for row stacking + inter-row strapping).
- Guard-ring on/off as a variant axis consumes isolation escalations more cheaply than distance. Larger effort; defer.

### Revisit note: met2 landing reservations
The rejected "met2 node reservations around landing pins" was rejected under 2 routing layers. `n_layers` is now deck-derived up to 6 (orchestrator.rs:462–469) and 5 in practice. With ≥4 layers, reserving a 1-node met2 halo around landings costs a small fraction of capacity and would eliminate a class of landing-stub shorts (the L-shaped met2 stubs at orchestrator.rs:1019–1049 dodge foreign wires heuristically). Worth a gated retry: `if n_layers >= 4`.

---

## 3. Literature cross-check (feedback vocabulary used elsewhere)

- **PathFinder** (McMurchie & Ebeling, FPGA'95): the canonical feedback *is* the persistent per-node history cost — congestion memory across iterations, never reset while the problem instance lives. S1 is just applying the original algorithm's contract across the outer loop.
- **MAGICAL** (Xu et al., ICCAD'19; Chen et al., TCAD'21): placement consumes *probabilistic routing congestion maps* and *pin-density maps* before routing runs; routing feeds back symmetry-aware net matching failures. Their placer is pin-aware (pin positions in the analytical objective) — matching P1. Post-route, failed nets feed a rip-up region that becomes a placement density penalty — matching S3.
- **ALIGN** (Kunal et al., DAC'19; Dhar et al., ICCAD'20): primitive generators emit *multiple variants per cell including aspect ratio and pin-side alternatives*, and the placer selects among them under router-derived congestion estimates — precisely the S2/P2 axis (variants differing in pin geometry, not just bbox). Their router reports per-net DRC hotspots back as soft placement blockages.
- **Modern digital flows** (RePlAce/OpenROAD, TritonRoute): global-route overflow per gcell → per-cell inflation *localized to overflowed gcells* (S3, not whole-cell scalars); detailed router publishes *pin-access analysis* (per-pin access-point counts, failed access = placer must move/flip the cell) — S2/P1; timing-driven loops weight nets by slack criticality — the analog analogue (parasitic-over-budget ratio) already exists here and is the healthiest part of the current vocabulary.

Common thread: the mature flows exchange **located** signals (maps, per-pin, per-gcell) and keep **memory** across iterations; this codebase currently exchanges per-object scalars and forgets everything except net order.

---

## 4. Prioritized top-3 with implementation sketches

Ranking metric: (expected utilization/DRC/convergence gain) ÷ (implementation size), weighted by how much existing plumbing is reused.

### #1 — S1: persist congestion memory + wire the dead EMA prior (effort: ~½ day)
- `backend/routing/src/lib.rs`: add `pub prior_hist: Vec<(i32, i32, u8, f32)>` (pos-keyed, die-size independent) to `RoutingConfig` (~line 30). In `run_routing_at` after `RouteHot::new` (~line 872): `for &(x,y,l,h) in &cfg.prior_hist { dhot.hist[grid.nearest(x,y,l) as usize] += 0.5*h; }`. At end, export nonzero hist into `RoutingResult` (new field `hist_out`).
- `frontend/core/src/orchestrator.rs` extract closure (~line 655): `rcfg.borrow_mut().prior_hist = routed.hist_out.clone();`
- Same file, line 648–651: add `let prev_weights = RefCell::new(HashMap::new());` next to `chosen_variant` (line 475), pass `Some(&prev_weights.borrow())` as `prior`, refresh it after extraction.
- Risk: low; both mechanisms decay, both are within the router's own convergence theory.

### #2 — S2 + S4: per-pin landing report, unrouted causes, pad-conflict pair adjustments (effort: ~1–2 days)
- `backend/engine/src/routing.rs`: in `build_track_grid` pass-1 failure branch (~line 925), record `(t.net, t.x, t.y)` misses into a new `Vec<LandingIssue>` return (widen the tuple or introduce a small struct return — the 5-tuple is already at the limit). Displacement is `(nx-t.x, ny-t.y)` at line 922. Blocker attribution: in the `clear` closure capture the first rejecting foreign term.
- `backend/routing/src/lib.rs`: surface as `RoutingResult.landing_report`; add `UnroutedCause` by re-probing `route_net` with `max_len: None` for failed nets (one call per failure, failures are rare).
- `backend/engine/src/block.rs`: add `HintReason::PinAccessFailed { pin: String, displacement_nm: i32 }` (~line 83).
- `frontend/core/src/orchestrator.rs`: map pin coords → (cell, pin_name) using the `pin_pos`/`stacks` it already builds (lines 588–607; keep a coord→(cell,pin) map alongside). Emit cell hints; for cross-cell blockers emit `constraint_adjustments` entries (existing channel, lines 555–575 consume them unchanged). Gate util-halving (line 854) on `NoPath`.
- `backend/cells/src/generators/mosfet.rs`: add `gate_side: Side` to `MosfetSpec` (~line 10), flip stub sign in `draw` (lines 144–153: `-(poly_ext+stub)` → `finger_w+poly_ext..+stub`), include both sides in `enumerate` when the cell has >4 variants of headroom under `MAX_VARIANTS`. Variant selection for `PinAccessFailed`: prefer opposite `gate_side` / larger `dummies_per_edge`.

### #3 — P1 + S3: pin-aware placement HPWL, orientation moves, hotspot-targeted inflation (effort: ~2–3 days)
- `frontend/core/src/orchestrator.rs` cells closure (~line 518): alongside `variant_sizes`, compute `variant_pin_offsets: Vec<Vec<Vec<(u32 /*net row later*/, i32, i32)>>>` from `pin_accesses` centroids per variant (same helper, line 178).
- `backend/placement/src/lib.rs`: new `PlacementConfig.variant_pin_offsets` field (~line 80); thread into `build_cold`.
- `backend/placement/src/model.rs` `build_cold` (~line 90, after net CSR is built so net rows exist): fill `cold.pin_dx/pin_dy/pin_net/cell_pins` (variant 0) and `cold.variant_pins`. Engine cost (`cost_at_pins`, `pin_pos`) already handles the rest.
- `backend/engine/src/placement.rs` `SymSaCore::propose` (~line 1551): add a third roll branch (e.g. `roll < 0.18`) emitting `mv.orient_changes.push((c, flip(hot.orient[ci]), hot.orient[ci]))` for non-symmetry cells; verify `SaCost::delta` routes through pin-aware HPWL when `orient_changes` is nonempty (extend `PlaceMv` handling in delta if it currently only reads idx/nx/ny).
- Hotspot inflation (S3): in `extract_feedback_with_prior` (routing/lib.rs:313), replace the wires-near-center count with: build per-gcell overflow from `result` wires vs. an overuse export (add `overused_nodes: Vec<(i32,i32,u8)>` to `RoutingResult` — computed where `overuse` is summed, line 1029), inflate only cells within one gcell of a hotspot, in the hotspot's layer direction. Keeps signatures; feedback struct unchanged.
- This is the largest item but unlocks the consumer side for everything above; without it, sharper signals still land on a center-based, orientation-blind cost.

### Deliberately deferred
- P2 row-folding & guard-ring variants (draw-code heavy; do after #2 proves variant-axis feedback works).
- Per-net via/layer feedback (S5): cheap but low leverage until #1–#3 land.
- met2 landing reservations: retry behind `n_layers >= 4` after #2, since landing_report will measure exactly the problem it solves.

---

## 5. References

- L. McMurchie, C. Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router for FPGAs," FPGA 1995.
- B. Xu et al., "MAGICAL: Toward Fully Automated Analog IC Layout Leveraging Human and Machine Intelligence," ICCAD 2019; H. Chen et al., "MAGICAL 1.0," TCAD 2021 (congestion/pin-density-aware placement, symmetry-aware routing feedback).
- K. Kunal et al., "ALIGN: Open-Source Analog Layout Automation from the Ground Up," DAC 2019; T. Dhar et al., "ALIGN: A System for Automating Analog Layout," ICCAD 2020 (multi-variant primitive generation incl. pin-side/aspect, congestion-driven variant selection).
- C.-K. Cheng et al., "RePlAce: Advancing Solution Quality and Routability Validation in Global Placement," TCAD 2019 (localized congestion-driven cell inflation).
- OpenROAD TritonRoute / pin-access analysis; A. B. Kahng et al., ISPD/ICCAD 2018–2021 (per-pin access-point feedback to detailed placement).
- Codebase sites audited: `backend/engine/src/block.rs`, `backend/engine/src/routing.rs`, `backend/engine/src/placement.rs`, `backend/routing/src/lib.rs`, `backend/placement/src/{lib,model}.rs`, `backend/cells/src/generators/mosfet.rs`, `frontend/core/src/orchestrator.rs`.
