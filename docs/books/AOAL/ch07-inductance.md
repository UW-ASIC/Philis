---
title: "7.2 Inductance"
chapter: 7
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-7, inductors, RF, parasitics, quality-factor]
---

# 7.2 Inductance

> **Chapter 7: Capacitors and Inductors**

## Key Concepts

### Fundamental Physics

A current flowing through a conductor generates a magnetic field. If the current changes over time, conservation of energy demands that the changing energy in the magnetic field produce a voltage across the conductor. This relationship is expressed as:

$$V = L \frac{dI}{dt}$$

where $L$ is the **inductance** -- a constant of proportionality measured in henries (H). The concept was independently discovered by Michael Faraday (1831) and Joseph Henry (1832), though the first iron-core solenoidal inductor was arguably constructed by William Sturgeon in 1825.

A henry is a large amount of inductance. Typical integrated inductors have values of only a few tens of **nanohenries**. Such small inductances are virtually useless below 100 MHz, which is why traditional analog ICs contain no inductors. Only RF integrated circuits operating at GHz frequencies employ them, despite their limitations.

### Loop Inductance vs. Self-Inductance

An important subtlety: when computing inductance, one must consider not just the magnetic field around a single conductor, but the coupling of that field with the **return wire** completing the circuit. Most analytical formulas therefore compute the **loop inductance** (total inductance of a complete circuit), not the self-inductance of a single wire.

For a **circular loop** of wire suspended in a nonconducting medium (Figure 7.21A):

$$L = \mu r \left[ \ln\left(\frac{8r}{a}\right) - 2 \right]$$

where $r$ is the loop radius (to the centerline) and $a$ is the wire radius. The permeability $\mu$ quantifies the energy stored in the magnetic field of the surrounding material:

$$\mu = \mu_r \cdot \mu_0$$

where $\mu_0$ is the permeability of free space and $\mu_r$ is the relative permeability. Most IC materials have $\mu_r \approx 1$. High-permeability ferrites have been explored for integrated inductors, and copackaged inductors using copper spirals sandwiched between ferrite plates have been developed.

### Bondwire Inductance

The circular loop formula provides a rough estimate for bondwire inductance. A typical bondwire with ~1 mil diameter and ~1 mm length contributes roughly **5.6 nH**. Even this tiny inductance generates substantial voltage drops in circuits with fast-slewing currents. For example, a switching converter gate driver conducting peak currents of 1 A with rise/fall times of 10--20 ns implies $dI/dt \approx 10^8$ A/s. An inductance of 5 nH subjected to this slew produces voltage drops of at least **0.5 V** -- enough to pull diffusions above the supply or below ground. This is why:

- Gate drivers and high-speed output structures require thorough guard ringing
- Switching converters benefit from solder bumps or copper pillars instead of bondwires
- PCB layout must carefully minimize loop areas in critical circuit paths

### Solenoid Architecture

Discrete inductors use insulated copper magnet wire wrapped around a toroidal ferrite core. The inductance is:

$$L = \mu \frac{N^2 A}{2\pi R}$$

where $N$ is the number of turns, $A$ is the area of one turn, and $R$ is the toroid diameter. The key insight is that inductance scales as $N^2$ because every turn's magnetic field passes through all $N$ turns. 60 Hz laminated iron-core inductors can reach several henries.

### Planar Spiral Inductors

The solenoid shape does not lend itself to integration. Instead, designers use **planar spiral inductors**. Three common geometries exist:

- **Circular spiral** -- lowest series resistance for a given inductance, but hard to digitize
- **Octagonal spiral** -- good compromise between performance and ease of digitization
- **Square spiral** -- easiest to implement, but highest series resistance

A bondwire or lower-metal jumper is used to reach the innermost turn of the spiral.

![[diagrams/ch07-inductance-fig1.png]]
*Figure 7.22: Geometries of planar spiral inductors -- (A) circular, (B) octagonal, and (C) square. Metal-2 forms the spiral body; Metal-1 provides the underpass jumper to the center.*

Planar spirals are less effective than solenoids for two reasons:

1. **Inner turns are smaller** and generate less inductance
2. **Not all flux couples between turns** -- the magnetic field from larger outer turns does not fully pass through the smaller inner turns, so the $N^2$ multiplication is diminished

### Empirical Inductance Formula

For square and octagonal planar spirals:

$$L = \frac{K_1 \mu_0 N^2 d_{avg}}{1 + K_2 \rho}$$

where $N$ is the number of turns, $d_{avg} = (d_{out} + d_{in})/2$, and $\rho = (d_{out} - d_{in})/(d_{out} + d_{in})$. The empirical constants are:

| Geometry | $K_1$ | $K_2$ |
|----------|-------|-------|
| Square | 2.34 | 2.75 |
| Octagonal | 2.25 | 3.55 |

The inside diameter is:

$$d_{in} = d_{out} - 2Np$$

where $p$ (the pitch) equals the sum of the turn width and the spacing between adjacent turns.

**Example:** A square planar inductor 300 $\mu$m on a side with 10 turns, each 5 $\mu$m wide spaced 5 $\mu$m apart, has an inner diameter of $d_{in} = 300 - 2(10)(10) = 100$ $\mu$m and an inductance of about **18.6 nH**. This illustrates the practical upper bound of integrated inductance without high-permeability core materials.

### Symmetric Inductors

The basic spiral inductor is asymmetric -- one terminal connects to the outside, the other to the inside, and their electrical properties differ. This asymmetry can be eliminated by inserting **jumpers** into the spiral to create a symmetrical structure (Figure 7.23). Symmetrical inductors benefit:

- **Differential/balanced circuits** directly
- **Single-ended circuits** as well, because symmetry minimizes losses within the structure

### Integrated Transformers

Multiple inductors can be magnetically coupled to form transformers for coupling energy across isolation barriers or between different impedance levels. Integrated transformers find limited application because they suffer even more from parasitic loss mechanisms than individual inductors. They are used as power combiners and baluns in certain RF ICs.

## Inductor Parasitics (Section 7.2.1)

Integrated inductors rarely exceed ~100 nH, making them useful only at very high frequencies -- precisely where parasitics are worst. This fundamental tension means integrated inductors seldom match discrete counterparts.

### DC Resistance (DCR)

The DC winding resistance follows:

$$R_{DC} = \frac{\rho \ell}{A}$$

where $\rho$ is the resistivity, $\ell$ is the winding length, and $A$ is the cross-sectional area. Integrated inductors have much larger DCR than discrete parts because IC metallization is far thinner than magnet wire. The thinnest standard magnet wire (#40 AWG) has a cross-sectional area of $5 \times 10^{-4}$ cm$^2$. An integrated inductor using five layers of 8000 A metal stacked together would need turns 12.5 $\mu$m wide to match this area. Thin aluminum films are also ~60% more resistive than copper.

**Ways to reduce DCR:**
- Wider turns
- Stacking multiple metal layers
- Using a dedicated thick (copper) metal layer
- Best: wide spiral of thick top-level metal with short jumpers from stacked lower layers

### AC Resistance (ACR) -- Eddy Current Losses

Three types of eddy current losses contribute to AC winding resistance:

#### 1. Skin Effect

Time-varying current in a conductor generates eddy currents within the conductor itself that cancel current flow in the interior, forcing high-frequency currents to flow near the surface. The **skin depth**:

$$\delta = \sqrt{\frac{\rho}{\pi \mu f}}$$

where $\rho$ is the conductor resistivity, $\mu$ is its permeability, and $f$ is the frequency. For thin-film aluminum, $\delta \approx 2.5$ $\mu$m at 1 GHz. The skin effect has little impact on integrated metal systems well below 1 GHz.

#### 2. Proximity Effects

When conductors lie adjacent to each other, time-varying currents in one induce eddy currents in the other. Currents flowing in the same direction crowd away from each other; opposite-direction currents crowd toward each other. Either way, current crowding reduces effective conductor area and increases AC resistance. Losses grow with more turns and especially with more winding layers -- this **discourages multilayer planar inductors**.

#### 3. Substrate Eddy Losses

The magnetic field from the inductor penetrates into the silicon substrate and generates eddy currents. These would be negligible if the substrate had very high or very low resistivity, but moderate resistivity (typical of CMOS/BiCMOS) creates large losses. This is the dominant AC loss mechanism.

**Mitigation strategies:**
- **High-resistivity substrate** ($> 10$ $\Omega \cdot$cm, preferably $> 20$ $\Omega \cdot$cm) -- but causes substrate debiasing and latchup issues
- **MEMS cavity etching** beneath the inductor -- adds cost and requires specialized equipment
- **Magnetic shielding** (thick nonconductive high-permeability material) beneath the inductor -- redirects flux away from silicon but introduces core losses above 100 MHz--10 GHz
- **Elevating the inductor** on higher metal layers to increase distance from substrate
- **Patterned ground shield** (see below)

### Subcircuit Model

![[diagrams/ch07-inductance-fig2.png]]
*Figure 7.24: Lumped-element subcircuit model for an integrated inductor on moderately doped silicon, along with discussion of the slotted ground shield technique.*

The model includes:
- **$L$**: the desired inductance
- **$R_s$**: series resistance (DCR + ACR combined)
- **$C_1$, $C_2$, $C_3$**: parasitic capacitances (interwinding and to substrate), difficult to compute analytically
- **$R_1$, $R_2$**: substrate resistances

Current crowding in a single-layer spiral becomes significant beyond a **critical frequency**:

$$f_{crit} = \frac{3.1}{\mu_0 p^2 / R_{sh}}$$

where $p$ is the pitch (metal width + spacing) and $R_{sh}$ is the sheet resistance of the metal. For example, a spiral from 8000 A aluminum with pitch 20 $\mu$m and width 10 $\mu$m, with $R_{sh} = 0.03$ $\Omega/\square$, gives $f_{crit} \approx 74$ GHz.

For an inductor whose inner diameter is approximately one-third of its outer diameter, the effective winding resistance:

$$R_{wind}(f) = R_{DC} \cdot \left(1 + \frac{f^2}{f_{crit}^2}\right)$$

### Quality Factor

The **quality factor** $Q$ quantifies the ratio of maximum stored energy to energy lost per cycle:

$$Q = \frac{2\pi f L}{R_s}$$

Key takeaways about $Q$:
- **Lower parasitics** mean higher $Q$
- $Q$ **rises with frequency** until it peaks, then rolls off due to AC resistance effects
- Peak $Q$ of integrated inductors ranges from ~1 to ~40
- Discrete air-core inductors easily exceed $Q = 100$

### Series Resonant Frequency (SRF)

Parasitic capacitances transform the inductor into a **series resonant LC tank**. Below the SRF, the impedance is principally inductive. At the SRF, capacitive and inductive reactances are equal. Above the SRF, the impedance becomes capacitive. The SRF represents the **upper frequency limit** for the inductor. Integrated planar inductors typically have SRFs well above 1 GHz.

The parasitic capacitances and substrate resistances in the model are difficult to compute by hand. Circuit designers rely on **finite element analysis** (3D or 2.5D) tools such as Ansys HFSS and Keysight EMPro.

## Inductor Construction (Section 7.2.2)

While any layout designer can draw a spiral, analyzing its performance requires specialized finite-element tools. Some tools (e.g., MIDAS) can automatically generate and iteratively optimize layouts from design criteria.

### The Substrate Problem

The greatest challenge for standard CMOS/BiCMOS inductors is **eddy current losses**. Most processes use moderate-resistivity epitaxial layers on low-resistivity substrates, limiting quality factors to 5--10.

### Elevating the Inductor

Without radical process changes, one can elevate the inductor above the silicon surface using:
- A dedicated **thick metal layer** deposited above the standard metallization stack
- Ideally above the **protective overcoat** (passivation)
- A patterned **polyimide layer** above the overcoat for even greater elevation

### Patterned Ground Shield

A metallic shield inserted between the inductor and the underlying silicon reduces parasitic substrate resistance and increases $Q$. However, the shield must be **slotted radially** to interrupt eddy currents. The slots must be oriented perpendicular to the direction of current flow in the spiral above.

![[diagrams/ch07-inductance-fig3.png]]
*Figure 7.25: Slotted ground shield for placement underneath an integrated inductor. The radial slots interrupt eddy currents while the shield still reduces parasitic resistance to the substrate. Silicided poly provides the best combination of low shield resistance and minimum parasitic capacitance.*

The metal strip widths between slots must satisfy the critical frequency equation ($f_{crit}$, Eq. 7.38) to ensure eddy losses within the shield remain negligible at the operating frequency.

### Thick Metal and Metal Strapping

Thicker metallization reduces winding resistance and improves $Q$. Options include:
- **Strapping** several metal layers together with vias (conflicts with the goal of maximizing distance from substrate)
- In a 4--5 metal process, combining the **top 2--3 layers** is typically beneficial
- Many **RF processes** include a special thick metal layer (copper or aluminum), typically ~3 $\mu$m thick, above standard metallization or atop the protective overcoat

### State of the Art

The best integrated inductors provide values up to ~100 nH with quality factors around 40 at frequencies up to a few GHz. Researchers have created a fully integrated buck converter processing 0.5 W at 75% efficiency in CMOS with two added thick metal layers. Copackaging discrete inductors with ICs offers a path to higher performance.

## Practical Takeaways -- Guidelines for Integrating Inductors

These 11 rules, largely based on Long and Copeland's foundational work, summarize best practices:

1. **Use the highest-resistivity substrate available.** Substrate resistivities below ~10 $\Omega \cdot$cm create large eddy losses that severely reduce $Q$ at high frequencies. Use guard rings and scattered substrate contacts to mitigate debiasing and latchup risks.

2. **Place inductors on the highest possible metal layers.** The spiral body goes on top metal; the jumper to the innermost turn goes underneath. This minimizes parasitic capacitance and slightly reduces substrate eddy losses.

3. **Consider strapping 2--3 metal layers together for the spiral body.** This reduces sheet resistance and increases $Q$, but avoid using first-level metal (too close to substrate).

4. **Keep unconnected metal away from inductors.** Minimum clearance should be at least **half the width of the completed inductor**, preferably more. Do not place circuitry in the empty center of the spiral -- it reduces $Q$.

5. **Avoid excessively wide or narrow metallization.** For inductors operating around 1 GHz, a width of ~10 $\mu$m is about optimal. Narrower leads have excessive DCR; wider ones suffer skin and proximity effect losses.

6. **Use the narrowest possible spacing between turns.** Tighter spacing enhances magnetic coupling, produces higher inductance and $Q$, and allows wider metal (reducing DCR). Interwinding capacitances from narrow spacing are relatively inconsequential in single-layer planar inductors.

7. **Minimize the number of inductor layers.** Proximity effects cause losses in multilayer inductors, and interwinding capacitances between layers drastically lower the SRF. Practical designs are limited to one, or at most two, layers. (Multiple metal layers strapped with vias count as a single winding layer.)

8. **Do not fill the entire inductor with turns.** The magnetic field is most intense at the center. Turns occupying this region suffer severe eddy losses and current crowding. The inside diameter should be at least **5x the metal width**, and for larger inductors, at least **1/3 of the outer diameter**.

9. **Do not place metal or poly above or below an inductor** (except slotted shields). Conductive plates generate large eddy losses unless radially slotted. Route all non-inductor leads around the structure, not through it. Remove dummy metal and poly fill out to roughly half the inductor width. If metal density rules require fill, size and place dummy shapes to minimize eddy losses.

10. **Do not place junctions beneath inductors.** High-frequency AC signals coupled into junctions can be rectified, causing parasitic losses or injecting unexpected currents into diffusions. Apply the same clearance rules as for metal leads.

11. **Keep inductor leads short and direct.** Leads contribute their own parasitics, so minimize their length and area. Use the highest possible metal layer for leads to reduce parasitic capacitance to substrate.

## Relation to the Bigger Picture

Section 7.2 complements the capacitor material in [[ch07-capacitance]] by completing the treatment of passive reactive components available on-chip. While capacitors are ubiquitous in analog IC design, inductors occupy a specialized niche -- they matter primarily in RF circuits operating above 1 GHz. The extensive discussion of parasitics (substrate eddy losses, skin effect, proximity effects) and the detailed integration guidelines connect directly to the broader themes of Chapter 7: that every integrated component carries parasitic baggage, and that layout choices profoundly affect electrical performance. The quality factor limitations of integrated inductors also motivate the matching and optimization techniques covered in Chapter 8 (Matching of Resistors and Capacitors), where understanding and controlling device parasitics is equally critical.

## See Also
- [[ch07-capacitance]]
