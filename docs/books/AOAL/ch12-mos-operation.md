---
title: "12.1 MOS Transistor Operation"
chapter: 12
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-12, mosfet, transconductance, threshold-voltage, channel-length-modulation, velocity-saturation, subthreshold, GIDL, breakdown]
---

# 12.1 MOS Transistor Operation

> **Chapter 12: Field-Effect Transistors**

## Key Concepts

### The Shichman-Hodges Model and Regions of Operation

A MOS transistor's behavior is governed by the relationship between the gate-to-source voltage $V_{GS}$, the drain-to-source voltage $V_{DS}$, and the threshold voltage $V_t$. The **Shichman-Hodges equations** divide operation into two primary regions:

- **Linear (triode) region**: When $V_{DS}$ is less than the effective gate voltage $(V_{GS} - V_t)$, drain current depends on both $V_{GS}$ and $V_{DS}$.
- **Saturation region**: When $V_{DS}$ exceeds the effective gate voltage, drain current becomes approximately independent of $V_{DS}$. The saturation drain current is:

$$I_D = \frac{k}{2}(V_{GS} - V_t)^2$$

where $k$ is the **device transconductance** (not to be confused with small-signal transconductance $g_m$).

A critical subtlety: the source and drain of a MOS transistor are determined by **electrical biasing**, not by terminal markings. If one tries to make $V_{DS}$ negative, the drain and source simply swap roles. The Shichman-Hodges equations always apply after redefining terminals to match the actual bias conditions.

### Small-Signal Transconductance vs. Device Transconductance

The small-signal transconductance $g_m$ of a MOS transistor in saturation is:

$$g_m = \sqrt{2 k I_D}$$

Compare this with the bipolar transistor's transconductance:

$$g_m^{BJT} = \frac{I_C}{V_T}$$

A MOS transistor's $g_m$ depends on **both** drain current and device transconductance $k$, which in turn depends on $W/L$. This gives the designer an extra degree of freedom: $g_m$ can be tuned by adjusting geometry, not just bias current. However, there is a limit -- if $k$ is made too large the transistor enters subthreshold, where $g_m$ becomes constant and always less than that of a bipolar at the same current. This is a fundamental drawback of MOS in analog applications.

---

## Device Transconductance ($k$)

The device transconductance quantifies transistor size, analogous to $I_S$ for a bipolar. It relates to geometry by:

$$k = k' \frac{W_{eff}}{L_{eff}}$$

where $k'$ is the **process transconductance**, and the effective dimensions are:

$$W_{eff} = W_{drawn} - \Delta W$$
$$L_{eff} = L_{drawn} - \Delta L$$

$\Delta W$ and $\Delta L$ are width and length process biases accounting for overetching and electrical edge effects (typically a few tenths of a micron -- negligible for large transistors).

### Estimating Process Transconductance

If no measured value is available:

$$k' = \mu_{eff} \frac{\varepsilon_0 \varepsilon_{ox}}{t_{ox}}$$

where:
- $\mu_{eff}$ = effective surface mobility (reduced from bulk values due to surface scattering)
- $\varepsilon_0 = 8.854 \times 10^{-14}$ F/cm (permittivity of free space)
- $\varepsilon_{ox} \approx 3.9$ for pure SiO$_2$
- $t_{ox}$ = capacitance-equivalent oxide thickness (slightly larger than physical thickness due to quantum-mechanical effects adding ~0.4 nm and polysilicon depletion)

### Effective Mobilities (surface-channel, (100) silicon, 27 C)

Approximate effective mobilities for electrons and holes are given by Equations 12.10 and 12.11 in the text. These depend on gate oxide thickness and yield units of cm$^2$/V$\cdot$s when $t_{ox}$ is in nanometers. Effective mobility depends on $V_{GS}$, which is why different values apply in linear vs. saturation regions.

### PMOS vs. NMOS Sizing

Because hole mobility is roughly one-third that of electron mobility, a PMOS transistor requires approximately **three times** the $W/L$ ratio of an NMOS to achieve the same transconductance. This is visible even in minimum-size logic gates, where PMOS transistors are enlarged for symmetric drive.

### Temperature Dependence

Device transconductance decreases as temperature rises because lattice vibrations increase carrier scattering. Effective mobilities scale approximately as $T^{-1.5}$ for electrons and $T^{-1}$ for holes. At 125 C, device transconductance is roughly **60%** of its room-temperature value.

This has led to claims that MOS transistors are immune to thermal runaway -- but these claims ignore the negative temperature coefficient of $V_t$ (covered in Section 13.1.3).

### Gate Oxide and Dielectric Strength

- Dielectric strength of gate oxide: ~$10$ MV/cm
- For oxides thicker than 30 nm: derate to $\leq 5$ MV/cm due to time-dependent dielectric breakdown
- For oxides $\leq 15$ nm: can safely operate at $\sim 7$ MV/cm
- Thin gate oxides maximize $k'$, but tunneling leakage becomes a concern below ~2 V operating voltage

### Berkeley $k$ Convention

The device transconductance in Equation 12.9 matches the level-1 SPICE MOS model (the "Berkeley $k$"). Some authors use an alternative definition equal to half the Berkeley $k$ and adjust the Shichman-Hodges equations accordingly.

---

## Threshold Voltage ($V_t$)

The threshold voltage is the gate-to-source voltage needed to just establish a channel beneath the gate dielectric (with backgate tied to source). It is governed by multiple factors, all of which contribute variability.

### Enhancement vs. Depletion Devices

| Type | Channel at $V_{GS} = 0$? | $V_t$ Sign | Switch Analogy |
|------|---------------------------|-----------|----------------|
| Enhancement NMOS | No | Positive | Normally Open |
| Enhancement PMOS | No | Negative | Normally Open |
| Depletion NMOS | Yes | Negative | Normally Closed |
| Depletion PMOS | Yes | Positive | Normally Closed |

Most processes optimize for enhancement-mode transistors (needed for current mirrors, CMOS logic). Analog processes often offer at least one polarity of depletion device.

### Factors Affecting $V_t$

**1. Gate-Backgate Contact Potential**
Depends on gate material and backgate doping. Substituting $N^+$ poly for $P^+$ poly increases the contact potential by ~1.1 V, shifting $V_t$ accordingly. This is why early CMOS processes used $N^+$ poly gates for both NMOS and PMOS -- a single threshold adjust implant could then set both thresholds.

**2. Backgate Doping**
Heavier doping requires a stronger gate field to invert the surface, increasing $|V_t|$. Table 12.3 in the text quantifies this for various oxide thicknesses.

**3. Gate Dielectric Thickness and Composition**
Thicker dielectrics require more voltage to attract channel charge (proportional to $t_{ox}$, inversely proportional to $\varepsilon_{ox}$). Modern trends:
- Pure SiO$_2$: standard for analog, $\varepsilon_r \approx 3.9$
- **Oxynitride**: higher $\varepsilon_r$, allows thicker dielectric with less tunneling leakage; common for 1.5-2.5 V rated oxides, but increases bias temperature instability
- **HfO$_2$ (high-$\kappa$)**: $\varepsilon_r \approx 23$, introduced at Intel's 45 nm node (2007). Incompatible with polysilicon gates (requires metal gates, typically hafnium). Contains defects causing high BTI. Doping with Si/N reduces $\varepsilon_r$ to 16-19 but mitigates BTI. Not yet common in analog processes.

**4. Backgate Bias (Backgate Modulation)**
Reverse-biasing the source-backgate junction widens the depletion region, making inversion harder. Backgate modulation is sometimes deliberately used to increase $|V_t|$, but it also increases random $V_t$ variation (Section 13.2.1).

**5. Oxide Charges**

| Charge Type | Symbol | Character | Key Details |
|-------------|--------|-----------|-------------|
| Fixed oxide charge | $Q_f$ | Always positive | Near Si surface; depends on crystal orientation ((111) >> (100)); eliminated by high-temp inert anneal |
| Oxide trapped charge | $Q_{ot}$ | + or - | Created by radiation, HCI, intense fields, Fowler-Nordheim tunneling; generally smaller than $Q_{it}$ |
| Mobile oxide charge | $Q_m$ | Positive (Na$^+$, K$^+$, Li$^+$) | Migrates under electric field; modern processes have reduced this to negligible levels via poly gates and chlorinated oxidation |
| Interface trapped charge | $Q_{it}$ | + or - | Dangling bonds at SiO$_2$/Si interface; (111) >> (100); reduced by H$_2$ anneal but regenerated by HCI; increases $1/f$ noise |

The historical "surface state charge" $Q_{ss}$ lumped $Q_f$ and $Q_{it}$ together -- this is now deprecated because the two behave very differently ($Q_f$ is fixed and positive; $Q_{it}$ varies in sign and shifts over time).

**6. Hydrogen Compensation**
In transistors with boron-doped backgates, hydrogen can deactivate boron acceptors, making surface charge more positive and decreasing $V_t$.

**7. Threshold Adjust Implants**
Deliberate shallow implants inject dopant atoms just beneath the gate oxide to shift $V_t$. A boron implant injects negative charge, raising the NMOS threshold and reducing the PMOS threshold magnitude equally (if oxide thicknesses match). Example from text: natural thresholds of +0.15 V (NMOS) and -1.15 V (PMOS) can be shifted by +0.35 V to yield +0.50 V and -0.80 V.

### Threshold Voltage Equation

All factors combine into:

$$V_t = \phi_{ms} - 2\phi_B - \frac{Q_d}{C_{ox}} - \frac{Q_{ss}}{C_{ox}}$$

where:
- $\phi_{ms}$ = gate-backgate contact potential
- $\phi_B$ = bulk potential = $V_T \ln(N_A / n_i)$
- $Q_d$ = depletion charge per unit area = $-\sqrt{2 q \varepsilon_{Si} N_A (2\phi_B - V_{BS})}$
- $Q_{ss}$ = surface state charge per unit area (including threshold adjust)
- $C_{ox} = \varepsilon_{ox} / t_{ox}$

### Threshold Voltage Variability

- Manufacturing variation: ~$\pm 30$ mV (with care)
- Temperature coefficient: typically $-1$ to $-3$ mV/C (magnitude of $V_t$ decreases as temperature rises)
- Over $-40$ C to $175$ C: ~$\pm 100$ mV from temperature alone
- Total practical variation: ~$\pm 130$ mV
- **Critical minimum**: $|V_t|$ should never drop below ~$4S$ (four times the subthreshold swing, ~300 mV) or the device will have significant subthreshold leakage at $V_{GS} = 0$

---

## Additional Modeling Considerations (12.1.3)

The Shichman-Hodges model is a first approximation. Modern submicron transistors require more sophisticated models (BSIM family). The key limitations are described below.

### Channel-Length Modulation (CLM)

As $V_{DS}$ increases beyond saturation, the pinched-off region at the drain widens, effectively shortening the channel. A shorter channel increases $k$ and therefore $I_D$. This is the MOS analog of the **Early effect** in bipolar transistors.

The corrected saturation current:

$$I_D = \frac{k}{2}(V_{GS} - V_t)^2 (1 + \lambda V_{DS})$$

where $\lambda$ is the **channel-length modulation parameter**. If the slightly tilted saturation-region I-V curves are extrapolated to intersect the $V_{DS}$ axis, the intercept voltage magnitude is $1/\lambda$, directly analogous to the Early voltage $V_A$.

**Output resistance:**

$$r_o = \frac{1}{\lambda I_D}$$

Since $r_o$ limits voltage gain, analog designers prefer small $\lambda$. Two strategies:
1. **Increase backgate doping** to narrow the pinched-off region
2. **Lengthen the channel** -- $r_o$ scales approximately linearly with drawn length. Lengths of $5$-$20 \;\mu$m are common in analog designs.

> **Warning**: Transistors with pocket implants do NOT exhibit the expected increase in $r_o$ with channel length and are thus ill-suited for many analog applications.

### Velocity Saturation

The Shichman-Hodges model assumes mobility is independent of lateral electric field. For channel lengths below a few microns, lateral fields become intense enough to saturate carrier velocities.

**In the linear region**, the effective channel length increases by:

$$\Delta L_{vsat} = \frac{V_{DS}}{2 E_{crit}}$$

where $E_{crit}$ is the critical field for surface carrier velocity saturation. Using Sodini's value of $E_{crit} = 8 \times 10^3$ V/cm for electrons and $V_{DS} = 100$ mV: a 10 $\mu$m channel lengthens by 1%, but a 1 $\mu$m channel lengthens by 100%. This confirms velocity saturation is negligible for $L > $ a few microns.

**In saturation**, the effective gate voltage $(V_{GS} - V_t)$ is replaced by a saturation voltage $V_{Dsat}$:

$$V_{Dsat} = \frac{(V_{GS} - V_t) \cdot E_{crit} \cdot L}{(V_{GS} - V_t) + E_{crit} \cdot L}$$

Key consequences:
- Drain current becomes **less than quadratically** dependent on $(V_{GS} - V_t)$
- For very short channels, $I_D$ approaches **linear** dependence on $(V_{GS} - V_t)$ and becomes **independent of channel length**
- Drain currents are diminished compared to Shichman-Hodges predictions

### Short-Channel Effect (SCE)

The gradual channel approximation (vertical-only field between gate and depletion region) breaks down when channel length approaches the depletion region thickness. Lateral fields from source-backgate and drain-backgate junctions reduce the charge the gate must supply, causing $|V_t|$ to **decrease** as the channel shrinks. Generally negligible for $L >$ a few microns.

**Drain-Induced Threshold Shift (DITS)** is a related phenomenon: as the drain-backgate and source-backgate depletions approach or merge, the drain's lateral field supplements the gate's field, reducing $V_t$ by an amount that depends on $V_{DS}$. DITS is one aspect of **Drain-Induced Barrier Lowering (DIBL)**.

At extreme drain voltages, DIBL becomes so severe that conduction occurs without any gate field -- this is **punchthrough**, a breakdown mode (Section 12.1.4).

### Narrow-Channel Effect

When channel width approaches the depletion region thickness, fringing fields at the channel edges reduce the gate's effectiveness, **increasing** $|V_t|$. This is the opposite direction from the short-channel effect. Analog designers rarely make transistors narrow enough to encounter this.

### Subthreshold Conduction

The Shichman-Hodges model predicts $I_D = 0$ at $V_{GS} = V_t$. In reality, a small current flows due to diffusion of minority carriers across the channel -- the physics closely parallels a forward-biased PN junction:

- **Weak inversion (subthreshold)**: Carriers move primarily by diffusion; $I_D$ increases exponentially with $V_{GS}$
- **Moderate inversion**: Drift begins to contribute; transition zone
- **Strong inversion**: Drift dominates; standard saturation behavior

The subthreshold drain current obeys:

$$I_D = I_0 \exp\left(\frac{V_{GS} - V_{off}}{n V_T}\right)$$

where $I_0$ is a proportionality constant, $V_{off}$ is an offset voltage, $V_T$ is the thermal voltage, and $n$ is the **subthreshold slope factor**:

$$n = 1 + \frac{\varepsilon_{Si} \cdot t_{ox}}{\varepsilon_{ox} \cdot x_d}$$

$x_d$ is the depletion region thickness beneath the gate:

$$x_d = \sqrt{\frac{2 \varepsilon_{Si} \cdot 2\phi_B}{q \cdot N_A}}$$

The **subthreshold swing** $S$ is the most important designer parameter:

$$S = n \cdot V_T \cdot \ln(10) \approx n \times 60 \;\text{mV/dec at 27 C}$$

- Theoretical minimum: $60$ mV/decade (at room temperature, when $n = 1$)
- Typical value: $80$-$100$ mV/decade
- For $L < 0.5\;\mu$m: incipient punchthrough increases $S$ to $\sim 100$-$150$ mV/decade

**Minimum threshold voltage guidelines:**
- Small transistors: $|V_t| \geq 4S \approx 300$ mV minimum
- Adding process variation margin (~$\pm 30$ mV) and temperature margin (~$\pm 100$ mV): nominal $V_t \approx 0.5$-$0.6$ V (NMOS), $-0.5$ to $-0.7$ V (PMOS)
- Power transistors: add another $\sim 3S$ (~$250$ mV) margin to avoid excessive leakage, yielding nominal $|V_t| \geq 0.8$ V (explains why LDMOS devices have high thresholds)

Analog designers sometimes deliberately bias transistors in subthreshold to exploit the exponential $I_D$-$V_{GS}$ relationship (mirroring bipolar behavior), but subthreshold currents are very small and such circuits may not function well beyond ~85 C.

### Four Regions of MOS Operation

| Region | NMOS Condition | PMOS Condition |
|--------|---------------|----------------|
| Cutoff | $V_{GS} < V_{FB}$ | $V_{GS} > V_{FB}$ |
| Subthreshold | $V_{FB} \leq V_{GS} < V_t$ | $V_{FB} \geq V_{GS} > V_t$ |
| Linear | $V_{GS} \geq V_t$, $V_{DS} < V_{GS} - V_t$ | $V_{GS} \leq V_t$, $V_{DS} > V_{GS} - V_t$ |
| Saturation | $V_{GS} \geq V_t$, $V_{DS} \geq V_{GS} - V_t$ | $V_{GS} \leq V_t$, $V_{DS} \leq V_{GS} - V_t$ |

### Gate-Induced Drain Leakage (GIDL)

GIDL is a leakage mechanism caused by Fowler-Nordheim tunneling through traps (dangling bonds) at the oxide-silicon interface. The vertical electric field from the gate adds to the horizontal field from the drain voltage. When the total field exceeds the Fowler-Nordheim onset threshold, leakage flows across the drain-backgate depletion region.

Key characteristics:
- Increases **exponentially** with drain-to-gate voltage and with temperature
- Most problematic when transistor is in **cutoff at high $V_{DS}$ and elevated temperature**
- Turning the transistor on reduces drain-to-gate voltage differential and suppresses GIDL
- **PMOS** is more susceptible than NMOS (NMOS has HCI-imposed field limits that inherently constrain GIDL)
- Any mechanism that creates interface traps (HCI, antenna effect, incomplete H$_2$ anneal) worsens GIDL

**Mitigation**: Reduce operating voltage, or decrease backgate doping to widen the drain-backgate depletion region (reducing field intensity).

---

## Breakdown in MOS Transistors (12.1.4)

MOS transistor operating voltages are limited by several breakdown mechanisms.

### Avalanche Breakdown vs. Punchthrough

$BV_{DSS}$ (breakdown with gate grounded) is determined by whichever occurs first:

- **Avalanche breakdown**: Intense drain-backgate field accelerates thermally generated carriers to impact-ionization velocities. Creates an abrupt, "hard" transition from cutoff to breakdown.
- **Punchthrough**: Drain-backgate depletion penetrates through the entire channel to touch the source region. The device acts like a pinched-off JFET and conducts regardless of gate voltage. Creates a gradual, "soft" transition -- undesirable because it increases leakage and decreases output resistance at high $V_{DS}$.

Device designers prefer avalanche over punchthrough characteristics.

### Snapback

In short-channel transistors, avalanche breakdown can trigger the parasitic lateral bipolar transistor inherent in the MOSFET structure:

1. As $V_{DS}$ approaches $BV_{DSS}$, avalanche multiplication creates electron-hole pairs
2. Holes flow through the backgate to backgate contacts, creating a voltage drop (IR debiasing)
3. When the backgate adjacent to the source debiases enough, the parasitic NPN (source = emitter, backgate = base, drain = collector) enters forward active
4. Positive feedback: more current $\rightarrow$ more impact ionization $\rightarrow$ more debiasing $\rightarrow$ more bipolar conduction
5. The device "snaps back" to a lower **sustain voltage**

The I-V curve folds back, creating a negative-resistance region between the **trigger voltage** (where slope becomes vertical) and the **sustain voltage** (where slope again becomes vertical at lower $V_{DS}$).

> **Danger**: Operating a MOS transistor in cutoff beyond its sustain voltage is risky if the circuit can supply enough current to sustain snapback. The transistor may be destroyed by overheating, electrical filamentation, or secondary breakdown.

### Impact Ionization Breakdown ($BV_{DSI}$)

With a nonzero $V_{GS}$ providing channel current, the trigger voltage for snapback decreases. $BV_{DSI}$ is measured at the gate voltage producing maximum substrate injection (typically $\sim V_{DD}/2$). It quantifies the drain voltage at which drain current increases by a defined amount (e.g., 20%).

$BV_{DSI}$ can be less than $BV_{DSS}$. Pass transistors and power switches where $BV_{DSI} < BV_{DSS}$ are vulnerable to hot-short failures. For such applications:
- $BV_{DSI}$ must exceed the maximum operating voltage
- Backgate resistance must be minimized (integrated backgate contacts, retrograde wells, buried layers)

### Gate Dielectric Breakdown

Gate-to-source dielectric breakdown can occur:
- **Instantaneously** at excessive $V_{GS}$
- **Gradually** via time-dependent dielectric breakdown at voltages below the instantaneous breakdown voltage
- Both result in short-circuit failure between gate and backgate

---

## Diagrams

### Figure 12.5 -- Subthreshold Conduction (Semilog $I_D$ vs. $V_{GS}$)

![[diagrams/ch12-mos-operation-fig1.png]]

Semilog plot of drain current versus gate-to-source voltage for an NMOS transistor. The subthreshold region appears as a straight line (exponential increase) on this semilog scale, bounded on the lower left by junction leakage and on the upper right by moderate inversion transitioning into strong inversion (saturation). The subthreshold swing $S$ is the inverse of the slope of this line, measured in mV/decade. The cross-section diagrams (A, B, C) illustrate the physical mechanism: at flatband, the source-backgate junction is unbiased; the gate field slightly forward-biases the surface of this junction, injecting minority carriers that diffuse to the drain.

### Figure 12.2 -- Four Types of MOS Transistors

![[diagrams/ch12-mos-operation-fig2.png]]

Cross sections of the four MOS transistor types: (A) enhancement NMOS ($V_t > 0$), (B) enhancement PMOS ($V_t < 0$), (C) depletion NMOS ($V_t < 0$), and (D) depletion PMOS ($V_t > 0$). Enhancement devices have no channel at $V_{GS} = 0$ and require bias to turn on. Depletion devices have a built-in channel and require bias to turn off.

### Figure 12.7 -- Snapback Breakdown

![[diagrams/ch12-mos-operation-fig3.png]]

Breakdown characteristics showing the snapback phenomenon. At the trigger voltage, avalanche-induced substrate current activates the parasitic bipolar, causing the I-V curve to fold back to the lower sustain voltage. The negative-resistance region between trigger and sustain voltages makes operation in this regime dangerous. Also shows GIDL discussion and the distinction between avalanche (hard) and punchthrough (soft) breakdown characteristics (Figure 12.6).

---

## Practical Takeaways

- **$g_m$ tuning**: MOS transistors offer an extra knob vs. bipolars -- adjust $W/L$ to set $g_m$ independently of $I_D$. But MOS $g_m$ is always less than bipolar $g_m$ at the same bias current.
- **PMOS sizing rule of thumb**: Make PMOS $W/L$ roughly 3x that of NMOS for equal transconductance.
- **Output resistance in analog**: Lengthen channels to $5$-$20\;\mu$m for high $r_o$. Avoid transistors with pocket implants in analog signal paths -- they break the $r_o \propto L$ scaling.
- **Threshold voltage margin**: Target nominal $|V_t| \geq 0.5$ V for small transistors and $\geq 0.8$ V for power devices to ensure negligible leakage across process corners and temperature.
- **Subthreshold design**: Possible to exploit exponential $I_D$-$V_{GS}$ for bipolar-like circuits, but currents are tiny and temperature range is limited.
- **GIDL mitigation**: Reduce $V_{DS}$, lower backgate doping, minimize interface trap density (proper H$_2$ anneal, avoid antenna violations).
- **Snapback protection**: Minimize backgate resistance with integrated contacts, retrograde wells, or buried layers. Ensure $BV_{DSI} \geq V_{DD,max}$ for pass transistors and power switches.
- **Gate oxide reliability**: Respect derated field limits ($\leq 5$ MV/cm for thick oxides, $\leq 7$ MV/cm for $t_{ox} \leq 15$ nm). Time-dependent dielectric breakdown is the long-term concern.
- **Crystal orientation**: Always use (100) silicon for CMOS to minimize both interface trapped charge and fixed oxide charge.
- **Temperature effects**: $k$ drops to ~60% at 125 C vs. 27 C; $|V_t|$ decreases with temperature. Both effects must be accounted for in worst-case analysis.

---

## Relation to the Bigger Picture

This section lays the theoretical foundation for everything that follows in Chapter 12. Understanding device transconductance, threshold voltage, and the deviations from ideal Shichman-Hodges behavior is essential before tackling transistor construction ([[ch12-constructing-cmos]]), matching, and power device layout. The concepts here -- particularly $g_m$ tuning via $W/L$, channel-length modulation, and subthreshold swing -- directly inform the sizing decisions and layout tradeoffs that analog designers face daily. The threshold voltage discussion also connects back to the semiconductor and device physics of [[ch01-mos-transistors]], providing quantitative detail on how process parameters (doping, oxide thickness, gate material, charges) translate into the $V_t$ values that constrain circuit design.

---

## See Also
- [[ch12-constructing-cmos]]
- [[ch01-mos-transistors]]
