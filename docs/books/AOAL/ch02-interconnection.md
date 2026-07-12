---
title: "2.7 Interconnection"
chapter: 2
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-2, interconnection, metallization, BEOL, aluminum, copper, silicide, tungsten-plug, damascene]
---

# 2.7 Interconnection

> **Chapter 2: Semiconductor Fabrication**

## Key Concepts

### Front End of Line vs. Back End of Line

The fabrication of an integrated circuit divides naturally into two phases:

- **Front End of Line (FEOL):** Creation of active devices in monocrystalline silicon --- epitaxial deposition, diffusion, ion implantation, and silicon etching.
- **Back End of Line (BEOL):** A series of polysilicon and metal depositions separated by dielectric depositions that create interconnections and passive components (resistors, capacitors).

Section 2.7 focuses on the BEOL --- the interconnection system that wires active devices together. The choice of interconnection technology profoundly affects the performance, reliability, and cost of the finished IC.

### The Single-Level Metal (SLM) System

The simplest interconnection system uses a single metal layer. After FEOL processing leaves oxide across the wafer, contact windows are etched, aluminum is deposited and patterned, and a **protective overcoat (PO)** --- typically oxide or nitride --- is applied. Openings in the PO expose bondpads for wirebonding.

Additional metal layers can be sequentially deposited and patterned to form **multilevel metal** systems. Modern digital logic needs five or more layers; most analog processes use three or four, with the topmost layer often thickened to carry high currents.

**Polysilicon as a routing layer:** CMOS processes use low-resistivity poly for self-aligned MOS gates. This poly can serve as a "free" extra routing layer, but its resistance is many times higher than true metal, so designers must avoid routing current-carrying signals through significant lengths of poly. A second poly layer (for capacitor top plates) can also be pressed into service as additional interconnect.

### Why Interconnection Technology Matters

As feature sizes shrink, interconnection becomes the performance bottleneck rather than the transistors themselves. The RC delay of wiring --- governed by metal resistivity ($\rho$), interlevel dielectric permittivity ($\varepsilon$), lead geometry, and parasitic capacitance --- dominates signal propagation in advanced nodes. The evolution from aluminum to copper metallization, the introduction of tungsten plugs and CMP planarization, and the development of low-$\kappa$ dielectrics are all responses to this fundamental challenge.

---

## Important Details

### 2.7.1 Aluminum

Most metallization systems employ **aluminum** or aluminum alloys for the primary interconnection layers. Aluminum conducts electricity almost as well as copper or silver, and it readily deposits in thin films that adhere to all semiconductor materials.

**Deposition:** Early ICs used pure aluminum deposited by **evaporation** --- wafers face a heated crucible of aluminum inside a high-vacuum chamber. Aluminum vapor deposits conformally on the wafer surfaces. High vacuum prevents oxidation of the vapor before deposition.

**Sintering and Ohmic Contact:** Briefly heating to roughly $450\,^\circ\text{C}$ forms an extremely thin aluminum-doped silicon layer beneath contact openings. This is called **sintering**. Since aluminum is an acceptor, it creates a shallow, heavily doped P-type diffusion that bridges between the metal and P-type silicon. Ohmic contact to heavily doped N-type silicon also works because the PN junction depletion region is thin enough for carriers to tunnel through. However, Ohmic contact *cannot* be established directly between aluminum and *lightly* doped N-type silicon (rectification occurs), so a shallow diffusion is needed in those regions.

**Contact Spiking:** During sintering, aluminum dissolves into underlying silicon, creating small pits. The deepest pits can punch entirely through a thin diffusion and the junction beneath it --- this is called **contact spiking** (or **emitter punchthrough** when it destroys NPN emitters). The solution is to use an **aluminum-silicon alloy** (a few percent Si). If the aluminum is already saturated with silicon, it theoretically cannot dissolve more. In practice, silicon content tends to segregate during sintering, leaving unsaturated aluminum that still attacks silicon. Careful control of sinter time and temperature minimizes this effect, though segregated silicon nodules give the metal a rough, pebbly appearance under magnification.

**Electromigration:** As IC dimensions shrank in the 1970s, current density in metallization approached $\sim 10^6\,\text{A/cm}^2$. At such densities, electron momentum can physically displace metal atoms, causing open or short-circuit failures --- this is **electromigration** (detailed in Section 5.1.3). Adding a fraction of a percent of **copper** to the aluminum alloy improves electromigration resistance by about an order of magnitude. Most modern aluminum systems use **Al-Cu-Si** or **Al-Cu** alloys.

### 2.7.2 Refractory Barrier Metal

A **diffusion barrier** is a layer inserted between two materials to prevent them from reacting. In the late 1970s, engineers began using refractory metals such as tungsten and molybdenum as barriers between copper-doped aluminum and silicon. A popular choice was a **tungsten-titanium alloy** (10--30% Ti by mass), called **refractory barrier metal (RBM)**.

**Purpose of RBM:**

1. **Prevents contact erosion** by blocking aluminum-silicon interdiffusion.
2. **Improves step coverage** on contact and via sidewalls (deposited by sputtering, which is more isotropic than evaporation).
3. **Acts as a diffusion barrier** between platinum silicide and aluminum in silicided systems.

**Step coverage problem:** Contact and via sidewalls are sharp steps. Evaporated aluminum thins dramatically where it crosses these steps (Figure 2.33A), raising current density and accelerating electromigration. Two solutions:

- **Reflow:** Heating the wafer until the oxide softens and slumps to form sloped sidewalls (Figure 2.33B). Pure oxide requires too high a temperature, so **PSG** (phosphosilicate glass) or **BPSG** (borophosphosilicate glass) is used with a lower softening point. However, reflow cannot be applied to vias because the necessary temperatures damage underlying aluminum.
- **RBM sputtering:** RBM deposits almost isotropically on steep sidewalls. In a **sputtering** apparatus, wafers sit in a low-pressure argon chamber facing a plate of refractory metal. Argon plasma bombards the plate, knocking loose metal atoms that deposit on the wafers.

**Sandwich structure:** Most metal systems use a thin RBM layer beneath a thick aluminum layer. The RBM provides step coverage and diffusion barrier properties; the aluminum provides low resistance. RBM is extremely resistant to electromigration, so aluminum thinning on sidewalls does not pose an electromigration hazard for the RBM layer itself.

### 2.7.3 Silicidation

Elemental silicon reacts with many metals (platinum, palladium, titanium, cobalt, nickel) to form **silicides** --- compounds of definite composition. Silicides serve multiple purposes:

1. **Low-resistance Ohmic contacts** to silicon.
2. **Schottky diodes** (certain noble silicides form rectifying barriers).
3. **Reduced resistance of silicon regions** --- silicides have much lower resistivity than even heavily doped silicon.
4. **Clad poly (silicided poly):** Used for MOS transistor gates to increase switching speed by lowering gate resistance. Advanced processes also clad source/drain regions.

**Self-aligned silicidation (salicide) process:**

1. After contacts are opened, a thin film of metal (e.g., platinum) is sputtered across the entire wafer.
2. Heating causes the metal to react with exposed silicon to form silicide (e.g., $\text{PtSi}$).
3. Unreacted metal on oxide regions is removed selectively (platinum uses **aqua regia** --- a nitric/hydrochloric acid mixture).
4. The silicide forms **only** where silicon was exposed --- it is self-aligned to oxide openings or poly edges.

**Silicide block mask:** If clad poly is used, a masking step with a **silicide block mask** is required to fabricate polysilicon resistors. Without this mask, silicidation would transform all poly into low-resistance material, making it impossible to produce resistors above a few kilohms.

**A typical silicided first-level metal stack** (bottom to top):
- Platinum silicide (in contacts only)
- Refractory barrier metal (diffusion barrier + step coverage)
- Copper-doped aluminum (low-resistance conductor)

Alternatively, **titanium nitride (TiN)** can replace conventional RBM with similar properties.

**Choice of silicide material:**

| Silicide | Advantages | Disadvantages |
|----------|-----------|---------------|
| Platinum, Palladium (noble) | Form Schottky diodes; useful components | Decompose at relatively low temperatures; limit subsequent processing |
| Titanium (refractory) | Reduces $\text{SiO}_2$, ensuring low-resistance contact even through native oxide | Resistivity increases dramatically in leads narrower than $\sim 0.5\,\mu\text{m}$ due to C49$\to$C54 phase transformation requiring large grains |
| Nickel, Cobalt (refractory) | No phase transformation; no linewidth-dependent resistivity increase | --- |

**Titanium silicide phase transformation detail:** $\text{TiSi}_2$ initially deposits in the **C49 phase** (higher resistivity), which transforms during annealing into the **C54 phase** (lower resistivity). This transformation increases grain size. In narrow leads, the dimensions are insufficient to accommodate the larger grains, so the high-resistivity C49 phase persists. This is why cobalt and nickel silicides are preferred for sub-$0.5\,\mu\text{m}$ technologies.

**Poly routing:** The resistance of silicided poly, while much higher than metal, is low enough to make it an attractive secondary routing material. Many digital standard cells include significant poly routing. Manual layout designers may use poly to route through congestion points, though most automated routers do not optimize for poly routing.

### 2.7.4 Tungsten Plugs

As contact and via openings approach $\sim 1\,\mu\text{m}$, conventional aluminum step coverage drops to nearly zero, and even RBM begins to fail. Various techniques (high-temperature deposition, pressure-assisted filling) were tried without much success.

**CVD Tungsten:** The solution is **chemical vapor deposition** of tungsten using a mixture of **tungsten hexafluoride ($\text{WF}_6$) and hydrogen**. CVD deposition fills narrow holes without voiding, producing exceptionally conformal layers. (No suitable CVD chemistry exists for aluminum.)

**Tungsten plug process:**

1. Deposit RBM or TiN (promotes adhesion, protects underlying aluminum).
2. CVD-deposit tungsten thick enough to fill all via/contact openings and produce a roughly planar surface.
3. **Chemical-mechanical polishing (CMP)** back to the oxide surface, leaving only the holes filled with tungsten.
4. Deposit the next metal layer on top of the plugs.

**Advantages of tungsten plugs:**
- **Low-resistance** connection through vias/contacts (thick tungsten fill).
- **Planar surface** eliminates the need for metal to overlap all sides of the via --- only two sides usually require overlap.
- **Stacked contacts and vias** become possible (direct vertical stacking through multiple layers).
- Enables **submicron** metal widths, spacings, contacts, and vias.

**Disadvantages:**
- Generally prevents arbitrarily sized/shaped contacts and vias.
- Adds process steps and cost.
- Typically employed only when openings shrink below $\sim 1\,\mu\text{m}$.

### 2.7.5 Dielectrics and Planarization

A typical **double-level-metal (DLM)** system of the 1980s (Figure 2.37) uses:
- LOCOS field oxide
- Single phosphorous-doped poly layer
- **Multilevel oxide (MLO):** A thin BPSG layer insulating the poly and thickening oxide over moat regions
- Silicided contacts + RBM + Cu-doped aluminum (metal-1)
- **Interlevel oxide (ILO):** Deposited oxide between metal layers
- Second metal: RBM + Cu-doped aluminum
- **Protective overcoat (PO):** Compressive nitride

This system requires **six masking steps:** poly, contact, metal-1, via, metal-2, and PO removal (POR).

**ILO materials:** Low-temperature deposited oxides such as TEOS-based films. Thicker ILO reduces parasitic capacitance between conductor layers but worsens step coverage in vias.

**Topography and planarization:** Each patterned layer introduces height variations. Modern photolithography has an extremely narrow depth of field and cannot tolerate surface nonplanarity. Three planarization techniques have evolved:

1. **Spin-on glass (SOG):** Deposited as a liquid film whose surface tension fills recesses more than elevated areas. Multiple applications progressively reduce nonplanarity.
2. **Resist etch-back:** A resist layer is spun and baked, then plasma-etched in a chemistry that attacks both resist and oxide. The highest oxide areas are exposed longest and erode most.
3. **Chemical-mechanical polishing (CMP):** Uses an alkaline slurry of fine, soft abrasive particles. The alkaline solution softens exposed oxide so abrasive particles can scrub it away. CMP selectively attacks the highest topography while leaving recessed areas untouched. It is vastly superior to SOG and resist etch-back.

**Dishing:** CMP is not perfect. Large recessed areas leave depressions because the flexible abrasive pad dips into them. The cure is adding arrays of unconnected **dummy metal or poly geometries** in regions devoid of these materials. Dummy geometries can be automatically generated without designer intervention.

**Modern DLM with CMP and tungsten plugs (Figure 2.38):**
- **Shallow trench isolation** replaces LOCOS (trenches cut, filled with oxide, CMP-polished smooth).
- After poly patterning, titanium is sputtered and annealed to **clad source/drain and poly** with titanium silicide.
- MLO deposited, contacts etched to expose silicide.
- RBM deposited as diffusion barrier and adhesion promoter, then **tungsten plug** CVD deposition + CMP.
- Metal-1: RBM + Cu-doped Al + **TiN antireflective coating (ARC)** (minimizes light scatter during patterning).
- ILO deposited, vias etched, RBM deposited, tungsten plugs formed by CVD + CMP.
- Metal-2: RBM + Cu-doped Al + ARC.
- Compressive nitride PO with bondpad openings.

This requires **seven masks** (adding silicide block for resistors): poly, contact, metal-1, via, metal-2, POR, silicide block.

**Protective overcoat notes:**
- Early processes used PSG or tensile nitride; newer ones prefer **compressive nitride** (hard, strong, impervious to mobile ionic contaminants like sodium).
- Some processes use PSG or oxynitride PO because conventional plasma-deposited $\text{Si}_3\text{N}_4$ is opaque to UV light, which is incompatible with UV-erasable devices.
- Without PO, exposed aluminum leads are so fragile that sliding against paper will destroy them.

### 2.7.6 Copper

By the late 1990s, aluminum metallization had been pushed to its limits. At submicron dimensions and tens-of-amps currents, aluminum resistance becomes significant and electromigration becomes a serious concern.

**Copper advantages:**
- **35% lower resistivity** than aluminum
- **Order-of-magnitude improvement** in electromigration resistance

**Two impediments to copper adoption:**
1. Copper easily oxidizes and readily **diffuses through oxide and silicon**. A diffusion barrier (typically **tantalum nitride, TaN**) must surround the copper.
2. Dry etching of copper leaves **nonvolatile residues**, making conventional subtractive patterning impossible.

#### Damascene Patterning

The etching problem was solved by **damascene patterning** --- instead of depositing metal and then etching it, trenches are etched into the dielectric and then filled with metal.

**Single damascene process (Figure 2.39):**

1. Form conventional tungsten plug vias; CMP-planarize the surface.
2. Deposit the ILO layer.
3. Etch **trenches** in the oxide where copper leads are desired.
4. Sputter **tantalum/tantalum nitride** diffusion barrier across the wafer.
5. Deposit a thin **CVD copper seed layer** for subsequent electroplating.
6. **Electrolytically deposit** the remaining copper (fills the trenches).
7. **Electrochemical-mechanical polish (ECMP)** removes all copper above the ILO surface, leaving only inlaid "damascene" copper lines.
8. Deposit a **silicon nitride diffusion cap** to prevent copper from diffusing into the next ILO layer.

**Dual damascene process (Figure 2.40):** Simultaneously fabricates both copper metal and copper plug vias:

1. Deposit blanket CVD nitride (etch stop).
2. Deposit blanket oxide (future ILO).
3. Etch trenches through oxide (stopping on nitride etch stop).
4. Second mask opens holes in nitride where vias are desired.
5. Anisotropic RIE drills vias down to the first metal layer.
6. Deposit TaN barrier + seed copper + electrolytic copper.
7. ECMP to remove excess copper.
8. Deposit nitride diffusion cap.

The dual damascene process eliminates the separate tungsten deposition sequence for plugs and provides lower-resistivity copper plug vias.

#### Power Copper

For currents of tens of amps, even thick damascene copper layers ($\sim\text{several}\,\mu\text{m}$) are insufficient. The **power copper** process provides $\sim 10\,\mu\text{m}$ thick copper:

1. Start with a fully processed wafer including PO (but PO openings are vias to power copper, not bondpads).
2. Sputter a thin copper seed layer.
3. Electrolytically deposit $\sim 10\,\mu\text{m}$ of copper.
4. Plate thin nickel then palladium layers (inhibit oxidation, provide wirebondable surface).
5. Wet etch patterns the power copper leads.

**Bond Over Active Circuitry (BOAC):** The thick copper cushions the IC from wirebonding forces, so bondpads can be placed directly over active circuitry and bonded with large-diameter copper wire without damage.

**Power copper drawbacks:**
- **Thermal expansion mismatch** between thick copper and the rest of the chip causes stress (worse with larger dies).
- **No insulating overcoat** allows sawing debris to short between exposed leads.
- Solutions: moderate copper thickness; apply a patterned polyimide insulating layer.

#### Copper Pillars

Copper pillars provide a bondwire-free chip-to-substrate connection with contact dimensions down to $\sim 40\,\mu\text{m}$:

1. Deposit and etch thick copper pillars.
2. Cap each pillar with a $\sim 25\,\mu\text{m}$ tin-silver solder alloy (lead-free).
3. Invert the chip onto the substrate and heat to reflow the solder, joining pillars to substrate copper traces.

The substrate is essentially a thin multilayer PCB embedded inside a plastic package.

---

## Diagrams

### Figure 2.31 -- Single-Level Metal Interconnection System

![[diagrams/ch02-interconnection-fig1.png]]

**Caption:** Formation of a single-level metal (SLM) interconnection system. FEOL processing leaves an oxide layer across the wafer. Contact windows are etched, aluminum is deposited and patterned, and a protective overcoat (PO) is applied with bondpad openings. This is the simplest interconnection scheme; modern processes stack multiple metal layers using the techniques described in this section.

### Figure 2.38 -- Modern DLM with CMP and Tungsten Plugs

![[diagrams/ch02-interconnection-fig2.png]]

**Caption:** Cross section of a double-metal, single-poly metallization system using CMP planarization and tungsten vias. This represents a significant evolution from the basic SLM system: shallow trench isolation replaces LOCOS, silicided contacts replace simple aluminum sintering, tungsten plug CVD replaces sputtered aluminum vias, and CMP provides rigorous surface planarization. The stack includes RBM, Cu-doped Al, TiN ARC, and compressive nitride PO.

### Figure 2.40 -- Dual Damascene Copper Process

![[diagrams/ch02-interconnection-fig3.png]]

**Caption:** Steps involved in the formation of the second layer of a dual damascene copper metallization system. This technique simultaneously creates both the copper metal leads (in etched trenches) and copper plug vias, eliminating the separate tungsten deposition step. Key features include the TaN diffusion barrier, CVD copper seed layer, electrolytic copper fill, and ECMP planarization.

---

## Practical Takeaways

- **Never route current-carrying signals through long runs of polysilicon.** Even silicided poly has much higher resistance than true metal. Use poly routing only for short connections or congestion relief.
- **Contact spiking is mitigated by aluminum-silicon alloys** (a few percent Si), but careful sinter temperature/time control is still required because silicon tends to segregate out of the alloy.
- **Electromigration resistance improves by ~10x** with the addition of a fraction of a percent of copper to aluminum alloys. Always check current density against electromigration design rules (see Section 5.1.3).
- **Reflow can only be applied to contacts, not vias,** because the temperature required to soften PSG/BPSG would damage underlying aluminum layers.
- **Titanium silicide is unsuitable for leads narrower than ~0.5 um** due to the C49-to-C54 phase transformation issue. Use cobalt or nickel silicide for sub-half-micron technologies.
- **Use a silicide block mask** wherever polysilicon resistors are needed. Without it, silicidation makes all poly low-resistance, destroying resistor functionality.
- **CMP dishing is controlled by dummy fill.** In layout, be aware that large open areas without metal or poly will cause CMP non-uniformity. Modern DRC/DFM tools automatically insert dummy geometries.
- **Stacked contacts and vias** are enabled by tungsten plug technology --- exploit this for dense layouts. However, tungsten plugs generally require fixed, regularly sized openings (no arbitrary shapes).
- **Copper requires a diffusion barrier on all sides** (typically TaN) plus a nitride diffusion cap on top. Unlike aluminum, copper cannot touch oxide or silicon directly --- it will diffuse through them and poison devices.
- **Power copper enables BOAC** (bond over active circuitry), allowing bondpads to be placed over transistors. This can dramatically reduce die area in power ICs.
- **Copper pillars** enable flip-chip packaging with contact dimensions as small as ~40 um, suitable for high-density I/O applications.

---

## Relation to the Bigger Picture

Section 2.7 covers the BEOL interconnection system, which is the bridge between the active devices formed in earlier FEOL sections (diffusion, implantation, isolation --- see [[ch02-isolation]]) and the packaged IC (see [[ch02-assembly]]). Understanding interconnection is essential for analog layout because parasitic resistance and capacitance introduced by the wiring directly impact circuit performance: metal choice affects current-carrying capacity and electromigration limits, dielectric properties govern parasitic capacitance, and planarization quality determines yield at advanced nodes. Many of the layout rules encountered later in the book (Chapters 3--5) --- contact sizing, metal width for current handling, via stacking constraints, dummy fill requirements --- trace directly back to the fabrication realities described here.

---

## See Also
- [[ch02-isolation]]
- [[ch02-assembly]]
