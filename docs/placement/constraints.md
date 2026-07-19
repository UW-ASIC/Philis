# Placement constraint handling

This page describes exactly how `ConstraintRecord` affects placement. “Used in
optimization” and “represented by a placement contract” are separate: some
routing contracts influence placement geometry without being closed here, and
some record families are ignored by this crate.

## Coverage matrix

| `ConstraintRecord` vector | Placement model | Placement contract | Exact behavior |
|---|---|---|---|
| `symmetry` | Yes | One per valid/invalid `MatchingPair`; no contract for `self_symmetric` alone | Vertical mirror groups, axes, symmetry-preserving moves and compaction. |
| `cc` | Yes | Yes | Side-centroid/pattern soft cost and a final centroid check. |
| `proximity` | Yes | Yes | Soft pair attraction and a maximum final raw edge-gap check. |
| `isolation` | Yes | Yes | Non-worsening minimum raw edge-gap legality, compaction spacing, and final check. |
| `thermal` | Yes | Yes | Strong zero-distance attraction heuristic and adjacency-style final check. Temperature fields are not numerically modeled. |
| `stress` | Full objective/audit only | Yes | Full-cost die-centre term and final check; current gradients/incremental move deltas do not optimize it. |
| `dti` | Yes | Yes | Non-worsening forbidden gap band, compaction near/far handling, and final outside-band check. |
| `guard_ring` | Yes | Yes, despite the constraint type declaring routing/signoff stages | Reserves `2 * min_width_um` raw edge gap from the guarded device to every other cell. Ring geometry and electrical closure are not placed here. |
| `environment` | No | No | Ignored by `pnr-placement`. |
| `lde` | No | No | Cell-generation concern. |
| `dummy` | No | No | Cell-generation concern. |
| `unitization` | No | No | Cell-generation concern. |
| `aging` | No | No | Cell-generation/signoff concern. |
| `net_class` | Yes | No | Sets retained-net HPWL weights. |
| `crosstalk` | No | No | Routing concern. An external controller can translate net-pair shortfalls into device pressure; the current integrated flow does not correctly resolve ordinary net names as devices. |
| `straight` | Yes | No | Adds cell-centre alignment cost for all distinct cells on the named net. The contract is closed by routing. |
| `parasitic` | No directly | No | Routing feedback may supply `net_weight_overrides`, but this record is not read by placement. |
| `differential` | No | No | Routing concern. |
| `antenna` | No | No | Signoff/routing concern not consumed here. |
| `current_flow` | No | No | Not consumed here. |
| `esd` | No | No | Not consumed here. |
| `bias_current` | No | No | Not consumed here. |

### Placement contract metadata

| Input | Contract kind / ID | Strength | Priority |
|---|---|---|---:|
| Matching pair | `matching_pair`, `match_{device_a}_{device_b}` | Hard for tier `Moderate` or `Exceptional`; soft for `None` or `Minimal` | `95` Exceptional, `80` Moderate, `50` Minimal, `30` None |
| Common centroid | `common_centroid`, `cc_{all_scope_names}` | Soft | `60` |
| Proximity | `proximity_rule`, `prox_{device_a}_{device_b}` | Soft | `40` |
| Isolation | `isolation`, `iso_{device_a}_{device_b}` | Hard | `85` |
| Thermal gradient | `thermal_gradient`, `tgrad_{device_a}_{device_b}` | Hard | `75` |
| Stress | `stress`, `stress_{device}` | Soft | `50` |
| DTI | `dti`, `dti_{device_a}_{device_b}` | Hard | `75` |
| Guard ring | `guard_ring`, `guard_ring_{device}` | Hard | `80` |

Unknown device IDs render as `<unknown>` in generated contract IDs/scopes.
The normal constraint source, confidence, declared stage list, and transition
history remain available through `PlacementReport::contracts`.

## Distance convention

The engine footprints contain half the configured `cell_margin` on every
edge. User and PDK distance constraints refer to drawn bounding boxes. At model
construction, raw isolation and DTI thresholds are reduced by the total margin
for optimization, with minimum-gap values floored at zero. Public ledger checks
add the margin back through the raw `gap()` calculation.

For cells `a` and `b`, the ledger’s raw edge gap is:

```text
gx = abs(xa - xb) - (base_virtual_half_width_a + base_virtual_half_width_b)
gy = abs(ya - yb) - (base_virtual_half_height_a + base_virtual_half_height_b)
raw_gap = max(gx, gy) + cell_margin
```

With no directional inflation and no post-initial reshape, this equals the
separation of the drawn rectangles under the placer’s `max(gx, gy)`
convention. With inflation active, the value is conservative relative to drawn
geometry. Ledger `gap()` uses the base dimensions constructed from the input
`sizes`; it does not use a later selected reshape variant. Optimization and
compaction use reshape-aware dimensions, so a final gap contract can differ
from the final selected-variant geometry when variants change. If layer masks
do not intersect, the public gap is effectively infinite.

## Exact translation and closure rules

The general check tolerance is `tol = 2 * grid`.

### Symmetry

Every record creates one engine group, even if all members are invalid. Valid
pair IDs set each other as partners; valid self-symmetric IDs partner
themselves. A free axis starts at the planning die centre. A fixed axis is
converted from angstroms to nm by division by 10.

`group_id` does not affect placement. For each `MatchingPair`, device IDs drive
geometry and `tier` drives contract strength/priority; `matching_type`,
`max_dvth_mv`, `max_did_pct`, and `w_ratio` do not change the placement
objective or moves.

Each matching pair creates a contract. It is consumed only if both IDs are
valid. Final satisfaction requires both:

```text
abs(xa + xb - 2*axis) <= tol
abs(ya - yb) <= tol
```

The recorded violation is the larger residual, converted from nm to µm by the
ledger. `self_symmetric` entries affect geometry but do not create their own
contracts. No contract checks that a fixed axis remains at its requested
absolute coordinate; only the pair-to-current-axis relation is checked.

Overlapping or contradictory group membership is not diagnosed. Later groups
overwrite the per-cell group/partner lookup while earlier group member lists
remain present, so callers must supply disjoint, consistent groups.

### Common centroid

Invalid device IDs are filtered independently from each side. The model emits
an engine group only when both filtered sides are non-empty. Every input record
nevertheless creates a consumed contract check at its original index; invalid
or empty groups are therefore unsupported and can mis-index reconciliation.

Optimization matches the two side centroids and optionally applies the
two-plus-two x-pattern heuristic. Final satisfaction requires Euclidean
centroid separation at most `4 * tol`, or `8 * grid`. The violation metric is
the centroid separation in µm.

### Proximity

A valid pair adds a weight-1 pull whose Euclidean centre-distance target is
`min_distance_um * 1,000`. Despite the field name, the final ledger treats this
as a maximum raw edge-gap budget:

```text
raw_gap <= min_distance_um * 1,000
```

When that configured value is zero, the final limit instead becomes twice the
larger of the pair’s summed virtual half-widths or summed virtual half-heights.
The constraint is soft. Invalid IDs leave an `Emitted` contract with no check.

### Isolation

A valid pair creates both a hard push and a sparse required-gap entry. Its
optimization threshold is `max(raw_min_distance_nm - cell_margin, 0)` in
virtual-footprint coordinates. Detailed/refinement moves may not worsen a gap
below this value, and compaction must preserve it.

The final hard contract checks the raw value directly:

```text
raw_gap >= min_distance_um * 1,000
```

Invalid IDs leave an `Emitted` contract and are included in the dropped-input
warning. `requires_guard_ring` and `reason` do not change placement behavior;
guard-ring spacing is supplied separately through `rec.guard_ring`.

### Thermal gradient

A valid record adds a zero-target pull with weight 3. Neither `max_delta_c` nor
`estimated_gradient_c` participates in placement math. Its hard final contract
uses `MaxDist` with a zero configured limit, which substitutes this adjacency
heuristic:

```text
limit = 2 * max(sum of virtual half-widths, sum of virtual half-heights)
raw_gap <= limit
```

This is a geometric proxy, not thermal simulation. Invalid IDs leave an
`Emitted` contract.

### Stress

A full objective evaluation contains a soft penalty for Euclidean centre
distance beyond `max_centroid_distance_um * 1,000` from the planning-die
centre. The current analytical gradient and incremental SA delta omit that
term, so it does not presently steer moves. The final soft contract is
evaluated against the final compacted die centre and allows an extra `tol`:

```text
distance_to_final_die_center <= max_distance_nm + tol
```

The violation metric records `distance - max_distance` in µm; because of the
tolerance this metric can be slightly positive on a satisfied contract.
Invalid IDs leave an `Emitted` contract.

### Deep-trench isolation

A valid pair becomes a forbidden virtual gap band. Model thresholds are:

```text
min = s_max * 1,000 - cell_margin
max = d_dti * 1,000 - cell_margin
```

Unlike minimum-gap conversion, these band edges are not floored at zero.
Detailed/refinement moves may not increase penetration into the open band.
Compaction keeps near pairs rigid when safe and gives far pairs a virtual lower
bound of `d_dti*1,000 - cell_margin + 50 nm`, equivalent to raw
`d_dti*1,000 + 50 nm`, before routing slack.

The final hard contract uses raw thresholds and strict inequalities:

```text
satisfied iff raw_gap < s_max*1,000 OR raw_gap > d_dti*1,000
```

Equality to either boundary is a violation. The metric is penetration depth
into the forbidden band in µm. Invalid IDs leave an `Emitted` contract.

### Guard ring

For every valid guarded device, placement adds a hard push to every other cell:

```text
required raw gap = 2 * min_width_um * 1,000
```

Only `device_id` and `min_width_um` affect placement. Ring type, sharing, tap
pitch, resistance, enclosure, and connection net are handled elsewhere.

The one guard-ring contract is consumed and receives one `MinGap` check per
other cell. A single-cell design produces a consumed contract with no checks.
An invalid guarded device is skipped entirely by the ledger—no emitted
contract is retained—although model construction includes it in the dropped
warning count.

Current reconciliation applies those neighbour checks sequentially to the
same contract. The `open` count correctly increments for each failing hard
check, but a later passing neighbour can overwrite the visible status left by
an earlier failing neighbour. See [known limitations](behavior-and-limitations.md#contract-and-constraint-limitations).

### Straight net

If the name resolves to a hypergraph net with at least two distinct cells,
placement adds an alignment objective:

- `vertical = true`: minimize deviation of cell-centre x values;
- `vertical = false`: minimize deviation of cell-centre y values.

It uses cell centres, not physical pin offsets, and creates no placement
contract. Routing later creates and judges the `StraightNet` contract.

### Net classification and name-based rails

The exact retained-net weight table is documented in
[Model construction](model-and-algorithms.md#nets-and-weights). Canonical
supply/ground names are recognized case-insensitively. The first matching
`NetClassification` record for an exact net name overrides that name-derived
base. Shielding flags, voltage domain, preferred layers, and coupling budgets
do not affect placement.

## Contract lifecycle

`PlaceLedger::build` creates only the contract families described above. A
valid check calls `consume("placement")`. At every optimizer epoch and once
after compaction, each check becomes `Satisfied` or `Violated`; hard failures
also contribute to the ledger’s open count. A status transition records
history in `ConstraintContract`.

Satisfied details say the result is within `2 * grid` nm tolerance even for
checks whose actual rule uses a different or no tolerance. Violations store the
engine metric divided by 1,000 with units `um`.

`ContractValidation` counts `Emitted`, `Consumed`, `Satisfied`, `Violated`, and
`Waived` separately. `hard_violations` contains only unwaived hard contracts
whose final visible status is `Violated`; it does not include `Emitted` or
still-`Consumed` hard contracts.

## Dropped-reference warning

Model construction prints one aggregate warning for out-of-range references in
proximity, thermal, stress, DTI, isolation, guard-ring, and symmetry-pair
records. It does not count invalid common-centroid members or invalid
self-symmetric IDs. The per-family lifecycle behavior still follows the rules
above; the warning is diagnostic, not an error return.
