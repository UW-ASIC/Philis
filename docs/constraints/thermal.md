# Thermal Constraints

Thermal gradients, self-heating, Seebeck effects, and thermal-aware placement.

---

## 1. Thermal Gradient Constraint

### 1.1 Maximum temperature delta

Per-tier temperature tolerance between matched devices:

| Tier | max_delta_c | Description |
|------|-------------|-------------|
| Minimal | 2.0 C | Basic matching |
| Moderate | 0.5 C | Precision analog |
| Exceptional | 0.1 C | Bandgap, ADC reference |

- **Books**: PNR_ANALOG 00/4.3, PNR_ANALOG 02/4.A.5 (#50), AOAL ch01/1.2, ch08/8.2.7 (#62), FOLD 6.6/6.6.4 (#8)
- **Rust**: `ThermalGradientConstraint` in `placement_level/thermal.rs`
- **Status**: Implemented. Fields: `max_delta_c`, `estimated_gradient_c`.

### 1.2 Thermal model

Temperature distribution via Green's function: T(x,y). Drives isotherm computation.

- **Books**: PNR_ANALOG 00/4.3, PNR_ANALOG 04/2.6 (#64)
- **Rust**: Engine computation consuming `ThermalTag` data.

### 1.3 Device-type thermal sensitivity

| Device | TC | Mismatch per 1C | Books |
|--------|-------|-----------------|-------|
| BJT | -2 mV/C Vbe | ~8% collector current | AOAL #62c |
| MOSFET | -1 to -2 mV/C Vth | ~2-4% Id | PNR_ANALOG 00/4.3 |
| Resistor | TCR-dependent | Material-specific | AOAL #62 |

- **Rust**: `ThermalGradientConstraint.max_delta_c` is device-type-agnostic. No per-device TC coefficient.
- **Status**: Implemented (generic). Device-type-specific TC coefficients not encoded.

### 1.4 Missing fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `seebeck_coefficient_mv_per_c` | Voltage error at contacts | AOAL #62a | Planned |
| `isotherm_alignment_required` | Place matched devices along isotherms | FOLD #8 | Planned |

---

## 2. Seebeck (Thermoelectric) Effect

### 2.1 Mechanism

At junctions between dissimilar conductors (e.g., Al-Si contacts), a temperature gradient produces a voltage:

    V = S * dT

where S ~ 50-500 uV/K for Al-Si contacts.

- **Books**: PNR_ANALOG 00/4.3 (#22), AOAL ch08/8.3.1 (#34), FOLD 6.6/6.6.5 (#10)
- **Rust**: No dedicated type. Proposed as field on `MatchingSpec` (`seebeck_cancellation_required`) or `UnitizationConstraint` (`even_segment_count`).
- **Status**: Planned.

### 2.2 Cancellation technique

Half of series segments must carry current in each direction (antiparallel flow) to cancel Seebeck potentials. Requires even segment count.

- **Books**: AOAL ch08/8.3.1 (#34 -- ThermoelectricCancellation), PNR_ANALOG 00/4.3
- **Rust**: Proposed `ThermoelectricCancellation` type or `even_segment_count` field on `UnitizationConstraint` (gap analysis 1.13 and 2.20).
- **Status**: Planned.

### 2.3 Current flow annotation

`CurrentFlowTag` has `CurrentFlowDir` (L-to-R, R-to-L, Bidirectional) but no `seebeck_cancellation: bool` or link to the CC group requiring it.

- **Books**: FOLD 6.6/6.6.5 (#10)
- **Rust**: `CurrentFlowTag` in `routing_level/current_flow.rs`. Proposed `seebeck_cancellation: Option<bool>` (gap analysis 2.26).
- **Status**: Planned.

---

## 3. Thermal Tag (Device-Level)

### 3.1 Power device annotation

Tags devices as power sources for thermal-aware placement. Minimum distance to matched devices.

- **Books**: AOAL ch01/1.1.1, ch08/8.2.7, ch15/15.2 (#63), PNR_ANALOG 04/2.6
- **Rust**: `ThermalTag` in `placement_level/thermal.rs`
- **Status**: Implemented. Fields: `is_power_device`, `power_dissipation_mw`, `min_distance_to_matched_um`.

### 3.2 Missing fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `thermal_via_requested` | Non-electrical metal/vias for heat conduction | FOLD #35 | Planned |
| `optical_sensitivity` | Bare-die circuits sensitive to light | AOAL #63a | Out of scope |
| `max_delta_t_c` | Computed from self-heating equations | AOAL #63b | Planned |
| `thermal_resistance_cw` | Device thermal resistance | AOAL #63b | Planned |

### 3.3 Heat source topology

Single, dual, and quad heat source configurations with preferred device positions at 1/2 to 3/4 die distance from the source.

- **Books**: AOAL ch08/8.2.7 (#63c), AOAL ch15/15.2
- **Rust**: `ThermalTag.min_distance_to_matched_um` is the only positioning field. No heat-source topology awareness.
- **Status**: Implemented (distance only). Topology-aware placement is an engine concern.

---

## 4. Thermal-Aware Placement

### 4.1 Isotherm-aligned placement

FOLD mandates placing matched devices along isotherms (lines of constant temperature).

- **Books**: FOLD 6.6/6.6.4 (#8)
- **Rust**: `ThermalGradientConstraint` bounds `max_delta_c` but has no explicit isotherm direction vector.
- **Status**: Planned. Requires thermal simulation model in the placement engine.

### 4.2 CC thermal cancellation

Common-centroid layouts cancel first-order thermal gradients when devices are symmetrically placed about the heat source.

- **Books**: PNR_ANALOG 04/2.6 (#64), AOAL ch08/8.2.7
- **Rust**: `CcGroup` pattern selection implicitly handles this.
- **Status**: Implemented (via CC layout; no explicit thermal term).

### 4.3 Interaction with matching

Thermal constraints interact with matching constraints: a thermal gradient across a matched pair degrades matching. The stage assignment of thermal terms (stage 1 topology vs stage 2 placement) needs validation.

- **Books**: AOAL ch08, FOLD ch7
- **Rust**: Both `ThermalGradientConstraint` and `MatchingPair` exist; their interaction is in the SA cost function.
- **Status**: Implemented (both constraints exist). Interaction weighting documented in `weights.md`.

---

## 5. Thermal Migration (Interconnect)

Mass transport from hot to cold in interconnects, distinct from electromigration.

- **Books**: FOLD 7.5/7.5.2 (#32)
- **Rust**: `ThermalGradientConstraint` addresses device matching, not interconnect-level thermal migration.
- **Status**: Out of scope (interconnect reliability, addressed in `reliability.md`).

---

## 6. Self-Heating

### 6.1 MOS trimming limitation

MOS offset trimming only removes ~2/3 of over-temperature variation. `trimmed_residual_fraction` not tracked.

- **Books**: AOAL ch08/8.2.7 (#62d)
- **Rust**: Not represented.
- **Status**: Out of scope (circuit-level trimming concern).

### 6.2 Thermal wires/vias

Non-electrical metal structures for heat conduction to reduce thermal gradients.

- **Books**: FOLD 7.5/7.5.5 (#35)
- **Rust**: `ThermalTag.is_power_device` flags hot devices. Proposed `thermal_via_requested` field (gap analysis 2.18).
- **Status**: Planned.
