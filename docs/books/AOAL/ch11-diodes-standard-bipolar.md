---
title: "11.1 Diodes in Standard Bipolar"
chapter: 11
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-11, diodes, standard-bipolar, zener, schottky, power-diode]
---

# 11.1 Diodes in Standard Bipolar

> **Chapter 11: Diodes**

## Key Concepts

Diodes in integrated circuits are not discrete components placed on a board -- they are constructed from the junctions available within the standard bipolar process. The designer's challenge is to repurpose transistor structures (base-emitter junctions, base-collector junctions, Schottky barriers) to create diodes with specific characteristics. This section covers four main diode types available in standard bipolar: **diode-connected transistors**, **Zener diodes** (emitter-base and buried variants), **Schottky diodes**, and **base-collector power diodes**.

The fundamental diode equation governing all PN junction and Schottky diodes is:

$$I = I_S \left( e^{V / nV_T} - 1 \right) \tag{11.1}$$

where $V$ is the voltage across the diode (positive = anode above cathode), $I_S$ is the saturation current, $n$ is the ideality factor (near unity for low-current PN junctions, can exceed unity for Schottkies and high-current operation), and $V_T$ is the thermal voltage:

$$V_T = \frac{kT}{q} \tag{11.2}$$

with $k = 1.381 \times 10^{-23}$ J/K and $q = 1.602 \times 10^{-19}$ C. At room temperature, $V_T \approx 26$ mV.

For a forward-biased diode with near-unity ideality factor:

$$V = V_T \ln\left(\frac{I}{I_S}\right) \tag{11.3}$$

**Temperature coefficients**: PN junction diodes conducting a small constant current exhibit a forward voltage temperature coefficient of approximately $-2$ mV/$^\circ$C. Schottky diodes are somewhat less negative, around $-1$ mV/$^\circ$C. The negative TC is dominated by the strong temperature dependence of $I_S$, not the positive TC of $V_T$.

---

## 11.1.1 Diode-Connected Transistors

### Operating Principle

A diode-connected transistor has its collector and base shorted together. Most current flows via transistor action (collector to emitter), but the device still obeys the diode equation at low currents. The series resistance is:

$$R_S = \frac{R_B}{\beta_F + 1} + R_E \tag{11.4}$$

where $R_B$ and $R_E$ are the internal base and emitter resistances and $\beta_F$ is the forward beta. The collector resistance can be ignored as long as the Ohmic drop across it stays below ~400 mV at 25$^\circ$C (or ~200 mV at 125$^\circ$C), which would otherwise forward-bias the base-collector junction. For a minimum-size device, $R_S$ is typically only a few ohms -- negligible for currents up to a few hundred microamps.

### Layout Guidelines

- **CBE configuration** is preferred over CEB because it simplifies the base-collector short connection.
- **Merged collector-base contacts** save significant area: the emitter diffusion surrounding the collector contact overlaps the base diffusion so a single contact touches both. The contact must extend far enough into the base to account for misalignment and outdiffusion while still providing adequate collector contact area.
- Devices conducting **more than a few hundred microamps** need a $N^+$ sinker to reduce collector resistance.
- Devices conducting **10 mA or more** should use a power transistor layout with $N^+$ sinker completely ringing the collector.

### Key Characteristics

| Property | Value |
|---|---|
| Forward voltage ($J = 1$ $\mu$A/$\mu$m$^2$, 25$^\circ$C) | ~0.65 V |
| Voltage shift per current doubling | +18 mV (at 25$^\circ$C) |
| Temperature coefficient | ~ $-2$ mV/$^\circ$C |
| Reverse breakdown ($BV_{EBO}$) | 6-12 V (typically 6.8 V) |
| Series resistance (min-size) | A few ohms |

### Cautions

- **Reverse breakdown** is limited by $BV_{EBO}$ of the NPN. Reverse voltage should not exceed about **two-thirds of $BV_{EBO}$** to avoid avalanche-induced beta degradation, which increases series resistance.
- **Lateral PNP** versions make poor diode-connected transistors: large tanks (area + parasitic capacitance), substrate current leakage of 0.1-1% prevents accurate current matching. However, their high breakdown voltage (equals $BV_{CEO}$) makes them useful as **high-voltage clamping devices**.
- **Substrate PNP** versions send collector current to the substrate; currents above ~1 mA can debias the substrate enough to saturate the transistor.

---

## 11.1.2 Zener Diodes

### Physics of Reverse Breakdown

Two mechanisms cause reverse breakdown in PN junctions, and which dominates depends on the depletion region width (controlled by doping):

| Mechanism | Doping threshold (lighter side) | Breakdown voltage | Temperature coefficient |
|---|---|---|---|
| **Avalanche** | $< 10^{18}$ cm$^{-3}$ | > 5-6 V | Positive ($+2$ mV/$^\circ$C at 7 V, approaching $+6$ mV/$^\circ$C above 50 V) |
| **Zener (tunneling)** | $> 10^{18}$ cm$^{-3}$ | < 5-6 V | Negative ($-2$ to $-5$ mV/$^\circ$C) |

**Avalanche breakdown**: Hot carriers traversing the depletion region gain enough energy (electric field ~200 kV/cm in silicon) to create electron-hole pairs by impact ionization, which then accelerate and create more pairs -- a multiplicative cascade. Higher temperature increases lattice scattering, hindering carrier acceleration, hence the positive TC.

**Zener breakdown (quantum tunneling)**: When the depletion region narrows below ~4 nm (doping $> 10^{18}$ cm$^{-3}$), electrons tunnel directly through the barrier. More valence electrons become available for tunneling at higher temperatures, hence the negative TC.

At 5-6 V breakdown, the two mechanisms cancel and the **temperature coefficient is nearly zero**, making this range attractive for voltage references. However, true Zener (tunneling) breakdown exhibits **soft breakdown** (gradual I-V transition), while avalanche breakdown is much sharper.

### Noise in Breakdown Diodes

Avalanche breakdown generates significant noise. Breakdown does not occur uniformly but at discrete defect sites called **microplasmas**. Localized self-heating causes current to jump between microplasmas, producing **burst noise** (random telegraph signal / RTS noise). Operating at higher current densities merges microplasmas and reduces RTS noise -- Zener voltage references are therefore often biased at relatively high current densities.

### The Emitter-Base Zener

The emitter-base junction of an NPN transistor serves as a convenient Zener diode:

- **Breakdown voltage** = $BV_{EBO}$ of the NPN, typically **6.8 V** (older processes) to **~8 V** (newer processes with lighter base doping).
- **Temperature coefficient**: $+1$ to $+3$ mV/$^\circ$C (varies with breakdown voltage).
- **Initial tolerance**: ~$\pm 100$ mV with ion-implanted bases (older spin-on processes had $\pm 500$ mV).

**Layout rules** (Figure 11.4):
- Emitter = cathode, base = anode.
- The tank provides isolation; connect it to anode, cathode, or a voltage $\geq$ anode voltage. Do **not** leave it floating -- parasitic substrate PNP can bleed away anode current at elevated temperatures.
- Tank does **not** need a $N^+$ sinker (only leakage currents). NBL can be omitted if base-to-substrate voltage is small enough to avoid punchthrough.
- **Circular or oval emitters** prevent electric field intensification at corners. To check if a process needs rounded emitters: examine a rectangular emitter in low-current avalanche under a microscope in a dark room; if light appears only at corners, field intensification is present.
- **Maximum current**: ~1 $\mu$A per micron of drawn emitter periphery for continuous operation. A 10 $\mu$m $\times$ 10 $\mu$m emitter can safely conduct ~40 $\mu$A continuously. An order of magnitude more is safe for microsecond transients.

**Zener walkout**: The breakdown voltage increases over time as charge is injected across the junction (up to +250 mV). Caused by hot-carrier injection generating positive surface state charges at the Si/SiO$_2$ interface (breaking Si-O and Si-H bonds), widening the surface depletion region. Also, hydrogen liberated from broken Si-H bonds can deactivate boron dopants (**boron compensation**), effectively reducing P-type doping and further widening the depletion region.

**Nonisolated emitter-base Zener**: When the anode connects to substrate potential, the base diffusion can overlap the isolation, eliminating the large base-to-isolation spacing and saving considerable area. A tank beneath the emitter prevents isolation from reducing the Zener voltage. The trade-off is vulnerability to substrate debiasing and noise coupling.

### Buried Zeners

To eliminate Zener walkout entirely, the avalanching junction must lie deeper than the **thermalization distance** -- the distance within which hot carriers lose their energy to the lattice. For electrons, this is ~140 nm; for holes, ~60 nm. Since hydrogen desorption (the walkout mechanism) primarily involves holes, 60 nm depth is likely sufficient.

Several buried Zener structures exist in standard bipolar:

**1. Emitter-in-isolation buried Zener** (Figure 11.6): An enlarged emitter with a plug of $P^+$ isolation diffused inside it. The extra boron doping beneath the emitter center forces breakdown to occur subsurface. Works best with moderately doped isolation; heavily doped conventional $P^+$ isolation often yields too-low or too-variable breakdown voltages. Processes with up-down isolation are better suited because the upper isolation is shallower and less heavily doped.

**2. Special $P^+$ diffusion buried Zener** (Figure 11.7): Uses an additional $P^+$ diffusion (after isolation, before base) placed as a plug inside the emitter. The $P^+$ plug doping exceeds the base doping, forcing breakdown at the bottom of the emitter where $P^+$ concentration peaks. The emitter = cathode; the $P^+$ plug (contacted via enclosing base) = anode. Breakdown voltage is adjustable by altering the $P^+$ profile, typically targeted at the **zero-temperature-coefficient (0TC) point of 5.0-5.4 V**. Variation around this target is ~$\pm 200$ mV, but the temperature coefficient rarely exceeds $\pm 100$ $\mu$V/$^\circ$C. Combined with on-chip thermal stabilization, these Zeners can achieve voltage references with drift as low as $\pm 1$ $\mu$V/$^\circ$C.

**3. Ion-implanted buried Zener** (Figure 11.8): A high-energy implant (performed through thin oxide to minimize channeling) replaces the emitter diffusion. The dopant peak lies well below the surface. Its intersection with a base diffusion plug forms the buried Zener. Breakdown voltage controlled within ~$\pm 50$ mV thanks to ion implantation precision.

### High-Voltage Zener Diodes

For voltages above 9 V (ESD protection, MOS gate clamps), **stack multiple emitter-base Zeners in series**. Optionally add diode-connected transistors to fine-tune the total voltage while partially compensating the positive TC of the avalanche Zeners.

The **isolation-NBL Zener** (Figure 11.9) uses a plug of $P^+$ isolation diffused into NBL, enclosed in a tank contacted via emitter diffusion. Typical breakdown ~20 V but varies $\pm 1$ V or more across process corners. No Zener walkout (junction is deep). Much higher power handling than emitter-base Zeners of similar dimensions.

---

## 11.1.3 Schottky Diodes

### Physics

Schottky diodes exploit the rectifying barrier formed between a conductor (metal or silicide) and lightly doped semiconductor. The saturation current follows thermionic emission theory:

$$I_S = A^{**} T^2 A_S \cdot e^{-\phi_B / V_T} \tag{11.5}$$

where $A^{**}$ is the effective Richardson constant (~110 A/cm$^2$K$^2$ for N-type Si, ~32 A/cm$^2$K$^2$ for P-type Si), $A_S$ is the Schottky barrier area, and $\phi_B$ is the barrier height in volts.

The forward voltage relates to barrier height as:

$$V_F = \phi_B - V_T \ln\left(\frac{A^{**} T^2}{J}\right) \tag{11.6}$$

At $J = 1$ $\mu$A/$\mu$m$^2$ and 25$^\circ$C, $V_F$ is approximately 300 mV below $\phi_B$.

### Barrier Heights (Selected Materials on N-type Si)

| Material | $\phi_B$ (N-Si) | $\phi_B$ (P-Si) | Notes |
|---|---|---|---|
| Aluminum | 0.72 V | 0.58 V | Historically problematic (contact sinter issues) |
| Cobalt disilicide (CoSi$_2$) | 0.65 V | 0.45 V | |
| Platinum silicide (PtSi) | 0.87 V | 0.23 V | Low leakage, good up to ~125$^\circ$C |
| Palladium silicide (PdSi) | 0.75 V | 0.35 V | Ideal for antisaturation clamps |
| Titanium silicide (TiSi$_2$) | 0.61 V | 0.45 V | Marginal for N-type Schottkies |
| Nickel monosilicide (NiSi) | 0.67 V | 0.43 V | |

**Critical constraints**:
- $\phi_B < 0.6$ V causes excessive leakage at high temperatures -- **no P-type Si Schottkies are practical** with common materials.
- N-type surface doping must be $< 10^{17}$ cm$^{-3}$ to prevent tunneling that shorts out the barrier. Above ~$5 \times 10^{17}$ cm$^{-3}$, the contact becomes Ohmic (this is how Ohmic contacts are intentionally made to N-type Si).
- **Optimum barrier height** for practical Schottkies: **0.75-0.80 V** -- yields forward voltages several hundred mV below a base-emitter junction, ideal for antisaturation clamps.

### Why Aluminum Schottkies Failed

During contact sintering (~450$^\circ$C), silicon dissolves slightly into aluminum. Upon cooling, dissolved silicon redeposits at the interface carrying aluminum dopant, creating $P^+$ silicon patches that interfere with Schottky conduction. Low-temperature sintering ($< 400^\circ$C) fixed Schottkies but failed to adequately reduce Ohmic contact resistance. The industry moved to **refractory barrier metal + silicide** stacks (e.g., PtSi/TiW/Al), which simultaneously solved Ohmic contact reliability (no junction spiking) and provided a Schottky-capable silicide.

### Field-Plated Schottky Diodes

The abrupt edges of a Schottky contact opening severely intensify the electric field, causing soft breakdown at only a few volts reverse bias despite the planar breakdown being much higher. A **field plate** -- metallization flanged beyond the contact opening over the field oxide -- creates a vertical electric field that repels electrons from the surface and extends the depletion region, raising the effective breakdown voltage by a few volts.

The construction requires an additional **Schottky contact mask** (oxide etchback step) performed after isolation drive but before base patterning. This thins the thick field oxide over the Schottky opening so it clears at the same time as the thinner base/emitter contact oxides. The Schottky contact geometry overlaps the regular contact geometry by several microns beyond misalignment requirements -- this creates two shallow oxide steps (better metal coverage) and forms the field plate over the thin oxide.

**Cathode construction**: NBL and $N^+$ sinker minimize series resistance. Low-current devices can use a small $N^+$ plug; higher-current devices need a complete $N^+$ ring around the tank periphery, which also acts as a **hole-blocking guard ring** (Schottky conduction is primarily majority-carrier, but a small minority-carrier component exists at high current densities).

### Guard-Ringed Schottky Diodes

For higher voltage applications, a narrow ring of **base diffusion** (field relief guard ring) encloses the Schottky contact edge. This completely eliminates lateral field intensification, raising the breakdown voltage to equal $BV_{CBO}$ of the NPN transistor. The cost is significantly more area, especially for small devices, because spacing must increase to avoid punchthrough between the guard ring and isolation, and outdiffusion constricts the contact opening by several microns on each side.

**Design heuristic**: Use **guard rings on large Schottkies** (negligible relative area penalty, much more predictable breakdown) and **field plates on small Schottkies** (area savings outweigh the less predictable breakdown).

### Schottky Transistors

A Schottky diode can clamp the base-collector junction of an NPN to prevent saturation (Baker's concept, 1956; Biard proposed Schottky clamp, 1964). The Schottky must have a forward voltage **at least 150 mV less** than the base-collector junction at the same current density.

| Silicide | Antisaturation effectiveness | Leakage | Max operating temp |
|---|---|---|---|
| Molybdenum / PdSi | Excellent (large clamp margin) | Higher | Good at high $T$ |
| PtSi | Moderate (higher $V_F$ reduces margin) | Very low | Begins to saturate above ~100$^\circ$C |

PtSi Schottky transistors fail at high temperature because the Schottky TC ($-1$ mV/$^\circ$C) is less negative than the base-collector junction TC ($-2$ mV/$^\circ$C), so the clamp margin shrinks with increasing temperature.

---

## 11.1.4 Base-Collector Power Diodes

### Structure and Application

The base-collector junction provides a high-voltage, high-current diode with breakdown voltage equal to $BV_{CBO}$ (which exceeds $BV_{CEO}$). Combined with NBL and $N^+$ sinker, this creates a robust **power diode**.

The anode is a single large rectangle of base diffusion. The high-resistivity N-epi beneath it acts as a **drift region** that:
1. **Ballasts the structure** -- prevents hot-spot formation by distributing current uniformly.
2. **Moves power dissipation deep into the silicon**, away from thermally fragile contacts.

### Drift Region Resistance

The series resistance of the drift region (ignoring outdiffusion and fringing):

$$R_{drift} = \frac{\rho_{epi} \left( t_{epi} - x_j - x_{NBL} \right)}{A_{anode}} \tag{11.7}$$

where $\rho_{epi}$ is epi resistivity, $t_{epi}$ is epi thickness, $x_j$ is base junction depth, $x_{NBL}$ is NBL up-diffusion distance, and $A_{anode}$ is the drawn anode area.

For a circular anode, the lateral resistance through the NBL from center to periphery is:

$$R_{lat} = \frac{R_{sheet}}{8\pi} \tag{11.8}$$

where $R_{sheet}$ is the NBL sheet resistance. This is independent of anode radius because the effective current path width increases toward the periphery, exactly compensating for the longer path.

### Layout Rules

- **Anode**: Single large base geometry filled with maximum contact area (single large contact or array of smaller contacts; arrays may be preferred for electromigration reasons to increase contact periphery).
- **Cathode**: NBL floors the drift region; $N^+$ sinker rings it. The $N^+$ ring width should be **at least twice the epi thickness** when conducting > 100 mA, to maximize sinker doping, reduce hole permeation through the guard ring, and minimize vertical resistance. NBL should extend at minimum to the outer edge of the drawn $N^+$ ring.
- **Emitter diffusion** covers the sinker ring to thin field oxide and reduce contact resistance. Should extend inward toward base and outward toward isolation as far as rules permit; at minimum, drawn emitter covers drawn $N^+$.
- **Cathode contacts**: Fill emitter diffusion atop sinker with maximum contact area.

### Advantages vs. Schottky Diodes

| Property | Power Diode | Schottky Diode |
|---|---|---|
| Process layers | Baseline (always available) | Requires noble silicide + extra mask |
| Junction depth | Deep (far below surface) | At surface (under metal) |
| Power handling | Much higher (deep heat dissipation) | Limited (surface dissipation) |
| Switching speed | Slow (large stored charge) | Fast (majority-carrier device) |
| Guard ring leakage at high $I$ | Degrades (high-level injection) | Minimal (majority-carrier) |

### Drawbacks

1. **Slow switching**: The large minority carrier population in the drift region represents stored charge that must be removed to turn off the diode. Unsuitable for high-speed switching.
2. **Guard ring degradation at high current**: High-level injection erodes the built-in potential at the N-epi/NBL interface. As holes flood the collector, electron population increases to maintain charge neutrality, reducing the barrier and allowing more holes through -- the hole-blocking guard ring becomes less effective.

---

## Diagrams

### Figure 11.2 -- Diode-Connected Transistor Layout and Cross Section

![[diagrams/ch11-diodes-standard-bipolar-fig1.png]]

*Page 535 -- Shows the layout of a standard bipolar diode-connected transistor with merged collector-base contact. The cross section illustrates how emitter diffusion overlaps the base diffusion so a single contact can touch both collector (N-epi in tank) and base ($P$-type base diffusion). The CBE configuration simplifies the base-collector short.*

### Figure 11.4 -- Emitter-Base Zener Diode Layout

![[diagrams/ch11-diodes-standard-bipolar-fig2.png]]

*Page 539 -- Layout (B) and schematic (A) of an emitter-base Zener diode. Note the circular/oval emitter geometry to minimize corner field intensification. The emitter serves as cathode, the base as anode. The tank provides isolation and can connect to anode, cathode, or another suitable voltage.*

### Figure 11.10 -- Field-Plated Schottky Diode Layout and Cross Section

![[diagrams/ch11-diodes-standard-bipolar-fig3.png]]

*Page 546 -- Shows the construction of a field-plated Schottky diode in standard bipolar. The metal stack (PtSi / TiW / AlCu) forms the Schottky barrier to lightly doped N-epi. The Schottky contact geometry overlaps the regular contact geometry to create two shallow oxide steps and a field plate over thin oxide. NBL and $N^+$ sinker form the cathode and hole-blocking guard ring.*

---

## Practical Takeaways

- **Default diode choice**: Use a diode-connected NPN for general-purpose diodes (current mirrors, biasing). CBE configuration with merged collector-base contact is the most area-efficient layout.
- **Current handling**: Add a $N^+$ sinker above a few hundred $\mu$A; use a power transistor layout with complete $N^+$ ring above 10 mA.
- **Voltage reference applications**: Emitter-base Zeners are convenient but suffer from walkout (up to 250 mV drift). For stable references, use **buried Zeners** targeted at the 5.0-5.4 V zero-TC point.
- **Never float the tank** of an emitter-base Zener -- the parasitic substrate PNP will bleed anode current, especially at elevated temperatures.
- **Round emitter geometries** for Zeners are a low-cost precaution against corner field intensification, even if the specific process does not require them.
- **Schottky diode material selection**: PtSi is best for low-leakage applications up to ~100$^\circ$C; PdSi is better for high-temperature antisaturation clamps. Neither aluminum nor titanium silicide Schottkies are recommended.
- **Guard rings vs. field plates** for Schottkies: Use guard rings on large devices (predictable breakdown, negligible area overhead) and field plates on small devices (area savings).
- **Power diodes** are the only option for high-voltage, high-current rectification in standard bipolar, but they are slow due to stored charge. Make the $N^+$ sinker ring at least $2 \times t_{epi}$ wide for currents above 100 mA.
- **Electromigration**: For high-current diodes (both power diodes and high-current Schottkies), use arrays of smaller contacts rather than single large contacts to increase total contact periphery.

---

## Relation to the Bigger Picture

This section bridges the transistor layout techniques from Chapters 9-10 with specialized diode applications. Every diode in standard bipolar is built from the same diffusions used for NPN and lateral PNP transistors -- the layout designer must understand transistor parasitics (Chapter 9) to properly construct diodes. The discussion of Zener walkout connects directly to hot-carrier reliability topics from Chapter 5 (surface effects, electrical overstress). The Schottky diode discussion ties back to contact metallurgy from Chapter 2 (silicides, refractory barrier metals, junction spiking). Understanding these standard bipolar diode structures is essential before moving to the CMOS/BiCMOS diode variants covered in [[ch11-diodes-cmos-bicmos]], which face different constraints (no base diffusion, different isolation schemes). The PN junction physics underpinning all these devices is covered in [[ch01-pn-junctions]].

## See Also
- [[ch11-diodes-cmos-bicmos]]
- [[ch01-pn-junctions]]
