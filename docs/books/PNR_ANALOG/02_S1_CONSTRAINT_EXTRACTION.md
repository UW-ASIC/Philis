# S1 -- Constraint Extraction: Detailed Specification

---

## 1. Purpose

S1 translates electrical intent into geometric constraints that S2 (placement) and S3 (routing) can enforce. This is the acknowledged bottleneck of the entire pipeline -- Sapatnekar (ISPD 2023) identifies it as the leading open problem, and constraint quality propagates multiplicatively through all downstream stages.

### 1.A Analog Considerations: Why Constraint Extraction Is the Bottleneck

The difficulty of analog constraint extraction is not merely technical -- it is epistemological. The majority of constraints that determine whether an analog circuit functions correctly are **implicit**: they exist in the experienced layout engineer's mental model but have never been formalized [ALS 7.1]. Jerke, Lienig, and Freuer identify three root causes [ALS 7.1, Sect. 7.2.2]:

1. **Richer constraint sets per device.** Each analog design object must satisfy a larger, more diverse set of constraints than any digital gate. A single matched transistor pair simultaneously carries matching constraints (Pelgrom random + systematic gradient + LDE), isolation constraints (guard rings, well spacing), parasitic constraints (symmetric routing R and C), thermal constraints (isotherm placement, Seebeck cancellation), and reliability constraints (EM, antenna). A digital standard cell carries timing and DRC.

2. **Unknown constraints at design start.** Many analog constraints are not discoverable until layout is partially complete. The WPE sensitivity of a matched pair depends on well placement, which depends on floorplanning, which depends on the hierarchy extraction that S1 itself produces. This creates circular dependencies that digital flows avoid through abstraction.

3. **Implicit expert knowledge.** An experienced layout engineer "knows" that a current mirror feeding a bandgap reference must have its matching constraint treated as exceptional (Hastings Tier 3), that the substrate contact ring around a charge pump must be a closed HBGR because the pump injects minority carriers on every clock edge [Hastings Ch.14], and that matched poly resistors in a voltage divider need L-shaped segments to cancel piezoresistivity [Hastings Ch.8]. None of this knowledge is in the netlist. GNN-based extractors cannot discover it from topology alone.

**Consequence for S1:** The constraint extraction stage must combine automated topology analysis (GNN, pattern matching) with a structured catalog of domain-specific constraints that are appended rule-by-rule based on circuit context. Sections 2.A through 8.A below define this catalog.

### 1.B Analog-Algorithm Interface

The constraint extraction algorithm must:

- **CI-1.1:** Accept a supplementary constraint catalog (defined in this document) and apply its rules after GNN/pattern-based extraction completes.
- **CI-1.2:** For every constraint emitted, classify it as `hard` or `soft` with a numeric priority (see Section 9 below).
- **CI-1.3:** Emit constraint provenance: `auto_GNN`, `auto_pattern`, `rule_catalog`, or `designer_override`.
- **CI-1.4:** Support constraint relaxation negotiation: when S2 or S3 reports that a constraint set is infeasible, S1 must identify which soft constraints can be relaxed and by how much (see Section 10).

---

## 2. Sub-Block A: Hierarchy Extraction

**Input:** Flat or partially hierarchical SPICE netlist.
**Output:** Hierarchy tree T with annotated building blocks.

**Algorithm (dual-path, from ALIGN + MAGICAL):**

Path 1 -- GCN topology recognition (ALIGN/Kunal DATE 2020):
- Convert netlist to graph.
- Run trained GCN classifier to label sub-circuits as OTA, LNA, mixer, comparator, VCO, etc.
- Confusion matrix target: 100% accuracy on trained topologies (ALIGN reports perfect classification on 6 circuit types).

Path 2 -- Pattern library + signal-flow traversal (MAGICAL):
- Match local structures against a pattern library (differential pair, cascode pair, current mirror, cross-coupled pair, etc.).
- Use matched patterns as seeds; traverse signal-flow graph to expand hierarchy.

Merge: take the union of both paths' hierarchy annotations. Conflicts resolved by confidence score (GCN probability > pattern-match distance).

**Output format:**
```json
{
  "hierarchy_tree": [
    {"id": "OTA1", "type": "telescopic_OTA", "children": ["DP1", "CM1", "CM2", "CASC1"]},
    {"id": "DP1", "type": "differential_pair", "devices": ["M1", "M2"]},
    ...
  ]
}
```

### 2.A Analog Considerations: Hierarchy and Isolation Context

The hierarchy extraction must identify not only functional blocks but also **isolation domains** -- regions of the circuit that require physical separation due to substrate coupling, minority carrier injection risk, or voltage domain differences. These isolation domains are invisible to topology-only analysis but are critical for correct constraint generation downstream.

**Isolation domain triggers** (from circuit context):

| Trigger | Why | Required Action | Source |
|---------|-----|-----------------|--------|
| Charge pump detected (switched-cap with clock-driven diffusions) | Every clock edge forward-biases a junction, injecting minority carriers into substrate | Tag as minority-carrier injector; require HBGR guard ring enclosure | [Hastings Ch.5 Sect.5.4], [Hastings Ch.14] |
| ESD protection diode at pad | Large-area junction can inject under transient conditions | Tag pad-connected diffusion; require ECGR if on P-sub | [Hastings Ch.5 Sect.5.4.1] |
| Power transistor (P_d > threshold) | Self-heating creates thermal gradient; substrate current under avalanche | Tag for thermal-aware placement; require guard ring | [Hastings Ch.5 Sect.5.1.1], [00_ANALOG_PRINCIPLES Sect.4.3] |
| Voltage domain boundary (e.g., 5V I/O adjacent to 1.2V core) | Voltage-dependent spacing rules; well breakdown risk | Tag as cross-domain boundary; generate spacing constraint | [Hastings Ch.2 Sect.2.6.1] |
| Internal capacitor connected to switching node | Even sub-pF capacitors can trigger latchup by coupling switching transients | Tag as potential injector; flag for guard ring assessment | [Hastings Ch.5 Sect.5.4.1] |
| Matched PMOS pair with differential Vgs bias > 100 mV | NBTI causes PMOS Vth drift under negative gate bias; matched PMOS at different Vgs values develop mismatch over product lifetime. Oxynitride dielectrics substantially worsen NBTI. | Flag for NBTI-induced mismatch over product lifetime; recommend equalizing Vgs or increasing device area to budget for drift | [Hastings Ch.5 Sect.5.3] |
| NMOS operating with Vgs ~ 0.4*Vds and Vds > 2V | Hot-Carrier Injection (HCI) is maximized when Vgs ~ 0.4*Vds; causes channel shortening and Vth shift over time | Flag for HCI susceptibility; recommend cascode to equalize Vds across matched pairs, or increased L (adding 0.5-1.0 um provides extra voltage margin) | [Hastings Ch.5 Sect.5.3] |
| Lateral PNP transistor (any) | Parasitic channels can form on the base surface between emitter and collector if thick field oxide is present over lightly doped silicon | Generate a field plate constraint covering the base surface between emitter and collector, connected to the emitter terminal | [Hastings Ch.5 Sect.5.3.5] |

**Merged device debiasing analysis.** For devices sharing a well/tank, S1 must compute the IR drop across the shared well resistance: V_debias = I_device * R_well. If V_debias > 500 mV at worst-case temperature (150 C for automotive, 125 C for commercial), the shared well resistance is high enough to forward-bias parasitic junctions, risking latchup. The constraint extractor must flag such devices for separate well isolation or deep-N+ plug insertion [Hastings, Ch. 14.1]. Even a minimum deep-N+ plug reduces tank resistance from approximately 6 kOhm to approximately 100 Ohm, eliminating the latchup risk. Noisy and sensitive devices must never share a tank/well due to capacitive coupling through the shared well node.

**Forward-biased junction analysis.** For every diffusion connected to a pin through less than a critical resistance (conservative: 10 kOhm), S1 must flag it as a potential minority-carrier injection source [Hastings Ch.5 Sect.5.4.1]. The analysis is:

```
For each pin P:
  Trace netlist from P to all diffusion terminals.
  Compute series resistance R_path from P to each diffusion.
  If R_path < R_critical (default 10 kOhm):
    Flag diffusion as potential injector.
    Generate guard ring constraint for the device containing this diffusion.
```

**Hierarchy-to-isolation mapping.** Each identified functional block is assigned to an isolation domain based on:
- Supply voltage domain (VDD_core, VDD_IO, etc.)
- Substrate type (P-sub, N-well, deep N-well, SOI island)
- Injection risk (from the forward-biased junction analysis above)

This mapping feeds into S2 placement as hard inter-domain spacing constraints.

### 2.B Analog-Algorithm Interface

The hierarchy extraction algorithm must:

- **CI-2.1:** For each hierarchy node, emit an `isolation_domain` tag (one of: `analog_core`, `digital`, `power`, `io`, `charge_pump`, or `custom`).
- **CI-2.2:** For each device identified as a potential minority-carrier injector, emit a `guard_ring_type` recommendation (`ECGR`, `EBGR`, `HCGR`, `HBGR`, or `HCGR+HBGR`) based on carrier type and substrate polarity [Hastings Ch.5 Sect.5.4.4].
- **CI-2.3:** For each cross-domain boundary, emit a `voltage_domain_spacing` constraint with minimum distance computed from well breakdown voltage and process-specific depletion width.

---

## 3. Sub-Block B: GNN Symmetry Extraction

**Algorithm:** Chen, Zhu, Liu, Tang, Sun, Pan -- "Universal Symmetry Constraint Extraction for AMS Circuits with GNNs" (DAC 2021). Open-source: github.com/baloneymath/AncstrGNN.

**Pipeline:**

1. **Heterogeneous multigraph construction:** Devices -> vertices. Nets -> directed edges with port-type labels (gate, drain, source, passive). Parallel edges allowed. Clique-based edge construction per net.

2. **Node feature initialization:** 18-dim vector = 15-dim one-hot device type + length + width + metal layers.

3. **Unsupervised GNN learning:** 2-hop aggregation using gated recurrent units with edge-type-specific weight matrices (4 matrices for 4 port types). Loss function: cross-entropy between neighbor similarity and negative samples. Output: learned embedding z_v for each device v.

4. **Circuit feature embedding:** For each subcircuit, compute PageRank on its subgraph, select top-10 vertices by PageRank score, concatenate their embeddings -> circuit embedding.

5. **Cosine similarity classification:**
   - For each valid candidate pair (t_i, t_j) within the same hierarchy level:
   - lambda_sim = cos(z_ti, z_tj)
   - Device-level threshold: lambda_th = 0.99
   - System-level threshold: lambda_th = min(0.999, 0.95 + 0.95/(1 + |N_max_subcircuit|))
   - If lambda_sim > lambda_th -> matched pair constraint.

**Targets:** F1 > 0.95 system-level, F1 > 0.80 device-level, FPR < 1%.

### 3.A Analog Considerations: What the GNN Misses

The GNN extracts structural symmetry from topology. This is necessary but not sufficient. Experienced layout engineers enforce matching constraints that the GNN systematically misses because they arise from electrical function, not topology:

#### 3.A.1 Matching Constraints Not Discoverable from Topology

| Constraint | Why the GNN Misses It | Failure Mode If Missed | Source |
|------------|----------------------|------------------------|--------|
| **Current mirror ratio accuracy** | GNN sees two topologically identical transistors but does not know that a 1:4 mirror feeding a bandgap reference requires exceptional matching (Hastings Tier 3: < 0.1% Id mismatch) | Systematic offset in reference voltage; bandgap tempco degrades from ~10 ppm/C to ~100 ppm/C | [Hastings Ch.13 Sect.13.3], [00_ANALOG_PRINCIPLES Sect.1.4] |
| **Capacitor matching in SAR DAC** | GNN sees capacitors of different sizes; cannot infer that binary-weighted caps C, 2C, 4C... must match to < 0.01% for 14-bit INL | INL/DNL blow out; effective resolution drops by 2-4 bits | [Hastings Ch.8 Sect.8.3.2], [00_ANALOG_PRINCIPLES Sect.1.9] |
| **Resistor divider in feedback network** | GNN sees two resistors in series; does not know they set closed-loop gain and must match to ~0.1% | Gain error; in a precision ADC driver, this directly maps to linearity error | [Hastings Ch.8 Sect.8.3.1], [00_ANALOG_PRINCIPLES Sect.1.6] |
| **Bias current matching across blocks** | A tail current mirror copied to multiple OTAs must track; GNN sees them in different hierarchy branches | Systematic offset in each OTA; CMRR degradation | [00_ANALOG_PRINCIPLES Sect.1.3] |
| **Thermal coupling requirement** | Two devices that must track over temperature (e.g., bandgap BJT and its bias resistor) need proximity, not symmetry | Temperature coefficient of the composite circuit degrades | [00_ANALOG_PRINCIPLES Sect.4.3] |

#### 3.A.2 Matching Tier Assignment

For every matched pair/group identified (whether by GNN or by the rule catalog), S1 must assign a **matching tier** using Hastings' classification [Hastings Ch.8, Ch.13], [00_ANALOG_PRINCIPLES Sect.1.4]:

```
Tier assignment algorithm:

1. From DC operating point, extract the sensitivity of each
   top-level performance metric (offset, gain error, CMRR, PSRR,
   INL, DNL) to the mismatch of each device pair.

2. For each pair (M_i, M_j):
   S_max = max over all metrics of |d(metric)/d(mismatch)|
   
3. Classify:
   If S_max maps to spec violation at 2-5% mismatch:
     tier = "minimal"       (Hastings Tier 1)
   If S_max maps to spec violation at 0.5-1% mismatch:
     tier = "moderate"      (Hastings Tier 2)
   If S_max maps to spec violation at < 0.1% mismatch:
     tier = "exceptional"   (Hastings Tier 3)

4. Tier determines downstream constraints:
   - minimal: same orientation, close proximity, standard sizing
   - moderate: interdigitated or CC-1D, dummies, identical SA/SB
   - exceptional: CC-2D, extensive dummies (>= 3 um from active),
     guard rings, no metal over gates, block dummy metal fill,
     symmetric routing to 1%, well edge >= 5 um
```

**The importance order** from Strasser et al. [ALS 3.2] provides the conflict-resolution priority when constraints from different sources conflict:

```
M_S (symmetry matching) > M_B (building-block matching) > P_B (building-block proximity) > S (symmetry) > P_N (netlist proximity)
```

This order reflects the analog hierarchy of needs [00_ANALOG_PRINCIPLES Sect.5]: matching correctness > isolation > parasitics > reliability > area.

#### 3.A.3 Self-Symmetric Device Detection

The GNN identifies matched pairs but may miss **self-symmetric** devices -- single devices that must be placed on the axis of symmetry of a matched group. Common examples:

- Tail current source of a differential pair
- Common-mode feedback (CMFB) sense device
- Bias transistor shared between two matched branches

S1 must identify these by checking: for each device connected to the common node of a matched pair (e.g., the source node of a diff pair), if removing that device would break the symmetry of the pair's bias point, tag it as `self_symmetric` with respect to that symmetry group.

### 3.B Analog-Algorithm Interface

The GNN symmetry extraction must:

- **CI-3.1:** Accept post-GNN constraint augmentation from the rule catalog (Section 3.A.1 table) without overwriting GNN-discovered constraints.
- **CI-3.2:** For each matched pair, emit a `matching_tier` field (`minimal`, `moderate`, `exceptional`) computed per Section 3.A.2.
- **CI-3.3:** Emit `self_symmetric` tags for devices identified per Section 3.A.3.
- **CI-3.4:** Never emit a matched-pair constraint without also emitting the corresponding LDE bounds (Section 4); the two are inseparable.

---

## 4. Sub-Block C: LDE Sensitivity Analysis

**Purpose:** For each matched pair, compute sensitivity to layout-dependent effects and generate placement bounds.

**WPE bounds:**
```
For each matched pair (M_i, M_j):
  Compute delta_Vth(WPE) = f(distance_to_well_edge)
  using the PDK's WPE model parameters.
  
  If |delta_Vth_i - delta_Vth_j| > WPE_tolerance:
    Generate constraint: min_distance_to_well_edge for both devices.
    WPE_tolerance = sigma_target / 3 (from performance spec).
```

**LOD/STI stress bounds:**
```
For each matched pair:
  Compute delta_Id(LOD) = f(SA, SB) where SA, SB are
  distances from gate edge to OD edge on each side.
  
  If |delta_Id_i / Id_i - delta_Id_j / Id_j| > LOD_tolerance:
    Generate constraint: SA_i ~ SA_j, SB_i ~ SB_j.
    This is enforced by requiring identical dummy context at S0.
```

**Output:** Per-pair LDE bound records appended to the constraint file.

### 4.A Analog Considerations: The Full LDE Constraint Set

The core spec covers WPE and LOD. Experienced layout engineers know that LDE sensitivity analysis must also address the following effects, each of which can dominate mismatch in specific contexts:

#### 4.A.1 Well Proximity Effect (WPE) -- Extended Model

The simplified WPE model in the core spec uses a single exponential. The full model sums contributions from all four well edges [00_ANALOG_PRINCIPLES Sect.2.1]:

```
dVth_WPE = sum_k  a_k * exp(-d_k / lambda_k)

where:
  k indexes {left, right, top, bottom} well edges
  d_k = distance from device channel center to k-th well edge
  a_k = scattering amplitude (mV), PDK-specific
  lambda_k = scattering decay length (um), PDK-specific
```

**Critical insight:** WPE is a **device-local** effect, not a gradient. Common-centroid layout does NOT cancel WPE. Each device in a matched array must individually satisfy the well-distance constraint. The only reliable mitigation is to ensure both devices have identical distances to all four well edges [00_ANALOG_PRINCIPLES Sect.2.1]:

```
|d_k(M_i) - d_k(M_j)| < epsilon_WPE  for all k in {L, R, T, B}

where epsilon_WPE is derived from:
  dVth_mismatch_budget_WPE = sigma_target / 3
  epsilon_WPE = lambda * ln(a / dVth_mismatch_budget_WPE)
```

**Quantitative impact** [Hastings Ch.13]:
- At 0.5 um from well edge: 5% current mismatch
- At 0.25 um from well edge: 25% current mismatch
- Effects observed up to 5 um from well edge

**Tier-specific well distance requirements:**

| Tier | Min well edge distance | Source |
|------|----------------------|--------|
| Minimal | >= 2 um or PDK min | [00_ANALOG_PRINCIPLES Sect.2.1] |
| Moderate | >= 3 um | [00_ANALOG_PRINCIPLES Sect.1.8] |
| Exceptional | >= 5 um, or >= 2x well junction depth | [Hastings Ch.13 Rule 19] |

#### 4.A.2 LOD/STI Stress -- Extended Model

The LOD correction function [00_ANALOG_PRINCIPLES Sect.2.2]:

```
f(SA, SB) = 1 + K1*(1/SA + 1/SB) + K2*(1/SA^2 + 1/SB^2)

Mismatch: delta_Id/Id = f(SA_i, SB_i) - f(SA_j, SB_j)
```

For matched pairs, the constraint is:

```
|SA_i - SA_j| < epsilon_SA  AND  |SB_i - SB_j| < epsilon_SB

where epsilon_SA, epsilon_SB are computed from:
  d(delta_Id/Id)/dSA = -K1/SA^2 - 2*K2/SA^3
  epsilon_SA = delta_Id_budget / |d(delta_Id/Id)/dSA|
```

**Dummy device requirements by tier:**

| Tier | Dummy Specification | Source |
|------|--------------------|--------|
| Minimal | No strict requirement | [Hastings Ch.13] |
| Moderate | Full dummy for L < 1 um; half dummy for L > 1 um | [Hastings Ch.13 Rule 12] |
| Exceptional | Outermost dummy poly >= 3 um from nearest active gate; moat extends >= 5 um beyond last active gate | [Hastings Ch.13 Rule 12] |

#### 4.A.3 Mechanical Stress (Packaging)

Packaging-induced piezoresistive effects create systematic mismatch that is invisible in pre-packaging simulation [00_ANALOG_PRINCIPLES Sect.2.3]:

```
dR/R = pi_L * sigma_L + pi_T * sigma_T + pi_S * tau_S
```

PMOS is approximately 2x more stress-sensitive than NMOS due to larger piezoresistance coefficients [00_ANALOG_PRINCIPLES Sect.2.3]:

| Parameter | NMOS <110> | PMOS <110> |
|-----------|-----------|-----------|
| pi_L | 30 x 10^-11 Pa^-1 | -65 x 10^-11 Pa^-1 |
| pi_T | -17 x 10^-11 Pa^-1 | 40 x 10^-11 Pa^-1 |

**Die-position constraints by tier:**

| Tier | Placement Region | Source |
|------|-----------------|--------|
| Minimal | Avoid die edges (>= 200 um from edge) | [Hastings Ch.8 Rule 12] |
| Moderate | Avoid edges (>= 500 um) and corners | [00_ANALOG_PRINCIPLES Sect.2.3] |
| Exceptional | Central half of die; use die axes of symmetry | [Hastings Ch.13 Rule 13-15] |

**For resistors:** Poly resistors are preferred over mono-Si for matching because poly has orientation-independent piezoresistivity. L-shaped segments (equal horizontal + vertical lengths) can cancel piezoresistivity for exceptional matching [Hastings Ch.8].

#### 4.A.4 Hydrogenation Blocking

Metal layers above active gates block hydrogen diffusion during passivation anneal, creating up to 20% systematic Id mismatch [Hastings Ch.13], [00_ANALOG_PRINCIPLES Sect.2.4]:

```
For each matched pair at tier moderate or exceptional:
  Generate constraint: no_metal_over_gate = true
  Generate constraint: block_dummy_metal_generation = true
  
  If tier = exceptional:
    Metal block zone extends >= 5 um beyond active gate
    in all directions [Hastings Ch.13 Rule 18]
```

#### 4.A.5 Thermal Proximity Constraints

For matched pairs near power devices, thermal gradient induces systematic Vth offset [00_ANALOG_PRINCIPLES Sect.4.3]:

```
dVth/dT ~ -1 to -2 mV/C

For a 1C temperature difference across a matched pair:
  systematic offset ~ 1-2 mV

Thermal gradient from a power device at distance d:
  T(d) ~ P / (2*pi*k_th*t_sub) * ln(R_max/d)
  where k_th = 148 W/(m*K), t_sub ~ 300-700 um
```

**Constraint generation:**

```
For each matched pair (M_i, M_j):
  For each power device d with P_d > P_threshold:
    Compute dT across the pair due to device d.
    If dT > dT_tolerance (tier-dependent):
      Generate constraint: min_distance_from_power_device
      
      Tier-specific thresholds:
        minimal:     dT_tolerance = 2 C
        moderate:    dT_tolerance = 0.5 C
        exceptional: dT_tolerance = 0.1 C
                     (place at opposite end of die, ~75% from
                      center to far edge) [Hastings Ch.13 Rule 14]
```

#### 4.A.6 Voltage-Dependent Spacing

Devices operating at different voltages require spacing that depends on the voltage difference, not just DRC minimum spacing. This constraint is critical at I/O boundaries and between high-voltage and low-voltage wells [Hastings Ch.2 Sect.2.6.1]:

```
For junction isolation:
  d_min = W_depletion(V_max) + margin

  W_depletion = sqrt(2 * epsilon_Si * V_reverse / (q * N_doping))

  For a 40V process: well junction depth ~ 6-8 um
  For a 5V process:  typical N-well spacing ~ 2-3 um
  For a 1.8V process: typical N-well spacing ~ 1-1.5 um
```

**S1 must generate voltage-dependent spacing constraints** for all pairs of wells or diffusions at different voltage potentials, computed from the maximum reverse bias across the isolation junction plus a process-specific margin.

### 4.B Analog-Algorithm Interface

The LDE sensitivity analysis must:

- **CI-4.1:** Compute and emit WPE bounds using the full four-edge model (Section 4.A.1), not just nearest-edge.
- **CI-4.2:** Compute and emit LOD bounds including dummy device specifications by tier (Section 4.A.2).
- **CI-4.3:** Emit `die_position_constraint` for moderate and exceptional tiers (Section 4.A.3).
- **CI-4.4:** Emit `no_metal_over_gate` and `block_dummy_metal` flags for moderate/exceptional matched pairs (Section 4.A.4).
- **CI-4.5:** Emit `min_distance_from_power_device` for each matched pair near each tagged power device (Section 4.A.5).
- **CI-4.6:** Emit voltage-dependent spacing constraints for cross-domain boundaries (Section 4.A.6).
- **CI-4.7:** For all LDE constraints, report the computed mismatch contribution as a percentage of the total mismatch budget, enabling downstream prioritization.

---

## 5. Sub-Block D: Net Classification

**Purpose:** Tag each net with a functional type that drives routing strategy in S3.

| Class | Criteria | Routing strategy |
|-------|----------|-----------------|
| Differential | Connected to matched diff-pair gates/drains | Symmetric routing, VAE diff-net model |
| Clock | Connected to switch gates, comparator clocks | Boundary routing, VAE clock model, shielding |
| Power (VDD) | Connected to supply rail | Wide metal, multi-via, VAE PG model |
| Ground (VSS) | Connected to ground rail | Wide metal, multi-via, VAE PG model |
| Sensitive | High-impedance nodes (OTA input, ref voltage) | Shielding, minimum coupling, short routes |
| General | Everything else | Standard A* routing |

**Algorithm:** Topology traversal + net-name pattern matching + impedance estimation from schematic simulation.

### 5.A Analog Considerations: Extended Net Classification

The six-class scheme above is insufficient for analog layout. Experienced engineers additionally classify the following net types, each of which carries routing constraints that the general scheme misses:

#### 5.A.1 Additional Net Classes

| Class | Criteria | Routing Constraints | Why It Matters | Source |
|-------|----------|-------------------|----------------|--------|
| **Substrate** | Substrate contact net (SUB, PSUB, BULK) | Dedicated net separate from circuit VSS; low-resistance star topology from pad; no shared routing with signal ground | Substrate debiasing creates ground bounce that shifts analog bias points and can forward-bias isolation junctions | [FOLD 4.6], [00_ANALOG_PRINCIPLES Sect.4.4] |
| **Guard_ring** | Guard ring contact metallization | Continuous metal ring (no gaps); width sufficient for transient currents (tens to hundreds of mA); connect to highest/lowest available supply | Guard ring effectiveness drops to zero at metallization gaps; metal must handle latchup test currents [Hastings Ch.5 Sect.5.4.4] | [Hastings Ch.14], [00_ANALOG_PRINCIPLES Sect.4.6] |
| **Matched_pair** | Net connecting gates/sources of matched transistor pair or terminals of matched passive | Symmetric routing: same metal layer(s), identical via stacks, total wire length matched to 5% (moderate) or 1% (exceptional); shield from clock/digital | Asymmetric parasitics create systematic offset indistinguishable from device mismatch | [00_ANALOG_PRINCIPLES Sect.3.4], [Hastings Ch.8 Rule 10] |
| **High_impedance** | Cascode drain, opamp virtual ground, reference node | Shortest possible route on lowest-C metal layer; no adjacent clock/digital routing; shielding on adjacent metal layers | BW ~ 1/(2*pi*R_out*C_par); even 0.1 fF extra capacitance can shift pole location | [00_ANALOG_PRINCIPLES Sect.3.1-3.2] |
| **Compensation** | Miller cap node, feedforward cap node | Controlled parasitic C; no stray coupling to other nodes | Extra C shifts compensation zero/pole; can destabilize feedback loop | [00_ANALOG_PRINCIPLES Sect.3.1] |
| **ESD_bus** | ESD reference ring | Continuous ring around die perimeter; max_ring_resistance_ohm: 2.0; max_total_resistance_ohm: 2.0; topology: peripheral_ring; chamfer_corners: true; width for 1.3 A peak current (2 kV HBM); CDM_clamp_max_distance_from_gate_um: 50 | ESD current must flow through at most two protection devices between any pin pair; ground ring resistance > 2 ohms between any two pads violates ESD sharing; CDM secondary protection must be placed near the gate oxides it protects, not at the bondpad | [Hastings Ch.5 Sect.5.1.5], [FOLD 7.4] |

#### 5.A.2 Crosstalk Sensitivity Matrix

Beyond classification, S1 must generate a **crosstalk sensitivity matrix** that identifies which pairs of nets must not be routed adjacent to each other:

```
For each pair (net_i, net_j):
  If net_i.class in {clock, digital} AND
     net_j.class in {sensitive, high_impedance, matched_pair, compensation}:
    Generate constraint: min_spacing(net_i, net_j) = shield_spacing
    
    shield_spacing = max(2 * interlayer_dielectric_thickness,
                         PDK_minimum_analog_digital_spacing)
```

Quantitative justification: Clock-to-signal coupling has been measured to cause 5x offset worsening in matched differential routing [00_ANALOG_PRINCIPLES Sect.3.4, citing PNR-Checklist item 2.8].

### 5.B Analog-Algorithm Interface

The net classification must:

- **CI-5.1:** Emit the extended net classes from Section 5.A.1 in addition to the core six classes.
- **CI-5.2:** Emit the crosstalk sensitivity matrix as a list of `(net_i, net_j, min_spacing)` tuples.
- **CI-5.3:** For `guard_ring` nets, emit `continuity_required = true` and `min_width_um` computed from latchup test current (default: width for 100 mA at J_max of the metal layer).
- **CI-5.4:** For `substrate` nets, emit `topology = star` and `dedicated_supply = true`.

---

## 6. Sub-Block E: Electrical Constraint Extraction

**From DC operating point simulation:**

```
For each net n:
  I_max(n) = max DC + peak AC current through n
  -> Compute min wire width for EM: w_min = I_max / J_max(metal_layer)
  -> Compute min via cuts: N_cuts = ceil(I_max / I_max_per_cut)

For each device d:
  P_d = V_ds * I_d (DC power dissipation)
  -> Tag for thermal-aware placement at S2

For each sensitive net n:
  R_max(n) = max tolerable parasitic resistance
  (from sensitivity analysis or designer spec)
  -> Routing constraint: total wire resistance < R_max
```

### 6.A Analog Considerations: Constraints Beyond the DC Operating Point

The core spec extracts constraints from DC operating point only. Analog circuits require constraints derived from AC analysis, transient analysis, and reliability models:

#### 6.A.1 AC-Derived Parasitic Budgets

```
For each high-impedance node n:
  From AC simulation, extract:
    R_out(n) = output impedance at node n
    f_target = required bandwidth at node n
  
  Compute max parasitic capacitance:
    C_par_max = 1 / (2 * pi * f_target * R_out)
  
  Example:
    R_out = 10 kOhm, f_target = 100 MHz
    C_par_max = 0.16 fF
    (This is sub-femtofarad -- requires minimum-length routing
     on lowest-capacitance metal layer)
```

#### 6.A.2 IR Drop Budgets for Analog Supplies

The digital IR drop budget (< 10% Vsupply) is too loose for analog. Analog supplies require [00_ANALOG_PRINCIPLES Sect.4.2]:

```
IR_drop_max = 5% of V_supply (analog)

For 1.0V analog supply: max 50 mV at any device pin
For 1.8V analog supply: max 90 mV at any device pin

This translates to wire resistance constraints:
  R_wire_max(n) = IR_drop_max / I_max(n)
  
  w_min_IR = R_sheet * L_estimated / R_wire_max
```

**For matched-net IR drop**, the constraint is tighter: the IR drop difference between matched nets must be a fraction of the mismatch budget. Metal and via resistance contributions must scale proportionally to desired resistance ratios [Hastings Ch.8 Rule 23]:

```
|IR_drop(net_a) - IR_drop(net_b)| < delta_V_match_budget

For a current mirror with 0.1% matching requirement at I_bias = 100 uA:
  delta_V_match = 0.001 * I_bias * R_out_mirror
  
  This constrains the resistance mismatch of mirror gate routing
  to typically < 1 Ohm.
```

#### 6.A.3 Electromigration with Temperature Derating

The core spec uses room-temperature J_max. Layout engineers derate for the actual junction temperature at each wire segment [00_ANALOG_PRINCIPLES Sect.4.1]:

```
J_max(T) = J_max(T_ref) * exp[Ea/(n*k) * (1/T - 1/T_ref)]

where:
  T_ref = reference temperature (typically 105 C = 378 K)
  Ea = 0.7 eV (Al) or 0.9 eV (Cu)
  n = 2 (Al) or 1 (Cu)
  k = 8.62e-5 eV/K

For a wire segment near a power device at T = 150 C:
  J_max(Cu, 150C) = J_max(Cu, 105C) * exp[0.9/(1*k)*(1/423-1/378)]
                   ~ 0.35 * J_max(Cu, 105C)
```

**Blech length check** [00_ANALOG_PRINCIPLES Sect.4.1]: Short intra-cell straps below the Blech length are "immortal" and can be granted EM waivers:

```
If J * L < (J*L)_Blech (~ 2000-5000 A/cm for Cu):
  EM constraint is waived for this segment.
```

#### 6.A.4 Antenna Rule Constraints

For every gate connected to long metal runs, S1 must pre-compute the antenna ratio and flag potential violations [00_ANALOG_PRINCIPLES Sect.4.5]:

```
For each gate g:
  For each metal layer m connecting to g:
    antenna_ratio(m) = total_metal_area(m, net(g)) / gate_oxide_area(g)
    
    If antenna_ratio > PDK_limit (typically 400-1000 for metal, 200 for poly):
      Flag violation.
      Generate constraint: insert metal jumper or protection diode.
```

#### 6.A.5 Substrate and Well Contact Density

S1 must generate well/substrate contact density constraints based on the debiasing analysis from Hastings [Hastings Ch.5 Sect.5.4.3]:

```
Target: distributed well/substrate resistance < 5 Ohm
(for 100 mA latchup test current, 0.5 V max debiasing)

For thin epi on heavy sublayer (Type 1):
  R_V = rho * t_epi / A_contact
  Space contacts at least 2 * t_epi apart for maximum benefit.
  Disperse arrays of small contacts achieve ~13% of the
  resistance of one large contact of equal area.

For thick lightly doped substrate (Type 2):
  Place substrate contacts within half the die thickness
  (~150 um for 300 um post-backgrind die) of all devices.
  Make individual contacts as large as space permits.

For thin isolated layers (Type 3):
  Ring injectors with annular contacts (width >= t_layer/2).
  Scatter additional contacts every 20-50x layer thickness.
```

### 6.B Analog-Algorithm Interface

The electrical constraint extraction must:

- **CI-6.1:** Emit AC-derived parasitic budgets (C_par_max per node) in addition to DC-derived constraints.
- **CI-6.2:** Use the analog IR drop budget (5% Vsupply) for all nets in analog isolation domains; digital budget (10%) for digital domains.
- **CI-6.3:** Emit temperature-derated J_max for wire segments near tagged power devices.
- **CI-6.4:** Flag Blech-length-immune segments and mark them for EM waiver.
- **CI-6.5:** Emit antenna ratio estimates and flag potential violations for S3 to resolve during routing.
- **CI-6.6:** Emit well/substrate contact density requirements per isolation domain.

---

## 7. Sub-Block F: Pattern Selection Advisor

**Purpose:** Recommend CC vs. interdigitated vs. clustered for S0 cell generation.

**Algorithm (sensitivity-driven):**

```
For each matching group:
  1. Compute sensitivity of primary performance metric to
     systematic (gradient) mismatch vs. random mismatch.
  
  2. If systematic >> random (gradient-dominated):
     -> CC 2-D for maximum gradient cancellation.
  
  3. If random >> systematic (area-dominated):
     -> Increase unit device area (Pelgrom).
     -> CC not needed; interdigitated sufficient.
  
  4. If parasitics dominate (high-speed, FinFET):
     -> Simpler pattern (interdigitated or CC 1-D) to
        minimize routing overhead.
     -> CC 2-D only for large arrays where dispersion
        outweighs parasitic cost.
  
  5. Pass recommendation to S0. If S0 can't implement
     the recommended pattern (area/DRC constraint),
     S0 reports back and S1 adjusts matching tolerance.
```

### 7.A Analog Considerations: Pattern Selection Heuristics

The core algorithm uses a sensitivity-based binary decision. Experienced layout engineers apply more nuanced heuristics based on the five rules of common-centroid layout [Hastings Ch.13 Table 13.2], [00_ANALOG_PRINCIPLES Sect.1.5]:

#### 7.A.1 The Five CC Rules as Constraints

| Rule | Requirement | What It Cancels | Constraint for S0 |
|------|------------|-----------------|-------------------|
| 1. COINCIDENCE | Centroids of matched devices coincide | Linear gradients (any direction) | `centroid_distance < epsilon_cc` |
| 2. SYMMETRY | Array symmetric about both H and V axes | Enables coincidence | `array_symmetry = {H, V}` |
| 3. DISPERSION | Subdivide large arrays into CC subarrays | Quadratic gradient residues | `max_subarray_size = N_sub` |
| 4. COMPACTNESS | Minimize array footprint | Higher-order gradient residues | `aspect_ratio < 3:1` (voltage), `~1:1` (exceptional current) |
| 5. ORIENTATION | All matched transistors have equal orientation chi | Stress/mobility asymmetry | `chi_A = chi_B` |

**Orientation metric** [Hastings Ch.13]: For multi-finger transistors, chi = (1/N) * sum(chi_i), where chi_i = +1 if current flows right, -1 if left. Zero orientation (equal left/right fingers) is preferred. S1 must emit this as a hard constraint for moderate and exceptional tiers.

#### 7.A.2 2D vs 1D Decision

A 2D cross-coupled AB/BA array exhibits approximately 60% of the residual gradient-induced mismatch of a 1D ABBA array [Hastings Ch.13], [00_ANALOG_PRINCIPLES Sect.1.5]. The decision rule:

```
If matching_tier = exceptional AND device_aspect_ratio ~ 1 (W ~ L):
  pattern = CC_2D
  (residual mismatch = 60% of CC_1D)

If matching_tier = exceptional AND W >> L:
  pattern = CC_1D_ABBA
  (2D infeasible due to extreme aspect ratio)

If matching_tier = moderate:
  pattern = CC_1D_ABBA or interdigitated
  (chosen by sensitivity ratio)

If matching_tier = minimal:
  pattern = interdigitated or clustered
  (CC overhead not justified)
```

#### 7.A.3 Resistor and Capacitor Pattern Rules

**Resistors** [Hastings Ch.8], [00_ANALOG_PRINCIPLES Sect.1.6]:
- Exceptional: arrayed segments (never serpentines); even number of segments with half in each current direction (Seebeck cancellation); identical segment geometries; minimum segment length >= 10x design-rule minimum.
- Moderate: arrayed segments; min segment length >= 5x design-rule minimum.

**Capacitors** [Hastings Ch.8], [00_ANALOG_PRINCIPLES Sect.1.9]:
- Always square geometries (minimize periphery-to-area ratio).
- Exceptional: identical unit capacitor copies; dummies on all four sides (exact unit-cap copies); cross-couple arrays for dielectric gradient cancellation; electrostatic shielding on adjacent metal layer with >= 50 um overhang if no dummies, >= 5 um with dummies.
- Lower plate connected to low-impedance node.

### 7.B Analog-Algorithm Interface

The pattern selection advisor must:

- **CI-7.1:** Emit the five CC rule constraints (Section 7.A.1) as hard constraints for moderate and exceptional tiers.
- **CI-7.2:** Apply the 2D vs 1D decision rule (Section 7.A.2) and emit `pattern_type` with justification.
- **CI-7.3:** For resistor matching groups, emit Seebeck cancellation constraint (`even_segments = true`, `antiparallel_current = true`).
- **CI-7.4:** For capacitor matching groups, emit unit-cap constraints, dummy requirements, and shielding requirements.
- **CI-7.5:** Emit orientation chi constraint for all matched transistor groups.

---

## 8. Constraint File Format

```json
{
  "version": "2.0",
  "symmetry_groups": [
    {
      "id": "SG1",
      "hierarchy": "OTA1",
      "axis": "vertical",
      "pairs": [
        {
          "device_a": "M1",
          "device_b": "M2",
          "type": "mirror",
          "strength": "hard",
          "matching_tier": "moderate",
          "source": "auto_GNN",
          "confidence": 0.98
        }
      ],
      "self_symmetric": ["M_tail"],
      "isolation_domain": "analog_core"
    }
  ],
  "lde_bounds": [
    {
      "pair": ["M1", "M2"],
      "matching_tier": "moderate",
      "wpe_min_well_distance_um": 3.0,
      "wpe_edge_distance_tolerance_um": 0.1,
      "lod_max_sa_mismatch_um": 0.1,
      "lod_max_sb_mismatch_um": 0.1,
      "dummy_spec": "full_dummy_L_lt_1um",
      "no_metal_over_gate": true,
      "block_dummy_metal": true,
      "die_position": "central_half",
      "min_distance_from_power_mW_um": 500,
      "thermal_budget_C": 0.5,
      "hydrogenation_block_um": 5.0,
      "mismatch_budget_breakdown": {
        "random_pct": 40,
        "wpe_pct": 15,
        "lod_pct": 15,
        "thermal_pct": 10,
        "stress_pct": 10,
        "routing_pct": 10
      }
    }
  ],
  "isolation_constraints": [
    {
      "domain": "charge_pump_1",
      "guard_ring_type": "HBGR",
      "guard_ring_encloses": "complete",
      "min_spacing_to_analog_core_um": 50,
      "substrate_contact_density": "type_3_annular",
      "source": "rule_catalog"
    },
    {
      "domain": "power_stage",
      "guard_ring_type": "ECGR",
      "min_spacing_to_matched_devices_um": 200,
      "source": "rule_catalog"
    }
  ],
  "net_classes": [
    {"net": "inp", "class": "differential", "match_with": "inn"},
    {"net": "clk", "class": "clock"},
    {"net": "vdd", "class": "power"},
    {"net": "vss_sub", "class": "substrate", "topology": "star", "dedicated_supply": true},
    {"net": "guard_ring_1", "class": "guard_ring", "continuity_required": true, "min_width_um": 0.8},
    {"net": "out_p", "class": "high_impedance", "C_par_max_fF": 0.5},
    {"net": "comp_node", "class": "compensation"}
  ],
  "crosstalk_exclusions": [
    {"net_a": "clk", "net_b": "out_p", "min_spacing_um": 2.0, "shield_required": true}
  ],
  "electrical_constraints": [
    {"net": "vdd", "min_wire_width_um": 0.5, "min_via_cuts": 4, "ir_drop_budget_mV": 50},
    {"net": "out_p", "max_parasitic_R_ohm": 10.0, "max_parasitic_C_fF": 0.5}
  ],
  "thermal_tags": [
    {"device": "M_power", "power_mW": 2.5, "thermal_zone_radius_um": 200}
  ],
  "pattern_recommendations": [
    {
      "group": ["M1", "M2"],
      "pattern": "CC_1D_ABBA",
      "reason": "diff_pair_moderate_matching",
      "orientation_chi": 0,
      "cc_rules": ["coincidence", "symmetry", "compactness", "orientation"],
      "dummy_type": "full"
    }
  ],
  "substrate_contact_requirements": [
    {
      "region": "analog_core",
      "contact_type": "disperse_small",
      "max_spacing_um": 50,
      "target_resistance_ohm": 5.0
    }
  ],
  "voltage_domain_spacings": [
    {
      "domain_a": "io_5v",
      "domain_b": "core_1v2",
      "min_spacing_um": 10.0,
      "source": "well_breakdown_analysis"
    }
  ]
}
```

### 8.A Analog Considerations: Constraint File Completeness Check

Before S1 emits its final constraint file, it must run a **completeness check** against the following checklist. Any missing constraint category for a relevant circuit context constitutes a potential failure mode:

| Check | Condition | Required Constraint | Failure If Missing |
|-------|-----------|--------------------|--------------------|
| 1 | Matched pair exists | Matching tier + LDE bounds + pattern recommendation | Systematic mismatch; offset exceeds spec |
| 2 | Power device exists | Thermal zone + guard ring + EM-derated wiring | Thermal gradient degrades nearby matching; latchup risk |
| 3 | Pin-connected diffusion with R_path < 10 kOhm | Guard ring around potential injector | Minority carrier injection; latchup under transient |
| 4 | Different voltage domains on same die | Voltage-dependent spacing | Well breakdown; latchup across domains |
| 5 | Charge pump or switched-cap circuit | HBGR enclosure + substrate contact density | Every clock edge injects minorities; chronic latchup |
| 6 | Sensitive node adjacent to clock/digital | Crosstalk exclusion | 5x offset worsening from coupling |
| 7 | Capacitor array for DAC/ADC | Unit-cap matching + dummy + shield | INL/DNL degradation |
| 8 | Resistor divider in precision path | Resistor matching constraints + Seebeck cancellation | Gain error; linearity error |
| 9 | Substrate net | Dedicated topology (star, separate from VSS) | Ground bounce; substrate debiasing |
| 10 | Guard ring net | Continuity + width constraints | Guard ring ineffective at gaps |
| 11 | Fuse or PO opening | Distance from sensitive analog | Mobile ion ingress shifts Vth over product lifetime |
| 12 | Die edge proximity | Scribe seal + edge exclusion zone | Mechanical stress gradient; contamination |

### 8.B Analog-Algorithm Interface

- **CI-8.1:** The completeness check (Section 8.A) must run before S1 output is finalized. Any failed check emits a warning with the specific missing constraint and the associated failure mode.
- **CI-8.2:** The constraint file version must be "2.0" to signal that analog-enriched fields are present.

---

## 9. Hard vs. Soft Constraint Classification

This section defines the analog-domain classification of constraints into hard (inviolable) and soft (negotiable), following the distinction from Lienig [FOLD 4.5 Sect.4.5.2]: "While missed optimization goals limit quality (but not functionality), violated constraints render the layout useless."

### 9.1 Hard Constraints (Inviolable)

Hard constraints are those whose violation causes the circuit to be non-functional, destructive, or unreliable beyond acceptable limits. They have binary pass/fail semantics -- the degree of violation is irrelevant.

| Constraint | Why It Is Hard | Source |
|-----------|---------------|--------|
| **DRC (all technology rules)** | Manufacturing impossible without compliance | [FOLD 4.5 Sect.4.5.2] |
| **LVS correctness** | Wrong connectivity = wrong circuit | [FOLD 4.5 Sect.4.5.2] |
| **Orientation uniformity** of matched devices | Orientation mismatch causes ~15% gm error -- exceeds any reasonable spec | [00_ANALOG_PRINCIPLES Sect.1.3, Rank 1] |
| **Guard ring completeness** for identified injectors | Incomplete guard ring = no guard ring; latchup risk unchanged | [Hastings Ch.14], [Hastings Ch.5 Sect.5.4.4] |
| **Isolation junction reverse bias** (substrate to most extreme supply) | Forward-biased isolation junction injects minority carriers; causes malfunction or destruction | [Hastings Ch.2 Sect.2.6.1] |
| **EM compliance** (J < J_max on all segments) | Violation causes open-circuit failure in field | [00_ANALOG_PRINCIPLES Sect.4.1] |
| **Antenna ratio compliance** | Violation causes gate oxide damage during fabrication | [00_ANALOG_PRINCIPLES Sect.4.5] |
| **ESD protection at every pad** | Missing protection = immediate field failure from first static discharge | [Hastings Ch.5 Sect.5.1.5] |
| **Latchup design rules** (well/substrate contact spacing) | Violation creates destructive latchup susceptibility | [Hastings Ch.5 Sect.5.4.2], [00_ANALOG_PRINCIPLES Sect.4.6] |
| **Voltage-dependent well spacing** | Violation causes well-to-well breakdown | [Hastings Ch.2 Sect.2.6.1] |
| **Exceptional-tier matching orientation chi = 0** | Any non-zero chi produces systematic mismatch that CC cannot cancel | [Hastings Ch.13], [00_ANALOG_PRINCIPLES Sect.1.5] |

### 9.2 Soft Constraints (Negotiable)

Soft constraints are those whose violation degrades performance but does not cause outright failure. They can be relaxed during constraint negotiation (Section 10) when the constraint set is over-constrained.

| Constraint | Negotiation Range | Performance Impact of Relaxation | Source |
|-----------|------------------|----------------------------------|--------|
| **Matched net length matching** | 1% -> 5% -> 10% | Parasitic R/C mismatch increases; offset degrades proportionally | [00_ANALOG_PRINCIPLES Sect.3.4] |
| **CC pattern type** (2D -> 1D -> interdigitated) | 2D best, interdigitated worst | Residual gradient mismatch: 60% (2D) -> 100% (1D) -> uncancelled | [00_ANALOG_PRINCIPLES Sect.1.5] |
| **Well edge distance** | 5 um -> 3 um -> 2 um | WPE contribution to mismatch increases exponentially | [00_ANALOG_PRINCIPLES Sect.2.1] |
| **Distance from power device** | 500 um -> 200 um -> 100 um | Thermal gradient contribution increases as 1/d | [00_ANALOG_PRINCIPLES Sect.4.3] |
| **Die-center placement** | Central 25% -> central 50% -> central 75% | Packaging stress gradient increases toward edges | [00_ANALOG_PRINCIPLES Sect.2.3] |
| **Dummy metal blocking zone** | 5 um -> 3 um -> 1 um | Hydrogenation blocking risk increases | [Hastings Ch.13 Rule 18] |
| **Crosstalk spacing** | 3x -> 2x -> 1x dielectric thickness | Coupling coefficient increases as 1/spacing | [00_ANALOG_PRINCIPLES Sect.3.4] |
| **Substrate contact density** | Every 20x -> 30x -> 50x layer thickness | Debiasing resistance increases; latchup margin decreases | [Hastings Ch.5 Sect.5.4.3] |
| **Array compactness** (aspect ratio) | 1:1 -> 2:1 -> 3:1 | Higher-order gradient cancellation degrades | [Hastings Ch.13 Rule 9] |

### 9.3 Priority Ordering

When soft constraints conflict with each other, the priority order from the analog hierarchy of needs [00_ANALOG_PRINCIPLES Sect.5] applies:

```
Priority 1: Matching correctness
Priority 2: Isolation / guard rings
Priority 3: Parasitic minimization
Priority 4: Reliability (EM, ESD, antenna)
Priority 5: Area
```

Within each priority level, the Strasser importance order [ALS 3.2] resolves sub-conflicts:

```
M_S > M_B > P_B > S > P_N
```

### 9.B Analog-Algorithm Interface

- **CI-9.1:** Every constraint in the output file must carry a `strength` field: `hard` or `soft`.
- **CI-9.2:** Every soft constraint must carry a `priority` field (integer 1-5, from Section 9.3) and a `relaxation_range` (min, default, max values).
- **CI-9.3:** The algorithm must never relax a hard constraint. If the constraint set is infeasible with all hard constraints, it must report failure rather than silently violating a hard constraint.

---

## 10. Constraint Negotiation and Relaxation

This section implements the constraint engineering methodology from Jerke, Lienig, and Freuer [ALS 7.1, 7.5] adapted to the analog PNR pipeline.

### 10.1 When Negotiation Occurs

Constraint negotiation is triggered when:
1. **S2 (placement)** reports that the constraint set is infeasible (no legal placement satisfies all constraints).
2. **S3 (routing)** reports that symmetric routing for a matched net pair requires more metal layers than available, or that parasitic targets cannot be met.
3. **S0 (cell generation)** reports that the recommended CC pattern does not fit within the area budget.

### 10.2 The Negotiation Protocol

Following the CLP-based constraint satisfaction resolution from [ALS 7.1 Sect.7.4.1]:

```
1. IDENTIFY the infeasible constraint subset.

2. RANK soft constraints in the subset by priority (Section 9.3).

3. RELAX the lowest-priority soft constraint first:
   - Move its value from current to next relaxation step
     (e.g., well_distance from 5 um to 3 um).
   - Re-run feasibility check.

4. If still infeasible, relax the next-lowest-priority constraint.

5. ITERATE until feasible or all soft constraints are at their
   maximum relaxation.

6. If still infeasible after all soft constraints are maximally
   relaxed, report HARD FAILURE:
   - List all hard constraints that contribute to infeasibility.
   - Suggest designer intervention (e.g., "increase die area",
     "use deep N-well for additional isolation",
     "relax matching spec from exceptional to moderate").
```

### 10.3 Constraint Sensitivity Analysis for Negotiation

When multiple soft constraints could be relaxed, the constraint sensitivity analysis (CSA) from [ALS 7.1 Sect.7.4.4] guides the choice. For each soft constraint c_i with current value x_i and bounds [x_l, x_u]:

```
Relative distance to bound:
  d_l(x) = exp(x_l - x) - 1    (lower bound proximity)
  d_u(x) = exp(x - x_u) - 1    (upper bound proximity)

If d_l > 0 or d_u > 0: constraint is violated.
If d_l < 0 and d_u < 0: constraint is satisfied with margin.
```

**Relaxation strategy:** Relax the constraint with the smallest absolute sensitivity to the primary performance metric (i.e., the constraint whose relaxation causes the least performance degradation) [ALS 7.1 Sect.7.4.4].

### 10.4 Feedback Loop to Circuit Design

If constraint negotiation exhausts all soft constraints and the system remains infeasible, this is an indication that the **circuit design itself** may need modification. The Lienig "bottom-up meets top-down" vision [FOLD 4.7 Sect.4.7.2] applies: the layout tool must provide structured feedback to the circuit designer specifying:

- Which constraints are infeasible
- What design change would resolve them (e.g., "increase overdrive voltage on matched pair M1/M2 from 50 mV to 150 mV to reduce Pelgrom area requirement by 9x")
- The quantitative tradeoff (performance loss vs. layout feasibility)

### 10.B Analog-Algorithm Interface

- **CI-10.1:** S1 must expose a `relax_constraint(constraint_id, new_value)` API that S2/S3 can call during placement/routing.
- **CI-10.2:** Each relaxation step must re-compute the mismatch budget breakdown (Section 8 constraint file) and verify that the total mismatch remains within the performance spec.
- **CI-10.3:** If relaxation causes the predicted total mismatch to exceed the spec, S1 must emit a warning with the quantitative impact before proceeding.

---

## 11. Failure Mode Catalog

This section documents specific failure modes that occur when constraint extraction misses critical constraints. Each entry gives the missing constraint, the circuit context, the physical mechanism, and the quantitative consequence.

### 11.1 Missing Current Mirror Matching Requirement

**Context:** A simple current mirror (M1, M2) biases a differential pair. The GNN identifies M1 and M2 as topologically identical but assigns only minimal matching tier because the mirror is not part of a symmetry compound.

**Missing constraint:** Moderate or exceptional matching tier (depending on offset spec).

**Physical mechanism:** Without interdigitation or CC layout, M1 and M2 experience different process gradients (oxide thickness, doping). The systematic Vth mismatch follows the gradient term of the Pelgrom model:

```
sigma^2(dVth)_systematic = S_VT^2 * D^2

where D = centroid-to-centroid distance between M1 and M2.
```

**Quantitative consequence:** For S_VT = 0.5 mV/um (typical 130 nm) and D = 20 um (non-interdigitated placement):

```
dVth_systematic = 0.5 * 20 = 10 mV

For a mirror at Vgs - Vth = 200 mV:
  dId/Id = 2 * dVth / (Vgs - Vth) = 2 * 10 / 200 = 10%

Impact: 10% systematic current offset in the diff pair tail,
which directly maps to input-referred offset via CMRR degradation.
```

### 11.2 Missing WPE Equalization

**Context:** A matched differential pair (M1, M2) is placed inside an N-well guard ring. M1 is 1 um from the left well edge; M2 is 4 um from the right well edge.

**Missing constraint:** WPE edge distance equalization.

**Physical mechanism:** Scattered well-implant ions raise |Vth| near well edges. The asymmetric placement creates [00_ANALOG_PRINCIPLES Sect.2.1]:

```
dVth_WPE(M1) = a * exp(-1/lambda) ~ a * exp(-0.5) = 0.61a  (at 1 um, lambda=2)
dVth_WPE(M2) = a * exp(-4/lambda) ~ a * exp(-2.0) = 0.14a  (at 4 um, lambda=2)

For a = 30 mV:
  mismatch = 0.61*30 - 0.14*30 = 14 mV
```

**Quantitative consequence:** 14 mV systematic Vth mismatch -- far worse than random Pelgrom mismatch for any reasonable device size. This is particularly insidious because common-centroid layout does NOT cancel it (WPE is device-local, not a gradient) [00_ANALOG_PRINCIPLES Sect.2.1].

### 11.3 Missing Guard Ring for Charge Pump

**Context:** A charge pump with flying capacitors is placed adjacent to an analog core without a guard ring. On every clock edge, the charge pump's switching transistors forward-bias a junction, injecting minority carriers into the substrate [Hastings Ch.5 Sect.5.4.1].

**Missing constraint:** HBGR enclosure around charge pump.

**Physical mechanism:** Minority carriers (electrons in P-sub) diffuse through the substrate. Average diffusion distance before recombination is approximately 200 um in 10 Ohm-cm P-type silicon [Hastings Ch.5 Sect.5.4.3]. Any reverse-biased junction within this distance collects some carriers, causing:

1. Parametric shift in nearby analog devices (dVth proportional to collected current)
2. Substrate debiasing (IR drop across R_sub forward-biases other junctions)
3. Possible latchup if beta_NPN * beta_PNP >= 1 [00_ANALOG_PRINCIPLES Sect.4.6]

**Quantitative consequence:** Without a guard ring, the substrate current from the charge pump can cause:
- 1-10 mV periodic Vth modulation in nearby analog devices (synchronous with pump clock)
- Substrate noise that appears as clock feedthrough in the analog output
- In worst case, latchup that destroys the chip. Latchup is triggered when substrate debiasing exceeds approximately 0.5 V (at 100 mA latchup test current through 5 Ohm distributed resistance) [Hastings Ch.5 Sect.5.4.2].

### 11.4 Missing LOD Equalization (No Dummies)

**Context:** An array of four matched transistors for a DAC current cell. End devices have different SA/SB than interior devices because the OD region terminates at the array edge.

**Missing constraint:** Dummy devices at array edges with specified SA/SB values.

**Physical mechanism:** STI compressive stress modulates carrier mobility. End devices see more stress from nearby STI than interior devices [00_ANALOG_PRINCIPLES Sect.2.2]:

```
f(SA_end, SB_end) != f(SA_interior, SB_interior)

For SA_end = 0.5 um, SA_interior = 2 um:
  delta_Id/Id = K1*(1/0.5 - 1/2) + K2*(1/0.25 - 1/4)
  
For typical K1 = 0.1, K2 = 0.01:
  delta_Id/Id ~ 0.1*(2.0 - 0.5) + 0.01*(4.0 - 0.25)
             = 0.15 + 0.0375 = 18.75%
```

**Quantitative consequence:** Up to 13-19% current mismatch between end and interior devices. In a DAC, this directly maps to INL error at the transitions involving end devices. For a 10-bit DAC with 0.5 LSB INL target, a single 18% current error in any unit cell violates the spec by over 100x.

### 11.5 Missing Hydrogenation Blocking Constraint

**Context:** A matched pair has metal routing crossing over one device's gate but not the other's. The dummy metal fill algorithm places fill patterns over one device but not its match.

**Missing constraint:** `no_metal_over_gate` and `block_dummy_metal` for matched pairs.

**Physical mechanism:** Metal blocks hydrogen diffusion during passivation anneal [Hastings Ch.13], [00_ANALOG_PRINCIPLES Sect.2.4].

**Quantitative consequence:** Up to 20% systematic Id mismatch between metal-covered and uncovered devices. This is a hidden failure mode -- it does not appear in any schematic simulation, only in silicon.

### 11.6 Missing Substrate Net Separation

**Context:** The substrate contact net (PSUB) is routed on the same metal as the circuit ground (VSS). Digital switching currents flow through shared ground routing, causing IR drop that modulates the substrate potential at analog device locations.

**Missing constraint:** Dedicated substrate net with star topology, separate from circuit VSS.

**Physical mechanism:** Substrate debiasing [00_ANALOG_PRINCIPLES Sect.4.4]:

```
V_debiasing = R_sub * I_sub

where I_sub includes digital switching current flowing through
the shared ground path.
```

**Quantitative consequence:** For a digital block switching 10 mA at 100 MHz through 1 Ohm of shared ground resistance: 10 mV ground bounce at the substrate contact. This modulates Vth of nearby analog devices by 10 mV and couples as a 100 MHz tone into the analog signal path. In a 14-bit ADC with 1 LSB = 60 uV, this represents a ~170 LSB disturbance.

---

## Appendix A: Source Cross-Reference

| Section | Primary Sources |
|---------|----------------|
| 1.A (Why constraint extraction is hard) | [ALS 7.1 Sect.7.2.2], [FOLD 4.6] |
| 2.A (Isolation domains) | [Hastings Ch.2 Sect.2.6], [Hastings Ch.5 Sect.5.4], [Hastings Ch.14] |
| 3.A (What GNN misses) | [Hastings Ch.8, Ch.13], [00_ANALOG_PRINCIPLES Sect.1.3-1.9] |
| 4.A (Full LDE set) | [00_ANALOG_PRINCIPLES Sect.2.1-2.4], [Hastings Ch.8, Ch.13] |
| 5.A (Extended net classes) | [00_ANALOG_PRINCIPLES Sect.3-4], [Hastings Ch.5, Ch.14] |
| 6.A (Beyond DC OP) | [00_ANALOG_PRINCIPLES Sect.3-4], [Hastings Ch.5 Sect.5.1.3] |
| 7.A (Pattern heuristics) | [Hastings Ch.8, Ch.13], [00_ANALOG_PRINCIPLES Sect.1.5-1.9] |
| 9 (Hard vs. soft) | [FOLD 4.5 Sect.4.5.2], [ALS 7.1], [00_ANALOG_PRINCIPLES Sect.5] |
| 10 (Negotiation) | [ALS 7.1 Sect.7.4.1, 7.4.4], [ALS 7.5], [FOLD 4.7] |
| 11 (Failure modes) | [Hastings Ch.5, Ch.8, Ch.13], [00_ANALOG_PRINCIPLES Sect.2, 4] |

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| COV-03 | Expanded ESD_bus net class entry in Section 5.A.1 to include `max_ring_resistance_ohm: 2.0`, `max_total_resistance_ohm: 2.0`, `topology: peripheral_ring`, `chamfer_corners: true`, and `CDM_clamp_max_distance_from_gate_um: 50` | ESD ground ring resistance budget and CDM placement proximity were not specified in the net class definition |
| MISSING-03 | Added three new isolation domain triggers in Section 2.A: NBTI-induced mismatch for matched PMOS pairs with differential Vgs > 100 mV, HCI susceptibility for NMOS at Vgs ~ 0.4*Vds with Vds > 2V, and field plate constraint for every lateral PNP transistor | Surface effects (HCI, NBTI, parasitic channels) from Hastings Ch. 5.3 were not represented as constraint generation triggers |
| MISSING-04 | Added merged device debiasing analysis paragraph in Section 2.A: compute V_debias = I_device * R_well for shared-well devices, flag if > 500 mV | Debiasing analysis for merged devices was missing; shared well resistance can forward-bias junctions causing latchup |
| MISSING-08 | ESD_bus net class expansion (same edit as COV-03) also addresses FOLD 7.4 ESD ground ring requirements | FOLD Section 7.4.1 ESD layout rules were not in the enriched specs |
