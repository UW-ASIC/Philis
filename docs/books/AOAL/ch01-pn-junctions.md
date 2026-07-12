---
title: "1.2 PN Junctions"
chapter: 1
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-1, pn-junction, diodes, depletion-region, schottky, zener, ohmic-contact]
---

# 1.2 PN Junctions

> **Chapter 1: Device Physics**

## Key Concepts

### The Metallurgical Junction and Equilibrium

Uniformly doped semiconductors have few practical applications. Almost all solid-state devices consist of multiple P- and N-type regions. The interface between a P-type region and an N-type region is called a **PN junction** (or simply a *junction*). The physical surface of contact between the two doping polarities is called the **metallurgical junction**. Since dopant atoms cannot move at ordinary temperatures, the metallurgical junction remains stationary once formed.

When P-type and N-type silicon are brought into contact, two simultaneous diffusion processes begin:

1. **Holes diffuse** from the P-type side (where they are abundant) to the N-type side (where they are few). This leaves a localized *negative* charge in the P-type region and creates a localized *positive* charge in the N-type region. The holes are minority carriers in N-type silicon and quickly recombine.
2. **Electrons diffuse** from the N-type side to the P-type side. This leaves a localized *positive* charge in the N-type region and deposits a localized *negative* charge in the P-type region. These electrons recombine as minority carriers.

Both processes reduce the concentration of majority carriers near the junction. The P-type side acquires a net negative charge, and the N-type side acquires a net positive charge. This generates an **electric field** across the junction, which in turn drives **drift currents** that oppose the diffusion currents.

The system reaches **equilibrium** when the hole drift current exactly equals and opposes the hole diffusion current, and likewise for electrons. The voltage difference across the junction at equilibrium is called the **built-in potential** $V_{bi}$, which equals approximately $0.6\text{--}0.7\,\text{V}$ for moderately doped silicon PN junctions at room temperature. The built-in potential biases the P-type silicon negative with respect to the N-type silicon.

**Crucially, $V_{bi}$ cannot be measured with a voltmeter.** The terminals of a PN diode consist of metal in contact with silicon, and contact potentials appear at these metal-silicon interfaces. As long as all contacts and junctions are at the same temperature, the sum of the contact potentials $V_1$ and $V_2$ plus the built-in potential $V_{bi}$ exactly equals zero (by thermodynamics). However, if temperature gradients exist, the potentials no longer cancel and a net voltage appears -- this is the **Seebeck effect** (thermoelectric effect).

---

## 1.2.1 Depletion Regions

The intense electric field across the junction sweeps mobile charges out of its region of influence. Electrons drift to the N-type side, holes drift to the P-type side, and the region left behind becomes **depleted of carriers** -- hence the name **depletion region** (older term: *space charge layer*, SCL).

### Key properties of depletion regions:

- **Width depends on doping**: The acceptor charge in the P-type silicon must equal the donor charge in the N-type silicon (charge neutrality). Therefore, the depletion region extends much further into the **lightly doped** side. For example, if P-type silicon is doped at $10^{15}\,\text{cm}^{-3}$ and N-type at $10^{17}\,\text{cm}^{-3}$, the depletion region extends ~100x further into the P-type side. In practice, unequal doping is so disproportionate that **almost the entire width of the depletion region lies in the lightly doped side**.
- **Numerical example**: For a junction uniformly doped at $\sim 10^{17}\,\text{cm}^{-3}$ on each side, the depletion region extends about $0.1\,\mu\text{m}$ on either side of the metallurgical junction, with the electric field rising linearly from ~zero at the edges to a peak of $\sim 20\,\text{kV/cm}$ at the center.
- **Heavier doping** reduces the width but intensifies the electric field.

### Carrier transit through the depletion region

Although carriers cannot dwell in the depletion region, they can **transit across it**. Due to statistical variations, a few carriers possess high enough instantaneous velocity to surmount the electric field and diffuse across. These become **excess minority carriers** on the other side. The depletion region remains almost entirely depleted because so few carriers are caught crossing at any instant.

In equilibrium:
- Hole drift current = hole diffusion current (in opposing directions)
- Electron drift current = electron diffusion current (in opposing directions)
- The hole and electron currents do **not** necessarily equal each other (they depend on relative doping levels)

---

## 1.2.2 PN Diodes

A PN junction with terminals added forms a **PN diode**. The **anode** connects to the P-type side; the **cathode** connects to the N-type side.

### Zero Bias

Under zero bias (assuming no temperature differences), the sum of the built-in potential and the two contact potentials equals zero (Kirchhoff's voltage law). The junction remains in equilibrium: all drift and diffusion currents cancel, so the diode conducts **zero current**.

### Reverse Bias (Anode negative w.r.t. Cathode)

- Holes in the anode drift toward the anode contact and recombine with electrons from the metal. The depletion region extends further into the anode.
- Electrons in the cathode flow out through the cathode contact into the external circuit. The depletion region extends further into the cathode.
- The depletion region widens until the increased voltage across the junction counterbalances the externally applied voltage. Current flow essentially ceases.
- A minute **leakage current** flows, caused by thermal generation of electron-hole pairs within the depletion region. Each generated pair produces one electron flowing through the external circuit.
- Leakage current **doubles approximately every $10\,^{\circ}\text{C}$** and becomes objectionably large at very high temperatures.
- Silicon ICs are typically designed for maximum junction temperatures of $125\text{--}150\,^{\circ}\text{C}$; some discrete devices can operate up to $250\,^{\circ}\text{C}$ or even $275\,^{\circ}\text{C}$.
- Wide-bandgap materials (e.g., SiC) have lower thermal generation rates and can therefore operate at even higher temperatures.

### Forward Bias (Anode positive w.r.t. Cathode)

- Electron-hole generation occurs at the anode contact; newly generated holes add to the anode's hole population.
- The depletion region **shrinks** on both sides as majority carriers fill in from the contacts.
- The electric field diminishes, reducing drift currents while diffusion currents remain unchanged. This imbalance produces a net current flow from anode to cathode.
- The current increases **exponentially** with forward bias voltage $V_F$:

$$I = I_S \left( e^{V_F / V_T} - 1 \right)$$

where $I_S$ is the reverse saturation current and $V_T = kT/q \approx 26\,\text{mV}$ at room temperature.

- As $V_F$ approaches $V_{bi}$, the resistance of the undepleted silicon (anode and cathode bulk) becomes significant, and the I-V curve transitions from exponential to a linear asymptote.

### Forward voltage values (practical)

| Condition | Typical $V_F$ |
|---|---|
| Milliamp-level currents (discrete diode, $25\,^{\circ}\text{C}$) | $\sim 0.65\,\text{V}$ |
| Microamp-level currents (integrated circuits) | $0.4\text{--}0.5\,\text{V}$ |

### Temperature dependence

- The forward bias required to sustain a constant current decreases by approximately $-2\,\text{mV}/^{\circ}\text{C}$.
- Example: if a certain current produces $V_F = 0.65\,\text{V}$ at $25\,^{\circ}\text{C}$, the same current produces $\sim 0.55\,\text{V}$ at $75\,^{\circ}\text{C}$ and $\sim 0.45\,\text{V}$ at $125\,^{\circ}\text{C}$.
- This predictable temperature coefficient makes forward-biased PN diodes useful as **temperature sensors**.

---

## 1.2.3 Zener Diodes

A reverse-biased PN diode conducts only a small leakage current until the reverse bias exceeds a critical voltage, at which point the current increases exponentially. This is **reverse breakdown**. If the external circuit limits the current, the diode survives and can provide an extremely stable voltage reference.

### Two breakdown mechanisms:

#### 1. Avalanche Multiplication

- Under increasing reverse bias, the electric field in the depletion region accelerates carriers to high velocities.
- Electrons, having lower effective mass, accelerate more quickly and gain more kinetic energy.
- Eventually, the fastest electrons have enough energy to knock a valence electron out of the lattice -- **impact ionization** -- creating a new electron-hole pair.
- These new carriers are also accelerated, creating more pairs via further impact ionization. A single carrier crossing the depletion region can spawn **thousands** of additional carriers.
- **Critical field** in silicon: $\sim 3 \times 10^5\,\text{V/cm}$ at $10^{15}\,\text{cm}^{-3}$ to $\sim 10^6\,\text{V/cm}$ at $10^{18}\,\text{cm}^{-3}$.
- Higher doping increases lattice collision rate, reducing acceleration time, and thus raising the critical field. But heavier doping also narrows the depletion region, so heavily doped junctions break down at **much lower voltages** than lightly doped ones.
- Avalanche breakdown voltages: a few volts (heavy doping) to hundreds or thousands of volts (light doping).
- **Temperature coefficient**: positive (breakdown voltage increases with temperature due to more frequent lattice collisions). Example: $\sim +2\,\text{mV}/^{\circ}\text{C}$ for a $6.2\,\text{V}$ avalanche diode.

#### 2. Zener Effect (Quantum Tunneling)

- Occurs in **very heavily doped** PN diodes where the depletion region is extremely thin (a few nanometers).
- Electrons tunnel through the thin barrier -- a quantum mechanical process where lighter particles (electrons) can traverse short distances regardless of obstacles.
- **Temperature coefficient**: negative (breakdown voltage decreases with temperature because more valence electrons are available at higher temperatures).

### The crossover point

In silicon diodes, Zener and avalanche conduction currents are equal at a breakdown voltage of approximately **5 V**:
- $V_{BR} < 5\,\text{V}$: Zener effect dominates
- $V_{BR} > 5\,\text{V}$: Avalanche multiplication dominates

Engineers traditionally call all breakdown diodes "Zeners" regardless of the mechanism, which can be confusing -- a $6.2\,\text{V}$ "Zener diode" actually conducts primarily by avalanche breakdown.

---

## 1.2.4 Schottky Diodes

Rectifying junctions can also form between a **semiconductor and a metal**. These are called **Schottky barriers**, and devices built from them are **Schottky diodes**.

### Formation of a Schottky barrier

Consider aluminum in contact with lightly doped N-type silicon:
- Naively, one might expect electrons to diffuse from aluminum (many electrons) into the N-silicon (fewer electrons), accumulating at the interface.
- In reality, **the opposite happens**: a large electric field biases the aluminum positively with respect to the N-type silicon, creating a drift current that transports electrons **from the silicon into the aluminum**.
- The departure of electrons from the N-silicon creates a **depletion region** adjacent to the Schottky barrier.
- Electrons arriving in the aluminum form a thin layer of negative charge at the interface.
- As charge accumulates, the drift current diminishes until equilibrium is reached.
- The resulting voltage drop is the **built-in potential** of the Schottky barrier.

### Reverse Bias

Biasing the cathode (N-silicon) positive with respect to the anode (metal) widens the depletion region. Drift and diffusion balance again; only leakage current flows.

### Forward Bias

Biasing the cathode negative with respect to the anode narrows the depletion region, reducing the drift current magnitude while diffusion current stays constant. Net electron flow occurs from cathode to anode, producing conventional current from anode to cathode. Current increases exponentially with voltage, but **most practical Schottky diodes have lower forward voltages** than PN diodes.

### Key difference: Majority-carrier vs. Minority-carrier device

| Property | PN Diode | Schottky Diode |
|---|---|---|
| Carrier type | Minority-carrier device | **Majority-carrier device** |
| Switching speed | Limited by minority carrier recombination | **Much faster** (no stored minority charge) |
| Forward voltage | Higher (~0.6-0.7 V) | **Lower** (depends on metal/doping) |
| Typical anode material | P-type silicon | Platinum silicide or palladium silicide |

The absence of stored minority carriers means Schottky diodes can switch at **substantially higher speeds** than PN diodes.

### Materials used in IC Schottky diodes

Lightly doped N-type silicon forms rectifying Schottky barriers with many metals and metallically bonded compounds. In integrated circuits, Schottky diodes typically use:
- **Platinum silicide** (PtSi)
- **Palladium silicide** (PdSi)

The N-type silicon forms the cathode; the silicide forms the anode.

### Important: Not all Schottky barriers rectify

Certain metal-semiconductor combinations create an electric field that causes majority carriers to **accumulate** at the silicon surface instead of depleting. These non-rectifying Schottky barriers behave as **Ohmic contacts** (see next section).

---

## 1.2.5 Ohmic Contacts

Contacts must be made between metals and semiconductors to connect devices into circuits. Ideally these would be perfect conductors; in practice they are **Ohmic contacts** exhibiting a small resistance. Unlike rectifying contacts, Ohmic contacts conduct current **equally well in either direction**.

### Two mechanisms for Ohmic contact formation:

#### 1. Tunneling through a thin depletion region

A rectifying Schottky barrier can become Ohmic if the semiconductor is **doped heavily enough**. The high dopant concentration thins the depletion region to the point where carriers can easily **tunnel across it** in either direction, even at extremely low voltages. Rectification is bypassed entirely.

#### 2. Majority carrier accumulation

If the voltage across the Schottky barrier causes majority carriers to **accumulate** at the semiconductor surface (rather than deplete), a thin charge layer forms at the interface. Without a depletion region, no voltage differential can be sustained, and any applied voltage sweeps carriers across the barrier in either direction.

### Practical rule of thumb

| Silicon Doping | Contact Type |
|---|---|
| Lightly doped | **Rectifying** Schottky barrier |
| Heavily doped | **Ohmic** contact |

A lightly doped region can be Ohmically contacted by placing a thin layer of **more heavily doped silicon of the same polarity** beneath the contact. Contact resistances of less than $10^{-6}\,\Omega\cdot\text{cm}^2$ can be achieved with a heavily doped silicon layer and a suitable metal system -- small enough to be neglected for most applications.

### Seebeck coefficient of contacts

All interfaces between dissimilar materials exhibit a contact potential. If all contacts and junctions in a circuit are at the same temperature, their contact potentials sum to zero. However, contact potentials are **strong functions of temperature**. The change in contact potential with temperature is the **Seebeck coefficient**, typically $\sim 0.1\text{--}1.0\,\text{mV}/^{\circ}\text{C}$ for silicon contacts. Since many analog ICs depend on voltages matching within a millivolt or two, even small temperature differentials from self-heating can degrade operation.

---

## Schematic Symbols

The default diode symbol uses a straight line (cathode) and an arrowhead (anode), indicating the direction of conventional current flow when forward biased. Modifications to the cathode bar distinguish different diode types:

- **PN diode**: Plain bar cathode
- **Schottky diode**: Cathode bar with bent ends (resembling an "S")
- **Zener diode**: Cathode bar with angled ends (resembling a "Z")

Note: The Zener symbol arrow can appear misleading because Zeners normally operate in *reverse* bias, so the symbol may seem "the wrong way around."

---

## Diagrams

### Figure 1.8 -- Formation of a PN Junction

![[diagrams/ch01-pn-junctions-fig1.png]]

*Hypothetical steps in the formation of a PN junction. (A) Two separate pieces of P-type and N-type silicon are brought into contact. (B) Holes diffuse across the metallurgical junction. (C) Holes recombine, leaving a negative charge in the P-side. (D) Electrons diffuse across the junction. (E) Electrons recombine, creating the depletion region. In reality, steps B--E occur simultaneously. Note the depletion region straddling the metallurgical junction, depleted of mobile carriers.*

### Figure 1.13 -- Carrier Flow in a Forward-Biased PN Junction & Figure 1.14 -- I-V Characteristic

![[diagrams/ch01-pn-junctions-fig2.png]]

*Top: Carrier flow in a forward-biased PN junction showing holes flowing from P+ anode through the depletion region to N- cathode, recombining with electrons. Arrow widths indicate relative current magnitudes. Bottom: The I-V characteristic of a PN diode showing exponential forward conduction, the forward voltage $V_F$, and the small reverse saturation current.*

### Figure 1.15 -- Idealized Schottky Diode Cross-Section

![[diagrams/ch01-pn-junctions-fig3.png]]

*Cross-section of an idealized Schottky diode. Aluminum metal on the left contacts lightly doped N-type silicon on the right. A thin film of electrons accumulates at the metal-silicon interface, and a depletion region forms in the N-silicon adjacent to the barrier. Unlike a PN diode, there is no P-type region -- the junction is between a metal and a semiconductor.*

---

## Practical Takeaways

- **Depletion region width is dominated by the lightly doped side.** When designing junctions, the lighter-doped region determines the physical extent of the depletion region and thus the breakdown voltage and parasitic capacitance.
- **Leakage current doubles every ~10 degrees C.** This is critical for high-temperature IC design; maximum junction temperatures are typically capped at 125--150 degrees C for silicon ICs.
- **Forward voltage temperature coefficient of approximately $-2\,\text{mV}/^{\circ}\text{C}$** is both a problem (voltage shifts with temperature) and an opportunity (temperature sensing).
- **Schottky diodes switch faster than PN diodes** because they are majority-carrier devices with no minority-carrier storage. Use them where switching speed matters.
- **Ohmic contacts require heavily doped silicon** beneath the metal contact. Always ensure contact regions are doped heavily enough to avoid inadvertently creating rectifying Schottky barriers.
- **The Seebeck effect matters for precision analog.** Even small temperature gradients across contacts or junctions produce thermoelectric voltages that can degrade matching in precision circuits. Self-heating is a significant concern.
- **Zener diodes below ~5 V** operate by tunneling (true Zener effect); **above ~5 V** they operate by avalanche multiplication. The two mechanisms have opposite temperature coefficients, which matters for voltage reference design.
- **Contact potentials cancel at uniform temperature** but shift when temperature gradients exist. Layout must minimize thermal gradients across matched devices.
- **A 6.2 V "Zener"** is actually an avalanche diode -- the naming convention is a historical artifact that can confuse.

---

## Relation to the Bigger Picture

Section 1.2 provides the foundational physics for nearly every device discussed in the rest of the book. PN junctions form the basis of bipolar transistor operation ([[ch01-bipolar-transistors]]), where two back-to-back junctions create a device with gain. The depletion region concepts introduced here directly govern junction capacitance (critical for Chapter 7 on capacitors), breakdown voltage (essential for Chapter 5 on failure mechanisms), and isolation between devices on an IC (Chapter 4 on process integration). Understanding Ohmic contacts versus Schottky barriers is essential for proper contact design in layout, and the Seebeck effect discussion foreshadows the detailed treatment of thermal matching in Chapter 8. The semiconductor fundamentals from [[ch01-semiconductors]] (doping, carriers, drift, and diffusion) are directly applied here to explain junction behavior.

---

## See Also
- [[ch01-semiconductors]]
- [[ch01-bipolar-transistors]]
