---
title: "5.3 Surface Effects"
chapter: 5
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-5, surface-effects, hot-carrier-injection, NBTI, zener-walkout, parasitic-channels, charge-spreading]
---

# 5.3 Surface Effects

> **Chapter 5: Failure Mechanisms**

## Key Concepts

Section 5.3 addresses failure mechanisms that occur at interfaces within integrated circuits -- primarily the silicon/oxide interface, but also the interlevel oxide/nitride overcoat interface and the protective overcoat/plastic encapsulant boundary. These **surface effects** are unified by a common theme: they involve the creation of superficial traps or the lateral movement of charge along interfaces, and they produce **gradual parametric shifts under bias** that stop when bias is removed and resume when it is reapplied.

A critical unifying mechanism across several of these effects is **dehydrogenation**. During the final stages of manufacturing, hydrogen migrates through gate oxide and passivates dangling bonds at the silicon-oxide interface by forming Si-H bonds. Various stress conditions (hot carriers, avalanche breakdown, thermal activation under bias) can break these Si-H bonds, regenerating the dangling bonds. These dangling bonds then act as:
- **Recombination centers** (degrading bipolar beta)
- **Trapping sites** (shifting MOS threshold voltage)
- **Fixed oxide charges** (modifying depletion widths)

An important practical feature of surface effects is their **partial reversibility**: an unbiased bake at $150$-$250\degree\text{C}$ for 10 minutes to several hours can partially reverse the parametric shifts. Temperatures at the low end reverse charge spreading; temperatures near the upper end partially reverse trap-induced effects. However, the shifts reappear once bias is restored.

---

## 5.3.1 Hot-Carrier Injection (HCI) in MOS Transistors

### Physics of Hot Carriers

Carriers in thermal equilibrium with the lattice are called **thermal carriers**. Their energies are distributed about a mean of $\frac{3}{2}kT$, where $k = 8.616 \times 10^{-5}\ \text{eV/K}$ is Boltzmann's constant. At room temperature ($T \approx 300\ \text{K}$), the mean thermal energy is about $30\ \text{meV}$. The silicon-oxide interface presents a barrier of roughly:

- **~3.2 eV** for electrons
- **~4.7 eV** for holes

Since virtually no thermal carriers have 10 times the mean thermal energy, the silicon-oxide interface is essentially impenetrable to thermal carriers. However, electric fields of several MV/cm can accelerate carriers to kinetic energies far exceeding thermal equilibrium -- these are called **hot carriers** (a misnomer, since "temperature" only applies to systems in thermal equilibrium).

### Channel Hot-Carrier (CHC) Injection

The dominant HCI mechanism in MOS transistors is **channel hot-carrier injection**. Consider an NMOS in saturation:

1. Most of $V_{DS}$ drops across the **pinched-off region** near the drain
2. The peak electric field occurs at the **drain-backgate metallurgical junction**
3. Electrons traversing this high-field region become hot carriers
4. Lattice collisions generate **electron-hole pairs** (impact ionization)
5. Holes drift to backgate (measurable as $I_{sub}$, the substrate current)
6. Most electrons drift to drain, but a few "lucky electrons" scatter with enough energy to penetrate the oxide interface
7. The resulting **gate current** ($I_G$) is 3-5 orders of magnitude smaller than $I_{sub}$

The gate-to-source voltage plays a critical role: **gate current is maximized when $V_{GS} \approx 0.4 \cdot V_{DS}$**, and diminishes at higher gate voltages.

At low $V_{GS}$ (near threshold), holes are preferentially injected into the oxide because:
- The pinched-off region is biased high relative to the gate
- A vertical field propels holes against the oxide interface
- **Band bending** in the depletion region lowers the effective barrier height
- The **Schottky effect** further reduces the barrier under intense fields

As $V_{GS}$ rises, the vertical field diminishes and hole injection tapers off while electron injection increases.

### PMOS vs. NMOS

PMOS transistors also exhibit CHC injection, but are **less susceptible** because holes have lower mobility than electrons, requiring about **twice the electric field** to sustain the same level of impact ionization.

### Drain Avalanche Hot-Carrier (DAHC) Injection

When a MOS transistor operates in avalanche breakdown, the high electric field generates large numbers of hot carriers through impact ionization. DAHC injection adds to CHC effects and is the mechanism used in **EPROM** transistors for programming.

### Damage Mechanisms

Hot carriers damage gate oxide through three principal mechanisms:

1. **Dehydrogenation** (likely dominant): Multiple successive hot carrier impacts break Si-H bonds at the interface, regenerating dangling bonds. This creates interface traps.
2. **Electron trapping**: Electrons become trapped within the oxide bulk, creating localized negative charges.
3. **Bond breakage**: Strained Si-O bonds within the amorphous oxide macromolecule are broken by hot carrier impact.

### Resulting Parametric Shifts

| Parameter | NMOS | PMOS |
|-----------|------|------|
| Transconductance ($g_m$) | Decreases (most pronounced in linear region) | May increase or decrease |
| $V_{th}$ shift | Non-linear: first shifts down, then up (can reach tens to hundreds of mV) | Shifts, but generally smaller magnitude |

The threshold voltage behavior is non-monotonic: initially, dehydrogenation creates positive fixed oxide charge (shifting $V_{th}$ down), then hot electron injection neutralizes and overloads traps with electrons (shifting $V_{th}$ above the original value).

### Preventative Measures

**Region-of-operation restriction:**
- Quantify $V_{th}$ shift and $g_m$ degradation as functions of time, $V_{DS}$, and $V_{GS}$
- Use contour plots of allowed operating duty cycle vs. $V_{DS}$ and $V_{GS}$ (e.g., time to 10% linear transconductance degradation)
- For matched input pairs (e.g., comparator inputs), insert **cascodes** to equalize $V_{DS}$ across matched transistors

**Drain engineering (process-level):**
- **Reduced backgate doping** near drain: Allows the pinched-off region to extend further, reducing peak field. Requires increased $L_{min}$ due to channel-length modulation. RESURF transistors (Section 13.1.5) use this approach.
- **Lightly Doped Drain (LDD) / Double-Doped Drain (DDD)**: Lighter drain doping extends depletion into drain, reducing peak field without altering channel. Increases on-resistance.
- **Deuterium annealing**: Replaces hydrogen with deuterium at Si-SiO$_2$ interface. Deuterium's doubled atomic mass causes it to resist desorption through the **giant isotope effect**, reducing degradation rate by $10\times$ or more.
- **Nitrogen addition** to gate oxide: Has also shown HCI improvement.

**Channel length increase (layout-level):**
- Since HCI only occurs near the drain, its impact on $V_{th}$ and $g_m$ diminishes as $L$ increases
- Increasing channel length by $0.5$-$1.0\ \mu\text{m}$ can provide a few extra volts of operating margin

---

## 5.3.2 Zener Walkout and Walkback

### Phenomenon

Any PN diode operated in reverse breakdown is colloquially called a "Zener." If breakdown occurs near the surface (**surface Zener**), the device is susceptible to time-dependent shifts in breakdown voltage:

- **Zener walkout**: Breakdown voltage increases as a function of total injected charge
- **Zener walkback**: After walkout reaches a maximum, breakdown voltage decreases and converges on a final value below the initial breakdown voltage

Standard bipolar Zeners (base-emitter junction) exhibit pure walkout -- the voltage rises and asymptotically converges on a value several hundred millivolts above the initial value.

BiCMOS Zeners exhibit an **inflection point**: the voltage first walks out, reaches a maximum, then walks back to a final value several hundred millivolts *below* the initial value. In some cases, walkouts of **several volts** have been observed. Seemingly identical Zeners can exhibit radically different walkout magnitudes.

### Mechanism -- Classical Model (Standard Bipolar)

In a standard bipolar base-emitter Zener:

1. The base diffusion is most concentrated at the surface, so the depletion region narrows there -- device acts as a surface Zener
2. Dangling bonds at the oxide interface are passivated by hydrogen during PO deposition and contact sinter
3. Hot carriers from avalanche breakdown break Si-H bonds, regenerating dangling bonds
4. Dangling bonds capture holes, creating **positive fixed oxide charge** above the anode
5. This positive charge widens the depletion region at the surface, increasing the breakdown voltage (walkout)
6. Eventually all Si-H bonds are broken and the charge converges, so walkout ceases

A $200\degree\text{C}$ bake will partially (but not entirely) reverse walkout.

### Mechanism -- Hydrogen Compensation (BiCMOS)

A more dramatic mechanism explains the large walkouts and subsequent walkbacks seen in some processes:

1. Hot carriers from surface avalanche desorb hydrogen atoms at the oxide-silicon interface
2. Some hydrogen ions diffuse into the depletion region and **complex with boron ions** (hydrogen compensation)
3. Complexed boron no longer ionizes, reducing effective P-type doping
4. The depletion region widens and breakdown voltage increases (large walkout)
5. When breakdown moves subsurface, no more hydrogen is released
6. The weak boron-hydrogen complexes gradually decompose
7. The depletion region narrows and breakdown voltage **walks back**

This effect is worst at low operating currents (near $1\ \mu\text{A}$). Closing protective overcoat openings above test pads (trapping hydrogen in the interlevel oxides) has been observed to trigger this mechanism. Titanium-tungsten barrier metal or titanium silicide has reduced walkout in some processes, possibly by gettering hydrogen.

### Preventative Measures

- **Buried Zeners**: Incorporate high-energy implants or buried layers to confine avalanche breakdown $\geq 1\ \mu\text{m}$ beneath the oxide interface. Buried Zeners exhibit no walkout or walkback and form the basis for high-precision voltage references.
- **Reference diodes**: Buried Zeners with carefully chosen doping produce $V_{BR} \approx 6.3\ \text{V}$ where the positive temperature coefficient of avalanche and the negative TC of Zener breakdown cancel, giving minimal temperature variation.
- **Field plates**: A conductor biased to generate a field across the oxide, controlling depletion/accumulation. For base-emitter Zeners, connect a metal-1 field plate to the emitter terminal and extend it beyond the drawn junction edges by several microns. The base contact can be moved back to make room. However, the emitter-base breakdown voltage is too low for the field plate to significantly affect the base-emitter depletion region -- the field plate primarily mitigates **charge spreading** (Section 5.3.5).
- **Independently biased field plates**: An annular metal-1 geometry connected to a separate higher-voltage supply could theoretically drive breakdown subsurface, but no published experimental evidence exists for its effectiveness.

---

## 5.3.3 Avalanche-Induced Beta Degradation

### Phenomenon

Avalanching the base-emitter junction of a bipolar transistor reduces its $\beta$ (current gain). Key observations:

- **Vertical NPN** is far more susceptible than **lateral PNP**
- **Polysilicon-emitter transistors** are more vulnerable than diffused-emitter transistors
- Degradation is much more pronounced at **low currents** (one device showed ~80% low-current $\beta$ drop but only ~12% high-current $\beta$ drop)
- Degradation increases with total injected charge and asymptotically approaches a limit
- **Reversible** with unbiased bakes: $150\degree\text{C}$ for 2 minutes reverses ~60%, $200\degree\text{C}$ for 5 minutes reverses ~95%

### Mechanism

The same dehydrogenation and trap generation mechanisms as HCI and Zener walkout apply here. The traps generated at the oxide interface act as **recombination centers** that increase surface recombination. When traps appear above the emitter-base depletion region, they disproportionately affect low-current beta (where surface recombination current dominates over bulk recombination).

**Why poly-emitter transistors are more vulnerable**: The emitter diffusion is extremely thin, with the polycrystalline/monocrystalline interface immediately above it. Dangling bonds along this interface are passivated by hydrogen, and the entire emitter-base junction lies near this surface. By contrast, in diffused-emitter transistors, only the perimeter of the emitter-base depletion region lies near the surface.

**Lateral PNP immunity**: Lateral transistors on standard bipolar generally do not exhibit avalanche-induced beta degradation, possibly because they break down by **punchthrough** before avalanching.

**Interesting anomaly**: Some poly-emitter transistors show *increased* medium-current beta after high-current operation, apparently because hydrogen migrates into the polysilicon emitter interface and ties off dangling bonds.

### Preventative Measures

- **Heavy emitter doping**: Trap sites can be passivated by dopant atoms. Heavier doping of emitter polysilicon reduces susceptibility. Arsenic is slightly more effective than phosphorus.
- **Operating limits**: Never operate base-emitter junctions beyond ~75% of $BV_{EBO}$. Poly-emitter transistors should not exceed $V_{BE}$ of more than a couple of volts.
- **ESD protection**: Base-emitter junctions connected to pins are susceptible to beta degradation from ESD strikes. Add ESD protection clamps (Section 14.4.2) or redesign circuits to avoid connecting base-emitter junctions to pins.

---

## 5.3.4 Negative-Bias Temperature Instability (NBTI)

### Phenomenon

NBTI primarily affects **PMOS transistors** and causes:
- $V_{th}$ shifts downward (more negative) under negative gate bias
- Transconductance decreases
- Higher temperatures accelerate the shift

**NBTI recovery**: A portion of the shift vanishes within seconds of removing bias, even at room temperature. This rapid recovery was long overlooked because it occurred during the interval between removing bias and measuring parameters. AC excitation produces less NBTI shift than DC, likely because the unbiased half-cycles partially reverse the shifts.

**Positive-Bias Temperature Instability (PBTI)**: A symmetric mechanism affects NMOS transistors under positive gate bias. PBTI primarily affects advanced devices with **high-$\kappa$ dielectrics**, but smaller shifts occur even in polysilicon-gate transistors with conventional oxides.

### Mechanism

The mechanisms are not completely understood, but the consensus involves:

1. Holes drawn to the oxide-silicon interface by negative gate bias combine with an unknown reactant species (candidates: water molecules, hydrogen ions, molecular hydrogen from boron-hydrogen complex dissociation)
2. This reaction breaks Si-H bonds, creating **dangling bonds**
3. Dangling bonds trap holes (under the depleted/enhanced PMOS field conditions), creating net positive charge
4. Positive charge shifts $V_{th}$ more negative
5. The reaction appears **reversible**, explaining NBTI recovery

**NMOS vs. PMOS**: NMOS transistors also show NBTI shifts, but much smaller. The difference arises because interface traps generate positive trapped charge in PMOS but negative trapped charge in NMOS. Deeper oxide traps generate positive charge in both cases. The two mechanisms add in PMOS and partially cancel in NMOS.

### Why Modern Processes Are More Vulnerable

| Factor | Effect on NBTI |
|--------|---------------|
| Thinner gate oxides with higher $E$-fields | Increases NBTI |
| Dual-doped gate poly (surface-channel PMOS replaces buried-channel) | Brings holes closer to interface |
| Oxynitride gate dielectric | Substantially worse NBTI than pure oxide |
| High-$\kappa$ dielectrics (for PBTI) | Electron trapping in dielectric |

Oxynitride is used because it has higher permittivity (allowing thicker dielectric with less tunneling leakage) and resists boron penetration from P-type poly gates, but it exacerbates NBTI.

### Preventative Measures

- **Deuterium annealing**: Provides slight improvement for NBTI (dramatic for HCI)
- **Fluorine addition** to gate oxide: Fluorine moderates bond distortions at the interface. Si-F bonds are stronger than Si-H bonds and less likely to break.
- **Circuit design**: Identify matched PMOS transistors operating at different $V_{GS}$ values. Larger voltage differentials magnify mismatch from BTI. Alter circuits to bias matched transistors under identical conditions. Use circuit simulators that model NBTI/PBTI to find vulnerable transistors.

---

## 5.3.5 Parasitic Channels and Charge Spreading

### Thick-Field Thresholds

Any conductor above an oxide can invert the underlying silicon if the voltage differential is large enough. The **thick-field threshold** $V_{TF}$ depends on the conductor layer, dielectric stack, and backgate doping. For a typical $5\ \text{V}$ CMOS process:

| Gate | Dielectric | Backgate | Typical $V_{TF}$ |
|------|-----------|----------|------------------|
| Polysilicon (NMOS) | Thick-field oxide | N-well | Moderate (~15 V) |
| Polysilicon (PMOS) | Thick-field oxide | P-epi | Moderate (~15 V) |
| Metal-1 | TOX + MLO | N-well / P-epi | Higher |
| Metal-2 | TOX + MLO + ILO | N-well / P-epi | Highest |

Higher metal layers have larger thick-field thresholds because additional interlevel oxides lie beneath them.

### Six Conditions for Parasitic Channel Conduction

For a parasitic channel to conduct:

1. A **lightly doped silicon region** must serve as backgate
2. A region of **opposite polarity** acts as source
3. Another region of **opposite polarity** acts as drain
4. A **conductor or static charge** lies between source and drain as gate
5. The gate-to-source voltage must approach or exceed $V_{TF}$ (including body effect)
6. A **nonzero $V_{DS}$** must exist

### Examples of Parasitic Channels

**Standard Bipolar:**
- **Parasitic PMOS**: N-epi tank = backgate, base diffusion = source, P-isolation = drain, metal lead = gate. Channel forms when voltage between base and metal exceeds metal-1 PMOS $V_{TF}$.
- **Parasitic NMOS**: P-isolation = backgate, two adjacent tanks = source/drain, metal lead = gate.

**CMOS:**
- **Parasitic PMOS**: N-well = backgate, PSD inside well = source, surrounding P-epi = drain, poly lead = gate. Occurs when $V_{GS}$ exceeds poly PMOS $V_{TF}$.
- **Parasitic NMOS**: P-epi or P-well = backgate, two adjacent N-wells = source/drain, metal-1 lead = gate.

### Charge Spreading

Parasitic channels can form **even without conductors** serving as gates. Static charges accumulate on insulating surfaces and drift laterally in response to electric fields -- this is **charge spreading**. Key characteristics:

- Devices initially operate normally but develop leakage after extended biased operation
- Leakage currents are typically a few microamps
- High temperature under bias accelerates the effect
- An **unbiased bake eliminates it**
- Parasitic channels formed by charge spreading always bridge between **P-type regions**
- **Nitride protective overcoats** increase susceptibility vs. oxide overcoats
- The charges responsible are believed to be a negatively charged "water-related species" at the nitride-oxide or overcoat-encapsulant interface

### Sodium Amplification

Charge spreading can invert silicon even when operating voltages are far below the thick-field threshold. The mechanism involves **sodium amplification**:

1. Mobile Na$^+$ ions are uniformly distributed through oxide, balanced by immobile negative countercharges
2. Negative charge accumulating at the nitride-oxide interface attracts Na$^+$ to this interface
3. The immobile negative countercharges remain closer to the silicon surface
4. Since charges closer to the silicon exert greater influence, the sodium ions effectively **amplify** the impact of charge spreading
5. This can invert silicon at operating voltages far below the expected thick-field threshold

### Preventative Measures -- Standard Bipolar

**For parasitic NMOS channels (across isolation):**
- Code **base diffusion over isolation (BOI)** to increase surface doping. BOI consumes no additional area since isolation spacings exceed base spacings.

**For parasitic PMOS channels (across N-epi tanks) and charge spreading:**

All P-type regions whose voltage relative to substrate exceeds the PMOS $V_{TF}$ require protection. Conservative practice: protect any P-type region operating at $\geq 75\%$ of the maximum top-metal PMOS thick-field threshold. Use:

- **Channel stops**: Minimum-width strips of emitter diffusion interrupting the parasitic channel path beneath a lead. Must overhang the lead by at least two-level misalignment plus $2\times$ oxide thickness.
- **Field plates** (preferred): A conductive layer biased to inhibit channel formation, placed over the vulnerable diffusion. Connect to the highest available potential to most strongly suppress PMOS channel formation.

**Field plate design principles:**
- Overhang the protected region enough for outdiffusion, misalignment, and fringing fields ($2\times$ oxide thickness)
- The field plate connected to the **highest potential** should cover as much of the device as possible
- Gaps in field plates can be mitigated by: (a) **flanging** -- extending plates to make gaps long and narrow, (b) bridging gaps with **channel stop strips**, (c) using a higher metal layer that needs no gap
- Field plates also block capacitive noise coupling from overlying leads

**Every lateral PNP transistor** should have a field plate covering the exposed base between emitter and collector, connected to the emitter terminal. This is critical because even charge accumulations far below inversion can cause $>30\%$ beta increases through conductivity modulation.

### Preventative Measures -- CMOS and BiCMOS

- Channel-stop implants typically raise metal-1 $V_{TF}$ above operating voltage
- Parasitic channels may still form beneath **poly leads** or **metal leads to high-voltage devices**
- For PMOS channels beneath poly: retract the poly lead inside the N-well. The N-well should overlap poly by the same amount N-well overlaps PSD, plus $1$-$2\ \mu\text{m}$.
- For PMOS channels beneath metal: reroute to a higher metal layer, or insert a field plate (minimum-width poly or lower metal strip connected to $\geq V_{well}$), or place a channel stop (minimum-width NMoat strip)
- Charge spreading seldom occurs in modern CMOS/BiCMOS below the top-metal $V_{TF}$ (typically $>50\ \text{V}$) due to process cleanliness
- In **high-voltage** CMOS/BiCMOS, ionic contaminants from plastic encapsulation can migrate on the protective overcoat surface. Field plating with the highest available metal layer can help, or a semi-insulating protective overcoat may be needed.

---

## 5.3.6 Substrate Influence (DI Processes)

In **dielectrically isolated** (DI) processes, the handle (substrate) is electrically insulated from the superficial silicon by the buried oxide (BOX). However, the handle/BOX/superficial-silicon sandwich forms a **parasitic MOS transistor** where:

- Handle = gate
- BOX = dielectric
- Superficial silicon = backgate

The voltage difference between handle and superficial silicon can **deplete or enhance** the bottom of the superficial silicon layer.

### Failure Mechanisms

If the substrate connection opens or is absent, electrostatic charge accumulates on the handle, causing:
- Depletion in the superficial silicon
- Reduced breakdown voltages of high-voltage structures (from intersecting depletion regions)
- Supply current leakages (mechanism not fully understood)
- Time-dependent behavior similar to charge spreading

### Preventative Measures

Establish a reliable connection to the handle via **backside contact**. Three requirements:

1. **Remove backside oxide** (backgrind strips it; engineering samples may retain it)
2. **Conductive die attach** (silver-filled epoxy, gold eutectic, or solder)
3. **Electrical connection** from die mount pad to lowest-voltage pin via:
   - **Downbonds**: Bondwire from lead finger to mount pad. Use two downbonds for detection of shearing. Vulnerable to delamination.
   - **Fused leadframes** (superior): Lead finger physically joins to mount pad. Ensures reliable contact. Custom-manufactured per product but costs have decreased with etched leadframes.
   - **Through-silicon vias**: Selectively etch through superficial silicon and BOX, backfill with doped polysilicon. Requires one additional mask step.

---

## Diagrams

### Figure 5.12 -- Channel Hot-Carrier Injection in NMOS

![[diagrams/ch05-surface-effects-fig1.png]]

Cross-section showing the mechanism of channel hot-carrier generation in an NMOS transistor operating in saturation. Most of $V_{DS}$ drops across the pinched-off region near the drain. The peak electric field at the drain-backgate metallurgical junction accelerates electrons to become hot carriers. Impact ionization generates electron-hole pairs; holes exit through the backgate while "lucky electrons" scatter into the gate oxide.

### Figure 5.15/5.16 -- Zener Walkout Mechanism and Field Plate

![[diagrams/ch05-surface-effects-fig2.png]]

Cross-sections showing the classical model of Zener walkout. (A) Initial condition: hot carrier generation occurs near the surface during avalanche breakdown. (B) After extended operation: dehydrogenation creates positive fixed oxide charge above the anode, widening the depletion region and increasing breakdown voltage. Also shows an emitter field plate layout for a base-emitter Zener in standard bipolar.

### Figure 5.17 -- Examples of Parasitic Channels

![[diagrams/ch05-surface-effects-fig3.png]]

Four examples of parasitic channel formation: (A) parasitic PMOS in standard bipolar (base-to-isolation across N-epi tank), (B) parasitic NMOS in standard bipolar (tank-to-tank across isolation), (C) parasitic PMOS in CMOS (PSD-to-P-epi across N-well), and (D) parasitic NMOS in CMOS (N-well to N-well across P-epi). Each shows the conductor acting as a gate and the conditions for channel formation.

---

## Practical Takeaways

### Hot-Carrier Injection
- HCI damage is maximized when $V_{GS} \approx 0.4 \cdot V_{DS}$ -- be wary of this operating point
- Increase channel length by $0.5$-$1.0\ \mu\text{m}$ for extra voltage margin on sensitive transistors
- Use cascodes to equalize $V_{DS}$ across matched input pairs (comparators, diff pairs)
- NMOS is more susceptible than PMOS; inspect NMOS transistors first

### Zener Diodes
- **Never use surface Zeners** for precision voltage references -- use buried Zeners instead
- Apply emitter field plates (metal-1 connected to emitter, extending several microns beyond drawn junction) to all base-emitter Zener diodes
- Be aware that closing PO test pad openings can trap hydrogen and worsen walkout
- Walkout behavior can vary radically between seemingly identical devices

### Avalanche-Induced Beta Degradation
- Never operate base-emitter junctions beyond ~75% of $BV_{EBO}$
- Poly-emitter transistors: limit $V_{BE}$ to no more than a couple of volts
- Protect base-emitter junctions on pins with ESD clamps
- Low-current beta is affected far more than high-current beta

### NBTI
- Inspect matched PMOS transistors for differential $V_{GS}$ bias conditions -- this creates mismatch over time
- Oxynitride dielectrics (common in advanced nodes) substantially worsen NBTI
- AC operation produces less NBTI than DC due to partial recovery during unbiased intervals
- Use circuit simulators that model BTI to identify vulnerable transistors early

### Parasitic Channels and Charge Spreading
- **Field plate everything**: Every lateral PNP needs a field plate on the base surface. Every high-voltage P-type region in standard bipolar needs protection.
- Conservative threshold: protect P-type regions operating at $\geq 75\%$ of the top-metal PMOS thick-field threshold
- Field plates connected to the **highest available voltage** provide the best suppression
- Field plates are preferred over channel stops because they are easier to insert and offer comprehensive protection (including against charge spreading)
- In CMOS, retract poly leads of high-voltage transistors inside the N-well to avoid parasitic PMOS channels
- For DI processes, always establish a reliable backside contact to the handle -- fused leadframes are preferred over downbonds

---

## Relation to the Bigger Picture

Section 5.3 occupies a central position in Chapter 5's treatment of failure mechanisms, bridging between the immediate destructive mechanisms of electrical overstress (Section 5.1, [[ch05-contamination]] discusses related contamination effects in Section 5.2) and the minority carrier injection problems of Section 5.4 ([[ch05-minority-carrier-injection]]). Surface effects are particularly insidious because they cause *gradual* parametric drift rather than immediate failure, making them difficult to detect during initial testing but devastating in the field. The concepts introduced here -- dehydrogenation, interface traps, dangling bonds, field plating, channel stops -- recur throughout the book in the design of specific components (resistors in Ch. 8, bipolar transistors in Ch. 9, MOS transistors in Ch. 12-13, and ESD structures in Ch. 14). Understanding these mechanisms is essential for any analog layout designer because the mitigations (field plates, channel stops, buried Zener structures, adequate channel length) are layout decisions that cannot be corrected by circuit redesign alone.

## See Also
- [[ch05-contamination]]
- [[ch05-minority-carrier-injection]]
