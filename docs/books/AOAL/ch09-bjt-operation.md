---
title: "9.1 Bipolar Transistor Operation"
chapter: 9
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-9]
---

# 9.1 Bipolar Transistor Operation

> **Chapter 9: Bipolar Transistors**

## Key Concepts

### Four Regions of Operation

A bipolar transistor operates in one of four distinct regions, determined by the biasing of its two junctions:

| Region | NPN Bias | PNP Bias |
|---|---|---|
| **Cutoff** | $V_{BE} < 0$, $V_{BC} < 0$ | $V_{EB} < 0$, $V_{CB} < 0$ |
| **Forward Active** | $V_{BE} > 0$, $V_{BC} < 0$ | $V_{EB} > 0$, $V_{CB} < 0$ |
| **Reverse Active** | $V_{BE} < 0$, $V_{BC} > 0$ | $V_{EB} < 0$, $V_{CB} > 0$ |
| **Saturation** | $V_{BE} > 0$, $V_{BC} > 0$ | $V_{EB} > 0$, $V_{CB} > 0$ |

The forward-active current gain $\beta_F$ (beta) is the ratio of collector current $I_C$ to base current $I_B$ in the forward-active region. The reverse-active current gain $\beta_R$ equals the ratio of emitter current to base current in the reverse-active region.

### Thermal Voltage

The thermal voltage is defined as:

$$V_T = \frac{kT}{q}$$

where $k$ is Boltzmann's constant ($1.381 \times 10^{-23}$ J/K) and $q$ is the electron charge ($1.602 \times 10^{-19}$ C). At $25\degree C$ (298.2 K), $V_T \approx 26$ mV. This quantity appears throughout bipolar transistor equations and sets the fundamental voltage scale for exponential I-V characteristics.

### Ebers-Moll Model

The first complete mathematical model of the bipolar transistor was developed by Ebers and Moll in 1954. The three fundamental equations are:

$$I_C = I_S \left( e^{V_{BE}/V_T} - e^{V_{BC}/V_T} \right) \quad [9.2]$$

$$I_B = \frac{I_S}{\beta_F} \left( e^{V_{BE}/V_T} - 1 \right) + \frac{I_S}{\beta_R} \left( e^{V_{BC}/V_T} - 1 \right) \quad [9.3]$$

$$I_E = I_C + I_B \quad [9.4]$$

where $I_S$ is the saturation current, which depends on the emitter-base junction area. $I_S$ is sometimes expressed as $I_S = A_E \cdot J_S$, where $A_E$ is the effective emitter area and $J_S$ is the saturation current density. However, emitter sidewalls prevent any simple relationship between effective and drawn emitter areas.

### Forward-Active Simplification

Ignoring leakage currents in the forward-active region yields:

$$I_C = I_S \, e^{V_{BE}/V_T} \quad [9.5]$$

$$I_B = \frac{I_C}{\beta_F} \quad [9.6]$$

$$I_E = I_C + I_B \quad [9.7]$$

The large-signal hybrid-$\pi$ model consists of a transconductance $g_m$ from collector to emitter and a diode from base to emitter.

### Small-Signal Model

The linearized small-signal parameters are:

$$g_m = \frac{I_C}{V_T} \quad [9.8]$$

$$r_\pi = \frac{\beta_F}{g_m} = \frac{\beta_F \, V_T}{I_C} \quad [9.9]$$

A crucial insight: the transconductance of a bipolar transistor depends **only** on its collector current, not on any device dimensions. Furthermore, $g_m$ of a bipolar transistor exceeds that of any MOS transistor operating at the same current. This is one of the primary reasons bipolar transistors are still preferred in certain applications.

### Temperature Sensitivity

Both $V_T$ and $I_S$ vary with temperature. Together they cause the base-emitter voltage required for a constant collector current to exhibit a temperature coefficient of approximately $-2$ mV/$\degree$C. Because collector current is an exponential function of $V_{BE}$, a mere $1\degree$C temperature difference between two bipolar transistors causes an **8% mismatch** in their collector currents -- corresponding to a temperature coefficient of roughly $+8\%/\degree$C. This enormous sensitivity has profound implications for matched bipolar transistor design and bipolar power transistor layout.

---

## 9.1.1 Beta Variation

Beta is not constant. It varies with **temperature**, **collector current**, and **collector-to-emitter voltage**.

### Temperature Dependence

The temperature coefficient of beta is primarily caused by variation of emitter injection efficiency with temperature. For standard bipolar NPN transistors with emitter doping levels around $10^{20}$ cm$^{-3}$, beta can increase by as much as a **factor of three** across a temperature range of $-55\degree$C to $+125\degree$C. This positive temperature coefficient contributes to thermal runaway in power NPN transistors. Standard bipolar lateral PNP transistors exhibit relatively little temperature variation in beta due to their more lightly doped emitters.

### Collector Current Dependence

NPN beta remains relatively constant over a wide range of collector currents but rolls off at both extremes:

- **High-current rolloff** (typically beyond $\sim 1$ mA/$\mu$m$^2$ of drawn emitter area): caused by **high-level injection** -- the injected minority carrier concentration approaches the majority carrier doping in the base.
- **Low-current rolloff** (typically below $\sim 1$ $\mu$A): caused by **recombination** within depletion regions and at the oxide-silicon interface, as well as short emitter effects.
- **Avalanche-induced degradation**: Avalanching the emitter-base junction severely degrades low-current beta by creating additional surface trap sites that increase recombination.

The lateral PNP has a markedly different beta curve: lower peak beta, more pronounced rolloff at both ends. The lightly doped emitter reduces injection efficiency (lower peak beta), surface recombination worsens low-current rolloff, and the lightly doped base triggers high-level injection at relatively low currents. The high-current and low-current rolloff regions often overlap, creating a pronounced peak -- and at peak beta, the device is already in high-level injection, complicating circuit design.

### Early Effect (Voltage Dependence)

Beta varies with $V_{CE}$ due to intrusion of the collector-base depletion region into the neutral base. This is the **Early effect**. It causes a slight upward tilt of the $I_C$ vs $V_{CE}$ curves. Extrapolating these slopes to the X-axis yields the **Early voltage** $V_A$:

$$\beta(V_{CE}) = \beta_0 \left(1 + \frac{V_{CE}}{V_A}\right) \quad [9.10]$$

Standard bipolar NPN transistors typically have $V_A \approx 150$ V.

The Early effect produces a finite output resistance:

$$r_o = \frac{V_A + V_{CE}}{I_C} \quad [9.11]$$

Since output resistance limits voltage gain, circuit designers prefer large Early voltages. The two ways to increase $V_A$ -- widening the base or increasing base doping -- both increase the Gummel number and reduce beta. Hence device designers use the **$\beta \cdot V_A$ product** as a figure of merit. Typical standard bipolar NPN: $\beta \cdot V_A \approx 20{,}000$ V.

---

## 9.1.2 Avalanche Breakdown

Maximum operating voltages are limited by **avalanche breakdown**, **punchthrough**, and **secondary breakdown**. Key breakdown voltages:

### $BV_{EBO}$ -- Emitter-Base Breakdown (Collector Open)

Equals the avalanche voltage of the emitter-base junction. For standard bipolar vertical NPN: $BV_{EBO} \approx 7$ V. This is relatively stable over process and temperature, making NPN transistors in emitter-base breakdown useful as **Zener diodes**. However, emitter-base avalanche occurs near the surface and causes **avalanche-induced beta degradation**. Polysilicon-emitter transistors should not operate at more than about half their $BV_{EBO}$ rating. Surface avalanche voltages also tend to drift over time (Zener walkout/walkback).

### $BV_{CBO}$ -- Collector-Base Breakdown (Emitter Open)

Relatively large because both base and collector are lightly doped. Standard bipolar vertical NPN: $BV_{CBO}$ ranges from **20 to 120 V** depending on process variant. These large values force **subsurface breakdown**, so avalanche-induced beta degradation does not occur. The standard bipolar lateral PNP has $BV_{EBO}$ and $BV_{CBO}$ equal in magnitude (but opposite in polarity) to the NPN $BV_{EBO}$. Lateral PNP transistors generally do not suffer avalanche-induced beta degradation, making them popular for input stages of amplifiers and comparators.

### $BV_{CEO}$ -- Collector-Emitter Breakdown (Base Open) and Beta Multiplication

$BV_{CEO}$ is substantially smaller than $BV_{CBO}$ due to **beta multiplication**. The mechanism:

1. Low levels of impact ionization begin at collector-base voltages well below $BV_{CBO}$.
2. With the base terminal open, impact ionization current flows into the base, biasing the transistor into forward active.
3. This current is multiplied by $\beta$, inducing additional impact ionization.
4. At some point, this positive feedback becomes self-sustaining -- this is **snapback**.

The voltage reaches a maximum called the **trigger voltage** ($BV_{CEO}$), then decreases (negative dynamic resistance region), until it stabilizes at the **sustain voltage** ($BV_{CES}$).

### $BV_{CER}$ -- Base Connected to Emitter Through a Resistor

Connecting the base to the emitter through a resistance $R$ allows impact ionization current to flow out the base terminal, keeping the transistor in cutoff. However, $I \cdot R_{base}$ drop can still bias it into forward active. Thus:

$$BV_{CES} < BV_{CER} < BV_{CBO}$$

The value of $BV_{CER}$ depends on the external resistance: for small $R$ it approaches $BV_{CES}$, for large $R$ it approaches $BV_{CBO}$.

### Substrate Breakdown

Junction-isolated bipolar transistors have an additional reverse-biased junction to the substrate. The breakdown voltage of this junction is typically equal to or larger than $BV_{CBO}$ and seldom limits circuit design, except for circuits biased relative to a positive supply rather than substrate ground.

---

## 9.1.3 Saturation in NPN Transistors

Saturation occurs when both emitter-base and collector-base junctions simultaneously forward bias. While it reduces $V_{CE(sat)}$ and minimizes power dissipation in switching applications, it creates several problems.

### Reverse Recovery Time

Forward biasing the collector-base junction injects minority carriers across it in both directions. In an NPN, holes diffuse into the neutral collector and electrons into the neutral base. Excess minority carriers accumulate on both sides. When the transistor is switched off ($V_{BE} \to 0$), these carriers must recombine before current flow ceases, creating **reverse recovery time**. Modern NPN transistors with lightly doped collector drift regions can have reverse recovery times of **several microseconds**. MOS transistors, being majority-carrier devices, do not suffer this problem.

### Substrate Injection in Junction-Isolated NPNs

When the collector-base junction forward biases, holes injected into the neutral collector can diffuse across to the reverse-biased isolation junction. This phenomenon -- **substrate injection** -- is modeled by a **parasitic PNP transistor**. The collector-base junction of the vertical NPN forms the emitter-base junction of this parasitic PNP, and the isolation junction forms its collector-base junction.

Consequences:
- Junction-isolated NPNs cannot be driven as deeply into saturation as dielectrically isolated ones.
- Substrate current can cause **substrate debiasing** and **latchup**.
- If base drive exceeds a few milliamps, guard rings may be needed.

### Base Current Hogging

When a transistor saturates, some base current flows through the collector-base junction rather than the emitter-base junction, reducing $V_{BE}$. In circuits where multiple transistor base-emitter junctions are paralleled (e.g., current mirrors), one saturating transistor can "hog" base current, reducing output currents of other transistors.

**Countermeasures:**

1. **Base-side ballasting**: Matched resistors inserted into each transistor's base lead, ratioed inversely to emitter areas. Typically no more than $\sim 100$ mV develops across the ballasting resistor. The ballasting resistor must **not** be in the same tank as the NPN it protects, or the parasitic PNP simply moves to a new location.

2. **Schottky clamps**: A Schottky diode (platinum or palladium silicide) placed across the base-collector junction provides an alternate current path before forward bias of the base-collector junction occurs. A Schottky-clamped NPN does not experience prolonged turnoff times or substrate injection. This was the basis of 74LS LSTTL logic.

### Saturation Voltage

$$V_{CE(sat)} = V_{BE} - V_{BC} + I_E R_E + I_B R_B + I_C R_C \quad [9.13]$$

At low currents where $I_E R_E$, $I_B R_B$, $I_C R_C$ are negligible:

$$V_{CE(sat)} = V_T \ln\left(\frac{1 + I_C/(\beta_R I_B)}{1 - I_C/(\beta_F I_B)}\right) \quad [9.14]$$

where $\beta_F$ and $\beta_R$ are forward and reverse beta, and the ratio $I_C/I_B$ in saturation is the **forced beta** (typical values: 5--10).

The **minimum intrinsic saturation voltage** (at zero current) is:

$$V_{CE(sat,min)} = V_T \ln\left(\frac{\beta_F}{\beta_R}\right) \quad [9.15]$$

For a typical vertical NPN with $\beta_R \approx 1$, this is about **18 mV** at $25\degree$C. This makes vertical NPNs poor voltage sampling switches. Lateral PNP transistors have higher $\beta_R$ and thus smaller minimum intrinsic saturation voltages. Alternatively, vertical NPNs can be used with collector and emitter interchanged for low offset applications.

### Quasisaturation and the Kirk Effect

A transistor with a lightly doped drift region exhibits **two distinct regions of saturation**:

- **Hard saturation**: The conductivity-modulated zone has penetrated entirely through the drift region. $V_{CE}$ drops steeply.
- **Quasisaturation**: The transistor is internally saturated but the drift region resistance causes the terminal voltage to appear as if the device is still in forward active. The collector current drops because holes injected into the collector are no longer available for base recombination.

**Kirk effect (base push out)**: At high collector currents, the electron space charge in the collector-base depletion region cancels the donor charge on the collector side. The depletion region leaps across the drift region to the NBL boundary. The effective base widens suddenly, reducing beta and switching speed. The critical current density for the Kirk effect decreases as drift region doping drops, explaining why high-voltage transistors must operate at lower current densities.

---

## 9.1.4 Saturation in Lateral PNP Transistors

The lateral PNP uses an N-epi tank as its base, a base-diffusion plug as its emitter, and an annular base diffusion as its collector. Its operation depends on holes finding the collector rather than escaping to the substrate or isolation sidewalls.

### Collector Efficiency

$$\eta_C = \frac{I_C}{I_E} = \frac{I_C}{I_C + I_B + I_{substrate}} \quad [9.12]$$

A properly designed lateral PNP operating away from saturation achieves $\eta_C > 95\%$, often exceeding 99%. This high efficiency depends on **NBL creating a high-low junction** whose built-in potential repels holes attempting to diffuse downward. Omitting NBL reduces collector efficiency to below 0.5.

### Parasitic Substrate PNP Transistors

Two parasitic substrate PNPs exist:
- $Q_2$: Emitter current injected downward through NBL to substrate.
- $Q_3$: In saturation, the outer collector perimeter re-injects holes into the tank, which flow to isolation sidewalls.

### Behavior During Saturation

Unlike the saturating vertical NPN, a saturating lateral PNP does **not** exhibit significant base current hogging because parasitic transistor $Q_3$ typically has high beta (>100). The emitter current remains approximately constant; instead, the collector current diminishes and the missing current flows to isolation/substrate. The main concern is **excessive substrate injection**, addressable with Schottky clamps or hole-blocking guard rings.

---

## Diagrams

### Figure 9.2 -- Beta vs. Collector Current

![[diagrams/ch09-bjt-operation-fig1.png]]

Beta versus collector current for small-signal NPN and lateral PNP transistors. The NPN maintains relatively flat beta over a wide current range, with rolloff at both high and low currents. The lateral PNP has lower peak beta with more pronounced rolloff at both extremes, often exhibiting an overlapping peak. The orange curve shows the devastating effect of emitter-base avalanche on NPN low-current beta.

### Figure 9.5 -- NPN Cross Section with Parasitic PNP

![[diagrams/ch09-bjt-operation-fig2.png]]

Cross section of a standard bipolar NPN transistor showing the parasitic PNP transistor formed by the base (P), collector (N-epi), and substrate/isolation (P). When the NPN saturates, minority carriers injected into the collector can reach the isolation junction, modeled by this parasitic PNP. Most parasitic current flows laterally to the isolation diffusion rather than vertically to the substrate.

### Figure 9.12 -- Hard Saturation vs. Quasisaturation

![[diagrams/ch09-bjt-operation-fig3.png]]

Plot of $I_C$ vs $V_{CE}$ showing the two distinct saturation regions in a transistor with a lightly doped drift region. The steep portion (left) is hard saturation where the conductivity-modulated zone penetrates the entire drift region. The shallower portion is quasisaturation, where the transistor is internally saturated but terminal voltages suggest forward-active operation due to drift region resistance. The dotted line marks the true boundary between forward active and saturation.

---

## Practical Takeaways

- **Temperature matching is critical**: A $1\degree$C mismatch between matched bipolar transistors causes 8% collector current mismatch. Thermal gradients on the die are the enemy of matched bipolar circuits.
- **Beta is not constant** -- it varies with temperature, current, and voltage. Circuit designs that depend on precise beta values are inherently fragile. Design for beta-independence wherever possible.
- **Emitter-base avalanche permanently damages NPN beta**: Never exceed $BV_{EBO}$ on NPN transistors, and keep polysilicon-emitter transistors below half their $BV_{EBO}$ rating.
- **Lateral PNPs are excellent input-stage devices**: Their subsurface breakdown avoids avalanche-induced beta degradation, and their relatively large $BV_{CBO}$ provides robust operation. They are preferred for amplifier and comparator input stages.
- **Saturating junction-isolated NPNs inject substrate current**: This can cause debiasing and latchup. Use guard rings if base drive exceeds a few milliamps, or employ active antisaturation clamps or Schottky clamps.
- **Base-side ballasting resistors must be in separate tanks**: Placing a ballasting resistor in the same tank as the NPN it protects simply relocates the parasitic PNP without eliminating it.
- **Schottky clamps prevent saturation-related problems**: They eliminate prolonged turnoff times, substrate injection, and base current hogging, at the cost of additional die area for the Schottky diode.
- **Vertical NPNs make poor voltage-sampling switches** due to their $\sim 18$ mV minimum intrinsic saturation voltage. Use lateral PNPs or reversed-connection NPNs instead.
- **NBL is essential for lateral PNP collector efficiency**: Without NBL, collector efficiency drops below 50%. The high-low junction formed by NBL repels holes attempting to reach the substrate.
- **Kirk effect limits high-current operation**: At high collector current densities, the effective base widens suddenly (base push out), degrading both beta and switching speed. This is particularly severe in high-voltage transistors with lightly doped drift regions.
- **Quasisaturation increases $V_{CE(sat)}$ dramatically**: Keep forced beta to no more than one-tenth of forward-active beta to avoid quasisaturation onset.

---

## Relation to the Bigger Picture

Section 9.1 establishes the foundational physics of bipolar transistor operation that governs every layout decision discussed in the rest of Chapter 9 and Chapter 10. Understanding beta variation, breakdown mechanisms, and saturation behavior is essential because these phenomena directly determine spacing rules, guard ring requirements, thermal design constraints, and the choice between NPN and lateral PNP structures. The temperature sensitivity discussed here ($-2$ mV/$\degree$C for $V_{BE}$, 8%/$\degree$C for $I_C$ mismatch) connects directly to the matching principles of [[ch08-matching-rules]] and motivates the common-centroid layout techniques applied to bipolar transistor pairs. The parasitic PNP structures introduced here are revisited in the context of minority carrier injection ([[ch05-minority-carrier-injection]]) and merged device design in Chapter 14.

---

## See Also
- [[ch09-standard-bipolar-transistors]]
- [[ch01-bipolar-transistors]]
