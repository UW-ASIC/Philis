---
title: "6.6 Adjusting Resistor Values"
chapter: 6
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-6, trimming, tweaking, fuses, zener-zap, laser-trim]
---

# 6.6 Adjusting Resistor Values

> **Chapter 6: Resistors**

## Key Concepts

Analog integrated circuits routinely need post-fabrication adjustments to resistor values to compensate for process variations and design uncertainties. Hastings distinguishes two fundamentally different adjustment strategies:

- **Tweaking** -- a mask-level change that affects *all* subsequent units manufactured with the new mask. One fabricates a sample lot, evaluates the results, then changes a single mask (or a small number of masks) to shift specific resistor values. This is a batch adjustment: cheap per unit, but slow because a new mask must be fabricated and the wafers re-processed from that point.

- **Trimming** -- an individualized, per-unit adjustment performed at wafer probe or final test. The test program measures each unit, trims it, then re-measures to confirm correctness. Trimming counters die-to-die (and wafer-to-wafer) process variation, which tweaking cannot address.

The practical importance is enormous: process variability in wafer fabrication is large enough that many precision analog products would be unusable without some form of post-fabrication adjustment. The choice between tweaking and trimming -- or combining both -- depends on cost targets, required precision, and the nature of the variation being compensated.

---

## 6.6.1 Tweaking Resistors

Tweaking is attractive because it modifies only one mask, dramatically reducing the cost and turnaround time of a design iteration. The strategy is:

1. Fabricate an initial lot with resistors set to reasonable estimated values.
2. Pull some wafers just before the tweakable-resistor step.
3. Evaluate the remaining wafers.
4. Redesign only the one mask and run the held-back wafers through the final steps.

### Sliding Contacts

The simplest tweak. One end of the resistor body is extended, and the contact can slide along this extension to add or subtract length (and thus resistance). The metal plate over the contact is also elongated so that a single metal mask covers all possible contact positions.

- **Without heads** (Figure 6.23A): The contact simply moves along the uniform-width resistor body. The contact should initially be placed at the midpoint of its travel range.
- **With heads** (Figure 6.23B): Works only when head material has the same resistivity as the body (e.g., standard bipolar base or emitter resistors). The head must be elongated to accommodate the sliding range. The resistance can be approximated as two sections in series (narrow body + wide head); nonuniform current flow at the junction introduces a small error that is acceptable because the resistor will be tweaked anyway.

Sliding contacts are **not useful** for resistors whose heads are made of a low-sheet material different from the body (e.g., HSR or high-sheet poly with silicided heads), because the contact can only move within the low-sheet head region and makes negligible changes to total resistance.

### Sliding Heads

For resistors with high-sheet bodies and low-sheet heads (HSR, high-sheet poly, silicided-head poly), the *head* is slid into or out of the resistor body. Extending the head further into the body shortens the high-sheet region and reduces resistance; pulling it back increases resistance.

The resistor is modeled as two resistors in series: one for the body (high-sheet) and one for the heads (low-sheet). Although nonuniform current flow at the head-body interface causes slight computation errors, these are acceptable since tweaking will correct them.

Commonly used for: HSR resistors, high-sheet poly resistors, poly resistors with silicided heads.

### Trombone Slides

For serpentine resistors, one or more turns can be slid inward or outward -- like the slide on a trombone. This changes the total length without altering the number of turns. Room must be left adjacent to the resistor for extension. If the resistor sits in a tank, well, or under an implant/silicide block mask, those enclosing geometries must also extend to cover the slide range.

### Metal Options

The resistor is divided into multiple sections. Most are connected in series to form the initial value; a few spare sections are left unconnected but include contacts covered by metal. Tweaking is done by adding or removing segments using a new metal mask. Segments can be joined in parallel as well as in series, increasing the number of achievable values beyond the raw number of spare segments. Metal options can be combined with sliding heads to provide a very wide adjustment range using only two new masks.

---

## 6.6.2 Trimming Resistors

Trimming provides individualized adjustment of each die. Five techniques are discussed: fuses, Zener zaps, nonvolatile memory (NVM), electrical trimming of polysilicon, and laser trimming.

### Fuses

A fuse is a narrow conductive link that initially has low resistance. Passing a large current through it creates a void (blows it open), producing a very high resistance (ideally open circuit).

**Materials and history:**

| Material | Notes |
|----------|-------|
| **Nichrome** | Earliest fuses (~200 A thick). Used in early PROMs. Susceptible to electrochemical corrosion and *fuse regrowth* (grow-back) -- programmed fuses returning to low resistance due to arc-over or ion migration across narrow gaps. Slow-rise-time pulses worsened regrowth. |
| **Aluminum** | Thicker than nichrome; voiding cracks the protective overcoat. Openings placed over fuses to prevent uncontrolled cracking. Metal droplets splatter onto probe needles, requiring periodic cleaning. |
| **Polysilicon** | Preferred in CMOS/BiCMOS. Requires higher melting temperature ($1415\degree C$ vs $660\degree C$ for Al). Slow-rise-time pulses can crack poly before it melts, leaving narrow gaps prone to regrowth. A rise time $< 25\,\text{ns}$ is recommended. Fully silicided poly can be programmed at lower currents via electromigration (resistance shifts from ~$50\,\Omega$ to ~$10\,\text{k}\Omega$) without cracking the overcoat. Minimum-width fuses $< 0.8\,\mu\text{m}$ generally do not crack the overcoat and can be "closed" (no opening needed). |

**Layout considerations (Figure 6.26):**

- **Metal fuse:** A constricted segment in a wide metal lead with a small nitride opening above. Unprogrammed resistance is a fraction of an ohm. Programming: ~5 V for 1 ms, several hundred mA, rise time $< 1\,\mu\text{s}$.
- **Poly fuse:** Minimum-width poly strip between enlarged heads with sufficient contacts to avoid contact damage during programming. Programming: 5--15 V for 1 ms, 50--150 mA, rise time $< 25\,\text{ns}$.

**Trimpads:** Fuses require dedicated probe pads (trimpads) placed around the die periphery. These are smaller than bondpads. Leads to fuses should be $\geq 5\times$ the fuse width. Minimizing trimpads saves die area; series or parallel fuse connections allow trimpad sharing.

**Fuses with openings** cannot be programmed after encapsulation (plastic seals the opening and may char). Closed fuses can potentially be programmed post-package, but large programming currents make compact on-chip programming circuitry difficult.

**Programming voltage caution:** Large voltages during fuse blowing can avalanche emitter-base junctions or damage gate oxides. Place fuses on the least-vulnerable end of a resistor (e.g., grounded end of a Brokaw bandgap trim resistor).

### Binary-Weighted Trim Networks

Multiple fuses are organized into binary-weighted networks where successive fuse weights follow the sequence $1, 2, 4, 8, \ldots, 2^{N-1}$, with $N$ the total number of fuses (typically 3--6). Each fuse corresponds to one bit in a binary trim code.

Two topologies exist:

- **Series-connected** (Figure 6.27A): Trim resistors $R_{\text{LSB}}, 2R_{\text{LSB}}, 4R_{\text{LSB}}, \ldots$ in series with the main resistor. Blowing a fuse *adds* that segment's resistance. Trims the voltage across the resistor.
- **Parallel-connected** (Figure 6.27B): Trim resistors $R_{\text{total}}, R_{\text{total}}/2, R_{\text{total}}/4, \ldots$ in parallel with the main resistor. Blowing a fuse *removes* a parallel path, increasing total resistance. Trims the current through the resistor.

The number of bits $N$ required for resolution $r$ across range $R_{\text{range}}$:

$$N = \lceil \log_2(R_{\text{range}} / r) \rceil \quad \text{[Eq. 6.25]}$$

For example, $0.1\%$ resolution across $\pm 7.5\%$ range requires $\lceil \log_2(150) \rceil = 8$ bits.

**Beyond 6 bits:** Matching errors between individual resistors may accumulate to more than 1 LSB. Solutions:
- A second correction network in series with the first (e.g., 8-bit main + 3-bit correction).
- Nonbinary-weighted networks with ratios slightly less than 2:1 (e.g., 1:2:4:6:12:24:36:72:144...), trimmed via binary search. These incorporate "spare bits" that absorb matching errors.

**Differential trimming** (Figure 6.28): For very small LSB resistances that are hard to construct precisely. Two resistors $R_A$ and $R_B$ are connected in parallel across the fuse. Blowing the fuse disconnects $R_B$, shifting the series resistance by:

$$\Delta R = \frac{R_A \cdot R_B}{R_A + R_B} - R_A = -\frac{R_A^2}{R_A + R_B} \quad \text{[Eq. 6.26]}$$

This allows arbitrarily small effective trim steps using physically large (and therefore precise, thermally robust) resistors.

**Remote trim with CMOS transistors** (Figure 6.29): Fuses at the die periphery drive MOS transistor switches adjacent to the actual resistors in the die interior. The control wires carry minimal current and can be minimum width. MOS transistor widths should scale inversely with segment resistance to impose equal percentage on-resistance errors on each segment. Current mirrors biased by a master transistor provide the control current; inverters fed from a clean supply prevent noise coupling.

**Look-ahead trimming:** By applying a small voltage across a poly fuse (enough for the comparator to register as "blown" but not enough to actually program it), the test program can preview the effect of a trim code before committing. Combined with a **binary search algorithm**, this eliminates the need to precompute trim codes.

**Achievable accuracy:** Trimmed resistors are typically limited to $\pm 0.1\%$ by thermal/mechanical stress gradients and long-term drift. Trimmed thin-film resistors may reach $\pm 0.05\%$.

### Zener Zaps

A Zener diode is used as an anti-fuse: it starts as a high-resistance (reverse-biased junction) and is programmed to low resistance (~$50\,\Omega$) by forcing a large reverse current that creates a metallic filament across the junction.

**Construction (Figure 6.30):** Laid out like a small NPN transistor in standard bipolar. Collector + emitter = cathode; base = anode. Deep-$N^+$ is omitted to save space. Emitter and base contacts are placed as close together as rules allow to facilitate zapping.

**Programming mechanism:** A current of 100--250 mA causes extreme localized heating in the emitter-base depletion region, current filamentation, and movement of a molten metallic filament (aluminum alloyed with silicon) across the junction gap.

**Programming protocol:** Either a single pulse of 100--250 mA for several ms, or a two-stage pulse: 100--250 mA for 0.5--1 ms (forms the filament), then 30--60 mA for 2--3 ms (reduces filament resistance without excessive heating).

**Alternative layouts (Figure 6.31):** Circular emitters (lower zap current, predictable filament location), triangular metal protrusions pointing toward the base contact (~10% current reduction), sharply pointed base diffusion overlapping a circular emitter (factor of ~2 current reduction).

**Advantages over fuses:**
- No openings in protective overcoat needed (no contamination pathway).
- Post-package trimming is possible (though large power devices are required).
- Support look-ahead programming using the same probes.

**Critical limitation:** Zener zaps should **not** be used on processes with refractory barrier metallization or silicided contacts. The inhomogeneous filament structure that forms in the presence of RBM makes zapping unreliable -- programming current nearly doubles and some lots fail entirely.

### Nonvolatile Memory (NVM)

EPROM or EEPROM cells store trim codes digitally. MOS transistors controlled by the NVM cells short or open individual trim resistor segments.

**Advantages:**
- Very small programming currents -- suitable for post-package trimming.
- Compact cells allow many more trim bits than fuses or Zener zaps (which are limited to ~10 per design due to size).
- EEPROM allows reprogramming.

**Circuit details (Figure 6.32):** MOS transistor $M_n$ shorts each trim resistor $R_n$. On-resistance of the transistor must be much smaller than its associated trim resistor. Inverters and buffers are fed from a clean local supply to prevent noise coupling from programming lines. A noninverting buffer on one bit ensures the untrimmed value lies at mid-range rather than at one extreme -- useful for initial evaluation with untrimmed devices.

### Electrical Trimming of Polysilicon Resistors

High current density pulses ($> 10^6\,\text{A/cm}^2$) through heavily doped poly (doping $> 10^{19}\,\text{cm}^{-3}$) cause **permanent, irreversible reductions** in resistance -- down to 50% of initial value. The temperature coefficient simultaneously becomes less negative.

**Mechanism:** Localized heating at grain boundaries (which are more resistive than grain interiors) causes localized melting. As the melted silicon cools, dopants concentrate in the last portion to crystallize, reducing grain boundary resistance. Application of lower-current pulses can partially reverse the effect by thermally diffusing dopants back toward their original distribution -- but recovery is incomplete.

**Tradeoff:** One cannot adjust sheet resistance without affecting the temperature coefficient, or vice versa. This limits the technique's usefulness for constructing precise resistor ratios.

**Advantage:** Can be performed after packaging using progressively shorter pulses for arbitrary precision.

### Laser Trim

A focused laser beam alters the resistance of a thin-film resistor by localized heating that changes grain structure and homogeneity. For sichrome resistors, heating causes chromium segregation into narrow filaments isolated by more resistive material. A small void forms beneath the intact protective overcoat.

**Equipment:** Q-switched Nd:YAG lasers generating extremely short pulses at a few kHz repetition rate. Each pulse strikes a spot ~$1\,\mu\text{m}$ in diameter; sweeping produces a path with ~$0.5\,\mu\text{m}$ resolution.

**Two modes:**

1. **Continuous trimming** -- monitor resistance (or a circuit parameter that depends on it) in real time and halt the laser when the target is reached. Finer resolution, but alters the temperature coefficient because current still flows through laser-modified material. The TC change is proportional to the resistance increase but seldom exceeds $\pm 5\,\text{ppm}/\degree\text{C}$ for traditional thin-film materials.

2. **Discrete trimming** -- sever complete resistor segments in a network. Current flows only through unaltered material, so no TC change. Resolution limited by the number of segments.

**Trim geometries (Figure 6.33):**

| Style | Description |
|-------|-------------|
| **(A) Notched bar** | Laser first cuts laterally across the resistor to ~90% of target, then turns longitudinally for fine adjustment. Requires resistor width $\geq 30\text{--}50\,\mu\text{m}$. Tolerance $< \pm 0.01\%$ achievable. |
| **(B) Tophat** | Alternative continuous-trim layout; the laser cuts into the body from one side. |
| **(C) Looped** | Discrete trim: segments connected in a loop; laser severs individual links. |
| **(D) Ladder** | Discrete trim: segments in a ladder network; laser severs rungs. |

For discrete networks, segments can be as narrow as the thin-film can be patterned but must be spaced $\geq 10\,\mu\text{m}$ apart so the laser severs only one link at a time.

**Laser ablation of metal/poly links:** The laser penetrates the protective overcoat, vaporizes the material, and the pressure shatters the overcoat allowing ejection. Link width is critical: too narrow and insufficient pressure builds to rupture the overcoat; too wide and multiple shots are needed, causing splattering. Typical links: ~$1\,\mu\text{m}$ wide by $5\,\mu\text{m}$ long, with $10\text{--}15\,\mu\text{m}$ spacing from adjacent circuitry.

**Cost:** Laser trim equipment is very expensive and operates slowly (10--30 mm/s cut rate). Light may disturb circuit operation, requiring several-ms settling delays after each cut. Extensive trimming can add several seconds to final test. Used only for high-precision products that absolutely require it.

---

## Diagrams

### Figure 6.23 -- Sliding Contacts for Tweaking Resistors
![[diagrams/ch06-adjusting-resistors-fig1.png]]
Two styles of sliding contacts: (A) without heads -- the contact slides along the extended resistor body, covered by an elongated metal plate; (B) with heads -- the head material must have the same sheet resistance as the body. The "range of slide" shows the extent of possible contact positions.

### Figure 6.26/6.27 -- Fuse Layouts and Binary-Weighted Trim Networks
![[diagrams/ch06-adjusting-resistors-fig2.png]]
Top: Metal fuse (A) with constricted neck, nitride opening, and trimpads; Poly fuse (B) with minimum-width poly strip, enlarged contact heads, and optional nitride opening. Bottom (Figure 6.27): Series-connected (A) and parallel-connected (B) binary-weighted trim schemes showing how multiple fuses share trimpads.

### Figure 6.33 -- Laser Trim Geometries
![[diagrams/ch06-adjusting-resistors-fig3.png]]
Four laser-trimming schemes for thin-film resistors: (A) notched bar -- lateral cut then longitudinal for fine resolution; (B) tophat -- continuous trim from one side; (C) looped layout for discrete trimming; (D) ladder layout for discrete trimming. Heavy black lines show the laser beam path.

---

## Practical Takeaways

- **Design for tweakability from the start.** Properly designed tweakable resistors can be adjusted with a single mask change, drastically reducing iteration cost and time. Always leave room for sliding contacts, sliding heads, or trombone extensions.
- **Place the sliding contact at the midpoint** of its travel range initially -- this maximizes adjustment range in both directions.
- **Sliding contacts only work** when head and body share the same sheet resistance. For high-sheet resistors with low-sheet heads, use sliding heads instead.
- **Trombone slides** require that enclosing geometries (tank, well, implant block, silicide block) also cover the slide extension area.
- **Metal options** can combine series and parallel segment connections for more adjustment values than the number of spare segments would suggest.
- **Poly fuses are preferred over aluminum** for trimming -- lower programming currents, and sub-$0.8\,\mu\text{m}$ fuses do not require openings in the protective overcoat.
- **Fast rise times are critical** for fuse programming: $< 25\,\text{ns}$ for poly, $< 1\,\mu\text{s}$ for metal. Slow pulses cause cracking (poly) or regrowth.
- **Place fuses on the least-vulnerable end** of a resistor to protect sensitive transistor junctions from programming transients.
- **Minimize trimpads** by connecting fuses in series or parallel to share pads. Trimpads consume significant die area.
- **Zener zaps cannot be used** with refractory barrier metallization or silicided contacts -- the filament formation becomes unreliable.
- **Differential trimming** solves the problem of constructing physically tiny LSB resistors -- use two larger resistors in parallel across the fuse instead.
- **Remote trim with CMOS switches** eliminates long high-current leads across the die; scale transistor widths inversely with segment resistance for uniform percentage error.
- **Look-ahead + binary search** eliminates the need to precompute trim codes and provides the fastest, most reliable trimming algorithm.
- **Nonvolatile memory trim** is the most flexible approach (many bits, small size, post-package programmable, reprogrammable with EEPROM), but requires CMOS process capability.
- **Electrical poly trimming** can adjust values after packaging but changes the temperature coefficient -- unsuitable for precise ratio matching.
- **Laser trimming** provides the highest precision ($< \pm 0.01\%$) but is expensive and slow. Use the notched-bar (L-cut) geometry for continuous trimming of wide resistors; use looped/ladder networks for discrete trimming.
- **Continuous laser trim alters the TC;** discrete laser trim does not. Choose accordingly based on TC sensitivity requirements.
- **Practical trim accuracy limits:** $\pm 0.1\%$ for diffused/poly resistors; $\pm 0.05\%$ for thin-film resistors. These limits arise from thermal gradients, mechanical stress, and long-term drift -- not from the trim technique itself.

---

## Relation to the Bigger Picture

Section 6.6 completes the resistor chapter by addressing the gap between as-fabricated resistor tolerances (typically $\pm 10\text{--}30\%$ for sheet resistance, plus matching errors) and the sub-percent precision demanded by real analog circuits like bandgap references, DACs, and precision current sources. The tweaking and trimming techniques described here work in conjunction with the matching strategies, layout practices, and parasitic management covered in [[ch06-resistor-parasitics]] and the earlier sections of Chapter 6. Understanding these adjustment methods is also essential context for Chapter 12's discussion of programmable devices and Chapter 5's treatment of electrical overstress, since fuse blowing and Zener zapping push devices far beyond their normal operating limits. Ultimately, the choice of adjustment technique feeds directly into die area, test time, and manufacturing cost -- making it a critical layout-level decision, not merely a circuit design afterthought.

## See Also
- [[ch06-resistor-parasitics]]
