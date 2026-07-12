---
title: "7.1 Capacitance"
chapter: 7
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-7, capacitors, dielectrics, parasitics, MOS-capacitor, junction-capacitor]
---

# 7.1 Capacitance

> **Chapter 7: Capacitors and Inductors**

## Key Concepts

### Fundamental Capacitance Equation

A capacitor stores energy in an electric field between two conductive surfaces (electrodes) separated by an insulator (dielectric). The defining relationship is:

$$Q = C \cdot V$$

where $C$ is the capacitance (in farads), $Q$ is the charge, and $V$ is the voltage differential. For a parallel-plate capacitor whose electrode dimensions greatly exceed the dielectric thickness $d$:

$$C = \frac{\varepsilon A}{d}$$

where $A$ is the electrode area and $\varepsilon$ is the permittivity of the dielectric. The permittivity of free space is $\varepsilon_0 = 8.854 \times 10^{-12}$ F/m, and the relative permittivity $\kappa$ (or $\varepsilon_r$) is defined as $\kappa = \varepsilon / \varepsilon_0$.

This equation immediately reveals why integrated capacitors are so difficult to make large: even a $100 \times 100$ $\mu$m$^2$ plate with $200$ A of gate oxide ($\kappa = 3.9$) yields only about $17$ pF. Practical integrated circuits rarely accommodate more than a few nanofarads.

### Relative Permittivities of IC Materials

| Material | $\kappa$ | Dielectric Strength (MV/cm) |
|---|---|---|
| Silicon | 11.8 | 30 |
| Dry oxide ($\text{SiO}_2$) | 3.9 | 11 |
| PECVD oxide | 4.9 | 3--6 |
| TEOS oxide | 4.0 | 10 |
| LPCVD nitride ($\text{Si}_3\text{N}_4$) | 6--7 | 10 |
| PECVD nitride | 6--9 | 5 |

High-$\kappa$ dielectrics like hafnium oxide ($\kappa \approx 25$) are used in advanced CMOS gate stacks to increase capacitance while maintaining physical thickness (reducing tunneling). Barium titanate ceramics reach $\kappa > 10{,}000$ but have extreme voltage and temperature dependence.

### Composite Dielectrics

Many capacitors use composite dielectrics (e.g., ONO = oxide-nitride-oxide). The effective relative permittivity of a two-material composite is:

$$\kappa_{\text{eff}} = \frac{d_1 + d_2}{\frac{d_1}{\kappa_1} + \frac{d_2}{\kappa_2}}$$

For example, $200$ A of nitride ($\kappa = 7.5$) sandwiched between two $50$ A oxide films ($\kappa = 3.9$) gives $\kappa_{\text{eff}} \approx 5.7$.

### Maximum Operating Voltage and Derating

The dielectric breaks down when the electric field exceeds its dielectric strength. The maximum safe operating voltage is:

$$V_{\max} = E_{\text{safe}} \cdot d_{\min}$$

where $E_{\text{safe}}$ is well below the dielectric strength to guard against **time-dependent dielectric breakdown (TDDB)**. Amorphous dielectrics like $\text{SiO}_2$ experience TDDB because their irregular bond network contains weak bonds that gradually break under electrical stress. Crystalline dielectrics (e.g., silicon) do not experience TDDB because all bonds are of similar strength.

Two TDDB models exist:
- **Anode Hole Injection (AHI) model** ($1/E$-model): time to failure scales exponentially with $1/E$.
- **McPherson model** ($E$-model): time to failure scales exponentially with $E$.

Failure rates are specified in FITs (failures in $10^9$ hours). Current practice assigns about 10 FITs to all TDDB mechanisms. The AHI derating equation is:

$$\lambda_2 = \lambda_1 \left(\frac{A_2}{A_1}\right) \left(\frac{D_2}{D_1}\right) \exp\left[\beta_W \left(\frac{1}{E_1} - \frac{1}{E_2}\right)\right]$$

where $\lambda$ is the failure rate, $A$ is area, $D$ is duty cycle, $\beta_W$ is the Weibull slope, and the exponential term captures the field acceleration. Analog designers can perform this derating exercise themselves to operate small dielectric areas at higher voltages than conservative blanket rules allow.

## Fringing Capacitance

When electrodes have finite extent, the electric field curves outward beyond the plate edges, creating a **fringing field** that adds to the parallel-plate capacitance. For a capacitor with electrode perimeter $P$, electrode thickness $t_e$, dielectric thickness $d$, and permittivity $\varepsilon$:

$$C_f = \frac{\varepsilon P}{\pi} \ln\left(\frac{2\pi t_e}{d} + e\right)$$

where $e \approx 2.718$ is Euler's number. For a capacitor with one finite plate and one infinite plate, set $t_e$ to half the dielectric thickness.

For a practical $5 \times 5$ $\mu$m poly-poly capacitor with $500$ A of oxide, fringing adds about 2.6 fF to the 43.1 fF plate capacitance -- small for large caps, but significant for small ones.

### Parasitic Wiring Capacitance (Van de Meijs-Fokkema)

For a metal lead of width $w$ and thickness $t$ at height $h$ above a ground plane, the total capacitance per unit length is:

$$C_l = \varepsilon \left[\frac{w}{h} + 0.77 + 1.06\left(\frac{w}{h}\right)^{0.25} + 1.06\left(\frac{t}{h}\right)^{0.5}\right]$$

The first term is the plate capacitance; the remaining terms are fringing. For a typical metal line ($1$ $\mu$m wide, $5000$ A thick, over $8000$ A field oxide), **fringing accounts for about 60% of total parasitic capacitance**. This is critical for digital timing verification and parasitic back-annotation.

## Junction Capacitance

A junction capacitor uses the depletion region of a reverse-biased PN junction as its dielectric. Silicon has $\sim 3\times$ the permittivity and dielectric strength of oxide, so junction caps can integrate more capacitance per unit area than oxide caps. However, they are profoundly **nonlinear**.

### Zero-Bias Junction Capacitance

For a large planar junction:

$$C_{j0} = \frac{\varepsilon_{Si} \cdot A}{W_0}$$

where $W_0$ is the zero-bias depletion width. For an abrupt junction between a heavily doped layer ($N_H$) and a lightly doped layer ($N_L$):

$$W_0 = \sqrt{\frac{2 \varepsilon_{Si} \phi_0}{q N_L}}$$

and the built-in potential is:

$$\phi_0 = V_T \ln\left(\frac{N_H N_L}{n_i^2}\right)$$

where $V_T = kT/q \approx 25.9$ mV at room temperature, and $n_i \approx 9.65 \times 10^9$ cm$^{-3}$ at 300 K.

### Sidewall and Peripheral Capacitance

A diffused junction has a flat bottom, curved sidewalls, and filleted corners. The total junction area is approximately:

$$A_{\text{total}} \approx A_{\text{drawn}} + \frac{\pi}{2} x_j P_{\text{drawn}}$$

where $x_j$ is the junction depth and $P_{\text{drawn}}$ is the drawn perimeter. An empirical alternative separates areal and peripheral components:

$$C_{j0} = C_A \cdot A_{\text{drawn}} + C_P \cdot P_{\text{drawn}}$$

where $C_A$ (fF/$\mu$m$^2$) and $C_P$ (fF/$\mu$m) are determined by measuring capacitors with different area-to-perimeter ratios.

### Voltage Modulation of Junction Capacitors

The dynamic depletion capacitance varies with voltage:

$$C_j(V) = \frac{C_{j0}}{(1 - V/\phi_0)^m}$$

where $V$ is the forward voltage (anode-to-cathode), $\phi_0$ is the built-in potential, and $m$ is the **grading coefficient** (0.5 for abrupt, 0.33 for linearly graded junctions; most diffused junctions are $\sim 1/3$). This makes junction caps useful as **varactors** (voltage-variable capacitors for LC tank tuning) but problematic for precision applications.

### Plate vs. Comb Layout

- **Plate layout** (Figure 7.6A): maximizes junction area.
- **Comb layout** (Figure 7.6B): maximizes junction periphery.

Comb is denser than plate when the finger spacing $s$ satisfies:

$$s < \frac{\pi}{2} x_j \cdot \frac{C_P}{C_A}$$

Most standard bipolar processes slightly favor comb layouts. Comb layouts also have lower parasitic series resistance because unpinched base regions lie between fingers.

## Capacitor Variability

### Process Variation
- **MOS capacitors** (gate oxide): Modern processes control gate oxide capacitance within $\pm 5\%$; some achieve $\pm 2\%$. Gate oxides below $\sim 30$ A experience significant electron tunneling leakage; oxynitrides or high-$\kappa$ dielectrics address this.
- **ONO dielectrics**: At least $\pm 15\%$ variation due to three-step formation process.
- **Junction capacitors (plate layout)**: $\pm 20\%$ or more; comb layouts are even worse ($\pm 30\%$+) due to sensitivity to emitter junction depth and surface effects.

### Voltage Modulation
- **Junction capacitors**: Severe -- depletion width varies dramatically with bias.
- **Poly-poly capacitors**: $\sim\pm 5\%$ due to poly depletion; heavier doping reduces it. The depletion width into poly doped at $N_D$ under voltage $V$ is:

$$W_d = \frac{\varepsilon_d}{2 \varepsilon_{Si}} \left[\sqrt{d^2 + \frac{4 \varepsilon_{Si} V}{q N_D}} - d\right]$$

The dynamic capacitance then becomes $C = \varepsilon_d A / (d + (\varepsilon_d / \varepsilon_{Si}) W_d)$.

- **MOS capacitors**: Strong voltage modulation. The CV curve shows maximum capacitance ($C_{\text{ox}} = \varepsilon_{\text{ox}} W L / t_{\text{ox}}$) in deep accumulation and deep inversion, with a minimum in depletion. To minimize voltage variation, operate with an overdrive of at least 1 V beyond $V_{FB}$ (accumulation) or $V_{th}$ (inversion).

### Temperature Modulation
- **Thin oxide films**: Temperature coefficient of permittivity $\sim +25$ ppm/$^\circ$C.
- **Thin nitrides**: $\sim +50$ ppm/$^\circ$C.
- **Junction capacitors**: Significant positive tempco because depletion regions thin as temperature rises (through $n_i(T)$ and $V_T$ dependence). With an empirical formula for $n_i(T)$:

$$n_i(T) \approx 5.29 \times 10^{19} \left(\frac{T}{300}\right)^{2.54} \exp\left(\frac{-6726}{T}\right) \text{ cm}^{-3}$$

Typical junction cap tempco: $\sim +200$ ppm/$^\circ$C (varies with doping).

- **Poly-poly capacitors**: Tempco typically $< +50$ ppm/$^\circ$C (depletion effects are small).

## Capacitor Parasitics

### Poly-Poly Capacitor Parasitics

A poly-poly capacitor has parasitic capacitance from the lower electrode to the substrate ($C_{par1}$) and from the upper electrode to overlying metal ($C_{par2}$). Series resistance of the poly electrodes becomes significant at high frequencies. The subcircuit model divides the capacitor into $n$ sections with distributed $R$ and $C$:

$$R_{\text{upper}} = \frac{R_{s,\text{upper}}}{n(n+1)}$$

The parasitic series resistance also models **dielectric losses** -- atoms in the dielectric move under changing fields, dissipating energy. Oxide and nitride have very low dielectric losses, keeping parasitic resistance constant up to very high frequencies.

### MOS Capacitor Parasitics

The lower electrode (diffusion or inversion layer) has significant parasitic junction capacitance to substrate. The upper electrode (gate) has relatively little parasitic capacitance. Circuit designers orient MOS capacitors to connect the lower plate to low-impedance nodes (supply/ground) to render parasitics irrelevant.

The channel resistance of a MOS capacitor in inversion is:

$$R_{\text{channel}} \approx \frac{1}{\mu C_{\text{ox}} (V_{GS} - V_{th}) (W/L)}$$

This is distributed along the channel length; the lumped equivalent is $R_{\text{channel}}/3$ for one-end contact, $R_{\text{channel}}/12$ for two-end contact.

### Junction Capacitor Parasitics

Parasitic junctions are modeled as diodes that must remain reverse-biased. If they momentarily forward-bias, large currents flow and latchup may occur -- proper guard ringing is essential.

## Comparison of Available Capacitors

### Junction Capacitors
- Available in standard bipolar and some BiCMOS processes.
- Typical capacitance: $\sim 0.8$ fF/$\mu$m$^2$ (zero-bias), dropping to $\sim 0.5$ fF/$\mu$m$^2$ at a few volts reverse bias.
- Process variation: $\pm 20$% to $\pm 50$%.
- Temperature coefficient: $\sim +200$ ppm/$^\circ$C.
- Must keep reverse bias below $\sim 75$% of breakdown ($\sim 4$ V for typical emitter-base).
- Advantage: dielectric breakdown is non-catastrophic (unlike oxide rupture).

### MOS Capacitors
- Four types: NMOS/PMOS in accumulation or inversion.
- **Accumulation**: electrodes = gate + backgate. **Inversion**: electrodes = gate + source (with source/drain diffusions adjacent to gate).
- Best biasing: overdrive of at least $0.5$ V (preferably $1$ V) beyond $V_{FB}$ or $V_{th}$ to stay deep in the desired region.
- For **polarity-reversing** applications: use antiparallel (back-to-back) inversion PMOS capacitors, each half the total value. Their depletion dips are offset by $2V_{th}$.
- Gate resistance: lumped equivalent is $R_{gate}/3$ (one-end contact) or $R_{gate}/12$ (two-end contact). If layout rules allow poly contacts over active gate area, series resistance is virtually eliminated.
- Standard bipolar variant: thin oxide over emitter diffusion; surface doping $> 10^{20}$ cm$^{-3}$ makes depletion negligible.
- **Sandwich/stacked capacitor**: base-emitter junction in parallel with emitter oxide cap gives $> 2$ fF/$\mu$m$^2$ but with high variability.

### Poly-Poly Capacitors
- Both electrodes deposited -- eliminates junction parasitics and voltage biasing restrictions.
- Dielectric options: **ONO** (older, $\sim 1.3$ fF/$\mu$m$^2$, but hysteresis/soakage at $> 10$ MHz), **pure nitride** (modern, eliminates charge-trapping issues), **pure oxide** (lower cap/area but better matching for small caps).
- ONO exhibits **asymmetric breakdown** because thermal oxidation of poly creates asperities that enhance Fowler-Nordheim tunneling from the negative plate. Breakdown can differ by up to 50% depending on polarity.
- **Poly stringers** form when poly-2 deposits over poly-1 steps. Modern processes solve this by patterning poly-2 first, then poly-1.
- Voltage modulation: $\sim \pm 5$% (unsilicided); can be reduced by cross-coupling two equal sections in opposite orientations.
- Temperature coefficient: $< +50$ ppm/$^\circ$C.

### Metal-Metal (MIM) Capacitors
- Metallic electrodes cannot deplete, so **negligible voltage and temperature variation** -- the gold standard for precision analog.
- Challenge: CVD oxides have poor dielectric integrity unless **densified** at $\sim 800$--$900$$^\circ$C, but aluminum melts at $\sim 660$$^\circ$C.
- Solutions:
  - **Refractory barrier metal** (e.g., TiN) as lower electrode -- can withstand densification. Requires 2 extra masks.
  - **Silicided poly** lower electrode -- permits densification. Requires 1 extra mask.
  - **ARC-based capacitor**: antireflective coating (silicon oxynitride) as dielectric between aluminum (lower) and TiN (upper). Excellent integrity without densification. Requires 1 extra mask. Can be inserted between any two metal layers, allowing placement over other circuitry.
- A $250$ A nitride between silicide and TiN achieved linear voltage coefficient $\sim 30$ ppm/V, quadratic $\sim 2$ ppm/V$^2$, and tempco $\sim 30$ ppm/$^\circ$C.
- Residual voltage modulation in nitride is attributed to Si-H bonds; adjusting deposition or $\text{N}_2\text{O}$ annealing removes them.

### Stack Capacitors (ILO Dielectric)
- Use interlevel oxide ($\sim 8000$--$10{,}000$ A) between adjacent metal layers. No extra masks needed.
- Very low capacitance per unit area ($\sim 0.05$ fF/$\mu$m$^2$) -- nearly $1$ mm$^2$ for $1$ pF.
- **Interleaving** multiple metal layers (e.g., poly + metal-1 + metal-2) doubles or triples the capacitance. The sandwiched electrode (e.g., metal-1) has virtually no parasitic capacitance.
- High voltage rating since ILO is thick.

### Lateral Flux Capacitors
- Exploit lateral electric fields between closely spaced strips on the same metal layer, in addition to vertical fields between layers.
- **Horizontal bar**: alternating A/B strips on each layer, stacked. Becomes competitive when metal-metal spacing $<$ ILO thickness.
- **Vertical bar**: minimum-size pillars (A/B in checkerboard) connected by vias; benefits from lateral flux in two dimensions. Outperforms horizontal bar when lateral spacing $\ll$ ILO thickness.
- **Woven structure**: orthogonal strips on alternating layers connected by vias at intersections. Lower parasitic inductance than horizontal bar -- important at very high frequencies.
- **Fractal geometries**: maximize perimeter to boost lateral flux; lower series resistance than narrow strips.

### Trench Capacitors
- True 3D structures exploiting deep-etched trenches with dielectric-lined sidewalls.
- Capacitance depends on **aspect ratio** (depth/width). Modern deep trenches achieve extreme aspect ratios.
- Researchers have demonstrated $\sim 400$ nF/mm$^2$ using $35$ nm LPCVD nitride on $70$ $\mu$m deep trenches -- over $1000\times$ more than parallel-plate caps.
- **Simple structure**: monocrystalline Si outside trench (one electrode), poly fill inside (other electrode). One electrode is common to substrate.
- **Concentric structures**: multiple poly layers separated by thin nitride inside the same trench. Two concentric caps $\approx 2\times$ capacitance. Values up to $\sim 500$ nF/mm$^2$ demonstrated.
- Sharp corners at trench top/bottom intensify electric field and reduce operating voltage; rounding etches can mitigate this.

### Summary Table of Capacitor Types

| Process | Capacitor Type | Typical Cap (fF/$\mu$m$^2$) | Process Var. (%) | Max V (V) |
|---|---|---|---|---|
| Standard Bipolar | Base-Emitter junction | 0.8 | 50 (dagger) | 5 |
| Standard Bipolar | Emitter oxide | 0.07 | 30 (dagger) | 40 |
| Poly-gate CMOS | GOX | 0.86 | 20 (dagger) | 15 |
| Poly-gate CMOS | Poly-poly (ONO) | 1.3 | 25 | 15 |
| Poly-gate CMOS | Stack (ILO) | 0.05 | 30 | 40 |
| Analog BiCMOS | Thick GOX | 2.2 | 20 (dagger) | 7 |
| Analog BiCMOS | Thin GOX | 4.3 | 20 (dagger) | 3.6 |
| Analog BiCMOS | TiN (MIM) | 3.1 | 20 | 7 |
| Analog BiCMOS | Lateral flux | 0.07 | 30 | 40 |

*(dagger) = significant voltage variation*

## Diagrams

### Figure 7.3 -- Fringing Field and Parasitic Wiring Capacitance

![[diagrams/ch07-capacitance-fig1.png]]

**Caption**: The electric field around a parallel-plate capacitor, showing the fringing field extending beyond the plate edges. Fringing capacitance adds to the ideal parallel-plate value and becomes dominant for narrow metal leads over field oxide (up to 60% of total parasitic capacitance for typical IC metal traces). The Van de Meijs-Fokkema equation (Eq. 7.7) is provided for computing total capacitance per unit length including fringing terms. This is critical for parasitic back-annotation in digital timing verification and analog noise analysis.

### Figure 7.8 -- CV Plot of NMOS Capacitor

![[diagrams/ch07-capacitance-fig2.png]]

**Caption**: Capacitance-voltage (CV) plot of an NMOS transistor showing three operating regions: accumulation (maximum $C_{\text{ox}}$), depletion (capacitance drops), and inversion (capacitance recovers to $C_{\text{ox}}$ only if source/drain are connected to backgate). The minimum capacitance $C_{\min}$ occurs at the onset of inversion. Circuit designers operate MOS capacitors either deep in accumulation ($V_{GB} < V_{FB} - 0.5$ V) or deep in inversion ($V_{GS} > V_{th} + 0.5$ V) to maximize and stabilize capacitance. Without source/drain connections, the inversion layer takes hundreds of milliseconds to form via thermal generation.

### Figure 7.18 -- Lateral Flux Capacitor Structures

![[diagrams/ch07-capacitance-fig3.png]]

**Caption**: Two styles of lateral flux capacitors: (A) horizontal bar structure with alternating A/B strips on each metal layer, exploiting lateral flux in one dimension; (B) vertical bar structure with minimum-size pillars in a checkerboard pattern connected through vias, exploiting lateral flux in two dimensions. The vertical bar outperforms horizontal bar when lateral metal spacings are much smaller than ILO thickness -- a common situation in advanced processes with sub-micron metal pitch.

## Practical Takeaways

- **Metal-metal (MIM) capacitors are the gold standard** for precision analog: negligible voltage and temperature modulation because metallic electrodes cannot deplete.
- **MOS capacitors require careful biasing**: operate deep in accumulation or inversion with at least $0.5$--$1$ V overdrive to minimize the CV dip through depletion. Connect the lower plate (silicon electrode) to a low-impedance node.
- **Poly-poly capacitors offer a good compromise**: better than junction caps (no bias restrictions, lower parasitics), but worse than MIM (some voltage modulation from poly depletion). Use pure nitride or oxide dielectric, not ONO, to avoid dielectric absorption at high frequencies.
- **Junction capacitors are nonlinear but non-catastrophic on breakdown**: useful in standard bipolar when no thin oxide is available, and in ESD/clamp applications.
- **ONO dielectrics have three problems**: dielectric absorption (hysteresis at > 10 MHz), asymmetric breakdown (poly asperities enhance tunneling from negative plate), and higher process variation (three-step formation).
- **Fringing capacitance dominates narrow interconnect parasitics**: for a $1$ $\mu$m metal line, fringing contributes $\sim 60$% of total capacitance to substrate. Always include fringing in parasitic extraction.
- **TDDB derating is process-specific**: analog designers can independently derate small dielectric areas to higher voltages than conservative blanket rules, using the AHI or McPherson models with actual FIT allocations.
- **Trench capacitors provide extreme capacitance density** ($100$--$500$ nF/mm$^2$) but require specialized processing. Concentric poly structures in a single trench can double capacitance.
- **Lateral flux capacitors become viable** when metal pitch shrinks below ILO thickness. They use only standard metal layers (no extra masks) and have negligible voltage modulation.
- **Layout orientation matters**: always verify that MOS and poly-poly capacitors are connected in the orientation intended by the circuit designer, as the two electrodes are never interchangeable due to asymmetric parasitics.
- **Gate resistance in MOS capacitors**: contact poly at both ends (reduces lumped resistance by $4\times$). If rules allow contacts over active gate, virtually eliminate gate resistance entirely.
- **Poly stringers**: in older poly-poly processes, poly-2 can form conductive filaments along poly-1 edges. Modern processes avoid this by patterning poly-2 before poly-1.

## Relation to the Bigger Picture

This section provides the foundational theory and practical design knowledge for all capacitor structures available in analog IC processes, connecting the material properties (Chapter 2: oxide growth, deposition) with the device-level concerns (Chapter 4: process architectures) and reliability mechanisms (Chapter 5: TDDB, ESD). Understanding capacitor variability, voltage modulation, and parasitics is essential for the matching and layout techniques covered in Chapter 8, where ratio-dependent circuits rely on precisely matched capacitors rather than absolute values. The section also sets up the contrast with inductors ([[ch07-inductance]]), which are far more difficult to integrate and suffer from severe parasitic losses that capacitors largely avoid.

## See Also
- [[ch07-inductance]]

