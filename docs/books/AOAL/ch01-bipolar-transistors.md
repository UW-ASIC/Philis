---
title: "1.3 Bipolar Transistors"
chapter: 1
section: 1.3
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-1, bipolar-transistors, beta, IV-characteristics, BJT]
---

# 1.3 Bipolar Transistors

> **Chapter 1: Device Physics**

## Key Concepts

### What Is a Bipolar Transistor?

A **bipolar junction transistor (BJT)** consists of three semiconductor regions -- the **emitter**, **base**, and **collector** -- arranged so that the base is always sandwiched between the other two. The device is formed from two back-to-back PN junctions: the base-emitter junction and the base-collector junction. The term "bipolar" reflects the fact that both electrons and holes participate in conduction, unlike a FET which relies on only one carrier type.

There are two complementary types:

| Type | Emitter | Base | Collector |
|------|---------|------|-----------|
| **NPN** | N-type | P-type | N-type |
| **PNP** | P-type | N-type | P-type |

The critical physical insight is that the base region is made extremely thin (a few microns at most). Because carriers injected from one junction can diffuse across this narrow base before they recombine, conduction across one junction directly influences the behavior of the other junction. This coupling between the two junctions is the origin of transistor action (current amplification).

Although the idealized BJT structure appears symmetric (emitter and collector look identical), in practical devices the emitter and collector have **different doping profiles and geometries**. Swapping them degrades performance -- the device is intentionally asymmetric for optimization.

### Four Regions of Operation

Each of the two PN junctions can independently be forward- or reverse-biased, yielding four operating regions:

| Mode | Base-Emitter Junction | Base-Collector Junction |
|------|----------------------|------------------------|
| **Cutoff** | Reverse biased | Reverse biased |
| **Forward active** | Forward biased | Reverse biased |
| **Reverse active** | Reverse biased | Forward biased |
| **Saturation** | Forward biased | Forward biased |

**Cutoff:** Both junctions are reverse biased. Only leakage currents flow -- the transistor is essentially off.

**Forward active:** The base-emitter junction is forward biased, and the base-collector junction is reverse biased. For an NPN transistor:
- Holes flow from base to emitter (they recombine quickly in the emitter as minority carriers).
- Electrons flow from emitter to base, becoming minority carriers in the base.
- Most electrons diffuse across the thin base and reach the collector-base depletion region before recombining.
- The strong electric field in the depletion region sweeps these electrons into the collector.
- Result: a large collector current flows, controlled by a small base current. This is the useful amplifying mode.

**Reverse active:** The roles of emitter and collector are swapped. The transistor still amplifies, but with much lower gain because the doping is optimized for forward-active operation (see the discussion of asymmetric doping below).

**Saturation:** Both junctions are forward biased. Both emitter and collector inject minority carriers into the base. High recombination currents flow in the base. The transistor floods the collector with minority carriers, and switching speed suffers because these stored carriers must recombine before the device can turn off. The net current direction depends on the relative biases of the two junctions. Chapter 9 of the text discusses saturation in depth.

## Important Details

### 1.3.1 Beta ($\beta$)

The **current gain** (beta, $\beta$) of a bipolar transistor is defined as the ratio of collector current to base current:

$$\beta = \frac{I_C}{I_B}$$

Beta has also been called the **forward current gain** and denoted by various symbols including $\beta_F$, $h_{FE}$, and $h_{fe}$.

**Typical values:**
- A well-designed integrated NPN transistor: $\beta \approx 150$
- Specialized high-gain devices: $\beta > 10{,}000$
- NPN transistors in a CMOS process (parasitic devices): $\beta < 10$

All of these can be useful in analog design -- a skilled circuit designer can work with any of them.

#### What Limits Beta?

Two primary mechanisms limit $\beta$:

1. **Recombination within the neutral base:** Minority carriers injected from the emitter must traverse the quasineutral (neutral) base region -- the portion of the base between the two depletion regions, where there is no significant electric field. Three factors control the recombination rate:
   - **Neutral base width** -- a thinner base means carriers traverse it faster, reducing recombination probability.
   - **Base doping** -- lighter doping reduces majority carrier concentration, lowering recombination.
   - **Recombination center density** -- defects and impurities that act as recombination centers.

   The **Gummel number** ($G_B$) quantifies the combined effect of base width and doping. For uniform doping:
   $$G_B = N_B \cdot W_B$$
   where $N_B$ is the base dopant concentration and $W_B$ is the neutral base width. $\beta$ is inversely proportional to the Gummel number.

2. **Emitter injection efficiency ($\gamma$):** In an optimized BJT, the emitter is heavily doped relative to the base. For an NPN:
   - The heavily doped emitter injects a large electron current into the base.
   - The lightly doped base injects a much smaller hole current into the emitter.
   - The emitter injection efficiency is the ratio of desired current to total current:
   $$\gamma = \frac{I_{n,\text{emitter}\to\text{base}}}{I_{n,\text{emitter}\to\text{base}} + I_{p,\text{base}\to\text{emitter}}}$$
   A properly optimized transistor achieves $\gamma > 0.995$.

#### How Beta Varies with Current

Beta is **not constant** -- it varies with collector current in three regimes:

1. **Low currents:** $\beta$ is reduced by recombination in the depletion regions (space-charge recombination). These tiny currents are relatively more significant when the total current is small.
2. **Moderate currents:** Depletion-region recombination becomes negligible, and $\beta$ reaches its **peak value**, determined by the Gummel number and emitter injection efficiency.
3. **High currents (high-level injection):** The minority carrier concentration in the base approaches the majority carrier concentration. To maintain charge neutrality, additional majority carriers must accumulate, which increases recombination and causes $\beta$ to **drop**. Power transistors are often deliberately operated in this regime to save die area.

#### Gold Doping (Historical)

Historically, **gold doping** was used to increase recombination centers, speeding up switching in saturating logic (e.g., the 7400 TTL family) at the cost of reduced $\beta$. This made gold-doped transistors unsuitable for analog applications. Modern high-speed CMOS and nonsaturating bipolar logic have rendered gold-doped bipolar logic obsolete.

#### Optimized BJT Doping Profile

An optimized bipolar transistor uses:
- **Heavily doped emitter** -- maximizes emitter injection efficiency
- **Thin, moderately doped base** -- minimizes Gummel number and thus maximizes $\beta$
- **Wide, lightly doped collector** -- allows the collector-base depletion region to extend deep into the collector (not into the base), enabling high $V_{CE}$ operation without avalanche breakdown

This asymmetric doping is why **reverse-active beta is much lower than forward-active beta**. A typical NPN with $\beta_F \approx 150$ may have $\beta_R < 5$.

#### NPN vs. PNP

PNP transistors behave analogously to NPN transistors with reversed voltage polarities, reversed current directions, and swapped roles of electrons and holes. However, PNP $\beta$ is inherently lower for two reasons:
1. **Holes are less mobile than electrons**, leading to lower transport efficiency.
2. **Process optimization trade-offs:** In standard bipolar processes, the NPN base material is used for the PNP emitter. Since this material is relatively lightly doped, the PNP suffers from low emitter injection efficiency.

### 1.3.2 I-V Characteristics

The performance of a BJT is graphically depicted by a **family of I-V curves** plotting $I_C$ vs. $V_{CE}$ for different values of base current $I_B$.

Key features visible on the I-V curves:

**Saturation region (left portion):** The curves dive sharply toward the origin. The so-called "forced beta" in saturation is much lower than the forward-active $\beta$ because additional base current is needed to support injection across the base-collector junction.

**Forward-active region (middle):** The curves fan out with approximately constant spacing. Each increment of $I_B$ produces a proportional increase in $I_C$, reflecting the high forward-active $\beta$.

**Early effect (slight upward tilt):** The curves in the forward-active region are not perfectly flat -- they tilt slightly upward with increasing $V_{CE}$. This is because increasing reverse bias across the collector-base junction widens the depletion region, which narrows the neutral base, which reduces the Gummel number, which increases $\beta$. The **Early effect can be minimized** by lightly doping the collector so the depletion region extends primarily into the collector rather than into the base.

**Breakdown (right portion):** Beyond a certain $V_{CE}$, the collector current increases rapidly due to **avalanche multiplication** in the collector-base depletion region. This $V_{CE(BR)}$ limits the maximum operating voltage. Process designers typically derate the maximum voltage (e.g., from $\sim 8\,\text{V}$ apparent breakdown to $\sim 6\,\text{V}$ rated maximum) to account for process and temperature variation.

**Punchthrough:** An alternative breakdown mechanism in some low-voltage, high-gain transistors. Punchthrough occurs when the collector-base depletion region extends entirely across the neutral base and merges with the emitter-base depletion region. The resulting electric field sweeps carriers directly from emitter to collector regardless of base bias. Most modern BJTs are optimized so that avalanche breakdown occurs before punchthrough.

**Curve tracers:** The Tektronix 576 (introduced 1969) and its successors can directly measure and display these I-V families. Hastings notes that spending a few hours with a curve tracer is invaluable for building intuition about transistor behavior -- but warns that curve tracers can generate lethal voltages.

## Diagrams

### Figure 1.17 -- BJT Structure and Schematic Symbols
![[diagrams/ch01-bipolar-transistors-fig3.png]]
*NPN and PNP transistor structures showing the three semiconductor regions (emitter, base, collector) and their corresponding schematic symbols. Note how the base is always sandwiched between emitter and collector. The idealized structures show uniform doping in rectangular bars of silicon.*

### Figure 1.18 -- Current Flow in Forward-Active NPN
![[diagrams/ch01-bipolar-transistors-fig1.png]]
*Carrier flow in an NPN transistor operating in the forward-active region. Electrons are injected from the emitter into the base, diffuse across the thin neutral base, and are swept into the collector by the electric field in the collector-base depletion region. Some base recombination occurs (requiring base current), and some emitter recombination occurs with holes injected from base to emitter. The table of four operating regions is also shown.*

### Figure 1.20 -- Typical NPN I-V Curves
![[diagrams/ch01-bipolar-transistors-fig2.png]]
*Family of I-V curves ($I_C$ vs. $V_{CE}$) for an integrated NPN transistor at different base currents ($I_B = 20\,\mu A, 30\,\mu A, 40\,\mu A$, etc.). The saturation region is visible at the left (curves dive toward origin), the forward-active region spans the middle (approximately constant-slope curves showing the Early effect), and breakdown is visible at the right where curves turn sharply upward.*

## Practical Takeaways

- **$\beta$ is not a design parameter you can rely on precisely.** It varies with current level, temperature, and process. Design circuits that are insensitive to $\beta$ variation.
- **NPN transistors are generally superior to PNP** in standard bipolar processes: higher $\beta$, higher speed, better optimization. Use NPN where possible.
- **Reverse-active operation degrades $\beta$ severely** (by a factor of 30x or more). Avoid swapping emitter and collector connections.
- **The Early effect matters for analog precision.** Lightly doping the collector minimizes the Early effect (output resistance increases), which is important for current mirrors and amplifier stages.
- **Avalanche breakdown sets the voltage limit.** Always derate the maximum $V_{CE}$ from the apparent breakdown voltage to account for process and temperature variation.
- **Punchthrough is a concern in high-gain, low-voltage devices.** Proper collector doping prevents the collector-base depletion region from reaching the emitter.
- **Gold doping sacrifices analog performance for speed.** Avoid gold-doped processes for analog design.
- **High-level injection reduces $\beta$** but is sometimes intentionally used in power transistors to reduce die area. Be aware of the trade-off.
- **Emitter injection efficiency ($\gamma > 0.995$)** is achieved by heavily doping the emitter relative to the base. This is a fundamental design principle for BJT optimization.
- **The Gummel number is the key figure of merit** for base design: $G_B = N_B \cdot W_B$. Minimize it (thin base, moderate doping) to maximize $\beta$.

## Relation to the Bigger Picture

This section provides the device physics foundation for understanding bipolar transistor behavior, which is essential before tackling the practical layout techniques covered in Chapter 9 (Bipolar Transistors) and Chapter 10 (Applications of Bipolar Transistors). The concepts of $\beta$, the four operating regions, and I-V characteristics directly inform layout decisions such as emitter/base/collector geometry, doping profile selection, and the trade-offs between speed, gain, and voltage handling. Understanding *why* asymmetric doping matters (forward vs. reverse $\beta$), why the Early effect depends on collector doping, and how high-level injection reduces gain at high currents gives the layout designer the physical intuition needed to make intelligent trade-offs rather than blindly following design rules. The PN junction physics from [[ch01-pn-junctions]] (built-in potential, depletion regions, avalanche breakdown) are directly applied here to explain BJT operation.

## See Also
- [[ch01-pn-junctions]] -- PN junction fundamentals that underpin BJT operation (depletion regions, forward/reverse bias, avalanche breakdown)
- [[ch01-mos-transistors]] -- The complementary device type (section 1.4), which follows immediately after this section
