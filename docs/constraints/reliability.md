# Reliability Constraints

Aging mechanisms, electromigration, ESD protection, and mechanical stress.

---

## 1. Electromigration (EM)

**The single highest-impact gap across all audits.** The router currently has zero current-awareness.

### 1.1 Black's law

Mean time to failure:

    MTTF = A / J^n * exp(Ea / (kT))

where J is current density, n ~ 2, Ea is activation energy, T is temperature. Minimum wire width:

    W_min = I_max / (J_max * t_min)

- **Books**: AOAL ch02/2.7.1, ch05/5.1, ch13/13.1 (#10, 3 sections), FOLD 7.5/7.5.1 (#29, full section), ALS 4.7 (#19), PNR_ANALOG 00/4.1 (#20), PNR_ANALOG 05/10.A (#74)
- **Rust**: No dedicated type. `ParasiticBudget` tracks R/C but not current capacity.
- **Status**: Planned. Proposed `ElectromigrationConstraint` in `routing_level/electromigration.rs` (gap analysis 1.1, **P0 priority**).

### 1.2 ElectromigrationConstraint (proposed)

| Field | Type | Description |
|-------|------|-------------|
| `net_name` | `String` | Target net |
| `max_current_density_a_per_um2` | `f64` | Black's law J_max per layer |
| `min_wire_width_um` | `f64` | W_min for EM compliance |
| `blech_length_um` | `Option<f64>` | Immortality threshold J*L < (J*L)_Blech |
| `temperature_derate_factor` | `f64` | Arrhenius derating |
| `direction` | `CurrentFlowDir` | Unidirectional worse than bidirectional |

- **Impl trait**: `NetConstraint` + `Contractable`
- **Consumed by**: Routing (wire sizing, topology), Signoff (EM verification)

### 1.3 Blech length immortality

If J * L < (J*L)_Blech, the segment is immortal regardless of current density (back-stress prevents void growth). Allows narrow short segments.

- **Books**: FOLD 7.5/7.5.1 (#29), PNR_ANALOG 00/4.1
- **Rust**: Proposed `blech_length_um` field on `ElectromigrationConstraint`.

### 1.4 Unidirectional vs bidirectional

Unidirectional DC current is far more damaging than AC/bidirectional. Analog power nets with unidirectional current are most at risk.

- **Books**: FOLD 7.5/7.5.1 (#29), PNR_ANALOG 00/4.1
- **Rust**: Proposed `direction: CurrentFlowDir` field. `BiasCurrentTag` provides current magnitude.

### 1.5 EM-aware routing

90-degree corner bends cause significantly higher current density than 135-degree bends. EM-aware topologies split currents into parallel paths.

- **Books**: FOLD 7.5/7.5.1 (#30-31)
- **Rust**: `StraightNet` enforces straight routing. No obtuse-bend preference for non-straight high-current nets.
- **Status**: Engine concern.

---

## 2. Via Reliability

### 2.1 Stress-induced voiding

Single vias fail from stress-induced voiding. Minimum via count >= 2 per transition.

- **Books**: AOAL ch15/15.4 (#35), FOLD 7.5/7.5.4 (#34)
- **Rust**: Proposed `ViaReliabilityConstraint` in `routing_level/via_reliability.rs` (gap analysis 1.2).
- **Status**: Planned.

### 2.2 Via orientation

Via-below vs via-above preference for EM robustness. Via arrays must be placed in-line with current direction, not at corners.

- **Books**: FOLD 7.5/7.5.4 (#34)
- **Rust**: Proposed fields `prefer_via_below`, `in_line_with_current` on `ViaReliabilityConstraint`.

---

## 3. Aging Mechanisms

### 3.1 Aging constraint type

- **Rust**: `AgingConstraint` in `cell_level/aging.rs`
- **Status**: Implemented (thin wrapper). Fields: `mechanism: AgingMechanism`, `severity: String`, `description: String`. Needs mechanism-specific quantitative fields (gap analysis 2.4).

### 3.2 Mechanism-specific gaps

| Mechanism | Key Missing Fields | Books | Status |
|-----------|--------------------|-------|--------|
| HCI | `max_vds_operating_v`, `critical_vgs_vds_ratio` (max at 0.4) | AOAL #39a | Planned |
| TDDB | `oxide_area_um2`, `max_field_mv_per_cm` (Weibull derating) | AOAL #39b | Planned |
| GOI | `max_gettering_distance_um` (~200 um to N+/NBL) | AOAL #39c | Planned |
| NBTI | `matched_pair_id` (cross-ref for differential drift) | AOAL #39d/f | Planned |

- **Books**: AOAL ch01/1.4.1, ch04/4.2, ch05/5.1-5.3, ch12/12.1.3, ch13/13.2.3 (#39), FOLD 7.2/7.2.2 (#18)

### 3.3 Missing AgingMechanism variants

Current: `Hci | Nbti | Tddb | Goi` (4 variants).

Add: `MobileIon | BjtAvalanche | Gidl` (AOAL #39e, gap analysis 2.5).

### 3.4 NBTI trigger conditions

NBTI for matched PMOS with differential Vgs > 100 mV. HCI for NMOS at Vgs ~ 0.4 * Vds.

- **Books**: PNR_ANALOG 02/2.A (#43)
- **Rust**: `AgingConstraint` with `AgingMechanism::Nbti` / `Hci`. Trigger conditions are extraction-engine logic, not constraint fields.

### 3.5 LDD as HCI countermeasure

Lightly Doped Drain (LDD) is the primary HCI countermeasure -- a process/cell-gen concern.

- **Books**: FOLD 7.2/7.2.2 (#19)
- **Rust**: Process feature, not layout constraint. Could enrich `AgingConstraint` with `min_channel_length_um`.
- **Status**: Out of scope (process feature).

---

## 4. ESD Protection

### 4.1 ESD constraint type

- **Books**: AOAL ch01/1.4.2, ch05/5.1-5.4, ch06/6.4, ch11/11.2.1, ch14/14.4 (#48), FOLD 7.4 (#24-28), PNR_ANALOG 02/5.A.1 (#54)
- **Rust**: `EsdConstraint` and `EsdRequirement` in `metadata/esd.rs`
- **Status**: Implemented. `EsdConstraint` has `max_bus_resistance_ohm` and protection types.

### 4.2 ESD design window (missing)

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `trigger_voltage_v` | ESD device turn-on | FOLD #24 | Planned |
| `holding_voltage_v` | Post-trigger sustain | FOLD #24 | Planned |
| `destruction_voltage_v` | Device damage threshold | FOLD #24 | Planned |
| `min_esd_wire_width_um` | Wire width for 100 mA ESD current | FOLD #25 | Planned |
| `cdm_proximity_um` | CDM clamp max 50 um from gate | PNR_ANALOG #54 | Planned |

### 4.3 Cross-domain ESD coupling

Antiparallel diodes between separate ground lines, pin grouping by domain. No constraint type models cross-domain ESD coupling.

- **Books**: FOLD 7.4/7.4.1 (#26)
- **Rust**: Proposed `PowerDomainEsd` type (gap analysis 1.13).
- **Status**: Planned (lower priority).

### 4.4 ESD metal rules

45-degree chamfers at inside metal corners, properly oriented slots, minimum width to limit adiabatic heating to ~200C during ESD.

- **Books**: AOAL ch14/14.4 (#13)
- **Rust**: No type. DRC-level concern.
- **Status**: Out of scope (DRC rule).

---

## 5. Mechanical Stress

### 5.1 Stress constraint type

- **Books**: AOAL ch02/2.8, ch08/8.2.8, ch10/10.2.4, ch13/13.2.5 (#61), FOLD 6.6/6.6.3-6.6.4 (#7/#9), PNR_ANALOG 00/2.3 (#11-12)
- **Rust**: `StressConstraint` in `cell_level/stress.rs`
- **Status**: Implemented. Field: `max_centroid_distance_um`. 6 enrichment gaps (gap analysis 2.10).

### 5.2 Die-position thresholds

| Tier | Placement Rule | max_centroid_distance_um |
|------|---------------|------------------------|
| Minimal | >= 200 um from die edge | Computed |
| Moderate | >= 500 um from die edge | Computed |
| Exceptional | Central half of die | Computed |

- **Books**: PNR_ANALOG 00/2.3 (#12), PNR_ANALOG 02/4.A.3 (#49)

### 5.3 Missing fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `sti_trench_proximity` | STI stress alters carrier mobility | FOLD #7 | Planned |
| `piezoresistance_type` | P-type 10x more sensitive than N-type | AOAL #61b | Planned |
| `die_zone` | Central50/Central75/Edge | AOAL #61f | Planned |
| `l_shaped_cancellation` | L-shaped segments cancel stress | AOAL #61d | Planned |

### 5.4 Package stress

Package-induced stress is position-dependent: worst at edges/corners, lowest at center. CTE mismatch between die and package material. Die aspect ratio <= 1.5:1 for plastic packages.

- **Books**: AOAL ch02/2.8 (#61a), FOLD 6.6/6.6.4 (#9)
- **Rust**: `StressConstraint.max_centroid_distance_um` captures distance from center. No package model, CTE factor, or corner-vs-edge anisotropy.
- **Status**: Implemented (distance only). Package model out of scope.

### 5.5 STI trench proximity stress

STI compressive stress increases hole mobility, tensile stress decreases electron mobility. Affects matching of devices near STI edges.

- **Books**: FOLD 6.6/6.6.3 (#7), PNR_ANALOG 00/2.2 (#10)
- **Rust**: `LdeBound` SA/SB fields handle LOD/STI at the LDE level. `StressConstraint` has no STI-proximity field.
- **Status**: `LdeBound` handles the LOD mismatch component. Separate STI stress field planned.

### 5.6 Interconnect stress voiding

CTE mismatch and packaging deformation cause void nucleation in interconnects.

- **Books**: FOLD 7.5/7.5.3 (#33)
- **Rust**: `StressConstraint` is device-centric. No interconnect-level stress voiding model.
- **Status**: Out of scope (interconnect reliability, partially covered by `ViaReliabilityConstraint`).

---

## 6. Safe Operating Area (SOA)

### 6.1 Electrical SOA

Snapback trigger/sustain voltage limits for power MOS devices. Operating beyond sustain voltage in cutoff risks destruction.

- **Books**: AOAL ch13/13.1 (#27), AOAL ch12/12.1.4 (#32)
- **Rust**: Proposed `SafeOperatingArea` in `cell_level/safe_operating_area.rs` (gap analysis 1.11).
- **Status**: Planned.

### 6.2 Electrothermal SOA (Spirito effect)

Thermal instability in power devices causing current localization.

- **Books**: AOAL ch13/13.1 (#27)
- **Rust**: Proposed `spirito_boundary: bool` field on `SafeOperatingArea`.

### 6.3 Ballast resistance

Power devices require series ballast resistance in each finger/segment to prevent current localization and thermal runaway.

- **Books**: AOAL ch05/5.1 (#2)
- **Rust**: Proposed `BallastRequirement` type (gap analysis 1.13).
- **Status**: Planned (lower priority, power device specialty).

---

## 7. Antenna Effect

### 7.1 Antenna constraint type

Antenna ratio limits prevent charge accumulation on gate oxide during fabrication.

- **Books**: AOAL ch05/5.1, ch15/15.5 (#41), FOLD 7.4/7.4.2 (#27-28), PNR_ANALOG 00/4.5 (#24)
- **Rust**: `AntennaConstraint` in `routing_level/antenna.rs`
- **Status**: Implemented. Field: `max_ratio`.

### 7.2 Missing fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `max_perimeter_ratio` | P_ant/A_gate (dominant during RIE) | FOLD #27, AOAL #41a | Planned |
| `repair_preference` | Jumper vs leaker (antenna diode) | FOLD #28, AOAL #41d | Planned |

### 7.3 Antenna ratio types

Two distinct ratios:

| Ratio | Limit | Process Phase | Books |
|-------|-------|---------------|-------|
| Area ratio (A_metal / A_gate) | ~400-1000 | Metal etch | PNR_ANALOG #24 |
| Perimeter ratio (P_metal / A_gate) | ~200 | Poly etch (peripheral) | FOLD #27, AOAL #41a |

### 7.4 Repair methods

- **Jumper**: Metal layer break to reduce antenna area
- **Leaker**: Antenna diode to discharge accumulated charge
- **PSD antenna diodes**: Require UV access keepout

- **Books**: FOLD 7.4/7.4.2 (#28), AOAL ch05/5.1 (#41c)
- **Rust**: `AntennaConstraint.matched_symmetric_repair` controls repair strategy. No explicit jumper vs leaker preference.
