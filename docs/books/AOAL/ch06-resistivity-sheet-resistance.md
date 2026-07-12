---
title: "6.1-6.2 Resistivity, Sheet Resistance, and Layout"
chapter: 6
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-6]
---

# 6.1-6.2 Resistivity, Sheet Resistance, and Layout

> **Chapter 6: Resistors**

## Key Concepts

### Resistors as Passive Elements

Resistors are *passive* devices -- they dissipate electrical energy but cannot generate it. Integrated resistors are relatively easy to fabricate, but their absolute tolerance is poor ($\pm 20\%$ is typical). However, the *matching* between two resistors on the same die is excellent (often better than $\pm 0.1\%$). This asymmetry -- poor absolute accuracy but superb relative matching -- is the fundamental reason analog IC designers rely so heavily on *ratios* of matched resistors rather than absolute resistance values.

Most processes offer several choices of resistance material (diffusions, polysilicon, metal, thin film), each suited for specific applications. The designer selects both the material and the width; the physical construction of the resistor (layout geometry, contacts, bends) also affects its performance significantly.

### Ohm's Law and Resistivity

Ohm's law relates voltage, current, and resistance:

$$V = I \cdot R \tag{6.1}$$

where $V$ is the voltage drop, $I$ is the current, and $R$ is the resistance. This relationship holds as long as the electric field is not too intense and the carrier concentration remains constant, so the drift velocity varies linearly with the electric field. A conductor exhibiting constant, positive, nonzero resistance is called *Ohmic*. Most integrated resistors exhibit at least some nonlinearity (covered in Section 6.3.3).

Every material possesses a characteristic **resistivity** $\rho$, usually measured in $\Omega \cdot \text{cm}$. Conductors have very low resistivities (copper: $\sim 1.7 \times 10^{-6}\ \Omega\cdot\text{cm}$), while doped semiconductors have moderate values that depend strongly on doping concentration:

| Material | Resistivity |
|---|---|
| Copper, bulk | $1.7 \times 10^{-6}\ \Omega\cdot\text{cm}$ |
| Gold, bulk | $2.4 \times 10^{-6}\ \Omega\cdot\text{cm}$ |
| Aluminum, thin film | $2.7 \times 10^{-6}\ \Omega\cdot\text{cm}$ |
| Al + 2% Si, thin film | $3.3 \times 10^{-6}\ \Omega\cdot\text{cm}$ |
| TiW (60/40), sputtered | $7.5 \times 10^{-5}\ \Omega\cdot\text{cm}$ |
| TiN, sputtered | $4.4 \times 10^{-5}\ \Omega\cdot\text{cm}$ |
| PtSi, thin film | $3.0 \times 10^{-5}\ \Omega\cdot\text{cm}$ |
| TiSi$_2$, C54, thin film | $1.5 \times 10^{-5}\ \Omega\cdot\text{cm}$ |
| CoSi$_2$, thin film | $1.8 \times 10^{-5}\ \Omega\cdot\text{cm}$ |
| NiSi, thin film | $1.1 \times 10^{-5}\ \Omega\cdot\text{cm}$ |
| Si, N-type ($N_D = 10^{16}\ \text{cm}^{-3}$) | 0.25 $\Omega\cdot\text{cm}$ |
| Si, N-type ($N_D = 10^{14}\ \text{cm}^{-3}$) | 48 $\Omega\cdot\text{cm}$ |
| Si, intrinsic | $2.3 \times 10^{5}\ \Omega\cdot\text{cm}$ |

**Key point:** The term *bulk resistivity* refers to resistivity measured through a material, not across its surface. Surface resistivities of insulators depend on environmental conditions and are frequently much smaller than bulk resistivities.

### Isotropy and Anisotropy of Resistivity

Bulk unstressed monocrystalline silicon has *isotropic* resistivity (same in all directions) because silicon crystallizes in the cubic system. However, several effects can introduce anisotropy:

- **Shallow diffused resistors** may exhibit slight anisotropy due to carrier interactions with the crystal surface.
- **Mechanical stress** causes orientation-dependent resistivity variations (*piezoresistivity*).
- **Polycrystalline thin films** often exhibit large differences between vertical and lateral resistivity due to grain orientation effects, though these are usually subsumed into contact resistance.

In practice, these orientation effects are small enough to matter only when constructing accurately matched resistors (Chapter 8).

### From Resistivity to Sheet Resistance

For a rectangular slab of homogeneous material with length $L$, width $W$, and thickness $t$, the resistance is:

$$R = \frac{\rho \cdot L}{W \cdot t} \tag{6.2}$$

Since diffusions can usually be modeled as films of constant thickness, it is convenient to combine resistivity and thickness into a single term called the **sheet resistance** $R_s$:

$$R_s = \frac{\rho}{t} \tag{for homogeneous material}$$

The resistance formula then simplifies to:

$$R = R_s \cdot \frac{L}{W} \tag{6.3}$$

The ratio $L/W$ is dimensionless but is assigned the fictitious unit of **squares** ($\square$). A resistor with $L = W$ contains one square; $L = 2W$ contains two squares, etc. Sheet resistance $R_s$ is given in units of $\Omega/\square$ (ohms per square). To find the resistance of any rectangular resistor, simply count the number of squares and multiply by $R_s$.

> **Example:** A resistor containing 10 squares of $1\ \text{k}\Omega/\square$ material has a value of $10\ \text{k}\Omega$.

For a diffusion of junction depth $x_j$, the sheet resistance is computed from:

$$R_s = \left( \int_0^{x_j} \frac{1}{\rho(x)} \, dx \right)^{-1} \tag{6.4}$$

This integral is difficult to evaluate for realistic diffusion profiles. One can use **Irvin's curves** to determine the sheet resistance of an idealized diffusion, but in practice sheet resistances are usually **determined by measurement** rather than computation.

## Important Details

### 6.2 Resistor Layout

#### The Simple Strip Resistor

A simple integrated resistor is a rectangular strip of resistive material with contacts at either end. The low resistance of a contact effectively shorts out the material beneath it, so most current exits along the inner edge of each contact.

- **Drawn length** $L_d$: distance between inner edges of the two contacts
- **Drawn width** $W_d$: width of the resistive strip

The drawn dimensions give an approximate resistance via Equation 6.3, but several practical factors cause deviations from this ideal.

#### Process Biases: Effective Width and Length

The *effective* (actual) width and length differ from drawn dimensions due to process biases:

$$W = W_d + W_b \tag{6.5}$$

$$L = L_d + L_b \tag{6.6}$$

where $W_b$ and $L_b$ are the width and length process biases, respectively. Substituting into Equation 6.3:

$$R = R_s \cdot \frac{L_d + L_b}{W_d + W_b} \tag{6.7}$$

**Width bias** $W_b$ usually has a much greater impact than length bias because resistors are typically much longer than they are wide. For diffused resistors, the width bias typically equals about 20% of the junction depth $x_j$. For example, a base resistor with $x_j = 8\ \mu\text{m}$ has $W_b \approx 1.6\ \mu\text{m}$ due to outdiffusion, causing roughly a 5% decrease in the value of an $8\ \mu\text{m}$-wide resistor. The width bias can be determined experimentally by measuring a set of resistors of varying widths.

#### Nonuniform Current Flow (End Effects)

Equation 6.7 assumes uniform current flow, but in practice:

1. **Lateral nonuniformity:** Contacts that do not extend entirely across the resistor width cause current crowding near the contacts. This is quantified by:

$$\Delta R = \frac{2}{3} \left[ 1 + \ln\left( \frac{W}{W_c} \right) - \frac{W_c}{W} \right] \tag{6.8}$$

   where $W$ is the effective resistor width and $W_c$ is the effective contact width. $\Delta R$ represents the increase in resistance (in squares) caused by nonuniform current flow at both ends. For a resistor $10\ \mu\text{m}$ wide with $5\ \mu\text{m}$-wide contacts, $\Delta R \approx 0.05$ squares -- negligible for resistors at least 10 squares long.

2. **Vertical nonuniformity (current crowding):** Current must bend upward to exit through the resistor surface, crowding toward contact inner edges. This is usually considered part of contact resistance and is negligible when the resistor length is $\geq 20 \times$ the thickness.

**Practical rule:** Accurate resistors should use segments at least $10\ \mu\text{m}$ long to avoid end-effect errors.

#### Serpentine (Meander) Resistors

Large resistors are often folded into serpentine shapes to save area. These typically use **rectangular turns** (Figure 6.3A), though **circular turns** (Figure 6.3B) are sometimes used in high-voltage resistors where sharp corners would diminish breakdown voltage.

**Corner counting rule:** Each square corner of a serpentine resistor adds approximately **0.56 squares** -- commonly rounded to the rule "a corner counts half a square." For a rectangular serpentine resistor (neglecting process biases and end effects):

$$R = R_s \left( \frac{2A + B}{W} + 0.56 \cdot N_c \right) \tag{6.9}$$

where $A$ and $B$ are the segment lengths and $N_c$ is the number of corners.

For a more precise calculation at a corner between segments of width $W$ and $aW$:

$$N = \frac{1}{2} - \frac{1}{\pi} \left[ \frac{a}{a+1} \cdot \ln(a) + \frac{1}{a+1} \cdot \ln\left(\frac{1}{a}\right) + \ln\left(\frac{(a+1)^2}{4a}\right) \right] \tag{6.10}$$

where $\cot^{-1}(a)$ returns radians. For equal-width segments ($a=1$), this gives 0.5587 squares.

The $180°$ circular end segment of a circular-turn serpentine adds **2.96 squares**.

$$R = R_s \left( \frac{2A}{W} + 2.96 \cdot N_{180} \right) \tag{6.11}$$

#### Dogbone (Dumbbell) Resistors

When a resistor is too narrow to place contacts inside it without violating design rules, the ends are enlarged to form "heads" around the contacts. This creates a **dogbone** or **dumbbell** resistor.

- **Drawn length** $L_d$ is measured contact-to-contact
- **Drawn width** $W_d$ is measured across the body (not the heads)
- Resistance is approximated with Equation 6.7, with bend corrections as for strip resistors

In a dogbone, current *spreads out* as it enters the heads (opposite of the strip resistor where current crowds inward). This effect is minimized by:
- Making the contact width $W_c$ equal to the resistor body width $W_d$
- Minimizing the overlap $W_o$ of the head over the contact

Published correction factors $\Delta R$ for two dogbone heads are given in Table 6.3 (in the text) and are usually less than 0.3 squares.

**Dogbone vs. strip accuracy:** Many designers consider dogbone resistors more accurate because lateral nonuniform current flow errors are smaller. However, for *matched* resistors this advantage is irrelevant -- as long as each section uses the same layout, matching is preserved regardless of head style.

## Diagrams

### Figure 6.1 -- Basic Resistor Model and Sheet Resistance Concept

![[diagrams/ch06-resistivity-sheet-resistance-fig1.png]]

**Caption:** A simple resistor consisting of a rectangular slab of resistance material (length $L$, width $W$, thickness $t$) contacted by highly conductive plates at either end. This is the fundamental model from which the sheet resistance concept ($R_s = \rho/t$) and the "counting squares" method ($R = R_s \cdot L/W$) are derived.

### Figure 6.2 and 6.3 -- Strip and Serpentine Resistor Layouts

![[diagrams/ch06-resistivity-sheet-resistance-fig2.png]]

**Caption:** Top: Layout of a simple strip resistor showing drawn width $W_d$, drawn length $L_d$, and contacts at either end. Bottom: Serpentine resistors with (A) rectangular turns and (B) circular turns. Rectangular turns are standard; circular turns are used in high-voltage applications to avoid breakdown at sharp corners. Each rectangular corner adds approximately 0.56 squares.

### Figure 6.5 -- Dogbone Resistor and Correction Factors

![[diagrams/ch06-resistivity-sheet-resistance-fig3.png]]

**Caption:** Layout of a dogbone resistor (top) showing how the resistor body widens into heads to accommodate contacts. The drawn length $L_d$ is measured contact-to-contact; drawn width $W_d$ is across the body. Table 6.3 (shown) gives correction factors $\Delta R$ for various ratios of contact width $W_c$ to resistor width $W_d$ and head overlap $W_o$.

## Practical Takeaways

- **Count squares:** The fastest way to estimate a resistor value is to count the number of squares ($L/W$) and multiply by the sheet resistance $R_s$ ($\Omega/\square$). This simple method is the foundation of all resistor layout.
- **Width bias dominates:** For diffused resistors, the width bias $W_b$ (caused by outdiffusion) is typically ~20% of the junction depth and is the largest source of systematic error. Always correct for it. Length bias is usually negligible by comparison.
- **Minimum segment length:** Use segments at least $10\ \mu\text{m}$ long to keep end effects (nonuniform current flow) below 1%. For thickness-related current crowding, make resistors at least $20 \times$ their thickness in length.
- **Corner rule of thumb:** Each rectangular corner in a serpentine resistor adds approximately $0.56 \approx 0.5$ squares. For precise work, use the exact formula (Eq. 6.10).
- **Dogbone heads:** When contacts cannot fit inside the resistor width, use dogbone heads. Minimize the overlap $W_o$ and match $W_c$ to $W_d$ to reduce correction factors.
- **Layout consistency for matching:** For matched resistors, use identical layout sections. The choice of dogbone vs. strip head style does not matter as long as all matched segments are identical.
- **Circular turns for high voltage:** Use circular (filleted) turns in serpentine resistors when breakdown voltage at sharp corners is a concern. Also fillet the corners behind contacts.
- **Sheet resistance is measured, not calculated:** While Irvin's curves exist for idealized diffusion profiles, practical sheet resistances are determined by measurement because real diffusion profiles do not perfectly match theoretical models.

## Relation to the Bigger Picture

Sections 6.1 and 6.2 establish the foundational framework that the entire rest of Chapter 6 builds upon: the sheet resistance abstraction reduces a 3D resistivity problem to simple 2D "square counting," and the layout rules (process biases, corner corrections, end effects) determine how accurately a drawn geometry translates into a realized resistance value. These concepts are essential prerequisites for understanding the specific resistor types discussed later in Chapter 6 (base, emitter, pinch, poly, thin-film resistors) and for the variability analysis covered next. The matching principles introduced here -- that relative accuracy far exceeds absolute accuracy -- connect directly to Chapter 8's treatment of matched resistor construction, which is one of the most critical skills in analog IC layout.

## See Also
- [[ch06-resistor-variability]]
