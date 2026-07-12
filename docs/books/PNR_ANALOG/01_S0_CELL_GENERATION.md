# S0 -- Device-Level Cell Generation: Detailed Specification (Analog-Enriched)

---

## 1. Purpose

S0 transforms each schematic device into a physical layout cell that embodies the matching, isolation, and reliability techniques that experienced analog layout engineers apply by hand. This stage runs before placement and routing, producing the atomic rectangles that S2 places and S3 routes.

The fundamental insight (Hastings, Razavi, Baker): **most analog performance is won or lost at the device level**, not the block level. A perfectly placed and routed layout built from poorly constructed cells will fail. S0 must get the cells right.

---

## 2. Inputs and Outputs

**Inputs:**
- Device instance from netlist (type, W, L, multiplier M, number of fingers NF)
- Matching group membership (from S1 pre-pass or designer annotation)
- PDK design rules (layers.json)
- PDK device models (for LDE parameters: WPE coefficients, STI stress tables)
- Target application class (precision, high-speed, power, general)

**Outputs per cell:**
- DRC-clean GDSII geometry
- Pin map: {pin_name -> list of (layer, rectangle)} for each terminal
- Bounding box (including guard ring padding)
- Metadata record:
  - Device type, W_eff, L_eff, finger count, finger width
  - Pattern type used (single, interdigitated, CC, clustered)
  - Dummy count (per edge)
  - Guard ring type and width
  - Internal parasitic estimate (R_gate, C_drain, C_source per finger)
  - Orientation tag

### 2.A Analog Considerations

The outputs listed above must be extended for analog correctness:

**Additional metadata fields required:**
- SA, SB per finger (source-side and drain-side OD-to-STI edge distances) -- needed for LOD/STI stress equalization verification [Hastings ch13, 00_ANALOG_PRINCIPLES Section 2.2].
- WPE distance vector: distance from each active gate center to the four nearest well edges [Hastings ch13, 00_ANALOG_PRINCIPLES Section 2.1].
- Orientation chi value: the signed orientation metric (+1 for current-flows-right, -1 for current-flows-left, averaged across all fingers) [Hastings ch13]. Matched devices must have equal chi.
- Dummy connection type: gate-to-ground, gate-to-signal, or floating. Affects parasitic capacitance accounting in S1.
- Hydrogenation-safe zone: the rectangular region over which dummy metal generation must be blocked (extends >= 5 um beyond active gate area for exceptional matching) [Hastings ch13].
- Piezoresistance/piezojunction orientation sensitivity class: records whether the device lies along <110> or <100> on (100) Si, and whether the device type is NMOS, PMOS, NPN, or lateral PNP, since each has different stress sensitivity [Hastings ch8, ch13, 00_ANALOG_PRINCIPLES Section 2.3].
- **bias_current_mA**: per-device DC bias current (Id) from .op data. Needed by S3 for EM wire sizing and by S1 for parasitic budget computation. Populated by S1 from .op simulation results; stored by S0 after S0-S1 iteration converges.
- **overdrive_voltage_mV**: per-device overdrive voltage (Vgs-Vth) from .op data. Needed by S1 for Pelgrom current mismatch calculation `sigma^2(dId)/Id^2 = 4*sigma^2(dVth)/(Vgs-Vth)^2 + sigma^2(d_beta/beta)` and for the subthreshold guard (Rule 4: avoid Veff < 100 mV). Populated by S1 from .op data; stored by S0 after S0-S1 iteration converges.

**Why this matters:** Without SA/SB and WPE metadata, S2 cannot verify that matched devices have equalized LDE contexts after placement. Without chi, S2 cannot detect orientation violations introduced by flipping/rotation. Without the hydrogenation zone, S3's dummy-metal fill can silently destroy matching.

### 2.B Analog-Algorithm Interface

The algorithm producing cell outputs must respect:

| Contract Item | Constraint | Source |
|---|---|---|
| SA/SB reporting | Every finger's SA and SB must be computed and stored; interior shared-diffusion fingers report distance to the merged OD edge, not to STI | [Hastings ch13], [00_ANALOG_PRINCIPLES 2.2] |
| WPE vector | Distances measured from channel center to each of the four well edges (left, right, top, bottom) | [00_ANALOG_PRINCIPLES 2.1] |
| Orientation chi | chi = (1/N_f) * sum(chi_i); must be reported as a signed float | [Hastings ch13] |
| Hydrogenation zone | Rectangle = cell_bbox inflated by max(5 um, PDK.metal_hydrogen_clearance) on all sides | [Hastings ch13] |
| Matching group devices | Must share identical (W_f, L, finger_width, dummy_count, guard_ring_type, orientation_chi) | [00_ANALOG_PRINCIPLES 1.5] |

---

## 3. Sub-Block: Multi-Finger Decomposition Engine

### 3.1 Why Fingers

A wide transistor W_total is split into N_f parallel fingers of width W_f = W_total / N_f sharing source/drain diffusions. Three simultaneous benefits:

1. **Gate resistance reduction:** R_g is proportional to (W_f/L) * R_sh / (3 * N_f^2) for single-side-contacted gates. Razavi's rule: R_g < (1/5 to 1/10) * (1/g_m) per finger for low-noise; R_g < 1/g_m for general.

2. **Junction capacitance reduction:** Shared S/D diffusions reduce total drain junction area. With N_f fingers, only (N_f+1)/2 drain diffusions exist instead of N_f.

3. **Matching enablement:** Fingers are the unit cells for interdigitation and common-centroid.

### 3.2 Finger Count Selection Algorithm

```
Input: W_total, L, R_sh_poly, g_m, application_class, PDK.min_W, PDK.max_W
Output: N_f, W_f

1. Compute target R_g based on application:
   - LNA/RF:     R_g_target = (1/10) * (1/g_m)
   - Precision:   R_g_target = (1/5)  * (1/g_m)
   - General:     R_g_target = 1/g_m
   - Power:       R_g_target = determined by EM, not noise

2. Compute minimum N_f from gate resistance:
   N_f_min_Rg = ceil(sqrt(W_total * R_sh_poly / (3 * L * R_g_target)))

3. Compute W_f = W_total / N_f_min_Rg
   If W_f > PDK.max_finger_width:
     N_f = ceil(W_total / PDK.max_finger_width)
   If W_f < PDK.min_finger_width:
     N_f = floor(W_total / PDK.min_finger_width)
     Flag warning: may not meet R_g target

4. Prefer even N_f for symmetric abutment of matched devices.
   If N_f is odd and device is in a matching group:
     N_f += 1 (or N_f -= 1, whichever keeps W_f in range)

5. For FinFET: N_f = number of fin groups; W_f = N_fins * fin_pitch.
   Width is quantized -- round to nearest legal fin count.

Output: N_f, W_f = W_total / N_f
```

### 3.3 Drain/Source Assignment

- **Interior shared diffusions:** assign to the high-swing node (typically drain) to minimize total drain capacitance.
- **Outer diffusions:** assign to the low-swing node (typically source or ground-connected).
- Exception: if the source node is high-impedance (source follower), swap.
- For FinFET: S/D assignment follows the same logic but via metal connection, not diffusion geometry.

### 3.4 Gate Contact Strategy

- **Single-sided contact:** simpler, used when R_g is acceptable.
- **Double-sided contact:** reduces R_g by 4x (1/12 vs 1/3 factor). Required for RF/high-speed.
- **Alternating contact sides:** for very long finger arrays, alternate which side each gate is contacted from to equalize parasitics.

### 3.A Analog Considerations

#### 3.A.1 Finger Count Is Not Purely an R_gate Optimization

The spec in Section 3.2 derives N_f primarily from gate resistance. This is a digital-inspired assumption. In analog, finger count is simultaneously constrained by four additional concerns:

**1. Matching granularity.** Fingers are the atomic units for interdigitation and common-centroid assignment (Section 4). The number of fingers per device in a matched pair must be chosen so that the five rules of common-centroid layout (coincidence, symmetry, dispersion, compactness, orientation) can all be satisfied simultaneously [Hastings ch13, Table 13.2]. This often requires:
- Even N_f (for ABBA or cross-coupled patterns)
- N_f divisible by the number of devices in the matching group (for equal subdivision)
- N_f large enough to enable 2D CC layouts when the matching tier demands it

**2. LOD/STI stress equalization.** Every finger's SA (gate-to-source-side OD edge) and SB (gate-to-drain-side OD edge) must be identical across all matched devices. Interior fingers naturally share diffusions, giving them equal SA/SB. But end fingers have asymmetric OD context unless dummies are present. Therefore:
- N_f should be chosen so that the number of active fingers is sufficient after reserving dummy slots on each edge.
- For precision applications, moat should extend >= 5 um beyond the last active transistor [Hastings ch13].

**3. Orientation chi equalization.** For multi-finger devices, the orientation metric chi = (1/N_f) * sum(chi_i), where chi_i = +1 if current flows right, -1 if left. Matched devices must have equal chi. Zero orientation (equal left/right fingers) is preferred and requires even N_f [Hastings ch13].

**4. Pelgrom sizing interaction.** The total device area W*L*N_units must satisfy the Pelgrom mismatch budget (Section 4.3). If Pelgrom requires a larger area than the schematic W*L, the algorithm must decide whether to increase N_f (more fingers at the same W_f) or to increase W_f or L. The choice has circuit implications:
- Increasing N_f at constant W_f: preserves gm/Id; adds routing complexity.
- Increasing W_f: increases Cgs, Cgd proportionally; may move device out of its operating region.
- Increasing L: reduces gm, increases ro; suitable for current-matching mirrors where long L is desired anyway [Hastings ch13, Rule 3].

#### 3.A.2 Practical MOS Finger Sizing from Hastings ch13

Hastings provides quantitative sizing tables for matched MOS transistors that go far beyond R_g optimization:

**Voltage matching** (sigma(dVth) target) -- required active area W*L*N_units [Hastings ch13, Table 13.5]:

| Accuracy (6-sigma) | 12V process | 5V process | 3.3V process | 1.8V process |
|---|---|---|---|---|
| Moderate (1-3 mV) | 4,700 um^2 | 630 um^2 | 220 um^2 | 64 um^2 |
| Exceptional (<0.3 mV) | 42,000 um^2 | 5,600 um^2 | 1,900 um^2 | 580 um^2 |

**Current matching** (sigma(dId/Id) target) -- required channel length L at 10 uA [Hastings ch13, Table 13.6]:

| Accuracy (6-sigma) | 12V NMOS | 5V NMOS | 3.3V NMOS | 1.8V NMOS |
|---|---|---|---|---|
| Minimal (2-5%) | 14 um | 8.6 um | 6.6 um | 4.9 um |
| Moderate (0.5-1%) | 79 um | 48 um | 37 um | 27 um |
| Exceptional (<0.1%) | 240 um | 140 um | 110 um | 81 um |

These lengths can be dramatically longer than anything a digital-inspired R_g calculation would produce. The algorithm must check current-matching length requirements before settling on N_f.

#### 3.A.3 Pocket-Implant Device Warning

For transistors with halo/pocket implants, the modified Pelgrom scaling law applies [Hastings ch13]:

```
sigma(dVth) = A_VT / sqrt(W * min(L, L_C))
```

where L_C is typically 1-2 um. Beyond L_C, further lengthening does NOT improve matching. The finger count algorithm must:
1. Check whether the PDK device has pocket implants.
2. If yes, flag that current matching via long L is futile beyond L_C.
3. Recommend switching to an analog-friendly (non-pocket) device variant, or using source degeneration resistors instead [Hastings ch13, Rule 5].

**Directional tilted pocket implant constraint [Hastings ch12, Section 12.2.7].** If the PDK uses directional tilted pocket implants (shot only left-right to selectively apply pockets to horizontally-oriented digital transistors while avoiding vertically-oriented analog transistors), the following absolute constraint applies:

- Analog transistors must be oriented orthogonally to the implant direction (typically vertical channel for horizontal implants).
- Matched analog cells MUST NOT be rotated by 90 degrees -- only 0-degree and 180-degree rotations are permitted.
- The cell generator must query `PDK.pocket_implant_direction` and enforce this constraint.
- Pocket-implant devices have output resistance that scales as sqrt(L) instead of L, breaking the linear ro vs. L scaling that analog designers rely on for current matching. Random Vth variation does not follow Pelgrom's 1/sqrt(WL) law in pocket-implant devices.

**Pocket implant blocking.** Where extra masks are available, long-channel analog transistors should block pocket implants entirely using the pocket-block mask. This restores standard Pelgrom scaling and linear ro vs. L behavior.

#### 3.A.4 BJT Finger Sizing

For bipolar transistors, the concept of "fingers" maps to unit emitters:

- **Vertical NPN:** Use identical unit emitters with width >= 2x minimum (typically 8-16 um for moderate matching). Compact geometries (square, circular, octagonal) maximize area-to-periphery ratio [Hastings ch10].
- **Lateral PNP:** Always use minimum-size emitters. Larger emitters degrade beta because the graded well doping generates a field that drifts minority carriers downward, away from the collector [Hastings ch9, ch10]. For larger devices, array minimum emitters; never elongate.
- **Emitter area ratio sweet spot:** For ratioed pairs (bandgap VPTAT), optimal ratio is 4:1 to 16:1 (8:1 most popular). Smaller ratios give too little VPTAT; larger ratios waste area and increase gradient vulnerability [Hastings ch10].
- **CBE orientation:** Collector-base-emitter from outside to inside is preferred, bringing emitters closer together for better thermal matching [Hastings ch10].

#### 3.A.5 Orientation and Stress Effects on Finger Sizing

On (100) silicon, carrier mobility is stress-sensitive and orientation-dependent [Hastings ch13]:

| Device | pi_L (10^-11 Pa^-1) | pi_T (10^-11 Pa^-1) | Minimum stress sensitivity direction |
|---|---|---|---|
| NMOS <110> | 30 | -17 | <110> (H/V in standard layout) |
| PMOS <110> | -65 | 40 | <100> (45 deg to wafer flat) |

PMOS is approximately 2x more stress-sensitive than NMOS. For exceptional PMOS matching, consider 45-degree orientation if directional implants are not used [Hastings ch13, Rule 24].

All fingers within a matched device must have identical current-flow direction relative to the crystal axes. Rotating or mirroring a cell containing one of two matched transistors silently introduces orientation mismatch that can cause several percent gm error [Hastings ch13].

### 3.B Analog-Algorithm Interface

The finger count selection algorithm (Section 3.2) must respect:

| Contract Item | Constraint | Source |
|---|---|---|
| Even N_f for matched devices | N_f must be even when device is in a matching group, to enable zero-orientation (chi=0) interdigitation | [Hastings ch13, Rule 7] |
| Minimum W_f for matching | W_f >= 150% of PDK.min_W (minimal), >= 200% (moderate), >= 400% (exceptional) | [Hastings ch13, Rule 11] adapted from [Hastings ch8, Rule 4] for resistors |
| Minimum L for current matching | L must meet the current-matching table values, not just R_g requirements | [Hastings ch13, Table 13.6] |
| Pocket-implant check | If device has pocket implants, cap effective L at L_C for Pelgrom computation | [Hastings ch13] |
| BJT: minimum-size emitters for laterals | Lateral PNP emitter size must not exceed the PDK minimum | [Hastings ch10, Rule 2 for laterals] |
| BJT: compact emitter geometry | Vertical NPN emitters should be square or circular (maximize area/perimeter) | [Hastings ch10, Rule 3] |
| No submicron dimensions for matched devices | Both W and L must be substantially above minimum for matched MOS | [Hastings ch13, Rule 11] |
| Finger width range for matching | Emitter widths of 2x to 10x minimum for BJTs; avoid very large single emitters | [Hastings ch10] |

---

## 4. Sub-Block: Pattern Selector

### 4.1 Pattern Types

| Pattern | Sequence | Centroid match? | Gradient cancellation | Routing complexity | Best for |
|---------|----------|-----------------|----------------------|-------------------|----------|
| Single  | A only   | N/A             | N/A                  | Minimal           | Non-matched devices |
| Clustered | AABB   | No              | None                 | Low               | Never for precision |
| Interdigitated | ABAB | Partial (1-D) | Partial (one axis)  | Medium            | Low-precision pairs, large ratios |
| CC 1-D  | ABBA     | Yes (1-D)       | Linear, one axis     | Medium-high       | Diff pairs, 1:1 mirrors |
| CC 2-D  | Cross-quad | Yes (2-D)     | Linear, both axes    | High              | Precision (bandgap, ADC) |

### 4.2 Selection Algorithm

```
Input: matching_group, sensitivity_analysis, application_class
Output: pattern_type

1. If device is not in any matching group -> Single.

2. If matching group has ratio != 1:1:
   Use interdigitated with unit-cell replication.
   For 1:N mirror: place N units of the mirror device
   interleaved around 1 unit of the reference.
   Example 1:4 -> D_ref D4 D3 D_ref D2 D1 (with dummies at ends)

3. If matching group is 1:1 pair:
   a. If application_class == "precision" (offset < 100 uV, INL < 0.5 LSB):
      Use CC 2-D (cross-quad) if area permits.
   b. If application_class == "high-speed" and parasitic budget is tight:
      Use CC 1-D (ABBA) -- less routing overhead than 2-D.
   c. If application_class == "general":
      Use CC 1-D (ABBA).

4. For capacitor arrays (DAC, filter):
   Always CC 2-D with spiral or block-chessboard placement.
   Co-optimize unit-cap size, array dimensions, and routing
   to balance f_3dB vs. INL/DNL (per Sapatnekar DATE 2021/2022).

5. For resistor arrays (R-2R, trim):
   1-D interdigitation with serpentine routing.
   Even number of segments per resistor (Hastings).
   Kelvin sensing for precision resistors.

6. Override: if designer specifies pattern in constraint file, honor it.
```

### 4.3 Pelgrom-Driven Unit Sizing

For any matched pair, the unit device area W*L must satisfy:

sigma(dV_th) = A_VT / sqrt(W * L * N_units) < sigma_target

where A_VT comes from the PDK (typically 1-5 mV*um for modern nodes), N_units is the number of unit cells per device, and sigma_target comes from the performance spec (e.g., for a diff pair with 1 mV offset at 3-sigma: sigma_target = 0.33 mV).

```
Input: A_VT (from PDK), sigma_target (from spec), W_schematic, L_schematic
Output: W_unit, L_unit, N_units

1. Compute required W*L per unit * N_units:
   (W*L*N) >= (A_VT / sigma_target)^2

2. Start with W_unit = W_schematic, L_unit = L_schematic, N_units = 1.
3. If W_unit * L_unit * N_units < required:
   Increase N_units (more common-centroid units) until met.
4. If N_units becomes impractically large (> 16):
   Increase W_unit and/or L_unit.
5. Verify: increasing L changes gm/Id; increasing W changes capacitance.
   Flag if sizing change moves the device out of its operating region.
```

### 4.A Analog Considerations

#### 4.A.1 The Five Rules of Common-Centroid Layout

The pattern selector must enforce all five rules of common-centroid layout [Hastings ch13, Table 13.2], [Lienig 6.6.2]:

| Rule | Requirement | What It Cancels |
|---|---|---|
| 1. COINCIDENCE | Centroids of matched devices coincide | Linear gradients (any direction) |
| 2. SYMMETRY | Array is symmetric about both H and V axes | Ensures coincidence is achievable |
| 3. DISPERSION | Large arrays subdivide into smaller CC subarrays | Quadratic gradient residues |
| 4. COMPACTNESS | Each array/subarray is as compact as possible | Higher-order gradient residues |
| 5. ORIENTATION | Matched transistors have equal orientation chi | Stress/mobility asymmetry |

**Quantitative benefit of 2D vs 1D:** A 2D cross-coupled AB/BA array of square elements exhibits approximately 60% of the residual gradient-induced mismatch of a 1D ABBA array of the same elements [Hastings ch13]. Always prefer 2D when device geometry allows (W approximately equal to L).

**Array aspect ratio constraints** [Hastings ch13, Rule 9]:
- Voltage matching: array aspect ratio <= 3:1
- Current matching: up to 10:1 for minimal, 3:1 for moderate, approximately 1:1 for exceptional

#### 4.A.2 Ratioed Pairs and Quads (BJT-Specific)

For bandgap and VPTAT circuits using bipolar transistors, the pattern selector must support ratioed configurations [Hastings ch10]:

- **4:1 ratio**: Place the 1X transistor in the center with two sections of the 4X transistor on either side (2:1:2 pattern).
- **8:1 ratio**: Use a 3x3 array -- the center transistor is the 1X device, surrounded by 8 unit transistors forming the 8X device. This 2D array spans only one-third the distance of a linear arrangement, reducing quadratic gradient residues by nearly an order of magnitude [Hastings ch10].
- **Ratioed quad (4:1:1:4):** Produces delta_VBE = VT * ln(16) = 72 mV at room temperature. The 2D arrangement of four transistors (two 4X and two 1X) cancels both linear thermal and stress gradients.

The optimal emitter area ratio lies between 6:1 and 16:1. As the ratio N increases, VPTAT increases as ln(N) (logarithmic -- diminishing returns) while the physical span increases as sqrt(N), making the array more vulnerable to nonlinear gradient residues [Hastings ch10].

#### 4.A.3 Interdigitation Pattern Details

For MOS transistors with common source connections, standard interdigitation patterns from Hastings ch13, Table 13.3 include:

1. D_S A_D B_S B_D A_S D (basic ABBA with dummies D)
2. (D_S A_D B_S B_D A_S)^n D (repeated ABBA for larger arrays)

If sources do NOT share a common node (e.g., current mirrors where sources connect to different potentials), spaces must be inserted between different devices' fingers, or **width-direction interdigitation** must be used [Hastings ch13]. Width-direction interdigitation is common for current mirrors since they share a gate net.

**Critical subtlety on source/drain notation** [Hastings ch13]: The subscript notation (S for source, D for drain) specifies which terminal faces left at each boundary. This determines:
- Whether adjacent fingers can share a diffusion (same terminal type = shareable)
- The orientation chi of each finger

#### 4.A.4 Current Matching vs. Voltage Matching -- Different Sizing Strategies

The spec's Pelgrom sizing (Section 4.3) treats all matching as scaling with 1/sqrt(WL). This is correct for voltage matching but incomplete for current matching [Hastings ch13, Rules 2-3]:

**Voltage matching** (minimizing sigma(dVth)):
- Increase total device area W*L*N_units.
- Thin-oxide devices need dramatically less area (a 1.8V device needs only about 10% of the area of a 12V device for the same voltage matching accuracy).
- Both W and L contribute equally.

**Current matching** (minimizing sigma(dId/Id)):
- Increasing W at fixed Id does NOT help. The dominant mechanism is channel-length modulation: dId/Id approximately equals lambda * dVDS, and lambda is inversely proportional to L [Hastings ch13].
- Increasing L is the primary knob. Required lengths can reach 240 um for exceptional matching at 12V/10uA [Hastings ch13, Table 13.6].
- Cascodes should be used to equalize VDS across matched devices [Hastings ch13].

The Pelgrom unit sizing algorithm must distinguish these two cases and apply the correct sizing strategy.

#### 4.A.5 Drain Current Mismatch Equation

The full drain current mismatch model for a matched MOS pair [00_ANALOG_PRINCIPLES 1.1]:

```
sigma^2(dId)/Id^2 = 4 * sigma^2(dVth) / (Vgs - Vth)^2 + sigma^2(d_beta/beta)
```

Design implication: run matched pairs at high overdrive (Vgs - Vth >= 100 mV at minimum, higher for precision) to suppress the Vth mismatch term. Avoid subthreshold operation of matched transistors -- stringer transistors at the channel edges cause dramatic mismatch below Veff = 100 mV [Hastings ch13, Rule 4].

### 4.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| CC rule enforcement | Pattern selector must verify all five CC rules before emitting a pattern | [Hastings ch13, Table 13.2] |
| 2D preference | For precision and exceptional matching, 2D CC must be attempted before falling back to 1D | [Hastings ch13, Rule 10] |
| Current vs. voltage matching | Algorithm must accept matching_type={voltage, current} as input and apply different sizing strategies | [Hastings ch13, Rules 2-3] |
| Aspect ratio limit | Array aspect ratio must not exceed 3:1 for voltage matching, 1:1 for exceptional current matching | [Hastings ch13, Rule 9] |
| Ratioed pair support | For BJT ratioed pairs, algorithm must support 2D array patterns (e.g., 3x3 "eight around one") | [Hastings ch10] |
| Subthreshold guard | If any matched MOS device has Veff < 100 mV, emit a warning and recommend either increasing Veff or adding poly fins to suppress stringers | [Hastings ch13, Rule 4] |

---

## 5. Sub-Block: Dummy Device Inserter

### 5.1 Why Dummies

Edge devices in an array experience:
- Different etch rates (aspect-ratio-dependent etching / microloading)
- Different lithographic exposure (pattern density effects)
- Different STI/LOD stress (asymmetric trench proximity)
- Different WPE (asymmetric well-edge distance)

Dummies sacrifice area to give every active unit an identical neighborhood.

### 5.2 Dummy Insertion Rules

```
1. Add at least 1 dummy finger/unit on each edge of the array.
   For precision applications, add 2 dummies per edge.

2. Dummies MUST be in the same OD (active/diffusion) region as
   the active devices -- a dummy in a separate OD island does not
   buffer STI stress for the active devices.

3. Default dummy connection: gate tied to ground (NMOS) or VDD (PMOS),
   source and drain tied to ground (NMOS) or VDD (PMOS).
   -> Device is off, presents uniform boundary.

4. Alternative (precision): gate tied to the matched-device gate net.
   -> Truly identical boundary at the cost of added capacitance.
   Use only when the added Cgs on the signal node is acceptable.

5. For capacitor arrays: add dummy unit capacitors around the
   entire perimeter. Connect bottom plate to ground.

6. For resistor arrays: add dummy serpentine segments at both ends
   of the array, same width and spacing as active segments.

7. Document the dummy connection choice in the cell metadata
   so S1 can account for added parasitics.
```

### 5.A Analog Considerations

#### 5.A.1 Quantitative Dummy Requirements by Matching Tier

The spec gives "1 dummy per edge" as a default and "2 for precision." Hastings ch13 provides more granular tier-specific requirements:

**MOS transistors** [Hastings ch13, Rule 12]:

| Tier | Dummy Requirement |
|---|---|
| Minimal | Optional (but strongly recommended) |
| Moderate, L < 1 um | Full dummy transistor at each end |
| Moderate, L > 1 um | Half dummy (single S/D termination) acceptable |
| Exceptional | Outermost dummy poly edge at least 3 um from nearest active gate; moat extends >= 5 um beyond last active gate |

**Resistors** [Hastings ch8, Rule 9]:

| Tier | Diffused/Implanted | Deposited (Poly) |
|---|---|---|
| Minimal | Not required | Single min-width dummy each end |
| Moderate | Full-width dummy each end | Full-width dummy each end |
| Exceptional | Full-width dummies each end | Multiple dummies spanning >= 10 um each end |

**Capacitors** [Hastings ch8, Rule 7]:
- Moderate with shield: dummy width >= 3x minimum.
- Without shield: much wider dummies needed (fringing extends >= 50 um).
- Exceptional: exact copies of unit capacitors as dummies on all four sides with matched spacing. Both electrodes of dummies should connect to a circuit node.

#### 5.A.2 LOD/STI Stress Equalization via Dummies

The primary purpose of dummies in modern processes is not etch-rate equalization (which diminishes rapidly beyond approximately 1 um) but LOD/STI stress equalization [Hastings ch13]:

STI generates compressive stress on adjacent silicon. The stress-induced transconductance shift obeys [00_ANALOG_PRINCIPLES 2.2]:

```
f(SA, SB) = 1 + K1*(1/SA + 1/SB) + K2*(1/SA^2 + 1/SB^2)
```

Without dummies, end devices have different SA/SB from interior devices, causing up to 13% NMOS Id reduction and 10%+ PMOS Id increase [00_ANALOG_PRINCIPLES 2.2]. Dummies fix this by:
1. Extending the OD region beyond the last active gate by >= 3 um (moderate) or >= 5 um (exceptional) [Hastings ch13].
2. Ensuring every active gate sees identical poly geometry on both sides.

**Specific technique** [Hastings ch13, Fig 13.52]:
- **Moat stretch**: Widen the moat (active region) at array ends so STI is pushed >= 5 um from the last active gate.
- **Full dummy transistor**: Place an electrically inactive transistor with its own wide moat, merged into the same OD as the active array.

#### 5.A.3 Microloading and Corner Rounding

Poly etch rates depend on surrounding geometry. End gates facing open space etch faster, becoming slightly shorter (microloading) [Hastings ch13]. Additionally, poly corners round during fabrication with effects extending approximately 0.5 um [Hastings ch13].

Mitigation rules:
- All poly gates (including dummies) should extend equally beyond the moat edge, by at least 0.5 um beyond design-rule minimum [Hastings ch13, Rule 21].
- The regular poly pattern should extend approximately 3 um beyond the last active gate for exceptional matching [Hastings ch13].
- Block CMP dummy poly generation within approximately 3 um (exceptional) or 1 um (moderate) of matched transistors [Hastings ch13, Rule 23].

#### 5.A.4 Dummy Elements for 2D Capacitor Arrays

For capacitor arrays requiring exceptional matching, dummies must surround the entire 2D perimeter [Hastings ch8, Rule 7]:
- Dummies must be exact copies of unit capacitors (same geometry, same spacing).
- Both electrodes of dummies must connect to a circuit node (not floating).
- An electrostatic shield (grounded metal plate) should cover the entire array. Shield overhang >= 50 um if no dummies, >= 5 um with dummies [Hastings ch8, Rule 8].

### 5.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| Tier-dependent dummy count | Algorithm must accept matching_tier={minimal, moderate, exceptional} and apply tier-specific rules from Section 5.A.1 | [Hastings ch8, ch13] |
| Same-OD requirement | Dummies must be generated in the same active/diffusion region as active devices, not as separate islands | [Hastings ch13] |
| Moat extension | For exceptional matching, the OD region extends >= 5 um beyond the last active gate | [Hastings ch13] |
| Poly extension uniformity | All gate poly (active + dummy) extends identically beyond moat | [Hastings ch13, Rule 21] |
| CMP dummy poly blocking | Block dummy poly generation within clearance distance of matched gates | [Hastings ch13, Rule 23] |
| 4-sided capacitor dummies | For 2D capacitor arrays, dummies must surround all four sides | [Hastings ch8, Rule 7] |

---

## 6. Sub-Block: Guard Ring and Well-Tap Generator

### 6.1 Guard Ring Types

| Ring type | Around | Tied to | Purpose |
|-----------|--------|---------|---------|
| P+ substrate ring | NMOS group | VSS (ground) | Collect substrate minority carriers, fix substrate potential |
| N+ well ring | PMOS group (in N-well) | VDD | Fix N-well potential, isolate from substrate noise |
| Deep N-well ring | Sensitive NMOS (triple-well) | VDD | Isolate P-well body from global substrate |
| Combined ring | Mixed group | Both VSS and VDD | Full isolation for sensitive analog blocks |

### 6.2 Guard Ring Parameters

```
Input: PDK.latchup_max_distance, PDK.guard_ring_width, sensitivity_class
Output: ring_width, tap_spacing, ring_to_device_spacing

1. ring_width = max(PDK.guard_ring_width, PDK.min_contact_ring_width)
   For sensitive blocks: ring_width = 2 * PDK.guard_ring_width

2. ring_to_device_spacing = PDK.min_well_to_active_spacing + margin
   margin = 0 for general, PDK.min_well_to_active_spacing for sensitive

3. tap_spacing within the ring:
   Default: PDK.latchup_max_distance / 2
   Analog guideline: <= 50 um (Silva-Martinez / TAMU guideline)
   Sensitive: <= 25 um

4. The ring inflates the cell bounding box by:
   inflation = 2 * (ring_width + ring_to_device_spacing)
   This inflation MUST be reported to S2 so placement accounts for it.
```

### 6.3 Well-Tap Insertion (Inside Cells)

For large cells where the guard ring alone doesn't meet the tap-distance rule:

```
1. Compute max distance from any point in the cell to the nearest
   guard ring tap.
2. If max_distance > PDK.latchup_max_distance:
   Insert internal well taps (butted contacts where possible)
   at regular intervals to bring every point within the limit.
3. For NMOS: P+ substrate taps tied to VSS.
   For PMOS: N+ well taps tied to VDD.
4. Butted contacts (shared diffusion for device S/D + substrate tap)
   provide compact, low-inductance body ties.
```

### 6.A Analog Considerations

#### 6.A.00 Parasitic Channel Prevention and Field Plate Generation

For high-voltage analog (Vds > 75% of thick-field threshold), the cell generator must insert channel stops (minimum-width NSD strips) or field plates (poly or lower metal connected to highest supply) to prevent parasitic channel formation [Hastings, Ch. 5, Section 5.3.5], [Hastings, Ch. 12, Section 12.2.3]. Parasitic channels form when six conditions coexist: lightly doped backgate, source diffusion, drain diffusion, gate conductor (metal or poly crossing over thick field oxide), sufficient Vgs, and nonzero Vds. Preventive measures:

1. **Channel stop implants:** Insert minimum-width NSD strips along paths where metal or poly crosses thick field oxide above lightly doped P-type silicon. The N+ doping raises the parasitic threshold voltage above the maximum operating voltage.

2. **Field plates for lateral PNP transistors:** Always generate a field plate covering the base surface between emitter and collector, connected to the emitter terminal. Without this field plate, the thick field oxide over the lightly doped base region acts as a parasitic MOS gate, potentially forming an inversion layer that shorts emitter to collector.

3. **Poly retraction for CMOS high-voltage PMOS:** Ensure poly gate leads are retracted inside the N-well by at least the N-well overlap of PSD plus 1-2 um. If poly extends beyond the N-well over P-substrate, it can form a parasitic PMOS channel beneath the field oxide.

4. **Field plate material:** Use poly (connected to VDD) or lower metal (connected to highest available supply) for field plates. The field plate bias must ensure the underlying silicon surface remains in accumulation, preventing inversion.

#### 6.A.0 Deep-N+ Plug Insertion for BiCMOS Merged Devices

For BiCMOS processes, every NPN transistor in a shared tank must have at least a minimum deep-N+ plug to keep tank resistance below 100 Ohm [Hastings, Ch. 14.1]. Without the deep-N+ plug, tank resistance can exceed 6 kOhm, and at worst-case temperature (150 C), debiasing voltage V_debias = I_device * R_tank can exceed 600 mV -- enough to forward-bias the parasitic PNP and trigger latchup. The cell generator must automatically insert a minimum deep-N+ plug for every NPN transistor placed in a shared tank, even if the PDK design rules do not explicitly require it. This plug overlaps the NBL and connects the collector to a low-resistance path to the supply.

#### 6.A.1 Guard Ring Theory Beyond PDK Latch-up Rules

The spec treats guard rings as purely PDK-driven (latch-up distance rules). In analog design, guard rings serve three distinct purposes, each with its own design rules [Hastings ch14]:

**Purpose 1: Latch-up prevention.**
The latch-up condition is B_NPN * B_PNP >= 1 [Lienig 7.1.3]. Guard rings reduce the parasitic transistor current gains by:
- Collecting minority carriers before they reach the parasitic base (collecting rings)
- Blocking minority carriers with a high-low junction (blocking rings)
- Reducing parasitic base resistance (majority-carrier guard rings / well taps)

**Purpose 2: Substrate noise isolation.**
Single guard ring: 20-40 dB attenuation. Deep N-well isolation: 40-60 dB [00_ANALOG_PRINCIPLES 4.4]. For analog blocks adjacent to digital, this isolation is often the primary motivation for guard rings.

**Purpose 3: WPE displacement.**
Guard rings push well edges away from active devices, mitigating WPE-induced Vth shifts. For exceptional matching, well boundaries should be >= 5 um from active gates [Hastings ch13, Rule 19], or >= 2x well junction depth (whichever is greater).

#### 6.A.2 Four Fundamental Guard Ring Types

Hastings ch14 defines four guard ring types based on the carrier they target and the mechanism they use:

| Type | Carrier | Mechanism | Typical Efficiency | Key Design Rule |
|---|---|---|---|---|
| ECGR (Electron-Collecting) | Electrons | Reverse-biased N-junction collects from substrate | >90% (BiCMOS with NBL); marginal without P+ substrate | Width >= P-epi thickness (no retrograde well) or >= P-well depth (retrograde) |
| EBGR (Electron-Blocking) | Electrons | N+/P- interface field repels electrons | N/A in standard bipolar or CMOS | Requires deep trench or DTI |
| HCGR (Hole-Collecting) | Holes | Reverse-biased P-junction collects from well | Approaches unity | PMoat ring width >= N-well depth |
| HBGR (Hole-Blocking) | Holes | N+/P- interface blocks holes | >95%, often >98% (std bipolar); requires >= 100:1 doping ratio | Must completely encircle the injector; NBL extends to outside edge of deep-N+ |

**Combined HCGR + HBGR achieves > 99% efficiency** [Hastings ch14].

#### 6.A.3 Guard Ring Width Formulas

The spec uses PDK.guard_ring_width as the baseline. Hastings ch14 provides process-dependent width formulas:

**ECGR (NMoat strip) in CMOS** [Hastings ch14]:
```
ECGR_width >= P_epi_thickness      (no retrograde P-well)
ECGR_width >= P_well_depth          (retrograde P-well)
```

**HCGR (PMoat strip) in BiCMOS** [Hastings ch14]:
```
HCGR_width >= N_well_depth
```

**HBGR (deep-N+ ring) in BiCMOS** [Hastings ch14]:
```
HBGR must completely encircle the injector (no gaps).
Drawn NBL must extend to the outside edge of drawn deep-N+.
Drawn N-well should also extend at least to outside edge of drawn deep-N+.
Doping ratio N+/P- >= 100:1 required for >95% efficiency.
```

#### 6.A.4 Guard Ring Completeness Requirement

**A hole-blocking guard ring that does not completely encircle the injector is useless** [Hastings ch14]. The ring must form an unbroken loop with no gaps, even at corners. Interruptions for routing must be routed around, not through, the guard ring.

For electron-collecting guard rings (ECGRs), gaps are more tolerable because collection does not require complete encirclement -- but efficiency degrades linearly with the fraction of the perimeter left unguarded.

#### 6.A.5 Guard Ring Connection Strategy

The connection voltage affects guard ring effectiveness [Hastings ch14]:

- **ECGRs**: Connect to the highest available supply voltage. This drives the depletion region as deep as possible into the substrate. A grounded ECGR functions but is shallower and more susceptible to debiasing.
- **HCGRs**: Connect to ground or a negative supply. Large reverse bias drives the depletion region deeper into the well.
- **HBGRs**: The blocking junction is created by the doping contrast, not by applied bias. Connection to any supply is acceptable, but VDD is conventional.

#### 6.A.6 Deep Trench Isolation for Extreme Analog Isolation

For DTI BiCMOS processes, deep trench + NBL forms a 100%-efficient ECGR [Hastings ch14]. The tilted arsenic implant along trench sidewalls contacts NBL, creating an impenetrable barrier to electrons.

DTI BiCMOS HBGR: Ring of deep trench around N-well edge prevents lateral hole escape. NBL coded across entire tank; drawn NBL edge extends at least to outside drawn edge of deep trench. Observed hole permeation less than 5% with floating P-type region [Hastings ch14].

#### 6.A.7 Critical Warning on P- Substrates

Designs on P- substrate (without P+ substrate) are far more susceptible to latchup [Hastings ch14]:
- The substrate is easily debiased by minority carrier injection.
- ECGRs become far less effective without the P+/P- interface.
- No amount of guard rings guarantees operation under severe inductive kickback.
- A significant percentage of such designs require additional latchup-fix passes.

The cell generator should flag devices on P- substrate processes and recommend conservative guard ring sizing (2x normal width, half-normal tap spacing).

#### 6.A.8 Well-Tap Spacing for Analog: Quantitative Rules

The spec says "PDK.latchup_max_distance / 2" for default tap spacing. This is the latch-up compliance minimum. For analog, additional requirements apply:

**For substrate noise isolation** [00_ANALOG_PRINCIPLES 4.4]:
- Substrate contact resistance to nearest tap determines local ground rise: V_debiasing = R_sub * I_sub [Lienig 7.1.1].
- Use dedicated "SUB" net with star routing topology, separate from circuit ground [Lienig 6.2.1].
- Abundant substrate contacts with minimal track resistance to nearest contact.

**For WPE displacement** [Hastings ch13, Rule 19]:
- Well boundaries >= 5 um from matched transistors (exceptional).
- Well boundaries >= 2x well junction depth (whichever is greater).
- Guard rings inherently push well edges away from active devices; wider rings provide more WPE clearance.

### 6.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| Guard ring completeness | HBGR rings must form an unbroken loop with zero gaps | [Hastings ch14] |
| Width formula | Ring width computed from process stack parameters (epi thickness, well depth), not just PDK.min | [Hastings ch14] |
| ECGR connection | ECGRs default to highest supply; algorithm must accept a supply_net parameter | [Hastings ch14] |
| WPE clearance | ring_to_device_spacing must be >= max(PDK.min, 5 um) for exceptional matching | [Hastings ch13, Rule 19] |
| P- substrate flag | If process uses P- substrate (no P+ bulk), emit a warning and double guard ring width | [Hastings ch14] |
| Deep trench support | If PDK offers DTI, cell generator must be capable of inserting trench rings instead of junction-only rings | [Hastings ch14] |
| Substrate contact net | Well taps must connect to the analog-specific substrate net (SUB), not circuit ground (GND) | [Lienig 6.2.1, 7.1.1] |

---

## 7. Sub-Block: Passive Array Generator

### 7.1 Capacitor Arrays (for DACs, SC filters)

```
Input: capacitor_ratios (e.g., 1:2:4:8:16 for 5-bit binary DAC)
       unit_capacitance, PDK.cap_type (MIM, MOM, MOSCAP)
Output: 2-D common-centroid array with dummies and routing

Algorithm:
1. Decompose each ratio into unit capacitors.
   Total units = sum of all ratios + dummy units.

2. Select array dimensions (rows * cols) to minimize aspect ratio
   while accommodating all units + perimeter dummies.

3. Place units using spiral or block-chessboard algorithm:
   - Spiral: units of each capacitor placed in an outward-spiraling
     pattern from the center. Minimizes centroid error.
   - Block chessboard: partition array into blocks, interleave
     blocks of each capacitor. Better for large arrays.

4. Insert dummy units around the perimeter (at least 1 ring).
   Connect dummy bottom plates to ground.

5. Route unit connections:
   - Top plate (sensitive node): shield with ground metal above/below.
   - Bottom plate: connect to shared bus.
   - Routing parasitics contribute to mismatch -- co-optimize
     placement and routing to balance via count vs. dispersion.

6. Report to S2: array bounding box, pin locations for each
   capacitor's top and bottom plates.
```

### 7.2 Resistor Arrays (R-2R, trim networks)

```
1. Build from identical unit resistors (serpentine segments).
2. 1-D interdigitation: alternate segments of matched resistors.
3. Even number of segments per resistor (Hastings) so both
   terminals exit the same side -> reduces thermoelectric EMF.
4. Dummy serpentine at each end of the array.
5. For precision: Kelvin (4-terminal) connections --
   separate force (current) and sense (voltage) leads.
6. Wide metal straps for current-carrying connections; thin
   sense leads for voltage measurement.
```

### 7.A Analog Considerations

#### 7.A.0 NBL Shadow Effect on Resistor Matching (BiCMOS / LOCOS Processes)

For BiCMOS processes without STI (using LOCOS), the cell generator must verify that the N-buried layer (NBL) shadow does not intersect matched resistor segments. During epitaxial growth, surface discontinuities from patterned NBL shift laterally: pattern shift on (111) wafers is 50-150% of epi thickness; pattern distortion on (100) wafers is smaller but non-negligible [Hastings, Ch. 8, Section 8.2.5]. If the NBL shadow intersects a matched resistor, it alters the resistor value due to the different doping profile in the shadowed region. HSR resistors are especially vulnerable because their higher sheet resistance amplifies the doping perturbation.

**Mitigation:** Increase NBL overlap on the pattern-shift side by at least the epi thickness plus alignment tolerance. STI/CMP processes are immune to this effect because they do not use epitaxial overgrowth.

**Cell generator rule:** For BiCMOS processes using LOCOS isolation, the cell generator must compute the expected NBL shadow boundary (epi_thickness * pattern_shift_factor + alignment_tolerance) and verify that no matched resistor segment falls within this boundary. If a segment is at risk, the cell generator must either relocate the segment or extend the NBL to encompass the entire resistor array.

#### 7.A.1 Capacitor Matching: The 13 Rules Applied to Cell Generation

Hastings ch8 provides 13 rules for capacitor matching. The following are the most critical for the cell generator (rules not already covered in Section 5.A):

**Rule 1 -- Identical unit capacitor geometries.** Use arrays of identical unit caps connected in parallel. For integer ratios (e.g., 1:2:4:8), use proportional numbers of unit capacitors. Avoid series connections (top/bottom plate parasitic asymmetry) [Hastings ch8].

**Rule 2 -- Square geometries.** Squares minimize periphery-to-area ratio, reducing peripheral effects. Rectangles up to approximately 3:1 are acceptable for moderate matching. Exceptional: always square [Hastings ch8].

**Rule 3 -- Sufficient area.** For a poly-poly capacitor with A_C approximately 0.5%*um: +/-0.1% pair needs approximately 250 um^2; +/-0.01% needs approximately 25,000 um^2. Optimal unit capacitor size: 25-100 um per side [Hastings ch8].

**Rule 6 -- Parasitic capacitance management.** Connect lower plate to low-impedance node. Place a well/NBL beneath capacitor arrays for substrate isolation [Hastings ch8].

**Rule 8 -- Electrostatic shielding.** All moderate and exceptional caps must be shielded. Shield overhang >= 50 um if no dummies; >= 5 um with dummies [Hastings ch8].

**Rule 10 -- Match lead capacitance.** Use jogs or dead-end branches to equalize interconnect parasitic capacitance. Lead-to-adjacent-metal spacing >= 2-3x interlayer oxide thickness [Hastings ch8].

**Rule 11 -- Thick homogeneous dielectrics.** Grown/LPCVD oxide preferred over TEOS or ONO for charge-redistribution converters (lower dielectric relaxation) [Hastings ch8].

#### 7.A.2 Capacitor Type Ranking

Capacitor types ranked by matching potential (best to worst) [Hastings ch8]:

1. **Poly-metal with silicided lower electrode + thick LPCVD oxide** -- metallic electrodes eliminate poly depletion; thick oxide reduces random mismatch
2. **MIM (metal-insulator-metal)** -- negligible voltage and temperature variation; the gold standard for precision analog [Hastings ch7]
3. **Poly-poly** -- good; eliminates MOS body parasitics/leakage, but upper electrode depletion limits accuracy
4. **MOS (accumulated/inverted)** -- moderate; poly depletion + parasitic capacitance + junction leakage; must operate deep in accumulation or inversion with >= 0.5-1 V overdrive [Hastings ch7]
5. **Junction capacitors** -- unsuitable for matching; large temperature-dependent depletion variations

#### 7.A.3 Resistor Matching: Hastings' 25 Rules Applied to Cell Generation

The most critical resistor matching rules for the cell generator [Hastings ch8]:

**Rule 1 -- Same material.** Never match poly to diffused. Material ranking: thin-film (nichrome, sichrome) > polysilicon > diffused [Hastings ch8].

**Rule 2 -- Same width.** Matched resistors must always have the same width [Hastings ch8], [Lienig 6.6.1]. Width biases from edge shifts (typically approximately 50 nm for poly) cause systematic mismatch. For a ratio R1:R2, build from identical unit resistors of equal width AND length, connected in series [Lienig 6.6.1].

**Rule 3a -- Conductivity modulation protection for high-sheet resistors.** For high-sheet resistors (Rsh > 200 Ohm/sq), ensure identical body/tank bias on all matched segments. For Rsh > 1 kOhm/sq, generate an electrostatic field plate (grounded metal layer over the resistor body); for exceptional matching (< 1% target), use split field plates where each section connects to the local resistor potential to prevent lateral field gradients across the resistor. See `00_ANALOG_PRINCIPLES.md` Section 2.5 for the full conductivity modulation model [Hastings ch8, Section 8.2.9].

**Rule 4 -- Sufficient width.** >= 150% of min linewidth (minimal), >= 200% (moderate), >= 400% (exceptional). Poly width >= 0.5 um to avoid the bamboo effect (single grain spanning the width) [Hastings ch6, ch8]. Diffused resistor width >= 2x junction depth to avoid dilution [Hastings ch6].

**Rule 10 -- No excessively short segments.** Min length: 3x design-rule minimum (minimal), 5x (moderate), 10x (exceptional). Poly segment total length >= 1000x grain diameter (>= 50-100 um) to average grain boundary effects [Hastings ch8].

**Rule 11 -- Cancel thermoelectrics.** Even number of series segments; half in each current direction [Hastings ch8], [Lienig 6.6.5]. A temperature difference of 2K with Seebeck coefficient = 50 uV/K produces 0.1 mV -- enough for 0.4% mismatch in a bipolar current mirror [Hastings ch8].

**Rule 16 -- Serpentines only for minimal matching.** Moderate matching requires arrayed segments (even dogbone segments). Serpentine resistors cannot be properly interdigitated [Hastings ch8].

#### 7.A.4 Kelvin Connections for Precision Resistors

For exceptional resistor matching, Kelvin (4-terminal) connections are essential [Hastings ch6]:

- **Separate force (current) and sense (voltage) leads.** The force leads carry the full operating current; the sense leads carry only the tiny measurement current.
- **Single-level-metal:** Sense taps connect to the side of the resistor. Force leads should extend >= 2W beyond the star connection before any bends [Hastings ch6].
- **Double-level-metal:** Sense leads on upper metal tap into the center, reducing sensitivity to nonuniform current flow [Hastings ch6].
- **Metal IR drops:** Metallization voltage drops create systematic errors in matched ratios. Metal and via resistance contributions should scale proportionally to desired resistance ratios [Hastings ch8, Rule 23].

#### 7.A.5 Resistor Head Correction for Ratios

When matched resistors have different numbers of segments (to create a ratio), the head resistance R_H must be accounted for [Lienig 6.6.1]:

```
R_total = R_sq * (l / w) + 2 * R_H(w)
```

For a required ratio r = R1/R2 > 1 with equal widths, the lengths are related by [Lienig 6.6.1]:

```
l_1 = r * l_2 + l_corr
l_corr = 2 * (r - 1) * (R_H / R_sq) * w
```

However, R_H and R_sq values are nominal; process tolerances cause residual error. The preferred solution is to split resistors into identical "basic resistors" of the same width AND length, then connect them in series to achieve the desired ratio [Lienig 6.6.1].

#### 7.A.6 Voltage-Dependent Poly Spacing

For multi-segment poly resistors (serpentines), the maximum voltage between adjacent segments is [Hastings ch6]:

```
V_seg = 2V / N
```

where V is the total voltage across the resistor and N is the number of segments. Modern processes implement voltage-dependent poly spacing rules to avoid TDDB [Hastings ch6].

#### 7.A.7 Power Dissipation in Matched Resistors

For exceptional matching, total power dissipation in the matched resistor array must not exceed approximately 1 mW [Hastings ch8, Rule 22]. Self-heating creates thermal gradients that are indistinguishable from process gradients but are bias-dependent.

Poly resistors have poor thermal conductivity through their enclosing oxide (k_ox approximately 1.4 W/(m*K)). Self-heating-induced voltage modulation coefficient [Hastings ch6]:

```
beta_V = (alpha * R_s) / (R * W^2) * (t_ox / k_ox)
```

where alpha is TCR, R_s is sheet resistance, t_ox is oxide thickness beneath resistor.

### 7.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| Unit capacitor geometry | All unit caps must be identical squares (exceptional) or rectangles with aspect ratio <= 3:1 (moderate) | [Hastings ch8, Rules 1-2] |
| Electrostatic shield | Moderate/exceptional capacitor arrays must include a grounded shield metal layer | [Hastings ch8, Rule 8] |
| Lower plate assignment | Lower plate of matched caps connects to lowest-impedance node | [Hastings ch8, Rule 6] |
| Resistor segment identity | All matched resistor segments must have identical width AND length; ratios via segment count | [Hastings ch8, Rule 5], [Lienig 6.6.1] |
| Thermoelectric cancellation | Even number of segments per resistor; half in each current direction | [Hastings ch8, Rule 11] |
| Kelvin contacts | Exceptional resistors must have 4-terminal connections | [Hastings ch8, Rule 23] |
| Power budget | Total resistor array power <= 1 mW for exceptional matching | [Hastings ch8, Rule 22] |
| Poly spacing | Adjacent poly segments spaced per voltage-dependent rules: V_seg = 2V/N | [Hastings ch6] |

---

## 8. Sub-Block: Intra-Cell Routing

Each cell needs internal metal connections that are part of the cell, not S3's responsibility:

### 8.1 Source/Drain Strapping
- Connect all source fingers and all drain fingers with metal straps.
- For high-current devices: use 3-4 parallel metal layers for EM.
- Strap width: sized for EM based on max DC + AC current per finger.

### 8.2 Gate Poly Connection
- For single-side-contacted: connect all gate fingers on one side.
- For double-side-contacted: connect on both sides with a bus.
- Gate bus resistance adds to R_g; verify R_g target is met.

### 8.3 Via Stacking
- Stack vias from poly/OD up to the routing layer (typically M1-M3).
- Multi-cut vias for EM on high-current paths.
- Via count per connection: sized for EM, with at least 2 cuts for redundancy.

### 8.4 Shield Metal
- For capacitor top plates: place grounded metal above and below.
- For high-impedance nodes: optional M1 ground shield under the signal metal.

### 8.A Analog Considerations

#### 8.A.1 Metal Over Active Gates -- The Hydrogenation Hazard

The spec does not address metal routing restrictions over active gate regions. This is one of the most insidious matching hazards in modern processes [Hastings ch13]:

**Physical mechanism:** Metal layers (Al, Cu) block hydrogen diffusion during passivation anneal. Ti-silicide and TiW barrier metals actively getter hydrogen. Transistors beneath metal receive less hydrogen passivation and have more random Vth variation [Hastings ch13, 00_ANALOG_PRINCIPLES 2.4].

**Quantitative impact:**
- Up to 20% systematic Id mismatch between metal-covered and uncovered transistors.
- Up to 1% mismatch from different metal fill patterns in vicinity.
- Effects extend >= 10 um from metal edges.

**Intra-cell routing rules for matched devices:**
1. Do NOT place internal metal straps directly above active gate regions of matched transistors.
2. Route S/D straps and gate buses to the sides of the active area, not over it.
3. If metal must cross an active gate (unavoidable for certain dense layouts), cover BOTH matched devices' gates with identical metal patterns [Hastings ch13].
4. Block dummy metal generation over matched transistors using pseudolayers. For exceptional matching, the blocked region extends >= 5 um beyond the active gate area in all directions [Hastings ch13, Rule 18].

#### 8.A.2 Gate Connection: Metal Straps Required for Matching

For moderate and exceptional matching, gate fingers must be connected by metal straps, not by poly interconnect [Hastings ch13, Rule 22]. Reasons:
- Poly-only gate connections create etch-rate variations due to different poly geometries near each gate finger.
- Metal straps provide a uniform, low-resistance connection that does not perturb the poly etch environment.

**Gate bus routing:** The metal gate strap should run parallel to (but not over) the active gate region. Contact from poly gate to metal should be placed at the end(s) of each gate finger, not over the active channel.

#### 8.A.3 No Contacts on Active Gate Regions

Even if PDK design rules allow placing contacts over active gate regions, matched transistors must NOT have contacts placed on their active gate areas [Hastings ch13, Rule 16]. The contact process damages the gate oxide and creates localized Vth shifts.

#### 8.A.4 EM Sizing for Intra-Cell Straps

Intra-cell straps that are shorter than the Blech length are "immortal" and can be granted EM waivers [00_ANALOG_PRINCIPLES 4.1]:

```
J_critical * L_strap < (J*L)_Blech
(J*L)_Blech ~ 2000-5000 A/cm for Cu (process-dependent)
```

For straps above the Blech length, width must satisfy:
```
width >= I_max_DC / J_max_DC
J_max_DC ~ 1 mA/um (Al) or ~ 5 mA/um (Cu)
```

Via count per connection:
```
N_vias >= I_max_DC / J_max_per_via
J_max_per_via ~ 0.1-0.5 mA/cut (process-specific)
```

Always use at least 2 via cuts for redundancy (yield).

#### 8.A.5 Matched Net Routing Within Cells

When a cell contains multiple matched devices (e.g., a CC pair), internal routing between devices must be symmetric:
- Route matched nets on the same metal layer(s).
- Use identical via stacks for each device's connections.
- Match total wire length to within approximately 5% (moderate) or approximately 1% (exceptional) [00_ANALOG_PRINCIPLES 3.4].
- Shield high-impedance matched nets with ground/supply metal on adjacent layers [00_ANALOG_PRINCIPLES 3.4].

### 8.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| No metal over active gates | Intra-cell metal routing must avoid crossing active gate regions of matched devices | [Hastings ch13, Rule 17] |
| Metal gate straps | Gate fingers connected by metal, not poly, for moderate/exceptional matching | [Hastings ch13, Rule 22] |
| No contacts on active gates | Contact-to-active-gate overlap prohibited for matched devices | [Hastings ch13, Rule 16] |
| Hydrogenation zone | Block dummy metal generation in the hydrogenation zone rectangle | [Hastings ch13, Rule 18] |
| EM sizing | Strap width computed from I_max and J_max; multi-cut vias >= 2 cuts | [00_ANALOG_PRINCIPLES 4.1] |
| Matched net symmetry | Internal routing for matched devices symmetric in R and C | [00_ANALOG_PRINCIPLES 3.4] |

---

## 9. Orientation Rules

```
1. All devices in a matching group MUST have identical orientation.
   No rotation, no mirroring of the base device.

2. Default orientation: gates parallel to the Y-axis (poly runs
   vertically), current flows horizontally (source-to-drain in X).
   This is the conventional <110> channel direction on (100) wafers.

3. For PMOS: if the PDK supports <100>-oriented channels for
   PMOS (45-degree rotated wells), verify with the PDK team. Most
   standard processes use <110> for both NMOS and PMOS.

4. Record orientation in cell metadata so S2 can verify
   all matched devices share orientation after placement.
   S2's device-flipping must NOT rotate matched devices.

5. Non-matched devices (biasing, digital logic): orientation is
   flexible. Prefer the default orientation for uniformity.
```

### 9.A Analog Considerations

#### 9.A.1 Orientation Mismatch Is the #1 Systematic Error

Orientation mismatch ranks as the single largest systematic mismatch mechanism, capable of producing approximately 15% gm error (several percent Id mismatch) [00_ANALOG_PRINCIPLES Table 1.3, Rank 1]. This exceeds even WPE and LOD effects.

**Why it happens:** Carrier mobility in silicon is anisotropic under mechanical stress. The piezoresistance coefficients differ between the <110> and <100> crystal directions [Hastings ch8, ch13]:

- NMOS along <110>: pi_L = 30 * 10^-11 Pa^-1, pi_T = -17 * 10^-11 Pa^-1
- PMOS along <110>: pi_L = -65 * 10^-11 Pa^-1, pi_T = 40 * 10^-11 Pa^-1

A device rotated 90 degrees swaps its longitudinal and transverse stress responses, creating a first-order mismatch that no amount of common-centroid layout can fix.

**Editing errors are the most common cause** [Hastings ch13]: A layout designer rotating a cell containing one of two matched transistors silently introduces orientation mismatch. The fix: always group matched devices in the same cell.

#### 9.A.1a Directional-Implant 90-Degree Rotation Prohibition

In directional-implant processes (where pocket/halo implants are applied at a tilted angle, typically horizontal), the rotation prohibition is absolute: 90-degree rotation swaps digital and analog transistor orientations and causes catastrophic matching degradation [Hastings ch12, Section 12.2.7]. Only 0-degree and 180-degree rotations are permitted for matched analog cells. Blocks can be reflected (mirror about the symmetry axis) but NEVER rotated by 90 degrees. The cell generator must enforce this constraint and S2's placement engine must be informed that 90-degree rotation is forbidden for all cells in matching groups when `PDK.pocket_implant_direction` is set.

#### 9.A.2 The Orientation Chi Metric

For multi-finger transistors, orientation is quantified by the chi metric [Hastings ch13]:

```
chi = (1/N_f) * sum(chi_i)
```

where chi_i = +1 if current flows right in finger i, -1 if current flows left. Two matched devices must have equal chi. Zero orientation (equal left/right fingers) is the ideal.

For 2D arrays, both horizontal orientation chi_H and vertical orientation chi_V must match between paired devices.

#### 9.A.3 Non-Self-Aligned Device Orientation

For non-self-aligned devices (e.g., asymmetric drain-extended NMOS), matched transistors must be **superimposable** -- achievable by mere translation, without rotation or reflection. Mirror-image devices have opposite sensitivity to photolithographic misalignment [Hastings ch13].

#### 9.A.4 PMOS 45-Degree Orientation for Exceptional Matching

PMOS has minimum piezoresistive sensitivity when channels run along <100>, which is 45 degrees to the wafer flat on (100) silicon [Hastings ch13, Rule 24]. This orientation reduces stress-induced gm mismatch by a large factor but is only practical if:
- The PDK permits 45-degree geometry (some processes have directional implant restrictions).
- The rest of the layout can accommodate non-Manhattan geometry.

For most designs, the standard <110> orientation with common-centroid layout provides sufficient stress cancellation.

#### 9.A.5 BJT Orientation Considerations

**Vertical NPN on (100) Si:** The piezojunction coefficient pi_T = -43.4 * 10^-12 Pa^-1 is large [Hastings ch10]. For bandgap references, consider using vertical PNP instead -- pi_T(PNP) = -13.3 * 10^-12 Pa^-1, approximately 3x less stress-sensitive than NPN [Hastings ch10].

**Lateral PNP:** The piezojunction coefficient pi_R = 11.6 * 10^-12 Pa^-1 is relatively small on (100) Si [Hastings ch10]. Circular/annular emitter geometries have radially symmetric stress response, inherently canceling orientation effects.

#### 9.A.6 Resistor Orientation and Piezoresistivity

Resistor orientation affects piezoresistive sensitivity [Hastings ch8]:

| Material | Minimum piezoresistivity direction on (100) Si |
|---|---|
| N-type mono-Si | <110> (horizontal/vertical) |
| P-type mono-Si | <100> (45 deg to wafer flat) -- impractical |
| Polysilicon | Orientation-independent (random grain structure) |
| Thin-film (nichrome) | Essentially zero piezoresistivity |

**L-shaped segments** with equal horizontal and vertical lengths can cancel piezoresistivity effects [Hastings ch8]. A practical implementation uses two identical common-centroid arrays oriented perpendicularly.

### 9.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| Orientation recording | Every cell must record its orientation as a chi value and a crystal direction tag | [Hastings ch13] |
| No rotation of matched devices | S2 must be informed that matched cells cannot be rotated or mirrored | [Hastings ch13, Rule 7] |
| Chi equality | Matched devices must have identical chi (value and sign) | [Hastings ch13, Table 13.2, Rule 5] |
| PMOS 45-deg option | Cell generator must support 45-degree orientation for PMOS if PDK allows and matching tier is exceptional | [Hastings ch13, Rule 24] |
| Same-cell grouping | Matched devices should be generated within the same cell to prevent accidental rotation during S2 placement | [Hastings ch13] |

---

## 10. FinFET / GAA Adaptations

At advanced nodes, S0 must change fundamentally:

```
1. Width quantization: W = N_fins * fin_pitch (FinFET)
   or W = N_sheets * sheet_width (GAA/nanosheet).
   No continuous W tuning -- matching by equal unit-cell count.

2. Channel length: only a few legal L values.
   For longer effective L: use series stacks of minimum-L devices.
   Each device in the stack must have identical context (dummies).

3. Restricted placement grid: devices placed on a fixed
   fin/poly grid. Use LAYGO-style template placement.

4. Dummy fins and dummy poly: mandatory at array edges.
   Dummy fin count: typically 2-4 per edge (PDK-specific).
   Dummy poly (gate): at least 1 per edge.

5. Self-heating: FinFETs and GAA have poor thermal conductivity
   through the fin/sheet. For high-power devices, increase
   fin-to-fin spacing or add thermal vias to the substrate.

6. STI/LOD effects are amplified: the narrow fin geometry
   concentrates stress. Context-identical placement is mandatory.

7. Cell generation should use the LAYGO2/BAG2 template-and-grid
   approach: hand-design one template per device type x PDK,
   then instantiate parameterically.
```

### 10.A Analog Considerations

#### 10.A.1 Width Quantization and Pelgrom

In FinFET/GAA, width is quantized to integer multiples of fin_pitch (or sheet count). This has profound implications for Pelgrom-driven sizing:

- The Pelgrom equation sigma(dVth) = A_VT / sqrt(W * L) still applies, but W can only take discrete values.
- Matching can only be achieved by equal unit-cell count, not by continuous W tuning [Lienig 6.6.1].
- For a 1:2 current mirror, the mirror transistor must have exactly 2x the fin count of the reference. Non-integer ratios require creative circuit techniques (e.g., source degeneration).

#### 10.A.2 Amplified LOD/STI Effects in FinFETs

The narrow fin geometry concentrates STI stress effects. Even small variations in SA/SB produce larger transconductance shifts than in planar devices. This makes dummy fins mandatory (not optional as in some planar processes) and demands that every active fin has identical context on all sides.

#### 10.A.3 Self-Heating and Matched Devices

FinFET and GAA devices have poor thermal conductivity through the fin/sheet structure. Self-heating creates local temperature gradients that can degrade matching. For matched devices:
- Keep power dissipation per device low (sub-mW for precision).
- Use common-centroid layout to cancel thermal gradients from self-heating.
- Consider adding thermal vias between the device and the bulk substrate.

### 10.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| Quantized W | W must be rounded to the nearest legal fin/sheet count; report actual W_eff | PDK-specific |
| Mandatory dummies | Dummy fins (2-4 per edge) and dummy poly (>= 1 per edge) are always required, regardless of matching tier | PDK-specific, amplified by [Hastings ch13] LOD principles |
| Context identity | Every active fin must have identical surrounding context (same number of dummy fins, same spacing) | Analog extension of [Hastings ch13, Rule 12] |
| Self-heating flag | For devices dissipating > 0.5 mW, flag potential self-heating matching degradation | [00_ANALOG_PRINCIPLES 4.3] |

---

## 11. Interface to S1 (Constraint Feedback)

S0 and S1 have a bidirectional dependency:

- **S1 -> S0:** S1 tells S0 which devices are in matching groups and what pattern is recommended (CC, interdigitated, clustered). S1 also provides sensitivity analysis results that drive Pelgrom sizing.

- **S0 -> S1:** S0 reports back the actual cells generated: finger count, pattern used, dummy count, guard ring size, pin locations. S1 uses this to refine constraints (e.g., if S0 couldn't achieve the requested CC pattern due to area, S1 relaxes the matching tolerance).

In practice, S0 runs a first pass with default assumptions, S1 extracts constraints from the netlist, and S0 re-runs for devices where S1's constraints changed the pattern selection. This typically converges in 1-2 iterations.

### 11.A Analog Considerations

The S0-S1 interface must also communicate the analog-specific metadata defined in Section 2.A:

- SA/SB per finger (for LOD verification by S2)
- WPE distance vector (for WPE verification by S2)
- Orientation chi (for orientation verification by S2)
- Hydrogenation zone (for S3 dummy metal blocking)
- Matching tier (minimal/moderate/exceptional) -- this drives dummy count, guard ring width, and routing constraints

Additionally, S1 should pass to S0:
- **Matching type** (voltage or current) -- this determines whether Pelgrom sizing scales W*L or primarily L [Hastings ch13, Rules 2-3].
- **Operating point information** (Vgs-Vth, Id) -- needed to evaluate whether pocket-implant devices are appropriate and whether subthreshold operation is flagged [Hastings ch13, Rules 4-5].
- **Power dissipation per device** -- needed for self-heating analysis and matched resistor power budgets [Hastings ch8, Rule 22].

### 11.B Analog-Algorithm Interface

| Contract Item | Constraint | Source |
|---|---|---|
| Extended metadata | S0 must export all fields from Section 2.A to S1 | This spec |
| Matching type | S1 must specify voltage vs. current matching; S0 uses this for sizing | [Hastings ch13, Rules 2-3] |
| Operating point | S1 provides Vgs-Vth and Id; S0 checks subthreshold and pocket-implant constraints | [Hastings ch13, Rules 4-5] |
| Power budget | S1 provides per-device power; S0 checks against self-heating limits | [Hastings ch8, Rule 22] |
| Convergence criterion | S0 and S1 iterate until all matching, dummy, guard ring, and orientation metadata are consistent | This spec |

---

## 12. Validation Checklist (per cell)

Before a cell exits S0:

- [ ] DRC clean (run PDK DRC on the cell in isolation, including guard ring inflation). Note: DRC clean applies to the cell including its guard ring geometry. S2 must verify inter-cell DRC after placement, as guard ring interactions between adjacent cells (e.g., minimum spacing violations between neighboring guard rings) are not covered by per-cell DRC.
- [ ] All terminals accessible (pins on routing layers)
- [ ] Gate resistance meets target (R_g < threshold)
- [ ] EM: all internal straps sized for max current
- [ ] Matched devices: identical orientation (chi values equal)
- [ ] Matched devices: identical finger count and width
- [ ] Matched devices: identical SA/SB per finger (LOD equalized)
- [ ] Matched devices: WPE distances symmetric
- [ ] Dummies present and connected (tier-appropriate count)
- [ ] Guard ring present and connected (completeness verified for HBGR)
- [ ] Well taps within latchup distance
- [ ] Bounding box includes guard ring inflation
- [ ] Metadata record complete (all fields from Section 2.A)
- [ ] No metal over active gates of matched devices (or identical metal pattern on all)
- [ ] Dummy metal generation blocked in hydrogenation zone
- [ ] No contacts on active gate regions of matched devices
- [ ] Gate connections use metal straps (moderate/exceptional)
- [ ] Moat extends >= 5 um beyond last active gate (exceptional matching)
- [ ] Resistor arrays: even segment count, thermoelectric cancellation verified
- [ ] Capacitor arrays: electrostatic shield present (moderate/exceptional)

### 12.A Analog Considerations

The above checklist expands the original 11-item list to 20 items. The nine new items (marked by their analog-specific content) directly prevent the failure modes identified in 00_ANALOG_PRINCIPLES:

| New Check | Failure Mode Prevented | Typical Impact if Omitted |
|---|---|---|
| SA/SB equalization | LOD/STI stress mismatch | Up to 13% NMOS Id error [00_ANALOG_PRINCIPLES 2.2] |
| WPE symmetry | Well proximity Vth shift | 10-50 mV Vth mismatch [00_ANALOG_PRINCIPLES 2.1] |
| No metal over active gates | Hydrogenation blocking | Up to 20% Id mismatch [00_ANALOG_PRINCIPLES 2.4] |
| Dummy metal blocking | Asymmetric hydrogen exposure | Up to 1% mismatch from fill patterns [Hastings ch13] |
| Metal gate straps | Poly etch-rate variation | Systematic L mismatch [Hastings ch13, Rule 22] |
| No contacts on active gates | Gate oxide damage | Vth shift [Hastings ch13, Rule 16] |
| Moat extension | End-device LOD asymmetry | Several percent Id mismatch [Hastings ch13] |
| Thermoelectric cancellation | Seebeck EMF in resistors | 0.1-1 mV offset [Hastings ch8, Rule 11] |
| Electrostatic shield | Capacitor fringing/coupling | Unpredictable mismatch [Hastings ch8, Rule 8] |

### 12.B Analog-Algorithm Interface

The validation checklist serves as the final gate before a cell is released to S2. The algorithm must:

1. Run all 20 checks automatically.
2. For any failure, classify it as BLOCKING (cell cannot be used) or WARNING (cell can be used with degraded matching).
3. BLOCKING failures: identical orientation, DRC clean, terminal accessibility, guard ring completeness (HBGR).
4. WARNING failures: SA/SB deviation > 5%, WPE asymmetry > 10%, metal over gates, insufficient dummy count for the declared matching tier.
5. Report all failures and warnings in the cell metadata so S1 can decide whether to accept the cell or request re-generation with relaxed constraints.

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| GAP-01 | Added `bias_current_mA` field to Section 2.A metadata | S0 did not export per-device DC bias current (Id); S3 needs it for EM wire sizing and S1 for parasitic budget computation |
| GAP-02 | Added `overdrive_voltage_mV` field to Section 2.A metadata | S0 did not export per-device overdrive voltage (Vgs-Vth); S1 needs it for Pelgrom current mismatch calculation and subthreshold guard |
| COV-03 | Added CDM proximity constraint (max 50 um) note; referenced in master architecture S0.A section | ESD CDM secondary protection placement proximity was not explicitly constrained in the cell generation spec |
| CONTRA-03 | Clarified DRC clean scope in Section 12 validation checklist to include guard ring inflation and note that inter-cell DRC is S2's responsibility | Per-cell DRC clean could miss guard ring spacing violations between adjacent cells after placement |
| MISSING-01 | Added Rule 3a (conductivity modulation protection) to Section 7.A.3 for high-sheet resistors | Conductivity modulation for Rsh > 200 Ohm/sq was not addressed; can cause 0.1%/V systematic mismatch |
| MISSING-02 | Added Section 7.A.0 covering NBL shadow effect on resistor matching for BiCMOS/LOCOS processes | NBL shadow can alter matched resistor values in non-STI processes; was completely unaddressed |
| MISSING-04 | Added Section 6.A.0 requiring deep-N+ plug insertion for BiCMOS NPN transistors in shared tanks | Without deep-N+ plug, tank resistance can exceed 6 kOhm causing latchup at worst-case temperature |
| MISSING-06 | Added directional tilted pocket implant constraint to Section 3.A.3 (90-degree rotation prohibition) and Section 9.A.1a | Directional-implant processes require that matched analog cells never be rotated 90 degrees; catastrophic matching failure otherwise |
| MISSING-11 | Added Section 6.A.00 for parasitic channel prevention via channel stops, field plates, and poly retraction | High-voltage analog without field plates or channel stops can form parasitic channels causing device malfunction |
