---
title: "5.4 Minority Carrier Injection"
chapter: 5
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-5, latchup, guard-rings, minority-carriers, debiasing, failure-mechanisms]
---

# 5.4 Minority Carrier Injection

> **Chapter 5: Failure Mechanisms**

## Key Concepts

### The Fundamental Problem: Parasitic Bipolar Transistors

Every integrated circuit contains **parasitic bipolar transistors** -- devices that are inherent to the construction of the IC but are not required for its operation. These parasitic devices activate whenever minority carriers are unintentionally injected near reverse-biased junctions. The consequences range from mild parametric shifts to catastrophic **latchup**, a persistent circuit malfunction caused by positive feedback between two parasitic bipolar transistors.

The root cause is simple: any PN junction that becomes forward-biased will inject minority carriers into the surrounding silicon. In normal operation, all junctions in an IC are either reverse-biased or zero-biased. But voltage transients on pins, switching events, or even capacitive coupling can momentarily forward-bias a junction, triggering injection.

### How Minority Carrier Injection Occurs

There are two fundamental injection scenarios (Figure 5.27):

1. **Pulling a pin above $V_{DD}$**: Consider a PMOS transistor whose drain connects to a pin and whose backgate (N-well) connects to supply. If the pin voltage exceeds $V_{DD}$, the PSD/N-well junction forward-biases, injecting holes into the N-well. Some holes diffuse across the well to the reverse-biased N-well/substrate junction. Electrically, this activates a **parasitic PNP**: the PSD is the emitter, the N-well is the base, and the P-epi is the collector. Hole current flows into the substrate.

2. **Pulling a pin below $V_{SS}$ (ground)**: Consider an NMOS transistor whose drain connects to a pin. If the pin drops below ground, the NSD/P-epi junction forward-biases, injecting electrons into the P-epi. These electrons diffuse to adjacent reverse-biased junctions (e.g., N-well boundaries). Electrically, this activates a **parasitic NPN**: the NSD is the emitter, the P-epi is the base, and an adjacent N-well is the collector.

### Sources of Unwanted Transients

Even a well-designed application circuit can inadvertently trigger minority carrier injection:

- **Hot-plugging** (e.g., USB cable insertion with residual charge)
- **Power supply sequencing** of multiple rails
- **Lightning or short-circuit transients**
- **Inductive kicks** from solenoids or relays
- **Capacitive coupling** from fast-slewing nodes
- **Internal capacitors** connected to switching circuitry (even sub-pF capacitors have triggered CMOS latchup in logic gates)
- **Schottky diodes** with PN guard rings can inject minority carriers into their cathode at high forward currents

### The Critical Rule for Pin-Connected Diffusions

Any diffusion connected to a pin through less than a critical resistance may trigger latchup. The reasoning:

- PN junctions inside ESD devices typically clamp pin voltages to about $\pm 1\,\text{V}$
- Latchup is usually triggered by negative transients sinking at least $1\,\text{mA}$
- Therefore, a series resistance of $1\,\text{k}\Omega$ usually prevents latchup
- **Conservative threshold**: Many designers use $10\,\text{k}\Omega$ or even $100\,\text{k}\Omega$ because low-current analog circuitry can be more sensitive

---

## 5.4.2 Latchup

### CMOS Latchup: The SCR Mechanism

CMOS latchup involves a positive feedback loop between two parasitic bipolar transistors that are inherent to the N-well CMOS structure (Figure 5.29):

- **Parasitic lateral NPN ($Q_1$)**: The NMOS source is the emitter, the P-epi is the base, and the N-well of the adjacent PMOS is the collector.
- **Parasitic lateral PNP ($Q_2$)**: The PMOS source is the emitter, the N-well is the base, and the P-epi is the collector.
- **$R_W$ (well resistance)**: Distributed resistance across the N-well. Normally biases $Q_2$ in cutoff.
- **$R_S$ (substrate resistance)**: Distributed resistance across the P-epi. Normally biases $Q_1$ in cutoff.

### The Latchup Trigger Sequence

1. A transient injects current into the substrate (node $V_{sub}$).
2. If the voltage drop across $R_S$ becomes large enough, $Q_1$ turns on.
3. The collector current of $Q_1$ flows through $R_W$.
4. If the voltage drop across $R_W$ becomes large enough, $Q_2$ turns on.
5. The collector current of $Q_2$ flows through $R_S$, reinforcing the original disturbance.
6. If the loop gain $\beta_1 \cdot \beta_2 > 1$, the feedback is self-sustaining.

### Three Conditions for Latchup

For latchup to persist, **all three** conditions must be met simultaneously:

1. Both parasitic transistors must be biased into the forward-active (or reverse-active) region.
2. The beta product $\beta_1 \cdot \beta_2 > 1$ over some range of collector currents.
3. The power supply must be able to source sufficient current to maintain conduction.

Once latched, collector currents increase until high-current beta rolloff reduces $\beta_1 \cdot \beta_2$ to unity, reaching an equilibrium state. The circuit remains latched until **power is cycled** (removed and restored).

### Debiasing: The Trigger Mechanism

**Debiasing** occurs whenever a significant voltage drop appears across a parasitic resistance:

- **Well debiasing**: Voltage drop across $R_W$ due to electron current through the N-well.
- **Substrate debiasing**: Voltage drop across $R_S$ due to hole current through the P-epi/substrate.

Latchup is almost always initiated by minority carrier injection, followed by drift of those carriers across a reverse-biased junction, leading to well or substrate debiasing. This understanding yields **four countermeasures**:

1. **Prevent** unwanted minority carrier injection from occurring
2. **Collect** unwanted minority carriers before they reach a reverse-biased junction
3. **Recombine** minority carriers before they reach a reverse-biased junction
4. **Minimize debiasing** by reducing well or substrate resistance

### SCR Latchup in General

CMOS latchup is a specific case of the more general **SCR latchup**. Any IC cross-section containing a PNPN four-layer structure can latch up if the beta product exceeds unity and biasing conditions are met. Examples:

- A lateral PNP merged with a vertical NPN in the same tank (standard bipolar)
- A vertical NPN with a field-plated Schottky diode integrated into its collector

### Latchup Testing

Standard latchup testing (per JESD78A):

1. Insert current meters in series with every power supply pin
2. Record baseline supply currents
3. Apply a current pulse (typically $\pm 100\,\text{mA}$ for $50\,\text{ms}$) to each pin
4. Allow supply currents to settle, then re-measure
5. If any supply current increases by more than 10%, latchup has occurred
6. Test both polarities
7. If latchup is found, **emission microscopy (EMMI)** can pinpoint the exact site by detecting the low-level infrared light emitted during silicon recombination

---

## 5.4.3 Debiasing Analysis

Hastings develops simplified closed-form models for three common distributed resistance structures. The goal is to keep distributed resistances to **no more than a few ohms** to prevent latchup.

### Type 1: Thin Lightly Doped Layer Atop a Heavily Doped Sublayer

Examples: P-epi on $P^+$ substrate; N-epi on $N^+$ buried layer (NBL); N-well joined to NBL; retrograde well.

The vertical resistance $R_V$ between a contact diffusion of area $A_C$ and the sublayer:

$$R_V = \frac{\rho \cdot t_{epi}}{A_C}$$

where $\rho$ is the resistivity of the epi layer and $t_{epi}$ is its thickness. Valid when $\sqrt{A_C} \gg t_{epi}$.

For a disperse array of small circular contacts (radius $r_C$) on a layer of thickness $t_{epi}$:

$$R_V = \frac{\rho}{2 \pi r_C} \cdot \arctan\!\left(\frac{t_{epi}}{r_C}\right)$$

**Key insight**: Disperse arrays of small contacts have significantly less total resistance than a single large contact of equal area. For example, with typical values, small contacts can have only 13% of the resistance of a large contact of equal total area. To get the full benefit, contacts must be spaced apart by at least $2 t_{epi}$.

Assuming a latchup test current of $100\,\text{mA}$ and a maximum allowed debiasing of $0.5\,\text{V}$, resistance must not exceed $5\,\Omega$.

**Layout rules for thin epi on heavy sublayer:**
1. Place small contacts wherever convenient
2. Fill unused area with additional contacts
3. Consider ringing majority-carrier injectors with contacts

### Type 2: Thick Lightly Doped Substrates

The resistance between two circular contact diffusions of radius $r_C$ separated by distance $d$ on an infinitely thick substrate of resistivity $\rho$:

$$R_{12} = \frac{\rho}{\pi r_C}\left[1 - \frac{2}{\pi}\arctan\!\left(\frac{2r_C}{d}\right)\right]$$

The critical difference from Type 1 is that resistance depends **strongly on contact size** but only **weakly on separation**. A larger separation allows current to penetrate more deeply into the substrate, partially compensating for the longer path.

For an annular contact of width $w$ enclosing a circular contact of radius $r_C$ with spacing $s$:

$$R = \frac{\rho}{2\pi r_C}\left[1 - \frac{2}{\pi}\arctan\!\left(\frac{2r_C}{2r_C + 2s + w}\right)\right]$$

**Counterintuitive result**: increasing the annular contact spacing can actually *decrease* resistance because the growing area of the annular contact more than compensates for the longer path.

Equations are valid only for structures smaller than the substrate thickness (typically $200\text{--}300\,\mu\text{m}$ post-backgrind).

**Layout rules for thick lightly doped substrate:**
1. Place substrate contacts within half the die thickness of all devices
2. Make individual substrate contacts as large as space permits
3. Contacts spaced more than half the die thickness away provide relatively little benefit

### Type 3: Thin Isolated Layers

Examples: Tank without buried layer; well without buried layer or retrograde profile; superficial silicon atop buried oxide (SOI); P-well atop $P^+$ substrate.

For two circular contacts of radius $r_C$ separated by distance $d$ on a layer of thickness $t$ and resistivity $\rho$:

- If $t \geq r_C$: use Equation 5.28 (similar form to thick substrate but bounded by layer thickness)
- If $t < r_C$: use Equation 5.29 (sheet-resistance-dominated regime)

**Critical difference from thick substrates**: resistance increases strongly with separation in thin layers due to constriction effects. Distant contacts provide little benefit.

Thin layers greatly enhance **proximity effects**. An annular contact gathers the vast majority of majority carriers injected inside it, as long as its width is at least half the layer thickness. However, annular contacts **cannot prevent debiasing caused by minority carrier injection** -- most minority carriers diffuse far beyond the annular contact before recombining. For example, the average distance an electron travels through $10\,\Omega\text{-cm}$ P-type silicon before recombining is about $200\,\mu\text{m}$.

**Layout rules for thin layers:**
1. Place annular contacts around all structures that inject majority carriers
2. Place these annular contacts as close to the injecting structures as possible
3. If possible, make annular contact width equal to half the layer thickness
4. Scatter additional small contacts everywhere, separating them by no more than 20--50 times the layer thickness

### Standard Bipolar Substrate Contacts

In standard bipolar, the isolation diffusion (typical $R_S \approx 30\,\Omega/\square$) forms a network of narrow strips separating larger tanks. The vertical resistance of the isolation diffusion is about $500\,\Omega$, so the isolation network acts as a reasonably efficient contact to the underlying substrate.

**Layout rules for standard bipolar:**
1. Ring majority-carrier injectors with as much substrate contact as possible
2. Minimize gaps in the substrate contact metallization around injectors
3. Scatter substrate contacts throughout the die as space and wiring permit

---

## 5.4.4 Guard Rings

Minority carriers can travel **hundreds or even thousands of microns** before recombining. Any reverse-biased junction within this distance can collect some of these carriers, causing parametric shifts or triggering latchup. Substrate/well contacts alone cannot stop minority carriers -- they only prevent debiasing from the majority carriers flowing in to support recombination. Instead, specialized **minority-carrier guard rings** are needed.

Guard rings fall into four categories:

| Type | Abbreviation | Function |
|------|-------------|----------|
| Electron-Collecting Guard Ring | ECGR | Collects minority electrons via reverse-biased junction |
| Electron-Blocking Guard Ring | EBGR | Blocks electron flow via $N^+/N^-$ high-low junction |
| Hole-Collecting Guard Ring | HCGR | Collects minority holes via reverse-biased junction |
| Hole-Blocking Guard Ring | HBGR | Blocks hole flow via $N^+/N^-$ high-low junction |

### High-Low Junctions: The Physics of Blocking Guard Rings

A **high-low junction** is a layer of heavily doped silicon in contact with a layer of lightly doped silicon of the same polarity. Consider an $N^+/N^-$ junction:

1. The $N^+$ region has a higher electron concentration, so electrons diffuse from $N^+$ to $N^-$.
2. Charge separation generates a built-in electric field.
3. Equilibrium is reached when drift exactly opposes diffusion.
4. The resulting built-in potential biases $N^+$ positive relative to $N^-$.
5. This potential **opposes the flow of holes** from the $N^-$ region into the $N^+$ region.
6. Therefore, the $N^+/N^-$ junction acts as a **hole-blocking guard ring**.

The effectiveness is quantified by the **permeation ratio** $P$:

$$P = \frac{I_{out}}{I_{in}} = \frac{A_J \cdot D_H}{V_L / \tau_L} \cdot \frac{1}{W_H} \cdot \frac{N_L}{N_H}$$

where:
- $A_J$ = area of the high-low junction
- $D_H$ = diffusion coefficient of minority carriers in the heavily doped region
- $\tau_L$ = lifetime of minority carriers in the lightly doped region
- $W_H$ = width of the heavily doped region
- $V_L$ = volume of the lightly doped region
- $N_H$ = doping concentration of the heavily doped region
- $N_L$ = doping concentration of the lightly doped region

A high-low junction is effective when $N_H / N_L > 100$. For a typical NBL flooring a $100\,\mu\text{m} \times 100\,\mu\text{m}$ tank with $5\,\mu\text{m}$ deep epi doped to $4 \times 10^{15}\,\text{cm}^{-3}$ and NBL doped to $1 \times 10^{19}\,\text{cm}^{-3}$, the permeation ratio $P \approx 0.002$ (only 0.2% of injected current escapes).

**Warning**: Some modern low-voltage BiCMOS processes have such high well concentrations (surface $N_D > 1 \times 10^{17}\,\text{cm}^{-3}$) that their $N^+$ sinkers no longer form effective high-low junctions.

### Electron-Collecting Guard Rings (ECGR)

An ECGR is an N-type region that collects minority electrons diffusing through adjacent P-type material (P-epi or P-well). Construction:

- **Standard bipolar**: Include all possible N-type layers (emitter, deep-$N^+$, NBL) to maximize doping and junction depth. Only marginally effective because electrons can flow underneath through the lightly doped substrate.
- **Analog BiCMOS**: Similar construction but with a $P^+$ substrate that constrains electrons to the P-epi, greatly improving collection efficiency.

**Collection efficiency**: ECGRs with $P^+$ substrate can reduce electron currents by a factor of 10--100.

An **improved ECGR** (van Zanten, 1984) connects the guard ring back to the substrate so that majority-carrier current flows beneath it. This current generates an opposing electric field that increases collection efficiency, with claimed attenuation factors exceeding $10^6$. However, it only protects in one direction.

**Practical considerations**:
- A grounded ECGR using low-resistance layers (emitter, NMoat, deep-$N^+$, NBL) with continuous metal connection to ground will typically protect against 100--200 mA latchup test currents
- Gaps in metallization increase resistance and risk reinjection
- A guard ring connected to a power supply can tolerate more debiasing, but the supply must source sufficient current
- Metal leads connecting to guard rings must handle transient currents of tens to hundreds of milliamps
- Guard ring segments along the die edge provide little benefit; use L- or U-shaped rings that terminate near the die edge

### Electron-Blocking Guard Rings (EBGR)

An EBGR would use $P^+/P^-$ high-low junctions surrounding an electron injector. Construction requires a P-buried layer (PBL) and a sufficiently heavily doped $P^+$ sinker reaching through a $P^-$ epi to a $P^+$ substrate. Few processes include both, so designers typically rely on ECGRs instead.

### Hole-Collecting Guard Rings (HCGR)

An HCGR is a P-type region that collects minority holes diffusing through an adjacent N-type region (N-well or N-tank). Construction in standard bipolar:

- An annular ring of P-isolation inside an N-tank containing NBL
- The P-iso bottom terminates atop the NBL (not sufficiently doped to invert it)
- The NBL blocks vertical hole escape; holes diffuse laterally to the P-iso ring
- The outer N-tank and NBL isolate the P-iso ring from substrate

**Connection options**: Connect to same potential as N-tank (risk of debiasing due to high base diffusion resistance), or connect to ground (prevents debiasing but limits tank voltage to P-iso/NBL breakdown, and increases power dissipation).

#### P-Bars: A Specialized HCGR

A **P-bar** is a strip of base diffusion placed between two devices in the same tank to prevent **cross-injection**. Each end extends into the isolation to guarantee electrical contact without requiring explicit contacts.

The P-bar works because the base diffusion reaches deeply into the epi, leaving little room for carriers to pass beneath. Most injected holes are collected by the P-bar and diverted to ground. The NBL beneath provides a low-impedance path for base current.

**Applications of P-bars**:
- Between lateral PNP transistors in a current mirror sharing a common tank (prevents saturation-induced cross-injection)
- Between an NPN and a PNP where the NPN collector is merged with the PNP base (suppresses SCR latchup)

**In retrograde wells**: PSD can be used for hole-collecting guard rings if the retrograde profile creates a sufficiently strong high-low junction at the well bottom. Even minimum-width PSD guard rings improve latchup immunity.

### Hole-Blocking Guard Rings (HBGR)

An HBGR uses $N^+/N^-$ high-low junctions to imprison holes. In standard bipolar, constructed using deep-$N^+$ sinker and NBL:

- The drawn NBL should extend to the outside edge of the drawn deep-$N^+$ to maximize doping at their junction
- The deep-$N^+$ ring must **completely enclose** the injector -- any gap opens a pathway for holes to escape
- Also functions as a low-resistance tank contact (useful for power bipolar transistors)

**In standard bipolar, HBGRs are much more effective at preventing latchup than ECGRs.** When space is limited, preference should be given to hole-blocking guard rings.

**Analog BiCMOS HBGRs**: Can use deep-$N^+$ and NBL, but effectiveness depends on the doping ratio. Older processes with deep, lightly doped N-wells typically work well. Newer low-voltage processes with shallow, heavily doped wells may not meet the $N_H/N_L > 100$ criterion. Processes with deep trench isolation can substitute trenches for deep-$N^+$ sinkers, saving area.

---

## Diagrams

### Figure 5.27 -- Sources of Minority Carrier Injection

![[diagrams/ch05-minority-carrier-injection-fig2.png]]

**Cross sections showing two fundamental injection scenarios.** (A) Pulling a pin connected to a P-type region (PSD/PMOS drain) above $V_{DD}$ forward-biases the PSD/N-well junction, activating a parasitic PNP that injects holes into the substrate. (B) Pulling a pin connected to an N-type region (NSD/NMOS drain) below ground forward-biases the NSD/P-epi junction, activating a parasitic NPN that injects electrons into the P-epi toward adjacent N-wells. These are the two root causes of minority carrier injection in CMOS.

### Figure 5.28 and Section 5.4.2 -- Latchup Mechanism

![[diagrams/ch05-minority-carrier-injection-fig1.png]]

**Capacitor-induced injection and the CMOS latchup structure.** Top: Figure 5.28 shows how an integrated capacitor in a timing circuit can couple a voltage step that momentarily pulls a node below ground, triggering injection. Bottom: Section 5.4.2 introduces the four-layer PNPN SCR structure formed by two parasitic bipolars ($Q_1$ NPN, $Q_2$ PNP) with well resistance $R_W$ and substrate resistance $R_S$ forming the positive feedback loop.

### Figure 5.33 -- Electron-Collecting Guard Rings

![[diagrams/ch05-minority-carrier-injection-fig3.png]]

**Cross sections of electron-collecting guard rings.** (A) Standard bipolar ECGR using all available N-type layers (emitter, deep-$N^+$, BOI). Only marginally effective because electrons can circumvent it through the lightly doped substrate. (B) Analog BiCMOS ECGR with a $P^+$ substrate that constrains electrons to the P-epi, dramatically improving collection efficiency. The $P^+$ substrate acts as a reflecting boundary.

---

## Practical Takeaways

### Minority Carrier Injection Prevention
- **Any diffusion connected to a pin through less than $1\,\text{k}\Omega$** (conservative: $10\,\text{k}\Omega$ to $100\,\text{k}\Omega$) may trigger latchup
- Examine all pin-connected diffusions and internal switching nodes for injection risk
- Even sub-pF capacitors coupling to switching nodes can trigger latchup in adjacent logic gates

### Debiasing Mitigation
- **Target less than $5\,\Omega$ distributed resistance** in substrate and wells (assuming $100\,\text{mA}$ latchup test current, $0.5\,\text{V}$ max debiasing)
- **Thin epi on heavy sublayer**: Use disperse arrays of small contacts (13% the resistance of one large contact of equal area); fill unused area with contacts; space contacts at least $2 t_{epi}$ apart
- **Thick lightly doped substrate**: Make contacts as large as possible; contacts within half the die thickness are effective; more distant contacts provide diminishing returns
- **Thin isolated layers**: Ring injectors with annular contacts (width $\geq t_{layer}/2$); annular contacts cannot stop minority carriers, only majority carriers; scatter contacts every 20--50 layer thicknesses

### Guard Ring Selection
- **Hole-blocking guard rings (HBGR) are the most effective** countermeasure in standard bipolar -- prefer them when space is limited
- ECGRs with $P^+$ substrate provide 10--100x attenuation; without $P^+$ substrate, only marginal
- Guard ring metal must be continuous (no gaps) and wide enough for transient currents
- Deep-$N^+$ rings must completely enclose the injector -- any gap defeats the guard ring
- P-bars between devices in a shared tank prevent cross-injection at minimal area cost
- NBL drawn edge should extend to the outside edge of deep-$N^+$ drawn edge for maximum doping at the junction

### Latchup Testing
- Standard test: $\pm 100\,\text{mA}$, $50\,\text{ms}$ pulses per JESD78A
- Both current polarities must be tested on every pin
- Use emission microscopy (EMMI) to pinpoint latchup sites if failures occur
- Power cycling is the only way to clear a latched state

---

## Relation to the Bigger Picture

Section 5.4 is arguably the most practically important part of Chapter 5 for layout designers. While other failure mechanisms in this chapter (electromigration, dielectric breakdown, [[ch05-surface-effects|surface effects]]) primarily affect long-term reliability and can often be addressed by process engineers, **latchup is a layout-dependent failure** that can cause immediate, catastrophic malfunction and is directly under the layout designer's control. The concepts of debiasing, guard ring placement, and substrate/well contact strategy introduced here form the foundation for robust analog layout practice and recur throughout the device-specific chapters (Chapters 9--13). Understanding these principles is essential before tackling the matching and isolation strategies in Chapters 7--8, where guard rings and well contacts appear as standard elements of every precision analog layout.

## See Also
- [[ch05-surface-effects]]
