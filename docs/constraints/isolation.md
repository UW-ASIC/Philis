# Isolation Constraints

Well separation, guard rings, latch-up prevention, and substrate injection control.

---

## 1. Isolation Constraint

### 1.1 Basic separation

Minimum distance between devices in different wells/tanks, driven by junction depletion widths plus safety margin.

- **Books**: AOAL ch01/1.2.1, ch02/2.6.1, ch03/3.2.3, ch14/14.1-14.2 (#51), FOLD 7.1 (#15/#17), ALS 3.1/3.1.2 (#7), PNR_ANALOG 02/2.A (#42), PNR_ANALOG 02/4.A.6 (#51)
- **Rust**: `IsolationConstraint` in `placement_level/isolation.rs`
- **Status**: Implemented. 9 enrichment gaps identified (gap analysis 2.6).

### 1.2 Voltage-dependent spacing

Depletion width scales 2-3x from 5V to 40V (AOAL Table 3.3). Spacing must increase with operating voltage.

    d_min = W_depletion(V_max) + margin

- **Books**: AOAL ch03/3.2.3 (#51a), PNR_ANALOG 02/4.A.6
- **Rust**: `IsolationConstraint.min_distance_um` -- voltage-blind. Proposed `max_operating_voltage_v: Option<f64>` (gap analysis 2.6).
- **Status**: Planned. Also proposed `VoltageSpacingRule` as a dedicated type (gap analysis 1.7).

### 1.3 Isolation technology

Different isolation technologies (junction, dielectric trench, SOI) fundamentally change guard ring requirements.

- **Books**: AOAL ch02/2.5.1 (#51b)
- **Rust**: Proposed `isolation_technology: Option<IsolationTech>` enum (`Junction/DielectricTrench/Soi`) on `IsolationConstraint` (gap analysis 2.6).
- **Status**: Planned.

### 1.4 Injection risk classification

Merger-risk classification: injection, debiasing, and coupling risk between devices sharing wells/tanks.

- **Books**: AOAL ch14/14.1 (#20 -- MergerSafetyConstraint), AOAL ch05/5.5 (#51f/h/i)
- **Rust**: `IsolationConstraint.reason` is free string. Proposed `injection_risk: Option<InjectionRisk>` enum (`Low/Medium/High`) (gap analysis 2.6).
- **Status**: Planned. Also proposed `MergerSafetyConstraint` (gap analysis 1.13).

### 1.5 "Guard the injector" principle

Placement engine should auto-generate guard ring requirements for injector devices (charge pumps, ESD diodes, power devices, voltage domain boundaries).

- **Books**: AOAL ch14/14.2 (#51h), PNR_ANALOG 02/2.A (#42)
- **Rust**: No auto-generation logic. Engine concern consuming `IsolationConstraint` + `GuardRingRequirement`.
- **Status**: Planned (engine feature, not constraint type).

---

## 2. Guard Rings

### 2.1 Guard ring types

AOAL defines four fundamental guard ring types based on carrier physics:

| Type | Full Name | Function | Connection |
|------|-----------|----------|------------|
| ECGR | Electron-Collecting | Collects injected electrons | Highest supply (VDD) |
| EBGR | Electron-Blocking | Blocks electron flow | Biased N-well |
| HCGR | Hole-Collecting | Collects injected holes | Lowest supply (VSS) |
| HBGR | Hole-Blocking | Blocks hole flow | Biased P-well |

Stacking: HCGR inside HBGR achieves >99% injection collection efficiency.

- **Books**: AOAL ch05/5.4, ch09/9.1.3, ch10/10.1.2, ch14/14.2.3 (#50), PNR_ANALOG 00/4.6 (#25-26)
- **Rust**: `GuardRingType` enum in `types.rs` -- currently only `PsubRing | NwellRing | DoubleRing` (3 variants).
- **Status**: Implemented (basic). Needs ECGR/EBGR/HCGR/HBGR variants -- planned, **critical** (gap analysis 2.1).

### 2.2 Guard ring requirement

Width, completeness, connection net, tap pitch, and resistance constraints.

- **Books**: AOAL ch05/5.4, ch08/8.3.1, ch09/9.1.3, ch12/12.2.9, ch14/14.2.3-14.2.5 (#49), PNR_ANALOG 01/6.A (#34)
- **Rust**: `GuardRingRequirement` in `cell_level/guard_ring.rs`
- **Status**: Implemented. Key existing fields: `min_width_um`, `enclosure_complete`, `connection_net`, `tap_pitch_um`, `max_ring_resistance_ohm`.

### 2.3 Guard ring enrichments needed

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `max_well_resistance_ohm` | Debiasing analysis | AOAL #49a | Planned |
| `max_substrate_resistance_ohm` | Substrate noise control | AOAL #49a | Planned |
| `nbl_policy` (Required/Forbidden/Optional) | NBL under guard ring | AOAL #49c | Planned |
| `max_backgate_distance_um` | 25-50 um max to backgate contact | AOAL #49e | Planned |
| `process_layer_depth_um` | Width >= junction depth | AOAL #49f | Planned |

### 2.4 Width formulas

- ECGR (NMoat): width >= P-epi thickness
- HCGR (PMoat): width >= N-well junction depth
- Substrate ground ring: W = Rs*(W+L-4E)/(2*Rmax), max resistance <= 1 ohm

- **Books**: AOAL ch14/14.4 (#17), PNR_ANALOG 01/6.A
- **Rust**: `GuardRingRequirement.min_width_um` stores the computed width.

### 2.5 Dispersed contacts

Dispersed contact arrays (spaced >= 2*t_epi) achieve 13% of the resistance of a single large contact area.

- **Books**: AOAL ch09/9.1.3 (#49b)
- **Rust**: Partially captured by `tap_pitch_um` on `GuardRingRequirement`.
- **Status**: Implemented (partial).

### 2.6 Substrate contact density

FOLD specifies maximum spacing between substrate contacts to control substrate debiasing. Dedicated "currentless" SUB net (star topology, separate from GND).

- **Books**: FOLD 7.1/7.1.1 (#12-13)
- **Rust**: Proposed `SubstrateContactDensity` type (gap analysis 1.13).
- **Status**: Planned (lower priority).

---

## 3. Latch-up

### 3.1 Parasitic SCR mechanism

Every adjacent NMOS/PMOS pair forms a parasitic SCR (thyristor). Latch-up triggers when beta_NPN * beta_PNP >= 1.

- **Books**: AOAL ch01/1.4.2 (#19), FOLD 7.1/7.1.3 (#16-17), PNR_ANALOG 00/4.6 (#25)
- **Rust**: Proposed `LatchupConstraint` in `placement_level/latchup.rs` (gap analysis 1.3).
- **Status**: Planned. Currently handled by generic `IsolationConstraint` + `GuardRingRequirement`, which cannot encode NMOS/PMOS pair semantics or trigger-current dependencies.

### 3.2 LatchupConstraint fields

| Field | Type | Description |
|-------|------|-------------|
| `nmos_device` | `DeviceId` | NMOS in parasitic SCR |
| `pmos_device` | `DeviceId` | PMOS in parasitic SCR |
| `min_spacing_um` | `f64` | NMOS-to-PMOS minimum separation |
| `well_tap_pitch_um` | `f64` | Maximum distance between well taps |
| `trigger_current_ma` | `f64` | SCR trigger threshold |

- **Impl trait**: `PairConstraint` + `Contractable`

### 3.3 Latch-up triggers

Supply spikes, coupling signals with steep edges through parasitic capacitances, pad-connected N-wells (high-risk minority-carrier injectors).

- **Books**: FOLD 7.1/7.1.2-7.1.3 (#15/#17)
- **Rust**: Engine concern -- auto-flagging high-risk pairs from pad connectivity.

### 3.4 Prevention rules

- PMOS: NSD guard ring connected to VDD
- NMOS: PSD guard ring connected to VSS
- Maximum backgate contact resistance per `GuardRingRequirement.max_ring_resistance_ohm`
- Well tap density per `GuardRingRequirement.tap_pitch_um`

- **Books**: FOLD 7.1/7.1.3 (#16), AOAL ch14

---

## 4. Substrate Injection

### 4.1 Injection sources

Charge pumps, ESD diodes, power devices, voltage domain boundaries, forward-biased junctions.

- **Books**: PNR_ANALOG 02/2.A (#42), AOAL ch14/14.2
- **Rust**: `IsolationConstraint.reason` (free string identifies injection source).

### 4.2 Substrate noise coupling model

    V_noise ~ I_inject * rho_sub / (2 * pi * d)

Guard ring attenuation: 20-40 dB per ring. Deep N-well: 40-60 dB.

- **Books**: PNR_ANALOG 00/4.4 (#23)
- **Rust**: `IsolationConstraint.min_distance_um` + `GuardRingRequirement`
- **Status**: Implemented (basic distance/ring model). No quantitative attenuation modeling.

### 4.3 Well bias constraint

Substrate must connect to most negative supply (P-sub) or most positive (N-sub). Incorrect biasing enables minority carrier injection failures.

- **Books**: AOAL ch02/2.6.1 (#37)
- **Rust**: No dedicated type. Block-level concern.
- **Status**: Out of scope (block-level verification, not PnR constraint).

### 4.4 P-bar insertion

P-bar (base diffusion strip) between devices in same tank prevents cross-injection in BiCMOS.

- **Books**: AOAL ch05/5.4 (#25)
- **Rust**: No type. Cell-gen concern.
- **Status**: Out of scope (BiCMOS cell-gen, narrow applicability).

---

## 5. Deep Trench Isolation (DTI)

### 5.1 DTI pair constraint

Devices sharing DTI have distance-dependent interaction: abutting or separated by >= d_DTI (forbidden zone in between).

- **Books**: AOAL ch02/2.5.1 (#45), ALS 3.1/3.1.2 (#6), PNR_ANALOG 04/4.A (#65)
- **Rust**: `DtiPair` in `placement_level/dti.rs`
- **Status**: Implemented. Missing forbidden-zone range fields (`d_forbidden_min`, `d_forbidden_max`) -- planned (gap analysis 2.16).

### 5.2 NBL pattern shift

NBL pattern shift displaces surface discontinuities up to 150% of epi thickness, causing systematic area mismatch.

- **Books**: AOAL ch08/8.2.5, ch10/10.2.5 (#22), PNR_ANALOG 01/7.A.0 (#38)
- **Rust**: Proposed `NblShadowClearance` type (gap analysis 1.13). Also proposed `nbl_policy` on `GuardRingRequirement` (gap analysis 2.2).
- **Status**: Planned (lower priority, BiCMOS-specific).

---

## 6. Voltage Domain Separation

### 6.1 VoltDomain enum

Current: `Core | Io | Analog` -- no numeric voltage for computing voltage-dependent spacing.

- **Books**: AOAL ch14/14.3 (#56b), PNR_ANALOG 02/4.A.3
- **Rust**: `VoltDomain` enum in `types.rs`
- **Status**: Implemented (basic). Needs `nominal_voltage_v: Option<f64>` -- planned (gap analysis 2.23).

### 6.2 Voltage-class spacing

Modern processes implement voltage-dependent poly spacing (TDDB avoidance: max inter-segment voltage = 2V/N).

- **Books**: AOAL ch06/6.3 (#36), FOLD 6.2/6.2.3 (#36)
- **Rust**: Proposed `VoltageSpacingRule` in `placement_level/voltage_spacing.rs` (gap analysis 1.7).
- **Status**: Planned.

---

## 7. Scribe Seal and Die Edge

### 7.1 Scribe seal

Continuous contact ring (no gaps) around die periphery, via rings for each metal layer, large-radius fillets at DI trench corners.

- **Books**: AOAL ch05/5.5 (#30)
- **Rust**: Proposed `ScribeSealConstraint` (gap analysis 1.13).
- **Status**: Out of scope (die-level, handled by foundry IP).

### 7.2 Ground ring

Substrate ground ring width formula with max resistance <= 1 ohm. Die-level constraint.

- **Books**: AOAL ch14/14.4 (#17)
- **Rust**: Proposed `GroundRingConstraint` (gap analysis 1.13).
- **Status**: Out of scope (padring generation).
