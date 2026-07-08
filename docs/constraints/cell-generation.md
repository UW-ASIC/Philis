# Cell Generation Constraints

Dummy placement, moat extension, finger sizing, interdigitation, guard ring construction, and device-level layout rules.

---

## 1. Dummy Devices

### 1.1 Dummy constraint type

- **Books**: AOAL ch02/2.3.3-2.7.5, ch08/8.2.3, ch13/13.2.2 (#46), FOLD 6.6/6.6.3 (#5), PNR_ANALOG 01/5.A (#33)
- **Rust**: `DummyConstraint` in `cell_level/dummy.rs`
- **Status**: Implemented. Fields: `dummy_type` (DummyType enum), `moat_ext_um`, `min_poly_clearance_um`.

### 1.2 DummyType enum

Current: `GateDummy | MoatDummy | Full`.

Missing variants:

| Variant | Purpose | Books | Status |
|---------|---------|-------|--------|
| `CmpFill` | Metal/poly fill for CMP density | AOAL #46a | Planned |
| `HalfDummy` | Acceptable for L > 1 um, moderate matching | AOAL #46f | Planned |
| `SilicideBlock` | Silicide blocking for poly resistor cells | AOAL #46c | Planned |

### 1.3 Tier-specific dummy rules

| Tier | Rule | Value | Books |
|------|------|-------|-------|
| Exceptional | Moat extension | >= 5 um | AOAL #46g, PNR_ANALOG 01/5.A |
| Moderate | Full dummy | Required for L < 1 um | PNR_ANALOG 01/5.A |
| All tiers | Same finger width | Dummies match array finger width | AOAL #46e |

### 1.4 2D surround for capacitors

Capacitor arrays need dummies on all four sides, not just two ends.

- **Books**: FOLD 6.6/6.6.3 (#5), AOAL ch08/8.2.3 (#46d)
- **Rust**: `DummyConstraint` has no `surround_2d` field.
- **Status**: Planned (gap analysis 2.19). Proposed `surround_2d: bool` and `same_finger_width: bool`.

---

## 2. Unitization (Interdigitation / Folding)

### 2.1 Unitization constraint type

- **Books**: AOAL ch01/1.3.1, ch01/1.4, ch02/2.5.2, ch08/8.2.7, ch08/8.3.2, ch10/10.2.3, ch12/12.2.8 (#64), FOLD 6.3 (#37-40), ALS 5.7/5.7.2 (#23), PNR_ANALOG 01/3.A (#29)
- **Rust**: `UnitizationConstraint` in `cell_level/unitization.rs`
- **Status**: Implemented. Fields: `device_type`, `instance_unit_counts`, `unit_geometry`, `pattern_type`, `same_variant_required`, `dummy_required`.

### 2.2 PatternType enum

Current: `Abba | Abab | CommonCentroid2d`.

Missing variants:

| Variant | Ratio | Books | Status |
|---------|-------|-------|--------|
| `Aba` | 2:1 | AOAL #64d | Planned |
| `Ababa` | 3:2 | AOAL #64d | Planned |
| `EightAroundOne` | 8:1 (3x3 2D) | AOAL #64d | Planned |

### 2.3 Missing fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `even_segment_count` | Thermoelectric cancellation | AOAL #64f, PNR_ANALOG #22 | Planned |
| `square_aspect_ratio` | Unit caps must be square | AOAL #64e | Planned |
| `source_drain_swap_permitted` | Asymmetric LDD transistors | AOAL #64a | Planned |

### 2.4 Folding procedure (FET)

Split-flip-pack-route: identical finger geometry, shared diffusions, gate resistance reduction by 1/n^2.

- **Books**: FOLD 6.3/6.3.1 (#37)
- **Rust**: `UnitizationConstraint.unit_geometry` captures unit dimensions. No explicit gate resistance budget or max finger length.

### 2.5 Finger count derivation

Driven by matching granularity (even N_f for chi=0), LOD equalization, and Pelgrom sizing. Even finger count required for orientation matching (chi=0).

- **Books**: PNR_ANALOG 01/3.A (#29)
- **Rust**: No explicit even-finger constraint field on `UnitizationConstraint`.
- **Status**: Planned (cell-gen improvement 3.11).

### 2.6 Resistor head resistance correction

    l_corr = 2(r-1) * R_H / R_sq * w

PCell generator concern. `resistor.rs` has `hastings_head_correction` but it is not wired to geometry.

- **Books**: FOLD 6.3/6.3.2 (#38)
- **Status**: Implemented (function exists). Not connected to geometry output.

### 2.7 Shape function (Pareto-optimal W/H)

ALS defines shape functions as Pareto-optimal (w,h) sets for each building block. `UnitizationConstraint.unit_geometry` stores a single geometry, not a set of alternatives.

- **Books**: ALS 6.4/6.4.1 (#39)
- **Rust**: Proposed enrichment for variant reshape support (roadmap 2.1).
- **Status**: Planned.

### 2.8 Variant constraint

Cross-device variant linkage (e.g., "both diff-pair transistors use same finger count").

- **Books**: ALS 3.1/3.1.2 (#5)
- **Rust**: `UnitizationConstraint.same_variant_required` is a boolean. Proposed `VariantConstraint` for cross-device linkage (gap analysis 1.13).
- **Status**: Planned (lower priority).

---

## 3. LDE (Layout-Dependent Effects)

### 3.1 LdeBound constraint type

- **Books**: AOAL ch02/2.3.3-2.4.2, ch03/3.2.3, ch06/6.2, ch08/8.2.3, ch13/13.2.2 (#52), FOLD 6.6/6.6.3 (#6), PNR_ANALOG 00/2.1-2.2 (#9-10), PNR_ANALOG 02/4.A.1 (#48)
- **Rust**: `LdeBound` in `cell_level/lde.rs`
- **Status**: Implemented. Key fields: `min_well_edge_distance_um`, `max_wpe_dvth_mv`, `sti_dvth_mv`, SA/SB fields.

### 3.2 WPE (Well Proximity Effect) 4-edge model

    dVth = sum_k a_k * exp(-d_k / lambda_k)

Tier-specific well distances: minimal >= 2 um, moderate >= 3 um, exceptional >= 5 um or >= 2x well junction depth.

- **Books**: PNR_ANALOG 00/2.1 (#9), PNR_ANALOG 02/4.A.1 (#48)
- **Rust**: `LdeBound.min_well_edge_distance_um` and `max_wpe_dvth_mv`.

### 3.3 LOD (Length of Diffusion) / STI stress model

    f(SA,SB) = 1 + K1*(1/SA + 1/SB) + K2*(1/SA^2 + 1/SB^2)

LOD is primarily a cell-gen concern (SA/SB equalization). When S0 equalizes properly, the GP LDE term reduces to WPE only.

- **Books**: PNR_ANALOG 00/2.2 (#10), PNR_ANALOG 04/2.5 (#63)
- **Rust**: `LdeBound` SA/SB fields.

### 3.4 Missing LdeBound fields

| Field | Purpose | Books | Status |
|-------|---------|-------|--------|
| `same_length` | Process bias cancellation | AOAL #52e | Planned |
| `min_moat_extension_um` | >= 5 um for LOD control | AOAL #52g | Planned |
| `min_diffusion_width_um` | Prevent dilution in critical devices | AOAL #52d | Planned |

---

## 4. Guard Ring Construction

### 4.1 Cell-level guard ring generation

Width formulas, completeness rules, connection strategy.

- **Books**: PNR_ANALOG 01/6.A (#34), AOAL ch05/5.4, ch14/14.2
- **Rust**: `GuardRingRequirement` in `cell_level/guard_ring.rs`
- **Status**: Implemented. See `isolation.md` for full guard ring coverage.

### 4.2 Deep-N+ plug insertion (BiCMOS)

Deep-N+ plugs for BiCMOS NPN in shared tanks: minimum plug to keep tank R < 100 Ohm.

- **Books**: PNR_ANALOG 01/6.A.0 (#35)
- **Rust**: No type. Proposed `DeepNPlusRequirement` or fold into `GuardRingRequirement`.
- **Status**: Planned (lower priority, BiCMOS-specific).

### 4.3 Parasitic channel prevention

Channel stops, field plates, poly retraction for high-voltage analog.

- **Books**: PNR_ANALOG 01/6.A.00 (#36)
- **Rust**: Proposed `ParasiticChannelPrevention` or fold into `FieldPlateRequirement`.
- **Status**: Planned (lower priority, high-voltage specialty).

---

## 5. Field Plate Requirement

Metal plate over exposed device regions for conductivity modulation control and parasitic channel prevention.

### 5.1 Proposed type

| Field | Type | Description |
|-------|------|-------------|
| `device_id` | `DeviceId` | Device requiring field plate |
| `plate_type` | `FieldPlateType` | None / Full / Split |
| `rsh_threshold_ohm_sq` | `f64` | Rsh threshold triggering requirement |
| `bias_net` | `String` | Net the plate connects to (highest available V) |
| `min_overlap_um` | `f64` | Metal plate overlap beyond exposed region (2-3 um) |

- **Books**: AOAL ch05/5.3, ch06/6.3, ch08/8.3.1, ch09/9.2.3 (#14, 4 sections), PNR_ANALOG 00/2.5 (#15)
- **Rust**: Proposed `FieldPlateRequirement` in `cell_level/field_plate.rs` (gap analysis 1.6).
- **Status**: Planned. Distinct from `ShieldType` (routing-level); this is a cell-gen device construction requirement.

### 5.2 Triggers

- Lateral PNP base surfaces
- Diffused resistors with Rsh >= 1k ohm/sq
- P-type regions at >= 75% of thick-field threshold
- High-sheet resistors (conductivity modulation guard)

### 5.3 Conductivity modulation guard

High-sheet resistors (Rsh > 200 ohm/sq) suffer 0.1%/V conductivity modulation. Requires identical tank bias across matched segments, separate tanks for different operating voltages.

- **Books**: AOAL ch08/8.2.9 (#8), PNR_ANALOG 00/2.5 (#15/#80)
- **Rust**: Proposed `ConductivityModulationGuard` (gap analysis 1.12). Overlaps with `FieldPlateRequirement`.

---

## 6. Environmental Constraints

### 6.1 Environmental constraint type

- **Rust**: `EnvironmentalConstraint` in `cell_level/environment.rs`
- **Status**: Implemented. Kinds: `MetalOverGate`, `HydrogenationKeepout`, and others via `EnvironmentalKind` enum.

### 6.2 Hydrogenation keepout

Metal over matched gates causes up to 20% Id mismatch from hydrogen diffusion. Keepout >= 5 um for exceptional matching.

- **Books**: PNR_ANALOG 00/2.4 (#13-14), AOAL ch02/2.3.4 (#47g)
- **Rust**: `EnvironmentalKind::HydrogenationKeepout`
- **Status**: Implemented. Missing `keepout_radius_um`, `block_dummy_metal`, `block_cmp_fill` fields (gap analysis 2.29).

### 6.3 No-metal-over-gate rule

Never route digital switching signals over matched gates, even with shields.

- **Books**: PNR_ANALOG 00/2.4, AOAL ch06/6.2 (#47f)
- **Rust**: `EnvironmentalKind::MetalOverGate`
- **Status**: Implemented.

---

## 7. Orientation and Rotation

### 7.1 Crystal orientation

(100) vs (111) planes affect oxidation rate and surface state density. Channel orientation critical for matching.

- **Books**: AOAL ch02/2.1.3, ch02/2.4.3, ch03/3.1.3 (#57)
- **Rust**: `MatchingSpec.orientation_rule` (string) and `allowed_transforms` (Vec<Orientation>).

### 7.2 Pocket implant rotation prohibition

Pocket/halo implant devices: 90-degree rotation forbidden. Only 0 and 180-degree rotations allowed.

- **Books**: AOAL ch12/12.2.7 (#26), PNR_ANALOG 01/3.A.3 (#30), PNR_ANALOG 11/MISSING-06 (#81)
- **Rust**: `MatchingSpec.allowed_transforms` blocks R90/R270. Proposed `PocketImplantExclusion` for per-device annotation (gap analysis 1.8).
- **Status**: Partially implemented (MatchingSpec level). Per-device annotation planned.

### 7.3 Circle approximation

BJT emitter circles should use multiples of 4 sides for symmetry.

- **Books**: AOAL ch02/2.4.3 (#57c)
- **Rust**: Cell generator concern. No constraint type.
- **Status**: Out of scope (cell generator detail).

---

## 8. Device Type Coverage

### 8.1 DeviceType enum

Current: `Nmos | Pmos | Npn | Pnp | LateralPnp | Resistor | Capacitor | Diode | Inductor`.

Missing: `Jfet` variant. JFETs appear in BiCMOS and precision analog (symmetric vs asymmetric, backgate connection rules).

- **Books**: AOAL ch01/1.5 (#44)
- **Rust**: `DeviceType` enum in `types.rs`
- **Status**: Planned (gap analysis 2.21).

---

## 9. Backgate Contact Strategy

Three styles with different area/resistance tradeoffs:

| Strategy | Description | Best For |
|----------|-------------|----------|
| Abutting | Ring contacts around device | Small devices |
| Interdigitated strips | Contact strips between fingers | Medium devices |
| Distributed plugs | Plugs in source fingers | Large multi-finger |

- **Books**: AOAL ch12/12.2.9 (#1)
- **Rust**: Currently hardcoded in `mosfet.rs`. Proposed `BackgateContactStrategy` type (gap analysis 1.13).
- **Status**: Planned (lower priority, cell-gen improvement 3.2).

---

## 10. Sense FET Placement (Power Devices)

Sense transistor placement: split into two sections on axis of symmetry, halfway between center and periphery of power device (not at center, which is hottest).

- **Books**: AOAL ch13/13.1 (#31)
- **Rust**: No type. Cell-gen concern for power device layout.
- **Status**: Out of scope (power device specialty).

---

## 11. Ohmic Contact Requirement

Ohmic contacts require heavily doped silicon beneath metal. Lightly doped N-type forms rectifying Schottky barriers with aluminum.

- **Books**: AOAL ch01/1.2.5 (#24)
- **Rust**: No type. DRC/process concern.
- **Status**: Out of scope (process verification).

---

## 12. Cell Metadata Extensions

S0 (cell generation) should export extended metadata for downstream stages:

| Field | Type | Purpose | Books |
|-------|------|---------|-------|
| SA/SB per finger | `f64` | LOD-equalized values | PNR_ANALOG 01/2.A |
| WPE distance vector | `[f64; 4]` | 4-edge WPE distances | PNR_ANALOG 01/2.A |
| `orientation_chi` | `f64` | Orientation metric | PNR_ANALOG 01/2.A |
| Hydrogenation zone | Rectangle | Metal keepout area | PNR_ANALOG 01/2.A |
| `bias_current_mA` | `f64` | Already in `BiasCurrentTag` | PNR_ANALOG 01/2.A |
| `overdrive_voltage_mV` | `f64` | Already in `BiasCurrentTag` | PNR_ANALOG 01/2.A |

- **Rust**: `BiasCurrentTag` covers bias current and overdrive. SA/SB and WPE vectors have no dedicated type.
- **Status**: Partially implemented (`BiasCurrentTag`). SA/SB metadata planned.
