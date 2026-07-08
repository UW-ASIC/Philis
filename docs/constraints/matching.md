# Matching Constraints

Rules for device matching, routing matching, and cell-level matching techniques.

---

## 1. Pelgrom Mismatch Model

Random mismatch between identically designed devices follows Pelgrom's law:

    sigma^2(dP) = A_P^2 / (W * L) + S_P^2 * D^2

where A_P is the area coefficient, S_P is the distance coefficient, and D is device separation.

- **Books**: AOAL ch13/13.2.1, FOLD 6.6/6.6.2, ALS 2.2.2, PNR_ANALOG 00/1.1
- **Rust**: `MatchingPair` (fields `max_dvth_mv`, `max_did_pct`) in `placement_level/symmetry.rs`
- **Status**: Implemented. Missing Pelgrom coefficients (`pelgrom_a_vt`, `pelgrom_s_vt`) on both `MatchingPair` and `MatchingSpec` -- planned (gap analysis 2.7/2.9).

### 1.1 Pocket-implant modified scaling

Pocket/halo implants cap the effective L in Pelgrom's formula at L_C. Devices with pocket implants must never be rotated 90 degrees (directional implant asymmetry).

- **Books**: AOAL ch12/12.2.7 (#26 -- "most consequential absent analog matching constraint"), PNR_ANALOG 00/1.2, PNR_ANALOG 01/3.A.3
- **Rust**: Partially handled by `MatchingSpec.allowed_transforms`. Proposed `PocketImplantExclusion` in `cell_level/pocket_implant.rs` (gap analysis 1.8).
- **Status**: Planned. `MatchingSpec.allowed_transforms` blocks rotations; dedicated `PocketImplantExclusion` with `effective_l_cap_um` not yet implemented.

### 1.2 BJT-specific matching

BJT area coefficient K_A differs from MOSFET A_VT. Lateral PNP requires field plates and NBL shadow clearance. Diode-connected transistors require beta-plateau awareness.

- **Books**: AOAL ch10/10.2-10.3 (#53), AOAL ch11/11.3 (14 diode matching rules), PNR_ANALOG 00/1.1
- **Rust**: `MatchingPair` in `placement_level/symmetry.rs`
- **Status**: Implemented (basic). Missing `pelgrom_k_a`, `emitter_area_um2`, `vce_equalization` fields -- planned (gap analysis 2.9).

### 1.3 Schottky and Zener matching limits

Schottky matching depends on silicide type (noble > CoSi2 > NiSi >> Al). Zener diodes do NOT follow Pelgrom ("winner takes all" breakdown); surface Zeners limited to 50-100 mV matching.

- **Books**: AOAL ch11/11.2.3 (#29), AOAL ch11/11.3.2 (#38)
- **Rust**: No dedicated type. Would enrich `MatchingPair` with device-type-specific rule sets.
- **Status**: Out of scope (narrow device types; document for reference).

---

## 2. Matching Tiers

Three tiers with quantitative targets (Hastings):

| Tier | Voltage | Current | Description |
|------|---------|---------|-------------|
| Minimal | +/-0.1-1% | +/-0.1-1% | Basic ratio accuracy |
| Moderate | +/-0.01-0.1% | +/-0.01-0.1% | Precision analog |
| Exceptional | +/-0.001-0.01% | +/-0.001-0.01% | Bandgap, ADC reference |

- **Books**: AOAL ch08/8.1 (#55), PNR_ANALOG 00/1.4
- **Rust**: `MatchingTier` enum in `types.rs`
- **Status**: Implemented (enum). Missing numeric threshold attachment -- planned (gap analysis 2.8).

---

## 3. Systematic Mismatch Sources (Ranked)

PNR_ANALOG 00/1.3 ranks nine systematic mismatch sources by magnitude:

1. **Orientation mismatch** (~15% gm error) -- `MatchingSpec.orientation_rule`
2. **Well proximity effect (WPE)** -- `LdeBound.min_well_edge_distance_um`
3. **Length-of-diffusion (LOD/STI stress)** -- `LdeBound` SA/SB fields
4. **Mechanical stress (die position)** -- `StressConstraint.max_centroid_distance_um`
5. **Thermal gradient** -- `ThermalGradientConstraint.max_delta_c`
6. **Hydrogenation** -- `EnvironmentalConstraint` (MetalOverGate, HydrogenationKeepout)
7. **Conductivity modulation** -- proposed `ConductivityModulationGuard`
8. **NBL shadow** -- proposed `NblShadowClearance`
9. **Aging drift** -- `AgingConstraint` (NBTI, HCI)

- **Books**: PNR_ANALOG 00/1.3, AOAL ch13/13.2
- **Rust**: Multiple types across `cell_level/`, `placement_level/`
- **Status**: Sources 1-6 implemented. Sources 7-9 planned or partially implemented.

---

## 4. Common-Centroid Layout

Five rules of common-centroid layout (Hastings):

1. **Coincidence**: COG(A) = COG(B)
2. **Symmetry**: Mirror symmetry about centroid axis
3. **Dispersion**: Distribute units across array, not clustered
4. **Compactness**: Minimize total array footprint
5. **Orientation**: All units same current-flow direction (chi metric)

- **Books**: AOAL ch08/8.2.7, AOAL ch13/13.2.6 (#42), FOLD 6.6/6.6.2 (#4), ALS 3.1/3.1.2 (#3), PNR_ANALOG 00/1.5, PNR_ANALOG 01/4.A
- **Rust**: `CcGroup` in `placement_level/cc.rs`, `PatternType` enum in `types.rs`
- **Status**: Implemented (Abba/Abab/CommonCentroid2d patterns). Missing: dispersion metric, compactness score, `centroid_tolerance_um`, `gradient_cancellation_order`, `residual_gradient_factor` -- planned (gap analysis 2.15).

### 4.1 Pattern types for non-unity ratios

| Pattern | Ratio | Books |
|---------|-------|-------|
| ABA | 2:1 | AOAL ch08 (#64d) |
| ABABA | 3:2 | AOAL ch08 (#64d) |
| AABAABAA | 3:1 | AOAL ch08 (#64d) |
| EightAroundOne (3x3 2D) | 8:1 | AOAL ch08 (#64d) |

- **Rust**: `PatternType` enum in `types.rs`
- **Status**: Planned (gap analysis 2.20). Currently only Abba/Abab/CommonCentroid2d.

### 4.2 2D vs 1D common centroid

2D CC achieves ~60% residual mismatch of 1D ABBA for the same device count.

- **Books**: AOAL ch13/13.2.6 (#42c), PNR_ANALOG 01/4.A
- **Rust**: `CcGroup` field `residual_gradient_factor` -- planned (gap analysis 2.15).
- **Status**: Planned.

---

## 5. Orientation Chi Metric

The orientation chi metric quantifies current-flow asymmetry per multi-finger device:

    chi = (1/N) * sum(chi_i)

where chi_i = +1 or -1 depending on current direction in each finger. chi = 0 means perfectly balanced.

- **Books**: PNR_ANALOG 00/1.5 (#6), AOAL ch13/13.2
- **Rust**: `CurrentFlowTag` in `routing_level/current_flow.rs` (direction enum only). Proposed `orientation_chi: Option<f64>` field (gap analysis 2.26).
- **Status**: Planned. `CurrentFlowTag` has direction enum but not the computed float chi.

---

## 6. Matching Rules (Hastings)

### 6.1 MOS matching (24 rules)

Key rules with constraint mappings:

| Rule | Description | Rust Type | Status |
|------|-------------|-----------|--------|
| Same orientation | All matched devices same crystal direction | `MatchingSpec.orientation_rule` | Implemented |
| Same W, same L | Identical dimensions | `MatchingPair.w_ratio` | Implemented |
| CC layout | Common-centroid for moderate+ | `CcGroup` | Implemented |
| Dummies | Gate/moat dummies per tier | `DummyConstraint` | Implemented |
| No contacts on active gate | | `EnvironmentalConstraint` | Implemented |
| Extend poly >= 0.5 um | | `MatchingSpec.orientation_rule` (string) | Implemented (string, not typed) |
| Metal straps for gates | | `MatchingSpec` | Implemented (string) |
| Kelvin connections | Separate force/sense leads | Proposed `KelvinConnectionConstraint` | Planned |
| 45-degree PMOS orientation | Exceptional matching under stress | `MatchingSpec.allowed_transforms` | Implemented |
| Moat extension >= 5 um | Exceptional matching | `DummyConstraint.moat_ext_um` | Implemented |

- **Books**: AOAL ch13/13.2, PNR_ANALOG 00/1.6-1.7

### 6.2 Resistor matching (25 rules)

Key rules:

| Rule | Description | Rust Type | Status |
|------|-------------|-----------|--------|
| Same material | All segments same resistor type | `UnitizationConstraint` | Implemented |
| Same width, sufficient area | Per-tier minima | `UnitizationConstraint.unit_geometry` | Implemented |
| Even segment count | Thermoelectric cancellation | Proposed `even_segment_count` field | Planned |
| Identical segments | All unit elements same geometry | `UnitizationConstraint` | Implemented |
| Body bias matching | Same tank/body voltage | Proposed `ConductivityModulationGuard` | Planned |
| Field plates for high-Rsh | Rsh >= 1k ohm/sq | Proposed `FieldPlateRequirement` | Planned |
| Kelvin connections | 4-wire for precision | Proposed `KelvinConnectionConstraint` | Planned |

- **Books**: AOAL ch08/8.2, PNR_ANALOG 01/7.A

### 6.3 Capacitor matching (13 rules)

Key rules:

| Rule | Description | Rust Type | Status |
|------|-------------|-----------|--------|
| Square geometry | Unit caps must be square | Proposed `square_aspect_ratio` field | Planned |
| Dummies on all 4 sides | 2D surround | `DummyConstraint` (needs `surround_2d`) | Planned |
| Electrostatic shielding | Shield plates above/below | `ShieldType` | Implemented |
| CC layout | Common-centroid for moderate+ | `CcGroup` | Implemented |
| Bias overdrive | >= 0.5-1V beyond Vfb/Vth | Proposed `CapBiasConstraint` | Out of scope (circuit-level) |

- **Books**: AOAL ch04/4.2, AOAL ch07/7.1, PNR_ANALOG 00/1.9

---

## 7. Route Matching

### 7.1 Matched net routing tolerances

| Parameter | Moderate | Exceptional | Rust Field |
|-----------|----------|-------------|------------|
| Length delta | < 5% | < 1% | `RouteMatchingTolerance.max_length_delta_pct` |
| R delta | < 5% | < 1% | `RouteMatchingTolerance.max_r_delta_pct` |
| C delta | < 10% | < 2% | `RouteMatchingTolerance.max_c_delta_pct` |
| Same layer | Required | Required | `RouteMatchingTolerance.same_layer_required` |
| Same via count | Required | Required | `RouteMatchingTolerance.same_via_count_required` |

- **Books**: PNR_ANALOG 00/3.4, PNR_ANALOG 05/3.A, AOAL ch08/8.2.4 (#60)
- **Rust**: `RouteMatchingTolerance` in `placement_level/matching_spec.rs`
- **Status**: Implemented. Missing symmetry-axis reference and jumper resistance equalization -- planned (gap analysis; ALS #12, AOAL #60).

### 7.2 Star node topology

All sensitive leads from matched devices must converge at a single common point to eliminate differential ground drops.

- **Books**: AOAL ch15/15.4 (#33), PNR_ANALOG 05/6.1.A
- **Rust**: Proposed `StarNodeConstraint` in `routing_level/star_node.rs` (gap analysis 1.4).
- **Status**: Planned.

### 7.3 Kelvin (four-wire) connections

Separate force and sense leads with matched sense-lead resistance. Force leads >= 2W beyond star connection.

- **Books**: AOAL ch06, AOAL ch15/15.4 (#18), PNR_ANALOG 01/7.A.4
- **Rust**: Proposed `KelvinConnectionConstraint` in `routing_level/kelvin.rs` (gap analysis 1.5).
- **Status**: Planned.

---

## 8. Symmetry Constraints

### 8.1 Mirror vs perfect symmetry

Mirror symmetry reflects devices about an axis. Perfect symmetry requires identical (not mirrored) orientations -- needed for anisotropic process effects like oblique-angle ion implantation.

- **Books**: ALS 1.1/1.1.4 (#1)
- **Rust**: `SymmetryGroup` in `placement_level/symmetry.rs`
- **Status**: Implemented (basic). Missing `symmetry_mode` (Mirror/Perfect) and `axis_direction` (Vertical/Horizontal) -- planned (gap analysis 2.14).

### 8.2 Hierarchical symmetry

Symmetry groups containing other symmetry groups (symmetry-of-groups).

- **Books**: ALS 2.8.1 (#37)
- **Rust**: `HsmpgNode` supports nesting but does not encode symmetry-of-symmetry semantics.
- **Status**: Planned (gap analysis 1.13, lower priority).

### 8.3 Multi-group axis alignment

Multiple symmetry groups sharing a common axis using zero-height dummy blocks.

- **Books**: ALS 2.7.1 (#36)
- **Rust**: Proposed `aligned_with: Option<String>` on `SymmetryGroup` (gap analysis 2.14).
- **Status**: Planned.

---

## 9. MatchingSpec (Comprehensive Aggregation)

`MatchingSpec` is the central aggregation type linking tier, orientation, shield policy, route tolerance, and dummy requirements for a matched device group. It is extracted but currently **not in the engine digest** -- placement/routing has no visibility into it.

- **Books**: AOAL ch08/8.2-8.3 (#54), FOLD 6.6 (#11), ALS 2.2 (#34), PNR_ANALOG 00/1.3-1.7 (#3/#39)
- **Rust**: `MatchingSpec` in `placement_level/matching_spec.rs`
- **Status**: Implemented (struct exists). 11 enrichment gaps identified (gap analysis 2.7). Not yet in engine digest (gap analysis 4).

### Key missing fields

| Field | Purpose | Books |
|-------|---------|-------|
| `same_backgate_net` | Backgate modulation control | AOAL #54a |
| `pelgrom_a_vt` / `pelgrom_s_vt` | Area/distance coefficients | AOAL #54b, ALS #34 |
| `kelvin_required` | 4-wire connection flag | PNR_ANALOG #39 |
| `seebeck_cancellation_required` | Even-segment antiparallel current | FOLD #10, PNR_ANALOG #22 |
| `cascode_recommended` | Cascode for Vds equalization | AOAL #54h |

Also: change `orientation_rule: String` to typed enum `OrientationRule { ChiZero, MatchedChi, Superimposable, GeometricMirror, CurrentPreserving }` (AOAL #54d, FOLD #11).
