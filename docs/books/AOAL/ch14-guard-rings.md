---
title: "14.2 Minority-Carrier Guard Rings"
chapter: 14
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-14, guard-rings, latchup, minority-carriers]
---

# 14.2 Minority-Carrier Guard Rings

> **Chapter 14: Special Topics**

## Key Concepts

Minority-carrier guard rings are the primary layout defense against **latchup** in integrated circuits. Latchup is one of the most elusive failure modes -- a device may operate correctly in one application but latch up in another, sometimes only after hundreds or thousands of hours. Simulation rarely catches these problems, and most forms of testing miss them as well.

### What Causes Latchup

The most frequent trigger for latchup is **external transients** that pull device pins above $V_{DD}$ or below ground. Common sources include:

- Low-level ESD events
- Momentary power interruptions
- Inductive kick-back from relays, motors, and solenoids
- Inductive spiking of rapidly switched signals

When a pin swings outside the supply rails, any diffusion connected directly to that pin can **inject minority carriers** into the substrate or well. Even diffusions connected through deposited resistors pose a concern if the series resistance is less than roughly $1\,\text{k}\Omega$ (larger resistances redirect transients through ESD protection devices). Certain internal circuits can also trigger latchup -- particularly **charge pumps** and any circuits containing a capacitor connected to a diffusion where a large voltage swing can forward-bias a PN junction.

### The Guard Ring Strategy

The fundamental approach is to **enclose each injecting device** with a suitable minority-carrier guard ring. The alternative -- enclosing all *vulnerable* devices -- is usually impractical because there are far more potentially vulnerable devices than injectors. ESD devices around the die periphery can often share a common guard ring that separates the core circuitry from the bondpads and ESD structures.

There are four fundamental types of minority-carrier guard rings:

| Guard Ring Type | Abbreviation | Carrier Targeted | Mechanism |
|---|---|---|---|
| Electron-Collecting Guard Ring | ECGR | Electrons | Reverse-biased N-type junction collects electrons from substrate |
| Electron-Blocking Guard Ring | EBGR | Electrons | $N^+/P^-$ interface field repels electrons |
| Hole-Collecting Guard Ring | HCGR | Holes | Reverse-biased P-type junction collects holes from well |
| Hole-Blocking Guard Ring | HBGR | Holes | $N^+/P^-$ interface field blocks holes |

The theory of how these structures work is detailed in Section 5.4.4, and the relationship between doping concentrations and blocking efficiency is captured by Equation 5.32.

## Standard Bipolar Guard Rings

### 14.2.1 Standard Bipolar Electron Guard Rings

Any tank connecting to a pin can inject electrons into the substrate. Standard bipolar **lacks** the layers needed for an electron-blocking guard ring (EBGR), but it can build an **electron-collecting guard ring (ECGR)**.

**Recommended Structure (Figure 14.10A):** A strip of deep-$N^+$ residing in an N-tank, augmented by both **NBL** (N-type buried layer) and **emitter** diffusion. This combination forms the deepest possible guard ring and therefore collects the largest fraction of electrons. The deep-$N^+$ also prevents debiasing.

**Connection guidelines:**
- **Ideally connect to the highest available supply voltage** -- this drives the depletion region as deeply as possible into the substrate and minimizes debiasing
- The guard ring *will* function if connected to ground, but becomes shallower and much more susceptible to debiasing
- Grounded guard rings are sometimes used in high-voltage, high-current designs to minimize power dissipation from minority carrier injection
- If a grounded ECGR might saturate, enclose it in a second ECGR connected to a supply

**Alternate Structure (Figure 14.10B):** Used only when deep-$N^+$ is unavailable. The vertical resistance of the epi layer separating NBL from the emitter diffusion makes this structure extremely vulnerable to debiasing. It can collect significant current when connected to a supply, but not when connected to ground.

**Key limitation:** Standard bipolar ECGRs are only **marginally effective** due to the lack of a $P^+$ substrate. Many designers omit them and instead rely on large spacings and strategically placed hole guard rings. This typically suffices for linear circuits (op-amps, voltage regulators) but is inadequate for inductive load drivers and MOSFET gate drivers.

**$P^+$ substrate enhancement:** If the process includes a $P^+$ substrate, the $P^+/P^-$ interface generates an electric field that traps most injected electrons in the $P^-$ epi. Those few that penetrate into the heavily doped substrate quickly recombine, making deep guard rings extremely effective.

### 14.2.2 Standard Bipolar Hole Guard Rings

Any P-type region that forward biases into a tank injects holes. Two types of hole guard rings exist:

**Hole-Collecting Guard Ring (HCGR, Figure 14.11A):**
- Placed to prevent holes from reaching tank sidewalls
- NBL prevents holes from flowing down to the substrate via the $N^+/P^-$ interface built-in potential
- Consists of a reverse-biased base diffusion acting as the collector of a lateral PNP
- Normally grounded or connected to a negative supply; large reverse bias drives the depletion region deeper
- Collection efficiency approaches unity even without reverse bias (cross sections are vertically exaggerated)
- Can be **merged with isolation** to save space (e.g., the P-bar structure of Figure 5.38) if current is below a few milliamps and substrate contacts are included
- Guard rings expecting large currents should **not** be merged with isolation due to debiasing

**Hole-Blocking Guard Ring (HBGR, Figure 14.11B):**
- Surrounds the injection point with heavily doped N-type regions
- The $N^+/P^-$ interface generates an electric field blocking hole influx, forcing recombination in the N-epi
- Requires NBL to block downward diffusion and deep-$N^+$ to block lateral diffusion
- Must **completely encircle** the injector with deep-$N^+$
- Drawn NBL should extend to the outside edge of drawn deep-$N^+$ to maximize doping at the junction
- Efficiency depends on doping ratio of adjacent $N^+$ and $P^-$ regions (Section 5.4.4)
- In standard bipolar (lightly doped N-epi), efficiency almost always exceeds **95%**, often exceeding **98%**
- Combining HCGR inside HBGR achieves efficiencies exceeding **99%**

Standard bipolar designs seldom experience latchup from hole injection into the substrate because deep-$N^+$ isolation and large component spacings reduce the beta product of the parasitic SCR.

## CMOS Guard Rings

### 14.2.3 CMOS Electron Guard Rings

CMOS processes are **generally more prone to latchup** than standard bipolar. Single-well CMOS uses lightly doped backgates; dual-well processes dope more heavily but pack components very tightly. Most CMOS processes use a $P^+$ substrate to reduce lateral substrate resistance, and P-well processes often employ **megavolt ion implantation** for a retrograde well profile (bottom doped more heavily than top -- similar effect to a buried layer without extra epitaxial deposition).

**Majority-carrier guard rings (a misnomer):** CMOS designers often ring NMOS transistors with substrate contacts, sometimes called "guard rings." These do **not** block or collect minority carriers -- they provide majority carriers to support recombination. They are properly called **majority-carrier guard rings** and work by pinning the backgate potential to prevent source-backgate junction forward bias.

**NMoat ECGR (Figure 14.12A):**
- NSD implant is shallow and in modern processes recessed into field oxide, so electrons can burrow underneath
- CMOS supply voltages are too low to deplete significantly into the backgate
- The only way to improve collection efficiency is to **widen the NMoat strip**:
  - Without retrograde P-well: width $\geq$ P-epi thickness
  - With retrograde P-well: width $\geq$ P-well depth

**N-well ECGR (Figure 14.12B):**
- A strip of NMoat contacts the N-well
- Maximize NMoat coverage to reduce vertical resistance through the N-well
- Connect to a **power supply** (not ground) to minimize debiasing
- Deep, lightly doped N-wells are of questionable use as guard rings due to high vertical resistance
- Shallower, more heavily doped N-wells in modern low-voltage CMOS are more useful
- If only grounded ECGRs are allowed, prefer **wide NMoat ECGR** over N-well ECGR

### 14.2.4 CMOS Hole Guard Rings

**Older-generation CMOS** processes lack strongly retrograde well profiles, so nothing constrains hole diffusion down to the substrate -- hole guard rings **cannot** be constructed.

**Newer low-voltage CMOS** processes with sufficiently shallow N-wells can create retrograde profiles via megavolt implants. If the bottom of the well is at least **10x** more heavily doped than the middle, useful hole guard rings become feasible.

**Key limitation:** Pure CMOS lacks deep-$N^+$ sinker and deep trench, so there is **no way to block** hole diffusion to N-well sidewalls. This prevents construction of hole-blocking guard rings.

**PMoat HCGR (Figure 14.13):**
- Ring of PMoat around the injector
- Limited effectiveness because PMoat does not reach the heavily doped portion of the well
- Improve by: increasing reverse bias (connect to ground), and increasing ring width
- Ideal PMoat ring width $\geq$ N-well depth

In practice, CMOS designers often rely on **rings of NMoat backgate contacts** around PMOS transistors rather than hole-collecting guard rings. These majority-carrier guard rings pin the backgate potential to minimize forward biasing of source-backgate junctions.

## BiCMOS Guard Rings

### 14.2.5 BiCMOS Electron Guard Rings

BiCMOS processes can fabricate the same ECGRs as CMOS, plus additional more effective structures depending on available layers.

**CDI BiCMOS ECGR (Figure 14.14A):**
- Two epitaxial layers on a $P^+$ substrate with patterned NBL between them
- Guard ring = ring of NBL contacted by deep-$N^+$
- NBL does not penetrate to $P^+$ substrate, but boron updiffusion partially closes the gap
- Observed efficiencies exceeding **90%**
- Includes NMoat for Ohmic contact and N-well to slightly reduce vertical resistance
- With deep-$N^+$: can be grounded without debiasing fear, even at standard latchup test currents ($\pm 100\,\text{mA}$)
- Without deep-$N^+$: must connect to supply, and vertical resistance must be computed for expected currents

**Isolated NMOS (Figure 14.14B):**
- NMOS resides inside a tank formed by P-epi cut off by NBL and deep-$N^+$
- Creates a **100%-efficient** ECGR surrounding the NMOS transistor
- Twin-well CDI BiCMOS uses the same structure plus a P-well around the NMOS

**DTI BiCMOS ECGR (Figure 14.14C):**
- Two $P^-$ epi layers on $P^+$ substrate with patterned NBL between them
- Deep trench penetrates through NBL to cut off P-epi sections
- Tilted arsenic implants along trench sidewalls contact NBL
- NBL + tilted arsenic implant = **100%-efficient** ECGR
- If injector cannot be contained in a tank, a strip of NBL contacted by deep trench serves as an ECGR

**Critical warning on $P^-$ substrates:** Some analog BiCMOS processes use a $P^-$ substrate to avoid growing two epitaxial layers. Designs on $P^-$ substrate are **far more susceptible to latchup** because:
- The substrate is easily debiased
- ECGRs become far less effective without the $P^+/P^-$ interface
- A significant percentage of such designs require additional passes to fix latchup issues
- No amount of guard rings and substrate contacts guarantees proper operation under severe inductive kickback or resonance

**Dielectrically isolated processes** are not immune to latchup despite common misconception. When PMOS and NMOS share a tank, latchup is a serious concern. The buried oxide creates high lateral resistances across both N-well and P-well regions. Devices known to inject minority carriers should be isolated in their own tanks, and large numbers of backgate contacts should be scattered through both well regions.

### 14.2.6 BiCMOS Hole Guard Rings

**CDI BiCMOS HBGR (Figure 14.15A):**
- NBL placed beneath N-well with deep-$N^+$ encircling it
- Drawn NBL should extend to the outside edge of drawn deep-$N^+$
- Drawn N-well should also extend at least to outside edge of drawn deep-$N^+$
- Effectiveness depends on maintaining a doping ratio of approximately **100:1** or greater between $N^+$ and $P^-$ sides
- **Problem area:** Where shallow N-well surface joins deep-$N^+$ -- surface doping of modern low-voltage N-wells can exceed $10^{17}\,\text{cm}^{-3}$, requiring deep-$N^+$ core doping above $10^{19}\,\text{cm}^{-3}$. This is achievable with $POCl_3$ deposition but beyond conventional ion implantation. Without adequate doping ratio, holes slip through deep-$N^+$ sidewalls.
- This problem has been observed in processes combining sub-5V wells with ion-implanted deep-$N^+$ sinkers

**Hole permeability through NBL** has been reported in BiCMOS processes with insufficiently doped buried layers. Adding a hole-blocking guard ring may paradoxically **increase** substrate injection through a permeable NBL by reducing the effective volume of the N-well (related to $V$ in the denominator of Equation 5.32).

**Twin-well BiCMOS HCGR (Figure 14.15B):**
- Process uses two $P^-$ epi layers on $P^+$ substrate with blanket boron PBL and patterned antimony NBL
- PBL between shallow N-well and NBL permits substantial voltage difference without punchthrough
- Creates a **doubly isolated PMOS** transistor structure
- Guard ring = PBL beneath N-well + P-well surrounding it = **100%-effective** HCGR
- PMoat ring in P-well provides contact; normally connected to ground
- Remains modestly effective even when connected to the deep-$N^+$ isolation ring
- Widen PMoat and increase spacing between deep-$N^+$ and enclosed N-well to reduce vertical resistance

**DTI BiCMOS HBGR (Figure 14.15C):**
- Deep trench + NBL form the hole-blocking guard ring
- Ring of deep trench around N-well edge prevents lateral hole escape
- NBL coded across entire tank; drawn NBL edge should extend at least to outside drawn edge of deep trench
- P-epi between shallow N-well and NBL has no effect on hole permeation through NBL
- Holes diffusing downward through N-well to P-epi debias it; both P-epi/N-well and P-epi/NBL junctions forward-bias equally, but more hole current flows across P-epi/N-well interface (N-well is more lightly doped than NBL)
- Observed hole permeation less than **5%** with floating P-type region

## Diagrams

### Figure 14.10 -- Electron-Collecting Guard Rings for Standard Bipolar

![[diagrams/ch14-guard-rings-fig1.png]]

Cross-sections of standard bipolar ECGRs. **(A)** Recommended structure using deep-$N^+$, NBL, and emitter in an N-tank -- this forms the deepest possible guard ring. **(B)** Alternate structure for processes without deep-$N^+$ -- more vulnerable to debiasing due to vertical epi resistance between NBL and emitter.

### Figure 14.11 -- Hole Guard Rings for Standard Bipolar

![[diagrams/ch14-guard-rings-fig2.png]]

Cross-sections of standard bipolar hole guard rings. **(A)** Hole-collecting guard ring (HCGR) using a reverse-biased base diffusion acting as a lateral PNP collector, with NBL repelling holes downward. **(B)** Hole-blocking guard ring (HBGR) using heavily doped N-type regions (NBL + deep-$N^+$) to create an electric field that blocks hole influx.

### Figure 14.14 -- Electron-Collecting Guard Rings in BiCMOS Processes

![[diagrams/ch14-guard-rings-fig3.png]]

Cross-sections of BiCMOS ECGRs. **(A)** Single-well CDI BiCMOS with NBL ring contacted by deep-$N^+$ (>90% efficiency). **(B)** Isolated NMOS in CDI BiCMOS -- NBL and deep-$N^+$ form a 100%-efficient ECGR. **(C)** DTI BiCMOS with deep trench, NBL, and tilted arsenic implants forming a 100%-efficient ECGR.

## Practical Takeaways

### General Principles
- **Guard the injector, not the victim:** Enclose each device that injects minority carriers with a guard ring, rather than trying to protect all vulnerable devices
- ESD devices at the die periphery can share a common guard ring separating the core from the bondpads
- Diffusions connected to pins through less than ~$1\,\text{k}\Omega$ of resistance are potential injectors
- Charge pumps and capacitor-connected diffusions are internal latchup triggers

### Standard Bipolar
- ECGRs are only marginally effective without a $P^+$ substrate; many designers omit them for linear circuits
- For inductive load drivers and gate drivers, use ECGRs placed as close to offending transistors as possible
- Gate driver resonance can be damped with series resistance (at the cost of switching speed)
- Hole-blocking guard rings must **completely encircle** the injector
- Combining HCGR inside HBGR achieves >99% efficiency
- Grounded HCGRs can merge with isolation if current is below a few milliamps

### CMOS
- Do not confuse backgate contact rings (majority-carrier guard rings) with true minority-carrier guard rings
- For NMoat ECGRs, make the strip at least as wide as P-epi thickness (no retrograde well) or P-well depth (retrograde well)
- Prefer NMoat ECGRs over N-well ECGRs when using grounded guard rings
- Hole guard rings require retrograde well profiles with $\geq 10\times$ doping ratio between well bottom and middle
- PMoat HCGR width should be at least equal to N-well depth

### BiCMOS
- Processes with $P^+$ substrates dramatically improve guard ring effectiveness
- **Never use $P^-$ substrates** for designs subject to inductive kickback or resonance
- Isolated NMOS structures (CDI or DTI) can achieve 100% electron collection
- For hole-blocking guard rings, verify that the doping ratio at the deep-$N^+$/N-well junction exceeds 100:1
- Watch for hole permeability through insufficiently doped NBL -- adding an HBGR can paradoxically increase substrate injection
- Even dielectrically isolated processes require latchup precautions when PMOS and NMOS share a tank

## Relation to the Bigger Picture

Minority-carrier guard rings are the layout engineer's primary weapon against latchup, which is arguably the most dangerous reliability failure mode in analog ICs. This section builds directly on the theory of minority-carrier injection presented in Section 5.4.4 and the parasitic SCR mechanisms of Section 5.4.2, translating that device physics into concrete, process-specific layout structures. The guard ring techniques here are essential context for the merged device layouts discussed in [[ch14-merged-devices]], where multiple transistors sharing tanks create opportunities for cross-injection. Understanding guard ring limitations also informs routing and spacing decisions covered in [[ch14-interconnection]], particularly when single-level metal constraints force components into closer proximity.

## See Also
- [[ch14-merged-devices]]
- [[ch14-interconnection]]
