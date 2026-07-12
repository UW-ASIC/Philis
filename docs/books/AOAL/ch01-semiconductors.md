---
title: "1.1 Semiconductors"
chapter: 1
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-1, semiconductors, device-physics, carrier-transport]
---

# 1.1 Semiconductors

> **Chapter 1: Device Physics**

## Key Concepts

### Why Semiconductors Exist: The Covalent Bond Story

The distinction between metals, semiconductors, and nonmetals arises from **electronic structure** -- specifically, how atoms fill their valence shells and what type of bonding results.

- **Metals** (e.g., sodium) use **metallic bonding**: atoms discard valence electrons, which wander freely through the lattice. This explains high electrical/thermal conductivity and metallic luster.
- **Nonmetals** (e.g., chlorine) use **ionic** or **covalent bonding**: electrons are tightly held, yielding poor conductivity.
- **Semiconductors** (e.g., silicon, germanium) use **covalent bonding** but with bonds weak enough that thermal energy can occasionally rupture them, freeing carriers. This places their conductivity between metals and insulators.

Silicon has **four valence electrons** (group IV). Each silicon atom shares one electron pair with each of four neighboring atoms, forming a **macromolecular crystal** held together by covalent bonds. The crystal is a single huge molecule -- strong, hard, high melting point ($1410\degree C$), and a very poor conductor at low temperatures because nearly all valence electrons are locked in bonds.

### The Bandgap: The Energy Barrier to Conduction

The **bandgap energy** ($E_g$) is the energy required to free a valence electron from the crystal lattice. It determines how easily thermal vibrations can rupture covalent bonds and generate carriers:

| Element | Atomic Number | Melting Point ($\degree C$) | Bandgap Energy ($eV$) |
|---------|:---:|:---:|:---:|
| Carbon (diamond) | 6 | 3550 | 5.2 |
| Silicon | 14 | 1410 | 1.1 |
| Germanium | 32 | 937 | 0.7 |
| Tin (gray) | 50 | 232 | 0.1 |

For context, the average thermal energy per degree of freedom at $25\degree C$ is about $0.013\;eV$, and a free electron moving in three dimensions has an average thermal energy of about $0.039\;eV$. Since silicon's bandgap ($1.1\;eV$) is far larger than this average, only very rare, energetic lattice vibrations can rupture bonds -- which is why intrinsic silicon has very low conductivity.

**Key insight:** Larger bandgap means stronger covalent bonds, fewer thermally generated carriers, and lower intrinsic conductivity. This is why diamond is an insulator, silicon is a semiconductor, and tin is nearly metallic.

---

## Important Details

### 1.1.1 Generation and Recombination

#### Thermal Generation

Heat is kinetic energy distributed randomly among particles. In a silicon crystal, atoms vibrate as covalent bonds stretch and compress like tiny springs. Occasionally, many vibrations combine constructively to produce a disturbance energetic enough to **rupture a covalent bond**, freeing an electron.

When an electron escapes the lattice:
- The freed electron becomes a **free carrier** that can conduct electricity.
- The atom it left behind now lacks one valence electron, creating a **hole** -- a mobile electron vacancy carrying an effective positive charge.
- The hole moves through the lattice by "jumping" from atom to atom as neighboring electrons fill the vacancy.

**Carriers are always generated in pairs** -- every freed electron simultaneously creates one hole. This is called **electron-hole pair generation**.

#### Carrier Behavior in Electric Fields

- **Electrons** drift toward positive potentials.
- **Holes** drift toward negative potentials (like bubbles in liquid -- they move opposite to the electron flow direction).
- **Both** contribute to conventional current in the same direction. If 1 A of electron current and 1 A of hole current flow through a crystal, the total conventional current is 2 A.

#### Mobility

The rate at which carriers move is quantified by **mobility** ($\mu$). In bulk silicon at $25\degree C$:
- Electron mobility: $\mu_n \approx 1350\;cm^2/V \cdot s$
- Hole mobility: $\mu_p \approx 480\;cm^2/V \cdot s$

Electrons are roughly $2.8\times$ more mobile than holes. This is why **N-channel devices switch faster** than P-channel devices, all else being equal.

#### Optical Generation

Photons with wavelengths shorter than about $1100\;nm$ carry enough energy to generate electron-hole pairs in silicon. Visible light ($390$--$700\;nm$) can therefore generate carriers. This is the basis for:
- **Solar cells** (useful optical generation)
- **Circuit malfunctions** in bare/chip-scale packaged ICs exposed to intense light (parasitic optical generation -- camera flashes have been documented to crash bare-die circuits like the Raspberry Pi 2)

#### Recombination Mechanisms

Carriers recombine in pairs, annihilating one electron and one hole:

1. **Radiative recombination** (direct-bandgap semiconductors like GaAs): An electron falls into a hole, releasing energy as a **photon**. This is the operating principle of LEDs and semiconductor lasers. The emitted wavelength depends on $E_g$.

2. **Shockley-Read-Hall (SHR) recombination** (indirect-bandgap semiconductors like Si, Ge): Recombination requires both energy and momentum changes. A photon alone cannot carry away the momentum. Instead, recombination occurs at **traps** -- crystal defects or foreign atoms that distort the lattice. The trap captures one carrier, making it vulnerable to recombination. Energy is released as **heat**, not light.

#### Carrier Lifetime and Recombination Centers

- **Carrier lifetime** ($\tau$): the average time between a carrier's generation and its recombination. Ranges from nanoseconds to hundreds of microseconds.
- **Recombination centers**: traps that accelerate recombination. Gold atoms are highly efficient recombination centers in silicon, sometimes deliberately added to increase switching speed in high-speed diodes and bipolar transistors.
- **Tradeoff**: Enhanced recombination degrades other device properties (e.g., bipolar transistor gain $\beta$).
- Many **transition metals** (Fe, Ni) also act as recombination centers, which is why semiconductor fabrication demands extraordinary material purity.

### 1.1.2 Extrinsic Semiconductors

Intrinsic (pure) semiconductors have impractically low conductivity. Practical devices use **extrinsic** (doped) semiconductors, where carefully controlled impurities (**dopants**) dramatically increase carrier concentrations.

#### N-type Doping (Donors)

A **group-V element** (e.g., phosphorus, arsenic, antimony) substitutes for a silicon atom in the lattice. It has **five** valence electrons -- four form covalent bonds with neighbors, and the fifth has no room in the valence shell. Thermal vibrations easily eject this extra electron, which becomes a free carrier.

- Each donor atom contributes **one free electron** and becomes a **fixed positive ion** (immobile, not a hole).
- The semiconductor remains **electrically neutral** overall.
- **Majority carriers**: electrons. **Minority carriers**: holes.
- Notation: $N^+$ or $N^{++}$ for heavy doping; $N^-$ for light doping.
- Below about $-40\degree C$ to $-50\degree C$, dopants "freeze out" and become ineffective.

#### P-type Doping (Acceptors)

A **group-III element** (e.g., boron) substitutes for silicon. It has only **three** valence electrons, leaving one incomplete covalent bond -- creating a **hole**. The hole is mobile; the boron atom becomes a **fixed negative ion**.

- Each acceptor atom contributes **one free hole**.
- **Majority carriers**: holes. **Minority carriers**: electrons.
- Notation: $P^+$ or $P^{++}$ for heavy doping; $P^-$ for light doping.

| Property | N-type | P-type |
|----------|--------|--------|
| Dopant type | Donors | Acceptors |
| Practical dopants (Si) | P, As, Sb | B |
| Majority carriers | Electrons | Holes |
| Minority carriers | Holes | Electrons |

#### Counterdoping

A semiconductor's type is determined by whichever dopant is **in excess**. You can invert P-type to N-type by adding more donors than acceptors already present (and vice versa). This is the foundation of all modern IC fabrication: selectively counterdoping silicon to form nested P- and N-type regions.

#### Compound Semiconductors

When counterdoping is taken to an extreme with equal group-III and group-V atoms, you get **III-V compound semiconductors** (e.g., GaAs, GaN, InSb). Key properties:
- Many are **direct-bandgap** -- useful for LEDs, lasers, and solar cells.
- GaAs enables very high-speed devices and ICs.
- GaN is used in fast switching power transistors.
- **II-VI compounds** (e.g., CdS for photocells, HgCdTe for IR detectors) and **IV-IV compounds** (e.g., SiC for fast power rectifiers) also exist.
- Manufacturing difficulties have limited compound semiconductors in mainstream IC production. This text focuses on **silicon devices**.

### 1.1.3 Diffusion and Drift

Carriers move through semiconductors via two fundamental mechanisms:

#### Diffusion

Carriers behave as particles undergoing random thermal motion, colliding with lattice vibrations and dopant atoms. In a **uniform** carrier distribution, random motion produces zero net current (equal numbers move in every direction).

When a **concentration gradient** exists, more carriers move away from the high-concentration region than toward it, producing a net **diffusion current**:
- Diffusion moves carriers from **high** concentration to **low** concentration.
- Without replenishment, diffusion current asymptotically approaches zero as the distribution equalizes.
- Diffusion current is proportional to the **concentration gradient**.

*Analogy*: A drop of food coloring in still water gradually spreads throughout the glass -- dye molecules diffuse from areas of high concentration to low concentration. Carriers behave identically but much faster due to their tiny mass.

#### Drift

An applied electric field superimposes a small **systematic velocity** on top of the random thermal motion. Even though the field barely affects the instantaneous speed of any individual carrier (the drift velocity increment per collision is tiny compared to thermal velocity), it consistently pushes carriers in one direction:

- Electrons drift toward **positive** potentials.
- Holes drift toward **negative** potentials.
- The resulting net current is called the **drift current**.
- For low-to-moderate fields ($< 5\;kV/cm$ in uniformly doped Si), drift current is proportional to the electric field -- this is **Ohm's law**.

#### Velocity Saturation and Hot Carriers

At high electric fields ($> 5\;kV/cm$):
- Carriers gain enough energy between collisions to exceed the typical thermal velocity -- these are called **hot carriers**.
- The drift velocity no longer increases linearly but instead **saturates** at a limiting value (the **saturation velocity**).
- This causes resistors to deviate from Ohm's law under large applied voltages.

#### Factors Affecting Carrier Mobility

| Factor | Effect on Mobility |
|--------|-------------------|
| Higher temperature | Decreases (more energetic lattice vibrations, more frequent collisions) |
| Higher doping | Decreases (more collisions with dopant atoms) |
| High electric field | Apparent decrease (velocity saturation) |
| Near crystal surface | Decreases (surface scattering); surface mobility < bulk mobility |

#### Summary of Carrier Transport

> Diffusion happens wherever **concentration gradients** exist. Drift happens wherever **electric fields** exist. Both can occur simultaneously. The **total current** equals the sum of drift and diffusion currents.

---

## Diagrams

### Silicon Crystal Lattice (Figure 1.2)

![[diagrams/ch01-semiconductors-fig1.png]]
*Simplified two-dimensional representation of the silicon crystal lattice. Each small circle is a silicon atom; each line between atoms represents a covalent bond (a shared electron pair). Every Si atom claims four shared pairs for a total of eight valence electrons. The entire crystal is essentially one enormous covalently-bonded molecule.*

### Phosphorus Donor in Silicon (Figure 1.5)

![[diagrams/ch01-semiconductors-fig2.png]]
*A phosphorus atom (group V, five valence electrons) substituting for silicon in the lattice. Four electrons form covalent bonds with neighbors; the fifth is ejected as a free electron. The phosphorus atom becomes a fixed positive ion -- it is NOT a hole because its valence shell is full. This page also shows boron (group III, acceptor) doping in Figure 1.6.*

### Diffusion vs. Drift (Figure 1.7)

![[diagrams/ch01-semiconductors-fig3.png]]
*Comparison of carrier transport mechanisms. (A) Pure diffusion: the carrier undergoes random walk with no net displacement. (B) Drift superimposed on diffusion: an applied electric field adds a small systematic bias to each collision, causing a gradual net drift toward the positive potential. The individual random steps are nearly unchanged, but their cumulative effect produces a measurable current.*

---

## Practical Takeaways

- **Material purity is paramount**: Even parts-per-billion impurity levels significantly affect semiconductor conductivity. Fabrication requires extraordinary cleanliness and material purity.
- **N-channel devices are inherently faster** than P-channel due to higher electron mobility ($\mu_n / \mu_p \approx 2.8$ in Si). This asymmetry pervades all of analog layout -- NMOS transistors are typically smaller than PMOS for equivalent drive strength.
- **Optical generation is a real layout concern**: Bare die and chip-scale packages can malfunction under intense light. Light-sensitive nodes must be shielded or the package must block stray photons.
- **Recombination centers (gold, transition metals) are double-edged**: They speed up switching but degrade gain. Contamination control during fabrication is critical for analog circuits that demand high $\beta$.
- **Counterdoping is the foundation of IC structure**: All devices (diodes, BJTs, MOSFETs) are formed by creating adjacent P- and N-type regions through selective doping and counterdoping.
- **Velocity saturation matters for layout**: At high fields, Ohm's law breaks down. Resistors under large voltage drops behave nonlinearly. This affects both device modeling and physical layout of high-voltage circuits.
- **Temperature dependence of mobility** means device characteristics shift with temperature -- a central concern for analog design. Higher temperature reduces mobility and therefore reduces transconductance.
- **Surface mobility is lower than bulk mobility**: Carriers near the Si/SiO$_2$ interface (as in MOSFETs) experience additional scattering, which is why surface-channel devices have lower mobility than bulk devices.

---

## Relation to the Bigger Picture

This section establishes the fundamental physics that every subsequent chapter builds upon. The concepts of carrier generation/recombination, doping, and transport (diffusion + drift) are prerequisites for understanding PN junctions (Section 1.2), which in turn underpin diodes, BJTs, and MOSFETs. For the layout engineer, the key takeaways are: (1) the asymmetry between electron and hole mobility drives the sizing difference between NMOS and PMOS, (2) doping levels directly control resistivity and junction behavior, (3) optical generation and contamination-induced recombination centers are physical phenomena that layout choices can mitigate or exacerbate, and (4) velocity saturation and temperature dependence set practical limits on device performance that must be accounted for in analog design.

---

## See Also
- [[ch01-pn-junctions]]
