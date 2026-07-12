---
title: "3.2 Design Rules"
chapter: 3
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-3, design-rules, DRC, geometric-operations, scalable-rules]
---

# 3.2 Design Rules

> **Chapter 3: Layout**

## Key Concepts

Design rules are the contract between the layout designer and the fabrication process. They encode the minimum dimensions and spacings that the photolithography, etching, diffusion, and depletion physics of the process can reliably achieve. Violating a design rule means the fabricated circuit may not function correctly -- features may short, open, or fail to form properly.

The layout must obey these rules so that:
1. Shapes on each photomask are **sufficiently large and far apart** for photolithography to resolve them.
2. Adequate space exists between diffusions for **outdiffusion and depletion** regions.
3. Overlaps and overhang margins accommodate **mask alignment errors** and **linewidth control** variations.

Originally, layout designers manually checked layouts with rulers and templates ("beer checks"). By the late 1970s, automated **Design Rule Checking (DRC)** programs emerged. Modern DRC tools include Cadence's Diva/Assura/Dracula, Mentor's Calibre, and Synopsys' Hercules. These programs:
1. Load the layout database into memory
2. Generate additional layers using **geometric operations** specified in the rule deck
3. Check various layers using rules specified in the rule deck
4. Report violations as **diagnostic** shapes superimposed on the layout

DRC can operate in two modes:
- **Flat verification**: Transforms the entire hierarchy into polygons, merges touching/overlapping polygons on the same layer, then runs checks. Simple but memory-intensive.
- **Hierarchical verification**: Retains the hierarchical structure of the layout. Much faster for most layouts, though may not help for layouts with many interpenetrating instances or heavy top-level routing.

---

## 3.2.1 Geometric Operations

Geometric operations transform data from one or more layers into data placed on a new (generated) layer. These are the building blocks of both DRC decks and PG (pattern generation) decks.

### Geometric OR (Union)

Accepts two input layers, produces one output layer. Any point in the interior of a shape on **either** input layer appears inside a shape on the output layer. Symbolized by `+`.

```
ALLMOAT = NMOAT + PMOAT
```

- Symmetric: $A + B = B + A$
- Also known as **union** or **combination**

### Geometric AND (Intersection)

Accepts two input layers, produces one output layer. Only points in the interior of shapes on **both** input layers appear on the output layer. Symbolized by `*`.

```
GATE = POLY * MOAT
```

- Symmetric: $A * B = B * A$
- Transitive over OR: $A * (B + C) = (A * B) + (A * C)$
- Also known as **intersection** or **common area**

### Geometric NOT (Color Reversal)

Accepts one input layer, produces one output layer. Every interior point becomes exterior, and vice versa. Symbolized by `\`.

```
FIELD = MOAT\
```

- The result has a **dark field** (interior extending indefinitely) which must be bounded for export
- Used to reverse clear/opaque regions of a photomask

### Geometric ANDNOT (Minus / Subtraction)

Combination of AND and NOT. Symbolized by `-`. The operation $A - B$ is equivalent to $A * B\backslash$, but executes faster and cannot generate a dark field.

```
FIELDPOLY = POLY - MOAT
```

- **Non-symmetric**: $A - B \neq B - A$
- Also called the **minus** or **subtraction** operation

### Geometric XOR (Exclusive OR)

Any point in the interior of a region on one input layer **but not the other** appears on the output layer.

```
C = A xor B
```

- Symmetric: $A \oplus B = B \oplus A$

### Size Adjust (Oversize / Undersize)

Copies data from an input layer to an output layer, splits it into distinct (non-touching, non-overlapping) polygons, then moves each boundary segment perpendicularly by the specified distance:

```
METAL_OS = METAL oversized by 1.0
METAL_US = METAL undersized by 1.0
```

- **Oversize**: moves boundaries outward (enlarges). A $2\,\mu m$ square oversized by $1\,\mu m$ becomes a $4\,\mu m$ square.
- **Undersize**: moves boundaries inward (shrinks). A $2\,\mu m$ square undersized by $1\,\mu m$ vanishes entirely.

**Critical caveat**: Oversize and undersize are **not reversible**. Applying an oversize that closes a notch, then undersizing by the same amount, does **not** restore the original geometry -- the notch stays filled. Similarly, an undersize that removes a narrow arm followed by the same oversize leaves the arm missing. This non-reversibility is a common source of unexpected results in DRC/PG decks.

---

## 3.2.2 Rule Checks

Before any rule checks run, touching or overlapping geometries on the same layer must be **merged** into single polygons. This is true for both flat and hierarchical verification.

### Width Check

Verifies that all dimensions of every geometry on a given layer equal or exceed a minimum value. Widths are measured:
- Perpendicular to every line segment of the boundary, across the interior
- Between vertices across the interior

```
CONT width       1.0 um
```

The keyword `exact` can be appended to require that only a square of the indicated size passes:

```
CONT width       1.0 um exact
```

**Pitfalls**:
- Polygonal approximations of circles/arcs can produce **false violations** at vertices that are closer together than the specified width but enclosed by obtuse-angle line segments.
- Non-orthogonal paths: pattern generation rounds coordinates to nearest database units, potentially reducing diagonal path widths by up to 1.4 database units. Most DRC programs compensate by reducing the enforced width on diagonal segments by 2 database units.

### Spacing Check

Verifies that all geometries on one layer maintain a minimum separation from geometries on another (or the same) layer. Measured perpendicularly from line segments and directly between vertices.

```
NMOAT spacing to PMOAT       2.0 um
NMOAT spacing to NWELL        9.5 um, overlap okay
NMOAT spacing to NMOAT        5.5 um
```

- `overlap okay` suppresses violations for geometries that touch or overlap.
- When applied to a single layer, both **intrafigure** spacing (across a gap in one polygon) and **interfigure** spacing (between separate polygons) are checked.
- Same pitfalls with non-orthogonal paths as width checks.

### Overlap Check

Applies to geometries on one layer that wholly or partially enclose geometries on a second layer. Measurements are made perpendicularly outward from each boundary segment and vertex of the enclosed geometry.

```
METAL overlap CONT       1.0 um
```

- A CONT geometry that does **not touch** METAL at all generates **no** diagnostics (it is not an overlap issue -- it is a connectivity issue).
- To find "naked" contacts not touching metal: `CONT must touch METAL`
- `exact` can be appended to flag any overlap that does not exactly equal the specified value.
- `overhang okay` suppresses diagnostics when the second layer extends beyond the first.

### Overhang Check

Applies only when a geometry on one layer **partially overlaps** a geometry on a second layer. The intersection polygon is created, and then measurements are made from its boundary to the boundary of the figure on the first layer.

```
POLY overhang NMOAT       1.0 um
```

Edges of the first-layer figure that lie entirely **inside** the second-layer figure do not produce diagnostics.

---

## 3.2.3 Design Rule Construction

This section explains **how** design rules are derived from process physics. The key factors are:

### Minimum Feature Size (Critical Dimension)

The smallest dimension patternable by photolithography at a given sophistication level. The Abbe criterion gives:

$$CD_{min} \approx \frac{\lambda}{2 \cdot NA}$$

where $\lambda$ is the wavelength and $NA$ is the numerical aperture. For an I-line stepper ($\lambda = 365\,nm$, $NA \approx 0.6$), this yields $CD_{min} \approx 0.3\,\mu m$. The minimum feature size limits both widths and spacings on a given layer.

| Light Source | Wavelength | Typical $CD_{min}$ |
|---|---|---|
| G-line | 436 nm | older processes |
| I-line | 365 nm | ~0.3 um |
| KrF excimer | 248 nm | sub-0.25 um |
| ArF excimer | 193 nm | below 50 nm |
| EUV | 13.5 nm | leading edge digital |

Analog processes typically trail digital processes by several generations; I-line steppers remain common for analog.

Wafer fabs monitor linewidths with **critical dimension figures** -- special test structures placed on the die (see Figure 3.19 in the text).

### Process Bias

The systematic difference between the drawn dimension and the fabricated dimension. Dry etching typically produces a bias no greater than 10% of the etched layer thickness. Two approaches to compensate:

1. **Factor into design rules directly** -- simple but inflexible across equipment variations.
2. **Process size adjusts during pattern generation** -- preferred. All geometries on a layer are oversized/undersized by the bias amount. If equipment changes, only the PG deck needs updating.

Process biases are **geometry-dependent**: larger openings etch faster and exhibit larger biases. A single size adjust corrects minimum-width features; additional rules handle larger features.

### Linewidth Control

The worst-case combination of all random variation sources (mask feature widths, photoresist thickness, focus variations). Generally considered satisfactory at **10% of the minimum feature size**.

Important for computing overlap, spacing, and overhang rules. Note that **adjacent identical devices match far better** than linewidth control suggests, because the same systematic factors affect them equally -- a key insight for analog matching.

### Mask Alignment

Each successive photomask must be aligned (registered) to features created by previous masks. Sources of error include:
- **Run-out**: thermal expansion mismatch between mask and wafer (reduced by fused silica masks and photoreduction)
- Lens distortions, wafer nonplanarity, mechanical repeatability limits

Worst-case mask alignment error is typically **20% of the minimum feature size**.

**Combining alignment errors**: When a rule involves layers from two different masks, both aligned to a common reference mask, their alignment errors combine. They do **not** add linearly -- as a rough rule, two mask alignment errors combine to about **160% of a single error** (because some components are random and some systematic).

Process designers carefully choose alignment references to minimize the impact: most important rules involve only **one** mask misalignment; less critical rules may suffer **two**.

### Outdiffusion

Diffusions extend laterally beyond the oxide opening by approximately **80% of the vertical junction depth** (lateral straggle for implants is similar). A geometry with drawn width $W_d$ and junction depth $x_j$ has a fabricated width:

$$W_{fab} = W_d + 2 \times 0.8 \times x_j$$

Spacing rules for diffusion layers must include margin for outdiffusion on both sides.

**Dilution effect**: For diffusion widths less than about $2 \times x_j$, the doping concentration is significantly reduced. This can be exploited (dilution patterns) or avoided (by setting minimum width $\geq x_j$).

### Depletion Region Width

Depletion regions around PN junctions consume space that must be accounted for in spacing rules. The width depends on dopant concentrations and applied voltage. Approximate values from Lawrence-Warner curves:

| Background ($cm^{-3}$) | $x_j$ ($\mu m$) | 5 V | 10 V | 20 V | 40 V |
|---|---|---|---|---|---|
| $10^{15}$ | 1 | 3 | 4 | 5 | 8 |
| $10^{15}$ | 5 | 5 | 6 | 7 | 10 |
| $10^{16}$ | 1 | 1.3 | 1.5 | 2 | --- |
| $10^{16}$ | 5 | 2 | 2.5 | 3 | 4 |
| $10^{17}$ | 1 | 0.5 | 0.6 | --- | --- |
| $10^{17}$ | 5 | 1 | 1.2 | --- | --- |

(Values in $\mu m$; "---" means avalanche breakdown precludes operation. Add a **50% safety margin** to account for doping and temperature variations.)

### Worked Example: Hypothetical 4-Mask Process

A simple process with masks: NDIF (N-type diffusion), CONT (contact), METAL (metallization), POR (protective overcoat removal). Assumptions: stepper with $0.8\,\mu m$ linewidth control, $0.5\,\mu m$ mask alignment error.

| Rule | Value | Rationale |
|---|---|---|
| `NDIF width` | $2.0\,\mu m$ | $\geq 2 \times x_j$ to avoid dilution |
| `NDIF spacing to NDIF` | $5.8\,\mu m$ (10V) / $7.8\,\mu m$ (20V) | outdiffusion + depletion + linewidth control |
| `CONT width` | $1.0\,\mu m$ exact | stepper limit + aluminum fill constraint |
| `CONT spacing to CONT` | $1.0\,\mu m$ | minimum feature + linewidth control |
| `NDIF overlap CONT` | $0.4\,\mu m$ | linewidth control + mask alignment |
| `METAL width` | $1.8\,\mu m$ | doubled min feature (thick metal, nonplanarity) + linewidth control |
| `METAL spacing to METAL` | $1.8\,\mu m$ | same reasoning as width |
| `METAL overlap CONT` | $0.4\,\mu m$ | linewidth control (2 layers) + mask alignment |
| `POR width` | $10.0\,\mu m$ | wet etch minimum; bondpads are large |
| `POR spacing to POR` | $4.2\,\mu m$ | overetch + linewidth control + min overcoat width |
| `METAL overlap POR` | $1.4\,\mu m$ | overetch + linewidth control (2 layers) + mask alignment |

This example illustrates how each rule is built up from **physical considerations** (feature size, linewidth control, alignment, outdiffusion, depletion) rather than being arbitrary numbers.

---

## 3.2.4 Scalable Design Rules

In the era of self-aligned CMOS (mid-1970s to mid-1990s), all processes used the same mask set and transistor style; only minimum gate lengths shrank. In 1977, **Lynn Conway** proposed that a single set of layout rules could span multiple process generations if all dimensions were expressed as multiples of a scaling factor $\lambda$ (lambda).

Example scalable rules:

| Rule | Scalable Value |
|---|---|
| `CONT width` | $2\lambda$ |
| `CONT spacing to CONT` | $2\lambda$ |
| `METAL width` | $3\lambda$ |
| `METAL spacing to METAL` | $3\lambda$ |
| `METAL overlap CONT` | $1\lambda$ |

An **optical shrink** during pattern generation converts lambda units to physical microns. For example, if $1\lambda = 0.5\,\mu m$, a 50% optical shrink is applied.

### Limitations for Analog

**No universal scaling laws exist for analog circuits.** Some analog circuit performance improves with scaling; others degrade. Unlike digital layouts, analog layouts **cannot** be ported between process nodes simply by changing $\lambda$. Extensive circuit redesign and complete relayout are typically required.

Some analog CMOS/BiCMOS processes of the 1980s-90s used a limited form: rules written in microns with a **90% optical shrink** applied during PG. This created confusion over whether dimensions were "drawn" or "silicon" microns -- a mistake could cause ~20% error in capacitance, enough to violate parametric specs.

The era of scalable CMOS ended when aluminum vias, LOCOS isolation, and uniformly doped channels all reached their limits in the 1990s. Modern layout rules are written in **silicon dimensions expressed in microns**.

---

## Diagrams

### Geometric AND and OR Operations (p. 130)

![[diagrams/ch03-design-rules-fig1.png]]
*Page 130: Illustrations of geometric OR (ALLMOAT = NMOAT + PMOAT, Figure 3.9) and geometric AND (GATE = POLY * MOAT, Figure 3.10) operations, plus the geometric NOT operation (FIELD = MOAT\, Figure 3.11). These are the fundamental Boolean set operations used in DRC and PG decks to generate derived layers from coded layers.*

### Width and Spacing Rule Checks (p. 133)

![[diagrams/ch03-design-rules-fig2.png]]
*Page 133: Figure 3.14 shows the effects of oversizing and undersizing a U-shaped polygon -- demonstrating that these operations are not reversible. Figure 3.15 shows how width checks measure dimensions perpendicular to every boundary segment across the polygon interior. Section 3.2.2 begins with the formal definition of rule checks.*

### Design Rule Construction Example (p. 140)

![[diagrams/ch03-design-rules-fig3.png]]
*Page 140: Table 3.2 lists the four mask steps (NDIF, CONT, METAL, POR) of the hypothetical example process. The worked example shows how each design rule is derived from minimum feature size, linewidth control, mask alignment error, outdiffusion, and depletion region width. This table-driven approach is the standard methodology for constructing design rules from process physics.*

---

## Practical Takeaways

- **Always merge touching/overlapping geometries** before running rule checks -- DRC tools do this automatically, but understanding it prevents confusion when interpreting results.
- **Oversize/undersize operations are irreversible** in general. Applying oversize then undersize (or vice versa) by the same amount does NOT restore the original geometry if the operation closes notches or removes narrow features.
- **Linewidth control compounds across layers**: when a rule involves two layers, the linewidth control contribution is the sum of both layers' individual contributions.
- **Mask alignment errors combine sub-linearly**: two alignment errors combine to roughly 160% of one, not 200% -- because some error components are correlated.
- **Outdiffusion adds ~80% of junction depth** to each side of a diffusion. Design rules must account for this on both edges.
- **Depletion region widths depend on voltage**: high-voltage analog circuits need significantly wider spacings than low-voltage ones. Always add a 50% safety margin to tabulated values.
- **Dilution occurs when diffusion width < 2x junction depth**: either exploit this intentionally or avoid it by setting minimum widths accordingly.
- **The `exact` keyword** in width and overlap rules enforces that ONLY the specified dimension passes -- useful for contacts/vias that must be exactly one size.
- **`overlap okay`** in spacing rules allows geometries to touch or overlap without generating violations -- essential for rules between layers that intentionally intersect.
- **Scalable (lambda) rules do not work for analog**: circuit performance does not scale uniformly, so analog layouts require process-specific rules in absolute microns.
- **Process size adjusts are preferred over baking bias into rules** -- they allow easy correction when process equipment changes without modifying the design rules themselves.
- **Critical dimension figures** should be placed on the die so the fab can monitor linewidths during manufacturing.

---

## Relation to the Bigger Picture

Section 3.2 is the bridge between the fabrication physics of Chapter 2 and the practical act of creating layouts in the rest of Chapter 3. The geometric operations (OR, AND, NOT, ANDNOT, XOR, size adjust) introduced here are the same operations used in pattern generation (Section 3.3, [[ch03-pattern-generation]]) to construct photomask data from coded layers. Understanding how design rules are constructed from minimum feature size, linewidth control, mask alignment, outdiffusion, and depletion width is essential for any analog layout designer who needs to make intelligent trade-offs -- for example, knowing whether a spacing rule has margin to spare or is already at the physical limit. This knowledge also directly informs the matching and symmetry techniques discussed later in the book, since linewidth control and mask alignment errors are the dominant sources of systematic mismatch in analog circuits.

---

## See Also
- [[ch03-layout-editors]]
- [[ch03-pattern-generation]]
