---
title: "14.4 ESD Protection"
chapter: 14
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-14, esd, reliability, protection-devices]
---

# 14.4 ESD Protection

> **Chapter 14: Special Topics**

## Key Concepts

Electrostatic discharge (ESD) protection is one of the most critical aspects of integrated circuit layout. Every pin on an IC (except substrate ground in pad-based networks) requires some form of protection against ESD events that can destroy gate oxides, PN junctions, and metal interconnects. The two dominant ESD models are the **Human Body Model (HBM)**, which generates peak currents of about 1.3 A over ~225 ns, and the **Charged Device Model (CDM)**, which generates peaks up to 10 A but lasts only a few nanoseconds.

ESD protection operates through two fundamental mechanisms:

1. **Snapback devices** -- These exploit avalanche breakdown followed by bipolar snapback. The device triggers at a high voltage $V_{t1}$ (first breakdown), then "snaps back" to a lower sustain/hold-in voltage. The key concern is that the hold-in voltage must exceed the maximum operating voltage of the pin, or the device will latch up during normal operation. Examples: GGNMOS, thick-field NMOS, MVSCR.

2. **Rate-fired (clipping) devices** -- These activate by capacitive coupling during fast ESD transients. They require disable circuits if the protected pin experiences large voltage slews during normal operation. Examples: Active FET, GCNMOS (hybrid).

The protection strategy is organized into two layers:
- **Primary protection** -- Located at each bondpad, absorbs the bulk of the ESD energy.
- **Secondary protection** -- Located near vulnerable structures (especially thin gate oxides), provides local clamping against residual voltage that primary protection cannot eliminate.

## Primary ESD Protection Devices

### Thick-Field NMOS

The thick-field NMOS was one of the earliest ESD protection structures. It uses a metal gate electrode over thick field oxide with NMoat source/drain regions. Since the thick-field threshold exceeds the NMoat/P-epi breakdown voltage, a channel never forms. Instead, the drain-backgate junction avalanches, injecting holes into the P-epi backgate. This debiases the backgate and turns on the parasitic lateral NPN transistor, which snaps back to a sustain voltage of roughly 50% of the NMoat/P-epi breakdown voltage.

**Key layout features:**
- The metal gate electrode reduces trigger voltage by projecting a vertical electric field into the drain-backgate depletion region, adding vectorially to the lateral field and causing earlier avalanche
- The NMoat overlap of contact at the drain must be **extended several microns beyond minimum** to provide ballasting and separate thermally fragile contacts from the heat generated in the drain-backgate depletion region
- Drain NMoat corners should be **filleted or chamfered** to minimize electric field intensification
- If NMoat is silicided, a **silicide block mask** must remove silicide everywhere except under contacts
- The metal gate must overlap into the drain to cover the metallurgical drain-backgate junction despite photolithographic misalignment

**Vulnerability to secondary breakdown:** During a positive strike, current filaments form between source and drain where localized avalanche occurs first. The NMoat overlap of contact provides ballasting to encourage the filament to spread. For multi-finger devices, one finger avalanches before the others, and the second snapback voltage $V_{t2}$ of the conducting finger must not be lower than the first snapback voltage $V_{t1}$ of the nonconducting fingers, or the conducting finger will self-destruct before others turn on.

Thick-field devices have largely been abandoned in modern processes because heavier NSD implants, drain silicidation, and shrinking geometries have degraded their ballasting and deepened snapback.

### Grounded-Gate NMOS (GGNMOS)

The GGNMOS is the most widely known CMOS ESD protection structure. It looks like a large multifinger NMOS transistor, but its operation depends on the parasitic lateral NPN inherent in its structure.

**Operation:** A positive ESD strike biases the drain positive. Since the gate is grounded, no channel forms. The drain-backgate junction avalanches (typically at a finger end), injecting holes that debias the backgate and turn on the parasitic NPN. Bipolar conduction creates a current filament from source to drain. Localized heating reduces carrier mobility, diminishing impact ionization and encouraging conduction to spread laterally along the finger. Resistance within one finger forces voltage to rise until additional fingers avalanche sequentially.

**Critical layout parameters:**
- **Gate length**: Typically minimum allowed for the NMOS type and voltage rating. Some designers use shorter channels to transform avalanche breakdown to punchthrough, reducing snapback depth but increasing trigger voltage variability
- **NMoat overlap of contact**: Larger overlaps improve robustness (ballasting) but increase series resistance. Optimal values are typically several microns greater than minimum
- **Silicide block**: For silicided-moat processes, silicide must be drawn back several microns from the NMoat edge. Silicided source/drain without blocking is extremely fragile
- **LDD implants**: Devices with lightly doped drain implants are typically less robust, possibly due to current intensification at the edge of the shallow LDD
- Total device width required for adequate protection is typically sized by constructing arrays and testing

**For proper function, all fingers must avalanche and conduct in unison.** This requires $V_{t2}$ (secondary breakdown) to considerably exceed $V_{t1}$ so the first conducting finger is not destroyed before others activate.

The GGNMOS also protects against negative strikes by forward-biasing the drain-backgate junction (reverse-active NPN mode), which develops very low voltage and is highly robust. An **electron-collecting guard ring** (ECGR) must be placed between the GGNMOS and other circuitry because the drain can inject electrons into the substrate.

### Gate-Coupled NMOS (GCNMOS)

The trigger voltage $V_{t1}$ of a GGNMOS is slightly higher than the $BV_{CEO}$ of the transistor because avalanche must inject enough current to debias the backgate and turn on the parasitic NPN. This means a GGNMOS cannot reliably protect other NMOS transistors of the same construction without series current-limiting resistors.

The GCNMOS solves this by coupling a fraction of the ESD transient to the gate through a capacitor $C_1$. The gate-to-drain capacitance $C_{gd}$ slightly augments $C_1$. When the gate voltage rises, a channel forms and impact ionization within the pinched-off region injects current into the backgate, reducing the trigger voltage. The trigger voltage reaches a minimum (roughly half its zero-bias value) when $V_{GS} \approx \frac{1}{2} V_{GD}$, which maximizes impact ionization.

**Circuit elements:**
- **Capacitor $C_1$**: Usually a PMOS transistor with drain, source, and backgate connected to the pin and gate connected to the NMOS gate node. Operates in inversion with capacitance per unit area equal to its gate oxide capacitance. Must be a significant fraction of the gate capacitance of the main transistor
- **Resistor $R_1$**: Holds the gate low during normal operation. Practical structures use $RC$ time constants of 20-50 ns
- **Disable circuit** (optional): Transistors $M_2$, $R_2$, $C_2$ hold the GCNMOS off during normal operation while allowing it to conduct during ESD. The disable filter has a time constant of at least 1 microsecond. HBM/CDM events occur before IC insertion, so leakage discharges $C_2$, allowing $M_1$ to turn on during strikes

**Advantage:** Channel conduction reduces trigger voltage enough to protect NMOS transistors of the same construction without current-limiting resistors.

**Drawback:** Larger than GGNMOS because of $C_1$, $R_1$, and possibly the disable circuit. The PMOS capacitor $C_1$ must be placed behind an electron-collecting guard ring along with the main transistor.

The gate voltage should be kept as low as possible to minimize channel current -- excessive channel current causes localized overheating near the surface in the pinched-off region. Reducing gate voltage forces more current through the bipolar transistor, where minority-carrier conduction is less localized and thus more robust.

### Backgate-Triggered NMOS (BTNMOS)

The GCNMOS's gate coupling encourages uniform bipolar conduction but also creates localized heating in the pinched-off region, limiting its $V_{t2}$ to roughly that of a similar GGNMOS. The BTNMOS takes an alternative approach: deliberately debiasing the NMOS backgate to promote bipolar conduction.

**Substrate-pumped NMOS** is a popular BTNMOS variant:
- Contains a main transistor $M_1$ and a predrive transistor $M_2$, both with gate coupling via capacitor $C_1$ and resistor $R_1$
- When the parasitic NPN within $M_2$ conducts, current flows into the substrate through a PMoat ring encircling both transistors
- This current generates a backgate-to-source voltage through the distributed substrate resistance $R_3$, further forward-biasing the parasitic NPN in both transistors ("substrate pumping")
- The predrive transistor $M_2$ is almost as large as the main transistor $M_1$ and is split into two symmetric halves flanking $M_1$

**Advantages over GCNMOS:**
- Lower $V_{t1}$ and potentially higher $V_{t2}$
- Current flows through a larger volume of silicon, resulting in a more robust device
- Often smaller than the corresponding GCNMOS

**Requires:** Disable circuit for pins with large voltage slews, and electron-collecting guard ring to prevent substrate debiasing from affecting nearby devices. Even an N-well strip connected to power supply suffices to cut off lateral majority carrier flow.

### Active FET

The active FET uses purely MOS conduction with no bipolar snapback. It has the same schematic as a GCNMOS but with $C_1$ large enough to fully enhance the pass transistor and the transistor large enough that peak ESD voltage never triggers snapback.

**Key properties:**
- The GGNMOS, GCNMOS, and Active FET form a continuum: GGNMOS is pure snapback, Active FET is pure rate-fired, GCNMOS is a hybrid
- Protection level scales **linearly** with pass transistor width -- easy to design
- Operating voltage equals the $V_{DS}$ rating of the NMOS used
- Can be constructed from any NMOS type, including high-voltage LDMOS
- Colloquially called "big FETs" because they are almost always larger than other protection devices -- conduction is constrained to a very shallow layer of silicon

**Drawbacks:** Very large area; requires disable circuitry for pins with fast voltage slews; injects electrons into the substrate (needs ECGR).

### Medium-Voltage Silicon-Controlled Rectifier (MVSCR)

The SCR achieves the deepest snapback of any ESD device, with hold-in voltages as low as ~1 V. This deep snapback greatly reduces power dissipation, making SCR-based structures among the smallest ESD solutions. However, the same deep snapback is also the SCR's greatest weakness: it intentionally latches up.

**Structure (in N-well CMOS):**
- Lateral PNP $Q_1$: PMoat emitter inside N-well (base), P-epi collector
- Lateral NPN $Q_2$: NMoat emitter adjacent to N-well, P-epi base, N-well collector
- These are the same two parasitic bipolars responsible for CMOS latchup
- Well resistance $R_1$ and substrate resistance $R_2$ hold the SCR off during normal operation and determine hold-in current
- Zener diode $D_1$ (NMoat/P-epi junction) triggers the SCR; trigger voltage $\approx BV_{NMoat/P\text{-}epi}$

**Critical design rule:** Only use an SCR-based device when the maximum operating voltage is less than the hold-in voltage, OR the maximum current available to the SCR is less than the hold-in current. Even modified high-hold-in-current SCRs achieved only ~100 mA hold-in, which many pins can exceed during normal operation.

Stacking two or three SCRs in series can raise the hold-in voltage, but this increases size and negates the primary benefit.

### Lateral PNP

The lateral PNP in CMOS processes exhibits deep snapback similar to vertical NPN transistors, due to conductivity modulation within the base (not the collector). This makes it attractive for high-voltage ESD protection.

**Structure:**
- PMoat emitter inside N-well (base)
- Retrograde well profiles suppress vertical conduction and enhance lateral conduction
- More lightly doped active base region produces more pronounced snapback
- Collector: surrounding P-well with embedded substrate contacts; heavier doping limits Kirk effect and maximizes base conductivity modulation

**Layout:** Closely interdigitated narrow emitter and collector fingers maximize peripheral gain and minimize base voltage drop. PNP transistors generally do **not require ballasting** because holes have lower impact ionization rates than electrons, making them less subject to electrical filamentation.

**Floating-base variant:** Omitting base contacts reduces collector-to-emitter spacing, yielding smaller structures with lower saturation voltage. Trigger voltage drops from $BV_{CES}$ (shorted base-emitter) to $BV_{CEO}$ (open base). Experimentally, these show almost no snapback. The floating-base configuration also eliminates the N-well connection to the pad, preventing electron injection during negative strikes (though capacitive coupling may still produce a small pulse).

### Comparison of Primary ESD Devices

| Device | Behavior | Area | Needs ECGR? |
|--------|----------|------|-------------|
| Buffered Zener | Clipping / Snapback | Medium | Yes |
| Antiparallel diodes | Clipping | Small | Yes |
| Dual diodes | Clipping | Small | Yes |
| Thick-field NMOS | Snapback | Medium | Yes |
| GGNMOS | Snapback | Medium | Yes |
| GCNMOS | Rate fired (hybrid) | Medium-Large | Yes |
| BTNMOS | Snapback (hybrid) | Medium | Yes |
| Active FET | Rate fired | Large | Yes |
| MVSCR | Snapback | Small | Yes |
| Lateral PNP | Snapback | Small | Yes (very low) |

## Secondary ESD Protection

Primary protection may be insufficient for especially vulnerable structures: emitter-base junctions of bipolar transistors, drains of certain MOS transistors, and MOS gate oxides.

### Emitter-Base Clamp

Some bipolar transistors (standard-bipolar NPN, poly-emitter bipolars) suffer **avalanche-induced beta degradation** -- reverse current across the base-emitter junction permanently degrades low-current $\beta$. This is a parametric shift (not functional failure) but can cause matched devices to mismatch, violating input bias current specifications.

**Solution:** A diode-connected transistor $Q_3$ clamps the reverse voltage across the emitter-base junction of the protected transistor $Q_1$. A resistor $R_1$ limits the current through $Q_3$, allowing it to be minimum-sized. A balancing resistor $R_2$ compensates the voltage drop from base current flowing through $R_1$. With $R_2 = R_1$ (for balanced collector currents), the matched pair remains balanced.

### Drain Ballasting Resistors

Standard digital CMOS output transistors act as unintentional GCNMOS devices during positive ESD strikes (via $C_{gd}$ coupling). The output transistor is seldom large enough to self-protect, and adding a GGNMOS is futile because the output transistor triggers at a lower voltage and absorbs the strike.

**Solution:** Series-limiting resistor $R_1$ between the pin and the output transistor. A GGNMOS protection device at the pin snaps back, and $R_1$ limits current through the output transistor to safe levels.

**Per-finger ballasting for multi-finger outputs:** ESD designers specify an $RW$ product (resistance times finger width). Instead of one large resistor for the entire transistor, separate smaller resistors connect in series with each drain finger. Five resistors each meeting the $RW$ product per finger provide $\frac{1}{5}$ the effective series resistance of a single ballasting resistor while still protecting each finger individually.

### RC Filters for CDM Protection

CDM strikes last only 1-2 ns. A thin gate oxide (e.g., 50 A) can withstand ~10 V for 100 ns, easily surviving HBM. But a CDM strike developing 10 A through a GCNMOS with ~2 $\Omega$ hold-in resistance plus metallization resistance can produce voltages far exceeding gate oxide breakdown.

**Key insight:** The secondary protection must be located **near the gate oxide it protects**, not next to the bondpad. CDM currents of 10 A can generate destructive voltages across metallization resistance alone.

A simple RC filter with $\tau \geq 10$ ns provides effective CDM protection. The capacitor connects across the gate-source of the protected transistor (placed near it); the resistor can reside anywhere along the metal line from the bondpad, since it limits current flow, not voltage drop. The $RC$ product must equal or exceed 10 ns.

### CDM Clamps

For CMOS input stages with both NMOS and PMOS gates, dedicated CDM clamp circuits are more efficient than two separate RC filters.

**Non-failsafe CDM clamp:** Resistor $R_1$ limits current; small GGNMOS devices $M_3$ and $M_4$ clamp voltage across the NMOS and PMOS gate oxides respectively. $M_3$ conducts during positive strikes; $M_4$ conducts during negative strikes.

**Failsafe CDM clamp:** An additional GGNMOS transistor prevents conduction when the input voltage rises above $V_{DD}$ during normal operation. Most designers prefer failsafe structures despite slightly larger dimensions.

**Layout rules for CDM clamps:**
- Clamp transistors ($M_3$, $M_4$) must reside **near the gate oxides they protect**
- Metal lines to ground/power buses should connect near the protected transistors to minimize metal debiasing
- The lead connecting back to the input pin must be wide enough to handle instantaneous CDM current
- Each clamp transistor should be a single finger wide enough to meet the applicable $RW$ product
- Guard ring needed unless $R_1$ is large enough (> ~$500\ \Omega$) to limit injected current to harmless levels

## Die-Level ESD Protection Strategies

### Pad-Based Protection Networks

Each pin (except substrate ground) gets a separate primary protection device connecting to a substrate return bus -- a wide, low-resistance lead running around the die edge.

For a 2 kV HBM strike between two pins (e.g., INP positive w.r.t. INM), the voltage difference is:

$$\Delta V = V_{E1}(I_p) + V_{E2}(-I_p) + I_p R_{bus}$$

where $I_p \approx 1.3$ A is the peak HBM current, $V_{E1}$ is the positive-strike voltage of one device, $V_{E2}$ is the negative-strike voltage of the other, and $R_{bus}$ is the substrate ring resistance between them. Most ESD designers target $R_{bus} \leq 1\ \Omega$, developing $\leq 1.3$ V.

### Rail-Based Protection Networks

Two wide rails (VSS and VDD) run around the die. A primary protection device connects between rails. Dual diodes connect each pin to both rails. For a strike between two pins:

$$\Delta V = V_{D1}(I_p) + V_{E5}(I_p) + V_{D4}(I_p) + I_p R_{VDD} + I_p R_{VSS}$$

Diode forward voltages are small (~2 V each). Bus resistances must be kept low, typically $\leq 1\ \Omega$ each. The primary device $E_5$ is usually a rate-fired device such as an active clamp.

**Rail-based networks are common in digital ICs but rare in analog** because:
- Analog devices often have multiple power supplies sequenced unpredictably, and dual diodes are not failsafe
- Analog processes can generally construct snapback/clamp devices suitable for pad-based networks

Many analog ICs use **customized hybrid networks**. For example, a power IC with separate GND and PGND pins may route the output protection to PGND (to avoid disturbing substrate ground) while connecting GND and PGND through antiparallel diodes.

### High-Current Metallization

ESD currents can generate large voltage drops and melt/vaporize narrow metal leads.

**Resistance target:** Keep metallization drops below ~2 V during HBM (1.3 A), so maximum resistance $\leq 1.5\ \Omega$. For pad-based networks, ~$\frac{1}{3}$ of total is allocated to the substrate ground ring.

**Minimum ground ring width:**

$$W_{bus} = \frac{R_s (W + L - 4E)}{2 R_{max}}$$

where $R_s$ is bus metal sheet resistance, $W$ and $L$ are die dimensions, $E$ is corner exclusion zone distance, and $R_{max}$ is the target maximum resistance. Use process maximum $R_s$ at ambient temperature ($25\ ^\circ C$). Stack all available metal layers to minimize sheet resistance.

**Adiabatic temperature rise** in a metal lead conducting peak HBM current $I_p$:

$$\Delta T = \frac{\rho \cdot \tau_{HBM} \cdot I_p^2}{2 \cdot A^2 \cdot c_v}$$

where $\rho$ is metal resistivity, $\tau_{HBM} = 225$ ns, $A$ is cross-sectional area, and $c_v$ is volumetric specific heat. The rise is treated as adiabatic because very little heat exchange occurs in a few hundred nanoseconds. Designers typically limit $\Delta T \leq 200\ ^\circ C$.

| Material | $\rho$ ($\mu\Omega \cdot$ cm) | $c_v$ (J/cm$^3$ K) |
|----------|------|------|
| Aluminum | 2.7 | 2.42 |
| Copper | 1.7 | 3.45 |
| Cobalt disilicide | 15 | 0.56 |
| Nickel monosilicide | 10.5 | 3.45 |
| Titanium disilicide (C54) | 15 | 0.85 |
| Silicon | varies | 1.66 |

**Layout precautions for ESD metal:**
- Add 45-degree chamfers to inside corners, extending at least half the lead width into the turn
- Orient slots (for stress relief) parallel to current flow, never perpendicular
- Fill via overlap areas completely with as many vias as possible when transferring between metal layers

### High-Current Resistors

Secondary protection resistors must be wide enough to survive ESD current heating. Use the same adiabatic temperature rise equation, with $\rho = R_s \times t$ (sheet resistance times film thickness). The peak current is computed by dividing the peak voltage across the resistor by its resistance.

For silicided resistors, almost all current flows through the silicide, so silicide properties (resistivity, thickness, specific heat) determine the temperature rise rather than the underlying silicon.

## Diagrams

### Figure 14.30 -- GGNMOS Schematic and Operation
![[diagrams/ch14-esd-protection-fig1.png]]
Page showing the GGNMOS device description: schematic, layout (multifinger NMOS with grounded gate), and detailed explanation of the parasitic lateral NPN operation, finger activation sequence, and the roles of NMoat overlap, silicide blocking, and LDD implants in determining device robustness.

### Figure 14.33 -- Medium-Voltage SCR (MVSCR) Cross-Section
![[diagrams/ch14-esd-protection-fig2.png]]
Schematic and cross-section of the MVSCR in an N-well CMOS process. Shows the lateral PNP ($Q_1$: PMoat emitter, N-well base, P-epi collector) and lateral NPN ($Q_2$: NMoat emitter, P-epi base, N-well collector) that form the SCR, along with well resistance $R_1$, substrate resistance $R_2$, and trigger Zener diode $D_1$ (NMoat/P-epi junction).

### Figure 14.39 -- Die-Level ESD Protection Networks
![[diagrams/ch14-esd-protection-fig3.png]]
Comparison of pad-based (A) and rail-based (B) primary ESD protection networks. Pad-based uses individual protection devices from each bondpad to a substrate ground ring. Rail-based uses dual diodes from each pin to VDD/VSS rails with a primary protection device between the rails.

## Guidelines for Choosing Primary ESD Devices (Pad-Based Networks)

1. **Substrate ground bondpads do not require primary protection** -- all other pins' ESD devices collectively protect it. Just ensure the ground ring is wide enough.

2. **Two bondpads connected by high-current metallization can share one primary protection device** -- metallization must handle both ESD currents and DC current imbalances from unequal bondwire lengths.

3. **Separate grounds should be interconnected by antiparallel diodes** -- separate grounds seldom differ by more than a few hundred millivolts.

4. **Separate bondpads to a common package pin need separate primary protection** -- bondwire inductance (~1 nH/mm) generates several volts per nanohenry during CDM strikes. One bondwire's protection does not cover another.

5. **CMOS inputs: use GGNMOS + CDM clamps** -- GCNMOS also works. If external circuitry can sustain snapback, use BTNMOS. Gate oxides need failsafe CDM clamps near the input transistors.

6. **CMOS inputs in BiCMOS: consider $BV_{CBO}$ clamps** -- trigger voltages of ~10 V, very compact, no ECGR needed. Works well for ~5 V pins.

7. **Large MOS output transistors can self-protect** -- if deliberately oversized and laid out like ESD devices (with silicide blocking, ECGR, ample substrate contacts). Excessive gate-drain coupling turns them into Active FETs (large area penalty).

8. **Large NPN/PNP collector-base junctions can self-protect** -- they act as $BV_{CBO}$ or $BV_{CEO}$ devices. NPN emitter-connected pins usually need additional antiparallel diode clamps.

9. **CMOS power supply pins: use GCNMOS or Active FET** -- GGNMOS trigger voltage is too close to the transistor avalanche voltage. GCNMOS hold-in voltage must exceed max operating voltage; otherwise use Active FET with disable circuit.

10. **High-voltage pins: use PNP, power Zener, $BV_{CBO}$, or $BV_{CEO}$** -- PNP is probably the best general-purpose high-voltage option (small, no rate-firing, shallow snapback). SCR devices are small but latch up easily.

11. **Consider stacking primary protection devices** -- two antiparallel diode pairs in series for power grounds with transients, two $BV_{CBO}$ clamps in series for higher-voltage pins. Each device in a stack is ~2x the size of a standalone device.

12. **If all else fails, use an Active FET** -- can be built from any NMOS type including LDMOS. Handles any voltage up to $BV_{DSS}$ of the highest-voltage MOSFET available. Large but reliable.

## Practical Takeaways

- **Every pin except substrate ground** needs primary ESD protection in a pad-based network
- **Electron-collecting guard rings** are required for almost all ESD devices (they can inject electrons into the substrate during negative strikes)
- **Silicide blocking** is essential for GGNMOS and similar devices in silicided processes -- without it, devices are extremely fragile
- **NMoat overlap of contact** is the single most critical dimension for snapback device robustness -- it provides ballasting that prevents thermal runaway
- **CDM protection must be local** -- place secondary clamps near the gate oxides they protect, not at the bondpad, because metallization resistance alone can generate destructive voltages at 10 A
- **RC time constants of $\geq 10$ ns** suffice for CDM secondary protection
- **Failsafe CDM clamps** are preferred over non-failsafe versions despite slightly larger area
- **Substrate ground ring width** must be computed from die dimensions and metal sheet resistance to keep resistance below ~1 $\Omega$
- **Metal leads carrying ESD current** need 45-degree chamfers at inside corners, properly oriented slots, and adequate width to prevent adiabatic heating above ~200 degrees C
- **Multi-finger devices** require careful attention to ensure all fingers activate -- per-finger ballasting (via NMoat overlap or explicit resistors) is critical
- **SCR-based devices are tempting** (smallest area) but **dangerous** -- only use when operating voltage < hold-in voltage or available current < hold-in current
- The **$RW$ product** (resistance times finger width) is the key design parameter for drain ballasting of output transistors

## Relation to the Bigger Picture

ESD protection is the bridge between the electrical overstress phenomena described in [[ch05-electrical-overstress]] and the practical layout of complete integrated circuits. The interconnection strategies, metal sizing, and guard ring techniques discussed here build directly on the metallization and via concepts from [[ch14-interconnection]]. Understanding ESD is essential because every bondpad on every IC must include protection, making ESD devices a significant fraction of the die periphery area and a major constraint on padring layout. The choice and sizing of ESD structures directly affects die area, cost, and reliability -- a poorly protected IC will fail in the field, while overprotected pins waste silicon area and may introduce unwanted parasitics into signal paths.

## See Also
- [[ch14-interconnection]]
- [[ch05-electrical-overstress]]
