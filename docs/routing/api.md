# Routing public API

The package is `pnr-routing`; its Rust crate name is `pnr_routing`. It depends
on `pnr-engine`, `pnr-placement`, `pnr-cells`, and `pnr-constraints`.

## Exports

The crate defines these public items:

- `RoutingConfig`
- `RoutingReport`
- `RoutingResult`
- `RoutingHotspot`
- `RoutingFeedback`
- `MatchedDelta`
- `run_routing`
- `run_routing_at`
- `extract_feedback`
- `extract_feedback_with_prior`

It re-exports these `pnr_engine::routing` items:

- `GlobalRouteCfg`
- `DetailedRouteCfg`
- `Wire`
- `Via`
- `PinLanding`
- `PinAccessFailure`
- `extract_geometry_minarea`
- `wire_rect`

It imports, but does not re-export, `ConstraintRecord`, `Placement`, `RouteHot`,
`TrackGrid`, and the other lower-level graph types. Name those through their
own crates when calling a low-level re-export such as
`extract_geometry_minarea`.

## `RoutingConfig`

| Field | Type | Default | Exact role |
|---|---|---:|---|
| `seed` | `u64` | `1` | Initializes SplitMix64 passed into both routing stages. Current PathFinder proposals do not sample the RNG, so changing only this seed currently does not change a route. |
| `global` | `GlobalRouteCfg` | see below | Gcell graph and global PathFinder controls. |
| `detailed` | `DetailedRouteCfg` | see below | Track graph, landing, detailed PathFinder, geometry, and optional repair controls. |
| `debug_dir` | `Option<PathBuf>` | `None` | Enables best-effort debug artifact writes. Errors are logged and do not fail routing. |
| `net_priority_overrides` | `HashMap<String, f64>` | empty | Exact-name sort priority. Higher values route earlier; this does not change the net weight or Dijkstra edge cost. |
| `wire_params` | `Option<WireParasiticParams>` | `None` | Enables post-route R/C judgment for parasitic and differential contracts. It does not configure the route-search sheet resistance and is not automatically retained for a later `extract_feedback_with_prior` call; pass the parameters there separately. |
| `extra_obstacles` | `Vec<(i32, i32)>` | empty | Additional point obstacles used only while selecting protected-mode landing corridors, primarily for in-cell met1 features absent from the router model. They have no effect through `run_routing`, which does not enable protected mode. |
| `global_history` | `Vec<f32>` | empty | Negotiated-congestion history from an earlier run. Loaded only when its length exactly matches the new global graph node count. |
| `detailed_history` | `Vec<f32>` | empty | Prior detailed history, likewise loaded only on exact node-count match. |
| `history_decay` | `f32` | `0.65` | Multiplier applied while loading matching prior history; clamped to `[0, 1]`. |

### `GlobalRouteCfg`

| Field | Default | Meaning |
|---|---:|---|
| `gcells_per_side` | `16` | Requested x and y gcell count. Each dimension is floored at 2. |
| `gcell_capacity` | `6` | Uniform capacity of every gcell graph node. |
| `max_iters` | `40` | Maximum global PathFinder epochs. The generic driver executes at least one epoch. |
| `p_fac` | `3.0` | Present-congestion cost multiplier. |
| `hist_inc` | `0.5` | History increment per unit of node over-capacity after an epoch. |

The global grid spans the placement die exactly. Gcell node positions are cell
centres; planar neighbours cost 1 and the graph has no layer dimension.

### `DetailedRouteCfg`

| Field | Default | Meaning |
|---|---:|---|
| `pitch` | `430` | Requested track pitch in nm. The graph raises it to at least `max(die dimension) / 1,200` and at least 1 nm. |
| `wire_width` | `290` | Width in nm assigned to extracted wires and vias. Also enters the search-time max-R route-length estimate. |
| `via_cost` | `4.0` | Dijkstra base cost for moving between adjacent conductor layers. |
| `max_iters` | `150` | Maximum detailed PathFinder epochs. |
| `p_fac` | `2.0` | Present-congestion multiplier. |
| `hist_inc` | `0.5` | Per-epoch history increment per overused track node. |
| `min_area` | `0` | Minimum emitted wire area in nm²; zero disables. It also filters landing candidates. |
| `obstacle_clearance` | `0` | Landing-corridor clearance from extra point obstacles. Zero falls back to configured `pitch`. |
| `eol_spacing` | `0` | Endpoint spacing threshold in nm for an optional one-shot post-route repair. Zero disables. |
| `wide_net_extra_spacing` | `0` | Same-layer parallel-run centreline spacing threshold in nm between canonical power nets and signal nets. Zero disables the one-shot repair. |
| `n_layers` | `2` | Number of ordered routing conductors. The graph floors this at 1. Pin landings are searched only on the first two layers. |

Preferred direction is fixed by layer index: even layers allow only x moves,
odd layers only y moves. Adjacent-layer vias are allowed at the same `(x, y)`
track coordinate.

## Geometry and landing types

### `Wire`

```rust
pub struct Wire {
    pub net: u32,
    pub layer: u32,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub width: i32,
}
```

Endpoints are centreline coordinates. Extracted track runs are Manhattan and
preferred-direction by layer. Minimum-area extension can move an endpoint off
the original track-tree span.

`wire_rect(&wire)` returns `(xmin, ymin, xmax, ymax)` by expanding the
centreline by integer `width / 2` on all sides. For an odd width this rectangle
has one nm less total width than the nominal value because integer division is
used.

### `Via`

```rust
pub struct Via {
    pub net: u32,
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub layer: u32,
}
```

`size` is set to `wire_width`; `layer` is the lower conductor index. Route-tree
vias at the same `(x, y, lower_layer)` are deduplicated during extraction.

### `PinLanding`

```rust
pub struct PinLanding {
    pub net: u32,
    pub pin: (i32, i32),
    pub node: (i32, i32),
    pub layer: u32,
}
```

This maps an original physical terminal point to its claimed track node. The
wire list does not contain the pin-to-node landing stub; a geometry consumer
must materialize it. The integrated flow emits met1/met2 landing pads, vias,
and L-shaped stubs from this record.

### `PinAccessFailure`

```rust
pub struct PinAccessFailure {
    pub net: u32,
    pub pin: (i32, i32),
}
```

It identifies a physical terminal for which no unique legal candidate was
found within the landing search. The net ID is the compact result-net index.

### `extract_geometry_minarea`

```rust
pub fn extract_geometry_minarea(
    hot: &RouteHot,
    grid: &TrackGrid,
    width: i32,
    min_area: i64,
) -> (Vec<Wire>, Vec<Via>)
```

This lower-level helper compresses straight route-tree runs, emits a via at
each layer transition, optionally extends short wires symmetrically to exceed
`min_area`, and deduplicates vias. It does not emit landing stubs, rerun DRC,
clip extensions to the die, or deduplicate overlapping wire segments.

## Routing entry points

### `run_routing`

```rust
pub fn run_routing(
    g: &BipartiteHypergraph,
    p: &Placement,
    rec: &ConstraintRecord,
    cfg: &RoutingConfig,
) -> RoutingResult
```

This is `run_routing_at(..., None, ...)`. For hypergraph pin index `pi` on cell
`c`, it synthesizes one point at:

```text
x = cell_center_x - cell_width/2
    + (2*pi + 1) * cell_width / (2*max(cell_pin_count, 1))
y = cell_center_y
```

It does not transform a per-pin offset or inspect generated pin geometry.
`Placement::orient` is therefore irrelevant to this convenience path; a caller
that needs oriented physical terminals must compute them and use
`run_routing_at`.

### `run_routing_at`

```rust
pub fn run_routing_at(
    g: &BipartiteHypergraph,
    p: &Placement,
    pin_pos: Option<&[Vec<Vec<(i32, i32)>>]>,
    rec: &ConstraintRecord,
    cfg: &RoutingConfig,
) -> RoutingResult
```

`pin_pos[cell][hypergraph_pin]` is a list of physical access points. Supplying
an existing but empty access list intentionally contributes no point; it does
not fall back to the synthesized location. Duplicate points on a net are
removed. One hypergraph pin may expand to many physical finger pads that the
router straps together.

Passing `Some` also enables protected-pin mode: met1 blockages grow by one
actual graph pitch around every placed cell, and landing-corridor clearance
checks become active.

### Input invariants

The routing functions do not validate all parallel arrays:

- `p.x`, `p.y`, and `p.sizes` must each have at least `g.cells.len()` entries;
- `pin_pos`, when present, must contain a row for every referenced cell because
  the outer row is indexed directly; inner pin rows may be short and then fall
  back only when the row itself is absent;
- every placement size and die dimension should be positive and coordinates
  should be representable by `i32` arithmetic;
- requested `pitch`, `wire_width`, and useful graph capacities should be
  positive; costs, history values, R/C parameters, priorities, and constraint
  budgets should be finite and non-negative in normal use;
- in particular, `wire_width == 0` with `min_area > 0` makes minimum-area
  geometry extension divide by zero;
- parasitic `max_r` used during search should be positive; zero or negative
  values can create a zero/negative route budget;
- `n_layers` and all PDK parameter arrays should describe the same ordered
  routing stack;
- caller-supplied histories must originate from a graph with the same node
  count to be loaded.

The functions return diagnostics rather than a Rust `Result`. Structural
index errors and arithmetic overflow can still panic.

## `RoutingResult`

| Field | Type | Meaning |
|---|---|---|
| `wires` | `Vec<Wire>` | Extracted route-tree wires after optional min-area extension and repair passes. Landing stubs are excluded. |
| `vias` | `Vec<Via>` | Deduplicated route-tree layer transitions. Landing-stack vias are excluded. |
| `landings` | `Vec<PinLanding>` | Successfully claimed physical-terminal mappings. |
| `net_names` | `Vec<String>` | Dense result-net ID to original name. Contains only nets with at least two deduplicated physical points before landing. |
| `net_hpwl` | `Vec<i64>` | Raw, unweighted HPWL of those pre-landing physical points. |
| `report` | `RoutingReport` | Stage telemetry, route metrics, failure lists, and contract summary. |
| `crosstalk_violations` | `Vec<(String, String, f64)>` | Violated crosstalk pairs and shortfall in nm, derived from ledger status. |
| `landing_failures` | `Vec<PinAccessFailure>` | One record per unclaimed physical terminal. |
| `hotspots` | `Vec<RoutingHotspot>` | Detailed nodes whose history is at least 5% of the maximum nonzero detailed history. |
| `global_history` | `Vec<f32>` | Final global history array for a later feedback iteration. |
| `detailed_history` | `Vec<f32>` | Final detailed history array. |
| `track_pitch` | `i32` | Configured `DetailedRouteCfg::pitch`, not necessarily the larger pitch selected internally for a very large die. |

### `RoutingHotspot`

`x`, `y`, and `layer` identify the detailed graph node;
`pressure` is its history value as `f64`.

## `RoutingReport`

| Field | Meaning |
|---|---|
| `global` | `Telemetry` from global PathFinder. |
| `detailed` | `Telemetry` from detailed PathFinder before route mirroring and repair passes. |
| `wirelength_nm` | Sum of Manhattan lengths of returned `wires`, including min-area extensions and duplicate branch segments, excluding landing stubs and via cost. |
| `via_count` | `vias.len()`; landing vias are excluded. |
| `overuse` | Final sum of `usage - capacity` over detailed nodes, after mirroring and repairs. Detailed capacity is 1. |
| `unrouted` | Net names whose final tree is empty or that had at least one missing pin landing. |
| `no_path` | Net names whose final tree is empty and had no missing landing. This is a subset of `unrouted`. |
| `missing_pin_count` | Number of `PinAccessFailure` records. |
| `validation` | `ContractValidation` for only routing-created crosstalk, parasitic, differential, and straight-net contracts. |
| `contract_lines` | Text snapshot containing contract ID, strength, and status. Metrics/details are omitted. |

`Display` prints the two stage summaries, returned wirelength/via count,
unrouted names, contract counts, and hard violation IDs. Its “unconsumed” value
is only `validation.emitted`; contracts left `Consumed` are a separate count
and are not printed on that line.

Both telemetry objects expose `iters`, `proposed`, `accepted`, `illegal`,
`cost`, `initial_cost`, `best_cost`, `overflow`, `epoch_accept_rate`, and the
per-epoch trace. Routing cost is weighted route-tree edge count, while overflow
is summed node over-capacity; neither is physical wirelength in nm. The engine
does not retain the state corresponding to `best_cost`. In particular, an
empty routing state begins at cost zero, so routing `best_cost` is not a
best-complete-route selector.

## `RoutingFeedback` and `MatchedDelta`

`RoutingFeedback` owns cell- and net-parallel data intended for a later
placement/routing iteration. Its exact fields and calculations are documented
in [Constraints and feedback](constraints-and-feedback.md#routingfeedback-fields).

```rust
pub struct MatchedDelta {
    pub device_a: String,
    pub device_b: String,
    pub net_a: String,
    pub net_b: String,
    pub r_a: f64,
    pub r_b: f64,
    pub c_a: f64,
    pub c_b: f64,
    pub r_delta_pct: f64,
    pub c_delta_pct: f64,
}
```

R values are ohms, C values are fF, and deltas use the symmetric average of
the two values as denominator. The type records only pairs that survive the
tier, pin-name, distinct-net, and 0.1% significance filters.

## `RoutingResult::write_debug`

```rust
pub fn write_debug(&self, dir: &Path) -> std::io::Result<()>
```

This re-emits artifacts for the owned candidate, which is useful after an
iterative flow chooses a non-final iteration as best. The method does not need
the hypergraph because `net_names` is stored in the result.

## Debug artifacts

| File | Contents |
|---|---|
| `global_route_trace.csv` | `iter,cost,temp,overflow,accept_rate` for global PathFinder. |
| `detailed_route_trace.csv` | The same for detailed PathFinder, before mirroring/repairs. |
| `route_report.txt` | `RoutingReport` display text. |
| `route_contracts.txt` | `contract_lines`. |
| `routes.txt` | Header `net layer x0 y0 x1 y1`, then one returned wire per line. Layers print as `met{layer+1}`. Width is omitted. |

Landings, landing failures, vias, hotspots, HPWL, route order, histories,
crosstalk shortfalls, and R/C estimates are not written to these artifacts.
`RoutingConfig::debug_dir` logs and ignores I/O errors;
`RoutingResult::write_debug` returns them.
