---
title: "1.4 MOS Transistors"
chapter: 1
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-1, mos, mosfet, threshold-voltage, iv-characteristics, cmos]
---

# 1.4 MOS Transistors

> **Chapter 1: Device Physics**

## Key Concepts

### The Field-Effect Principle

Unlike the [[ch01-bipolar-transistors|bipolar transistor]], which amplifies a change in base-emitter voltage to produce a change in collector current *while consuming base current*, the **field-effect transistor (FET)** develops transconductance **without drawing any input current**. The gate terminal influences current flow by projecting an electric field across an insulating layer called the **gate dielectric**. Virtually no DC current flows through the dielectric. This is the fundamental advantage of the FET over the BJT for many applications.

The most common FET uses silicon dioxide ($\text{SiO}_2$) as its dielectric. Early versions used a metal gate, giving rise to the name **metal-oxide-semiconductor field-effect transistor (MOSFET)**. The name persists even though most modern devices use **polycrystalline silicon (poly)** gates instead of metal.

### The MOS Capacitor: Foundation for Understanding

To understand the MOS transistor, one must first understand the **MOS capacitor** -- a simpler structure consisting of two electrodes (one metal/poly, one extrinsic silicon) separated by a thin oxide layer. The metal electrode is the **gate**, and the silicon electrode is the **backgate** (also called the *body* or *bulk*).

Consider an aluminum gate on lightly doped P-type silicon. When a voltage $V_{GB}$ is applied from gate to backgate:

1. **Flatband condition** ($V_{GB} = V_{FB}$): The voltage that reduces the electric field across the dielectric to zero. For an aluminum gate on P-type backgate, $V_{FB} \approx -0.8\,\text{V}$. Even at $V_{GB} = 0$, a slight voltage difference exists because aluminum has a stronger electrostatic attraction for free electrons than P-type silicon, so a small positive bias already exists on the gate.

2. **Accumulation** ($V_{GB} < V_{FB}$): Making $V_{GB}$ more negative than $V_{FB}$ attracts majority holes toward the dielectric surface, forming a thin film of holes called an **accumulation layer**.

3. **Depletion** ($V_{GB} > V_{FB}$): Making $V_{GB}$ more positive repels majority holes from the surface. A depletion region forms, and its negative charge supports the increasing electric field across the dielectric.

4. **Inversion** ($V_{GB} > V_t$): When $V_{GB}$ crosses a critical threshold, thermally generated electrons drift toward the dielectric surface, forming a thin film of electrons called a **channel**. This marks the onset of **inversion**. The channel is an exception to the usual majority/minority carrier rules -- in P-type backgate, electrons (normally minority carriers) become the majority carriers *within the channel*.

### From MOS Capacitor to MOS Transistor

A true MOS transistor adds **source** and **drain** regions formed by selectively counter-doping the silicon on either side of the gate:

- **NMOS (N-channel MOS)**: P-type backgate with N-type source and drain. Channel consists of electrons.
- **PMOS (P-channel MOS)**: N-type backgate with P-type source and drain. Channel consists of holes.

The identity of source and drain depends on biasing: in an NMOS, the more positive terminal acts as the drain; in a PMOS, the more negative terminal acts as the drain. The source/drain identity can actually swap as circuit voltages fluctuate.

### Four Regions of Operation

Consider an NMOS transistor with backgate tied to source ($V_{BS} = 0$):

1. **Cutoff** ($V_{GS} \ll V_t$): The gate voltage is insufficient to form a channel. Accumulation may exist under the gate. Only tiny junction leakage currents flow.

2. **Subthreshold / Weak Inversion** ($V_{GS}$ slightly below $V_t$): The channel does not fully materialize, but a small concentration of electrons exists in the depleted backgate. Some diffuse into the strong electric field near the drain and are swept into it. The subthreshold current increases **exponentially** with $V_{GS}$ at lower voltages and **linearly** at higher voltages. The transition from exponential to linear marks the threshold voltage. In practice, subthreshold current becomes negligible when $V_{GS}$ falls about 200--300 mV below $V_t$.

3. **Linear (Triode) Region** ($V_{GS} > V_t$, $V_{DS}$ small): A channel connects source to drain. Drain current increases approximately linearly with $V_{DS}$. The channel is thicker at the source end and thinner at the drain end because the increasing voltage differential toward the drain widens the channel-backgate depletion region, reducing channel charge. Electrons accelerate along the channel to maintain constant current: near the source, many electrons move slowly; near the drain, few electrons move quickly.

4. **Saturation** ($V_{GS} > V_t$, $V_{DS} > V_{DS,\text{sat}}$): As $V_{DS}$ increases, eventually the channel charge at the drain end drops to zero -- the channel has **pinched off**. Carriers enter from the source, accelerate through the channel, and are swept by the lateral electric field across the pinched-off region into the drain. Further increases in $V_{DS}$ increase the lateral field across the pinched-off region but do not substantially change the channel length or the current. The drain current becomes nearly independent of $V_{DS}$.

| Region | NMOS | PMOS |
|--------|------|------|
| Cutoff | $V_{GS} \ll V_t$ | $V_{GS} \gg V_t$ |
| Subthreshold | $V_{GS} \lesssim V_t$ | $V_{GS} \gtrsim V_t$ |
| Linear | $V_{GS} > V_t$, $V_{DS} < V_{DS,\text{sat}}$ | $V_{GS} < V_t$, $V_{DS} > V_{DS,\text{sat}}$ |
| Saturation | $V_{GS} > V_t$, $V_{DS} \geq V_{DS,\text{sat}}$ | $V_{GS} < V_t$, $V_{DS} \leq V_{DS,\text{sat}}$ |

### Channel Length Modulation

In saturation, the pinched-off region widens slightly as $V_{DS}$ increases, because more charged dopant ions must be uncovered to support the growing lateral electric field. This slightly reduces the effective channel length and causes a slight increase in drain current with $V_{DS}$. This phenomenon is called **channel length modulation** and is analogous to the **Early effect** in [[ch01-bipolar-transistors|bipolar transistors]].

### Unipolar Conduction

MOS transistors are **unipolar devices** -- they rely on only one type of carrier (electrons for NMOS, holes for PMOS). The current consists solely of majority carriers within the channel. Consequently, MOS transistors do **not** exhibit the recombination delays seen in saturated bipolar transistors, which greatly simplifies high-speed circuit design.

A PMOS transistor typically exhibits somewhat less than half the transconductance of a comparable NMOS because hole mobility is lower than electron mobility. This is why most power MOS transistors are N-channel devices.

## 1.4.1 Threshold Voltage

### Enhancement vs. Depletion Mode

- **Enhancement-mode** transistors: No channel at $V_{GS} = 0$. A gate voltage must be applied to create the channel. Enhancement NMOS has **positive** $V_t$; enhancement PMOS has **negative** $V_t$. The vast majority of practical MOS transistors are enhancement devices.

- **Depletion-mode** transistors: A channel exists even at $V_{GS} = 0$. A gate voltage of opposite polarity is needed to remove the channel. Depletion NMOS has **negative** $V_t$; depletion PMOS has **positive** $V_t$.

| | Enhancement | Depletion |
|------|------------|-----------|
| NMOS | $V_t > 0$ | $V_t < 0$ |
| PMOS | $V_t < 0$ | $V_t > 0$ |

> **Convention warning**: Many engineers omit the sign when discussing PMOS threshold voltages. For example, saying "the PMOS $V_t$ increased from 0.6 to 0.7 V" really means $V_t$ shifted from $-0.6$ to $-0.7\,\text{V}$. Context usually clarifies the true sign, but this practice can confuse newcomers.

### Factors Affecting Threshold Voltage

**1. Backgate-to-source voltage (Backgate Modulation)**

The apparent $V_t$ of an NMOS increases as its backgate voltage becomes more negative (and vice versa for PMOS). This occurs because the increased voltage differential between channel and backgate widens the depletion region between them, increasing its charge. To maintain electrostatic balance, the channel charge must decrease, which shifts $V_t$. Circuit designers sometimes deliberately exploit backgate modulation to alter transistor threshold voltages.

**2. Backgate Doping**

A more heavily doped backgate requires a stronger electric field to invert, so $|V_t|$ increases with backgate doping. Device designers routinely adjust the doping concentration immediately beneath the gate dielectric to fine-tune $V_t$.

**3. Gate Material**

Different gate materials produce different flatband voltages, which directly shifts $V_t$. The choice of $N^+$ vs. $P^+$ polysilicon gate shifts $V_t$ by nearly **1 V**. Most modern MOS transistors use heavily doped polysilicon gates.

| Structure | Flatband Voltage |
|-----------|-----------------|
| Aluminum gate, P-backgate | $\approx -0.8\,\text{V}$ |
| $N^+$ poly gate, P-backgate | $\approx -0.8\,\text{V}$ |
| $N^+$ poly gate, N-backgate | $\approx -0.12\,\text{V}$ |
| $P^+$ poly gate, P-backgate | $\approx 0.12\,\text{V}$ |
| $P^+$ poly gate, N-backgate | $\approx 0.8\,\text{V}$ |

**4. Gate Dielectric Thickness**

A thinner gate dielectric requires a lower voltage to produce the same electric field intensity. As operating voltages have decreased, so have dielectric thicknesses. However, insulators thinner than about **9 nm** begin to exhibit leakage due to **electron tunneling**. Advanced digital processes replace $\text{SiO}_2$ with **high-k dielectrics** (e.g., hafnium oxide, $\text{HfO}_2$) that increase the electric field for a given $V_{GS}$, allowing thicker and less leaky gate dielectrics.

**5. Dielectric Charges**

Excess charges in the gate dielectric or along its surfaces alter the electric field and shift $V_t$. Sources include:
- **Surface state charge**: Structural defects along the oxide-silicon interface (usually positive). This is the largest source in modern processes.
- **Mobile ions**: Historically, sodium contamination introduced ions that drifted under bias, causing time-varying $V_t$ shifts. Modern processing minimizes this (see Section 5.2.2).
- Ionized impurity atoms and trapped carriers.

## 1.4.2 I-V Characteristics

The performance of a MOS transistor is graphically illustrated by a family of I-V curves. For an enhancement NMOS with source and backgate tied together:

- **Vertical axis**: Drain current $I_D$
- **Horizontal axis**: Drain-to-source voltage $V_{DS}$
- **Each curve**: A specific gate-to-source voltage $V_{GS}$

This is analogous to bipolar I-V families, but MOS curves are obtained by stepping $V_{GS}$ (a voltage), whereas bipolar curves are obtained by stepping $I_B$ (a current).

### Key Features of the I-V Plot

- **Linear region** (far left): $I_D$ increases linearly with $V_{DS}$.
- **Saturation region** (middle): $I_D$ becomes nearly independent of $V_{DS}$. Channel length modulation causes a slight upward tilt.
- **Breakdown region** (far right): Drain currents suddenly rise.

### Breakdown Mechanisms

**Avalanche breakdown** occurs in longer-channel transistors. Impact ionization in the pinched-off region causes drain current to increase rapidly. Significant current then flows into the backgate. Every MOS transistor contains a **parasitic bipolar transistor**: the source forms the emitter, the backgate acts as the base, and the drain serves as the collector. Impact ionization current flowing into the backgate biases this parasitic BJT into forward-active mode. The additional current causes $V_{DS}$ to decrease as $I_D$ increases -- a **negative-resistance** regime known as **snapback**, which causes the drain curves to bend back toward the left at high currents.

**Punchthrough** occurs in short-channel transistors when the pinched-off region extends entirely across the backgate. Majority carriers flow from source to drain regardless of gate voltage. Punchthrough does *not* inject current into the backgate and therefore does *not* exhibit snapback.

### NMOS vs. PMOS Transconductance

A PMOS transistor typically has **less than half** the transconductance of a comparable NMOS due to lower hole mobility. This is why:
- Most power MOS transistors are N-channel devices
- CMOS logic sizes PMOS transistors wider than NMOS to balance drive strength

### MOS Terminology vs. Bipolar Terminology

There is an unfortunate historical confusion: the MOS **saturation** region corresponds to the bipolar **forward-active** region, and the MOS **linear** region corresponds to the bipolar **saturation** region. This is simply a result of different research groups developing similar technologies independently.

## Diagrams

### Figure 1.22: Cross Sections of NMOS and PMOS Transistors

![[diagrams/ch01-mos-transistors-fig1.png]]

*Cross sections showing the structural difference between NMOS (P-type backgate with N-type source/drain, labeled NSD) and PMOS (N-type backgate with P-type source/drain, labeled PSD). Note the gate (G), source (S), drain (D), and backgate (BG) terminals. Table 1.4 above shows flatband voltages for different gate/backgate material combinations.*

### Figure 1.23: NMOS Transistor Regions of Operation

![[diagrams/ch01-mos-transistors-fig2.png]]

*An idealized NMOS transistor shown in three states: (A) cutoff -- no channel, depletion regions around source and drain junctions; (B) linear -- a continuous channel from source to drain, thinner at the drain end due to the drain-to-source voltage widening the depletion region; (C) saturation -- the channel has pinched off at the drain end, and carriers are swept across the pinched-off region by the lateral electric field.*

### Figure 1.25: Typical I-V Curves of an NMOS Transistor

![[diagrams/ch01-mos-transistors-fig3.png]]

*Family of drain current ($I_D$) vs. drain-to-source voltage ($V_{DS}$) curves for an enhancement NMOS, parameterized by gate-to-source voltage $V_{GS}$. The linear region is at the far left, saturation occupies the middle (with slight upward tilt from channel length modulation), and drain-to-source breakdown appears at the right with snapback behavior at high currents.*

## Practical Takeaways

- **Source/drain identity is bias-dependent**: In NMOS, the more positive terminal is the drain; in PMOS, the more negative terminal is the drain. For asymmetric transistors (e.g., lightly doped drain structures), swapping source and drain can reduce voltage rating or cause catastrophic failure.

- **Backgate modulation is a design tool**: Circuit designers can deliberately alter $V_t$ by biasing the backgate relative to the source. This is commonly exploited in low-voltage analog design.

- **Gate dielectric scaling has limits**: Below ~9 nm, tunneling leakage becomes significant. High-k dielectrics (e.g., $\text{HfO}_2$) are used in advanced nodes to maintain gate control with thicker, less leaky dielectrics.

- **Beware the parasitic BJT**: Every MOS transistor contains a parasitic bipolar (source = emitter, backgate = base, drain = collector). At high voltages, impact ionization can turn this on, causing snapback -- a potential reliability and latch-up hazard.

- **PMOS has lower transconductance than NMOS**: Roughly half, due to lower hole mobility. This affects sizing decisions in analog and digital design.

- **Subthreshold conduction matters**: Even below $V_t$, current flows exponentially with $V_{GS}$. It only becomes truly negligible about 200--300 mV below $V_t$. This is critical for leakage-sensitive designs and subthreshold analog circuits.

- **Channel length modulation degrades output resistance**: The slight increase of $I_D$ with $V_{DS}$ in saturation limits the output impedance of MOS current sources and amplifiers. Longer channels reduce this effect (analogous to reducing the Early effect in BJTs).

- **Enhancement vs. depletion choice**: Almost all practical transistors are enhancement-mode. Depletion devices are used in special applications. Zero-$V_t$ transistors are useful in low-voltage circuits but have process-dependent behavior.

## Relation to the Bigger Picture

This section provides the device physics foundation for all MOS-based analog layout discussed throughout the rest of the book. Understanding the four regions of operation, threshold voltage dependencies, and breakdown mechanisms is essential for Chapters 12--13, which cover MOS transistor construction, coding, drain engineering, and matching in detail. The parasitic bipolar transistor within every MOSFET connects directly to latch-up concerns covered in Chapter 5. The distinction between NMOS and PMOS characteristics -- particularly the mobility difference -- drives many of the sizing and matching strategies that are central to analog layout practice.

## See Also

- [[ch01-bipolar-transistors]]
- [[ch01-jfet-transistors]]
