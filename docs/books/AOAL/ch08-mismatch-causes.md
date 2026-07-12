---
title: "8.1-8.2 Mismatch and Causes"
chapter: 8
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-8, matching, mismatch, common-centroid, piezoresistivity, thermal-gradients, process-bias]
---

# 8.1-8.2 Mismatch and Causes

> **Chapter 8: Matching of Resistors and Capacitors**

Integrated resistors and capacitors typically exhibit absolute tolerances of $\pm 20\%$ to $30\%$, far worse than discrete components ($\pm 5\%$). However, integrated devices *track one another* much better than their absolute tolerances suggest. Matched resistors can achieve $\pm 0.1\%$ matching (some cases even $\pm 0.01\%$); matched capacitors can do as well or better. Understanding and controlling the causes of mismatch is one of the most important skills in analog IC layout.

---

## Key Concepts

### 8.1 Mismatch Defined

**Mismatch** is the fractional deviation of the measured ratio of two device values from their intended ratio. For two devices with measured values $V_1, V_2$ and intended values $V_{1i}, V_{2i}$:

$$\delta = \frac{V_1/V_2 - V_{1i}/V_{2i}}{V_{1i}/V_{2i}} \tag{8.1}$$

Mismatch is signed (positive or negative) -- the sign must be retained for valid statistical analysis.

**Population vs. Sample Statistics:** The population is the set of all units ever manufactured. In practice we measure a *sample* (ideally $\geq 30$ units to avoid sampling error). From the sample mismatches $\delta_1, \delta_2, \ldots, \delta_N$, we compute:

- **Sample mean** (systematic mismatch): $\bar{\delta} = \frac{1}{N}\sum_{i=1}^{N} \delta_i$ [Eq. 8.2]
- **Sample standard deviation** (random mismatch): $s = \sqrt{\frac{1}{N-1}\sum_{i=1}^{N}(\delta_i - \bar{\delta})^2}$ [Eq. 8.3]

**Six-sigma mismatch** defines the data limits:
- $UDL = \bar{\delta} + 3s$ [Eq. 8.4]
- $LDL = \bar{\delta} - 3s$ [Eq. 8.5]

The vast majority of the population falls between these bounds.

**Systematic vs. Random mismatch:** Systematic mismatch affects all units identically and can be eliminated by proper layout. Random mismatch is caused by microscopic process fluctuations and can only be reduced by increasing device area. The mean measures systematic mismatch; the standard deviation measures random mismatch.

**Precision vs. Accuracy:** These are not interchangeable. *Precision* refers to measurement repeatability; *accuracy* refers to closeness to a target. Mismatch is a measure of accuracy (target = 0% mismatch).

---

## Important Details

### 8.2.1 Random Variation (Pelgrom's Law)

All integrated components have microscopic irregularities in dimensions and composition. For two-dimensional devices (resistors, capacitors), irregularities fall into two categories:

- **Peripheral variations** -- scale with device periphery $P$ (edge roughness, etch irregularities)
- **Areal variations** -- scale with device area $A$ (bulk doping fluctuations, grain boundary effects)

The standard deviation of a device parameter is modeled as:

$$\frac{\sigma}{\mu} = \sqrt{\frac{K_A^2}{A} + \frac{K_P^2}{P}} \tag{8.6}$$

where $K_A$ is the **areal matching coefficient** and $K_P$ is the **peripheral matching coefficient**.

For two devices of the same type but not necessarily the same value, the standard deviation of mismatch is:

$$\sigma_\delta = \sqrt{\frac{\sigma_1^2}{\mu_1^2} + \frac{\sigma_2^2}{\mu_2^2}} \tag{8.7}$$

When the areal term dominates (most common case) and devices have equal dimensions:

$$\sigma_\delta = \frac{K_A}{\sqrt{A}} \tag{8.10}$$

This inverse-square-root relationship between mismatch and area is called **Pelgrom's law** (Pelgrom, Duinmaijer, Welbers, 1989).

#### Capacitors

For a parallel-plate capacitor, $A$ is proportional to $C$, yielding:

$$\sigma_\delta = \frac{K_{AC}}{\sqrt{C_1 \cdot C_2}} \tag{8.12}$$

For two equal capacitors: $\sigma_\delta = K_{AC}/C$. To halve mismatch, you must quadruple capacitance -- a harsh diminishing-returns relationship. The **smaller capacitor dominates** the mismatch in unequal pairs.

#### Resistors

For a rectangular resistor of width $W$ and length $L$ with sheet resistance $R_s$: area $A = W \cdot L$ and resistance $R = R_s \cdot L/W$, so $A = R \cdot W^2 / R_s$. Substituting:

$$\sigma_\delta = \frac{K_{AR} \cdot \sqrt{R_s}}{W \cdot \sqrt{R}} \tag{8.14}$$

Two fundamental insights:
1. Random mismatch scales inversely with $\sqrt{R}$ (same as for capacitors)
2. Random mismatch scales inversely with resistor width $W$ -- wider is better

For unequal resistors, the **smaller resistor dominates** the mismatch. This can be mitigated by constructing the smaller resistor from multiple parallel segments.

**Poly resistor matching coefficient** is theoretically:

$$K_{AR} = d\sqrt{k} \tag{8.17}$$

where $d$ is the mean poly grain diameter and $k \approx 2$. This holds as long as $W \gg d$ (otherwise bamboo effects appear). Grain diameters are typically $< 1\,\mu\text{m}$.

### 8.2.2 Process Biases

The difference between measured and drawn dimensions is the **process bias** $\Delta$:

$$\Delta = D_{meas} - D_{drawn} \tag{8.18}$$

Process biases during photolithography, etching, diffusion, and implantation systematically distort device dimensions. Key examples:

- **Width biases:** Two resistors with drawn widths of $2\,\mu\text{m}$ and $4\,\mu\text{m}$ with a $-0.1\,\mu\text{m}$ etch bias yield an actual ratio of $1.9/3.9 = 0.487$ instead of $0.5$ -- a 2.6% systematic mismatch. **Solution:** Use segments of the same width; construct the wider resistor from parallel segments of the narrower width.

- **Length biases:** Contact width bias displaces contact centers, altering the spacing between contacts. **Solution:** Use segments of the same length; divide longer resistors into series connections of equal-length segments.

- **Capacitor biases:** Matched capacitors become insensitive to process biases when their **area-to-periphery ratios are equal**. For integer ratios, use arrays of identical unit capacitors. For non-integer ratios, one can compute dimensions $L_B$ and $W_B$ for the larger capacitor so that it has the same area-to-periphery ratio as the smaller square capacitor of side $S$:

$$L_B = S \cdot \frac{r+1}{2} \quad \text{and} \quad W_B = S \cdot \frac{2r}{r+1} \tag{8.19, 8.20}$$

where $r = C_B/C_A$ is the capacitance ratio.

### 8.2.3 Proximity Effects

#### Photolithographic Effects

- **Iso-dense print bias:** Difference in linewidth between isolated lines and lines in dense arrays. Negligible for linewidths greater than:

$$W_{min} \approx \frac{\lambda}{2 \cdot NA} \tag{8.21}$$

where $\lambda$ is light wavelength and $NA$ is numerical aperture. For I-line steppers ($\lambda = 365\,\text{nm}$, $NA = 0.6$), this is about $300\,\text{nm}$. In practice, significant below about $2\times$ minimum feature size.

**Solution:** Add **dummy resistors** at either end of the array, matching the spacing used between active segments. Dummies can be unconnected (Figure 8.3A) or connected to a suitable circuit node (Figure 8.3B) -- the latter is preferred to avoid static charge accumulation via hot carrier injection.

- **H-V bias (horizontal-vertical bias):** Optical astigmatism causes vertical and horizontal lines to experience slightly different process biases. **Always orient matched resistors in the same direction.**

- **Photoresist spin effects:** Tall pre-existing geometries (e.g., poly-poly capacitors) block photoresist flow, causing width variations in nearby structures within about $20\,\mu\text{m}$ of their edges.

#### Etch-Rate Variations (Microloading)

Larger openings adjacent to etched features provide more access to etchant and more opportunity for byproduct escape, leading to overetching. This creates an **iso-dense etch bias** where isolated lines become narrower than dense-array lines.

Even small etch biases matter: a $100\,\text{\AA}$ undercut with a fractional iso-dense bias of $20\,\text{\AA}$ per side represents a 2% error. Etch-rate variations extend at least $20\,\mu\text{m}$ into an array, so for the highest matching, dummies on each side should span at least $20\,\mu\text{m}$.

Capacitor arrays also require dummies. A typical array uses the outermost rows and columns of capacitors as dummies (see Figure 8.5), with a common poly-1 lower plate as an electrostatic shield. Both electrodes of dummy capacitors should connect to a suitable circuit node.

#### Dopant Interactions

Diffusion tails extend beyond the metallurgical junction. When two diffusions of the same polarity are adjacent, each tail increases the other's doping, reducing sheet resistance and increasing width. Opposite-polarity diffusions partially counterdope each other, increasing sheet resistance.

**Solutions:**
- Add dummy diffusions at array ends (same width as active segments)
- Keep spacing between serpentine turns consistent
- Keep base heads away from resistor bodies
- Keep $N^+$ sinkers away from resistor bodies (their deep junctions produce long tails)

**Well Proximity Effect (WPE):** High-energy well implant ions scatter from photoresist edges, doping surface layers up to $1\,\mu\text{m}$ away. Keep matched diffusions at least $2\,\mu\text{m}$ inside drawn well edges.

### 8.2.4 Interconnection Parasitics

Metal leads possess enough resistance and capacitance to cause significant mismatches.

**Resistor arrays:** A minimum-width metal jumper with $R_s = 0.05\,\Omega/\Box$ can add 1% to a $5\,\Omega$ segment. Mismatches accumulate if jumpers have different lengths. **Solutions:**
- Make jumpers equal resistance (proportional to segment resistance)
- Widen jumpers, use multiple vias
- Insert jogs in short jumpers; insert vias in all jumpers for consistency (Figure 8.8)

**Capacitor arrays:** A $10\,\mu\text{m}$-wide metal-1 lead over field oxide has about $13\,\text{fF}$ of capacitance -- 1.3% of a $1\,\text{pF}$ capacitor. **Solutions:**
- Adjust leads so every unit capacitor sees the same lead capacitance
- Insert jogs or dead-end branches to equalize lead lengths (Figure 8.9)
- Keep lead widths uniform
- Use electrostatic shields (e.g., metal-2 plate over poly-poly capacitor array)

Electrostatic shielding simplifies fringing-field computation but does not fully block high-frequency/high-slew signals. Never route digital clock lines across shielded capacitors.

### 8.2.5 NBL Shadow

During epitaxial growth, surface discontinuities from a patterned N-buried layer (NBL) can shift laterally (**pattern shift**), distort (**pattern distortion**), or vanish (**pattern washout**). On (111) wafers, pattern shift can be 50-150% of epi thickness. On (100) wafers, distortion occurs without shift (tilted wafers trade distortion for shift).

If the NBL shadow intersects a matched resistor, it can alter its value. Shallow resistors like HSR are especially vulnerable.

**Solutions:**
- Remove NBL from beneath resistors (but increases tank resistance)
- Increase NBL overlap on the pattern-shift side, with allowance for alignment errors
- Processes using STI (CMP planarization) do NOT exhibit NBL shadow

### 8.2.6 Hydrogenation

Silicon nitride deposition introduces hydrogen ($\sim 20\%$ atomic fraction). During high-temperature steps, hydrogen diffuses through oxide to silicon surfaces where it passivates dangling bonds (**hydrogen passivation**), reducing trap density, surface recombination, and $1/f$ noise.

**Effects on polysilicon resistors:**
- **High-sheet poly:** Hydrogen passivates grain boundaries, reducing $R_s$ by up to 30%. Grain-boundary dehydrogenation causes long-term drift in boron-doped HSR poly.
- **Low-sheet boron-doped poly:** Hydrogen compensation slightly increases $R_s$ by neutralizing boron atoms.
- **N-doped poly:** Donor compensation mechanism exists but is negligible.

**Critical layout issue:** Aluminum metal **blocks hydrogen diffusion** and some metal system materials actively **getter hydrogen**. Metal plates or leads above poly resistors can cause up to 10% systematic mismatch between metallized and unmetallized resistors.

**Solutions:**
- **Folded-out jumpers** (Figure 8.12B) are superior to folded-in jumpers (Figure 8.12A) because they minimize metal overlap over resistors
- Place a metal-1 shield over the entire poly array; route metal-2 jumpers above it
- If no shield is used, block all metal (including dummy metal) from crossing active resistor segments
- Block dummy metal generation on all metal layers near unshielded resistors

### 8.2.7 Temperature

#### Thermal Gradients

Power devices are heat sources. Fourier's law governs heat flow:

$$q = -\kappa \nabla T \tag{8.22}$$

where $q$ is thermal flux density ($\text{W/cm}^2$), $\kappa$ is thermal conductivity, and $\nabla T$ is the thermal gradient (${}^\circ\text{C/cm}$).

| Material | $\kappa$ (W/cm-K) |
|---|---|
| Silicon | 1.56 |
| SiO$_2$ | 0.013 |
| Copper alloys | 3.0-3.6 |
| Mold compounds | 0.008-0.021 |
| Solder (SAC305) | 0.59 |
| Silver sinter | 0.80 |

Heat-sunk packages exhibit large lateral thermal gradients near power devices (heat flows vertically down through die attach). Non-heat-sunk packages are nearly isothermal in steady state but experience transient gradients.

#### Centroids

Under four simplifying assumptions (planar active region, uniform contribution, linear temperature coefficient, constant thermal gradient), the **average temperature** of a device equals the temperature at its **centroid** (geometric center of its active area).

The mismatch between two equal matched resistors due to temperature:

$$\delta = \alpha \cdot \nabla T \cdot d \tag{8.23}$$

where $\alpha$ is the temperature coefficient of resistance, $\nabla T$ is the thermal gradient, and $d$ is the distance between centroids.

Four ways to minimize temperature-induced mismatch:
1. Use materials with lower $\alpha$
2. Place devices where $\nabla T$ is smaller
3. Orient devices so $\nabla T$ is perpendicular to the axis between centroids
4. Minimize separation between centroids (ideally make them coincide)

#### Common-Centroid Layout

Devices whose centroids exactly coincide have a **common centroid** -- Equation 8.23 predicts zero temperature-induced mismatch. In practice, a small residual remains due to nonlinearities.

**The Four Rules of Common-Centroid Layout (Table 8.4):**

| Rule | Description |
|---|---|
| **1. Coincidence** | Centroids of matched devices should coincide (at least approximately) |
| **2. Symmetry** | Array should be symmetric about both horizontal and vertical axes |
| **3. Dispersion** | Subdivide large arrays into smaller common-centroid subarrays; only subarrays need obey rules 1 and 2 |
| **4. Compactness** | Array (or subarrays) should be as compact as possible |

**Why dispersion matters:** The residual mismatch after common-centroid cancellation comes from the quadratic and higher-order terms in the Taylor expansion of the temperature distribution. This residual is proportional to the *square* of the array dimensions. Subdividing ABBAABBA into two ABBA subarrays reduces quadratic residue by half compared to AABBBBAA.

**One-dimensional interdigitation patterns:**
- ABBA (best basic pattern; has axis of symmetry)
- ABAB (does NOT have a common centroid -- inferior to ABBA)
- ABA (implements 2:1 ratio)

**Two-dimensional cross-coupled pairs** (Figure 8.16): Arrange segments in rows and columns (e.g., AB/BA). More compact than 1D arrays; partially suppresses quadratic residue. Ideal for capacitors (square segments).

**Optimum interdigitation patterns for various ratios:**
| Ratio | Pattern |
|---|---|
| 1:1 | ABBA |
| 2:1 | ABA |
| 3:1 | AABAABAA |
| 3:2 | ABABA |
| 4:1 | AABAA |
| 5:3 | ABAABABAABABAABA |

#### Segmenting Resistors

Guidelines for segment dimensions:
- **Width:** At least 150% of minimum for minimal matching; 300%+ for high accuracy
- **Length:** At least 3x minimum for minimal; 5x+ for accurate matching
- **Area (TiSi$_2$):** Must exceed minimum to avoid C49-to-C54 recrystallization failure (bimodal resistance distribution at 3-5x mean). CoSi$_2$ and NiSi$_2$ do not have this problem.

**Segmentation sensitivity** $\eta$ quantifies how sensitive a non-integer ratio becomes to head resistance errors [Eq. 8.27]. Use spreadsheets to find optimal segment counts that minimize $\eta$.

#### Placement Relative to Heat Sources

- Single heat source: Place at one end of die, matched devices about halfway to far edge (Figure 8.21A)
- Two heat sources: Both at same end (Figure 8.21B), or one at each end with matched devices in middle (Figure 8.21C)
- Four heat sources: One at each end, matched devices centered (Figure 8.21D); may need 1.5:1 or 2:1 die aspect ratio

#### The Thermoelectric (Seebeck) Effect

When contacts at opposite ends of a resistor are at different temperatures, a voltage develops:

$$V_{th} = \alpha_S \cdot \Delta T \tag{8.34}$$

where $\alpha_S$ is the Seebeck coefficient (50 to $500\,\mu\text{V/K}$ for Al-Si contacts). A $2\,{}^\circ\text{C}$ difference with $\alpha_S = 50\,\mu\text{V/K}$ produces $0.1\,\text{mV}$ -- enough to cause 0.4% mismatch in a bipolar current mirror.

Common-centroid layout **cannot** cancel thermoelectrics (they arise from temperature differences *along* each segment, not *between* segments). **Solution:** Connect an even number of series segments alternating direction so thermoelectric potentials cancel (Figure 8.22B). Place contacts of serpentine resistors close together with opposite head orientations (Figure 8.23C).

### 8.2.8 Mechanical Stress and Package Shift

Silicon is **piezoresistive** -- resistivity changes under mechanical stress. Capacitors are also affected but much less so (dimensional and permittivity changes largely cancel).

The stress-induced resistance shift:

$$\frac{\Delta R}{R} = \pi_L \sigma_L + \pi_T \sigma_T + \pi_S \tau_S \tag{8.38}$$

where $\pi_L$, $\pi_T$, $\pi_S$ are the longitudinal, transverse, and shear piezoresistance coefficients; $\sigma_L$, $\sigma_T$ are normal stresses along/across the resistor; and $\tau_S$ is the in-plane shear stress.

**Piezoresistance coefficients (units: $10^{-12}\,\text{Pa}^{-1}$, for doping $< 10^{17}\,\text{cm}^{-3}$):**

| Orientation | $\pi_L$ (P-type) | $\pi_L$ (N-type) |
|---|---|---|
| $\langle 110 \rangle$ on (100) | 71.8 | -31.2 |
| $\langle 100 \rangle$ on (100) | 6.6 | -102.2 |
| Any on (111) | 71.8 | 29.7 |
| Polysilicon | 24 | 24 |

- N-type (100): min piezoresistivity along $\langle 110 \rangle$ (horizontal/vertical in layout) -- **these are the preferred orientations**
- P-type (100): min piezoresistivity along $\langle 100 \rangle$ ($45^\circ$ to wafer flat) -- awkward to digitize, so most designers use horizontal/vertical despite higher sensitivity
- Polysilicon: orientation-independent (random grain orientations cancel)
- Thin-film (NiCr): $\pi_L \approx 0.4$ -- orders of magnitude less sensitive than silicon

**Stress distribution on a die:**
- Mold compounds ($\text{CTE} \approx 8$-$35\,\text{ppm/K}$) shrink more than silicon ($\text{CTE} = 2.6\,\text{ppm/K}$) when cooling from cure temperature ($\sim 175\,{}^\circ\text{C}$)
- Resulting stresses are **compressive** and largest near die center, diminishing toward edges (except for anomalous edge effects)
- Shear stress is largest in die **corners** (causes delamination/metal damage)
- **Stress gradient** is smallest near die center, largest near edges and corners
- **Best locations for matched arrays: near die center, on axes of symmetry**

**Filler-induced stress:** Mold compound filler particles create random microscopic stress fluctuations. Modern low-stress compounds with rounded filler particles ($\geq 90\%$ loading) reduce this 2-3x versus older formulations. Elastomeric die overcoats ($\geq 40\,\mu\text{m}$ thick) provide further 3x improvement.

**Package shift phenomena:**
- **Stress relaxation:** Viscoelastic flow gradually reduces stress (mostly in first 1-2 hours at temperature)
- **Thermal hysteresis:** Package shift does not fully return after heating/cooling cycle
- **Moisture absorption:** Mold compound swelling relaxes compressive stress; baking reverses it
- **Incomplete curing:** Continued cure at high temperature increases shrinkage stress

**L-shaped resistors:** Series connection of horizontal and vertical segments of equal dimensions reduces piezoresistivity and eliminates orientation dependence of principal stresses. Effective piezoresistance coefficient: $\pi_{eff} = (\pi_L + \pi_T)/2$ [Eq. 8.39].

**Piezocapacitance** is very small ($\sim 0.3 \times 10^{-12}\,\text{Pa}^{-1}$ for dry oxide on (100) Si) -- several orders of magnitude less than piezoresistance. Capacitors can tolerate less optimal die locations than resistors.

### 8.2.9 Electric Fields

#### Voltage Modulation (Conductivity Modulation)

Electric fields can deplete or accumulate carriers in resistors:
- **Body/tank modulation:** Reverse-biased isolation junction depletes into resistor body. Base diffusion: $\sim 0.2\%/\text{V}$; HSR implant: several $\%/\text{V}$.
- **Solution:** Ensure identical body bias on all matched segments. Different operating voltages require separate tanks, each connected to the positive end of its resistor segment (Figure 8.28).

Leads routed over resistors also cause conductivity modulation. Metal-1 over HSR can produce $\sim 0.1\%/\text{V}$. **Never route unconnected leads over matched resistors.** Use electrostatic (Faraday) shielding -- a metal plate connected to a low-impedance reference node (Figure 8.30). Shield overhang of $\sim 3\,\mu\text{m}$ intercepts most fringing fields.

For very high-sheet resistors ($> 1\,\text{k}\Omega/\Box$), even a common shield can cause conductivity modulation. Use **segmented shields** -- individual metal plates over each segment, connected to that segment.

#### Charge Spreading

Hot carriers injected into oxide accumulate as negative charge at the oxide/nitride or oxide/mold-compound interface. This charge migrates laterally under electric fields, causing long-term conductivity modulation of high-sheet resistors. Electrostatic shielding (field plating) prevents this. Connect field plates to a potential close to the resistor body voltage.

#### Dielectric Absorption (Soakage)

Mobile charges within insulators slowly rearrange under external electric fields. Sources include:
- **Mobile ion contamination** (Na$^+$) in oxide -- causes long-term drift under bias
- **Phosphate groups** in PSG/BPSG -- polar groups shift slightly
- **Interface charges** in composite dielectrics (ONO) -- microsecond-scale relaxation

For resistors: causes time-dependent conductivity modulation in high-sheet devices. Observed ~0.1% variation in $2\,\text{k}\Omega/\Box$ HSR with BPSG.

**Split field plates** (Figure 8.32) combat dielectric absorption: the gap falls halfway along the resistor, so the electric field in one half opposes that in the other, and the absorption effects cancel. Recommended for diffused resistors with $R_s > 1\,\text{k}\Omega/\Box$ requiring better than 1% matching.

For capacitors: affects charge-retention circuits (integrators, floating-gate). Grown oxides and LPCVD oxides are preferred over TEOS-derived oxides for circuits requiring accurate high-frequency capacitor matching.

---

## Diagrams

### Common-Centroid Interdigitated Arrays (Figure 8.15)

![[diagrams/ch08-mismatch-causes-fig1.png]]
*One-dimensional common-centroid array examples. (A) ABBA pattern -- the simplest common-centroid layout with axis of symmetry. (B) ABAB pattern -- does NOT have a common centroid and is inferior. (C) ABA pattern implementing a 2:1 ratio. Dummies should be placed at both ends of any array.*

### Resistor Array Interconnection: Folded-In vs. Folded-Out Jumpers (Figure 8.12)

![[diagrams/ch08-mismatch-causes-fig3.png]]
*Comparison of (A) folded-in jumpers (metal routed over resistor bodies -- causes hydrogenation-induced mismatch) versus (B) folded-out jumpers (metal kept outside resistor bodies -- provides better matching). Even with dummy metal to equalize metal coverage, folded-out arrays match better.*

### Stress Distribution Across a Die (Figure 8.26)

![[diagrams/ch08-mismatch-causes-fig2.png]]
*Isobaric contour plot of minimum principal stress on a plastic-encapsulated die, with cross-section graphs. Stress is compressive and most intense near die center. The stress GRADIENT is smallest near center and largest near edges/corners. Best location for matched arrays is near die center on axes of symmetry.*

---

## Practical Takeaways

### Random Variation
- Random mismatch scales as $1/\sqrt{A}$ (Pelgrom's law) -- to halve mismatch, quadruple area
- The smaller device in a mismatched pair dominates the random mismatch
- Use parallel segments to effectively increase the area of smaller resistors without changing total resistance
- Poly resistor widths should greatly exceed grain diameter ($\sim 1\,\mu\text{m}$) to avoid bamboo effects

### Process Biases & Proximity
- **Always** use identical segment widths for matched resistors
- **Always** use identical segment lengths (or minimize segmentation sensitivity for non-integer ratios)
- Add **dummy segments** at array ends (full-width for moderate matching; $\geq 20\,\mu\text{m}$ deep for exceptional)
- Space dummy segments identically to active segments
- For matched capacitors, use identical unit capacitors connected in parallel (not series)

### Interconnection
- Equalize jumper resistance across all segments (add jogs, extra vias)
- Equalize lead capacitance to each unit capacitor (jogs or dead-end branches)
- Keep current-carrying leads short and wide; use Kelvin connections when needed

### NBL Shadow
- Keep NBL shadow away from matched resistors; increase overlap on pattern-shift side
- STI processes are immune to NBL shadow (CMP removes it)

### Hydrogenation
- Metal blocks/getters hydrogen: never route unshielded metal over matched poly resistors
- Use folded-out jumpers or a metal shield over the entire poly array
- Block dummy metal generation near unshielded poly resistor arrays

### Temperature
- Use **common-centroid layout** (ABBA, cross-coupled pairs) to cancel linear thermal gradients
- Place matched devices far from heat sources, on axes of thermal symmetry
- Connect series segments in alternating directions to cancel thermoelectric potentials
- Use materials with low temperature coefficients for critical resistors

### Mechanical Stress
- Place matched arrays **near die center**, on die axes of symmetry
- Avoid die corners and edges (highest stress gradients)
- Use low-stress mold compounds ($\geq 90\%$ filler loading)
- Consider elastomeric die overcoats ($\geq 40\,\mu\text{m}$) for precision circuits
- Polysilicon and thin-film resistors are less stress-sensitive than monocrystalline silicon
- L-shaped (or horizontal+vertical paired) segments reduce net piezoresistivity

### Electric Fields
- Ensure identical body/tank bias on matched resistor segments
- Different operating voltages require separate tanks, each biased to its segment's positive end
- Use electrostatic shields (field plates) for $R_s > 1\,\text{k}\Omega/\Box$ or when leads must cross resistors
- Use split field plates for HSR resistors requiring $< 1\%$ matching to combat dielectric absorption
- Never route switching digital signals over matched resistors, even with shielding

---

## Relation to the Bigger Picture

Sections 8.1-8.2 establish the complete taxonomy of mismatch mechanisms and their countermeasures, forming the theoretical foundation for the quantitative matching rules presented in [[ch08-matching-rules]] (Section 8.3). These mechanisms -- random variation, process biases, proximity effects, interconnection parasitics, NBL shadow, hydrogenation, temperature gradients, mechanical stress, and electric fields -- reappear throughout the book whenever matching is discussed: in bipolar transistor matching (Section 10.2), diode matching (Section 11.3), and MOS transistor matching (Section 13.2). The discussion of resistor variability in [[ch06-resistor-variability]] (Chapter 6) provides the device-level context for understanding *why* certain materials exhibit better matching coefficients, while Chapter 8 tells you *how* to exploit that understanding through layout techniques like common-centroid interdigitation, dummy structures, electrostatic shielding, and optimal die placement.

---

## See Also
- [[ch08-matching-rules]]
- [[ch06-resistor-variability]]
