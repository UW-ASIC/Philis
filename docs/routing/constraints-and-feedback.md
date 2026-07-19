# Routing constraints and feedback

## Constraint coverage matrix

| `ConstraintRecord` vector | Affects route construction | Routing contract | Exact behavior |
|---|---|---|---|
| `symmetry` | Post-route missing-mate mirroring | No matching-pair contract here | Corresponding pair pin indices can mirror one nonempty tree into an empty mate. Also used only by feedback matched-delta extraction when explicitly passed there. |
| `cc` | No | No | Placement/cell-generation concern. |
| `proximity` | No | No | Placement concern. |
| `isolation` | No | No | Placement concern. |
| `thermal` | No | No | Placement concern. |
| `stress` | No | No | Placement concern. |
| `dti` | No | No | Placement/cell concern. |
| `guard_ring` | No direct record read | No | The integrated flow adds ring terminal points to `pin_pos`; the standalone router does not inspect guard-ring records. |
| `environment` | No | No | Not consumed. |
| `lde`, `dummy`, `unitization`, `aging` | No | No | Cell/signoff concerns. |
| `net_class` | Yes | No | Sets net weight/order. Other fields such as preferred layers and shielding are ignored. |
| `crosstalk` | No path cost/blockage effect | Yes | Same-layer route-tree node clearance is judged during detailed epochs. |
| `straight` | Yes | Yes | Routes earlier; placement alignment is external. Final perpendicular wire spread is judged. |
| `parasitic` | Yes | Yes | `max_r` creates a detailed Dijkstra cost cap; post-route R/C is judged only with `wire_params`. Also drives feedback weighting. |
| `differential` | No coupled search | Yes | Post-route length/layer and optional R/C deltas are judged. |
| `antenna` | No | No | Not consumed by this router. |
| `current_flow` | No | No | Not consumed. |
| `esd` | No | No | Not consumed. |
| `bias_current` | No | No | Not consumed. |

Only crosstalk, parasitic, differential, and straight records contribute to
`RoutingReport::validation.total`.

### Routing contract metadata

| Input | Contract kind / ID | Strength | Priority |
|---|---|---|---:|
| Crosstalk exclusion | `crosstalk_exclusion`, `xtalk_{net_a}_{net_b}` | Hard | `70` |
| Parasitic budget | `parasitic_budget`, `parasitic_{net}` | Soft | `60` |
| Differential pair | `differential_pair`, `diff_{net_pos}_{net_neg}` | Hard | `80` |
| Straight net | `straight_net`, `straight_{net}` | Soft | `50` |

The contract vector itself is private to the routing result path; callers
receive aggregate validation and `contract_lines`, not owned contracts or
their source/history fields.

## Contract construction and unresolved states

The router first compacts its net set. A routing contract is consumed only when
all of its named nets resolve into that compact set. A missing, empty, or
obstacle-only name leaves the contract `Emitted` and adds it to validation’s
missing-consumer list.

Consumption does not guarantee judgment:

- crosstalk stays `Consumed` when either tree is empty;
- a parasitic contract stays `Consumed` when no `wire_params` were supplied;
- a differential contract stays `Consumed` when either returned wire length is
  zero;
- a straight contract stays `Consumed` when returned wire length is zero.

`ContractValidation::hard_violations` includes only hard contracts whose final
status is `Violated`; hard `Emitted`/`Consumed` contracts must be checked
through lifecycle counts.

## Crosstalk exclusion

For each valid pair, `min_spacing_um` becomes `min_nm`. During detailed routing
the ledger finds the minimum Euclidean centre-to-centre distance between nodes
of the two route trees that lie on the same layer.

- If the nets use no common layer, clearance is treated as effectively
  infinite.
- Wire width, endpoint shape, vias, landing stubs, and coupling are ignored.
- Satisfaction is `clearance >= min_nm`.
- Violation metric is `(min_nm - clearance) / 1,000` µm.

Hard failing pairs contribute to the ledger open count but do not alter edge
cost or legality. The detailed-stage ledger is last reconciled before symmetry
mirroring and EOL/PRL repairs. It is not reconciled again afterward, so final
`crosstalk_violations` and validation describe the last detailed PathFinder
state, not necessarily the returned post-repair trees.

Only crosstalk checks participate in the detailed stage’s live `open` count.
Differential constraints are hard too, but they are first judged from extracted
geometry after PathFinder has already stopped.

`RoutingResult::crosstalk_violations` converts a violated contract’s metric
back to `(net_a_name, net_b_name, shortfall_nm)`.

## Parasitic budget

The search-time `max_r` cap is documented in
[Search-time parasitic length bound](model-and-algorithms.md#search-time-parasitic-length-bound).
Post-route judgment requires `RoutingConfig::wire_params`.

For a routed net estimate `(R, C)` and budget `(max_r, max_c)`:

```text
over_r = if max_r > 0 { R/max_r - 1 } else { 0 }
over_c = if max_c > 0 { C/max_c - 1 } else { 0 }
worst  = max(over_r, over_c)
```

`worst <= 0` satisfies the soft contract; otherwise it violates with
`worst * 100` and units `% over budget`. A compact but unrouted net has zero
returned wire geometry; with parameters present its estimated R/C is zero and
can therefore satisfy the budget even though `unrouted` separately reports the
failure.

## Differential pair

Returned Manhattan wire lengths are compared using symmetric percent delta:

```text
delta_pct(a,b) = abs(a-b) / ((a+b)/2) * 100
```

Zero average gives zero delta. If either net has zero wire length, the contract
remains consumed and unresolved. Otherwise the worst excess above
`max_length_delta_pct` is evaluated. When `wire_params` exist, excess R and C
deltas above their budgets join that maximum; without parameters the R/C
limits are not judged.

If `same_layer_required`, the two sorted sets of layers containing wires must
be exactly equal. This checks set equality, not per-segment correspondence or
via-count equality. A layer-set mismatch takes precedence and records the size
of the symmetric set difference in `asymmetric layers`. Otherwise a positive
worst excess violates in `% over budget`; a nonpositive excess satisfies.

The router does not route the two nets as a coupled pair. Symmetry mirroring
only recovers a route when exactly one mate tree is empty.

## Straight net

The contract measures the complete returned-wire envelope perpendicular to the
requested direction:

- vertical net: `max(all x endpoints) - min(all x endpoints)`;
- horizontal net: `max(all y endpoints) - min(all y endpoints)`.

It satisfies when spread is no more than configured detailed `pitch`; otherwise
the violation is `(spread - pitch)/1,000` µm. It does not require a single
segment, require every segment to follow the named direction, or include the
pin-to-landing stubs absent from `RoutingResult::wires`.

## Routing R/C model

Both geometry contract closure and optional feedback use the same analytical
model. For each wire on layer `l`:

```text
length_nm = abs(x1-x0) + abs(y1-y0)
width_nm  = max(width, 1)                 // resistance only
R += sheet_r[l] * length_nm / width_nm

length_um = length_nm / 1,000
width_um  = width_nm / 1,000
C_aF += area_cap[l] * length_um * width_um
C_aF += fringe_cap[l] * 2 * (length_um + width_um)
```

Every returned via adds the scalar `via_r`; via capacitance is not modeled.
Finally `C_aF / 1,000` is returned as fF. A missing layer entry omits that
specific R, area-C, or fringe-C term. Landing geometry is absent, coupling
capacitance is absent, and all vias share one resistance value.

## Feedback entry points

```rust
pub fn extract_feedback(
    result: &RoutingResult,
    placement: &Placement,
    parasitic_budgets: &[ParasiticBudget],
    weight_cap: f64,
    inflation_cap: f64,
) -> RoutingFeedback
```

This convenience form has no prior weights, no R/C parameters, no symmetry
groups, and no device/net closure. It delegates to
`extract_feedback_with_prior`.

```rust
pub fn extract_feedback_with_prior(
    result: &RoutingResult,
    placement: &Placement,
    parasitic_budgets: &[ParasiticBudget],
    weight_cap: f64,
    inflation_cap: f64,
    prior: Option<&HashMap<String, f64>>,
    wire_params: Option<&WireParasiticParams>,
    symmetry: &[SymmetryGroup],
    device_names: &[String],
    net_of_device: &dyn Fn(u32, &str) -> Option<usize>,
) -> RoutingFeedback
```

`weight_cap` and `inflation_cap` must be finite and at least 1 because they are
used as upper bounds to `clamp(1, cap)`.

## Per-net weight feedback

First, returned wire Manhattan length is summed per result net. It excludes
landing stubs and vias. Base feedback weight is:

```text
if HPWL > 0: clamp(actual_wire_length / net_hpwl, 1, weight_cap)
else:        1
```

For every matching parasitic budget:

- with `wire_params`, multiply by the larger of
  `max(R/max_r, 1)` and `max(C/max_c, 1)` for positive budgets and positive
  estimates;
- without `wire_params`, multiply by 2 unconditionally.

The result is capped at `weight_cap`. Duplicate budgets can apply multiple
boosts. The convenience `extract_feedback` therefore doubles every named
budget net even though it cannot estimate R/C.

When `prior` exists, exact-name previous weight defaults to 1 and the result
becomes `clamp(0.7 * current + 0.3 * old, 1, weight_cap)`.

## Directional cell inflation

Pressure arrays begin at zero for every placed cell.

- Every hotspot contributes to cells whose Manhattan distance from the cell’s
  drawn rectangle is at most `3 * track_pitch`. Contribution is
  `history / (1 + distance/pitch)`. Even layers add x pressure; odd layers add
  y pressure.
- Every successful landing adds normalized absolute x/y displacement to the
  closest cell rectangle.
- Every landing failure adds 4 to both axes of the closest cell.

The normalization pitch is `max(result.track_pitch, 1)`, where
`result.track_pitch` is the configured rather than necessarily actual graph
pitch.

For each axis independently:

```text
peak      = max pressure
mean      = sum pressure / max(cell_count, 1)
threshold = max(1.5 * mean, 1)
```

Only a cell with `pressure > threshold` and `peak > threshold` inflates:

```text
factor = clamp(
    1 + 0.30 * (pressure-threshold)/(peak-threshold),
    1,
    inflation_cap,
)
```

Thus the algorithm itself never requests more than 1.30 even when
`inflation_cap` is larger.

## Net order feedback

`net_order_priority` contains `(name, actual_length/HPWL)`, or ratio 1 for zero
HPWL, sorted descending by ratio. Feeding this map into
`RoutingConfig::net_priority_overrides` makes high-detour nets route earlier in
the next iteration.

## Matched R/C deltas

Matched-delta extraction considers only symmetry pairs at tier `Moderate` or
`Exceptional` and only pin-name strings `G`, `D`, and `S`. The callback maps
`(device_id, pin_name)` to a result-net index. Shared nets and missing mappings
are skipped.

For distinct nets it computes symmetric R and C percent deltas. A record is
omitted only when both deltas are below 0.1%. `MatchedDelta` returns device/net
names, both R/C values, and both percentages. This is feedback information;
the routing ledger’s differential contracts are a separate mechanism.

## `RoutingFeedback` fields

| Field | Meaning |
|---|---|
| `net_weights` | Per-result-net detour/parasitic multipliers. |
| `cell_inflation_x`, `cell_inflation_y` | Per-placement-cell virtual footprint factors. |
| `max_weight` | Maximum feedback weight, floored at 1 by initialization. |
| `clean` | True only when `report.unrouted` is empty and `report.overuse == 0`; contracts and post-route DRC heuristics are not included. |
| `net_order_priority` | Descending detour ratio list for the next route order. |
| `constraint_adjustments` | Always empty in the current extractor. The integrated flow uses `RoutingResult::crosstalk_violations` separately. |
| `net_r_ohm`, `net_c_ff` | R/C estimates, or all zeros when no feedback `wire_params` were supplied. |
| `matched_deltas` | Significant matched-device pin-net R/C asymmetries. |
| `spread_required` | True only for final track overuse or a nonempty `no_path`; pin-access-only failures do not request global spreading. |

The production feedback loop feeds weights/inflation into placement and detour
order/history into the next routing run. It also places
`RoutingResult::crosstalk_violations` into the generic placement-adjustment
channel, but that channel expects device names while crosstalk tuples contain
net names. Ordinary net names therefore do not currently resolve into device
isolation constraints. Device-layer DRC feedback independently adds correctly
named device-pair adjustments. Pin-access failures generate local cell-variant
hints so they do not automatically halve global utilization.
