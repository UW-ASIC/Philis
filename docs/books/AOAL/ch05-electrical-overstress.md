---
title: "5.1 Electrical Overstress"
chapter: 5
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-5, EOS, self-heating, filamentation, electromigration, TDDB, ESD, antenna-effect]
---

# 5.1 Electrical Overstress

> **Chapter 5: Failure Mechanisms**

## Key Concepts

Electrical overstress (EOS) encompasses any situation that applies excessive voltages or currents to an electrical device. A properly designed IC should not experience EOS when operated within its datasheet ratings, but layout plays a significant role in ensuring this is true.

Section 5.1 covers four fundamental EOS mechanisms that depend (at least in part) upon layout:

1. **Self-heating** -- power dissipation raises junction temperatures to destructive levels
2. **Filamentation** -- current localizes into a narrow destructive filament
3. **Electromigration** -- electron wind physically displaces metal atoms, causing opens and shorts
4. **Time-dependent dielectric breakdown (TDDB)** -- gate oxides degrade over time under electric field stress

These four mechanisms are then used to explain two additional phenomena:

5. **Electrostatic discharge (ESD)** -- transient high-voltage/high-current events from handling
6. **The antenna effect** -- plasma processing deposits charge that damages gate oxides

The central insight is that these are not purely circuit-level problems: they are layout-level problems. The geometry, width, and routing of metal leads; the size and placement of power devices; the area ratios of poly to gate oxide -- all of these layout decisions determine whether a chip will survive years of operation or fail prematurely.

### The Arrhenius Relationship

Many failure mechanisms obey the Arrhenius relationship, which governs temperature-dependent failure rates:

$$R = R_0 \, e^{-E_a / kT}$$

where $R$ is the failure rate, $R_0$ is the rate constant, $E_a$ is the activation energy, $k$ is Boltzmann's constant ($8.62 \times 10^{-5}$ eV/K), and $T$ is the absolute temperature in kelvin.

Expressed as median time to failure ($t_{50}$):

$$t_{50} = A \, e^{E_a / kT}$$

To predict how temperature accelerates a failure mechanism, given known failure time $t_1$ at temperature $T_1$:

$$t_2 = t_1 \, \exp\!\left[\frac{E_a}{k}\left(\frac{1}{T_2} - \frac{1}{T_1}\right)\right]$$

Most failure mechanisms have activation energies between 0.5 and 1.0 eV.

---

## 5.1.1 Self-Heating

### Temperature Limits and Thermal Resistance

Power dissipation inside an IC raises its temperature above ambient. The junction temperature is governed by:

$$T_J = T_A + P \cdot \theta_{JA}$$

where $T_J$ is the junction temperature, $T_A$ is the ambient temperature, $P$ is the power dissipation, and $\theta_{JA}$ is the thermal resistance from junction to ambient. Small surface-mount packages without heatsinking have $\theta_{JA}$ values exceeding 100 degC/W, limiting dissipation to a fraction of a watt.

For power packages with external heatsinks:

$$T_J = T_C + P \cdot \theta_{JC}$$

where $T_C$ is the case temperature and $\theta_{JC}$ (junction-to-case thermal resistance) remains relatively constant regardless of board layout. Power packages typically have $\theta_{JC} < 5$ degC/W.

### Why Temperature Matters

- Junction leakages double approximately every 10 degC.
- A 1 nA leakage at 25 degC becomes significant at around 100 degC and causes functional failures beyond about 150 degC.
- When silicon goes **intrinsic** (thermally generated carriers outnumber doped carriers), uncontrolled current flow destroys the device. The intrinsic temperature varies with doping: ~300 degC for $10^{16}$ cm$^{-3}$, ~450 degC for $10^{18}$ cm$^{-3}$.
- Heating an operating IC to 300 degC will probably destroy it.
- Most power ICs include **thermal protection** circuits that disable the device ~25 degC above the rated maximum junction temperature.

### Self-Heating of On-Die Devices

For a rectangular power device with dimensions much smaller than die thickness (~300 um), the temperature rise at the center is:

$$\Delta T = \frac{P}{2\pi k_s} \cdot \frac{1}{\sqrt{A_d/\pi}}$$

where $k_s$ is the thermal conductivity of silicon (~1.3 W/cm-degC) and $A_d$ is the device area. A 100 um x 100 um device dissipating 100 mW sees a rise of ~14 degC.

### Self-Heating of Deposited Resistors

Deposited resistors (metal, poly, thin-film) sit atop oxide, which is a poor thermal conductor. The temperature rise for a current $I$ through a resistor of width $W$ and sheet resistance $R_s$ is:

$$\Delta T = \frac{I^2 R_s \, t_{ox}}{W^2 \, k_{ox}}$$

where $t_{ox}$ is the oxide thickness beneath the resistor and $k_{ox}$ is the thermal conductivity of oxide (~0.014 W/cm-degC).

The **minimum width** to carry a maximum continuous current $I_{max}$:

$$W_{min} = I_{max} \sqrt{\frac{R_s \, t_{ox}}{\Delta T_{max} \, k_{ox}}}$$

Designers typically use $\Delta T_{max} \approx 30$ degC to avoid thermal gradients affecting nearby electromigration calculations. A maximum of 100 degC is recommended to minimize thermally induced mechanical stresses.

### Adiabatic Conditions (Short Pulses < ~1 us)

For pulses too short for heat to escape the resistor:

$$W_{min} = I_{max} \sqrt{\frac{R_s \, t_{pulse}}{\Delta T_{max} \cdot d \cdot \rho \cdot c_p}}$$

where $d$ is resistor thickness, $\rho$ is density, and $c_p$ is volumetric specific heat. Polysilicon has $c_p \approx 1.7$ J/cm$^3$-degC; aluminum has $c_p \approx 2.4$ J/cm$^3$-degC.

### Bondwire Current Limits

Bondwires dissipate heat into the surrounding mold compound (a poor thermal conductor) and conduct heat to the die and leadframe. Copper wires carry more current than gold (lower resistivity, higher thermal conductivity). Short wires carry more than long ones due to heat sinking at the ends. Typical limits for 1 mil gold wire: 1.8 A (1 mm) down to 1.0 A (10 mm); for 1 mil copper: 2.2 A (1 mm) down to 1.1 A (10 mm).

---

## 5.1.2 Filamentation

Thermal destruction of an IC usually involves melting. SiO$_2$ melts at ~1700 degC and silicon at ~1415 degC, but aluminum-silicon contacts fail at ~577 degC due to **eutectic** formation. Only a thin filament bridging two conductors needs to liquefy to destroy a die.

### Three Filamentation Mechanisms

**1. Intrinsic conduction:** When doped silicon exceeds its critical temperature (~300+ degC), thermally generated carriers outnumber dopant carriers. Resistance drops exponentially, current localizes, positive feedback drives the filament to destruction in tens of microseconds. However, some other mechanism must first raise the temperature to the critical point.

**2. Thermal runaway (bipolar transistors):** The $V_{BE}$ temperature coefficient is about $-2$ mV/degC. A hotter region has lower $V_{BE}$, conducts more current, heats further -- forming a **hot spot**. Beta rolloff from high-level injection can stabilize the hot spot, but if temperature or current becomes too great, intrinsic conduction takes over. Timescale: tens to hundreds of microseconds. The **Spirito effect** is the analogous mechanism in MOS transistors, driven by the negative temperature coefficient of $V_{th}$.

**3. Electrical filamentation (avalanche runaway):** In a lightly doped region (e.g., N$^-$) between two N$^+$ diffusions carrying increasing current:
- Low current: uniform drift (Ohmic regime)
- Higher current: velocity saturation occurs; extra electrons flow from the cathode, creating space charge that increases the electric field linearly across the region
- Still higher current: the field at the anode reaches the critical value for **avalanche multiplication**; holes flow back, neutralize space charge, resistance drops suddenly
- Current localizes and triggers filamentation in **less than a nanosecond** -- fast enough to occur even during ESD events

Thermal vs. electrical filamentation can be distinguished by subjecting a device to very short high-current pulses: nanosecond failure indicates avalanche runaway; microsecond failure indicates thermal runaway. Both are lumped under the term **secondary breakdown**.

### Prevention: Ballasting

Inserting **series resistance** (ballast resistance) in the current path creates negative feedback that counteracts the positive feedback causing current localization. This is the primary layout technique for preventing filamentation in both bipolar (Section 9.1.3) and MOS (Section 13.2.1) power devices.

**Zener zap:** Filamentation can be deliberately exploited. A Zener diode can be intentionally filamented to create a permanent low-resistance path, forming a nonvolatile memory element (Section 6.6.2).

---

## 5.1.3 Electromigration

When current densities exceed roughly $10^5$ A/cm$^2$ in aluminum leads, the **electron wind** -- momentum transfer from drifting electrons to metal atoms -- physically displaces atoms. This causes **voids** (opens) and **hillocks/dendrites** (potential shorts).

### Key Physics

- Electromigration in aluminum occurs primarily along **grain boundaries**.
- **Blech effect:** mechanical stress opposes metal atom flow; if a lead is short enough, electromigration halts. The Blech length $\approx 50/J$ um (for $J$ in mA/um$^2$). At a typical maximum working current density, Blech length $\approx 50$ um.
- **Nucleation-dominated failure** (aluminum without barrier metal): voids nucleate where three grains meet, then cascade.
- **Growth-dominated failure** (aluminum with refractory barrier metal, or copper): void growth dominates over nucleation.

### Black's Law

$$t_{50} = A \cdot J^{-n} \cdot e^{E_a / kT}$$

where $J$ is current density, $n$ is the current exponent ($n = 2$ for nucleation-dominated/aluminum, $n = 1$ for growth-dominated/copper), $E_a$ is activation energy, and $A$ is a proportionality constant.

- **Aluminum** $E_a \approx 0.6$ eV
- **Copper** $E_a \approx 0.9$ eV
- **High-tin solder** $E_a \approx 0.5$ eV (very prone to electromigration)

### Process Improvements

1. **Copper doping of aluminum** (0.5-4%, typically 0.5%): improves lifetime by >10x; copper accumulates at grain boundaries.
2. **Compressively stressed protective overcoats:** confine metal under pressure, inhibit void formation.
3. **Bamboo effect:** in very narrow leads (width < grain diameter, ~1 um for Al), grains span the full width, and grain boundaries are perpendicular to current flow -- greatly reducing electromigration susceptibility.
4. **Copper metallization:** can conduct $\geq 5\times$ more current than aluminum for a given lifetime at 105 degC. However, copper's advantage diminishes at elevated temperatures (above ~250 degC, aluminum may be superior) because copper exhibits growth-dominated failure ($n = 1$) vs. aluminum's nucleation-dominated ($n = 2$).

### Electromigration Design Rules

Process designers specify maximum current density at a reference temperature and activation energy. Given current density $J_1$ at temperature $T_1$, the allowed density $J_2$ at temperature $T_2$:

$$J_2 = J_1 \cdot \exp\!\left[\frac{E_a}{n \, k}\left(\frac{1}{T_2} - \frac{1}{T_1}\right)\right]$$

The minimum lead width for a constant current $I$:

$$W_{min} = \frac{I}{J_{max} \cdot t_{metal}}$$

where $t_{metal}$ is the thickness of the conducting metal only (exclude barrier metals).

### Contacts and Vias

- **Aluminum contacts/vias:** step coverage thins metal on sidewalls; only sidewalls facing the direction of current flow conduct significant current.
- **Tungsten-plug vias:** electromigration behavior is **asymmetric** -- voiding occurs faster where conventional current flows into a tungsten plug than where it flows out, because tungsten is far more resistant to electromigration than aluminum.
- Refractory barrier metal (RBM) beneath vias provides a backup current path if aluminum voids.

### AC and Pulsed Currents

- Pure AC at >10 kHz: effectively **no electromigration** (stress relief from bidirectional wind).
- Pulsed unidirectional current with duty cycle $d$ and frequency $f$:

$$t_{50,\text{pulsed}} = t_{50,\text{DC}} \cdot \frac{1}{d^n}$$

(assuming $1/f \ll \tau_{\text{vacancy}}$, typically >0.1 ms). For aluminum ($n = 2$) at 50% duty cycle, lifetime increases 4x and the lead can be made half as wide.

---

## 5.1.4 Time-Dependent Dielectric Breakdown (TDDB)

A gate oxide does not fail the instant a voltage is applied; instead, it degrades progressively. The charge to breakdown $Q_{bd}$ scales linearly with gate area. Even at voltages below the dielectric strength (~10 MV/cm for gate oxide), leakage eventually appears, increases exponentially, and culminates in a **pinhole** short.

### Tunneling Mechanisms

Three forms of tunneling underlie TDDB:

1. **Direct electron tunneling:** electron wave function penetrates through very thin oxide (tens of Angstroms) without localizing inside it. Significant in advanced CMOS ($\leq 20$ A oxides), but does **not** damage oxide because no wave function collapse occurs within it.

2. **Trap-assisted tunneling:** electron tunnels to a trap inside the oxide, then tunnels again to the far side. Effective over distances of ~50 A. If enough traps align, a **percolation path** forms across the oxide.

3. **Fowler-Nordheim tunneling:** under intense electric field, an electron entering the oxide gains enough energy for its wave function to collapse inside, then drifts through the remaining distance. High-energy drifting electrons cause **avalanche generation** of electron-hole pairs, and these holes create new traps via the anode hole injection (AHI) mechanism.

### TDDB Models

**AHI (1/E) model:**

$$t_{50} = t_0 \, \exp\!\left(\frac{G}{E_{ox}}\right)$$

where $t_0$ is the reference lifetime, $G$ is the field acceleration factor (~350 MV/cm), and $E_{ox}$ is the oxide electric field.

**McPherson (E) model (thermochemical model):**

$$t_{50} = t_0 \, \exp(-\gamma \, E_{ox})$$

where $\gamma$ is the field acceleration factor (~3-4 cm/MV at operating temperatures), varying inversely with absolute temperature.

Both models can fit experimental data; the distinction is especially difficult for thick oxides under moderate stress due to the extremely long test times required.

### Weibull Area Scaling

Cumulative dielectric failures follow a **Weibull distribution**. If an oxide area $A_1$ has lifetime $t_1$, then area $A_2$ has lifetime:

$$t_2 = t_1 \left(\frac{A_1}{A_2}\right)^{1/\beta}$$

where $\beta$ is the Weibull slope (often $\approx 2$). Larger oxides fail sooner because they have a higher probability of containing a critical defect.

Area scaling of maximum allowed electric field (AHI model):

$$E_2 = E_1 + \frac{1}{G \beta} \ln\!\left(\frac{A_1}{A_2}\right)$$

### Preventative Measures

- **Maximum continuous oxide stress:** ~5 MV/cm for dry oxides $\geq 100$ A; ~7 MV/cm for thinner oxides. Thicker dielectrics are somewhat more fragile.
- **Gate oxide integrity (GOI):** heavily doped $N^+$ regions getter heavy metal impurities that would otherwise weaken oxides. Layout rules may mandate $N^+$ or NBL within ~200 um of MOS gates to improve GOI.
- **Overvoltage stress testing (OVST):** applies ~2x max operating voltage for ~100 ms to detect latent defects before shipping.
- **Cone defects in STI:** foreign particles in STI trenches create conical silicon protrusions that greatly reduce oxide integrity. Poly over STI oxide is more vulnerable than metal (which has additional MLO above). LOCOS field oxides are immune to cone defects.
- **Never use grown oxide above $N^+$** as a capacitor dielectric -- lattice strain and heavy metal segregation reduce integrity.

---

## 5.1.5 Electrostatic Discharge (ESD)

Static electricity from handling can charge the human body to 10 kV or more. Even 1 kV -- below the threshold of perception -- can destroy an unprotected IC.

### ESD Test Models

| Model | Circuit | Typical Rating | Key Parameters |
|-------|---------|---------------|----------------|
| **Human Body Model (HBM)** | 150 pF through 1.5 kOhm | 1-2 kV | Peak current ~1.3 A at 2 kV |
| **Machine Model (MM)** | 200 pF, ~750 nH, no R | 200 V | Now considered redundant |
| **Charged Device Model (CDM)** | Device charged on FR4, probed | 250-500 V | Models handling machinery |
| **Human Metal Model (HMM)** | Similar to IEC-61000-4-2; 150 pF through 330 Ohm | 8 kV | Models metal tool in hand |

The industry has trended toward **lower required ESD ratings** (HBM reduced from 2 kV to 1 kV in 2010; CDM proposed reduction from 500 V to 250 V) because modern ESD handling practices are improved, and advanced CMOS makes protection increasingly difficult.

### ESD Failure Mechanisms

- **Oxide rupture:** transient voltages rupture thin gate dielectrics, or more dangerously, create "walking wounded" -- devices that pass initial testing but fail after hundreds or thousands of hours.
- **Electrical filamentation:** silicided CMOS source/drain regions are especially vulnerable because silicide cladding eliminates the ballast resistance that would otherwise protect the junction.
- **Filament-contact interaction:** if a molten filament touches a contact, electron wind drives molten metal through it, creating a permanent short. Even without contact, recrystallized silicon in the filament zone creates trap-filled regions that cause junction leakage.
- **Resistor burnout:** ESD energy is deposited adiabatically; thin-film resistors are most fragile (smallest volume), then poly resistors. The most robust resistors are deep, lightly doped diffusions (wells).

### ESD Protection Strategy

- Connect one ESD device from each pin to a **common reference node** (usually substrate). An ESD strike between any two pins passes through at most two devices.
- The reference node metallization forms a **ring encircling the die perimeter**.
- Connections to ESD devices must carry ~1.3 A (for 2 kV HBM) with no more than ~2 Ohm total resistance.
- ESD devices are placed **adjacent to the bondpads** they protect.
- Power transistors may be self-protecting if sufficiently large.
- Multiple bondpads for the same pin need separate ESD devices unless connected by wide on-die metal (bondwire inductance prevents one device from protecting another during CDM events).
- Subsections with separate reference nodes require additional ESD devices between those nodes.

---

## 5.1.6 The Antenna Effect

Plasma processing (dry etching and ashing) deposits charge on exposed conductors. These conductors inject charge through thin gate dielectrics, causing either immediate damage, TDDB degradation, or $V_{th}$ shifts by increasing fixed oxide charge. This is formally called **plasma process-induced damage (PPID)**.

### Antenna Ratios

**During etching** (periphery-dependent): as a conductor sheet separates into individual geometries, each geometry absorbs charge around its periphery. The **peripheral antenna ratio** equals the geometry's perimeter divided by the gate oxide area beneath it. Typical maximum for poly: ~200.

**During ashing** (area-dependent): charge deposits on the entire surface of each geometry. The **areal antenna ratio** equals the geometry's surface area divided by the gate oxide area beneath it. Typical maximum for poly: ~500.

For metal layers, one must evaluate antenna ratios on a **node basis** (all electrically connected geometries), not geometry-by-geometry, because metal geometries may be connected through lower layers.

### The Latent Antenna Effect

During etching, geometries separated by minimum spacings clear later than those with wider spacings. A group of adjacent geometries at minimum spacing remains connected after separating from surrounding geometries -- this is the **latent antenna effect** and can be modeled using oversize-undersize operations.

### Preventative Measures

**1. Metal jumpers for poly antenna violations:** Break a long poly lead with a short metal jumper so that the large poly area does not connect to the small gate oxide area during poly etch.

**2. Protective junctions for metal antenna violations:**
- NSD/substrate junction clamps negative excursions (forward bias) and moderate positive excursions (avalanche, if operating voltage < ~150% of avalanche voltage).
- PSD/N-well junction clamps positive excursions (forward biases into N-well; the N-well/substrate junction then leaks via UV-generated photocurrent from the plasma).
- Both junctions together provide bidirectional protection.
- For this to work, the N-well must be large enough and not shadowed by metal/poly that blocks UV.

**3. Antenna diodes:** Special NSD or PSD structures placed at nodes that violate antenna rules:
- **NSD antenna diode:** NMoat region above substrate -- clamps negative voltages.
- **PSD antenna diode:** PMoat inside floating N-well -- requires UV access to N-well/substrate junction; no metal or poly should route over it or within a few microns. Dummy metal/poly block layers must be drawn over and beyond it.

**4. Higher-metal jumpers:** Antenna violations in lower metal layers can be eliminated by jumpering through a higher metal layer. The topmost metal layer cannot be jumpered -- it requires protective junctions.

---

## Diagrams

### Figure 5.1 -- Avalanche Runaway in an N$^-$ Resistor

![[diagrams/ch05-electrical-overstress-fig1.png]]

Illustrates the progression from low-field Ohmic conduction through velocity saturation and space-charge buildup to avalanche injection and filamentation. Panels (A) through (E) show increasing current density: (A) uniform drift, (B) higher drift velocity, (C) velocity saturation with space charge, (D) avalanche injection at the anode, (E) current localization and filamentation.

### Figure 5.4 -- Tunneling Mechanisms in Gate Oxide

![[diagrams/ch05-electrical-overstress-fig2.png]]

Shows the three tunneling mechanisms responsible for TDDB: (A) direct electron tunneling where the wave function crosses the oxide without localizing, (B) trap-assisted tunneling where the electron localizes at a mid-oxide trap before tunneling to the far side, and (C) Fowler-Nordheim tunneling where a strong electric field allows the electron to gain sufficient energy for its wave function to collapse inside the oxide.

### Figure 5.8 -- Antenna Effect Fix with Metal Jumper

![[diagrams/ch05-electrical-overstress-fig3.png]]

Demonstrates how a long poly lead crossing a minimum-size transistor (A) creates an excessive antenna ratio. Inserting a short metal jumper near the transistor (B) breaks the poly into two electrically separate geometries during poly etch: a small one over gate oxide with an acceptable antenna ratio, and a large one that does not overlie gate oxide and is therefore not a vulnerability.

---

## Practical Takeaways

### Self-Heating
- Compute temperature rise for every power device and deposited resistor using the equations above.
- Use $\Delta T_{max} \leq 30$ degC for resistors near electromigration-sensitive metal; $\leq 100$ degC absolute maximum.
- Account for adiabatic conditions for pulses shorter than ~1 us.
- Verify bondwire current ratings against the bondwire length and material (copper > gold).

### Filamentation
- **Ballast all power devices** with series resistance in each finger/segment to prevent current localization.
- Bipolar transistors: emitter ballast resistors (Section 9.1.3).
- MOS transistors: source ballast resistors or distributed layouts (Section 13.2.1).
- Remember that silicided source/drain removes natural ballast -- layout may need to add it back.

### Electromigration
- Obey the process-specified maximum current density rules for every metal layer, contact, and via.
- Use Black's law to derate current density for elevated operating temperatures.
- Account for step coverage thinning at contact/via sidewalls when computing effective metal thickness (unless refractory barrier metal is present).
- For pulsed unidirectional current at >50% duty cycle, leads can potentially be narrower.
- Pure AC at >10 kHz has negligible electromigration concern.
- Watch for asymmetric via behavior in tungsten-plug systems.
- Place multiple vias in parallel for high-current paths.

### TDDB
- Never exceed ~5 MV/cm continuous stress on dry oxides (7 MV/cm for thin oxides).
- Ensure $N^+$ or NBL is placed within ~200 um of all MOS gate oxides for heavy-metal gettering.
- Avoid growing capacitor oxide over $N^+$ diffusions.
- Be cautious with poly over STI field oxide (cone defects); LOCOS field oxide is safer.
- Use OVST testing to screen for latent GOI defects.
- Larger oxide areas fail sooner (Weibull scaling) -- account for area when evaluating reliability.

### ESD
- Every bondpad needs an ESD protection device unless it connects to a self-protecting power device.
- Route ESD current paths with very low resistance ($\leq 2$ Ohm between any two pads).
- Use a continuous substrate ring as the ESD reference bus.
- Multiple bondpads on the same net still need individual ESD devices unless connected by wide on-die metal.
- Consider that advanced CMOS with silicided junctions is especially vulnerable -- plan protection early.

### Antenna Effect
- Run antenna rule checks (DRC) on every design.
- Fix poly antenna violations with metal jumpers that break poly near the gate.
- Fix lower-metal antenna violations with higher-metal jumpers.
- Fix topmost-metal violations with antenna diodes (NSD for negative clamp, PSD/N-well for positive clamp).
- Ensure PSD antenna diodes have UV access (no metal/poly routing overhead; draw dummy block layers).
- Watch for the latent antenna effect at minimum spacings.

---

## Relation to the Bigger Picture

Section 5.1 introduces the foundational physics of how electrical stress destroys or degrades integrated circuits, establishing the failure mechanisms that inform virtually every layout decision in the rest of the book. Self-heating and filamentation directly motivate the ballasting techniques covered in the power device chapters (Chapters 9 and 13). Electromigration rules constrain metal routing throughout the entire design, while TDDB explains why certain oxide structures must be avoided and why process control is critical. The ESD and antenna effect discussions bridge from physics to practical layout structures (protection devices, scribe seals, antenna diodes) that every production layout must include. Together with [[ch05-contamination]] and [[ch05-surface-effects]], this section provides the reliability foundation upon which all analog layout decisions rest.

---

## See Also
- [[ch05-contamination]]
- [[ch05-surface-effects]]
