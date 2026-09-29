# Hastings 3e, ch. 6–7: resistors, capacitors, inductors (generator specs for Philis)

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson, 2023), Chapter 6 "Resistors" and Chapter 7 "Capacitors and Inductors".
Reftext: `scratchpad/reftext/hastings.txt` (pdftotext -layout), lines **15989–21772** (ch. 6 opens at L15989; ch. 8 opens at L21772). PDF pages 275–366 (PDF page = 1 + form feeds before the line; printed page = PDF page − 1).
Most equations, and many numbers inside sentences, were rendered as images and are blank in the reftext. Every equation and number quoted below was read from the PDF page images. Where a value could not be read, the entry says "not given" or "illegible".

This document went through two full passes over the range. The second pass re-read every line, re-checked the equations against the page images, corrected three values (sliding-contact threshold, emitter-resistor sheet, sandwich-capacitor density), re-verified every code citation against the current working tree, and added H06-56.

---

## 1. Coverage

Consecutive Read chunks of the reftext. A 2000-line chunk exceeds the tool's token limit, so smaller chunks were used.

| Pass | Chunks (reftext lines) |
|---|---|
| 1 | 15989–17088, 17089–18188, 18189–19288, 19289–20388, 20389–21488, 21489–21778 |
| 2 | 15989–16888, 16889–17788, 17789–18688, 18689–19588, 19589–20488, 20489–21188, 21189–21772 |

The range ends at L21772, the ch. 8 heading.

PDF page images read for garbled equations and values:
- Pass 1: 279–284, 285–290, 292–294, 296, 298, 301–305, 306–307, 315–316, 317, 322, 328–332, 334–338, 339–344, 347–351, 354–363, 364.
- Pass 2: 279–282, 284–305 (except 299), 310–311, 315–316, 321–322, 327–345, 348–350, 357, 359–363.

Values from pages read only in pass 1 (306–307 thin film, 317 trim accuracy, 351–354 lateral-flux, trench and Table 7.3, 364 summary) are retained as pass 1 recorded them.

Four worked examples do not reproduce from their own equations: Eq. 6.16 (×100), Eq. 7.7 (60% vs 69%), Eq. 7.34 (18.6 vs 24.8 nH), and Eq. 7.8, whose printed sign has failure rate falling with field. Four examples do reproduce: Eq. 7.6 (2.6 fF, using t/2), Eq. 7.9 (0.36 FIT), Eq. 7.20 (19 Å) and Eq. 7.38 (2.4 GHz). Each is flagged in its entry.

Headings in the range (reftext line):

- Ch. 6 Resistors (15989)
  - 6.1 Resistivity and Sheet Resistance (16006)
  - 6.2 Resistor Layout (16261)
  - 6.3 Resistor Variability (16511)
    - 6.3.1 Process Variation (16519)
    - 6.3.2 Temperature Variation (16613)
    - 6.3.3 Nonlinearity and Conductivity Modulation (16748)
    - 6.3.4 Contact Resistance (16932)
    - 6.3.5 Hydrogenation and Dehydrogenation (17022)
  - 6.4 Resistor Parasitics (17062)
  - 6.5 Comparison of Available Resistors (17249)
    - 6.5.1 Base Resistors (17255)
    - 6.5.2 Emitter Resistors (17312)
    - 6.5.3 Base Pinch Resistors (17382)
    - 6.5.4 High-Sheet Resistors (17418)
    - 6.5.5 Epi Pinch Resistors (17536)
    - 6.5.6 Metal Resistors (17577)
    - 6.5.7 Poly Resistors (17655)
    - 6.5.8 NSD and PSD Resistors (17894)
    - 6.5.9 N-Well Resistors (17917)
    - 6.5.10 Thin-Film Resistors (17963)
    - 6.5.11 Summary of Available Resistor Types (18092)
  - 6.6 Adjusting Resistor Values (18202)
    - 6.6.1 Tweaking Resistors (18218): Sliding Contacts (18234), Sliding Heads (18281), Trombone Slides (18317), Metal Options (18341)
    - 6.6.2 Trimming Resistors (18352): Fuses (18359), Zener Zaps (18669), Nonvolatile Memory (18810), Electrical Trimming of Polysilicon Resistors (18850), Laser Trim (18898)
  - 6.7 Summary (18983); Selected Bibliography (19008); 6.8 Exercises (19026)
- Ch. 7 Capacitors and Inductors (19099)
  - 7.1 Capacitance (19119): Fringing Capacitance (19345), Derating Amorphous Dielectrics (19432), Junction Capacitance (19505)
    - 7.1.1 Capacitor Variability (19698): Process Variation (19706), Voltage Modulation (19752), Temperature Modulation (19929)
    - 7.1.2 Capacitor Parasitics (19974)
    - 7.1.3 Comparison of Available Capacitors (20075): Junction Capacitors (20092), MOS Capacitors (20174), Poly-Poly Capacitors (20437), Metal-Metal Capacitors (20579), Stack Capacitors (20689), Lateral Flux Capacitors (20741), Trench Capacitors (20811), Summary of Available Capacitor Types (20911)
  - 7.2 Inductance (20981)
    - 7.2.1 Inductor Parasitics (21200)
    - 7.2.2 Inductor Construction (21418); Guidelines for Integrating Inductors (21494)
  - 7.3 Summary (21573); Selected Bibliography (21605); 7.4 Exercises (21640)
- Ch. 8 heading (21772–21778, out of scope; read only to close the range)

---

## 2. Section-by-section digest

### Ch. 6 intro (L15989–16000)
- Integrated resistors have poor absolute tolerance but excellent matching; analog designers rely heavily on matched resistors (L15991–15994).
- The process offers several resistor materials; the designer picks material and width, and physical construction also affects performance. Matching construction is deferred to ch. 8 (L15997–16000).

### 6.1 Resistivity and Sheet Resistance (L16006–16251)
- Ohm's law V = IR. A resistance that varies with voltage is nonlinear, and most integrated resistors are slightly nonlinear (L16007–16028).
- Table 6.1 lists the SI prefixes with their SPICE suffixes (F, P, N, U, M, K, MEG). Note that SPICE "M" means milli, not mega (L16051–16096).
- Table 6.2 gives resistivities of metals, silicides and doped Si. Bulk resistivity differs from surface resistivity (L16102–16167).
- Unstressed silicon is isotropic. Diffused resistors under stress are piezoresistive, and poly shows vertical/lateral anisotropy that is absorbed into contact resistance. These effects matter only for accurate matching (L16183–16188).
- R = ρL/(Wt) (Eq. 6.2). Sheet resistance R_s = ρ/t gives R = R_s·L/W (Eq. 6.3), with L/W in "squares". Diffusion R_s (Eq. 6.4 integral) is measured, not computed (L16199–16251).

### 6.2 Resistor Layout (L16261–16505)
- The drawn length L_d runs between the inner contact edges. Effective W = W_d + W_b and L = L_d + L_b (Eqs. 6.5–6.6). R = R_s(L_d+L_b)/(W_d+W_b) (Eq. 6.7) (L16262–16304; PDF p. 279).
- The width bias dominates. For diffusions it is ≈20% of the junction depth (1.25 µm x_j gives 0.25 µm, a 5% error on a 5 µm base resistor). Layout rules may tabulate width biases (L16309–16314).
- Lateral current crowding at narrow contacts is given by Ting–Chen Eq. 6.8. A 5 µm resistor with 3 µm contacts gains 0.05 □. Vertical crowding is negligible if L ≥ 20·thickness. **Accurate resistors should use segments at least 10 µm long** (L16326–16360; PDF p. 280).
- Serpentines use rectangular turns. Rounded turns and filleted corners behind the contacts are used for high voltage (L16363–16366).
- A square corner adds ≈0.56 □; Eq. 6.9 gives R = R_s[(2A+B)/W + 1.12]. Hall's Eq. 6.10 handles unequal-width corners (0.5587 □ for equal widths). A 180° circular end adds 2.96 □ (Eq. 6.11) (L16382–16436; PDF p. 281).
- In a dogbone, heads enlarge around contacts and current spreading lowers R. Table 6.3 gives ΔR = −0.7 □ (W_o = W_d, W_c = W_d) and −0.3 □ (W_o = ½W_d). The correction is usually < 0.3 □ (L16440–16483; PDF p. 282).
- Dogbones pack worse. Their accuracy advantage is illusory: **matched resistors should always be laid out in identical sections** (L16486–16490).

### 6.3 Resistor Variability (L16511–16515)
- The dominant factors are process variation, temperature, nonlinearity and contact resistance. Orientation, stress/thermal gradients, thermoelectric effects, etch, NBL push, charge spreading, hydrogenation and PSG polarization are deferred to ch. 8.

### 6.3.1 Process Variation (L16519–16609)
- Fab sheet-resistance control limits are ±20–25% for most layers and up to ±50% for very high-sheet poly and pinch resistors (L16525–16529; PDF p. 284).
- Linewidth control is ≈ ±10% of the minimum feature for deposited layers (0.25 µm gate ⇒ ±0.03 µm). Diffusions add ±5% of x_j (L16532–16537).
- Eq. 6.12: δR ≅ (δW/W + δR_s/100%)·100%. With ±20% R_s and ±0.05 µm LW, a 0.5 µm resistor is ±30%, 1 µm is ±25% and 10 µm is ±21% (L16545–16557).
- Bamboo effect: grains ≤ 0.1 µm, so it seldom affects poly wider than ≈0.25 µm. Narrow TiSi₂ (< 0.2 µm) stays in the C49 phase (80–100 µΩ·cm vs 13–20 µΩ·cm), so TiSi₂-silicided resistors need W ≥ 0.25 µm. Diffusions narrower than ≈2·x_j suffer dilution, and a constant W_b underestimates R there (L16560–16594; PDF pp. 284–285).
- Width guidelines. Tolerance not important: minimum width. Moderate: 2–3× minimum, poly ≥ 0.3 µm, diffusion ≥ 2·x_j, no poly above 5 kΩ/□ and no pinch resistors. Crucial: consider trimming, poly ≥ 0.5 µm, no lightly doped resistors (L16598–16609; PDF p. 285).

### 6.3.2 Temperature Variation (L16613–16743)
- Linear TCR: R(T) = R(T₀)[1 + α(T−T₀)] (Eq. 6.13), α in ppm/°C (L16613–16626).
- Table 6.4 (T₀ = 25 °C, valid −40…125 °C, ppm/°C): Al +3800, Cu +4000, Au +3700, CoSi₂ 1.2 kÅ +3000, 160 Ω/□ base +1500, 7 Ω/□ emitter +600, 5 kΩ/□ base pinch +2500, 2 kΩ/□ HSR +3000, 500 Ω/□ poly (4 kÅ P-doped) −1000, 25 Ω/□ poly +1000, 10 kΩ/□ N-well +6000 (L16632–16695; PDF pp. 285–286).
- High-sheet poly has a negative TC because of grain-boundary conduction (L16700–16703).
- Quadratic model R(T) = R(T₀)[1 + α₁ΔT + α₂ΔT²] (Eq. 6.14). The coefficients are regression fits, valid only over their fit range, and α₁ ≠ the α of Eq. 6.13 (L16706–16726).
- Low-TC composites combine low- and high-sheet poly in series/parallel. High-sheet TC is poorly controlled, but some series-parallel networks are insensitive to TC variation (Gregoire–Moon) (L16730–16738).

### 6.3.3 Nonlinearity and Conductivity Modulation (L16748–16928)
- Sources are self-heating, velocity saturation and depletion encroachment. Absolute R = V/I differs from incremental r = dV/dI, and "resistance" means absolute by default (L16749–16785).
- Voltage modulation: R(V) = R(V₀)[1 + β₁(V−V₀) + β₂(V−V₀)²] (Eq. 6.15), β in ppm/V and ppm/V², fitted over a range (L16788–16803; PDF p. 287).
- Self-heating: β₂ ≅ α·t_ox·R_s/(κ·R₀²·W²) (Eq. 6.16), with κ(SiO₂) ≈ 0.013 W/cm/°C. Book example: 10 kΩ, 5 µm, 500 Ω/□, −1000 ppm/°C, 1 µm oxide ⇒ −1.5 %/V² (L16806–16828; PDF pp. 287–288).
- Velocity saturation above 0.2 V/µm (electrons) and 0.6 V/µm (holes). With a safety factor 2, L_min = 10 µm/V·V_max (N-type Si, Eq. 6.17) and 3.3 µm/V·V_max (P-type Si, Eq. 6.18). Poly grains: L ≥ 1000 × grain diameter (0.05 µm ⇒ 50 µm), though 10 µm is acceptable in practice (L16832–16856; PDF p. 288).
- Pinching: base resistor β₁ = 100 ppm/V, high-sheet 2.5 %/V, base pinch β₁ = 6 %/V and β₂ = 2 %/V² (doubles at 5 V) (L16864–16870).
- Tank/body modulation. For a 700 Ω/□ P-type resistor β₁ = 1000 ppm/V and β₂ = −20 ppm/V². N-well resistors show body modulation. Leads crossing a 2 kΩ/□ HSR cause 0.1 %/V. Poly ≤ 1 kΩ/□ is largely immune. Split field plates are recommended for accurate HSR (L16897–16928; PDF p. 289).

### 6.3.4 Contact Resistance (L16932–17018)
- R_c = (√(R_s·ρ_c)/W_c)·coth(L_c·√(R_s/ρ_c)) (Eq. 6.19), where ρ_c is the specific contact resistance in Ω·µm² (L16932–16956; PDF p. 290).
- Table 6.5 (Ω·µm²): Al(2%Si,0.5%Cu) to 160 Ω/□ base 750; refractory barrier metal to base 2500; PtSi to base 1250; Al to 5 Ω/□ emitter 40; TiSi₂ to (100) NSD 30; TiSi₂ to PSD 100 (L16961–16990).
- Barrier metal without silicide raises both contact R and its variability, and silicide fixes both (L17006–17010).
- Example: a 1 kΩ base resistor with 8×8 µm contacts sees 43 Ω/contact, 86 Ω total (9%). Segments under 10 □ may need oversized contacts, and **parallel longer segments are better than dogbone heads** (L17014–17018).

### 6.3.5 Hydrogenation and Dehydrogenation (L17022–17051)
- High-sheet P-type poly drifts up by several percent over 10²–10³ h at high temperature. About 1% of the Si–H bonds are weak (0.5 eV) and break. The drift harms current sources (L17023–17041).
- Phosphorus passivates grain boundaries, so N-type high-sheet poly (P > B) drifts less. Compensated P-type poly (B > P) drifts more because of 0.3 eV B–P complexes (L17045–17051).

### 6.4 Resistor Parasitics (L17062–17243)
- A poly resistor over field oxide has ≈0.05 fF/µm² to substrate. A 1 µm × 1000 □ resistor has ≈50 fF, which is distributed (L17069–17077; PDF p. 292).
- Single-π model: C/2 at each end. Dual-π: C/4, C/2, C/4 with R/2, R/2 (Fig. 6.10) (L17074–17077).
- A 1 µm lead crossing a 1 µm resistor adds ≈0.05 fF. On a 0.1 pF high-impedance node, a 2 Vpp square wave couples ≈1 mVpp, enough to hurt a 16-bit converter. Long parallel runs are worse (L17110–17123).
- Diffused resistors are modeled with diodes to the body and body to substrate (Fig. 6.12A/B). Model B adds body resistance (L17127–17170).
- Improper body connections or transients can forward-bias the junctions, causing latchup (L17173–17182).
- Body biasing (Fig. 6.13): both bodies to V_CC means ratio drift. Giving each segment its own body tied to its positive end makes body modulation track. Floating body connections can forward-bias. Stay ≤ 2/3 of the body-junction voltage rating, and segment into multiple bodies for higher V (L17199–17225; PDF p. 294).
- Junction capacitance is 1–5 fF/µm². Diffused resistors dissipate more power than poly because they sit in contact with silicon, so they are used for ESD (L17229–17243).

### 6.5 Comparison of Available Resistors (L17249–17251)
- The section compares resistor types by process.

### 6.5.1 Base Resistors (L17255–17308)
- R_s is 100–200 Ω/□, best for 50 Ω–20 kΩ. Field-plate base resistors when the tank voltage exceeds 2/3 of the top-metal thick-field threshold. Reroute noisy leads away from base resistors (L17256–17279; PDF p. 296).
- NBL under matched resistors should overlap by 5–8 µm. Do not merge with minority-carrier injectors. NBL plus a sinker is needed when merging with NPNs (L17282–17304).

### 6.5.2 Emitter Resistors (L17312–17378)
- R_s is 2–10 Ω/□, used for resistors from a few tenths of an ohm to a couple hundred ohms. Low VCR and TC (L17318–17323; PDF p. 297).
- A thin emitter oxide is 0.7 fF/µm² and a thick one is 0.2 fF/µm², so coupling to overlying leads matters (L17327–17328; PDF p. 297).
- Thin emitter oxide is ESD-vulnerable, so pin-connected leads must not cross it (L17327–17330).
- Body rules: base body to the low end, tank to the high end. Stay ≤ 2/3 of the E-B avalanche voltage, or segment. Tankless emitter "tunnel" (crossunder) resistors exist (L17340–17378).

### 6.5.3 Base Pinch Resistors (L17382–17414)
- R_s is 2–10 kΩ/□ with ±50% variation, tracking β. JFET-like, so they can be matched only with identical sections in separate, suitably biased tanks, and even then ±5% residual. Keep ≤ 2/3 of E-B BV. For noncritical high values only (L17387–17414; PDF p. 297).

### 6.5.4 High-Sheet Resistors (L17418–17532)
- R_s is 1–10 kΩ/□ and TC is several thousand ppm/°C. Base heads are needed for contact (L17419–17425).
- R = R_s(L_d+L_b)/(W_d+W_b) + 2R_h (Eq. 6.20). The head is R_h = k·R_sb·(S₂ + W_c/2)/(W_h + W_hb) (Eq. 6.21, numerator read from a low-resolution render), with k ≈ 0.7, approaching 1 for elongated heads. L_b ≈ −0.5 µm (L17433–17454; PDF p. 298).
- HV HSR needs field plating above 2/3 of the top-metal thick-field threshold. For a divider, use equal segments, each in its own tank tied to its positive end, with the field plate also tied to that end (L17470–17477; PDF p. 299).
- Extended base heads let leads cross (Fig. 6.16) (L17481–17487).
- BV is 20–30 V, raised a few volts by filleting corners. Segmenting into tanks allows higher V and lower nonlinearity (L17502–17508).
- Always place NBL under HSR. The optimal sheet is ≈2 kΩ/□ (L17514–17525).

### 6.5.5 Epi Pinch Resistors (L17536–17573)
- R_s is 5–20 kΩ/□ with ±50%; these are JFETs (epi-FETs) with pinch-off between −20 and −50 V, used for startup trickle currents. In CMOS/BiCMOS, depletion MOS or narrow high-sheet poly are better (L17537–17573; PDF p. 300).

### 6.5.6 Metal Resistors (L17577–17651)
- Bipolar metal is 10–15 kÅ at 20–30 mΩ/□. Lower CMOS metals are 3–5 kÅ at 50–90 mΩ/□. Values range 50 mΩ–50 Ω, for current sensing and ballast. Lay out as a run or serpentine, over field oxide where there is no CMP (L17583–17597; PDF p. 301).
- Paralleled layers need enough vias that via R is a small fraction of metal R (L17600–17602).
- Kelvin (force/sense) connections, star node. L_d is measured between the inner edges of the sense taps. Extend force leads ≥ 2× the resistor width before bends or width changes, or use field solvers or trims. A two-level layout (sense on upper metal via vias) is less sensitive (L17606–17638).
- Metal R_s varies ±20% and is often not monitored by the fab. Metal TCR ≈ +3300 ppm/°C equals the tempco of a VPTAT at 25 °C (L17642–17651; PDF p. 302).

### 6.5.7 Poly Resistors (L17655–17890)
- Gate poly is 25–50 Ω/□, silicided 2–5 Ω/□. High-sheet poly is ≈500 Ω/□ and VHSR ≥ 5 kΩ/□ (L17656–17666).
- 500 Ω/□ varies ±20% and 5 kΩ/□ varies ±50%. Deposition and anneal drive the variability (L17670–17681; PDF p. 302).
- TCs: 70 Ω/□ +500 ppm/°C, 500 Ω/□ −1000 ppm/°C, 10 kΩ/□ −7000 ppm/°C. 0TC occurs at 200–300 Ω/□ ±100 ppm/°C. Ramped (tilted) poly causes wafer-to-wafer spread (L17689–17720; PDF pp. 302–303).
- R = R_s(L_d − 2L_b)/(W_d + W_b) + 2R_h(L_h + L_b)/(W_d + W_b) (Eq. 6.22), where R_h is the head sheet and L_h the head-implant overlap past the contact. The head implant must cover the full width (L17729–17744; PDF p. 303).
- W_b can be a significant fraction of a µm, so narrow resistors vary. Most processes hold poly within ≈10% of the minimum feature. L_b is the head-dopant intrusion and matters for short resistors (L17760–17770).
- Put poly resistors on field oxide, not gate oxide, to cut parasitic C. Deep-N+ under the resistor thickens the oxide where allowed (L17774–17782).
- Low power handling (oxide). Overheating shifts R and TC permanently (L17786–17788).
- Silicide block: R ≈ R_s·L_d/(W_d + W_b) (Eq. 6.23). A gate-doping block needs a large overlap because L_b is several µm from fast dopant diffusion in poly (L17791–17837; PDF p. 304).
- **Voltage-dependent poly spacing**: dense poly resistors risk ILO TDDB. Use spacing equal to the block operating voltage, except between turns, where ΔV = 2V_d/N (Eq. 6.24; 6 V over 6 segments ⇒ 2 V) (L17857–17874; PDF p. 305).
- Cone defects in STI under poly break down below 20 V. Use overvoltage screening or segment HV poly resistors over separately biased wells (L17877–17883).
- Poly is the best general resistor. Use diffused resistors for high power and thin film for extreme accuracy (L17887–17890).

### 6.5.8 NSD and PSD Resistors (L17894–17913)
- 20–50 Ω/□ (2–5 Ω/□ silicided, with heads left silicided per Eq. 6.22). Negligible VCR and conductivity modulation, and lower BV from sidewall curvature. Used in ESD as clamps and for power (L17895–17913; PDF p. 305).

### 6.5.9 N-Well Resistors (L17917–17959)
- Deep well up to 10 kΩ/□, shallow ≈1 kΩ/□, PMoat-pinched up to 20 kΩ/□ with severe nonlinearity. Unpinched N-well must be field plated. Draw width ≥ 2–3·x_j, because W_b becomes width-dependent below ≈2·x_j. Parallel narrow strips use dilution deliberately (L17918–17959; PDF pp. 305–306).

### 6.5.10 Thin-Film Resistors (L17963–18088)
- TC < 100 ppm/°C, process TC variation < ±50 ppm/°C, sheet ±20%, laser-trimmed to ±0.05% or better, minimal aging (L17963–17972; PDF p. 306).
- Table 6.6: NiCr 0.1 mΩ·cm, 20–200 Ω/□, +50…+100 ppm/°C. Ta 0.2 mΩ·cm, 20–200 Ω/□, −100…−150. SiCr 1–20 mΩ·cm, 100–2000 Ω/□, 0…−150 (L17999–18019; PDF p. 307).
- The single-mask TFR cannot have the next metal routed across it and is incompatible with W plugs. The two-mask TFR/TFC flow avoids this (L18036–18076).

### 6.5.11 Summary of Available Resistor Types (L18092–18196)
- Table 6.7 (R_s Ω/□, ±%, V):
  - Bipolar: base 150/20/40, emitter 5/20/5, HSR 2000/30/40, base pinch 3000/50†/5, epi pinch 10000/†/40, metal 0.03/30/40.
  - Poly-gate CMOS: gate poly 20/30/40, HSR poly 500/30/40, PSD 50/20/15, NSD 30/20/15, N-well 2000/40†/40, metal 0.05/20/40.
  - Analog BiCMOS: LSR poly 5/20/40, MSR poly 200/20/40, HSR poly 1000/30/40, base 400/20/30, N-well 1500/40/40, N-well pinch 5000/50†/30, metal 0.05/20/40.
  - \* = diffused (must be biased); † = too much VCR for most uses (L18103–18196).

### 6.6 Adjusting Resistor Values (L18202–18214)
- Trimming adjusts each unit at probe or final test. Tweaking changes one mask for all later units (L18203–18214).

### 6.6.1 Tweaking Resistors (L18218–18348)
- A well-designed resistor can be tweaked with one mask. Methods are sliding contacts, sliding heads, trombone slides and metal options (L18219–18230).
- Sliding contact: extend the body and start the contact mid-travel under an elongated metal plate. It suits heads made of the same material as the body, and is of little use for bodies above ≈200 Ω/□ with low-sheet heads (L18234–18279; PDF p. 311).
- Sliding heads suit HSR, high-sheet poly and silicided-head poly (L18281–18315).
- Trombone: slide the serpentine turns. The well, implant and silicide block must enclose the slide region (L18317–18321; PDF p. 311).
- Metal options: spare segments with contacts under metal, combined in series or parallel through a metal mask (L18341–18348).

### 6.6.2 Trimming Resistors (L18352–18977)
- Discrete trims are fuses, Zener zaps and NVM. Continuous trims are transient heating and laser (L18353–18356).
- Fuses. NiCr regrowth. Al fuses need overcoat openings and spatter. Poly melts at a higher temperature than Al, so use a < 25 ns rise to avoid cracking and regrowth. Silicide electromigration fuses shift resistance without voiding. Closed fuses are preferred (L18359–18447).
- Metal fuse programming: 5 V, 1 ms, several hundred mA, < 1 µs rise. Trimpads are smaller than bondpads, and circuitry is sometimes allowed beneath them. **Leads to fuses ≥ 5× the fuse-link width, so place fuses near their trimpads.** Share trimpads through series or parallel fuse connections (Fig. 6.27) (L18451–18464; PDF p. 313).
- Poly fuse: lowest-sheet poly, enlarged heads with enough contacts. Programming is 5–15 V, 1 ms, 50–150 mA, < 25 ns rise. Omit the overcoat opening if possible (L18493–18513).
- Put the fuse on the least-vulnerable end of the resistor (the grounded end in a Brokaw bandgap) (L18516–18522; PDF p. 315).
- Binary weights 1:2:…:2^N with 3–6 fuses typical. Series R_lsb·2^k trims voltage and parallel R_lsb/2^k trims current. N = ceil[3.32·log(X/δX)] (Eq. 6.25): ±0.25% over ±5% needs 4 bits, giving 95 kΩ + 15·667 Ω for a 100 kΩ nominal. Above 6 bits, add a secondary network or use sub-2:1 ratios (L18530–18570; PDF p. 315).
- Differential trim ΔR = R_B²/(R_A + R_B) (Eq. 6.26) for tiny LSBs (L18574–18605; PDF p. 316).
- Remote trim: MOS switches with widths scaled inversely to segment R (4W/L, 2W/L, W/L for R_lsb, 2R_lsb, 4R_lsb) give an equal % on-resistance error. Minimum-width control lines, a clean supply for the inverters, R_S isolating ground during programming, R4–R7 protecting body diodes (Fig. 6.29) (L18606–18642; PDF p. 316).
- Look-ahead trimming lets a binary search program the code. Nonbinary weights 1:2:4:6:12:24:36:72:144… work provided each weight is ≤ 2× the previous. Trimmed accuracy is limited to ±0.05% (thin film ±0.01%) (L18643–18665; PDF p. 317).
- Zener zaps: ≤ 2/3 of E-B BV in normal operation, ≈10 Ω after zapping, 100–250 mA. Not on RBM or silicided contacts. No overcoat opening, so post-package trimming is possible (L18669–18807).
- NVM trim: compact and low current. A non-inverting buffer on the MSB starts the untrimmed value mid-range (L18810–18832).
- Electrical poly trim: current pulses lower R by up to 50% but shift the TC, so it is unsuitable for precise ratios (L18850–18894).
- Laser: notched bar (lateral cut to 90%, then longitudinal), tophat, loop, ladder. Link ablation uses links ≈1 µm × 15 µm with 10–15 µm clearance to other circuitry. Cuts run at 10–30 mm/s, which is costly (L18898–18977; PDF p. 322).

### 6.7 Summary, Bibliography, 6.8 Exercises (L18983–19093)
- Recap. A width correction is needed, each turn adds ≈½ □, and contact end effects matter only for the shortest resistors (L18983–19003).
- Exercises 6.1–6.19 cover sheet R, width/contact corrections, serpentine layout, width selection, TCR, self-heating/granularity length, HSR heads, tweak designs, fuses and trim networks. Ex. 6.18 describes a bimodal R distribution in narrow TiSi₂-silicided poly (C49 phase). Ex. 6.19 asks about a metal plate over boron-doped poly (hydrogen) (L19026–19093).

### Ch. 7 intro (L19099–19114)
- Only a few nF can be integrated. Integrated inductors are small and lossy, so their use is limited to RF.

### 7.1 Capacitance (L19119–19326)
- C = εA/t (Eq. 7.2), ε_r = ε/ε₀ (Eq. 7.3). Table 7.1 (ε_r, MV/cm): Si 11.8/30; dry SiO₂ 3.9/11; PECVD oxide 4.9/3–6; TEOS 4.0/10; LPCVD nitride 6–7/10; PECVD nitride 6–9/5 (L19156–19252; PDF p. 328).
- Example: 0.1 mm² of 200 Å oxide = 170 pF (L19263–19265).
- V_max = t_min·E_max (Eq. 7.4); 200 Å at 5 MV/cm ⇒ 10 V (L19269–19287; PDF p. 329).
- Composite dielectric: ε_r = (t₁+t₂)/(t₁/ε_r1 + t₂/ε_r2) (Eq. 7.5). ONO example: 200 Å nitride (7.5) between two 50 Å oxides ⇒ 5.7 (L19290–19303).
- Capacitor taxonomy and schematic symbols. The curved plate denotes the "outside foil", the electrode designers ground as a shield (L19307–19326).

### Fringing Capacitance (L19345–19428)
- C_F ≈ (εP/π)[ln(2eP/t) + ½·ln(1 + 4t_e/t)] (Eq. 7.6), with P the perimeter and t_e the electrode thickness. For one finite plate over an infinite plate, use t = half the actual thickness (L19364–19380; PDF p. 330).
- Example: a 5 µm square over 200 Å oxide gives 43.1 fF plate + 2.6 fF fringe. Fringe matters only for the smallest capacitors (L19383–19386).
- Line over plane: C/L = ε[W/h + 0.77 + 1.06(W/h)^0.25 + 1.06(t/h)^0.5] (Eq. 7.7). Accuracy is ±2% for W/h > 1 and 0.1 ≤ t/h < 4, and ±6% for W/h ≥ 0.3 and 4 ≤ t/h ≤ 10. For a 1 µm × 5 kÅ line over 8 kÅ, fringe is ≈60% of the total (L19389–19414; PDF pp. 330–331).
- Back-annotation uses correction tables. It does not handle distant-line coupling or power-grid IR, which need a field solver (L19418–19428).

### Derating Amorphous Dielectrics (L19432–19502)
- TDDB: AHI (1/E) and McPherson (E) models. 1 FIT = 1 failure per 10⁹ h, and ≈10 FIT total is budgeted for TDDB (e.g., 9 FIT thin oxide over 0.1 mm², 1 FIT thick over 0.01 mm²) (L19433–19450; PDF p. 331).
- AHI: F₂ = F₁(A₂/A₁)(d₂/d₁)^β·exp[βG(1/E₂ − 1/E₁)] (Eq. 7.8, sign as printed; see H06-33). McPherson: F₂ = F₁(A₂/A₁)(d₂/d₁)^β·exp[βγ(E₂ − E₁)] (Eq. 7.9) (L19460–19488).
- Example: 120 Å oxide at 1 FIT/mm² at 4.5 MV/cm, β = 2.0, γ = 3.5 cm/MV. 1000 µm² at 6 MV/cm for 10% of the time adds 0.36 FIT. Crystalline dielectrics do not show TDDB (L19492–19502; PDF p. 332).

### Junction Capacitance (L19505–19694)
- Reverse bias gives depletion capacitance only. Absolute C = Q/V_R (Eq. 7.10) and dynamic c = dQ/dV_R (Eq. 7.11). Zero-bias c_j0 = ε_Si·A/w₀ (Eqs. 7.12–7.15) (L19506–19601).
- A_total ≅ A_d + (π/2)·x_j·P_d (Eq. 7.16). Empirically c_j0 = c_a·A_d + c_p·P_d (Eq. 7.17). The 40 V bipolar B-E junction gives c_a = 0.82 fF/µm² and c_p = 2.8 fF/µm (L19607–19659; PDF p. 334).
- A comb beats a plate if the finger spacing S_f < 2c_p/c_a (Eq. 7.18). Junction BD is non-catastrophic unless current flows (L19667–19694).

### 7.1.1 Capacitor Variability (L19698–19971)
- Process: gate oxide ±20% (some ±10%), ONO ≥ ±20%, junction plate ≥ ±20%, comb ≥ ±30% (L19706–19747; PDF pp. 335–336).
- Junction c_j = c_j0(1 − V_F/φ_B)^(−m) (Eq. 7.19), with m from 0.5 (abrupt) to 0.33 (graded). Varactors. The peak near 0.7 V forward is not usable (L19752–19795; PDF p. 336).
- Poly depletion: w_d = εV/(qN_P·t) (Eq. 7.20), c = εA/(t + w_d·ε/ε_Si) (Eq. 7.21). Example: 5e19 As, 200 Å, 4.5 MV/cm ⇒ 19 Å, and C drops from 1.73 to 1.67 fF/µm² (L19796–19831; PDF p. 337).
- MOS C–V: c_ox = WLε_ox/t_ox (Eq. 7.22), c_min (Eq. 7.23), V_FB (Eq. 7.24). Example: 1e17 backgate and 200 Å ⇒ V_FB ≈ V_t − 1.8 V. Overdrive ≥ 0.5–1.0 V beyond V_t or V_FB is needed. Without S/D, inversion takes up to hundreds of ms (L19835–19919; PDF pp. 337–338).
- Temperature: thin oxide +20 ppm/°C, nitride +12 ppm/°C. Junction example (1e17/1e19) ≈ +600 ppm/°C, using n_i(T) = 5.29e19(T/300)^2.54·e^(−6726/T) (Eq. 7.25). Poly-poly < 50 ppm/°C (L19929–19971; PDF pp. 338–339).

### 7.1.2 Capacitor Parasitics (L19974–20071)
- Poly-poly model (Fig. 7.9). C₂ is the lower plate to substrate. C₃ is the upper plate to overlying leads, significant when metal routes over. The π split is C_nA = C_nB = C_n/2 (Eqs. 7.26–7.28), with R₁ upper, R₂ lower and R₃ epi. R₁ also models dielectric loss, which rises with frequency (L19980–20025; PDF pp. 339–340).
- Diffused electrodes become diodes. Leakage matters at high temperature, and a forward bias can trigger latchup unless guard-ringed (L20028–20031).
- B-E junction capacitor (Fig. 7.10A) has a pinched-base series R. MOS capacitor (Fig. 7.10B) has lower-electrode R₁, gate R₂, backgate diodes and substrate R₃ (L20035–20071).

### 7.1.3 Comparison of Available Capacitors (L20075–20088)
- Which capacitor types each process family can make. Analog processes add optimized poly-poly, metal-poly, MIM and metal-TiN capacitors (L20076–20088).

### Junction Capacitors (L20092–20170)
- 0.8 fF/µm² at zero bias, 75% of that at −1 V and 50% at −5 V (≈0.5 fF/µm² in use), ±30%, +500…+700 ppm/°C (L20093–20098; PDF p. 341).
- A comb has lower series R than a plate. A spine with short fingers and emitter spacing a couple of µm above minimum is the compromise (Fig. 7.11) (L20101–20108).
- Tank contact required. NBL is useless and adds parasitic (L20127–20133).
- Operate ≤ 75% of BV (6.8 V ⇒ 5 V). Forward bias is 400 mV at room temperature, −2 mV/°C, down to 200 mV at 125 °C. Junction caps over isolation give 100–500 pF (L20142–20170; PDF p. 342).

### MOS Capacitors (L20174–20433)
- Accumulation vs inversion capacitors. Table 7.2 optimal biasing: "NMOS accumulation" V_GB < V_FB − 0.5 V, "NMOS inversion" V_GS > V_t + 0.5 V, "PMOS accumulation" V_GB > V_FB + 0.5 V, "PMOS inversion" V_GS < V_t − 0.5 V. Even 0.5 V may leave C 10% short, so use ≥ 1 V to minimize VCR. The backgate-polarity column prints N-type for both NMOS rows and P-type for both PMOS rows, which is inconsistent with the p. 337 text (L19900–19902). Implement from the physics (L20175–20222; PDF pp. 342–343).
- Selection (P-substrate):
  - One terminal at substrate: NMOS inversion or PMOS accumulation.
  - One terminal at the positive supply: PMOS inversion.
  - Floating, minimize parasitics on the lower-V electrode: PMOS inversion, gate low, source high.
  - Floating, minimize parasitics on the higher-V electrode: PMOS accumulation or isolated NMOS inversion.
  - Polarity reversal: an antiparallel pair of PMOS inversion caps, each C/2, with backgate to source; the dips are offset by 2V_t (L20224–20259).
- Without poly contacts over the gate, lay out like a transistor with S/D on all four sides and backgate to source. The distributed gate R lumps to 1/3 of the end-to-end R with one contacted end, and to 1/4 of that with both ends. Split or reshape to lower it (L20262–20275).
- Channel R_ch ≈ L/(W·k′·(V_GS − V_t)) (Eq. 7.29). If gate R ≫ channel R, reduce W, increase L or add sections (L20285–20299; PDF p. 344).
- With contacts over the gate: a sparse contact array (spacing several × minimum, for antenna and verification load), a PMoat ring on all four sides, an elongated shape (Fig. 7.12A). An accumulation cap uses a backgate-contact ring (Fig. 7.12B) (L20303–20338).
- Bipolar thin-oxide MOS cap on emitter: tie the emitter plate to a low-impedance node. The stacked (sandwich) capacitor is frequently > 1.5 fF/µm² but highly variable, with low breakdown, and is used for compensation and bypass (L20344–20384; PDF p. 345).
- MOS cap on N+ sinker varies ≤ 5% over voltage (L20388–20418).
- **The two electrodes of a MOS cap are never interchangeable. The lower plate carries the junction parasitic and must go to a low-impedance node, and orientation must match the designer's intent** (L20427–20433; PDF p. 346).

### Poly-Poly Capacitors (L20437–20568)
- The heaviest-doped (lowest-sheet) implant makes the best top plate: lower series R and less depletion VCR (L20438–20446).
- The ONO sequence can leave poly-2 stringers, causing shorts and MOS drift. ONO dielectric absorption appears at ≥ 10 MHz, so prefer a single-layer dielectric where C must be constant over frequency. Low-k and large-area dielectrics improve small-cap matching (L20450–20506; PDF p. 347).
- ONO breakdown is asymmetric: up to 50% lower with the lower plate negative (L20515–20527).
- Patterning poly-2 first removes stringers. VCR ≈ 150 ppm/V and TC < 250 ppm/°C (unsilicided) (L20531–20548; PDF p. 348).
- Oxide steps should not intersect poly-poly caps. Deep-N+ under LOCOS thickens the oxide, which does not work with STI (L20556–20561).
- A silicided lower plate still depletes at the top plate. Split into two equal antiparallel halves ⇒ 2 ppm/V (L20564–20568).

### Metal-Metal Capacitors (L20579–20661)
- Metal electrodes do not deplete, so VCR and TC come only from the dielectric (L20580–20585).
- CVD oxide needs 800–1000 °C densification, but Al tolerates only 400–450 °C. Options are an RBM lower plate (2 extra masks) or a silicided-poly lower plate (1 mask), the latter with VC 2–4 ppm/V. 600 Å nitride between silicide and TiN gave −13 ppm/V, −4 ppm/V² and 39 ppm/°C (L20588–20633; PDF pp. 348–349).
- Si–H bonds cause residual nitride VCR (L20637–20640).
- ARC (SiON) + TiN capacitor: one mask, insertable between any two metals, can sit over circuitry wired in poly/M1 (L20649–20661).

### Stack Capacitors (L20689–20737)
- ILO is 5–10 kÅ with a high voltage rating and no extra masks. Nearly 30,000 µm² of 10 kΩ… (sic: 10 kÅ) ILO is needed per 1 pF (L20690–20694; PDF p. 350).
- A poly–M1–M2 stack is ≈2× the density. The contact/via ring is staggered for Al and stacked for W plugs (L20697–20721).
- **Plates have radically different parasitics.** The sandwiched M1 electrode has almost none, while the poly/M2 electrode couples to substrate and to overhead leads, so connect in the correct orientation. A poly-poly plus gate-oxide stack has the same asymmetry (N-well plate carries the well junction) (L20725–20737; PDF p. 351).

### Lateral Flux Capacitors (L20741–20806)
- Horizontal-bar lateral flux (alternating A/B strips on each layer, each strip surrounded by the other electrode) is competitive when the lateral metal spacing < ILO thickness (5–10 kÅ). Vertical-bar pillars (lateral flux in 2D, no vertical flux) win when the spacing ≪ ILO (L20742–20757; PDF p. 351).
- Woven structure: same C as horizontal bars, but the via lattice lowers series inductance. Fractal: trades series R against density (L20778–20806; PDF p. 352).

### Trench Capacitors (L20811–20907)
- High aspect ratio gives very high density: 35 nm nitride reaches ≥ 3 orders above planar, and multi-concentric trenches have reached 0.4 µF/mm². Resistance and substrate-common electrode issues are solved with a sidewall implant or sinker trenches. Hexagonal array. Round the top corners for voltage (L20812–20907; PDF pp. 353–354).

### Summary of Available Capacitor Types (L20911–20976)
- Table 7.3 (fF/µm², ±%, V):
  - Bipolar: B-E\* 0.8/50†/5; emitter oxide 0.07/30†/40.
  - Poly-gate CMOS: GOX 0.86/20†/15; poly-poly 1.3/25/15; stack 0.05/30/40.
  - Analog BiCMOS: thick GOX 2.2/20†/7; thin GOX 4.3/20†/3.6; TiN 3.1/20/7; lateral flux 0.07/30/40 (L20924–20976; PDF p. 354).

### 7.2 Inductance (L20981–21198)
- V = L·dI/dt (Eq. 7.30). Integrated L is tens of nH, useless below 100 MHz. Loop inductance matters, not self-inductance (L20982–21015; PDF p. 355).
- Loop: L = µr[ln(8r/a) − 1.75] (Eq. 7.31), µ₀ = 1.26 µH/m. A bondwire (25 µm, 1 mm loop) is ≈5.6 nH. di/dt > 5×10⁷ A/s on 5 nH ⇒ ≥ 0.25 V, so gate drivers need thorough guard rings and PCB loop-area control (L21024–21079; PDF p. 356).
- Toroid L = µN²r²/D (Eq. 7.33) (L21082–21097).
- Planar spirals: circular has the least series R for a given L, square the most. Octagons are the practical compromise (L21101–21105).
- Mohan: L = µ·K₁N²(d_o + d_i) / {2[1 + K₂(d_o − d_i)/(d_o + d_i)]} (Eq. 7.34), with square K₁ = 2.34, K₂ = 2.75 and octagonal K₁ = 2.25, K₂ = 3.55. d_i = d_o − 2Np (Eq. 7.35). The book's example (300 µm, 10 turns, 9/1 µm) states 18.6 nH, but Eq. 7.34 with those inputs evaluates to ≈24.8 nH, so validate before use (L21128–21156; PDF p. 357).
- A symmetric spiral with crossover jumpers (Fig. 7.23) is for differential circuits and also lowers losses for single-ended ones. Transformers are used as combiners and baluns (L21159–21189).

### 7.2.1 Inductor Parasitics (L21200–21414)
- DCR = ρL/A (Eq. 7.36). IC metal is thin, and Al is ≈60% more resistive than Cu. Remedies are wider turns, strapped layers and thick top Cu (L21208–21239; PDF pp. 358–359).
- Eddy currents: skin depth δ = √(ρ/(πfµ)) (Eq. 7.37), ≈2.3 µm for thin-film Al at 1 GHz. Proximity effect grows with turns and layers. Substrate eddy loss dominates the AC R on moderate-resistivity substrates (L21243–21281; PDF p. 359).
- Mitigations: substrate > 10 Ω·cm (preferably > 100), a MEMS cavity, or magnetic shielding (core losses from 100 MHz–10 GHz). In a standard process, elevate the spiral to the top metal (L21284–21324).
- Model (Fig. 7.24). f_crit = p·R_s/(2µw²) (Eq. 7.38); a 5 kÅ Al spiral with 10 µm pitch, 9 µm width and 50 mΩ/□ ⇒ 2.4 GHz. R_W = R_DC[1 + (1/10)(f/f_crit)²] (Eq. 7.39) for d_i ≈ d_o/3 (L21327–21366; PDF p. 360).
- Q = 2πfL/R_s (Eq. 7.40), peaking between ≈1 and 40 (discrete > 100). The SRF caps the usable frequency and is usually > 1 GHz. Use a 3D or 2.5D field solver (HFSS, EMPro) (L21377–21414; PDF p. 361).

### 7.2.2 Inductor Construction (L21418–21488)
- Automatic generators iterate layout and field solve (MIDAS). Standard CMOS on low-ρ substrates struggles to reach Q > 5–10 (L21419–21435).
- Elevate: thick metal above the stack or on polyimide above the overcoat. A slotted metallic shield raises Q: strip widths are chosen so f_crit > f_op, and silicided poly is the best shield material (Fig. 7.25) (L21435–21442; PDF p. 361).
- Strap the top 2–3 metals. Dedicated inductor metals are 2–5 µm. Best parts are 10–100 nH at Q ≈ 40 up to a few GHz (L21463–21479; PDF p. 362).

### Guidelines for Integrating Inductors (L21494–21567)
1. Highest-resistivity substrate (< 10 Ω·cm is bad), with guard rings and scattered substrate contacts.
2. Body on the highest metals, the underpass jumper beneath and never above.
3. Strap 2–3 metals if there is no thick top metal, and avoid metal-1.
4. Keep unconnected metal away by ≥ ½ the inductor width. Circuitry in the hollow center lowers Q.
5. Width 10–15 µm is optimal at 1–3 GHz; also use Eq. 7.38.
6. Narrowest turn spacing.
7. ≤ 1–2 winding layers (SRF drops); strapped layers count as one.
8. Inner diameter ≥ 5 × the metal width, and ≥ d_o/3 for large inductors.
9. No metal or poly above or below except slotted shields. Remove dummy fill out to ≈½ the spiral width, or size it to minimize eddy loss.
10. No junctions beneath (rectification, injection).
11. Short, direct leads on the highest metal.

(L21510–21567; PDF p. 363)

### 7.3 Summary, Bibliography, 7.4 Exercises (L21573–21766)
- Absolute accuracy is ±30% for junction caps and ±10% for thin-film caps. Capacitors are hard to trim, so circuits trim resistors or currents, or rely only on capacitor ratios. Only ≈100 nH can be integrated, with low Q (L21573–21600; PDF p. 364).
- Exercises 7.1–7.15 cover:
  - oxide thickness for 15 V;
  - ONO permittivity and voltage;
  - thick Cu over PO capacitance;
  - NSD junction cap with sidewalls;
  - extracting c_a/c_p and the comb threshold;
  - junction, thin-oxide, poly-poly and MOS cap layouts (sparse gate contacts);
  - accumulation modification;
  - M1 fringe fraction;
  - horizontal-bar lateral flux sizing;
  - square spiral for a target L;
  - thick-metal benefit;
  - a circular Cu spiral built from filleted L-paths plus skin-effect onset.

  (L21640–21766)

---

## 3. Actionable extraction

Conventions: R_s sheet resistance (Ω/□), W_b/L_b width/length bias (nm), □ squares. "Op-point" means Philis `frontend/library/src/oppoint.rs`. That module today carries per-device power, I_d, headroom and g_m only. It has no node voltages and no currents for non-FET devices (`oppoint.rs:13-29`, `oppoint.rs:32-38`), so several checks below first need node voltages (V per net) and branch currents for R and C added to `OpPoint`.

### H06-01 Deck-driven resistor value model
- Kind: formula, data-model, deck-requirement
- Statement:
  - Effective W = W_d + W_b and L = L_d + L_b (Eqs. 6.5–6.6).
  - Strip resistor: R = R_s(L_d + L_b)/(W_d + W_b) (Eq. 6.7).
  - Implanted-head poly: R = R_s(L_d − 2L_b)/(W_d + W_b) + 2R_h(L_h + L_b)/(W_d + W_b) (Eq. 6.22), with R_h the head sheet resistance and L_h the head-implant overlap beyond the contact.
  - Silicide-blocked poly with silicided heads: R ≈ R_s·L_d/(W_d + W_b) (Eq. 6.23).
  - The width bias dominates (diffusion W_b ≈ 0.2·x_j). The fab's width-bias tables take precedence (L16309–16314).
- Source: §6.2 Eqs. 6.5–6.7, §6.5.7 Eqs. 6.22–6.23; L16285–16314, L17729–17744, L17791–17806; PDF pp. 279, 303–304.
- Philis stage: deck, cells, annotator (sizing), verify (LVS parameter check).
- Automation recipe:
  - Inputs: per-recipe deck entries `sheet_ohm_sq`, `head_sheet_ohm_sq`, `width_bias_nm`, `length_bias_nm`, `head_overlap_nm`, and a W-dependent bias table (dilution/bamboo region).
  - When the netlist gives R (not W/L), solve Eq. 6.22 for L_d at the chosen W_d.
  - When it gives W/L, report the drawn-vs-model R error.
  - Output: drawn L_d snapped to grid; residual ΔR/R emitted as a hard check (|ΔR/R| ≤ grid quantization).
- Beats hand layout because the value is recomputed exactly for every width, recipe and segment count, instead of the hand habit of "L = R/R_s·W" with ad-hoc corrections.
- Philis status: **partial (R_s only, outside the flow)**.
  - `Process::sheet_ohm(role)` exists and reads the deck `pex` `sheet_res_ohm_sq` for the resistor layer (`kernel/core/src/process.rs:19`, `backend/verify/src/pdk.rs:965-967`).
  - Only benchmark fixture generation uses it, with the uncorrected Eq. 6.3 `l = r·w/r_sheet` (`benchmarks/src/fixtures.rs:550-551`, `:743-744`). That form has no W_b, L_b or heads.
  - The cell generator never reads it. `resistor.rs:81-82` uses the netlist W/L directly (`body_w = unit_w`, `seg_l = unit_l / n_segments`).
  - The sky130 recipe block has no bias or head-sheet keys (`pdks/sky130.json:137-171`).
  - `sheet_tolerance`/`linewidth_control_nm` are empty (`pdks/sky130.json:74-75`) and no Rust code reads them.

### H06-02 Segmentation must preserve the resistor value (heads and contacts per segment)
- Kind: formula, check
- Statement:
  - Each segment of a series string of N separately contacted strips carries two heads and two contacts.
  - R_total = N·[R_s(L_seg − 2L_b)/(W + W_b) + 2R_h(L_h + L_b)/(W + W_b) + 2R_c], with R_c from Eq. 6.19.
  - Splitting a body of length L into N strips of L/N therefore adds 2(N−1)(R_head + R_c) relative to one strip.
  - For short segments this is not small: a 1 kΩ base resistor with two 43 Ω contacts is +9% (L17014–17015).
  - Parallel longer segments are preferred to enlarged heads when a segment is < 10 □ (L17016–17018).
- Source: §6.3.4 Eq. 6.19 + example, §6.5.7 Eq. 6.22; L16932–17018, L17729–17744; PDF pp. 290–291, 303.
- Philis stage: cells, verify.
- Automation recipe:
  - For each candidate segment count N in `feasible_segments`, solve for L_seg such that R_total(N) = R_target.
  - Reject N where the head/contact share exceeds a budget (e.g., > 1% for matched classes).
  - Emit the per-segment L and the modeled R into the macro metadata, so the LVS reference and `perf` read the same value.
- Beats hand layout because every enumerated variant is value-exact, whereas manual folding usually keeps the body length and silently adds head resistance.
- Philis status: **missing**.
  - `resistor.rs:82` sets `seg_l = unit_l / n_segments`.
  - `resistor.rs:313-337` picks N purely by aspect ratio and adds no head/contact compensation.

### H06-03 Minimum segment length and end-effect rule
- Kind: rule
- Statement:
  - Make resistors long enough that end effects vanish.
  - Accurate resistors should use segments ≥ 10 µm long (L16358–16360).
  - Lateral end correction is < 1% for resistors ≥ 10 □ (L16345–16347).
  - Vertical crowding is negligible for L ≥ 20·t (L16353–16354).
  - Poly grain nonlinearity is negligible for L ≥ 1000 × the grain size (≈50 µm), and not severe at 10 µm (L16852–16856).
- Source: §6.2, §6.3.3; L16326–16360, L16852–16856; PDF pp. 280, 288.
- Philis stage: cells, deck.
- Automation recipe: hard lower bound L_seg ≥ max(`res_min_segment`, 10·W, 20·t_film). For matched classes, raise it by a tolerance class (see LAYOUT-FUNDAMENTALS #22).
- Beats hand layout because it is applied uniformly, including to every enumerated fold.
- Philis status: **implemented (partial)**. `res_min_segment` = 10 µm (`pdks/sky130.json:49`) is the default body and minimum segment (`resistor.rs:295-299`, `resistor.rs:318-319`). There is no squares-based or thickness-based floor.

### H06-04 Contact-end nonuniform-current correction (Ting–Chen)
- Kind: formula
- Statement: ΔR = (R_s/π)·[(1/k)·ln((k+1)/(k−1)) + ln((k²−1)/k²)], with k = W/(W − W_c), for both ends together. It is strictly valid for W_c ≫ W − W_c. Example: 5 µm wide with 3 µm contacts adds 0.05 □.
- Source: §6.2 Eq. 6.8; L16326–16347; PDF p. 280.
- Philis stage: cells.
- Automation recipe: add ΔR to the H06-01 model whenever the head contact is narrower than the strip (sky130 slot contacts are 190 nm wide, `pdks/sky130.json` recipe `res_contact_w`). Evaluate it only for segments < 10 □ and skip it otherwise.
- Beats hand layout because the correction is included automatically for short, trim or LSB segments, where hand estimates ignore it.
- Philis status: **missing** (no end-correction term anywhere in `resistor.rs`).

### H06-05 Bend squares for serpentines
- Kind: formula
- Statement:
  - A square corner adds 0.56 □ (≈½ □). For Fig. 6.3A, R = R_s[(2A + B)/W + 1.12] (Eq. 6.9).
  - Unequal widths (Hall, Eq. 6.10): N = 1/a − (2/π)·ln(4a/(a²+1)) + ((a²−1)/(πa))·cos⁻¹((a²−1)/(a²+1)), which gives 0.5587 □ at a = 1.
  - A 180° circular end adds 2.96 □: R = R_s[2C/W + 2.96] (Eq. 6.11).
  - Use circular turns and filleted corners for high-voltage resistors (L16363–16366).
- Source: §6.2 Eqs. 6.9–6.11, Fig. 6.3–6.4; L16363–16436; PDF pp. 280–281.
- Philis stage: cells, deck.
- Automation recipe: if a bent (continuous-poly) serpentine variant is added, count corners and add N_corner(a) per corner in the H06-01 model. Offer rounded turns when V_max across the resistor exceeds a deck HV threshold.
- Beats hand layout because the value stays exact for any fold count.
- Philis status: **dead config**.
  - `res_corner_squares: 0.56` and `res_serpentine_aspect: 10.0` exist in `pdks/sky130.json:47-48`, but no Rust code reads them (grep: no hits).
  - The generator uses straight strips with li/met1 jumpers (`resistor.rs:137-155`), so there are no poly corners.
  - Either implement a bent-serpentine variant using the key, or delete both keys.

### H06-06 Identical-section unitization for matched and ratioed resistors
- Kind: rule, algorithm
- Statement:
  - Matched resistors should always be built from identical sections. When they are, the head style (dogbone or strip) does not affect matching (L16486–16490).
  - Dogbone heads lower the value by 0.3–0.7 □ (Table 6.3). The correction cancels only if every section has the same head.
- Source: §6.2 Table 6.3; L16462–16490; PDF p. 282.
- Philis stage: annotator, cells.
- Automation recipe:
  - For every resistor set with a ratio constraint (divider, gain-setting pair, R-2R, trim ladder), choose a unit R_u (W, L_seg, heads) such that each member is n_i·R_u in series or R_u/m_i in parallel.
  - A non-integer ratio gets one "tweak" segment common to all members, or the ratio error is reported.
  - Members share the Unitization, so segments are interdigitated and common-centroided.
- Beats hand layout because the unit choice is searched (minimize Σ ratio error + area) rather than guessed.
- Philis status: **partial**.
  - The annotator unitizes only devices with identical (kind, W, L) (`backend/annotator/src/constraints.rs:28-62`). Resistors of different lengths (a 1:2 ratio) land in different classes and are never cut into a common unit.
  - The annotator catalog has no resistor-divider pattern (grep for `Resistor` in `catalog.rs`/`pattern.rs`: only comments).

### H06-07 Tolerance model and width selection by tolerance class
- Kind: formula, algorithm, deck-requirement
- Statement:
  - δR ≅ (δW/W + δR_s/100%)·100% (Eq. 6.12).
  - Linewidth control is ≈ ±10% of the minimum feature for deposited layers; diffusions add ±5% of x_j.
  - Guidelines:
    - Tolerance does not matter: minimum width.
    - Moderate: 2–3× minimum width, poly ≥ 0.3 µm, diffusion ≥ 2·x_j, no poly above 5 kΩ/□, no pinch resistors.
    - Crucial: trim, poly ≥ 0.5 µm, no lightly doped materials.
  - Example: ±20% R_s with ±0.05 µm gives ±30%, ±25% and ±21% at 0.5, 1 and 10 µm.
- Source: §6.3.1 Eq. 6.12 + guidelines; L16525–16609; PDF pp. 284–285.
- Philis stage: deck, annotator, cells.
- Automation recipe:
  - Deck: `sheet_tolerance{recipe: %}` and `linewidth_control_nm{layer: nm}`.
  - The annotator assigns each resistor a tolerance class from its role: bias/reference → moderate, gain/ratio → crucial for matching (ch. 8), pull-up → none.
  - Cells pick W = argmin area s.t. δR(W) ≤ class budget and W ≥ class floor.
  - The result is a hard floor on W and a cost on area.
- Beats hand layout because every resistor gets a quantitatively justified width, not a "2× min" habit.
- Philis status: **missing**.
  - Keys `sheet_tolerance` and `linewidth_control_nm` exist but are empty (`pdks/sky130.json:74-75`) and unread.
  - `res_min_width` (`pdks/sky130.json:84`) is the only width floor (`resistor.rs:81`).

### H06-08 Narrow-width anomaly floors
- Kind: rule, deck-requirement
- Statement:
  - Poly bamboo effect: grains ≤ 0.1 µm, harmless above ≈0.25 µm.
  - TiSi₂ below 0.2 µm stays C49 (80–100 vs 13–20 µΩ·cm), so TiSi₂-silicided resistors need W ≥ 0.25 µm. Small silicide areas may also fail to transform.
  - Diffused and well resistors dilute below ≈2·x_j, where W_b becomes width-dependent. Wells need W ≥ 2–3·x_j.
  - Ex. 6.18: a bimodal R distribution in narrow silicided poly means C49.
- Source: §6.3.1, §6.5.9; L16560–16594, L17953–17959; PDF pp. 284–285, 306.
- Philis stage: deck, cells.
- Automation recipe: the deck carries `precision_min_width_nm{recipe}` and, for diffusions/wells, `junction_depth_nm` (sky130 has `n_well_depth`, `p_well_depth`: `pdks/sky130.json:66-67`). Cells enforce W ≥ max(DRC min, anomaly floor) for any resistor in a matched or precision class.
- Beats hand layout because the anomaly floors are process physics that DRC does not flag.
- Philis status: **missing** (only `res_min_width`).

### H06-09 Resistor temperature-coefficient data model
- Kind: data-model, formula
- Statement:
  - R(T) = R(T₀)[1 + α(T−T₀)] (Eq. 6.13), or the quadratic form (Eq. 6.14) when the range is wide. Coefficients are regression fits valid only over their range.
  - Table 6.4 (ppm/°C): 500 Ω/□ poly −1000; 25 Ω/□ poly +1000; 70 Ω/□ poly +500; 10 kΩ/□ poly −7000; 0TC at 200–300 Ω/□ ±100; metal ≈ +3300–4000; N-well 10 kΩ/□ +6000.
- Source: §6.3.2 Eqs. 6.13–6.14, Table 6.4; §6.5.6–6.5.7; L16613–16726, L17647–17711; PDF pp. 285–286, 302–303.
- Philis stage: deck, gp/dp (thermal cost), verify.
- Automation recipe:
  - Deck: `tc1_ppm_per_k`, `tc2_ppm_per_k2` and fit range per resistor recipe.
  - Placement: the thermal-gradient mismatch cost for matched resistor pairs uses ΔR/R = α·ΔT between partners, with ΔT from the thermal field.
  - Verify: report worst ΔR/R at T_min/T_max.
- Beats hand layout because the α-weighted isotherm placement is computed per recipe; hand layout uses one rule of thumb for all materials.
- Philis status: **partial**. `ThermalGradient` prices ΔT between partners (`kernel/analog/src/placement/thermal.rs:7-20`), but there is no per-recipe TCR, so the ΔT budget does not scale with α.

### H06-10 TC-compensated composite resistors
- Kind: heuristic, algorithm
- Statement: designers combine positive-TC low-sheet and negative-TC high-sheet poly in series or parallel to cancel TC. High-sheet TC is poorly monitored by fabs, but series-parallel networks exist that are insensitive to small TC variation (Gregoire–Moon).
- Source: §6.3.2; L16730–16738; PDF p. 286.
- Philis stage: annotator (recognize), cells.
- Automation recipe: recognize a two-material composite (series R with different recipes on one branch) and keep its sections under one isotherm (a common-centroid of both sub-resistors). Do not segment them apart.
- Beats hand layout because the thermal co-location is enforced for mixed-material composites that human layout often places by material.
- Philis status: **missing** (annotator has no resistor patterns).

### H06-11 Voltage coefficient and body/tank modulation
- Kind: data-model, check
- Statement:
  - R(V) = R(V₀)[1 + β₁ΔV + β₂ΔV²] (Eq. 6.15).
  - Values: base 100 ppm/V; high-sheet diffused 2.5 %/V; base pinch 6 %/V and 2 %/V²; body modulation for a 700 Ω/□ resistor 1000 ppm/V and −20 ppm/V²; overlying-lead modulation 0.1 %/V on 2 kΩ/□ HSR.
  - Poly ≤ 1 kΩ/□ is largely immune (L16920–16928).
  - A divider with both bodies at V_CC drifts in ratio. Giving each equal segment its own body tied to its positive end makes the modulation track (L17199–17212; §6.5.4 L17470–17477).
- Source: §6.3.3 Eq. 6.15, Fig. 6.13; L16788–16928, L17199–17212, L17470–17477; PDF pp. 287–289, 294, 299.
- Philis stage: deck, annotator, verify.
- Automation recipe:
  - Deck: `vc1_ppm_per_v`, `vc2_ppm_per_v2` per recipe.
  - With op-point node voltages, compute each matched resistor's R(V) and report the ratio error of dividers.
  - For body-bearing recipes (well/diffused), emit a body-net constraint: body = the segment's positive terminal.
- Beats hand layout because ratio error under actual bias is computed for every divider, not assumed zero.
- Philis status: **missing** (no VCR keys; OpPoint has no node voltages, `oppoint.rs:13-29`).

### H06-12 Self-heating voltage coefficient
- Kind: check, formula
- Statement: β₂ ≅ α·t_ox·R_s/(κ·R₀²·W²) (Eq. 6.16), with κ(SiO₂) ≈ 0.013 W/(cm·°C). A negative α gives decreasing R with V.
- Book example: 10 kΩ, W = 5 µm, 500 Ω/□, α = −1000 ppm/°C, t_ox = 1 µm gives "−1.5 %/V²".
- **Caution:** evaluating Eq. 6.16 with those inputs gives |β₂| ≈ 1.5×10⁻⁴ V⁻² (0.015 %/V²). The example and the equation disagree by ×100, so validate the units before coding.
- Poly handles less power than diffusion because of the oxide, and overheating permanently shifts R and TC (L17786–17788).
- Source: §6.3.3 Eq. 6.16; L16806–16828; PDF pp. 287–288.
- Philis stage: verify, cells.
- Automation recipe: for each resistor, get V across it from the op-point. ΔR/R_self = β₂·V². For matched pairs, check the self-heating mismatch |β₂,A·V_A² − β₂,B·V_B²| ≤ budget. If it fails, widen W (β₂ ∝ 1/W² at fixed R and R_s). Also cap the power density P/A.
- Beats hand layout because self-heating is evaluated per resistor at the actual bias, which hand layout never does.
- Philis status: **missing**. `elaborate.rs:246-247` explicitly notes that local self-heating is not fed back (it is used for EM only).

### H06-13 Velocity-saturation and grain-limited minimum length
- Kind: check
- Statement: critical fields are 0.2 V/µm (electrons) and 0.6 V/µm (holes). With a safety factor 2, L_min = (10 µm/V)·V_max for N-type silicon (Eq. 6.17) and (3.3 µm/V)·V_max for P-type (Eq. 6.18). For poly, use L ≥ 1000 × the grain diameter to avoid grain-barrier nonlinearity.
- Source: §6.3.3 Eqs. 6.17–6.18; L16832–16856; PDF p. 288.
- Philis stage: verify, cells.
- Automation recipe: for diffused and well resistor recipes, require L_total ≥ k·V_max using the op-point voltage across the resistor. For poly, apply the grain floor from the deck `grain_nm`.
- Beats hand layout because it is a per-instance hard check.
- Philis status: **missing**.

### H06-14 Leads over resistor bodies: conductivity modulation and coupling
- Kind: rule, cost term
- Statement:
  - Leads crossing lightly doped resistors modulate them (0.1 %/V on 2 kΩ/□ HSR). Such resistors need field plates, preferably split plates (L16920–16928).
  - Poly ≤ 1 kΩ/□ is not modulated, but capacitive coupling remains. Reroute noisy leads so they do not cross (L17273–17279).
  - HV diffused resistors above 2/3 of the top-metal thick-field threshold need field plating against charge spreading (L17273–17274, L17470–17473).
  - When leads must cross a high-sheet resistor, stretch the low-sheet head and cross over the head instead of the body. The head's R is then Eq. 6.21 with k = 1 (L17481–17487, Fig. 6.16).
  - A single-mask thin-film resistor blocks routing on the metal level immediately above it (L18062–18063).
- Source: §6.3.3, §6.5.1, §6.5.4, §6.5.10; L16920–16928, L17273–17279, L17470–17487, L18062–18063; PDF pp. 289, 296, 299, 308.
- Philis stage: gr, dr, annotator.
- Automation recipe:
  - Every resistor body polygon becomes a routing keep-out region.
  - For foreign nets the cost is hard where the recipe is lightly doped (R_s > 1 kΩ/□ or a well). Otherwise it is priced by the aggressor's activity class times the overlap area (see H06-18).
  - Optional field-plate generation: a metal plate over the body tied to the segment's positive end, or to a quiet net.
  - Crossing corridors: mark each resistor's contacted heads as zero-penalty crossing regions. A foreign net that must cross the device then pays nothing over the heads and the keep-out price over the body.
- Beats hand layout because every crossing is priced and every exception is recorded.
- Philis status: **partial**. gr charges `KEEPOUT_COST` (2.0) per node inside other *matched* cells (`backend/gr/src/lib.rs:749-757`, `:874-878`; populated in `backend/dr/src/lib.rs:480-495`). This covers matched cells only, has no material dependence, and has no field-plate option. Compare LAYOUT-FUNDAMENTALS #25/#56.

### H06-15 Contact resistance and contact sizing
- Kind: formula, rule, deck-requirement
- Statement:
  - R_c = (√(R_s·ρ_c)/W_c)·coth(L_c·√(R_s/ρ_c)) (Eq. 6.19), where ρ_c is in Ω·µm².
  - Table 6.5 (Ω·µm²): Al–Si to base 750, RBM to base 2500, PtSi to base 1250, Al to emitter 40, TiSi₂ to NSD 30, TiSi₂ to PSD 100.
  - Unsilicided barrier metal raises R_c and its variability.
  - For short segments, use larger contacts or, better, parallel longer segments rather than dogbone heads.
- Source: §6.3.4; L16932–17018; PDF pp. 289–290.
- Philis stage: deck, cells.
- Automation recipe: the deck carries `rho_c_ohm_um2` per contact type (licon to poly/implant). Cells fill each head with the maximum number of cuts (already done) and compute R_c. If R_c/R_segment > budget (e.g., 1%), prefer the variant with fewer, longer parallel segments (H06-02).
- Beats hand layout because contact resistance is computed for every variant and weighed against the other variants.
- Philis status: **missing** (no ρ_c in the deck; `resistor.rs:89-92` fills cuts but never evaluates R_c).

### H06-16 Poly hydrogenation drift in recipe choice
- Kind: heuristic
- Statement: high-sheet P-type poly drifts up by several % over hundreds to thousands of hours at high temperature (dehydrogenation of 0.5 eV bonds). N-type high-sheet poly with P > B drifts less. Compensated P-type poly (B > P) drifts more. This matters for current-setting resistors.
- Source: §6.3.5; L17022–17051; PDF pp. 290–291.
- Philis stage: annotator (flag), deck.
- Automation recipe: tag each recipe with a `drift_class` in the deck. When the annotator marks a resistor as a bias/reference current setter, warn if the recipe's drift class is high and an alternative recipe exists.
- Beats hand layout because the long-term drift risk is surfaced at synthesis time.
- Philis status: **missing**.

### H06-17 Distributed parasitic model of a resistor
- Kind: data-model
- Statement:
  - Poly over field oxide: ≈0.05 fF/µm² × body area, distributed. A 1 µm × 1000 □ resistor is ≈50 fF.
  - Single-π: C/2 at each end. Dual-π: C/4, C/2, C/4 with R/2 + R/2 (Fig. 6.10).
  - Diffused resistors: body diodes to the body node and body-to-substrate diodes (Fig. 6.12A/B), with junction C of 1–5 fF/µm².
- Source: §6.4; L17069–17077, L17144–17170, L17229–17232; PDF pp. 292–294.
- Philis stage: verify (PEX), flow (perf).
- Automation recipe: for every resistor instance, emit a dual-π subcircuit into the `perf` netlist, using the deck's area capacitance of the resistor layer to substrate (`pex` area term). Use it for bandwidth/pole checks on high-value resistors in feedback paths.
- Beats hand layout because the parasitic pole of every high-value resistor is modeled, not just the one the designer remembers.
- Philis status: **missing**. PEX supplies wire R/C (`backend/verify/src/pdk.rs:680-703`), and no device-body model is emitted for resistors.

### H06-18 Crossing-lead coupling budget over high-impedance resistor nodes
- Kind: check, metric
- Statement: interlevel oxide is ≈0.05 fF/µm², so a 1 µm × 1 µm crossing couples ≈0.05 fF. On a 0.1 pF high-impedance node, a 2 Vpp digital lead injects ≈1 mVpp, enough for a 16-bit converter to lose bits. Long parallel runs are far worse. V_victim = V_aggr·C_c/(C_c + C_node).
- Source: §6.4; L17110–17123; PDF pp. 292–293.
- Philis stage: annotator, gr, dr, verify.
- Automation recipe: C_c = Σ overlap_area × area-cap(layer pair) + fringe terms, over aggressor wires crossing resistor bodies attached to high-impedance nets. Compare V_victim with the net's noise budget from classification (`max_coupling_af`). The result is a hard fail when V_victim exceeds the budget, and a cost in gr.
- Beats hand layout because every crossing is summed quantitatively.
- Philis status: **partial**.
  - Net coupling budgets exist (`backend/annotator/src/classify.rs:40-45`, `:92-101`), but resistor bodies are not wires, so their crossings are not counted.
  - Side finding: `classify.rs:61-84` applies the FET slot indices (G = 0, D = 1, `classify.rs:12-15`) to every non-capacitor device. A resistor's first terminal is therefore marked `touches_gate` and its second `touches_channel`, which skews net classes for resistor-connected nets.

### H06-19 Diffused/well resistor body biasing and voltage rating
- Kind: rule, check
- Statement:
  - The body must keep the resistor–body junction reverse-biased. Bodies that connect neither to the resistor's positive end nor to a supply can transiently forward-bias, causing latchup or large body currents.
  - Do not exceed ≈2/3 of the resistor–body voltage rating (the same for emitter and pinch resistors with respect to E-B BV).
  - For higher V, segment into separate bodies, each tied to its positive end.
  - Avoid merging with minority-carrier injectors (L17293–17299).
- Source: §6.4 Fig. 6.13; §6.5.1–6.5.4; L17173–17225, L17293–17299, L17340–17346, L17407–17409, L17502–17508; PDF pp. 293–294, 296–299.
- Philis stage: annotator, cells, verify (ERC).
- Automation recipe: for body-bearing recipes (sky130 `res_generic_nd/pd`, `res_xhigh_po` over nwell where applicable, well resistors), generate a body tap tied by default to the positive terminal. ERC checks V_body ≥ max(V_P, V_N) (N-body) with a margin. If |V_P − V_N| > (2/3)·BV, split into k segments with separate bodies.
- Beats hand layout because the check is exhaustive for body-bearing resistors.
- Philis status: **missing**. No diffused/well resistor generator exists, and guard rings are generated for FETs only (`backend/annotator/src/constraints.rs:65-69`).

### H06-20 Resistor material/recipe selection
- Kind: algorithm
- Statement:
  - Poly is the best general resistor. Use diffused resistors for high power and ESD, thin film for extreme precision, metal for sub-ohm sensing, and wells only when high-sheet poly is absent.
  - Table 6.7 gives typical R_s, ±% and V per type.
  - 5 kΩ/□ poly varies ±50%, versus ±20% for 500 Ω/□ (L17679–17680).
  - 0TC poly sits at 200–300 Ω/□ (L17703–17711).
- Source: §6.5.7, §6.5.11 Table 6.7; L17670–17711, L17887–17890, L18092–18196; PDF pp. 302–303, 305, 308–309.
- Philis stage: annotator, cells, deck.
- Automation recipe: when the schematic model is generic, or allows alternatives, score each deck recipe by area (R/R_s·W²), tolerance (H06-07), TC (H06-09), VCR (H06-11), power (H06-12) and drift (H06-16) for the resistor's role, and pick the minimum-cost legal one.
- Beats hand layout because the choice is exhaustive and reproducible.
- Philis status: **partial**. The deck has per-model recipes (`pdks/sky130.json:137-171`, default `high_po`), but the choice follows the schematic model. There is no scoring.

### H06-21 Poly resistor placement over field oxide and HV cone-defect segmentation
- Kind: rule
- Statement:
  - Place poly resistors on thick field oxide, not gate oxide or active, to minimize the parasitic C to substrate.
  - STI cone defects can break down below 20 V under poly. Screen them, or segment HV poly resistors over separately biased wells so the STI field stays below the threshold.
- Source: §6.5.7; L17774–17782, L17877–17883; PDF pp. 303–305.
- Philis stage: cells, gp/dp (no active under the body), verify.
- Automation recipe: a DRC-like check that no diffusion or active lies under a resistor body. For resistors with V_max > deck `sti_cone_v`, generate per-segment wells biased to track the segment voltage.
- Beats hand layout because the check runs on every instance.
- Philis status: **implemented by construction** (the generator draws no diffusion under the body, `resistor.rs:112-115`). The HV segmentation is **missing**.

### H06-22 Voltage-dependent spacing between resistor segments
- Kind: check
- Statement: dense poly lines risk ILO TDDB, so modern decks carry voltage-dependent poly spacing. Use spacing equal to the block operating voltage for everything except adjacent turns of one resistor. For a resistor of N equal segments with V_d across it, the maximum adjacent-segment voltage is ΔV = 2V_d/N (Eq. 6.24).
- Source: §6.5.7 Eq. 6.24; L17857–17874; PDF p. 305.
- Philis stage: cells, verify, deck.
- Automation recipe:
  - Deck: a `poly_space_vs_v` table.
  - Single-pattern strings: gap ≥ space(2V_d/N).
  - Interdigitated pattern (adjacent columns belong to different devices): gap ≥ space(|V_nodeA − V_nodeB|max) from op-point node voltages.
  - Also apply between the resistor and neighboring cells.
- Beats hand layout because voltage-aware spacing is computed per column pair instead of a worst-case global spacing (which wastes area) or none (which risks reliability).
- Philis status: **missing**. The segment gap is the fixed `res_seg_gap` 400 nm (`resistor.rs:257-260`, `pdks/sky130.json:50`).

### H06-23 Metal sense resistors and Kelvin connections
- Kind: rule, cells, dr
- Statement:
  - Metal resistors cover 50 mΩ–50 Ω for sensing and ballast.
  - When paralleling metal layers, use enough vias that via R is a small fraction of metal R.
  - Kelvin connections: force and sense pairs, with the sense taps defining L_d at their inner edges. Keep force leads straight and at constant width for ≥ 2 × the resistor width beyond the resistor, or use a field solver.
  - A two-level layout with sense taps via an upper metal into the resistor center is less sensitive.
  - Metal TCR ≈ +3300 ppm/°C (VPTAT-compatible). R_s varies ±20% and is often unmonitored.
- Source: §6.5.6 Fig. 6.18; L17577–17651; PDF pp. 300–302.
- Philis stage: annotator (sense-resistor pattern), cells (metal resistor generator), dr (sense branches), verify.
- Automation recipe:
  - Recognize a 4-terminal or sense resistor, or a small R in a current-sense loop.
  - Generate a metal strip with sense taps.
  - In dr, route the sense nets as separate branches from the tap (no shared segments with force current), and assert I_sense·R_lead ≤ ε.
- Beats hand layout because the "no shared force/sense segment" rule is checked topologically.
- Philis status: **missing** (LAYOUT-FUNDAMENTALS #55; no metal resistor generator).

### H06-24 Head construction: implant/silicide-block overlaps and length bias
- Kind: rule
- Statement:
  - The head implant must overlap the poly enough to cover the full resistor width (L17729–17732).
  - L_b models head-dopant intrusion and matters for short resistors.
  - A gate-doping block needs a large overlap because fast diffusion in poly gives an L_b of several µm (L17832–17837).
  - A silicide-blocked body keeps silicided heads (Eq. 6.23).
  - Poly-resistor heads have no width step, so there is no lateral current-spreading error (L17742–17744).
- Source: §6.5.7 Figs. 6.19–6.20; L17729–17837; PDF pp. 303–304.
- Philis stage: cells, deck.
- Automation recipe: derive every overlap from the deck (`enclosure`/`extension`) and include L_b in H06-01. Keep heads the same width as the body (a strip, not a dogbone) wherever the contact fits.
- Beats hand layout because it is deck-exact.
- Philis status: **implemented (geometry)**. Implant, rpm/npc and salicide block are drawn from deck enclosures (`resistor.rs:194-230`), and heads are full body width (`resistor.rs:131`). L_b is not in any value model.

### H06-25 Well resistor construction
- Kind: rule
- Statement: N-well resistors must be ≥ 2–3 × x_j wide, because below 2·x_j the width bias is width-dependent. Unpinched wells need field plates. PMoat pinching raises R_s (non-retrograde wells) and self-shields, but is severely nonlinear. Parallel narrow strips raise R_s by dilution with less variability.
- Source: §6.5.9 Fig. 6.21; L17917–17959; PDF pp. 305–306.
- Philis stage: cells, deck.
- Automation recipe: a well-resistor variant family (plain, field-plated, pinched) with W ≥ max(2.5·x_j, rule) and a body tap (H06-19).
- Beats hand layout because it is deck-driven.
- Philis status: **missing**.

### H06-26 Tweakable-resistor provisions
- Kind: algorithm (optional cell variant)
- Statement:
  - Sliding contact: extended body, contact initially mid-travel, elongated metal plate. It only works if the head material equals the body material.
  - Sliding head: for high-sheet poly, HSR and silicided heads.
  - Trombone slide: movable serpentine turns, with every enclosing well, implant or silicide-block geometry covering the slide region.
  - Metal options: spare contacted segments, combinable in series or parallel through one mask.
- Source: §6.6.1 Figs. 6.23–6.25; L18218–18348; PDF pp. 310–312.
- Philis stage: cells.
- Automation recipe: a flag on a resistor (`tweak_range_pct`) reserves slide area inside the enclosing layers and generates spare metal-option segments sized to the range. Record them in the macro metadata for later single-mask edits.
- Beats hand layout because the reserved area is computed from the requested range.
- Philis status: **missing**.

### H06-27 Trim-network synthesis
- Kind: algorithm
- Statement:
  - Binary weights: series R_lsb·2^k trims voltage and parallel R_lsb/2^k trims current.
  - Bits N = ceil[3.32·log₁₀(X/δX)] (Eq. 6.25). Example: ±0.25% over ±5% needs 4 bits, so a 100 kΩ nominal uses 95 kΩ + R_lsb·(1+2+4+8) with R_lsb = 667 Ω.
  - Above ≈6 bits, matching errors exceed 1 LSB, so add a secondary network or use sub-2:1 steps.
  - Nonbinary weights (1:2:4:6:12:24:36:72:144…) stay binary-searchable if each weight is ≤ 2× the previous.
  - Differential LSB: ΔR = R_B²/(R_A + R_B) (Eq. 6.26).
  - Trimmed accuracy is ≈ ±0.05% (thin film ±0.01%).
- Source: §6.6.2 Eqs. 6.25–6.26; L18530–18605, L18659–18665; PDF pp. 315–317.
- Philis stage: annotator (recognize trim ladders), cells.
- Automation recipe:
  - Given a nominal, range and resolution, emit the main resistor plus weighted segments, all built from the unit segment of H06-06 (weights as series or parallel unit counts).
  - Common-centroid the ladder with the main resistor.
  - Output the trim table.
- Beats hand layout because weights are realized from identical units, so the ladder inherits matching.
- Philis status: **missing**.

### H06-28 Trim/fuse physical rules
- Kind: rule
- Statement:
  - Leads to fuses must be ≥ 5 × the fuse-link width, so fuses sit near their trimpads (L18461–18463).
  - Share trimpads through series or parallel fuse arrangements (Fig. 6.27).
  - Put the fuse on the least-vulnerable end of the resistor (the grounded end) (L18516–18522).
  - Remote trim switches: MOS widths scale inversely with segment R (4W/L, 2W/L, W/L for R_lsb, 2R_lsb, 4R_lsb) for equal % error; control lines at minimum width; trim logic on a clean local supply (L18606–18627).
  - NVM trim: a non-inverting MSB buffer makes the untrimmed value mid-range (L18827–18832).
  - Poly fuse: lowest-sheet poly with enough head contacts. Programming is 5–15 V, 1 ms, 50–150 mA, < 25 ns rise. Omit the overcoat opening if possible (L18493–18505).
- Source: §6.6.2 Figs. 6.26–6.29, 6.32; L18359–18642, L18810–18832; PDF pp. 312–319.
- Philis stage: cells, gp (fuse-to-pad proximity), dr (lead widths), annotator.
- Automation recipe: gp adds an attraction between each fuse and its trimpad. dr sizes the fuse leads at ≥ 5·W_link. Switch W is derived from the segment R: W_k ∝ 1/R_k with R_on,k/R_k ≤ ε.
- Beats hand layout because the constraints are derived from values rather than eyeballed.
- Philis status: **missing** (no fuse or trim constructs; "fuse" grep hits are unrelated).

### H06-29 Laser link geometry
- Kind: rule, deck-requirement
- Statement:
  - Laser-ablated links are ≈1 µm × 15 µm with 10–15 µm clearance from other circuitry.
  - Links that are too narrow will not rupture the overcoat; links that are too wide splatter.
  - Notched-bar trim: cut laterally to ≈90% of the target, then longitudinally for fine resolution. Discrete (loop or ladder) trims avoid the TC shift of continuous trimming.
- Source: §6.6.2 Laser Trim, Fig. 6.33; L18898–18977; PDF pp. 321–322.
- Philis stage: cells, deck.
- Automation recipe: if a deck declares a laser-link layer, place links with the declared clearance keep-out.
- Beats hand layout: deck-exact clearance.
- Philis status: **missing** (none of the Philis PDKs declare it; low priority).

### H06-30 Parallel-plate plus fringe model for unit capacitors
- Kind: formula
- Statement:
  - C = εA/t (Eq. 7.2).
  - Fringe: C_F ≈ (εP/π)[ln(2eP/t) + ½·ln(1 + 4t_e/t)] (Eq. 7.6), with P the perimeter, t_e the electrode thickness and t the dielectric thickness. For one finite plate over a larger one, use t/2.
  - Check: 5 µm square over 200 Å gives 43.1 fF + 2.6 fF, which reproduces with this formula. Fringe matters only for small caps.
- Source: §7.1 Eqs. 7.2, 7.6; L19156–19172, L19345–19386; PDF pp. 327, 330.
- Philis stage: cells (unit-cap sizing), verify.
- Automation recipe:
  - Model C_unit = c_a·A + c_p·P, with c_p from Eq. 7.6 or a deck `perimeter_cap_af_um`.
  - For ratioed caps built from units, keep the P/A ratio identical, i.e., only whole units.
  - If a fractional unit is unavoidable, solve its W,L so that (c_a·A + c_p·P)/C_unit equals the fraction exactly.
- Beats hand layout because fractional caps are value-exact including fringe.
- Philis status: **partial**. `cap_array.rs` uses whole unit plates, so the P/A ratio is preserved (`cap_array.rs:439-443`, `:518`). `capacitor.rs` merges a device's units into one plate (`capacitor.rs:34-37`, `:75-78`), which changes P/A between devices of different unit counts. No generator computes C.

### H06-31 Wire-over-plane capacitance with fringe (PEX formula)
- Kind: formula, verify
- Statement:
  - C/L = ε[W/h + 0.77 + 1.06(W/h)^0.25 + 1.06(t/h)^0.5] (Eq. 7.7).
  - Accuracy: ±2% for W/h > 1 and 0.1 ≤ t/h < 4; ±6% for W/h ≥ 0.3 and 4 ≤ t/h ≤ 10.
  - The book says fringe is ≈60% of a 1 µm × 5 kÅ line over 8 kÅ (a direct evaluation gives ≈69%, so check it).
  - Table-based back-annotation misses distant-line coupling and IR in power grids.
- Source: §7.1 Fringing, Eq. 7.7; L19389–19428; PDF pp. 330–331.
- Philis stage: verify, gr (cost estimates).
- Automation recipe: use Eq. 7.7 as the in-loop wire-to-ground estimate (layer thickness t, height h, width W from the deck `pex` stack) instead of a constant per-µm fringe. Cross-check with the signoff PEX.
- Beats hand layout because the in-loop estimate tracks width changes (non-minimum-width shields, wide supply lines).
- Philis status: **partial**. `backend/verify/src/pdk.rs:696-703` uses `area·W + 2·fringe` per µm, which is linear in W with no W/h or t/h dependence.

### H06-32 Dielectric voltage limit and composite permittivity
- Kind: check, formula
- Statement: V_max = t_min·E_max (Eq. 7.4); 200 Å at 5 MV/cm gives 10 V. For composites, ε_r = (t₁ + t₂)/(t₁/ε_r1 + t₂/ε_r2) (Eq. 7.5). Table 7.1 lists dielectric strengths: SiO₂ dry 11 MV/cm, PECVD 3–6, TEOS 10; nitride LPCVD 10, PECVD 5.
- Source: §7.1 Eqs. 7.4–7.5, Table 7.1; L19212–19303; PDF pp. 328–329.
- Philis stage: deck, verify.
- Automation recipe: the deck carries `v_max` per capacitor recipe (or t and E_max). Verify checks |V_P − V_N|max ≤ V_max using op-point node voltages (DC plus the designer-supplied swing). The check is hard.
- Beats hand layout because it runs for every capacitor.
- Philis status: **missing**.

### H06-33 TDDB derating by area, duty and field
- Kind: check, metric
- Statement:
  - AHI: F₂ = F₁(A₂/A₁)(d₂/d₁)^β·exp[βG(1/E₂ − 1/E₁)] (Eq. 7.8, as printed).
  - **Caution:** with G > 0, the printed form makes the failure rate fall as the field rises. The 1/E model, t_f ∝ exp(G/E), implies exp[βG(1/E₁ − 1/E₂)]. Implement the physically monotone form and confirm the sign of G against the fab's fit.
  - McPherson: F₂ = F₁(A₂/A₁)(d₂/d₁)^β·exp[βγ(E₂ − E₁)] (Eq. 7.9).
  - Budget ≈10 FIT for all TDDB.
  - Example: 120 Å oxide, 1 FIT/mm² at 4.5 MV/cm, β = 2.0, γ = 3.5 cm/MV. 1000 µm² at 6 MV/cm and 10% duty gives 0.36 FIT, which reproduces.
  - This allows small analog areas to run above "one size fits all" voltage limits.
- Source: §7.1 Derating; L19432–19502; PDF pp. 331–332.
- Philis stage: deck, verify.
- Automation recipe: the deck holds (F₁, A₁, E₁, β, γ or G) per dielectric (gate oxide, MIM, ILO). Sum the FIT over all capacitors and gates using area, field (V/t) and duty (from the op-point or a user annotation). Report the total against the budget as a metric. Optionally allow a per-instance override of V_max when the FIT budget holds.
- Beats hand layout because the quantitative area×field×duty budget replaces conservative blanket limits.
- Philis status: **missing**.

### H06-34 Junction capacitance area and perimeter; comb vs plate
- Kind: formula, algorithm
- Statement:
  - A_total ≅ A_d + (π/2)·x_j·P_d (Eq. 7.16), or empirically c_j0 = c_a·A_d + c_p·P_d (Eq. 7.17), for example c_a = 0.82 fF/µm² and c_p = 2.8 fF/µm.
  - A comb beats a plate if the finger spacing S_f < 2c_p/c_a (Eq. 7.18).
  - The voltage law is c_j = c_j0(1 − V_F/φ_B)^(−m), with m 0.33–0.5 (Eq. 7.19).
- Source: §7.1 Junction Capacitance, §7.1.1; L19607–19694, L19752–19764; PDF pp. 334–336.
- Philis stage: verify (PEX of S/D and wells), cells (MOS S/D sharing decisions).
- Automation recipe: use c_a·A + c_p·P for diffusion parasitics when comparing MOS fold variants. More fingers means less drain area but more perimeter, and the choice is decided by the deck's c_a/c_p.
- Beats hand layout because the fold count is chosen from computed junction C, not a rule of thumb.
- Philis status: **unknown or partial** (junction extraction is done by the signoff extractor; no in-loop area/perimeter model was found by grep).

### H06-35 Junction-capacitor operating limits
- Kind: rule
- Statement: operate ≤ 75% of BV (6.8 V gives 5 V). Forward bias is ≤ 400 mV at room temperature at −2 mV/°C, which is only 200 mV at 125 °C. C falls to 75% of zero-bias at −1 V and 50% at −5 V. The tank contact is required, and NBL is useless.
- Source: §7.1.3 Junction Capacitors; L20092–20170; PDF pp. 341–342.
- Philis stage: verify.
- Automation recipe: ERC on any junction used as a capacitor or varactor, checking V_R ≤ 0.75·BV and V_F ≤ V_F,max(T_max).
- Beats hand layout because the check is exhaustive.
- Philis status: **missing** (no junction-cap or varactor generator; the sky130 varactor is explicitly out of scope, `pdks/sky130.json` `inapplicable_rules_note`).

### H06-36 Poly-depletion VCR and antiparallel cancellation
- Kind: formula, algorithm
- Statement:
  - w_d = εV/(qN_P·t) (Eq. 7.20) and c = εA/(t + w_d·ε/ε_Si) (Eq. 7.21). Example: 5e19 cm⁻³, 200 Å, 4.5 MV/cm gives 19 Å and 1.73 → 1.67 fF/µm².
  - The heaviest-doped implant makes the best top plate (L20445–20446).
  - An unsilicided poly-poly cap has VCR ≈ 150 ppm/V.
  - Splitting an asymmetric-electrode cap (silicided lower plate) into two equal halves in parallel with opposite orientation gives ≈2 ppm/V (L20564–20568).
- Source: §7.1.1 Eqs. 7.20–7.21; §7.1.3 Poly-Poly; L19796–19831, L20437–20568; PDF pp. 336–337, 346–348.
- Philis stage: cells, annotator.
- Automation recipe: when a capacitor recipe has asymmetric electrodes (poly/silicide, MOS) and the net's classification demands linearity (SC/ADC signal path), emit it as two half-caps with swapped plate assignment, placed common-centroid. This is a cell variant.
- Beats hand layout because it is applied systematically to every linearity-critical asymmetric cap.
- Philis status: **missing** (only symmetric-metal MOM/plate caps are generated: `cellgen.rs:803-805`, `capacitor.rs:11-32`).

### H06-37 MOS-capacitor bias legality
- Kind: check
- Statement:
  - Keep MOS caps deep in accumulation or inversion with ≥ 0.5 V overdrive beyond V_FB or V_t (Table 7.2). Even then C may be 10% short, so use ≥ 1 V to minimize VCR (L20185–20188).
  - Physics: P-backgate accumulation V_GB < V_FB − 0.5 V; inversion V_GS > V_t + 0.5 V; N-backgate mirrored.
  - Table 7.2's backgate-polarity column is inconsistent with the text at L19900–19902 (see §2), so implement from the physics.
  - V_FB is best taken from a measured C–V curve. Eq. 7.24 gives, for example, V_FB ≈ V_t − 1.8 V for 1e17 backgate and 200 Å (L19907–19914).
  - In inversion, S/D must be present and tied to the backgate, or the inversion layer forms slowly and C stays at c_min (L19889–19896).
- Source: §7.1.1 Voltage Modulation (Eqs. 7.22–7.24), §7.1.3 MOS Capacitors (Table 7.2); L19835–19919, L20174–20222; PDF pp. 337–338, 342–343.
- Philis stage: verify, annotator.
- Automation recipe: for each MOS used as a capacitor (the annotator recognizes D = S = B tied, or a cap-model MOS), take the op-point V_GB/V_GS over the operating range and require the overdrive margin. Report C_eff/C_ox from a C–V table in the deck. The check is hard at 0.5 V and a warning below 1 V.
- Beats hand layout because the bias region of every MOS cap is checked at the actual operating points.
- Philis status: **missing**. All capacitors are drawn as MOM plates or combs (`frontend/library/src/cellgen.rs:803-805`). The comment at `cellgen.rs:844-848` notes that gf180's recognized capacitor is a MOS cap and ihp's a MIM, and neither is generated.

### H06-38 MOS-capacitor type selection from terminal connectivity
- Kind: algorithm
- Statement (P-substrate):
  - One terminal at substrate: NMOS inversion or PMOS accumulation (silicon electrode on ground).
  - One terminal at the positive supply: PMOS inversion (source and backgate to supply).
  - Both floating, minimize parasitics on the lower-voltage electrode: PMOS inversion with gate low and source high, backgate to source or to V⁺. V⁺ also isolates from substrate noise and injection.
  - Both floating, minimize parasitics on the higher-voltage electrode: PMOS accumulation, or isolated NMOS inversion if substrate noise matters.
  - Bias polarity reverses: two antiparallel PMOS inversion caps of C/2 each, backgates to their sources; the dips are offset by 2V_t.
- Source: §7.1.3 MOS Capacitors; L20224–20259; PDF p. 343.
- Philis stage: annotator, cells.
- Automation recipe: inputs are each cap terminal's net role (supply, ground, signal) from `netrole.rs` and the op-point voltage range. The decision table above picks the device type and orientation. The output is a cell variant plus the pin-to-net mapping.
- Beats hand layout because the whole decision table is applied consistently, including the rarely remembered antiparallel case.
- Philis status: **missing**.

### H06-39 MOS-capacitor layout: gate and channel resistance balance
- Kind: algorithm, rule
- Statement:
  - Distributed gate R lumps to 1/3 of the end-to-end R with one contacted end, and to 1/4 of that with both ends contacted (L20270–20275).
  - Channel R_ch ≈ L/(W·k′·(V_GS − V_t)) (Eq. 7.29). If the gate R dominates, reduce W, increase L or split sections (L20285–20299).
  - With poly contacts over the gate: a sparse array at several × minimum pitch (antenna charge during contact etch, and verification load), a diffusion ring on all four sides, an elongated shape, splitting when "spindly" (L20303–20329).
  - Without them: a transistor-like layout with S/D on all four sides and backgate tied to source (L20262–20266).
- Source: §7.1.3 MOS Capacitors, Fig. 7.12; L20262–20338; PDF pp. 343–344.
- Philis stage: cells.
- Automation recipe: enumerate (W, L, sections, contacts on one or both ends). Compute R_g,eff = R_□·(W/L)/3 (or /12) and R_ch from the op-point k′ and V_ov. Pick the variant minimizing area subject to ESR = R_g,eff + R_ch ≤ the budget from the cap's role (e.g., compensation zero) and the antenna ratio ≤ the deck limit.
- Beats hand layout because the ESR is computed per variant.
- Philis status: **missing** (no MOS-cap cell). The MOSFET generator's both-ends gate strapping (`kernel/cells/src/mosfet.rs:44`) could be reused.

### H06-40 Capacitor plate orientation by net impedance
- Kind: algorithm, rule
- Statement:
  - The two electrodes of MOS, poly-poly, stack and MIM capacitors are never interchangeable. The lower (diffusion, poly, outer-stack) electrode carries a large parasitic to the substrate and must connect to a low-impedance node. The upper or sandwiched electrode has little parasitic (L20427–20433, L20725–20737).
  - In a poly–M1–M2 stack, the sandwiched M1 plate has virtually no parasitic, while the poly/M2 plate couples to substrate and overhead leads (L20725–20731).
  - The schematic symbol's curved plate is the "outside foil" that designers ground as a shield (L19320–19324).
- Source: §7.1.2, §7.1.3 MOS, Stack; L19318–19326, L19980–19984, L20427–20433, L20725–20737; PDF pp. 329, 339, 346, 351.
- Philis stage: annotator, cells.
- Automation recipe:
  - For each cap, rank its two nets by impedance: supply/ground < driven output < high-impedance node (gate, virtual ground, integrator input) from `netrole.rs`.
  - Map the lower-impedance net to the high-parasitic electrode (BOT, or the outer plates of the `VerticalAcrossLayers` sandwich) and the high-impedance net to the shielded electrode (TOP sandwiched).
  - If both nets are high-impedance, choose by the C_parasitic-to-budget ratio of each net.
  - Record the decision so LVS and `perf` see the same mapping.
- Beats hand layout because it applies to every capacitor, and the choice is recorded rather than implied by symbol orientation.
- Philis status: **missing**. `capacitor.rs:37` fixes `P` = top and `N` = bottom by SPICE terminal order. `cap_array.rs` fixes TOP as the common net (`cap_array.rs:12-17`), which is correct for a CDAC but not derived from net impedance.

### H06-41 Capacitor parasitic subcircuit for PEX and perf
- Kind: data-model
- Statement:
  - Deposited-plate cap (Fig. 7.9): C₁ main; C₂ lower plate to substrate; C₃ upper plate to overlying leads; π-split halves (Eqs. 7.26–7.28) with R₁ (upper plate, also dielectric loss, rising with f), R₂ (lower plate) and R₃ (epi).
  - Diffused electrodes become diodes, which can leak or forward-bias, so guard-ring them.
  - MOS cap (Fig. 7.10B): voltage-dependent C₁/C₂, channel/accumulation R₁, gate R₂, backgate diodes and substrate R₃.
- Source: §7.1.2 Figs. 7.9–7.10, Eqs. 7.26–7.28; L19974–20071; PDF pp. 339–341.
- Philis stage: verify, flow (perf).
- Automation recipe: emit, per capacitor instance, C₂ = bottom-plate area·c(bottom → substrate) + perimeter fringe, C₃ from any routed overlap, and ESR from the plate sheet R and geometry (π split) into the `perf` netlist.
- Beats hand layout because each capacitor's parasitic is modeled in its drawn orientation (H06-40).
- Philis status: **partial**. `cap_array.rs:53-72` reports route length and via counts per slot (ARR metrics). No C₂/C₃/ESR subcircuit is emitted, and capacitors are an LVS skip (`cellgen.rs:844-848`, `:870`).

### H06-42 Capacitor TC/VCR data for deck and selection
- Kind: data-model, deck-requirement
- Statement:
  - TC: thin oxide +20 ppm/°C; nitride +12 ppm/°C; poly-poly < 50 ppm/°C (L19968–19971), or < 250 ppm/°C (L20546–20548; the book gives both); junction ≈ +600 ppm/°C (example) and +500…+700.
  - VCR: poly-poly ≈ 150 ppm/V; silicide–oxide–metal 2–4 ppm/V; 600 Å nitride MIM −13 ppm/V and −4 ppm/V² at 39 ppm/°C; N+ sinker MOS ≤ 5% over voltage.
  - Table 7.3 gives the density/tolerance/voltage of common types.
- Source: §7.1.1, §7.1.3, Table 7.3; L19929–19971, L20413–20418, L20546–20548, L20614–20633, L20924–20976; PDF pp. 338–339, 345, 348–349, 354.
- Philis stage: deck, annotator (recipe choice), verify.
- Automation recipe: per-capacitor-recipe deck keys `c_area_af_um2`, `c_perim_af_um`, `tc1`, `vc1`, `vc2`, `v_max` and `tol_pct`. Choose the recipe by the net's needs (H06-43/44) and report the linearity error from V swing × VCR.
- Beats hand layout because recipe trade-offs are quantified.
- Philis status: **partial**. `cap_density_ff_um2` (`pdks/sky130.json:8`) is read only by benchmarks (`benchmarks/src/fixtures.rs:556`). There are no TC or VCR keys.

### H06-43 Capacitor style selection by density (stack and lateral flux)
- Kind: algorithm
- Statement:
  - Stack caps using ILO (5–10 kÅ) need ≈30,000 µm² per pF. A poly–M1–M2 stack roughly doubles the density.
  - Horizontal-bar lateral flux (alternating A/B strips on every layer) is competitive when the lateral spacing < ILO thickness.
  - Vertical-bar pillars (2D lateral flux, no vertical) win when the spacing ≪ ILO.
  - A woven structure (orthogonal layers with vias at same-electrode intersections) matches horizontal bars in C with lower series L.
  - Fractal geometries trade series R against density.
- Source: §7.1.3 Stack, Lateral Flux, Figs. 7.17–7.19; L20689–20806; PDF pp. 350–352.
- Philis stage: cells.
- Automation recipe: for each deck, compute the density of each style from the stack (thickness, spacing, height, k): C/A = Σ vertical + Σ lateral sidewall terms. Enumerate only styles within x% of the best density, plus the style with the lowest ESR/ESL for RF nets. Add multi-layer alternating bars and vertical-pillar variants.
- Beats hand layout because the style is chosen per deck from computed density, not habit.
- Philis status: **partial**. `capacitor.rs:11-32` offers a single-layer comb (`VerticalInOneLayer`), 2-metal plates and a 3-metal sandwich, and enumerates every kind the stack allows (`capacitor.rs:48-67`, `:299-314`). There is no multi-layer alternating-bar or pillar MOM and no density-based pruning.

### H06-44 Dielectric choice for switched-capacitor and ADC paths
- Kind: heuristic
- Statement:
  - ONO stack dielectrics show dielectric absorption (soakage) at ≥ 10 MHz. Where C must be constant over frequency, prefer single-layer oxide or nitride.
  - Low-C-density dielectrics improve small-cap matching because plates are larger.
  - ONO breakdown is asymmetric, up to 50% lower with the lower plate negative, so orient the plates accordingly (L20515–20527).
- Source: §7.1.3 Poly-Poly; L20499–20527; PDF p. 347.
- Philis stage: deck, annotator, cells.
- Automation recipe:
  - Deck tag `dielectric: ono|oxide|nitride`.
  - The annotator marks charge-redistribution or SC capacitors and forbids ONO recipes for them when an alternative exists.
  - For asymmetric-breakdown dielectrics, add a polarity check on the plate orientation (the lower plate must not go negative beyond the reduced limit).
- Beats hand layout: the rule is applied every time.
- Philis status: **missing**.

### H06-45 Poly-poly and MIM construction rules
- Kind: rule, deck-requirement
- Statement:
  - Oxide steps should not intersect a poly-poly capacitor (L20556).
  - Poly-2 stringers along poly-1 edges can short or charge nearby MOS gates. Newer flows avoid this by patterning poly-2 first (L20482–20490, L20531–20538).
  - An ARC/TiN MIM can be inserted between any two metals and placed over circuitry wired in lower layers (L20659–20661).
  - The stack-cap ring uses staggered contacts/vias for Al and stacked vias for W plugs (L20718–20721).
- Source: §7.1.3 Poly-Poly, Metal-Metal, Stack; L20437–20737; PDF pp. 346–351.
- Philis stage: cells, gp (allow MIM over active cells), deck.
- Automation recipe: the deck declares a MIM recipe with `over_circuit_allowed`. When true, gp may overlap the MIM macro footprint with cells that use only layers below the MIM bottom plate. The overlap is still priced for coupling to sensitive nets beneath.
- Beats hand layout because the area saving is applied wherever legal.
- Philis status: **missing** (no MIM generator; `capm` appears only as a DRC-layer comment at `backend/verify/src/pdk.rs:260`).

### H06-46 Absolute cap tolerance forces ratio-only designs
- Kind: heuristic
- Statement: capacitors are ±10% (thin film) to ±30% (junction) in absolute value and hard to trim, since trim circuitry adds R and C. Circuits trim resistors or currents instead, or depend only on capacitor ratios. Laser link trim of caps is possible.
- Source: §7.3; L21591–21595; PDF p. 364.
- Philis stage: annotator.
- Automation recipe: any capacitor pair or set in one functional block (SC ratio, CDAC, compensation vs load) gets a ratio-matching constraint by default. Absolute-value-critical caps (RC timers) get a flag for a trim resistor instead.
- Beats hand layout because the ratio constraints are extracted automatically.
- Philis status: **partial** (CDAC arrays via `cap_array.rs`; a general SC-ratio recognizer is not visible in the annotator catalog).

### H06-47 Planar spiral inductance model (Mohan)
- Kind: formula
- Statement:
  - L = µ·K₁N²(d_o + d_i) / {2[1 + K₂(d_o − d_i)/(d_o + d_i)]} (Eq. 7.34), with square K₁ = 2.34, K₂ = 2.75 and octagonal K₁ = 2.25, K₂ = 3.55. d_i = d_o − 2Np, where p = width + space (Eq. 7.35).
  - **Caution:** the book's example (300 µm square, 10 turns, 9 µm/1 µm, d_i = 100 µm) states 18.6 nH, but the equation evaluates to ≈24.8 nH (µ₀ = 1.2566 µH/m). Validate against a field solver.
  - Circular spirals have the lowest R for a given L and square the highest (L21103–21105).
- Source: §7.2 Eqs. 7.34–7.35; L21101–21156; PDF pp. 356–357.
- Philis stage: cells (synthesis), annotator (L target from netlist).
- Automation recipe: given L_target, f_op and the metal stack, search (shape ∈ {octagon, square}, N, w, s = s_min, d_o) such that L(Eq. 7.34) = L_target, d_i ≥ max(5w, d_o/3) (H06-50) and Q (H06-48) is maximized. Output the geometry plus modeled L and Q.
- Beats hand layout because the geometry is searched rather than iterated by hand.
- Philis status: **missing**. `inductor.rs:23` sets turns = Σnf, and the netlist W/L are taken as trace width and outer diameter (`inductor.rs:30-31`). No L is computed.

### H06-48 Winding loss, skin depth, critical frequency, Q
- Kind: formula, metric
- Statement:
  - DCR = ρL/A (Eq. 7.36).
  - Skin depth δ = √(ρ/(πfµ)) (Eq. 7.37), ≈2.3 µm for thin-film Al at 1 GHz.
  - f_crit = p·R_s/(2µw²) (Eq. 7.38): for p = 10 µm, w = 9 µm, 50 mΩ/□ it gives 2.4 GHz, which reproduces.
  - R_W = R_DC[1 + (1/10)(f/f_crit)²] for d_i ≈ d_o/3 (Eq. 7.39).
  - Q = 2πfL/R_s (Eq. 7.40), peaking between ≈1 and 40.
  - The SRF bounds the usable frequency and is usually > 1 GHz.
- Source: §7.2.1 Eqs. 7.36–7.40; L21208–21404; PDF pp. 358–361.
- Philis stage: cells, verify (metric).
- Automation recipe: in the H06-47 search, compute R_DC from the stack sheet R of the (strapped) layers, R_W(f_op), and a substrate-loss term from a deck constant. Maximize Q at f_op and constrain w so that f_crit ≥ f_op. Report Q and SRF estimates.
- Beats hand layout because every candidate is scored.
- Philis status: **missing**.

### H06-49 Inductor metal-layer assignment
- Kind: rule
- Statement:
  - Put the spiral on the highest metal. If there is no thick top metal, strap the top 2–3 metals, which count as one winding layer.
  - Never use metal-1 in the body.
  - The underpass jumper goes beneath the spiral, never above.
  - Leads are short and on the highest metal.
  - Use dedicated 2–5 µm inductor metals where offered.
- Source: §7.2.1–7.2.2, Guidelines 2, 3, 11; L21320–21324, L21463–21469, L21514–21522, L21565–21567; PDF pp. 360, 362–363.
- Philis stage: cells, dr.
- Automation recipe: layer = top routing metal from the deck's stack (or a declared `inductor_metal`), with a strapped variant using the top-k metals. The underpass is on the next-lower metal. Pins sit on the top metal.
- Beats hand layout: deterministic.
- Philis status: **violated**. `inductor.rs:35`, `:46-57` draw the spiral on `met1`, with the center lead on `li` (`inductor.rs:36`, `:63`) and the `N` pin on `li` (`inductor.rs:65`). This is exactly the layer choice Guidelines 2–3 forbid: lowest metal, nearest the substrate.

### H06-50 Spiral geometry rules
- Kind: rule
- Statement:
  - Narrowest turn spacing (better coupling and Q; interwinding C is minor for single-layer spirals).
  - Width ≈10–15 µm at 1–3 GHz, or from Eq. 7.38.
  - ≤ 1–2 winding layers.
  - Inner diameter ≥ 5 × the metal width, and ≥ d_o/3 for large inductors.
  - Octagonal or circular in preference to square.
  - Symmetric (center-tapped, crossover) spirals for differential or balanced circuits; they also lower loss for single-ended use.
- Source: §7.2 Figs. 7.22–7.23; Guidelines 5–8; L21101–21163, L21528–21546; PDF pp. 356–357, 363.
- Philis stage: cells.
- Automation recipe: hard constraints in the H06-47 search (s = s_min(layer), d_i ≥ max(5w, d_o/3)). Choose the symmetric variant when the annotator marks both terminals as a differential pair (e.g., an LC-tank of a cross-coupled pair).
- Beats hand layout because the constraints are guaranteed for every instance.
- Philis status: **violated or missing**.
  - Spacing = trace width (`inductor.rs:33`), not the layer minimum.
  - Rectangular turns only (the ponytail note at `inductor.rs:39-40`).
  - No inner-diameter check: turns are drawn until `ring_outer <= 2*trace_w` (`inductor.rs:42-45`), which fills the center that Guideline 8 says to leave empty.
  - No symmetric variant.

### H06-51 Inductor keep-out zone
- Kind: rule, cost term
- Statement:
  - Keep unconnected metal ≥ ½ × the inductor width away, more if possible.
  - No circuitry in the hollow center, since it lowers Q.
  - No metal or poly above or below except slotted shields.
  - No junctions beneath (rectification and injection).
  - Remove dummy fill out to ≈½ the spiral width, or size and place it to minimize eddy losses.
- Source: Guidelines 4, 9, 10; L21523–21564; PDF p. 363.
- Philis stage: gp, dp (placement blockage), gr/dr (routing blockage), flow (fill), verify.
- Automation recipe: each inductor macro carries a keep-out polygon = its bounding box grown by 0.5·d_o, applied on all layers for foreign nets and devices, plus its interior. gp/dp treat it as a hard overlap region. gr/dr treat it as blocked. `fill.rs` excludes it. Verify checks that no foreign shape lies inside.
- Beats hand layout because the exclusion is complete across placement, routing and fill.
- Philis status: **missing**. `fill.rs:16-18` keeps fill off matched cells only, and the inductor macro declares no keep-out.

### H06-52 Patterned (slotted) ground shield
- Kind: algorithm (cells)
- Statement: a shield between spiral and substrate raises Q by replacing the lossy substrate path with a low-R one. It must be slotted radially to break eddy currents (Fig. 7.25), with strip widths chosen so that f_crit(strip) > f_op. Silicided poly is the best combination of low shield R and low winding-to-shield C. The shield is the exception to Guideline 9.
- Source: §7.2.2 Fig. 7.25; Guideline 9; L21435–21442, L21547–21553; PDF pp. 361–363.
- Philis stage: cells.
- Automation recipe: optional PGS layer under the spiral footprint. Radial slots from the center (four quadrant comb patterns as in Fig. 7.25), strip width w_s from Eq. 7.38 with the shield layer's R_s, tied at the perimeter to the local ground/substrate tap.
- Beats hand layout because the geometry is derived from equations.
- Philis status: **missing**.

### H06-53 Substrate and lead rules for inductors
- Kind: rule, deck-requirement
- Statement:
  - Substrate resistivity < 10 Ω·cm gives severe eddy loss; prefer > 100 Ω·cm.
  - Use guard rings and scattered substrate contacts against debiasing and latchup on high-ρ substrates.
  - Elevate the spiral (thick metal above the passivation, polyimide).
  - Standard CMOS/BiCMOS on low-ρ substrates rarely reaches Q > 5–10.
- Source: §7.2.1–7.2.2; Guideline 1; L21284–21287, L21432–21438, L21510–21513; PDF pp. 359, 361, 363.
- Philis stage: deck, flow (report).
- Automation recipe: the deck carries `substrate_ohm_cm`. The flow reports an expected Q ceiling and warns when an inductor with a Q requirement targets a low-ρ deck. Substrate-contact density around inductors becomes a placement constraint.
- Beats hand layout because the expectation is set quantitatively before layout.
- Philis status: **missing**.

### H06-54 Loop and bondwire inductance and di/dt bounce
- Kind: check
- Statement: loop L = µr[ln(8r/a) − 1.75] (Eq. 7.31). A 25 µm bondwire in a 1 mm loop is ≈5.6 nH. A gate driver with ≥ 1 A in 10–20 ns (di/dt > 5×10⁷ A/s) through 5 nH drops ≥ 0.25 V, which can push diffusions beyond the rails. Guard-ring thoroughly and minimize loop areas such as the bypass cap → driver → power device → ground loop.
- Source: §7.2 Eqs. 7.31, 7.33; L21019–21079; PDF pp. 355–356.
- Philis stage: gp (loop-area objective), verify.
- Automation recipe: for nets marked high-di/dt (driver outputs, switching nodes), compute the loop area of the current path (supply, device, return) from placement and routing. Estimate L from Eq. 7.31 with r = √(area/π), and check L·di/dt ≤ the margin. Placement cost minimizes the loop area.
- Beats hand layout because the loop area is minimized by the optimizer rather than by eye.
- Philis status: **missing**.

### H06-55 Inductor EM verification and SRF check
- Kind: check, verify
- Statement: parasitic C₁–C₃ and substrate R₁–R₂ (Fig. 7.24) are hard to compute analytically. Designers use 3D or 2.5D field solvers (HFSS, EMPro). Automatic tools (MIDAS) iterate generation and analysis. The SRF must exceed the operating frequency.
- Source: §7.2.1–7.2.2; L21327–21349, L21400–21423; PDF pp. 360–361.
- Philis stage: verify, flow.
- Automation recipe: export each inductor (plus shield and keep-out neighborhood) as a standalone GDS with ports for an external EM solver. Import the S-parameters or a fitted π model (Fig. 7.24) into `perf`. Check SRF ≥ k·f_op.
- Beats hand layout: every inductor is verified in its as-placed environment.
- Philis status: **missing**. Inductors have no LVS recognizer and are skipped (`inductor.rs:1-2`, `cellgen.rs:870`).

### H06-56 Precision passives away from the die edge; remote trim instead of long trim leads
- Kind: heuristic, cost term
- Statement:
  - Precision resistors normally sit some distance inside the die to minimize mechanical stress, while fuse trimpads sit at the periphery where probes can reach them (L18606–18608).
  - Long leads from edge fuses to central resistors waste area and pick up noise. When CMOS is available, put the switch next to the resistor and send only minimum-width, near-zero-current control lines across the die (L18608–18611).
  - The switch on-resistance must be small relative to the segment it shorts, with W scaled inversely to the segment R (L18611–18614).
- Source: §6.6.2 Fuses, Figs. 6.28–6.29; L18606–18627; PDF p. 316.
- Philis stage: gp (placement cost), annotator (tag precision passives), cells, dr.
- Automation recipe:
  - The annotator tags resistors and capacitors in matched or ratio constraints as precision passives.
  - gp adds a cost that pulls precision passives away from the die/block boundary (distance ≥ a deck `stress_edge_margin_um`, which ch. 8 quantifies). A trimpad or fuse cell is pulled toward the boundary instead.
  - Where a switch-based trim exists, dr routes the control nets at minimum width and away from the resistor bodies (H06-14).
- Beats hand layout because the stress-edge distance and trim-lead routing are enforced for every precision device, not only for the ones a reviewer inspects.
- Philis status: **missing**. No placement term references the die or block boundary. In `kernel/analog/src/placement`, "edge" means device-to-device edge gap (`isolation.rs:24`, `dti.rs:43`) or well edge (`environment.rs:10-15`). Stress terms are MOS LOD only (`cc.rs:37-38`).

---

## 4. Top-15 priorities for Philis

1. **Make resistor values exact under segmentation** (H06-02, H06-01, H06-15, H06-04). Today N-segment variants silently add 2(N−1) heads and contacts (`resistor.rs:82`). R_s is already reachable through `Process::sheet_ohm` (`kernel/core/src/process.rs:19`) but is used only by benchmark fixtures (`benchmarks/src/fixtures.rs:743-744`). Add W_b, L_b, the head sheet and ρ_c to the deck, and solve L_seg per variant in `resistor.rs`.
2. **Orient every capacitor by net impedance** (H06-40, H06-41). The high-parasitic plate goes to the low-impedance net and the sandwiched plate to the sensitive net. Today P is always top (`capacitor.rs:37`). This costs nothing and cuts parasitic load on high-impedance nodes.
3. **Rewrite the inductor generator to the guidelines** (H06-49, H06-50, H06-51). Today it is a met1 spiral with an li center lead, spacing = width, a filled center and no keep-out (`inductor.rs:30-65`), which violates Guidelines 2, 3, 6, 8 and 9.
4. **Add the MOS-capacitor generator, type selection and bias-legality check** (H06-37, H06-38, H06-39). gf180's recognized capacitor is a MOS cap, and every C is currently drawn as MOM (`cellgen.rs:803-805`, `:844-848`).
5. **Tolerance-driven resistor width** (H06-07, H06-08). Fill the empty `sheet_tolerance`/`linewidth_control_nm` deck keys (`pdks/sky130.json:74-75`) and choose W per tolerance class with anomaly floors.
6. **Ratioed-resistor unitization into identical sections** (H06-06, feeding H06-27). The annotator only unitizes equal W/L (`constraints.rs:28-62`), so 1:2 dividers get no common unit.
7. **Priced routing keep-out and coupling budget over resistor bodies and capacitor plates** (H06-14, H06-18). Extend the matched-cell keep-out (`gr/lib.rs:749-757`) to all passives by material, with zero-penalty crossing corridors over the heads. Also fix the resistor-terminal-as-gate misclassification (`classify.rs:61-84`). Pair this with the die-edge stress margin for precision passives (H06-56).
8. **Voltage-aware resistor segment spacing** (H06-22). This needs node voltages in `OpPoint`. ΔV = 2V_d/N inside a string, and node-voltage differences for interdigitated columns.
9. **Self-heating, TCR and VCR checks for matched resistors** (H06-12, H06-09, H06-11). These are per-instance, op-point driven, and scale the thermal-gradient budget by the recipe α. Validate the Eq. 6.16 units first (the book's example is off by ×100).
10. **Fringe-aware capacitor and wire models** (H06-30, H06-31). Keep P/A equal for ratioed caps in `capacitor.rs` (today devices are merged into single plates). Use Eq. 7.7 in the in-loop PEX estimate.
11. **Capacitor dielectric voltage and TDDB FIT budget** (H06-32, H06-33). These are cheap hard checks once node voltages exist.
12. **Deck-computed capacitor style selection** (H06-43, H06-42). Compute density per style from the stack, add multi-layer alternating-bar and pillar MOM, and carry TC/VCR per recipe.
13. **Resistor dual-π and capacitor bottom-plate models in perf** (H06-17, H06-41). Performance estimates then see device-body parasitics.
14. **Kelvin sense routing for sense and reference resistors** (H06-23). Sense branches must share no segment with force current.
15. **Inductor synthesis, Q metric and patterned ground shield** (H06-47, H06-48, H06-52, H06-55). Search geometry for L_target at f_op with the Mohan and f_crit models, then verify externally. Clean up the dead deck keys `res_corner_squares`/`res_serpentine_aspect` (H06-05) in the same pass as item 1.
