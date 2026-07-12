---
title: "15.1 Die Area Estimation"
chapter: 15
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-15, die-area, cost-estimation, floorplanning]
---

# 15.1 Die Area Estimation

> **Chapter 15: Assembling the Die**

## Key Concepts

Die area estimation is the critical first step of the layout assembly process. Contrary to common belief, these estimates can be quite accurate -- within $\pm 10\%$ -- provided that system architects and circuit designers invest adequate planning time. Accurate area estimates are essential for quoting prices, choosing packages, and overall project planning.

The estimation process follows a structured pipeline:

1. **Partition** the design into a hierarchy of cells
2. **Populate** the database with component schematics
3. **Compute cell areas** using empirical formulas or quick autoplacement
4. **Sum up** all cell areas with routing overhead and packing factors
5. **Determine** whether the design is core-limited or pad-limited
6. **Estimate cost** from die area, yield, and assembly costs

The total die area is not just the circuitry -- it equals the sum of the core area (circuitry), the padring (bondpads, ESD structures, scribe seals, ground rings), scribe streets, and a safety margin.

## Partitioning and Populating the Database

### Cell Hierarchy

Layout editors store data in **cells**. A cell contains polygons, paths, and primitive shapes, as well as **instances** (references to other cells). Each instance specifies a location, a transformation, and the name of the instantiated cell. The cell containing the instance is the **parent cell**; the instantiated cell is the **child cell**. A cell with no instances is a **leaf cell**.

The hierarchy can be visualized as a tree diagram. The top level always contains exactly one **top cell**, which represents the entire integrated circuit. Few practical designs require more than 10-12 levels of hierarchy.

**Partitioning** -- the process of defining the hierarchy -- is performed by the lead circuit designer. The quality of partitioning directly impacts how easy the design is to comprehend, which speeds development and reduces errors. Key guidelines:

- Circuit designers partition by simulation needs: start with small blocks (Schmitt triggers, comparators), each in its own cell, then compose them into larger analog subsystems
- Keep cells manageable -- all details should be visible on a B-sized sheet ($11 \times 17$ in). This limits cells to about 50-100 components
- **Matched components** that occupy an array should reside in the same cell (easier to lay out arrays when all components are together)
- Cell names should be meaningful, relatively short, all lower-case, start with a letter, use only letters/numerals/underscores, and not exceed 32 characters (GDSII compatibility)

### Populating the Design

Once partitioned, the lead designer builds the database bottom-up: leaf cells first (with just pins), then instances placed in parent cells, working up to the top cell. Pin naming follows similar conventions to cell names (short, meaningful, lower-case, starting with letter or numeral).

**Bus conventions**: Most digital designers start bus indices at zero and list the upper limit first (e.g., `data(7:0)`). An index of zero always represents the least significant bit.

**Pin directions** (input, output, input/output) allow the schematic editor to check:
- A net connected only to input pins is flagged as floating
- A net connected to two output pins is flagged as shorted
- For analog circuits, most pins that are not obviously input or output should be marked input/output

**Signal naming conventions**: Appending "z" or "b" to negative-logic signals (e.g., `enableb` is active-low). Voltage domain suffixes like "1p5" or "3p3" help identify which power domain a signal belongs to.

After populating, circuit designers sketch what circuitry each cell contains. These initial schematics are rough estimates, but skilled designers can guess reasonably well. Components that consume significant area (large capacitors, high-value resistors, matched components, power devices) should be assigned realistic values, since they form the basis of area estimates.

## Computing Cell Areas

### The Cell Area Formula

The area $A_c$ of a cell is estimated using:

$$A_c = P_c \sum A_i$$

where $\sum A_i$ is the sum of all individual component areas, and $P_c$ is the **cell-level packing factor** that accounts for isolation, interconnection, and imperfect packing.

Typical packing factors:

| Process | Packing Factor $P_c$ |
|---------|----------------------|
| Standard bipolar, SLM | 1.5 -- 3.0 |
| Standard bipolar, DLM | 1.3 -- 2.5 |
| CMOS/BiCMOS, 2+ metal layers | 1.2 -- 1.8 |

Lower values represent tightly packed designs with custom devices and mergers; higher values represent hastily created layouts with standardized components. Additional metal layers beyond two do not substantially improve packing density for typical analog circuitry.

### Component Area Formulas

**Resistors** -- The area $A_R$ for one or more resistors of the same material and width:

$$A_R = 1.2 \cdot \frac{R}{R_s} \cdot (W_R + S_R)$$

where $R$ is the desired resistance, $R_s$ is the sheet resistance, $W_R$ is the resistor width, and $S_R$ is the spacing between adjacent segments. The factor 1.2 accounts for dummy resistors, contact heads, and layout inefficiency.

**Capacitors** -- The area $A_C$ for parallel-plate capacitors:

$$A_C = 1.1 \cdot \frac{C}{C_A}$$

where $C$ is the desired capacitance and $C_A$ is the capacitance per unit area. The factor 1.1 accounts for terminations. For a homogeneous dielectric:

$$C_A = \frac{\varepsilon_r}{4\pi t} \quad (\text{in } \text{pF}/\mu\text{m}^2)$$

where $\varepsilon_r$ is the relative permittivity (about 3.9 for dry oxide) and $t$ is the dielectric thickness in Angstroms.

**MOS Transistors** -- The area $A_T$ for a typical CMOS transistor:

$$A_T = 1.3 \cdot W_G \cdot (L_G + S_G)$$

where $W_G$ is the gate width, $L_G$ is the gate length, and $S_G$ is the spacing between adjacent gate stripes of a multiple-section transistor. The factor 1.3 accounts for terminations, well spacings, and packing.

**Bipolar Transistors** -- Cannot be reliably estimated from emitter area alone, because the transistor area depends more on spacings and overlaps than on emitter area. The best approach is to lay out representative devices and record their areas. A useful set of sample devices includes: minimum-area vertical NPN, single-emitter NPN for matching, four-emitter NPN for matching, minimum-area lateral PNP, and minimum-area substrate PNP.

**MOS Power Transistors** -- For interdigitated layouts that scale linearly with on-resistance:

$$A_{PT} = \frac{R_{sp}}{R_{on} - R_{pkg}}$$

where $R_{sp}$ is the specific on-resistance, $R_{on}$ is the desired on-resistance, and $R_{pkg}$ is the package resistance (metallization, bondwires, leadframe). The accuracy depends on the desired transistor resembling the reference device (same type, gate length, $V_{GS}$). On-resistance varies $\pm 20\%$ over process and increases roughly 50% from $25°C$ to $125°C$, so worst-case high-temperature $R_{on}$ can be nearly twice the nominal value.

**Bipolar power transistors** have no simple area formula and generally require preliminary layouts.

Large power devices should be estimated separately from cells (not multiplied by the cell packing factor $P_c$), since they are large rectangular structures that pack nearly perfectly.

## Estimating Die Areas

### Core Area

The core area $A_{core}$ is:

$$A_{core} = R_f \cdot P_{die} \left( \sum A_{cells} + \sum A_{power} \right)$$

where $\sum A_{cells}$ is the sum of all cell areas, $\sum A_{power}$ is the sum of all major power device areas, $R_f$ is the **routing factor** (area consumed by top-level routing), and $P_{die}$ is the **die-level packing factor** (wasted area between cells plus safety margin).

**Die-level packing factor** $P_{die}$:
- 20-30 moderate-sized non-custom cells: $P_{die} = 1.1$ to $1.2$
- Custom "tight pack" layouts: $P_{die} < 1.05$ (very time-consuming, difficult to modify, only justified for very small designs where cost overrides everything)

### Routing Factors by Metallization

| Metal Stack | Routing Style | $R_f$ |
|---|---|---|
| Single-level metal (SLM) | Through custom components (no channels possible) | $\leq 1.2$ for 50-100 components |
| SLM + poly ("1.5 layer") | Channel routing with poly jumpers | 1.2 -- 1.4 |
| Double-level metal (DLM) | Channel routing (H and V on separate layers) | 1.1 -- 1.3 |
| DLM with maze routing | Maze through cells using min metal-2 | $\approx 1.0$ to $1.1$ |
| Triple-level metal (TLM) | Channel routing | 1.1 -- 1.2 |
| TLM with maze routing | Cells use little metal-3, metal-2 consistently oriented | $\approx 1.0$ |
| Quad-level metal (QLM) | Maze routing (cells use little metal-3/4) | $\approx 1.0$ |

Key routing considerations:
- **Channel routing** places wires in designated regions between cells -- consumes extra area but greatly simplifies interconnection
- **Maze routing** forces wiring over/through existing circuitry -- saves area but requires much more time
- Parasitic capacitive coupling as small as **1 fF** can cause circuit malfunctions -- parasitic extraction and back-annotation are essential
- QLM enables total signal shielding using 3 routing layers (signal plus shields on both sides, and shields above and below)

### Total Die Area (Square Aspect Ratio)

$$A_{die} = \left( \sqrt{A_{core}} + 2W_{pad} + W_{scribe} \right)^2$$

where $W_{pad}$ is the padring width and $W_{scribe}$ is the scribe street width. A typical padring is $\sim 250 \; \mu m$ wide (including ground ring, scribe seal, bondpads, and guard rings). Scribe widths are typically $60$-$80 \; \mu m$ for sawing; laser singulation permits as little as $20 \; \mu m$.

### Core-Limited vs. Pad-Limited

A design is **core-limited** when the core packs tightly into the padring but there are not enough pads to fill the padring perimeter. Gaps are used for ESD structures and non-critical circuitry.

A design is **pad-limited** when there are so many pads that the padring perimeter forces a die larger than the core requires. The minimum die perimeter $P_{min}$ for $N$ bondpads:

$$P_{min} = N \cdot (W_{bp} + S) + 2 \cdot (W_{bp} + W_{scribe/seal})$$

where $W_{bp}$ is bondpad width, $S$ is minimum pad-to-pad spacing, and $W_{scribe/seal}$ is the width of scribeline, scribe seal, and ground ring combined. If bondpads must be kept away from die corners, add 8 times that corner distance.

The **perimeter utilization factor** $U$:

$$U = \frac{P_{min}}{4 \cdot \sqrt{A_{die}}}$$

- $U < 1$: core-limited (padring has spare room)
- $U > 1$: pad-limited (die must grow to fit all pads)

For a pad-limited die with square aspect ratio:

$$A_{die,pad} = \left( \frac{P_{min}}{4} \right)^2$$

The wasted space equals $A_{die,pad} - A_{die,core}$. Elongating the die slightly increases perimeter but rarely transforms a pad-limited design into a core-limited one. A better solution is to place additional bondpads in an inner ring, offset so bondwires do not intersect.

## Cost Estimation

### Dice Per Wafer

$$N_{die} = \frac{\pi D_w^2}{4 \cdot A_{die}} \cdot U_w$$

where $D_w$ is the wafer diameter, $A_{die}$ is the die area including scribe streets, and $U_w$ is the **wafer utilization factor** (fraction of wafer surface covered by usable dice, typically 0.85 to 0.95 depending on die size, photolithography, and wafer flats/notches).

Example: a $10{,}000 \; \mu m^2$ die on a 200 mm wafer with $U_w = 0.95$ yields about 2980 potentially usable dice.

### Cost of a Functional Die

$$C_{die} = \frac{C_{wafer} + C_{probe}}{N_{die} \cdot Y_p}$$

where $C_{wafer}$ is the wafer fabrication cost (including backgrind and singulation), $C_{probe}$ is the wafer probing cost, and $Y_p$ is the **probe yield** (fraction of usable dice that pass wafer probe, typically 0.8 to 0.95 for analog ICs). If wafer probing is omitted, $C_{probe} = 0$ and $Y_p = 1$.

### Total IC Cost

$$C_{total} = \frac{C_{die} + C_{assy}}{Y_a}$$

where $C_{assy}$ is the assembly cost (packaging, symbolization, final test, storage, shipping) and $Y_a$ is the **assembly yield** (usually $> 0.95$ since most defective dice were already rejected at probe). If probe was skipped, $Y_a$ drops by a few percent.

### Gross Profit Margin

$$GPM = \frac{P_{sell} - C_{total}}{P_{sell}} \times 100\%$$

where $P_{sell}$ is the sales price. GPM must cover sales/distribution, engineering, R&D, fixed overhead, and administration. Most companies require at least **50% GPM** for a design to be considered feasible.

Example: If a die costs 44 cents to produce and sells for $1.00, then $GPM = 56\%$.

## Diagrams

### Figure 15.1 -- Database Hierarchy Tree Diagram and Section 15.1.1

![[diagrams/ch15-die-area-estimation-fig1.png]]
*Tree diagram showing the hierarchical structure of a simple layout database. The top cell `protector` contains instances of `esd_diodes` and `diode_nsd_pepi`. The cell `esd_diodes` in turn contains instances of `diode_psd_nwell` and `diode_nsd_pepi`. This illustrates how parent-child relationships form the hierarchy, with leaf cells at the bottom containing no further instances.*

### Figure 15.3 -- Core-Limited vs. Pad-Limited Dice and Table 15.1

![[diagrams/ch15-die-area-estimation-fig2.png]]
*Comparison of core-limited (A) and pad-limited (B) dice. In a core-limited design, the core packs tightly but the padring has gaps between bondpads. In a pad-limited design, the padring is packed full but wasted space exists inside the core. The routing factors and die area equations sections begin below the figure.*

### Table 15.1 -- Bandgap Reference Area Estimate

![[diagrams/ch15-die-area-estimation-fig3.png]]
*Worked example: estimated area for a standard-bipolar bandgap reference circuit, showing component-by-component area breakdown. Base resistors, HSR resistors, junction capacitance, NPN/PNP transistors are individually sized and summed. Total component area is $95{,}800 \; \mu m^2$, and with a conservative packing factor of 2, the estimated cell area is $0.19 \; mm^2$.*

## Practical Takeaways

- **Invest planning time** in partitioning and populating the database before layout begins -- good partitioning directly reduces design time and errors
- **Use conservative packing factors** early in the design: it is far better to overestimate area than to underestimate it
- **Keep cells to 50-100 components** for readability and manageability; break larger cells into subcells
- **Matched components must share a cell** to enable proper array layout and future reuse
- **Power devices should be estimated separately** from cell areas -- they are large rectangular structures that pack near-perfectly and should not be inflated by the cell-level packing factor
- **Worst-case $R_{on}$** for MOS power transistors can be nearly **2x nominal** when accounting for process variation ($\pm 20\%$) and temperature ($+50\%$ from $25°C$ to $125°C$)
- **Parasitic capacitances as small as 1 fF** can cause circuit malfunctions in analog designs -- always use parasitic extraction
- A design target of at least **50% GPM** is the standard feasibility threshold
- **Channel routing** is faster and easier than maze routing; use maze routing only when area is critical
- Check early whether your design is **core-limited or pad-limited** -- this fundamentally affects die sizing and pad placement strategy
- When pad-limited, consider a **dual-rank pad arrangement** (inner ring offset from outer ring) rather than enlarging the die

## Relation to the Bigger Picture

Die area estimation is the quantitative foundation for everything that follows in Chapter 15. The cell areas, packing factors, and routing estimates computed here feed directly into [[ch15-floorplanning]], where the physical arrangement of blocks, bondpads, and routing channels is determined. Getting the area estimate right (or at least conservatively right) prevents costly surprises during layout -- an underestimated die that does not fit its chosen package may require a complete restart. The cost estimation formulas also close the loop between layout engineering and business feasibility, ensuring that design decisions are economically sound before significant layout effort is committed.

## See Also
- [[ch15-floorplanning]]
