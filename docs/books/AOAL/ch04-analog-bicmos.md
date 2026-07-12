---
title: "4.3 Analog BiCMOS"
chapter: 4
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-4, bicmos, bipolar, cmos, dmos, dielectric-isolation]
---

# 4.3 Analog BiCMOS

> **Chapter 4: Representative Processes**

## Key Concepts

### Why BiCMOS?

During the 1970s, analog designers recognized that CMOS transistors outperformed bipolar in some applications (low power, high input impedance, rail-to-rail operation) while bipolar outperformed CMOS in others (transconductance per unit area, speed, matching, low-noise performance). A process that could fabricate *both* bipolar and CMOS transistors on the same die -- a **BiCMOS process** -- provides the best of both worlds. RCA commercialized the first BiCMOS process in the mid-1970s; Texas Instruments extended this to include lateral DMOS power devices (BiDFET).

By the mid-1980s, analog BiCMOS processes were being built upon the framework of existing digital CMOS process flows. This approach is economically brilliant: it reuses existing process equipment and expertise, adding only a minimum number of additional mask steps to create bipolar and DMOS transistors. This strategy has allowed analog BiCMOS to benefit from over 30 years of steady CMOS scaling improvements.

### Process Complexity and Capability

Modern analog BiCMOS processes are complex: typically **25--30 mask steps** fabricating **more than 50 different circuit components**. This complexity costs money but buys enormous versatility. A single die can integrate:

- Tens of thousands of gates of digital logic
- Multiple high-precision data converters
- Ten-amp power switches

These ICs find applications in products ranging from automobiles to smartphones.

### Collector-Diffused Isolation (CDI)

The fundamental trick that makes BiCMOS work is **collector-diffused isolation (CDI)**. An N-well -- already present in the CMOS process -- serves double duty as the collector of a vertical NPN transistor. The well-epi junction isolates this transistor from the substrate. This elegant reuse of existing structures minimizes the number of additional process steps needed.

The CDI approach means the NPN collector is formed from a *graded* well diffusion rather than a uniformly doped epi layer (as in standard bipolar). This has consequences: the graded collector exhibits relatively high resistance if deep-$N^+$ is not placed beneath the collector contact, causing a soft saturation transition and possible internal saturation even when terminal voltages suggest otherwise.

### The NBL Debate

The inclusion of **N-buried layer (NBL)** in BiCMOS has been "hotly debated by process engineers, circuit designers, and fab managers." CMOS wafer fabs normally purchase pre-made epi-coated wafers; adding NBL forces them to install and operate epitaxial reactors. Despite this cost, NBL's advantages are so compelling that almost all analog BiCMOS processes include it:

1. **Reduces NPN collector resistance** -- allows NPN transistors to operate at currents above a few hundred $\mu A$
2. **Increases operating voltage** by preventing vertical punchthrough
3. **Blocks hole flow to the substrate** -- critical for minority carrier guard rings
4. **Enables isolated NMOS transistors** whose backgates are not tied to substrate potential

### Twin-Well Architecture

The process described in the text uses a **twin-well** (N-well + P-well) architecture on a $\langle 100 \rangle$ P-type substrate with $\langle 100 \rangle$ P-type epi. The twin-well approach is necessary because:

- The P-epi is too lightly doped for high-performance NMOS transistors at low operating voltages
- If the epi were doped heavily enough for the NMOS, the N-well/epi junction would break down below the operating voltage

Both wells represent **doping compromises** between the optimal profiles for 5V and 3.3V transistors. If the core voltage were much lower (e.g., 1.8V), the core transistors would need their own dedicated shallow wells, creating a **quad-well** process with four separate well implants.

## Fabrication Sequence

The baseline analog BiCMOS process described requires **18 masks**; adding 3.3V core transistors increases this to 21; designs with significant digital content needing four metal layers bring the total to 25.

### Step-by-Step Process Flow

1. **Starting Material**: $P^+$ $\langle 100 \rangle$ substrate. Because NBL is used with a $P^+$ substrate, an extra epitaxial deposition is required -- without it, NBL would abut the substrate forming an $N^+/P^+$ junction with very low breakdown voltage.

2. **First Epi Growth**: Upon the unpatterned wafer. These epi-coated wafers can be stockpiled as starting material.

3. **Etched Alignment Marker**: Anisotropic RIE forms alignment markers (typically in scribe streets) to which the N-well mask will align. This eliminates the need for an NBL shadow that might interfere with advanced photolithography's narrow depth of field.

4. **N-Buried Layer (NBL)**: Thermal oxidation, patterning with NBL mask, antimony implant through oxide windows, brief anneal. Antimony is preferred over arsenic to minimize lateral autodoping during subsequent epitaxy.

5. **Second Epitaxial Growth**: ~$10\mu m$ thick P-type epi. During this growth, some NBL dopant outgasses and redeposits elsewhere (**lateral autodoping**), potentially forming a thin N-type layer at the epi-epi interface that shorts adjacent wells. Using antimony (instead of arsenic) or reduced-pressure epitaxy minimizes this. The NBL dopant concentration should exceed the bottom N-well doping by at least a factor of **50** to limit hole permeation.

6. **N-Well Implant**: Phosphorus implant followed by a partial high-temperature drive. The N-well doping profile is a compromise between 5V and 3.3V PMOS requirements. It also affects drain-extended NMOS, LDMOS, NPN, and lateral PNP transistors. The lateral PNP suffers most from this compromise, with nominal $\beta \approx 20$.

7. **P-Well Implant**: Boron implant followed by partial drive. Makes this a twin-well process. The higher backgate doping eliminates the need for dedicated channel stop implants -- the P-well itself substitutes for the P-type channel stop, and the N-well is sufficiently doped to serve as its own N-type channel stop.

8. **Deep-$N^+$ Sinker**: Patterned with deep-$N^+$ mask; heavy phosphorus deposition (typically from $POCl_3$ source because the high concentrations required make ion implantation impractically slow). A long high-temperature drive forces the sinker down to penetrate a few microns into NBL. This overlap is critical for:
   - Reducing vertical resistance between sinker and buried layer
   - Raising doping at the interface to suppress hole permeation
   
   This same drive also pushes the N-well down into the NBL and the P-well to similar depth.

9. **Base Implant**: Boron implant through oxide openings, annealed under inert ambient. The counterdoping of the N-well degrades NPN $\beta$ by raising total base dopant concentration and recombination rate. A typical compromise yields NPN $\beta \approx 50$ and Early voltage $V_A$ of adequate magnitude. On more advanced processes, a shallow P-well is sometimes used as the NPN base (saves a mask but gives a less-than-ideal retrograde profile favoring lateral over vertical conduction).

10. **Inverse Moat (LOCOS)**: Pad oxide deposition, LPCVD nitride deposition, patterning with inverse moat mask. The moat generation combines NMoat, PMoat, *and* Base layers:
    ```
    Moat    = NMoat + PMoat + Base
    InvMoat = NOT(Moat)
    ```
    Base regions are placed within moats to prevent **oxidation-enhanced diffusion** from increasing base junction depth variability.

11. **LOCOS Oxidation and Dummy Gate Oxidation**: Steam-driven oxidation, nitride strip, then dummy gate oxidation to remove Kooi effect nitride residue.

12. **Gate Oxidation**: Dry oxidation creates the gate oxide for 5V transistors. No threshold adjust implants are needed -- the separate N-well and P-well implants provide the proper surface doping.

13. **Polysilicon Deposition, Doping, and Patterning**: CVD polysilicon forms gates, resistor bodies, and capacitor electrodes. PMOS transistors receive **P-type poly gates** while NMOS transistors receive **N-type poly gates** to avoid buried-channel effects. The Ngate mask selectively opens the phosphorus doping regions. Polysilicon is then patterned using the poly mask.

14. **Source/Drain Implants (LDD process)**:
    - Light phosphorus implant ($N^-$ S/D, NMSD) self-aligned to NMOS gates
    - Light boron implant ($P^-$ S/D, PMSD) self-aligned to PMOS gates
    - CVD nitride sidewall spacers formed by isotropic deposition + anisotropic etch
    - Heavy arsenic implant (NSD) self-aligned to sidewall spacers
    - Heavy boron implant (PSD) self-aligned to sidewall spacers
    - Brief anneal to activate, drive junctions, and push boron through PMOS gate poly
    
    NSD also serves as NPN emitters and lateral PNP base contacts. PSD forms extrinsic NPN base contacts and lateral PNP emitter/collector contacts.

15. **Silicidation**: TEOS oxide deposited, patterned with silicide mask (SBlk blocks regions that should not be silicided, such as resistor bodies). Cobalt sputtered, rapid thermal anneal forms $CoSi$ (monosilicide), unreacted cobalt removed with $H_2SO_4/H_2O_2$, second anneal converts to $CoSi_2$ (disilicide). Silicide shorts out PN junctions in poly where P-type and N-type regions abut (at Ngate geometry edges) and reduces source/drain resistance ("clad" S/D).

16. **Contacts**: Thick MLO deposited and CMP-planarized. Contact openings etched. TiN liner deposited, then CVD tungsten fills contacts (from $WF_6$ decomposition -- TiN liner prevents fluorine corrosion of oxide). CMP removes excess tungsten, leaving **tungsten plugs**.

17. **Metallization**: Ti adhesion layer, TiN diffusion barrier, thick Cu-doped Al, TiN cap (serves as etch stop for via process and as anti-reflective coating for photolithography). Metal-1 patterned, ILO deposited and CMP-planarized, via openings filled with tungsten plugs, Metal-2 deposited and patterned. Additional metal layers can be added by repeating via/metal steps.

18. **Protective Overcoat**: BPSG + compressive nitride. POR mask opens bondpad windows.

## Available Devices

### NMOS Transistors

The NMOS transistor uses NSD/NMSD implants self-aligned to the poly gate and sidewall spacers respectively. Pattern generation:

```
PSD    = PMoat oversized by 0.3    {P+ S/D layer}
NSD    = NMoat oversized by 0.3    {N+ S/D layer}
PMSD   = PSD                       {P- S/D layer}
NMSD   = NSD                       {N- S/D layer}
InvMoat = NOT(PMoat + NMoat)       {Inverse moat layer}
```

The backgate is a P-well that connects to substrate. PWell is derived:
```
PWell = NOT(NWell oversized by 4.0)
```

The use of LDD implants minimizes hot carrier generation to the point where the transistor can operate at its full blocking voltage (no separate "operating" vs "blocking" voltage ratings needed).

### PMOS Transistors

Resides in an N-well (backgate). NBL placed inside the N-well reduces lateral backgate resistance and suppresses hole flow to the substrate if a S/D region forward-biases into the well. P-type poly gates are required -- the NGate mask must exclude PMOS gate poly:

```
NGate = NOT((PSD incremented by 3.0))
```

The 3.0 size adjust prevents N-type doping from diffusing through the polysilicon into the PMOS gate region -- poly grain boundaries greatly accelerate dopant outdiffusion, requiring a much larger margin than monocrystalline silicon would need.

### DMOS Transistors (LDMOS)

High-voltage transistors need short, moderately doped backgates and wide, lightly doped drift regions. The **double-diffused MOS (DMOS)** creates these by diffusing boron and arsenic through the same opening: boron outdiffuses further, forming the backgate around the shallower arsenic source. Channel length depends only on diffusion time/temperature, not photolithography.

Key design features of the **lateral DMOS (LDMOS)**:
- N-well forms the drift region; NSD enables drain contact
- LOCOS bird's beak positioned beneath the gate electrode at the point where the vertical drain-to-gate field approaches the maximum -- critical for gate oxide integrity and minimizing $R_{on}$
- PSD plugs inside DMOS diffusion contact the backgate, interspersed with NSD source plugs
- PSD plugs at transistor ends suppress edge transistor action (curvature reduces breakdown voltage)

### NPN Transistors

The CDI NPN uses an N-well as collector, with base and NSD emitter successively diffused into it. NBL beneath the active region and deep-$N^+$ sinker minimize collector resistance. Key differences from standard bipolar:

- Shallow NSD emitter reduces $\beta$ to roughly **50**
- Graded well collector has higher resistance than standard bipolar epi collector
- Without deep-$N^+$, the transistor shows soft saturation transition and possible internal saturation
- Minimum-size NPN is about **an order of magnitude larger** than its CMOS counterpart

Despite these limitations, the CDI NPN suffices for many applications.

### Lateral PNP Transistors

Uses the base diffusion for both emitter and collector, residing in an N-well base. NBL serves multiple critical functions:
1. **Depletion stop** -- allows higher operating voltages without punchthrough
2. **Enables use of base implant** (deeper than PSD) for emitter/collector -- the greater depth enhances emitter sidewall injection, increasing $\beta$
3. **Blocks substrate injection** -- without NBL, apparent $\beta < 10$

The minimum-geometry lateral PNP can achieve $\beta > 50$. Several factors contribute:
- Fine-line photolithography allows narrow base width and small emitter area
- Small emitter increases periphery-to-area ratio, enhancing lateral injection over vertical injection
- Graded well doping drifts carriers away from the surface, reducing surface recombination
- $\langle 100 \rangle$ silicon (vs $\langle 111 \rangle$ in standard bipolar) further reduces surface recombination

Older processes with lightly doped wells achieved lateral PNP $\beta$ up to 500; the heavier doping in this process reduces it, but the device still compares favorably with standard bipolar.

### Poly Resistors (Three Types)

| Parameter | LSR (Low Sheet) | MSR (Medium Sheet) | HSR (High Sheet) |
|---|---|---|---|
| Construction | Silicided poly | PSD blocks Ngate, SBlk blocks silicide | HSR layer blocks Ngate, SBlk blocks silicide |
| Sheet resistance | ~$5 \;\Omega/\square$ | ~$200\text{--}400 \;\Omega/\square$ | ~$1\text{--}2 \;k\Omega/\square$ |

- **LSR**: Simple strip of silicided poly. Very low resistance.
- **MSR**: PSD layer blocks N-type gate implant; SBlk blocks silicidation of the body. Heads (ends) remain silicided to ensure Ohmic contact for tungsten plug contacts.
- **HSR**: HSR drawing layer blocks Ngate implant without adding doping. A light blanket phosphorus implant sets the sheet resistance. Heads protrude from under the HSR geometry so sufficient doping exists for Ohmic contact. Circuit designers want high values, but variability increases with sheet resistance.

The NGate mask generation becomes:
```
NGate = NOT(((PSD - NWELL) incremented by 3.0) + HSR)
```

### Gate Oxide Capacitors

Same construction as in poly-gate CMOS: one plate is doped polysilicon with silicide, the other is N-well. The well electrode must remain at least 1V above the poly electrode to avoid dramatic capacitance drop. Main drawbacks: excessive bottom-plate parasitic junction capacitance, series resistance, and voltage-dependent capacitance variation.

## Process Extensions

### 3.3V CMOS Core Transistors

To support digital logic, three additional mask steps create CMOS transistors with 3.3V operating voltage and minimum channel lengths smaller than the 5V devices. The 5V transistors serve as **I/O devices**, while the 3.3V devices are **core transistors**.

The extension reuses the existing wells (cost savings) at the expense of some performance loss. Adding dedicated shallow wells for the core transistors would improve density but adds mask cost -- only justified when digital logic dominates the design (which it usually does not in analog BiCMOS).

**Etch-and-Regrow Gate Oxide Process**:
1. Grow dummy gate oxide across all moat regions, then remove
2. Begin true gate oxidation -- interrupt before reaching full 5V thickness
3. Pattern with LVMOS mask to expose 3.3V transistor regions
4. Etch gate oxide from LVMOS windows
5. Continue oxidation: LVMOS regions grow thin oxide while existing regions continue to thicken

A single LVMOS drawing layer generates three new mask layers:
```
AllMoat = NMoat + PMoat + Moat
LVGOX   = (AllMoat * LVMOS) oversized by 0.2      {3.3V gate oxide}
NVT     = (NMoat * Poly * LVMOS) oversized by 0.2  {NMOS threshold adjust}
PVT     = (PMoat * Poly * LVMOS) oversized by 0.2  {PMOS threshold adjust}
```

The 3.3V transistors require two threshold adjust implants (unlike the 5V transistors which rely solely on well doping). The threshold target is a compromise between full enhancement at low $V_{DD}$ and minimal subthreshold leakage.

If we accept slightly increased minimum channel lengths for 3.3V transistors, they can share the same LDD implants as the 5V devices -- a significant cost saving.

### Dielectric Isolation (DI)

Dielectric isolation using **wafer bonding** combined with **deep trench isolation (DTI)** provides several unique advantages over junction isolation (JI):

1. **Area savings**: deep trenches consume less lateral space than outdiffusion-based junction isolation
2. **Superior minority carrier injection protection**
3. **Better substrate noise isolation**

**Four termination schemes** for junctions against deep trenches (in order of increasing aggressiveness):

| Scheme | Description | Risk | Area |
|---|---|---|---|
| A | All junctions kept away from trench | None | Largest (more than JI) |
| B | Junctions terminate within trench but not against it | Minimal | Good savings |
| C | Junctions intersect trench sidewalls | Anomalous breakdown/leakage possible | Better savings |
| D | Multiple junctions intersect same trench sidewall | Breakdown + reduced $\beta$ from surface recombination | Maximum savings |

**Scheme B** represents the best compromise for most analog BiCMOS applications. Schemes C and D suit high-speed low-voltage processes where compact transistors reduce both area and capacitance.

**DI Fabrication Flow**:
1. Deposit and densify ~$1\mu m$ oxide on $\langle 100 \rangle$ substrate
2. Bond a $\langle 100 \rangle$ wafer on top, cleave to form thin active silicon layer over **buried oxide (BOX)**
3. CMP, thermal oxidation, NBL implant (antimony/arsenic) and anneal
4. Strip oxide, deposit ~$6\mu m$ P-type epi
5. Pad oxidation, CVD nitride deposition
6. Pattern with **deep trench (DT)** mask, anisotropic plasma etch
7. Thermal oxidation of trench sidewalls, CVD oxide thickening
8. Fill trenches with polysilicon, CMP planarization
9. Continue with standard BiCMOS process flow (N-well, deep-$N^+$, etc.)

The DI process requires only **one additional mask** (DT) versus JI. The DI NPN breakdown between collector and other devices depends on trench oxide sidewall thickness and can easily exceed 60V.

## Diagrams

### Figure 4.35: CDI NPN Transistor Cross-Section and Essential Features
![[diagrams/ch04-analog-bicmos-fig1.png]]
*Cross section of collector-diffused isolation (CDI) for an NPN transistor, showing how the N-well doubles as the NPN collector. Deep-$N^+$ sinker and NBL create a low-resistance collector contact path. This page also introduces the essential features of analog BiCMOS: reuse of the CMOS N-well as a bipolar collector, the role of NBL, and the LDMOS power transistor structure.*

### Figure 4.43: Completed BiCMOS Wafer Cross-Section and NMOS Layout
![[diagrams/ch04-analog-bicmos-fig2.png]]
*Top: Cross section of the completed analog BiCMOS wafer showing PMOS (left), NMOS (center), and NPN (right) transistors with Metal-1, Metal-2, tungsten-plug vias, silicide, N-well, P-well, NBL, and the full metal stack. Bottom: Layout and cross section of an NMOS transistor in analog BiCMOS with pattern generation rules for deriving mask layers from drawn layers.*

### Figure 4.55: DI BiCMOS Vertical NPN Layout and Cross-Section
![[diagrams/ch04-analog-bicmos-fig3.png]]
*Layout and cross section of a dielectrically isolated (DI) BiCMOS vertical NPN transistor. The transistor is fully enclosed by deep isolation trenches (DTI) filled with poly, sitting on a buried oxide (BOX) layer formed by wafer bonding. Note the large-radius corner bends in the trench to avoid etching and planarity problems. The device uses termination scheme B, where the N-well terminates along the trench centerline.*

## Practical Takeaways

- **NBL dopant concentration must exceed N-well bottom doping by at least 50x** to limit hole permeation through the buried layer.
- **Use antimony rather than arsenic for NBL** to minimize lateral autodoping during epitaxy. Reduced-pressure epitaxy also helps.
- **Deep-$N^+$ must overlap NBL by several microns** -- this overlap is critical for reducing resistance and suppressing hole permeation at the sinker/NBL interface.
- **Place base regions within LOCOS moats** (not in field oxide regions) to prevent oxidation-enhanced diffusion from increasing base junction depth variability.
- **P-type poly gates for PMOS, N-type poly gates for NMOS** -- avoids buried-channel effects that worsen subthreshold conduction, especially at higher backgate dopings.
- **Poly grain boundaries accelerate dopant diffusion** -- the NGate-to-PMOS-gate spacing must be much larger (~3.0 units) than would be required for monocrystalline silicon.
- **Silicide block (SBlk) is essential for resistors** -- without it, silicidation reduces all poly to ~$5\;\Omega/\square$, precluding anything but the lowest-value resistors.
- **A silicide block mask is part of the baseline** because modern analog circuits almost invariably require megohms of resistance.
- **LDMOS bird's beak positioning is critical** -- it must appear beneath the gate electrode where the vertical drain-to-gate field approaches the maximum to ensure both gate oxide integrity and minimum $R_{on}$.
- **PSD plugs at DMOS transistor ends** suppress parasitic transistor action where diffusion curvature reduces breakdown voltage.
- **For DI BiCMOS, use termination scheme B** (junctions terminate within the trench but not against the sidewall) for the best area-vs-reliability tradeoff.
- **DI trench corners need large-radius bends** to avoid etching and planarity problems.
- **Substrate PNP in low-voltage CMOS** has lower $\beta$ ($\approx 10\text{--}20$) than in higher-voltage processes because lower-voltage processes use more heavily doped wells.
- **The lateral PNP suffers the most** from well doping compromises forced by supporting both 5V and 3.3V CMOS.

## Relation to the Bigger Picture

Section 4.3 represents the culmination of Chapter 4's progression from simple to complex processes: [[ch04-standard-bipolar]] introduced the foundational bipolar concepts (NBL, deep-$N^+$, isolation), [[ch04-poly-gate-cmos]] added CMOS transistors with self-aligned gates and wells, and now analog BiCMOS merges both into a single process flow while adding DMOS power devices. This section is essential for understanding the device structures and layout constraints that dominate modern mixed-signal IC design. Nearly every concept introduced in the earlier sections -- collector resistance, lateral outdiffusion, junction isolation, channel stops, self-aligned gates, LOCOS, LDD -- reappears here in a more complex, interconnected context. The process extensions (3.3V core, dielectric isolation) foreshadow the advanced techniques covered in later chapters on matching (Ch. 8), bipolar transistor layout (Ch. 9), and CMOS transistor layout (Ch. 12--13).

## See Also
- [[ch04-poly-gate-cmos]]
- [[ch04-standard-bipolar]]
