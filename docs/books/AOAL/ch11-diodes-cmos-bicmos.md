---
title: "11.2-11.3 Diodes in CMOS/BiCMOS and Matching"
chapter: 11
book: "The Art of Analog Layout, 3rd Ed."
author: "Alan Hastings"
tags: [analog-layout, chapter-11]
---

# 11.2-11.3 Diodes in CMOS/BiCMOS and Matching

> **Chapter 11: Diodes**

## Key Concepts

### The CMOS Diode Problem

CMOS processes were never designed with forward-biased PN junctions in mind. The PN junctions of a MOS transistor are *supposed* to remain reverse-biased at all times. Consequently, CMOS processes make little or no provision to contain minority carrier injection or optimize forward-biased junction characteristics. Despite this, designers have found ways to combine existing process elements to create usable diodes -- primarily for ESD protection, antenna diodes, and voltage references.

In contrast, **analog BiCMOS** processes inherit enough layers (buried layers, sinkers, separate base diffusions) to construct multiple types of diodes: diode-connected transistors, emitter-base Zeners, base-collector power diodes, PMoat power diodes, and (depending on silicidation) Schottky diodes. The CDI (collector-diffused-isolation) analog BiCMOS process closely resembles standard bipolar in its diode capabilities.

### Diode-Connected MOS Transistors

All MOS processes can fabricate diode-connected MOS transistors. There are **six possible configurations** (three NMOS, three PMOS), differing in how the backgate is connected:

1. **Separate backgate** (Fig. 11.14A/D): Drain and gate tied together form the anode; source forms the cathode; backgate connects to substrate or an independent node. The body diodes $D_{DB}$ and $D_{SB}$ remain reverse-biased in both conduction and cutoff. Used in current mirrors and leakage clamps.

2. **Antiparallel body diode** (Fig. 11.14B/E): Backgate tied to the cathode (source). This places the drain/backgate body diode antiparallel to the MOS channel. Commonly seen in current mirrors. In isolated processes, occasionally used as antiparallel voltage clamps.

3. **Parallel body diode** (Fig. 11.14C/F): Backgate tied to the anode (drain+gate). The source/backgate body diode is in parallel with the MOS channel. Current splits between the MOS channel and the body diode depending on $V_{th}$ and current level. At high currents or high $V_{th}$, the body diode dominates. This configuration typically requires the NMOS to reside in an isolated P-type tank.

For PMOS, most P-substrate CMOS processes can build all three configurations, but cannot prevent **substrate injection** when the body diode conducts. The most common application (PMOS current mirror reference) uses the antiparallel configuration (Fig. 11.14E), biased to keep the body diode off.

### Why CMOS PN Diodes Are Limited

A traditional N-well CMOS process offers three monocrystalline PN junctions:

| Junction | Anode | Cathode | Key Limitation |
|----------|-------|---------|----------------|
| NSD/P-epi | P-epi (substrate) | NSD | Cathode must be below substrate potential |
| N-well/P-epi | P-epi (substrate) | N-well | Cathode must be below substrate potential |
| PSD/N-well | PSD | N-well | Forms base-emitter of a substrate PNP |

Despite limitations, both NSD/P-epi and PSD/N-well diodes serve as **ESD protection devices** and **antenna diodes**. The PSD/N-well diode also makes a useful Zener. Dual-well, triple-well, and quad-well processes provide additional junction options (NSD/P-well, NSD/SPWell, NSD/DNWell, NSD/SNWell).

## Important Details

### 11.2.1 CMOS Junction Diodes

#### The NSD/P-epi Diode

The simplest possible diode: a square of NSD with contacts inside it. The NSD forms the cathode; the P-type substrate is the anode (contacted elsewhere on the die). Used directly as an **antenna diode** (Section 5.1.6).

**ESD application:** For 2 kV HBM protection (peak current ~1.3 A, $\tau \approx 220$ ns), series resistance must not exceed ~3 $\Omega$ to keep peak voltage below ~5 V. ESD designers interdigitate narrow NMoat and PMoat strips in a dense rectangular array, achieving protection in roughly $50 \times 50\ \mu\text{m}^2$.

**Why it is so robust:** Current consists almost entirely of electrons injected from heavily doped NSD into lightly doped P-epi. These electrons diffuse downward to the epi/substrate interface and laterally tens of microns before recombining. This provides:
- **Distributed ballasting** -- current spreads over a large volume naturally
- **Heat dissipation away from the surface** -- away from the thermally fragile metal system

**Key layout rules:**
- Do NOT add silicide block for ballasting -- the inherent distributed ballasting is sufficient, and siliciding the moat regions helps minimize series resistance
- Maximize contacts and metal coverage
- Even though NSD/P-epi diodes are not intended to forward-bias during normal operation, they must still pass latchup testing if connected to a pin

**P-well variants:** Adding a P-well around the structure (NSD/P-well diode) reduces resistance but concentrates self-heating. Shallow retrograde P-well implants further constrain minority carriers. The net result: the area required for a given ESD protection level does not vary much regardless of whether a P-well is present.

#### The PSD/N-well Diode

One or more PSD regions placed inside an N-well. When forward-biased, it behaves as the base-emitter junction of a substrate PNP. At extreme ESD current densities, beta rolls off and it behaves as a true diode.

**ESD use:** Layout resembles the NSD/P-epi diode (interdigitated NMoat/PMoat strips) but enclosed in an N-well rectangle. Robustness is similar for the same reasons. Latchup precautions required if connected to a pin.

**Zener use:** Breakdown voltage was >20 V in early (lightly doped well) processes. Newer, more heavily doped wells have lower $V_{BR}$, but it never drops much below 7 V because soft Zener breakdown characteristics would cause unacceptable PMOS "leakage." All PSD/N-well "Zeners" are therefore really **avalanche breakdown** diodes.

Layout of PSD/N-well Zener (Fig. 11.16):
- PMoat anode geometry
- Enclosing N-well contacted by NMoat ring (cathode)
- **Poly field plate** ring over the PSD/N-well junction, tied to the anode -- creates a vertical E-field that widens the depletion region and forces avalanche breakdown beneath the surface
- Without the field plate, Zener walkout (and possibly walkback) will occur
- Metal field plates are much less effective due to thicker oxide

**Operating limits:** Not more than ~$100\ \mu\text{A}/\mu\text{m}$ of continuous current per linear micron of cathode periphery. Higher currents require interdigitated PSD/NSD strip arrays, with PSD always as the outermost strips for uniform NSD finger conduction.

**Robustness concern:** PSD/N-well Zeners are vulnerable to short high-current pulses. Energy dissipates in the limited depletion region volume. Depletion negates most N-well ballasting. Siliciding moat removes remaining ballasting. To improve robustness:
- In silicided processes: use silicide block mask, leaving at least $2\ \mu\text{m}$ of unsilicided implant between contact and drawn moat edges
- In non-silicided processes: increase moat-over-contact overlap by $2\ \mu\text{m}$ on all sides

#### Poly Diodes

PN junctions can be created in polysilicon, but poly diodes behave very differently from monocrystalline ones due to grain boundary effects.

**Polysilicon properties relevant to diodes:**
- Polysilicon is an aggregate of crystal grains with misaligned orientations creating dangling bonds at grain boundaries
- All common dopants (B, P, As) diffuse preferentially along grain boundaries -- they move much further through poly than through monocrystalline silicon
- Arsenic and phosphorus segregate at grain boundaries; boron does not

**Leakage mechanisms:** Grain boundaries act as Shockley-Hall-Read (SHR) recombination centers. Their presence within the depletion region greatly increases **generation-recombination current**, the dominant leakage mechanism. Additional leakage at high doping concentrations may come from thermal emission from traps or tunneling. Measured reverse leakage is orders of magnitude greater than monocrystalline junctions, but still small enough for many applications.

**Junction asperities:** Dopant diffusion along grain boundaries creates spike-like protrusions at the metallurgical junction, locally reducing breakdown voltage. The solution is a **PIN structure**: sandwich a layer of intrinsic (or near-intrinsic) silicon between P and N regions. The intrinsic layer depletes easily, widening the depletion region enough to tolerate asperities. Tradeoffs of PIN:
- Increased generation-recombination leakage (proportional to depletion volume)
- Switching delays of several microseconds due to charge storage

**Single-doped poly process (PN diode):** (Fig. 11.17A) Poly doped in-situ with boron ($R_s \approx 10\ \text{k}\Omega/\square$). Heavy phosphorus implant reduces gate poly $R_s$ to $\sim 30\ \Omega/\square$. A gate doping block mask (HSR) over the anode creates the lightly doped P-type region. PSD implant inside HSR ensures Ohmic contact. The drawn width of the lightly doped anode must be at least $\sim 5\ \mu\text{m}$ (due to enhanced grain-boundary diffusion) for a minimum reverse breakdown of ~6 V.

**Dual-doped poly process (PIN diode):** (Fig. 11.17B) Intrinsic poly deposited, then separate $N^+$ and $P^+$ implants create NMOS and PMOS gate poly. Silicide block (SiBlk) restricts titanium silicide from the active region. NGate and PGate rectangles define N-type and P-type regions; the gap between them is the intrinsic region. Drawn intrinsic width must be several microns wider than desired due to enhanced grain-boundary diffusion.

**Thermal fragility:** Poly diodes contain a very small volume of silicon thermally insulated by surrounding oxides. Excessive power can cause melting/alloying (short circuit) or cracking/vaporization (open circuit).

### 11.2.2 Analog BiCMOS Junction Diodes

#### Diode-Connected Bipolar Transistors

Any analog BiCMOS process can fabricate diode-connected transistors, typically offering at least one NPN and one PNP. NPNs are preferred because:
- Higher $\beta$ reduces the impact of base resistance on $I$-$V$ characteristics
- Higher $f_T$ minimizes reverse recovery delay

Optimal configuration: collector and base tied together (anode), emitter is cathode.

**Extended-base and DWell Zeners:** Diode-connected transistors with extended bases, shallow-well bases, or DWell bases provide Zener voltages from ~6 to ~20 V with temperature coefficients of $+1$ to $+4$ mV/K. Surface breakdown structures exhibit Zener walkout and potentially **Zener walkback** -- where the breakdown voltage first increases, then slows, reverses, and asymptotically approaches a final value 50-250 mV (up to 1 V in extreme cases) below the initial value.

**Walkback mechanism:** Most likely involves neutralization of positively charged interface traps by hot electrons. Walkback is prevalent in BiCMOS (using (100) silicon) but rare in standard bipolar (using (111) silicon).

**Poly-emitter degradation:** Poly-emitter transistors suffer severe avalanche-induced $\beta$ degradation. Hot carriers generate SHR recombination centers at the poly/monocrystalline interface, increasing generation-recombination current and causing orders-of-magnitude increases in reverse saturation current. Poly emitters are also thermally fragile. **Avoid constructing Zener diodes using poly emitters when possible.**

#### BiCMOS Power Diodes

Processes with N-type buried layer (NBL) and $N^+$ sinker enable power diodes with superior performance. Four main structures exist:

**Structure A (Fig. 11.18A) -- Deep N-well with NBL and sinker (best):**
- PMoat anode inside N-well floored by NBL and ringed by $N^+$ sinker
- NBL + sinker provide low-resistance cathode path, enabling compact PSD anode rather than interdigitated fingers
- Forms an effective **hole-blocking guard ring** that minimizes substrate injection
- Can conduct amps of current without debiasing the substrate
- Rule: extend drawn NBL to at least the outside edge of the drawn $N^+$ sinker

**Structure B (Fig. 11.18B) -- Shallow N-well with NBL:**
- When shallow N-well does not reach NBL, floating P-epi exists between them
- The floating P-epi does NOT compromise the hole-blocking guard ring (because the shallow-well/P-epi junction has lower $V_f$ than the NBL/P-epi junction)
- However, NBL cannot provide low-resistance path -- must use interdigitated NMoat/PMoat fingers (less space-efficient)

**Structure C (Fig. 11.18C) -- Shallow N-well without effective guard ring:**
- When sinker doping is insufficient for effective hole blocking
- Uses a zero-biased hole-collecting guard ring (P-well ring between N-well and sinker)
- Saturates at high currents causing substantial substrate injection
- Only suitable for currents of a few milliamps

**Structure D (Fig. 11.19) -- Isolated $N^+$/P-epi diode:**
- An $N^+$/P-epi diode built inside an isolated tank
- $N^+$ sinker contacts connect to cathode
- Low substrate injection because heavily doped sinker/NBL means current is dominated by electrons injected into P-epi

### 11.2.3 CMOS and BiCMOS Schottky Diodes

The availability of Schottky diodes has oscillated with silicide technology choices:

| Era/Technology | Silicide | Schottky Available? | Reason |
|---|---|---|---|
| Early metal-gate MOS | Pt or Pd silicide (noble) | Yes | Noble silicides have favorable barrier heights |
| Poly-gate era | Ti silicide (refractory) | No | TiSi$_2$ barrier height too low for low-leakage Schottky |
| Sub-micron ($<1\ \mu\text{m}$) | Co disilicide (CoSi$_2$) | Yes | Favorable barrier height to N-type Si |
| Modern | Ni monosilicide (NiSi) | Yes | Good barrier height, no asperity problems |

**CMOS Schottky layout (Fig. 11.21):**
- Moat geometry across contact opening ensures contact penetrates thin oxide
- PSD ring around the contact opening serves as a **field-relief guard ring**
- Reverse breakdown is limited by PSD/N-well avalanche rather than planar Schottky breakdown
- For higher breakdown, replace PSD with a more lightly doped P-type region or use a field plate
- Field-plated Schottkies often exhibit lower breakdown and higher leakage than guard-ringed ones

**Series resistance problem:** CMOS Schottkies lack NBL and $N^+$ sinker, so series resistance is inherently high. Mitigation: elongate the Schottky contact and surround with cathode contacts. Practical current limit: a few milliamps; hundreds of milliamps require prohibitively large layouts.

**BiCMOS Schottky (Fig. 11.22):** NBL and $N^+$ sinker greatly reduce parasitic resistance. The sinker can be extended as a hole-blocking guard ring for high-current operation (multiple amps without substrate injection).

**Cobalt vs. nickel silicide:** CoSi$_2$ suffers from asperities caused by native oxide interference with silicidation, leading to junction spiking and forward voltage variability. NiSi does not have this problem and may offer better matching.

**Array contacts:** Some processes require arrays of small contacts rather than single large openings. These cannot be guard-ringed individually. Schottky diodes in such processes require fully silicided source/drain regions -- a moat region not covered by PSD or NSD receives silicide, becoming the Schottky anode, with contact arrays providing electrical connection.

### 11.3 Matching Diodes

The three broad categories (PN junction, Zener, Schottky) will **not match across categories** -- they depend on fundamentally different conduction mechanisms. Within a category, matching requires identical diffusions and proper layout.

#### 11.3.1 Matching PN Junction Diodes

Most bipolar/BiCMOS PN diodes are really diode-connected transistors, so the rules from Sections 10.3.1 and 10.3.2 apply.

**Merged collector-base contacts** (Fig. 11.2 style): The emitter diffusion beneath the merged contact must maintain the emitter-to-unconnected-emitter spacing rule, increased by several microns for matched devices to allow minority carriers to diffuse away from the junction. Constraining minority carrier flow decreases effective device size, affecting matching of ratioed pairs.

**Ratioed pairs of diode-connected transistors** present a special challenge:
- Circuits typically control emitter current, not collector current
- Voltage differences are susceptible to both low-current and high-current $\beta$ rolloff
- Devices must operate within a distinct **beta plateau** region
- Standard bipolar vertical NPNs usually have a broad, well-defined plateau
- Most CMOS/BiCMOS bipolars show little or no beta plateau
- If no plateau exists and base current compensation is not feasible, operate at or near peak $\beta$

**PSD/N-well diode matching issues:**
- These are substrate PNPs without a distinct beta plateau -- avoid ratioed pairs if possible
- Even with hole-blocking guard rings forcing true two-terminal diode behavior, the mechanisms causing beta rolloff still distort ratioed pair voltages
- **N-well resistance mismatch:** Variations in $\beta$ cause variations in current through N-well resistance. Near-unity betas (common in shallow heavily-doped wells) exacerbate this. Minimize by elongating PMoat into long thin strips between NMoat contacts (Fig. 11.23)

**Poly diodes should never be used where matching matters** -- random fluctuations in grain number/disposition relative to the PN junction produce large generation-recombination current variations.

#### 11.3.2 Matching Zener Diodes

Zener matching is inherently difficult because breakdown voltage depends extremely sensitively on electric field intensity, and $E$-field depends on junction geometry curvature. Key issues:

- **Corner elimination:** Designers remove corners from matched Zeners to avoid preferential breakdown. However, outdiffusion usually rounds corners sufficiently that lateral curvature is less than the vertical curvature of the junction sidewall, making corners less relevant in practice.

- **Pelgrom's law does NOT apply** to Zener diodes at low current densities. The exponential relationship between reverse bias and avalanche current creates a **"winner takes all"** situation: the point that avalanches at the lowest voltage sets the device breakdown voltage.

- **Ballasting analysis:** Can higher current operation + resistive ballasting fix this? For emitter-base Zeners, the voltage drop across the base annulus is given by:

$$V_{bal} = J_E \cdot R_{SB} \cdot \left(\frac{r_B^2 - r_E^2}{2}\right)$$

where $J_E$ is the emitter current density per unit periphery, $R_{SB}$ is the base sheet resistance, $r_B$ is the inner radius of the base contact, and $r_E$ is the emitter radius. Typical values yield only ~4 mV of ballasting -- far short of the ~50 mV needed for effective ballasting.

- **Quatrefoil layout (Fig. 11.24):** Cross-coupled pairs of matched emitter-base Zeners using circular geometries with four-leaf-clover-shaped anode contacts for lead routing. Four Zeners share a common tank (no NBL/$N^+$ needed). Emitter metallization flanges over the junction for uniform vertical E-field. Despite elaborate layout, matching is limited to ~50-100 mV (possibly up to 1 V) due to variable walkout/walkback rates.

- **Buried Zeners** do not exhibit gross walkout mismatch, but still do not follow Pelgrom's law due to the "winner takes all" conduction characteristic.

**Bottom line:** Surface Zeners match to no better than 50-100 mV. Stacks of PN diodes or MOS transistors have more temperature variation but match far more accurately.

#### 11.3.3 Matching Schottky Diodes

Schottky matching depends on metal composition, silicon doping, edge effects, asperities, annealing conditions, and surface contaminants -- most are hard to control.

**Ranking by matching quality:**
1. **Aluminum Schottky** -- worst. Aluminum dissolves during sintering and redeposits randomly around contact edges. Never use where matching matters.
2. **Noble silicide Schottky** (Pt, Pd) -- best. Chemical reaction with silicon creates material of definite composition and constant barrier height. Properly guard-ringed PtSi Schottky has ideality coefficient $n \approx 1.02$.
3. **CoSi$_2$** -- moderate. Asperities from native oxide interference increase ideality from $n = 1.06$ at low anneal temperatures to $n = 1.50$ at high anneal temperatures.
4. **NiSi** -- promising. Does not have native oxide problems, may match better than CoSi$_2$.

**Size effects on ideality:** Small Schottky diodes suffer greater barrier inhomogeneity. Gold/P-Si data showed ideality of 1.006 at large diameters rising to 1.41 at $10\ \mu\text{m}$ diameter, with barrier height dropping from 0.791 V to 0.623 V. Therefore, **avoid ratioed pairs of Schottky diodes**.

**Temperature dependence of non-ideal Schottky:** The ideality coefficient follows:

$$n(T) = 1 + \frac{T_0}{T}$$

where $T$ is absolute temperature and $T_0$ is the "excess temperature." For $n = 1.1$, $T_0 \approx 30$ K. A ratioed pair would show a constant offset voltage proportional to $T_0$.

#### 11.3.4 Rules for Matching Diodes (Complete List)

Three tiers of matching accuracy:

| Level | Offset Voltage | Current Mismatch | Application |
|-------|---------------|-----------------|-------------|
| **Minimal** | $\pm 3$ mV | $\pm 12\%$ | General-purpose op-amps, comparators |
| **Moderate** | $\pm 1$ mV | $\pm 4\%$ | Bandgap references, precision op-amps ($1$-$2$ mV without trim) |
| **Exceptional** | $< 0.5$ mV | $< 2\%$ | Only achievable with vertical bipolars with beta plateaus; may require trimming |

Assumptions: six-sigma statistics, 10-year operating lifetimes, standard junction temperatures, conventional plastic packaging.

**The 14 Matching Rules:**

1. **Poly diodes and surface-breakdown Zeners do not match well.** Poly diodes have excessive generation-recombination current variability; surface Zeners suffer walkout/walkback. Neither achieves even minimal matching.

2. **High ideality factor diodes are unsuited for ratioed pairs.** Ideality coefficient variations over process limit voltage differential accuracy. For minimal matching: $n \leq 1.1$. Moderate: $n < 1.05$ (preferably $< 1.03$).

3. **Diode-connected transistors without beta plateaus are unsuited for ratioed pairs.** Must operate within a definite beta plateau. Beta compensation can address low-current rolloff but not high-current rolloff from high-level injection.

4. **Use identical PN junction or Schottky contact geometries.** The geometry defining the diode area (PN junction or Schottky barrier) must be an exact duplicate. Isolation diffusion geometries matter far less. Multiple diodes may share a common isolation region if only majority carriers flow through it.

5. **Minimum width 2-10x minimum feature size.** Excessively small geometries increase random variations from perimeter effects. Some Schottkies shift barrier height below ~$20\ \mu\text{m}$. Moderately matched diode-connected transistors need active areas of $\sim 100\ \mu\text{m}^2$. Very large diodes increase sensitivity to nonlinear gradients -- use arrays of moderate devices instead. **Zeners do not obey Pelgrom's law.**

6. **Use circular or square geometries.** Large area-to-periphery ratios minimize peripheral random variations. Use 32- or 64-sided circles where allowed. Exception: devices with high series resistance operating at high current may benefit from elongated geometries.

7. **Place matched diodes in close proximity.** Diodes are extremely sensitive to thermal gradients. PN junction diodes are also sensitive to mechanical stress (piezojunction effect). Minimal: adjacent, $< 200\ \mu\text{m}$ apart. Common-centroid is recommended for minimal matching, mandatory for moderate/exceptional.

8. **Keep the layout compact.** Tight clusters outperform linear arrangements. Equal-size pairs: cross-coupled pairs. Exceptional: multiple cross-coupled pairs in parallel for 2D common-centroid arrays.

9. **Place matched diodes far from power devices.** Minimum separations from major heat sources ($\geq 250$ mW): minimal $\geq 500\ \mu\text{m}$; moderate $\geq 1$-$2$ mm (opposite side of die). Exceptional: consider elongating die to 1.5:1 or 2:1 aspect ratio.

10. **Place matched PN diodes in low stress-gradient areas.** Die center has highest compressive stress but lowest stress gradients. Keep PN diodes $> 200\ \mu\text{m}$ from die edges. Never place in die corners (largest stress gradients).

11. **Place moderate/exceptional matched PN diodes on axes of die symmetry.** Stress distribution is generally symmetric with respect to die axes -- exploit this.

12. **Do not let NBL shadow intersect the active area.** Enlarge NBL geometry so its shadow misses the PN junction (for PN diodes) or Schottky barrier (for Schottkies). Schottky diodes are especially vulnerable because the barrier lies at the surface. Even minimal Schottky matching requires clear NBL shadow. Shallow trench isolation processes have no NBL shadow.

13. **Space merged PN diodes sufficiently to avoid minority carrier interactions.** Forward-biased junctions launch minority carriers laterally. Use the emitter-to-*unconnected*-emitter spacing rule (not connected-emitter rule) with $2$-$4\ \mu\text{m}$ additional margin.

14. **Increase diffusion overlaps for moderate/exceptional matching.** If one diffusion barely overlaps the junction, misalignment causes lateral current flow variations. Increase overlap of base over emitter by $2\ \mu\text{m}$ beyond the minimum rule.

### The DMOS Buried Zener (BiCMOS)

The DWell mask creates a structure where both arsenic and boron are implanted through oxide openings. The boron outdiffuses beyond the arsenic, producing a shallow heavily-doped N-type region enclosed by a deeper, more lightly-doped P-type region.

**Layout (Fig. 11.20):**
- Circular DWell plug inside an isolated P-type tank
- NSD encloses the entire DWell (cathode)
- P-type DWell component is the anode, contacted by PMoat strips outside the NSD
- Breakdown between NSD and P-type DWell occurs subsurface (6-9 V typical)

**Isolation connection:** The $N^+$ sinker and NBL can connect to anode, cathode, or any node whose voltage always exceeds the anode voltage. Most often tied to the anode. **Critical warning:** Do not connect to high-resistance nodes at very low currents. In one documented case, electron collection from switching transients in adjacent circuitry bled current from the anode of a microamp-level Zener, causing its voltage to drop below breakdown. Grounding the sinker/NBL cured the problem.

## Diagrams

### Figure 11.14 -- Diode-Connected MOS Transistors
![[diagrams/ch11-diodes-cmos-bicmos-fig1.png]]
*Six possible diode-connected MOS transistor configurations: (A) NMOS with separate backgate, (B) NMOS with antiparallel body diode, (C) NMOS with parallel body diode, (D-F) corresponding PMOS variants. The backgate connection determines whether the body diode is isolated, antiparallel, or parallel to the MOS channel.*

### Figure 11.16 -- PSD/N-well Zener Diode Layout and Cross Section
![[diagrams/ch11-diodes-cmos-bicmos-fig2.png]]
*PSD/N-well Zener diode in an analog BiCMOS process. The poly field plate ring over the junction is tied to the anode, creating a vertical E-field that forces subsurface avalanche breakdown and prevents Zener walkout. The NMoat cathode ring encloses the PMoat anode.*

### Figure 11.23 -- Matched PSD/N-well Diodes
![[diagrams/ch11-diodes-cmos-bicmos-fig3.png]]
*Matched pair of PSD/N-well diodes. Long thin PMoat strips placed between NMoat contacts minimize the voltage drop across N-well resistance, which is critical because near-unity betas in shallow wells cause large current-dependent resistance variations.*

## Practical Takeaways

- **For ESD protection in CMOS:** Use interdigitated NMoat/PMoat strip arrays. The NSD/P-epi diode is inherently robust due to distributed ballasting and sub-surface heat dissipation. A typical 2 kV HBM protection structure occupies only ~$50 \times 50\ \mu\text{m}^2$.
- **Do NOT add silicide block to NSD/P-epi ESD diodes** -- they already have distributed ballasting. Full silicidation minimizes series resistance.
- **For PSD/N-well Zeners:** Always include a poly field plate tied to the anode to force subsurface breakdown. Add distributed ballasting via silicide block ($\geq 2\ \mu\text{m}$ unsilicided moat around contacts).
- **Best BiCMOS power diode:** Fig. 11.18A (deep N-well + NBL + sinker). Most compact, lowest resistance, built-in hole-blocking guard ring.
- **Avoid poly emitters for Zeners** -- avalanche-induced $\beta$ degradation and thermal fragility.
- **Poly diodes** are thermally fragile and have high leakage -- use only when other diode types are unavailable. PIN structures improve breakdown voltage control but add delay.
- **Schottky diode availability** depends entirely on the silicide used. Check your process: noble silicides and CoSi$_2$/NiSi work; TiSi$_2$ does not.
- **For Schottky matching:** Use noble silicide, field-relief guard rings (not field plates), identical compact geometries ($> 20\ \mu\text{m}$), and equal areas (avoid ratioed pairs).
- **Zener matching** is fundamentally limited: surface Zeners to 50-100 mV at best. Buried Zeners are better but still do not follow Pelgrom's law. Prefer stacks of PN diodes for precision voltage references.
- **Diode-connected transistor matching** requires operation within the beta plateau. Most CMOS/BiCMOS bipolars lack a distinct plateau -- consider base current compensation.
- **Always use the emitter-to-unconnected-emitter rule** (not connected-emitter) for matched diode-connected transistors sharing a common region, plus $2$-$4\ \mu\text{m}$ additional margin.
- **Critical placement rules:** Keep matched PN diodes near the die center (low stress gradient), on axes of symmetry, far from power devices and die edges/corners.

## Relation to the Bigger Picture

This section bridges the gap between the ideal diode structures available in standard bipolar (covered in [[ch11-diodes-standard-bipolar]]) and the reality of what CMOS and BiCMOS processes can actually provide. The limitations are severe -- CMOS was never designed for forward-biased junctions -- but understanding how to exploit existing process layers (NSD/P-epi for ESD, PSD/N-well for Zeners, poly for high-voltage diodes) is essential for analog designers working in mixed-signal processes. The matching section (11.3) is particularly important because it connects back to the general matching principles of Chapter 8 while revealing device-specific pitfalls: the inapplicability of Pelgrom's law to Zeners, the beta-plateau requirement for ratioed diode-connected transistors, and the size-dependent barrier height variations that make Schottky matching fundamentally harder than PN junction matching. These constraints directly influence the achievable precision of bandgap references, current mirrors, and ESD structures -- core building blocks of every analog IC.

## See Also
- [[ch11-diodes-standard-bipolar]]
