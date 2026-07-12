---
title: "14.3 Single-Level Interconnection"
chapter: 14
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-14, interconnection, tunnels, routing, mock-layouts]
---

# 14.3 Single-Level Interconnection

> **Chapter 14: Special Topics**

## Key Concepts

### Why Single-Level Interconnection Matters

Most modern IC processes provide two or more metal layers, making routing straightforward: leads on different metal layers can freely cross one another, so placement is constrained only by matching and packing considerations. Even autorouting tools can handle analog blocks given appropriate keep-out zones and metal width rules.

However, when a process offers **only one level of metallization**, routing becomes dramatically harder. Without a second metal layer, leads cannot simply cross over each other. The designer must resort to **tunnels** (also called crossunders) --- low-value resistors made from diffusion layers that pass signals beneath the single metal layer. These tunnels are costly: they consume die area, add parasitic resistance and capacitance, and degrade circuit performance.

Although single-level-metal (SLM) processes are largely obsolete, the skills remain relevant in specific modern contexts. For example, when a metal-2/metal-3 capacitor is placed over active circuitry, the underlying routing may be restricted to a single metal layer, requiring SLM techniques in that local area.

A good SLM layout employs various **stratagems to minimize the number of tunnels** and maximize packing density. The core challenge is an interconnection puzzle: how to arrange components so that as few leads as possible need to cross.

### The Fundamental Trade-off

Every tunnel introduces:
- **Series resistance** (hundreds of ohms for base tunnels, less for emitter tunnels)
- **Parasitic junction capacitance** (tens to hundreds of femtofarads)
- **Area overhead**
- **Vulnerability to junction leakage and minority carrier collection**

Widening a tunnel to reduce resistance *increases* its capacitance, so there is an inherent trade-off. The designer must balance electrical performance against layout compactness.

## 14.3.1 Mock Layouts

### Purpose and Method

The greatest challenge of single-level routing is **arranging components to minimize tunnels**. Matched components make this especially difficult because they impose symmetry constraints. Even skilled designers must try several arrangements before finding a suitable one.

These trials take the form of **mock layouts** --- rough sketches (not to scale) that capture the essential topological relationships:

- **Transistors** are drawn as rectangles with terminals (E, B, C) marked by letters
- **Resistors** appear as strips with connections at either end
- **Dummies and resistor tanks** are omitted (they do not affect routing)
- **Tank contacts** are marked "TC"
- **Merged devices** sharing a common tank are drawn abutting one another

The mock layout does not need to be geometrically accurate. Its purpose is to illustrate all the important routing features so the designer can evaluate whether the arrangement requires tunnels and where crossings occur.

### Matching Constraints in Mock Layouts

When a layout contains matched devices, each set of matched components should be arranged **symmetrically around one axis** of the layout (see Section 8.2.8). This axis of symmetry typically passes through the middle of the sketch.

For matched resistors whose values are not in a simple integer ratio, the designer must carefully choose segmentation. For example, resistors in a ratio that requires 7 segments for one and 1 segment for another cannot achieve a perfect common centroid. The solution may involve using 6 segments plus a partial segment, with a sliding contact to allow ratio tweaking.

### Avoiding Tunnels Through Clever Arrangement

A well-designed mock layout can eliminate tunnels entirely through several techniques:
- **Routing leads through resistor arrays**: A lead interconnecting two terminals can be routed through gaps in a segmented resistor array
- **Stretching transistors**: Transistors can be elongated to create routing channels between their terminals. This works particularly well when deep-$N^+$ sinkers have already increased terminal spacing
- **Merged devices in common tanks**: Matched devices sharing a common terminal (e.g., common base) can reside side-by-side in a single tank, simplifying interconnection

### Paper Dolls

Designers sometimes refine mock layouts by using **paper plots of actual components** at a convenient scale (100:1 or 250:1). Components are cut out and physically shuffled on paper until a good arrangement is found, then glued down and connected with drawn lines. These "paper dolls" were historically used for difficult layouts. Modern layout editors with PCell and flight-line capabilities can achieve similar results, though most PCell implementations are not sufficiently flexible for SLM routing.

## 14.3.2 Techniques for Crossing Leads

Despite best efforts, most SLM layouts still require some crossings. Six techniques are available, listed in roughly decreasing order of preference:

### 1. Cross Leads Over Resistors

Routing a lead across a resistor provides a crossing point **without consuming additional area**. However, not every lead can safely cross every resistor:
- Some resistors require **field plating** that restricts or prevents crossings
- Resistors carrying **sensitive signals** are susceptible to noise coupling from the crossing lead
- Lightly doped materials (e.g., HSR) may experience **voltage modulation effects** from the crossing metal
- Hydrogenation effects seldom cause significant variation in monocrystalline resistors

### 2. Rearrange Device Terminals

Crossings can often be eliminated by changing terminal order. For example, an NPN transistor can use either:
- **CEB layout**: emitter between collector and base
- **CBE layout**: base between collector and emitter

The CBE layout has slightly more collector resistance, but the difference seldom matters in practice.

### 3. Stretch Devices to Allow Leads to Pass Through

Most device types can be **stretched** to accommodate one or more leads between their terminals. Stretched devices have more parasitic resistance and capacitance than unstretched ones, potentially degrading performance. **If one of a group of matched devices is stretched, all others must be similarly stretched** to preserve matching.

### 4. Connect Signals Through Merged Devices

Certain devices can serve as built-in tunnels. An NPN transistor with a **stretched base region and two contacts** effectively merges a transistor and a base tunnel into one tank. Similarly, multiple tank contacts can provide a signal path. Caution: large currents through merged tunnels can cause **debiasing** that interferes with other devices in the same tank.

### 5. Insert Tunnels

Tunnels (crossunders) are dedicated low-value resistors added specifically to allow lead crossings. All tunnels have disadvantages:
- They consume area and add parasitic $R$ and $C$
- High-current leads can suffer excessive **debiasing and power dissipation**
- They can upset matching by introducing voltage drops
- The tunneled lead's voltage must not exceed the **breakdown voltage** of the tunnel's reverse-biased junctions
- Circuit designers must **approve or reject each proposed tunnel** and resimulate the circuit with tunnels included

### 6. Rearrange the Bondpads

If high-current leads must cross to reach their bondpads, consider rearranging the bondpad positions. This requires coordination with the system designer and possibly the package designer, since it changes the package pin order.

### Annotated Schematics for Communication

The layout designer needs guidance from the circuit designer about which stretches and tunnels are acceptable. This is communicated through an **annotated schematic** using color-coded wire segments:

| Category | Precautions | Annotation |
|---|---|---|
| **Power leads** | No tunnels allowed; leads must meet minimum width | Highlight in **red**; mark width |
| **Noisy leads** | Do not cross sensitive leads or their devices | Highlight in **yellow** |
| **Sensitive leads** | Do not tunnel; no substrate contacts in sensitive ground leads | Highlight in **green** |
| **Noncritical leads** | No special precautions | None |

Additional annotations typically list matched components and devices requiring guard rings.

## 14.3.3 Types of Tunnels

In standard bipolar processes, tunnels can be constructed from several diffusion layers. The choice depends on the required resistance, isolation, and available area.

### Base Tunnels

- **Sheet resistance**: ~$200\ \Omega/\square$ (typical)
- **Series resistance**: a few hundred ohms (typical)
- **Parasitic capacitance**: tens to hundreds of femtofarads
- **Key advantage**: base diffusion can occupy a common tank with other components without merging signals
- **Key limitation**: resistance is high enough to interfere with device matching and upset delicate circuit balance

Base tunnels are the **most commonly used** tunnel type in standard bipolar because of their ability to coexist in a tank without signal merging.

### Emitter Tunnels

- **Sheet resistance**: approximately an order of magnitude lower than base diffusion (~$5\ \Omega/\square$ typical)
- **Structure**: a strip of emitter diffusion placed in a tank (see Figure 14.17)
- The **tank-substrate junction** provides isolation from the substrate
- The tank requires **no separate contact** --- the tunnel itself provides the necessary connection
- **NBL should be omitted**: it does not significantly reduce tunnel resistance but increases parasitic capacitance

### Emitter-in-Isolation Tunnels

- **Structure**: emitter diffusion placed directly in isolation, eliminating the tank entirely (see Figure 14.18)
- **Area savings**: considerable, since no tank is needed
- **Limitation**: the emitter counterdopes the isolation, and the resulting $N^+/P^+$ junction typically has a breakdown voltage of only a few volts
- Low breakdown junctions tend to **leak**, so these tunnels are mostly restricted to **ground leads**
- Processes with emitter-in-iso breakdown voltages of $\geq 10\ \text{V}$ can use them for other signals, but the junction capacitance is relatively large (~$0.5\ \text{pF}$)
- **BOI (base over isolation)** is generated automatically, not drawn

### Stacked Tunnels

- **Structure**: combines all available N-type materials (N-epi, NBL, deep-$N^+$, and emitter) in a single stack (see Figure 14.19)
- **Sheet resistance**: slightly lower than emitter alone (typical ~$3\ \Omega/\square$)
- NBL provides **little benefit** without the addition of deep-$N^+$
- Only available in processes that offer deep-$N^+$ sinkers

### NBL Tunnels

- **Structure**: bridges between two adjacent tanks using a buried layer connection (see Figure 14.20)
- The strip of **isolation between the tanks** acts as an effective P-bar, preventing cross-injection between tanks
- **Requirement**: the NBL/isolation breakdown voltage must exceed approximately $7\ \text{V}$ to avoid excessive Zener leakage
- Most standard bipolar processes meet this requirement
- Deep-$N^+$ sinkers can be added to further reduce resistance

## Diagrams

### Figure 14.16 -- Mock Layout Example

![[diagrams/ch14-interconnection-fig1.png]]

*Mock layout for a portion of a circuit containing matched components. Transistors are drawn as labeled rectangles (E, B, C terminals), resistors as strips, and tank contacts marked "TC." The layout demonstrates how matched components are arranged symmetrically around a horizontal axis, and how clever arrangement can avoid tunnels entirely by routing leads through resistor arrays and stretching transistors.*

### Figures 14.17 and 14.18 -- Emitter Tunnel and Emitter-in-Iso Tunnel

![[diagrams/ch14-interconnection-fig2.png]]

*Top: Conventional emitter tunnel layout and cross section showing a strip of emitter diffusion inside a tank, with the tank-substrate junction providing isolation. Bottom: Emitter-in-iso tunnel, which saves area by eliminating the tank but has low breakdown voltage ($\sim$ a few volts) at the $N^+/P^+$ junction, restricting its use primarily to ground leads.*

### Figures 14.19 and 14.20 -- Stacked Tunnel and NBL Tunnel

![[diagrams/ch14-interconnection-fig3.png]]

*Top: Stacked tunnel cross section combining emitter, deep-$N^+$, and NBL for minimum sheet resistance. Bottom: NBL tunnel bridging two adjacent tanks, with the isolation strip between tanks acting as a P-bar to prevent cross-injection. Deep-$N^+$ sinkers can optionally reduce resistance.*

## Practical Takeaways

- **Always start with a mock layout** when doing single-level-metal routing. Try multiple arrangements before committing to one, focusing on minimizing tunnel count.
- **Exploit symmetry** for matched components: arrange them around a common axis to satisfy matching constraints while simplifying routing.
- **Prefer crossing leads over resistors** as the first option --- it adds no area cost. But check for field plating requirements, noise sensitivity, and voltage modulation concerns.
- **Rearranging device terminals** (e.g., CEB vs. CBE for NPN) is a low-cost way to eliminate crossings with minimal performance impact.
- **If stretching a matched device, stretch all its partners** identically to preserve matching.
- **Circuit designers must approve all tunnels.** Tunnels must be added to the schematic and resimulated to verify they do not shift critical parameters.
- **Use the annotated schematic convention** (red/yellow/green/none) to communicate routing constraints between circuit and layout designers.
- **Choose tunnel type based on requirements:**
  - Base tunnels for most signals (can coexist in tanks)
  - Emitter tunnels for lower resistance
  - Emitter-in-iso tunnels only for ground leads (low breakdown voltage)
  - Stacked tunnels for minimum resistance (requires deep-$N^+$)
  - NBL tunnels for bridging between adjacent tanks
- **Never put tunnels in power leads** --- the added resistance causes unacceptable debiasing and power dissipation.
- **Tunneled leads must not exceed the tunnel junction's breakdown voltage** or excessive leakage will result.
- **Widening a tunnel reduces resistance but increases capacitance** --- there is no free lunch. Larger tunnels are also more vulnerable to junction leakage and minority carrier collection.

## Relation to the Bigger Picture

Single-level interconnection techniques, while largely historical, embody fundamental principles of analog layout: minimizing parasitics, preserving matching, and thinking topologically about component placement. The mock layout methodology --- sketching, rearranging, and iterating --- remains valuable even in multi-metal processes when routing is locally constrained. The tunnel types described here (base, emitter, stacked, NBL) reuse the same diffusion layers discussed throughout Chapters 4 (process architectures), 6 (resistors), and 9 (bipolar transistors), reinforcing how analog layout constantly repurposes process layers for multiple functions. The annotated schematic convention and the close collaboration between circuit and layout designers emphasized here is a recurring theme throughout the book, connecting to the guard ring placement decisions in [[ch14-guard-rings]] and the routing constraints imposed by ESD protection structures in [[ch14-esd-protection]].

## See Also

- [[ch14-guard-rings]]
- [[ch14-esd-protection]]
