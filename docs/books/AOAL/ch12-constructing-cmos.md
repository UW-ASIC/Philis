---
title: "12.2 Constructing CMOS Transistors"
chapter: 12
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-12]
---

# 12.2 Constructing CMOS Transistors

> **Chapter 12: Field-Effect Transistors**

## Key Concepts

### Substrate and Well Choices

CMOS fabrication begins with a fundamental choice of substrate. Three options exist: insulator (SOI), P-type silicon, and N-type silicon. Almost all analog CMOS and BiCMOS processes use **P-type substrates with epitaxy**, for two critical reasons:

1. **Latchup suppression**: Without an epitaxial layer, the substrate itself acts as the NMOS backgate, requiring a lightly doped substrate that is suboptimal for latchup suppression. Czochralski silicon also contains oxygen precipitates that interfere with transistor operation.
2. **Supply flexibility**: P-substrates allow NMOS backgates to connect to a common ground, which suits the dominant architecture of analog systems (single negative ground, multiple positive supplies). N-substrates would force all PMOS backgates to a common supply, which is less desirable.

Three fundamental process flows exist for epitaxial P-substrate CMOS:

- **N-well CMOS**: $P^-$ epi on $P^+$ substrate. NMOS sits in the P-epi; PMOS goes into a deep, lightly doped N-well. Historically the most popular for $5\text{V}$ processes.
- **P-well CMOS**: $N^-$ epi on $P^+$ substrate. PMOS sits in the N-epi; NMOS resides in a P-well driven through the epi to the substrate. The P-well bottom is lightly doped, causing higher vertical resistance to substrate contacts. Less desirable due to N-substrate limitations.
- **Twin-well CMOS**: Both a P-well (for NMOS) and an N-well (for PMOS) are fabricated, allowing independent optimization of doping profiles for both backgates. Required for operating voltages of $3.3\text{V}$ or less.

### Wells versus Tanks

These are often confused but differ fundamentally:

- A **well** is a deep diffusion that **counterdopes** the surrounding silicon.
- A **tank** is a region of silicon **isolated by reverse-biased junctions** (or oxide layers) constructed around it.

The distinction matters because tanks provide true electrical isolation (5-terminal or 7-terminal devices), while wells share a common backgate through the substrate. Analog BiCMOS processes typically include an N-type buried layer (NBL) that enables P-type tank construction for isolated NMOS backgates.

### Dual Gate Oxide and Multi-Well Processes

Modern CMOS processes fabricate two classes of transistors: low-voltage **core** transistors for high-speed digital logic, and higher-voltage **I/O** transistors. This requires dual gate oxide thicknesses and evolved through several process generations:

- **Triple-well** ($5\text{V}$ core / $12\text{V}$ I/O era): DNWell for I/O PMOS, SPWell and SNWell for core devices, plus P-epi for I/O NMOS.
- **Quad-well** (later, as I/O voltages dropped to $5\text{V}$): DNWell, DPWell, SNWell, SPWell.
- **Modern twin-well dual gate oxide**: Most current-generation analog CMOS processes use this simplified approach, integrating core and I/O transistors in the same wells at slight performance cost.

## Important Details

### 12.2.1 Well Coding and Pattern Generation

In twin-well processes, the layout designer typically draws only one well (usually N-well), and the complementary well is generated automatically during pattern generation. A spacing is introduced between drawn wells to raise the breakdown voltage of the PN junction between them:

```
Pwell = ~Nwell undersized by 2.0
```

This inserts $2\,\mu\text{m}$ of space between N-well and generated P-well. For multiple voltage levels, each level gets its own spacing:

```
NwellLVOS = NwellLV oversized by 2.0
NwellHVOS = NwellHV oversized by 3.0
Nwell     = NwellLV + NwellHV
PWell     = ~(NwellLVOS + NwellHVOS)
```

### 12.2.2 Isolated P-Type Tanks

Four approaches exist to create an isolated P-tank using NBL:

| Approach | Description | Vertical Resistance | Notes |
|---|---|---|---|
| **Deep N-well** | N-well driven down to intersect NBL | Quite substantial | PMOS can reside in isolation diffusion (space saving) |
| **Deep-$N^+$ sinker** | Dedicated deep-$N^+$ forms low-R connection to NBL | Low | Even a small plug greatly improves latchup immunity |
| **Up-down isolation** | Phosphorus implant before top epi layer; diffuses up to meet shallow N-well | Moderate | More compact than deep-$N^+$; PMOS can merge into isolation sidewall |
| **Deep trench isolation (DTI)** | Trenches etched through NBL, oxidized and backfilled with poly | Very low | Tilted implant on trench sidewalls provides low-R path; no outdiffusion issues |

An isolated NMOS effectively has **five terminals**: gate, source, drain, backgate, and isolation. The isolation must be biased at or above the backgate to maintain junction isolation.

A **doubly isolated PMOS** (shallow N-well inside a P-tank inside deeper isolation) has **seven terminals**: gate, source, drain, backgate, tank, isolation, and substrate. The backgate need not connect to the isolation, but punchthrough limits the voltage difference (typically $\sim 5\text{V}$, extendable to $15\text{V}+$ with a P-type buried layer).

**Critical design rule for tanks**: Each isolated NMOS that can inject electrons must be identified before layout. Transistors injecting under different conditions must reside in separate tanks. Noisy switching transistors must never share tanks with sensitive analog circuits due to **parasitic rectification** -- a mechanism where capacitively coupled noise shifts the DC operating point of nonlinear nodes (e.g., bandgap references).

### 12.2.3 Channel Stop Implants

Parasitic MOS transistors can form wherever a conductor crosses thick field oxide above an improperly doped field region. **Channel-stop implants** raise the thick-field threshold to suppress these parasitic devices.

Channel stops come in pairs:
- A **blanket boron** implant (raises NMOS thick-field threshold)
- A **patterned phosphorus** implant inside N-well regions (raises PMOS thick-field threshold)

Both diffuse downward during the high-temperature field oxidation, intersecting in a PN junction near the N-well edge. Pattern generation for the patterned channel stop:

```
NwellOS = Nwell oversized by 4.0
MoatOS  = Moat oversized by 1.0
PChst   = NwellOS - MoatOS
```

The oversizes displace the channel-stop intersection outside the drawn N-well and far enough from moat boundaries to avoid disturbing transistor characteristics.

**Important caveats about thick-field thresholds**:
- Each conductor layer has its own threshold: poly (lowest), then metal-1, metal-2, etc.
- Published thresholds are typically for metal-1, assuming designers will not route high-voltage signals in poly.
- **Poly stubs** (gates extending beyond wells) can invert underlying silicon.
- Subthreshold conduction, fringing fields, and mobile ion contamination cause leakage even below published thresholds.
- **Derate published thick-field thresholds by at least 30%** for safe design.

Submicron CMOS can often achieve adequate thick-field thresholds without channel-stop implants because higher backgate doping and lower operating voltages naturally suppress parasitic channels.

### 12.2.4 Threshold Adjust Implants

CMOS transistors need nominal thresholds of at least $\sim 0.4\text{V}$ to avoid excessive subthreshold conduction, but not so large as to restrict effective gate voltage. Process designers typically target $0.60$ to $0.80\text{V}$.

A P-type (boron) implant into the channel shifts the threshold more positive (or less negative). This simultaneously **strengthens NMOS** (raising $|V_{th}|$) and **weakens PMOS** (reducing $|V_{th}|$). The threshold adjust modifies the $Q_{ss}$ term in the threshold equation.

For a typical $5\text{V}$ N-well process with $N^+$ poly gates, natural thresholds might be $V_{th,NMOS} \approx +0.20\text{V}$ and $V_{th,PMOS} \approx -1.20\text{V}$. A $+0.60\text{V}$ boron adjust yields adjusted thresholds of $+0.80\text{V}$ and $-0.60\text{V}$.

**Natural (native) transistors**: Those that do not receive the threshold adjust implant. Many processes offer these as options via a `NatVt` mask layer drawn around the gate region of each natural transistor, with slight overlap for misalignment tolerance.

**Dual-doped poly for low-voltage CMOS**: A single blanket boron adjust creates a **buried channel** in PMOS transistors -- a thin inverted layer below the surface. For short-channel, heavily-doped-backgate transistors, the gate loses control of this buried channel, causing leakage. The solution:

- Use **boron-doped poly** ($P^+$ poly) gates for PMOS
- Use **phosphorus-doped poly** ($N^+$ poly) gates for NMOS
- Separate threshold adjust implants: PVT (boron, adjusts NMOS) and NVT (phosphorus, adjusts PMOS)

This dual-doped poly process requires up to four new masks (PVT, NVT, PPoly, NPoly), but well masks can often be reused if no natural transistors are needed.

**Poly diode hazard**: Wherever PPoly and NPoly abut, an unwanted PN junction forms. Silicidation shorts these diodes out, but accelerates dopant diffusion through the poly, so NPoly/PPoly intersections must be spaced well away from gate regions.

### 12.2.5 Multiple Gate Oxides

Two fabrication techniques:

**Staged oxidation**:
1. Grow thin gate oxide, deposit and pattern poly-1 (thin-gate transistors)
2. Poly-1 masks further oxidation growth
3. Continue oxidation to grow thicker oxide, deposit and pattern poly-2 (thick-gate transistors)
- Drawback: requires multiple polysilicon depositions

**Etch-and-regrowth**:
1. Grow thin gate oxide over entire wafer
2. Pattern photoresist, etch exposed oxide regions
3. Resume oxidation: thin oxide forms over etched areas, thicker oxide where initial oxide remained
4. Single poly deposition forms all gates
- Requires one extra mask but not an extra poly layer

Both use a `Moat-2` geometry around gate regions, but with different semantic meanings. In staged oxidation, it defines thick-oxide threshold adjust regions; in etch-and-regrowth, it defines regions protected from etchback.

### 12.2.6 Scaling the Transistor

**Moore's Law** predicted transistor count doubling every 18 months, holding from 1965 to ~2012. Channel lengths shrank from $10\,\mu\text{m}$ (1973) to ~20 nm (2012). ITRS predicted planar CMOS reaching ultimate limits around 2020 with FinFET widths of ~6 nm, limited by direct electron tunneling.

**Two scaling regimes** (Dennard scaling laws):

| Quantity | Constant Voltage ($S$) | Constant Field ($S$) |
|---|---|---|
| $W_{min}$, $L_{min}$ | $S$ | $S$ |
| $t_{ox}$ | $S$ | $S$ |
| Backgate doping | $1/S$ | $1/S$ |
| Supply voltage | $1$ | $S$ |
| Electric field | $1/S$ | $1$ |
| Propagation delay | $S^2$ | $S^2$ |
| Power dissipation | $1/S^2$ | $S^2$ |
| Power-delay product | $S^2$ | $S^4$ |

Constant-voltage scaling increases electric fields, eventually causing hot-carrier problems (hit this wall in the 1980s at $> 200\,\text{kV/cm}$). The industry switched to constant-field scaling for core logic, necessitating dual-gate processes.

**Analog scaling limits**: Analog CMOS will likely stop at approximately the **65 nm node** due to:
- Minimum threshold of $\sim 4 \times$ subthreshold swing ($\sim 100\,\text{mV/dec}$) needed
- Gate oxide tunneling becomes excessive below $\sim 30\,\text{\AA}$ (3 nm)
- Minimum practical supply voltage of $\sim 1\text{V}$ for analog
- Nothing prevents integrating more advanced digital transistors alongside end-generation analog devices

**FEOL vs. BEOL scaling**: By the 1980s, metal layers could no longer scale at the same rate as gate dimensions, ending the era of simple optical shrinks and scalable design rules. Analog layout was never amenable to optical shrinks because analog performance depends on parameters that do not scale in unison (e.g., resistor values are scale-invariant while capacitances scale as $S^2$).

### 12.2.7 Drain Engineering

As CMOS was constant-voltage scaled, intensifying electric fields at drain junctions generated **hot carriers**, NMOS first (higher electron mobility), then PMOS. Drain engineering addresses this by modifying the drain doping profile near the channel.

**Single-Doped Drain (SDD)**: One implant per polarity (NSD or PSD). Uses oxide sidewall spacers to minimize gate-drain overlap capacitance. Limited to minimum channel lengths of $\sim 1.5\,\mu\text{m}$ at $5\text{V}$.

**Lightly Doped Drain (LDD)**: Two implants per polarity, separated by oxide sidewall spacer formation:
1. First implant (before spacers): shallow, lightly doped **drift region**, self-aligned to poly gate
2. Second implant (after spacers): deeper, heavily doped **extrinsic drain**, aligned to spacer edge
- Four masks typically used: NSD, PSD, NMSD, PMSD (the "M" denotes "minus" for light doping)
- Reduces peak electric field by allowing depletion to extend into the lightly doped drain region

**Double-Diffused Drain (DDD)**: Two implants (arsenic + phosphorus) through the same oxide opening. Phosphorus outdiffuses beyond arsenic to form the drift region. Advantages:
- Very narrow, precisely controlled drift regions
- Graded junction further reduces electric field
- Drawback: wide drift regions require long drives that disrupt $V_{th}$ control
- Only works for NMOS (PMOS lacks suitable dopant pairs)

Many processes combine **LDD NMOS + SDD PMOS** for $\leq 5\text{V}$ operation, since the hot-hole generation threshold is 2-3x higher than for hot electrons.

**Buried-Channel LDD (BCLDD)**: In older single-doped $N^+$ poly processes, the SDD PMOS was actually a buried-channel device. The contact potential of the $N^+$ poly gate inverted the buried channel under the gate, but portions under the spacers only saw fringing fields, acting like lightly doped P-type drain regions. This structure became impractical for low-voltage CMOS, necessitating dual-doped poly.

**Pocket (Halo) Implants**: Used in deep submicron CMOS to increase backgate doping adjacent to source/drain junctions. Created by **tilted implants** where the ion beam strikes diagonally (wafer rotated for uniformity). They suppress drain-induced barrier lowering (DIBL) and punchthrough, but have two major analog drawbacks:

1. **Output resistance scales as $\sqrt{L}$** instead of $L$: The pocket regions create parasitic sub-transistors at source and drain ends. Drain-induced threshold shift (DITS) reduces the drain pocket threshold below the main channel, breaking the linear $r_o \propto L$ scaling analog designers rely on.
2. **Mismatch does not follow Pelgrom's law**: Random $V_{th}$ variation does not scale as $1/\sqrt{L}$, devastating for current mirror matching (which depends primarily on channel length).

**Solution for analog**: Block pocket implants from long-channel analog transistors using extra masks, or use **directional tilted implants** shot only left-right, so horizontally-oriented digital transistors receive pockets while vertically-oriented analog transistors do not. This approach requires no extra masks but imposes a layout constraint: blocks can be reflected or rotated $180°$ but **never $90°$**.

### 12.2.8 Variant CMOS Layouts

**Sectioned (interdigitated) transistors**: For $W/L > 10$, transistors become unwieldy. Dividing into parallel fingers improves aspect ratio and saves area because adjacent sections share source/drain fingers, reducing parasitic junction capacitances by up to 50%.

Notation: $N(W/L)$ means $N$ sections each with drawn width $W$ and length $L$. Example: $1000/0.5$ can become $20(50/0.5)$.

**Odd vs. even sections**:
- **Odd** number: Equal source and drain fingers; equal parasitic capacitances.
- **Even** number: Unequal -- one extra drain or source finger. PCells let designers choose "minimize drain" (preferred, as drain capacitance affects circuit performance more).

**Abutting (butting) backgate contacts**: When backgate connects to source, a strip of backgate contact can abut the source, saving space. Odd-section transistors get one abutting contact on one end; even-section drain-minimized transistors can have contacts on both ends.

**Merged transistors**: Transistors sharing a common source or drain can merge using notched moat geometries. This saves area and reduces parasitic capacitance, even accounting for the poly-to-moat spacing requirement.

**Standard cell layout conventions** (N-well CMOS):
- Power (VDD) and ground (VSS) rails run horizontally at top and bottom
- PMOS transistors in a common N-well spanning the cell top
- Wells overlap between adjacent cells in a "stick" of logic
- Pin geometries define autorouter connection points; names must match schematic (case-sensitive)
- **Tapless libraries**: Backgate contacts omitted from logic cells, provided by separate "tap cells" at intervals -- enables connecting backgates to potentials other than VDD/VSS

**Serpentine transistors**: For very long channels, moat is folded under a poly plate. Each $90°$ bend adds $W/2$ to the effective channel length: $L_{eff} = L_{drawn} + n_{bends} \cdot W/2$. Poor matching unless identical layouts are used. Suitable for trickle current sources where accuracy is unimportant.

**Annular transistors**: Surround the drain on all sides with a gate to minimize the $C_d/W$ ratio (drain capacitance per unit width). Two shapes:
- **Circular**: Theoretically optimal $C_d/W$ ratio. Width and length given by:
  $$W = \frac{\pi(D_o^2 - D_i^2)}{2(D_o - D_i) \cdot \ln(D_o/D_i)} \cdot \frac{D_o - D_i}{2}$$
  Simplified: $W = \pi(D_o + D_i)/2 \cdot L_{eff}$ approximately. The common approximation uses the perimeter at the midpoint: $W \approx \pi(D_i + D_o)/2$.
- **Square**: Similar concept but corner effects cause non-uniform current flow and can induce premature avalanche breakdown at higher voltages.

For processes that do not allow contacts over active gate area, elongated annular gates with poly extensions onto field oxide provide gate contact access.

### 12.2.9 Backgate Contacts

Backgate contacts are essential even though little current normally flows through them. Every MOS transistor contains parasitic bipolar devices (lateral PNP in PMOS, lateral NPN in NMOS) that form a positive-feedback latchup loop. Backgate contacts short the base-emitter junctions of these parasitic transistors.

**Latchup trigger voltage**: Forward bias of $\sim 0.65\text{V}$ at $25°\text{C}$, dropping to $\sim 0.45\text{V}$ at $125°\text{C}$. Parasitic betas also increase with temperature, making high-temperature operation more latchup-prone.

**Latchup immunity conditions** -- at least one must be satisfied:

$$\beta_1 \cdot \beta_2 \cdot (1 - \eta_1)(1 - \eta_2) < 1 \quad \text{(Eq. 12.32, unconditional immunity)}$$

$$I_{test} \cdot R_{bg} \cdot (1 - \eta) \cdot \beta < V_f \quad \text{(Eq. 12.33, conditional immunity)}$$

where $\beta_{1,2}$ are parasitic bipolar betas, $\eta_{1,2}$ are guard ring collection fractions, $R_{bg}$ is backgate resistance, and $V_f \approx 0.65\text{V}$.

Equation 12.32 examines loop gain -- if satisfied, latchup is impossible regardless of test current. Equation 12.33 addresses the more common case: guard rings and backgate contacts together reduce debiasing below the trigger threshold.

**Backgate contact strategies**:

| Situation | Strategy |
|---|---|
| Backgate over heavily doped sublayer ($P^+$ substrate or NBL) | Vertical conduction dominates; large area of distant contacts can be more effective than small adjacent contact |
| No heavily doped sublayer (P-epi over $N^-$ substrate, N-well without NBL, isolated P-tank) | Must place contacts adjacent to every transistor |
| Forward-biased source-backgate junctions | Add guard rings if current exceeds a few hundred $\mu\text{A}$ |

**Backgate pinning**: When an injector transistor forward-biases into its backgate near a victim transistor, place backgate contacts on the victim **on the side facing the injector**, across all direct minority-carrier paths, as close to the victim as possible.

**Contact styles for large transistors**:
- **Interdigitated backgate contacts**: Strips of backgate contact threaded through the transistor at regular intervals (effective but area-expensive)
- **Distributed backgate contacts**: Small rectangles placed in holes within source fingers (slightly increases source resistance but greatly reduces area; can be placed on every source finger or at regular intervals)

**Maximum spacing rule**: Typically 25--$50\,\mu\text{m}$ from any point in a transistor to the nearest backgate contact. Can be ignored if backgate contacts a low-resistance sublayer or if the transistor always operates in the linear region.

## Diagrams

### Figure 12.14 -- Four Approaches to P-Tank Isolation
![[diagrams/ch12-constructing-cmos-fig1.png]]
Cross sections of four methods for creating isolated P-type tanks using NBL: deep N-well (A), deep-$N^+$ sinker (B), up-down isolation (C), and deep trench isolation (D). These structures enable isolated NMOS backgate connections, critical for analog circuits that need independent backgate biasing.

### Figure 12.22 -- LDD and DDD Drain Structures
![[diagrams/ch12-constructing-cmos-fig2.png]]
Cross sections comparing lightly doped drain (LDD) and double-diffused drain (DDD) structures. Both use oxide sidewall spacers. The LDD uses two separate implants (before and after spacer formation), while the DDD uses two implants of different diffusivities (arsenic + phosphorus) through the same opening. These structures reduce peak electric fields at the drain to suppress hot-carrier generation.

### Figure 12.25 -- Sectioned Transistors with Backgate Contacts
![[diagrams/ch12-constructing-cmos-fig3.png]]
Layout of three-section (A) and four-section (B) interdigitated transistors showing source (S), drain (D), and backgate (BG) finger arrangements. Odd section counts yield symmetric source/drain capacitances; even counts require choosing whether to minimize drain or source capacitance.

## Practical Takeaways

- **Always use epitaxial substrates** for analog CMOS to ensure proper latchup suppression and avoid oxygen precipitate problems from Czochralski silicon.
- **Derate thick-field thresholds by at least 30%** when routing signals near field regions. Never assume channel stops provide unconditional protection -- poly stubs extending beyond wells are a common source of parasitic channels.
- **Identify all potential minority-carrier injectors** before layout begins. Place them in separate tanks from sensitive analog circuitry. Bandgap references and similar nonlinear circuits are particularly vulnerable to parasitic rectification.
- **Scattered tank contacts everywhere** within every P-type tank. Traditional rules impose a maximum distance (25--$50\,\mu\text{m}$) between any NMOS transistor and its nearest backgate contact.
- **Use odd section counts** for matched transistors when possible to equalize source and drain parasitic capacitances. When even sections are necessary, minimize the drain.
- **Matched transistors must use identical section widths** -- only the number of sections should differ between matched pairs (e.g., $4(25/0.5)$ matched to $8(25/0.5)$).
- **Communicate all fingering changes to the circuit designer** -- ESD robustness, for instance, depends on individual finger width, not total width.
- **Pocket implants break two key analog scaling assumptions**: output resistance no longer scales linearly with $L$, and mismatch no longer follows Pelgrom's $1/\sqrt{WL}$ law. Block pockets from analog transistors where possible, or use directional implants.
- **Never rotate blocks by 90 degrees** in processes using directional pocket implants -- this swaps digital and analog transistor orientations.
- **Analog circuits should never be optically shrunk** without full resimulation, because not all parameters (resistors, capacitances, thresholds) scale in unison.
- **Use distributed backgate contacts** (plugs in source fingers) for large multi-finger transistors to save area while maintaining adequate backgate resistance.

## Relation to the Bigger Picture

This section provides the fabrication foundation for everything that follows in Chapter 12: the specific well structures, channel stops, threshold adjusts, gate oxides, and drain engineering techniques determine the electrical characteristics discussed in [[ch12-mos-operation]]. The variant layouts (sectioned, serpentine, annular) directly impact the matching and parasitic behavior analyzed in the transistor operation sections. The isolated tank structures and backgate contact strategies connect back to the latchup and minority-carrier injection discussions of Chapter 5. Understanding these construction details is essential for analog layout because, unlike digital design, analog performance critically depends on the physical structure -- backgate resistance affects latchup immunity, drain engineering affects output resistance scaling, and well choices constrain circuit topologies.

## See Also
- [[ch12-mos-operation]]
- [[ch12-nvm]]
- [[ch12-jfet]]
