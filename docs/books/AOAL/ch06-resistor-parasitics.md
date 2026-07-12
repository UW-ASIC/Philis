---
title: "6.4-6.5 Resistor Parasitics and Comparison"
chapter: 6
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-6, resistors, parasitics, resistor-comparison]
---

# 6.4-6.5 Resistor Parasitics and Comparison

> **Chapter 6: Resistors**

## Key Concepts

### Why Parasitics Matter

Real-world integrated resistors do not simply add resistance to a circuit. They also introduce **capacitance** and, in the case of diffused resistors, **PN junctions**. These unwanted circuit elements are called *parasitics*. To accurately model a resistor, one must use a *subcircuit* containing properly sized parasitic devices. Understanding these subcircuit models is essential for layout designers because they reveal the fundamental limitations of each resistor type and dictate when one type should be chosen over another.

The central tradeoff is this: **deposited resistors** (poly, thin-film) are surrounded by oxide and have only capacitive parasitics, while **diffused resistors** (base, emitter, HSR, NSD, PSD, N-well) are isolated by reverse-biased PN junctions that contribute both capacitance and leakage, and can forward-bias under transient conditions, potentially triggering latchup.

### Parasitic Capacitance in Deposited Resistors

A poly resistor sits atop field oxide with interlevel oxide (ILO) above it. Assuming no metal leads cross the resistor, the dominant parasitic capacitance is between the poly and the underlying substrate through the field oxide. Typical field oxide capacitance is approximately $0.05 \text{ fF/}\mu\text{m}^2$. For example, a $5\,\mu\text{m}$-wide resistor containing 1000 squares would have a total parasitic capacitance of about 50 fF. This capacitance is *distributed* uniformly along the resistor body.

The distributed capacitance is modeled using **$\pi$-section** lumped-element models:

- **Single $\pi$-section**: The total distributed capacitance is split into two lumped capacitors $C_1$ and $C_2$, each equal to half the total. The resistor is represented by a single ideal resistor $R$ between terminals $T_1$ and $T_2$, with $C_1$ and $C_2$ shunting from each terminal to the substrate.
- **Dual $\pi$-section**: For better accuracy, the resistor is split into two halves ($R_1$ and $R_2$, each half the total resistance). Capacitors $C_1$ and $C_3$ each equal one quarter of the total, and a center capacitor $C_2$ equals one half. This provides a more faithful representation of the distributed RC behavior at higher frequencies.

### Capacitive Coupling from Overlying Leads

Metal leads routed across a poly resistor introduce additional parasitic capacitance through the interlevel oxide ($\approx 0.05 \text{ fF/}\mu\text{m}^2$, roughly equal to field oxide capacitance). Even a tiny capacitance can devastate sensitive analog circuits. Consider: a $5\,\mu\text{m}$-wide lead crossing a $10\,\mu\text{m}$-wide resistor introduces about 0.05 fF. If this couples to a high-impedance node with 0.1 pF total capacitance, a 2 V peak-to-peak square wave on the metal lead injects a $1\,\text{mV}$ signal onto the high-impedance node -- enough to inject a tone into a microphone amplifier or cause a 16-bit data converter to lose multiple bits of resolution.

This is why layout designers must take extreme care routing digital signals through analog circuitry. Capacitive coupling worsens dramatically when leads run parallel (or stacked) for long distances. In extreme cases, capacitive crosstalk between digital signals can cause outright functional failures.

### Parasitic Junctions in Diffused Resistors

Diffused resistors are isolated from an enclosing region called the **body** (an N-epi tank in standard bipolar, or a well in CMOS/BiCMOS) by one or more reverse-biased PN junctions. A body contact is required to maintain the necessary reverse bias. The parasitics consist of:

1. **Resistor-body junction**: A distributed reverse-biased diode between the resistor material and the body.
2. **Body-substrate junction**: A second reverse-biased diode between the body and the substrate (if the body is not directly connected to substrate).

These junctions produce several undesirable effects:
- **Junction capacitance**: Typically $\sim 0.1 \text{ fF/}\mu\text{m}^2$, substantially higher than deposited-resistor capacitance because depletion regions are thinner than deposited oxide layers, and silicon has higher relative permittivity ($\varepsilon_r \approx 11.7$) than oxide ($\varepsilon_r \approx 3.9$). This capacitance varies nonlinearly with reverse bias.
- **Forward biasing risk**: Improper body connections or unexpected transients can forward-bias the isolation junctions, potentially triggering latchup.
- **Avalanche breakdown**: Emitter and base-pinch resistors are limited by the base-emitter avalanche voltage ($\approx 7\,\text{V}$ in standard bipolar). Diffused resistors should not operate beyond about $\frac{2}{3}$ of the resistor-body voltage rating.

## Subcircuit Models for Diffused Resistors

Two levels of complexity are presented:

**Model A -- Neglecting body resistance**: An ideal resistor $R$ with three diodes: $D_1$ and $D_2$ each model half the resistor-body junction, while $D_3$ models the full body-substrate junction. This is accurate when the body resistance is much smaller than the enclosed resistor value.

**Model B -- Including body resistance**: Adds a resistor $R_B$ modeling body resistance, and uses a pair of diodes ($D_3$, $D_4$) to model the distributed nature of the body-substrate junction. Use this when the body resistance is comparable to or larger than the resistor value.

## Body Biasing of Diffused Resistors

The text presents three important body-biasing schemes for P-type resistors (illustrated with resistor divider examples):

1. **Simple biasing**: Both bodies connect to $V_{CC}$. This reverse-biases both, but each resistor sees a different body-to-resistor voltage, so body modulation affects the two resistors unequally. The divider ratio becomes supply-voltage dependent.

2. **Matched body modulation**: Each resistor is placed in its own body region, biased so that the body-resistor voltage differentials are equal. For a 1:1 divider with $V_{CC}$ applied, connecting each body to a voltage that tracks the resistor midpoint causes body modulation to track between the two resistors. This can be extended to unequal-value resistors by dividing them into sections of equal value, each in its own body. The downside is significant area consumption due to separate body regions.

3. **Active biasing**: The body connects to a transistor output rather than a supply. This requires careful analysis to ensure the resistor-body junction never forward-biases under any operating condition, including transients.

**Key rule**: Connections that do not tie back to the positive end of a P-type resistor or to a supply should be carefully analyzed for possible forward-bias conditions.

## Important Details

### 6.5 Comparison of Available Resistors

This section is a comprehensive survey of all resistor types available in standard bipolar, poly-gate CMOS, and analog BiCMOS processes.

### 6.5.1 Base Resistors

- **Sheet resistance**: 100 to $250\,\Omega/\square$
- **Surface concentration**: $10^{17}$ to $10^{18}\,\text{cm}^{-3}$
- **Best for**: General-purpose resistors in standard bipolar, range $1\,\text{k}\Omega$ to $50\,\text{k}\Omega$
- **Advantages over HSR**: Better sheet resistance control, less tank modulation due to heavier doping
- **Contact resistance**: Can be problematic at lower doping concentrations, especially with refractory barrier metal (RBM) without underlying silicide. Analog BiCMOS processes add PSD doping to ends to reduce contact resistance.
- **Field plates**: Required when tank voltage exceeds about $\frac{2}{3}$ of the thick-field threshold voltage (uppermost metal) to prevent charge spreading
- **NBL**: Should always be placed underneath to reduce lateral tank resistance and minimize noise coupling. NBL overlap of $5\,\mu\text{m}$ is typically sufficient for matched resistors.
- **Tank merging**: Safe to merge with NPN transistors (majority carrier injectors) if tank has NBL and deep-$N^+$ sinker. Do NOT merge with saturating lateral PNP transistors (minority carrier injectors); use a P-bar if merger is necessary.

### 6.5.2 Emitter Resistors

- **Sheet resistance**: 2 to $10\,\Omega/\square$
- **Best for**: Resistors from a few tenths of an ohm to a couple hundred ohms (ballasting, current sensing, tunnels/crossunders)
- **Advantages**: Very low contact resistance, negligible voltage nonlinearity, negligible body modulation, low TCR
- **Capacitive coupling**: Thin emitter oxide has capacitance $\sim 0.7\,\text{fF/}\mu\text{m}^2$ (vulnerable to ESD); thick emitter oxide has $\sim 0.35\,\text{fF/}\mu\text{m}^2$
- **Body arrangement**: Normally enclosed in base diffusion within a tank. Base connects to voltage at or below lowest resistor voltage; tank connects to voltage at or above highest. Voltage differential limited to $\sim \frac{2}{3}$ of emitter-base avalanche.
- **Tankless variant**: Can be placed directly in a tank without base body -- saves considerable space but limits to one resistor per tank. Especially suited for tunnels (crossunders) in single-level-metal designs.

### 6.5.3 Base Pinch Resistors

- **Sheet resistance**: 2 to $10\,\text{k}\Omega/\square$ (standard bipolar)
- **Variation**: Tracks NPN beta (both depend on Gummel number); process variability $\pm 50\%$ or more
- **Nonlinearity**: So extreme they are better modeled as JFETs. Resistance can double over 0-5 V range.
- **Tank modulation**: Even more extreme than nonlinearity
- **Matching**: $\pm 5\%$ to $\pm 10\%$ even between seemingly identical devices due to short-range doping variations
- **Use case**: Compact high-value resistors for noncritical applications only

### 6.5.4 High-Sheet Resistors (HSR)

- **Sheet resistance**: 1 to $10\,\text{k}\Omega/\square$
- **TCR**: Several thousand $\text{ppm/}^{\circ}\text{C}$ (can be minimized by incomplete anneal)
- **Junction depth**: Generally $< 1\,\mu\text{m}$, surface doping $\sim 5 \times 10^{16}\,\text{cm}^{-3}$
- **Contact**: Requires base heads due to low surface doping. Resistance computed by:

$$R = R_{sh} \cdot \frac{L_d + \Delta L}{W_d + \Delta W} + 2 R_{head}$$

where $R_{head} \approx 0.7 \cdot R_{sh,base} \cdot \frac{d}{W_{head} + \Delta W_{head}}$, with $d$ being the overlap of base over the contact and 0.7 being an empirical constant for nonuniform current flow.

- **Charge spreading**: Notoriously prone. All HSR resistors at voltages exceeding $\frac{2}{3}$ of thick-field threshold require careful field plating.
- **Conductivity modulation**: Severe. Designers sometimes extend a base head and route leads over it instead of over the HSR body.
- **Avalanche**: Limited to 20-30 V (shallow implant limits breakdown to a fraction of planar breakdown). Higher voltages require segmentation into separate tanks.
- **Optimal sheet**: Around $2\,\text{k}\Omega/\square$ -- provides substantial area savings without excessive conductivity modulation concerns.
- **NBL**: Always place underneath to minimize tank debiasing. HSR more sensitive to NBL shadow than base.

### 6.5.5 Epi Pinch Resistors (Epi-FETs)

- **Sheet resistance**: 5 to $50\,\text{k}\Omega/\square$
- **Structure**: N-epi layer pinched between substrate (diffusing upward) and overlying base diffusion
- **Voltage modulation**: So extreme they are treated as JFETs. Pinchoff voltage typically $-5\,\text{V}$ to $-15\,\text{V}$; current becomes essentially independent of voltage beyond pinchoff.
- **Use**: Almost exclusively in startup circuits to provide a trickle current. Low pinchoff voltage actually beneficial since it limits current at higher operating voltages.

### 6.5.6 Metal Resistors

- **Sheet resistance**: Standard bipolar $\sim 0.03\,\Omega/\square$ (8000 A thick); CMOS/BiCMOS lower metals $\sim 0.05\,\Omega/\square$ (3000-5000 A thick)
- **Value range**: $10\,\text{m}\Omega$ to $5\,\Omega$
- **Applications**: Current sense circuits, ballasting bipolar power transistors
- **TCR**: Close to $+3300\,\text{ppm/}^{\circ}\text{C}$ -- significant because this matches the TCR at $+25^{\circ}\text{C}$ of a voltage proportional to absolute temperature (VPTAT), enabling temperature-invariant current measurement.
- **Kelvin connections**: Essential for accurate voltage sensing. Two pairs of leads: force (current) and sense (voltage). The star connection (where sense meets force lead) must be carefully designed.
  - *Single-level-metal*: Sense taps connect to the side of the resistor. Force leads should extend $\geq 2W$ beyond the star connection before any bends.
  - *Double-level-metal*: Sense leads on upper metal tap into the center, reducing sensitivity to nonuniform current flow.
- **Process variation**: $\sim \pm 20\%$ (comparable to other types), but wafer fabs may not monitor metal sheet resistance.

### 6.5.7 Poly Resistors

This is the most extensively discussed resistor type, reflecting its dominance in modern processes.

**Sheet resistance tiers**:
- Silicided poly (LSR): $\sim 5\,\Omega/\square$
- Unsilicided gate poly: $\sim 20\,\Omega/\square$
- Medium-sheet (MSR): $\sim 200\,\Omega/\square$
- High-sheet (HSR poly): $\sim 500\text{-}1000\,\Omega/\square$
- Very high-sheet (VHSR): $\geq 1000\,\Omega/\square$ (megohms possible, but high variability)

**Physics of high-sheet poly resistivity**: At doping levels below $\sim 10^{19}\,\text{cm}^{-3}$, poly resistivity rises dramatically (over five orders of magnitude above monocrystalline silicon at $10^{17}\,\text{cm}^{-3}$). Three mechanisms:
1. Carrier trapping at grain boundaries reduces effective doping
2. Depletion regions at grain boundaries create potential barriers
3. Phosphorus segregation at grain boundaries in electrically inactive form

**Temperature coefficient**:
- Heavily doped: Small positive ($\sim +500\,\text{ppm/}^{\circ}\text{C}$ for $2000\,\text{A}$ phosphorus-doped at $20\,\Omega/\square$)
- High-sheet: Negative ($\sim -1500\,\text{ppm/}^{\circ}\text{C}$, can reach $-5000$ for VHSR)
- Zero-temperature-coefficient (0TC): Achievable at $\sim 200\,\Omega/\square$ for $2000\,\text{A}$ films, but with $\pm 500\,\text{ppm/}^{\circ}\text{C}$ variation across lots

**Ramped (tilted) vs. flat poly**: Temperature ramping in the reactor tube (used to offset reactant gas depletion) creates wafer-to-wafer grain structure variations. This has little effect on gate poly but causes significant high-sheet poly variation. Analog fabs should use flat poly (uniform temperature profile).

**Layout considerations**:
- **Silicide block mask**: Required to prevent siliciding of the resistor body while leaving heads silicided. Resistance: $R = R_{sh} \cdot \frac{L_d + \Delta L}{W_d + \Delta W}$ (head resistance negligible due to low silicided sheet).
- **Gate doping block mask**: For blanket-doped processes. Large overlap required because dopants diffuse faster along grain boundaries in poly than in monocrystalline silicon. Oxygen or nitrogen implantation can reduce this fast diffusion.
- **Placement**: Atop field oxide (not thin gate oxide) to reduce parasitic capacitance. Deep-$N^+$ can sometimes be coded underneath to thicken field oxide via dopant-enhanced oxidation.
- **Power handling**: Much less than diffused resistors due to poor thermal conductivity of enclosing oxide.
- **Voltage-dependent spacing rules**: Modern processes implement voltage-dependent poly spacing rules to avoid TDDB. For a multi-segment resistor, the maximum voltage between adjacent segments: $V_{seg} = \frac{2V}{N}$, where $V$ is total voltage and $N$ is the number of segments.
- **Cone defects**: Can cause TDDB failures in STI oxide beneath poly (breakdown < 20 V observed). Not present in LOCOS processes.

### 6.5.8 NSD and PSD Resistors

- **Sheet resistance**: $30\text{-}100\,\Omega/\square$ (silicided: $\sim 5\,\Omega/\square$ unless silicide blocked)
- **Advantages**: Negligible voltage modulation and conductivity modulation due to heavy doping
- **Limitations**: Shallow implants cause reduced avalanche breakdown. NSD limited by NSD/P-epi breakdown; PSD limited by PSD/N-well breakdown.
- **Use**: Primarily in ESD devices (parasitic junction diodes serve as voltage clamps) and transient suppressors. Rarely used otherwise because poly offers equal or greater sheet resistance.

### 6.5.9 N-Well Resistors

- **Sheet resistance**: Up to $5\,\text{k}\Omega/\square$ (deep well); lower for shallow low-voltage wells ($\sim 1\text{-}2\,\text{k}\Omega/\square$)
- **Pinch plate**: PSD over N-well can increase sheet resistance to $\sim 10\,\text{k}\Omega/\square$ in non-retrograde wells (counterdopes heaviest portion). Much less effect on retrograde wells where heavy doping is deep.
- **Width effects**: Well does not achieve full junction depth/doping unless drawn width $\geq 2\text{-}3\times$ junction depth. Narrower wells exhibit increased sheet resistance (dilution), but the width bias becomes width-dependent and cannot be modeled with a constant.
- **Field plating**: Always required for unpinched N-well resistors. Pinched variants are self-field-plated by the pinch plate.

### 6.5.10 Thin-Film Resistors (TFR)

- **Materials**: Nichrome ($\rho \sim 0.1\,\mu\Omega\cdot\text{cm}$, TCR 50-100 ppm), tantalum ($\rho \sim 0.2$, TCR $-100$ to $-350$ ppm), sichrome ($\rho \sim 1\text{-}20$, TCR 0 to $-150$ ppm)
- **TCR**: Less than $\pm 100\,\text{ppm/}^{\circ}\text{C}$
- **Process variation**: $< \pm 20\%$ as deposited; laser trimming achieves $< \pm 0.1\%$
- **Conductivity modulation**: Minimal
- **Power handling**: Limited (like all deposited resistors)
- **Aging**: Minimal, unlike polysilicon
- **Fabrication**:
  - *Single-mask*: TFR deposited before a metal layer, patterned, then metal contacts overlap the ends. Drawbacks: cannot route leads across, incompatible with tungsten-plug vias, dry etch selectivity issues.
  - *Two-mask*: TFR body patterned first, then oxide deposited, then TFC (thin-film contact) mask opens contacts. Wet etch used for final contact opening to avoid damaging the film. Compatible with modern processes.
- **Historical note**: Nichrome suffered electrochemical corrosion and open-circuit failures in early processes with oxide-only overcoats. Modern nitride overcoats largely solved this.

## Diagrams

### Polysilicon Resistor Cross Section and $\pi$-Section Models

![[diagrams/ch06-resistor-parasitics-fig2.png]]

**Figure 6.9** (top): Cross section of a polysilicon resistor showing the poly body surrounded by field oxide below and interlevel oxide above, sitting over the substrate. **Figure 6.10** (bottom): Two subcircuit models approximating the distributed parasitic capacitance using (A) a single $\pi$-section and (B) a dual $\pi$-section for higher accuracy.

### Diffused Resistor Cross Section and Subcircuit Models

![[diagrams/ch06-resistor-parasitics-fig1.png]]

**Figure 6.11** (top): Cross section of a typical diffused resistor in an N-epi tank, showing the tank contact, isolation, P-subs, and the resistor body with terminals $T_1$ and $T_2$. **Figure 6.12** (bottom): Subcircuit models for diffused resistors: (A) neglecting body resistance (uses diodes $D_1$, $D_2$ for resistor-body junction and $D_3$ for body-substrate), and (B) including body resistance $R_B$ with distributed body-substrate junction diodes $D_3$, $D_4$.

### Table 6.7 -- Summary of Available Resistor Types

![[diagrams/ch06-resistor-parasitics-fig3.png]]

Table 6.7 compares all resistor types across standard bipolar, poly-gate CMOS, and analog BiCMOS processes. Columns include typical sheet resistance ($\Omega/\square$), typical process variation (%), and typical operating voltage (V). Boldface entries indicate the most commonly used resistors in each process. Asterisked entries are diffused (require proper biasing); daggered entries have too much voltage modulation for most applications.

## Practical Takeaways

- **Prefer poly resistors** in CMOS/BiCMOS processes: no junction parasitics, no tank/body modulation, narrow pitch enables compact layouts even at moderate sheet resistance. The only exceptions are high-power applications (use diffused) and extreme accuracy requirements (use thin-film).
- **Never route noisy/digital signals across analog resistors** without considering capacitive coupling. Even femtofarads of coupling capacitance can corrupt high-impedance nodes.
- **Always field-plate lightly doped resistors** (HSR, N-well, high-sheet poly $> 1\,\text{k}\Omega/\square$) when tank/body voltage exceeds $\frac{2}{3}$ of thick-field threshold. Use split field plates for best results.
- **Place NBL under all diffused resistors in tanks** to minimize debiasing, reduce noise coupling, and provide a low-impedance body path. Ensure NBL overlap is sufficient ($\geq 5\,\mu\text{m}$) for matched resistors.
- **Body bias carefully**: Always connect the body of a P-type diffused resistor to the positive end of the resistor or to a supply that guarantees reverse bias under all conditions, including transients.
- **Segment high-voltage resistors** into separate tanks/wells to avoid exceeding junction breakdown limits and to reduce voltage nonlinearity.
- **Do not merge diffused resistors with minority-carrier injectors** (e.g., saturating lateral PNP) in the same tank. Use P-bars if space forces co-habitation.
- **Deposited resistors have lower parasitic capacitance** ($\sim 0.05\,\text{fF/}\mu\text{m}^2$ through field oxide) than diffused resistors ($\sim 0.1\,\text{fF/}\mu\text{m}^2$ through depletion regions), and their capacitance is voltage-independent.
- **Diffused resistors handle more power** than deposited resistors because silicon conducts heat much better than oxide. Diffused resistors are therefore preferred in ESD protection and power circuits.
- **Kelvin (4-wire) connections** are essential for accurate metal current-sense resistors. Extend force leads $\geq 2W$ beyond star connections before bends.
- **For poly spacing in serpentine resistors**, use voltage-dependent spacing rules. Maximum inter-segment voltage is $V_{seg} = 2V/N$ where $V$ is total voltage across the resistor and $N$ is the number of segments.
- **Thin-film resistors** are the gold standard for precision analog (TCR < 100 ppm, trimmable to $< 0.1\%$) but add cost. Reserve for applications that truly require them.

## Relation to the Bigger Picture

Sections 6.4 and 6.5 bring together the physics of resistor parasitics with the practical reality of choosing among available resistor types -- the bridge between understanding *how* resistors behave (variability and nonlinearity from [[ch06-resistor-variability]]) and *what to do about it* (trimming and tweaking from [[ch06-adjusting-resistors]]). The subcircuit models presented here are directly used by circuit simulators (SPICE) to predict real-world circuit behavior, making them essential knowledge for any analog layout designer. The comprehensive comparison in Table 6.7 serves as a decision matrix that will be referenced throughout the rest of the book, particularly in Chapter 8 (Matched Resistors) where the parasitic properties discussed here dictate which resistor types can achieve the matching needed for precision analog circuits.

## See Also
- [[ch06-resistor-variability]]
- [[ch06-adjusting-resistors]]
