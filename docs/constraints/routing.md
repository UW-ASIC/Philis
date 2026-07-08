# Routing Constraints

Shielding, length matching, crosstalk, net classification, and routing-level rules.

---

## 1. Crosstalk Exclusion

### 1.1 Spacing rules

Minimum spacing between aggressor and victim net pairs.

- **Books**: AOAL ch02/2.7.6, ch06/6.3, ch08/8.3.2, ch15/15.4 (#43), FOLD 7.3/7.3.3 (#22), ALS 4.4/4.4.3 (#13), PNR_ANALOG 02/5.A.2 (#55), PNR_ANALOG 05/6.3.A (#71)
- **Rust**: `CrosstalkExclusion` in `routing_level/crosstalk.rs`
- **Status**: Implemented. Field: `min_spacing_um`.

### 1.2 Missing fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `shield_type` | ShieldType enum ref | ALS #13, FOLD #22 | Planned |
| `shield_net` | DC potential for shield | ALS #13 | Planned |
| `shield_extension_um` | 2-5 um beyond intersection | AOAL #43c | Planned |

### 1.3 Lead-over-resistor coupling

Metal crossing resistors injects ~0.05 fF/um^2 coupling capacitance.

- **Books**: AOAL ch08/8.3.2 (#43a)
- **Rust**: Not modeled. Engine routing avoidance rule.
- **Status**: Out of scope (routing engine heuristic, not constraint type).

### 1.4 Digital signal crossing rules

Digital signals must never cross shielded capacitors. Noisy signals must not run above/below/adjacent to sensitive nets.

- **Books**: AOAL ch02/2.7.6 (#43b-c), AOAL ch15/15.4
- **Rust**: `NetClassification.net_class` drives routing strategy. No explicit signal-class-aware keepout.
- **Status**: Engine concern consuming `NetClassification`.

---

## 2. Shielding

### 2.1 Three shielding methods

| Method | Description | Books |
|--------|-------------|-------|
| Lateral (same-layer) | Ground wires flanking signal | FOLD #22, ALS #13 |
| Vertical (different-layer) | Ground planes above/below | FOLD #22, ALS #13 |
| All-around | Both lateral and vertical | FOLD #22, ALS #13, PNR_ANALOG 05/9.A |

- **Rust**: `ShieldType` enum in `types.rs` (`None | Grounded | DiffShield`). Used in `MatchingSpec.shield_policy` and `NetClassification.shielding_required`.
- **Status**: Implemented (basic). Missing `Segmented` and `SplitFieldPlate` variants (AOAL #54c, gap analysis 2.7). `ShieldType` is not connected to `CrosstalkExclusion` (gap analysis 2.12).

### 2.2 Five shield rules (PNR_ANALOG)

1. Shield extends >= 2-5 um beyond intersection
2. Shield connects to quiet DC potential
3. Shield on same layer as victim preferred
4. Multiple shield layers for exceptional
5. Shield insertion must respect matched-net symmetry

- **Books**: PNR_ANALOG 05/9.A (#73)
- **Rust**: Routing engine consumes `NetClassification.shielding_required` and `ShieldType`.

---

## 3. Net Classification

### 3.1 Current enum

`NetClass`: `Signal | Clock | Supply | Ground | Sensitive | Substrate` (6 variants).

- **Books**: AOAL ch14/14.3 (#56), ALS 4.5 (#16), PNR_ANALOG 02/5.A.1 (#52-53)
- **Rust**: `NetClassification` in `metadata/classify.rs`, `NetClass` enum in `types.rs`
- **Status**: Implemented.

### 3.2 Missing NetClass variants

| Variant | Purpose | Books | Status |
|---------|---------|-------|--------|
| `Noisy` | Distinguish aggressor from victim | AOAL #56a, ALS #16 | Planned |
| `HighImpedance` | Sensitive high-Z nodes | PNR_ANALOG #52-53 | Planned |
| `MatchedPair` | Matched routing nets | PNR_ANALOG #53 | Planned |
| `Compensation` | Compensation networks | PNR_ANALOG #53 | Planned |
| `GuardRing` | Guard ring nets | PNR_ANALOG #53 | Planned |
| `EsdBus` | ESD bus nets | PNR_ANALOG #53 | Planned |
| `Bias` | Bias distribution nets | PNR_ANALOG #53 | Planned |

Also needed: `routing_priority: Option<u8>` on `NetClassification` (ALS #16).

### 3.3 VoltDomain enrichment

Current: `Core | Io | Analog` -- no numeric voltage.

Need: `nominal_voltage_v: Option<f64>` for voltage-dependent spacing computation (AOAL #56b, gap analysis 2.23).

### 3.4 Routing priority order

    matched > sensitive > power > noncritical > noisy

- **Books**: PNR_ANALOG 05/6.1.A (#70), ALS 4.5 (#16)
- **Rust**: Implicit in routing engine. No explicit priority field on `NetClassification`.

### 3.5 Clock nets as aggressors

Clock nets are the biggest crosstalk aggressors. Routing engine should auto-apply increased spacing for clock-to-sensitive-analog net pairs.

- **Books**: FOLD 7.3/7.3.3 (#23)
- **Rust**: `NetClass::Clock` variant exists. Engine behavior, not constraint type.

---

## 4. Layer Assignment

### 4.1 Layer assignment rules

| Net Type | Preferred Layer | Books |
|----------|----------------|-------|
| High impedance | Highest metal (least coupling) | PNR_ANALOG 05/12.A |
| Power | Thickest metal (lowest R) | PNR_ANALOG 05/12.A |
| Matched pairs | Same layer (R/C matching) | PNR_ANALOG 05/12.A |

- **Rust**: `NetClassification.preferred_layers` encodes this.
- **Status**: Implemented.

---

## 5. Straight Net Routing

Nets that must route as a single straight segment. Placement aligns pins along the perpendicular axis.

- **Books**: (engine design decision, not directly from books)
- **Rust**: `StraightNet` in `routing_level/align.rs`
- **Status**: Implemented.

---

## 6. Matched Route Symmetry

### 6.1 Symmetric routing about an axis

Matched nets routed symmetrically, handling non-symmetric obstacles, matching parasitic R/C between symmetric paths.

- **Books**: ALS 4.4/4.4.2 (#12), PNR_ANALOG 05/8.A (#72)
- **Rust**: `RouteMatchingTolerance` in `placement_level/matching_spec.rs`
- **Status**: Implemented. Missing symmetry-axis reference and obstacle-mirroring policy (gap analysis, ALS #12).

### 6.2 Parasitic matching by construction

Length/via/layer equalization, coupling environment matching, C equalization stubs.

- **Books**: PNR_ANALOG 05/8.A (#72)
- **Rust**: Router algorithm using `RouteMatchingTolerance` thresholds.

---

## 7. Differential Pair Routing

Equal impedance (R, C) on both lines, tight coupling for noise immunity, EMI reduction.

- **Books**: FOLD 5.3/5.3.3 (#43)
- **Rust**: `RouteMatchingTolerance` covers length/R/C deltas. Proposed `DifferentialPairRouting` type for explicit diff-pair net-pair identification (gap analysis 1.13).
- **Status**: Planned (lower priority). `RouteMatchingTolerance` covers the parametric matching; a dedicated type would add explicit pair identification.

---

## 8. Star Node and Kelvin Topology

### 8.1 Star node

All sensitive leads from matched devices converge at a single common point to eliminate differential ground drops.

- **Books**: AOAL ch15/15.4 (#33), PNR_ANALOG 05/6.1.A (#70)
- **Rust**: Proposed `StarNodeConstraint` (gap analysis 1.4).
- **Status**: Planned.

### 8.2 Kelvin connection

Four-wire connections with separate force and sense leads. Force leads >= 2W beyond star.

- **Books**: AOAL ch06, ch15/15.4 (#18), PNR_ANALOG 01/7.A.4 (#39)
- **Rust**: Proposed `KelvinConnectionConstraint` (gap analysis 1.5).
- **Status**: Planned.

---

## 9. CMP Density

### 9.1 Pattern density rules

CMP planarization requires minimum/maximum metal and poly density per window. Affects dummy fill insertion.

- **Books**: AOAL ch02/2.7.5 (#6), FOLD 2.8/2.8.4 (#44)
- **Rust**: Proposed `CmpDensityConstraint` (gap analysis 1.9).
- **Status**: Planned. Typically handled by foundry dummy-fill scripts post-routing.

### 9.2 CMP dummy blocking near matched devices

CMP dummy poly insertion near matched transistors causes etch-rate variation. Block dummy poly within 3 um (exceptional) or 1 um (moderate) of matched gates.

- **Books**: AOAL ch13/13.2.2 (#7)
- **Rust**: `EnvironmentalConstraint` (HydrogenationKeepout) partially covers metal blocking. No CMP-specific blocking field.
- **Status**: Partially addressed. Explicit CMP blocking not implemented.

---

## 10. Bus Routing

### 10.1 Bus termination

Opposite-end bus termination for buses > 2*lambda. Double-terminated buses reduce resistance 4x.

- **Books**: AOAL ch15/15.4 (#4)
- **Rust**: No type. Engine routing strategy.
- **Status**: Out of scope (wide-bus routing specialty).

### 10.2 ESD bus

Max ring resistance 2 ohms, peripheral ring topology.

- **Books**: PNR_ANALOG 02/5.A.1 (#54)
- **Rust**: `EsdConstraint.max_bus_resistance_ohm` exists.
- **Status**: Implemented.
