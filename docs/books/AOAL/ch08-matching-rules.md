---
title: "8.3 Rules for Device Matching"
chapter: 8
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-8, matching, resistors, capacitors, common-centroid, piezoresistivity, electrostatic-shielding]
---

# 8.3 Rules for Device Matching

> **Chapter 8: Matching of Resistors and Capacitors**

## Key Concepts

This section culminates Chapter 8 by translating all the mismatch mechanisms discussed in [[ch08-mismatch-causes]] into actionable layout rules. Hastings defines three tiers of matching accuracy:

- **Minimal matching**: $\pm 0.1\%$ to $\pm 1\%$ (six-sigma, 10-year life, $\leq 150\,^\circ\text{C}$ junction temperatures, conventional plastic packaging). Easily achieved; sufficient for general-purpose analog circuits.
- **Moderate matching**: $\pm 0.01\%$ to $\pm 0.1\%$. Covers nearly all applications. Requires careful layout of fairly large devices.
- **Exceptional matching**: $\pm 0.001\%$ to $\pm 0.01\%$. Demanded only by highly specialized circuits such as precision data converters. Requires very large area, deposited capacitors or thin-film resistors, and meticulous attention to every layout detail.

The overarching philosophy is that quantitative matching data is rarely available for a specific process, so designers must rely on **qualitative rules** that are broadly applicable. The rules address every mismatch mechanism covered earlier in the chapter: random statistical variation (Pelgrom's law), gradients (thermal, oxide thickness, stress), thermoelectrics, mechanical stress and package shift, conductivity modulation, charge spreading, dielectric absorption, and parasitic effects.

### Why Matching Matters at These Levels

Hastings points out that $\pm 0.01\%$ accuracy is genuinely exceptional. Even the best laboratory multimeters (e.g., Keysight 3458A) achieve only $\sim 8\,\text{ppm}$ ($0.0008\%$) annual accuracy with special options. An integrated resistor or capacitor achieving comparable accuracy without trimming is a remarkable engineering feat.

## Important Details

### 8.2.7-8.2.9: Remaining Mismatch Sources (Context for the Rules)

Before presenting the rules, the text concludes the discussion of mismatch mechanisms that feed into them:

#### Thermoelectric Effects (8.2.7 continued)

When two dissimilar materials contact each other (e.g., aluminum-to-silicon), a **contact potential** appears that depends on temperature. If the two ends of a resistor are at different temperatures, a net Seebeck voltage appears:

$$\Delta V = \alpha_S \cdot \Delta T$$

where $\alpha_S$ is the Seebeck coefficient (typically $50$--$500\,\mu\text{V/K}$ for Al-Si contacts) and $\Delta T$ is the temperature difference. A $\Delta T$ of $2\,\text{K}$ with $\alpha_S = 50\,\mu\text{V/K}$ produces $0.1\,\text{mV}$, enough to cause $0.4\%$ mismatch in a bipolar current mirror.

**Key layout countermeasures:**
- Use an **even number** of series-connected segments, with half carrying current in each direction, to cancel thermoelectric potentials (Figure 8.22B).
- Place the two contacts of a serpentine resistor **close together**. Orient the heads in **opposite directions** to simultaneously cancel mask misalignment (Figure 8.23C).
- Common-centroid layout alone does **not** eliminate thermoelectrics because they arise from temperature differences across individual segments.

#### Mechanical Stress and Package Shift (8.2.8)

Silicon is **piezoresistive**: its resistivity changes under mechanical stress. The stress at any point in the die plane is characterized by three components: two normal stresses ($\sigma_{xx}$, $\sigma_{yy}$) and a shear stress ($\tau_{xy}$). These can be rotated into **principal stresses** $\sigma_1$ (maximum) and $\sigma_2$ (minimum) at a characteristic angle.

The dominant source of stress is **packaging**. Epoxy mold compounds shrink more than silicon as they cool from cure temperature ($\sim 175\,^\circ\text{C}$), generating compressive stresses. Modern low-stress compounds use $\geq 90\%$ filler (rounded silica particles) to reduce CTE mismatch:

| Material | CTE ($\text{ppm/}^\circ\text{C}$) |
|---|---|
| Silicon | 2.6 |
| Silicon dioxide | 0.5 |
| Alloy 42 | 4.5 |
| Copper alloys | 16--18 |
| Epoxy mold compounds | 8--35 |
| Printed circuit boards | 12--18 |

**Stress distribution on a die:** The minimum principal stress (most compressive) is oriented **radially** from the die center. It is most intense at the center and falls off near the edges. Shear stress is negligible near die axes of symmetry but rises sharply near die **corners**. Stress **gradients** (which cause mismatch) are smallest near the center and largest near edges and corners.

**Piezoresistivity and Orientation:**

The resistance shift under stress is:

$$\frac{\Delta R}{R} = \pi_L \sigma_L + \pi_T \sigma_T + \pi_S \tau_S$$

where $\pi_L$, $\pi_T$, $\pi_S$ are the longitudinal, transverse, and shear piezoresistance coefficients. Key values (in $10^{-11}\,\text{Pa}^{-1}$, for lightly doped silicon at $25\,^\circ\text{C}$):

| Orientation | P-type $\pi_L$ | P-type $\pi_T$ | N-type $\pi_L$ | N-type $\pi_T$ |
|---|---|---|---|---|
| $\langle 110 \rangle$ on (100) | 71.8 | $-$ | $-31.2$ | $-$ |
| $\langle 100 \rangle$ on (100) | 6.6 | $-$ | $-102.2$ | $-$ |
| Any on (111) | 71.8 | $-$ | $-31.6$ | $-$ |
| Any in polysilicon | 24 | 9.5 | 24 | 9.5 |

**Practical implications:**
- N-type resistors on (100) silicon: minimum piezoresistivity along $\langle 110 \rangle$ (horizontal/vertical in layout).
- P-type resistors on (100) silicon: minimum piezoresistivity along $\langle 100 \rangle$ ($45^\circ$ to wafer flat), but this is awkward to lay out.
- Polysilicon: orientation-independent (grains are random); medium/high-sheet poly is less stress-sensitive than mono-Si.
- Thin-film (nichrome): piezoresistivity coefficients are orders of magnitude smaller than silicon.
- **L-shaped segments** (equal horizontal + vertical lengths) can cancel piezoresistivity; a practical version uses two identical common-centroid arrays oriented perpendicularly.

**Piezocapacitance:** Capacitors are far less stress-sensitive than resistors. The piezocapacitance coefficient for oxide on (100) silicon is $\sim -1.5 \times 10^{-11}\,\text{Pa}^{-1}$, several orders of magnitude smaller than piezoresistance coefficients. Still, common-centroid layout is recommended for precision capacitors.

**Package shift mitigation strategies:**
- Low-stress mold compounds ($\geq 90\%$ rounded filler, CTE $\sim 8\,\text{ppm/}^\circ\text{C}$)
- Elastomeric die overcoats ($\geq 50\,\mu\text{m}$ polyimide or silicone)
- Multiple devices placed $\geq 200\,\mu\text{m}$ apart (averages out filler-induced random stress)
- Selection from multiple matched arrays post-packaging
- Preferred die aspect ratios (plastic: $\leq 1.5{:}1$ preferred, $3{:}1$ max; chip-scale with solder: $\leq 1.5{:}1$)

#### Electric Fields (8.2.9)

Electric fields cause several types of mismatch:

**Voltage (body/tank) modulation:** Reverse-biased junctions isolating diffused resistors create depletion that depends on applied voltage. Standard bipolar base diffusion shows $\sim 0.1\%/\text{V}$ of tank modulation; HSR can reach several percent per volt. Matched resistors at different operating voltages need **separate tanks**, each tied to the positive end of its resistor segment.

**Conductivity modulation from leads:** Metal-1 over HSR produces $\sim 0.1\%/\text{V}$ of conductivity modulation. Do not route unconnected leads over critically matched resistors. If unavoidable, use **electrostatic (Faraday) shielding**: interpose a metal layer connected to the circuit reference node between the resistor and the overlying leads. The shield should overhang the resistor by $\geq 5\,\mu\text{m}$ to intercept fringing fields.

**Segmented shields:** If poly sheet resistance exceeds $\sim 1\,\text{k}\Omega/\Box$ or the voltage across the array exceeds a few volts, a common shield itself causes conductivity modulation. Split the shield into individual sections connected to each resistor segment.

**Charge spreading:** Hot carriers injected into interlevel oxide accumulate negative charge that drifts laterally under electric fields. This can modulate high-sheet resistors. Field plates (connected to the well/tank containing the resistors) prevent both channel formation and charge-spreading-induced drift.

**Dielectric absorption (soakage):** Mobile ions or polar groups in phosphosilicate/borophosphosilicate glass rearrange slowly under electric fields, causing long-term drift under bias. Observed $\sim 0.1\%$ variation in high-sheet bipolar resistors. **Split field plates** (gap at midpoint of resistor) create equal-and-opposite fields in each half, canceling dielectric absorption while still blocking charge spreading. Recommended for diffused resistors with $R_s > 1\,\text{k}\Omega/\Box$ that must match better than $0.1\%$.

**Dielectric relaxation in capacitors:** Composite dielectrics (ONO stacks, TEOS oxides) exhibit microsecond-scale dielectric relaxation that affects charge-redistribution data converters. Grown oxides and LPCVD oxides are preferred.

---

### 8.3.1: Rules for Resistor Matching

The 25 rules are organized here by theme:

#### Material Selection

**Rule 1 -- Same material.** Matched resistors must be built from the same material. Different materials show $\geq 5\%$ mismatch from process variation and $\geq 1\%$ mismatch over temperature. Material quality ranking (best to worst for matching):
- **Thin-film** (nichrome, sichrome): TCR $< 100\,\text{ppm/}^\circ\text{C}$, areal matching coefficient $< 0.1\%\cdot\mu\text{m}$, essentially zero voltage modulation, very low piezoresistivity.
- **Polysilicon**: TCR $\sim 1000\,\text{ppm/}^\circ\text{C}$ (for N-type, $\sim 500\,\text{\AA}$), areal matching $\sim 0.5\%\cdot\mu\text{m}$, low conductivity modulation, moderate piezoresistivity. Adequate for moderate matching.
- **Diffused/implanted**: May have better areal matching coefficients than poly, but worse in every other respect (high TCR, voltage modulation, piezoresistivity). HSR has difficulty achieving even moderate matching.

#### Geometry

**Rule 2 -- Same width.** Width biases cause systematic mismatch. Edges differ from centers (doping profiles, etch rates). If different values are needed, use same-width segments with different numbers in parallel.

**Rule 3 -- Sufficient area.** Random mismatch decreases with area per Pelgrom's law. Guidelines for how much random mismatch should contribute to total:

| Matching tier | Max random mismatch fraction |
|---|---|
| Minimal | $\leq 75\%$ |
| Moderate | $\leq 50\%$ |
| Exceptional | $\leq 25\%$ |

Example for poly ($A_R \approx 1\%\cdot\mu\text{m}$): $\pm 0.1\%$ matched pair needs $\sim 1000\,\mu\text{m}^2$; $\pm 0.01\%$ needs $\sim 100{,}000\,\mu\text{m}^2$. Exceptional matching almost invariably requires **trimming**.

**Rule 4 -- Sufficient width.** Use $\geq 150\%$ of minimum linewidth for minimal, $\geq 200\%$ for moderate, $\geq 400\%$ for exceptional matching. Extremely narrow poly ($< 0.5\,\mu\text{m}$) may exhibit "bamboo structure" (single grain spanning the width), increasing variability.

**Rule 5 -- Identical segment geometries.** Corner and end effects preclude matching of different lengths/shapes. Build from arrays of identical rectangular segments. Partial segments require sensitivity analysis.

**Rule 10 -- No excessively short segments.** Head resistance and contact effects dominate short segments. Minimum length: $3\times$ design-rule minimum for minimal, $5\times$ for moderate, $10\times$ for exceptional. Exceptionally matched poly should have total length $\geq 1000\times$ grain diameter ($\geq 50\,\mu\text{m}$) to average grain boundary effects.

**Rule 16 -- Serpentines only for minimal matching.** Serpentine resistors cannot be properly interdigitated. Moderate matching requires arrayed segments (even dogbone segments). Exceptional matching uses widths sufficient for simple rectangular segments.

#### Orientation and Placement

**Rule 6 -- Same orientation.** Even minimally matched resistors should be oriented identically to avoid stress-induced mismatch. For minimum piezoresistivity: N-type mono-Si on (100) uses horizontal/vertical; P-type mono-Si on (100) ideally at $45^\circ$ but practically horizontal/vertical using poly instead.

**Rule 7 -- Close proximity.** Gradients increase with separation. Minimal: $\leq$ a few hundred microns apart. Moderate: immediately adjacent, interdigitated if thermal/stress gradients exist (large dice $> 50\,\text{mm}^2$, power $> 1\,\text{W}$, heat-sunk packages, solder/eutectic die attach, near die edges within $500\,\mu\text{m}$, near heat sources within $1\,\text{mm}$). Exceptional: always common-centroid.

**Rule 12 -- Low stress gradient region.** The broad interior of the die (out to $\sim$halfway to edges) has low stress gradients. Minimal: avoid die corners and within $200\,\mu\text{m}$ of edges. Moderate: interior preferred, or center of one side, inset $\geq 500\,\mu\text{m}$. Exceptional: always near die center.

**Rule 13 -- Away from power devices.** Exceptional matching is nearly impossible on power ICs ($> 1\,\text{W}$). If attempted: use low-TCR material, place resistors and power device on one die axis, resistors at far end ($\sim 3/4$ from center toward far edge), elongate die to $\sim 2{:}1$ ratio. Moderate: keep well away from multi-watt sources, interdigitate, place on axis of symmetry. For small power devices, maintain $\geq 1\,\mu\text{m/mW}$ separation (half or quarter in practice).

**Rule 14 -- Axes of symmetry.** Stress distributions are symmetric about die axes. Exceptional: always on an axis of symmetry. Moderate mono-Si: near an axis when possible. Thin-film: more tolerant but still avoid edges/corners.

#### Array Construction

**Rule 8 -- Interdigitate.** Arrange identical segments to obey common-centroid layout rules (from Table 8.4): symmetry, coincidence, dispersion, and compactness. Exceptional matching requires close attention to **dispersion** (subdivide into smaller common-centroid elements to minimize quadratic residue). Avoid partial segments; if used, inset equally on both ends. Long arrays can use multiple banks, each obeying common-centroid rules.

**Rule 9 -- Dummies.** Requirements scale with matching tier:

| Tier | Diffused/implanted | Deposited |
|---|---|---|
| Minimal | Not required | Single min-width dummy each end |
| Moderate | Full-width dummy each end | Full-width dummy each end |
| Exceptional | Full-width dummies each end | Multiple dummies spanning $\geq 10\,\mu\text{m}$ each end |

Spacings between all segments (dummy or active) must be identical. Segment ends must extend past the active region: $3\times$ minimum width for moderate, $5\times$ for exceptional.

**Rule 11 -- Cancel thermoelectrics.** Connect half of each set of series segments with current flowing in one direction, half in the other. Use even number of segments per set when possible.

#### Substrate and Environment

**Rule 17 -- Same field oxide.** Poly over different field oxides (N-well vs. P-well) will have different film thicknesses. Even with CMP, small differences remain. Moderate/exceptional resistors should stay $\geq 10\,\mu\text{m}$ from moat edges (LOCOS bird's beak and STI stress effects extend several microns).

**Rule 18 -- NBL shadow.** The buried layer outdiffuses and can modulate overlying diffused/implanted resistors. If NBL shift direction is unknown, overlap NBL over resistor by $\geq 150\%$ of max epi thickness on all sides. STI processes have no NBL shadow.

**Rule 19 -- Gate doping block clearance.** Grain boundaries accelerate dopant diffusion in poly. Use $\geq 150\%$ of design-rule spacing for moderate and exceptional matching.

**Rule 24 -- Distance from other diffusions.** Diffusion/implant tails extend well beyond metallurgical junctions. Moderate/exceptional resistors need $\geq 150\%$ of minimum spacing.

**Rule 25 -- Distance from poly-poly capacitors.** Photoresist thinning occurs in the "shadow" of large poly-2 features. Space matched resistors from capacitors by at least the poly-2 width.

#### Shielding and Modulation

**Rule 15 -- Conductivity modulation.** Mono-Si suffers much worse conductivity modulation than deposited resistors. HSR is very difficult to match moderately. Base diffusion is possible but poly is almost always better. Poly with $R_s \leq$ a few hundred $\Omega/\Box$: conductivity modulation is negligible. Higher-sheet poly: consider field plating. Exceptional: use thin-film $\leq$ a few hundred $\Omega/\Box$; nichrome is nearly immune.

**Rule 20 -- Field plating.** Field plate any diffused/implanted resistor operating above $50\%$ of thick-field threshold. Field plate all moderate-matching diffused resistors with $R_s \geq 1\,\text{k}\Omega/\Box$. Field plate all exceptionally matched diffused resistors.

**Rule 21 -- No unconnected leads overhead.** Minimal: okay if $R_s \leq 1\,\text{k}\Omega/\Box$ and signals are low-frequency analog or static digital. Moderate: no unconnected leads at all; if unavoidable, use a field plate on a low-impedance node. Exceptional poly: requires metal-2 field plate extending $\geq 5\,\mu\text{m}$ beyond resistors in all directions; no metal-1 crossing active bodies.

#### Electrical Considerations

**Rule 22 -- Power dissipation.** Self-heating creates thermal gradients. Exceptional: $\leq 1\,\text{mW}$ total. Temperature rise can be estimated from Equation 5.7 and must cause shifts much less than the target mismatch.

**Rule 23 -- Metallization voltage drops.** Current in interconnect generates IR drops that can disturb matching. Each metal layer's and via's resistance contribution should scale proportionally to the desired resistance ratios. Aluminum via resistances are highly variable. Use **Kelvin connections** (separate sense and force leads) for exceptional matching.

---

### 8.3.2: Rules for Capacitor Matching

Properly constructed oxide-dielectric capacitors in plastic packages can achieve $\pm 0.01\%$ and perhaps $\pm 0.001\%$ matching -- better than any type of resistor. They are commonly used in monolithic data converters with 14--17 bit relative accuracy.

**Capacitor types ranked by matching potential:**
1. **Poly-metal with silicided lower electrode + thick LPCVD oxide dielectric** -- best: metallic electrodes eliminate poly depletion; thick oxide reduces random mismatch; LPCVD oxide has low dielectric relaxation.
2. **Poly-poly** -- good: eliminates MOS body parasitics/leakage, but upper electrode depletion limits accuracy.
3. **MOS (accumulated/inverted)** -- moderate: poly depletion + parasitic capacitance + junction leakage.
4. **Junction capacitors** -- unsuitable: large temperature-dependent depletion variations.

#### The 13 Rules for Capacitor Matching

**Rule 1 -- Identical geometries.** Width biases and fringing capacitances cause mismatch between different-sized capacitors. Use arrays of identical **unit capacitors** connected in parallel. For integer ratios (e.g., 1:2:4:8), use proportional numbers of unit capacitors. Avoid series connections (top/bottom plate parasitic asymmetry); if needed, use **antiparallel** connections. For non-integer ratios, compute the non-unitary capacitor's dimensions to match the unit capacitor's area-to-periphery ratio.

**Rule 2 -- Square geometries.** Squares minimize the periphery-to-area ratio, reducing peripheral effects. Rectangles up to $\sim 3{:}1$ are acceptable for moderate matching. Exceptional: always square. Avoid complex shapes (OPC effects at corners are not fully repeatable).

**Rule 3 -- Sufficient area.** Same Pelgrom scaling as resistors. For a poly-poly capacitor with $A_C \approx 0.5\%\cdot\mu\text{m}$: $\pm 0.1\%$ pair needs $\sim 250\,\mu\text{m}^2$; $\pm 0.01\%$ needs $\sim 25{,}000\,\mu\text{m}^2$. Large capacitors should be subdivided into unit capacitors and cross-coupled. Optimal unit capacitor size: $25$--$100\,\mu\text{m}$ per side (balances peripheral effects vs. gradient sensitivity).

**Rule 4 -- Adjacent placement.** Even minimally matched capacitors must reside next to each other. Arrays should form compact row-column patterns (e.g., $4 \times 8$ for 32 units) with equal row and column spacings.

**Rule 5 -- Same field oxide.** Topography variations cause parasitic capacitance differences. Even with CMP, residual effects remain. Moderate: $\geq 10\,\mu\text{m}$ from moat edges. More accurately matched devices: no lower-level routing beneath them (or use a solid plate beneath).

**Rule 6 -- Parasitic capacitance management.** Lower plates couple to substrate. Connect lower electrode to a low-impedance node, or place a well/NBL beneath the capacitor for substrate isolation. Critical for switching-converter applications with substrate noise.

**Rule 7 -- Dummies on all four sides.** All matched capacitor arrays need dummies around the perimeter. Moderate with shield: dummy width $\geq 3\times$ minimum. Without shield: much wider dummies needed (fringing extends $\geq 50\,\mu\text{m}$). Exceptional: exact copies of unit capacitors as dummies on all sides with matched spacing. Both electrodes of dummies should connect to a circuit node.

**Rule 8 -- Electrostatic shielding.** Shield (a) contains fringing fields (reducing dummy size needs), (b) allows leads to cross without mismatch/noise, (c) blocks external electrostatic coupling. All moderate and exceptional capacitors should be shielded. Shield overhang: $\geq 50\,\mu\text{m}$ if no dummies. Digital logic signals should never cross shielded capacitors.

**Rule 9 -- Cross-couple arrays.** Cross-coupling (common-centroid assignment of unit capacitors) eliminates the dominant source of mismatch: **dielectric thickness gradients**. Even two equal-value capacitors benefit from splitting into two sections and cross-coupling. Large arrays should maximize dispersion.

**Rule 10 -- Match lead capacitance.** Interconnect contributes parasitic capacitance that must be equalized. Shield the interconnect network. Maintain lead-to-adjacent-metal spacing $\geq 2$--$3\times$ the interlayer oxide thickness. Use jogs or dead-end branches to equalize lead capacitance. LVS tools with parasitic extraction can compute these automatically.

**Rule 11 -- Thick homogeneous dielectrics.** Thicker dielectrics exhibit less random variation. Homogeneous dielectrics (single-layer) are better than composites (multiple processing steps add variation). Important for exceptional matching of small-value capacitors.

**Rule 12 -- Low stress gradient location.** Capacitors are much less stress-sensitive than resistors, but extreme matching benefits from placement near die center, $\geq$ several hundred microns from die edges. High-$k$ dielectrics (titanates) are piezoelectric and much more stress-sensitive.

**Rule 13 -- Away from power devices.** Most dielectrics have small permittivity variations with temperature (inverse volume dependence). Poly-electrode capacitors have TCCs up to $\sim 50\,\text{ppm/}^\circ\text{C}$ from depletion width variation. Exceptionally matched capacitors with poly electrodes should avoid proximity to power devices.

## Diagrams

### Figure 8.26 -- Isobaric Stress Contour Plot

![[diagrams/ch08-matching-rules-fig1.png]]

**Page 407 showing Figure 8.26:** Isobaric contour plot of the minimum principal stress distribution across a typical epoxy-mounted silicon die, with section-line graphs showing the compressive stress profile along horizontal (A-A) and vertical (B-B) axes. The minimum principal stress is oriented radially and is most intense in a broad region near the die center. Stress gradients (isobar crowding) are smallest near the center and largest near edges and corners -- hence matched devices belong near the center.

### Figure 8.27 -- Favored Locations for Matched Arrays

![[diagrams/ch08-matching-rules-fig2.png]]

**Page 408 showing Figure 8.27:** Recommended placement of accurately matched common-centroid arrays on (100) and (111) dice. On both types, the preferred locations lie on the die axes of symmetry, near the center. On (100) silicon, N-type resistors should be oriented along $\langle 110 \rangle$ (H/V), while P-type resistors exhibit a four-fold symmetry pattern rotated $45^\circ$.

### Figure 8.30 -- Electrostatic Shielding of a Matched Poly Resistor Array

![[diagrams/ch08-matching-rules-fig3.png]]

**Page 413 showing Figure 8.30:** Practical example of electrostatic shielding applied to a matched poly resistor array. The array includes dummy segments at either end, and a metal shield connected to ground is interposed between the resistors and overlying leads. The shield overhangs the array by $\geq 5\,\mu\text{m}$ to intercept fringing fields. Leads can then cross the shielded resistors without conductivity modulation or noise injection.

## Practical Takeaways

### For Resistors

- **Always use the same material** for matched resistors. Never match a poly resistor to a diffused resistor.
- **Use identical rectangular segments** with the same width, orientation, and generous dimensions. Wider and longer segments reduce random mismatch and geometry-dependent effects.
- **Interdigitate into common-centroid arrays** for moderate and exceptional matching. Obey all four rules: symmetry, coincidence, dispersion, compactness.
- **Place dummies** at array ends -- full-width for moderate, multiple extending $\geq 10\,\mu\text{m}$ for exceptional.
- **Cancel thermoelectrics** by connecting half the segments in each direction. Use even numbers of series segments.
- **Put matched arrays near the die center**, on axes of symmetry, far from edges, corners, and power devices.
- **Do not route unrelated leads over matched resistors.** If unavoidable, interpose an electrostatic shield (metal layer tied to analog ground).
- **Field plate all diffused/implanted resistors** with $R_s \geq 1\,\text{k}\Omega/\Box$ to prevent charge spreading and dielectric absorption. Use **split field plates** for HSR requiring $< 0.1\%$ matching.
- **Watch for metallization IR drops** -- scale metal and via resistance contributions proportionally to desired resistance ratios. Use Kelvin connections for exceptional matching.
- **Nichrome thin-film is the gold standard** for matching: lowest TCR, lowest piezoresistivity, no conductivity modulation. Sichrome is second; poly is adequate for moderate work.

### For Capacitors

- **Use unit capacitor arrays** with identical square unit cells. Cross-couple them in a common-centroid pattern.
- **Thick, homogeneous, grown or LPCVD oxide dielectrics** with metallic (or silicided) electrodes give the best results.
- **Surround arrays with dummies on all four sides.** Cover with an electrostatic shield.
- **Match lead capacitance** meticulously -- use parasitic extraction and equalize with jogs or stubs.
- **Capacitors can tolerate locations farther from die center** than resistors (much lower piezosensitivity), but extreme matching still benefits from center placement.
- **Avoid series connections** of unit capacitors (parasitic asymmetry). If needed, use antiparallel connections.
- **Grown/LPCVD oxide** preferred over TEOS or ONO stacks for charge-redistribution converters (lower dielectric relaxation).

## Relation to the Bigger Picture

This section is the practical payoff of Chapter 8 -- it converts the theoretical mismatch mechanisms from [[ch08-mismatch-causes]] (random variation, gradients, thermoelectrics, stress, electric fields) into a checklist of 25 resistor rules and 13 capacitor rules that a layout designer can directly apply. These rules connect back to fundamental physics discussed in Chapters 1-2 (piezoresistivity, Seebeck effect, diffusion tails), fabrication effects from Chapters 3-4 (etch biases, CMP, NBL shadow), and reliability concerns from Chapter 5 (charge spreading, dielectric absorption, contamination). The matching principles here also foreshadow the transistor matching techniques in Chapters 9-10, where many of the same layout strategies (common-centroid arrays, dummies, orientation rules, stress placement) recur for bipolar and MOS devices.

## See Also
- [[ch08-mismatch-causes]]
