# Placement model and algorithms

## Initial canvas

For each input drawn size `(w, h)`, the planning-area estimate uses
`(w + max(cell_margin, 0)) * (h + max(cell_margin, 0))`. Let the sum be `A`
and let `u = clamp(utilization, 0.05, 0.95)`. The area-derived side is
`ceil(sqrt(A / u))`.

The final initial square side is the maximum of:

- that area-derived side;
- the largest single `(dimension + non-negative margin)`;
- `min_side`.

It is rounded upward to `grid`. This sizing step does not include
`cell_inflation_x/y`, `boundary_halo`, or every alternative variant. The global
and detailed stages run inside this square; compaction later derives a smaller
rectangular die and adds boundary halo.

## Cold model construction

`model::build_cold` converts frontend records into structure-of-arrays and CSR
tables used by the engine hot loops.

### Cells and variants

- Base half-extents include directional inflation and half the configured
  margin.
- A variant row is installed in the engine only when it has more than one
  candidate. Zero- and one-entry rows do not enable reshape moves.
- `initial_variants` entries default to zero and are clamped to the last
  configured candidate.
- Variant penalties and legal abutments are copied without normalization.
- Device classes are fixed as NMOS `0`, PMOS `1`, resistor `2`, capacitor
  family `3`, and all other/missing device records `4`.
- The current dense 5×5 class-spacing table is all zeros. Explicit isolation
  rules populate the sparse per-pair table instead; duplicate pairs retain the
  largest effective gap after sorting and deduplication.

The layer mask is copied exactly. If it is empty, all cell pairs conflict. If
it is non-empty, two cells conflict only when their masks share a bit; a zero
entry therefore conflicts with no other cell.

### Nets and weights

For every hypergraph net, connected cell IDs are sorted and deduplicated. Nets
with fewer than two distinct cells are omitted from placement. Retained nets
receive these base weights:

| Classification | Base weight |
|---|---:|
| Supply or ground by canonical name | `0.2` |
| `NetClass::Sensitive` | `3.0` |
| `NetClass::Clock` | `2.0` |
| `Supply`, `Ground`, or `Substrate` class | `0.2` |
| `Signal` or ordinary net | existing name-derived weight, normally `1.0` |

An exact-name `net_weight_overrides` value multiplies this base. Values are not
clamped or validated. Net-to-cells and cell-to-nets are stored as parallel CSR
tables.

### Physical pin mapping

Every configured `(hypergraph_net, dx, dy)` is remapped to the compact retained
placement-net index. Pins on omitted or invalid nets are discarded. The first
variant row also populates a flat base-pin table. Pin-aware HPWL is globally
enabled only when this flat table is non-empty; otherwise all nets use cell
centres even if a later variant row happens to contain offsets.

When enabled, the engine looks up the current variant's offset for each
cell/net pair, falls back to the flat base offset, applies the current
orientation transform, and adds the cell centre. At most the first matching
offset for a cell/net is used by placement HPWL; multiple physical accesses on
the same net are not represented in this objective.

## Objective components

### HPWL

Global placement uses weighted cell-centre HPWL. Detailed and refinement stages
use weighted physical-pin HPWL when pin data are active. For each retained net:

```text
weighted HPWL = net_weight * ((max_x - min_x) + (max_y - min_y))
```

### Soft analog terms

- Proximity pulls use squared Euclidean centre-distance excess beyond the
  configured distance, scaled by `1e-3` and rule weight `1`.
- Thermal pulls use the same formula with zero target distance and weight `3`.
- Common-centroid cost is squared x/y distance between side centroids, scaled
  by `1e-3`.
- ABBA and ABAB add a one-dimensional x-slot pull only for exactly two A and
  two B cells. `CommonCentroid2d` currently uses the same one-dimensional slot
  assignment as ABBA; it is not a 2-D geometric construction.
- Straight-net placement cost is the sum of absolute deviation from the mean x
  for vertical routes or mean y for horizontal routes.
- Stress cost is squared distance excess outside the allowed radius from the
  current planning-die centre, scaled by `1e-3`, in a full cold-path objective
  evaluation.
- Explicit pair-spacing violations add a squared soft penalty scaled by
  `4e-3` in a full cold-path objective evaluation, in addition to hard
  non-worsening legality.
- Variant penalties are additive in detailed/refinement SA.

The current global gradient does not include the stress term, and the
detailed/refinement incremental delta does not include either stress or the
explicit pair-spacing soft term. Consequently those full-evaluation terms do
not guide accepted moves: explicit spacing is driven by hard non-worsening
legality, while stress is currently a final audit rather than an effective
optimization force. Exact stage-end telemetry recomputes the full objective.

### Outline

Detailed/refinement SA adds `outline_weight * outline_metric`. The metric uses
the virtual bounding-box area plus half the sum of virtual occupied cell area,
normalizes by the square root of total base virtual cell area, and adds a small
`0.05 * abs(width - height)` aspect penalty. It also prices the legal envelope
needed to separate currently overlapping mirror partners, preventing an
overlapping symmetric pair from looking artificially compact.

### Overlap

Detailed/refinement density is total pairwise virtual-rectangle overlap
divided by total virtual cell area. Disjoint layer masks and active qualified
abutments contribute zero. Overlap is a ramped density penalty, not an absolute
move rejection, so a budget-exhausted run can retain overlap.

## Stage 0: initialization

All cells start within seeded jitter of the initial die centre; jitter radius
is `0.15 * min(die width, die height)`. Symmetry axes start at their fixed
constraint position or the die centre. Self-symmetric cells are put on their
axis. Each mirror partner is derived as `x_partner = 2*axis - x_member` with
equal y, and receives `Orient::FN`; other cells receive `Orient::N`.

The current hot dimensions start from `sizes`, even when a nonzero initial
variant index is supplied. This is why production callers keep `sizes` and
`initial_variants` consistent.

## Stage 1: analytical global placement

Each epoch computes one all-cell momentum step from:

- a weighted HPWL subgradient;
- proximity and thermal gradients;
- common-centroid and supported pattern gradients;
- straight-net alignment gradients;
- soft symmetry error and free-axis updates;
- a coarse density push toward less occupied neighbouring bins.

The bin count is `clamp(ceil(sqrt(cell_count)), 4, 24)`. Each entire virtual
cell area is charged to the bin containing its centre. Proposed cell centres
are clamped to the die using base half-extents. Every analytical proposal is
accepted; hard gaps and overlaps are not legality filters in this stage.

The stage stops at `max_iters`, or after at least 60 epochs when density
overflow is at or below `overflow_target`. Placement contracts are reconciled
each epoch, but global stopping uses the ledger as well: all hard checks counted
as open must be closed for target-based termination.

## Stage 2: detailed simulated annealing

The engine samples initial move deltas to choose a temperature, then performs
Metropolis SA with a shrinking displacement window. It computes HPWL and soft
cost deltas incrementally for only touched nets and constraints.

### Moves for a symmetry-group cell

The move always preserves the configured vertical mirror relation:

- 30%: translate the whole symmetry island and its axis together. The current
  move generator does this even for an axis marked fixed;
- next 20% opportunity: reshape a self-symmetric cell or a partner pair;
- next 10% opportunity: change symmetry-compatible orientations, but only when
  physical pin data are active;
- otherwise: move a self-symmetric cell along y on its axis, or move a mirror
  pair with mirrored x and common y.

Failed reshape/orientation opportunities fall through to the positional move.
Paired reshape uses the same variant index on both cells and requires the two
variant half-extents to be exactly equal at that index.

### Moves for an ungrouped cell

- first 10%: select a different variant when alternatives exist;
- next 10%: select another orientation when physical pin data are active;
- up to 35%: snap to an available qualified abutment transform;
- up to 70%: random bounded displacement;
- otherwise: swap positions with another ungrouped cell, or emit a no-op when
  the selected partner is unsuitable.

Abutment moves can also select the required variant and reset orientation to
`N`. Position and shape changes are committed atomically.

### Hard legality

Isolation, explicit required gaps, and DTI forbidden bands use non-worsening
legality. A proposed move is rejected only if it deepens an existing violation
or creates a worse one. This allows detailed placement to escape a globally
illegal starting point without freezing every move. Ordinary rectangle
overlap remains a density cost rather than a hard legality test.

The stage stops at its budget or, after `min_iters`, when all of these hold:

- epoch acceptance rate is below `min_accept_rate`;
- normalized overlap is at most `1e-4`;
- the ledger reports no open hard checks.

## Stage 3: refinement

Refinement reuses the same move generator, incremental objective, overlap
density, non-worsening legality, and stopping rule. It changes proposal range,
cooling, epoch budget, outline weight, and the initial/growth schedule of the
overlap penalty as described in [the API defaults](api.md#refinementcfg).

The returned refinement telemetry is intentionally discarded by the façade at
present. Final coordinates and costs do include its effects.

## Grid snap and symmetry restoration

After annealing, every axis and cell centre is independently rounded to the
nearest `grid`. The code then re-derives each mirror partner from the snapped
axis and leading member, and puts self-symmetric cells exactly on the axis.
This preserves the mirror equation through quantization.

## Constraint-graph compaction

Compaction operates on final virtual footprints before contract reconciliation.
It builds rigid clusters using:

- all members of each symmetry group;
- both sides of every common-centroid group;
- currently active legal abutments;
- selected DTI pairs on the near/abutting branch when welding them does not
  conflict with a stronger explicit required gap.

Straight-net alignment is deliberately not a rigid cluster edge because it is
only one-dimensional. A connected component containing a fixed symmetry axis
is frozen in x.

Required inter-cluster gap is the maximum applicable explicit/class/push gap,
plus `compaction_slack`. Far-branch DTI pairs receive an effective virtual-gap
floor of `d_dti*1,000 - cell_margin + 50 nm`, which corresponds to a raw drawn
gap of `d_dti*1,000 + 50 nm` before slack. Mirror partners are first pushed symmetrically
outward if their common y makes x carry the full gap. Pure symmetry clusters
may also be packed internally in y without breaking paired y equality.

The compactor then performs x and y leading-edge constraint passes for up to
four rounds, snapping every shift upward to the grid. It returns the bounding
box over virtual footprints. Complexity is approximately quadratic in the
number of clusters times their members; no interval tree is used.

`run_placement` rejects the compacted state and restores the pre-compaction
state when:

- virtual overlap worsens by more than `0.5 nm²`; or
- bounding-box area expands by more than one grid-cell area without at least a
  `0.5 nm²` overlap improvement.

Whether accepted or restored, the selected bounding box is translated and a
new die is derived. The halo is `boundary_halo` when positive, otherwise
`cell_margin`, and is floored at `2 * grid`. Non-fixed-axis placements are
translated in both x and y so their virtual lower bounds begin at the halo.
When any axis is fixed, x translation is zero to preserve its absolute
coordinate; y is still translated. Axes move with any allowed x translation.

The final die dimensions are snapped to the nearest grid, not rounded upward.
Contracts are reconciled only after this die replacement and translation.

## Complexity and reproducibility

- Net storage and touched-term indexing use CSR tables.
- Local overlap checks use a bucket grid sized for the largest available
  variant; candidates come from a 3×3 bucket neighbourhood.
- Exact final overlap is an O(n²) pair scan.
- Compaction is an O(cluster² × members) bounded iterative sweep.
- The engine is single-threaded.
- `seed` controls both initial jitter and SA choices. Identical inputs and seed
  are deterministic; the integrated feedback flow deliberately derives a new
  seed per iteration to avoid replaying the same local trajectory.
