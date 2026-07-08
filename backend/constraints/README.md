Constraint extraction pipeline for analog PNR. Owns `ConstraintRecord` and all constraint sub-types. Each sub-module implements one stage of the extraction
pipeline. Downstream crates (placement, routing, verify) depend on these types.

All constraint types reference devices by `DeviceId(u32)` for O(1) arena indexing. Use `ConstraintRecord::device_name()` to resolve back to SPICE instance
names.

## Extraction Stages

| Module          | Constraint Type                                | Description                                                                                  |
| --------------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `symmetry`      | `SymmetryGroup` / `MatchingPair`               | Matched device pairs, symmetry axes, w-ratios                                                |
| `matching`      | `MatchingTier` inference                       | Tier assignment (Exceptional/Moderate/Minimal) from topology + PDK characterization          |
| `matching_spec` | `MatchingSpec`                                 | Full AOAL-grade budgets (random, systematic, LDE, thermal, stress, routing tolerances)       |
| `classify`      | `NetClassification`                            | Net class (signal/clock/supply/sensitive), voltage domain, shielding, parasitic budgets      |
| `lde`           | `LdeBound`                                     | WPE + LOD mismatch budgets, per-finger SA/SB, STI dVth                                       |
| `parasitic`     | `ParasiticBudget`                              | Max R/C budgets per net                                                                      |
| `thermal`       | `ThermalTag` / `ThermalGradientConstraint`     | Power dissipation tags, min-distance-to-matched, pairwise delta-T bounds                     |
| `align`         | `StraightNet`                                  | Series-stack junctions that must route as one straight segment                               |
| `crosstalk`     | `CrosstalkExclusion`                           | Aggressor/victim net spacing                                                                 |
| `cc`            | `CcGroup`                                      | Common-centroid interdigitation patterns                                                     |
| `dummy`         | `DummyConstraint`                              | Edge dummy devices for LOD matching                                                          |
| `stress`        | `StressConstraint`                             | Max centroid distance from die center                                                        |
| `isolation`     | `IsolationConstraint` / `GuardRingRequirement` | Noisy/sensitive device separation, guard ring specs                                          |
| `esd`           | `EsdConstraint`                                | Pad-level ESD protection (primary/secondary clamp, CDM, silicide block)                      |
| `antenna`       | `AntennaConstraint`                            | Metal-to-gate area ratio limits                                                              |
| `aging`         | `AgingConstraint`                              | HCI, NBTI, TDDB, GOI reliability                                                             |
| `environment`   | `EnvironmentalConstraint`                      | WPE four-edge, LOD, dummy moat, hydrogenation keepout, metal-over-gate, thermal exclusion    |
| `unitization`   | `UnitizationConstraint`                        | Unit-element decomposition for interdigitation + ratio matching                              |
| `blocks`        | `AnalogBlock`                                  | Building block recognition (diff pair, mirror, cascode)                                      |
| `smp`           | `SmpEdge` / `HsmpgNode`                        | Symmetry-Matching-Proximity multigraph + hierarchical clustering                             |
| `contract`      | `ConstraintContract`                           | Lifecycle tracking (emitted -> consumed -> satisfied/violated/waived) across pipeline stages |

## Additional Types in ConstraintRecord

| Type                     | Description                                                 |
| ------------------------ | ----------------------------------------------------------- |
| `ProximityRule`          | Pairwise min/max spacing between devices                    |
| `BiasCurrentTag`         | Per-device drain current and overdrive voltage              |
| `CurrentFlowTag`         | Orientation constraint from current flow direction          |
| `EsdRequirement`         | Pad net ESD protection requirement (lightweight, pre-typed) |
| `DtiPair`                | Deep trench isolation pair constraint                       |
| `injection_risk_devices` | Devices with drain/source connected to pad-like nets        |
| `bias_currents`          | Peak drain bias current per net (amperes)                   |

## Entry Point

`extract_constraints(netlist, pdk, f_target) -> ConstraintRecord` runs all sub-stages in order and assembles the final record. ~30 distinct constraint types
total.
