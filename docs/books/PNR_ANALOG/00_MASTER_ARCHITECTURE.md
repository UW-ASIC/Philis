# Automatic Analog Place & Route Engine: Master Architecture Specification

**Version:** 1.0 — June 2026
**Scope:** Complete netlist-to-GDSII automatic analog/mixed-signal layout engine

---

## 1. Executive Summary

This document specifies a five-stage automatic analog P&R engine that takes an unannotated SPICE netlist and a PDK as input and produces a DRC/LVS-clean, performance-verified GDSII layout. The architecture integrates techniques from the leading academic systems (MAGICAL, ALIGN, BAG/LAYGO) with device-level layout knowledge from Razavi, Hastings, Baker, and Sansen, and physics-aware optimization for layout-dependent effects, thermal gradients, electromigration, and substrate noise.

The pipeline stages are:

- **S0 — Device-level cell generation:** Converts schematic devices into physical cells with multi-finger decomposition, interdigitation/common-centroid pattern selection, dummy insertion, guard rings, and well taps.
- **S1 — Constraint extraction:** Automatically identifies symmetry groups, matching requirements, LDE sensitivity bounds, net classifications, and electrical constraints from the netlist using GNN-based graph learning.
- **S2 — Placement:** Positions cells using NLP-based global placement (with wirelength, area, symmetry, LDE, thermal, and performance terms) followed by ILP-based detailed placement with hard constraints and device flipping.
- **S3 — Routing:** Connects nets using VAE-learned routing guidance, four-variant symmetry-aware A* routing with pin clustering, and post-route verification for parasitic matching, EM, and IR drop.
- **S4 — Sign-off & verification:** DRC, LVS, antenna check, density fill, parasitic extraction, and post-layout SPICE simulation with spec comparison.

Three feedback loops close the optimization:
- **Inner loop 1 (S3->S2):** Routability/congestion feedback into placement.
- **Inner loop 2 (S4->S3):** DRC/antenna violation feedback into routing.
- **Outer loop (S4->S1/S0):** Performance-gap-driven re-optimization.

### 1.A Analog Design Philosophy

The following overarching principles, distilled from Hastings, Lienig, and industry practice, must pervade every stage of the pipeline. They are listed in priority order and correspond to the "Hierarchy of Needs" in `00_ANALOG_PRINCIPLES.md`.

**Principle 1 — Matching by construction, not by correction.** Matching is the single most important layout concern in analog IC design [Hastings, Ch. 8, Ch. 13]. Every mechanism of mismatch -- orientation, WPE, LOD/STI stress, thermal gradient, mechanical stress, hydrogenation blocking, process gradients, etch-rate variation, thermoelectric EMF -- must be addressed at device-generation time (S0), not patched later. The Pelgrom model (`sigma^2(dVth) = A_VT^2 / (W*L) + S_VT^2 * D^2` [Pelgrom JSSC 1989]) governs random mismatch; systematic mismatch sources (ranked in 00_ANALOG_PRINCIPLES Section 1.3) are reduced by layout technique. Hastings defines three tiers of matching -- minimal, moderate, exceptional -- each demanding progressively more area, more common-centroid discipline, and more careful dummy/guard-ring practice.

**Principle 2 — Isolation before optimization.** Isolation failures (latchup, substrate noise coupling, minority carrier injection) are catastrophic and can destroy the chip [Lienig, 7.1.3]. Guard rings around every potential minority-carrier injector, dedicated analog supply domains, and substrate contact discipline must be allocated before any area optimization pass. The latchup condition `B_NPN * B_PNP >= 1` [Lienig, 7.1.3] must never be approachable.

**Principle 3 — Signal flow drives geometry.** Analog circuits are not random graphs; they have a signal flow from input to output, with bias networks, feedback paths, and power delivery that must be spatially coherent [Lienig, Ch. 5; Hastings, Ch. 15]. The floorplan should reflect the schematic topology: forward signal path left-to-right, feedback bottom-to-top [MAGICAL ICCAD 2020]. Elongated cells that separate heat sources from sensitive inputs improve thermal matching [Hastings, Ch. 15, Section 15.2].

**Principle 4 — Analog is full-custom.** Analog circuits are always full-custom designed because they require many more degrees of freedom than digital to handle their vast diversity of constraints [Lienig, Section 4.3]. There are no standard cells; geometric representations are created in real time using layout generators or by manual drawing [Lienig, Section 4.1]. The meet-in-the-middle design style is standard: bottom-up device generation + top-down system composition [Lienig, Section 4.3.2].

**Principle 5 — Parasitics are first-order, not post-hoc.** Parasitic capacitances as small as 1 fF can cause circuit malfunctions [Hastings, Ch. 15, Section 15.1]. High-impedance nodes (cascode drains, opamp outputs) have bandwidth `BW ~ 1/(2*pi*R_out*C_parasitic)` that degrades with any added capacitance. Interconnect parasitics became a significant concern below the 0.5 um technology node [Lienig, Section 5.4.6]. The engine must model parasitics at every stage, not just at sign-off.

**Principle 6 — The hierarchy of needs.** When algorithm objectives conflict, the priority order is: (1) Matching correctness, (2) Isolation / guard rings, (3) Parasitic minimization, (4) Reliability (EM/ESD/antenna), (5) Area. A larger die that meets matching spec is always preferable to a smaller die that does not. See `00_ANALOG_PRINCIPLES.md` Section 5 for quantitative justification.

### 1.B Architectural Assumptions Flagged for Analog Practice

The following assumptions in the original architecture require explicit attention in an analog context:

| Assumption | Conflict with Analog Practice | Resolution |
|---|---|---|
| "Unannotated SPICE netlist" as input | Analog netlists carry implicit matching intent, bias conditions, signal sensitivity, and noise criticality that are not captured in bare SPICE. Circuit designers must communicate noisy signals ("_n" suffix) and sensitive signals ("_s" suffix) [Hastings, Ch. 15, Section 15.4.4]. | S1 must infer or request annotations for matching groups, signal sensitivity, noise classification, and current flow direction. The optional `sigpath` and `sym/symnet` files partially address this, but the engine should also accept explicit matching-tier annotations (minimal/moderate/exceptional). |
| Single NLP objective with weighted sum | Analog constraints are not smoothly tradeable. Matching is a hard requirement, not a soft penalty. A 0.1% improvement in area is never worth a 1% degradation in matching. | The soft-to-hard progression (Section 3) is correct. But the weight hierarchy must be enforced: matching and isolation weights must dominate area weights by at least 10x at all stages. |
| "Runtime < 5 min for block-level" target | Analog layout quality is not time-bounded. A 5-minute runtime that produces a 5% offset is useless; a 60-minute runtime that produces a 0.1% offset is valuable. | Runtime targets should be secondary to quality targets. Report runtime alongside quality, but never sacrifice matching for speed. |
| Generate-and-select over single-point optimization | Correct for analog: multiple Pareto-optimal candidates let the designer choose the matching-area tradeoff. | The Pareto front must include matching quality as a primary axis, not just area and wirelength. |

---

## 2. Pipeline Overview

```
Input: Unannotated SPICE netlist + PDK + Performance specifications
  |
  v
+-----------------------------------------------------+
| S0 - DEVICE-LEVEL CELL GENERATION                   |
|                                                     |
|  +----------+ +----------+ +----------+ +--------+ |
|  | Finger   | | Pattern  | | Dummy    | | Guard  | |
|  | decomp.  |->| select   |->| insert   |->| ring   | |
|  |          | | CC/ID/CL | |          | | + taps | |
|  +----------+ +----------+ +----------+ +--------+ |
|  Output: Cell library with pin maps & bounding boxes|
+-------------------------------------+---------------+
                                      |
                                      v
+-----------------------------------------------------+
| S1 - CONSTRAINT EXTRACTION                          |
|                                                     |
|  +----------+ +----------+ +----------+ +--------+ |
|  | Hierarchy| | GNN sym. | | LDE/     | | Net    | |
|  | extract  |->| extract  |->| thermal  |->| class  | |
|  |          | |          | | bounds   | |        | |
|  +----------+ +----------+ +----------+ +--------+ |
|  Output: Constraint file (sym groups, LDE, nets)    |
+-------------------------------------+---------------+
                                      |         ^
                                      v         | Outer
+-----------------------------------------------------+
| S2 - PLACEMENT                          <-- S3->S2  |
|                                          routability|
|  +-----------------------+  +------------------+    |
|  | NLP Global Placement  |  | ILP Detail       |    |
|  | WL+Area+Sym+LDE+     |-> | Hard symmetry    |    |
|  | Thermal+Performance   |  | Flipping, align  |    |
|  +-----------------------+  +------------------+    |
|  Output: Legalized placement with pin positions     |
+-------------------------------------+---------------+
                                      |
                                      v
+-----------------------------------------------------+
| S3 - ROUTING                            <-- S4->S3  |
|                                          DRC fixes  |
|  +--------+ +----------+ +----------+ +----------+ |
|  | VAE    | | Sym.     | | A* route | | Verify:  | |
|  | guide  |->| alloc +  |->| + rip-up |->| parasit. | |
|  | gen    | | pin clus | | reroute  | | EM/IR    | |
|  +--------+ +----------+ +----------+ +----------+ |
|  Output: Routed layout with verified interconnect   |
+-------------------------------------+---------------+
                                      |
                                      v
+-----------------------------------------------------+
| S4 - SIGN-OFF & VERIFICATION                        |
|                                                     |
|  +--------+ +--------+ +--------+ +--------------+ |
|  | DRC    | | LVS    | | PEX    | | Post-layout  | |
|  | + ant. |->|        |->| R+C+CC |->| SPICE sim    | |
|  | + fill | |        | |        | | vs. specs    | |
|  +--------+ +--------+ +--------+ +--------------+ |
|                                                     |
|  +----------+                                       |
|  | PASS?    |--yes--> GDSII Output                  |
|  |          |--no---> Outer loop (->S1 or ->Input)  |
|  +----------+                                       |
+-----------------------------------------------------+
```

### 2.A Analog Considerations

The pipeline diagram above correctly shows the linear flow with feedback loops. From an analog perspective, several observations:

**Signal flow must be preserved.** The pipeline transforms a netlist (connectivity) into geometry (layout). At no point should the algorithm lose track of the circuit's signal flow direction. S1 must extract or accept a signal-flow annotation, and S2 must use it to orient blocks so the layout reads like the schematic [Lienig, Section 4.3.2; MAGICAL ICCAD 2020]. This is not merely aesthetic; it makes the layout verifiable by the circuit designer.

**The outer loop must include S0.** If post-layout simulation reveals that a matched pair's offset exceeds spec, the fix may not be a placement or routing change -- it may require changing the S0 cell generation parameters (more fingers, 2D CC instead of 1D, wider dummies, different guard ring style). The outer loop's path from S4 back to S1 must also trigger S0 re-generation when matching-related failures are detected.

**Hierarchical vs. flat processing.** Analog ICs are designed meet-in-the-middle [Lienig, Section 4.3.2]. The pipeline should process hierarchically: generate cells bottom-up (S0), extract constraints per hierarchy level (S1), place blocks at the system level using block-level results as fixed macros. Flat processing of a 200-device modulator would lose the matching relationships that hierarchy encodes. Memory blocks and IP elements should be verified hierarchically to reduce LVS runtime [Lienig, Section 5.4.6].

### 2.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| S0 must produce cells where all matched devices within a cell have identical orientation (chi = 0 for multi-finger devices) | S0 cell generator | Orientation mismatch causes ~15% gm error [Hastings, Ch. 13] |
| S1 must classify every net as one of: differential, clock, power/ground, sensitive-analog, noisy-digital, general | S1 constraint extractor | Net classification drives routing strategy, shielding, and parasitic budgets [Hastings, Ch. 15, Section 15.4.4] |
| S2 must never place a matched pair at different distances from a well edge | S2 placement engine | WPE causes 5-25% current mismatch within 5 um of well edge [Hastings, Ch. 13] |
| S3 must route matched nets on the same metal layers with identical via stacks | S3 routing engine | Asymmetric parasitics create offset; matched routing must match R and C to within ~1% [Hastings, Ch. 8] |
| S4 must report matching-related metrics (offset, CMRR, PSRR, INL/DNL) separately from generic performance metrics | S4 sign-off | Matching failures have different root causes than bandwidth/gain failures and require different remediation paths |
| The outer loop must distinguish matching failures from performance failures and route them to different remediation stages (S0 for matching, S2 for performance) | Flow controller | A matching failure cannot be fixed by adjusting GP weights; it requires cell-level changes |

---

## 3. Design Principles

1. **Matching by construction, not by correction.** Device-level matching (unit fingers, common-centroid, dummies, guard rings) is built into cells at S0, not patched after placement.

2. **Constraints are first-class objects.** Every constraint has a type (symmetry, matching, LDE, thermal, electrical), a scope (device-level, system-level), a strength (hard, soft with weight, advisory), and provenance (auto-extracted, designer-specified, learned from data).

3. **Soft-to-hard constraint progression.** Global placement uses soft penalties; detailed placement enforces hard constraints. This avoids over-constraining the global optimizer.

4. **Physics-aware at every stage.** LDE effects (WPE, LOD/STI), thermal gradients, parasitic matching, EM, IR drop, and substrate noise are modeled, not ignored until sign-off.

5. **Proxy-reality feedback.** Every geometric proxy (HPWL, symmetry score, congestion estimate) is validated against electrical reality (post-layout SPICE) in the outer loop. The GNN performance predictor is a fast proxy; PEX+SPICE is the truth.

6. **Generate-and-select over single-point optimization.** Produce multiple candidate placements/routings (Pareto front of area vs. performance vs. matching) and select, rather than committing to a single solution early.

### 3.A Analog Considerations

**On Principle 1 (Matching by construction):** This principle, correctly stated, is the single most important architectural decision. It aligns with Hastings' 25 resistor matching rules [Ch. 8, Section 8.3.1], 24 MOS transistor matching rules [Ch. 13, Section 13.3], and 13 capacitor matching rules [Ch. 8, Section 8.3.2]. The five rules of common-centroid layout -- coincidence, symmetry, dispersion, compactness, orientation [Hastings, Ch. 13, Table 13.2] -- must be enforced at S0 cell generation time. A 2D cross-coupled AB/BA array exhibits ~60% of the residual gradient-induced mismatch of a 1D ABBA array [Hastings, Ch. 13]; the engine should prefer 2D when device geometry allows (W ~ L).

**On Principle 3 (Soft-to-hard progression):** This is correct but must be applied carefully. Some constraints are never soft: orientation uniformity within a matching group, guard ring completeness, and dummy device presence. These are hard from the first GP iteration. The constraints that benefit from soft-to-hard progression are geometric alignment (exact axis position) and relative ordering (which device is left vs. right).

**On Principle 4 (Physics-aware at every stage):** The LDE models in `00_ANALOG_PRINCIPLES.md` Section 2 (WPE: `dVth = sum a_k * exp(-d_k/lambda_k)`, LOD: `f(SA,SB) = 1 + K1*(1/SA+1/SB) + K2*(1/SA^2+1/SB^2)`) are differentiable and can be included in the GP objective. The thermal model (`T(x,y) = T_ambient + sum P_d * G(x-x_d, y-y_d)`) is also differentiable. These should have non-zero gradient weight from the first GP iteration, not just added in Phase 3.

**On Principle 6 (Generate-and-select):** The Pareto front should be three-dimensional: area, wirelength, and matching quality (defined as worst-case mismatch across all matched pairs, normalized to spec). The designer should be able to slide along this front and see the tradeoff explicitly.

### 3.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Orientation uniformity and dummy presence are HARD constraints at all stages, never softened | S0, S2 | Orientation mismatch: ~15% gm error; missing dummies: ~1% end-device mismatch [Hastings, Ch. 13] |
| Guard ring completeness is a HARD constraint; guard rings must fully encircle the injector [Hastings, Ch. 14] | S0, S2 | Incomplete guard rings provide negligible protection; latchup risk is binary |
| The Pareto front must include a matching-quality axis | S2, flow controller | Area-only Pareto optimization is meaningless for analog |

---

## 4. Input Specification

| Input | Format | Required? | Description |
|-------|--------|-----------|-------------|
| Netlist | SPICE/Spectre | Yes | Unannotated flat or hierarchical netlist |
| PDK | layers.json / tech LEF | Yes | Design rules, layer stack, device models |
| Performance specs | JSON | Yes | Target metrics (gain, BW, PM, offset, INL, ...) with min/max |
| Designer constraints | sym/symnet files | Optional | Override auto-extracted constraints |
| Signal flow | sigpath file | Optional | System signal flow for schematic-like placement |
| Layout templates | GDSII/OA | Optional | Reference manual layouts for GeniusRoute training |

### 4.A Analog Considerations

**Missing inputs that analog practice requires:**

| Input | Format | Why Needed | Source |
|---|---|---|---|
| Matching tier annotations | JSON or inline SPICE comments | Specifies whether each matched pair requires minimal/moderate/exceptional matching. Without this, the engine must guess, risking under- or over-constraining. | [Hastings, Ch. 8, Ch. 13] |
| Net sensitivity classification | JSON or naming convention (_n, _s suffixes) | Circuit designers must communicate which signals are noisy and which are sensitive [Hastings, Ch. 15, Section 15.4.4]. Layout designers cannot reliably identify them independently. | [Hastings, Ch. 15] |
| DC operating point data | SPICE .op output | Per-device current for EM sizing, per-net impedance for parasitic budgeting, per-device power for thermal modeling. The engine needs `Vgs-Vth` (overdrive) per matched pair to compute `sigma^2(dId)/Id^2 = 4 * sigma^2(dVth) / (Vgs-Vth)^2 + sigma^2(d_beta/beta)`. | [Pelgrom JSSC 1989] |
| Package/die information | JSON (`PackageInfo`) | Die position in package affects mechanical stress distribution. Matched devices should be placed near die center where stress gradients are lowest [Hastings, Ch. 8, Ch. 13]. Package CTE determines stress magnitude. | [Hastings, Ch. 8] |

**PackageInfo data structure** (consumed by S2 for mechanical stress placement constraints):

```
PackageInfo:
  die_width_um: float
  die_height_um: float
  package_type: enum (QFP, BGA, WLCSP, FCBGA, QFN, DIP, SOIC)
  mold_cte_ppm_per_C: float (CTE of mold compound; typical 8-35 ppm/C)
  die_attach_type: enum (EPOXY, SOLDER, EUTECTIC, DAF)
```

S2 consumes `PackageInfo` to compute die-center distances for mechanical stress placement constraints. When `PackageInfo` is absent, S2 assumes a default die size of 2000 x 2000 um with epoxy mold at 15 ppm/C. | [Hastings, Ch. 8] |

### 4.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| If matching tier annotations are absent, S1 must infer them from circuit topology (diff pair gates -> moderate, current mirror gates -> moderate, DAC unit elements -> exceptional) | S1 | Under-constraining matching causes silicon failure; over-constraining wastes area |
| If DC operating point data is absent, S1 must run a SPICE .op simulation to obtain bias currents and voltages | S1 | EM wire sizing, thermal modeling, and Pelgrom-based area sizing all require bias data |

---

## 5. Output Specification

| Output | Format | Description |
|--------|--------|-------------|
| Final layout | GDSII | DRC/LVS-clean, density-filled, antenna-fixed |
| Post-layout netlist | SPICE | Extracted parasitic netlist (R+C+CC) |
| Performance report | JSON/CSV | Every metric vs. spec, pass/fail, margin |
| Constraint report | JSON | All constraints used, which were relaxed, why |
| Layout database | OA/JSON | Intermediate results for debug and re-entry |

### 5.A Analog Considerations

**Additional outputs that analog practice requires:**

| Output | Format | Why Needed |
|---|---|---|
| Matching report | JSON | Per matched pair: achieved mismatch (WPE dVth, LOD dId/Id, parasitic asymmetry), orientation chi, distance D, dummy coverage, CC quality metric. This lets the designer identify which pair limits yield. |
| Thermal map | Image/JSON | 2D temperature distribution across the die, computed from device power dissipation. Highlights matched pairs that straddle isotherms. |
| Parasitic budget report | JSON | Per sensitive net: parasitic R and C vs. budget. Flags nets where parasitic C exceeds `1 / (2*pi*f_target*R_node)`. |
| Guard ring coverage report | JSON | Per device: guard ring type, width, completeness. Flags devices without complete guard rings. |
| ESD protection map | JSON | Per pad: primary protection device type, size, ECGR presence. Flags pads without adequate protection [Hastings, Ch. 14]. |

---

## 6. Stage Specifications (Summary -- see individual documents)

### S0 — Device-Level Cell Generation

**Purpose:** Convert each schematic device into a physical cell that embodies analog layout best practices.

**Key sub-blocks:**
- Multi-finger decomposition engine
- Pattern selector (CC / interdigitated / clustered)
- Dummy device inserter
- Guard ring and well-tap generator
- Passive array generator (capacitor CC arrays, resistor ladders)
- Intra-cell routing (source/drain strapping, gate poly connection)

**Key decisions encoded (from Razavi, Hastings, Baker):**
- Finger width: R_gate < (1/5 to 1/10) x (1/g_m) per finger
- Even finger count for symmetric abutment
- Drain on interior shared diffusions (minimize C_drain)
- ABBA interdigitation for 1:1 pairs; 2-D cross-quad for precision
- At least one ring of OD-merged dummies at array edges
- Guard ring width and tap spacing per PDK latch-up rules
- Identical orientation for all matched devices
- Kelvin connections for precision resistors

**What S0 produces:** A parameterized cell library where each cell has:
- DRC-clean geometry with pin locations
- Bounding box (including guard ring inflation)
- Metadata: device type, sizing, finger count, pattern type, dummy count
- Internal parasitic estimate (per-cell R, C)

#### S0.A Analog Considerations

**Guard ring generation must follow Hastings' guard ring hierarchy [Ch. 14]:**
- Guard the INJECTOR, not the victim (fewer injectors than potential victims)
- HCGR + HBGR combined achieves >99% efficiency for hole blocking
- ECGR width >= P-epi thickness (no retrograde well) or >= P-well depth (retrograde well)
- HBGR must completely encircle the injector
- Connect ECGRs to highest available supply voltage for deepest depletion
- Block dummy metal generation over guard ring contacts

**Dummy device insertion must follow Hastings' rules [Ch. 13, Section 13.3]:**
- Moderate: full dummy for L < 1 um, half dummy for L > 1 um
- Exceptional: outermost dummy poly >= 3 um from nearest active gate; moat extends 5 um beyond last active gate
- Dummies must be OD-merged with active devices to equalize LOD/STI stress

**Passive device generation:**
- Resistors: obey the 25 matching rules [Hastings, Ch. 8]. Same material, same width, sufficient area (random fraction <=75% minimal, <=50% moderate, <=25% exceptional), sufficient width (>=150% min linewidth minimal, >=200% moderate, >=400% exceptional), identical segment geometries, no excessively short segments (min length: 3x design-rule min for minimal, 5x moderate, 10x exceptional).
- Capacitors: obey the 13 matching rules [Hastings, Ch. 8]. Identical unit caps, square geometries (exceptional: always square), dummies on all four sides (exceptional: exact unit-cap copies), electrostatic shielding (all moderate and exceptional caps), cross-couple arrays for CC assignment.

**BiCMOS considerations [Hastings, Ch. 4]:**
- CDI NPN transistors: deep-N+ sinker must overlap NBL by several microns to reduce collector resistance and suppress hole permeation
- NBL dopant concentration must exceed N-well bottom doping by at least 50x
- Lateral PNP: NBL is essential (without it, apparent beta < 10)
- Substrate PNP: NBL must be omitted (it severely reduces beta)
- Base regions must be placed within LOCOS moats to prevent oxidation-enhanced diffusion variability

**ESD protection cell generation [Hastings, Ch. 14], [FOLD 7.4]:**
- Every pin except substrate ground needs primary ESD protection in a pad-based network
- GGNMOS: NMoat overlap of contact must be extended several microns beyond minimum for ballasting; silicide block mask required in silicided processes
- CDM secondary protection must be placed near the gate oxides it protects, not at the bondpad. Maximum distance from CDM clamp to protected gate oxide: 50 um. The cell generator must enforce this proximity constraint when generating secondary ESD cells.
- Electron-collecting guard rings required for almost all ESD devices
- ESD ground ring maximum resistance between any two chip points: 2 ohms [FOLD 7.4.1]. Supply lines should run at the chip periphery for direct access from all bond pads.

#### S0.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Cell generator must accept a matching tier parameter (minimal/moderate/exceptional) and scale dummy count, guard ring width, device sizing, and CC pattern accordingly | S0 | Different tiers have quantitatively different requirements [Hastings, Ch. 8, Ch. 13] |
| All devices within a matching group must have orientation chi = 0 (equal left/right fingers) or identical chi | S0 | Orientation mismatch is the #1 systematic mismatch source (~15% gm error) |
| SA and SB (gate-to-OD-edge distances) must be identical for all active devices within a matched array | S0 | LOD/STI mismatch up to 13% Id without equalization [PNR-Physics] |
| No metal routing over active gates of matched transistors [Hastings, Ch. 13, Rule 17] | S0 intra-cell router | Hydrogenation blocking causes up to 20% Id mismatch |
| Guard rings must fully encircle the injector, not just two or three sides | S0 | Incomplete guard rings provide negligible latchup protection [Hastings, Ch. 14] |

### S1 — Constraint Extraction

**Purpose:** Translate electrical intent into geometric constraints.

**Key sub-blocks:**
- Hierarchy extractor (GCN-based topology recognition + pattern library)
- GNN symmetry extractor (unsupervised, heterogeneous multigraph, cosine similarity, F1 > 0.95)
- LDE sensitivity analyzer (WPE distance bounds, LOD/STI mismatch limits)
- Net classifier (differential, clock, PG, sensitive, general)
- Electrical constraint extractor (per-net current for EM, per-device power for thermal, max parasitic R)
- CC/interdigitated/clustered selector (sensitivity-driven pattern choice)

**What S1 produces:** A constraint file containing:
- Symmetry groups with axes (device-level and system-level)
- Matching pairs with matching type (mirror, cross, self, partial)
- LDE bounds (min distance from well edge, max SA/SB mismatch)
- Net classification tags
- Wire width / via cut requirements per net
- Thermal power tags per device
- Pattern selection recommendations for S0 (feedback)

#### S1.A Analog Considerations

**Matching tier inference.** When explicit matching tier annotations are not provided, S1 must infer them from circuit topology using the following heuristic (derived from [Hastings, Ch. 8, Ch. 13] and [Lienig, Section 6.6.6]):

| Circuit Structure | Inferred Tier | Justification |
|---|---|---|
| Differential pair gates | Moderate | Offset is a primary spec; 1-3 mV Vth mismatch target |
| Current mirror gates | Moderate | Current ratio accuracy directly affects bias and gain |
| DAC unit elements (caps or current sources) | Exceptional | INL/DNL require <0.1% matching |
| Bandgap reference matched transistors | Exceptional | Vref accuracy requires <0.3 mV Vth mismatch |
| Bias network resistor ratios | Minimal to Moderate | Depends on sensitivity; startup resistors can be minimal |
| Common-mode feedback devices | Moderate | CMFB accuracy affects output range |

**Net classification beyond the five basic types.** The original spec classifies nets as differential, clock, PG, sensitive, general. Analog practice requires additional distinctions [Hastings, Ch. 15, Section 15.4.4]:

| Net Class | Routing Implications |
|---|---|
| High-impedance analog (cascode drains, opamp virtual ground) | Shortest possible routing on lowest-C metal; shield from digital; parasitic C budget typically < 0.16 fF for 100 MHz BW at 10 kOhm [00_ANALOG_PRINCIPLES, Section 3.2] |
| Matched-pair nets (diff pair gates, mirror gates) | Identical metal layers, identical via stacks, matched R and C to within ~1%; use jogs/stubs to equalize parasitic C [Hastings, Ch. 8, Rule 10] |
| Current bias lines | Low resistance to preserve bias accuracy; shield from switching noise |
| Feedback/compensation nodes | Extra C shifts pole/zero locations; can destabilize feedback loop |
| Noisy digital | Never adjacent to sensitive analog; series resistors to reduce slew rate; noise coupling: `dV = C * R * dV/dt` [Hastings, Ch. 15, Section 15.4.4] |

**Parasitic budget computation.** For each sensitive net, S1 should compute a parasitic capacitance budget:
```
C_budget = 1 / (2 * pi * f_target * R_node)
```
where `f_target` is the circuit's bandwidth target and `R_node` is the impedance at that node (from .op data). This budget is passed to S3 as a hard routing constraint.

#### S1.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Every matched pair must have a matching tier tag (minimal/moderate/exceptional) | S1 output | S0 and S2 need this to select pattern type and constraint strength |
| Every net must have a classification tag from the extended set (not just the 5 basic types) | S1 output | S3 routing strategy depends on net type |
| Each sensitive net must have a parasitic C budget (in fF) | S1 output | S3 must know when to stop adding routing length |
| Each device must have a thermal power tag (in mW) from .op data | S1 output | S2 placement uses thermal gradient model |
| Matching groups must include the current flow direction for each device (for Seebeck cancellation) | S1 output | Even number of segments with antiparallel current flow cancels thermoelectric EMF [Hastings, Ch. 8, Rule 11] |

### S2 — Placement

**Purpose:** Position all cells to minimize area and wirelength while satisfying matching, LDE, thermal, and performance constraints.

**Global placement objective:**

min W(v) + lambda*N(v) + tau*Sym(v) + eta*Area(v) + delta*LDE(v) + theta*Thermal(v) + alpha*Phi(G)

- W(v): Weighted-average smoothed HPWL
- N(v): Electrostatic potential energy (overlap)
- Sym(v): Soft quadratic symmetry penalty
- Area(v): WA bounding-box area
- LDE(v): WPE + LOD/STI mismatch for matched pairs
- Thermal(v): Thermal gradient across matched pairs
- Phi(G): GNN performance predictor (gradient via autodiff)

**Detailed placement (ILP):**
- Single-stage integrated area + HPWL minimization
- Hard symmetry, alignment, ordering, abutment constraints
- Device flipping (binary variables)
- Aspect ratio bounds
- Guard ring space reservation
- Substrate isolation spacing

**Solver selection by problem size:**
- < 8 cells: Enumerative (exhaustive sequence-pair search)
- 8-20 cells: ILP
- 20-100 cells: Analytical (ePlace-A)
- > 100 cells: SA with analytical warm-start

#### S2.A Analog Considerations

**Matched pair placement is the primary objective, not area.** The GP weight hierarchy must enforce: `delta (LDE) >> eta (Area)` and `tau (Sym) >> eta (Area)`. Quantitatively, if LDE mismatch for a moderate-matched pair exceeds 1 mV dVth, the LDE penalty must dominate the area term regardless of the area increase. The following weight ordering is recommended:

```
delta (LDE) >= 100 * eta (Area)
tau (Sym)   >= 50  * eta (Area)
theta (Thermal) >= 10  * eta (Area)
```

**Mechanical stress placement.** Matched devices should be placed near the die center where stress gradients are lowest [Hastings, Ch. 8, Ch. 13]. Exceptional matching requires placement within the central half of the die. The ILP should include a constraint:
```
distance(matched_pair_centroid, die_center) <= die_diagonal / 4  (for exceptional)
distance(matched_pair_centroid, die_center) <= die_diagonal / 2  (for moderate)
```

**Thermal placement.** Power devices must be placed far from matched pairs. Exceptional matching: matched transistors at the opposite end of the die from power devices, ~75% from center to far edge [Hastings, Ch. 13, Rule 14]. The thermal model:
```
T(x,y) = T_ambient + sum_d P_d * G(x-x_d, y-y_d)
G(dx,dy) = 1/(2*pi*k_th*t_sub) * ln(R_max / sqrt(dx^2 + dy^2))
```
should be evaluated for every matched pair at every GP iteration. If `|T(device_i) - T(device_j)| > 0.5 C` for a moderate pair, the thermal penalty must increase sharply (dVth/dT ~ -1 to -2 mV/C means 0.5 C produces ~1 mV offset).

**Floorplanning integration [Hastings, Ch. 15, Section 15.2].** The placement engine should respect floorplanning principles:
- Mirror-image placements for identical blocks (halves layout effort, ensures matched characteristics)
- Pin proximity: each block near its bondpads
- Shared blocks (bias generators) in the center
- Reserve explicit routing channels (typically ~20% of core area)
- Elongated cells are acceptable and can separate heat sources from sensitive inputs

#### S2.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Matched pairs must be placed at equal distance from all well edges (WPE equalization) | S2 | 5-25% current mismatch within 5 um of well edge [Hastings, Ch. 13] |
| Matched pairs must be placed along isotherms (equal temperature) | S2 | dVth/dT ~ -1 to -2 mV/C [00_ANALOG_PRINCIPLES, Section 4.3] |
| Matched pairs with exceptional tier must be placed within the central half of the die | S2 | Mechanical stress gradients are steepest at edges/corners [Hastings, Ch. 8] |
| Power devices must be placed >= 1 um/mW from exceptional matched devices [Hastings, Ch. 8, Rule 13] | S2 | Thermal gradient directly creates Vth mismatch |
| Device flipping within a matching group must be linked (all flip or none flip) | S2 ILP | Flipping one device but not its pair destroys orientation matching |

### S3 — Routing

**Purpose:** Connect all nets while preserving matching, minimizing parasitics, and achieving human-layout-quality performance.

**Key sub-blocks:**
1. VAE routing guide generation (per net type: differential, clock, PG)
2. Symmetry constraint allocation (Edmonds' blossom matching)
3. Pin access assignment (preferred direction sets)
4. Pin clustering (max fully-symmetric parts)
5. Constraint-aware A* routing (with priority queue and guidance costs)
6. Parasitic matching verification
7. Shielding insertion (grounded adjacent tracks for sensitive nets)
8. EM/IR verification and correction
9. Negotiation-based rip-up and reroute

**Cost function for A*:**

Cost = Cost_wire + Cost_via + Cost_history + Cost_violate + Cost_compete

- Cost_violate: Penalty for routing outside VAE-predicted region
- Cost_compete: Penalty for routing in regions demanded by other nets

**Symmetry variants:** Mirror, cross, self, partial -- selected per net pair by the symmetry allocator based on pin geometry.

#### S3.A Analog Considerations

**Matched net routing is a hard constraint, not a soft penalty.** For matched nets, the router must enforce [Hastings, Ch. 8; 00_ANALOG_PRINCIPLES, Section 3.4]:
- Same metal layer(s) for both nets
- Identical via stacks (same number of cuts, same via type)
- Total wire length matched to within ~5% (moderate) or ~1% (exceptional)
- Parasitic C matched to within ~1% of total intentional capacitance
- Use jogs or dead-end branches to equalize parasitic capacitance [Hastings, Ch. 8, Rule 10]

**Shielding requirements [Hastings, Ch. 15; 00_ANALOG_PRINCIPLES, Section 3.4]:**
- Never route clock or digital switching signals adjacent to high-impedance analog nodes
- Use ground metal interposed between noisy and sensitive signals on the same layer
- Electrostatic shields (metal tied to quiet analog ground) between noisy and sensitive signals on adjacent layers; shield overhang >= 2-5 um beyond intersection area [Hastings, Ch. 15, Section 15.4.4]
- Shield high-impedance matched nets with ground/supply metal on adjacent layers

**No metal over matched gates [Hastings, Ch. 13, Rule 17].** The router must maintain a keep-out zone over active gate regions of matched transistors. Moderate/exceptional: no leads crossing active areas. Exceptional: block dummy metal generation in a zone extending 5 um beyond active gate in all directions.

**Star nodes and Kelvin connections [Hastings, Ch. 15, Section 15.4.4]:**
- All sensitive leads return to a single common point (star node)
- Precision resistors use Kelvin connections (separate sense and force leads)
- Match the currents flowing through the two sense leads and route them with equal resistance

**Antenna rule fixes must not break symmetry [Lienig, Section 5.4.5].** When inserting jumpers or protection diodes to fix antenna violations, the fix must be applied to both nets of a matched pair, even if only one net violates. Otherwise the fix introduces parasitic asymmetry.

**Via reliability [Hastings, Ch. 15, Section 15.3].** Many fabs demand pairs of vias rather than single vias to protect against stress-induced voiding. Multiple vias in close proximity have a protective effect beyond mere redundancy. The router should use multi-cut vias by default, not just when EM requires it.

#### S3.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Matched nets: same layers, same via stacks, matched R and C to within tolerance specified by matching tier | S3 | Asymmetric parasitics create offset |
| No routing over active gates of matched transistors | S3 | Hydrogenation blocking: up to 20% Id mismatch [Hastings, Ch. 13] |
| Sensitive nets: parasitic C must not exceed budget from S1 | S3 | BW ~ 1/(2*pi*R_out*C_parasitic) |
| Antenna fixes must be applied symmetrically to matched net pairs | S3 | Asymmetric fixes introduce parasitic mismatch |
| Multi-cut vias by default on all signal paths (not just power) | S3 | Stress-induced voiding risk [Hastings, Ch. 15] |
| Noise coupling check: no noisy signal adjacent to, above, or below a sensitive signal | S3 | dV = C * R * dV/dt can exceed 1 V with 10 fF coupling into 100 kOhm at 10^10 V/s slew [Hastings, Ch. 15] |

### S4 — Sign-off & Verification

**Purpose:** Verify the layout against all manufacturing and performance requirements.

**Sub-blocks:**
1. DRC (full design-rule deck, Calibre nmDRC or equivalent)
2. LVS (graph-isomorphism equivalence, Calibre nmLVS)
3. Antenna check (metal-area-to-gate-area ratio per layer per net)
4. Density fill (dummy metal insertion with sensitive-net keep-out)
5. PEX (R+C+CC extraction, Calibre PEX or StarRC)
6. Post-layout SPICE (Cadence Spectre on extracted netlist)
7. Performance comparator (each metric vs. spec, pass/fail, margin)

**Decision logic:**
- All DRC clean + LVS pass + all specs met -> GDSII output
- DRC violations -> inner loop 2 (S4->S3, local routing fixes)
- Antenna violations -> insert protection diodes or metal jumpers
- Performance close miss -> tighten constraints at S1, re-run S2->S3->S4
- Performance large miss -> re-examine placement with adjusted GP weights
- Performance fundamental miss -> exit to circuit re-design (re-sizing)

#### S4.A Analog Considerations

**Density fill must respect matching [Hastings, Ch. 13, Rules 17-18].** Dummy metal fill generation must:
- Block dummy metal over active gates of matched transistors (use pseudolayers)
- Exceptional: block extends 5 um beyond active gate in all directions
- Match the metal fill pattern surrounding each matched transistor
- Asymmetric dummy metal fill creates offset in production silicon (up to 1% mismatch from different fill patterns [Hastings, Ch. 13])

**LVS must be topological zero-error [Hastings, Ch. 15, Section 15.5].** Zero topological LVS errors is non-negotiable. The presence of one error can confuse the LVS algorithm and mask others. Parametric LVS errors (e.g., guard ring area mismatches at top level vs. subcell level) may be reviewed and waived.

**The 17-point final layout checklist [Hastings, Ch. 15, Section 15.5]** should be automated where possible:
1. DRC diagnostics reviewed and waived
2. LVS topological errors = ZERO
3. LVS parametric errors reviewed
4. Electromigration compliance
5. High-current lead resistances verified
6. Antenna violations corrected
7. ESD compliance (primary + CDM clamps near gate oxides)
8. Latchup checks (guard rings on vulnerable diffusions)
9. Dummy metal block zones for matched devices
10. Noisy/sensitive signal separation
11. Bondpads and probe pads properly sized and located
12. Kelvin connections and star nodes properly located
13. Corner exclusion zones and metal slotting
14. Scribe seals and scribe streets present and correct
15. Substrate contacts filling empty die areas
16. Die symbolization (part number, logo, mask work notice)
17. Layout properly archived

#### S4.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Density fill must respect matched-device keep-out zones (pseudolayer-controlled) | S4 fill engine | Hydrogenation blocking from asymmetric fill: up to 1% mismatch |
| Post-layout simulation must include Monte Carlo mismatch analysis (Phase 3+), not just nominal simulation | S4 SPICE | Nominal simulation does not capture the offset, CMRR, PSRR that mismatch causes |
| The outer loop must distinguish matching-related spec failures from general performance failures | S4 decision logic | Matching failures route to S0 (cell re-generation); performance failures route to S2 (placement re-optimization) |
| The checklist items 7-10 (ESD, latchup, dummy metal, noisy/sensitive separation) must be automated checks, not manual | S4 | These are the most commonly missed items in analog layout review |

---

## 7. Feedback Loop Specifications

### Inner Loop 1: Routability (S3->S2)

**Trigger:** After initial placement (S2), before committing to full routing.
**Mechanism:** Estimate per-region routing congestion from HPWL-based demand vs. track capacity, or run GeniusRoute VAE to predict routing density. If congestion exceeds threshold in any region, add congestion penalty to GP objective and re-run S2.
**Iteration limit:** 2-3 passes. If congestion persists, accept and let S3's rip-up handle it.

### Inner Loop 2: DRC/Antenna Fixes (S4->S3)

**Trigger:** S4 DRC or antenna check finds violations.
**Mechanism:** Feed specific violations back as routing blockages or constraints. Re-route locally (rip-up only the offending nets). For antenna violations, insert protection diodes or route jumpers to higher metals.
**Iteration limit:** 5 passes. If violations persist, flag for manual review.

### Outer Loop: Performance Re-optimization (S4->S1)

**Trigger:** S4 SPICE simulation shows spec failure.
**Mechanism:** Compute performance gap per metric. If gap < 10% of spec -> tighten parasitic-matching tolerances and routing constraints at S1, re-run S2->S4. If gap 10-30% -> adjust GP weights (increase alpha for performance term), re-run S2->S4. If gap > 30% -> likely a sizing/topology issue; exit to circuit design.
**Fast proxy:** Use GNN performance predictor Phi(G) to pre-screen placement candidates before committing to full routing + PEX + SPICE.
**Iteration limit:** 3 outer-loop iterations. If specs not met, report best-achieved Pareto point and flag for designer intervention.

### 7.A Analog Considerations

**The outer loop must classify failures.** A matching failure (offset > spec, CMRR < spec, INL/DNL > spec) has a fundamentally different root cause than a performance failure (bandwidth < spec, gain < spec). The remediation paths are:

| Failure Type | Root Cause | Remediation Path |
|---|---|---|
| Offset > spec | Matched pair mismatch (LDE, parasitic asymmetry, orientation, thermal) | S0: increase device area, switch to 2D CC, add dummies. S2: equalize WPE distances. S3: re-route matched nets. |
| CMRR < spec | Tail current source mismatch or layout asymmetry | Same as offset |
| INL/DNL > spec | Unit element mismatch in DAC array | S0: increase unit element area, improve CC pattern, add more dummies |
| Bandwidth < spec | Parasitic C on high-impedance node | S3: re-route on lower-C metal, shorten routing. S2: place to reduce routing length. |
| Gain < spec | Usually sizing, not layout (gain = gm * Rout) | Exit to circuit design unless parasitic loading is the cause |
| Phase margin < spec | Extra parasitic pole | S3: reduce parasitic C on compensation node. S2: place to minimize routing on feedback path. |

The outer loop should tag each spec failure with its type and route it to the appropriate remediation stage, not blindly re-run the entire pipeline.

### 7.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Outer loop must classify failures as matching-type or performance-type | Flow controller | Different failure types require different remediation stages |
| Matching failures trigger S0 re-generation before S2 re-placement | Flow controller | Matching is built at S0, not patched at S2 |
| Performance failures with parasitic root cause trigger S3 re-routing before S2 re-placement | Flow controller | Re-routing is cheaper than re-placement |
| Antenna fixes applied in inner loop 2 must be symmetric for matched net pairs | S3/S4 interface | Asymmetric antenna fixes introduce parasitic mismatch |

---

## 8. Technology Portability

The engine separates technology-dependent from technology-independent components:

**Technology-dependent (must be configured per PDK):**
- Design rules (layers.json / tech LEF)
- Device generators (transistor, capacitor, resistor PCells)
- LDE model parameters (WPE scatter coefficients, STI stress tables, LOD model)
- EM current-density limits and Blech-length parameters
- Antenna rule ratios
- Density fill rules
- Guard ring and well-tap spacing rules

**Technology-independent (portable across PDKs):**
- Constraint extraction GNN model (retrain on new process, architecture is fixed)
- Placement NLP/ILP solver
- Routing A* engine and symmetry allocator
- VAE routing guide model (retrain on new layout data)
- GNN performance predictor (retrain on new simulation data)
- Sign-off flow orchestration

**Node-specific considerations:**
- Planar CMOS (>=65 nm): Classic multi-finger, continuous W tuning, moderate LDEs.
- FinFET (16-7 nm): Quantized W (integer fins), grid-based placement (LAYGO-style), strict orientation rules, amplified LDEs, routing-parasitic-dominated performance.
- GAA/Nanosheet (<=5 nm): Quantized W (discrete sheets), even stricter layout rules, self-heating first-order, likely template-only cell generation.

### 8.A Analog Considerations

**Compaction-based layout retargeting for process migration [ALS Ch. 5].** For process migration of existing layouts, compaction-based retargeting offers an alternative to full re-synthesis: the source layout's placement and routing are preserved while adapting to new design rules and device dimensions. The multivariable constraint-graph simplex method (ALS Section 5.5) supports symmetry and hierarchy constraints natively, making it well-suited for analog layouts where matching relationships must be preserved during migration. Retargeting proceeds by: (1) constructing a constraint graph from the source layout's geometric relationships, (2) adding new design-rule constraints from the target process, (3) solving the constrained optimization to produce a compacted layout that satisfies all target rules while minimizing perturbation from the source. This approach is complementary to the full PNR pipeline and may be faster for incremental process migrations (e.g., same foundry, adjacent node) where the circuit topology is unchanged and only geometric rules differ.

**No universal scaling laws exist for analog circuits [Hastings, Ch. 3, Section 3.2.4].** Unlike digital layouts, analog layouts cannot be ported between process nodes simply by changing lambda. Extensive circuit redesign and complete relayout are typically required. The engine must not assume that a cell designed for one node can be geometrically shrunk for another.

**BiCMOS processes add significant complexity [Hastings, Ch. 4, Section 4.3].** Modern analog BiCMOS processes have 25-30 mask steps fabricating more than 50 different circuit components. The device generators must handle:
- CDI NPN transistors (N-well as collector, deep-N+ sinker, NBL)
- Lateral PNP transistors (base diffusion emitter/collector in N-well)
- Substrate PNP transistors (NBL must be omitted)
- LDMOS power transistors (bird's beak positioning, PSD plugs at ends)
- Three types of poly resistors (LSR ~5 Ohm/sq, MSR ~200-400 Ohm/sq, HSR ~1-2 kOhm/sq)
- Gate oxide capacitors, poly-poly capacitors

**Dielectric isolation (DI) processes [Hastings, Ch. 4, Section 4.3]** using wafer bonding + deep trench isolation provide superior minority carrier injection protection and better substrate noise isolation than junction isolation. The engine should recognize DI process features and relax guard ring requirements where DTI provides equivalent isolation.

### 8.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| PDK configuration must include matching coefficients (A_VT, A_beta) per device type | PDK abstraction layer | Pelgrom model requires process-specific coefficients to size devices for matching |
| PDK configuration must include LDE parameters (WPE a, lambda; LOD K1, K2) per device type | PDK abstraction layer | LDE penalty in GP requires these coefficients |
| PDK configuration must specify available guard ring types and their efficiency | PDK abstraction layer | Guard ring selection depends on process (BiCMOS w/ NBL: ECGR >90%; std bipolar: ECGR marginal) |
| Scalable (lambda) design rules must NOT be used for analog cell generation | S0 | Analog circuit performance does not scale uniformly [Hastings, Ch. 3] |

---

## 9. Quality Metrics and Targets

| Metric | Target | Validation |
|--------|--------|------------|
| DRC violations | 0 | Calibre nmDRC |
| LVS errors | 0 | Calibre nmLVS |
| Post-layout performance vs. spec | All metrics within spec | PEX + SPICE |
| Post-layout vs. schematic degradation | < 10% on primary metrics | SPICE comparison |
| Symmetry degree (d_SYM) | > 0.7 for matched nets | Wirelength-weighted symmetry ratio |
| Runtime (block-level, < 50 devices) | < 5 minutes | Wall clock |
| Runtime (system-level, < 500 devices) | < 60 minutes | Wall clock |
| Constraint extraction F1-score | > 0.95 (system), > 0.80 (device) | vs. designer ground truth |

### 9.A Analog Considerations

**Additional quality metrics specific to analog:**

| Metric | Target | Validation | Source |
|---|---|---|---|
| Matched pair Vth mismatch (estimated) | < matching tier spec (minimal: 5-15 mV, moderate: 1-3 mV, exceptional: <0.3 mV) | Pelgrom + LDE model evaluation on final placement | [Hastings, Ch. 13] |
| Matched pair current mismatch (estimated) | < matching tier spec (minimal: 2-5%, moderate: 0.5-1%, exceptional: <0.1%) | From Vth mismatch and overdrive | [Hastings, Ch. 13] |
| Matched net parasitic asymmetry | < 1% of total C for moderate, < 0.1% for exceptional | PEX comparison of matched net pairs | [Hastings, Ch. 8] |
| Guard ring coverage | 100% of identified injectors enclosed | Geometric check | [Hastings, Ch. 14] |
| IR drop (analog supply) | < 5% of Vsupply at any device pin | Resistive network solve | [00_ANALOG_PRINCIPLES, Section 3.3] |
| IR drop (digital supply) | < 10% of Vsupply | Resistive network solve | [00_ANALOG_PRINCIPLES, Section 3.3] |
| Dummy metal keep-out compliance | 100% of matched devices have correct keep-out | Geometric check | [Hastings, Ch. 13] |
| Monte Carlo yield (Phase 3+) | > 99.7% (3-sigma) on all specs | 1000-point MC simulation | Industry standard |
| Orientation uniformity | chi = 0 or identical for all matched groups | Metadata check | [Hastings, Ch. 13] |

### 9.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Matching quality metrics must be reported per matched pair, not averaged | S4 report generator | One poorly matched pair can sink the entire circuit |
| The worst-case matched pair must be highlighted in the report | S4 report generator | This pair limits yield and should be the focus of re-optimization |
| Runtime targets are secondary to quality targets | Flow controller | A 60-minute run that meets spec is preferable to a 5-minute run that does not |

---

## 10. Key References

**Academic systems:**
- MAGICAL: Zhu, Chen, Liu, Pan et al. (UT Austin) -- IEEE TCAS-II 2022, ICCAD 2019/2020, DAC 2020/2021
- ALIGN: Sapatnekar, Harjani, Hu et al. (UMN/TAMU/Intel) -- ISPD 2023, DAC 2019
- ePlace-A/AP: Lin, Li, Fang, Sapatnekar, Hu (TAMU/UMN) -- DATE 2022
- GeniusRoute: Zhu, Liu, Lin, Pan et al. (UT Austin) -- ICCAD 2019
- BAG/BAG2: Crossley, Chang, Han, Alon, Nikolic (UC Berkeley) -- CICC 2018, TCAS-I 2021

**Textbooks:**
- Razavi, "Design of Analog CMOS Integrated Circuits" (2nd ed., 2017) -- Chapter 19
- Hastings, "The Art of Analog Layout" (3rd ed., 2006) -- Chapters 3, 4, 7, 8, 11, 12, 13, 14, 15
- Baker, "CMOS: Circuit Design, Layout, and Simulation" (4th ed.) -- Chapters 2-5, 28
- Sansen, "Analog Design Essentials" (Springer, 2006)
- Lienig, "Fundamentals of Layout Design for Electronic Circuits" -- Chapters 4, 5, 6, 7

**Foundational:**
- Pelgrom et al., "Matching Properties of MOS Transistors," IEEE JSSC 1989
- Ou et al., "Layout-Dependent-Effects-Aware Analytical Analog Placement," IEEE TCAD 2016
- Chen et al., "Universal Symmetry Constraint Extraction with GNNs," DAC 2021
- Chen et al., "Toward Silicon-Proven Detailed Routing for AMS Circuits," ICCAD 2020

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| GAP-06 | Added `PackageInfo` data structure (`die_width_um`, `die_height_um`, `package_type`, `mold_cte_ppm_per_C`, `die_attach_type`) to Section 4.A input specification | No stage defined the data structure for package/die stress model input; S2 needs this for mechanical stress placement constraints |
| COV-03 | Added CDM proximity constraint (max 50 um) and ESD ground ring max resistance (2 ohms) to S0.A ESD protection cell generation section | ESD ground ring resistance budget and CDM placement proximity were missing from the detailed ESD cell generation rules |
| MISSING-10 | Added compaction-based layout retargeting paragraph to Section 8.A | Process migration via retargeting (ALS Ch. 5) was not mentioned as a complementary strategy to full PNR re-synthesis |
