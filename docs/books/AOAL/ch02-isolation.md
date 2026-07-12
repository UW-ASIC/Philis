---
title: "2.6 Isolation"
chapter: 2
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-2, isolation, junction-isolation, dielectric-isolation, wafer-bonding, SOS, SIMOX, SOI]
---

# 2.6 Isolation

> **Chapter 2: Semiconductor Fabrication**

## Key Concepts

An integrated circuit requires a means of **confining currents to selected regions of silicon** so that individual devices do not interfere with one another. Without isolation, all transistors on a die would share the same body of silicon, and currents would flow freely between them. Two fundamentally different approaches exist:

1. **Junction isolation** -- uses reverse-biased PN junctions to block majority-carrier conduction. This is the simplest and cheapest approach and dominates mainstream CMOS and bipolar processes.
2. **Dielectric isolation** -- interposes an insulating material (a *dielectric*) between active silicon regions. This is more expensive but offers superior performance in radiation-hardened and high-voltage applications.

The choice between them is driven by cost, radiation tolerance, voltage capability, and parasitic performance requirements. Junction isolation dominates in commercial electronics; dielectric isolation dominates in military/aerospace (radiation hardness) and emerging high-voltage applications.

---

## 2.6.1 Junction Isolation

### Operating Principle

Reverse-biased PN junctions block majority-carrier conduction. The wafer itself becomes the **substrate**, and counterdoping creates isolated regions in the surface. The PN junctions that form between isolated regions and the substrate are called **isolation junctions**.

### Polarity Choices

Two basic configurations exist:

| Configuration | Substrate | Isolated Regions | Substrate Bias |
|---|---|---|---|
| **(A) P-type substrate** | P-type (most negative) | N-type isolated regions | Must be biased below the lowest voltage on any N-region |
| **(B) N-type substrate** | N-type (most positive) | P-type isolated regions | Must be biased above the highest voltage on any P-region |

**P-type substrates are more popular** because most system designers prefer a negative ground reference to a positive one. In either case, the substrate effectively serves as the **reference node** for the integrated circuit.

### Critical Biasing Requirement

**All isolation junctions must remain reverse-biased at all times.** If a pin is improperly biased relative to the substrate, one or more isolation junctions can become forward-biased. This causes the affected isolation regions to **inject minority carriers into the substrate**, which diffuse across to other isolated regions. The consequences range from:

- Momentary parametric anomalies (e.g., dip in a voltage regulator output)
- Complete malfunction and self-destruction (see Section 5.4.2 in the text)

This is a critical failure mode that layout and circuit designers must guard against.

### N-Well Isolation

The simplest fabrication approach is to drive deep, lightly doped diffusions into the substrate to create isolation regions called **wells**.

- Start with a lightly doped P-type substrate
- Oxidize and pattern to open windows where wells are needed
- Deposit or implant phosphorus through the oxide windows
- A long, high-temperature drive pushes phosphorus down to form deep, lightly doped N-type wells
- For a **40 V process**, a typical well junction depth is approximately $6\text{--}8\ \mu\text{m}$. This depth allows the depletion region around the isolation junction to extend upward far enough to support the full operating voltage without intruding upon the topmost region where the active devices reside

### N-Tank Isolation

An alternative approach uses **epitaxial silicon**:

1. Deposit a layer of lightly doped N-type epitaxial silicon upon a lightly doped P-type substrate
2. Oxidize and pattern -- open windows everywhere *except* where isolation regions will appear
3. Deposit or implant boron into the windows
4. A long, high-temperature drive forces the isolation diffusions **downward** to meet the **updiffusing boron** from the underlying substrate
5. The result: regions of N-type epi called **tanks** (or **tubs**) surrounded by P-type isolation diffusions

**Historically**, standard bipolar processes have used N-tank isolation, and most CMOS processes have used N-well isolation.

### Well vs. Tank -- Terminology

| Term | Definition |
|---|---|
| **Well** | A lightly doped *diffusion* -- counterdoped silicon with a nonuniform doping profile |
| **Tank** (or **tub**) | An epi region *isolated by surrounding diffusions* -- uniform doping from the epi layer |

Some hybrid structures combine features of both, but most structures clearly fall into one category.

---

## 2.6.2 Dielectric Isolation

### Motivation: Radiation Hardness

Ionizing radiation compromises junction isolation. When a high-energy particle (e.g., a cosmic ray) passes through a reverse-biased depletion region, it knocks loose valence electrons, producing a **current pulse**. Isolation junctions are particularly susceptible because of their large dimensions. Military and aerospace engineers therefore sought alternatives to junction isolation.

### Silicon-on-Sapphire (SOS)

Developed by **Harold Manasevit in 1963**, SOS was one of the first dielectric isolation systems.

**Process:**
- Start with a $(1\overline{1}02)$-oriented sapphire ($\text{Al}_2\text{O}_3$) substrate
- Epitaxially deposit silicon, which assumes the (100) orientation on the sapphire surface
- Pattern-etch the silicon to form isolated regions called **islands**

**Advantages:**
- Islands sit on an insulating substrate, so **no substrate injection** occurs regardless of biasing
- SOS circuits can withstand radiation levels **an order of magnitude greater** than conventional junction isolation
- Modern ultra-thin SOS processes achieve extremely low drain capacitances, enabling high-speed operation

**Disadvantages:**
- Higher cost and smaller sapphire wafer sizes relative to conventional silicon wafers
- Difficulty of obtaining suitable sapphire substrates

Despite these drawbacks, SOS continues to find applications in high-speed products and is experiencing renewed interest for **fully depleted MOS transistors**.

### Shape-Back Process (Early Silicon-DI)

The difficulty of obtaining sapphire substrates led to several **silicon-based dielectric isolation** processes in the late 1960s. The **shape-back process** is representative:

1. **Etch grooves** in the surface of a lightly doped N-type silicon wafer (originally isotropic etch; later V-groove variants used orientation-dependent etchants). Grooves are typically about $25\ \mu\text{m}$ deep.
2. **Wet oxidation** grows approximately $1\ \mu\text{m}$ of oxide upon the grooved wafer
3. **Deposit polysilicon** (about $300\ \mu\text{m}$) on top of the oxide -- this becomes the structural **handle**
4. **Flip and grind** -- turn the wafer over and grind down until the polysilicon-filled grooves intersect the surface
5. The remaining monocrystalline silicon forms **tanks** in which active devices reside; the oxide layer between tanks and the handle is called the **buried oxide (BOX)**

**Problems:** Severe wafer nonplanarity (bowing), and large isolation spacings with isotropic etches. V-groove variants reduced spacing but never fully solved the bowing problem.

### SIMOX (Separation by Implanted Oxygen)

SIMOX eliminates the polysilicon handle entirely:

1. **Blanket implant oxygen** into a (100)-oriented silicon substrate at high energy (~200 keV)
2. **High-temperature anneal** -- the lattice damage beneath the surface recrystallizes to form a (100) silicon layer approximately $0.2\ \mu\text{m}$ thick, while the implanted oxygen reacts with silicon to form a **buried oxide** layer approximately $0.4\ \mu\text{m}$ thick
3. **Grow epitaxial layer** on top of the superficial silicon
4. **Anisotropic reactive ion etching** cuts trenches down through the epi layer to the buried oxide
5. **Sidewall oxidation** followed by **polysilicon backfill** of the trenches
6. **Chemical-mechanical polishing (CMP)** removes unwanted poly and creates a planar surface

The result: lightly doped N-tanks surrounded by deep trench isolation, floored by buried oxide.

---

## 2.6.3 Wafer Bonding

Wafer bonding is a more modern approach to creating dielectrically isolated substrates.

### Basic Process

1. Start with **two silicon wafers**
2. **Oxidize** one wafer
3. Place the second wafer on top of the oxidized surface
4. **Heat to approximately $1100\ ^\circ\text{C}$** -- this causes the wafers to tightly adhere (wafer fusion)
5. The result: a **buried oxide (BOX)** sandwiched between two thick layers of silicon
6. One of the silicon layers must be **thinned** to form the active device layer

### Thinning Methods

Several methods exist for thinning the bonded wafer:

- **Lap-and-polish** -- mechanical grinding
- **Selective wet etch-back** -- terminates on a heavily doped layer adjacent to the BOX
- **Wafer cleaving** (most ingenious) -- a highly strained layer is created beneath the surface of one wafer by implanting **hydrogen or germanium**. A subsequent **thermal shock** induces the silicon to cleave along the strain layer, leaving a thin layer of monocrystalline silicon bonded on top of the buried oxide

Wafer cleaving can only produce thin layers of silicon, but subsequent **epitaxial deposition** can build up a monocrystalline silicon layer of any desired thickness.

### Wafer Bonding with Cleaving -- Detailed Steps

The complete fabrication sequence (Figure 2.30):

1. **Oxidize handle** -- grow a thick layer of wet oxide on a $P^-$ wafer (the handle)
2. **Germanium implant** -- implant germanium beneath the surface of a second $P^-$ wafer
3. **Wafer bonding** -- place the germanium-implanted wafer atop the handle (germanium layer facing down); heat to bond
4. **Wafer cleaving** -- a sharp thermal shock shears through the germanium strain layer, leaving only a thin layer of monocrystalline silicon on the BOX
5. **Polish back** -- etch back the newly exposed surface to remove germanium-contaminated silicon
6. **Epitaxy** -- deposit $P^-$ epitaxial silicon on top to form the active silicon layer
7. **Deep trench formation** -- anisotropic RIE forms trenches down to the BOX, followed by sidewall oxidation, polysilicon deposition, and CMP to complete the deep trench isolation system

The resulting wafer closely resembles one created by SIMOX.

---

## Diagrams

### Figure 2.26 / 2.27 -- Junction Isolation Polarities and Systems

![[diagrams/ch02-isolation-fig1.png]]

**Caption:** Top: Two choices of junction isolation polarity -- (A) P-type substrate with N-type isolated regions, (B) N-type substrate with P-type isolated regions. The substrate is always connected to the most extreme supply rail to ensure all isolation junctions remain reverse-biased. Bottom (Figure 2.27): Typical junction isolation implementations -- (A) N-well isolation formed by diffusing phosphorus into a P-substrate, and (B) N-tank isolation where N-type epi regions are surrounded by P-type isolation diffusions driven down to meet the updiffusing substrate.

### Figure 2.29 -- SIMOX Dielectric Isolation System

![[diagrams/ch02-isolation-fig2.png]]

**Caption:** Steps in the manufacture of a SIMOX dielectric isolation system. Step 1: High-energy oxygen implant into silicon. Step 2: Anneal to form recrystallized silicon surface and buried oxide. Step 3: Epitaxial deposition. Step 4: Deep trench etch, sidewall oxidation, poly fill, and CMP to form isolated tanks floored by buried oxide (BOX).

### Figure 2.30 -- Wafer Bonding and Cleaving

![[diagrams/ch02-isolation-fig3.png]]

**Caption:** Steps in the fabrication of dielectric isolation by wafer bonding and cleaving. The process uses germanium implant to create a strain layer for controlled cleaving, producing a thin monocrystalline silicon layer on a buried oxide. Subsequent epitaxial growth and deep trench formation complete the isolation system.

---

## Practical Takeaways

- **Always ensure isolation junctions remain reverse-biased.** Connect the substrate to the most negative supply (for P-substrates) or most positive supply (for N-substrates). Forward-biasing an isolation junction causes minority-carrier injection into the substrate, leading to inter-device crosstalk or latchup-induced destruction.
- **Well depth scales with operating voltage.** A 40 V process requires well junctions approximately $6\text{--}8\ \mu\text{m}$ deep to accommodate the depletion region extension at maximum operating voltage.
- **Know the difference between wells and tanks.** Wells are counterdoped diffusions (nonuniform doping profile); tanks are epi regions isolated by surrounding diffusions (uniform doping). Standard bipolar uses N-tank; most CMOS uses N-well.
- **Dielectric isolation eliminates substrate injection entirely** but at higher cost. It is the go-to choice for radiation-hardened circuits (military, aerospace) and is increasingly attractive for **high-voltage ICs** (hundreds of volts) where junction isolation would consume excessive die area.
- **SIMOX and wafer bonding** are the two dominant modern approaches to dielectric isolation. Both produce silicon-on-insulator (SOI) substrates with deep trench isolation and buried oxide. Wafer bonding with cleaving is more flexible in achievable silicon thickness.
- **SOS is making a comeback** for fully depleted MOS transistors, which offer speed advantages that are difficult to achieve through conventional device scaling.
- **Wafer bowing and nonplanarity** plagued early silicon-DI processes (shape-back). Modern SIMOX and wafer bonding processes have largely solved these issues through CMP and refined processing.

---

## Relation to the Bigger Picture

Isolation is the critical bridge between the front-end fabrication steps (diffusion, implantation, epitaxy -- covered in [[ch02-silicon-deposition]]) and the back-end interconnection steps (metallization, dielectrics -- covered in [[ch02-interconnection]]). Without effective isolation, the active devices created by front-end processing cannot function independently. The choice of isolation technology profoundly influences the entire process flow: junction isolation dictates well depths, substrate doping, and biasing constraints; dielectric isolation requires additional steps like trench etching, BOX formation, and CMP. In Chapter 4 (Representative Processes), the standard bipolar process uses N-tank junction isolation while advanced BiCMOS processes may incorporate deep trench dielectric isolation. Understanding isolation is therefore essential for interpreting process cross-sections and making informed layout decisions throughout the rest of the book.

## See Also
- [[ch02-silicon-deposition]]
- [[ch02-interconnection]]
