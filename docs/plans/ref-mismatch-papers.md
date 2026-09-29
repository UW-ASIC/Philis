# Mismatch papers: Pelgrom 1988, Drennan & McAndrew 2003, Schaper 2001 (long distance), Schaper & Linnenbank 2003 (distance vs pair)

Sources and reftext files (all read in full; the reftext directory is `…/scratchpad/reftext/`):

| Key | Source | reftext file | Lines read | PDF pages |
|---|---|---|---|---|
| PEL | M. J. M. Pelgrom, A. C. J. Duinmaijer, "Matching properties of MOS transistors", ESSCIRC'88, pp. 327–330 | `pelgrom.txt` | 1–222 (all) | p1 L1–53, p2 L54–117, p3 L118–171, p4 L172–222 |
| DRE | P. G. Drennan, C. C. McAndrew, "Understanding MOSFET Mismatch for Analog Design", IEEE JSSC 38(3), Mar. 2003, pp. 450–456 | `drennan_mismatch.txt` | 1–409 (all) | p1 L1–69, p2 L70–139, p3 L140–207, p4 L208–237, p5 L238–281, p6 L282–359, p7 L360–409 |
| LDM | U. Schaper, C. G. Linnenbank, R. Thewes, "Precise Characterization of Long-Distance Mismatch of CMOS Devices", IEEE TSM 14(4), Nov. 2001, pp. 311–317 | `long_distance_mismatch.txt` | 1–375 (all) | p1 L1–67, p2 L68–99, p3 L100–153, p4 L154–205, p5 L206–261, p6 L262–321, p7 L322–375 |
| DVP | U. Schaper, C. Linnenbank, "Comparison of Distance Mismatch and Pair Matching of CMOS Devices" (Infineon, 130 nm), IEEE, pp. 703–705 | `distance_vs_pair_mismatch.txt` | 1–230 (all) | p1 L1–69, p2 L70–141, p3 L142–229, p4 L230 (blank) |

The text layer of all four papers garbles every equation and most tables. Every equation and table below was checked against the PDF page image: PEL PDF pp. 1–4, DRE PDF pp. 1–6, LDM PDF pp. 3–6, DVP PDF pp. 1–3. Reftext line numbers point to where the garbled equation or table sits in the text file.

Conventions used in this document:
- `A` is a **pair** constant: it gives σ of the *difference* of two equal devices. All four papers use this convention (PEL eq.(1) L37–43, LDM eq.(1) L148–151, DVP eq.(1) L85–101). A single device's σ is σ_pair/√2 (LDM eq.(4), L144–148).
- Numbers marked **[derived]** are my arithmetic on source data. They are not stated in the source.

---

## 1. Coverage

Read chunks (Read tool, consecutive, each file in one call since all are < 2000 lines):
- `pelgrom.txt` offset 1..222 (EOF).
- `drennan_mismatch.txt` offset 1..409 (EOF).
- `long_distance_mismatch.txt` offset 1..375 (EOF).
- `distance_vs_pair_mismatch.txt` offset 1..230 (EOF).
- PDF page images read for garbled equations and tables: PEL pp. 1–4; DRE pp. 1–6 (p. 7 is references and biographies, which the text layer covers); LDM pp. 3–6; DVP pp. 1–4.

Section and subsection headings in the range:
- **PEL**: Title/Abstract (L1–8); Introduction (L9–17); Analysis (L18–43), including eq.(1); Matching of MOS transistors (L46–124), with eqs.(2)–(3) and Table 1 (L102–108); Matching in a band-gap circuit (L126–145); Conclusions (L146–153); Acknowledgement (L155–157); References (L159–165); Figures 1–4 (L172–216).
- **DRE**: Abstract and Index Terms (L12–25); I. Introduction (L28–42, continued in the right column L12–41); II. Mismatch Model (L43–226), with eqs.(1)–(11), Table I (L74–82) and Figs. 1–5; III. Mismatch Application (L228–236); III-A Geometry and Bias Interrelationship (L256–275, Figs. 6–8 at L243–271); III-B NMOS or PMOS? (L276–334, Table II at L298–301, Fig. 9 at L295–296); III-C Multiple-Unit Devices (L336–358, Table III at L286–308); IV. Conclusion (L310–333); References (L335–403); Biographies (L372–404).
- **LDM**: Abstract and Index Terms (L11–26); I. Introduction (L28–53); II. Test Structure and Characterization Principle (L55–89, continued L105–152), with Figs. 1–4; III. Results (L92–261), with eqs.(1)–(4) (L129–151), Table I (L116–124), Figs. 5–11 and Table II (L266–270); IV. Summary (L288–308); References (L311–353); Biographies (L326–370).
- **DVP**: Abstract (L6–18); Introduction (L19–44); Test Macro (L46–74), with Figs. 1–3; Analysis (L76–101), with Figs. 4–5 and eq.(1); Result (L103–198), with Figs. 6–8 and Table 1 (L172–198); Conclusion (L200–213); References (L215–223).

---

## 2. Section-by-section digest

### PEL — Title/Abstract (L1–8), Introduction (L9–17)
- The paper analyses and measures the matching of threshold voltage, substrate factor K and current factor β as functions of **area, distance and orientation**. It verifies the results on a band-gap reference (L5–8, L14–17).
- Matching limits multiplexed analog systems, DACs, reference sources, memory read/write circuits and SRAM margins (L10–14).

### PEL — Analysis (L18–43)
- Assumptions for the area law: the mismatch is composed of many singular events; each event's effect is small and additive; the events' correlation distance is much smaller than the device (L22–28).
- Under these assumptions P is normally distributed with σ²(P) ∝ 1/WL (L29–30). This holds for V_t, K, μ_n, μ_p and the fixed gate charge Q_fc (L31–33).
- A short correlation distance means no dependence on the distance D. Wafer maps nevertheless show a **cone-shaped** parameter distribution from oxidation. That process is deterministic, but because a transistor's position on the wafer is unknown it is modelled as an *additional stochastic process with a long correlation distance* (L33–38).
- Eq.(1): σ²(P) = A_P²/(WL) + S_P²·D². Here A_P is the area proportionality constant and S_P the variation with spacing (L37–43, PDF p.1).

### PEL — Matching of MOS transistors (L46–124)
- Eq.(2): I_D = (β/2)·(V_GS − V_t)²/(1 + θ(V_GS − V_t)), with β = C_ox·μ·W/L and V_t = V_t0 + K(√(V_SB + 2φ_F) − √(2φ_F)). θ absorbs mobility reduction and source resistance (θ := θ + βR_S), so it was used only to monitor measurement accuracy (L50–61, PDF p.2).
- Eq.(3): σ²(β)/β² = A_W²/(W²L) + A_L²/(WL²) + A_μ²/(μ²WL) + A_Cox²/(C_ox²WL) ≈ A_β²/(WL). The edge-roughness terms scale as σ²(L) ∝ 1/W and σ²(W) ∝ 1/L. The approximation "is less valid if W and L decrease" (L62–79).
- Reticle (mask) W/L errors produce non-zero **mean** mismatch that correlates with the mask set. This is a systematic error, not a random one (L64–67).
- Test chip: 2.5 µm n-well CMOS, 500 Å oxide. Devices: n and p, W/L = 3/3, 5/5, 10/5, 20/20. Positions: a reference device, in-line devices at 30, 250 and 500 µm, one parallel device and one 90°-rotated device at 30 µm, and a 700/10 isolated-drain module. Extraction in the linear region; 6 transistors per die, 130 dies per wafer (L80–87; Fig.1 L172–190).
- Distance matters "only for large area devices with a considerable spacing". Fig.2a gives **S_Vt0 ≈ 4 µV/µm** (L88–90).
- Table 1: A_Vt0 = 30 (n) / 35 (p) mV·µm; A_β = 2 (n) / 3 (p) %·µm; A_K = 16×10⁻³ (n) / 12×10⁻³ (p) V^0.5·µm. The data holds within 5 % inside a batch and within 20 % across batches and factories over 3 years (L93–108, PDF p.2).
- V_t mismatch dominates when V_GS − V_t < 3 V (L97). Rotation affects **only β**. A_β for a rotated pair varies strongly with wafer and batch, and mobility is the only candidate cause (L98–110).
- ΔV_t and Δβ are uncorrelated for close pairs; the correlation is **−0.35 at large spacing**, probably from gate oxide (L111–119). A_Vt0 and A_K are proportional to oxide thickness d_ox, while A_β is constant (L119–121).
- No significant effect from parallel versus in-line placement, 100 °C heating, scratch protection, or threshold-implant dose (L122–124).

### PEL — Band-gap (L126–145)
- 17 samples give σ_bg = 19 mV. The contributions are V_BE process spread 6 mV, ΔV_BE 0.14 mV × R1a/R2 = 11 → 1.5 mV, resistor ratio 0.3 mV and resistor wafer variation 0.7 mV. The op-amp contributes 18 mV at the output, 1.6 mV input-referred (L128–139).
- The mismatch data predicts 1.7 mV input offset for the 8 folded-cascode input transistors: 1.6 mV from V_t and 0.35 mV from β (L139–142).
- Reaching 9 mV output needs 0.6 mV op-amp offset, which costs 9× the gate area, i.e. 0.1 mm² (L143–145).

### PEL — Conclusions (L146–153), Figures (L172–216)
- The variances of V_t, β and K are ∝ 1/area. V_t dominates at normal V_GS. **Spacing can be ignored for areas < 100 µm²**. Thinner oxides reduce mismatch (L147–153).
- Fig.2: σ(V_t) and σ(β) versus D (30–750 µm) for 3/3, 5/5, 10/5 and 20/20. Fig.3: σ versus 1/√(WL) for V_t (V_SB = 0 and −3 V), K and β. Fig.3d shows rotated versus normal placement for β. Fig.4 shows the band-gap and the op-amp (L190–216, PDF p.4).

### DRE — Abstract, I. Introduction (L12–42 and right column)
- V_t mismatch does not follow a simple 1/√area law, especially for **wide/short and narrow/long** devices, which are common in analog circuits. V_t and gain factor are not appropriate parameters for modelling mismatch (L15–20).
- The model is characterized in the same domain (I_d) with the same tools (SPICE, BSIM) that design uses (L21–29).
- The model must be valid in weak and strong inversion, in the linear and saturated regions, and across body bias. Simple I_d-square-law models are not (L43–54).
- Binning the geometry space produces thick reports and discontinuities (L55–62).

### DRE — II. Mismatch Model (L43–226)
- Local variation has two kinds of term. Edge terms: σ_L² ∝ 1/W (eq.1) and σ_W² ∝ 1/L (eq.2). Area terms: σ_p² ∝ 1/(LW) (eq.3) for sheet resistance, channel dopant, mobility and t_ox. The physical causes are poly/metal edge grains, resist edge roughness, dopant clustering, and oxide thickness or permittivity (L44–67, PDF p.1).
- Mismatch (intradie) is local variation. Interdie variation contains both global and local parts, and the local part often dominates interdie variation (L85–105).
- Eq.(4): σ²_Id = σ²_β + 4σ²_Vt/(V_gs − V_t)². This formula double-counts t_ox, which appears in both V_t and β, so it can overestimate σ_Id by up to 2× (L124–135, PDF p.2).
- Table I lists the process parameters (V_fb, μ, N_sub, ΔL, ΔW, V_tl short-channel, V_tw narrow-width, t_ox, ρ_sh) and the electrical parameters (I_d, V_gs, g_m, g_o). V_t mismatch belongs to neither list (L74–84).
- Eq.(5), σ_Vt = A_Vt/√(LW), is "physically incorrect" because V_t depends on L and W through SCE, RSCE, NWE, INWE and halo implants. L_eff/W_eff corrections do not fix it (L83–114). Eq.(6): σ_Vgs = σ_Id/g_m, and **σ_Vt ≠ input offset** because σ_Id and g_m vary with bias (L116–128).
- Test DOE choices near L = W hide the failure of eq.(5), a "self-fulfilling" fit. Large errors then appear for wide/short and narrow/long devices (L129–138).
- POV: Δy = (∂y/∂x)Δx (eq.7); σ_e² = Σ_i (∂e/∂p_i)² σ²_pi (eq.8); I_d = 0.5β(V_gs − V_t)² (eq.9); σ_e² = Σ(∂e/∂p_i)² σ²_pi(geometry) (eq.10). The matrix form eq.(11) takes the squared SPICE sensitivities of I_d to [ΔW, t_ox, V_fb, μ_o, ΔL, V_tl, ρ_sh, N_sub] times the normalized variances [σ̃²_ΔW/L, σ̃²_tox/(LW), σ̃²_Vfb/(LW), σ̃²_μo/(LW), σ̃²_ΔL/W, σ̃²_Vtl/W, σ̃²_ρsh/(LW), σ̃²_Nsub/(LW)] (L140–191, PDF p.3).
- **BPV**: solve eq.(11) for the process-parameter variances by linear regression over hundreds of bias × geometry σ_Id measurements. The parameters are assumed independent, and a correlation means the parameter set is wrong. Example: ρ_sh is observable only for short devices in the linear region at high V_gs (L166–206). BPV is not PCA, which is purely empirical (L219–220).
- Geometric scaling enters both the variances and, through the SPICE model, the sensitivities (L221–226).

### DRE — III. Mismatch Application (L228–236), III-A Geometry and Bias (L256–275, Figs. 6–8)
- The model is applied in SPICE through Monte Carlo or sensitivity analysis (L229–236).
- In a current-biased mirror, raising L lowers the intrinsic mismatch *and* raises V_od, so both factors improve. Raising W lowers the intrinsic mismatch but lowers V_od, so the two offset each other and there can be "little or no improvement" (L257–275, Fig.6 0.13 µm, I_ref = 10 µA). Increasing V_od costs dynamic range (higher V_dsat) (L273–276).
- For graded-channel or halo devices, a small W/L ratio gives a dramatic improvement (Fig.7). At L = 25 µm, wide devices are dominated by the dopant and graded-channel length, and narrowing raises V_od (Fig.8, L256–270).
- "Current mirror mismatch depends strongly on L and not W". Mismatch is not constant when I_ref is not constant, e.g. an active load (Fig.9, L269–275; Fig.9 caption L295–296).

### DRE — III-B NMOS or PMOS? (L276–334), III-C Multiple-Unit Devices (L336–358), IV (L310–333)
- Under voltage bias there is no consistent trend. Under current bias pMOS runs at a higher V_od (lower mobility) and matches better. Table II (2×2 µm², 0.4 µm BiCMOS): mirror at I_ref = 10 µA gives nMOS V_gs 1.09 V → 1.22 %, pMOS 1.70 V → 1.09 %. Voltage-driven at V_gs = 1.70 V gives nMOS 0.589 %, pMOS 1.09 % (L277–334, PDF p.6).
- Comparisons of device type, geometry and bias must be made in the design application. "V_t mismatch" metrics may be meaningless (L323–334).
- Placing n units in parallel multiplies the variance component by n and divides the squared sensitivity by n². Net σ_Id falls by √n. An 80 µm device = 4 × 20 µm, and a single 20 µm device has twice the σ (L336–358, L296).
- Integer ratios: 1:n differs from n:1. Table III (2×2 µm² nMOS, 0.13 µm; I_REF scaled to keep the reference at constant voltage bias) gives SD(Id mismatch) of 3.86 % (1 ref : 1 out), 1.52 % (1:4), 3.05 % (4:1) and 1.41 % (4:4). "The majority of the improvement … is gained by using [multiple units] for the output, not the reference device" (L286–308).
- Conclusion: modelling on physical, uncorrelated process parameters is more accurate and identifies the dominant contributors (L310–333).

### DRE — References and biographies (L335–409)
- The references include [8] Pelgrom 1989 JSSC, [12,13] Shyu (local variation) and [18] the comprehensive model from IEDM 1999 (L335–403). No technical content in the biographies.

### LDM — Abstract, I. Introduction (L11–53)
- A test structure characterizes long-distance mismatch of n/p MOS and poly/diffusion resistors using a four-terminal method with a regulated reference potential. Data from 0.5, 0.35 and 0.25 µm processes shows linear and nonlinear distance dependence (L11–23).
- Long-distance mismatch must be considered for **short-channel transistors and narrow resistors**. Example: a resistor ladder reference in an ADC spanning several mm (L21–46).

### LDM — II. Test Structure and Characterization Principle (L55–89, L105–152)
- A central shift register addresses each DUT with only 3 clocks (reset, select, deselect), independent of stage count (L11–26). The units, with different device types, geometries and orientations, repeat along a stripe (Fig.2: 6.1 mm × 0.4 mm, process B) (L27–46, L80).
- Force I through the DUT, measure V. The reference-node IR drop over the parasitic interconnect R_com is compensated by an op-amp force/sense loop. Sense and measure paths carry negligible current (L47–89, Fig.4 L105–113).

### LDM — III. Results (L92–261), Tables I–II, Figs. 5–11
- Processes: A (0.5 µm, 5.0 V, t_ox 15 nm), B (0.35 µm, 3.3 V, 7.5 nm), C (0.25 µm, 2.5 V, 5.5 nm). Table I pair constants: A_ΔVGS nMOS 19 / 8.75 / 6.7 mV·µm and pMOS 16 / 8.5 / 6.9 mV·µm; A_ΔR/Rmean poly-high 4 / 2.4 / 4.7 (nPoly) %·µm and poly-2 6 / 3.3 / 3.0 (pPoly) %·µm (L93–124, PDF p.3).
- Eq.(1): σ_P(short distance) = A_P/√(WL). Eq.(2): I_DS = ½K(V_GS − V_th)², K = μC_ox·W/L. Eq.(3): σ²_ΔVGS = σ²_ΔVth + ¼(V_GS − V_th)²σ²_ΔK/K at fixed I_DS. At V_GS − V_th ≈ 0.2 V, ΔV_th contributes > 90 % of the right-hand side. Eq.(4): σ_VGS = σ_ΔVGS/√2 (L119–148, PDF p.3).
- Without regulation the measurement is badly wrong. A 90 mΩ/□ interconnect at 320 µA drops **8.3 mV/mm**, which masquerades as a gradient. The regulation error is < 10 µV (L180–204, Fig.5).
- Patterns range from no distance dependence to linear gradients to nonlinear distributions, and they change **wafer to wafer and chip to chip**. They were classified by L for transistors and W for resistors (L181–201).
- Long-L transistors and wide resistors show negligible long-distance mismatch. Short-L transistors and narrow resistors show linear gradients (Figs.6–9, L202–246). The effect grows as L or W shrinks and is described to first order as a gradient (L247–261).
- Table II gives |ΔP/Δx| against the pair σ. 0.35 µm nMOS: L = 1.71 / 0.71 / 0.35 µm → 0.1 / 0.8 / 9.8 mV/mm, pair σ 1.6 / 1.1 / 1.6 mV. 0.35 µm poly: W = 2.2 / 1.1 / 0.55 µm → 0.19 / 0.41 / 1.2 %/mm, pair σ 0.19 / 0.27 / 0.37 %. 0.25 µm pMOS: L = 1.21 / 0.57 / 0.25 µm → 0.51 / 0.73 / 1.3 mV/mm, pair σ 4.2 / 2.1 / 3.0 mV. 0.25 µm poly: W = 3.2 / 0.8 / 0.4 µm → 0.06 / 0.11 / 0.36 %/mm, pair σ 0.46 / 0.65 / 0.91 % (L266–270, PDF p.6).
- Orientation: narrow poly resistors (L/W = 35.2/0.55) placed **parallel to the wafer flat** show no long-distance mismatch, while the same devices on the same chip placed **perpendicular** show dominant long-distance mismatch (Fig.10, L300–313).
- Nonlinear case: fitted by a second-order polynomial over a few mm in all three technologies (Fig.11). "A quadratic behavior over a few mm distance cannot be compensated by means of circuit techniques" (L275–287, L315–320).

### LDM — IV. Summary (L288–308), References and biographies (L311–370)
- Long-distance mismatch can exceed pair mismatch for short-channel transistors and narrow resistors over 1 mm. It must be considered when minimum-size devices must match over mm distances (L291–308).
- No further technical content.

### DVP — Abstract, Introduction (L6–44)
- 130 nm technology. Distance mismatch is described by a **statistical gradient parameter**, which must be considered for short-channel transistors or small-width resistors. The gradient parameter is an increasing function of the critical geometry, e.g. it grows as channel length shrinks (L8–18).
- Circuits that need mm-scale matching: resistor-ladder ADC references, flash-converter input stages, memory sense amplifiers (L29–37).

### DVP — Test Macro (L46–74)
- Equal devices sit at short (µm, Fig.1, pair pitch 7.25 µm) and long (mm, Fig.2, 0.630 mm) spacings, repeated over 10 mm. Force/sense measurement (L48–74).

### DVP — Analysis (L76–101)
- Pair (local) mismatch is statistical, e.g. dopant-number fluctuation. Distance mismatch is **systematic**, from chip or wafer gradients (L78–84).
- Systematic effects show up as **shift or skewness** of the normal probability plot. Independent events give centred symmetric distributions (Fig.4: 6×1 µm fit y = 49.56x − 49.47, R² = 0.982; 6×0.2 µm y = 38.46x − 38.58, R² = 0.961) (L86–111).
- Eq.(1): σ²_ΔT = σ²_ΔP + σ²_D = A²_ΔP/(WL) + S_D²·x². The pair and distance parts are independent (L85–101, PDF p.2).

### DVP — Result (L103–198), Conclusion (L200–213)
- Step 1: extract A_ΔR/R and A_ΔVt from close pairs. Step 2: take ΔP between equal devices at various x across all chips, giving σ_ΔT(x). The minimum σ_ΔT is the pair value, and the upper limit is the process tolerance (L105–135).
- Resistors: > 400 pairs per geometry. Both widths show a distance effect, and **S_D depends only weakly on resistor width** (Fig.6, L142–177).
- Transistors: σ(V_GS) approximates σ(V_T) for V_Geff ≈ 200 mV (L179–191). S_D grows as L shrinks, with S_D ∝ 1/L_eff (Fig.8 plots S_D² against 1/L_eff²) (L213–224, L142–170).
- Table 1, resistors (W, L µm; σ_ΔP %; S_D %/mm): 0.2, 6.0; 1.55; 0.212 | 0.2, 3.0; 2.20; 0.205 | 0.5, 6.0; 0.982; 0.185 | 1.0, 6.0; 0.694; 0.163 | 1.0, 12.0; 0.491; 0.163. Transistors (L, W µm; σ_ΔP mV; S_D mV/mm): 0.12, 1.60; 12.1; 1.63 | 0.24, 1.60; 7.42; 0.780 | 0.24, 12.8; 2.62; 0.941 | 0.48, 1.60; 5.25; 0.582 | 0.48, 3.20; 3.71; 0.644 (L172–198, PDF p.3).
- Conclusion: accuracy limits set by local mismatch must be reconsidered when devices are far apart (L202–213).

---

## 3. Actionable extraction

### MM-01 Pelgrom pair-variance law (area + distance)
- Kind: formula
- Statement: σ²(ΔP) = A_P²/(W·L) + S_P²·D². A_P is the pair area constant (P-unit·µm), W·L the gate area of one device (µm²), D the device spacing (µm) and S_P the spacing coefficient (P-unit/µm). Validity: many small additive events whose correlation distance is much smaller than the device (PEL L22–28). S_P models a deterministic wafer-scale field (oxidation cone) as a random process because the die position is unknown (PEL L33–38). DVP restates it as σ²_ΔT = A²_ΔP/(WL) + S_D²·x², with the two parts independent (DVP L85–101).
- Source: PEL eq.(1), L37–43, PDF p.1; DVP eq.(1), L85–101, PDF p.2; LDM eq.(1), L148–151, PDF p.3.
- Philis stage: annotator, gp, dp, verify.
- Automation recipe: For every matched set, evaluate both terms on the placed geometry. Take W·L from the netlist and D from the placed unit centroids (MM-30). Use the result as a budget term (existing `MatchingPair`/`CentroidGroup` residual) and report σ in mV.
- Beats hand layout because: it is evaluated for every matched set, every epoch, on the actual coordinates. A human applies it only to the pairs they remember.
- Philis status: **implemented (ratio form)**. `Pelgrom` in backend/annotator/src/emit.rs:66-106. The ratio `(S/A)·D·√(WL)` is in kernel/analog/src/placement/matching_pair.rs:74-78, and budget semantics are at matching_pair.rs:49-60.

### MM-02 Pair constant versus single-device and per-unit variance
- Kind: data-model
- Statement: σ_single = σ_pair/√2 (LDM eq.(4)). Per-device parameter variance: Var(P_dev) = (A_pair²/2)/(W·L). For two devices of unequal area a₁, a₂ (µm²): Var(ΔP) = (A_pair²/2)(1/a₁ + 1/a₂). This reduces to A²/(WL) when a₁ = a₂.
- Source: LDM eq.(4), L144–148, PDF p.3; convention in PEL L37–43 and DVP L85–101.
- Philis stage: deck, annotator.
- Automation recipe: Store decks in the pair convention (they already are). Inside the estimator (MM-30), convert to per-device variance before summing unequal sides (ratioed mirrors 1:n, unit caps, resistor ladders).
- Beats hand layout because: ratioed and unequal members get the exact variance, not the equal-pair shortcut.
- Philis status: **partial**. The deck keys are in pair convention: pdks/sky130.json:93-96 (the `avt_*_source` notes say "sigma(dVT) fit") and pdks/ihp_sg13g2.json:9-12 (the note says model card delvto_mm ×√2 = pair). emit.rs:220-222 uses the single largest gate for the whole array, and matching_pair.rs uses `min(gate_a, gate_b)` (emit.rs:159). There is no unequal-area formula.

### MM-03 Validity limits of the area law
- Kind: check
- Statement: 1/WL scaling holds only when the correlation distance of the events is much smaller than the device (PEL L28). The β-law approximation "is less valid if W and L decrease" (PEL L78–79). V_t mismatch departs from 1/√area for wide/short and narrow/long devices (DRE L15–17, L129–138) because V_t depends on L and W through SCE, RSCE, NWE, INWE and halo (DRE L83–114).
- Source: PEL L22–30, L78–79, PDF pp.1–2; DRE eqs.(5), L83–138, PDF p.2.
- Philis stage: deck, annotator.
- Automation recipe: Flag matched devices outside the deck's characterized W/L range, or with W/L > 10 or < 0.1. For them, use a per-geometry A_VT (table lookup) or the per-group Monte Carlo of MM-29 instead of the single deck A_VT.
- Beats hand layout because: the tool knows when its own model is out of range and says so.
- Philis status: **missing**. The deck notes record that A_VT depends on geometry (sky130 pfet point estimates 8.4–12.4 mV·µm, pdks/sky130.json:95-96), but only one scalar is stored per polarity (backend/annotator/src/netrole.rs:91-92).

### MM-04 Current-factor mismatch with edge terms
- Kind: formula
- Statement: σ²(β)/β² = A_W²/(W²L) + A_L²/(WL²) + A_μ²/(μ²WL) + A_Cox²/(C_ox²WL) ≈ A_β²/(WL). The edge terms come from edge roughness: σ²(L) ∝ 1/W and σ²(W) ∝ 1/L.
- Source: PEL eq.(3), L62–79, PDF p.2; DRE eqs.(1)–(3), L44–67, PDF p.1.
- Philis stage: deck, annotator.
- Automation recipe: Default to the area-only A_β. Add the edge terms only when a deck supplies A_W and A_L, which none do today. Use the result in MM-09.
- Beats hand layout because: narrow and short units (fingered layouts) get their extra edge variance counted.
- Philis status: **missing**. There is no A_β deck key (the grep of pdks/*.json finds only `avt_*`, `svt_*`, `vt_tc_*` and `lod_kvth0_*`).

### MM-05 Pelgrom 2.5 µm constants and reproducibility (reference data)
- Kind: deck-requirement
- Statement: 2.5 µm n-well, t_ox 50 nm. A_Vt0 = 30 mV·µm (n) and 35 mV·µm (p). A_β = 2 %·µm (n) and 3 %·µm (p). A_K = 16×10⁻³ (n) and 12×10⁻³ (p) V^0.5·µm. S_Vt0 ≈ 4 µV/µm. Reproducibility: 5 % within a batch, 20 % across batches and factories over 3 years.
- Source: PEL Table 1, L102–108, PDF p.2; S_Vt0 L90; accuracy L94–97.
- Philis stage: deck.
- Automation recipe: Do not use these as defaults for modern decks. They are the provenance for the default *ratios* only: A_β/A_VT ≈ 2 %/30 mV. Use them in tests of the estimator (MM-30), because the band-gap (MM-15) gives a closed-form check.
- Beats hand layout because: a regression test ties the estimator to published numbers.
- Philis status: **partial**. The emit.rs tests use 30/35 mV·µm and S = 4 µV/µm (backend/annotator/src/emit.rs:295).

### MM-06 Distance crossover D* and the "spacing ignorable below 100 µm²" rule
- Kind: metric
- Statement: The distance term equals the area term at D* = A/(S·√(WL)). PEL: "spacing … can be ignored for transistor areas less than 100 µm²" (L150–151); distance matters "only … for large area devices with a considerable spacing" (L89–90). [derived] With PEL constants (30 mV·µm, 4 µV/µm), D* = 7.5 mm/√(WL/µm²): 2.5 mm at 9 µm², 0.75 mm at 100 µm², 0.375 mm at 400 µm². [derived] With the sky130 deck values (A = 9.5 mV·µm, S = 1.63 µV/µm, a proxy), D* = 5.8 mm at 1 µm², 1.84 mm at 10 µm² and 0.58 mm at 100 µm². Under the existing η = 0.3 budget, a pair binds at D = 0.3·D*, e.g. 175 µm at 100 µm².
- Source: PEL L88–90, L150–151, PDF pp.2–3; formula from eq.(1).
- Philis stage: annotator, flow.
- Automation recipe: Compute D* per matched set and export it as the set's distance scale. If a set's bounding box is ≪ η·D*, mark the linear-gradient term "non-binding". Placement then spends its effort on deterministic terms (thermal, LOD, WPE, IR drop) and on second-order moments (MM-26).
- Beats hand layout because: effort goes where the numbers say it matters instead of into ritual interleaving.
- Philis status: **missing** as a reported number. It is implicit in matching_pair.rs:76-78.

### MM-07 V_t dominance region
- Kind: rule
- Statement: V_t mismatch dominates the drain-current mismatch when V_GS − V_t < 3 V (2.5 µm process). At V_GS − V_th ≈ 0.2 V the ΔV_th term is > 90 % of σ²_ΔVGS (LDM, 0.35 µm). σ(V_GS) approximates σ(V_T) around V_Geff ≈ 200 mV (DVP).
- Source: PEL L97; LDM eq.(3) and text L140–144, PDF p.3; DVP L185–191.
- Philis stage: annotator.
- Automation recipe: When there is no A_β, compute σ in the V_T domain only. Label it "valid for V_ov ≲ 0.2–0.3 V" and emit a warning for matched devices whose op-point V_ov (from V_dsat) is larger.
- Beats hand layout because: every pair's bias region is checked against the model's validity.
- Philis status: **partial**. V_dsat is parsed (frontend/library/src/oppoint.rs:186, 205) but not used for mismatch.

### MM-08 σ(V_t) is not the offset; use the electrical parameter at bias
- Kind: rule
- Statement: σ_Vgs = σ_Id/g_m (DRE eq.6). σ_Vt is not the input offset because σ_Id and g_m vary with bias (halo devices especially) while σ_Vt does not. Mismatch comparisons "must be performed in the design application" (DRE L323–334).
- Source: DRE eq.(6), L116–128, PDF p.2; L323–334, PDF p.6.
- Philis stage: annotator, verify.
- Automation recipe: Express each matched set's budget in its electrical domain. A diff pair uses input-referred σ(V_os) in mV. A mirror or load uses σ(ΔI/I) in %. Both come from the op-point g_m and I_D (MM-09).
- Beats hand layout because: budgets are in the units the spec is written in.
- Philis status: **partial**. The budget `offset_sigma_mv` exists (backend/annotator/src/netrole.rs:72-74) but is compared with the V_T-domain σ_rand (emit.rs:83-87) for every block kind, mirrors included.

### MM-09 Mapping ΔV_T, Δβ to σ(ΔI/I) and σ(V_os)
- Kind: formula
- Statement: From PEL eq.(2) with θ = 0 (and DRE eq.4, LDM eq.3):
  - Mirror (equal V_GS): ΔI/I = Δβ/β − (g_m/I_D)·ΔV_T. So σ²(ΔI/I) = (g_m/I_D)²σ²(ΔV_T) + σ²(Δβ/β) − 2(g_m/I_D)·ρ·σ(ΔV_T)σ(Δβ/β).
  - Diff pair (equal I): V_os = ΔV_T − (I_D/g_m)·Δβ/β. So σ²(V_os) = σ²(ΔV_T) + (I_D/g_m)²σ²(Δβ/β) − 2(I_D/g_m)·ρ·σ(ΔV_T)σ(Δβ/β).
  - Square law: g_m/I_D = 2/(V_GS − V_T). This gives LDM eq.(3), σ²_ΔVGS = σ²_ΔVth + ¼(V_GS − V_th)²σ²_ΔK/K, and DRE eq.(4), σ²_Id = σ²_β + 4σ²_Vt/(V_gs − V_t)².
  - ρ = corr(ΔV_T, Δβ/β): ≈ 0 for close pairs, −0.35 at large spacing (PEL L111–119). The sign algebra is [derived]. A negative ρ increases both variances.
  - Caveat: DRE L131–135 notes that V_t and β share the t_ox dependence, so eq.(4) can overestimate σ_Id by up to 2×.
- Source: PEL eq.(2), L50–57, PDF p.1; DRE eq.(4), L124–135, PDF p.2; LDM eqs.(2)–(3), L129–138, PDF p.3.
- Philis stage: annotator, verify.
- Automation recipe: g_m and I_D come from `oppoint::extract` (frontend/library/src/oppoint.rs:184-187, fields `gm_us` and `id_ua`). Using measured g_m/I_D instead of 2/V_ov covers weak inversion, as DRE L43–54 requires. A_β comes from the new deck key (MM-28). When A_β is absent, drop the β term and report it "unknown".
- Beats hand layout because: every mirror gets a current-accuracy number at its actual bias.
- Philis status: **missing**. `gm_us` is consumed only by cellgen gate-resistance folding (frontend/library/src/cellgen.rs:663-669).

### MM-10 Rotation affects β: matched devices share orientation (hard)
- Kind: rule
- Statement: 90° rotation of one member affects only the current factor. A_β for a rotated pair "varies considerably with wafer and batch" and is attributed to mobility (Fig.3d).
- Source: PEL L98–110, PDF pp.2–3; Fig.3d L198–199, PDF p.4.
- Philis stage: cells, dp.
- Automation recipe: Hard constraint: all units of a matched set have identical orientation and identical signed S→D direction.
- Beats hand layout because: it is enforced by construction and cannot be broken by a late manual rotation.
- Philis status: **implemented**. backend/dp/src/lib.rs:555-560 (`rotatable` refuses turns inside groups) and frontend/library/src/cellgen.rs:255-258 (`currents_run_alike`).

### MM-11 In-line versus side-by-side placement is neutral (2.5 µm data)
- Kind: heuristic
- Statement: No significant matching difference between parallel and in-line placement, nor from 100 °C heating, scratch protection, or V_t-implant dose.
- Source: PEL L122–124, PDF p.3.
- Philis stage: gp, dp.
- Automation recipe: Do not add a constraint that forces one arrangement over the other for a simple pair. Let the moment cost (MM-30) choose. Scope note: this is one 1988 process with 30 µm spacing, so do not generalize it to LOD/WPE-sensitive nodes, where Philis already prices LOD separately (kernel/analog/src/placement/cc.rs:38-42).
- Beats hand layout because: it avoids over-constraining the search space.
- Philis status: **consistent**. No such constraint exists.

### MM-12 A_VT, A_K scale with oxide thickness; A_β does not
- Kind: heuristic
- Statement: On one batch with varied d_ox, A_Vt0 ∝ d_ox and A_K ∝ d_ox while A_β stayed constant. This points to a charge-related cause (PEL). Thinner oxides reduce mismatch (PEL L152–153). [derived] PEL's ratio is 30 mV·µm / 50 nm = 0.6 mV·µm/nm.
- Source: PEL L119–121, L152–153, PDF p.3.
- Philis stage: deck.
- Automation recipe: Use it only as a deck sanity check: warn if A_VT/t_ox is far outside the range of the other decks. Do **not** use it as a fallback estimator. The sky130 deck's MC value, 9.5 mV·µm (pdks/sky130.json:93), shows halo-era processes break t_ox scaling, as DRE L86–91 explains.
- Beats hand layout because: it catches unit errors in deck constants.
- Philis status: **missing** (low value).

### MM-13 Mask-induced systematic mean mismatch
- Kind: rule
- Statement: Reticle W/L errors produce **non-zero mean** differences that correlate with the mask set, not with area.
- Source: PEL L64–67, PDF p.2.
- Philis stage: cells.
- Automation recipe: Matched units must be drawn from the identical cell master with the same grid alignment, so that any reticle error is common-mode. This is a rule for the cell generator's unit reuse.
- Beats hand layout because: identical instancing is guaranteed.
- Philis status: **implemented** in the unit-based generators (kernel/core/src/units.rs:6-25 records identical units per member). Not separately checked.

### MM-14 Area/accuracy trade: σ ∝ 1/√(area)
- Kind: metric
- Statement: Lowering the op-amp offset from about 1.6–1.7 mV to 0.6 mV needed "9 times as much gate area", i.e. 0.1 mm² of active gate.
- Source: PEL L143–145, PDF p.3.
- Philis stage: annotator (report).
- Automation recipe: When a matched set's *random* σ alone exceeds its budget (η = 0 in emit.rs:85-86), report the gate-area multiplier (σ_rand/σ_budget)² needed. Placement cannot fix this, so emit a sizing diagnostic instead of letting the annealer chase an impossible residual.
- Beats hand layout because: it separates "the layout cannot fix this" from "the layout is bad".
- Philis status: **partial**. η clamps to 0 (emit.rs:85-86), but no diagnostic is emitted.

### MM-15 Error-budget ledger (band-gap decomposition)
- Kind: algorithm
- Statement: PEL decomposes σ_bg = 19 mV into independent contributions: V_BE process 6 mV; ΔV_BE 0.14 mV × gain 11 = 1.5 mV; resistor ratio 0.3 mV; resistor wafer variation 0.7 mV; op-amp 18 mV (1.6 mV input × gain). The op-amp input offset is itself computed from the 8 input-stage transistors: 1.6 mV (V_t) ⊕ 0.35 mV (β) → 1.7 mV.
- Source: PEL L128–142, PDF p.3.
- Philis stage: annotator, verify, flow.
- Automation recipe: For each matched set keep a ledger of named terms in the set's electrical unit: random V_T, random β, linear gradient, quadratic gradient, thermal, LOD, WPE and common-node IR drop. Random terms add in quadrature. Deterministic signed terms add linearly as μ_sys. The Θ residual is max(0, |μ_sys| + √Σσ² − budget)/budget, or the existing `over()` form. Print the ledger in the bench table.
- Beats hand layout because: a human does this for one block. The tool does it for every set, every epoch, and shows which term dominates.
- Philis status: **partial**. The terms exist as separate rules with separate η shares: `MatchingPair` (matching_pair.rs), `CentroidGroup` including LOD (cc.rs:123-160), `ThermalGradient` (emit.rs:176-185) and common-node ΔR (frontend/library/src/lib.rs:780-797). They are not summed into one budget.

### MM-16 Propagation of variance (POV)
- Kind: formula
- Statement: σ_e² = Σ_i (∂e/∂p_i)²·σ²_pi(geometry). e is any electrical parameter and p_i are independent process parameters (DRE eq.8/10). Geometric scaling enters both σ²_pi (eqs.1–3) and the sensitivities through the SPICE model (DRE L221–226).
- Source: DRE eqs.(7)–(10), L140–171, PDF p.3.
- Philis stage: annotator.
- Automation recipe: The same linear form sums **systematic** layout shifts: Δe = Σ_i (∂e/∂p_i)·Δp_i. Here Δp_i is ΔV_T from a thermal ΔT, a LOD Δ(1/SA+1/SB), WPE or the gradient, and ΔR_S from routing. The sensitivities are ∂I/∂V_T = −g_m, ∂I/∂β = I/β, ∂V_os/∂R_S = I_D. This is the ledger's arithmetic (MM-15).
- Beats hand layout because: every layout-dependent effect lands in one number on a common scale.
- Philis status: **partial**. Each rule divides by σ_rand in the V_T domain (emit.rs:94-102, lib.rs:788-797), which is a POV with sensitivity 1.

### MM-17 Back-propagation of variance (BPV) for deck characterization
- Kind: algorithm
- Statement: Measure (or simulate) σ_Id at n bias × geometry points. Build the squared-sensitivity matrix row by row in SPICE at each point, then solve eq.(11) by linear regression for the process-parameter variances. The design must make each parameter observable above the measurement error. The parameters are assumed independent, and residual correlation means the parameter set is wrong.
- Source: DRE eq.(11), L151–206, PDF p.3.
- Philis stage: deck (offline tool).
- Automation recipe: Minimal version for Philis: extend benchmarks/characterize_mismatch.py, which today fits σ(ΔV_T) at constant current through the origin (header L1–12, `run` L86–103). Add a voltage-biased I_D-pair run at two overdrives per geometry (e.g. V_ov ≈ 0.1 and 0.4 V). Solve the 2-parameter BPV σ²(ΔI/I)_k = (g_m/I)_k²·A_VT²/(WL) + A_β²/(WL) by least squares, which yields `abeta_*_pct_um`. A full 8-parameter BPV is not needed, because the PDK mismatch models already implement physical parameters.
- Beats hand layout because: the deck gets a β constant derived from the foundry's own statistical models.
- Philis status: **missing** (script fits A_VT only).

### MM-18 Characterization geometry set must span wide/short and narrow/long
- Kind: deck-requirement
- Statement: DOEs that choose geometries near L = W make the flawed eq.(5) look correct, and large errors then appear for wide/short and narrow/long devices.
- Source: DRE L129–138, PDF p.2.
- Philis stage: deck.
- Automation recipe: In characterize_mismatch.py add W/L points of at least 10:1 and 1:10 at minimum and 4× minimum L. Store per-geometry A_VT when the point estimates spread by more than ±15 % (the sky130 pfet spreads 8.4–12.4, pdks/sky130.json:96). Optionally fit σ² = A_a²/(WL) + A_e²/(W²L) + A_l²/(WL²) (MM-04 form).
- Beats hand layout because: the deck model is validated on the geometries analog designers actually use.
- Philis status: **partial**. The script uses 6 W/L from 1/0.15 to 8/2 µm (pdks/sky130.json:94).

### MM-19 Multi-unit (fingered) devices: area law over total area
- Kind: formula
- Statement: n parallel units: the variance component rises n× and the squared sensitivities fall n², so σ_Id falls by √n relative to one unit. An 80 µm device built as 4 × 20 µm has half the σ of one 20 µm unit, the same as the monolithic 80 µm device.
- Source: DRE §III-C, L336–358, PDF p.6.
- Philis stage: annotator, cells.
- Automation recipe: A device's random σ uses total gate area Σ W_u·L_u over all its units. Take this from `Layout.units` weights (gate area nm², kernel/core/src/units.rs:23-25) rather than netlist params, so folding and unitization choices are counted as drawn.
- Beats hand layout because: the σ tracks the drawn unit set exactly.
- Philis status: **partial**. `gate_um2 = w·l·max(nf, m)` (backend/annotator/src/lib.rs:155-160; `fingers` at backend/annotator/src/constraints.rs:22-24). A device with both nf > 1 and m > 1 is undercounted, since max rather than product is used. Verify this against the netlist convention (cellgen.rs:842 says per-finger w).

### MM-20 Ratioed mirrors: put the multiplicity in the output
- Kind: heuristic
- Statement: Table III (2×2 µm² nMOS, 0.13 µm, I_REF scaled to keep the reference at constant voltage bias) gives SD(ΔI) of 3.86 % (1:1), 1.52 % (1 ref : 4 out), 3.05 % (4:1) and 1.41 % (4:4). "The majority of the improvement … is gained by using them for the output, not the reference device." [derived] The 4:1 row matches the unequal-area formula (MM-02): √(1/4+1)/√2 × 3.86 = 3.05 %. The 1:4 row does **not** (the formula gives the same 3.05 %). Evaluate ratioed mirrors in SPICE (MM-29), not with the area law alone.
- Source: DRE Table III, L286–308, PDF p.6.
- Philis stage: annotator (report), cells.
- Automation recipe: For a mirror block (reference in slot 0, backend/annotator/src/block.rs:26-27) with ratio 1:n, compute σ(ΔI/I) with per-device variance (MM-02) and flag when the reference owns more units than an output. Report the SPICE MC value when it is available.
- Beats hand layout because: the quantity is computed per output leg.
- Philis status: **missing**.

### MM-21 Mirror matching is set by L, not W (current bias)
- Kind: heuristic
- Statement: At fixed current, raising L lowers the intrinsic mismatch and raises V_od, which improves matching twice over. Raising W lowers the intrinsic mismatch but lowers V_od, and "can give rise to little or no improvement". For halo or graded-channel devices, small W/L helps dramatically. Higher V_od reduces headroom.
- Source: DRE §III-A, L256–275, PDF p.5; Figs.6–8, L243–255.
- Philis stage: annotator (diagnostic).
- Automation recipe: When a mirror's σ(ΔI/I) (MM-09) exceeds its budget with η = 0, the MM-14 diagnostic names L as the lever. It lists the current V_od (from V_dsat, oppoint.rs:186) and headroom (oppoint.rs `headroom_mv`) as the cost.
- Beats hand layout because: the tool attaches the physics-correct fix to the violation.
- Philis status: **missing**.

### MM-22 Device-type comparison under current bias
- Kind: heuristic
- Statement: Under current bias pMOS appears to match better than nMOS because of its higher V_od. Table II (2×2 µm², 0.4 µm BiCMOS): mirror at 10 µA gives nMOS 1.22 % and pMOS 1.09 %. Voltage-driven at 1.70 V gives nMOS 0.589 % and pMOS 1.09 %.
- Source: DRE §III-B, L276–334, PDF pp.5–6.
- Philis stage: annotator.
- Automation recipe: None for P&R. This supports computing σ at the op-point (MM-08, MM-09) instead of comparing A_VT between polarities.
- Beats hand layout because: n/a.
- Philis status: n/a.

### MM-23 Mismatch varies with bias current (active loads)
- Kind: rule
- Statement: Mirror I_d mismatch is not constant when I_ref varies, as in an active load (Fig.9, W/L = 2/2 µm, 0.13 µm; the total falls as I_ref rises from 1 µA to 100 µA). Exact values are not given in the text.
- Source: DRE L269–275, Fig.9 caption L295–296, PDF p.6.
- Philis stage: annotator.
- Automation recipe: Evaluate MM-09 at the op-point I_D of each member and not at a nominal value. When `id_ua` is `None` (bias unknown, frontend/library/src/lib.rs:482-483), report the V_T-domain σ only and mark the current-domain value unknown.
- Beats hand layout because: the op-point is re-read for every run.
- Philis status: **partial**. The op-point feeds common-node ΔR (frontend/library/src/lib.rs:792-797).

### MM-24 Long-distance mismatch grows as L (MOS) or W (resistor) shrinks
- Kind: rule
- Statement: Long-channel transistors and wide resistors show negligible long-distance mismatch. Short-L transistors and narrow resistors show linear gradients, and at some chips nonlinear ones. DVP (130 nm): S_D rises as L falls, S_D ∝ 1/L_eff, and resistor S_D depends only weakly on W. LDM (0.35 µm poly) shows a strong W dependence (0.19 → 1.2 %/mm for W 2.2 → 0.55 µm).
- Source: LDM L194–246, L255–261, Table II L266–270, PDF pp.4–6; DVP L142–149, L213–224, Fig.8, PDF p.3.
- Philis stage: deck, annotator.
- Automation recipe: Store S as a function of the critical dimension: S_VT(L) for MOS, S_R(W) for resistors (MM-27). The annotator evaluates it at each matched set's actual L or W instead of one worst-case scalar.
- Beats hand layout because: a long-L device is not penalized with a short-L gradient.
- Philis status: **partial**. There is one scalar `svt_uv_per_um`, taken as the worst case at the shortest L (pdks/sky130.json:97-98 and pdks/ihp_sg13g2.json:17-18 use 1.63; pdks/gf180mcu.json:22-23 uses 0.941), and the same S for pMOS.

### MM-25 S_D(L) calibration from DVP Table 1
- Kind: data-model
- Statement: 130 nm thin-oxide nMOS (L µm → S_D mV/mm = µV/µm): 0.12 → 1.63; 0.24 → 0.780 (W 1.6) and 0.941 (W 12.8); 0.48 → 0.582 (W 1.6) and 0.644 (W 3.2). Fig.8 plots S_D² against 1/L_eff². [derived] Least-squares fit on drawn L: S_D² = 0.1835 + 0.03533/L² (µV/µm)², L in µm. It reproduces 1.62, 0.89 and 0.58 at L = 0.12, 0.24 and 0.48. The pure-proportional S_D = 0.2065/L fits worse at L = 0.48 (0.43 against 0.58–0.64).
- Source: DVP Table 1, L172–198, Fig.8 L142–170, PDF p.3.
- Philis stage: deck.
- Automation recipe: New optional deck keys `svt_a_uv2_per_um2` = 0.1835 and `svt_b_uv2` = 0.03533 (proxy, 130 nm). Evaluate S(L) = √(a + b/L²) with L in µm. Keep `svt_uv_per_um` as the fallback when the pair is absent. Label the provenance "PROXY" exactly as the existing notes do.
- Beats hand layout because: the distance budget becomes geometry-aware.
- Philis status: **missing**.

### MM-26 Nonlinear (quadratic) long-distance profiles: match second moments
- Kind: check
- Statement: For short-L and narrow devices in all three technologies, the long-distance profile is nonlinear and is fitted by a **second-order polynomial** over a few mm. "A quadratic behavior over a few mm distance cannot be compensated by means of circuit techniques." The paper gives no coefficient.
- Source: LDM L275–287, L315–320, Fig.11, PDF p.6.
- Philis stage: dp, verify.
- Automation recipe: Model the field as P(r) = P₀ + g·r + ½·rᵀH·r. For a matched set, compute the area-weighted second central moments of each side about the set centroid c: M_k = Σ a_u (r_u − c)(r_u − c)ᵀ / Σ a_u (three numbers: xx, yy, xy). Then ΔP_quad = ½·Σ_ij H_ij (M_k − M_l)_ij. Because H is not given by any source, report ‖M_k − M_l‖_F (µm²) normalized by the set's extent² as a dimensionless cost term. Priced only when a deck supplies `quad_uv_per_um2`; otherwise "unknown", never "pass".
- Beats hand layout because: moments are exact on the drawn units. Hand layout checks only visual symmetry (ABBA is centroid-matched but not second-moment-matched; ABBA versus BAAB differ in M_xx).
- Philis status: **missing**. It is acknowledged at kernel/analog/src/placement/cc.rs:84-86 ("a curved field would want the second moments too"). The cap generator reasons about quadratic gradients qualitatively (kernel/cells/src/cap_array.rs:61).

### MM-27 Orientation-dependent (anisotropic) distance coefficient
- Kind: data-model
- Statement: The same narrow poly resistors on the same chip show no long-distance mismatch when oriented parallel to the wafer flat and dominant long-distance mismatch when perpendicular (Fig.10: L/W = 35.2/0.55, I_IN = 119 µA, process B). Short-channel transistors also show orientation dependence.
- Source: LDM L199–201, L300–313, Fig.10 L249–260, PDF pp.5–6.
- Philis stage: deck, annotator, gp/dp.
- Automation recipe: The deck carries `svt_x_uv_per_um` and `svt_y_uv_per_um` (die x and y). The gradient variance becomes σ²_grad = S_x²·Δm_x² + S_y²·Δm_y² (MM-30), which rewards spreading matched sets along the quiet axis. Default: S_x = S_y = S.
- Beats hand layout because: the anisotropy is applied quantitatively to every set.
- Philis status: **missing**. It uses the isotropic `hypot` (kernel/analog/src/placement/matching_pair.rs:70; cc.rs:96).

### MM-28 A_β deck key (and per-polarity S)
- Kind: deck-requirement
- Statement: Current-domain mismatch needs A_β (%·µm) per polarity (PEL Table 1 gives 2 and 3 %·µm at 2.5 µm; modern values are not given in these sources). S is measured for nMOS V_GS only in DVP. PEL Fig.2b shows σ(β) rising with D, but no S_β value is given.
- Source: PEL Table 1, L102–108; Fig.2b L190, PDF pp.2,4; DVP L179–191.
- Philis stage: deck.
- Automation recipe: Add `abeta_n_pct_um` and `abeta_p_pct_um` to pdks/*.json, filled by the MM-17 BPV run with a `_source` note. Read them in `library::annotation` next to `avt_*` (frontend/library/src/lib.rs:505) into `ProcessNumbers.abeta_pct_um: [Option<f32>; 2]` (backend/annotator/src/netrole.rs:91). The gradient acts on V_T only; S_β stays "not given".
- Beats hand layout because: mirrors get priced in current error.
- Philis status: **missing**.

### MM-29 Per-group SPICE Monte Carlo for the random term (SPICE-domain mismatch)
- Kind: algorithm
- Statement: DRE's central recommendation is to characterize and apply mismatch in the same domain and tools as design (SPICE with a physical mismatch model, via MC or sensitivity analysis). This is valid across bias and geometry, including weak inversion and short channel.
- Source: DRE L21–29, L159–199, L229–236, PDF pp.1,3,4.
- Philis stage: flow, annotator.
- Automation recipe: Philis already builds an ngspice op deck (frontend/library/src/oppoint.rs:155-171). For each matched set, add a small MC deck: the set's devices at their op-point terminal voltages, with the PDK mismatch switch that characterize_mismatch.py already uses (pdk["mm"]), N = 200 samples. Measure ΔI/I (mirror/load) or the V_GS difference at equal I (diff pair). Cache the result by (model, W, L, nf, V_GS, V_DS). This σ_rand replaces A_VT/√WL in the ledger (MM-15) and removes MM-03, MM-17 and MM-20 from the critical path. Cost: one ngspice batch per unique geometry and bias, run once before the epoch loop.
- Beats hand layout because: the random floor is exact for the actual devices and bias, not a textbook law.
- Philis status: **missing**. The MC infrastructure exists only offline (benchmarks/characterize_mismatch.py:74-84).

### MM-30 Placed-group mismatch estimator (the model Philis should score with)
- Kind: algorithm
- Statement: This consolidates MM-01, 02, 09, 19, 25, 26 and 27. For a matched set with sides k = 1..K (diff pair or load K = 2; mirror: reference plus outputs, evaluated pairwise against the reference):
  1. Per side: A_k = Σ_u a_u (µm², a_u = unit W·L from `Layout.units[].weight`/1e6). Centroid m_k = Σ a_u r_u / A_k (µm). Second central moment about the set centroid c: M_k = Σ a_u (r_u − c)(r_u − c)ᵀ / A_k (µm²).
  2. Random, from MM-02: σ²_VT,rand = (A_VT²/2)(1/A_k + 1/A_l) (mV²) and σ²_β,rand = (A_β²/2)(1/A_k + 1/A_l) (%²). If MM-29 has a SPICE value, it replaces both.
  3. Statistical linear gradient, from MM-01 and MM-27: σ²_grad = (S_x(L)·Δm_x)² + (S_y(L)·Δm_y)². S in mV/µm (deck µV/µm × 1e-3), Δm = m_k − m_l in µm, and S(L) from MM-25.
  4. Quadratic, from MM-26: σ_quad = ½·Q·‖M_k − M_l‖_F when the deck has Q (mV/µm²); otherwise report ‖ΔM‖ and mark unknown.
  5. Deterministic mean offset μ_sys (mV, signed), summed linearly: TC·ΔT (existing thermal rule), KVTH0·Δ(1/SA+1/SB) (existing LOD term), I_D·ΔR_S (existing common-node term), and WPE.
  6. Electrical mapping, from MM-09 with G = g_m/I_D from the op-point (fallback 2/V_ov, where V_ov is not given → V_T domain only):
     - mirror/load: σ²(ΔI/I) = G²(σ²_VT,rand + σ²_grad + σ²_quad) + σ²_β,rand, and μ(ΔI/I) = G·μ_sys;
     - diff pair: σ²(V_os) = σ²_VT,rand + σ²_grad + σ²_quad + σ²_β,rand/G², and μ(V_os) = μ_sys.
     - ρ = 0 by default (PEL L111–118: close pairs); −0.35 when the pair spacing is ≥ 250 µm (PEL L118; the paper's "large spacing" data point at 250–500 µm).
  7. Residual against the spec budget B (the `offset_sigma_mv` or a new `mirror_sigma_pct`): Θ = over(|μ| + σ − B, B). When B is absent, keep the current η semantics: usage = √(σ²_grad + σ²_quad + μ²)/(η·σ_rand).
- Source: PEL eqs.(1)–(3), L37–79; DVP eq.(1), L85–101; LDM eqs.(1)–(4), L129–151, Fig.11; DRE eqs.(4),(8), §III-C.
- Philis stage: annotator (emits the model), gp/dp (cost), verify/bench (report).
- Automation recipe (minimal diff):
  - (a) kernel/analog/src/placement/cc.rs:54-63 `unit_centroid` → also accumulate Σa·x², Σa·y² and Σa·xy, returning (A, m, M).
  - (b) kernel/analog/src/placement/matching_pair.rs:76 `gradient_over_random(d_nm, gate, s_over_a)` → take (Δm_x, Δm_y), (S_x, S_y)/A and the two side areas.
  - (c) backend/annotator/src/emit.rs:66-106 `Pelgrom` gains `abeta`, `g_over_i` and anisotropic S.
  - (d) The bench row (benchmarks/src/bench.rs:321-350) prints σ and μ per set in mV or %.
  - Complexity O(units) per set per evaluation, the same as the existing centroid.
- Beats hand layout because: it produces an expected σ(V_os) or σ(ΔI/I) with a per-term breakdown for every matched set of every candidate placement. Hand layout has only rules of thumb.
- Philis status: **partial**. Steps 1 (first moment), 2 (V_T only, single gate), 3 (isotropic, one S) and 5 (as separate rules) exist. Steps 4, 6 and 7 and the β term are missing.

### MM-31 Gradient offset budget allocation in quadrature
- Kind: formula
- Statement: With a total 1σ budget σ_b and a random floor σ_rand, the systematic share allowed is σ_sys,max = √(σ_b² − σ_rand²). This follows from independence of pair and distance parts (DVP L85–92: "considered to be independent … total mismatch … sum of the variances"). It gives D_max = σ_sys,max/S for a simple pair.
- Source: DVP eq.(1), L85–101, PDF p.2.
- Philis stage: annotator.
- Automation recipe: Keep the existing η = √(σ_b² − σ_rand²)/σ_rand. Apply it once per set to the *sum* of systematic terms (MM-15, MM-30), not separately per rule.
- Beats hand layout because: the budget is spent once across all terms, not several times over.
- Philis status: **implemented per rule** (backend/annotator/src/emit.rs:79-87). The same η is reused independently by MatchingPair, CentroidGroup, thermal (emit.rs:94-102) and common-node ΔR (frontend/library/src/lib.rs:794). Each rule may therefore consume the full allowance, and together they can overspend it by up to √(number of rules) or more.

### MM-32 Gradients are random in sign and direction: 2-D centroid matching
- Kind: rule
- Statement: Long-distance patterns change wafer to wafer and chip to chip (LDM L187–190). PEL treats the deterministic wafer cone as stochastic because the die position is unknown (PEL L33–38). DVP calls the gradient parameter "statistical" (DVP L11–12).
- Source: LDM L185–201, PDF p.4; PEL L33–38; DVP L8–13.
- Philis stage: gp, dp, cells.
- Automation recipe: Treat the gradient as a zero-mean random vector. Require coincident centroids in **both** axes (the variance form of MM-30 step 3 does this). A 1-D interdigitation only cancels one axis. Do not optimize against one assumed direction unless the gradient is deterministic and known (thermal, from placed heat sources).
- Beats hand layout because: the design is robust to every gradient direction by construction.
- Philis status: **implemented**. The 2-D centroid offset is `hypot` in cc.rs:90-97.

### MM-33 Common-node IR drop masquerades as a gradient: Kelvin connection and ΔR budget
- Kind: rule
- Statement: A 90 mΩ/□ interconnect carrying 320 µA drops 8.3 mV/mm, which completely corrupts a long-distance comparison and exceeds device mismatch (pair σ ≈ 1.6 mV; Table II). The fix is force/sense regulation of the reference node: the sense path carries no current, so its resistance does not matter (LDM Fig.4).
- Source: LDM L47–89, L180–204, Fig.5 L108–116, PDF pp.1–4.
- Philis stage: gr, dr.
- Automation recipe: For every matched set with a shared source or reference net, route the net as a star from one tap, or add a separate sense branch to the gate reference, i.e. a Kelvin connection. Bound ΔV = I·R□·Δ(length/width) ≤ the systematic allowance. Philis computes max ΔR = η·σ_rand/I_D per common node, and dr/gr must meet it.
- Beats hand layout because: every shared node's R imbalance is computed from the stack's R□ and the actual routes.
- Philis status: **partial**. `common_nodes` budgets ΔR (frontend/library/src/lib.rs:780-797) for 2-device DiffPair, CurrentMirror and Load blocks only (lib.rs:782-785). Force/sense topology is not generated.

### MM-34 Far-apart matched sets need long L / wide W
- Kind: heuristic
- Statement: The distance at which the gradient equals the pair σ is x* = σ_ΔP/|ΔP/Δx|. [derived] From LDM Table II: 0.35 µm nMOS L = 1.71/0.71/0.35 µm → 16 / 1.4 / 0.16 mm. 0.35 µm poly W = 2.2/1.1/0.55 µm → 1.0 / 0.66 / 0.31 mm. 0.25 µm pMOS L = 1.21/0.57/0.25 µm → 8.2 / 2.9 / 2.3 mm. 0.25 µm poly W = 3.2/0.8/0.4 µm → 7.7 / 5.9 / 2.5 mm. [derived] From DVP Table 1 (σ/S_D): nMOS 2.8–9.5 mm, resistors 3.0–10.7 mm. The papers' own figure captions state "comparable at Δx = 1 mm", "larger for Δx > 0.5 mm", and so on (LDM L159–178, L211–230).
- Source: LDM Table II, L266–270, PDF p.6; Figs.6–9 captions; DVP Table 1.
- Philis stage: annotator, gp.
- Automation recipe: For matched sets whose members must be far apart (resistor-ladder taps, flash-comparator inputs, arrays spanning more than a few hundred µm), compute x* per set with the deck S(L) or S_R(W). If the placed span exceeds x*, emit a sizing diagnostic (increase L or W) and weight the set's gradient term higher in gp. Resistor-ladder support needs a resistor-array block kind, which is currently absent.
- Beats hand layout because: every long-span set is checked, not just the obvious ones.
- Philis status: **missing**. BlockKind has no resistor or long-span array kind (backend/annotator/src/block.rs:23-36).

### MM-35 Resistor matching constants (pair and distance)
- Kind: deck-requirement
- Statement: 130 nm resistors (DVP Table 1): σ_ΔP = 1.55 / 2.20 / 0.982 / 0.694 / 0.491 % for W×L = 0.2×6 / 0.2×3 / 0.5×6 / 1×6 / 1×12 µm. [derived] A_R = σ·√(WL) = 1.70 %·µm for all five rows (1.698–1.704). S_D = 0.163–0.212 %/mm. LDM Table I: A_ΔR/R poly-high 4 / 2.4 / 4.7 %·µm and poly-2 6 / 3.3 / 3.0 %·µm (processes A/B/C).
- Source: DVP Table 1, L172–198, PDF p.3; LDM Table I, L116–124, PDF p.3.
- Philis stage: deck, annotator.
- Automation recipe: Add per-resistor-layer keys `ar_pct_um` and `sr_pct_per_mm` (plus `_source`). Then the MM-30 estimator runs for resistor pairs and arrays, with a unit weight equal to the resistor area W·L.
- Beats hand layout because: resistor ratios in references and gain setting get the same quantitative scoring as MOS pairs.
- Philis status: **missing**. There is no resistor matching key and no resistor matched block. The annotator emits MatchingPair only for FET block kinds (backend/annotator/src/emit.rs:150-170).

### MM-36 Detect systematic (layout-induced) error from distribution shape
- Kind: check
- Statement: Systematic influences show as a **shift or skewness** of the normal probability plot of ΔP. Independent random events give centred, symmetric distributions (DVP Fig.4: narrow resistor fit R² = 0.961 against 0.982 for wide).
- Source: DVP L86–111, Fig.4, PDF p.2.
- Philis stage: verify.
- Automation recipe: If Philis adds post-layout MC (extracted netlist + mismatch models), test each matched set's ΔP sample for mean ≠ 0 (t-test) and skewness. A significant mean is a layout systematic (IR drop, LDE, asymmetry). Report it next to the MM-30 μ_sys prediction as a cross-check.
- Beats hand layout because: the tool closes the loop between predicted and extracted systematic offsets.
- Philis status: **missing** (no post-layout MC).

### MM-37 Total mismatch is bounded by process tolerance
- Kind: rule
- Statement: The minimum σ_ΔT is the pair value (x → 0). σ_ΔT grows with x per eq.(1) and is capped by the process tolerance.
- Source: DVP L130–135, PDF p.2.
- Philis stage: annotator.
- Automation recipe: Clamp σ_grad ≤ the global (die-to-die) σ of the parameter if a deck provides it. Otherwise no clamp. Without a clamp, a linear S·x extrapolated over a large die overstates the mismatch.
- Beats hand layout because: n/a (model hygiene).
- Philis status: **missing** (low priority).

### MM-38 Characterization extraction: σ² against x² slope
- Kind: algorithm
- Statement: Step 1: A from close pairs. Step 2: σ_ΔT(x) from equal devices at many x across all chips. Then S_D² is the slope of σ²_ΔT against x², and the x = 0 intercept should equal the pair value (Figs.6–7). More than 400 pairs per geometry.
- Source: DVP L105–135, Figs.6–7 L142–212, PDF pp.2–3.
- Philis stage: deck (offline).
- Automation recipe: The PDK MC models carry no spatial term. The sky130 deck note says "Gradient S_VT is wafer-level and not in the models" (pdks/sky130.json:94). S therefore cannot come from characterize_mismatch.py. If a user has silicon data, a helper script fits S_D² as the slope of σ² against x² through the pair intercept. Document this in the deck `_source` convention.
- Beats hand layout because: n/a (data provenance).
- Philis status: **missing**. S is a proxy from DVP (pdks/sky130.json:97-98).

### MM-39 Separate systematic-mean effects from random effects in reporting
- Kind: metric
- Statement: DVP separates statistical pair mismatch (σ_ΔP) from the systematic distance effect (σ_D). PEL separates random mismatch from mask-induced non-zero means (L64–67) and from wafer-level variation (resistor 0.7 mV in the band-gap, L137–138).
- Source: DVP L78–101; PEL L64–67, L137–138.
- Philis stage: verify, bench.
- Automation recipe: In the bench output print three columns per matched set: σ_rand (sizing-limited, not placement's fault), σ_sys (placement or routing), and μ_sys (deterministic). Only the last two feed the Θ residual. The first is a design diagnostic.
- Beats hand layout because: layout quality is scored on what layout controls.
- Philis status: **partial**. Bench prints worst usage per kind (benchmarks/src/bench.rs:321-350) but not σ in mV.

---

## 4. Top-15 priorities for Philis

1. **MM-30 placed-group estimator**: turn the existing first-moment and Pelgrom code into one σ(V_os)/σ(ΔI/I) per matched set, with a per-term breakdown. This is the model this study was asked for, and most inputs already exist (cc.rs:54-63, emit.rs:66-106, oppoint.rs:184-187).
2. **MM-31 + MM-15 single budget per set**: one η spent across gradient, thermal, LOD, WPE and IR-drop terms. Today each rule gets the full allowance independently, which overspends it (emit.rs:79-102, lib.rs:794).
3. **MM-09 current-domain mapping with op-point g_m/I_D**: mirrors and loads are currently priced as if they were diff pairs in the V_T domain. g_m and I_D are already parsed.
4. **MM-28 + MM-17 A_β deck key via 2-point BPV in characterize_mismatch.py**: this unlocks the β term for all three open PDKs at the cost of one script extension.
5. **MM-29 per-group SPICE MC random floor**: it removes the 1/√area inaccuracy for wide/short, narrow/long, halo and ratioed devices (MM-03, MM-20). The ngspice plumbing exists.
6. **MM-26 second-moment matching**: quadratic fields are the only gradient form a centroid-matched layout still misses (LDM Fig.11). It costs three extra sums in cc.rs:54-63.
7. **MM-06 D* diagnostic**: shows per set whether the linear statistical gradient can bind at all, since with the sky130 deck numbers D* falls below 1 mm only for gates above about 34 µm² [derived]. This keeps the annealer from wasting moves.
8. **MM-24 + MM-25 S(L) instead of one worst-case S**: removes the systematic over-penalty on long-L devices. The fit comes from DVP Table 1.
9. **MM-33 Kelvin/star routing for shared matched nodes**: IR drop (8.3 mV/mm example) exceeds device mismatch. Extend `common_nodes` beyond 2-device blocks.
10. **MM-35 resistor matching (A_R, S_R) + resistor block kind**: references and gain ratios are not scored today. DVP gives A_R ≈ 1.70 %·µm [derived] as a proxy.
11. **MM-02 + MM-19 unequal-area and unit-exact random term**: ratioed mirrors and fingered devices get the exact variance. Fix `max(nf, m)` if both can occur.
12. **MM-27 anisotropic S_x/S_y**: it costs two deck numbers and one line in `gradient_over_random` and captures the wafer-flat orientation effect.
13. **MM-39 bench columns σ_rand / σ_sys / μ_sys in mV or %**: makes "better than hand layout" measurable in spec units.
14. **MM-14 + MM-21 sizing diagnostics**: when σ_rand alone breaks the budget, report the area multiplier and the L lever instead of letting the annealer chase an impossible residual.
15. **MM-34 long-span sets**: x* check and sizing warning for ladders and comparator arrays spanning more than a few hundred µm.
