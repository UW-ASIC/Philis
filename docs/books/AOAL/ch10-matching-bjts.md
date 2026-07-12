---
title: "10.2-10.3 Matching Bipolar Transistors"
chapter: 10
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-10, matching, bipolar, common-centroid, piezojunction, thermal-gradient, NBL-shadow]
---

# 10.2-10.3 Matching Bipolar Transistors

> **Chapter 10: Applications of Bipolar Transistors**

## Key Concepts

Matched bipolar transistors are foundational to precision analog circuits: current mirrors, differential pairs, bandgap references, and Gilbert translinear cells all depend on accurate matching of collector currents ($I_C$) and base-to-emitter voltages ($V_{BE}$). This section explains every mechanism that degrades matching and provides a comprehensive set of layout rules to combat them.

The central insight is that bipolar transistor collector current depends *exponentially* on $V_{BE}$:

$$I_C = I_S \cdot e^{V_{BE}/V_T}$$

This exponential relationship is both a blessing and a curse. It means that a 1% mismatch in saturation current $I_S$ (equivalently, effective emitter area $A_E$) produces only a ~0.25 mV offset in $V_{BE}$, making bipolar transistors superb for low-offset differential pairs. However, it also means that even tiny temperature differences (fractions of a degree) produce measurable collector current mismatches, since $V_{BE}$ has a temperature coefficient of roughly $-2 \text{ mV/}^\circ\text{C}$.

Matching quality is limited by four major categories of effects:
1. **Random variations** -- statistical fluctuations in doping, junction area, and periphery
2. **Thermal gradients** -- temperature differences across the die causing $V_{BE}$ shifts
3. **Mechanical stress** -- piezojunction effect altering $I_S$ through package-induced stresses
4. **NBL shadow** -- surface discontinuities from the buried layer anneal perturbing emitter area

The designer's job is to minimize all four simultaneously, which often involves trade-offs (e.g., larger emitters reduce random mismatch but increase vulnerability to nonlinear thermal gradients).

## Random Variations (Section 10.2.1)

### The Statistical Model

Random mismatch originates from fluctuations in base doping, base-emitter junction area, and junction periphery. The standard model lumps all variation into the effective emitter area $A_E$. The saturation current becomes:

$$I_S = J_S \cdot A_E$$

where $J_S$ is a non-varying saturation current density and $A_E$ follows a normal distribution whose standard deviation is:

$$\sigma(\Delta A_E / A_E) = \frac{K_A}{\sqrt{A_E}} + \frac{K_P}{\sqrt{P_E}}$$

Here $A_E$ and $P_E$ are the effective area and periphery of the emitter, and $K_A$ and $K_P$ are the areal and peripheral matching coefficients. For all but the smallest transistors, the areal term dominates and the peripheral term can be neglected. When all devices use multiples of identical unit emitters, peripheral effects scale linearly with areal effects and can be combined.

### Mismatch in Current and Voltage

For a pair of same-size transistors at equal $V_{BE}$, the standard deviation of their collector current ratio is:

$$\sigma\left(\frac{\Delta I_C}{I_C}\right) = \frac{K_A}{\sqrt{A_E}} \tag{10.8}$$

For the same transistors at equal $I_C$, the standard deviation of their $V_{BE}$ difference is:

$$\sigma(\Delta V_{BE}) = \frac{V_T \cdot K_A}{\sqrt{A_E}} \tag{10.9}$$

Because of the high transconductance of bipolar transistors, $\sigma(\Delta V_{BE})$ is much smaller than $\sigma(\Delta I_C / I_C)$. For example, with $K_A = 5 \times 10^{-3} \ \mu\text{m}$ and $A_E = 225 \ \mu\text{m}^2$, the standard deviation of the collector current ratio is 0.33%, and $\sigma(\Delta V_{BE}) \approx 86 \ \mu\text{V}$ at room temperature.

### Practical Considerations for Random Mismatch

- The matching coefficient $K_A$ is essentially independent of emitter current density for most bipolar processes, but some newer BiCMOS processes with shallow retrograde wells show increased $K_A$ at low current densities (possibly due to sidewall injection dominance at low currents).
- If $K_A$ varies with current density, then Equations 10.8 and 10.9 only hold at constant emitter current density -- quadrupling emitter area only halves mismatch if you also quadruple the emitter current.
- Forward beta variation ($\sigma(\beta_F)$) seldom matters unless the circuit is very sensitive to base current errors *and* beta is relatively low (e.g., PNP mirror transistors with $\beta < 10$ in a Brokaw bandgap).
- Compact emitter geometries (circles, squares, octagons) are preferred because they maximize the area-to-periphery ratio, improving both $\beta$ and random matching.
- Emitter widths of 2x to 10x the minimum allowed width are typical for matched devices. Going beyond this range makes the array unnecessarily sensitive to nonlinear thermal/stress gradients.

## Emitter Degeneration (Section 10.2.2)

When one cannot afford to increase emitter area (common with lateral PNPs where larger emitters degrade $\beta$), **emitter degeneration** transfers the matching burden from the bipolar transistors to a set of associated resistors. By adding resistance in series with the emitter, the effective transconductance is reduced, making the collector current less sensitive to $V_{BE}$ variations.

### Quantitative Benefit

The improvement factor for collector current matching is:

$$\frac{\sigma(\Delta I_C / I_C)_{\text{degen}}}{\sigma(\Delta I_C / I_C)_{\text{no degen}}} \approx \frac{V_T}{V_T + V_{\text{degen}}} \tag{10.10}$$

where $V_{\text{degen}}$ is the DC voltage drop across the degeneration resistor.

| $V_{\text{degen}}$ | Matching improvement factor |
|---|---|
| 50 mV | ~3x |
| 100 mV | ~6x |
| 250-500 mV | Enables non-integer ratios |

### Additional Benefits and Limitations

- Emitter degeneration also improves output resistance by the same factor it improves matching, which is especially valuable for lateral PNPs with low Early voltages.
- The Early effect mismatch between transistors at different $V_{CE}$ is described by:

$$\frac{\Delta I_C}{I_C} \approx \frac{\Delta V_{CE}}{V_A} \tag{from 10.11}$$

  Degeneration reduces this systematic error proportionally.
- **Cannot be applied to split-collector transistors** since all split collectors share a common emitter.
- Degeneration resistors are also affected by thermal gradients, but interdigitation of the resistors can minimize this.
- Large degeneration (250-500 mV) enables non-integer current ratios. For example, a 3.4:1 ratio can be obtained by ratioing a 3X transistor with $R$ and a 1X transistor with $3.4R$.

## Thermal Gradients (Section 10.2.3)

### Why Thermal Gradients Matter So Much

$V_{BE}$ has a temperature coefficient of approximately $-2 \text{ mV/}^\circ\text{C}$, corresponding to a collector current temperature coefficient of about $+8\%/^\circ\text{C}$. Matched bipolar transistors routinely target offsets below $100 \ \mu\text{V}$, which corresponds to a temperature difference of only $0.05^\circ\text{C}$. Such tiny temperature variations occur in nearly any integrated circuit.

### Thermal Feedback

In high-gain amplifiers, thermal gradients create a secondary feedback path: output-stage power dissipation heats the die non-uniformly, creating offsets in the input stage that are amplified as if they were input signals. This produces spurious low-frequency poles and zeros in the amplifier's frequency response. The cure is to maximize the physical separation between input and output stages and to use common-centroid layout for the input transistors.

### The Differential Pair and Cross-Coupled Quad

The most important matched transistor structure is the differential pair. To cancel linear thermal gradients, differential pairs use a **cross-coupled quad** layout -- a two-dimensional common-centroid arrangement. Critical details:

- Not only the emitters, but also the **base contacts** must form a common-centroid pattern. If the base contacts are not symmetric, thermoelectric potentials at the base contacts can generate mismatch.
- Collector contacts have little impact on matching and need not be symmetrically arranged.
- **CBE orientation** (collector-base-emitter from outside to inside) is preferred over CEB because the temperature coefficient of $V_{BE}$ is larger than that of the base contact potential, so bringing emitters closer together minimizes thermal sensitivity.

### Ratioed Pairs and Quads

A ratioed pair consists of two transistors operating at different collector current densities, producing a voltage proportional to absolute temperature (VPTAT):

$$\Delta V_{BE} = V_T \ln\left(\frac{I_{C1}/A_{E1}}{I_{C2}/A_{E2}}\right) \tag{10.12}$$

For equal collector currents and emitter area ratio $N = A_{E2}/A_{E1}$:

$$\Delta V_{BE} = V_T \ln(N) = \frac{kT}{q}\ln(N) \tag{10.13}$$

This VPTAT voltage is remarkably linear with temperature and independent of current over up to 8 decades of operation in standard bipolar -- no other device relationship exhibits such independence from bias conditions.

A **ratioed quad** (four transistors) produces:

$$\Delta V_{BE} = V_T \ln\left(\frac{A_{E1} \cdot A_{E4}}{A_{E2} \cdot A_{E3}}\right) \tag{10.14}$$

This allows larger $\Delta V_{BE}$ from moderate area ratios (e.g., two 4X and two 1X transistors produce $\Delta V_{BE} = V_T \ln(16) = 72 \text{ mV}$).

### Optimal Emitter Area Ratios

As the emitter area ratio $N$ increases:
- VPTAT increases as $\ln(N)$ (logarithmic -- diminishing returns)
- The physical span of the array increases as $\sim\!\sqrt{N}$, making the array more vulnerable to nonlinear gradient residues that increase approximately linearly with $N$

The optimal ratio typically lies between **6:1 and 16:1**. Even-number ratios simplify common-centroid construction. The most popular choices are:
- Ratioed pairs: **4:1, 6:1, 8:1** (8:1 is most common)
- Ratioed quads: **4:1:1:4**

### Layout Techniques for Ratioed Pairs

- **4:1 ratio**: Place the 1X transistor in the center with two sections of the 4X transistor on either side (2:1:2 pattern).
- **8:1 ratio**: Use a $3 \times 3$ array -- the center transistor is the 1X device, surrounded by 8 unit transistors forming the 8X device. This two-dimensional array spans only one-third the distance of a linear arrangement, reducing quadratic gradient residues by nearly an order of magnitude.

### Merging Transistors

Multiple vertical NPN transistors can share a common collector (tank) without affecting current flow because the base, not the collector, controls conduction. A more aggressive merger combines both collector and base regions, placing multiple emitters in a shared base. This requires:
- Neutral base width between adjacent emitters $\geq 2 \times$ base junction depth
- Base overlap of emitter increased by a couple of microns
- **Never** use "connected emitter" spacing rules for matched devices, even if the emitters connect to the same node, because depletion region merger significantly interferes with current flow

## Mechanical Stress (Section 10.2.4)

### The Piezojunction Effect

Mechanical stress alters the saturation current of bipolar transistors through the **piezojunction effect**, analogous to piezoresistivity in resistors. For a circular (radially symmetric) lateral transistor:

$$\frac{\Delta I_S}{I_S} = \pi_R (\sigma_x + \sigma_y) \tag{10.16}$$

For a vertical transistor:

$$\frac{\Delta I_S}{I_S} = \pi_T (\sigma_x + \sigma_y) \tag{10.17}$$

where $\pi_R$ and $\pi_T$ are the radial and traverse piezojunction coefficients, and $\sigma_x$, $\sigma_y$ are the normal stresses along the die axes.

### Piezojunction Coefficients (units: $10^{-12} \text{ Pa}^{-1}$)

| Parameter | (100) Si | (111) Si |
|---|---|---|
| $\pi_T$ (NPN vertical) | $-43.4$ | $-15.1$ |
| $\pi_T$ (PNP vertical) | $-13.3$ | $-21.7$ |
| $\pi_R$ (NPN lateral) | $7.5$ | $-28.2$ |
| $\pi_R$ (PNP lateral) | $11.6$ | $-8.9$ |

Key insight: On (100) silicon, vertical NPN transistors are nearly 4x more stress-sensitive than vertical PNP counterparts. This has led researchers to suggest reconsidering PNP-based circuit topologies for stress-critical applications like bandgap references.

### Stress Magnitudes

For 100 MPa of in-plane stress at constant $I_C$:
- Vertical NPN on (100) Si: $\Delta V_{BE} \approx 0.9 \text{ mV}$
- Vertical NPN on (111) Si: $\Delta V_{BE} \approx 0.4 \text{ mV}$
- Lateral PNP on (100) Si: $\Delta V_{BE} \approx 0.3 \text{ mV}$
- Lateral PNP on (111) Si: $\Delta V_{BE} \approx 0.8 \text{ mV}$

The stress has little effect on $\Delta V_{BE}$ between ratioed transistors as long as both experience the same stress levels.

### Package-Induced Stress

Plastic mold compounds cure at ~175 deg C. Their CTE exceeds silicon's, so cooling generates compressive stress directed radially inward toward the die center. This causes a **package shift** in $V_{BE}$ that becomes more negative at low temperatures. In one family of commercial SOT-23 bandgap references, the package shift had a mean of ~4 mV and standard deviation of 2.3 mV.

Additional stress concerns:
- **Filler particles** in the mold compound press vertically on the die, creating highly localized out-of-plane stresses that common-centroid layout cannot combat
- **PCB reflow soldering** generates stresses from CTE mismatch between board and package; QFN packages (no projecting pins) are especially vulnerable
- **Long-term drift**: continued polymer cross-linking in the mold compound causes gradual stress changes over months/years
- **Thermal hysteresis**: stresses from temperature excursions may take minutes to hours to dissipate

### Mitigation Strategies

- Common-centroid layout cancels linear stress gradients
- Place matched transistors near the **center of the die** (smallest stress gradients, though highest absolute stress)
- Stay at least 200 um from die edges; never place matched devices near die corners
- Polymer overcoats (~20 um thick) substantially eliminate filler-induced stresses from particles ~100 um in diameter
- Thick copper metallization (~8 um) over compressive nitride has been shown to roughly halve package shift
- Low-stress mold compounds with higher filler loadings reduce all package shifts
- On-chip stress sensors (using resistor ratio techniques) can provide real-time correction signals

## NBL Shadow (Section 10.2.5)

During the NBL (N-type Buried Layer) anneal, surface oxidation creates a discontinuity. As epitaxial silicon is deposited, this discontinuity propagates upward as the **NBL shadow**. A phenomenon called **pattern shift** can displace this shadow laterally by up to ~2x the epi thickness.

If the NBL shadow intersects the emitter of a vertical NPN transistor, it introduces a systematic area mismatch. For example, if a 2:1 ratioed pair has one emitter of the 2X transistor affected by a 1% area reduction, the ratio shifts to 2.01:1 -- a 0.5% mismatch (~0.13 mV offset).

### Prevention

- Oversize the NBL geometry so the shadow cannot reach the emitter, even with worst-case pattern shift. If the shift direction is unknown, overlap NBL on all sides by at least **150% of the epi thickness**.
- Orient the array so the main axis of symmetry is parallel to the direction of pattern shift, pushing the shadow into the collector or base contact regions instead.
- **Processes using shallow trench isolation (STI) do not have an NBL shadow** because CMP removes the surface discontinuity.

### Pattern Shift Details

- On (111) Si: minimized by tilting wafer ~3.5 deg around a $\langle\bar{1}10\rangle$ axis; pattern shift typically 50-100% of epi thickness.
- On (100) Si: minimized using on-axis wafers, but even 10 arcminutes of misalignment can create pattern shift equal to the epi thickness at low deposition pressures. At higher temperatures, pattern shift becomes negligible.

## Other Causes of Systematic Mismatch (Section 10.2.6)

### Early Effect

Two transistors at different $V_{CE}$ exhibit systematic mismatch:

$$\frac{\Delta I_C}{I_C} = \frac{\Delta V_{CE}}{V_A} \tag{10.21}$$

For example, two NPNs with $V_A = 150 \text{ V}$ and $\Delta V_{CE} = 1 \text{ V}$ show 0.7% mismatch. Cascodes are the standard circuit technique to equalize $V_{CE}$.

### Collector Efficiency Variation (Laterals)

In lateral PNP transistors, collector efficiency varies with $V_{CE}$ because the collector-base depletion region depth changes. Higher $V_{CE}$ drives the depletion region deeper, improving collector efficiency. This effect is usually smaller than the Early effect but becomes severe if one transistor saturates.

### Cross-Injection in Merged Laterals

Multiple lateral PNPs merged in a common tank can inject carriers into each other, especially if one transistor saturates. The simplest cure is to give each lateral PNP its own separate tank. P-bars can achieve partial isolation with less area, but are not recommended for matched devices.

### Depletion Region Intrusion

Multiple emitters in a common base region can have their depletion regions merge if placed too closely, reducing effective emitter area. Use the spacing rule for unconnected emitters plus several additional microns.

## Matching Rules Summary (Section 10.3)

### Matching Tiers

| Tier | $\sigma(\Delta V_{BE})$ | $\sigma(\Delta I_C / I_C)$ | Application |
|---|---|---|---|
| **Minimal** | $0.5 - 1.0$ mV | $2 - 4\%$ | General-purpose op-amps, non-critical current mirrors |
| **Moderate** | $0.25 - 0.5$ mV | $1 - 2\%$ | Bandgap references, precision amplifiers without trim |
| **Exceptional** | $< 0.25$ mV | $< 1\%$ | Requires post-package trim or heavy emitter degeneration |

These figures assume 6-sigma statistics, 10-year operating lifetimes, 150 deg C junction temperature, and conventional plastic packaging.

### Rules for Vertical Transistors (Section 10.3.1)

1. **Identical emitter geometries** -- different sizes/shapes match very poorly. Use integer ratios with unit emitters.
2. **Emitter width >= 2x minimum** -- typically 8-16 um. Larger emitters improve random matching but increase gradient sensitivity. Use arrays of moderate emitters rather than single very large ones.
3. **Circular or square emitters** -- maximize area-to-periphery ratio. Circular layouts should use polygons with segments divisible by 4 (32 or 64 segments). Match contact geometry to emitter geometry.
4. **Close proximity** -- minimal: adjacent placement; moderate/exceptional: common-centroid mandatory.
5. **Compact layout** -- tight 2D clusters outperform long rows. Cross-coupled pairs for equal-size transistors; 2D common-centroid arrays for exceptional matching.
6. **Ratioed pairs: 4:1 to 16:1** -- optimal range. 8:1 is most popular. Smaller ratios generate small VPTAT; larger ratios require large arrays.
7. **Distance from power devices** -- minimal: >= 500 um from major (>=250 mW) heat sources; moderate: 100-200 um from any >50 mW device, opposite side of die; exceptional: as far as possible, consider 1.5:1 or 2:1 die aspect ratio.
8. **Low stress gradient areas** -- center of die has smallest gradients. Stay >= 200 um from edges. Never place near corners. For chip-scale packages, best location is center of a group of four solder balls/copper pillars near die center.
9. **On die axes of symmetry** -- stress distribution is symmetric about die axes.
10. **NBL shadow clearance** -- overlap NBL beyond emitter by enough to account for pattern shift plus misalignment. If shift is unknown, assume 150% of epi thickness.
11. **Emitter spacing in common base** -- depletion region gap >= base junction depth; use unconnected-emitter spacing rules even for connected emitters.
12. **Increase base overlap** -- for moderate/exceptional matching, add 1-2 um beyond minimum rule to minimize beta variation from misalignment.
13. **Operate below high-level injection** -- no more than 1/3 to 1/2 of the onset current. Check semilog $I_C$ vs $V_{BE}$ plot for linearity departure.
14. **Match contact geometry to emitter geometry** -- circular contacts for circular emitters, square for square. Consider reducing contact size in thin-emitter BiCMOS processes to minimize recombination.
15. **Consider emitter degeneration** -- 50 mV for moderate, 100 mV for exceptional matching. Not applicable to ratioed pairs/quads.
16. **Equal $V_{CE}$** -- insert cascodes to equalize collector-to-emitter voltages.
17. **Prevent $V_{BE}$ avalanche** -- keep reverse $V_{EB}$ below 50% of $BV_{EBO}$. Add diode clamps on external pins.
18. **Consider vertical PNP on (100) Si** -- traverse piezojunction coefficient is ~3x smaller than NPN, reducing package shifts in bandgap references.

### Rules for Lateral Transistors (Section 10.3.2)

1. **Identical emitter AND collector geometries** -- both affect conduction. For exceptional matching, each emitter gets its own identical collector in its own base region.
2. **Minimum-size emitters** -- unlike vertical transistors, larger emitters degrade lateral $\beta$, hurting matching more than the increased area helps.
3. **Collector inner periphery matches emitter geometry** -- circular emitter demands circular collector inner edge. Emitter must be concentric within collector for equal base widths.
4. **Field plate the base region** -- connects to emitter, spans the entire neutral base and slightly into the collector. Prevents conductivity modulation from surface charges. Use lowest metal or poly.
5. **Split-collector laterals can achieve moderate matching** -- all collectors must be identical copies, and none may saturate.
6. **Close proximity / common tank** -- moderately matched non-saturating transistors may share a tank; exceptionally matched transistors only if collector efficiency > 0.999.
7. **Distance from heat sources** -- same rules as vertical (500 um minimal, 100-200 um moderate).
8. **Low stress gradient areas** -- same die-center preference as vertical.
9. **On die axes of symmetry** -- same as vertical.
10. **NBL shadow clearance for base region** -- the NBL shadow must not intrude into the exposed neutral base between emitter and collector. Overlap drawn NBL over inner collector perimeter by >= 150% of epi thickness.
11. **Operate below high-level injection** -- lateral transistors are especially susceptible due to lightly doped bases. Their $\beta$ vs $I_C$ plots show a pronounced hump. Limit current to 30-50% of the onset of nonlinearity in the semilog $I_C$-$V_{BE}$ plot.
12. **Match emitter contact to emitter geometry** -- same as vertical.
13. **Use emitter degeneration** -- laterals benefit more than verticals due to lower $V_A$ and the inadvisability of increasing emitter area. Minimum 50 mV for minimal/moderate, 100 mV for exceptional. Can create non-integer ratios with 200-300 mV degeneration.
14. **Equal $V_{CE}$** -- lateral Early voltages (50-200 V) are lower than vertical, making this even more important. Cascodes help.

## Diagrams

### Figure 10.23 -- Cross-Coupled Bipolar Quad

![[diagrams/ch10-matching-bjts-fig1.png]]

The cross-coupled quad is the standard two-dimensional common-centroid layout for differential pairs. Both emitters and base contacts must have coincident centroids. Collector contacts are placed on the outside of the array to bring emitters and bases closer together (CBE orientation). This cancels linear thermal and stress gradients across the matched pair.

### Figure 10.24 -- Ratioed Pair Construction Techniques

![[diagrams/ch10-matching-bjts-fig2.png]]

Two approaches for constructing ratioed pairs from unit transistors: (A) a 2:1:2 linear array for a 4:1 ratio, and (B) a 3x3 "eight around one" array for an 8:1 ratio. The 2D arrangement in (B) spans only 1/3 the distance of a linear array, reducing quadratic gradient residues by nearly an order of magnitude. The base contacts in the 3x3 pattern form their own common-centroid arrangement.

### Figure 10.30 -- NBL Shadow Causing Mismatch

![[diagrams/ch10-matching-bjts-fig3.png]]

Pattern shift displaces the NBL shadow laterally, potentially intersecting one or more emitters asymmetrically and creating systematic area mismatches. The solution (B) is to oversize the NBL geometry so that even with worst-case pattern shift, the shadow cannot reach any matched emitter.

## Practical Takeaways

- **Always use identical unit emitters** for matched bipolar transistors. Integer ratios only; no different sizes or shapes.
- **Common-centroid layout is mandatory** for moderate or exceptional matching. Cross-coupled quads for equal-size pairs; 2D arrays for ratioed pairs.
- **CBE orientation** (collector outside, emitter inside) is preferred over CEB to minimize thermal sensitivity.
- **Emitter degeneration is your friend** when you cannot increase emitter area (especially for lateral PNPs). Even 50 mV of degeneration gives 3x improvement.
- **Thermal gradients dominate** for most practical designs. Place matched transistors as far as possible from power devices and as close together as possible.
- **Stress is the second enemy**: place matched arrays near die center, on axes of symmetry, and far from edges/corners. Consider thick copper overcoats and low-stress mold compounds.
- **NBL shadow**: always ensure drawn NBL overlaps emitters by enough margin for pattern shift. If unknown, use 150% of epi thickness.
- **Never operate near high-level injection**: keep $I_C$ at no more than 1/3 to 1/2 of the onset current.
- **For bandgap references on (100) Si**, consider vertical PNP topologies -- their piezojunction sensitivity is 3x lower than NPN, significantly reducing package shift.
- **Package shift, long-term drift, and thermal hysteresis** are unavoidable realities of plastic packaging. Post-package trim can cancel the initial shift, but long-term drift (~25 ppm/1000 hours) and thermal hysteresis set fundamental accuracy limits.
- **Emitter area ratio sweet spot** for ratioed pairs is 4:1 to 16:1 (8:1 most popular). Smaller ratios give too little VPTAT; larger ratios waste area and increase gradient vulnerability.
- **Field plate the base region** of all matched lateral PNP transistors to prevent conductivity modulation.

## Relation to the Bigger Picture

This section is the bridge between the theoretical matching framework of [[ch08-matching-rules]] (which covers general common-centroid techniques, gradient cancellation, and the Pelgrom model) and its practical application to the most matching-critical devices in analog IC design. While Chapter 8 provides the "how" of common-centroid layout and gradient analysis, Sections 10.2-10.3 provide the "why" specific to bipolar physics -- the exponential $I_C(V_{BE})$ relationship, the piezojunction effect, and the NBL shadow. The power transistor layouts discussed in [[ch10-power-bjts]] create the thermal gradients that make these matching techniques essential; without the matching rules here, no precision analog circuit could coexist on the same die as a power stage.

## See Also
- [[ch10-power-bjts]]
- [[ch08-matching-rules]]

---
