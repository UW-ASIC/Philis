---
title: "6.3 Resistor Variability"
chapter: 6
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-6, resistors, variability, TCR, nonlinearity, contact-resistance, hydrogenation]
---

# 6.3 Resistor Variability

> **Chapter 6: Resistors**

## Key Concepts

The value of an integrated resistor is not a fixed number -- it depends on numerous factors that the layout designer must understand and manage. The most significant sources of variability are:

1. **Process variation** -- fluctuations in sheet resistance and linewidth during fabrication
2. **Temperature variation** -- resistivity changes with operating temperature
3. **Nonlinearity (voltage modulation)** -- resistance changes with applied voltage
4. **Contact resistance** -- parasitic resistance added by the metal-to-resistor contacts

Less-significant factors that primarily affect **matching** (rather than absolute value) include orientation, stress/temperature gradients, thermoelectric effects, nonuniform etch rates, NBL push, voltage modulation, charge spreading, hydrogen compensation, and PSG polarization. These matching-related effects are treated separately in Chapter 8.

The key insight is that a resistor's "value" is not a single number but a function of process corner, temperature, applied voltage, and layout geometry. A good analog layout minimizes the impact of all these factors through careful choice of width, length, materials, and contact design.

---

## 6.3.1 Process Variation

### Sheet Resistance and Linewidth Control

A resistor's value depends primarily on its **sheet resistance** $R_s$ and its **effective width** $W$. Both vary due to fabrication tolerances:

- **Sheet resistance** varies due to fluctuations in film thickness, doping concentrations, doping profiles, and annealing conditions. Wafer fabs specify control limits, typically $\pm 15\%$ to $\pm 25\%$ for most resistor types, increasing to as much as $\pm 40\%$ for very high-sheet poly resistors and base pinch resistors.
- **Linewidth control** $\Delta W$ is the variation in the effective width of a given material:
  - For **deposited layers** (e.g., polysilicon): $\Delta W \approx \pm 10\%$ of minimum feature size. A CMOS process with $0.35\,\mu\text{m}$ gate length achieves poly linewidth control of about $\pm 0.035\,\mu\text{m}$.
  - For **diffused layers**: $\Delta W \approx \pm 10\%$ of minimum feature size $+ \pm 10\%$ of junction depth $x_j$.

### Tolerance Equation

Given the sheet resistance control $\Delta R_s$ (in percent) and linewidth control $\Delta W$, the overall resistor tolerance $\Delta R$ (in percent) is:

$$\Delta R = \pm\sqrt{(\Delta R_s)^2 + \left(\frac{2 \Delta W}{W}\right)^2} \tag{6.12}$$

where $W$ is the effective width of the resistor. The factor of 2 in front of $\Delta W$ arises because linewidth error affects both edges. **Narrower resistors exhibit more variation** because $\Delta W / W$ becomes larger. For example, with $\Delta R_s = \pm 20\%$ and $\Delta W = \pm 0.035\,\mu\text{m}$:

| Resistor Width | Tolerance |
|---|---|
| $0.5\,\mu\text{m}$ | $\pm 23\%$ |
| $1.0\,\mu\text{m}$ | $\pm 21\%$ |
| $5.0\,\mu\text{m}$ | $\pm 20\%$ |

### Special Width-Dependent Effects

- **Bamboo effect in polysilicon**: When poly width approaches the grain diameter ($\sim 0.1\,\mu\text{m}$), grains can grow entirely across the lead, and grain boundaries impede current flow, causing a dramatic increase in sheet resistance. This is most pronounced below doping concentrations of about $1 \times 10^{19}\,\text{cm}^{-3}$. Keep poly resistors wider than about $0.3\,\mu\text{m}$ to avoid this.
- **Titanium silicide narrow-line effect**: Below about $0.5\,\mu\text{m}$ linewidth, $\text{TiSi}_2$ cannot recrystallize from the high-resistivity C49 phase ($60$--$80\,\mu\Omega\cdot\text{cm}$) to the low-resistivity C54 phase ($13$--$16\,\mu\Omega\cdot\text{cm}$) because insufficient space exists for the larger C54 grains. This limits Ti-silicided low-sheet poly resistors to widths $\geq 0.5\,\mu\text{m}$.
- **Diffusion dilution**: Diffused resistors increase in resistance as their width approaches the junction depth $x_j$, because outdiffusion reduces the dopant concentration within narrow resistors. The width bias $\Delta W$ that works for wider resistors underestimates resistance for widths less than about $2 x_j$.

### Design Guidelines for Width Selection (Tolerance-Driven)

| Tolerance Requirement | Guideline |
|---|---|
| Tolerance doesn't matter | Use minimum widths from layout rules |
| Moderate concern | Use $2$--$3\times$ minimum width. Poly $\geq 0.5\,\mu\text{m}$. Diffused width $\geq 2 x_j$. Avoid very high-sheet poly and pinch resistors. |
| Crucial tolerance | Consider trimming. Poly $\geq 0.5\,\mu\text{m}$. Do not use lightly doped resistors of any type (high-sheet poly, wells, epi layers, pinch resistors) due to significant voltage variation. |

---

## 6.3.2 Temperature Variation

### Linear TCR Model

Resistivity is a nonlinear function of temperature, but a linear approximation usually suffices:

$$R(T) = R(T_0) \cdot [1 + \alpha(T - T_0)] \tag{6.13}$$

where $R(T)$ is resistance at desired temperature $T$, $R(T_0)$ is resistance at reference temperature $T_0$, and $\alpha$ is the **temperature coefficient of resistance (TCR)**. Since $\alpha$ is small, it is typically expressed in $\text{ppm}/{}^{\circ}\text{C}$.

### Table of Typical TCR Values

| Material | TCR ($\text{ppm}/{}^{\circ}\text{C}$) |
|---|---|
| Aluminum, bulk | $+3900$ |
| Copper, bulk | $+4000$ |
| Gold, bulk | $+3700$ |
| Cobalt disilicide ($1.2\,\text{k\AA}$) | $+3400$ |
| $180\,\Omega/\square$ Base diffusion | $+1300$ |
| $7\,\Omega/\square$ Emitter diffusion | $+400$ |
| $3\,\text{k}\Omega/\square$ Base pinch diffusion | $+3500$ |
| $3\,\text{k}\Omega/\square$ HSR implant (boron) | $-3000$ |
| $500\,\Omega/\square$ Poly (phosphorus-doped, $4\,\text{k\AA}$) | $-1000$ |
| $25\,\Omega/\square$ Poly (phosphorus-doped, $4\,\text{k\AA}$) | $>1000$ |
| $10\,\text{k}\Omega/\square$ N-well | $-6000$ |

### Why TCR Has the Sign It Does

- **Most resistor materials have positive TCR**: Higher temperatures increase lattice scattering, reducing carrier mobilities, increasing resistivity.
- **High-sheet polysilicon is an exception (negative TCR)**: Its resistivity is dominated by grain boundaries, not bulk mobility. Higher temperatures increase the probability that carriers can surmount grain boundary barriers by tunneling or detrapping, thus *decreasing* resistance.

### Quadratic TCR Model

For greater accuracy or wide temperature ranges:

$$R(T) = R(T_0) \cdot [1 + \alpha_1(T - T_0) + \alpha_2(T - T_0)^2] \tag{6.14}$$

where $\alpha_1$ is the linear TCR and $\alpha_2$ is the quadratic TCR ($\text{ppm}/{}^{\circ}\text{C}^2$). The quadratic coefficient is usually much smaller than the linear coefficient but can matter for large temperature swings. Coefficients derived by least-squares regression are optimized for a specific temperature range; using them outside that range degrades accuracy.

### Practical Temperature Compensation

- Circuit designers sometimes connect low-sheet poly (positive TCR) in series or parallel with high-sheet poly (negative TCR) to create resistors with near-zero overall TCR.
- **Caution**: The TCR of high-sheet poly depends strongly on processing conditions and is not typically monitored by the fab, so it can vary more than expected.
- Series-parallel composite resistor networks can be designed to be relatively insensitive to small variations in TCR or resistance.

---

## 6.3.3 Nonlinearity and Conductivity Modulation

### Absolute vs. Incremental Resistance

For a **nonlinear** resistor, two distinct "resistance" quantities exist:

- **Absolute (static, DC) resistance**: $R_{abs} = V / I$
- **Incremental (dynamic, small-signal) resistance**: $r = dV / dI$

For a linear resistor these are equal. For a nonlinear resistor they are not. Unless stated otherwise, "resistance" means the absolute resistance.

A passive resistor cannot have negative absolute resistance, but it *can* have a region of negative incremental resistance.

### Voltage Modulation Model

Voltage modulation is modeled quadratically:

$$R(V) = R(V_0) \cdot [1 + \alpha_V(V - V_0) + \beta_V(V - V_0)^2] \tag{6.15}$$

where $\alpha_V$ is the linear voltage coefficient ($\text{ppm/V}$) and $\beta_V$ is the quadratic voltage coefficient ($\text{ppm/V}^2$).

### Sources of Nonlinearity

#### 1. Self-Heating

Any resistor with appreciable TCR will exhibit voltage modulation due to power dissipation increasing its temperature. For a poly resistor of width $W$ and sheet resistance $R_s$, the quadratic voltage modulation coefficient is:

$$\beta_V = \frac{\alpha \cdot R_s}{R \cdot W^2} \cdot \frac{t_{ox}}{k_{ox}} \tag{6.16}$$

where $\alpha$ is the TCR, $R$ is the resistance, $t_{ox}$ is the oxide thickness beneath the resistor, and $k_{ox}$ is the thermal conductivity of $\text{SiO}_2$ ($\approx 1.4\,\text{W/m}\cdot{}^{\circ}\text{C}$). For a $5\,\mu\text{m}$-wide, $500\,\Omega/\square$ poly resistor with TCR $= -1000\,\text{ppm}/{}^{\circ}\text{C}$ atop $1\,\mu\text{m}$ field oxide, this gives $\beta_V \approx -70\,\text{ppm/V}^2$. The negative sign means resistance *decreases* with voltage (a consequence of the negative TCR).

#### 2. High-Field Velocity Saturation

Carrier mobility drops at electric field intensities above about $10\,\text{kV/cm}$ for electrons and $30\,\text{kV/cm}$ for holes. To avoid nonlinearity at voltages up to $V_{max}$, the minimum resistor length $L_{min}$ should satisfy:

$$L_{min} = \frac{V_{max}}{5\,\text{kV/cm}} \quad \text{(N-type)} \tag{6.17}$$

$$L_{min} = \frac{V_{max}}{15\,\text{kV/cm}} \quad \text{(P-type)} \tag{6.18}$$

(with a safety factor of 2 applied to the critical fields).

#### 3. Polysilicon Grain Boundary Effects

Short poly resistors can exhibit nonlinearity if appreciable voltage drops appear across individual grains. The barrier resistance between grains becomes voltage-dependent. This can be neglected if resistor length $\geq 1000 \times$ grain diameter. Given typical grain diameters of $\sim 0.1\,\mu\text{m}$, resistors longer than about $100\,\mu\text{m}$ are safe. In practice, nonlinearities remain manageable down to about $30\,\mu\text{m}$.

#### 4. Depletion Region Intrusion (Junction-Isolated Resistors)

In lightly doped diffused resistors, depletion regions widen toward the high-voltage end, narrowing the resistor body. This phenomenon is called **pinching**. Typical voltage coefficients:
- Base resistors: $\alpha_V \approx 100$--$200\,\text{ppm/V}$
- High-sheet resistors: $\alpha_V \approx 500$--$2000\,\text{ppm/V}$
- Base pinch resistors: $\alpha_V$ and $\beta_V$ are so large that the resistor nearly doubles in value at $5\,\text{V}$ -- better modeled as a JFET

High-sheet poly has $\sim 200\times$ higher doping than equivalent-sheet monocrystalline Si, so it shows only $\sim 0.5\%$ of the depletion intrusion effect.

#### 5. Conductivity Modulation (Tank/Body Modulation)

The voltage difference between a diffused resistor and its enclosing tank (or well) modulates the resistor value, analogous to backgate modulation in a MOSFET. Example: a $200\,\Omega/\square$ P-type resistor shows tank modulation coefficients of $\alpha_V \approx 1000\,\text{ppm/V}$ and $\beta_V \approx -200\,\text{ppm/V}^2$. Emitter resistors show negligible tank modulation; HSR and pinch resistors show very pronounced effects.

#### 6. Lead Crossover Effects

Metal leads crossing a resistor create an electric field that causes accumulation or depletion at the resistor surface, modulating its conductivity. This can cause $\pm 5\%$ variation in $3\,\text{k}\Omega/\square$ HSR. **Accurate HSR resistors should be field plated** (split field plates are recommended, Section 8.2.9). Base, emitter, and low-sheet poly resistors ($\leq 200\,\Omega/\square$) are largely unaffected due to their higher surface doping.

---

## 6.3.4 Contact Resistance

Every resistor has at least two contacts, each adding parasitic resistance from two sources: (1) a potential barrier between the contact metal and the resistor material, and (2) current crowding at the contact.

### Contact Resistance Equation

The resistance $R_c$ added by one contact of width $W_c$ and length $L_c$ (where $L_c$ is oriented along the resistor length) is:

$$R_c = \frac{\sqrt{\rho_c \cdot R_s}}{W_c} \cdot \coth\left(L_c \sqrt{\frac{R_s}{\rho_c}}\right) \tag{6.19}$$

where $R_s$ is the sheet resistance of the resistor material and $\rho_c$ is the **specific contact resistance** (units: $\Omega\cdot\mu\text{m}^2$).

### Typical Specific Contact Resistances

| Contact System | $\rho_c\,(\Omega\cdot\mu\text{m}^2)$ |
|---|---|
| Al(Cu,Si) to $180\,\Omega/\square$ (111) base | 750 |
| Refractory barrier metal to $180\,\Omega/\square$ (111) base | 2500 |
| PtSi to $180\,\Omega/\square$ (111) base | 1250 |
| Al(Cu,Si) to $7\,\Omega/\square$ (111) emitter | 40 |
| TiSi$_2$ to (100) NSD | 30 |
| TiSi$_2$ to (100) PSD | 100 |

### Practical Impact

- For a $200\,\Omega/\square$ base resistor with $2\,\mu\text{m} \times 2\,\mu\text{m}$ contacts using Al-Cu-Si metallization, each contact adds $\sim 4.5\,\Omega$, so two contacts add $\sim 9\,\Omega$ total -- about 9% of a $100\,\Omega$ resistor.
- Resistors with fewer than $\sim 10$ squares per segment may need oversized contacts to avoid excessive contact resistance variability.
- Placing several longer resistor segments in parallel is usually better than using dogbone-style enlarged contact heads.

### Evolution of Contact Systems

The older Al-Cu-Si system had significant contact resistance, especially for lightly doped materials. Refractory barrier metallization (RBM) increased both resistance and its variability due to lack of alloying. Adding silicide beneath the barrier metal reduced resistance *and* variability. Virtually all modern contact systems use some form of silicide.

---

## 6.3.5 Hydrogenation and Dehydrogenation

High-sheet P-type poly resistors can exhibit a **slow drift (increase) in resistance** when operated at high temperatures for extended periods. Shifts of several percent over hundreds to thousands of hours have been observed.

### Mechanism

1. **Grain boundaries in polysilicon contain dangling bonds** that act as carrier traps.
2. **Hydrogen** (a byproduct of poly deposition and plasma nitride overcoats) reacts with dangling bonds, eliminating traps. This **hydrogenation reduces sheet resistance** because the traps would otherwise capture carriers and create potential barriers that impede current flow.
3. About $\sim 1\%$ of the Si-H bonds at grain boundaries have weak bond energies ($\sim 0.5\,\text{eV}$). At high temperatures or under hot-carrier stress, these bonds rupture, regenerating the traps and **increasing resistivity**. This is **dehydrogenation drift**.

### Phosphorus Compensation

- **N-type (phosphorus-doped) poly** exhibits reduced dehydrogenation drift because phosphorus segregates at grain boundaries and ties off dangling bonds (like hydrogen does, but more permanently).
- **Compensated P-type poly** (containing more boron than phosphorus) actually shows *greater* drift than boron-only devices. Boron-phosphorus complexes form at grain boundaries, acting as hole traps. These complexes can be hydrogenated, but the resulting bonds are extremely weak ($\sim 0.3\,\text{eV}$) and dehydrogenate readily.

### Design Implication

This drift is of particular concern for **precision current sources**, where dehydrogenation can cause current decreases of up to several percent over the product lifetime.

---

## Diagrams

### Figure 6.7 -- Voltage vs. Current for Linear and Nonlinear Resistors

![[diagrams/ch06-resistor-variability-fig1.png]]

**Caption**: (A) Linear resistor: $V/I$ is constant and equals the slope $dV/dI$. (B) Nonlinear resistor: the absolute resistance $R_{abs} = V/I$ differs from the incremental resistance $r = dV/dI$. (C) A nonlinear resistor possessing a region of negative incremental resistance -- passive devices can exhibit this even though negative absolute resistance is physically impossible. This page also presents the voltage modulation model (Eq. 6.15) and the self-heating-induced voltage modulation coefficient (Eq. 6.16).

### Figure 6.8 -- Base Pinch Resistor Cross Section Showing Depletion Intrusion

![[diagrams/ch06-resistor-variability-fig2.png]]

**Caption**: Cross section of a base pinch resistor. Depletion regions from both the collector-base and emitter-base junctions intrude into the neutral base. The intrusion is worse at the high-voltage end because the reverse bias is larger there. This pinching effect causes pronounced nonlinearity -- the resistor value can nearly double at 5 V. Also shown: introduction to conductivity modulation and tank modulation effects, and the start of Section 6.3.4 on contact resistance.

### Table 6.4 -- Temperature Coefficients of Resistivity

![[diagrams/ch06-resistor-variability-fig3.png]]

**Caption**: Table 6.4 listing typical TCR values for common IC materials (valid $-55$ to $125\,{}^{\circ}\text{C}$), along with width selection design guidelines and the start of Section 6.3.2 on temperature variation. Note the negative TCR of high-sheet poly and N-well, contrasting with the positive TCR of bulk metals and diffused resistors.

---

## Practical Takeaways

- **Width selection is a tolerance trade-off**: Wider resistors cost area but dramatically reduce the impact of linewidth variation. Use Eq. 6.12 to quantify.
- **Avoid poly widths below $0.3\,\mu\text{m}$** to prevent the bamboo effect, and below $0.5\,\mu\text{m}$ for Ti-silicided poly to prevent the C49/C54 phase issue.
- **Avoid diffused resistor widths below $2 x_j$** to prevent dilution-induced resistance increase.
- **High-sheet poly and pinch resistors are poor choices when tight tolerance or low voltage modulation is needed** -- they suffer from large process variation, large voltage coefficients, and (for poly) dehydrogenation drift.
- **Field plate HSR resistors** (split field plates preferred) to shield them from conductivity modulation by overlying leads.
- **Series/parallel combinations of low- and high-sheet poly** can create near-zero TCR, but beware of fab-to-fab variation in high-sheet poly TCR since fabs rarely monitor it.
- **Contact resistance matters most for short, lightly-doped resistors** (few squares). Use silicided contacts when available. For base resistors under 10 squares, consider oversized contacts or parallel segments.
- **For precision current sources using high-sheet P-type poly**, account for dehydrogenation drift over product lifetime. Prefer N-type poly where possible, or avoid compensation doping in P-type poly.
- **Minimum resistor length** must satisfy both velocity saturation constraints (Eqs. 6.17/6.18) and grain-boundary nonlinearity constraints ($L > 30$--$100\,\mu\text{m}$ for poly).

---

## Relation to the Bigger Picture

Section 6.3 bridges the gap between the idealized resistor models of [[ch06-resistivity-sheet-resistance]] and the practical realities of analog IC design. While Section 6.1--6.2 show how to calculate a resistor's nominal value from geometry and sheet resistance, this section reveals all the ways that value can shift in practice -- through process variation, temperature, applied voltage, contact parasitics, and long-term drift. Understanding these variability mechanisms is essential for choosing appropriate resistor types, widths, and layouts, which directly feeds into the matching and parasitic considerations of [[ch06-resistor-parasitics]] and the matching techniques of Chapter 8. For any analog circuit where resistor accuracy matters (current references, voltage dividers, gain-setting networks), this section provides the quantitative tools to predict and control resistor tolerance.

---

## See Also
- [[ch06-resistivity-sheet-resistance]]
- [[ch06-resistor-parasitics]]
