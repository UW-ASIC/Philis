---
title: "9.2 Standard Bipolar Small-Signal Transistors"
chapter: 9
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-9, bipolar, NPN, PNP, lateral-PNP, substrate-PNP, super-beta, high-voltage]
---

# 9.2 Standard Bipolar Small-Signal Transistors

> **Chapter 9: Bipolar Transistors**

## Key Concepts

The standard bipolar process was originally designed in the 1970s for digital logic -- small, fast NPN transistors switching at 10--100 MHz from a 5 V supply, drawing no more than a few tens of milliamps. Analog designers (notably Robert Widlar) adapted this process to create analog ICs. The key insight is that by slightly thickening the epitaxial layer, the same process that builds digital NPNs can produce small-signal NPN transistors with $BV_{CEO}$ ratings of 40--60 V, plus acceptable PNP transistors repurposed from existing NPN layers.

This section covers five distinct transistor types available in standard bipolar:

1. **Vertical NPN** -- the primary, fully-optimized device
2. **Substrate PNP** -- a vertical PNP using the substrate as collector (not fully isolated)
3. **Lateral PNP** -- an isolated PNP using lateral carrier flow between two base diffusions
4. **High-voltage transistors** -- extensions for higher $BV_{CEO}$ ratings
5. **Super-beta NPN** -- extremely high $\beta$ devices for low-input-current applications

The fundamental tension throughout is that no single device structure can simultaneously optimize all parameters. Beta trades off against Early voltage (their product $\beta \cdot V_A$ is a figure of merit around 20 kV for standard bipolar NPN). Higher breakdown voltages require thicker epi, which increases parasitic resistance and area. Lateral PNP transistors are inherently slower than vertical NPNs but offer full isolation. Every layout decision involves these trade-offs.

---

## 9.2.1 The Standard Bipolar Vertical NPN Transistor

### Device Structure

The vertical NPN is the star of standard bipolar. Its structure includes five critical features optimized for performance:

| Layer | Role | Key Properties |
|-------|------|---------------|
| **Emitter diffusion** | Maximizes emitter injection efficiency | Heavily doped with phosphorus; sheet resistance $> 3\ \Omega/\square$ to avoid slip defects |
| **Base diffusion** | Sets $\beta$ and $V_A$ | $\sim 1\ \mu\text{m}$ deep, sheet resistance $\sim 200\ \Omega/\square$; graded doping from $\sim 10^{18}\ \text{cm}^{-3}$ (emitter-base junction) to $\sim 10^{16}\ \text{cm}^{-3}$ (collector-base junction) |
| **N-epi (drift region)** | Lightly doped collector region | Doped $\sim 10^{15}\ \text{cm}^{-3}$ phosphorus; allows depletion region to extend into collector rather than base |
| **NBL (N-type buried layer)** | Bounds drift region; reduces lateral collector resistance | Arsenic or antimony, peak doping $\sim 10^{20}\ \text{cm}^{-3}$, sheet resistance 30--$50\ \Omega/\square$ |
| **Deep-$N^+$ sinker** | Low-resistance path from collector contact to NBL | Phosphorus, surface concentration $\sim 10^{20}\ \text{cm}^{-3}$, driven to $\sim 120\%$ of epi thickness |

**Why the graded base matters:** The base diffusion's doping gradient (from heavy near the emitter to light near the collector) creates a built-in electric field that drifts minority carriers across the base. This reduces base transit time and therefore increases the maximum operating frequency $f_T$. The elevated doping near the emitter-base junction also suppresses high-level injection.

**Why NBL uses arsenic/antimony instead of phosphorus:** These heavier dopant atoms diffuse much more slowly than phosphorus (critical since the NBL must remain well-defined after subsequent thermal processing), and their atomic size more closely matches silicon, allowing very heavy doping ($\sim 10^{20}\ \text{cm}^{-3}$) without generating slip defects.

### Emitter Pipes

The emitter must not be doped too heavily. Excessive phosphorus causes lattice strain due to the mismatch in atomic sizes. This strain can produce slip defects -- lateral displacements between adjacent atomic layers. Dopants diffuse rapidly along these defects. A slip defect that penetrates through the base creates a direct collector-to-emitter short known as an **emitter pipe**. Emitter sheet resistance should be kept above about $3\ \Omega/\square$ (for a typical junction depth) to minimize this risk. Substituting part of the phosphorus with arsenic or antimony reduces strain.

### Reachthrough Collector and Quasisaturation

Most standard bipolar processes use epi thickness and doping such that the collector-base depletion region reaches the NBL interface before maximum $V_{CE}$ -- this is called a **reachthrough collector**. The lightly doped drift region has significant resistance, which leads to a phenomenon called **quasisaturation**:

- The transistor internally saturates at terminal voltages that appear to imply forward-active operation, because collector current flowing through the drift resistance creates a voltage drop that reduces the actual junction voltage below what the terminal voltages suggest.
- Quasisaturation appears as a shallow-sloped region in the $I_C$-$V_{CE}$ curve between the forward-active region and hard saturation.
- As $V_{CE}$ decreases, base current injection into the drift region increases and the conductivity-modulated zone widens. When it penetrates entirely through the drift region, the transistor enters **hard saturation** (the knee between the two regions).

The critical current to trigger quasisaturation is approximately $1\ \text{mA}/\mu\text{m}^2$ for a standard bipolar process with 40 V rating. For optimum saturation performance, the forced beta should not exceed one-tenth of the forward-active beta.

### Kirk Effect (Base Push-Out)

At very high collector current densities, electrons traversing the collector-base depletion region represent non-negligible space charge. This charge:
- Adds to acceptor charge on the base side of the metallurgical junction
- Subtracts from donor charge on the collector side
- Causes the depletion region to retreat from the base and extend further into the collector

When electron space charge completely cancels the donor charge, the depletion region leaps across the drift region to the NBL. The effective base suddenly widens into the collector, reducing $\beta$ and increasing base transit time. This is the **Kirk effect**. It roughly coincides with the upper limit of useful collector current density. High-voltage transistors (with lighter drift doping) are more susceptible; standard bipolar devices are more affected by high-level injection into the base than by the Kirk effect.

### Collector Resistance

The extrinsic collector resistance of a minimum-area NPN is typically $\sim 200\ \Omega$, roughly half from the lateral NBL resistance and half from the vertical deep-$N^+$ sinker resistance. The deep-$N^+$ sinker has an areal resistance of $\sim 25\ \Omega\cdot\text{mm}^2$. Omitting the sinker raises collector resistance dramatically to the order of $\sim 5\ \text{k}\Omega$.

### Construction and Layout

Two primary layout styles exist:

- **CEB (Collector-Emitter-Base):** Emitter placed between collector and base contacts. Slightly lower collector resistance because emitter and collector contacts are closer.
- **CBE (Collector-Base-Emitter):** Base contact between collector and emitter contacts. Often interchanged with CEB; the difference is usually negligible. Using both styles can simplify lead routing in single-level-metal designs.

**Emitter sizing and beta:** Effective emitter area is NOT the same as drawn emitter area. Sidewalls contribute to conduction, and the junction extends outside the oxide window. Crucially, carriers injected laterally across emitter sidewalls enter a more heavily doped base region with more recombination and surface trap sites. Therefore:
- Smaller emitters have **lower** betas than large ones (lower area-to-periphery ratio)
- Example: on a standard bipolar op-amp, a $25 \times 30\ \mu\text{m}^2$ emitter had peak $\beta = 290$ while a $100 \times 150\ \mu\text{m}^2$ emitter had peak $\beta = 520$

**Layout guidelines for the vertical NPN:**
- Emitter contact should be as large as possible within the emitter area to minimize emitter resistance
- Base diffusion must overlap emitter on all sides to prevent lateral punchthrough (include two-level mask misalignment allowance)
- Base is usually contacted along only one side; elongating the base contact to the full width of the base diffusion significantly reduces base resistance without increasing area
- Deep-$N^+$ sinker should be elongated across the full tank width to minimize collector resistance
- NBL should fill as much of the tank as possible (reduces collector resistance, blocks punchthrough, minimizes substrate injection)
- Drawn NBL geometry should at least touch, preferably slightly overlap, the drawn deep-$N^+$ geometry

### Stretched Transistors

In single-level-metal designs, leads must route through transistors. Three stretched variants exist:

- **Stretched-collector:** Collector and base contacts moved apart for lead routing. Slightly increases collector resistance and substantially increases collector-substrate capacitance. Elongate the deep-$N^+$ sinker to compensate for resistance increase.
- **Stretched-base:** Base region elongated for lead routing between base and emitter contacts. Increases base resistance and $C_{CB}$ -- worse than stretched-collector, use only when necessary.
- **Tunnel-through-base:** Lead passes through base diffusion. Inserts $\sim 5\ \text{k}\Omega$ resistance between the two base contacts (for a $200\ \Omega/\square$ base sheet).

Double-level metal (DLM) largely eliminates the need for stretched transistors.

### Scaling Up NPN Transistors

**Compact-emitter transistor:** Enlarges the emitter while keeping the same overall geometry. Higher beta (better area-to-periphery ratio), but the lightly doped pinched base beneath the emitter ($2\text{--}10\ \text{k}\Omega/\square$) introduces phase shifts, slows switching, and causes **emitter crowding** -- portions of the emitter closest to the base contact inject more carriers since even 18 mV of debiasing doubles the emitter current. This reduces effective emitter area, complicates matching, and increases susceptibility to secondary breakdown.

**Narrow-emitter (finger) transistor:** Uses long, narrow emitter stripes with base contacts on both sides. Lower beta (poor area-to-periphery), but much faster switching and lower base resistance ($\sim 1/4$ of single-base layout). Vulnerable to thermal runaway at emitter current densities above a few $\text{mA}/\mu\text{m}$ of emitter.

**Double-base transistor:** Minimum-geometry emitter stripe with base contacts on both sides. Base resistance $\approx 1/4$ of single-base layout.

---

## 9.2.2 The Standard Bipolar Substrate PNP Transistor

### Device Structure

The substrate PNP repurposes NPN layers without any additional process steps:

| NPN Layer | Substrate PNP Role |
|-----------|--------------------|
| Base diffusion | **Emitter** |
| N-epi (tank) | **Base** (contacted via emitter diffusion) |
| P-substrate + isolation | **Collector** |

This confusing naming convention exists because the process was designed for NPN transistors; nobody anticipated PNP use.

### Performance Characteristics

- **Emitter injection efficiency** is reduced because the base diffusion ($\sim 5 \times 10^{18}\ \text{cm}^{-3}$ surface doping) is much lighter than the NPN emitter ($> 10^{20}\ \text{cm}^{-3}$)
- **High-level injection** begins at $\sim 10\ \mu\text{A}/\mu\text{m}^2$ (vs. $\sim 100\ \mu\text{A}/\mu\text{m}^2$ for NPN)
- **Emitter resistance:** A minimum-emitter substrate PNP has $\sim 200\ \Omega$ (vs. $\sim 5\ \Omega$ for comparable NPN)
- **Peak beta** typically $\sim 100$ in a 40 V process, at collector current density slightly less than $10\ \mu\text{A}/\mu\text{m}^2$
- Beta depends on epi thickness (which is set by NPN $BV_{CEO}$ requirements)
- The lightly doped substrate collector gives high Early voltage and high $BV_{CEO}$

### Substrate Debiasing Limits

Since the collector is the substrate, collector current flows through the substrate. This can cause **substrate debiasing** that disrupts other devices. Guidelines:
- Keep each substrate PNP's collector current $\leq 1\ \text{mA}$
- Keep total substrate current $\leq 10\ \text{mA}$
- Place substrate contacts near substrate PNP transistors
- If $I_C > 1\ \text{mA}$, add additional substrate contacts around the transistor
- For higher currents, consider replacing with a lateral PNP (which does not inject into the substrate when operating in forward-active)

### Construction Styles

Three layout styles are described:

**Standard substrate PNP (Figure 9.18A):** Square or rectangular base diffusion emitter inside an N-tank. Base contact (emitter diffusion) on one side, spaced from the emitter by minimum base-to-emitter spacing. Larger, more compact emitter structures produce higher betas (deeper base diffusion narrows the base width; fewer sidewall-injected carriers reach isolation).

**Emitter-ringed substrate PNP (Figure 9.18B):** Rectangle of base diffusion surrounded by a thin ring of emitter diffusion (drawn touching, not overlapping). The emitter ring raises the voltage needed to forward-bias the emitter sidewalls, suppressing lateral conduction and boosting low-current beta. Also acts as a channel stop, eliminating the need for a field plate. However, it requires more room, and the advantage disappears at higher current densities.

**Verti-lat (tombstone/cathedral) PNP (Figure 9.18C):** Circular base diffusion emitter plug with a semicircular tank. Combines vertical conduction (downward to substrate) with lateral conduction (outward to a partial ring of base diffusion collector). Can include a strip of NBL from beneath the base contact toward the emitter to reduce base resistance, but the NBL must not come close enough to the emitter to interfere with vertical conduction. Requires field plating of exposed tank surface. In practice, the emitter-ringed structure usually outperforms the verti-lat because it suppresses lateral conduction in favor of the more efficient vertical path.

**Interdigitated substrate PNP (Figure 9.19):** For higher currents. Uses wide ($\sim 25\ \mu\text{m}$) base diffusion emitter stripes with thin emitter diffusion base contacts between them. Pinched sheet resistance beneath the base diffusion may exceed $10\ \text{k}\Omega/\square$, so emitter stripes wider than 25--$30\ \mu\text{m}$ are inadvisable. Requires a large surrounding substrate contact ring.

### Matching Considerations

Substrate PNP saturation current scales with emitter area, but not linearly (due to outdiffusion, lateral conduction, doping profile effects). For accurate matching:
- Use **identical emitter geometries**
- For different sizes, use **multiple copies of a unit emitter** separated by enough distance that several microns of undepleted base (N-epi) exists between them

---

## 9.2.3 The Standard Bipolar Lateral PNP Transistor

### Operating Principle

Invented by H. C. Lin in 1963. Two separate base diffusions placed in a common N-epi tank: one acts as the emitter, the other as the collector. When the emitter-base junction forward biases, holes diffuse **laterally** through the N-epi to the collector. The device is inherently slower than vertical transistors, but proper design can substantially improve beta.

The layout designer controls two of the five factors that determine beta:
1. Emitter injection efficiency -- set by process (moderate base diffusion doping)
2. Base doping -- set by process
3. Base recombination rate -- set by process
4. **Base width** -- controllable by layout
5. **Collector efficiency** -- controllable by layout

### Beta-Early Voltage Trade-off

The product $\beta \cdot V_A$ remains approximately constant. Wider base increases Early voltage at the expense of beta; narrower base raises beta at the expense of Early voltage. This trade-off is fundamental and allows the layout designer to tweak performance.

### Base Width Details

Three distinct base widths matter:

- **Drawn base width** $W_D$: separation between the drawn base diffusions (emitter and collector)
- **Actual surface base width** $W_S$: substantially smaller than $W_D$ due to outdiffusion from both sides and depletion region encroachment. No misalignment allowance is needed because the same mask creates both emitter and collector (self-aligned).
- **Effective base width** $W_E$: a weighted average of all carrier paths. Carriers near the surface travel shorter distances; carriers deeper travel through curving paths with wider base widths. The effective base width substantially exceeds $W_S$.

A practical consequence: beta scales **less than linearly** with the inverse of drawn base width. For example, halving $W_D$ yields a beta reduction of less than half. If $W_D = 8\ \mu\text{m}$ gives $\beta = 80$, then $W_D = 4\ \mu\text{m}$ gives $\beta > 40$.

### Role of NBL

NBL beneath the lateral PNP is critical. It creates a **high-low junction** that repels holes attempting to diffuse downward to the substrate (see [[ch09-bjt-operation]] Section 9.1.4 for collector efficiency discussion). Without NBL, collector efficiency drops below 0.5. With NBL, less than 1% of injected holes reach the isolation sidewalls.

### The Nitride Overcoat Effect on Beta

Early standard bipolar processes with oxide protective overcoats produced lateral PNP peak betas $< 10$. The introduction of **compressive nitride overcoats** dramatically increased beta (modern processes sometimes exceed 500). The mechanism: hydrogen released during nitride deposition (from silane + ammonia reaction) diffuses to the silicon surface during contact sinter, tying off dangling bonds and eliminating surface recombination centers. Processes with oxide overcoats can achieve the same improvement by performing the contact sinter in **forming gas** (nitrogen + hydrogen mixture).

### Construction and Layout

**Traditional circular geometry (Figure 9.21):** A small circular base diffusion plug (emitter) inside a circular hole in a larger base diffusion rectangle (collector). This annular collector intercepts nearly all carriers before they reach the isolation.

Key layout rules:
- **Circular emitter should be minimum size** -- smaller emitters have shorter effective base widths and higher betas
- Use **polygonal approximations** with 32 or 64 sides (divisible by 4 for X/Y symmetry)
- Annular collector can be coded as two touching geometries or a single semisimple geometry
- **NBL should enclose the emitter** and extend to the drawn inner edge of the collector (at minimum)
- Deep-$N^+$ sinker is not normally required (base current rarely exceeds tens of $\mu$A)
- Collector contact: one end of the collector rectangle extends sufficiently for contact placement

### Field Plating (Critical)

A metal field plate connected to the **emitter** must cover all exposed N-epi between the drawn emitter and drawn collector. The field plate should overlap the collector periphery by 2--$3\ \mu\text{m}$. Purpose:
- Prevents unpredictable surface fields from modulating the Gummel number and thus beta
- Eliminates anomalous current steps that can appear at $V_{CE}$ as low as 5--10 V (well below the thick-field threshold)
- Suppresses beta instabilities from mobile ion contamination

Without field plating, beta fluctuates with surface potentials and exhibits time-dependent drift.

### Split-Collector Lateral PNP

A single lateral PNP can be subdivided into multiple transistors sharing common base and emitter connections. Each collector segment receives a share of emitter current proportional to the fraction of emitter periphery it subtends.

- **1/2--1/2 split:** Two half-collectors, each receiving half the total injected current
- **1/4--1/4--1/4--1/4 split:** Four quarter-collectors
- **Unequal splits:** e.g., three $3/10$ collectors and two $1/20$ collectors

Matching accuracy: $\pm 2\%$ when split collectors have identical geometries and are placed symmetrically about the emitter. Collectors of different geometries will NOT accurately match one another, even if the intended ratios are simple fractions.

Split-collector laterals are commonly used for **current mirrors**. A 1:1 mirror connects one collector back to the common base (reference) and uses the other as output. This saves area but cannot use emitter degeneration (shared emitter) and matches less precisely than two separate lateral PNPs.

### Square vs. Circular Geometry

Square emitters are easier to digitize but have slightly wider effective base widths at the diagonal corners. Designers can add **fillets** (rounded corners) on the collector opening to partially compensate. The lack of radial symmetry limits split-collector options to half and quarter collectors.

### Scaling Up Lateral PNP Transistors

- **Elongated-emitter (hot-dog) transistor:** Emitter stretched into a thin stripe. Area and periphery increase at similar rates. Beta decreases slightly as emitter lengthens (carriers moving along the stripe contribute to effective base width), but far less than with an enlarged square emitter.
- **Arrayed-emitter transistor:** Multiple minimum-size emitters arrayed; current handling is linearly proportional to emitter count. Large arrays use hexagonal packing. This is preferred for accurate scaling.

### Scaling Formula

Lateral PNP transistors are scaled by drawn emitter periphery. The size assigned to a given collector is:

$$M = \frac{P_E}{P_U} \cdot \frac{P_C}{P_{CA}}$$

where $P_E$ = perimeter of actual emitter(s), $P_U$ = perimeter of a unit (minimum) emitter, $P_C$ = perimeter of the collector under consideration facing the emitter, and $P_{CA}$ = sum of all collector perimeters facing the emitter.

---

## 9.2.4 High-Voltage Bipolar Transistors

### Voltage Rating Fundamentals

The maximum operating voltage of the process is set by the lower of the NPN and PNP $BV_{CEO}$ ratings. For standard bipolar, the NPN usually breaks down first. The NPN $BV_{CEO}$ relates to the planar collector-base breakdown voltage $BV_{CB(planar)}$ by:

$$BV_{CEO} = \frac{BV_{CB(planar)}}{\beta_{peak}^{1/n}}$$

where $n$ typically lies in the range 3--4. Lower-beta devices have higher $BV_{CEO}$.

Manufacturers offer several epi thickness/doping choices corresponding to convenient operating voltages (20, 40, or 60 V). **Always use the lowest possible voltage rating** to minimize isolation spacings.

### Sidewall Curvature and Breakdown

Junction breakdown voltage depends on curvature -- sharper curvature means lower breakdown. All patterned diffusions have characteristic sidewall curvature, and **corners** have even more curvature than sidewalls. For example:
- A base-collector junction with 120 V planar breakdown may actually break down at only 60 V due to sidewall curvature
- 90-degree corners can reduce breakdown voltage by $\sim 30\%$ for base diffusions with $\sim 3\ \mu\text{m}$ junction depth
- HSR (shallow) diffusions suffer even larger reductions

### Fillets and Chamfers

When operating voltages push base or HSR diffusions close to their breakdown limits:
- **Fillets** (rounded corners tangent to both sides): radius $\geq 150\%$ of junction depth
- **Chamfers** (diagonal facets): segment length similar to fillet radius
- Apply to both inside and outside corners
- Fillets slightly outperform chamfers (chamfers still contain obtuse $135\degree$ vertices)
- Filleting tank geometries is usually unnecessary since isolation diffusion is deep enough that the NBL sets the isolation/substrate breakdown voltage

### Voltage Recognition Layers

High-voltage layouts use **pseudolayers** called voltage recognition layers to apply voltage-dependent spacing rules selectively. One layer per voltage level (except the highest, which is default). For example, with 20/40/60 V rules: V20 and V40 layers are defined. An NPN enclosed by V20 is checked to 20 V rules; enclosed by V40, to 40 V rules; enclosed by neither, to 60 V rules.

Two approaches:
1. **Device-level:** Enclose entire devices in voltage recognition layers
2. **Segment-level:** Assign voltage levels to individual boundary segments of diffusions (more sophisticated, allows mixed-voltage devices like a two-collector lateral PNP with collectors at different voltages)

### Caution on Extending Voltage Ratings

Attempting to raise $BV_{CEO}$ by inserting resistance between base and ground (operating in $BV_{CER}$ rather than $BV_{CEO}$) is **dubious**. Transistors pushed beyond their $BV_{CEO}$ rating are seldom specified or controlled by the fab and may catastrophically fail via snapback from $BV_{CER}$ to $BV_{CES}$. Use a process with a higher rated voltage instead.

---

## 9.2.5 Super-Beta NPN Transistors

### Purpose and Structure

For applications requiring very low input bias currents (e.g., low-input-current op-amps), standard NPN beta ($\sim 200$) is insufficient. With $50\ \mu\text{A}$ per side bias, input currents of $\sim 250\ \text{nA}$ result, potentially approaching $1\ \mu\text{A}$ with process and temperature variation.

Super-beta transistors achieve peak $\beta \sim 5000$ with Early voltage of only 2--3 V by reducing the Gummel number. Two fabrication approaches:
1. **Alternate base mask** producing a shallower base
2. **Alternate emitter mask** (more common) producing a deeper emitter

Since base doping diminishes with depth, both approaches reduce both base width and base doping simultaneously.

The typical process flow (for the alternate emitter approach):
1. Perform base drive
2. Etch oxide windows where super-beta emitters are desired
3. Implant/diffuse phosphorus into these openings
4. Partial drive to move phosphorus partway toward final junction depth
5. Grow oxide over super-beta emitters during the drive
6. Pattern, etch, and implant regular emitters
7. Regular emitter drive moves both super-beta and regular emitters to final depths

The super-beta emitter ends up deeper than the standard emitter, creating a narrower neutral base.

### Limitations

- Very narrow base width is extremely difficult to control
- $\beta$, $V_A$, and $V_{CE(max)}$ all vary far more than for regular NPN
- Low $V_A$ (2--3 V) means severe Early effect
- Low $BV_{CEO}$ due to punchthrough susceptibility
- Limited to specialized applications: input stages of low-current amplifiers and comparators

CMOS transistors have largely replaced super-beta devices -- the gate current of a MOS transistor with gate oxide $> 100\ \text{A}$ is effectively zero. However, MOS has larger input offset and much worse $1/f$ noise (from trapping/detrapping at the oxide interface), so bipolar and JFET transistors remain preferred for low-noise amplifiers.

---

## Diagrams

### Figure 9.11 -- Standard Bipolar Vertical NPN Cross-Section (p. 441)
![[diagrams/ch09-standard-bipolar-transistors-fig1.png]]
*Cross section of the standard bipolar vertical NPN transistor showing all key features: the heavily doped emitter diffusion, tailored base diffusion, lightly doped N-epi drift region, NBL, and deep-$N^+$ sinker. This is the foundational device that the entire standard bipolar process is optimized to construct.*

### Figure 9.13 -- CEB and CBE NPN Layout Styles (p. 445)
![[diagrams/ch09-standard-bipolar-transistors-fig2.png]]
*Two standard layout styles for NPN transistors: (A) Collector-Emitter-Base (CEB) with emitter between collector and base contacts, and (B) Collector-Base-Emitter (CBE) with base between collector and emitter contacts. The CEB layout slightly reduces collector resistance. Both are often used interchangeably, especially in single-level-metal designs where varying the terminal ordering simplifies routing.*

### Figure 9.21 -- Lateral PNP Layout with Field Plate (p. 453)
![[diagrams/ch09-standard-bipolar-transistors-fig3.png]]
*Layout of a lateral PNP transistor. The circular base diffusion plug at center is the emitter; the surrounding annular base diffusion is the collector. The metal field plate (connected to emitter) covers all exposed N-epi between emitter and collector, preventing surface charge effects from modulating beta. Note the emitter diffusion strip at one end of the tank for the base contact, and NBL covering most of the tank area.*

---

## Practical Takeaways

### Vertical NPN Layout
- Use CEB or CBE layouts interchangeably for routing flexibility; CEB has slightly lower collector resistance
- Emitter contact should fill as much of the emitter area as possible to minimize $R_E$
- Always include deep-$N^+$ sinker unless area is extremely constrained and current is very low ($< 1\ \text{mA}$)
- Fill the tank with NBL -- it reduces collector resistance, blocks punchthrough, and minimizes substrate injection during saturation
- For scaled-up devices, use narrow-emitter (finger) layouts with double base contacts for high-speed applications, or compact-emitter layouts for high-beta/low-speed applications
- Avoid emitter stripes wider than about $25\ \mu\text{m}$ to prevent excessive current crowding

### Substrate PNP Layout
- Keep collector current $\leq 1\ \text{mA}$ per transistor, $\leq 10\ \text{mA}$ total, to avoid substrate debiasing
- Place substrate contacts adjacent to substrate PNPs
- For matching, use identical emitter geometries; for different sizes, use multiple unit emitters with several microns of undepleted base between them
- Emitter-ringed structure provides higher low-current beta and acts as channel stop (no field plate needed)
- Emitter stripes wider than 25--$30\ \mu\text{m}$ are inadvisable due to high pinched sheet resistance beneath the base diffusion

### Lateral PNP Layout
- **Always field-plate** the exposed N-epi between emitter and collector (connect field plate to emitter, overlap collector by 2--$3\ \mu\text{m}$)
- Use minimum-size circular emitters for highest beta
- Use 32- or 64-sided polygonal approximations (divisible by 4)
- Fill the tank with NBL (at minimum, NBL should enclose emitter and extend to drawn inner edge of collector)
- For matched split collectors, all segments must have identical geometries and be placed symmetrically about the emitter ($\pm 2\%$ matching)
- Scale lateral PNPs by drawn emitter periphery, not area
- For larger devices, prefer arrayed emitters (with hexagonal packing) over elongated emitters for accurate scaling

### High-Voltage Design
- Always use the lowest possible voltage rating to minimize area
- Apply fillets (radius $\geq 1.5 \times$ junction depth) or chamfers to corners of base and HSR diffusions when operating near breakdown limits
- Do not attempt to extend voltage ratings beyond $BV_{CEO}$ using circuit tricks -- use a higher-voltage process instead
- Use voltage recognition pseudolayers to apply voltage-dependent design rules selectively

### Super-Beta NPN
- Reserve for specialized applications (low-bias-current amplifier input stages)
- Expect extreme parameter variability: $\beta$, $V_A$, $BV_{CEO}$ all vary much more than standard NPN
- Early voltage is only 2--3 V -- account for the severe Early effect in circuit design
- Consider MOS or JFET alternatives if $1/f$ noise and offset requirements permit

---

## Relation to the Bigger Picture

Section 9.2 bridges the theoretical discussion of bipolar transistor operation in [[ch09-bjt-operation]] (Section 9.1) with the practical realities of laying out these devices. The vertical NPN, substrate PNP, and lateral PNP form the core analog component set of standard bipolar and many BiCMOS processes. Understanding their structure, parasitic effects (substrate injection, current hogging, quasisaturation, the Kirk effect), and layout trade-offs is essential for all analog IC design work. The concepts introduced here -- collector efficiency, field plating, split-collector current mirrors, emitter crowding, reachthrough collectors -- recur extensively in Chapter 10 (matching and power bipolar devices) and Chapter 14 (merged devices). The discussion of CMOS and BiCMOS variants of these transistors continues in [[ch09-cmos-bicmos-transistors]] (Section 9.3), where shallow wells, CDI NPNs, and SiGe devices extend the standard bipolar concepts to modern process technologies.

---

## See Also
- [[ch09-bjt-operation]]
- [[ch09-cmos-bicmos-transistors]]
