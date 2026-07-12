---
title: "10.1 Power Bipolar Transistors"
chapter: 10
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-10, power-transistors, bipolar, secondary-breakdown, thermal-runaway, emitter-ballasting]
---

# 10.1 Power Bipolar Transistors

> **Chapter 10: Applications of Bipolar Transistors**

## Key Concepts

Power bipolar transistors are operated in **high-level injection** to minimize die area, which means they run at lower betas than small-signal devices. A beta of 10 is often taken as the minimum acceptable value. NPN transistors are preferred over PNP for power applications because NPN inherently achieves higher beta, and processes are typically optimized for NPN. A standard bipolar power NPN can conduct emitter current densities up to roughly $1 \text{ mA}/\mu\text{m}^2$.

Small-signal NPN layouts are adequate up to about 10 mA / 100 mW. Beyond that, failure mechanisms become increasingly dangerous, and above 100 mA / 500 mW they become acute. Special layouts can handle currents exceeding 10 A and power levels above 100 W, but only if the designer thoroughly understands the three primary failure mechanisms: **emitter debiasing**, **thermal runaway**, and **secondary breakdown**.

The operating mode of the transistor dictates which layout to use:

- **Linear-mode** transistors remain in forward-active for extended periods and must withstand simultaneous large $V_{CE}$ and large $I_C$. FBSOA is paramount. Conservative guidelines: no more than $\sim 5 \text{ mW}/\text{mil}^2$ of emitter, no more than $\sim 2 \text{ mA}/\text{mil}$ of emitter.
- **Switched-mode** transistors alternate between cutoff and saturation, with power dissipated mainly during brief transitions. RBSOA is usually more important. Conservative current density limit: $\sim 0.5 \text{ mA}/\text{mil}$ of emitter.
- **Pulsed-mode** transistors (e.g., driving capacitive MOS gates) are immune to both emitter current focusing and thermal runaway because the pulse ends before the load drains. They can tolerate much higher current densities ($\sim 4 \text{ mA}/\text{mil}$ average) provided pulse duration $\leq 1 \,\mu\text{s}$, inter-pulse intervals $\geq 250 \text{ ns}$, and electromigration rules for intermittent currents are followed.

---

## Failure Mechanisms (10.1.1)

### Emitter Debiasing

Emitter debiasing is a **nonuniform current distribution** caused by voltage drops in the extrinsic base, emitter regions, and their metal leads. Because bipolar transistors have extremely high transconductance, even millivolt-scale $V_{BE}$ differences can produce dramatic current imbalances. For two lumped-element transistors with a $V_{BE}$ difference of $\Delta V_{BE}$, the emitter current ratio is:

$$\frac{I_{E1}}{I_{E2}} = e^{\Delta V_{BE}/V_T}$$

where the thermal voltage $V_T = kT/q \approx 25.7 \text{ mV}$ at $25^\circ\text{C}$ (Eq. 10.1). A mere 6 mV difference causes a 26% current imbalance.

**Emitter ballasting** counteracts this by inserting resistances in series with each emitter finger. These resistors create localized negative feedback: as current rises through a finger, the voltage drop across its ballasting resistor increases, reducing $V_{BE}$ and thus limiting the current. The widely accepted rule is that ballasting resistors should generate voltage drops of **2--3 times $V_T$**, i.e., **50--75 mV** at room temperature.

**Intrafinger debiasing** occurs along the length of a single emitter finger. The voltage drop from one end to the other is given by:

$$\Delta V_E = \frac{R_{\square} \cdot L \cdot I_E}{2W}$$

where $R_{\square}$ is the metallization sheet resistance, $L$ is the emitter contact length, $W$ is the emitter finger lead width, and $I_E$ is the total finger current (Eq. 10.2). This drop should not exceed about **5 mV**. Remedies include double-level metal, shorter/wider fingers, or distributed emitter ballasting.

### Thermal Runaway and Secondary Breakdown

The **forward-bias safe operating area (FBSOA)** bounds operation in the forward-active region. It is limited by:

1. Maximum collector current (horizontal bound)
2. Maximum $V_{CE}$ -- typically $BV_{CEO}$ (vertical bound)
3. Maximum continuous power dissipation (diagonal bound on log-log plot)
4. **Secondary breakdown** region (further constraining upper-right of SOA)

Secondary breakdown was first identified in 1958. The mechanism: when a large-emitter transistor dissipates significant power, the center of the emitter (where heat can only flow vertically) becomes hotter than the edges (where heat spreads laterally too). Since $V_{BE}$ has a negative temperature coefficient of about $-2 \text{ mV}/^\circ\text{C}$, the hotter center conducts more current, which heats it further. This positive feedback loop is **thermal runaway**, and it collapses conduction into an ever-shrinking **hot spot**.

If the hot spot reaches $\sim 300^\circ\text{C}$, thermal generation in the collector-base depletion region supplies additional base drive. Unless externally limited, the temperature continues rising until metallization melts. Aluminum forms a eutectic with silicon at $577^\circ\text{C}$, producing a visible filament drawn between contacts by the "electron wind." This filament is the classic failure signature of secondary breakdown.

A stable hot spot can form if the base resistance is large enough that beta rolloff from high-level injection stabilizes the spot before destructive temperatures are reached. However, such a stable hot spot still dangerously overstresses the transistor and accelerates electromigration and other failure mechanisms.

**Electrical filamentation** (distinct from thermal runaway) occurs when current density triggers avalanche injection. The resulting filament volume is so small it melts almost immediately. This is the typical failure mode during ESD strikes on NPN transistors.

### Emitter Current Focusing (RBSOA)

During turnoff of an inductive load, high $I_C$ and high $V_{CE}$ occur simultaneously. The **reverse-bias safe operating area (RBSOA)** governs this regime. The periphery of the emitter turns off first because charge extraction from the pinched base is easiest there, while the center turns off last. During the final stages, nearly all current flows through a tiny area in the emitter center. If this current density exceeds the avalanche injection threshold, electrical filamentation destroys the transistor.

RBSOA can be improved by designs favoring peripheral conduction: many small square emitters, or "hollow emitter" rings around base contacts. These sacrifice emitter ballasting to reduce pinched base resistance. **It is generally difficult to optimize a single transistor for both FBSOA and RBSOA simultaneously.**

---

## Power NPN Transistor Layouts (10.1.2)

### The Interdigitated-Emitter Transistor

The oldest power transistor style. Multiple narrow emitter fingers, each with its own dedicated ballasting resistor (typically $\sim 1$ square of emitter diffusion, giving $\sim 2.5 \,\Omega$ per finger). Each finger is sized for about 20 mA. Characteristics:

- **FBSOA**: Fair (with ballasting; poor without)
- **RBSOA**: Excellent (narrow fingers minimize pinched base resistance and current crowding)
- **Frequency response**: Excellent (narrow fingers reduce base transit time)
- Preferred emitter width: 8--12 $\mu$m. Narrower is faster and more robust but harder to metallize.
- Base contacts should flank **both sides** of **every** finger, including outermost fingers, to ensure uniform turnoff.
- Deep-$N^+$ sinker along both sides reduces NBL resistance by $4\times$; an unbroken ring reduces it further and simultaneously forms a hole-blocking guard ring to minimize substrate injection during saturation.

### The Wide-Emitter Narrow-Contact Transistor

Uses **distributed emitter ballasting** by placing narrow contacts within wide emitter fingers. The overlap of wide emitter over narrow contact creates a distributed network of:

- **Emitter resistance** (largest at periphery, smallest at center)
- **Pinched base resistance** (smallest at periphery, largest at center)

These two mechanisms are complementary. At low currents, distribution is uniform. As current rises, base-side debiasing pushes conduction outward toward the periphery, but emitter resistance there pushes it back inward. The resulting three-dimensional ballasting is much more effective than discrete per-finger resistors.

The emitter-side ballasting voltage is estimated as:

$$V_E = R_{\square E} \cdot d_E^2 \cdot J_E$$

where $R_{\square E}$ is emitter sheet resistance, $d_E$ is the emitter overlap of contact, and $J_E$ is emitter current density (Eq. 10.3). The base-side ballasting voltage:

$$V_B = \frac{R_{\square PB} \cdot d_E^2 \cdot J_E}{\beta}$$

where $R_{\square PB}$ is pinched base sheet resistance (Eq. 10.4). Typical emitter overlap of contact: $\sim 15\text{--}20 \,\mu\text{m}$.

Key rules:
- The narrow contact must **not** extend to the ends of the emitter finger; emitter must overlap contact ends by the same amount as it overlaps the sides.
- Metallization drops should not exceed 5--10 mV even with distributed ballasting.
- Base debiasing can be reduced $\sim 4\times$ by connecting both ends of a serpentined base lead.
- **FBSOA**: Excellent. **RBSOA**: Good (but not as good as interdigitated). Best all-round compromise for general-purpose applications.

### The Christmas-Tree Transistor

The emitter has a central spine with triangular prongs connected by narrow stripes that serve as built-in ballasting resistors. At high currents, base-side debiasing forces conduction to the periphery, where current must flow through the ballasting resistors. Provides **excellent FBSOA** due to extensive emitter ballasting. However, during turnoff, almost all conduction occurs through the spine -- a small fraction of total emitter area -- causing extremely high current density and likely filamentation. **Poor RBSOA; never use for switching or pulse-mode.**

### The H-Emitter Transistor

Similar to the Christmas-tree but the central spine is not contiguous. Trapezoidal prongs connect to emitter contacts through narrow diffusion strips acting as ballasting resistors. The "H"-shaped openings in the emitter give it its name. Separating emitter banks by significant distances ($\sim 100 \,\mu\text{m}$) can triple FBSOA performance due to reduced thermal interaction.

### The Cruciform-Emitter Transistor

An evolution of the wide-emitter narrow-contact design. The emitter consists of cross-shaped sections stacked end-to-end to form a continuous finger (typically $\sim 40 \,\mu\text{m}$ wide). Small square or circular contacts occupy the center of each cross, producing a **distributed three-dimensional ballasting effect** more efficient than the two-dimensional ballasting of the wide-emitter narrow-contact structure. Base contacts fill the notches between the cross arms.

- **FBSOA**: Excellent
- **RBSOA**: Good
- Drawbacks: small emitter contacts (mitigated by refractory barrier metal and Blech effect), and compact design can cause excessive localized heating at high power (mitigated by splitting into spaced sections).

### Layout Selection Summary

| Property | Interdigitated | Wide-Emitter NC | Christmas-Tree | Cruciform |
|---|---|---|---|---|
| **FBSOA** | Fair* | Good | Excellent | Excellent |
| **RBSOA** | Excellent | Good | Poor | Good |
| **Frequency** | Excellent | Good | Poor | Fair |
| **Compactness** | Poor | Good | Good | Excellent |
| **Emitter Sensing** | Excellent | Fair | Poor | Poor |

*With individually ballasted fingers; otherwise Poor.

---

## Power PNP Transistors (10.1.3)

### Power Substrate PNP

More compact and faster than lateral PNP, but the lightly doped substrate collector cannot handle more than a few tens of milliamps without excessive debiasing via front-side contacts. Power substrate PNP therefore generally requires **backside contact**:

- Wafer is back-ground as thin as possible (down to $\sim 200 \,\mu\text{m}$ from the normal $\sim 500 \,\mu\text{m}$).
- Solder mounting (Ti adhesion / Ni wetting / electrodeposited solder) or silver sintering replaces silver-filled epoxy (too resistive).
- Delamination between adhesion layer and silicon is the primary reliability threat; mitigated by thorough cleaning + in-situ Ar sputter before Ti deposition.
- Fused leadframes preferred over downbonds.
- Mechanical stress from CTE mismatch between silicon and leadframe is the most serious challenge; requires common-centroid layout of matched components.

Substrate PNP can use any emitter structure from the NPN designs. Interdigitated emitters are common because the substrate PNP's pronounced beta rolloff makes it far less prone to thermal runaway.

### Power Lateral PNP

Cannot increase emitter size without degrading beta, so they use **arrays of minimum-geometry emitters** in square or hexagonal arrays (hexagonal is slightly denser). A typical minimum-emitter lateral PNP handles only 0.25--1 mA before beta drops below 5, so power devices may contain 100+ individual emitters.

High-current beta rolloff actually acts as a natural form of ballasting -- if any portion conducts too much, its beta diminishes. Combined with the deep, robust base-epi junction, the device becomes **nearly indestructible**.

A **deep-$P^+$ (DP) process extension** creates a deeper, more heavily doped emitter diffusion that improves injection efficiency and sidewall injection fraction. DP laterals can operate at 2--3$\times$ higher current densities than base laterals, and since the power lateral PNP dominates LDO die area, this can reduce die size by more than half. The DP extension requires only a single mask.

---

## Advanced Power BJTs (10.1.4)

All standard bipolar power layouts adapt to CDI (collector-diffused isolation) analog BiCMOS processes. Double-level metallization enables:

- Complete base contact rings around each emitter finger (instead of 2--3 sides)
- Metal-2 plate over emitter dramatically reduces emitter debiasing
- Base grid in metal-1 with metal-2 jumper exit
- Complete $N^+$ ring as both low-resistance collector contact and hole-blocking guard ring
- Vias from narrow emitter contacts to metal-2 plate

For pulse-power gate-driver transistors: current densities exceeding $4 \text{ mA}/\text{mil}$ with emitter-over-base overlap of 8--10 $\mu$m and continuous $N^+$ ring $\geq 2\times$ epi thickness wide. NBL must overlap $N^+$ sinker to its outer edges.

### SiGe RF Power Transistors

SiGe power transistors for RF amplifiers operate at very high current densities ($\sim 1 \text{ mA}/\mu\text{m}^2$ of emitter). Low-current beta rolloff in SiGe is severe (often beginning at $10 \text{ mA}/\mu\text{m}^2$ or higher) due to recombination at the polycrystalline/monocrystalline interface and boron penetration into emitter poly.

SiGe transistor beta typically **diminishes** with increasing temperature (unlike conventional silicon where beta increases), which argues for **base-side ballasting**. However, base-side ballasting requires bypass capacitors to route high-frequency signals around the resistors. Emitter-side ballasting wastes power and reduces RF amplifier efficiency. A third approach avoids ballasting entirely by subdividing the transistor into many small sections placed far apart.

Ballasting in RF transistors uses **thin-film (polysilicon) resistors** rather than diffused resistors to avoid parasitic capacitances. Typical RF SiGe layouts use cellular arrays of very short, very narrow emitter fingers (several microns wide, $\sim 10 \,\mu\text{m}$ long) individually fitted with ballasting resistors, with deep trench isolation through epi, NBL, and substrate.

---

## Saturation Detection and Limiting (10.1.5)

Both vertical NPN and lateral PNP transistors inject holes into the substrate when they saturate, wasting power and potentially triggering latchup.

### Hole-Blocking Guard Ring

A continuous unbroken ring of $N^+$ (deep-N sinker) around the outside of the tank, merging with NBL, forms a high-low junction that holes must surmount. Doping ratios of $\geq 100:1$ between $N^+$/NBL and N-epi reduce substrate injection to acceptable levels.

### Secondary Collector (Ring Collector)

A ring of base diffusion encircling the primary collector of a lateral PNP. When the primary collector saturates, reinjected holes transit to the secondary collector. Functions:

- **As guard ring**: connect to ground -- collects minority carriers.
- **As beta limiter**: connect to base lead -- collected current adds to base current, reducing apparent beta until just sufficient to support primary collector current. Provides similar functionality to an $N^+$ ring with less area.
- **As saturation detector**: current begins flowing through secondary collector exactly at onset of primary saturation. Can drive dynamic antisaturation circuits that throttle base drive. These feedback loops may become unstable unless properly compensated. Phase shift across the secondary collector is notoriously hard to predict (no device model exists).

### NPN Saturation Detection

A base diffusion placed within the NPN collector acts as a saturation detector. Power switching transistors use dynamic antisaturation circuits to meter base drive. Even with antisaturation circuitry, hole-blocking guard rings should be included to protect against transients (antisaturation circuits cannot react instantaneously).

---

## Diagrams

### Figure 10.7 -- Interdigitated-Emitter Power Transistor Layout
![[diagrams/ch10-power-bjts-fig1.png]]
Layout of an interdigitated-emitter power transistor with individually ballasted emitter fingers. Each emitter finger connects to a pair of parallel ballasting resistors formed from emitter diffusion in a separate tank. The comb-style base metallization develops much less resistance than a serpentine route.

### Figure 10.4 -- Forward-Bias Safe Operating Area (FBSOA)
![[diagrams/ch10-power-bjts-fig2.png]]
Typical FBSOA plot (log-log, $I_C$ vs. $V_{CE}$) showing the bounds: maximum collector current (horizontal), $BV_{CEO}$ (vertical), maximum power dissipation (diagonal), and secondary breakdown region. Dotted curves show relaxed SOA for pulsed operation of various durations (duty cycle must remain below 5--10%).

### Figure 10.11 -- Cruciform-Emitter Transistor and Power Transistor Comparison
![[diagrams/ch10-power-bjts-fig3.png]]
The cruciform-emitter power transistor layout: cross-shaped emitter sections with small contacts at each cross center produce highly effective three-dimensional distributed ballasting. Also shows comparison table of the four main power NPN layout styles (interdigitated, wide-emitter narrow-contact, Christmas-tree, cruciform) across FBSOA, RBSOA, frequency response, compactness, and emitter sensing ease.

---

## Practical Takeaways

- **Emitter ballasting resistors** should generate 50--75 mV of drop (2--3$\times V_T$) to control interfinger debiasing. If total debiasing between any two fingers exceeds 100 mV, redesign the transistor.
- **Intrafinger debiasing** along a single emitter finger should not exceed 5 mV. Use double-level metal, shorter fingers, or distributed ballasting to achieve this.
- For **linear-mode** applications (sustained high $V_{CE}$ and $I_C$), use wide-emitter narrow-contact, Christmas-tree, or cruciform layouts. Never use Christmas-tree for switching.
- For **switching applications**, interdigitated-emitter or cruciform layouts are preferred (good RBSOA). Always ensure outermost emitter fingers have base contacts on both sides.
- Place deep-$N^+$ sinker rings around power NPN transistors: they simultaneously reduce collector resistance and form hole-blocking guard rings against substrate injection during saturation.
- **NBL should extend to at least the drawn outer edge of the $N^+$ sinker** to minimize resistance and block minority carriers.
- The **wide-emitter narrow-contact** structure is the best all-round compromise when a single layout must serve multiple operating modes.
- Emitter overlap of contact in wide-emitter narrow-contact transistors should be approximately 15--20 $\mu$m. Larger overlaps degrade frequency response; smaller overlaps may not prevent hot spots.
- For **substrate PNP power devices**, backside contact is usually necessary above a few tens of milliamps. Use solder or silver-sinter mounting, not silver-filled epoxy.
- **Power lateral PNP** transistors using a deep-$P^+$ process extension can handle 2--3$\times$ more current density, reducing LDO die size by more than half.
- Any saturating NPN consuming more than a few milliamps of base drive requires an $N^+$ hole-blocking guard ring. Antisaturation circuits should still include guard rings for transient protection.
- In SiGe RF power transistors, use polysilicon thin-film ballasting resistors (not diffused) to minimize parasitic capacitance.
- Base-side ballasting is preferred when beta decreases with temperature (SiGe, III-V HBTs); emitter-side when beta increases with temperature (conventional silicon).

---

## Relation to the Bigger Picture

This section bridges the gap between the small-signal bipolar transistor layouts of [[ch09-bjt-operation]] and the precision matching techniques of [[ch10-matching-bjts]]. Understanding failure mechanisms -- emitter debiasing, thermal runaway, and secondary breakdown -- is essential because these same phenomena degrade matching accuracy in less dramatic ways even in small-signal circuits. The distributed ballasting concepts introduced here (wide-emitter narrow-contact, cruciform) represent some of the most sophisticated layout techniques in analog IC design, where the physical geometry of the device directly determines its electrical robustness. The saturation detection and guard ring techniques are closely related to the minority carrier injection and latchup prevention strategies from Chapter 5.

---

## See Also
- [[ch10-matching-bjts]]
- [[ch09-bjt-operation]]
