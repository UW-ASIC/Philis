# Parasitic Constraints

Parasitic capacitance, resistance, inductance, and extraction considerations.

---

## 1. Parasitic Budget

Per-net budgets for maximum resistance, capacitance, and related quantities.

- **Books**: AOAL ch01/1.3.2, ch02/2.7.1, ch06/6.2, ch07/7.3, ch13/13.1 (#58), FOLD 7.3 (#20-21), ALS 4.4/4.5 (#14/#24), PNR_ANALOG 00/3.1-3.4 (#16-17)
- **Rust**: `ParasiticBudget` in `routing_level/parasitic.rs`
- **Status**: Implemented. Fields: `max_r`, `max_c`, net name. Enrichment gaps identified (gap analysis 2.13).

### 1.1 Existing fields

| Field | Description | Status |
|-------|-------------|--------|
| `net_name` | Target net | Implemented |
| `max_r` | Maximum path resistance (ohms) | Implemented |
| `max_c` | Maximum load capacitance (fF) | Implemented |

### 1.2 Missing fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `max_ir_drop_mv` | IR drop budget (>10 mV needs checking) | FOLD #20, ALS #29 | Planned |
| `max_coupling_c_ff` | Inter-net coupling capacitance budget | ALS #14 | Planned |
| `max_rc_delay_ps` | Combined RC delay target | FOLD #21 | Planned |

### 1.3 Parasitic budget derivation

AC-derived: C_par_max = 1 / (2 * pi * f_target * R_out)

IR-drop: < 5% Vsupply for analog, < 10% for digital.

- **Books**: PNR_ANALOG 00/3.1-3.2 (#16), PNR_ANALOG 00/3.3 (#17)
- **Rust**: Derivation formulas are not in the constraint type; they live in the constraint extraction engine.

---

## 2. IR Drop

### 2.1 Voltage-drop bound

Combines parasitic resistance with current to bound voltage drop on a routing path:

    V_drop = I_path * R_path < V_drop_max

- **Books**: ALS 7.3/7.4.4 (#29 -- IrDropConstraint), FOLD 7.3/7.3.1 (#20), PNR_ANALOG 00/3.3
- **Rust**: `ParasiticBudget.max_r` is related but does not combine with current. Proposed `IrDropConstraint` (gap analysis 1.13, routing_level).
- **Status**: Planned (lower priority). Requires `BiasCurrentTag` data for current values.

### 2.2 FOLD rule of thumb

> "IR drop > 10 mV requires checking."

- **Books**: FOLD 7.3/7.3.1 (#20)
- **Rust**: Proposed `max_ir_drop_mv: Option<f64>` on `ParasiticBudget` (gap analysis 2.13).

---

## 3. Parasitic Capacitance

### 3.1 Fringing capacitance

Fringing capacitance is ~60% of total for narrow leads. Must be included in extraction.

- **Books**: AOAL ch01/1.3.2 (#58c)
- **Rust**: Engine extraction concern; `ParasiticBudget.max_c` is the budget.
- **Status**: Engine concern, not constraint type.

### 3.2 Voltage coefficient

Resistor and capacitor parasitic values are voltage-dependent (nonlinear). No `voltage_coefficient` field exists.

- **Books**: AOAL ch06/6.2 (#58b)
- **Rust**: Not represented. Proposed enrichment on `ParasiticBudget` (gap analysis 2.13, lower priority).
- **Status**: Out of scope (extraction model detail).

### 3.3 Coupling capacitance

Inter-net coupling capacitance budget for sensitive nets. Drives spacing rules beyond `CrosstalkExclusion.min_spacing_um`.

- **Books**: ALS 4.4/4.4.3 (#14)
- **Rust**: Proposed `max_coupling_c_ff` on `ParasiticBudget` (gap analysis 2.13).
- **Status**: Planned.

---

## 4. Parasitic Resistance

### 4.1 Contact integrity

Contact spiking through shallow junctions. No tracking in constraint types.

- **Books**: AOAL ch01/1.3.2 (#58a)
- **Rust**: Not represented. Cell-gen / DRC concern.
- **Status**: Out of scope (DRC rule, not PnR constraint).

### 4.2 Metallization contribution

Power transistor routing contributes significant resistance. `metallization_contribution_fraction` and `penetration_distance_um` needed for power devices.

- **Books**: AOAL ch13/13.1 (#58d)
- **Rust**: Not represented on `ParasiticBudget`.
- **Status**: Out of scope (power device specialty concern).

### 4.3 RC delay optimization

Wire length reduction gives quadratic RC improvement (R and C both proportional to length).

- **Books**: FOLD 7.3/7.3.2 (#21)
- **Rust**: `ParasiticBudget` has separate `max_r` and `max_c`. Proposed `max_rc_delay_ps` (gap analysis 2.13).
- **Status**: Planned.

---

## 5. Wire Parasitic Parameters

Per-layer wire parasitic model for routing cost estimation.

- **Rust**: `WireParasiticParams` in `routing_level/parasitic.rs`
- **Status**: Implemented. Fields: `r_per_sq_ohm`, `c_per_um2_ff`, `min_width_um`, `min_spacing_um`.

---

## 6. Parasitic Extraction Models

### 6.1 Extraction dimensionality

ALS describes four extraction tiers:

| Tier | Description | Accuracy | Speed |
|------|-------------|----------|-------|
| 1-D | Wire self-R/C only | Low | Fast |
| 2-D | Cross-section capacitance | Medium | Medium |
| 2.5-D | Per-layer 2D + interlayer coupling | Good | Medium |
| 3-D | Full 3D field solver | High | Slow |

- **Books**: ALS 4.4/4.4.3 (#14)
- **Rust**: No extraction-tier field on `ParasiticBudget`.
- **Status**: Out of scope (extraction engine configuration, not constraint type).

### 6.2 Sensitivity-derived bounds

Parasitic bounds derived from circuit performance sensitivity analysis: S_ij = dW_i / dp_j.

- **Books**: ALS 4.5/4.5.3 (#15)
- **Rust**: Proposed `SensitivityBound` type (gap analysis 1.13, routing_level).
- **Status**: Out of scope (requires circuit simulation integration).

---

## 7. Net Split for Current Density

Splitting high-current from low-current paths within the same net to prevent voltage drops on sensitive branches.

- **Books**: ALS 4.4/4.4.1 (#11)
- **Rust**: Proposed `NetSplitConstraint` (gap analysis 1.13). `NetClassification` classifies whole nets, not sub-nets.
- **Status**: Planned (lower priority).

---

## 8. Inductor Parasitics

### 8.1 Coverage gap

No book provides comprehensive inductor constraint coverage: Q-factor-aware geometry, metal stack selection, substrate shield patterns, center-tap symmetry. AOAL mentions inductor keepout only.

- **Books**: AOAL ch06/6.4 (#59c -- ProximityRule: half inductor width keepout, no circuitry in center, no junctions beneath)
- **Rust**: `inductor.rs` generator exists but constraint coverage is thin.
- **Status**: Gap -- additional sources needed (gap analysis 6.1). Recommended: Niknejad or Mohan et al.
