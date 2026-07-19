# Routing model and algorithms

## Net construction

The router walks hypergraph nets in original order.

1. A net with no hypergraph pins is skipped.
2. Each hypergraph pin expands to its supplied physical points, or to one
   synthesized point when `pin_pos` is absent or its inner pin row is missing.
   An explicitly empty physical-access list contributes no point.
3. Points are sorted and deduplicated.
4. Fewer than two distinct points makes the points landing obstacles only; the
   net is omitted from route/result indexing. Those obstacle points affect
   candidate clearance only when protected-pin mode is enabled.
5. Two or more points creates a compact routed net.

This means a one-pin hypergraph net can be routable when that pin expands to
multiple finger pads. Conversely, a multi-pin net can become obstacle-only if
all physical locations collapse to one point.

### Net weights

| Classification | Weight |
|---|---:|
| Canonical supply/ground name | `0.5` |
| `NetClass::Sensitive` | `3.0` |
| `NetClass::Clock` | `2.0` |
| `Supply`, `Ground`, or `Substrate` class | `0.5` |
| `Signal` or ordinary net | existing name-derived value, normally `1.0` |

The first exact-name `NetClassification` match wins. Other classification
fields are not used by this router.

### Route order

Nets are sorted by these keys, in order:

1. nets named by a `StraightNet` first;
2. larger exact-name `net_priority_overrides` first;
3. larger net weight first;
4. fewer physical points first.

The weight contributes to routing-cost telemetry and ordering. It does not
multiply Dijkstra edge cost, and because PathFinder accepts every proposed
reroute, it does not act as an acceptance tradeoff.

## Global routing

`GcellGrid` is a 2-D `N × N` grid over the final placement die, with
`N = max(gcells_per_side, 2)`. Every node has uniform configured capacity and
four planar neighbours of cost 1. Each physical point maps to its containing
gcell; terminals are deduplicated per net.

Global `RouteHot` starts with empty trees, zero usage, and zero history. Prior
history is loaded only on an exact length match and is multiplied by clamped
`history_decay`.

PathFinder processes dirty nets in route order. A net is dirty when its tree is
empty or any tree node is over capacity. It connects the first terminal to
remaining terminals, which are ordered by Manhattan distance from the first.
Each target is connected by multi-source Dijkstra from all nodes already in
the tree, producing a branch tree rather than a single path through all pins.

For candidate node `n`, incremental cost is:

```text
base_edge_cost
+ history[n]
+ p_fac * max(effective_usage[n] + 1 - capacity, 0)
```

`effective_usage` subtracts this net’s old-tree claim so rip-up/reroute does not
penalize reusing its own resource. After each epoch, overused nodes add
`hist_inc * (usage - capacity)` to history. Every proposed reroute is accepted.

Global routing stops after at least one epoch when overflow is zero, at
`max_iters`, or when an epoch proposes no reroutes. Its output is used only for
corridors/history/telemetry; final wire geometry comes from detailed routing.

## Detailed corridors

For each global route-tree node, the façade adds that gcell and its valid 8
neighbours to the detailed corridor. The resulting gcell IDs are sorted and
deduplicated.

During detailed PathFinder, a net is corridor-restricted for the first eight
epochs. A failed corridor search immediately retries unrestricted, and after
eight epochs all searches are unrestricted. A corridor is guidance, not a
hard final constraint.

## Track graph

The detailed graph is a rectangular track lattice:

- requested pitch is raised to at least `max(die dimension)/1,200` and 1 nm;
- `nx = max(die_width / actual_pitch, 2)` and similarly for `ny`;
- layer count is at least one;
- capacity is always 1;
- even layers have only horizontal neighbours;
- odd layers have only vertical neighbours;
- adjacent layers connect by `via_cost` at the same track coordinate.

Track-node centres are `index * actual_pitch + actual_pitch/2`. Integer
division can leave an unused strip at the die’s top/right edge.
The method named `nearest` maps non-negative coordinates with integer
`coordinate / actual_pitch` and clamps only the upper index; it selects the
containing pitch bin rather than rounding to the mathematically nearest track
centre. Callers are expected to keep coordinates non-negative.

Every placed drawn cell rectangle blocks met1. When physical `pin_pos` is
supplied, each blockage grows by one actual graph pitch. Upper routing layers
are not blocked by cell footprints.

## Pin landing

The grid claims one unique node per physical terminal. Candidate search visits
Chebyshev rings of radius 0 through 8 around the terminal’s pitch index and
searches only layers 0 and 1 (or just layer 0 in a one-layer graph).

### Candidate conditions

- The node must not already be claimed.
- Layer-1 candidates are rejected when another claimed layer-1 node lies at the
  same x within ±2 y tracks; this avoids closely stacked vertical landings.
- In protected-pin mode, met1 landing corridors must clear other terminal
  points and `extra_obstacles`. Layer-1 candidates bypass this geometric
  clearance closure.
- `obstacle_clearance`, or configured pitch when zero, controls point-obstacle
  clearance. Other terminal points use a full configured-pitch corridor box,
  with a special exception for nearby same-net mates.
- With `min_area > 0`, `max(|dx|, |dy|, actual_pitch) * actual_pitch` must meet
  the minimum area.

A selected node is reserved to the owning net. Other nets cannot traverse it.
If the selected met1 node was inside a blocked cell, the code opens a
`terminal_only` hole: vias may enter it, but lateral met1 edges to/from it are
disabled.

After first-pass claims, each fully landed net with at least two physical pins
gets an optional refinement. Up to four candidates per pin are gathered,
candidate choices are greedily assigned in ascending displacement, and the
whole reassignment is accepted only when every pin is assigned and total
Manhattan displacement strictly decreases.

When a refinement is accepted, obsolete met1 landing holes are unclaimed and
unreserved but are not re-blocked in the grid. They remain terminal-only
via-access nodes inside the cell blockage for the remainder of that run.

Failure to claim a terminal increments that net’s missing count and creates a
`PinAccessFailure`. Routing can still build a partial tree for its other
landings, but the final net is classified as `unrouted`.

## Search-time parasitic length bound

For every compact net, the first exact-name `ParasiticBudget`, if any, makes the
detailed route context derive:

```text
max_length_nm = max_r * wire_width_nm / 0.125 ohm_per_square
max_cost      = max_length_nm / configured_pitch_nm
```

The global route has no such limit. The detailed Dijkstra rejects a candidate
when the cumulative search cost for the current target-to-existing-tree
connection exceeds `max_cost`. Search distance resets for each additional
target, so this does not bound aggregate multi-terminal tree length. The
cumulative value includes planar/via base cost, congestion history, and
present-congestion penalty, so it is not a pure geometric hop count. The
formula always uses the hard-coded `0.125 Ω/square`, not
`RoutingConfig::wire_params`, and ignores the capacitance budget.

## Detailed PathFinder

Detailed routing uses the same dirty-net negotiated-congestion loop as global
routing, with capacity 1, configured detailed costs/history, terminal
reservations, global corridors, and optional max-cost bounds. A route with no
path remains empty. A clean nonempty tree is not revisited merely to shorten
it; rerouting is driven by emptiness or overuse.

Crosstalk contracts are reconciled after each detailed epoch and can keep the
normal zero-overflow stop condition open. However, an epoch with no proposed
moves ends the generic stage even when a contract remains open, and crosstalk
does not directly change Dijkstra costs or block resources.

## Symmetry-route mirroring

After detailed PathFinder, the façade walks every placement symmetry pair and
compares corresponding pin positions by vector index. If those pins connect to
different compact nets and exactly one net has a nonempty route tree, it tries
to mirror the routed tree into the empty net about `Placement::axes[group]`.

Every source node maps to the nearest detailed node at
`x' = 2*axis_x - x`, with the same y/layer. Installation is abandoned if a
mapped node is already at capacity and is not a destination terminal. On
success, the destination usage is incremented.

This is recovery for a missing mate, not coupled differential-pair routing. It
does nothing when both nets are routed, compares pins by index rather than by
pin-name string, and does not check the mapped node’s blockage, reservation,
or corridor status.

## Optional post-route repairs

### EOL repair

When `eol_spacing > 0`, the router extracts probe geometry and scans
different-net endpoints on the same layer. Endpoint centres with Euclidean
distance strictly less than the threshold mark one offender net and the grid
node nearest that offender endpoint.

All affected nets are ripped up. Marked nodes receive a temporary usage claim,
and each affected net receives one `route_net` attempt using its stored global
corridor and length budget. This repair path has no unrestricted fallback when
the corridor attempt fails. Temporary claims are then removed. The scan is not
repeated and failed reroutes remain empty.

### Wide-net PRL repair

When `wide_net_extra_spacing > 0`, canonical supply/ground names identify power
nets. Same-layer signal segments that are parallel, have a strictly positive
projection overlap, and have nonzero centreline separation below the threshold
are affected. The nearest node to the signal segment’s low coordinate is
temporarily usage-claimed, affected signal nets are ripped up and rerouted once
through their stored corridors with no fallback, then claims are removed.

This is a centreline heuristic. It does not include wire half-width in the
threshold, does not affect power nets, and is not rechecked after repair.

Both repair mechanisms use a usage bump, which increases path cost but is not
an absolute blockage. Neither updates detailed stage telemetry.

## Final classification and hotspots

After mirroring and repairs:

- `overuse` sums every detailed node’s usage above capacity 1;
- `unrouted` includes empty trees and all nets with any missing landing;
- `no_path` includes empty trees only when missing count is zero;
- a hotspot is any detailed node with history at least 5% of the maximum
  nonzero history.

History can be inherited from an earlier run, so hotspots describe negotiated
pressure memory, not only final-tree occupancy.

## Geometry extraction

Each route-tree branch is split at layer transitions. Collinear same-layer
runs are compressed into `Wire` centreline segments; every transition emits a
`Via`. Branches are processed independently, so shared runs can be duplicated.

With `min_area > 0`, a short wire is symmetrically extended along its routing
direction until approximately `ceil(min_area / width)` long. The extension is
not clipped or revalidated against obstacles, spacing, EOL, PRL, or die bounds.

Vias are sorted by `(x, y, layer, net)` and then deduplicated by
`(x, y, layer)` only. On an already-invalid shared site, this can retain one
net’s via and remove another; the final overuse metric is the signal that the
route is not clean.

Returned wires/vias exclude physical pin-to-landing geometry. The integrated
flow consumes `landings` separately when assembling final layout geometry.

## Complexity and determinism

- Each route target runs Dijkstra on the current resource graph with a binary
  heap; stamp arrays avoid clearing all distance/membership arrays per search.
- Dirty detection and history updates are linear in current tree nodes and
  graph nodes respectively.
- Crosstalk clearance uses an O(|tree A| × |tree B|) node comparison.
- EOL and PRL repair scans are quadratic in endpoint/segment counts.
- The implementation is single-threaded.
- Route order and graph searches are deterministic. The current PathFinder
  does not sample the supplied RNG, so routing is deterministic independently
  of `RoutingConfig::seed` for identical inputs.
