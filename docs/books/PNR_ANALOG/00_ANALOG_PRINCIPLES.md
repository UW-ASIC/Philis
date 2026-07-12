# Analog Cross-Cutting Principles

This document is the shared reference for all downstream agents in the PNR analog layout pipeline. Every rule is cited to its source. Equations are given in operational form -- the variables are those the layout engineer controls.

Sources referenced throughout:
- **[Hastings]** -- Alan Hastings, *The Art of Analog Layout*, 3rd Ed.
- **[Lienig]** -- Jens Lienig, *Fundamentals of Layout Design for Electronic Circuits* (FOLD), Chapter 6-7.
- **[PNR-Physics]** -- `06_PHYSICS_LDE_REFERENCE.md` (project-internal physics reference).
- **[PNR-Checklist]** -- `08_OPTIMAL_LAYOUT_CHECKLIST.md` (project-internal concern checklist).
- **[Pelgrom]** -- Pelgrom, Duinmaijer, Welbers, IEEE JSSC 1989.

---

## 1. Matching Hierarchy

Matching is the single most important layout concern in analog IC design. This section ranks every mismatch mechanism by impact magnitude and gives the quantitative model for each.

### 1.1 The Pelgrom Model

The foundational statistical model for random mismatch between two identically-drawn devices [Pelgrom]:

```
sigma^2(dVth) = A_VT^2 / (W * L)  +  S_VT^2 * D^2

sigma^2(d_beta/beta) = A_beta^2 / (W * L)
```

Where:
- `A_VT` = threshold voltage matching coefficient (mV*um). Process-specific.
- `A_beta` = current-factor matching coefficient (%*um). Process-specific.
- `W, L` = gate width and length (um).
- `S_VT` = gradient sensitivity coefficient (mV/um).
- `D` = centroid-to-centroid distance between matched devices (um).

**First term (random):** Reduced by increasing device area W*L.
**Second term (systematic):** Reduced by minimizing separation D and using common-centroid layout.

Representative A_VT values [PNR-Physics]:

| Node / Oxide | A_VT (NMOS) | A_VT (PMOS) | Source |
|---|---|---|---|
| Original Pelgrom (50 nm SiO2) | 30 mV*um | 35 mV*um | Pelgrom JSSC 1989 |
| ~130 nm (2.5 nm tox) | ~5 mV*um | ~6 mV*um | Industry rule-of-thumb |
| ~28 nm HKMG | ~3.9 mV*um | -- | Published data |
| Rule-of-thumb | ~1 mV*um per nm tox | -- | Community convention |

**Drain current mismatch** (the quantity that actually matters for mirrors):

```
sigma^2(dId)/Id^2 = 4 * sigma^2(dVth) / (Vgs - Vth)^2  +  sigma^2(d_beta/beta)
```

Design implication: run matched pairs at high overdrive (Vgs - Vth) to suppress the Vth mismatch term [PNR-Physics].

### 1.2 Pocket-Implant Devices (Modified Pelgrom)

Transistors with halo/pocket implants obey a modified scaling law [Hastings, Ch. 13]:

```
sigma(dVth) = A_VT / sqrt(W * min(L, L_C))
```

where L_C is a critical channel length, typically 1-2 um. Beyond L_C, further lengthening does NOT improve matching. Pocket-implant devices are poorly suited for current matching. Use analog-friendly (non-pocket) device options when available.

### 1.3 Systematic vs. Random Mismatch

Random mismatch is reduced by area. Systematic mismatch is reduced by layout technique. The following table ranks systematic mismatch sources by typical impact magnitude (worst to least):

| Rank | Mismatch Source | Typical Magnitude | Layout Knob | Source |
|---|---|---|---|---|
| 1 | Orientation mismatch | ~15% gm error (several % Id) | All matched devices same orientation | [Hastings Ch.13], [PNR-Checklist 2.12] |
| 2 | WPE (well proximity effect) | 10-50 mV Vth shift; 5-25% Id mismatch within 5 um of well edge | Equal distance from well edges; guard rings to push well edges away | [PNR-Physics], [Hastings Ch.13] |
| 3 | LOD/STI stress | Up to 13% NMOS Id reduction; 100 mV Vth at 45nm | Equal SA/SB via identical dummy context | [PNR-Physics], [Hastings Ch.13] |
| 4 | Thermal gradient | 1-2 mV/C systematic Vth offset | Place along isotherms; common-centroid; distance from power devices | [PNR-Physics], [Lienig 6.6.4] |
| 5 | Mechanical stress gradient | Several % gm via piezoresistivity | Place near die center; common-centroid; identical orientation | [Hastings Ch.8, Ch.13] |
| 6 | Hydrogenation blocking by metal | Up to 20% Id mismatch (metal over gate) | No metal over matched gates; block dummy metal generation | [Hastings Ch.13] |
| 7 | Process gradients (oxide thickness, doping) | ~0.5 ppm/um oxide gradient; accumulates with D | Common-centroid; minimize D | [Hastings Ch.13], [Lienig 6.6.2] |
| 8 | Etch-rate variation (microloading) | ~1% for end devices without dummies | Dummy devices at array edges | [Hastings Ch.13], [Lienig 6.6.3] |
| 9 | Thermoelectric (Seebeck) | 0.1-1 mV/K at metal-silicon junctions | Even number of segments, antiparallel current flow | [Hastings Ch.8], [Lienig 6.6.5] |

### 1.4 Hastings' Matching Tiers

Hastings defines three tiers of matching accuracy [Hastings, Ch. 8 and Ch. 13]:

**Resistors/Capacitors:**
| Tier | Accuracy (6-sigma) | Area Implication |
|---|---|---|
| Minimal | +/-0.1% to +/-1% | Standard sizing |
| Moderate | +/-0.01% to +/-0.1% | Careful layout of fairly large devices |
| Exceptional | +/-0.001% to +/-0.01% | Very large area; likely requires trimming |

**MOS Transistors:**
| Tier | Voltage Mismatch (6-sigma) | Current Mismatch (6-sigma) |
|---|---|---|
| Minimal | 5-15 mV | 2-5% |
| Moderate | 1-3 mV | 0.5-1% |
| Exceptional | <0.3 mV | <0.1% |

### 1.5 The Five Rules of Common-Centroid Layout

From [Hastings, Ch. 13, Table 13.2] and [Lienig, 6.6.2]:

| Rule | Requirement | What It Cancels |
|---|---|---|
| 1. COINCIDENCE | Centroids of matched devices coincide | Linear gradients (any direction) |
| 2. SYMMETRY | Array is symmetric about both H and V axes | Ensures coincidence is achievable |
| 3. DISPERSION | Large arrays subdivide into smaller CC subarrays | Quadratic gradient residues |
| 4. COMPACTNESS | Each array/subarray is as compact as possible | Higher-order gradient residues |
| 5. ORIENTATION | Matched transistors have equal orientation chi | Stress/mobility asymmetry |

**Orientation metric** [Hastings, Ch. 13]: For multi-finger transistors, orientation chi = (1/N) * sum(chi_i), where chi_i = +1 if current flows right, -1 if left. Matched devices must have equal chi. Zero orientation (equal left/right fingers) is preferred.

**2D vs 1D:** A 2D cross-coupled AB/BA array exhibits ~60% of the residual gradient-induced mismatch of a 1D ABBA array [Hastings, Ch. 13]. Always prefer 2D when device geometry allows (W ~ L).

### 1.6 Hastings' 25 Resistor Matching Rules (Condensed)

Critical rules from [Hastings, Ch. 8, Section 8.3.1]:

1. **Same material.** Never match poly to diffused. Material ranking: thin-film (nichrome) > polysilicon > diffused.
2. **Same width.** Width biases cause systematic mismatch.
3. **Sufficient area.** Random fraction: <=75% (minimal), <=50% (moderate), <=25% (exceptional).
4. **Sufficient width.** >=150% of min linewidth (minimal), >=200% (moderate), >=400% (exceptional).
5. **Identical segment geometries.** Build from arrays of identical rectangular segments.
6. **Same orientation.** Even minimal matching requires identical orientation.
7. **Close proximity.** Minimal: <=few hundred um. Moderate: immediately adjacent. Exceptional: always common-centroid.
8. **Interdigitate.** Obey all five CC rules (symmetry, coincidence, dispersion, compactness, orientation).
9. **Dummies.** Full-width at each end (moderate); multiple dummies spanning >=10 um each end (exceptional).
10. **No excessively short segments.** Min length: 3x design-rule min (minimal), 5x (moderate), 10x (exceptional).
11. **Cancel thermoelectrics.** Even number of series segments; half in each current direction.
12. **Low stress-gradient region.** Interior of die. Avoid edges (>=200 um), corners. Exceptional: near die center.
13. **Away from power devices.** Exceptional: >=1 um/mW separation.
14. **Axes of symmetry.** Stress distributions are symmetric about die axes.
15. **No conductivity modulation.** Poly <=few hundred Ohm/sq is safe. Higher: field plate. Exceptional: thin-film.
16. **Serpentines only for minimal matching.** Moderate requires arrayed segments.

### 1.7 Hastings' 24 MOS Transistor Matching Rules (Condensed)

Critical rules from [Hastings, Ch. 13, Section 13.3]:

1. **Identical sections.** Never mix W/L. Voltage-match by paralleling; current-match by series stacking.
2. **Large devices for voltage matching.** Mismatch scales as 1/sqrt(WL).
3. **Long devices for current matching.** Increase L, not W. Required lengths can reach 240 um (exceptional, 12V NMOS at 10 uA).
4. **Avoid subthreshold operation of matched transistors.** Stringers cause dramatic mismatch. Maintain Veff >= 100 mV.
5. **Avoid pocket-implant devices for long-channel matching.**
6. **Prefer thin-oxide devices.** Vth mismatch scales linearly with tox.
7. **Same orientation.** Compute chi for multi-section devices.
8. **Close proximity.**
9. **Compact layout.** Aspect ratio: <=3:1 (voltage matching), ~1:1 (exceptional current matching).
10. **Use 2D common-centroid layouts.** Cross-coupled pairs: ~60% of 1D ABBA residual mismatch.
11. **Avoid submicron dimensions.**
12. **Place dummies.** Moderate: full dummy for L<1 um, half dummy for L>1 um. Exceptional: outermost dummy poly >= 3 um from nearest active gate; moat extends 5 um beyond last active gate.
13. **Low stress-gradient locations.** Central half of die.
14. **Distance from power devices.** Exceptional: opposite end of die, ~75% from center to far edge.
15. **Die axes of symmetry.**
16. **No contacts on active gate regions.**
17. **No metal routing over active gates.** Moderate/exceptional: no leads crossing active areas.
18. **Block dummy metal generation.** Exceptional: block extends 5 um beyond active gate in all directions.
19. **Deep diffusion spacing.** Well boundaries >= 5 um from exceptional matched transistors, or >= 2x well junction depth.
20. **Use metal straps for gate connections.** Required for moderate and exceptional.

### 1.8 Lienig's Matching Concept Tiers

[Lienig, Section 6.6.6] provides a three-tier classification:

**(a) Normal Matching (mandatory for any matched pair):**
- Same device type
- Same size and shape (splitting into identical basic elements)
- Minimum distance between matched devices
- Same orientation (resistors, transistors)

**(b) Higher Matching:**
- 1D or 2D interdigitation
- Placement along isotherms
- Dummy elements for uniform environment
- Consider current flow direction (Seebeck cancellation)
- Increase device dimensions

**(c) Highest Matching:**
- Common-centroid layout
- Placement in low-stress chip regions (die center)
- Symmetrical routing
- Well border distance > 1 um

### 1.9 Capacitor Matching (Hastings' 13 Rules, Condensed)

From [Hastings, Ch. 8, Section 8.3.2]:

1. **Identical unit capacitor geometries.** Use arrays of identical unit caps connected in parallel.
2. **Square geometries.** Minimize periphery-to-area ratio. Exceptional: always square.
3. **Sufficient area.** Poly-poly with A_C ~ 0.5%*um: +/-0.1% pair needs ~250 um^2; +/-0.01% needs ~25,000 um^2.
4. **Adjacent placement.** Compact row-column patterns.
5. **Same field oxide.** >=10 um from moat edges for moderate.
6. **Parasitic capacitance management.** Lower plate to low-impedance node.
7. **Dummies on all four sides.** Exact unit-cap copies for exceptional.
8. **Electrostatic shielding.** All moderate and exceptional caps must be shielded. Overhang >= 50 um if no dummies.
9. **Cross-couple arrays (CC assignment).** Eliminates dielectric thickness gradient mismatch.
10. **Match lead capacitance.** Use jogs/stubs to equalize.
11. **Thick homogeneous dielectrics.** Grown/LPCVD oxide preferred over TEOS or ONO.
12. **Low stress-gradient location.**
13. **Away from power devices.** Poly-electrode caps have TCC up to ~50 ppm/C.

Capacitor type ranking (best to worst matching potential) [Hastings, Ch. 8]:
1. Poly-metal with silicided lower electrode + thick LPCVD oxide
2. Poly-poly
3. MOS (accumulated/inverted)
4. Junction capacitors (unsuitable)

---

## 2. LDE Quick Reference

Layout-Dependent Effects (LDE) are deterministic Vth and Id shifts caused by the physical surroundings of a device. They are the dominant source of systematic mismatch in modern processes.

### 2.1 Well Proximity Effect (WPE)

**Physical mechanism:** During high-energy well implant, ions scatter off photoresist sidewalls at the well edge. Scattered ions land in nearby channels, raising |Vth| [PNR-Physics], [Hastings, Ch. 13].

**Equation:**

```
dVth_WPE = sum_k  a_k * exp(-d_k / lambda_k)
```

Where:
- `d_k` = distance from device channel center to k-th well edge (um)
- `a_k` = scattering amplitude coefficient (mV), PDK-specific
- `lambda_k` = scattering decay length (um), PDK-specific
- `k` indexes four well edges (left, right, top, bottom)

Typical values: lambda ~ 1-5 um, a ~ 10-50 mV [PNR-Physics].

**Mismatch for matched pair (i, j):**

```
dVth_mismatch_WPE = |dVth_WPE(i) - dVth_WPE(j)|
```

**Quantitative impact** [Hastings, Ch. 13]:
- At 0.5 um from well edge: 5% current mismatch
- At 0.25 um from well edge: 25% current mismatch
- Effects observed up to 5 um from well edge

**The knob (what the layout engineer controls):**
- Place both matched devices at EQUAL distance from well edges
- Use guard rings to push well edges far from active devices (>= 5 um for exceptional matching)
- Ensure well-mask symmetry around matched arrays
- Use wider transistors to reduce the fraction of channel in the enhanced-doping zone
- Add dummy gates whose moat regions displace the well edge [Hastings, Ch. 13, Fig 13.51]

**The failure mode (what breaks if ignored):**
- 10-50 mV Vth mismatch [PNR-Checklist, item 2.9]
- Bias point errors in current mirrors
- Offset in differential pairs that cannot be corrected by common-centroid layout alone (WPE is a device-local effect, not a gradient)

### 2.2 LOD / STI Stress

**Physical mechanism:** Shallow Trench Isolation exerts compressive stress on the silicon channel. Stress magnitude depends on SA (gate-to-source-side OD edge) and SB (gate-to-drain-side OD edge) [PNR-Physics].

- NMOS: compressive stress REDUCES electron mobility -> Id decreases
- PMOS: compressive stress ENHANCES hole mobility -> Id increases

**Equation:**

```
f(SA, SB) = 1 + K1*(1/SA + 1/SB) + K2*(1/SA^2 + 1/SB^2)

dId/Id = f(SA, SB) - f(SA_nom, SB_nom)
```

Where K1, K2 are PDK-specific stress correction coefficients [PNR-Physics].

**Quantitative impact:**
- Up to 13% NMOS drive current reduction [Scott et al., IEEE IEDM 1999, cited in PNR-Physics]
- At 45 nm: systematic variation can reach 30%+ in Id and 100 mV in Vth [PNR-Physics]
- 10%+ transconductance shift; 10 mV+ Vth shift [Hastings, Ch. 13]
- Effects diminish rapidly with distance from STI sidewalls but are significant within ~5 um

**The knob:**
- Ensure SA_i ~ SA_j and SB_i ~ SB_j by using identical dummy context
- Stretch moat regions at array ends by >= 3 um [Hastings, Ch. 13, Fig 13.52A]
- Use dummy transistors with wide moat regions [Hastings, Ch. 13, Fig 13.52B]
- For optimal matching, moat extends >= 5 um beyond last active transistor [Hastings, Ch. 13]
- Merge dummies into the same OD region as active devices

**The failure mode:**
- Drive current mismatch up to 13% (NMOS) [PNR-Checklist, item 2.10]
- Bias errors in current mirrors
- INL/DNL degradation in DACs
- End devices in arrays differ systematically from interior devices

### 2.3 Mechanical Stress (Packaging)

**Physical mechanism:** Epoxy mold compounds shrink more than silicon on cooling from cure temperature (~175C), generating compressive stress. Stress is most intense at die center, gradients are steepest at edges/corners [Hastings, Ch. 8].

**Equation (piezoresistivity):**

```
dR/R = pi_L * sigma_L + pi_T * sigma_T + pi_S * tau_S
```

Where pi_L, pi_T, pi_S are longitudinal, transverse, and shear piezoresistance coefficients (units: 10^-11 Pa^-1) [Hastings, Ch. 8].

**Piezoresistance coefficients for MOS on (100) silicon** [Hastings, Ch. 13]:

| Parameter | NMOS <110> | PMOS <110> |
|---|---|---|
| pi_L | 30 x 10^-11 Pa^-1 | -65 x 10^-11 Pa^-1 |
| pi_T | -17 x 10^-11 Pa^-1 | 40 x 10^-11 Pa^-1 |

PMOS is ~2x more stress-sensitive than NMOS.

**CTE mismatch data** [Hastings, Ch. 8]:

| Material | CTE (ppm/C) |
|---|---|
| Silicon | 2.6 |
| Copper alloys | 16-18 |
| Epoxy mold compounds | 8-35 |
| PCB | 12-18 |

**The knob:**
- Place matched devices near die center (lowest stress gradients)
- Use die axes of symmetry
- Avoid edges (>= 200 um for minimal, >= 500 um for moderate) and corners
- Common-centroid layout
- Use low-stress mold compounds (>= 90% filler, CTE ~ 8 ppm/C)
- For resistors: poly preferred over mono-Si (orientation-independent piezoresistivity)
- L-shaped segments (equal H + V lengths) can cancel piezoresistivity [Hastings, Ch. 8]

**The failure mode:**
- Several percent gm mismatch between identically drawn devices at different stress locations
- Up to 5% current mismatch on tilted wafers [Hastings, Ch. 13]
- Package shift: parameters drift after assembly

### 2.4 Hydrogenation Blocking

**Physical mechanism:** Metal layers block hydrogen diffusion during passivation anneal. Transistors beneath metal receive less hydrogen passivation and have more random Vth variation [Hastings, Ch. 13].

**Quantitative impact:**
- Up to 20% systematic Id mismatch between metal-covered and uncovered transistors
- Up to 1% mismatch from different metal fill patterns in vicinity
- Effects extend >= 10 um from metal edges

**The knob:**
- Do NOT place metal above active gate regions of matched transistors
- Block dummy metal generation over matched devices (use pseudolayers)
- Match the metal pattern surrounding each transistor
- If metal must cross: cover BOTH matched gates with identical metal field plate, then route higher metals

**The failure mode:**
- Hidden systematic mismatch that does not appear in schematic simulation
- Asymmetric dummy metal fill creates offset in production silicon

### 2.5 Conductivity Modulation and Dielectric Absorption

**Physical mechanism:** Electric fields from leads routed over resistors or from body/tank bias differences cause conductivity modulation in high-sheet resistors, producing up to 0.1%/V systematic mismatch [Hastings, Ch. 8, Section 8.2.9]. Three distinct mechanisms contribute:

**1. Body/tank modulation.** Different operating voltages on matched resistor segments require separate tanks, each biased to the positive end of its segment. If matched resistor segments operate at different potentials within a shared tank, the electric field from the tank-to-resistor junction modulates the resistor's sheet resistance. For proper matching, all matched segments must have identical body/tank bias conditions.

**2. Field plate (Faraday shield) design.** For resistors with Rsh > 1 kOhm/sq (high-sheet resistors), an electrostatic field plate is required to shield the resistor body from stray electric fields. The field plate is a grounded (or signal-connected) metal layer placed directly over the resistor. For exceptional matching of HSR resistors (< 1% target), split field plates are needed: the field plate is divided into sections, each connected to the corresponding resistor segment's potential, to prevent the field plate itself from introducing a lateral electric field gradient across the resistor.

**3. Charge spreading (mobile ion contamination).** Mobile ions in the passivation or interlayer dielectric drift under sustained bias, causing long-term resistance drift. This mechanism is slow (hours to days) and bias-dependent. Matched resistors operating at different DC voltages develop progressive mismatch over the product lifetime. Field plates mitigate this by shielding the resistor from external fields that drive ion migration.

**The knob:**
- For Rsh <= 200 Ohm/sq (poly LSR/MSR): conductivity modulation is negligible; no special measures needed.
- For Rsh > 200 Ohm/sq: ensure identical body/tank bias on all matched segments.
- For Rsh > 1 kOhm/sq (HSR): generate an electrostatic field plate connected to ground or to the resistor's midpoint.
- For exceptional matching of HSR: use split field plates, each section connected to the local resistor potential.
- For thin-film resistors (nichrome, sichrome): conductivity modulation is negligible due to metallic conduction.

**The failure mode:**
- Up to 0.1%/V systematic mismatch in high-sheet resistors without field plates
- Long-term drift under sustained bias (charge spreading) that is not captured by initial characterization
- Bandgap reference voltage drift over product lifetime if VPTAT resistors lack field plates

---

## 3. Parasitic Budget Rules

### 3.1 When Parasitics Matter

Not all nodes are equally sensitive. The following node types require parasitic-aware layout [PNR-Checklist]:

| Node Type | Why It Matters | Typical Sensitivity |
|---|---|---|
| High-impedance nodes (cascode drains, opamp outputs) | Parasitic C directly degrades bandwidth, phase margin | BW ~ 1/(2*pi*R_out*C_parasitic) |
| Matched nets (diff pair gates, mirror gates) | Asymmetric parasitics create offset | Must be matched to within ~1% of each other |
| Feedback paths (compensation nodes) | Extra C shifts pole/zero locations | Can destabilize feedback loop |
| Supply/ground rails | R causes IR drop; L causes ground bounce | Budget: < 5% of Vsupply [PNR-Physics] |
| Clock distribution | Parasitic R*C causes skew | Matched routing required |

### 3.2 Capacitance Thresholds

Rules of thumb for when parasitic capacitance becomes problematic:

**For bandwidth-limited nodes:**
```
C_parasitic < 1 / (2 * pi * f_target * R_node)

Example: For a 100 MHz bandwidth node with 10 kOhm output impedance:
  C_parasitic < 1 / (2 * pi * 100e6 * 10e3) = 0.16 fF
  This is extremely tight -- requires minimum-length routing on lowest-C metal layers.
```

**For matched nets:**
- Parasitic C mismatch < 1% of total intentional capacitance [Hastings, Ch. 8, Rule 10]
- For high-impedance matched nodes: absolute C mismatch < ~0.1 fF (process dependent)
- Lead-to-adjacent-metal spacing >= 2-3x interlayer oxide thickness [Hastings, Ch. 8]

**For supply decoupling:**
- Place decap within 1/(10 * f_switching) * v_propagation of the load

### 3.3 Resistance Thresholds

**Supply/ground IR drop** [PNR-Physics]:
```
IR_drop < 5% of V_supply (analog)
IR_drop < 10% of V_supply (digital)

For 1.0V analog supply: max IR drop = 50 mV at any device pin.
```

**Wire resistance model:**
```
R = R_sheet * length / width
R_via = R_contact / num_cuts
```

**Via resistance** [Hastings, Ch. 8, Rule 23]:
- Aluminum via resistances are highly variable
- Each metal layer and via contribution should scale proportionally to desired resistance ratios
- Use Kelvin connections (separate sense and force leads) for exceptional matching

### 3.4 Routing Implications

**Matched net routing requirements:**
- Route matched nets on the same metal layer(s)
- Use identical via stacks
- Match total wire length to within ~5% (moderate) or ~1% (exceptional)
- Use jogs or dead-end branches to equalize parasitic capacitance [Hastings, Ch. 8, Rule 10]
- Shield high-impedance matched nets with ground/supply metal on adjacent layers
- Maintain lead-to-adjacent-metal spacing >= 2-3x interlayer dielectric thickness

**Signal integrity** [PNR-Checklist, item 2.8]:
- Clock-to-signal coupling can cause 5x offset worsening (GeniusRoute measurement cited in PNR-Checklist)
- Never route clock or digital switching signals adjacent to high-impedance analog nodes
- Use shielding (ground metal interposed) for sensitive signals [PNR-Checklist, item 2.14]

**Parasitic capacitance management for capacitors** [Hastings, Ch. 8, Rule 6]:
- Connect lower plate of matched capacitors to a low-impedance node
- Place a well/NBL beneath capacitor arrays for substrate isolation
- Shield entire array with electrostatic shield (metal layer tied to analog ground)
- Shield overhang >= 50 um if no dummies; >= 5 um with dummies [Hastings, Ch. 8]

---

## 4. Thermal and Reliability Baseline

### 4.1 Electromigration (EM)

**Black's Equation** [PNR-Physics]:

```
MTTF = A / J^n * exp(Ea / (k * T))

Where:
  J = current density (mA/um width)
  n = current density exponent (~2 for Al, ~1 for Cu)
  Ea = activation energy (0.7 eV for Al grain boundary, 0.9 eV for Cu)
  A = empirical constant
  k = Boltzmann constant
  T = absolute temperature (K)
```

**DC and AC current density limits** [PNR-Physics]:

| Material | J_max (DC) | J_max (AC/RMS) |
|---|---|---|
| Aluminum | ~1 mA/um width | ~5 mA/um width |
| Copper | ~5 mA/um width | ~25 mA/um width |
| Via (per cut) | 0.1-0.5 mA/cut | Process-specific |

**Blech Length (immortality condition)** [PNR-Physics]:

```
J_critical * L < (J*L)_Blech

(J*L)_Blech ~ 2000-5000 A/cm for Cu (process-dependent)
```

Short intra-cell straps below the Blech length are "immortal" and can be granted EM waivers.

**Practical implication for layout:**
- Width each wire segment to keep J < J_max under worst-case DC current
- Use multi-cut vias: each via cut carries 0.1-0.5 mA max; compute required cuts from total current
- Temperature dependence is exponential: a 10C increase roughly halves MTTF

### 4.2 IR Drop

**Budget** [PNR-Physics]:
```
Analog: IR_drop < 5% of V_supply at any device pin
Digital: IR_drop < 10% of V_supply

For 1.0V analog: max 50 mV drop
For 1.8V analog: max 90 mV drop
```

**Resistive network model:**
```
R_segment = R_sheet * length / width
R_via = R_contact / num_cuts
IR_drop = V_supply - V_at_device_pin

Computed by solving: G * V = I
(G = conductance matrix, V = voltage vector, I = current vector)
```

**IR drop effects on matching:**
- Metallization voltage drops create systematic errors in matched ratios
- Metal and via resistance contributions should scale proportionally to desired resistance ratios [Hastings, Ch. 8, Rule 23]
- Use Kelvin connections for exceptional matching

### 4.3 Thermal Gradient Effects on Vth

**Temperature coefficient of Vth** [PNR-Physics]:
```
dVth/dT ~ -1 to -2 mV/C
```

For a matched pair with 1C temperature gradient: ~1-2 mV systematic offset.

**Thermal model** [PNR-Physics]:
```
T(x,y) = T_ambient + sum_d  P_d * G(x-x_d, y-y_d)

G(dx, dy) = 1/(2*pi*k_th*t_sub) * ln(R_max / sqrt(dx^2 + dy^2))

Where:
  P_d = power dissipated by device d (W)
  k_th = thermal conductivity of silicon (148 W/(m*K) at 300K)
  t_sub = substrate thickness (300-700 um)
  R_max = die-edge thermal boundary radius (um)
```

**Thermoelectric EMF (Seebeck Effect)** [PNR-Physics], [Hastings, Ch. 8]:
```
V_Seebeck = S * dT

S ~ 1-10 uV/C for typical metal junctions (Cu-W, Al-Cu)
S ~ 50-500 uV/K for Al-Si contacts [Hastings, Ch. 8]
```

A dT of 2K with S = 50 uV/K produces 0.1 mV -- enough for 0.4% mismatch in a bipolar current mirror [Hastings, Ch. 8].

**DMOS-FET thermal impact** [Lienig, 6.6.4]:
- Power transistors can create temperature differences of several tens of Kelvin across a chip
- Heat distributions are predictable from power device locations

**Self-heating of matched resistors** [Hastings, Ch. 8, Rule 22]:
- Exceptional matching: total power dissipation <= 1 mW
- Temperature rise must cause shifts much less than target mismatch

**Layout mitigation:**
- Place matched devices along isotherms [Lienig, 6.6.4]
- Common-centroid cancels linear thermal gradients
- Route resistor leads so both terminals exit from same end of serpentine [PNR-Physics]
- Keep matched signal paths at the same temperature
- Cancel thermoelectrics with even number of segments, antiparallel current flow [Hastings, Ch. 8, Rule 11]

### 4.4 Substrate Noise Coupling

**Attenuation model** [PNR-Physics]:
```
V_noise(d) ~ I_inject * R_sub(d)

R_sub(d) ~ rho_sub / (2*pi*d)        for large d
         ~ rho_sub / (4*A_contact)    for small d near injection point

Where:
  d = distance from noise source to sensitive device
  rho_sub = substrate resistivity (10-20 Ohm*cm for lightly-doped P-sub)
```

**Attenuation from guard rings** [PNR-Physics]:
- Single guard ring: 20-40 dB
- Deep N-well isolation: 40-60 dB
- Physical separation: noise drops as ~1/d

**Substrate debiasing** [Lienig, 7.1.1]:
```
V_debiasing = R_sub * I_sub
```

This local ground rise disrupts analog bias points, can forward-bias junction isolation diodes, and causes ground bounce through parasitic junction capacitances.

**Mitigation** [PNR-Physics], [Lienig, 7.1]:
- Guard rings around sensitive devices (20-40 dB per ring)
- Deep N-well isolation for critical analog (40-60 dB)
- Physical separation: maximum practical distance between analog and digital
- Dedicated analog supplies (VDD_analog, VSS_analog separate from digital)
- "Currentless" SUB net: substrate contacts on a dedicated net, not the circuit ground [Lienig, 7.1.1]
- Abundant substrate contacts with small track resistance to nearest contact

### 4.5 Antenna Effect

**Physical mechanism:** During plasma etching, charge collects on metal connected to a gate without a diffusion discharge path. Charge accumulates until Fowler-Nordheim tunneling damages the gate oxide [PNR-Physics].

**Antenna rule:**
```
Antenna_Ratio = Metal_area_on_layer / Gate_oxide_area

Violation if Antenna_Ratio > PDK limit (typically 400-1000 for metal, 200 for poly)
```

**Fixes** [PNR-Physics]:
1. Metal jumper: break long lower-layer segment with via to higher layer
2. Protection diode: reverse-biased diode from gate to substrate
3. Shorter segments: rearrange routing to keep ratios below limits

### 4.6 Latchup Prevention

**Trigger condition** [Lienig, 7.1.3]:
```
B_NPN * B_PNP >= 1
```

When the product of parasitic transistor current gains exceeds unity, regenerative feedback creates a destructive low-resistance path between VDD and VSS.

**Latchup triggers** [Hastings, Ch. 14], [Lienig, 7.1.3]:
- External transients pulling pins above VDD or below VSS
- Low-level ESD events
- Inductive kick-back from motors, relays, solenoids
- Charge pumps and capacitor-connected diffusions (internal triggers)
- Supply spikes and steep coupling signals through parasitic capacitances

**Guard ring types** [Hastings, Ch. 14]:

| Type | Carrier | Mechanism | Typical Efficiency |
|---|---|---|---|
| Electron-Collecting (ECGR) | Electrons | Reverse-biased N-junction collects from substrate | 90%+ (BiCMOS with NBL); marginal (std bipolar) |
| Electron-Blocking (EBGR) | Electrons | N+/P- interface field repels electrons | N/A in std bipolar |
| Hole-Collecting (HCGR) | Holes | Reverse-biased P-junction collects from well | ~unity (base diffusion as lateral PNP collector) |
| Hole-Blocking (HBGR) | Holes | N+/P- interface blocks holes | >95%, often >98% (std bipolar); requires 100:1 doping ratio |

**Guard ring design rules** [Hastings, Ch. 14]:
- Guard the INJECTOR, not the victim (fewer injectors than potential victims)
- HCGR + HBGR combined achieves >99% efficiency
- ECGR width: >= P-epi thickness (no retrograde well) or >= P-well depth (retrograde well)
- HBGR must completely encircle the injector
- Connect ECGRs to highest available supply voltage for deepest depletion
- Block dummy metal generation over guard ring contacts

**Physical design countermeasures** [Lienig, 7.1.3]:
- Low-resistance backgate connections (most effective): reduce R_well and R_sub so IR drop never reaches forward-bias threshold
- Guard rings: ring-fence NMOS with PSD contacts, PMOS with NSD contacts
- Spacing: increases parasitic base width but costs area
- Fully contacted Metal1 rings (interrupted only for drain contact)

**Critical warning on P- substrates** [Hastings, Ch. 14]:
- Designs on P- substrate are far more susceptible to latchup
- No amount of guard rings guarantees operation under severe inductive kickback
- A significant percentage of P- substrate designs require additional latchup-fix passes

---

## 5. The Analog Layout Engineer's Hierarchy of Needs

This prioritized list serves as a tiebreaker when algorithm objectives conflict. Lower-numbered concerns always override higher-numbered ones.

### Priority 1: Matching Correctness

**Why first:** A matched pair with poor matching produces a non-functional circuit (offset exceeds spec, CMRR/PSRR collapse, DAC INL/DNL blow out). No amount of area optimization or routing elegance can fix a circuit whose fundamental accuracy is wrong. The impact is immediate and binary: the circuit either meets its matching spec or it does not.

**What this means in practice:**
- Matched device pairs/arrays get placed FIRST, before any other constraint is resolved
- Common-centroid layout is mandatory for moderate and exceptional matching
- LDE equalization (WPE, LOD) is enforced during cell generation
- Dummy devices are always included at array boundaries
- Orientation uniformity is non-negotiable
- Matched net routing must be symmetric in R and C

**Quantitative reference:**
- Orientation mismatch alone: ~15% gm error [PNR-Checklist]
- WPE within 0.25 um of well edge: 25% current mismatch [Hastings, Ch. 13]
- LOD without dummies: up to 13% Id mismatch [PNR-Physics]
- Hydrogenation blocking: up to 20% Id mismatch [Hastings, Ch. 13]

### Priority 2: Isolation / Guard Rings

**Why second:** Isolation failures (latchup, substrate noise coupling, minority carrier injection) can be destructive or cause irreversible malfunction. Unlike matching, which degrades performance gradually, isolation failures are catastrophic. However, they rank below matching because a circuit with perfect isolation but poor matching still does not meet spec, whereas a circuit with perfect matching but marginal isolation may work in benign conditions.

**What this means in practice:**
- Every potential minority-carrier injector is enclosed by a guard ring [Hastings, Ch. 14]
- Well taps are placed per design rules (maximum spacing) and preferentially near matched devices
- Analog and digital domains get separate supply domains and substrate isolation
- Deep N-well isolation used for critical analog blocks where available
- Guard ring area is allocated BEFORE area optimization runs

**Quantitative reference:**
- Latchup can destroy the chip [Lienig, 7.1.3]
- Substrate noise: 20-40 dB attenuation per guard ring [PNR-Physics]
- Deep N-well: 40-60 dB isolation [PNR-Physics]

### Priority 3: Parasitic Minimization

**Why third:** Parasitics (C, R, L) degrade performance specifications (bandwidth, phase margin, noise, gain) but do not make the circuit non-functional. A slow but accurate amplifier is usually preferable to a fast but inaccurate one. Parasitic minimization is a continuous optimization -- more is better, but there is no hard pass/fail threshold.

**What this means in practice:**
- High-impedance nodes get shortest possible routing on lowest-capacitance metal layers
- Matched nets get parasitically symmetric routing (R and C matched)
- Sensitive signal routing is shielded from clock/digital
- Supply routing is sized for IR drop budget (< 5% V_supply)
- Via stacks are minimized on critical signal paths

**Quantitative reference:**
- Clock-to-signal coupling: 5x offset worsening [PNR-Checklist]
- IR drop budget: < 50 mV on 1.0V supply [PNR-Physics]
- BW degradation: proportional to parasitic C on high-Z nodes

### Priority 4: Reliability (EM / ESD / Antenna)

**Why fourth:** Reliability failures manifest over time (months/years) rather than at time-zero. A circuit that passes all specifications at t=0 but fails EM checks will degrade in the field. This is serious but ranks below the three concerns that determine whether the circuit works at all.

**What this means in practice:**
- All wires sized to meet J_max under worst-case DC current
- Multi-cut vias on all current-carrying paths (redundancy for yield and reliability)
- Antenna rules checked and fixed during routing (jumpers or protection diodes)
- ESD protection structures at all I/O pads

**Quantitative reference:**
- Al J_max(DC): ~1 mA/um; Cu J_max(DC): ~5 mA/um [PNR-Physics]
- MTTF halves per ~10C temperature increase (exponential in Black's equation)
- Antenna ratio limits: 400-1000 for metal, 200 for poly [PNR-Physics]

### Priority 5: Area

**Why last:** Area is a continuous economic metric. A larger die costs more but is otherwise functional. Every other concern in this hierarchy has a harder failure mode -- functional incorrectness, destruction, performance degradation, or field failure. Area should only be minimized AFTER all higher-priority concerns are satisfied.

**What this means in practice:**
- Dummy devices, guard rings, and generous matched-device sizing all cost area -- accept this cost
- Wirelength minimization is pursued but never at the expense of matching symmetry or parasitic balance
- Aspect ratio constraints serve packaging and routability, not pure area minimization
- Area optimization is the LAST pass in the layout flow

**Why this ordering serves as a tiebreaker:**
When an algorithm must choose between, for example, a more compact placement that worsens LOD matching versus a larger placement with equal SA/SB for all matched devices, this hierarchy dictates: take the larger placement. When a wider guard ring conflicts with shorter routing, take the wider guard ring. When symmetric matched-net routing costs extra wirelength, accept the wirelength. The ordering reflects the fundamental asymmetry of analog design: getting the answer right matters more than getting it small or fast.

---

## Appendix A: Quick-Reference Equations

| Quantity | Equation | Source |
|---|---|---|
| Vth mismatch (random) | sigma(dVth) = A_VT / sqrt(W*L) | [Pelgrom] |
| Vth mismatch (full) | sigma^2(dVth) = A_VT^2/(W*L) + S_VT^2*D^2 | [PNR-Physics] |
| Id mismatch | sigma^2(dId)/Id^2 = 4*sigma^2(dVth)/(Vgs-Vth)^2 + sigma^2(d_beta/beta) | [PNR-Physics] |
| Pocket-implant Pelgrom | sigma(dVth) = A_VT / sqrt(W * min(L, L_C)) | [Hastings, Ch.13] |
| WPE | dVth = sum a_k * exp(-d_k/lambda_k) | [PNR-Physics] |
| LOD | f(SA,SB) = 1 + K1*(1/SA+1/SB) + K2*(1/SA^2+1/SB^2) | [PNR-Physics] |
| Piezoresistivity | dR/R = pi_L*sigma_L + pi_T*sigma_T + pi_S*tau_S | [Hastings, Ch.8] |
| Black's equation (EM) | MTTF = A/J^n * exp(Ea/(k*T)) | [PNR-Physics] |
| IR drop | V_drop = R_sheet * L/W * I | [PNR-Physics] |
| Thermal gradient | T(x,y) = T_amb + sum P_d * G(dx,dy) | [PNR-Physics] |
| dVth/dT | -1 to -2 mV/C | [PNR-Physics] |
| Seebeck EMF | V = S * dT; S ~ 50-500 uV/K (Al-Si) | [Hastings, Ch.8] |
| Substrate noise | V_noise ~ I_inject * rho_sub / (2*pi*d) | [PNR-Physics] |
| Antenna ratio | Metal_area / Gate_oxide_area < PDK limit | [PNR-Physics] |
| Latchup condition | B_NPN * B_PNP >= 1 | [Lienig, 7.1.3] |
| Blech immortality | J*L < (J*L)_Blech ~ 2000-5000 A/cm (Cu) | [PNR-Physics] |

## Appendix B: Key Numerical Constants

| Constant | Value | Context |
|---|---|---|
| k_th (Si, 300K) | 148 W/(m*K) | Thermal conductivity |
| dVth/dT (MOS) | -1 to -2 mV/C | Vth temperature coefficient |
| Ea (Cu EM) | 0.9 eV | Black's equation activation energy |
| Ea (Al EM) | 0.7 eV | Black's equation activation energy |
| J_max DC (Cu) | ~5 mA/um | EM limit |
| J_max DC (Al) | ~1 mA/um | EM limit |
| rho_sub (P-sub) | 10-20 Ohm*cm | Substrate resistivity |
| WPE lambda | 1-5 um | WPE decay length |
| WPE amplitude | 10-50 mV | WPE Vth shift coefficient |
| Guard ring atten. | 20-40 dB per ring | Substrate noise |
| Deep N-well atten. | 40-60 dB | Substrate noise |
| CTE Si | 2.6 ppm/C | Packaging stress |
| CTE epoxy mold | 8-35 ppm/C | Packaging stress |
| Seebeck (Al-Si) | 50-500 uV/K | Thermoelectric EMF |
| HBGR efficiency | >95%, often >98% | Hole blocking (std bipolar) |
| ECGR efficiency | >90% (BiCMOS w/ NBL) | Electron collection |

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| MISSING-01 | Added new subsection "2.5 Conductivity Modulation and Dielectric Absorption" covering body/tank bias matching for resistors, field plate requirements for Rsh > 1 kOhm/sq, and split field plates for exceptional HSR matching | Conductivity modulation (Hastings Ch. 8 Section 8.2.9) was an unaddressed mismatch mechanism for high-sheet resistors, capable of causing up to 0.1%/V systematic mismatch |
