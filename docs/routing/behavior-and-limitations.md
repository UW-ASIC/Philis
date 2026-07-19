# Routing operational behavior and limitations

## What counts as a clean route

At minimum, check all route completion and capacity signals:

```rust
let route_complete = result.report.unrouted.is_empty()
    && result.report.no_path.is_empty()
    && result.report.missing_pin_count == 0
    && result.report.overuse == 0;

let contracts_closed = result.report.validation.hard_violations.is_empty()
    && result.report.validation.emitted == 0
    && result.report.validation.consumed == 0;
```

These conditions still do not replace DRC, LVS, or extracted PEX. Optional EOL
and PRL passes are one-shot heuristics, and many PDK rules are outside the
router’s graph model. The integrated flow materializes landing geometry and
runs signoff on the assembled layout.

`RoutingFeedback::clean` is intentionally narrower: it checks only
`unrouted.is_empty()` and `overuse == 0`.

## Failure classification

- A net with an empty final tree or any missing physical landing appears in
  `unrouted`.
- An empty tree with no landing failure also appears in `no_path`.
- Every failed terminal has a `PinAccessFailure`; `missing_pin_count` is the
  number of those records.
- Shared capacity appears in `overuse`, including conflicts introduced by
  symmetric mirroring or left by one-shot repair.
- Contract problems appear in lifecycle validation, but unresolved
  `Emitted`/`Consumed` hard contracts are not `hard_violations`.
- The routing functions return all of these states normally; they have no
  `Result` error channel.

## Structural input hazards

- Placement cell-parallel vectors are indexed without validating their lengths.
- A present `pin_pos` outer vector is indexed directly by cell.
- Invalid geometry/cost values and integer overflow are not comprehensively
  checked.
- `weight_cap < 1`, `inflation_cap < 1`, or NaN caps can panic feedback
  extraction through invalid `clamp` bounds.
- A nonpositive parasitic `max_r` can make the search-time route budget
  impossible even though post-route contract code treats nonpositive budget
  fields as disabled.
- Very large dies can force an actual graph pitch larger than configured while
  some façade calculations and `RoutingResult::track_pitch` continue using the
  configured value.
- An excessively large requested pitch can leave the graph’s forced second
  track centre outside a smaller die; meaningful callers keep pitch well below
  both die dimensions.

## Routing-model limitations

- The graph is a uniform rectangular lattice with one capacity per node. It
  does not model per-edge capacity, track-specific width, nonuniform pitch,
  preferred-layer constraints from `NetClassification`, or polygonal
  blockages.
- Cell footprints block only met1. Upper layers remain open over cells.
- Accepted multi-candidate landing refinement leaves superseded met1 landing
  holes open as unreserved terminal-only via nodes.
- `extra_obstacles` affect landing-corridor selection only; they do not become
  general route blockages.
- Even/odd layer index hard-codes horizontal/vertical preferred direction.
- Net weights mainly order nets and scale telemetry; they do not directly
  scale Dijkstra cost.
- Clean nonempty trees are not revisited solely for wirelength improvement.
- `Telemetry::best_cost` does not retain a corresponding route snapshot, and
  empty initial trees make the scalar routing best cost especially unsuitable
  for candidate selection.
- The route-search parasitic cap uses hard-coded met1 sheet resistance and a
  congestion-weighted Dijkstra cost, not the supplied R/C model or pure length.
  It bounds each target connection search, not total multi-terminal tree
  length.
- Symmetric route handling is missing-mate mirroring after independent
  routing, not coupled symmetric or differential routing.
- Mirrored nodes are checked for usage capacity but not `allowed`, reservation,
  or landing-hole legality.
- EOL and wide-net PRL repair are order-dependent, one-shot, and not rechecked.
  Temporary usage claims are penalties rather than absolute blocks.
- Crosstalk is audited after detailed epochs but does not affect resource cost
  or blockages.
- There is no shielding insertion, antenna repair, current-flow orientation,
  Kelvin/star routing, redundant-via planning, electromigration sizing, or
  coupling-aware routing in this crate.

## Geometry limitations

- `wires` and `vias` are track-tree geometry only. Pin pads, landing stubs,
  landing vias, cell geometry, and guard-ring geometry are absent.
- `wirelength_nm` excludes all landing stubs and vias.
- `via_count` excludes landing vias.
- Minimum-area extension happens after routing and can extend outside the die
  or into spacing/blockage violations.
- Wire segments are not globally merged or deduplicated; separate tree
  branches can emit duplicate/shared runs and inflate wirelength/R/C estimates.
- Via deduplication ignores net identity at a shared site.
- EOL and PRL checks use centre coordinates and do not consistently include
  wire width or full rectangle geometry.
- `net_pair_clearance` uses same-layer node centres, not wire-edge clearance.

## Contract/report timing limitations

- Detailed `Telemetry` ends before symmetry mirroring and repair passes.
- Crosstalk contracts are not reconciled after those post-stage mutations.
- Geometry contracts are reconciled after final wire extraction, but a
  parasitic budget on an unrouted compact net can pass with estimated zero R/C.
- Differential R/C limits remain unjudged without `wire_params`; length and
  layer set can still close the contract.
- Straight-net spread ignores pin-to-landing stubs because they are not in the
  wire list.
- `RoutingResult` does not expose the owned `ConstraintContract` vector; only
  aggregate validation and a metric-free text summary are returned.
- A later `Satisfied` lifecycle transition does not clear an older stored
  violation metric in the underlying contract type. Routing does not expose
  that metric vector, but lifecycle status—not a stale metric—is authoritative.
- The report display labels only `Emitted` contracts as “unconsumed” and does
  not show the separate `Consumed` count.

## Feedback limitations

- Feedback actual length and R/C exclude landing geometry and can double-count
  duplicate branch wires.
- The convenience feedback function has no R/C parameters and therefore
  blindly applies a 2× boost to every supplied parasitic-budget net.
- `constraint_adjustments` is currently always empty; use
  `RoutingResult::crosstalk_violations` for the implemented crosstalk feedback.
- The integrated flow currently passes those net-name crosstalk tuples into a
  device-name adjustment channel, so they normally fail to resolve into new
  placement isolation constraints.
- Directional pressure is inferred from historical nodes and landing
  displacement, not a continuous congestion map.
- Hotspots include decayed history carried from previous iterations.
- `spread_required` deliberately excludes pin-access-only failures; callers
  need a local variant/access response for those failures.
- Matched-delta extraction recognizes only `G`, `D`, and `S` pin names and
  depends entirely on the caller’s mapping closure.

## Debugging checklist

1. Inspect `unrouted`, `no_path`, `missing_pin_count`, and
   `landing_failures` separately to distinguish access failure from no path.
2. Check `overuse` after post-route mirroring/repair; the detailed trace’s
   final overflow predates those operations.
3. Compare physical pin coordinates with `landings` and confirm that the final
   geometry consumer emitted every required stub/via.
4. Validate lifecycle counts, not only `hard_violations`; unresolved contracts
   can remain `Emitted` or `Consumed`.
5. When history feedback seems absent, verify exact graph node-count equality
   and stable pitch/layer settings across iterations.
6. When a net is missing from `net_names`, check whether its physical points
   deduplicated to fewer than two; its points become obstacles only.
7. When a route violates a parasitic budget unexpectedly, compare the
   hard-coded search cap with the post-route `wire_params` model—they are
   intentionally separate calculations.
8. Run assembled-layout DRC/LVS/PEX; `routes.txt` omits width, landings, and
   vias and is not a signoff representation.

Set `PNR_DEBUG_LANDINGS` in the environment to make `run_routing_at` print
terminal points, obstacle points, and successful landing assignments to
stderr. This is an implementation diagnostic and has no structured output.

## Tests in the crate

Routing unit tests cover:

- a clean routed OTA with in-die wires;
- crosstalk contract consumption;
- differential and parasitic closure with R/C parameters;
- deterministic repeated runs;
- debug artifact creation;
- a numerical R/C example;
- feedback R/C population.

The engine separately checks that a track grid exposes the entire declared
layer stack. These regressions do not substitute for PDK signoff or malformed
input validation.
