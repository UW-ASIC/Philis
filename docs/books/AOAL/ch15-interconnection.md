---
title: "15.4-15.5 Top-Level Interconnection and Checklist"
chapter: 15
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-15, routing, vias, kelvin-connections, electromigration, noise-coupling, checklist]
---

# 15.4-15.5 Top-Level Interconnection and Checklist

> **Chapter 15: Assembling the Die**

## Key Concepts

### Manual Top-Level Interconnection (15.4)

While digital designs rely almost exclusively on autorouting due to their complexity, small analog ICs can still be interconnected manually faster than they can be autorouted. Manual interconnection therefore remains a core skill in analog layout.

There are two fundamental top-level interconnection strategies for designs with two or more metal layers:

- **Maze routing**: Signals run over or through individual circuit blocks. Saves 5-10% die area, but is slower to lay out, harder to modify, and more prone to unexpected coupling between wires and underlying components.
- **Channel routing**: Signals run through dedicated channels placed between circuit blocks. Easier to manage, modify, and verify. This is the preferred approach for most analog designs.

Single-level-metal (SLM) designs require hand-crafted devices to minimize signal crossings. Tunnels (jumpers) must be inserted wherever crossings remain. A single overlooked signal can force a complete die relayout. Double-level metal actually reduces cost for designs with more than about 50 components because area savings outweigh the additional processing cost.

### The Final Layout Checklist (15.5)

Before tapeout, design teams perform a final layout review -- the last chance to catch errors before committing to the expensive manufacturing process. The checklist provided is a comprehensive set of 17 questions that every layout team should be able to confidently answer. These span DRC/LVS verification, electromigration, ESD, latchup, noise coupling, bondpad placement, Kelvin connections, corner exclusion zones, scribe seals, substrate contacts, die symbolization, and archival procedures.

## Vias (15.4.1)

Vias are required whenever a signal transitions between metal layers. Four distinct via technologies exist, each with different properties:

### Aluminum Vias
The original via technology. A hole is etched through the interlevel oxide (ILO) above a lower metal geometry, and the upper metal (including a thin refractory barrier metal layer for sidewall step coverage plus sputtered aluminum alloy) is deposited over it. **Not stackable** -- a VIA2 cannot sit on a VIA1 due to the nonplanar surface. Both the upper and lower metal layers must symmetrically overlap the via by several tenths of a micron.

### Tungsten Plug Vias
Developed because aluminum cannot reliably fill submicron openings. A hole is etched through the ILO and filled entirely with tungsten, then polished back using CMP to create a highly planar surface. **Stackable** -- VIA2 can sit directly on VIA1. Asymmetric top-metal overlaps are permitted: two opposite sides can have smaller overlaps than the other two sides (sometimes even zero overlap).

### Single Damascene Copper Vias
Use tungsten plug vias in conjunction with CMP-planarized lower metal. Because the lower metal surface is also planarized, asymmetric overlaps are permitted on **both** sides of the via (above and below). Stackable.

### Dual Damascene Copper Vias
Created by a different manufacturing process (the via and metal are deposited together), but share the same characteristics as single damascene: asymmetric overlaps both above and below, and stackable.

| Technology | Stackable | Lower Metal Overlap | Upper Metal Overlap |
|---|---|---|---|
| Aluminum vias | No | Mandatory (symmetric) | Mandatory (symmetric) |
| Tungsten plug | Yes | Mandatory (2 sides) | Optional |
| Single damascene | Yes | Optional | Optional |
| Dual damascene | Yes | Optional | Optional |

**Via Reliability**: Differences in coefficients of thermal expansion between silicon, ILO, and metal generate mechanical stresses. Thermal cycling can nucleate voids at interfaces within vias. Because void formation is probabilistic, each via has a small but nonzero chance of failing. Therefore, many fabs mandate **pairs of vias** rather than single vias. Multiple vias in close proximity have a protective effect beyond mere redundancy -- void formation in one via may relieve stress on adjacent vias.

## Routing Pitches and Grids (15.4.2)

Metal systems are characterized by their **pitch** -- the center-to-center distance between adjacent leads. Three routing pitch definitions exist:

**Lead-to-lead routing pitch** $P_{LL}$ -- the minimum pitch when leads are wide enough to accommodate vias on their own:

$$P_{LL} = W_{min} + S_{min}$$

where $W_{min}$ is the minimum drawn metal width and $S_{min}$ is the minimum metal-to-metal spacing.

**Via-to-via routing pitch** $P_{VV}$ -- the pitch when leads must be widened to accommodate vias (the common case):

$$P_{VV} = V + 2 \cdot OL_{min} + S_{min}$$

where $V$ is the drawn via width and $OL_{min}$ is the minimum drawn metal overlap of via (using the smallest overlap for asymmetric rules).

**Via-to-lead routing pitch** $P_{VL}$ -- achieved by staggering vias, allowing one lead to be at minimum width:

$$P_{VL} = \frac{V}{2} + OL_{min} + S_{min} + \frac{W_{min}}{2}$$

Staggering vias creates congestion at channel intersections, so most designers prefer $P_{VV}$.

The **channel width** required to accommodate $N$ minimum-width leads is:

$$W_{ch} = N \cdot P + W_{min}$$

### Grid Considerations

- The routing grid increment must be an integer multiple of the coding increment
- A **consistent grid** has metal edges falling on grid points; an **inconsistent grid** only has path centerlines on grid
- Skilled designers sometimes use inconsistent grids for rapid routing, then revert to a smaller consistent grid for interfacing
- **Octagonal routing** (45-degree segments) is useful for routing across corner exclusion zones, but raises concerns about roundoff errors -- some rules require wider diagonal segments

### Example

For a tungsten-plug aluminum metal system with $W_{min} = 0.3\,\mu m$, $S_{min} = 0.3\,\mu m$, $V = 0.25\,\mu m$, $OL_{min} = 0.05\,\mu m$: $P_{LL} = 0.6\,\mu m$ and $P_{VV} = 0.65\,\mu m$. However, if the coding increment is $0.05\,\mu m$, the lead width must increase to $0.4\,\mu m$ to stay on grid, making $P_{VV} = 0.7\,\mu m$.

## Channel Routing (15.4.3)

Channel routing requires at least two interconnection layers. At a minimum, one level of metal plus polysilicon (sometimes called "one-and-a-half layer metal"). Unsilicided gate poly has sheet resistance of $20$--$40\,\Omega/\square$; silicided poly can achieve less than $10\,\Omega/\square$.

### Rules for Channel Routing

1. **Orthogonal metal directions**: Metal-1 runs in one direction (e.g., horizontal), metal-2 in the perpendicular direction (e.g., vertical). Additional layers alternate. Enforce this convention across the entire die.
2. **Route wide leads first**: Power and ground lines are typically wider than signal lines. Choose widths that consume integer multiples of the routing pitch so minimum-width leads can replace unused portions. The ideal widths are:

$$W = n \cdot P - S_{min}$$

where $P$ is the routing pitch, $S_{min}$ is the metal spacing, and $n$ is a positive integer.

3. **Work from edges inward**: Place first signals at the outside edges of the channel, maintaining minimum spacing throughout. Jogs propagate laterally and cause congestion elsewhere.
4. **Via arrays at wide-lead transitions**: When wide leads change layers, extend both metal leads fully across each other and fill the overlap with as many vias as possible.
5. **Primary routing channels** should hold about 10-20% of all top-level signals. A design with 100 signals needs room for ~20 signals in each primary channel. Channels taper narrower toward die edges. Even the narrowest feeder channels should allow at least 5 signals -- it is nearly impossible to increase channel widths once routing has begun.

### Overloading Channels

When a channel runs out of room:
- Reroute signals through alternate channels
- Run extra signals on the "wrong" metal layer, snaking to avoid collisions
- Push signals onto polysilicon (reserve metal-required signals first; route poly-capable signals last)

### Color-Coding Scheme for Top-Level Routing

| Signal Type | Color | Label |
|---|---|---|
| High-voltage | Red | Voltage |
| High-current | Yellow | Width |
| High-voltage + high-current | Orange | Voltage, width |
| Sensitive analog | Green | -- |
| Noisy digital | Purple | -- |
| Static digital (can route in poly) | Gray | -- |
| Other (must route in metal) | Light blue | -- |

Circuit designers should avoid "air wires" (signals connected by matching names but not explicitly wired on the schematic) as these make color-coding systems far less legible.

## Special Routing Techniques (15.4.4)

### Star Nodes and Kelvin Connections

Metal lead resistance, though small, is not always negligible. Consider matched bipolar transistors whose emitters connect to a ground return line carrying 1 mA. If the ground line between points A and B contains 10 squares of $30\,m\Omega/\square$ metal, a voltage drop of $0.3\,mV$ develops -- enough to upset sensitive circuitry.

The solution is a **star node**: all sensitive emitter leads return to a **single common point** C. Since both leads connect to the same point, the ground current cannot generate any differential between them. Voltage drops elsewhere along the lead cause both emitters to vary in unison (common mode), to which most circuits are highly immune.

Star nodes derive from the **Kelvin connection** (named for Lord Kelvin's double bridge circuit of 1861). In a Kelvin arrangement, two **force leads** ($F_1$, $F_2$) carry the large current through a component (e.g., a low-value resistor), while two **sense leads** ($S_1$, $S_2$) connect to a low-current sensing circuit. Because sense leads carry negligible current, almost no voltage drops occur along them. For extreme accuracy, match the currents flowing through both sense leads and route them with equal resistance so that any residual voltage drops cancel.

### Noisy Signals and Sensitive Signals

Electrical noise in ICs is dominated by **capacitive coupling** (interference), not device noise. Metal leads running adjacent to or above/below each other can exhibit tens to hundreds of femtofarads of capacitance. Even 1 fF can generate significant interference.

The voltage shift induced by capacitive coupling is:

$$\Delta V = C_{coupling} \cdot \frac{dV}{dt} \cdot R_{node}$$

A modern logic gate switching in < 0.1 ns produces slew rates exceeding $10\,V/ns$. Such a signal coupling across 10 fF into a $100\,k\Omega$ node generates a 1 V disturbance.

**Noisy signals** are those with rapid voltage slewing (e.g., high-frequency clocks, fast digital transitions). **Sensitive signals** include:

- Inputs to high-gain amplifiers and precision comparators
- Inputs to ADCs
- Outputs of precision voltage references
- Analog ground lines to accurate circuitry
- High-value analog resistor networks
- Very low-voltage or very low-current signals
- Current bias lines into low-current analog circuitry

**Layout guidelines for noise management:**

- Noisy signals must not run on top of, below, or adjacent to sensitive signals
- If crossing is unavoidable, minimize intersection area by crossing at right angles
- Use parasitic extraction and back-annotation to quantify coupling
- Insert **electrostatic shields** (a quiet, low-impedance metal plane) between noisy and sensitive signals, extending beyond the intersection area by roughly $5\,\mu m$ to account for fringing fields
- The shield should tie to the reference node of the sensitive signal (e.g., analog ground)
- Insert series resistors ($\sim 10$--$100\,k\Omega$) in slow digital lines to reduce their slew rate and hence noise injection
- Use adjacent low-noise leads (static digital, extra power/ground) as lateral shields between noisy and sensitive signals running side-by-side

Circuit designers must communicate which signals are noisy and which are sensitive. A common convention is to suffix signal names with "_n" (noisy) or "_s" (sensitive).

### High-Voltage Signals

Modern metal-to-metal spacings are so small that lateral electric fields can approach ILO breakdown. A typical maximum ILO field is $\sim 5\,MV/cm$, translating to about 25 V for a minimum spacing of $0.5\,\mu m$ -- but actual spacing varies due to proximity effects, OPC, damascene sidewall tapering, etc.

**Low-k dielectrics** (used in advanced CMOS for capacitance reduction) rely on porosity and therefore have reduced dielectric strength compared to fully densified oxide.

Many analog processes include **voltage-dependent spacing rules**. Implementing these in DRC is extremely difficult because voltages depend on circuit state and vary with time. Practical approaches:
- Flag high-voltage leads with a suffix (e.g., "_hv") and apply larger spacings
- Use voltage-aware DRC tools (e.g., Mentor Calibre) that extract voltage differentials from the netlist

### High-Current Signals

Two constraints on lead width for high-current signals:

1. **Electromigration**: Failures occur at points of greatest stress, so the lead must meet EM rules at **every point** along its length. No localized necking is allowed.
2. **Resistance**: An average over the entire lead length. Short narrow sections have minimal impact on long leads. Selective widening wherever room exists is beneficial.

The minimum lead width for electromigration compliance is:

$$W_{min} = \frac{I_{max}}{J_{max} \cdot t_{min}}$$

where $I_{max}$ is the maximum DC current, $J_{max}$ is the allowed current density, and $t_{min}$ is the minimum metal thickness. Example: a 50 mA lead in $8000\,\text{\AA}$ thick metal with $J_{max} = 1\,mA/\mu m$ requires $W_{min} = 62.5\,\mu m$.

The well-known MIL-M-38510 value for $J_{max}$ is $1\,mA/\mu m$ for aluminum (pure or doped) under glass passivation. In practice, $1\,mA/\mu m$ is often assumed for copper-doped aluminum under compressive nitride at $150^\circ C$.

**Temperature derating** uses Black's law:

$$D = \exp\left[\frac{E_a}{k_B}\left(\frac{1}{T_{ref}} - \frac{1}{T_{op}}\right) \cdot \frac{1}{n}\right]$$

where $E_a$ is the activation energy ($\sim 0.5\,eV$ for pure Al, $\sim 0.7\,eV$ for Cu-doped Al), $k_B = 8.62 \times 10^{-5}\,eV/K$, and $n$ is the current exponent ($\sim 2$ for aluminum, $\sim 1$ for copper). A lead rated for 25 mA at $125^\circ C$ can only handle ~15 mA at $125^\circ C$ if the reference was $25^\circ C$ (derating factor $D = 0.58$).

**Copper vs. Aluminum**: Copper has higher $J_{max}$ at moderate temperatures ($85$--$125^\circ C$), but its current exponent is ~1 (vs. ~2 for Al). The two systems reach parity around $200^\circ C$; above that, aluminum is actually superior. Copper's lower exponent stems from difficulty passivating the upper copper surface.

**Electromigration at corners**: Current crowds toward the inside of 90-degree bends, creating localized stress. Replace 90-degree bends with two 45-degree bends. Via arrays placed at bends experience drastically nonuniform current density -- place via arrays in straight lead sections instead.

## Diagrams

### Via Cross-Sections (Figure 15.21)

![[diagrams/ch15-interconnection-fig1.png]]

Cross-sections of four via technologies: (A) aluminum via with nonplanar surface, (B) tungsten plug via with CMP-planarized top, (C) single damascene tungsten-plug via with CMP-planarized lower metal, and (D) dual damascene copper via. The progression from (A) to (D) shows increasing planarity and stackability, enabling denser multi-level interconnect.

### Star Nodes and Kelvin Connections (Figures 15.26-15.27)

![[diagrams/ch15-interconnection-fig2.png]]

Kelvin connection schematic and layout. (A) Shows how force leads $F_1$, $F_2$ carry large current through a component while sense leads $S_1$, $S_2$ accurately measure the voltage drop. (B) Layout of an integrated metal resistor with four-wire Kelvin connections implemented on metal-2. The key insight is that sense leads carry negligible current and thus develop negligible voltage drops.

### Electrostatic Shielding (Figure 15.28)

![[diagrams/ch15-interconnection-fig3.png]]

Electrostatic shielding techniques. (A) A grounded metal-2 shield plane placed between a noisy metal-3 signal and a sensitive metal-1 signal -- the shield must extend beyond the intersection area to capture fringing fields and must connect to the reference node of the sensitive signal. (B) Shield leads run between noisy and sensitive signals routed on the same layer.

## Final Layout Checklist (15.5)

The 17-point checklist for final layout review before tapeout:

1. **DRC Diagnostics**: All remaining DRC diagnostics reviewed and waived by process engineering. Never waive actual problems.
2. **LVS Topological Errors = Zero**: One topological error can mask others. These cannot be waived -- fix the layout or the LVS deck.
3. **LVS Parametric Errors**: Review and waive false errors (e.g., guard ring area mismatches at top-level). Confirm with the circuit designer.
4. **Electromigration on Power Wires**: Manually verify crucial power routing widths. Generate net-specific power plots. Use finite-element tools (e.g., Silicon Frontline R3D) for complex power transistor metallization.
5. **High-Current Lead Resistances**: Verify that leads with specific resistance requirements meet specifications. The circuit designer should communicate requirements before routing begins.
6. **Antenna Violations**: All antenna rule violations corrected by insertion of jumpers or antenna diodes. Zero violations at tapeout.
7. **ESD Protection**: Review primary ESD structures on every pin. Check metallization adequacy, inter-device resistance, and CDM clamp placement near gate oxides. Verify CDM clamps at supply/ground domain crossings.
8. **Latchup**: Check all diffusions connected to pins (directly or through $< 1\,k\Omega$). I/O pins are most vulnerable (both positive and negative transients). Use minority carrier guard rings or strategic substrate/well contacts.
9. **Dummy Metal Blocking**: Verify dummy metal block layers are placed over matched devices (especially MOS transistors) that lack metal field plates. CMP dummy fill can degrade matching.
10. **Noisy vs. Sensitive Signal Routing**: Verify no noisy signals run above, below, or adjacent to sensitive signals. Apply electrostatic shielding where unavoidable. Use parasitic extraction to quantify coupling.
11. **Bondpads and Probe Pads**: Review sizes, locations, and patterns with assembly/test site. Check lead widths at pad junctures. Verify pad #1 identification.
12. **Kelvin Connections and Star Nodes**: Verify physical locations match schematic intent, especially at bondpads. Schematic resistors separating sense and force leads ensure different signal names but cannot guarantee correct physical placement.
13. **Corner Exclusion Zones and Metal Slotting**: Verify exclusion zones are properly implemented with only approved structures inside.
14. **Scribe Seals and Scribe Streets**: Verify presence, proper construction, correct width, and connection to ground metallization.
15. **Substrate Contacts in Empty Areas**: On $P^-$ substrate dice, fill unused areas with substrate contacts. Balance against bypass capacitance needs.
16. **Die Symbolization**: Part number, company logo, mask work notice (circled "M" plus completion date).
17. **Archival**: Archive all files (layout database, libraries, DRC/LVS/PG decks, GDSII, mask data, documentation). Use checksums for verification. Store at least two copies in separate physically secure locations. Check annually.

## Practical Takeaways

- **Always use pairs of vias** (or more) rather than single vias to guard against stress-induced voiding. This is a fab requirement at many foundries and a reliability best practice.
- **Maintain consistent routing direction per metal layer** across the entire die. Metal-1 horizontal, metal-2 vertical (or vice versa), alternating for additional layers.
- **Size primary routing channels for 10-20% of total signals**. Even the narrowest feeder channels should accommodate at least 5 signals. Channel widths are nearly impossible to increase after routing begins.
- **Route wide (power/ground) leads first**, choosing widths that are integer multiples of the routing pitch minus the spacing.
- **Never neck down a high-current lead**, even briefly, to get around an obstruction -- electromigration failures occur at the narrowest point.
- **Place via arrays in straight lead segments**, not at corners, to avoid current crowding in the inside-corner vias.
- **Replace 90-degree bends with pairs of 45-degree bends** in high-current leads to reduce electromigration stress.
- **Use star nodes** for matched device connections -- bring all sensitive leads back to a single common point to eliminate differential ground drops.
- **Kelvin connections** (four-wire sensing) are essential for accurate measurement of on-chip resistors: force leads carry current, sense leads measure voltage with negligible current flow.
- **Shield noisy signals from sensitive signals** using interposed grounded metal planes or adjacent low-impedance leads. The shield must tie to the sensitive signal's reference node.
- **Insert series resistors** in slow-switching digital lines to reduce slew rate and thus capacitive noise injection into analog circuitry.
- **Color-code signals** on the schematic by type (high-voltage, high-current, noisy, sensitive, etc.) to communicate routing requirements to layout designers.
- **Complete the 17-point checklist** before tapeout. LVS must have zero topological errors (non-negotiable). DRC diagnostics and LVS parametric errors may be waived only after review with process/circuit engineering.
- **Archive everything** with checksums, in at least two separate physical locations, and verify annually.

## Relation to the Bigger Picture

Sections 15.4 and 15.5 represent the culmination of the entire layout process described throughout the book. Everything from device physics (Chapter 1), fabrication (Chapter 2), design rules (Chapter 3), matching (Chapter 8), and component-level layout (Chapters 6-14) converges here at the top-level interconnection stage, where individual circuit blocks are wired together into a complete die. The routing techniques (star nodes, Kelvin connections, electrostatic shielding, electromigration-aware lead sizing) translate circuit-level requirements into physical layout constraints. The final checklist ties together every concern addressed in the book -- ESD (Chapter 5), latchup (Chapter 5), matching (Chapter 8), DRC/LVS (Chapter 3) -- into a single pre-tapeout verification procedure. This section also connects directly to the pad ring and die assembly topics covered in [[ch15-padring]], as bondpads, scribe seals, and corner exclusion zones are all verified in the final checklist.

## See Also
- [[ch15-padring]]
