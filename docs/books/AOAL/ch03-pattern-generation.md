---
title: "3.3 Pattern Generation"
chapter: 3
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-3, pattern-generation, photolithography, MEBES, OPC, reticle, photomask]
---

# 3.3 Pattern Generation

> **Chapter 3: Layout**

## Key Concepts

Pattern generation (PG) is the process of converting a completed layout database into the data files used to manufacture photomasks (reticles). Although the term originated with early optical pattern generators that are now obsolete, the name persists: layout designers still call the preparation of mask data "pattern generation" regardless of the underlying technology.

The PG process bridges the gap between the abstract world of layout design and the physical world of photolithography. It involves several critical transformations:

1. **Layer combination** -- Merging multiple coding layers into the generated layers that correspond to actual photomasks. For example, in a CMOS process with layers MOAT, PMOAT, and NMOAT, the PG deck produces a unified moat mask via: `ALL_MOAT = MOAT + NMOAT + PMOAT`.
2. **Process size adjusts** -- Deliberately oversizing or undersizing geometries to compensate for known etch biases and other fabrication effects (e.g., `FINAL_MOAT = ALL_MOAT oversized by 0.05`).
3. **Tone selection (dark vs. clear masks)** -- Specifying whether the mask is dark-field or clear-field based on the resist type and what the drawn data represents.
4. **Geometric decomposition** -- Breaking arbitrary polygons into primitives the mask-writing equipment can handle (rectangles for optical PG, trapezoids for MEBES/laser writers).
5. **OPC insertion** -- Adding optical proximity correction features for advanced processes.

The PG deck (the script that drives this process) was traditionally the last step executed by the layout team before handing data to the mask shop. It is effectively a recipe that encodes all of the above transformations.

### Historical Evolution

The technology has passed through three distinct eras:

| Era | Technology | Data Format | Decomposition |
|---|---|---|---|
| 1960s--1970s | Rubylith masters, 400:1 photoreduction | Hand-cut artwork | None (manual) |
| 1970s--1980s | Optical pattern generators (e.g., D.H. Mann/GCA) | Mann format (rectangles) | Rectangular decomposition |
| 1980s--present | Electron beam (MEBES) and excimer laser scanners | MEBES format (trapezoids) | Trapezoidal decomposition |

## Important Details

### 3.3.1 Optical Pattern Generation

The D.H. Mann series of pattern generators (manufactured by GCA Corporation) were the workhorses of optical PG. Their operating principle was conceptually simple:

- A glass blank coated with photosensitive emulsion was mounted on a flat plate.
- Two pairs of blades in a rotating turret formed a rectangular aperture whose size and orientation were electromechanically controlled.
- A data file drove the controller, which sequentially positioned the turret, adjusted the aperture, and fired a flash of light through the aperture and a lens system to expose a small rectangle on the reticle.
- This was repeated thousands of times to build up the complete layout image.

The data had to be provided in **Mann format**: a series of rectangles in IBM EBCDIC text, with one file per reticle. Since real layouts contain polygons and paths (not just rectangles), translators were needed to convert GDSII-like data into Mann format. These translators were typically built into DRC programs.

**Rectangular decomposition** was the core algorithm: any polygon was broken into a set of axis-aligned or rotated rectangles. This had one fatal limitation -- no combination of rectangles can perfectly fill an **acute interior angle**. This is why layout design rules universally prohibit exposed acute interior angles, even in decorative chip art and labels. The fab may put a lot on hold for a presumed rule violation if such angles appear.

#### Coding Layers, Generated Layers, and Pseudolayers

The PG deck works with three categories of layers:

- **Coding layers** -- Layers drawn directly by the layout designer (e.g., MOAT, PMOAT, NMOAT, POLY, METAL1).
- **Generated layers** -- Layers created by the PG deck through Boolean operations on coding layers (e.g., ALL_MOAT = MOAT + NMOAT + PMOAT). These correspond to actual photomasks.
- **Pseudolayers** -- Coded by the designer but do not affect mask geometries. They carry information to the DRC program for rule checking purposes (see Section 9.2.4 in the text).

#### Dark Masks vs. Clear Masks

The choice between dark-field and clear-field masks depends on two factors: (1) what the drawn layout data represents, and (2) the type of photoresist used.

- **Positive resist**: Light causes resist to dissolve in developer. Transparent mask regions create openings in resist.
- **Negative resist**: Light causes resist to polymerize (become insoluble). Opaque mask regions create openings in resist.

| Layout data represents... | Positive Resist | Negative Resist |
|---|---|---|
| Openings in photoresist | Dark mask | Clear mask |
| Regions between openings | Clear mask | Dark mask |

A **clear-field mask** (clear mask) consists of opaque shapes on a mostly transparent background. A **dark-field mask** (dark mask) is the opposite: clear openings in a mostly opaque field. The PG deck performs a **color reversal** when the drawn data must be inverted to produce the correct mask tone.

#### Address Units and Scale

The PG deck specifies the reticle scale (e.g., 10X) and defines an **address unit (AU)** -- the smallest increment allowed in the reticle data. For a 10X reticle, the AU on the reticle maps to $\text{AU}/10$ on silicon. The coding unit used by designers must be a multiple of $\text{AU}/\text{scale}$ to ensure all coordinates land on the reticle grid. Process size adjusts must also be multiples of this quantum. Optical pattern generators typically used address units of about $0.25\,\mu\text{m}$.

### 3.3.2 Advances in Photolithography

Photolithographic equipment evolved through several generations, each improving resolution, throughput, and mask longevity:

1. **Contact printers** (1960s--70s) -- The photomask was pressed directly against the wafer. This damaged the mask emulsion rapidly, requiring frequent replacement from stepped working plates.
2. **Proximity printers** -- A small gap separated mask and wafer, greatly extending mask life.
3. **Projection printers** -- Optics projected the mask image onto the wafer without contact.
4. **Steppers** -- A reticle containing one or a few die images was stepped across the wafer, exposing one field at a time. This eliminated the need for stepped working plates.
5. **Scanners** (post-2000) -- Only a narrow slit of the reticle is exposed at once; the reticle and wafer move simultaneously in opposite directions. This simplifies optics but demands extremely tight mechanical tracking. Scanners typically use 4X reduction (vs. 5X for steppers).

**Key milestones:**

- Mask materials progressed from soda-lime glass with emulsion to borosilicate glass to **fused silica (quartz)** with chromium patterning, reducing thermal expansion by an order of magnitude.
- Wafer sizes grew from 2-inch and 3-inch to 200 mm and 300 mm.
- Light sources progressed from **G-line** ($\lambda = 436\,\text{nm}$) to **I-line** ($\lambda = 365\,\text{nm}$) to **excimer lasers** ($\lambda = 248\,\text{nm}$ KrF, $\lambda = 193\,\text{nm}$ ArF).
- Numerical apertures increased from ~0.35 to ~0.6 and beyond.
- Direct step on wafer (DSW) patterning eliminated stepped working plates entirely.

#### Stepped Working Plates and Reticle Composition

Early steppers created **stepped working plates** by repeatedly exposing a reticle image across a larger mask plate. Process control structures ("plugs") were inserted at a few locations by substituting a different reticle during the stepping process. When DSW eliminated working plates, process control structures moved to:

- Dedicated die locations within the reticle array (wastes area).
- The edges of the reticle array, selectively exposed by adjusting stepper blades.
- Within the **scribe streets** between dies (most area-efficient).

**Composed reticles** (also called multi-project reticles) allow multiple different designs on a single reticle. This concept was pioneered by **MOSIS** (Metal Oxide Semiconductor Implementation Service, est. 1981), the first wafer foundry to offer multi-project wafers. This approach is now standard for "fabless" semiconductor companies to reduce development costs.

### 3.3.3 MEBES

**MEBES** (Manufacturing Electron Beam Exposure Systems) was originally developed by Bell Labs as EBES and commercialized by Etec Systems. It replaced optical pattern generation as the dominant reticle-writing technology by the mid-1980s.

**How it works:**

- Operates inside a vacuum chamber using electron optics.
- The electron beam sweeps back and forth across the mask surface in a **raster scan** pattern (analogous to a CRT television display), rather than exposing discrete rectangular apertures.
- Electrons experience quantum diffraction, but at properly selected energies, diffraction effects are negligible, enabling extremely fine feature resolution.
- Because only one point is illuminated at a time, the nonlinear optical effects (such as proximity effects in multi-aperture optical systems) are eliminated.

**Data format:** Despite using raster imaging internally, the MEBES file format requires input data as **trapezoids** (not arbitrary polygons). Each trapezoid has horizontal top and bottom edges, with sides at arbitrary angles. Etec chose this constraint to reduce the computation required for rasterization inside the machine.

This led to a new class of **trapezoidal decomposition** algorithms for PG, which have a key advantage over rectangular decomposition: trapezoids *can* fill acute interior angles, so this limitation of the older rectangular approach disappears. MEBES files also do not require figure sorting to optimize write times, as the machine transforms all trapezoidal data into raster data before writing begins.

**The PG process for MEBES** follows the same logical steps as optical PG:
1. Combine coding layers into generated layers
2. Apply process size adjusts
3. Apply shrinks and color reversal
4. Perform trapezoidal decomposition
5. Output in MEBES format

The only difference from an optical PG deck is the choice of decomposition algorithm and output format.

**Modern successors:** Excimer laser scanners have largely replaced MEBES electron-beam writers, but they still use raster imaging and expect trapezoidal input data. Many use the **MEBES-5** file format. Mask shops commonly operate a mix of older MEBES tools and newer laser scanners -- the layout designer often does not know which equipment will actually process the data.

### 3.3.4 Optical Proximity Correction (OPC)

When light passes through a narrow opening on a reticle, **diffraction** causes the projected image to deviate from the ideal geometric shape. Corners round off, narrow lines thicken or thin, and closely spaced features interact. **Optical proximity correction (OPC)** deliberately distorts the mask geometry to pre-compensate for these diffraction effects, so the image printed on the wafer more closely matches the designer's intent.

#### Types of OPC

1. **Primitive OPC ("dog ears")** -- Small square protrusions added at exterior corners of rectangles to compensate for corner rounding. This technique was known informally to layout designers long before OPC was formalized.

2. **Simple rule-based OPC** -- Square additions at every exterior corner and corresponding subtractions at every interior corner of each geometry.

3. **Aggressive rule-based OPC** -- Corner corrections that vary depending on spacings to adjacent geometries. May also include spacing variations that depend on distances to corners and spacings to adjacent features. Rules take the form of a matrix of values derived from modeling and applied via special geometric operations during PG.

4. **Model-based OPC** -- Process designers simulate the behavior of light passing through the reticle and iteratively optimize the OPC geometry for best fidelity. Produces the best theoretical results but requires immense computational power and generates enormous data volumes.

In practice, process designers typically use **modeling to derive a single set of rules**, then apply those rules universally -- a hybrid approach that balances quality and computational cost.

#### OPC in Analog vs. Digital

Analog IC layouts generally use **less OPC**, and what they do use is **less aggressive** than their digital counterparts. Current practice for analog:

- **Gate mask (POLY over active)**: at least mild OPC for submicron gate lengths.
- **Moat/active mask**: may use OPC.
- **Damascene metal layers**: may use OPC.
- **Other layers**: typically have large enough minimum feature sizes that OPC is unnecessary.

## Diagrams

### Figure 3.20 -- Photomask and Photoresist Relationship

![[diagrams/ch03-pattern-generation-fig1.png]]

**Caption:** Shows the relationship between the photomask image (A) and the resulting photoresist patterns for positive resist (B) and negative resist (C). With positive resist, light causes dissolution, so transparent regions create openings. With negative resist, light causes polymerization, so opaque regions create openings. This determines whether a clear-field or dark-field mask is needed.

### Figure 3.21 -- Stepped Wafer Pattern

![[diagrams/ch03-pattern-generation-fig2.png]]

**Caption:** A typical stepped working plate showing the main die array and "plugs" -- locations where process control structures replace die images. Also shows the progression from early aligners to modern DSW steppers, and the evolution of mask materials from soda-lime glass emulsion masks to chromium-on-quartz hard-surface masks.

### Figure 3.22 -- Optical Proximity Correction Examples

![[diagrams/ch03-pattern-generation-fig3.png]]

**Caption:** Drawn geometries (A) compared with their resulting mask images after OPC figures are added (B). Note the "dog ear" additions at exterior corners and notches at interior corners that pre-compensate for diffraction-induced corner rounding during photolithography.

## Practical Takeaways

- **Never create exposed acute interior angles** in layout geometries -- not even in labels or decorative chip art. Rectangular decomposition cannot fill them, and the fab may flag these as rule violations and hold the lot.
- **Understand the PG deck flow**: coding layers are combined into generated layers, process size adjusts are applied, mask tone (dark/clear) is selected, and decomposition runs. Errors in any of these steps produce bad masks.
- **Process size adjusts are not the designer's responsibility** to apply manually -- they are handled by the PG deck. But designers must be aware that drawn dimensions differ from final silicon dimensions by these adjusts.
- **Address units constrain your grid**: the coding unit in the layout database must be a multiple of the reticle address unit divided by the reticle scale factor. Coordinates that do not land on the AU grid will be rounded, potentially causing errors.
- **Analog designs use less OPC than digital**, but submicron analog processes still need at least mild OPC on gate and moat masks. Designers should consult the process design kit (PDK) documentation to know which layers receive OPC.
- **The MEBES file format persists** even though MEBES machines are being replaced by excimer laser scanners. Many modern mask writers still accept MEBES-5 format input, so trapezoidal decomposition remains the standard PG algorithm.
- **Multi-project wafers** (via MOSIS or similar foundry services) allow cost-effective prototyping by composing multiple designs onto a single reticle. This is especially valuable for academic and low-volume analog designs.
- **Mask materials matter**: fused silica (quartz) with chromium patterning provides far better dimensional stability than soda-lime glass with emulsion. Modern masks using these materials can last the entire production life of a part.

## Relation to the Bigger Picture

Pattern generation is the critical interface between the layout designer's work and the physical fabrication process. Understanding PG is essential for appreciating why [[ch03-design-rules]] exist the way they do -- many design rules (minimum widths, spacings, prohibited angles) originate directly from the limitations of PG decomposition algorithms, photolithographic resolution, and mask-writing technology. The concepts of coding layers, generated layers, process size adjusts, and mask tone introduced here recur throughout the book whenever specific layout techniques are discussed. Furthermore, OPC awareness is increasingly important even for analog designers as process nodes shrink below $0.25\,\mu\text{m}$, since OPC figures can subtly alter the effective geometry of critical transistors and resistors.

## See Also
- [[ch03-design-rules]]
