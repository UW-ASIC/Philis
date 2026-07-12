---
title: "12.4 The JFET Transistor"
chapter: 12
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-12, jfet, field-effect-transistors]
---

# 12.4 The JFET Transistor

> **Chapter 12: Field-Effect Transistors**

## Key Concepts

### JFET Operating Principles

Junction field-effect transistors (JFETs) are **depletion-mode** devices: a conducting channel exists at zero gate-to-source voltage, and a nonzero $V_{GS}$ is required to turn the device off. This is the fundamental distinction from enhancement-mode MOSFETs. The gate-to-source voltage that reduces drain current to zero is called the **pinchoff voltage** $V_P$, which plays the same role as the threshold voltage $V_{th}$ in a MOSFET. For an NJFET, $V_P < 0$; for a PJFET, $V_P > 0$.

The JFET exhibits three regions of operation, analogous to those of a MOSFET:

| Region | NJFET Condition | PJFET Condition |
|--------|----------------|-----------------|
| Cutoff | $V_{GS} < V_P$ | $V_{GS} > V_P$ |
| Linear (Triode) | $V_{GS} > V_P$, $V_{DS} < V_{GS} - V_P$ | $V_{GS} < V_P$, $V_{DS} > V_{GS} - V_P$ |
| Saturation | $V_{GS} > V_P$, $V_{DS} > V_{GS} - V_P$ | $V_{GS} < V_P$, $V_{DS} < V_{GS} - V_P$ |

In the **linear region**, the channel acts like a variable resistor whose conductance is modulated by $V_{GS}$. As $|V_{DS}|$ increases, the depletion regions widen near the drain end of the channel. When $|V_{DS}|$ is large enough, the drain end **pinches off** and the transistor enters **saturation** -- further increases in $|V_{DS}|$ drop across the pinched-off region rather than the channel, making $I_D$ largely independent of $V_{DS}$.

### Pinchoff Voltage

For a uniformly doped channel with abrupt junctions, the magnitude of the pinchoff voltage is:

$$|V_P| = \phi_0 + \frac{q N_C t_C^2}{2 \epsilon_r \epsilon_0}$$

where:
- $\phi_0 = \frac{kT}{q} \ln\left(\frac{N_G \cdot N_C}{n_i^2}\right)$ is the built-in potential of the gate-channel PN junction
- $q$ is the electron charge ($1.6 \times 10^{-19}$ C)
- $N_G$ is the gate doping concentration
- $N_C$ is the channel doping concentration
- $t_C$ is the channel thickness
- $\epsilon_r \approx 11.7$ is the relative permittivity of silicon
- $\epsilon_0$ is the permittivity of free space
- $n_i \approx 1.45 \times 10^{10}$ cm$^{-3}$ at 300 K

In practice, nonuniform doping and grading effects complicate this calculation, so $V_P$ is usually determined by measurement.

### JFET Current Equations (Shockley Model)

While the Shichman-Hodges equations can be applied by substituting $V_P$ for $V_{th}$, accuracy suffers. Shockley's original derivation is preferred:

**Linear region:**

$$|I_D| = \frac{W}{\rho \cdot L} \left[ |V_{DS}| \cdot t_C - \frac{2}{3} \sqrt{\frac{2 \epsilon_r \epsilon_0}{q N_C}} \left( (|V_{DS}| + |V_{GS}| + \phi_0)^{3/2} - (|V_{GS}| + \phi_0)^{3/2} \right) \right]$$

where $\rho$ is the channel resistivity.

**Saturation:**

$$|I_D| = I_{DSS} \left(1 - \frac{V_{GS}}{V_P}\right)^2$$

where $I_{DSS}$ is the saturation current at $V_{GS} = 0$.

### Design Target for Pinchoff Voltage

Ideally, a JFET should have $|V_P| \approx 5$ V. Smaller pinchoff voltages are difficult to control in manufacturing, while larger ones require inconveniently high supply voltages. Most devices fabricated without dedicated process extensions have $|V_P| > 10$ V, which limits their utility as true JFETs (they behave more like nonlinear resistors).

## JFET Construction Variants

### Epi-FET (Standard Bipolar)

The standard bipolar **epi pinch resistor** is actually an N-channel JFET with $V_P$ between $-5$ V and $-15$ V. The conventional layout extends the base pinch plate out into the isolation, creating a three-terminal transistor whose gate is tied to ground.

At low voltages, the device behaves as a resistor with a linear voltage coefficient. At higher voltages, it is more accurately modeled as a JFET. However, updiffusion of substrate doping into the channel creates substantial grading that makes Shockley's equations unreliable. A commonly used empirical model is:

$$I = \frac{V_1 - V_2}{R_0 \left(1 - \alpha \cdot \frac{V_1 + V_2}{2}\right)}$$

where $V_1$ and $V_2$ are the terminal voltages (referenced to gate/backgate), $R_0$ is the zero-bias resistance, and $\alpha$ is an empirically determined factor quantifying depletion effects. The zero-bias resistance is:

$$R_0 = R_{S0} \cdot \frac{L_D - \Delta L}{n(W_D - \Delta W)}$$

where $R_{S0}$ is the effective sheet resistance at zero bias, $L_D$ is the drawn length along the centerline, $W_D$ is the drawn width, $\Delta W$ and $\Delta L$ are correction factors, and $n$ is the number of corners. The tank geometry sets the drawn width, while the base pinch plate determines the drawn length.

**Key layout considerations for Epi-FETs:**
- The effective width is substantially smaller than drawn width due to **isolation outdiffusion**
- The width-to-effective-width relationship becomes **nonlinear for small widths** due to diffusion interactions between opposing channel sidewalls
- $\Delta L$ has little effect on devices with channel lengths greater than about 40 $\mu$m
- Designed for **compactness over accuracy** -- minimum drawn width is typical even though wider devices have less variability
- Channels are frequently **serpentined** to fit unused layout areas
- Contacts placed over the base pinch plate and tied to substrate potential help minimize current variation from substrate debiasing
- Rounded bends in serpentine layouts are sometimes used to prevent electric field intensification, but this provides **little or no benefit** because the exposed edge of the base pinch plate usually breaks down before the isolation sidewalls

### N-well JFET (CMOS/BiCMOS)

Analog CMOS and BiCMOS processes can build an N-well JFET analogous to the epi-FET by substituting N-well for the tank and **PMoat** for the base pinch plate.

**Critical process dependency:** If the well has a **retrograde doping profile** (where peak doping is subsurface), the magnitude of $V_P$ becomes very large and the device is better treated as a nonlinear resistor rather than a true JFET.

**Alternatives for the pinch plate:** The PMoat can be replaced by a base diffusion or a shallow P-well diffusion to:
- Reduce the magnitude of $V_P$
- Increase the zero-bias resistance

**Key difference from epi-FETs:** The pinch plate of an N-well JFET must extend **much further** into the isolation than for an epi-FET. The reason is geometric: the N-well diffuses **outward**, whereas standard bipolar isolation diffuses **inward**. This means the JFET channel extends beyond the drawn well boundary, and the pinch plate must cover this extended region.

**Width effects on pinchoff voltage:** A minimum-width N-well JFET has a **lower** $|V_P|$ than a wider device because outdiffusion lightens the effective channel doping. In extreme cases (minimum channels with deep pinch plates), $|V_P|$ can become intolerably low, requiring the drawn width to exceed the minimum allowed N-well width.

### Double-Diffused JFET (Standard Bipolar)

The standard bipolar **base pinch resistor** (Section 6.5.3) is actually a P-channel JFET. The channel is the portion of the base diffusion beneath the emitter plate. The emitter serves as the top gate and the N-epi as the bottom gate.

The problem: the pinched base region is **wider and more heavily doped than optimal**, giving a pinchoff voltage of $|V_P| \approx 10$-$18$ V. This typically exceeds the breakdown voltage of the structure (~7 V), making the device useless as a JFET.

To create a useful JFET, two goals must be met simultaneously:
1. **Reduce** $|V_P|$ below 5 V
2. **Raise** breakdown voltage above 30 V

Two approaches exist, both requiring **one additional mask**:

**Approach A -- Deeper, lighter emitter replacement:** Replace the emitter pinch plate with a somewhat deeper and more lightly doped N-type diffusion. The deeper it penetrates into the base, the thinner and more lightly doped the channel becomes, reducing $|V_P|$. The lighter doping also allows the depletion region to encroach into the pinch plate, raising the breakdown voltage.

**Approach B -- Shallower, lighter base replacement:** Retain the ordinary emitter as the pinch plate but replace the base diffusion with a shallower, more lightly doped P-type diffusion. This again reduces channel thickness and doping, lowering $|V_P|$. The depletion region around the emitter pinch plate can extend further into the lighter P-type region, raising breakdown voltage. One reported device using this structure achieved $V_P \approx 2$ V and a breakdown voltage of ~40 V (Wilson, 1968).

**Why double-diffused PJFETs became obsolete:** Although used in early BiFET op-amps (including the Fairchild uA740), the pinchoff voltage was **extremely sensitive to exact junction depths** of the base and pinch plate. The diffusion technologies of the early 1970s could not maintain tight enough control -- $V_P$ could vary by tens of millivolts across a single die. Even common-centroid layout techniques could only partially compensate. The double-diffused PJFET was superseded by the ion-implanted JFET.

### Ion-Implanted JFET

Ion implantation achieves **much more accurate control** of junction depth and doping than thermal diffusion from a surface source. This is the key advantage that makes precision JFETs practical.

**Construction method:**
1. Implant a relatively **deep, lightly doped P-type region** to form the device body/channel
2. Implant an extremely **shallow, heavily doped N-type region** to form the top gate

Because the top gate is so thin, essentially the **entire implant dose** of the P-type region goes into the channel. This means the conductivity and pinchoff voltage can be controlled with considerable precision -- the channel properties are determined almost entirely by the implant dose, which is one of the most precisely controllable parameters in semiconductor fabrication.

The first ion-implanted PJFETs appeared in the **LF155 op-amp** released in 1974 (Russell and Culmer).

**Device structure (Figure 12.46):**
- Requires **two additional masking steps** beyond the standard bipolar process:
  1. A mask for the P-type channel implant
  2. A mask for the shallow N-type top gate implant
- Two **base regions** placed at either end of the channel terminate the source and drain
- The N-type gate extends out into the N-epi tank, **shorting the gate and backgate** to create a three-terminal PJFET
- Gate contacts are **not placed in the N-type implant region** due to the risk of **contact spiking**; instead, they are placed inside an emitter diffusion

**Layout styles:**
- The standard layout uses a **wide, short** channel geometry, typical for input differential pairs in BiFET amplifiers
- **Multiple gate fingers** placed in parallel create compact wide devices, resembling multifinger MOSFET layouts

### Annular JFET (Separate Gate and Backgate)

In all the standard layouts above, the top gate extends beyond the JFET body into the backgate, shorting gate and backgate into a three-terminal device. However, some applications benefit from **separate connections** to the top gate and backgate:

- The backgate has a parasitic capacitance to the substrate that the top gate does not
- A circuit requiring minimal leakage and input capacitance benefits from connecting the input to the **top gate** and tying the backgate to a reference voltage

An **annular layout** (Figure 12.47) separates the top and back gates. The width and length of an annular JFET are computed using the same equations as for annular MOS transistors (Section 12.2.8).

## Diagrams

### Figure 12.44 -- N-well JFET Layout and Cross Section

![[diagrams/ch12-jfet-fig1.png]]

*Layout and cross section of an N-well JFET constructed in an analog BiCMOS process. The N-well forms the channel, while the PMoat plate provides the top gate. NSD contacts connect to the source and drain at either end of the N-well channel. The thick field oxide separates the active regions. Note how the PMoat pinch plate must extend well beyond the N-well boundary into the isolation -- this is because the well diffuses outward, unlike standard bipolar isolation which diffuses inward.*

### Figure 12.45 -- Double-Diffused PJFET Structures

![[diagrams/ch12-jfet-fig2.png]]

*Three variants of double-diffused PJFET construction. (A) The basic base pinch resistor structure -- the emitter acts as the top gate and N-epi as the bottom gate, but pinchoff voltage is too high (~10-18 V) and breakdown too low (~7 V). (B) Modified structure using a deeper, lighter N-type diffusion as the pinch plate instead of the emitter -- the deeper penetration thins the channel and reduces $V_P$. (C) Modified structure using a shallower, lighter P-type diffusion instead of the standard base -- again reduces channel thickness and doping. Both (B) and (C) require one additional mask step.*

### Figure 12.46 -- Ion-Implanted PJFET

![[diagrams/ch12-jfet-fig3.png]]

*Layout and cross section of a typical ion-implanted PJFET. The P-implant forms the channel, while the very shallow N-implant serves as the top gate. Base regions at either end create source and drain contacts. The N-implant extends into the N-epi tank, shorting gate to backgate for a three-terminal device. Gate contacts are placed in emitter diffusions (not in the N-implant) to avoid contact spiking. This structure allows precise control of $V_P$ through the implant dose.*

## Practical Takeaways

- **Pinchoff voltage target:** Aim for $|V_P| \approx 5$ V for practical JFET operation. Below this, manufacturing control is difficult; above this, supply voltage requirements become burdensome.
- **Epi-FET layout:** Use minimum drawn width for compactness. Serpentine the channel to fill unused areas. Always place contacts over the base pinch plate tied to substrate potential to reduce debiasing-induced current variation. Rounded bends are harmless but provide negligible benefit.
- **N-well JFET caution:** Check whether the well has a retrograde profile before attempting to use it as a JFET. If so, the device will have an unacceptably high $|V_P|$ and should be treated as a nonlinear resistor instead.
- **N-well JFET pinch plate extension:** The pinch plate must extend much further into isolation than for an epi-FET because the well diffuses outward. Insufficient extension will leave the channel uncovered.
- **Avoid double-diffused JFETs** for precision applications -- the pinchoff voltage is too sensitive to junction depth variations across the die.
- **Ion-implanted JFETs are the gold standard** for precision analog applications (e.g., BiFET op-amp input pairs). The implant dose provides excellent control over $V_P$.
- **Contact spiking risk:** Never place contacts directly in the thin N-type gate implant of an ion-implanted JFET. Use emitter diffusion regions for gate contacts.
- **Multifinger layouts** for ion-implanted JFETs create compact wide devices and closely resemble multifinger MOSFET layouts.
- **Separate gate/backgate connections** using annular layouts when the application requires minimum input capacitance or leakage -- connect the signal to the top gate and tie the backgate to a quiet reference.
- **Width effects matter:** For both epi-FETs and N-well JFETs, the relationship between drawn and effective width is nonlinear at small widths due to lateral diffusion. Minimum-width N-well JFETs may have unacceptably low pinchoff voltages.

## Relation to the Bigger Picture

The JFET section completes Chapter 12's coverage of field-effect transistors by bridging the gap between MOSFET devices (covered earlier in Sections 12.1-12.3) and the specialized nonvolatile memory structures of Section 12.3 ([[ch12-nvm]]). JFETs are essential components in BiFET operational amplifiers, where their extremely high input impedance and low input bias current make them superior to bipolar transistors for certain front-end applications. The evolution from double-diffused to ion-implanted JFETs illustrates a recurring theme throughout the book: process control and manufacturability often determine which device structures are practical, not just theoretical performance. The foundational JFET physics covered here connects directly to the introductory treatment in [[ch01-jfet-transistors]], while the layout techniques (serpentining, annular geometries, multifinger structures) draw on the same principles used for MOSFET layout earlier in this chapter.

## See Also
- [[ch12-nvm]]
- [[ch01-jfet-transistors]]
