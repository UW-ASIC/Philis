---
title: "15.3-15.5 Copper Pillars, Manual Top-Level Interconnection, and Final Layout Checklist"
chapter: 15
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-15, interconnection, vias, routing, electromigration, ESD, latchup, noise-coupling]
---

# 15.3 (cont.) Copper Pillars / 15.4 Manual Top-Level Interconnection / 15.5 Final Layout Checklist

> **Chapter 15: Assembling the Die** -- Pages 810-824

These pages conclude Section 15.3's discussion of copper pillar technology, then cover the full manual top-level interconnection methodology (Section 15.4) and the final layout checklist (Section 15.5). Together they form the practical "how to wire up and verify a complete analog die" portion of the book.

---

## Key Concepts

### Copper Pillar Technology (Tail of Section 15.3)

Copper pillars were originally developed by IBM in 2005 under the name **metal-post solder chip connection (MPS-C2)**. They represent a significant evolution beyond solder bumps for flip-chip packaging, especially at fine pitches.

A copper pillar-on-I/O structure resembles a bump-on-I/O, except that the thin copper redistribution layer is replaced with a much thicker copper deposition forming the pillar itself. Key characteristics:

- Pillar height: typically 25-75 $\mu$m tall
- Pillar diameter: typically about 50 $\mu$m, with commercial examples as thin as 25 $\mu$m
- Achievable pitch: as fine as 40 $\mu$m (far finer than solder bumps)
- Capped with deposited solder -- only enough to join to the underlying board/substrate, since the pillar itself provides standoff spacing

**Why copper pillars beat solder bumps:**
1. **Finer pitch** -- the narrow pillar diameter and the fact that less solder is needed enables much tighter spacing than solder bumps (which had trouble below ~130 $\mu$m pitch)
2. **Shape flexibility** -- pillars need not be circular; elongated oval pillars are used for high-current applications
3. **Thermal conductivity** -- copper is about 5x more thermally conductive than solder, making pillars excellent for power ICs where strategic placement can draw heat away from power devices

### Manual Top-Level Interconnection (Section 15.4)

Even as autorouting becomes dominant for digital, small analog ICs are still often manually interconnected faster than they can be autorouted. The section covers vias, routing pitches, channel routing, and special routing techniques.

### Final Layout Checklist (Section 15.5)

A 17-point checklist covering DRC, LVS, electromigration, ESD, latchup, antenna rules, bondpads, Kelvin connections, dummy metal, scribe seals, and more -- the last chance to catch errors before tapeout.

---

## 15.4.1 Via Technologies

Four via technologies are discussed, each with distinct properties that affect how leads can be routed and stacked:

### Aluminum Vias (oldest)
- A hole in the interlevel oxide (ILO) above a lower metal geometry; upper metal is deposited over the ILO
- Upper metal includes a thin **refractory barrier metal (RBM)** for sidewall step coverage, topped with thicker sputtered aluminum alloy
- All vias must have the **same dimensions** (minimum-dimension via arrays for high current)
- **Not stackable** -- nonplanar upper surface means VIA2 cannot sit atop VIA1
- Both upper and lower metal must **symmetrically overlap** the via by several tenths of a micron

### Tungsten Plug Vias
- A hole through the ILO is filled entirely with tungsten, then polished back via **CMP** to a planar surface
- **Stackable** -- VIA2 can sit directly atop VIA1 atop a contact
- CMP planarization eliminates the need for the upper metal to overlap the via on all four sides
- Supports **asymmetric top-metal overlaps**: overlaps on two opposite sides can be smaller than the other two sides (sometimes reduced to zero)

### Single Damascene Copper Vias
- Use tungsten plug vias, but CMP planarization of the lower metal also eliminates the need for the lower metal to fully enclose the via
- **Asymmetric metal overlaps both above and below**
- Stackable

### Dual Damascene Copper Vias
- Created by a different manufacturing process (see Section 2.7.6), but similar properties
- Asymmetric overlaps above and below; stackable

| Technology | Stackable | Lower Metal Overlap | Upper Metal Overlap |
|---|---|---|---|
| Aluminum vias | No | Mandatory (all sides) | Mandatory (all sides) |
| Tungsten plug vias | Yes | Mandatory (2 sides) | Optional |
| Single damascene | Yes | Optional | Optional |
| Dual damascene | Yes | Optional | Optional |

### Stress-Induced Voiding in Vias

Differences in coefficients of thermal expansion between silicon, ILO, and metal generate mechanical stresses. Thermal cycling can cause **voids at interfaces between metal layers within vias**. Void formation is probabilistic -- each via has a small but non-zero chance of being severed. Therefore:

> **Many wafer fabs demand that designers use pairs of vias rather than single vias.** Multiple vias in close proximity have a protective effect beyond mere redundancy -- void formation in one via relieves stresses on the adjacent via.

---

## 15.4.2 Routing Pitches and Grids

Three routing pitch definitions govern how tightly signals can be packed:

### Lead-to-Lead Routing Pitch $P_{LTL}$

The minimum pitch if minimum-width leads can accommodate vias:

$$P_{LTL} = W_m + S_m$$

where $W_m$ is the minimum drawn metal width and $S_m$ is the minimum drawn metal-to-metal spacing.

### Via-to-Via Routing Pitch $P_{VTV}$

Since minimum-width leads are frequently too narrow to accommodate a via, designers widen leads to fit. Attempts to jog metal leads around vias usually create more problems than they solve:

$$P_{VTV} = W_v + 2M_{ov} + S_m$$

where $W_v$ is the drawn via width and $M_{ov}$ is the minimum drawn metal overlap of via (using the smallest overlap allowed for two opposite sides in asymmetric rules).

### Via-to-Lead Routing Pitch $P_{VTL}$

For desperate situations, vias can be staggered:

$$P_{VTL} = \frac{W_v + W_m}{2} + M_{ov} + S_m$$

Staggering creates considerable congestion at routing channel intersections, so most designers prefer $P_{VTV}$.

### Channel Width

The channel width $W_C$ required to accommodate $N$ minimum-width leads:

$$W_C = N \cdot P_r + S_m$$

where $P_r$ is whichever routing pitch the designer has chosen.

### Worked Example (Table 15.4)

For a tungsten-plug aluminum system with MET1 width = 0.3 $\mu$m, MET1 spacing = 0.3 $\mu$m, VIA1 width = 0.25 $\mu$m (exact), MET1 overlap VIA1 (two sides) = 0.05 $\mu$m, MET1 overlap VIA1 (other two sides) = 0.1 $\mu$m:

- $P_{LTL}$ = 0.3 + 0.3 = 0.6 $\mu$m
- $P_{VTV}$ = 0.25 + 2(0.05) + 0.3 = 0.65 $\mu$m

**Grid complications:** Metal lines must be widened to 0.35 $\mu$m to accommodate vias. If the coding increment is 0.05 $\mu$m, the path edges of a 0.35 $\mu$m path fall on a 0.025 $\mu$m grid (not the 0.05 $\mu$m coding grid), causing rounding errors. The path width must be increased to 0.4 $\mu$m, making $P_{VTV}$ = 0.7 $\mu$m.

### Consistent vs. Inconsistent Grids

- **Consistent grid**: both centerlines and edges of paths fall on the grid
- **Inconsistent grid**: only centerlines fall on the grid; edges do not
- Skilled designers sometimes use inconsistent grids for rapid wiring in routing channels, then revert to a smaller consistent grid for interfacing to other geometries

### Orthogonal (Manhattan) vs. Octagonal Routing

Standard routing uses only horizontal and vertical segments (Manhattan routing). Diagonal (45-degree) routing can pack signals more tightly against corner exclusion zones, but raises concerns about off-grid vertices in path outlines. Some layout rules enforce wider widths on diagonal segments.

---

## 15.4.3 Channel Routing

Channel routing requires at least two interconnection layers. The minimum combination is one metal + one polysilicon layer ("one-and-a-half layer metal"):

- **Unsilicided gate poly**: sheet resistance 20-40 $\Omega/\square$
- **Silicided poly**: less than 10 $\Omega/\square$

Most signals tolerate short poly jumpers (especially silicided), but long poly runs create too much resistance. Double-level metal is standard for modern designs.

### Rules for Channel Routing

1. **Alternate metal directions**: Metal-1 runs in one direction (e.g., vertical), Metal-2 runs perpendicular (horizontal). Each additional metal layer alternates. Enforce this across the entire die.
2. **Route wide signals first** (power, ground). Use widths that consume integer multiples of the routing pitch:
$$W_w = N \cdot P_r - S_m$$
where $N$ is an integer, $P_r$ is routing pitch, $S_m$ is metal spacing. For $P_r$ = 0.7 $\mu$m, $S_m$ = 0.3 $\mu$m: widths of 1.0, 1.7, 2.4 $\mu$m, etc.
3. **Work from outside edges inward**, maintaining minimum spacing throughout. Jogs propagate laterally and cause distant congestion.
4. **Fill wide-lead layer transitions with via arrays**: wherever a wide lead changes layers, extend both metals fully across each other and fill the overlap with as many vias as possible.

### Channel Overloading

When a channel runs out of room, options include:
- Rerouting signals through alternate channels
- Running extra leads on the "wrong" metal layer, snaking them to avoid collisions
- Pushing signals onto polysilicon (least likely to cause blockage since leads branching from channels are seldom on poly)

Route all signals that *must* use metal first; reserve poly-tolerant signals for last.

### Primary vs. Secondary Channels

Two or three **primary routing channels** carry a large percentage of signals. They usually intersect near the die center and can become choke points. Conservative rule: each primary channel should hold space for 10-20% of all top-level signals. Primary channels narrow toward the die edges; feeder channels narrow away from primaries. Even the narrowest feeder channel should allow at least 5 signals.

### Signal Color Coding (Table 15.5)

| Signal Type | Color | Label |
|---|---|---|
| High-voltage | Red | Voltage |
| High-current | Yellow | Width |
| High-voltage + high-current | Orange | Voltage, width |
| Sensitive analog | Green | -- |
| Noisy digital | Purple | -- |
| Static digital (can route in poly) | Gray | -- |
| Other (must route in metal) | Light blue | -- |

---

## 15.4.4 Special Routing Techniques

### Star Nodes and Kelvin Connections

**Problem:** Metal lead resistance, though small, is not always negligible. Example: 10 squares of 30 m$\Omega/\square$ metal carrying 1 mA develops 0.3 mV -- enough to upset sensitive matched circuitry.

**Star node solution:** All sensitive leads return to a single common point. Since they share the same point, ground current cannot generate any differential between them. Ground drops elsewhere cause common-mode variation, to which most circuits have high immunity.

**Kelvin connection:** The classic four-wire measurement arrangement (invented by Lord Kelvin in 1861). Two **force leads** ($F_1$, $F_2$) carry large current through a resistor; two **sense leads** ($S_1$, $S_2$) connect to a low-current sensing circuit. Since sense leads carry very little current, almost no voltage drops occur along them. For extreme accuracy, match the currents flowing through the two sense leads and route them with equal resistance so residual drops cancel.

### Noisy Signals and Sensitive Signals

**Noise coupling equation:** If a signal slewing $dV/dt$ volts per second couples through a capacitance $C$ into a sensitive node with resistance $R$:

$$\Delta V = C \cdot R \cdot \frac{dV}{dt}$$

Modern logic gates switch in < 0.1 ns (slew rates > $10^{10}$ V/s). A signal coupling across 10 fF into a 100 k$\Omega$ node generates 1 V of interference.

**Noise-sensitive signals include:**
- Inputs to high-gain amplifiers and precision comparators
- ADC inputs
- Precision voltage reference outputs
- Analog ground lines
- High-value analog resistor networks
- Very low-voltage or very low-current signals
- Current bias lines to accurate analog circuitry

**Mitigation strategies:**
1. **Never route noisy signals on top of, below, or adjacent to sensitive signals**
2. If crossing is unavoidable, cross at **right angles** to minimize overlap capacitance
3. Use **parasitic back-annotation** to simulate coupling and verify acceptable noise levels
4. Insert **electrostatic shields** (a metal-2 square tied to quiet analog ground) between noisy and sensitive signals on adjacent layers; extend the shield 2-5 $\mu$m beyond the intersection area
5. Insert **series resistors** (tens of k$\Omega$) in slow digital lines to reduce slew rate. Example: 1 pF parasitic + series R limits slew to approximately $V_{DD}/(R \cdot C)$
6. Run a **shield lead** (low-noise, low-impedance signal like static digital or extra power/ground) between noisy and sensitive signals on the same layer

**Circuit designers must communicate** which signals are noisy ("_n" suffix) and which are sensitive ("_s" suffix) -- layout designers cannot reliably identify them independently.

### High-Voltage Signals

ILO dielectric strength is lower than gate oxide: typically around 5 MV/cm, translating to ~250 V/$\mu$m. But actual minimum metal spacing may only support about 25 V due to:
- Proximity effects (microloading)
- Optical proximity correction
- Damascene sidewall tapering
- **Low-k dielectrics** (porosity reduces dielectric constant but also reduces dielectric strength)

Solutions:
- Voltage-dependent spacing rules for metal and polysilicon
- Flag high-voltage leads with suffixes like "_hv"
- Voltage-aware DRC tools (e.g., Mentor Graphics Calibre) that extract voltage differentials from the netlist -- works better for digital than analog

### High-Current Signals

Two constraints govern lead width -- **electromigration** and **resistance** -- and they differ in character:

| Concern | Nature | Implication |
|---|---|---|
| Electromigration | Worst-case at every point | Cannot neck down a lead even briefly |
| Resistance | Average over entire length | Can selectively widen where room exists |

**Minimum lead width for electromigration:**

$$W_{min} = \frac{I_{max}}{J_{max} \cdot t_{min}}$$

where $I_{max}$ is the maximum DC current, $J_{max}$ is the allowed current density, and $t_{min}$ is the minimum metal thickness.

Example: 50 mA through 8000 A thick metal with $J_{max} = 3 \times 10^5$ A/cm$^2$ requires minimum width of 21 $\mu$m.

**Temperature derating (Black's Law):**

$$D = \exp\left[\frac{E_a}{k} \left(\frac{1}{T_d} - \frac{1}{T_r}\right)\right]^{1/n}$$

where:
- $E_a$ = activation energy (0.5 eV for pure Al, ~0.7 eV for Cu-doped Al)
- $k$ = Boltzmann's constant ($8.62 \times 10^{-5}$ eV/K)
- $T_r$ = reference temperature (K), $T_d$ = desired operating temperature (K)
- $n$ = current exponent (~2 for Al, ~1 for Cu)

Example: derating from 100 C to 125 C (398 K) gives $D$ = 0.58, so a lead safe for 25 mA at 100 C handles only 15 mA at 125 C.

**Copper vs. Aluminum electromigration:**
- Copper has much higher $J_{max}$ at low temperatures (85-105 C)
- But copper's current exponent $n \approx 1$ vs. aluminum's $n \approx 2$, reducing copper's advantage at higher temperatures
- Parity occurs around 200 C; above that, aluminum is actually better
- Copper's lower exponent is largely due to inability to passivate its upper surface

**Via placement and current crowding:**
- Current crowds toward the inside corner at 90-degree bends
- Replace 90-degree interior angles with pairs of 135-degree angles
- Via arrays at corners see drastically nonuniform current: inside-corner via sees highest stress, outside-corner via sees least
- **Place via arrays in straight sections of leads** for uniform current distribution across all vias

---

## 15.5 Final Layout Checklist (17 Points)

This checklist represents the last opportunity to catch errors before tapeout:

1. **DRC diagnostics** -- All remaining diagnostics reviewed and waived with process engineering
2. **LVS topological errors = ZERO** -- Cannot be waived; the presence of one error can confuse the LVS algorithm and mask others
3. **LVS parametric errors** -- Reviewed and waived (e.g., guard ring area mismatches at top level vs. subcell level)
4. **Electromigration compliance** -- Power wire widths verified; power plots examined for pinch points and inadequate via counts
5. **High-current lead resistances** -- Verified against design specifications; finite-element tools (e.g., Silicon Frontline R3D) useful for irregular devices
6. **Antenna violations corrected** -- All violations fixed by jumpers or antenna diodes
7. **ESD compliance** -- Primary ESD structures reviewed; CDM clamps checked for proximity and connection adequacy; cross-domain signals inspected
8. **Latchup checks** -- Vulnerable diffusions (connected to pins through < ~1 k$\Omega$) protected by minority carrier guard rings or strategic contacts
9. **Dummy metal block zones** -- Present for matched devices (especially MOS transistors) not protected by metal field plates
10. **Noisy/sensitive signal separation** -- Routing checked; electrostatic shielding verified where needed
11. **Bondpads and probe pads** -- Properly sized, located, and spaced; pad #1 identification marks present
12. **Kelvin connections and star nodes** -- Properly located; resistors ensuring separate force/sense signal names present
13. **Corner exclusion zones and metal slotting** -- Implemented if required by process
14. **Scribe seals and scribe streets** -- Present, properly constructed, correct width, joined to adjacent ground metallization
15. **$P^-$ substrate contacts** -- Empty die areas filled with substrate contacts (or bypass capacitance for digital-heavy designs)
16. **Die symbolization** -- Part number, company logo, mask work notice (circled "M" + date) present
17. **Layout properly archived**

---

## Diagrams

### Via Technologies (Figure 15.21)
![[diagrams/ch15-padring-fig1.png]]
Cross sections of four via types: aluminum via (A), tungsten plug via (B), single damascene tungsten-plug via (C), and dual damascene copper via (D). Note the progression from non-planar (aluminum, requiring symmetric overlap) to fully planar CMP-processed vias (damascene, permitting asymmetric or zero overlap). The tungsten plug via shows the CMP-polished surface that enables stacking.

### Noise Coupling and Electrostatic Shielding (Figure 15.28)
![[diagrams/ch15-padring-fig2.png]]
Two shielding techniques: (A) A metal-2 shield square placed between a noisy metal-3 signal and a sensitive metal-1 signal -- the shield must tie to a quiet low-impedance node (ideally the reference for the sensitive signal) and extend 2-5 $\mu$m beyond the intersection area. (B) A shield lead run on the same layer between a noisy signal and a sensitive signal. The page also covers the noise coupling equation $\Delta V = C \cdot R \cdot dV/dt$ and lists noise-sensitive signal categories.

### Electromigration and Via Arrays (Figure 15.29)
![[diagrams/ch15-padring-fig3.png]]
Via arrays placed at a 90-degree bend (A) vs. in a straight segment (B). At the bend, the inside-corner via (#1) sees the highest electromigration stress while the outside-corner via (#8) sees the least. Placing vias in a straight section equalizes current density across all vias in each column. This page also discusses Black's Law derating and copper vs. aluminum electromigration behavior.

---

## Practical Takeaways

- **Always use pairs (or more) of vias** rather than single vias to protect against stress-induced voiding -- the protective effect goes beyond simple redundancy
- **Choose via-to-via routing pitch** ($P_{VTV}$) over via-to-lead pitch ($P_{VTL}$) to avoid congestion at channel intersections
- **Verify the coding grid** before routing: if metal widths to accommodate vias don't align with the coding increment, you will get rounding errors during pattern generation
- **Route wide power/ground signals first**, using widths that are integer multiples of the routing pitch minus the metal spacing
- **Work from channel edges inward** and maintain minimum spacing everywhere -- jogs propagate laterally and cause distant congestion
- **Primary routing channels** should hold 10-20% of all top-level signals; even the narrowest feeder channel should allow at least 5 signals
- **Kelvin connections** are essential for accurate sensing of small voltages: route sense leads with matched resistance and minimal current
- **Never route noisy signals adjacent to, above, or below sensitive signals.** If crossing is unavoidable, cross at right angles and consider electrostatic shielding
- **Insert series resistors** in slow digital lines to reduce slew rate and noise injection into analog circuitry
- **Electromigration rules apply at every point** along a lead -- you cannot neck down even briefly. Resistance is an average, so you can selectively widen where room exists
- **Place via arrays in straight lead sections**, not at bends, to equalize current density
- **Replace 90-degree bends** with pairs of 135-degree angles to reduce current crowding
- **Copper metallization** has higher $J_{max}$ at low temperatures but loses its advantage above ~200 C due to its current exponent of ~1 (vs. ~2 for aluminum)
- **Run the full 17-point checklist** before tapeout -- zero topological LVS errors is non-negotiable

---

## Relation to the Bigger Picture

These sections form the capstone of the physical design process described throughout the book. After individual devices are designed (Chapters 6-11), matched (Chapter 8), protected against failure mechanisms (Chapter 5), and assembled into circuit blocks ([[ch15-floorplanning]]), the designer must physically wire everything together ([[ch15-interconnection]]) and verify the complete die before tapeout. The routing techniques here -- channel routing, Kelvin connections, noise shielding, electromigration-aware lead sizing -- are where all the analog layout knowledge from earlier chapters converges into a working integrated circuit. The final layout checklist ties together concerns from ESD (Chapter 14), latchup (Chapter 5), matching (Chapter 8), and process rules (Chapter 3) into a single verification pass, underscoring that analog layout is ultimately a holistic discipline where every detail must be correct simultaneously.

---

## See Also
- [[ch15-floorplanning]]
- [[ch15-interconnection]]
