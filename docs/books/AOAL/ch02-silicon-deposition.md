---
title: "2.5 Silicon Deposition and Etching"
chapter: 2
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-2, epitaxy, polysilicon, etching, fabrication]
---

# 2.5 Silicon Deposition and Etching

> **Chapter 2: Semiconductor Fabrication**

## Key Concepts

This section addresses the three interconnected processes that shape silicon films on a wafer: **epitaxy** (growing monocrystalline silicon), **poly deposition** (growing polycrystalline silicon), and **silicon etching** (selectively removing silicon). These processes are fundamental to every modern IC process flow because they determine the quality of the active device regions, enable self-aligned transistor gates, and carve the isolation structures that keep devices electrically independent.

The central physical idea is straightforward: **silicon atoms deposited on a crystalline silicon surface will attempt to align with the existing crystal lattice**. If the surface is crystalline and conditions are right (sufficient temperature, appropriate deposition rate), the deposited film becomes a seamless monocrystalline extension of the substrate -- this is *epitaxy*. If the surface is amorphous (e.g., oxide or nitride), there is no lattice to guide crystal growth. Nucleation occurs at random points, producing a jumble of tiny interlocking crystals called *grains* -- this is *polycrystalline silicon* (poly). The distinction matters enormously: monocrystalline silicon has well-behaved electrical properties suitable for active devices, while polycrystalline silicon has grain-boundary defects that increase resistivity and cause leakage in PN junctions, but these very properties make it useful for resistors and gate electrodes.

## Epitaxy (Section 2.5.1)

### What It Is

Epitaxy is the growth of a monocrystalline film upon a crystalline substrate. The substrate usually has the same crystal structure as the film, but suitable lattice-matched materials also work. A striking example: high-quality (100) silicon films can be grown on $(11\overline{0}2)$ sapphire surfaces because the oxygen atom positions on the sapphire surface happen to match the (100) silicon lattice spacing. This is the basis of **silicon-on-sapphire (SOS)** technology, though sapphire's cost limits it to niche applications.

### Deposition Method: LPCVD

Most modern epitaxial depositions use **low-pressure chemical vapor deposition (LPCVD)**. The process works as follows:

1. Wafers are mounted on an inductively heated carrier block inside a fused silica tube.
2. Wafers are heated to approximately $1200\,^{\circ}\mathrm{C}$.
3. **Dichlorosilane** ($\mathrm{SiH_2Cl_2}$) gas is passed across the wafers.
4. On contact with the hot silicon surface, the gas decomposes: the silicon atoms bond to the exposed surface, and $\mathrm{HCl}$ gas is released.
5. The epitaxial layer ("epi") faithfully reproduces the topography of the underlying surface -- no polishing is needed.

The growth rate is controlled by adjusting wafer temperature and reactant gas composition. Introducing gaseous dopant sources such as **phosphine** ($\mathrm{PH_3}$, for N-type) or **diborane** ($\mathrm{B_2H_6}$, for P-type) enables **in-situ doping** of the epi layer during growth.

### Crystal Orientation Matters

- Deposition on **(100)** silicon surfaces proceeds without difficulty.
- Deposition on **(111)** surfaces is much harder -- high-quality films typically require cutting the wafer a few degrees **off-axis** to encourage orderly crystal growth.

### Epi-Coated Wafers

Most modern processes use **epi-coated wafers**: an epitaxial layer grown on a conventional Czochralski wafer (the "substrate"). Critically, the epi layer need not match the substrate's dopant type or concentration:

| Process Type | Epi | Substrate |
|---|---|---|
| Standard bipolar | $N^-$ | $P^-$ |
| Most CMOS | $P^-$ | $P^+$ |

Multiple successive layers of differently doped silicon are also possible.

### N-Buried Layer (NBL)

The **N-buried layer** is one of the most important structures enabled by epitaxy. It serves a vital function in bipolar processes by providing a low-resistance collector contact path for vertical NPN transistors, and in CMOS processes by enabling isolated NMOS transistors in $P^-$ epi.

**NBL fabrication steps:**

1. Start with lightly doped P-type (111) silicon.
2. Oxidize the wafer and pattern windows in the oxide.
3. Implant **arsenic** or **antimony** through the windows.
4. Brief anneal to eliminate implant damage (thermal oxidation occurs simultaneously).
5. Strip all oxide from the wafer.
6. Deposit $N^-$ epitaxial layer on top.
7. Result: patterned $N^+$ regions buried underneath the $N^-$ epi.

**Dopant choice for NBL:**

- **Antimony**: Preferred because it exhibits less *lateral autodoping* (i.e., less tendency to spread sideways during epitaxy). Its slow diffusion rate minimizes outdiffusion during subsequent high-temperature processing.
- **Arsenic**: Has higher solid solubility and can produce more heavily doped buried layers, but suffers from greater lateral autodoping.

### NBL Shadow and Pattern Shift

During the NBL anneal, oxidation erodes the silicon surface slightly beneath the implant windows. The epi layer faithfully reproduces these surface discontinuities, creating a faintly visible outline called the **NBL shadow**. This shadow serves as an **alignment marker** for subsequent photomasks.

**Important caveats:**
- Processes using **shallow trench isolation (STI)** cannot rely on the NBL shadow because the CMP planarization step removes shallow surface discontinuities. These processes use a separately etched alignment marker placed before buried layer deposition.
- **Pattern shift**: The NBL shadow often appears laterally displaced from the actual NBL diffusion because epitaxial growth proceeds *diagonally upward*, not vertically. The angle depends on deposition temperature, gas composition, pressure, and crystal orientation. On (111) silicon, cutting wafers approximately $3^{\circ}$ to $4^{\circ}$ off-axis minimizes pattern shift and maximizes epi deposition rate.

## Poly Deposition (Section 2.5.2)

### Grain Structure and Properties

When silicon is deposited on an amorphous surface (oxide, nitride), there is no crystal template. The resulting film consists of an aggregate of small intergrown crystal grains with diameters typically averaging $0.03$ to $0.3\,\mu\mathrm{m}$. Grain size depends on:

- Film thickness
- Deposition conditions (temperature, pressure, gas composition)
- Annealing time

The **grain boundaries** exhibit numerous lattice defects. This has two consequences:

1. **PN junctions cannot be reliably fabricated in poly** -- the grain boundary defects cause excessive leakage if they appear inside a depletion region.
2. **Lightly doped poly has much higher resistivity** than equivalently doped monocrystalline silicon -- typically an order of magnitude or more. This is actually *useful* for fabricating high-value resistors.

### Why Poly Is Used

Poly serves three critical roles in modern ICs:

1. **MOS gate electrodes**: Poly can withstand the high temperatures needed to anneal source/drain implants, enabling the construction of **self-aligned transistors**. It also improves threshold voltage ($V_{th}$) control because phosphorus in the poly can immobilize ionic contaminants (see Section 5.2.2).
2. **Deposited resistors**: Poly resistors exhibit fewer parasitics than diffused counterparts. Modern etching can create very narrow linewidths, and the elevated resistivity from grain boundaries allows construction of resistors in the **tens of megohms** range, versus only a few hundred kilohms for diffused resistors.
3. **Deposited capacitors**: Poly capacitors similarly offer reduced parasitic effects compared to diffused structures.

### Deposition Process

Poly deposition uses the same LPCVD reactors as epitaxy, but with different chemistry:

- **Silane** ($\mathrm{SiH_4}$) is used as the reactant instead of dichlorosilane, to obtain the small-grained thin films desired for gate electrodes and resistors.
- **P-type doping**: Adding **diborane** ($\mathrm{B_2H_6}$) enables in-situ doping.
- **N-type doping**: The corresponding N-type dopant gases greatly reduce the poly deposition rate and increase its variability. Therefore, N-type poly is formed by first depositing intrinsic (undoped) poly and then **diffusing or implanting** the dopant in a separate step.

### Patterning

A patterned poly layer is produced by:

1. Depositing polysilicon across the entire wafer.
2. Doping as desired.
3. Coating with photoresist.
4. Patterning and etching to selectively remove poly.

## Silicon Etching (Section 2.5.3)

### Isotropic Wet Etch

The most common isotropic wet etchant for silicon is a mixture of **hydrofluoric acid ($\mathrm{HF}$)**, **nitric acid ($\mathrm{HNO_3}$)**, and **acetic acid ($\mathrm{CH_3COOH}$)**:

- The nitric acid superficially oxidizes the silicon surface.
- The hydrofluoric acid dissolves the resulting $\mathrm{SiO_2}$.

This etch attacks silicon uniformly in all directions, producing sidewall erosion similar to buffered oxide etches. It is **seldom used today** because the sidewall erosion makes it difficult to pattern small features.

### Anisotropic Dry Etch (Reactive Ion Etching)

**Reactive ion etching (RIE)** using fluorinated gases (e.g., trifluoromethane $\mathrm{CHF_3}$) is preferred over plasma etching because it offers much higher **anisotropy**, producing nearly vertical sidewalls. RIE is essential for:

1. **Narrow poly gate geometries** required by modern CMOS transistors.
2. **Shallow trenches** (subsequently filled with deposited oxide) that replace LOCOS oxides for **shallow trench isolation (STI)** in advanced CMOS and BiCMOS processes. STI with RIE can fabricate MOS transistors with channel widths of less than $0.5\,\mu\mathrm{m}$.
3. **Deep trenches** for dielectric isolation systems or through-silicon vias (TSVs). With careful control, trench depths can exceed their widths by an order of magnitude.

### Orientation-Dependent Wet Etch

A class of wet etches preferentially etches down to expose $\{111\}$ crystal planes. These **orientation-dependent etches** create:

- **V-shaped grooves** with sidewalls $54.7^{\circ}$ from vertical on (100) wafers.
- The etchant is typically **potassium hydroxide in propanol** ($\mathrm{KOH}$).
- The interior angle formed at groove intersections displays faceting corresponding to $\{311\}$ crystal planes. If this faceting is undesirable, a small protrusion can be added to the photomask corners to compensate.

Applications include discrete MOS power transistors and certain dielectric isolation systems.

## Diagrams

### Figure 2.24 -- Formation of N-Buried Layer (NBL) with Pattern Shift

![[diagrams/ch02-silicon-deposition-fig1.png]]

*Cross-section showing the NBL fabrication sequence: oxide patterning, implantation of arsenic or antimony, anneal with simultaneous oxidation (which erodes the silicon surface at windows), oxide strip, and epitaxial deposition. Note the pattern shift -- the NBL shadow at the epi surface is laterally displaced from the actual buried layer beneath it because epitaxial growth proceeds at an angle rather than vertically.*

### Figure 2.25 -- Cross Sections of Anisotropically Etched Silicon

![[diagrams/ch02-silicon-deposition-fig2.png]]

*Three types of etched silicon structures: (A) shallow trench for STI -- nearly vertical sidewalls, moderate depth, subsequently filled with oxide; (B) deep trench -- much greater depth-to-width ratio, used for dielectric isolation or TSVs; (C) V-groove -- orientation-dependent wet etch exposing {111} planes at $54.7^{\circ}$ on (100) wafers.*

### Figure 2.23 -- Simplified Epi Reactor Diagram

![[diagrams/ch02-silicon-deposition-fig3.png]]

*Simplified diagram of an early horizontal tube LPCVD epi reactor. Wafers sit on an inductively heated carrier block inside a fused silica tube. Dichlorosilane flows across the heated wafers (approximately $1200\,^{\circ}\mathrm{C}$), decomposing on contact to deposit silicon atoms that bond epitaxially to the substrate. Modern reactors use different geometries but the same fundamental chemistry.*

## Practical Takeaways

- **Epi quality depends on crystal orientation**: (100) substrates are far easier to deposit on than (111). If (111) must be used, cut the wafer a few degrees off-axis.
- **Antimony vs. arsenic for NBL**: Use antimony when lateral autodoping is a concern (tighter layouts); use arsenic when you need heavier doping (lower buried layer resistance).
- **NBL shadow alignment is fragile**: Any process with CMP (e.g., STI) will destroy the NBL shadow. Plan for a separate etched alignment marker, which adds a mask step and cost.
- **Pattern shift must be accounted for in layout**: The lateral displacement between the NBL and its surface shadow depends on process-specific parameters. Layout rules will specify guard bands to accommodate this.
- **N-type poly requires a two-step process**: You cannot efficiently in-situ dope N-type poly. Always deposit intrinsic poly first, then implant or diffuse the N-type dopant. This affects process flow scheduling.
- **Poly resistors can reach tens of megohms**: The grain boundary effect that increases resistivity is a feature, not a bug, for resistor design. But it also means poly resistor matching and temperature coefficient behavior differ from diffused resistors.
- **RIE is essential for modern geometries**: Isotropic wet etches are obsolete for feature patterning. RIE provides the anisotropy needed for sub-micron poly gates and STI trenches.
- **Deep trench aspect ratios > 10:1 are achievable** with careful RIE control, enabling advanced dielectric isolation and TSV fabrication.
- **V-groove etching requires mask compensation**: Add protrusions at convex corners of the photomask to counteract $\{311\}$ faceting at groove intersections.

## Relation to the Bigger Picture

Silicon deposition and etching are the bridge between the raw wafer preparation (crystal growth, Section 2.1) and the isolation structures (Section 2.6) that define where devices can live on the die. Epitaxy creates the high-quality monocrystalline layers in which active bipolar and MOS transistors are built, while poly deposition provides the gate electrodes and passive components that modern CMOS demands. The etching techniques -- particularly RIE -- directly enable the isolation schemes (STI, deep trench isolation) discussed in the next section. For the analog layout designer, understanding these processes explains why certain layout rules exist: minimum poly widths are set by etch resolution, NBL overlap rules account for pattern shift and autodoping, and the choice between poly and diffused resistors involves tradeoffs in parasitic capacitance, resistivity range, and matching that trace back to the grain structure of deposited poly versus the uniform doping of monocrystalline diffusions.

## See Also

- [[ch02-diffusion-implantation]]
- [[ch02-isolation]]
