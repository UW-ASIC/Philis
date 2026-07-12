---
title: "2.4 Diffusion and Ion Implantation"
chapter: 2
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-2, diffusion, ion-implantation, doping, planar-process, fabrication]
---

# 2.4 Diffusion and Ion Implantation

> **Chapter 2: Semiconductor Fabrication**

## Key Concepts

### The Grown Junction Problem and the Planar Process

Before planar processing existed, PN junctions were formed by the **grown junction process**: a silicon ingot was grown from a melt, and successive counterdopings during crystal growth created stacked PN junctions. The ingot was then sliced into wafers and diced. This approach had a fatal flaw -- the PN junction **terminated on the sawn edges** of the die, where contamination and surface defects caused severe leakage across the exposed depletion region.

**Jean Hoerni's planar process** (Fairchild, 1959) solved this by forming junctions from the *top surface* of an oxidized wafer:

1. Oxidize a P-type wafer.
2. Photolithographically pattern and etch oxide windows.
3. Spin on a phosphosilicate glass (dopant source).
4. Heat in a tube furnace -- phosphorus diffuses into exposed silicon forming shallow N-type diffusions.
5. Etch away the doped oxide from the windows.
6. Dice and package.

The key advantage is that junctions **curve up to intersect the oxidized surface**, providing inherent passivation. This also enables the creation of multiple separate junctions on a single die -- the foundation of the **integrated circuit**, independently invented by Jack Kilby and Robert Noyce in 1959.

### Thermal Diffusion: The Fundamental Doping Mechanism

Dopant atoms move through the silicon lattice by thermal diffusion, analogous to carrier diffusion (Section 1.1.3), but requiring much higher temperatures ($800$--$1200\,^\circ\text{C}$) because dopant atoms are heavier and more tightly bound to the crystal lattice. Once cooled, dopant atoms become **immobilized** within the lattice, forming a permanent doped region called a **diffusion**.

### Ion Implantation: The Modern Alternative

Ion implantation uses a particle accelerator to shoot ionized dopant atoms directly into the silicon crystal. It provides:
- **Superior dose control** (dose $=$ beam current $\times$ time, both precisely measurable).
- **No high-temperature requirement** during implant itself (photoresist can serve as a mask).
- **Tailorable depth profiles** via energy selection and stacked implants.
- **Self-aligned structures** using deposited materials (e.g., polysilicon gates) as masks.

---

## Important Details

### 2.4.1 The Diffusion Process

#### Predeposition and Drive

A diffusion is formed in two steps:

1. **Predeposition (deposition):** The wafer is heated in contact with an external dopant source. Dopant atoms diffuse a short distance into the surface, creating a shallow, heavily doped region.
2. **Drive (drive-in):** The dopant source is removed and the wafer is heated to a *higher* temperature for a prolonged time. The deposited dopants are driven deeper into the silicon, producing a deeper but less heavily doped diffusion.

If a very heavily doped region is desired, the dopant source can remain in contact with the silicon during the drive.

#### Common Dopants

Four dopants dominate silicon processing:

| Dopant | Type | Diffusion Rate | Notes |
|---|---|---|---|
| **Boron** (B) | Acceptor (P-type) | Fast | Only common P-type dopant |
| **Phosphorus** (P) | Donor (N-type) | Fast | Can cause lattice strain at high concentrations |
| **Arsenic** (As) | Donor (N-type) | Slow | Used for shallow junctions; does not cause emitter push |
| **Antimony** (Sb) | Donor (N-type) | Slow | Used where slow diffusion is advantageous |

Boron and phosphorus diffuse relatively rapidly. Arsenic and antimony diffuse much more slowly, which is advantageous for forming **very shallow junctions**. Even the fast diffusers do not diffuse appreciably below about $800\,^\circ\text{C}$, requiring specialized high-temperature diffusion furnaces.

#### Representative Junction Depths

For $10^{20}\,\text{atoms/cm}^3$ source, $10^{15}\,\text{atoms/cm}^3$ background, 15 min deposition, 1 hr drive:

| Dopant | $950\,^\circ\text{C}$ | $1000\,^\circ\text{C}$ | $1100\,^\circ\text{C}$ | $1200\,^\circ\text{C}$ |
|---|---|---|---|---|
| Boron | $0.9\,\mu\text{m}$ | $1.5\,\mu\text{m}$ | $3.6\,\mu\text{m}$ | $7.3\,\mu\text{m}$ |
| Phosphorus | -- | $0.5\,\mu\text{m}$ | $1.6\,\mu\text{m}$ | $4.6\,\mu\text{m}$ |
| Antimony | -- | -- | $0.8\,\mu\text{m}$ | $2.1\,\mu\text{m}$ |
| Arsenic | -- | -- | $0.7\,\mu\text{m}$ | $2.0\,\mu\text{m}$ |

#### Deposition Sources

A **phosphorus deposition** uses a tube furnace with a $\text{POCl}_3$ (phosphorus oxychloride, pronounced "pockle") bubbler. Dry $\text{O}_2$ carries $\text{POCl}_3$ vapor over the wafers. Phosphorus atoms released by decomposition diffuse into the oxide film, forming a doped oxide that acts as the deposition source. After deposition, the doped oxide is stripped away (**deglazing**) before the drive step.

Other source types include:
- **Liquid sources:** $\text{POCl}_3$ (phosphorus), boron tribromide (boron)
- **Gaseous sources:** diborane, phosphine, arsine -- injected directly into gas stream
- **Solid sources:** boron nitride disks placed between wafers; boron trioxide outgasses to adjacent wafers
- **Spin-on glasses:** proprietary liquids spun onto the wafer and baked to form conformal dopant coatings

None of these deposition sources are particularly well controlled. Nonuniform gas flow around wafers inevitably introduces doping variations, which is why modern CMOS/BiCMOS processes rely primarily on ion implantation for precision. $\text{POCl}_3$ deposition remains in use for forming deep heavily doped N-type **sinker** regions in certain BiCMOS processes.

---

### 2.4.2 Other Effects of Diffusion

#### Outdiffusion (Lateral Diffusion)

Diffusions can only be performed from the wafer surface, limiting geometries. When patterned through an oxide mask, the dopant diffuses out **in all directions** at roughly the same rate. The junction therefore moves laterally under the oxide window edges by a distance equal to approximately **80% of the junction depth** ($x_j$):

$$x_\text{lateral} \approx 0.8 \cdot x_j$$

This **outdiffusion** causes the final diffused region to be larger than the oxide window. It is invisible under a microscope because oxide color fringes correspond to oxide step locations, not junction positions.

#### Doping Profile

The doping concentration varies with depth. Neglecting segregation:
- **Highest** at the surface (surface concentration $N_S$).
- **Gradually decreases** with depth.
- The junction forms where the diffused dopant concentration $N_D(x)$ equals the background concentration $N_B$.

Oxide segregation effects complicate this picture:
- **Boron suckup:** Boron segregates into the oxide, *reducing* surface doping of P-type diffusions. Can even cause a lightly counterdoped diffusion to invert to N-type.
- **Pileup:** Dopant accumulates at the surface; does not cause inversion but still affects surface doping levels.

#### Emitter Push

Phosphorus atoms are significantly smaller than silicon atoms. High concentrations of phosphorus introduce **lattice strain**, spawning defects within heavily doped regions. Some defects:
- Migrate to the surface, causing **dopant-enhanced oxidation**.
- Migrate downward, increasing the diffusion rate of other dopants (e.g., boron).

A classic example: in NPN transistors, a heavily doped phosphorus emitter accelerates boron diffusion in the underlying base region. This phenomenon is called **emitter push**. Arsenic does *not* cause emitter push because arsenic atoms are approximately the same size as silicon atoms.

#### NBL Push

Similarly, the tail of an N-type buried layer (NBL) diffusion can intersect and displace the PN junction above it. This **NBL push** effect shifts junction positions in processes that use buried layers.

#### Oxidation-Enhanced Diffusion (OED)

The oxidation process itself spawns defects that migrate downward, enhancing dopant diffusion beneath the growing oxide. This **oxidation-enhanced diffusion** affects all dopants and can produce significantly deeper diffusions under a LOCOS field oxide than under adjacent moat regions.

#### Practical Consequence: Process Complexity

All these interactions mean that even sophisticated computer simulations cannot always predict actual doping profiles and junction depths. Process engineers must experimentally optimize recipes. This is why:
- Most companies use only a **few standardized processes** for all products.
- Process engineers are reluctant to modify existing process recipes.
- Incorporating new process steps is extremely difficult.

---

### 2.4.3 Ion Implantation

#### The Ion Implanter

An ion implanter is a specialized particle accelerator operating in a **high vacuum** environment:

1. **Ion source** generates a beam of ionized dopant atoms.
2. **Linear accelerator** accelerates ions to tens or hundreds of keV.
3. **Magnetic analyzer** bends the beam; only ions with the correct mass-to-charge ratio pass through a narrow slit (isotope selection).
4. **Deflection plates** (scanning electrodes) sweep the beam back and forth across the wafer face.

#### Implant Physics

When ions enter the silicon lattice:
- They **decelerate** via collisions with lattice atoms, transferring momentum.
- The beam **scatters** (straggle), analogous to outdiffusion.
- Silicon atoms are **knocked out** of lattice positions, causing extensive **lattice damage**.

This damage must be repaired by a subsequent **anneal** (minimum $\sim 600\,^\circ\text{C}$). During annealing:
- Silicon atoms become mobile.
- The intact crystal structure around the implant edges acts as a **seed** for regrowth.
- Damage anneals progressively from the outside in.
- Implanted dopant atoms assume lattice positions (**activation**) and begin generating charge carriers.

Two annealing methods:

| Method | Temperature | Duration | Diffusion? |
|---|---|---|---|
| **Conventional anneal** | $800$--$1000\,^\circ\text{C}$ | 10--15 min in tube furnace | Causes some diffusion |
| **Rapid Thermal Anneal (RTA)** | $1000$--$1200\,^\circ\text{C}$ | A few seconds (halogen lamps) | Minimal diffusion |

**RTA** is particularly useful for annealing the extremely shallow source/drain implants of modern MOS transistors, where any additional diffusion would degrade device performance.

#### Dose Control

The implant dose is:

$$D = I_\text{beam} \times t$$

where $I_\text{beam}$ is the ion beam current and $t$ is the implant time. Both quantities are accurately measurable, giving ion implantation **much better doping control** than conventional deposition techniques.

#### Range and Straggle

The **range** is the depth at which peak dopant concentration occurs. Longer ranges require higher energies and produce somewhat larger straggle.

Typical ranges in microns for selected dopants in silicon:

| Dopant | 10 keV | 50 keV | 200 keV | 1 MeV |
|---|---|---|---|---|
| $^{11}\text{B}$ (Boron) | $0.03$ | $0.16$ | $0.51$ | $1.1$ |
| $^{31}\text{P}$ (Phosphorus) | $0.01$ | $0.06$ | $0.25$ | $1.0$ |
| $^{122}\text{Sb}$ (Antimony) | -- | $0.02$ | $0.08$ | $0.4$ |
| $^{75}\text{As}$ (Arsenic) | -- | $0.03$ | $0.11$ | $0.6$ |

Most implants use energies of **20--200 keV**, creating relatively shallow implants. Deeper profiles can be achieved by:
- Subsequent drive in a tube furnace.
- MeV-range implanters (can counterdope regions several microns deep to create **buried layers**).

#### Tailored Doping Profiles

Ion implantation enables sophisticated profile engineering:
- **Retrograde profile:** A large dose at high energy followed by smaller doses at lower energies creates a profile with *lower* doping at the surface than beneath it.
- **Chained implant:** Equal doses at varying energies create a deep implant with nearly **vertical sidewalls**.
- **Stacked implants:** Multiple implants at different energies to tailor the profile of any implant.

#### Self-Aligned Structures

A deposited material (e.g., polysilicon) can serve as an implant mask. The classic example is **self-aligned MOS source/drain formation**:

- A polysilicon gate is deposited and patterned on thin gate oxide.
- The polysilicon blocks the implant beneath the gate electrode.
- Source and drain regions are precisely aligned to the gate with only minimal straggle.
- Without self-alignment, photolithographic misalignment would require deliberate gate-drain overlap, greatly increasing $C_{GD}$ and slowing switching speeds.

#### Channeling

When the ion beam is aligned with a crystallographic axis ((100) or (111)), ions can travel deep into the crystal through **channels** (interstices between columns of atoms) before scattering. This produces unpredictable dopant distributions. To prevent channeling, most implanters project the beam at an angle of about **7 degrees** off-axis.

#### Tilted Implants

Off-axis implants can also be used *deliberately* to create implants under gate edges. A pair of tilted implants can fabricate **lightly doped drain (LDD) extensions** to the source and drain of a self-aligned MOS transistor. By restricting the tilt direction, one can create two classes of transistors (vertically vs. horizontally aligned) with different device characteristics.

#### Novel Applications

Almost any element can be implanted into silicon:
- **Argon** implanted into the wafer backside enhances **gettering** of impurities.
- **Germanium** implanted into silicon creates small regions of IV-IV compound semiconductors within a conventional silicon IC.

---

## Diagrams

### Diffusion Cross-Section, Doping Profile, and Diffusion Interactions (Figures 2.18 and 2.19)

![[diagrams/ch02-diffusion-implantation-fig1.png]]

**Figure 2.18 (top):** Cross-section (A) of a typical planar diffusion showing how the junction curves beneath the oxide mask edge due to outdiffusion. The doping profile (B) shows surface concentration $N_S$ decreasing with depth until it intersects the background concentration $N_B$ at junction depth $x_j$. **Figure 2.19 (bottom):** Three mechanisms that alter diffusion rates: (A) **emitter push** -- heavily doped phosphorus emitter accelerates boron diffusion in the base; (B) **NBL push** -- buried layer tail displaces the junction above; (C) **oxidation-enhanced diffusion** -- LOCOS oxidation drives dopants deeper beneath the field oxide than under adjacent moat regions.

### Ion Implanter Schematic (Figure 2.20)

![[diagrams/ch02-diffusion-implantation-fig2.png]]

**Figure 2.20:** Simplified diagram of an ion implanter showing the ion source, linear accelerator, magnetic analyzer (for mass/isotope selection), deflection plates (for beam scanning), and wafer target. The entire system operates in high vacuum. Also describes the annealing process, implant dose control, range/straggle, and the two annealing methods (conventional vs. RTA).

### Self-Aligned Implants and Tilted Implants (Figures 2.21 and 2.22)

![[diagrams/ch02-diffusion-implantation-fig3.png]]

**Figure 2.21 (top):** Self-aligned source and drain formation. The polysilicon gate blocks the implant, so source/drain regions are perfectly aligned to the gate edge with no photolithographic misalignment. LOCOS field oxide defines the lateral extent. **Figure 2.22 (bottom):** Tilted ion implantation creates lightly doped extensions that reach beneath the gate edge, useful for LDD structures. The tilt causes the implant to extend under the gate on one side and under the field oxide on the other.

---

## Practical Takeaways

- **Outdiffusion adds ~80% of junction depth laterally** beneath the mask edge. Always account for this when sizing diffusion windows in layout -- the actual doped region is larger than the drawn oxide opening.
- **Boron suckup** can reduce or even invert the surface doping of P-type diffusions. Be aware of this when relying on P-type diffusions for surface concentration-sensitive devices (e.g., resistors).
- **Emitter push** is caused by phosphorus, not arsenic. Processes using arsenic emitters avoid this problem, which is one reason arsenic is preferred for shallow emitter diffusions in modern bipolar processes.
- **Oxidation-enhanced diffusion** deepens junctions under LOCOS field oxide relative to moat regions. This asymmetry affects device matching and must be considered during layout.
- **Ion implantation provides far superior dose control** compared to diffusion deposition. Modern CMOS and BiCMOS processes use implantation for essentially all critical doping steps.
- **Self-aligned implants** (using polysilicon gates as masks) are essential for minimizing gate-drain overlap capacitance ($C_{GD}$) in MOS transistors. This is a cornerstone of modern CMOS fabrication.
- **RTA** is preferred over conventional furnace annealing for shallow implants (source/drain of modern MOSFETs) because it minimizes unwanted diffusion that would deepen the junction.
- **Channeling** is mitigated by tilting the wafer ~7 degrees off-axis during implantation. This is standard practice.
- **Retrograde and chained implants** enable profile engineering impossible with diffusion alone -- retrograde wells, deep implants with vertical sidewalls, and precisely controlled buried layers.
- **Process recipe optimization is empirical** -- interactions between multiple diffusion/implantation steps are too complex for even sophisticated simulations to predict perfectly. This is why fabs use a small number of standardized processes.

---

## Relation to the Bigger Picture

Diffusion and ion implantation are the two fundamental techniques for introducing dopant atoms into silicon, and they underpin nearly every device structure discussed in the rest of the book. The planar process (diffusion through oxide windows) is the historical foundation of all integrated circuit fabrication, while ion implantation has become the dominant modern technique due to its superior dose control and ability to create self-aligned structures. Understanding the limitations of diffusion -- outdiffusion, emitter push, OED, and process interactions -- is essential for analog layout because these effects directly impact device matching, junction depths, and parasitic characteristics. The concepts in this section connect directly to oxide growth and patterning ([[ch02-oxide-growth]]) which defines the masks for diffusion, and to silicon deposition ([[ch02-silicon-deposition]]) which provides the polysilicon films used as self-aligned implant masks.

## See Also
- [[ch02-oxide-growth]]
- [[ch02-silicon-deposition]]
