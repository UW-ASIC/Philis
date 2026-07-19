# Placement public API

The crate package is `pnr-placement`; its Rust crate name is `pnr_placement`.
It depends on `pnr-engine`, `pnr-cells`, and `pnr-constraints`.

## Exports

The crate defines these public items:

- `PlacementConfig`
- `Placement`
- `PlacementReport`
- `PlacementResult`
- `estimate_sizes`
- `run_placement`
- public module `model`, containing `build_cold` and `PlaceLedger`

It re-exports:

- `pnr_constraints::ConstraintRecord`
- `pnr_constraints::ConstraintContract`
- `pnr_constraints::ConstraintStatus`
- `pnr_engine::placement::Abutment`
- `pnr_engine::placement::GlobalCfg`
- `pnr_engine::placement::DetailedCfg`
- `pnr_engine::placement::RefinementCfg`

`Placement::orient` exposes `pnr_engine::placement::Orient` values, although
`Orient` itself is not re-exported at the crate root. Import it from
`pnr_engine::placement` when it must be named explicitly.

## `PlacementConfig`

All fields are public. `PlacementConfig::default()` produces the following
configuration.

| Field | Type | Default | Exact role |
|---|---|---:|---|
| `seed` | `u64` | `1` | Seeds SplitMix64 for initial jitter and SA proposals. A fixed seed and identical inputs produce the same placement. |
| `utilization` | `f32` | `0.4` | Planning utilization. `run_placement` clamps it to `[0.05, 0.95]` before initial die sizing. |
| `min_side` | `i32` | `12_000` | Lower bound for each side of the initial square canvas. |
| `cell_margin` | `i32` | `1_300` | Total extra width and height in every virtual cell footprint; equivalently half this margin is added on each edge. It also acts as the default boundary halo. |
| `boundary_halo` | `i32` | `0` | Routing space around the compacted placement. A value `<= 0` falls back to `cell_margin`; the effective halo is then at least `2 * grid`. |
| `grid` | `i32` | `5` | Manufacturing snap grid. Must be positive. It controls initial die rounding, final coordinate/axis snapping, compaction shifts, and constraint tolerances. |
| `global` | `GlobalCfg` | see below | Analytical global-stage controls. |
| `detailed` | `DetailedCfg` | see below | Main detailed-SA controls. |
| `refinement` | `RefinementCfg` | see below | Narrow refinement-SA controls. |
| `debug_dir` | `Option<PathBuf>` | `None` | When set, `run_placement` attempts to overwrite placement debug artifacts in this directory. Write failures are logged to stderr and do not fail the run. |
| `net_weight_overrides` | `HashMap<String, f64>` | empty | Multiplies the base weight of an exact-name retained net. It does not replace the base weight. |
| `variant_sizes` | `Vec<Vec<(i32, i32)>>` | empty | Candidate drawn `(width, height)` values per cell. The engine offers reshape moves only for rows with more than one entry. |
| `initial_variants` | `Vec<usize>` | empty | Initial variant per cell. Missing entries become zero; supplied indices are clamped to the last row entry. |
| `variant_pin_offsets` | `Vec<Vec<Vec<(u32, i32, i32)>>>` | empty | `[cell][variant]` physical pins as `(hypergraph_net_id, dx, dy)` relative to the cell centre. Used by detailed/refinement HPWL and orientation moves. |
| `variant_penalties` | `Vec<Vec<f64>>` | empty | Learned additive placement cost per cell and variant. Missing entries cost zero. |
| `legal_abutments` | `Vec<Abutment>` | empty | DRC/LVS-qualified direct-connect transforms that can waive a rectangular overlap for one exact cell/variant/orientation transform. |
| `compaction_slack` | `f32` | `0.0` | Extra inter-cluster gap in nm, added to every required compaction gap. It does not enlarge intra-cluster relations and is not clamped; a negative value reduces gaps. |
| `cell_inflation_x` | `Vec<f64>` | empty | Per-cell virtual drawn-width multiplier from routing feedback. Missing entries and values below 1 become 1. Drawn output sizes do not change. |
| `cell_inflation_y` | `Vec<f64>` | empty | Per-cell virtual drawn-height multiplier from routing feedback, with the same rules as `cell_inflation_x`. |

The virtual half-width for base size `(w, h)` is
`w * max(inflation_x, 1) / 2 + cell_margin / 2`; half-height is analogous.
Variant footprints use the same formula.

### `GlobalCfg`

| Field | Default | Meaning |
|---|---:|---|
| `max_iters` | `500` | Maximum analytical epochs. |
| `overflow_target` | `0.15` | Target bin-density overflow fraction. The global stage also has a hard-coded minimum of 60 epochs before target-based convergence. |
| `step0` | `0.04` | Initial descent step as a fraction of the maximum die span. The schedule multiplies it by `0.995` per epoch and floors it at `0.002`. |
| `momentum` | `0.85` | Momentum retained in the analytical x/y velocities. |
| `lambda0` | `0.5` | Initial density-push multiplier. While internal overflow exceeds `0.05`, it grows by `1.05` per epoch up to `1,000`. |
| `sym_weight` | `4.0` | Global symmetry-gradient multiplier. |
| `target_util` | `0.7` | Per-bin utilization target used by the analytical density model; this is distinct from top-level `PlacementConfig::utilization`. |

### `DetailedCfg`

| Field | Default | Meaning |
|---|---:|---|
| `max_iters` | `220` | Maximum detailed-SA epochs. |
| `min_iters` | `20` | Minimum epochs before freeze-out convergence is allowed. |
| `moves_per_cell` | `60` | Proposal count per cell per epoch. |
| `alpha` | `0.93` | Geometric temperature multiplier per epoch. |
| `range0` | `0.4` | Initial displacement window as a fraction of the maximum die span. The window decays by `0.96` per epoch and is floored at `grid / max(die span)`. |
| `min_accept_rate` | `0.02` | Freeze-out threshold. Early convergence additionally requires negligible normalized overlap and no open hard ledger checks. |
| `outline_weight` | `1.5` | Weight of normalized outline area/aspect cost. Zero or a negative value disables this objective component. |

The initial detailed temperature is derived from 128 sampled proposal deltas:
`max(mean(abs(delta)), 1) * 0.02`.

### `RefinementCfg`

| Field | Default | Meaning |
|---|---:|---|
| `max_iters` | `80` | Maximum refinement epochs. |
| `min_iters` | `10` | Minimum epochs before freeze-out convergence. |
| `moves_per_cell` | `40` | Proposal count per cell per epoch. |
| `alpha` | `0.96` | Geometric temperature multiplier. |
| `range0` | `0.1` | Initial displacement fraction. It decays by `0.98` per epoch to the same grid-derived floor. |
| `min_accept_rate` | `0.01` | Freeze-out threshold, subject to overlap and contract closure. |
| `constraint_boost` | `4.0` | Multiplier on the initial overlap-penalty weight. It does not multiply all soft constraint costs. |
| `outline_weight` | `2.0` | Normalized outline objective weight; negative values are clamped to zero. |

The refinement temperature is derived from 64 sampled proposal deltas:
`max(mean(abs(delta)), 1) * 2`. Its overlap penalty starts at
`(pin-aware cost / virtual cell area) * constraint_boost`, grows by `1.04`,
and is capped at `100,000` times that initial value.

## `Abutment`

```rust
pub struct Abutment {
    pub a: u32,
    pub b: u32,
    pub variant_a: u16,
    pub variant_b: u16,
    pub dx: f32,
    pub dy: f32,
}
```

`dx` and `dy` are the required centre displacement `b - a` in nm. A rule is
active only when:

- the queried pair is ordered exactly as `a < b` by the engine;
- both cells use the listed variants;
- both orientations are `Orient::N`;
- the actual x and y displacement is within `2 * grid` of the rule;
- neither an isolation pair override nor an explicit push rule separates the
  pair.

Only that qualified overlap is ignored. An `Abutment` is a trust boundary:
`pnr-placement` does not run DRC or LVS to validate caller-supplied rules. The
integrated flow derives them by composing generated cells and qualifying the
combination with DRC and extracted-device checks.

## `estimate_sizes`

```rust
pub fn estimate_sizes(g: &BipartiteHypergraph) -> Vec<(i32, i32)>
```

Returns one crude drawn `(width, height)` in nm per hypergraph cell.

| Cell record | Estimated `(width, height)` |
|---|---|
| Resistor | `(w + 400, l + 800)` |
| `Cap`, `Ncap`, or `Pcap` | `(w + 400, l + 400)` |
| Any other recorded device | See folding formula below. |
| No device record | `(2_000 + 300 * pin_count, 3_000)` |

For other devices, `nf = max(record.nf, 1)`, `m = max(multiplier, 1)`,
`finger_width = clamp(w / nf, 100, 5_000)`, and
`effective_fingers = ceil(w / finger_width)`. The result is:

```text
width  = effective_fingers * (l + 460) + 460
height = (finger_width + 700) * m
```

This helper is a fallback for flows without generated-cell bounding boxes. It
does not inspect PDK geometry, enclosure, guard-ring, or routing rules.

## `run_placement`

```rust
pub fn run_placement(
    g: &BipartiteHypergraph,
    sizes: &[(i32, i32)],
    rec: &ConstraintRecord,
    cfg: &PlacementConfig,
    layer_masks: &[u64],
) -> PlacementResult
```

### Inputs

- `g` supplies ordered cells, ordered nets, cell pins, device records, and
  cell/net lookup names.
- `sizes[i]` is cell `i`'s initial drawn size in nm. In the integrated flow it
  is the generated size of `initial_variants[i]`.
- `rec` supplies the constraint families listed in
  [Constraint handling](constraints.md).
- `cfg` controls planning, optimization, variants, feedback, abutments, and
  debug output.
- `layer_masks` is either empty, meaning all cell pairs can conflict, or a
  cell-parallel layer-presence array. Two cells with non-intersecting masks
  contribute no overlap and have effectively infinite reported gap.

### Input invariants

The function explicitly asserts `sizes.len() == g.cells.len()`. Other
parallel-array contracts are assumed by downstream indexed code:

- `grid > 0`;
- sizes and configured geometric distances are non-negative and finite in
  normal use;
- `layer_masks` is empty or has one entry per cell;
- each physical pin offset uses a valid hypergraph net ID; invalid net IDs are
  silently discarded;
- if physical pin offsets are used, their outer cell ordering and variant rows
  correspond to `variant_sizes`;
- `sizes[i]` should equal
  `variant_sizes[i][initial_variants[i]]` after index clamping. The engine
  initializes the hot dimensions from `sizes`, while recording the initial
  variant index separately;
- cells should belong to at most one symmetry group, partner relations should
  be non-conflicting, and all referenced group indices should be valid;
- every common-centroid record should retain at least one valid cell on each
  side. Invalid/empty groups are skipped by model construction but still get a
  ledger check, which can mis-index or panic during reconciliation;
- variant counts and indices must fit `u16` because the engine stores variant
  indices as `u16`;
- `legal_abutments` must use valid, canonically ordered cell and variant IDs.

Short optional vectors generally fall back per entry. They are not required to
be padded when the documented fallback is acceptable.

### Side effects

The function writes progress and warnings to stderr. If `debug_dir` is set it
also creates the directory and overwrites the files listed below. It performs
no other external mutation.

## Outputs

### `Placement`

| Field | Type | Meaning |
|---|---|---|
| `x`, `y` | `Vec<i32>` | Final cell centres in nm, indexed by cell. They are intended to lie on `grid`. |
| `sizes` | `Vec<(i32, i32)>` | Drawn size of the final selected variant, or the input `sizes[i]` when no selected variant entry exists. Margin and inflation are not included. |
| `die` | `(i32, i32)` | Final compacted, halo-padded die width and height. It is normally rectangular even though the initial canvas is square. |
| `cell_margin` | `i32` | The configured planning margin, retained so consumers can distinguish drawn and planning utilization. |
| `axes` | `Vec<i32>` | Final vertical symmetry-axis x coordinate per `ConstraintRecord::symmetry` group. |
| `variant` | `Vec<usize>` | Selected variant index per cell. It is zero when no alternatives are active. |
| `orient` | `Vec<Orient>` | Final orientation per cell: `N`, `S`, `FN`, or `FS`. Mirror partners start as `FN`; orientation moves occur only when pin-offset data is active. |

The five cell-parallel vectors `x`, `y`, `sizes`, `variant`, and `orient` are
debug-asserted to match the hypergraph cell count at the source.

### `PlacementReport`

| Field | Meaning |
|---|---|
| `global` | Full `Telemetry` for the analytical stage. |
| `detailed` | Full `Telemetry` for the main detailed-SA stage. Refinement telemetry is currently not retained. |
| `hpwl_initial` | Weighted, pin-aware HPWL immediately after seeded initialization and before global placement. If pin data are inactive, cell centres are used. |
| `hpwl_final` | The same metric after compaction. Because net weights are included, the value is wirelength-like and may not equal raw physical HPWL despite the display suffix `nm`. |
| `overlap_final` | Pairwise overlap area of final virtual footprints, accounting for selected reshape sizes, inflation, margin, layer masks, and qualified abutments. |
| `validation` | `ContractValidation` summary for placement-created contracts. |
| `contract_lines` | Text snapshot of every placement contract, strength, status, and optional violation metric. |
| `contracts` | Owned placement contracts, including lifecycle history and metrics. |

`Telemetry` exposes `iters`, `proposed`, `accepted`, `illegal`, `cost`,
`initial_cost`, `best_cost`, `overflow`, `epoch_accept_rate`, and an epoch
trace. `trace_csv()` serializes the trace. `best_cost` is only a scalar record;
the engine does not snapshot or restore the state that achieved it, so the
returned placement is the final committed state.

`Display` prints global and detailed summaries, initial/final HPWL, residual
overlap, lifecycle counts, and hard violation IDs. The displayed “unconsumed”
count is `ContractValidation::emitted`; contracts left in the distinct
`Consumed` state are not included in that label.

### `PlacementResult`

`PlacementResult` simply owns `placement` and `report`. Its method

```rust
pub fn write_debug(
    &self,
    dir: &Path,
    g: &BipartiteHypergraph,
) -> std::io::Result<()>
```

re-emits artifacts for this exact owned candidate. The integrated feedback
flow uses it after best-candidate selection so artifacts describe the selected
iteration rather than merely the last iteration.

## Debug artifacts

| File | Contents |
|---|---|
| `global_trace.csv` | `iter,cost,temp,overflow,accept_rate` rows for global placement. |
| `detailed_trace.csv` | The same rows for the main detailed stage. There is no refinement trace file. |
| `contracts.txt` | `contract_lines`. |
| `report.txt` | `PlacementReport` display text. |
| `placement.txt` | Die dimensions followed by `cell_name x y w h`; orientation, variant index, axes, margin, and inflation are omitted. |
| `hypergraph.txt` | `BipartiteHypergraph` display text. |

`run_placement` logs artifact errors and continues. Calling
`PlacementResult::write_debug` directly returns the I/O error.

## Public `model` module

`model::build_cold` exposes the frontend-to-engine conversion used by
`run_placement`:

```rust
pub fn build_cold(
    g: &BipartiteHypergraph,
    sizes: &[(i32, i32)],
    rec: &ConstraintRecord,
    die: (f32, f32),
    grid: f32,
    margin: i32,
    net_weight_overrides: &HashMap<String, f64>,
    layer_masks: &[u64],
    variant_sizes: &[Vec<(i32, i32)>],
    initial_variants: &[usize],
    variant_pin_offsets: &[Vec<Vec<(u32, i32, i32)>>],
    variant_penalties: &[Vec<f64>],
    legal_abutments: &[Abutment],
    cell_inflation_x: &[f64],
    cell_inflation_y: &[f64],
) -> PlaceCold
```

It accepts each configuration view separately and does not perform canvas
sizing, optimization, snap, compaction, reconciliation, or artifact output. It
is public for lower-level integration but has the same array invariants as
`run_placement`; it performs only debug assertions for most parallel-array
consistency. `PlaceCold` is defined in `pnr_engine::placement`, not re-exported
by `pnr-placement`.

`model::PlaceLedger` exposes:

- public field `contracts: Vec<ConstraintContract>`;
- `PlaceLedger::build(g, rec)`;
- `validation()`;
- `summary()`.

It implements `pnr_engine::Ledger<PlaceDomain>`. Its internal check list and
open-hard-check count are private. Most users should consume the ledger already
returned through `PlacementReport` rather than construct one independently.
