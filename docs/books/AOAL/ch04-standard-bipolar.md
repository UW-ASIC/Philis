---
title: "4.1 Standard Bipolar"
chapter: 4
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-4, standard-bipolar, fabrication, junction-isolation, NPN, PNP, resistors]
---

# 4.1 Standard Bipolar

> **Chapter 4: Representative Processes**

## Key Concepts

Standard bipolar was the **first high-volume analog integrated circuit process**, initially developed in 1961 by Jay Last's team at Fairchild for digital logic. It proved capable of creating analog ICs as well, producing legendary designs like the $\mu$A709, $\mu$A741 operational amplifiers, the LM109 voltage regulator, and the 555 timer. Although rarely used for new designs today (CMOS offers lower supply currents, BiCMOS provides superior analog performance), the knowledge gained from standard bipolar remains foundational -- the same devices, parasitic mechanisms, design tradeoffs, and layout principles reappear in modern processes.

### The NPN-First Philosophy

The defining architectural decision of standard bipolar is the **conscious optimization of the NPN transistor at the expense of the PNP**. Electron conduction (NPN) yields both higher $\beta$ and faster switching than hole conduction (PNP) by a factor of more than 2:1 for equivalent geometries and doping profiles. Optimizing both transistor types simultaneously would require several additional processing steps, so the early processes optimized NPN transistors exclusively. PNP transistors were later "cobbled together" from existing process steps -- performing relatively poorly but sufficing for many useful circuits.

### Junction Isolation

Standard bipolar employs **junction isolation (JI)** to prevent unwanted currents between devices on the same substrate. Active devices reside in a lightly doped N-type epitaxial layer deposited on top of a lightly doped P-type substrate. A heavy $P^+$ isolation diffusion driven down to contact the underlying substrate isolates each "tank" from the surrounding silicon. The use of tanks (rather than wells) ensures a uniform and reasonably controlled doping level in the starting material.

This is fundamentally different from the oxide-based isolation (LOCOS, STI) used in CMOS processes. Junction isolation consumes more area due to the lateral outdiffusion of the isolation diffusion, but it was simpler to implement with 1960s-era technology.

## Fabrication Sequence (8 Mask Steps)

The baseline standard bipolar fabrication sequence consists of **eight mask steps**. The vertical scale in all cross sections is exaggerated by at least 2x for clarity, and the substrate is far thicker than depicted (for mechanical strength).

### 1. Starting Material

Standard bipolar is fabricated on a lightly doped $\langle 111 \rangle$-oriented P-type substrate. This crystal orientation was chosen because:
- It produces the **fastest Czochralski crystal growth rate**
- It **minimizes twinning defects**
- Serendipitously, $\langle 111 \rangle$ silicon increases the positive surface state charge at the oxide interface, which **suppresses parasitic PMOS channels** across the surface of N-type tanks

The wafers are cut $3^\circ$ off-axis tilted toward a $\langle 110 \rangle$ axis to minimize pattern shift and maximize the epitaxial deposition rate.

### 2. N-Buried Layer (NBL) -- Mask 1

A thin oxide is grown, then patterned with the **NBL mask**. After oxide etch, ion implantation or thermal deposition places an N-type dopant (arsenic or antimony) into the wafer. These heavy, low-diffusivity elements are chosen specifically to **limit up-diffusion during subsequent high-temperature processing**. A brief anneal drive:
- Heals lattice damage from implantation
- Grows a slight oxide discontinuity at the silicon surface -- this **NBL shadow** serves as an alignment mark for subsequent masks

### 3. Epitaxial Deposition

The remaining oxide is stripped, and approximately $10\text{-}15\;\mu\text{m}$ of lightly doped N-type epi is grown. The epi thickness directly determines the **collector-to-emitter operating voltage** of the NPN transistors. Surface discontinuities from the NBL shadow propagate diagonally upward during epitaxial growth at an angle of approximately $17^\circ$ to the perpendicular (though this varies with processing conditions). If the wafer is not tilted toward the flat, the pattern shifts in the $\langle 110 \rangle$ direction opposite the direction of tilt.

### 4. Isolation Diffusion -- Mask 2

After oxidation, the **isolation mask** is applied. This mask must be aligned to the NBL shadow using a **deliberate offset to correct for pattern shift**. A heavy boron deposition followed by a high-temperature drive forces the isolation diffusion partway through the epi layer. Oxide growth during the drive covers isolation windows with thermal oxide. The drive stops before the isolation junction reaches the substrate -- the subsequent deep-$N^+$ drive will complete the diffusion.

Base over isolation (BOI) -- implanting base across isolation regions -- substantially increases the NMOS thick-field threshold without requiring a separate channel stop implant.

### 5. Deep-$N^+$ Sinker -- Mask 3

A deep, heavily doped N-type diffusion called a **deep-$N^+$ sinker** enables low-resistance connection to the NBL. A heavy phosphorus deposition followed by a long, high-temperature drive accomplishes three things simultaneously:
1. The deep-$N^+$ diffuses **down** to meet the **upward-diffusing NBL**
2. The isolation drive is completed (isolation diffusion reaches the substrate)
3. The junctions are **overdriven** by approximately $25\%$

The overdrive is critical: without it, the bottom of the isolation and deep-$N^+$ diffusions would be very lightly doped. Overdrive simultaneously:
- Reduces vertical resistance through isolation and deep-$N^+$ sinkers
- Prevents depletion regions from punching laterally through the lowest portions of the isolation diffusion

A **wet oxidation** during the deep-$N^+$ drive forms the thick field oxide.

**Important design rule:** NBL regions are normally spaced some distance inside the isolation diffusions to increase the tank-to-substrate breakdown voltage. Without this spacing, the $N^+/P^+$ junction formed by the intersection of NBL and isolation would avalanche at a low voltage.

### 6. Base Implant -- Mask 4

Photoresist patterned with the **base mask** opens windows through the field oxide. A boron implant counterdopes the N-epi surface to form the NPN base regions. Ion implantation (rather than deposition) allows **precise control of base doping**, minimizing $\beta$ variation. The subsequent drive:
- Anneals implant damage
- Sets the base junction depth
- Grows oxide that serves as a mask for subsequent emitter deposition

### 7. Emitter Diffusion -- Mask 5

Patterned with the **emitter mask**, an oxide etch exposes regions where NPN emitters will form and where Ohmic contact to N-epi or deep-$N^+$ is needed. A phosphorus deposition (often from a $POCl_3$ source) forms the emitter. Precise doping control is less important than **maximizing doping concentration**. A brief drive sets the final emitter junction depth, thereby determining the **width of the active base region**.

An oxide film grown over the emitter insulates it from subsequent metallization. Older processes used dry oxidation producing a "thin emitter oxide" vulnerable to ESD; newer versions use wet oxidation or oxide deposition for a **thick emitter oxide** with better ESD resistance.

**Emitter pilot:** Older processes used a dummy wafer to perform an experimental emitter drive, allowing the actual drive to be adjusted to target the desired NPN $\beta$ within approximately $\pm 10\%$. Modern implanted base processes generally achieve this without a pilot.

### 8. Contact OR -- Mask 6

All diffusions are complete. The wafer is patterned with the **contact mask** and etched to expose bare silicon. "OR" stands for **oxide removal**.

### 9. Metallization -- Mask 7

A layer of aluminum-copper-silicon alloy is sputtered across the wafer. The alloy typically contains:
- $\sim 2\%$ silicon to suppress emitter punchthrough
- $\sim 0.5\%$ copper to improve electromigration resistance

Standard bipolar uses **thick metallization** (at least $10{,}000\;\text{\AA}$) to reduce interconnection resistance and improve electromigration performance. The metal is patterned and etched using the **metal mask**.

### 10. Protective Overcoat -- Mask 8

A thick layer of **compressive nitride** is deposited as a protective overcoat (PO) against mechanical damage and chemical contamination. The moderate deposition temperature simultaneously **sinters the aluminum metallization**. The **POR (protective overcoat removal) mask** opens bondpad windows through the overcoat.

## Available Devices

### NPN Transistors

The vertical NPN is the **best active device** on standard bipolar -- it consumes relatively little area and offers good performance. Circuit designers try to maximize its use.

**Structure:** The collector is an N-epi tank, and the base and emitter are successive counterdopings. Carriers flow **vertically** from emitter to collector through the thin base region. The effective base width is set entirely by diffusion processing (base junction depth minus emitter junction depth), making it **independent of photolithographic misalignment** -- a key advantage that allows extremely thin base widths.

**Collector design:** Lightly doped N-epi sits on heavily doped NBL. The light epi doping allows a wide collector-base depletion region without excessive intrusion into the neutral base, enabling:
- Relatively large $V_{CE}$ operating voltages
- Minimized Early effect

NBL and deep-$N^+$ create a **low-resistance path** from the collector contact to the epi beneath the active base. This reduces collector resistance of a minimum NPN to less than $200\;\Omega$ and a large power NPN to less than $20\;\Omega$.

The heavily doped NBL also halts the downward growth of the collector-base depletion region. The distance from the bottom of the base diffusion to the top of the NBL sets the **maximum operating voltage**. Thicker epi allows higher voltage but increases isolation spacings. $BV_{CEO}$ (collector-emitter breakdown, base open) may range from less than $15\;\text{V}$ to more than $40\;\text{V}$ depending on epi thickness, base depth, and dopings.

**Diode configurations:** The NPN can serve as a diode. The **CB-shorted diode** (collector-base tied as anode, emitter as cathode) provides the least series resistance and fastest switching, but its breakdown voltage equals $BV_{EBO}$ ($\approx 7\;\text{V}$), which can also serve as a useful Zener diode (with $\pm 20\%$ tolerance).

### PNP Transistors

Standard bipolar cannot fabricate an isolated vertical PNP (no P-type tank). Two alternatives exist:

#### Substrate PNP
- **Collector:** the P-type substrate (always connected to ground or negative supply)
- **Base:** the N-tank
- **Emitter:** base diffusion
- Collector current flows through substrate and isolation; substrate contacts placed adjacent to the transistor minimize substrate debiasing
- NBL **must be omitted** (it severely reduces $\beta$); therefore deep-$N^+$ serves no useful function
- An emitter diffusion beneath the base contact ensures Ohmic contact
- Surprisingly good performance despite being non-optimized

#### Lateral PNP
- Both collector and emitter are **base diffusions** in a common N-tank
- Transistor action occurs **laterally** from central emitter to surrounding collector
- Emitter and collector are **self-aligned** (single masking step forms both), so the base width is precisely controlled
- Effective base width is less than drawn width due to outdiffusion; minimum drawn width is about $2\times$ the base junction depth
- **NBL is essential**: without it, a parasitic substrate PNP steals most of the emitter current, drastically reducing apparent $\beta$
- Surface recombination (especially in $\langle 111 \rangle$ silicon) reduces effective $\beta$, but modern processes achieve $\beta \geq 50$
- **Very slow** due to large parasitic junction capacitances on the base terminal

Neither PNP type is a true complement to the vertical NPN. Circuit designers avoid routing active signal paths through lateral PNPs because of poor frequency response. Substrate PNPs are faster but lack isolation.

### Resistors

The baseline process has **no dedicated resistor diffusion**, but three types of resistors can be constructed:

#### Base Resistor
- Strip of base diffusion in an N-tank, reverse-biased at the base-epi junction
- Tank connected to the more positive end of the resistor (or any higher-voltage point)
- NBL placed beneath to prevent vertical punchthrough
- Deep-$N^+$ not needed (tank draws negligible current)
- Sheet resistance: $R_s \approx 150\text{-}250\;\Omega/\square$

#### Emitter Resistor
- Strip of emitter diffusion isolated by base diffusion in an N-tank
- Base connected to low-voltage end, tank to high-voltage end (keep both junctions reverse-biased)
- NBL beneath the base diffusion prevents depletion punchthrough
- Sheet resistance: $R_s \approx 5\text{-}10\;\Omega/\square$
- Emitter-base breakdown limits differential voltage to $\approx 7\;\text{V}$

#### Pinch Resistor
- Combination of base and emitter diffusions: emitter plate overlaps the middle of a thin base strip
- Tank and emitter plate (both N-type) are electrically united; biased slightly above resistor voltage
- Body = portion of base diffusion beneath emitter plate (pinched base)
- Sheet resistance: can exceed $10{,}000\;\Omega/\square$
- **Major drawbacks:** highly variable (far worse than base or emitter), severe voltage modulation (acts like a JFET with large pinchoff voltage)
- Use limited to startup circuits and noncritical applications

### Capacitors (Junction Capacitor)

Standard bipolar was **not designed for capacitors** -- all oxide layers are too thick. However, the depletion region of a base-emitter junction exhibits a capacitance of approximately $1.2\;\text{fF}/\mu\text{m}^2$, usable as a **junction capacitor**:
- Base diffusion overlapping emitter diffusion, both in a common tank
- Emitter shorts to tank, adding base-tank capacitance to base-emitter capacitance
- Emitter plate must be biased positive to base plate (maintain reverse bias)
- Maximum differential voltage $\leq BV_{EBO} \approx 7\;\text{V}$
- Capacitance varies with bias and temperature by $\pm 25\%$ or more
- Frequently used for feedback loop compensation despite high variability

## Process Extensions

### Up-Down Isolation

Standard bipolar's top-down isolation diffusion suffers from large outdiffusion ($20\;\mu\text{m}$ or more), limiting component packing density. Adding a **P-buried layer (PBL)** creates up-down isolation:
- Isolation diffuses **down** from the surface
- PBL diffuses **up** from the epi-substrate interface
- Each diffusion crosses only half the total distance, so **outdiffusion is approximately halved**

**Tradeoff:** Lateral autodoping limits the PBL implant dose, resulting in higher vertical resistance than conventional isolation. Requires one additional masking step. Despite this, up-down isolation typically **reduces die area by 15-25%**, more than compensating for the extra mask cost. Many modern standard bipolar processes and BiCMOS processes with deep wells/sinkers employ this technique.

### Double-Level Metal (DLM)

The baseline process is single-level metal (SLM), which forces the use of diffusion-based **crossunders or tunnels** for wire crossings. This requires deep understanding of device and circuit operation, and few practitioners remain who have mastered single-level routing.

DLM adds two extra masks (via and metal-2). The first metal layer is often thinned to aid planarization. Benefits:
- Eliminates need for customized crossunder devices
- Enables component standardization
- Reduces layout time substantially
- Can reduce die area by up to $40\%$

Almost all new standard bipolar designs employ DLM.

### High-Sheet Resistors (HSR)

Base diffusion sheet resistance rarely exceeds $250\;\Omega/\square$, limiting total on-die resistance to a few hundred kilohms. Pinch resistors offer higher $R_s$ but lack precision. The **HSR implant** provides a precisely controlled, shallow, lightly doped P-type implant:
- Sheet resistance: $1\;\text{k}\Omega/\square$ to $10\;\text{k}\Omega/\square$ (higher values suffer voltage modulation)
- Most processes use $1\text{-}5\;\text{k}\Omega/\square$
- HSR body is contacted at each end by small base diffusion regions for Ohmic contact
- Occupies an N-tank, isolated by the reverse-biased HSR-tank junction
- Tank often connected to the more positive end of the resistor
- Requires one additional mask step and one dedicated implant
- Cost-effective if the circuit includes more than $100\text{-}200\;\text{k}\Omega$ of total resistance

## Diagrams

### Figure 4.1 -- Junction Isolation Cross Section and Full Fabrication Page

The page below shows the finished wafer cross section illustrating the junction isolation system, along with the beginning of the fabrication sequence discussion. Note the P-substrate, N-epi tanks, $P^+$ isolation diffusions, and NBL regions.

![[diagrams/ch04-standard-bipolar-fig1.png]]

### Figure 4.9 -- NPN Transistor Layout and Cross Section

This page shows the representative layout and cross section of a minimum-area NPN transistor with deep-$N^+$ sinker and NBL. The vertical carrier flow path from emitter through base to collector is visible, along with the low-resistance collector contact path through deep-$N^+$ and NBL.

![[diagrams/ch04-standard-bipolar-fig2.png]]

### Figure 4.11 -- Lateral PNP Transistor Layout and Cross Section

This page shows the lateral PNP transistor where both collector and emitter are base diffusions in an N-tank. The collector encircles the emitter (visible as two collector regions in the cross section). NBL beneath the structure suppresses the parasitic substrate PNP.

![[diagrams/ch04-standard-bipolar-fig3.png]]

## Practical Takeaways

- **Maximize NPN usage:** The vertical NPN is the best-performing device on standard bipolar; design circuits to rely on it as much as possible.
- **NBL spacing from isolation:** Always space NBL inside the isolation diffusion boundary to avoid low-voltage avalanche at the $N^+/P^+$ intersection.
- **NBL under lateral PNP is mandatory:** Without it, parasitic substrate PNP action steals most of the emitter current, collapsing lateral PNP $\beta$.
- **NBL must be omitted from substrate PNP:** Its presence severely degrades substrate PNP $\beta$.
- **Base over isolation (BOI)** serves as a free channel stop -- implanting base across isolation regions raises the thick-field NMOS threshold without an extra mask.
- **Alignment to NBL shadow requires pattern shift correction:** The isolation mask must be offset to account for the diagonal propagation of the NBL shadow during epitaxial growth.
- **Overdrive junctions by ~25%:** Insufficient drive of isolation and deep-$N^+$ leaves lightly doped junction bottoms vulnerable to depletion punchthrough.
- **Use thick emitter oxide:** Newer processes use wet oxidation or oxide deposition over emitter to prevent ESD damage (avoid the "thin emitter oxide" problem).
- **Pinch resistors are a last resort:** High variability and voltage modulation make them unsuitable for precision applications. Use only in startup circuits or where exact resistance values do not matter.
- **Consider HSR extension early:** If the circuit needs more than 100-200 k$\Omega$ of total resistance, the extra mask step for high-sheet resistors is cost-effective.
- **DLM is almost always worthwhile:** The die area savings (up to 40%) and layout simplification far outweigh the cost of two additional masks.
- **Up-down isolation saves 15-25% die area** and is standard on many modern versions of the process.
- **Junction capacitors are highly variable ($\pm 25\%$+):** Acceptable for loop compensation but not for precision applications.

## Relation to the Bigger Picture

Standard bipolar is the historical foundation upon which all subsequent analog IC processes were built. The NPN transistor structure, junction isolation concepts, buried layers, and sinker diffusions introduced here reappear directly in analog BiCMOS processes (Section 4.3), which merge these bipolar elements with CMOS transistors. Understanding the fabrication sequence and its constraints -- why NBL uses low-diffusivity dopants, why the isolation must be overdriven, why base width is set by diffusion rather than lithography -- is essential for grasping the parasitic mechanisms and matching principles discussed extensively in later chapters (Chapters 5, 9, and beyond). The available devices catalog (NPN, lateral PNP, substrate PNP, base/emitter/pinch resistors, junction capacitors) establishes the component vocabulary that analog circuit designers work with, and the tradeoffs among these devices inform virtually every layout decision in bipolar and BiCMOS design.

## See Also
- [[ch04-poly-gate-cmos]]
- [[ch01-bipolar-transistors]]
