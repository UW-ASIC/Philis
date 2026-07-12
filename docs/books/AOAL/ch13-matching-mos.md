---
title: "13.2-13.3 Matching MOS Transistors"
chapter: 13
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-13, matching, mos-transistors, common-centroid, mismatch, pelgrom]
---

# 13.2-13.3 Matching MOS Transistors

> **Chapter 13: Applications of MOS Transistors**

## Key Concepts

Matching MOS transistors is critical for analog IC performance --- current mirrors, differential pairs, and comparators all depend on two or more transistors behaving identically. This section systematically dissects every mechanism that can cause mismatch between MOS devices and provides quantitative layout rules to combat each one.

The overarching theme is that mismatch has three distinct origins:

1. **Random mismatch** --- statistical variations in dopant placement, oxide thickness, and interface traps that obey Pelgrom's law and scale as $1/\sqrt{WL}$.
2. **Systematic mismatch** --- deterministic errors introduced by etch-rate differences, stress, orientation, metallization patterns, and other layout-dependent effects.
3. **Bias-dependent mismatch** --- time-varying errors that accumulate due to hot-carrier injection, channel-length modulation, and bias-temperature instability (BTI).

A well-matched layout must address all three simultaneously. The section culminates in a set of 24 comprehensive matching rules (Section 13.3) that codify decades of practical experience.

---

## 13.2.1 Geometry Effects

### Pelgrom's Law Violations in Submicron Devices

Standard Pelgrom's law predicts threshold voltage mismatch as:

$$\sigma(\Delta V_{th}) = \frac{A_{VT}}{\sqrt{WL}}$$

However, **submicron devices** with short-channel and narrow-channel effects violate this law --- their mismatches exceed predictions. The fix is straightforward: make both $W$ and $L$ substantially greater than minimum for matched transistors.

### Pocket Implant Devices

Transistors with **pocket (halo) implants** obey a *modified* Pelgrom's law:

$$\sigma(\Delta V_{th}) = \frac{A_{VT}}{\sqrt{W \cdot \min(L, L_C)}}$$

where $L_C$ is a critical channel length (typically 1--2 $\mu$m). Once $L > L_C$, further lengthening does not improve matching. This happens because the pocket implant doping exceeds the backgate doping between them. The device behaves like a series stack of three transistors; for $L > L_C$, drain-induced threshold shift (DITS) causes the drain-end pocket transistor to dominate, making matching depend on $L_C$ rather than $L$.

**Consequences:**
- Pocket-implant devices are **poorly suited for current matching** because neither increasing $W$ nor increasing $L$ beyond $L_C$ helps.
- Voltage-matched transistors can simply use $L \leq L_C$ and increase $W$ for area.
- Analog processes often provide alternative "analog-friendly" devices that omit pocket implants.

### Transconductance Matching

Device transconductance also obeys Pelgrom's law:

$$\sigma(\Delta g_m) = \frac{A_{\beta}}{\sqrt{WL}}$$

where $A_{\beta}$ is the transconductance mismatch constant. For submicron devices, the full form includes peripheral mismatch terms:

$$\sigma(\Delta g_m) = \sqrt{\frac{A_{\beta,area}^2}{WL} + \frac{A_{\beta,W}^2}{W^2 L} + \frac{A_{\beta,L}^2}{WL^2}}$$

The peripheral terms ($A_{\beta,W}$ and $A_{\beta,L}$) are only significant for devices with submicron dimensions.

### Orientation

MOS transconductance depends on carrier mobility, which is **stress-sensitive and orientation-dependent**. Transistors oriented along different crystal axes exhibit different transconductances under mechanical stress:

- Devices oriented in the **same direction** (Figure 13.43A) match far better than those oriented in different directions.
- Stress-induced mobility variations can cause **several percent** current mismatch between rotated devices, and up to **5%** on tilted wafers.
- Editing errors (e.g., rotating a cell containing one of two matched transistors) silently introduce orientation mismatches. The fix: always group matched devices in the same cell.

For **non-self-aligned devices** (e.g., asymmetric drain-extended NMOS), matched transistors must be **superimposable** --- achievable by mere translation, without rotation or reflection. Mirror-image devices will have opposite sensitivity to photolithographic misalignment.

---

## 13.2.2 Diffusion, Implantation, Etch, and Trench Effects

### Polysilicon Etch Rate Variation (Microloading)

Etch rates depend on surrounding geometry. In a transistor array, the gate of an interior transistor is flanked by other gates on both sides, while end transistors face open space on one side. The end gates etch faster, becoming slightly shorter.

**Solution: Dummy transistors (or dummy gates).**

- **Full dummies** (Figure 13.45B): electrically inactive transistors placed at each end of the array so that every active gate sees identical poly geometry on both sides. Dummy gates should be connected (typically to backgate potential) to keep them in cutoff.
- **Half dummies** (Figure 13.45C): have only a single source/drain termination; field oxide intrudes partway into the poly. Acceptable for less critical matching with $L > 1\,\mu$m. Minimum poly width of half dummies should be at least $\sim 1\,\mu$m.
- Microloading effects decrease exponentially with distance and are negligible beyond $\sim 1\,\mu$m for most devices.
- For extremely accurate matching, the regular poly pattern should extend $\sim 3\,\mu$m beyond the last active gate.
- **Poly combs** (interconnecting multiple gate fingers with a bar of polysilicon) are acceptable if the interconnecting poly lies at least $1\,\mu$m from the moat.

### Corner Rounding

Poly corners round during fabrication due to optical diffraction and etch variation. Effects extend $\sim 0.5\,\mu$m. Mitigation: extend all poly gates (including dummies) at least $0.5\,\mu$m beyond the moat, and ensure all gates have identical extensions.

### CMP Dummy Poly

Modern processes use CMP planarization with minimum-density rules. Pattern generation decks insert **dummy poly** geometries that should not be confused with dummy gates. For accurate matching, **block dummy poly generation** within $\sim 3\,\mu$m of matched transistors.

### Photoresist Disturbance from Topography

In processes with poly-poly capacitors (poly-0 and poly-1), the poly-0 geometries disturb photoresist flow during spinning, causing mismatches up to **12%** at distances over $200\,\mu$m. Matched transistors should be placed at least $500\,\mu$m from poly-0 geometries.

### Diffusion Penetration of Polysilicon

Gate polysilicon is deposited intrinsic and then doped by implantation + anneal. Dopants diffuse along grain boundaries first:
- **Under-annealing**: incomplete doping causes excessive $V_{th}$ variation due to partial depletion.
- **Over-annealing**: dopant penetrates the gate oxide near grain boundaries, again introducing mismatch.
The process window must be carefully controlled; this is a fabrication concern rather than a layout concern.

### Diffusions Near the Channel

Deep diffusions (e.g., N+ sinker in BiCMOS) have tails that extend well beyond their metallurgical junctions. These tails shift $V_{th}$ and alter $g_m$ of nearby transistors. **Increase minimum spacings** from layout rules by several microns.

**Wells** are also deep diffusions:
- N-well boundaries should not be placed near matched NMOS transistors.
- Matched PMOS transistors should be placed well inside their enclosing N-well, several microns beyond the minimum design rule spacing.

### Well Proximity Effect (WPE)

Retrograde wells use high-energy (megavolt) ion implants. Ions deflected from adjacent photoresist scatter laterally into the exposed silicon, increasing doping near the well edge. Effects have been observed up to $5\,\mu$m from the well edge:

- At $0.5\,\mu$m spacing from well edge: **5% current mismatch**.
- At $0.25\,\mu$m spacing: **25% current mismatch**.

**Fixes:**
- Keep well edges at least $5\,\mu$m from active gate regions.
- Use dummy gates whose moat regions push the well edge away from active devices (Figure 13.51A).
- Displace well edges further into field oxide along the sides of the transistor (Figure 13.51B).
- Use wider transistors (less of the channel lies within the enhanced-doping zone).

### NBL Shadow

In BiCMOS with deep N-wells over NBL, the NBL shadow (surface discontinuity used for alignment) can shift laterally due to off-axis wafer cuts. If it intersects the active gate of a PMOS transistor, it causes mismatch. Pattern shift typically does not exceed 150% of epi thickness. STI processes eliminate this issue (CMP removes surface discontinuities).

### First-Finger Effect (LDMOS)

In multi-finger LDMOS transistors, the outermost fingers differ from inner fingers by up to **10%** due to implant scattering and microloading. Fix: add dummy fingers so the senseFET is never an outermost finger.

### Length of Diffusion (LOD) Effect

STI generates **compressive stress** in adjacent silicon (oxide volume is ~2.2x the silicon it replaces). This stress:
- Decreases NMOS transconductance and increases PMOS transconductance by **10% or more**.
- Causes threshold voltage shifts of **10 mV or more**.
- Diminishes rapidly with distance from STI sidewalls.

Widening source/drain terminations pushes STI away from the channel. End devices in an array experience greater traverse stress.

**Fixes:**
- Stretch moat regions at either end of the array by $\sim 3\,\mu$m (Figure 13.52A).
- Use dummy transistors with wide moat regions (Figure 13.52B).
- For optimal matching, moat should extend at least $5\,\mu$m beyond the last active transistor.

### Stringers and Subthreshold Humps

In some low-voltage CMOS, the channel region abutting field oxide has a slightly lower $V_{th}$ than the rest of the device --- these parasitic edge transistors are called **stringers**. Stringers cause:
- Minimal mismatch when $V_{GS} - V_{th} > 100\,$mV (saturation).
- Dramatically increased mismatch in **subthreshold** operation.
- A characteristic **subthreshold hump** on the $\log(I_D)$ vs. $V_{GS}$ plot.

**Fixes:**
- Operate matched transistors at $V_{eff} = V_{GS} - V_{th} \geq 100\,$mV.
- Use **poly fins** that extend the gate beyond the moat, breaking the stringer channel path (Figure 13.53B).
- Use **annular transistors** (grid gates), which inherently avoid stringers.

### PMOS vs. NMOS Matching

PMOS transistors often exhibit **30--50% more transconductance mismatch** than comparable NMOS devices. However, this is process-dependent --- in some processes NMOS is worse. Always consult actual matching data for the target process.

---

## 13.2.3 Bias-Dependent Mismatches

### Channel-Length Modulation

Causes severe mismatch between short-channel transistors at different $V_{DS}$. The ratio of drain currents is:

$$\frac{I_{D1}}{I_{D2}} \approx 1 + \lambda (V_{DS1} - V_{DS2})$$

where the channel-length modulation factor $\lambda$ roughly equals:

$$\lambda \approx \frac{k}{\sqrt{N_B} \cdot L}$$

with $N_B$ the backgate doping and $k$ a constant ($\approx 0.01$). The error scales inversely with $L$. Lower-voltage processes with heavier doping permit shorter mirror lengths (from $\sim 20\,\mu$m at 20V down to $\sim 2\,\mu$m at 2V).

**Mitigation:** Increase $L$, minimize successive mirror stages, and use **cascodes** to equalize $V_{DS}$.

### Hot-Carrier Generation

**Impact ionization** increases drain current at high $V_{DS}$ --- NMOS is more affected than PMOS. Creates systematic mismatch between devices at different $V_{DS}$.

**Hot-carrier injection** gradually shifts $V_{th}$ (toward less positive / more negative) and degrades transconductance through dehydrogenation of interface traps. Threshold shifts of tens of millivolts can accumulate. NMOS experiences larger shifts than PMOS.

**Solution:** Use **cascodes** to reduce $V_{DS}$ of matched transistors (Figure 13.54). Connect cascode backgates to their sources to ensure impact ionization current does not leak into other paths. The cascode sizing condition is:

$$V_{DS,matched} = V_{GS,cascode} - V_{th,cascode}$$

### Bias-Temperature Instability (BTI)

- **NBTI** (negative-bias temperature instability): Affects PMOS transistors operating at negative $V_{GS}$. Shifts $V_{th}$ toward more-negative values and degrades $g_m$. Driven by dehydrogenation of interface traps by holes. Thinner gate oxides suffer more.
- **PBTI** (positive-bias temperature instability): Usually much smaller than NBTI, but can be significant in oxynitride/ONO dielectrics.

Unlike hot-carrier injection, BTI accumulates **even without drain current flowing** --- only $V_{GS}$ bias matters. This is critical for PMOS differential pairs in comparators where inputs are unbalanced.

**Mitigation:** Clamp diodes, substitution of NMOS for PMOS differential pairs where possible.

---

## 13.2.4 Hydrogenation

Hydrogen annealing passivates interface trap sites ($P_b$ and $P_{b1}$ dangling bonds), reducing random $V_{th}$ variation. Hydrogen comes from polysilicon deposition and PECVD compressive nitride overcoats.

**The problem:** Metal layers (Al, Cu) block hydrogen diffusion. Ti-silicide and TiW barrier metals actively **getter** (absorb) hydrogen. This means:

- Transistors beneath metal receive less hydrogen and have **more random $V_{th}$ variation**.
- Systematic $I_D$ mismatches of up to **20%** have been observed between transistors covered by metal and those without.
- Even transistors merely *surrounded* by different metal fill patterns show mismatches up to **1%**.
- Metal effects extend to distances of **10 $\mu$m or more**.

**Layout rules for hydrogenation:**
1. Do **not** place metallization above active gate regions of critical matched transistors.
2. Minimize metal adjacent to matched devices.
3. Match the metal pattern surrounding each transistor as closely as possible.
4. A less optimal but workable alternative: cover *both* transistors' active areas with a metal field plate, then route higher metals freely above.
5. **Block dummy metal generation** over matched transistors using pseudolayers. Manually ensure metal density rules are met within blocked regions.
6. Surround critically matched transistors with custom-crafted dummy metal patterns identical for each device, extending several microns (up to $10\,\mu$m for exceptional matching).

---

## 13.2.5 Gradients

Any device parameter or operating condition that varies linearly across the die creates a **gradient-induced mismatch** proportional to the gradient magnitude times the distance between device centroids:

$$\Delta P = \alpha_P \cdot P_0 \cdot \nabla T \cdot d$$

where $\alpha_P$ is the temperature coefficient of parameter $P$, $P_0$ is the nominal value, $\nabla T$ is the gradient, and $d$ is the centroid separation. The closer matched devices are, the smaller the mismatch.

### Oxide Thickness Gradients

Gate oxide thickness varies radially across the wafer. A 5% radial variation across a 200 mm wafer gives a gradient of only $\sim 0.5\,$ppm$/\mu$m --- negligible for properly laid-out devices.

### Stress Gradients

Mechanical stress causes transconductance variation (not threshold shifts). For a transistor with unstressed transconductance $g_{m0}$:

$$\frac{\Delta g_m}{g_{m0}} = \pi_L \sigma_L + \pi_T \sigma_T + \pi_S \tau_S$$

where $\pi_L$, $\pi_T$, $\pi_S$ are piezoresistance coefficients for longitudinal, transverse, and shear stress respectively.

Key crystallographic dependencies on (100) silicon:
- **NMOS** (in $\langle 110 \rangle$ direction): $\pi_L \approx 30 \times 10^{-11}\,$Pa$^{-1}$, $\pi_T \approx -17 \times 10^{-11}\,$Pa$^{-1}$. Minimum stress sensitivity when channels run along $\langle 100 \rangle$ (i.e., horizontal/vertical in standard die orientation).
- **PMOS** (in $\langle 110 \rangle$ direction): $\pi_L \approx -65 \times 10^{-11}\,$Pa$^{-1}$, $\pi_T \approx 40 \times 10^{-11}\,$Pa$^{-1}$. Minimum stress sensitivity when channels run along $\langle 100 \rangle$, which is **diagonal** (45 degrees) to standard layout orientation.

Higher gate voltages reduce NMOS piezoresistance coefficients by up to 50%.

**Placement guidance:**
- Place matched transistors near the die center (lowest stress gradients in plastic packages).
- Avoid edges and corners.
- Prefer placement on die axes of symmetry.
- Common-centroid layouts are strongly recommended.

### Temperature Gradients

MOS threshold voltages decrease with temperature at roughly $-2\,$mV/K (similar to bipolar $V_{BE}$). For **bipolar** circuits, trimming the input offset at one temperature also corrects it over temperature (because mismatch is PTAT). For **MOS** circuits, the offset contains both $V_{th}$-dependent and $g_m$-dependent terms. Since $g_m$ is itself temperature-dependent, trimming MOS offset at one temperature only removes about **two-thirds** of the over-temperature variation.

The input offset voltage of a MOS differential pair is:

$$\sigma(V_{os}) = \sqrt{\sigma^2(\Delta V_{th,input}) + \left(\frac{\sigma(\Delta I_{D,mirror})}{2 g_m}\right)^2 + \sigma^2(\Delta V_{th,mirror})/4 + \left(\frac{\sigma(\Delta I_{D,mirror})}{2 g_m}\right)^2}$$

To improve trimmability, maximize $g_m$ and minimize current mismatch so the $V_{th}$ term dominates.

---

## 13.2.6 Common-Centroid Layout of MOS Transistors

### Centroid and Gradient Cancellation

The **centroid** of a device is the weighted geometric center of its active area. Common-centroid layouts place the centroids of matched devices at the same point, canceling **linear** gradient-induced mismatches. In practice, gradients have nonlinear (quadratic) components, so the layout must also be **compact** to minimize residual mismatch.

### Interdigitation

MOS transistors are divided into **sections (fingers)** arranged in compact arrays. An ABBA interdigitation pattern (Figure 13.57) aligns centroids along the midpoint of the array. The subscript notation ($S$ for source, $D$ for drain) specifies source/drain placement:

$$D_S A_D B_S B_D A_S D$$

### Orientation (Formally Defined)

Each section has an orientation value: $+1$ if current flows right, $-1$ if current flows left. The orientation of a multi-section transistor is:

$$\chi = \frac{1}{N} \sum_{i=1}^{N} \chi_i$$

Two transistors must have **equal orientation** (same magnitude and sign) to avoid orientation-dependent mismatch. Zero orientation (equal numbers of left and right sections) is preferred.

For two-dimensional arrays, both horizontal orientation $\chi_H$ and vertical orientation $\chi_V$ must match.

### The Five Rules of Common-Centroid Layout (Table 13.2)

| Rule | Requirement |
|------|-------------|
| **1. COINCIDENCE** | Centroids of matched transistors should coincide (ideally exactly). |
| **2. SYMMETRY** | Array should be symmetric around both horizontal and vertical axes. |
| **3. DISPERSION** | Large arrays should be subdivided into smaller common-centroid subarrays; only subarrays need satisfy coincidence and symmetry. |
| **4. COMPACTNESS** | Each array/subarray should be as compact as possible. |
| **5. ORIENTATION** | Matched transistors must have equal orientation $\chi$. |

### Sample Interdigitation Patterns (Table 13.3)

For transistors with common source connections, Hastings lists standard patterns such as:

1. $D_S A_D B_S B_D A_S D$ (ABBA basic)
2. $(D_S A_D B_S B_D A_S)^n D$ (repeated ABBA)
3. And more complex patterns with dashes indicating locations where source/drain cannot merge.

If sources do not share a common node, spaces must be inserted between different devices, or **width-direction interdigitation** can be used (common for current mirrors, since they share a gate).

### Two-Dimensional Arrays (Cross-Coupled Pairs)

A two-dimensional common-centroid array achieves better matching than a one-dimensional interdigitated array. The simplest is the **cross-coupled pair** (Figure 13.59):

$$\frac{A \quad B}{B \quad A}$$

Finite-element analysis shows a 2D AB/BA array of square elements exhibits about **60%** of the residual gradient-induced mismatch of a 1D ABBA array of the same elements. Transistors with $W \approx L$ are ideal candidates.

For larger devices, sections are grouped into multiple cross-coupled subarrays:
- 4 sections each: $AABB / BBAA$
- Larger: $AABB/BBAA/AABB/BBAA$, etc.

---

## Diagrams

### Figure 13.45 -- Dummy Transistor Arrays

![[diagrams/ch13-matching-mos-fig1.png]]

**Caption:** MOS transistor arrays showing: (A) No dummies --- end transistors have asymmetric poly environment causing etch-rate mismatch. (B) Full dummies --- inactive transistors placed at each end provide symmetric poly geometry for all active gates. (C) Half dummies --- space-saving alternative with a single source/drain termination.

### Figure 13.51 -- Well Proximity Effect Fixes

![[diagrams/ch13-matching-mos-fig2.png]]

**Caption:** Two approaches to mitigating the well proximity effect: (A) Dummy gates push the well edge away from active devices via their moat regions. (B) Well boundaries are explicitly displaced further into the field oxide along the sides of the transistor array.

### Figure 13.59 -- Cross-Coupled MOS Pair (2D Common-Centroid)

![[diagrams/ch13-matching-mos-fig3.png]]

**Caption:** The simplest two-dimensional common-centroid array: a cross-coupled pair with half dummies. The AB/BA arrangement cancels both horizontal and vertical linear gradients while satisfying the orientation rule (each device has one left-oriented and one right-oriented section, giving $\chi = 0$).

---

## 13.3 Rules for MOS Transistor Matching (Complete Set)

### Matching Accuracy Tiers

| Tier | Voltage Mismatch ($6\sigma$) | Current Mismatch ($6\sigma$) |
|------|------|------|
| **Minimal** | 5--15 mV | 2--5% |
| **Moderate** | 1--3 mV | 0.5--1% |
| **Exceptional** | < 0.3 mV | < 0.1% |

(Assumes 10-year lifetime, $150\,^\circ$C junction temperature, conventional plastic packaging.)

### The 24 Rules (Condensed)

1. **Use identical sections.** Never mix different $W$/$L$ in matched devices. Voltage-match by paralleling identical sections; current-match by series-stacking.
2. **For voltage matching, use large devices.** Mismatch scales as $1/\sqrt{WL}$. Thin-oxide devices need less area.
3. **For current matching, use long devices.** Increasing $W$ at fixed $I_D$ does not help current matching; increasing $L$ does. Extraordinary lengths are needed at low currents (e.g., $240\,\mu$m for exceptional 12V NMOS at $10\,\mu$A).
4. **Avoid operating current-matched transistors in subthreshold.** Stringers dramatically increase mismatch. Maintain $V_{eff} \geq 100\,$mV.
5. **Avoid pocket-implant devices for long-channel matching.** Beyond $L_C \approx 1$--$2\,\mu$m, matching does not improve. Use analog-friendly devices without pocket implants.
6. **Prefer thin-oxide devices.** $V_{th}$ mismatch scales linearly with $t_{ox}$. Thin oxides also give higher $g_m$.
7. **Same orientation.** All matched sections must have channels running parallel. Compute $\chi$ for multi-section devices; matched transistors need equal $\chi$.
8. **Close proximity.** Reduces gradient-induced mismatch. Even minimal matching requires close placement.
9. **Compact layout.** Avoid long, spindly arrays. Voltage matching: array aspect ratio $\leq 3$:1. Current matching: up to 10:1 for minimal, 3:1 for moderate, ~1:1 for exceptional.
10. **Use 2D common-centroid layouts.** Cross-coupled pairs achieve ~60% of the residual mismatch of 1D ABBA arrays.
11. **Avoid submicron dimensions.** Peripheral effects dominate; space is wasted on source/drain terminations.
12. **Place dummies.** Minimal: optional. Moderate: full dummy for $L < 1\,\mu$m, half dummy for $L > 1\,\mu$m. Exceptional: outermost dummy poly edge at least $3\,\mu$m from nearest active gate; moat extends $5\,\mu$m beyond last active gate.
13. **Low stress-gradient locations.** Place matched transistors in the central half of the die. Avoid edges and corners (within $200\,\mu$m). On solder-bump dice, place on axes of symmetry between bumps.
14. **Distance from power devices.** Exceptional: opposite end of die from power devices, ~75% from center to far edge. Consider elongating die to 2:1 or 3:1 aspect ratio.
15. **Place on die axes of symmetry.** Stress gradients in plastic packages are symmetric about die axes.
16. **No contacts on active gate regions.** Even if design rules allow it, avoid placing contacts over matched transistor active areas.
17. **No indiscriminate metal routing over active gates.** Moderate/exceptional: no metal leads crossing active areas, no field plates. Minimal: okay if metal pattern is identical over all sections.
18. **Block dummy metal generation.** Unless a field plate lies underneath. For exceptional matching, block extends $5\,\mu$m beyond active gate area in all directions.
19. **Deep diffusion spacing.** Well boundaries at least $5\,\mu$m from exceptional matched transistors, or at least $2 \times$ well junction depth (whichever is greater). For WPE-prone processes: $1\,\mu$m for minimal, $2\,\mu$m for moderate.
20. **NBL shadow clearance.** Do not let NBL shadow cross active areas. If shift direction is unknown, allow 150% of max epi thickness on all sides. (Not an issue in STI processes.)
21. **Extend gate poly $0.5\,\mu$m beyond rules.** All gate geometries (including dummies) should extend equally. No poly combs unless interconnecting poly is $\geq 1\,\mu$m from moat.
22. **Use metal straps for gate connections.** Prevents etch-rate variations from poly geometry differences. Required for moderate and exceptional matching.
23. **Keep extraneous poly away.** Block dummy poly within $3\,\mu$m (exceptional) or $1\,\mu$m (moderate). Without dummies, keep other poly at least $1\,\mu$m away.
24. **Consider 45-degree PMOS orientation.** PMOS has minimum stress sensitivity at 45 degrees to wafer flat. Only for exceptional matching with significant mechanical stress. Not permitted if directional implants are used.

### Typical Areas and Lengths Required

**Voltage matching** (active area in $\mu$m$^2$):

| Accuracy | 12V | 5V | 3.3V | 1.8V |
|----------|-----|-----|------|------|
| Minimal | -- | -- | -- | -- |
| Moderate | 4,700 | 630 | 220 | 64 |
| Exceptional | 42,000 | 5,600 | 1,900 | 580 |

**Current matching** at $10\,\mu$A (channel length in $\mu$m, NMOS):

| Accuracy | 12V | 5V | 3.3V | 1.8V |
|----------|-----|-----|------|------|
| Minimal | 14 | 8.6 | 6.6 | 4.9 |
| Moderate | 79 | 48 | 37 | 27 |
| Exceptional | 240 | 140 | 110 | 81 |

---

## Practical Takeaways

- **Always use dummy transistors** at the ends of matched arrays. The investment in area is tiny compared to the matching improvement from eliminating etch-rate, LOD, and WPE asymmetries.
- **Hydrogenation is the hidden killer.** Metal above or near matched transistors blocks hydrogen passivation of interface traps. Block dummy metal generation and minimize metal routing over active gates.
- **Cascodes are the circuit designer's best friend for matching.** They equalize $V_{DS}$, suppress hot-carrier injection, and eliminate channel-length modulation mismatch --- all in one device.
- **Thin-oxide devices match dramatically better.** A 1.8V device needs only ~10% of the area of a 12V device for the same voltage matching accuracy.
- **Current matching is much harder than voltage matching.** At low currents with thick oxides, required channel lengths can reach hundreds of microns. Consider degeneration or alternative circuit topologies.
- **Common-centroid layout is essential for moderate and exceptional matching.** 2D cross-coupled pairs provide ~40% improvement over 1D ABBA arrays for the same total area.
- **Pocket-implant devices are analog-unfriendly.** Their modified Pelgrom scaling makes length increases futile beyond $L_C$. Use analog-specific device options if current matching is needed.
- **Subthreshold operation of matched transistors is risky** due to stringer transistors. Always verify with $\log(I_D)$ vs. $V_{GS}$ measurements before operating matched devices near subthreshold.
- **Stress gradients matter more for PMOS** ($|\pi_L| \approx 65$) than NMOS ($|\pi_L| \approx 30$). For extreme PMOS matching, consider diagonal (45-degree) orientation.
- **Well proximity effect can cause 25% mismatch** at 0.25 $\mu$m from the well edge. Keep well boundaries far from active gates.

---

## Relation to the Bigger Picture

This section is the MOS counterpart to the bipolar matching discussion in Chapter 10 and the general matching theory of [[ch08-matching-rules]]. While Chapter 8 establishes the universal principles (Pelgrom's law, common-centroid layout, gradient cancellation), this section applies them specifically to MOS transistors and adds MOS-specific concerns: pocket implants, LOD/WPE effects, hydrogenation blocking by metal, stringers, and orientation. These matching techniques are directly relevant to analog building blocks (current mirrors, differential pairs, DACs) that drive the performance of real circuits. The 24 rules given in Section 13.3 represent the single most comprehensive checklist for MOS matching in analog layout. For designers working with MOS power devices, the companion section [[ch13-power-mos]] covers the distinct concerns of high-current, high-voltage layout where matching is less critical but thermal and electromigration issues dominate.

---

## See Also
- [[ch13-power-mos]]
- [[ch08-matching-rules]]
