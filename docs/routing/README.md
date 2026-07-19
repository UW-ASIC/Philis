# Routing

This directory is the implementation reference for the `pnr-routing` crate in
[`backend/routing`](../../backend/routing/). It describes the current code,
including the `pnr-engine` routing types re-exported by the façade. It does not
claim support for routing features that exist only in constraint schemas or
research notes.

The crate converts a placed hypergraph and optional physical pin accesses into
wire/via planning geometry. Its main flow is:

1. select routable nets and construct physical terminal points;
2. assign net weights and a deterministic route order;
3. run negotiated-congestion PathFinder on a 2-D gcell graph;
4. expand global trees into detailed-route corridors;
5. build a multilayer preferred-direction track graph and claim pin landings;
6. run capacity-1 detailed PathFinder;
7. optionally mirror a missing symmetry-mate route;
8. optionally attempt one EOL repair and one wide-net PRL repair;
9. extract wires/vias, judge geometry contracts, and return routing feedback
   state.

The router returns planning geometry and diagnostics even when nets are
unrouted, pin access fails, tracks remain overused, or hard contracts are
violated. Callers must inspect `RoutingReport`; a returned `RoutingResult` is
not itself proof of a clean route.

## Reading guide

- [Public API and configuration](api.md) inventories every public item,
  default, array contract, output field, and debug artifact.
- [Model and algorithms](model-and-algorithms.md) documents net selection,
  graphs, PathFinder costs, pin landing, route mirroring, repair passes, and
  geometry extraction.
- [Constraints and feedback](constraints-and-feedback.md) gives the exhaustive
  constraint coverage matrix, exact contract tests, R/C equations, and every
  feedback calculation.
- [Operational behavior and limitations](behavior-and-limitations.md) covers
  success criteria, failure classification, report boundaries, and known
  implementation limitations.

## Minimal use

```rust
use pnr_routing::{run_routing, RoutingConfig};

let routed = run_routing(
    &graph,
    &placed.placement,
    &constraints,
    &RoutingConfig::default(),
);

let clean = routed.report.unrouted.is_empty()
    && routed.report.overuse == 0
    && routed.report.validation.hard_violations.is_empty();
```

`run_routing` synthesizes one terminal per hypergraph pin from the placed cell
rectangle. Production integration normally calls `run_routing_at` with actual
generated-cell access points and later materializes the returned
`PinLanding` stubs into final GDS geometry.

## Units and indexing

- Coordinates, pitch, width, spacing, wirelength, and displacement are in nm.
- Wire minimum area is in nm².
- Constraint fields ending in `_um` are converted from µm by multiplying by
  1,000.
- Resistance is in ohms and capacitance outputs are in fF.
- `WireParasiticParams::area_cap` is interpreted as aF/µm² and `fringe_cap` as
  aF/µm; accumulated capacitance is divided by 1,000 to produce fF.
- Result net indices address `RoutingResult::net_names`, not the original
  hypergraph net array. Empty and obstacle-only nets are omitted and retained
  nets are densely remapped.
- Cell indices remain in `BipartiteHypergraph::cells` order.
- Routing layer `0` is the first PDK routing conductor, layer `1` the second,
  and so on. Even indices route horizontally; odd indices route vertically.
- `Via::layer` is the lower conductor index of the transition.

## Source map

| Concern | Source of truth |
|---|---|
| Public façade, orchestration, feedback, ledgers, debug files | [`backend/routing/src/lib.rs`](../../backend/routing/src/lib.rs) |
| Graphs, PathFinder, landing claims, geometry and repair scans | [`backend/engine/src/routing.rs`](../../backend/engine/src/routing.rs) |
| Generic stage driver and telemetry | [`backend/engine/src/traits.rs`](../../backend/engine/src/traits.rs) |
| Placement input schema | [`backend/placement/src/lib.rs`](../../backend/placement/src/lib.rs) |
| Constraint and parasitic parameter schemas | [`backend/constraints/src`](../../backend/constraints/src/) |
| Production pin extraction, feedback loop, and GDS materialization | [`backend/src/flow.rs`](../../backend/src/flow.rs) |
