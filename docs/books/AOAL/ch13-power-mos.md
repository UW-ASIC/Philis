---
title: "13.1 Power MOS Transistors"
chapter: 13
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-13, power-mos, DMOS, RESURF, SOA, high-voltage]
---

# 13.1 Power MOS Transistors

> **Chapter 13: Applications of MOS Transistors**

## Key Concepts

Power MOS transistors must handle large currents and/or high voltages, making their layout fundamentally different from small-signal devices. The three central concerns are:

1. **On-resistance ($R_{on}$)**: Comprises silicon resistance, metallization resistance, and packaging resistance. Minimizing $R_{on}$ requires careful attention to all three, but metallization resistance often dominates in low-voltage integrated devices.

2. **Switching losses**: During each on/off transition, both current and voltage are simultaneously nonzero, dissipating power. At high frequencies, switching losses can exceed conduction losses.

3. **Safe operating area (SOA)**: Unlike the ideal rectangular SOA boundary, real integrated MOS transistors suffer from electrical SOA (impact ionization triggering parasitic bipolar action) and electrothermal SOA (thermal runaway of the parasitic bipolar).

The fundamental tradeoff in power MOS design is between voltage handling and on-resistance. Higher voltage ratings demand wider, more lightly doped drift regions, which increase $R_{on}$. Techniques like RESURF and superjunctions exist specifically to break this tradeoff.

---

## On-Resistance and Metallization

### Specific On-Resistance

The figure of merit for power transistors is the **specific on-resistance**:

$$R_{on,sp} = R_{on} \cdot A$$

where $A$ is the drawn area and $R_{on}$ is measured on-resistance. Units are $\Omega \cdot \text{cm}^2$ or $m\Omega \cdot \text{cm}^2$. Packaging resistance is excluded (use Kelvin connections for measurement). Estimates degrade when extrapolating area by more than a factor of 2-3.

### Vertical vs. Lateral Devices

- **Vertical MOSFETs**: One terminal (source or drain) exits through the backside. Easier to metallize but harder to integrate (multiple devices share a terminal or need trench isolation).
- **Lateral MOSFETs**: Both terminals exit the topside. More difficult to metallize because source and drain compete for space, but multiple independent devices can share a die.

### Interdigitated Layout

The most common lateral power MOS layout uses interdigitated source/drain fingers (see Figure 13.1 in the text). The **pitch** $p$ of the interdigitated structure is constrained by three limits:

1. **Silicon limit**: $p_{Si} = L_{min} + W_{ct} + 2 S_{pc}$ (minimum channel length + contact width + 2x poly-to-contact spacing)
2. **Metal-1 limit**: $p_{M1} = W_{ct} + 2 M_{mc} + S_{mm}$ (contact width + 2x metal overlap of contact + metal spacing)
3. **Via limit**: $p_{via} = W_{via} + 2 M_{mv} + S_{mm}$ (via width + 2x metal-1 overlap of via + metal spacing)

Process designers attempt to balance these so metallization does not force a larger pitch than silicon.

### Dual-Level Metallization Pattern

The standard pattern uses metal-1 strips running down each source/drain finger, connected via rows of contacts to the source/drain. Two perpendicular metal-2 buses collect source and drain currents. Vias connect metal-1 fingers to their respective metal-2 buses wherever they cross.

When multiple metal layers are stacked on fingers, their combined sheet resistance is:

$$R_{s,eff} = \frac{R_{s1} \cdot R_{s2}}{R_{s1} + R_{s2}}$$

### The Rule of One-Third

This is the key tool for computing metallization resistance of power transistors. It states:

> The lateral metallization resistance of a singly terminated source/drain finger equals one-third of its end-to-end resistance if disconnected from the transistor.

$$R_{metal,finger} = \frac{1}{3} \cdot R_s \cdot \frac{L}{W_f}$$

where $L$ is the finger length, $W_f$ is the finger width, and $R_s$ is the sheet resistance. This rule is accurate to within 10% if finger length does not exceed the **penetration distance** $\lambda$, and within 20% for finger lengths up to $1.8\lambda$.

The penetration distance is:

$$\lambda = \sqrt{\frac{R_{Si} \cdot N}{R_s / W_f}}$$

where $R_{Si}$ is the silicon resistance of the entire transistor and $N$ is the number of sections.

**Critical guideline**: Designers should avoid creating layouts with finger lengths exceeding $1.8\lambda$. Beyond this, current distribution becomes highly nonuniform.

### Metal-1 Finger Resistance

For a transistor with $N$ sections, pitch $p$, and width $W_T$:

$$R_{M1} = \frac{1}{6N} \cdot R_{s1} \cdot \frac{W_T}{W_f}$$

### Metal-2 Bus Resistance

The bus resistance for the entire transistor:

$$R_{bus} = \frac{1}{6} \cdot R_{s2} \cdot \frac{N \cdot p}{W_T}$$

The penetration distance for buses uses:

$$\lambda_{bus} = \sqrt{\frac{4(R_{Si} + R_{M1})}{R_{s2} \cdot N \cdot p / (W_T/2)^2}}$$

### Bus Termination

Two common schemes exist: both buses exit the same end, or buses exit opposite ends. Counterintuitively, for short buses they have identical resistance. For long buses, same-end termination has less total resistance but worse current uniformity. **Prefer opposite-end termination** for buses longer than $2\lambda$ to ensure more uniform current distribution.

**Double termination** (connecting both ends of each bus) reduces bus resistance by a factor of four.

---

## Switching Losses

### Conduction vs. Switching

During each switching transition, the transistor simultaneously conducts current and sustains voltage. The energy dissipated per transition is:

$$E_{sw} = \frac{1}{2} V_{DS} \cdot I_D \cdot t_{sw}$$

The average switching power at frequency $f$:

$$P_{sw} = V_{DS} \cdot I_D \cdot t_{sw} \cdot f$$

Example: 10 A at 5 V with 10 ns transitions at 1 MHz produces 0.5 W of switching losses. This illustrates why nanosecond switching times and powerful gate drivers are essential in high-frequency converters.

### Gate Propagation Delay

The gate finger acts as a distributed RC network. The propagation delay for voltage to reach the far end of a finger:

$$t_{pd} = k \cdot R_{gate} \cdot C_{gate}$$

where $k = 1.0$ for 50% settling, 1.3 for 63%, and 2.0 for 90%. For practical estimation:

$$t_{pd} \approx k \cdot R_{s,poly} \cdot 2\epsilon_{ox} \cdot \frac{W^2}{t_{ox}}$$

The propagation delay scales with $W^2$ (width squared) and is independent of channel length. The maximum allowed gate finger width for a given delay $t_{pd,max}$:

$$W_{max} = \sqrt{\frac{t_{pd,max} \cdot t_{ox}}{k \cdot R_{s,poly} \cdot 2\epsilon_{ox}}}$$

**Connecting both ends** of each gate finger doubles the allowed width.

For transistors too wide for a single bank, divide into **multiple banks** where each bank width equals $W_{max}$.

---

## Safe Operating Area (SOA)

### Ideal vs. Real SOA

An ideal MOS power transistor has a rectangular SOA bounded by $V_{DS,max}$ (breakdown), $I_{D,max}$ (electromigration), and $P_{max}$ (thermal). Real integrated devices suffer two additional SOA reductions:

### Electrical SOA

Limited by **impact ionization** at high $V_{DS}$. Hot carriers generated in the pinched-off region create electron-hole pairs. Holes (in NMOS) flow to the backgate contact, causing debiasing. If the source-backgate junction becomes forward-biased, the **parasitic bipolar transistor** turns on. Its collector current fuels more impact ionization, creating positive feedback leading to snapback and potential destruction in microseconds.

Quantified by $BV_{DSS}$ -- the drain-to-source breakdown voltage under impact ionization. If $BV_{DSS} < BV_{DS}$, the transistor has electrical SOA limitations. The gate cannot control the parasitic bipolar once it triggers.

**Key parameters**:
- **Trigger voltage**: $V_{DS}$ at which snapback occurs
- **Sustain voltage**: $V_{DS}$ after snapback (lower than trigger)
- **Avalanche energy rating**: Maximum energy safely absorbed during a transient overload

**Countermeasures**:
- Reduce backgate resistance via distributed backgate contacts
- Place PSD plugs at ends of source fingers (where field intensification is worst)
- Use deeper, lightly doped drains to encourage vertical current spreading
- Add diffusions within the drift region to delay the Kirk effect (adaptive RESURF)

### Electrothermal SOA

Involves the same parasitic bipolar, but failure occurs over hundreds of microseconds rather than nanoseconds. The mechanism:

1. Impact ionization enables parasitic bipolar conduction
2. Assuming no immediate electrical filamentation, power dissipation causes localized heating
3. Heat spreads nonuniformly; the hottest point of the source-backgate junction conducts disproportionately
4. Progressive current localization into a **hot spot**
5. Either silicon melts or metallization alloys into silicon -- catastrophic short-circuit failure

**Prevention**: Minimize effective backgate resistance. Add or modify distributed backgate contacts.

### The Spirito Effect

A newer form of electrothermal SOA failure that does **not** involve parasitic bipolar action. It occurs in modern compact low-voltage power MOS transistors with very high transconductance. The saturated drain current rises with temperature if:

$$\left| \frac{dV_{th}}{dT} \right| > \left| \frac{d\beta}{dT} \right| \cdot \frac{(V_{GS} - V_{th})}{2\beta}$$

where $\frac{dV_{th}}{dT}$ is the threshold voltage temperature coefficient and $\frac{d\beta}{dT}$ is the transconductance temperature coefficient. Both are negative. Modern devices with high $g_m$ and small $V_{GS} - V_{th}$ can satisfy this inequality, making the MOS transistor behave thermally like a bipolar.

The **Spirito stability factor**:

$$S = I_D \cdot V_{DS} \cdot R_{th,JC} \cdot \left| \frac{1}{I_D} \frac{dI_D}{dT_j} \right|$$

If $S > 1$, thermal runaway is possible. Low $R_{th,JC}$ (good thermal design) helps suppress it.

### Backgate Resistance Strategies

- **With low-resistance sublayer** (e.g., NMOS on $P^+$ substrate, or PMOS in N-well touching NBL with $R_s < 50\ \Omega/\square$): Omit distributed backgate contacts; focus on low-resistance connection to the sublayer. For PMOS, surround with $N^+$ ring contacting NBL.
- **Without sublayer** (isolated P-tank, N-well without NBL): Must use interdigitated or distributed backgate contacts throughout the device (see [[ch12-constructing-cmos]] Section 12.2.9). Distributed contacts consume less area but require source-backgate electrical common and layout rule support.

---

## CMOS Power Transistors

### Metallization Design Process

1. Start with maximum $R_{on}$ specification. Guess metallization contributes ~1/3 of total $R_{on}$.
2. Subtract metallization estimate from total to get silicon resistance target.
3. Compute required total gate width, divide into fingers for convenient aspect ratio.
4. Check gate propagation delay constraints.
5. Apportion metal layers: fingers vs. buses.
6. Compute metallization resistance using the rule of one-third.
7. Iterate until convergence.
8. Verify electromigration compliance. Maximum current in source/drain fingers:

$$I_{finger,max} \approx \frac{I_D}{N}$$

### Alternate Metallization Patterns

**Partial-width buses** (Figure 13.16): Buses do not cover the entire transistor width. In the gap between buses, both metal layers are stacked on fingers. Relaxes electromigration constraints. Finger resistance approximated by:

$$R_{M1} = \frac{1}{6N}\left(\frac{W_T - W_{bus}}{W_f \cdot R_{s1}} + \frac{W_{bus}}{W_f \cdot R_{s,combo}}\right)^{-1}$$

**Diagonal transistor** (Figure 13.17): Sections placed diagonally so buses assume trapezoidal shapes. Buses are narrowest where current is least, widest where current is greatest. Allows buses to carry more current before electromigration. Drawback: wasted space beneath trapezoidal buses (use for bypass caps or substrate contacts).

### Alternate Layouts

**Waffle transistor**: A mesh of horizontal and vertical poly stripes creates an array of source/drain squares. Each source is surrounded by four drains and vice versa. Packing density improvement:

$$\frac{(W/L)_{waffle}}{(W/L)_{conv}} = \frac{S_{gg} + L_g + \frac{\pi}{4}L_g}{S_{gg} + L_g}$$

Provides ~64% more $W/L$ per unit area in typical processes. **However**, three crucial defects:
1. Metallization resistance is hard to optimize (diagonal fingers of unequal length)
2. 90-degree channel bends cause localized avalanche, reducing ESD robustness (mitigate with fillets/chamfers)
3. No provision for backgate contacts -- very susceptible to debiasing and latchup

**Bent-gate transistor** (Figure 13.19): Uses zigzag gate patterns with gentler bends (no 90-degree corners). Readily accommodates distributed backgate contacts. Provides source/drain ballasting for ESD robustness. Attractive for line drivers and applications with transient overloads. Metal-1 fingers run vertically; resistance computable via rule of one-third.

### SenseFETs

Current sensing without adding to $R_{on}$. Options:
- **Full-finger senseFET**: Allocate one entire source/drain finger (place in the middle for thermal averaging). Ratio limited (e.g., 49:1 for 100-section device).
- **Partial-finger senseFET**: Cut rectangular openings to segment a drain finger; connect selected segments as senseFET. Achieves ~490:1 ratios.
- **Series-connected senseFETs**: Multiple small senseFETs in series achieve extreme ratios (e.g., 4850:1). Width $W$, length $n \cdot L$ for $n$ series sections. Best matching approach for large ratios. Use dummy gates on either side for matching.

---

## High-Voltage Transistors

### Voltage Limitation Mechanisms

Three mechanisms limit $V_{DS}$:
1. **Punchthrough**: Depletion region width $x_d$ reaches the channel length. Avoid by increasing $L$.
2. **Avalanche breakdown**: Peak electric field $E_{max}$ exceeds ~$3 \times 10^5$ V/cm.
3. **Hot-carrier injection**: Occurs at lower fields than avalanche; frequently the limiting factor.

For a single-doped drain (SDD) with abrupt junction and uniform backgate doping $N_B$:

$$x_d = \sqrt{\frac{2 \epsilon_{Si} V_{DS}}{q N_B}}$$

$$E_{max} = \sqrt{\frac{2 q N_B V_{DS}}{\epsilon_{Si}}}$$

Both scale as $\sqrt{V_{DS}}$. Higher voltage ratings would naively require lighter backgate doping and longer channels, increasing $R_{on}$.

### The Drift Region Concept

The solution is a **lightly doped drift region** between the channel and the heavily doped extrinsic drain. The gate electrode ends just short of the metallurgical junction, allowing use of a thin low-voltage gate oxide even with high $V_{DS}$.

The fundamental tradeoff between breakdown voltage $BV_{DS}$ and specific on-resistance:

$$R_{on,sp} = 5.93 \times 10^{-9} \cdot BV_{DS}^{2.5}\ \ \Omega \cdot \text{cm}^2 \quad \text{(NMOS)}$$

$$R_{on,sp} = 1.76 \times 10^{-8} \cdot BV_{DS}^{2.5}\ \ \Omega \cdot \text{cm}^2 \quad \text{(PMOS)}$$

### RESURF (Reduced Surface Field)

RESURF circumvents the conventional $R_{on}$ vs. $BV_{DS}$ tradeoff by using interaction between multiple depletion regions to reduce the lateral electric field.

**Single RESURF**: A thin N-epi drift region sits atop P-epi. Two depletion regions (lateral from backgate, vertical from P-epi below) interact. Where they overlap, the lateral depletion must extend further to uncover sufficient charge, effectively reducing the surface field:

$$E_{lat} = E_{lat,0} \cdot \sqrt{1 - \frac{t_{drift}}{x_d}}$$

In the extreme (fully depleted drift region), the voltage rating approaches 2x the conventional value, or equivalently, drift doping can be doubled without increasing the field.

**Double RESURF**: Additional shallow P-type diffusion atop the drift region creates a second horizontal depletion interaction zone.

**Triple RESURF**: A P-type buried layer (PBL) via megavolt boron implant creates second and third horizontal depletion zones.

**Dielectric RESURF**: STI strips interdigitated with the drift region force the electric field to intrude into oxide, reducing field intensity in silicon. Uses existing STI layer -- no additional process steps.

**Superjunction**: Drift region interdigitated with narrow stripes of opposite doping polarity (high aspect ratio). Each depletes fully before max operating voltage. Theoretically achieves >100x lower $R_{on}$ than conventional. Used in discrete devices (e.g., Infineon CoolMOS) but difficult to implement in low-voltage integrated lateral devices.

**RESURF drawbacks**: Breakdown voltage becomes sensitive to charge balance variations (doping, thickness). Also susceptible to field-plate effects and charge spreading.

### Drain-Extended Transistors (DEMOS/DENMOS/DEPMOS)

Use a non-self-aligned composite drain: shallow heavily doped extrinsic drain inside a deeper, lighter drift diffusion (typically an N-well). The gate must overlap the N-well by more than worst-case misalignment; exact spacing is critical and determined via 2D device simulation.

**Key layout rules**:
- Use an even number of gate fingers with source contacts on both ends
- Spacing between drawn poly and drawn N-well must exactly match device designer specifications
- Drawn channel length is much longer than effective channel length (N-well outdiffusion)
- Poly/N-well misalignment sensitivity is minimized by using paired gate fingers around a single drain stripe (misalignment that shortens one channel lengthens the other)

**Asymmetric DENMOS**: Only one source/drain has a drain extension; only that terminal handles high voltage.

**Symmetric DENMOS**: Both source/drain regions have identical drain-extended structure. Either terminal can sustain high voltage (but not both simultaneously, due to gate oxide stress). Benefits from self-alignment. Longer minimum drawn channel but most consumed by outdiffusion.

**Drain-centric (annular) layout**: Gate completely encloses the drain, eliminating parasitic channels and field-intensifying corners. Elongated into "sausage" shapes for area efficiency.

### Field-Gapped Drain-Extended Transistors

A thick-field oxide region (LOCOS preferred for its gradual bird's beak transition) is placed beneath the drain end of the gate for **field relief**. Benefits:
- Moves the point of maximum hot-carrier generation from the surface to a depth roughly equal to the field oxide bottom
- Reduces surface state generation and transconductance degradation
- Can support operating voltages approaching well-to-epi breakdown

For DEPMOS with field gaps, breakdown of the drain extension (P-epi or P-well) to NBL often limits $V_{DS}$. **NBL dilution** (drawing closely spaced narrow strips to reduce dopant concentration via outdiffusion) combined with a gate field plate can extend operating voltage.

### DMOS Transistors

**Double-Diffused MOS**: Uses the different diffusivities of boron and arsenic to create self-aligned source and backgate from a single mask opening. The channel length equals the difference in outdiffusion distances -- immune to photolithographic misalignment.

**Fabrication steps** (N-channel DMOS in N-well CMOS):
1. N-well forms the drift region
2. DWell mask creates oxide opening inside N-well
3. Boron and arsenic implanted through the same opening
4. During LOCOS field oxidation, boron diffuses deeper and wider than arsenic, creating a deep P-type backgate with a shallow N-type source inside it
5. Gate poly deposited and patterned
6. NSD implant creates extrinsic drain and source contact regions
7. PSD plugs penetrate through the arsenic layer to contact the boron backgate

**Source-centric layout**: The source/backgate finger occupies the center, with alternating NSD (source) and PSD (backgate) rectangles running down it. Ends are rounded to eliminate field-intensifying corners. The drawn width equals the perimeter of the drawn DWell geometry:

$$W_{drawn} = 2L_f + 2\pi r$$

where $r$ is the radius of curvature at the finger ends and $L_f$ is the straight section length.

**PSD end caps**: Enlarging PSD at finger ends to completely suppress the channel in curved regions further reduces hot-carrier generation at these points.

**LSD vs. HSD DMOS**:
- **Low-Side Drive (LSD)**: Source connects to substrate potential. Omits NBL. Achieves higher $V_{DS}$ because drain/backgate depletion can punch through the well and merge with drain/substrate depletion, reducing peak field. Benefits from natural RESURF.
- **High-Side Drive (HSD, isolated)**: Source operates above substrate. Requires NBL for isolation. Lower $V_{DS}$ because NBL constrains depletion and intensifies the field.

**Adaptive RESURF**: An N-channel stop implant beneath the field gap halts the Kirk effect (which would move peak field from the drain/backgate junction to the drift/extrinsic drain interface), enhancing electrical SOA.

**Separate backgate**: Possible by placing a hole in the N-well beneath the DWell, allowing the boron backgate to contact substrate through P-epi. Permits source operation above substrate potential in LSD devices, but increases backgate resistance and may reduce SOA.

**P-channel DMOS**: Less common (lower hole mobility = larger devices). Requires depositing phosphorus first, then boron through the same opening (since boron is the only practical acceptor in silicon; indium is only partly ionized at room temperature).

**DMOS advantage beyond self-alignment**: The graded channel (doping decreases from source toward drain) causes the pinched-off region to widen more slowly at higher voltages, enabling shorter channels for high-voltage operation and better transconductance than DEMOS counterparts.

---

## Diagrams

### Figure 1: SOA and Gate Metallization Patterns (p. 654)

![[diagrams/ch13-power-mos-fig1.png]]

Shows gate metallization patterns connecting one end (A) and both ends (B) of each gate finger, illustrating how double-ended connection doubles the allowed gate width. Also shows the beginning of the Safe Operating Area discussion, contrasting ideal rectangular SOA with the reduced SOA of real integrated devices due to electrical and electrothermal mechanisms.

### Figure 2: SDD NMOS Depletion Region and Electric Field (p. 668)

![[diagrams/ch13-power-mos-fig2.png]]

Cross section of a conventional single-doped drain (SDD) NMOS transistor showing the drain depletion region geometry and the corresponding plot of electric field intensity across the surface. The field peaks at the metallurgical junction and the depletion extends almost exclusively into the backgate. This motivates the need for drift regions in high-voltage designs.

### Figure 3: DMOS Transistor Fabrication and Drain-Centric Layout (p. 679)

![[diagrams/ch13-power-mos-fig3.png]]

Shows the annular drain-centric field-gapped drain-extended NMOS layout and the beginning of the DMOS transistor discussion, including the historical V-groove MOS (VMOS) concept and the steps in fabrication of diffusion self-aligned transistors that evolved into modern DMOS devices.

---

## Practical Takeaways

- **Metallization is often the bottleneck**: In low-voltage integrated power MOSFETs, metallization resistance can exceed silicon resistance. Always compute both.
- **Use the rule of one-third** for finger resistance estimation, but verify that finger length does not exceed $1.8\lambda$ (penetration distance). If it does, use full equations or finite-element analysis.
- **Connect both ends of gate fingers** in switching power transistors to double the allowed gate width and halve propagation delay.
- **Divide wide transistors into banks** when gate finger widths exceed $W_{max}$ for the desired switching speed.
- **Opposite-end bus termination** provides more uniform current distribution; prefer it for buses longer than $2\lambda$.
- **Double-terminate buses** (contact both ends) to reduce bus resistance by 4x.
- **Backgate contacts are critical for SOA**: Place PSD plugs at the ends of source/backgate fingers where field intensification is worst. Use distributed backgate contacts throughout the device if no low-resistance sublayer exists.
- **Use even numbers of gate fingers** in DENMOS with source on both ends to minimize misalignment sensitivity.
- **Field-gapped structures** should use LOCOS (not STI) where possible for gradual oxide transitions.
- **Exact dimensions of field gap and poly/N-well spacing are critical** -- implement precisely as specified by the device designer. Small variations dramatically alter device characteristics.
- **DWell PSD plug patterns, dimensions, and placement** are crucial elements of DMOS design. Use PCells where available.
- **Round the ends** of source/backgate fingers to eliminate field-intensifying corners.
- **LSD DMOS without NBL** achieves higher voltage than HSD; use LSD when source connects to substrate potential.
- **Waffle transistors** are area-efficient but problematic for ESD robustness and backgate contact; prefer bent-gate layouts for applications with transient overloads.
- **SenseFETs with series-connected sections** achieve large ratios (>1000:1) with good matching; place sense fingers in the thermal center of the device.

---

## Relation to the Bigger Picture

This section bridges the fundamental MOS transistor construction techniques of [[ch12-constructing-cmos]] with the matching requirements covered in [[ch13-matching-mos]]. The on-resistance analysis, metallization patterns, and SOA concepts here are essential for any analog IC that must drive loads -- from switching power supplies to motor drivers to audio amplifiers. The RESURF and DMOS techniques represent the primary methods by which standard CMOS and BiCMOS processes are extended to support the higher voltages required in mixed-signal and power management ICs. Understanding these structures is prerequisite to the matching discussion in Section 13.2, where the geometric and process factors that affect power transistor matching (threshold voltage, transconductance, pocket implants, Pelgrom's law) build directly on the device structures introduced here.

---

## See Also
- [[ch13-matching-mos]]
- [[ch12-constructing-cmos]]
