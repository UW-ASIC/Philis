---
title: "15.2 Floorplanning"
chapter: 15
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-15, floorplanning, die-assembly, padring-placement]
---

# 15.2 Floorplanning

> **Chapter 15: Assembling the Die**

## Key Concepts

Floorplanning is the final phase of the die planning process. It produces a **sketch of the layout** showing the locations of bondpads and the shapes and placements of major subcells. The floorplan is not a finished layout; it is a *guide* that will be revised as individual blocks are laid out and turn out to require more or less area than originally estimated.

The fundamental purpose of a floorplan is to answer several interrelated questions simultaneously:

1. **Where does each circuit block go?** Blocks must be positioned so their pins face the bondpads they connect to, and so that routing channels between blocks are wide enough to carry all signals without creating choke points.
2. **Where do the bondpads go?** Pad placement is constrained by the leadframe finger positions, the requirement that bondwires must not cross or come too close to one another, and the need to minimize bondwire length.
3. **Where do the power and ground leads run?** High-current leads must be explicitly planned because electromigration and resistive drop impose minimum-width and maximum-length constraints that can dominate the topology of the die.
4. **Where do the scribe streets, scribe seal, and ground ring reside?** These consume peripheral area and must be accounted for before cell placement begins.

The floorplan is created from information gathered during die area estimation (see [[ch15-die-area-estimation]]): cell areas, die area, pad list with pin ordering, package type, bondwire diameter, and padring/scribe dimensions.

## The Floorplanning Worksheet

Before sketching anything, the designer compiles a **floorplanning worksheet** (Table 15.2 in the text). This worksheet collects all of the information needed in one place:

- **Device name and process** (e.g., "Dual operational amplifier, standard bipolar with double-level metal")
- **Package type** (e.g., 8-pin SOIC)
- **Die area estimate** from the computations in Section 15.1
- **Bondwire type and diameter** (e.g., palladium-coated copper)
- **Bondpad width, padring width (without scribe), and scribeline width**
- **Circuit blocks with their areas, dedicated pins, and shared pins** -- the distinction between dedicated and shared pins is critical because shared pins (like $V_{DD}$ and $V_{SS}$) must route to multiple blocks, which constrains their placement
- **Pin list** in package order, with pin number, name, and description

The example in the text is a dual operational amplifier with three circuit blocks: `amp1`, `amp2` (both instances of the same cell), and `bias`. The amp blocks each have three dedicated pins (two inputs and one output) plus shared supply pins.

## Step-by-Step Floorplanning Process

### Step 1: Determine Die Dimensions

Starting from the die area estimate (from [[ch15-die-area-estimation]]), compute the die side length assuming a square aspect ratio. For example, a $1.33\,\text{mm}^2$ die has dimensions of approximately $1.153\,\text{mm} \times 1.153\,\text{mm}$. Round this to the nearest increment allowed by the photolithographic equipment (typically 1 um or finer).

### Step 2: Select a Leadframe

Once die dimensions are known, choose a leadframe whose **mount pad** is large enough to accommodate the die with alignment margin. The text uses a typical 8-pin SOIC leadframe as an example. Key considerations:

- The die must be **smaller than the mount pad** to allow for misalignment and die-attach extrusion. A typical allowance is on the order of hundreds of microns per side.
- Choose the **smallest leadframe** that fits the die with a 10-20% margin in X and Y for unexpected die growth.
- **Excessively large leadframes** cause delamination due to CTE mismatch between the copper leadframe and plastic encapsulation. Mitigation options include reducing the exposed mount pad area or using a roughened leadframe.

### Step 3: Mark the Scribe Streets

The floorplan must show scribe street locations. Different processes place them differently:
- Bottom and left sides of the die
- Top and right sides of the die
- Half-width scribes on all four sides

All arrangements produce the same wafer-level scribe pattern, but each requires a slightly different layout origin convention. The text's example places scribes on the top and right, with the layout origin at the lower-left corner of the top cell.

### Step 4: Sketch the Core and Padring Boundaries

Draw two nested rectangles:
- **Outer rectangle**: the die extents
- **Inner rectangle**: the core boundary (die minus scribe streets and padring)

Strips along the die edges show where scribe streets, scribe seal, ground ring, bondpads, and ESD structures reside.

### Step 5: Place Circuit Blocks

This is the heart of floorplanning. The designer places rectangles representing each circuit block inside the core boundary, observing the following principles:

- **Mirror-image placements** for identical blocks. In the dual op-amp example, `amp1` and `amp2` are mirror images of the same cell. This saves layout effort and ensures matched electrical characteristics.
- **Pin proximity**: each block should be placed near the bondpads it connects to. `amp1` (connecting to pins 1-3) goes on the left; `amp2` (connecting to pins 5-7) goes on the right.
- **Shared blocks** (like `bias`) go in the middle where they can connect to both sides.
- **Cell shape flexibility**: elongated cells are acceptable and can even be beneficial. In the example, elongated amplifier cells naturally separate the sensitive input circuitry from the heat-generating output stages, improving matching.
- **Routing channels**: reserve explicit space for top-level wiring. The die area estimate typically allocates ~20% of core area for routing. In the floorplan, this space appears as strips (e.g., two vertical routing channels) running across the die.

### Step 6: Place Bondpads

Superimpose the floorplan on a copy of the leadframe drawing. Initial placement puts each bondpad adjacent to its leadframe finger for shortest bondwires and maximum wire separation. Then adjust for better interconnection to circuit blocks, subject to the constraint that **bondwires must never intersect or closely approach one another**.

Three rules for bondpad placement:
1. **Wires must not cross.** Crossed wires will short or make bonding impossible.
2. **Wires must not closely approach adjacent ball bonds.** A wire passing too close to a neighboring ball bond risks mechanical damage during bonding.
3. **Assembly site verification is required.** Most assembly sites have verification programs that check whether a proposed arrangement is manufacturable.

In the dual op-amp example, I/O pads for `amp2` are placed directly opposite those of `amp1` for symmetric routing. Power pads ($V_{DD}$, $V_{SS}$) move to the top and bottom of the die to minimize bondwire interference and provide straightforward power distribution to all three blocks.

### Step 7: Plan the Ground Ring and Substrate Connection

Most dice include a ring of metal around the die edge connecting to substrate potential. ESD devices return to this **substrate ground ring**. Key points:

- Most ICs use P-type substrates connected to the lowest on-die potential.
- For single-supply designs, this is ground. For dual-supply designs (like op-amps with $V_{DD}$ and $V_{SS}$), the substrate connects to $V_{SS}$ (the negative supply).

### Step 8: Plan High-Current Leads

If the design has high-current circuitry, the designer must explicitly plan the routing of all high-current leads. For each such lead, mark on the floorplan:
- Its anticipated path
- The equivalent DC current it must carry
- Its maximum allowed resistance (if applicable)

**Electromigration** sets a minimum width for high-current leads, but **metal resistance** often demands even wider leads. Keep high-current leads as short as possible. The resulting diagram reveals awkward or unnecessarily long leads.

In the dual op-amp example, the $V_{CC}$ lead routes horizontally across the top of the bias cell in metal-2, allowing signals to cross underneath in metal-1. The $V_{SS}$ leads merge with the scribe seal metallization -- a common design pattern that leverages the wide ground ring metal already present around the die perimeter.

## Scaling to Complex Designs

The dual op-amp example is deliberately simple, but the same principles apply to large, complex designs. As complexity grows:

- **Block arrangement becomes a jigsaw puzzle.** The designer must fit irregularly shaped blocks together efficiently.
- **Routing channel width becomes critical.** A carelessly placed block can constrict a routing channel, creating a **choke point** that needlessly complicates routing.
- **Choke points are expensive to fix late.** If a choke point is not discovered until routing has begun, significant rework may be required. This makes the floorplan a critical early-stage investment.

The text shows a complex floorplan (Figure 15.8) with two obvious choke points -- narrow routing channels squeezed between abutting blocks -- as a cautionary example.

## Diagrams

### Figure 15.5 -- Floorplan of the Eight-Pin Dual Op-Amp

![[diagrams/ch15-floorplanning-fig1.png]]

This figure shows the complete floorplan sketch for the dual operational amplifier example. The die is bounded by a rectangle with scribe streets on the top and right. Inside, `amp1` occupies the left side, `amp2` the right side (as a mirror image), and `bias` fits between them. Two vertical routing channels reserve space for top-level wiring. Bondpads are positioned around the perimeter to align with leadframe fingers.

### Figure 15.6 -- Three Possible Bondpad Placements

![[diagrams/ch15-floorplanning-fig2.png]]

This figure illustrates three bondpad placements for the 8-pin leadframe: (A) pads placed directly adjacent to leadframe fingers (shortest wires, largest separation), (B) an acceptable adjusted placement that better serves the circuit layout, and (C) an unacceptable placement where pin 2's wire comes too close to pin 1's ball bond and pin 3's wire too close to pin 2's ball bond. This figure demonstrates that bondwire clearance is a hard constraint that must be verified by the assembly site.

### Figure 15.8 -- Complex Die Floorplan with Choke Points

![[diagrams/ch15-floorplanning-fig3.png]]

This figure shows a more realistic, complex floorplan resembling a jigsaw puzzle of irregularly shaped blocks. Two routing channel choke points are highlighted where block placement has constricted the available routing space. This illustrates why careful floorplanning is essential for large designs -- choke points discovered during routing cause costly rework.

## Practical Takeaways

- **Always create a floorplanning worksheet** before sketching. Gather die area, pad list, package type, bondwire specs, and block areas systematically.
- **Use mirror-image placements** for identical blocks. This halves layout effort and ensures matched electrical characteristics.
- **Place blocks near their dedicated pins.** Minimize the distance between each block and its bondpads to simplify routing and reduce parasitic resistance.
- **Shared supply pins** ($V_{DD}$, $V_{SS}$) should be placed where they can route to all blocks with minimal resistance and no routing conflicts.
- **Reserve explicit routing channels** (typically ~20% of core area). Do not rely on routing "finding a way" through cells.
- **Verify bondpad placement with the assembly site.** Bondwire crossing and proximity violations are hard failures that cannot be fixed without moving pads.
- **Superimpose the floorplan on the leadframe drawing** to confirm bondwire feasibility before committing to a pad arrangement.
- **Mark all high-current leads** on the floorplan with their current requirements. Electromigration and $IR$ drop can dominate lead widths and thus die topology.
- **Merge ground returns with scribe seal metallization** where possible. This is a common and effective technique for minimizing ground resistance.
- **Watch for choke points** in routing channels, especially at block corners and intersections. These are much cheaper to fix during floorplanning than during routing.
- **Choose the smallest adequate leadframe** with 10-20% margin. Oversized leadframes cause delamination from CTE mismatch with plastic encapsulation.
- **Elongated cells are acceptable** and can even be beneficial (e.g., separating heat sources from sensitive inputs improves thermal matching).
- **The floorplan is a living document.** Revise it whenever a block requires significantly more or less space than originally estimated.

## Relation to the Bigger Picture

Floorplanning bridges the gap between the numerical die area estimation of Section 15.1 (see [[ch15-die-area-estimation]]) and the physical construction of the padring and core layout in Sections 15.3-15.5 (see [[ch15-padring]]). It is the step where abstract area numbers become a spatial arrangement that must simultaneously satisfy electrical, mechanical, and manufacturing constraints. A poor floorplan cascades into routing difficulties, thermal problems, matching degradation, and potentially a die that does not fit in its package. For analog ICs in particular, where matching, noise coupling, and thermal gradients are first-order concerns, the floorplan is arguably the most consequential single drawing in the entire design process.

## See Also
- [[ch15-die-area-estimation]]
- [[ch15-padring]]
