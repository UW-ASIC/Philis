---
title: "1.5 JFET Transistors"
chapter: 1
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-1]
---

# 1.5 JFET Transistors

> **Chapter 1: Device Physics**

## Key Concepts

The **junction field-effect transistor (JFET)** is an alternative to the MOSFET that uses **reverse-biased PN junction depletion regions** as its gate dielectric, rather than a thin oxide layer. This fundamental difference has important consequences for both device behavior and layout.

### How a JFET Works

Consider an N-channel JFET (NJFET). The device consists of a bar of **lightly doped N-type silicon** called the **body**, into which two P-type regions intrude:

- **Gate** -- a P-type region from above
- **Backgate** -- a P-type region from below

The thin region of **undepleted N-type silicon** between the gate-body and backgate-body junctions forms the **channel**. Connections at either end of the body serve as the **drain** and **source**. As with MOS transistors, which terminal is which depends on biasing: the drain-to-source voltage $V_{DS}$ of an NJFET is always $\geq 0$.

The key insight is that the JFET controls current flow by modulating the width of an undepleted channel between two depletion regions, rather than by creating or destroying an inversion layer as in a MOSFET. Because the gate-body junction must remain reverse-biased during normal operation, the gate draws essentially no DC current -- only a small reverse-bias leakage current flows through the gate terminal.

### Depletion-Mode Operation

A critical distinction: JFETs are inherently **depletion-mode devices**. A channel exists when $V_{GS} = 0$, meaning current flows with zero gate bias. This contrasts with enhancement-mode MOSFETs, which require a gate voltage to form a channel. To turn off an NJFET, one must apply a sufficiently negative $V_{GS}$; for a PJFET, a sufficiently positive $V_{GS}$.

## Important Details

### Regions of Operation

The JFET operates in three distinct regions, analogous to those of a MOSFET:

**1. Linear Region (Triode):**
When all four terminals are grounded, depletion regions form around the gate-body and backgate-body junctions but do not touch each other, leaving a conductive channel. If $V_{DS}$ rises above zero, current (electrons in an NJFET) flows from source to drain. At low $V_{DS}$, the drain current $I_D$ increases **linearly** with $V_{DS}$, and the device acts like a voltage-controlled resistor.

However, because the drain end of the channel sits at a higher voltage than the source end, the reverse bias across the gate-body and backgate-body junctions is larger at the drain end, causing the depletion regions to be wider there. This is visible in the cross-section diagrams as a tapered channel.

**2. Saturation Region:**
As $V_{DS}$ increases further, the gate-body and backgate-body depletion regions eventually **touch at the drain end**, forming a **pinched-off region**. Beyond this point, further increases in $V_{DS}$ simply intensify the electric field across the pinched-off region without significantly increasing $I_D$. The channel length remains almost unchanged, and $I_D$ levels off.

The electric field in the pinched-off region actually **pulls carriers across** from the channel into the drain -- this is distinct from the cutoff mechanism where the field opposes current flow.

**Channel-length modulation** occurs in saturation: the pinched-off region widens slightly as $V_{DS}$ increases, causing the effective channel length to decrease and $I_D$ to increase slightly. This is exactly analogous to channel-length modulation in MOSFETs and appears as a slight upward tilt in the I-V curves.

**3. Cutoff:**
When $V_{GS}$ is made sufficiently negative (for an NJFET), the gate-body depletion region extends into the channel and touches the backgate-body depletion region **at all points** between source and drain. Current flow ceases. The mechanism is that the negative gate voltage reduces the potential in the depletion region below the source voltage, preventing electrons from drifting across.

### Pinchoff Voltage $V_P$

The **pinchoff voltage** $V_P$ is the gate-to-source voltage just sufficient to interrupt current flow from source to drain:

- For an **NJFET**: $V_P$ is **negative** (e.g., $V_P = -8\,\text{V}$ in Figure 1.27)
- For a **PJFET**: $V_P$ is **positive**

This is the voltage at which the transistor enters cutoff. It is set by the channel doping and thickness.

### Gate and Backgate Influence

Both the gate and backgate electrodes influence channel current through the same physical mechanism -- expanding their respective depletion regions into the channel. In principle, they are symmetric. In practice, the **gate is usually designed to have a stronger effect** on the channel than the backgate, because the doping levels and geometry of the two junctions typically differ.

### Drain-to-Source Breakdown

At very high $V_{DS}$, the electric field in the pinched-off region becomes large enough to cause **impact ionization**. This triggers an exponential increase in drain current -- **drain-to-source breakdown**. This is visible in the I-V curves as a sharp upturn at high $V_{DS}$.

### Symmetric vs. Asymmetric JFETs

- **Symmetric JFETs** use identical drain and source geometries and doping levels. The source and drain can be interchanged without affecting electrical characteristics.
- **Asymmetric JFETs** use different source and drain geometries or doping. Interchanging source and drain changes the operating characteristics.

### Circuit Symbols (Figure 1.28)

Six symbol variants exist for JFETs:

| Symbol | Type | Features |
|--------|------|----------|
| A | NJFET 3-terminal | Official; arrow on gate points inward (N-body) |
| B | PJFET 3-terminal | Official; arrow on gate points outward (P-body) |
| C | NJFET 3-terminal | Offset gate toward source for identification |
| D | PJFET 3-terminal | Offset gate toward source for identification |
| E | NJFET 4-terminal | Includes backgate connection |
| F | PJFET 4-terminal | Includes backgate connection |

The **arrow on the gate lead** indicates the polarity of the gate-body PN junction. It does **not** indicate the direction of current flow through the gate. In fact, the small leakage current that flows through the gate terminal flows in the direction **opposite** the arrow. The arrow represents the orientation of the PN junction that isolates the gate.

## Diagrams

### Figure 1.26 -- NJFET Cross-Sections (Linear and Saturation)

![[diagrams/ch01-jfet-transistors-fig1.png]]

*Cross-sections of an N-channel JFET showing: (A) linear region operation where the channel is open between gate and backgate depletion regions, and (B) saturation where the depletion regions pinch off at the drain end. Note how the channel tapers toward the drain due to the higher reverse bias at that end. The body is lightly doped N-type, with P-type gate above and P-type backgate below.*

### Figure 1.27 -- NJFET I-V Characteristics

![[diagrams/ch01-jfet-transistors-fig2.png]]

*I-V curves for an NJFET with $V_P = -8\,\text{V}$. The backgate is tied to the source. Each curve corresponds to a different $V_{GS}$ value (0 V, -2 V, -4 V, -6 V, -8 V). Key features: (1) linear region at low $V_{DS}$ where $I_D$ scales with $V_{DS}$, (2) saturation region where $I_D$ plateaus, (3) slight upward slope in saturation due to channel-length modulation, and (4) breakdown at high $V_{DS}$ due to impact ionization. Note that the device conducts at $V_{GS} = 0$ (depletion-mode).*

### Figure 1.28 -- JFET Circuit Symbols

![[diagrams/ch01-jfet-transistors-fig3.png]]

*Six variants of JFET schematic symbols. A-B: official three-terminal symbols for NJFET and PJFET. C-D: three-terminal with offset gate displaced toward the source for easy identification. E-F: four-terminal symbols showing the backgate (BG) connection explicitly.*

## Practical Takeaways

- **JFETs are depletion-mode devices**: they conduct at $V_{GS} = 0$. This is the opposite of enhancement-mode MOSFETs and has major implications for bias circuit design -- a JFET-based circuit must actively apply a gate voltage to reduce or shut off current.
- **The gate-body junction must remain reverse-biased** during normal operation. Forward-biasing it would inject minority carriers and destroy the high-impedance gate characteristic. This limits the useful range of $V_{GS}$.
- **Channel-length modulation** in JFETs behaves identically to that in MOSFETs -- the same small-signal output resistance considerations apply. The finite output resistance in saturation means $I_D$ is not perfectly flat.
- **Impact ionization at high $V_{DS}$** sets the maximum operating voltage. Layout must ensure adequate spacing and field relief to avoid premature breakdown.
- **Symmetric vs. asymmetric geometry** matters for layout: symmetric JFETs are more forgiving of source/drain interchange, while asymmetric designs require careful terminal identification.
- **The backgate connection** should not be ignored in layout. Even if the schematic uses a 3-terminal symbol, the backgate is physically present and its potential affects device behavior (threshold shift, channel modulation). Proper backgate biasing or connection is a layout responsibility.
- **JFETs have extremely high input impedance** because the gate junction is reverse-biased, drawing only leakage current. This makes them useful in precision analog front-ends, but also means they are sensitive to charge accumulation and require careful handling of gate routing in layout.

## Relation to the Bigger Picture

The JFET section completes Chapter 1's survey of the four fundamental active devices available in silicon IC processes: PN diodes, bipolar transistors, MOSFETs, and JFETs. While MOSFETs dominate modern digital and mixed-signal design, JFETs remain important in analog layout because they appear as parasitic devices in many BiCMOS and bipolar processes, and they are deliberately used in precision analog circuits for their low noise and high input impedance. Understanding the JFET's depletion-region-based operation also reinforces the physics of PN junctions covered in [[ch01-mos-transistors]] and earlier in the chapter, providing a bridge between the junction physics of Section 1.2 and the practical transistor structures used in later chapters on layout techniques.

## See Also
- [[ch01-mos-transistors]]
