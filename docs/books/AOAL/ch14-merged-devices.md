---
title: "14.1 Merged Devices"
chapter: 14
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-14, merged-devices, minority-carrier, latchup, guard-rings]
---

# 14.1 Merged Devices

> **Chapter 14: Special Topics**

## Key Concepts

Device merging is the practice of placing two or more circuit components into a **shared tank or well** to save die area and simplify interconnection. While merging can dramatically reduce layout size -- sometimes by 50% or more -- it introduces risks that stem from the shared semiconductor region connecting the merged devices in unintended ways. The three principal failure mechanisms of merged devices are:

1. **Minority carrier injection** -- When a PN junction in one device forward-biases into the shared region, it injects minority carriers that can diffuse to other devices. If those carriers reach the base of a bipolar transistor, they provide unwanted base drive and can trigger **latchup** through parasitic PNPN (SCR) structures.

2. **Debiasing** -- Current flowing through the finite resistance of a shared tank or well creates voltage drops ($IR$ drops) that can shift the local potential away from the intended bias. This can inadvertently forward-bias junctions that should remain reverse-biased, enabling minority carrier injection where none was expected.

3. **Capacitive coupling** -- Devices sharing a tank or well are capacitively coupled through the shared region. Fluctuations in tank voltage caused by one device's switching current can inject signals into other devices. This is especially problematic when noisy and noise-sensitive devices share a region.

The key insight is that these three mechanisms often **interact synergistically**: debiasing can trigger minority carrier injection, which provides base drive that increases current, which causes more debiasing -- a positive feedback loop that leads to latchup.

### The Latchup Condition for Merged Devices

When two devices are merged such that a parasitic PNP and NPN form a PNPN structure, latchup occurs when the loop gain exceeds unity. With a blocking bar inserted between the devices, the condition becomes:

$$\beta_{PNP} \cdot \beta_{NPN} \cdot \gamma_{bar} \geq 1 \quad \text{[14.2]}$$

where $\gamma_{bar}$ represents the fraction of minority carriers that pass through (are *not* blocked by) the bar. For a standard bipolar NPN with $\beta_{NPN} > 300$ and a parasitic lateral PNP with $\beta_{PNP} > 10$, the bar must block at least $1 - \frac{1}{3000} \approx 99.97\%$ of carriers. There is **no certainty** that a P-bar or N-bar can achieve this efficiency, making such mergers inherently risky.

## Problematic Device Mergers

### NPN Transistor Merged with Base Resistor (Figure 14.6)

This is a particularly instructive failure case because it combines **both** debiasing and minority carrier injection in a positive feedback loop:

- An NPN transistor $Q_1$ (configured as an emitter follower) shares a tank with a base resistor $R_1$
- The same contact serves as both the collector contact of $Q_1$ and the tank contact of $R_1$
- Normally, both the base-collector junction of $Q_1$ and the base-tank junction of $R_1$ remain reverse-biased
- **Failure mechanism**: If $Q_1$ draws enough current through the shared tank contact, the voltage drop between the tank contact and the intrinsic collector becomes large enough to forward-bias the positive end of $R_1$ into the tank
- The injected minority carriers from $R_1$ reach the base of $Q_1$, providing additional base drive
- $Q_1$ now pulls even more current through the shared tank contact, causing more debiasing -- **positive feedback causes latchup**

The positive end of $R_1$ acts as the emitter of a parasitic PNP, the tank forms its base, and the base of the NPN acts as its collector.

**Critical threshold**: Debiasing of about $600 \text{ mV}$ at $150\degree\text{C}$ triggers latchup. If the NPN conducts $100 \,\mu\text{A}$, the tank resistance must equal $6 \text{ k}\Omega$ to produce $600 \text{ mV}$ of debiasing. Without a deep-N+ sinker, vertical tank resistance can reach hundreds or thousands of ohms. Even a **minimum plug of deep-N+** reduces tank resistance to no more than $\sim 100 \,\Omega$, making latchup unlikely.

### NPN Transistor Merged with Schottky Diode (Figure 14.7)

- The collector of the NPN connects to the cathode of the Schottky diode through the shared tank
- Many Schottky diodes have a P-type **field-relief guard ring** that begins injecting minority carriers when the voltage across the Schottky exceeds the guard ring's PN junction forward voltage
- Series resistance of small Schottky diodes can be hundreds to thousands of ohms, so guard rings are **easily debiased into conduction**
- Even **field-plated** Schottky diodes (without guard rings) inject minority carriers at ~0.5% of majority-carrier current through the Schottky barrier itself -- enough to cause latchup if $\beta_{NPN} > 200$

A related problem occurs in ordinary NPN transistors: if a heavily doped diffusion does not entirely enclose the collector contact, the exposed portion of lightly doped N-epi forms an unintended Schottky diode that can inject minority carriers into the base.

## Successful Device Mergers (Section 14.1.2)

### Darlington Pair (Figure 14.8)

The merged Darlington is a classic example of a **safe and beneficial merger**:

- Power NPN $Q_2$ and predrive NPN $Q_1$ share a common collector (tank)
- Each transistor has an associated base turnoff resistor (high-sheet resistor)
- Tank contact: a bar of deep-N+ along one side of the tank (full deep-N+ rings are only needed for saturating NPNs and large power devices)
- **Why it works**: $Q_1$ is unlikely to saturate because its $V_{CE}$ cannot drop below $V_{BE(Q_2)} + V_{CE(sat,Q_1)}$, which is about $1 \text{ V}$ at high current levels. The deep-N+ bar exhibits no more than $10$-$15 \,\Omega$ of vertical resistance, handling several hundred milliamps without $Q_1$ saturating.

**Even if $Q_1$ does saturate**, the consequences are benign:
- Holes injected into the common tank are collected by $R_1$, $R_2$, $Q_2$, or the substrate
- Holes collected by $R_1$ return to the base of $Q_1$ (from which they originated) or flow to the base of $Q_2$ (useful base drive)
- Holes collected by $R_2$ add to the base drive of $Q_2$ or flow harmlessly into its emitter lead
- Substrate collection is safe as long as it stays below a few milliamps

**Layout details**:
- Base contacts of each NPN also serve as contact heads for their respective base turnoff resistors (saves area)
- HSR implant must be spaced far enough from emitter to prevent intersection -- achieved by running the HSR behind the base contact
- Layout arranged for single-level metal interconnection

### Merged Op-Amp Input Stage (Figure 14.9)

A differential pair ($Q_1$/$Q_2$) with split-collector lateral PNPs, current mirror, and substrate PNP emitter followers -- all merged in a compact layout:

- Split-collector lateral PNPs: larger collectors run into isolation to save space; narrower tanks used with NBL outdiffusion preventing minority carrier loss
- The critical mergers of NPN transistors with PNP emitter followers use **lateral PNPs instead of substrate PNPs**. The lateral PNP collectors function as P-bars, blocking minority carrier flow
- The circuit inherently **tolerates low levels of cross-injection** because balanced devices ($Q_5$ and $Q_6$) experience equal amounts of hole collection, maintaining offset balance

## Low-Risk Device Mergers (Section 14.1.3)

These mergers should be used **whenever possible** because benefits greatly outweigh risks:

| Merger Type | Key Details |
|---|---|
| **Multiple sections of matched devices** | Sections in common tank; more compact = less gradient sensitivity. Emitters must be spaced to avoid depletion region merging and allow minority carrier diffusion through quasineutral base. |
| **Power device fingers** | Multiple emitter/source fingers in shared tank/well. May interdigitate deep-N+ strips between banks for heat distribution and reduced collector resistance. |
| **Sense + power transistors** | Sense FET ideally split into two sections on axis of symmetry, halfway between center and periphery of power device, to match average temperature. Single center placement is less desirable (hottest spot). |
| **Back-to-back LDMOS** | Common drain LDMOS transistors: source/backgate fingers interdigitated, drain fingers eliminated entirely. |
| **Schottky-clamped NPN** | Schottky clamp shares deep-N+ sinker with NPN; can use NPN base diffusion extension as guard ring (Section 11.1.4). |
| **Multi-emitter NPN ($I^2L$)** | Inverse-active mode NPN transistors for integrated injection logic. Poor matching but sufficient for digital logic. |
| **Darlington NPN** | Power Darlingtons with predrive merged in same tank/well. |
| **Base turnoff resistors** | P-type diffused resistors in same tank as NPN. If connected to substrate, one end can run into isolation to save one resistor head area. |

## Medium-Risk Device Mergers (Section 14.1.4)

These offer substantial area savings but require careful attention:

### MOS Transistors in Common Backgate

- **Risk**: Output transistors (source/drain connected to pins) can inject minority carriers when transients pull NSD below or PSD above backgate potential
- **Mitigation**: Place output transistors in own backgate regions, or use minority-carrier guard rings (see [[ch14-guard-rings]]) and low-resistance backgate contacts
- Transistors with **deliberately forward-biased** backgate junctions (charge pumps, clamps) must be clearly marked and isolated
- Always provide **low-resistance backgate connections** to minimize debiasing and latchup risk

### Diffused Resistors in Common Tanks

- Safe as long as no resistor forward-biases into the shared tank
- Pin-connected resistors should have their own tanks and may need guard rings
- **Capacitive coupling** between resistors is a concern: do not merge sensitive resistors with noisy ones

### Lateral PNP Transistors (Common Base Connection)

- Collectors of unsaturated lateral PNPs act as natural P-bars, isolating transistors from each other
- **Risk**: If any transistor saturates, cross-injection occurs. P-bars and N-bars (Section 5.4.4) only partially block it
- Saturating transistors must go in their own tanks

### Split-Collector Lateral PNP

- Single split-collector device replaces several ordinary lateral PNPs
- Saturation of any collector increases currents in remaining collectors
- **No way to block cross-injection** between split collectors -- must divide into separate devices if any collector saturates

### Zener Diodes

- Emitter-base Zeners can merge with other components if tank voltage always $\geq$ Zener cathode voltage (prevents parasitic NPN conduction)

### Bipolar Transistors with Shared Collectors

- Multiple NPN in common tank works if none saturate from collector debiasing
- Add deep-N+ plug beneath tank contact to prevent saturation
- Capacitive coupling remains a concern for fluctuating collector currents

## Devising New Device Mergers (Section 14.1.5)

Before implementing any new merger, answer these three questions:

1. **Can any merged device inject minority carriers into the shared region?**
   - Sources: saturating BJTs, forward-biased Schottky diodes, diffusions connected to pins
   - Merging NPN with minority-carrier injectors creates PNPN structures prone to latchup

2. **Can any merged device debias the shared region?**
   - Devices drawing significant current through shared regions cause $IR$ drops
   - Debiasing triggers injection and couples noise
   - Mitigation: reduce tank/well resistance (e.g., deep-N+ plug handles tens of milliamps)

3. **Can noise coupling upset the circuit?**
   - Noisy + sensitive devices in shared region = capacitive coupling degradation
   - Especially problematic without solid low-resistance connection to shared region (e.g., merged NMOS in P-epi with high backgate resistance)

## Diagrams

### Figure 14.6/14.7 -- Problematic Mergers

![[diagrams/ch14-merged-devices-fig1.png]]

**Top**: NPN transistor $Q_1$ merged with base resistor $R_1$ in a common tank. The shared tank contact creates a debiasing path that can forward-bias $R_1$ into the tank, triggering a latchup loop. **Bottom**: NPN transistor $Q_1$ merged with Schottky diode $D_1$. The field-relief guard ring or even the Schottky barrier itself can inject minority carriers into the shared tank.

### Figure 14.8 -- Successful Darlington Pair Merger

![[diagrams/ch14-merged-devices-fig2.png]]

Schematic (A) and standard bipolar layout (B) of a merged Darlington pair. Metal is omitted for clarity. Note the deep-N+ bar along the left side for tank contact, and the base turnoff resistors sharing contact heads with the NPN base contacts. The layout is arranged for single-level metal interconnection.

### Figure 14.11 -- Hole Guard Rings

![[diagrams/ch14-merged-devices-fig3.png]]

Cross-sections of hole guard rings for standard bipolar. **(A)** Hole-collecting guard ring (HCGR): a reverse-biased base diffusion intercepts holes before they reach tank sidewalls; NBL blocks downward flow. **(B)** Hole-blocking guard ring (HBGR): deep-N+ and NBL form a heavily doped N-type barrier that generates an electric field repelling holes and forcing recombination. The HBGR must completely encircle the injector. Combined HCGR inside HBGR achieves >99% efficiency.

## Practical Takeaways

- **Always include deep-N+ sinkers** (even minimum plugs) for NPN transistors in shared tanks. This reduces vertical tank resistance from thousands of ohms to ~$100 \,\Omega$, preventing debiasing-induced latchup.
- **Never merge a saturating NPN with a lateral PNP** that drives it -- the $\beta_{PNP} \cdot \beta_{NPN}$ product almost certainly exceeds unity, and P-bars cannot reliably achieve 99.97% blocking efficiency.
- **Output transistors** (connected to external pins) should preferably have their **own backgate regions** due to transient-induced minority carrier injection risk.
- **Darlington pairs** are safe to merge because even under saturation, injected holes are either returned to their source or provide useful base drive to the power transistor.
- When merging matched devices, the **compactness of the merged structure reduces gradient sensitivity**, making it superior to multiple separated devices.
- For sense transistors merged with power devices, use **two sense sections on the axis of symmetry**, each halfway between center and periphery, to match average temperature. A single centered sense transistor sits at the hottest point and gives biased readings.
- **Capacitive coupling** through shared tanks is often overlooked -- never merge noisy devices (switching outputs, high-current collectors) with noise-sensitive circuitry in the same tank.
- Use **mock layouts** (rough sketches) to test routing before committing to a merged arrangement, especially with single-level metal constraints.
- Standard bipolar designers have historically used mergers extensively. **CMOS/BiCMOS designers** primarily merge MOS transistors with shared backgate contacts but generally avoid other mergers because area savings are smaller.

## Relation to the Bigger Picture

Merged devices represent the intersection of several critical topics covered earlier in the book: minority carrier injection (Chapter 5), device physics of BJTs and MOSFETs (Chapters 9-11), and matching (Chapter 8, 10). The merger techniques described here are the practical application of understanding parasitic structures -- knowing which junctions can forward-bias, where minority carriers will flow, and how shared regions create unintended coupling. This section also sets up the immediately following discussion of minority-carrier guard rings (Section 14.2), which are the primary defense mechanism when mergers or external transients cause injection. Mastering merged device layout is essential for achieving competitive die area in bipolar and BiCMOS analog ICs while avoiding the catastrophic failure mode of latchup.

## See Also
- [[ch14-guard-rings]]
- [[ch05-minority-carrier-injection]]
- [[ch09-standard-bipolar-transistors]]
- [[ch10-power-bjts]]
- [[ch11-diodes-standard-bipolar]]
- [[ch08-mismatch-causes]]
