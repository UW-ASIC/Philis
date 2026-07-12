---
title: "4.2 Poly-Gate CMOS"
chapter: 4
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-4, CMOS, fabrication, LDD, drain-extended, self-aligned-gate]
---

# 4.2 Poly-Gate CMOS

> **Chapter 4: Representative Processes**

## Key Concepts

### Historical Context and the Self-Aligned Gate Revolution

The MOS transistor was first anticipated by J. E. Lilienfeld in a 1926 patent, but practical devices did not emerge until M. M. Atalla and D. Kahng succeeded in 1959 by using silicon as the backgate, thermal oxide as the dielectric, and aluminum as the gate. C.-T. Sah and colleagues at Fairchild developed the first viable metal-gate MOS process in 1963, and RCA introduced the first commercial metal-gate CMOS process in 1968.

Early metal-gate MOS processes suffered from three major problems:
1. **Mobile ion contamination** -- caused threshold voltage instabilities; eventually controlled through improved process cleanliness, phosphosilicate glass overcoats, and chlorine injection (as trichloroethylene) into oxidation furnaces.
2. **ESD vulnerability** -- MOS gate electrodes proved uniquely vulnerable to electrostatic discharge, necessitating both integrated ESD protection devices and industry-wide ESD control practices.
3. **Extreme slowness** -- caused by substantial overlap capacitances between deposited aluminum gate electrodes and underlying source/drain regions.

The overlap capacitance problem was solved by the **self-aligned gate structure**, developed between 1963 and 1966 by multiple groups (H. Dill and R. Bower at Hughes Aircraft; R. Kerwin, D. Kline, and J. Sarace at Bell Labs; B. Watkins at General Micro-Electronics). The key insight: deposit polysilicon first and pattern it to form the gate electrode, then use the gate itself as a mask during the source/drain implant. Because the polysilicon gate blocks the dopant from penetrating into the backgate region beneath it, the source and drain self-align to the gate edges, eliminating the overlap capacitance inherent in metal-gate processes.

### Why $(100)$ Silicon Instead of $(111)$

Standard bipolar uses $(111)$ silicon because it grows the fastest and suppresses parasitic PMOS channels via positive surface state charge. Poly-gate CMOS instead uses $(100)$ silicon because the $(100)$ surface aligns much more closely with the silicon dioxide lattice, generating far less surface state charge and thereby providing far better threshold voltage control. This is critical for CMOS because both NMOS and PMOS transistors require precisely controlled $V_{th}$.

### Channel Stop Implants

Since $(100)$ silicon does not generate enough surface state charge to suppress parasitic MOS transistors under metal leads crossing the thick field oxide, poly-gate CMOS relies on dedicated **channel stop implants**:
- **P-type channel stop** implanted into P-epi field regions to raise thick-field NMOS threshold
- **N-type channel stop** implanted into N-well field regions to raise thick-field PMOS threshold

In older LOCOS-based processes with relatively lightly doped backgates, these implants are essential. Newer lower-voltage STI processes use higher backgate dopings and are therefore less dependent on channel stops.

### Oxide Sidewall Spacers and "Zero Drain Overlap"

Modern poly-gate CMOS uses oxide sidewall spacers (invented by W. D. Ryden and coworkers at INMOS) to compensate for outdiffusion and straggle of source/drain dopant beneath the gate. These spacers are formed by:
1. Isotropic deposition of oxide (e.g., TEOS) over the patterned polysilicon gates
2. Anisotropic etching to leave thin filaments of oxide along the edges of poly geometries

The spacers block the source/drain implants, moving the heavily doped source and drain regions slightly away from the gate edge. By adjusting spacer thickness (controlled by poly thickness and deposition/etch conditions), process designers can exactly compensate for straggle and outdiffusion -- achieving what Ryden called **"zero drain overlap."**

### Threshold Voltage Targeting

There is a key difference between analog and digital CMOS threshold voltage targets:
- **Digital CMOS** (older, $\geq 5\,\text{V}$): targets $V_{th}$ between $0.7$ and $0.9\,\text{V}$ to prevent subthreshold leakage
- **Analog CMOS**: targets $V_{th} \approx 0.7\,\text{V}$ to maximize headroom for circuit operation; tolerates tiny subthreshold leakage at $V_{GS} = 0\,\text{V}$

Threshold adjust is accomplished via a blanket boron implant (called a $V_t$ adjust) that simultaneously reduces PMOS threshold and increases NMOS threshold. When properly tuned, a single implant can set nominal $V_{th} \approx 0.7$ to $0.8\,\text{V}$ for both transistor types.

## Fabrication Sequence (9 Mask Steps)

The baseline process described is a $5\,\text{V}$ analog CMOS process with minimum NMOS channel length of $3\,\mu\text{m}$ and PMOS channel length of $3\,\mu\text{m}$. The fabrication flow proceeds as follows:

### 1. Starting Material
- Extremely heavily doped P-type $(100)$ substrate to minimize substrate resistivity
- Low substrate resistivity provides CMOS latchup immunity by minimizing substrate debiasing
- Off-axis wafers are NOT used (unlike standard bipolar) since no buried layer exists

### 2. Epitaxial Growth
- 5--$15\,\mu\text{m}$ of $P^-$ silicon deposited on the $P^+$ substrate
- The $P^+$ substrate greatly reduces CMOS latchup severity
- Epitaxy allows precise doping control
- Manufacturers can purchase epi-coated wafers in volume, eliminating the need for in-house epi reactors

### 3. N-Well Diffusion (Mask 1: N-Well)
- Thermal oxidation, photoresist patterning, oxide etch to open windows
- Phosphorus ion implantation followed by prolonged high-temperature drive
- Creates deep, lightly doped N-type regions (N-wells) with junction depth $\sim 6$--$8\,\mu\text{m}$ for a typical $20\,\text{V}$ process
- PMOS transistors reside in N-wells; NMOS transistors reside in P-epi
- N-well process optimizes NMOS performance at the expense of PMOS (due to increased total dopant from counterdoping)
- N-well enables negative-ground power supply systems (the traditional arrangement)

### 4. Inverse Moat (Mask 2: Inverse Moat)
- LOCOS process: nitride deposited atop pad oxide, patterned using inverse moat mask
- Nitride removed over field regions (where thick oxide will grow)
- Called "inverse moat" because the mask is a color reversal of the moat drawing layer -- openings appear where moat is absent
- Pad oxide underneath nitride provides mechanical compliance to absorb strain and prevent silicon lattice dislocations

### 5. Channel Stop Implants (Mask 3: Channel Stop)
- **Blanket boron implant**: uses photoresist left from inverse moat patterning; implants all field regions; raises PMOS thick-field threshold above maximum operating voltage
- **Selective phosphorus implant**: new photoresist patterned with channel stop mask; counterdopes boron in N-well field regions; raises NMOS thick-field threshold above maximum operating voltage
- All photoresist stripped after phosphorus implant

### 6. LOCOS Oxidation and Dummy Gate Oxidation
- Steam or high-pressure oxidation grows thick field oxide
- Nitride block mask stripped
- **Bird's beak**: curved transition region at moat edges caused by oxidants diffusing under nitride film edges
- **Dummy gate oxidation**: if steam was used, the Kooi effect creates nitride deposits beneath pad oxide around moat edges; a brief etch strips the pad oxide, then a brief dry oxidation grows a sacrificial oxide that consumes any remaining nitride deposits
- Dummy gate oxide is then stripped to reveal bare silicon in moat regions

### 7. Threshold Adjust (Mask 4: $V_t$ Adjust)
- Photoresist patterned to open windows over MOS transistor locations
- Boron $V_t$ adjust implant penetrates dummy gate oxide to dope underlying silicon
- Dummy gate oxide stripped after implant
- **True gate oxidation**: dry oxygen to minimize surface state and fixed oxide charges; produces gate oxide of $\sim 300$--$500\,\text{A}$ for $5\,\text{V}$ devices

### 8. Polysilicon Deposition and Patterning (Mask 5: Poly)
- Polysilicon deposited in intrinsic state, then blanket-doped with phosphorus
- In-situ doping not used because it reduces deposition rates
- Heavy phosphorus doping reduces resistivity to $\sim 20$--$30\,\Omega/\square$
- Phosphorus doping produces threshold voltages compatible with single-step $V_t$ adjust
- Enables threshold voltage control of $\pm 50$--$100\,\text{mV}$
- Poly width sets the MOS channel length -- the smallest feature size in the process

### 9. Source/Drain Implants (Masks 6 & 7: NSD and PSD)
- **Sidewall spacer formation**: TEOS oxide deposited then removed by reactive ion etching, leaving spacers on gate edges
- **NSD implant** (Mask 6): photoresist patterned, shallow arsenic implant forms heavily doped N-type source/drain; polysilicon gate + spacers block implant from regions beneath gate
- **PSD implant** (Mask 7): second photoresist layer, shallow boron implant forms heavily doped P-type source/drain
- Brief anneal activates dopants and grows thin oxide over source/drain

### 10. Contacts (Mask 8: Contact)
- Multilevel oxide (MLO) deposited before contact patterning -- BPSG (borophosphosilicate glass)
- MLO thickens oxide over moat regions and insulates exposed polysilicon
- Contact openings patterned; NSD/PSD plugs placed near backgate contacts for Ohmic contact (backgate regions too lightly doped for direct contact)
- Brief BPSG reflow at high temperature moderates contact sidewall geometries -- this is the **final high-temperature step**

### 11. Metallization (Mask 8: Metal)
- Contact silicidation (platinum silicide) ensures reliable contact to shallow source/drain without junction spiking
- Thin refractory barrier metal + thick copper-doped aluminum
- Reactive ion etching patterns the interconnection
- Most versions include a second metal layer separated by interlevel oxide (ILO) with planarization
- Vias etched through ILO connect to second metal

### 12. Protective Overcoat (Mask 9: POR)
- Thick BPSG, compressive nitride, or combination deposited for mechanical protection and contamination prevention
- POR mask etches openings for bondpad attachment

## Available Devices

### NMOS Transistor
- Source/drain: NSD implants self-aligned to polysilicon gate
- Backgate: P-epi (and by extension, substrate) -- substrate contacts serve as backgate terminal
- Compact layouts abut PSD substrate contacts against NSD source (when source is at substrate potential)
- **Coding practice**: Drawing layers NMoat and PMoat simultaneously generate figures on both NSD/PSD and moat masks. NSD/PSD oversized by $\sim 1.0\,\mu\text{m}$ to cover moat despite worst-case misalignment
- **Contacts**: only square contacts of a specific minimum size allowed (etch rate depends on opening size); larger contacts use arrays of minimum contacts
- **Drawn vs. effective dimensions**: drawn length $L$ = distance across poly from source to drain side; drawn width $W$ = perimeter of poly touching source (= perimeter touching drain); effective dimensions $L_{eff}$, $W_{eff}$ extracted from electrical behavior

**Hot Electron Degradation**: limits NMOS to relatively low $V_{DS}$ operating voltages when in saturation. High electric field across the pinched-off region accelerates electrons; some penetrate into gate oxide and generate surface state charges that shift $V_{th}$. Two voltage ratings exist:
- **Blocking voltage**: limited by junction breakdown and punchthrough (applies to switches)
- **Operating voltage**: limited by hot electron degradation (applies to transistors in saturation, common in analog circuits)

**Natural NMOS**: blocking $V_t$ adjust implant produces a device with $V_{th} \approx 0\,\text{V}$, useful in analog circuits where the normal threshold is inconveniently large.

### PMOS Transistor
- Resides in N-well (backgate)
- Multiple PMOS transistors can share a well if their backgates are at the same potential -- saves substantial area due to large N-well outdiffusion
- Backgate can connect to any voltage $\geq V_S$ -- provides an extra degree of freedom for analog designers
- Less susceptible to hot carrier degradation than NMOS (holes are less mobile than electrons; larger $V_{DS}$ needed to inject charge)
- **Natural PMOS**: blocking $V_t$ adjust yields $|V_{th}| > 1\,\text{V}$; useful for power devices to inhibit subthreshold conduction

### Typical Device Parameters (Table 4.4)

| Parameter | NMOS | PMOS |
|-----------|------|------|
| Min. channel length | $3\,\mu\text{m}$ | $3\,\mu\text{m}$ |
| Gate oxide thickness | $\sim 300$--$500\,\text{A}$ | $\sim 300$--$500\,\text{A}$ |
| $V_{th}$ (adjusted) | $\sim +0.7\,\text{V}$ | $\sim -0.7\,\text{V}$ |
| $V_{th}$ (natural) | $\sim 0\,\text{V}$ | $> -1\,\text{V}$ |
| Max $V_{GS}$ | rated | rated |
| Max $V_{DS}$ (blocking) | higher | higher |
| Max $V_{DS}$ (operating) | lower (hot-e) | higher |

### Substrate PNP Transistor
- Only bipolar transistor available in N-well CMOS
- Emitter: PSD implant in N-well; Base: N-well; Collector: substrate + P-epi surrounding N-well
- $\beta \approx 50$--$100$ for a $5\,\text{V}$ process; drops to $10$--$20$ for lower-voltage processes with heavier well doping
- Injects current into substrate -- requires adequate substrate contact area
- Typical IC includes enough scribe-street substrate contacts for $10$--$100\,\text{mA}$ of substrate current
- Lateral PNP theoretically possible but exhibits $\beta < 1$ due to high substrate injection (no NBL)

### Resistors (4 Types)

| Parameter | Poly | PSD | NSD | N-Well |
|-----------|------|-----|-----|--------|
| Sheet resistance | $20$--$30\,\Omega/\square$ | $\sim 50\,\Omega/\square$ | $30$--$50\,\Omega/\square$ | $2$--$3\,\text{k}\Omega/\square$ |
| Min. drawn width | $3\,\mu\text{m}$ | $3\,\mu\text{m}$ | $3\,\mu\text{m}$ | $5\,\mu\text{m}$ |
| Breakdown voltage | $> 100\,\text{V}$ | $20\,\text{V}$ | $20\,\text{V}$ | $40\,\text{V}$ |
| Variability ($10\,\mu\text{m}$ width) | $< 30\%$ | $< 20\%$ | $< 20\%$ | $< 40\%$ |

**Poly resistor**: most useful; strip of poly on field oxide; dielectrically isolated so can be biased arbitrarily (withstands $> 100\,\text{V}$ differential to substrate, can operate below ground or above $V_{DD}$). Low parasitic capacitance to substrate. Drawback: oxide isolation does not conduct heat well -- self-induced annealing at high power, and extreme power can melt or crack poly. This property enables polysilicon fuses for wafer-level trimming but makes poly resistors poor for pulsed-power (e.g., ESD protection).

**NSD resistor**: strip of NSD diffusion; $R_s \approx 30$--$50\,\Omega/\square$; limited to $\sim 8\,\text{V}$ by shallow junction avalanche.

**PSD resistor**: strip of PSD in N-well; N-well must be biased above the resistor for isolation; limited sheet resistance and low breakdown.

**N-well resistor**: high $R_s$ ($2$--$3\,\text{k}\Omega/\square$) but notoriously variable due to doping variation, voltage modulation of depletion regions, and surface effects. Most designers prefer narrow poly resistors. Field plates can help minimize variability.

### Gate Oxide Capacitor
- One plate: doped polysilicon; other plate: N-well diffusion
- Capacitance of $\sim 400\,\text{A}$ oxide $\approx 0.86\,\text{fF}/\mu\text{m}^2$
- Tight oxide thickness control yields tolerance of $\pm 2\%$
- **Critical**: N-well electrode must remain $\geq 1\,\text{V}$ above poly electrode; failure causes dramatic capacitance drop (depletion of the well surface)
- Drawbacks: excessive bottom-plate parasitic junction capacitance, series resistance, voltage-dependent capacitance variation

## Process Extensions

### Lightly Doped Drain (LDD) Transistors

Hot carrier degradation becomes a concern when MOS transistors operate in saturation at high $V_{DS}$. Typical $\sim 400\,\text{A}$ gate oxide NMOS with $3\,\mu\text{m}$ channel length has operating voltage of $\sim 8$--$10\,\text{V}$; PMOS of similar dimensions: $\sim 15$--$20\,\text{V}$. Higher voltages require alternative structures.

**Root cause**: In a conventional singly doped drain (SDD) transistor, the depletion region cannot intrude significantly into the heavily doped drain. The entire voltage drop occurs across a narrow pinched-off region, creating an intense electric field. If the drain were more lightly doped, the depletion region could extend into the drain as well, spreading out the electric field and reducing its peak value.

**LDD construction using oxide sidewall spacers**:
1. Pattern polysilicon gate
2. Shallow $N^-$ implant self-aligned to gate edges (lightly doped drain, also called $N^-$ S/D or NMSD)
3. Deposit isotropic oxide layer
4. Anisotropic etch leaves sidewall spacers on gate edges
5. Second, heavier $N^+$ implant self-aligned to spacers forms heavily doped extrinsic drain

The width of the lightly doped drift region $\approx$ width of the sidewall spacer (typically $\sim 0.5\,\mu\text{m}$).

**Performance**: A $\sim 400\,\text{A}$ gate oxide LDD NMOS with $3\,\mu\text{m}$ channel length achieves operating voltage of $12$--$15\,\text{V}$, roughly equivalent to SDD PMOS. For this reason, most $10$--$20\,\text{V}$ poly-gate CMOS processes use **LDD NMOS combined with SDD PMOS**.

**Symmetry**: only the drain needs LDD, but the sidewall spacer cannot be selectively blocked from the source side. The resulting transistor is symmetric -- source and drain can be interchanged. PMOS also receives sidewall spacers (but no lightly doped diffusion); the channel that forms underneath gives it a slightly longer effective channel length.

**Optional $N^-$ block mask**: short-channel transistors break down by punchthrough before hot carrier degradation matters, so the $N^-$ drift region serves no purpose. Blocking the $N^-$ implant allows reducing drawn channel length by $0.5$--$1.0\,\mu\text{m}$, which can significantly impact designs with large amounts of low-voltage digital logic.

### High-Voltage Drain-Extended Transistors

For operating voltages beyond $\sim 15$--$20\,\text{V}$, sidewall spacers alone are insufficient. **Drain-extended transistors** use existing masks (no additional cost) from the standard N-well poly-gate CMOS process:

- **Drain-extended NMOS**: uses the N-well as the drift (lightly doped drain) region. The N-well is deep and lightly doped, giving it a breakdown voltage in excess of $40$--$50\,\text{V}$. An NSD plug within the N-well forms the extrinsic drain contact. The source is NSD without N-well -- making this an **asymmetric** device (source and drain cannot be interchanged).

- **Drain-extended PMOS**: uses the P-type channel stop implant to construct the lightly doped drain region (discussed in Section 13.1.2).

**Key characteristics**:
- Drains do NOT self-align to gates -- large overlap capacitances
- Much more resistive than similarly sized LDD or SDD transistors
- Permit higher-voltage operation without additional masks

**Gate oxide dilemma**: standard $\sim 300$--$500\,\text{A}$ gate oxide can safely handle only $\sim 10$--$15\,\text{V}$. Solutions:
- A separate thick gate oxidation (but requires larger gate voltage to fully enhance)
- **Better solution**: thicken the gate oxide only over the lightly doped drain using the **LOCOS bird's beak** as a field-relief structure. The gradual oxide taper reduces the vertical electric field at the drain edge. STI can also create a similar structure, but the abrupt transition makes electric field control more challenging.

## Diagrams

### Figure 4.18 -- Self-Aligned Polysilicon-Gate NMOS Fabrication

![[diagrams/ch04-poly-gate-cmos-fig1.png]]

The foundational concept of poly-gate CMOS: (A) polysilicon is deposited and patterned to form the gate electrode atop gate oxide; (B) phosphorus source/drain implant is performed -- the poly gate blocks the dopant from the channel region beneath it, while thick field oxide prevents implant into the field; (C) the resulting self-aligned source/drain structure. This eliminates the overlap capacitance that plagued metal-gate processes.

### Figure 4.33 -- LDD NMOS Fabrication Steps

![[diagrams/ch04-poly-gate-cmos-fig2.png]]

The four steps to fabricate an LDD NMOS transistor using oxide sidewall spacers: Step 1: shallow $N^-$ source/drain implant self-aligned to the poly gate. Step 2: isotropic oxide deposition. Step 3: anisotropic etch leaves sidewall spacers. Step 4: heavy $N^+$ source/drain implant self-aligned to the spacers, creating a lightly doped drift region approximately equal to the spacer width.

### Figure 4.34 -- Drain-Extended NMOS Transistor

![[diagrams/ch04-poly-gate-cmos-fig3.png]]

Layout and cross section of a drain-extended NMOS. The N-well forms the lightly doped drift region (the "extended drain"), with an NSD plug inside the well for the drain contact. The source is standard NSD without N-well. The LOCOS bird's beak at the drain side thickens the gate oxide over the drift region, providing field relief. The backgate is common to the substrate.

## Practical Takeaways

- **N-well process is the default**: optimizes NMOS performance and supports the standard negative-ground power supply arrangement. PMOS transistors at different supply voltages occupy separate N-wells.
- **Always provide backgate contacts near NMOS transistors**: even though substrate provides the backgate, nearby PSD contacts improve latchup immunity by pinning the epi surface potential. Pcells that include built-in backgate contacts create significant substrate contact area without extra layout effort.
- **Never abut PSD and NSD at different potentials**: the resulting $P^+N^+$ junction leaks excessively and breaks down at very low voltage.
- **Use arrays of minimum-size square contacts** rather than single large contacts: oxide etch rate varies with opening size, causing reliability issues with non-standard contact dimensions.
- **LDD NMOS + SDD PMOS** is the standard combination for $10$--$20\,\text{V}$ processes -- the inherently lower hot-carrier susceptibility of PMOS means it does not need LDD at these voltages.
- **Drain-extended transistors are free** in terms of mask cost -- they reuse the N-well mask. Trade-offs are higher resistance and higher overlap capacitance.
- **For drain-extended NMOS, use the LOCOS bird's beak for gate oxide field relief**: this avoids the need for a separate thick gate oxide while still protecting against oxide breakdown at the drain edge.
- **Poly resistors are the workhorse** of CMOS analog: dielectrically isolated, can be biased arbitrarily, low parasitic capacitance. But avoid high power dissipation (self-annealing) and pulsed-power applications.
- **Gate oxide capacitors require proper bias**: the N-well plate must remain $\geq 1\,\text{V}$ above the poly plate, or capacitance drops dramatically due to surface depletion.
- **Substrate PNP beta degrades with lower-voltage processes** (heavier well doping): $\beta \approx 50$--$100$ at $5\,\text{V}$, but only $10$--$20$ at $3.3\,\text{V}$.
- **PG deck geometric operations** for generating masks from drawn layers are non-trivial; NSD/PSD regions are oversized relative to moat to ensure implant coverage despite misalignment.

## Relation to the Bigger Picture

Poly-gate CMOS is the second of three archetypal processes examined in Chapter 4 (after [[ch04-standard-bipolar]] and before [[ch04-analog-bicmos]]). It represents the transition from bipolar-dominated analog design to the CMOS-dominated world we inhabit today. The self-aligned gate structure, LOCOS isolation, sidewall spacers, and LDD concepts introduced here form the foundation upon which modern analog BiCMOS processes are built -- indeed, Section 4.3 explicitly describes how analog BiCMOS adds bipolar and DMOS devices to an existing poly-gate CMOS process flow. Understanding the device physics here -- particularly hot carrier degradation, threshold adjust mechanisms, and the trade-offs between SDD, LDD, and drain-extended structures -- is essential for making informed layout decisions in any CMOS-derived process, as explored further in the device-level chapters (Chapters 9, 12, 13) and the MOS transistor fundamentals in [[ch01-mos-transistors]].

## See Also
- [[ch04-standard-bipolar]]
- [[ch04-analog-bicmos]]
- [[ch01-mos-transistors]]
