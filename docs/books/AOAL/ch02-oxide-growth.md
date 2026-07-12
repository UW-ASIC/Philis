---
title: "2.3 Oxide Growth and Removal"
chapter: 2
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-2, oxide, LOCOS, fabrication, SiO2]
---

# 2.3 Oxide Growth and Removal

> **Chapter 2: Semiconductor Fabrication**

## Key Concepts

Silicon dioxide ($\text{SiO}_2$) is arguably the most important material in silicon IC fabrication. It can be **thermally grown** on a silicon wafer simply by heating the wafer in an oxidizing atmosphere. The resulting film has several critical properties that make it indispensable:

- **Mechanically rugged** -- resists most common solvents
- **Chemically selective** -- readily dissolves in hydrofluoric acid (HF), providing a convenient etch chemistry
- **Superb electrical insulator** -- used for capacitor dielectrics, gate dielectrics, and inter-metal insulation
- **Low surface-state charge** at the $\text{SiO}_2$ / (100) Si interface -- enables stable, predictable MOS threshold voltages

Silicon dioxide is so central to silicon processing that it is universally referred to simply as **oxide**.

The section covers four key topics: (1) how oxide is grown or deposited, (2) how it is selectively removed (etched), (3) side effects of repeated oxidation/etching cycles on topography and doping, and (4) the LOCOS technique for selective thick-oxide growth.

---

## 2.3.1 Oxide Growth and Deposition

### Native Oxide
Even momentary exposure of a silicon wafer to air causes atmospheric oxygen to react with the surface, forming a **native oxide** only a few tenths of a nanometer thick. This is far too thin for device fabrication, but it is always present.

### Thermal Oxidation (Dry and Wet)

Thick oxide films are grown by placing wafers in a **fused silica tube** wrapped in an electrical heating mantle (an oxidation furnace). Wafers sit in a fused silica rack (a **wafer boat**) and are slowly inserted into the heating zone. Oxygen gas flows through the tube and, at elevated temperatures, oxygen molecules **diffuse through the existing oxide** to reach the underlying silicon surface, where they react to grow the oxide thicker.

Because oxygen must diffuse through the already-grown oxide to reach fresh silicon, the **growth rate decreases with time** -- thicker oxides grow progressively more slowly.

**Dry oxide:** Grown using pure dry $\text{O}_2$. Produces the highest-quality films (fewest defects, lowest fixed charge). Used for gate dielectrics and other critical insulating layers.

**Wet oxide:** Formed by injecting **steam** into the furnace tube. Water vapor diffuses through oxide much more rapidly than molecular oxygen, so wet oxidation is significantly faster. However, hydrogen atoms liberated from $\text{H}_2\text{O}$ decomposition create imperfections that can degrade oxide quality (increased fixed oxide charge). Wet oxidation is commonly used to grow **thick field oxide** where no active devices reside.

### Growth Rate Data

The following table shows times required to grow $1\ \mu\text{m}$ of oxide on undoped silicon at various temperatures:

| Conditions | 800 C | 900 C | 1000 C | 1100 C |
|---|---|---|---|---|
| (100) Si in dry $\text{O}_2$ | 63 hr | 11 hr | 2.5 hr | 43 min |
| (111) Si in dry $\text{O}_2$ | 30 hr | 6 hr | 1.6 hr | 30 min |
| (100) Si in wet $\text{O}_2$ | 2.8 hr | 43 min | 9 min | -- |
| (111) Si in wet $\text{O}_2$ | 1.7 hr | 25 min | 6 min | -- |

Key observations:
- **Temperature dramatically accelerates growth** -- going from 800 C to 1100 C reduces dry oxidation time by roughly two orders of magnitude.
- **(111) silicon oxidizes faster than (100) silicon** -- due to the higher density of surface atoms available for bonding on the (111) plane.
- **Wet oxidation is roughly 10x faster** than dry oxidation under the same conditions.

### High-Pressure and Rapid Thermal Oxidation

- **High-pressure oxidation:** Oxidation time varies inversely with pressure. At 5 atm, growing $1\ \mu\text{m}$ of wet oxide on (100) Si at 1000 C takes only ~50 minutes versus ~4 hours at 1 atm. This technique reduces processing time or enables lower temperatures.
- **Rapid thermal oxidation (RTO):** Uses powerful tungsten-halogen lamps to ramp wafer temperature to ~1000 C in seconds. Enables production of high-quality gate oxides less than 10 nm thick, which is difficult in conventional tube furnaces due to the slow gas-switching dynamics.

### Deposited Oxides

When oxide must be formed on a material **other than silicon** (e.g., between metallization layers), thermal growth is not possible. Instead, **deposited oxides** are used:

- **CVD oxides:** Produced by reactions between gaseous silicon compounds and oxidizers (e.g., dichlorosilane + nitrous oxide).
- **Spin-on glass (SOG):** An organosilicon compound in solution is spun onto the wafer and decomposed by heating. The most common formulation is based on **tetraethoxysilane (TEOS)**, also known as tetraethyl orthosilicate.

### Doped Oxides

Deposited oxides are often doped to tailor their properties:

- **Phosphosilicate glass (PSG):** Adding phosphorus immobilizes sodium ions (preventing them from migrating into gate dielectrics) and lowers the softening temperature, enabling **reflow** -- a process where heating smooths out sharp corners in patterned oxide.
- **Borophosphosilicate glass (BPSG):** Includes both boron (~4%) and phosphorus (~4%) to achieve reflow at 800-900 C without the aluminum corrosion problems of high-phosphorus PSG.

### Thin-Film Interference Colors

Oxide films are brightly colored due to **thin-film interference** -- destructive interference selectively absorbs certain wavelengths. This produces the vivid iridescent colors visible in microphotographs of older-generation ICs. The approximate thickness of an oxide film can be estimated from a table of oxide colors. Modern ICs, however, appear relatively drab due to surface planarity requirements and dense dummy metal patterns.

---

## 2.3.2 Oxide Removal

The process of creating a patterned oxide involves six steps (illustrated in Figure 2.8):

1. **Oxidation** -- grow a thin oxide layer across the wafer
2. **Photoresist spin** -- apply positive photoresist
3. **Exposure** -- photolithographic exposure through a mask
4. **Development** -- dissolve exposed photoresist to open windows
5. **Oxide etch** -- selectively remove oxide in the windows (also called **oxide removal**, OR)
6. **Photoresist strip** -- remove remaining photoresist

![[diagrams/ch02-oxide-growth-fig1.png]]
*Figure 2.8: Steps in oxide growth and removal -- the fundamental patterning sequence for creating oxide windows.*

### Wet Etching

- Uses **buffered oxide etch (BOE)**: hydrofluoric acid (HF) buffered with ammonium fluoride ($\text{NH}_4\text{F}$).
- BOE readily dissolves $\text{SiO}_2$ but does **not attack** elemental silicon or organic photoresists.
- Process: immerse wafers in a plastic tank of HF solution for a controlled time, then rinse thoroughly.
- **Safety critical:** both the etchant and its gaseous byproducts are extremely toxic and corrosive.
- **Isotropic** -- etches laterally at the same rate as vertically, causing **undercutting** beneath the photoresist edges. The etch must continue long enough to clear all openings, so some overetch is inevitable, producing sloped sidewalls.
- Cannot provide the tight linewidth control needed for modern submicron processes.

### Dry Etching

Three classes: **reactive ion etching (RIE)**, **plasma etching**, and **chemical vapor etching**.

**Reactive Ion Etching (RIE):**
- A silent electrical discharge through a low-pressure gas mixture creates highly energetic charged molecular fragments (reactive ions).
- Ions are projected **downward** onto the wafer at high velocity, impacting at a steep angle.
- This produces **anisotropic** etching -- vertical etch rate far exceeds lateral etch rate, yielding nearly vertical sidewalls.
- Etch gas: typically a fluorocarbon compound (e.g., hexafluoroethane, $\text{C}_2\text{F}_6$) mixed with an inert gas (argon). The reactive ions selectively attack $\text{SiO}_2$ over photoresist or elemental silicon.
- Different gas mixtures enable anisotropic etching of other materials (silicon nitride, elemental silicon).

**Plasma etching** resembles RIE but without a voltage differential to accelerate ions -- they simply diffuse through the plasma.

Modern processes **rely on dry etching** for submicron geometries. The increased packing density and performance of anisotropic structures more than compensates for the added complexity and cost.

---

## 2.3.3 Other Effects of Oxide Growth and Removal

### Surface Topography -- Oxide Steps

Repeated cycles of oxidation, patterning, and etching cause the silicon surface to become **highly nonplanar**. This is problematic because modern fine-line photolithography has a very narrow depth of field.

When a planar silicon surface is oxidized, patterned, and etched to form oxide openings, subsequent thermal oxidation creates an **oxide step**:

- The bare silicon in the opening oxidizes rapidly, while surfaces already coated with oxide oxidize more slowly.
- The silicon surface **erodes by about 45% of the oxide thickness grown** (this is the inverse of the Pilling-Bedworth ratio of silicon, which equals 2.2). Silicon under the previous opening therefore recedes to a greater depth.
- The oxide in the old opening is always thinner than the surrounding oxide (which already had some oxide when regrowth began).
- The combination of different oxide thicknesses and different silicon surface depths creates the characteristic **oxide step** discontinuity.

![[diagrams/ch02-oxide-growth-fig3.png]]
*Figure 2.11: Effects of patterned oxidation on wafer topography, showing oxide step formation (top). Figure 2.12: Dopant segregation mechanisms -- boron suckup (A) and phosphorus plow (B) (bottom).*

### Dopant Segregation During Oxidation

Thermal oxide growth also redistributes dopants in the underlying silicon. The behavior depends on the **relative solubility** of the dopant in oxide versus silicon:

**Boron suckup:** Boron is **more soluble in oxide than in silicon**. During oxidation, boron migrates from the silicon into the growing oxide, **depleting** the silicon surface of dopant. This lowers the effective surface concentration.

**Phosphorus plow (pileup):** Phosphorus, arsenic, and antimony are **more soluble in silicon than in oxide**. The advancing oxide-silicon interface pushes these dopants ahead of it, causing a **localized increase** in doping concentration near the surface. For phosphorus, this accumulation is sometimes called phosphorus pileup or phosphorus plow.

These segregation mechanisms complicate the design of dopant profiles for integrated circuits.

### Dopant-Enhanced Oxidation

Heavy doping also affects the **rate** of oxide growth:

- High levels of **arsenic or phosphorus** substantially increase oxidation rates, especially for thin oxides.
- High **boron** levels have a smaller but still noticeable effect.
- The mechanism: high doping levels introduce **lattice vacancies** that catalyze oxidation, accelerating growth of the overlying oxide.

This effect is called **dopant-enhanced oxidation**. A practical example is the thickening of field oxide over the $N^+$ sinker diffusion in certain older BiCMOS processes. This thicker oxide has historically been exploited to **reduce parasitic capacitance** between deposited devices and the substrate.

---

## 2.3.4 Local Oxidation of Silicon (LOCOS)

LOCOS is a technique for **selectively growing thick oxide** in specific regions of the wafer. It is fundamental to CMOS and BiCMOS process isolation.

### The LOCOS Process

The process relies on **silicon nitride** ($\text{Si}_3\text{N}_4$) as a high-temperature oxidation mask. Nitride is normally deposited by CVD (e.g., silane + ammonia). The five steps are:

1. **Pad oxidation** -- Grow a thin thermal oxide (pad oxide) on the silicon surface. This is necessary because nitride cannot be deposited directly on silicon -- mismatches in thermal expansion coefficients would cause silicon defects during subsequent heating.
2. **Nitride deposition** -- Deposit a thick CVD nitride layer over the pad oxide.
3. **Nitride etch** -- Pattern and etch the nitride to open windows where thick oxide is desired.
4. **LOCOS oxidation** -- Perform wet oxidation. The nitride blocks oxygen and water diffusion, so oxidation occurs **only** in the nitride windows. Some oxidant diffuses a short distance under the nitride edges, producing the characteristic curved transition region called a **bird's beak**.
5. **Nitride strip** -- Remove the remaining nitride to reveal the patterned thick oxide.

![[diagrams/ch02-oxide-growth-fig2.png]]
*Figure 2.14: The five-step LOCOS process sequence (top). Figure 2.15: The Kooi effect -- nitride contamination under the bird's beak (A) leads to defective gate oxide in subsequent oxidation (B) (bottom).*

### Moat Regions and Field Oxide

CMOS and BiCMOS processes use LOCOS to grow a **thick field oxide** over electrically inactive regions. The areas **not** covered by field oxide are called **moat regions** -- they form shallow depressions in the wafer topography. A very thin, high-quality **gate oxide** is subsequently grown in the moat regions to serve as the gate dielectric of MOS transistors.

### The Kooi Effect

A subtle but important complication arises during LOCOS:

1. The water vapor used in wet LOCOS oxidation attacks the nitride surface, producing **ammonia** ($\text{NH}_3$).
2. Some ammonia diffuses beneath the pad oxide near the nitride window edges and reacts with silicon to form a thin layer of **silicon nitride** at the Si-SiO$_2$ interface.
3. These nitride deposits lie beneath the pad oxide and survive both LOCOS nitride stripping and pad oxide removal (since the etch is selective to oxide, not nitride).
4. During subsequent gate oxidation, these residual nitride deposits act as an **unintentional LOCOS mask**, retarding oxide growth around the moat edges.
5. The resulting gate oxide at these points may be **too thin to withstand the full operating voltage** -- a reliability hazard.

**Workaround:** Grow a thin **dummy gate oxide** and then strip it away before growing the real gate oxide. Because silicon nitride slowly oxidizes, this sacrificial oxidation converts the nitride residues to oxide and removes them, restoring gate oxide integrity.

### LOCOS Scaling Limits

- Standard LOCOS can isolate MOS transistors with dimensions larger than about $0.6\ \mu\text{m}$. Below this, the bird's beak encroaches too much.
- **Fully recessed LOCOS:** Etch the silicon in the nitride windows before growing oxide, aligning the field oxide surface with the moat surface to minimize the bird's beak. This extends the limit down to about $0.4\ \mu\text{m}$.
- Below $0.4\ \mu\text{m}$, **shallow trench isolation (STI)** is required (covered in Section 2.5.3).

---

## Practical Takeaways

- **Dry oxide for critical dielectrics, wet oxide for thick field regions.** The quality/speed tradeoff is the fundamental decision in oxide processing.
- **Wet etching is simple but isotropic** -- expect undercutting. For submicron features, **dry etching (RIE) is mandatory** to maintain linewidth control.
- **Every oxidation cycle consumes silicon** -- the surface erodes by ~45% of the oxide thickness grown. Repeated oxidation/etch cycles create topography (oxide steps) that can compromise photolithographic focus.
- **Dopant segregation is real and must be accounted for.** Boron depletes from the surface (suckup); phosphorus/arsenic/antimony pile up at the surface (plow). These effects shift effective surface concentrations and complicate profile design.
- **Dopant-enhanced oxidation** can be exploited (e.g., thicker field oxide over heavily doped regions reduces parasitic capacitance) or can be a nuisance if not expected.
- **LOCOS bird's beak** sets a fundamental minimum feature size for isolation. Layouts must account for the lateral encroachment of the bird's beak when sizing moat regions.
- **Always perform a dummy gate oxidation** after LOCOS to eliminate Kooi-effect nitride residues before growing the real gate oxide. Failure to do so risks thin-oxide reliability failures at moat edges.
- **BPSG reflow** is the standard technique for planarizing inter-level dielectrics. The 4% B / 4% P formulation balances low reflow temperature against aluminum corrosion resistance.

---

## Relation to the Bigger Picture

Oxide growth and removal is the engine that drives the entire patterning-based fabrication flow described in [[ch02-patterning]]. Every patterning step depends on the ability to selectively grow and etch oxide, and the side effects of oxidation (topography, dopant redistribution) directly influence how diffusions and implants behave, as covered in [[ch02-diffusion-implantation]]. The LOCOS process is the bridge between oxide processing and device isolation -- it defines the active regions (moats) where transistors live and the field regions that separate them. Understanding oxide growth kinetics, etch anisotropy, and segregation effects is essential for anyone designing analog layouts, because these phenomena determine the actual doping profiles, oxide thicknesses, and surface topography that the layout ultimately produces on silicon.

---

## See Also
- [[ch02-patterning]]
- [[ch02-diffusion-implantation]]
