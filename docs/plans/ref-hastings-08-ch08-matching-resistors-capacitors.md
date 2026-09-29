# Hastings ch.8: matching of resistors and capacitors, extracted for Philis

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson, 2023), Chapter 8 "Matching of Resistors and Capacitors" (book pp. 366–427, PDF pp. 367–429).
Reftext file: `scratchpad/reftext/hastings.txt` (pdftotext -layout). Line range read: **21772–25761**. Every line in that range was read.
The text extraction drops most inline numbers, units and equations. Numbers quoted here come from the PDF pages that were opened to recover them (listed in §1). A value that is blank in the PDF as well is marked "not given".

Code citations point at the working tree on 2026-09-28. Status checks are quick greps, not an audit.

---

## 1. Coverage

**Consecutive Read chunks of the reftext (offset..end):**
1. 21772..22771 (1000 lines)
2. 22772..24271 (1500 lines)
3. 24272..25021 (750 lines)
4. 25022..25761 (740 lines)

**PDF pages opened to recover dropped numbers and equations:** 367–381, 384–386, 388–401, 403–417, 418–426. That covers every equation (8.1–8.40), Tables 8.1–8.8, every rule in §8.3.1/§8.3.2 and all figures cited below.

**Headings in the range (reftext line):**
- Chapter 8 intro (21772)
- 8.1 Mismatch (21790)
- 8.2 Causes of Mismatch (21948)
  - 8.2.1 Random Variation (21965): Capacitors (22050), Resistors (22097)
  - 8.2.2 Process Biases (22184)
  - 8.2.3 Proximity Effects (22282): Photolithographic Effects (22290), Etch-Rate Variations (22378), Dopant Interactions (22486)
  - 8.2.4 Interconnection Parasitics (22585)
  - 8.2.5 NBL Shadow (22689)
  - 8.2.6 Hydrogenation (22794)
  - 8.2.7 Temperature (22931): Thermal Gradients (22940), Centroids (23074), Common-Centroid Layout (23143), Segmenting Resistors (23577), Placement of Resistor Arrays Relative to Heat Sources (23878), The Thermoelectric Effect (23960)
  - 8.2.8 Mechanical Stress and Package Shift (24055): Filler-Induced Stress (24225), Effects of Stress Upon Resistors (24384), Effects of Stress Upon Capacitors (24717)
  - 8.2.9 Electric Fields (24763): Voltage Modulation (24775), Charge Spreading (24950), Dielectric Absorption (24999)
- 8.3 Rules for Device Matching (25112)
  - 8.3.1 Rules for Resistor Matching (25148): rules R1–R25 (25164–25428)
  - 8.3.2 Rules for Capacitor Matching (25432): rules C1–C13 (25470–25617)
- 8.4 Summary (25623)
- Selected Bibliography (25648)
- 8.5 Exercises (25673–25756)

Tables: 8.1 thermal conductivity (22963), 8.2 1-D interdigitation patterns (23207), 8.3 2-D patterns (23331), 8.4 four CC rules (23425), 8.5 optimum ratio patterns (23643), 8.6 CTE (24144), 8.7 piezoresistance coefficients (24413), 8.8 die aspect ratios (24638). Figures 8.1–8.32.

---

## 2. Section-by-section digest

### Chapter intro (21772–21785, PDF p.367)
- Integrated R and C have absolute tolerances of ±20–30%. Matched pairs track far better: resistors ±0.1% and sometimes ±0.01%; capacitors as well or better (21774–21778).
- The same material also applies to BJTs (§10.2), diodes (§11.3) and MOS (§13.2) (21781–21784).

### 8.1 Mismatch (21790–21944, PDF pp.368–370)
- Eq 8.1: δ = [(x2/x1) − (X2/X1)]/(X2/X1) = X1·x2/(X2·x1) − 1. Measured x, intended X. Worked example: a nominally equal pair measured 12.47 kΩ and 12.34 kΩ gives δ = 1.1% (21791–21802).
- Sampling: at least 30 units, drawn from several wafers at the front, middle and back of the boat, random die locations but not the extreme wafer edge, no misprocessed or reworked wafers, production packaging (21813–21843).
- Eq 8.2 mean m_δ = (1/N)Σδ_i. Eq 8.3 s_δ = sqrt(Σ(δ_i − m_δ)²/(N−1)). The mean is the systematic mismatch, s_δ the random mismatch. Signs must be kept (21850–21886).
- Systematic example: 2 kΩ and 4 kΩ single strips with 50 Ω contacts give 2.4% mismatch. Building the 4 kΩ from two series 2 kΩ segments removes it (21871–21878).
- Eqs 8.4/8.5: UDL = m_δ + 6s_δ and LDL = m_δ − 6s_δ. Example: m = −0.35% and s = 0.22% give −1.07% to +0.37% (21906–21934).
- Mismatch measures accuracy, not precision (21938–21943).

### 8.2 Causes of Mismatch, intro (21948–21961, PDF p.371)
- Random mismatch comes from microscopic fluctuations and is reduced by value and dimension choice. Systematic mismatch comes from process bias, contact resistance, nonuniform current, diffusion interaction, stress and temperature gradients, and is reduced by layout technique.

### 8.2.1 Random Variation (21965–22180, PDF pp.371–374)
- Eq 8.6: s = m·sqrt(k_A²/(2A) + k_P²/(2P)), with areal coefficient k_A and peripheral coefficient k_P. The coefficients vary with material, process, fab and even wafer (21973–21993).
- Eq 8.7: s_δ = sqrt((s1/m1)² + (s2/m2)²). Eq 8.8 sums the four areal and peripheral terms. Eq 8.9, same geometry: s_δ = sqrt(k_A²/A + k_P²/P). Eq 8.10, areal-dominated: s_δ = k_A/√A, which is Pelgrom's law (22001–22045).
- Capacitors. Eq 8.11: s_δ = k_C/√C. Eq 8.12: s_δ = k_C·sqrt((C1+C2)/(2C1C2)). The smaller capacitor dominates, and halving the mismatch takes 4× the capacitance. Avoid large ratios. Series-built small capacitors do not work because their parasitics are uncontrolled. Consider trimming (22058–22091).
- Resistors. Eq 8.13: A = (R/R_S)·W². Eq 8.14: s_δ = (k_R/W)·sqrt(1/R). Eq 8.15: s_δ = (k_R/W)·sqrt((R1+R2)/(2R1R2)). Mismatch scales as 1/W and 1/√R (22103–22137).
- Eq 8.16: building the smaller resistor from N2 parallel segments cuts its term. A 100k:10k divider with the 10k made of 2×20k in parallel roughly halves the mismatch (22137–22154).
- Eq 8.17 (poly): k_R = d_g·sqrt(η·R_S), η ≈ 2, grain diameter d_g usually < 0.1 µm. It holds down to widths somewhat below 0.3 µm. Narrower widths approach the bamboo regime (22158–22171).

### 8.2.2 Process Biases (22184–22276, PDF pp.374–375)
- Eq 8.18: x_b = x − x_d (22187–22191).
- Width bias: 2 µm and 4 µm poly with −0.1 µm bias give a 0.487 ratio (2.6% error). Two parallel 2 µm strips give 0.500 (22196–22203).
- Length bias from a −0.2 µm contact bias: 20 µm and 40 µm give 0.503 (0.5%). Two series 20 µm segments give 0.500. Equal segments also cancel contact R and end current crowding (22207–22215).
- Capacitors: 10×10 and 10×20 µm with −0.1 µm bias give 0.497 (0.6%). Matching needs equal area/periphery ratio, meaning unit arrays for integer ratios (3 pF and 5 pF from 1 pF units) (22218–22228).
- Non-integer ratios, Fig 8.2 (eqs 8.19/8.20), with C1 a square of side W1: L2 = (C2/C1)(1 + sqrt(1 − C1/C2))·W1 and W2 = (C2/C1)(1 − sqrt(1 − C1/C2))·W1. The drawback is that W2 ≠ W1, so the unit does not tile. Alternatives keep W2 = W1 and add a notch or tang (22229–22272).

### 8.2.3 Proximity Effects (22282–22580, PDF pp.375–380)
- Iso-dense print bias is negligible above w_min = λ/NA (eq 8.21). For i-line (365 nm, NA 0.6) that is about 0.6 µm. In practice it matters below about 2× the minimum feature (22294–22312).
- End segments of a dense array need dummies at the same spacing. A dummy can be narrower (≥ w_min) and needs no contacts (Fig 8.3A). Connected dummies are preferred when room exists, because an unconnected poly can charge up. Never tie dummies to noisy nodes or to nodes their capacitance would disturb. Consult the designer (22312–22347).
- H-V bias: orient matched resistors the same way even when they are not adjacent (22351–22354).
- Photoresist flow around tall topography: poly-poly capacitors affect MOS matching within about 30 µm. The effect is worst at the wafer edge. Move matched geometry away (22362–22368).
- Etch microloading narrows the outer segments (Fig 8.4). Example: 10 kÅ poly etched 90% anisotropically gives 0.1 µm undercut, and a 0.02 µm/side iso-dense bias gives a 2% error on 1 µm-wide resistors. The effect penetrates ≥10 µm. For the best matching use dummies spanning ≥10 µm, equal widths, spacings and lengths, and segments extending ≥10 µm past the active region. Otherwise one full-width dummy (22378–22438).
- Capacitor arrays: the outer ring is dummies (Fig 8.5: 6 active, 14 dummies). A dummy needs only span the etch penetration, so ≤10 µm-wide dummy columns and rows suffice if that is the measured depth. Tie dummies to a node. Shields above (metal-2) and below (poly-1) are tied together to a low-Z node and the dummies join them. Arrays whose capacitors do not all have one side on a low-Z node need parasitic analysis (22441–22482).
- Diffusion tails: same-polarity neighbours lower sheet R and widen the diffusion. Opposite polarity does the reverse. An iso-dense diffusion bias hits the end resistors, so add dummies. Dummy width needs to be only 3–4× the junction depth (2 µm junction gives 6–8 µm). Diffused dummies copy the heads and contacts and are biased to the enclosing region (22486–22537).
- Serpentine: keep turn spacings equal and heads away from the body (Fig 8.7B). Put a deep-N+ sinker behind a head, not beside the body (Fig 8.7D) (22541–22565).
- WPE: matched diffusions sit ≥2–3 µm inside drawn well edges. Ions scatter >1 µm (22569–22575).

### 8.2.4 Interconnection Parasitics (22585–22685, PDF pp.380–382)
- Jumper R: with 1 kΩ segments, 50 mΩ/□ metal and 0.5 µm width, a 100 µm jumper adds 1%. Watch segments ≤1 kΩ. Longer segments reduce the effect but make spindly, gradient-prone arrays (22590–22601).
- Fixes: put interconnected segments adjacent, widen jumpers, use multiple vias. The best fix makes each jumper's R proportional to its segment's R: lengthen short jumpers with jogs, widen long ones, and insert vias in every jumper if any has one (Fig 8.8). Via R is random, so use several vias (22604–22632).
- Current-carrying leads: short, direct, widened. Use Kelvin connections where needed (22633–22636).
- Lead C: a 7.5 kÅ-thick, 1 µm-wide metal over 10 kÅ FOX is about 0.13 fF/µm. 100 µm is 13 fF, or 1.3% of 1 pF. Equalize per-unit lead C with jogs or dead-end branches (Fig 8.9). Use equal lead widths and ≥2 µm spacing to other metal. If one lead crosses a metal, all cross. Exclude dummy metal. Verify by back-annotation (22640–22670).
- A met2 shield over poly-poly capacitors ties to analog ground. Never route clocks across it. Keep noisy lines 3–4 µm clear of the shielded area. The shield raises lead C but makes it predictable (22671–22685).

### 8.2.5 NBL Shadow (22689–22790, PDF pp.382–384)
- Epitaxy shifts, distorts or washes out buried-layer topography. On tilted (111) wafers the shift is 50–150% of epi thickness with dichlorosilane and 100–200% with SiCl4. Measure it experimentally (22696–22741).
- Diffused resistors in NBL tanks are vulnerable, HSR most of all. The fix is to increase NBL overlap on the side the shadow shifts toward, including alignment error. STI/CMP processes have no shadow (22750–22790).

### 8.2.6 Hydrogenation (22794–22925, PDF pp.384–386)
- CVD nitride contains 4–40% H. Hydrogen passivates dangling bonds, lowering MOS V_T mismatch and 1/f noise. It reduces HSR poly sheet R by up to 30% and slightly raises low-sheet boron-doped poly (22795–22831).
- Aluminium blocks and getters H. Metallized and unmetallized poly resistors have mismatched by up to 10%, with gradients microns away from metal edges. A ~400 °C anneal helps, but deep-submicron flows anneal less (22850–22857).
- Fig 8.12: folded-out jumpers match better than folded-in ones, even with dummy metal. A met1 plate over the array, with only the heads exposed, lets met2 jumpers fold in. A met2 plate is less effective. Tie the plate to a low-Z node with no voltage relative to the resistors (22872–22909).
- Without a plate: no leads over matched segments, not even between them, and dummy-fill block on all metal layers (22913–22920).
- Monocrystalline boron HSR shows small hydrogen-compensation effects. The same measures apply (22923–22925).

### 8.2.7 Temperature (22931–24051, PDF pp.386–402)
- Most integrated resistors have TC ≥1000 ppm/°C. Power devices create tens of °C across a die (22932–22937).
- Eq 8.22: Q = −κ∇T. Table 8.1 κ at 25 °C in W/(cm·°C): Si 1.56, SiO2 0.013, mold 0.008–0.021, Cu alloys 3.0–3.6, Ag epoxy 0.08–0.20, Ag sinter 0.80, SAC solder 0.59, Alloy-42 0.11 (22941–23024).
- Heat-sunk packages give steep gradients near power devices and a flat field far away. Non-heat-sunk packages are nearly isothermal but see transient waves (23029–23070).
- Centroid assumptions: (1) planar active region, (2) uniform contribution, (3) linear parameter-vs-T, (4) linear T-vs-position. Then the device temperature is T at its centroid. Centroidal symmetry: the centroid lies on every symmetry axis (23074–23114).
- Eq 8.23: ΔR = α·R·d_CC·∇T_CC. The four levers are lower TC, lower gradient, separation perpendicular to the gradient, and smaller d_CC (23120–23139).
- Common centroid: ABBA is CC. ABAB has centroids one pitch apart and still needs dummies. ABA gives 2:1. Series-parallel mixes are common for R and rare for C. Table 8.2 lists 1-D patterns (23143–23245).
- Pair construction: each mirrored pair belongs to the same device, has the same impact on the value and sits at equal distance from the axis. An optional central segment is allowed. Examples: SPPS/PSSP, QRRRQ/RQRQR, SPRQRQRPS. If exact CC is impossible, minimize the separation (23248–23273).
- Eqs 8.24/8.25: X_R = Σ(∂R/∂R_i·X_i)/Σ(∂R/∂R_i), and the same for Y. This covers arbitrary networks (23277–23290).
- 2-D: the cross-coupled pair (Erdi, ~1970) is compact for square capacitor units and rare for resistors. Table 8.3 lists 2-D patterns (23292–23415).
- Table 8.4, the four CC rules (Coincidence, Symmetry about both axes, Dispersion, Compactness). Preference order: ABBA > ABAB > AABB (23417–23447).
- Symmetry is the reliable route to coincidence. Asymmetric CC arrays are discouraged (23449–23458).
- Dispersion, eq 8.26 Taylor series: CC cancels c0 and c1. The residue is dominated by c2(x−a)², which is proportional to the squared distance from the centroid. ABBAABBA has half the residue of AABBBBAA. A 4×4 ABAB/BABA (9 cross-coupled pairs) beats ABBA/BAAB (4). For three or more devices, give the most important pair its own CC subgroups (ABBACCABBA for A–B, BBACAACABB for A–C). Spacing waste lowers the active area and raises the random term, so an optimum exists. Large gradients favour dispersion. With no gradients, ABBA or AB/BA is best, and CC is no better than non-CC. Subarrays that each obey the rules release the whole array from them (23462–23523).
- Compactness: the residue is proportional to the square of the span. Example: A_P A_P D B_S B_S A_S A_S D B_P B_P is inferior to A_P B_P B_P A_P A_S B_S B_S A_S (23529–23538).
- Fig 8.17: a 4-segment array cancels the linear term and most of the quadratic one. An octagonal array cancels both but costs area and needs non-Manhattan shapes. 2-D CC suffices in practice (23541–23573).
- Segmenting: matched groups come from the designer's schematic notes. Width ≥150% of minimum, ≥300% for accurate work. Length ≥3× minimum, ≥5× for accurate work. TiSi2 segments need a minimum area (C49→C54, 80–100 → 13–20 µΩ·cm). Small areas fail to recrystallize, leaving outliers at 3–5× the mean in string DACs; Co and Ni silicides are immune (23577–23618).
- Maximize CC subarrays until the segment-length limit or until the subarray aspect reaches 1:1. Arrange subarrays in 2-D (23626–23632).
- Table 8.5 optimum patterns: 1:1 ABBA; 2:1 ABA; 3:1 AABAABAA; 3:2 ABABA; 4:1 AABAA; 4:3 ABABABA; 5:1 AAABAAAABAAA; 5:2 AABABAA; 5:3 ABAABABAABABAABA; 5:4 ABABABABA (23636–23705).
- Arbitrary M:N: place M+N segments and scatter uniformly (9:5 gives ABAABAABABAABA). A 9:1 divider becomes a 2×2 series-parallel small resistor, pattern 9:4 AABAABABAABAA, with 4× the area and half the mismatch (23707–23721).
- Non-integer ratio, per-resistor segment length: lengthen both ends equally. Dogbones and enclosing layers force embedded dummies (Fig 8.18). Eq 8.27 S = |N/R_N − M/R_M|. Eq 8.28 M = round(N·R_M/R_N). Fig 8.19 (R_M = 200k, R_N = 146k) is best at N = 3, 8, 11, for example R_N = 8×18.25k and R_M = 11×18.18k (23725–23804).
- Partial segments: eq 8.29 S = |(N+1)/(N+k) − (M+1)/(M+j)|, with eq 8.30 M = trunc(R_M/R0), eq 8.31 N = trunc(R_N/R0), eq 8.32 j = R_M/R0 − M and eq 8.33 k = R_N/R0 − N. Fig 8.20 optima are R0 = 10.34k and 10.64k. These are the minima of eq 8.29 with j and k exchanged, not of eq 8.29 as printed (`ref-hastings-99` §2.1). At 10.34k: M = 19, N = 14, j = 0.342, k = 0.120. The book says "R_N consists of 19 sections"; it should read 14, since 14×10.34 + 1.24 = 146k (23808–23859).
- Heat sources: power devices go on a die symmetry axis, and matched arrays are aligned to an axis of the thermal field. The gradient decays roughly exponentially from the power-device edge. One source: power at one end, matched about halfway across the rest; elongation to 1.3–1.5 aspect helps. Two sources: in line on one axis, or one at each end with the matched devices centred and ≥0.5 mm away. Four sources: aspect 1.5:1–2:1. More than 1.5:1 is risky for solder or eutectic mounts on dice longer than 3–4 mm (23878–23942).
- Oxide and poly thickness gradients are handled by dispersion. Diffusion gradients are negligible today (23945–23951).
- Thermoelectric: eq 8.34 E_T = S·ΔT_C, with S = 50–500 µV/°C for Al–Si contacts (silicide does not change it; Cu–Si ≈ Al–Si). 1 °C × 100 µV/°C gives 0.1 mV, which is a 0.4% bipolar mirror mismatch. CC cannot cancel it. Connect series segments alternately (Fig 8.22B) with an even count. Serpentine heads should be close together and oriented oppositely against misalignment (Fig 8.23C) (23960–24051).

### 8.2.8 Mechanical Stress and Package Shift (24055–24759, PDF pp.402–412)
- In-plane stress is described by σx, σy and τxy. Eq 8.35: θp = ½·tan⁻¹(2τxy/(σx−σy)). Eqs 8.36/8.37 give the principal stresses (σx+σy)/2 ± sqrt(((σx−σy)/2)² + τxy²) (24062–24127).
- Package CTE mismatch causes package shift. Table 8.6 CTE in ppm/°C: Si 2.6, SiO2 0.5, Alloy-42 4.5, alumina 6.9–7.5, Cu alloys 16–18, mold 8–35, PCB 12–18. Unfilled epoxy is ~70; 70% filler gives ~30; ≥90% filler gives 8–10. Tg is 120–190 °C and CTE can quadruple above it. Cure is at 150–200 °C (24129–24208).
- Compressive stress is radial, most intense at the die centre, and fades at edges and corners. Shear peaks at the corners and is ~0 on the die axes. Edge anomalies extend about one die thickness (typically 250 µm; 75 µm or 50 µm for thin dice) (24212–24221, 24514–24523).
- Filler stress is random. Pre-2000 crushed silica (15–150 µm) caused mismatch σ up to 2%; modern compounds are 2–3× better. Post-package trim is limited by temperature dependence, stress relaxation (first 1–2 h), moisture swelling (reversed by a >100 °C bake) and thermal hysteresis (24225–24318).
- Mitigations: an elastomeric overcoat ≥ filler diameter, or 10–30 µm (10 µm polyimide gives 3× less random shift, as does 10–15 µm power copper), though it does not reduce gradients. Split devices ≥50–100 µm apart and average them. Build redundant arrays and select the best post-package (24320–24354).
- Die attach: gold eutectic at ~400 °C, solder 185–225 °C and silver sinter >200 °C are rigid and high-stress. Molybdenum and Alloy-42 frames match CTE (24363–24380).
- Eq 8.38: ΔR = R(π_L·σ_L + π_T·σ_T + π_LT·τ_LT). π_LT is 0 for monocrystalline silicon and small for poly. The coefficients shrink above about 1e18 cm⁻³ (24384–24401).
- Table 8.7 (1e-11 Pa⁻¹, (π_L, π_T)). P-type: <100> on (100) (6.6, −1.1); <110> on (100) (71.8, −66.3); (111) any direction (71.8, −22.8); poly (24, −10). N-type: <100> on (100) (−102, 53.4); <110> on (100) (−31.2, −17.6); (111) (−31.2, 29.7); poly (−16, 9.5) (24413–24442).
- Layout X/Y is <110> on (100). N-type diffused resistors are least stress-sensitive along X/Y. P-type diffused resistors are least sensitive at 45°, but designers use X/Y anyway. (111) and poly are orientation-independent. Poly π peaks near 1e19 cm⁻³, so medium- and high-sheet poly is less sensitive. Nichrome: π_L = 8.9e-12 to 1.3e-11 and π_T = 5e-13 to 1e-12 Pa⁻¹. Sichrome is about 10× nichrome (24450–24499).
- Peak centre stress is −100 to −200 MPa for older compounds. Poly at σ_L = σ_T = −110 MPa shifts 4%. Shear does not affect resistors along <100>, <110> or <211> (24554–24575).
- The stress gradient is smallest at the centre, larger at the edges and largest at the corners. Place matched arrays at the centre, or at the middle of a long side if they must be peripheral, never at corners. On (100) the resistance variation has four-fold symmetry, so prefer the die axes near the centre (Fig 8.27) (24579–24600).
- Larger and more elongated dice see more stress. Table 8.8 aspect limits (preferred/maximum): metal can and hermetic ceramic 2:1/any; plastic epoxy ≤10 mm² 1.5:1/3:1, >10 mm² 1.5:1/2:1; solder or eutectic ≤10 mm² 1.5:1/2:1, >10 mm² 1.5:1/1.5:1; CSP 1.5:1/2:1 (24625–24685).
- Single heat source compromise: on the die axis through the heat source, about halfway from its edge to the far die edge (24687–24690).
- Eq 8.39: an L-shaped segment, or equal series H+V segments, gives π = (π_L+π_T)/2 and is orientation-insensitive. Better: two identical CC arrays, one horizontal and one vertical, each resistor half in each. Useful for P-type monocrystalline on (100) and N-type on (111) (24694–24713).
- Eq 8.40: ΔC = C·ξ·(σx+σy). Dry oxide ξ ≈ 5.8e-13 Pa⁻¹ on (100) and 4.6e-13 on (111), orders below the π values. Capacitors can go in less ideal spots but should still be CC, which also cancels oxide-thickness gradients (24717–24759).

### 8.2.9 Electric Fields (24763–25101, PDF pp.412–418)
- Conductivity and body (tank) modulation: base 160 Ω/□ has about 0.1%/V and 2 kΩ/□ HSR several %/V. Matched resistors need equal resistor-to-body voltage. Resistors at different voltages need separate tanks, each tied to its segment's positive end (Fig 8.28). Groups of G segments (G a common factor of the segment counts) share a tank. Example: R1 with 4 segments and R2 with 8 use groups of 4, with R2 either side of R1 (24775–24821).
- Tank biasing belongs to the designer and must be marked on the schematic. Low-sheet base resistors can share one tank (24825–24836).
- Leads over resistors cause conductivity modulation, hydrogenation and noise coupling. Metal-1 over 2 kΩ/□ HSR gives 0.1%/V. The effect depends on ΔV, oxide thickness and overlap area. A lead to a head may cross next to that head. A folded-in jumper crossing every segment identically (Fig 8.29) still causes modulation (24840–24864).
- Faraday shield: a metal layer between the resistor and overlying leads, tied to the circuit reference. It blocks DC and attenuates low frequency, but not digital edges. Static logic lines may cross. Shield overhang 2–3 µm. A common shield is fine unless poly is above ~500 Ω/□ or more than a few volts appear across the array; then use per-segment shields tied to the segments with 2–3 µm overlap, or reroute (24881–24930).
- Substrate noise: a grounded well or tank under deposited devices is a lower shield. A deep-N+ sinker is better (24934–24940).
- Charge spreading: a field plate tied to the resistor's well or tank, per segment (Fig 8.31). Do not channel-stop the gaps (24950–24981).
- Dielectric absorption: 0.1% drift was seen in 2 kΩ/□ HSR under BPSG. Split field plates, with the gap halfway down each segment (Fig 8.32), are recommended for diffused resistors >1 kΩ/□ that must match better than ±0.5%. They are not needed for base resistors or poly below a few kΩ/□ (24999–25091).
- Dielectric relaxation: ONO and TEOS dielectrics relax in ~µs. Grown or LPCVD oxide is preferred for charge-redistribution converters (25094–25101).

### 8.3 Rules for Device Matching, intro (25112–25143, PDF p.418)
- Classes, stated at six-sigma over a 10-year life, −40 to 125 °C junction and plastic packaging: MINIMAL ±1 to ±5% (general analog), MODERATE ±0.1 to ±1% (careful layout, fairly large devices), EXCEPTIONAL ±0.01 to ±0.1% (data converters; huge area; capacitors reach it more easily than resistors) (25121–25133).

### 8.3.1 Rules for Resistor Matching (25148–25428, PDF pp.418–422)
- 25 rules, R1–R25, all extracted in §3 with parameters per class. Exceptional matching generally needs thin film plus laser trim and is rarely compatible with significant power devices (25149–25153).

### 8.3.2 Rules for Capacitor Matching (25432–25617, PDF pp.422–425)
- Oxide capacitors in plastic reach ±0.01% and perhaps ±0.001%, enough for 14–17-bit converters (25433–25435).
- Junction capacitors are unsuitable. MOS capacitors biased several volts into accumulation or inversion are moderate. Poly-poly is limited by top-electrode depletion. Metallic or silicided electrodes are best. The optimal capacitor is a thick LPCVD oxide between metallic electrodes (25448–25459).
- Rules C1–C13 are extracted in §3.

### 8.4 Summary (25623–25643, PDF p.426)
- Decide which components must match and to what accuracy. Then floorplan the power devices and matched components. The most critical sets go on a die axis, far from edges, corners and power devices; with no power devices they still go near the middle. Properly interdigitated arrays reach figures that are blank in the PDF as well ("not given").

### Selected Bibliography (25648–25668)
- Davis (1981, bipolar layout notes), Lane & Wrixon (1989, poly resistors), Pelgrom/Tuinhout/Vertregt (2013, mismatch overview), Ragab & Bayoumi (1998, stress).

### 8.5 Exercises (25673–25756, PDF pp.427–429)
- These drill the six-sigma computation (8.1, 8.3, 8.4), sampling plans (8.2), segmenting R and C (8.5, 8.6), orientation choice (8.7), segmentation-sensitivity plots (8.8), interdigitation patterns for 4:5, 2:7, 1:3:5 and 1:2:4:8 (8.9), piezo shift (8.10), die placement (8.13, 8.15), a shielded CC capacitor array 0.5/1/2/4/8 pF (8.16), overcoat effectiveness (8.17), shield vs dummy-metal block (8.19) and contact redundancy (8.20). They are useful as Philis regression fixtures (see H08-39).

---

## 3. Actionable extraction

Precision classes are abbreviated MIN, MOD and EXC (H08-02). "Deck" means `pdks/*.json`.

### H08-01 Mismatch metric and six-sigma limits
- Kind: metric
- Statement: δ = X1·x2/(X2·x1) − 1 (eq 8.1). The sample mean m_δ (8.2) is systematic mismatch, and the sample SD s_δ (8.3, N−1) is random. Limits are UDL = m_δ + 6s_δ and LDL = m_δ − 6s_δ (8.4, 8.5). A valid sample needs ≥30 units spread across wafers, boat positions and die locations.
- Source: §8.1, eqs 8.1–8.5; reftext 21790–21934; PDF pp.368–370.
- Philis stage: verify, flow (benchmarks)
- Automation recipe: For every matched set, compute δ from a Monte Carlo of the extracted layout: random terms from H08-04 plus systematic terms from the gradient and residue metrics (H08-32, H08-36). Report m_δ and s_δ, and gate on max(|UDL|, |LDL|) ≤ the class band (H08-02). This replaces a pass/fail "CC yes/no" with one number per set.
- Beats hand layout because: every set gets a quantitative ±6σ figure, including the systematic part, instead of a qualitative sign-off.
- Philis status: partial. There is a MOS Pelgrom σ (`backend/annotator/src/emit.rs:67-90`) and cap-array INL/DNL (`kernel/cells/src/cap_array.rs:59-70`). There is no per-set δ with systematic plus random terms for R or C.

### H08-02 Precision class per matched set
- Kind: data-model
- Statement: MIN is ±1 to ±5%, MOD ±0.1 to ±1%, EXC ±0.01 to ±0.1%. All are six-sigma figures over a 10-year life, −40 to 125 °C junction and plastic packaging. Nearly every rule in §8.3 is parameterized by class.
- Source: §8.3 intro; reftext 25121–25133; PDF p.418.
- Philis stage: annotator, flow
- Automation recipe: add `MatchClass {Min, Mod, Exc}` plus `target_pct` to each matched set (Unitization/CentroidGroup). Sources are, in order: a user annotation, then the pattern class (bandgap ratio and ADC/DAC capacitor bank default to EXC; diff pair, mirror and feedback divider to MOD; bias to MIN), then the op-point sensitivity if available. All class-dependent rules (H08-13, -14, -23, -28, -41, -47, -48) read it.
- Beats hand layout because: the effort a human spreads by feel is allocated by computed need, so the area and time go only where the spec demands them.
- Philis status: missing. `Unitization` has no class field (`kernel/analog/src/cell.rs:41-56`). Pelgrom budgets carry an η share, not a class (`backend/annotator/src/emit.rs:40-64`).

### H08-03 Random-share budget and required matched area
- Kind: formula
- Statement: the random part may take ≤75% of the mismatch for MIN, ≤50% for MOD and ≤25% for EXC. With s_δ = k_A/√A and the 6σ criterion, the required area per device is A ≥ (6·k_A/(f·ε))², where f is the random share and ε the class mismatch. The pair consumes 2A. With k_A = 2%·µm (500 Ω/□ poly, or a 10 V poly-poly capacitor) this gives 500 µm² for ±1%, 0.12 mm² for ±0.1% and 46 mm² for ±0.01%. Thin film (k_A < 0.5%·µm) needs 2.9 mm² for ±0.01%. EXC therefore almost always needs trimming. The closed form is derived here and reproduces all four book numbers.
- Source: §8.3.1 R3 and §8.3.2 C3; reftext 25193–25204, 25494–25514; PDF pp.419, 423.
- Philis stage: annotator, cells
- Automation recipe: given a set's class and the deck's k_A (H08-04), compute A_min per device, then the unit count and unit size (H08-41, H08-57). If A_min exceeds a user area cap, emit a "needs trim" diagnostic instead of silently under-sizing. The systematic residual budget left over is (1−f)·ε, which becomes the budget for the gradient, residue and lead-mismatch terms (H08-32, -36, -20, -22).
- Beats hand layout because: sizing and the systematic budget come from one equation per set rather than rule-of-thumb multiples.
- Philis status: missing for R and C. For MOS, σ_rand = A_VT/√(WL) with an η gradient share exists (`backend/annotator/src/emit.rs:67-90`). The deck has no R or C matching coefficient (`pdks/sky130.json` holds only `avt_*`).

### H08-04 Areal and peripheral mismatch model (deck coefficients)
- Kind: deck-requirement / formula
- Statement: s = m·sqrt(k_A²/(2A) + k_P²/(2P)) (8.6). For the pair, s_δ = sqrt(k_A²/(2A1) + k_P²/(2P1) + k_A²/(2A2) + k_P²/(2P2)) (8.8), which reduces to sqrt(k_A²/A + k_P²/P) for equal devices (8.9) and to k_A/√A (8.10). Coefficients differ between materials, processes and fabs.
- Source: §8.2.1, eqs 8.6–8.10; reftext 21973–22045; PDF pp.371–372.
- Philis stage: deck, annotator, verify
- Automation recipe: add per-resistor-model and per-capacitor-model `k_a_pct_um` and `k_p_pct_um05` (units per eq 8.6), with a `_source` field in the same style as `avt_n_mv_um_source`. Use drawn A and P (the book allows that for large devices). If a coefficient is absent, mark the random term "unknown", the way cc.rs does for S/A.
- Beats hand layout because: the perimeter term is evaluated for every unit shape, and so is the choice between square and elongated units.
- Philis status: missing. The deck has `avt_*` and `sheet_tolerance` only (`pdks/sky130.json`).

### H08-05 Capacitor random mismatch and ratio penalty
- Kind: formula / check
- Statement: s_δ = k_C/√C (8.11), and in general s_δ = k_C·sqrt((C1+C2)/(2C1C2)) (8.12). The smaller capacitor dominates. Series-connected capacitors cannot shrink the small one reliably because their parasitics are uncontrolled. Flag large ratios and consider trimming.
- Source: §8.2.1 Capacitors; reftext 22058–22091; PDF p.372.
- Philis stage: annotator, verify
- Automation recipe: for each capacitor set, compute eq 8.12 with the unit count. If the smallest member's term exceeds the random budget (H08-03), grow the unit or raise the diagnostic. Forbid series units unless they are antiparallel (H08-11).
- Beats hand layout because: the dominant member is identified automatically on every set.
- Philis status: missing. Cap-array INL/DNL uses a gradient field, not random k_C (`kernel/cells/src/cap_array.rs:63-66`).

### H08-06 Resistor random mismatch law
- Kind: formula
- Statement: A = (R/R_S)·W² (8.13). s_δ = (k_R/W)·sqrt(1/R) for equal resistors (8.14), and (k_R/W)·sqrt((R1+R2)/(2R1R2)) in general (8.15). Mismatch falls as 1/W and 1/√R, so low-value matched resistors need wider segments.
- Source: §8.2.1 Resistors; reftext 22103–22137; PDF p.373.
- Philis stage: annotator, cells
- Automation recipe: invert eq 8.15 for the minimum W given R, R_S, k_R and the budget. Then take W = max(W_rand, class width floor from H08-41). This feeds `Unitization.unit_w`.
- Beats hand layout because: the width is solved rather than guessed, so resistors are neither over-sized nor under-sized.
- Philis status: missing. Resistor width is `unit_w.max(res_min_width)` (`kernel/cells/src/resistor.rs:81`).

### H08-07 Parallel or series-parallel units for the smaller resistor
- Kind: algorithm
- Statement: making the smaller resistor from N2 parallel segments of N2·R2 each reduces its term, with s_δ ∝ (k_R/W)·sqrt(1/R1 + 1/(N2²·R2)) per eq 8.16 (the printed constant is on PDF p.373). A 10:1 divider with the small resistor as 2×20k in parallel halves the mismatch at far less area than widening both resistors. A 9:1 divider with a 2-parallel × 2-series small resistor has the same value, 4× the area and half the mismatch, and interdigitates as 9:4 AABAABABAABAA.
- Source: §8.2.1 eq 8.16; §8.2.7 Segmenting; reftext 22137–22154, 23716–23721; PDF pp.373, 396.
- Philis stage: annotator, cells
- Automation recipe: for resistor ratio sets with ratio r > 2, enumerate series-parallel (s, p) decompositions of the small member with equal unit value. Score each by eq 8.15/8.16 against the area and the H08-43 sensitivity. Emit the unit count and topology (`SeriesParallel`), which the generator must honour.
- Beats hand layout because: every series-parallel decomposition is enumerated and scored exactly.
- Philis status: missing. `Unitization.series_parallel` exists (`kernel/analog/src/cell.rs:52`). The resistor generator draws every device with the same `n_segments` and never reads `dev_nf`, so unequal members cannot be drawn (`kernel/cells/src/resistor.rs:101`, `:304-311`).

### H08-08 Poly grain model and bamboo width floor
- Kind: formula / deck-requirement
- Statement: k_R = d_g·sqrt(η·R_S) with η ≈ 2 (8.17). It is valid only for W ≫ d_g, and d_g is usually < 0.1 µm. The model holds to widths somewhat below 0.3 µm. Narrower widths give erratic matching (bamboo onset); R4 puts that onset below 0.3 µm.
- Source: §8.2.1 eq 8.17; §8.3.1 R4; reftext 22158–22171, 25210–25214; PDF pp.373, 419.
- Philis stage: deck, cells
- Automation recipe: if the deck lacks k_R but gives R_S and `poly_grain_um`, estimate k_R by eq 8.17 and tag it "estimated". Enforce W ≥ max(0.3 µm, 3·d_g) as a hard floor for matched poly.
- Beats hand layout because: the estimate is used, and labelled as such, even when no fab data exists.
- Philis status: missing.

### H08-09 Equal segment width (width-bias immunity)
- Kind: rule
- Statement: matched resistors use segments of identical width. A wider resistor is built from parallel segments. Example: 2 µm vs 4 µm with −0.1 µm bias gives 0.487 (2.6% error), while 2×2 µm in parallel gives 0.500. R2 applies to all classes. Trimming cannot fix width mismatch because edge material differs (doping, TC, modulation).
- Source: §8.2.2; §8.3.1 R2; reftext 22196–22203, 25187–25192; PDF pp.374, 419.
- Philis stage: annotator, cells, verify
- Automation recipe: unitization forces one `unit_w` per resistor set (already a group scalar). A verify check asserts every rpoly body in a matched set has the same drawn width.
- Beats hand layout because: the equal-width condition is guaranteed structurally rather than by inspection.
- Philis status: implemented structurally (one `unit_w` per group, `kernel/cells/src/builder.rs:224-266`). The limit is that different-W resistors land in different classes (`backend/annotator/src/constraints.rs:30-33`), so a 2 µm/4 µm pair is never converted to parallel units.

### H08-10 Equal segment length (length and contact-bias immunity)
- Kind: rule
- Statement: all segments of a set have the same length, with a larger R built from series segments. Example: 20 µm vs 40 µm with a −0.2 µm contact bias gives 0.503 (0.5%), while 2×20 µm gives 0.500. Equal segments also cancel head resistance and end current crowding (the 2 kΩ/4 kΩ example gives 2.4% from 50 Ω contacts).
- Source: §8.1 (21871–21878); §8.2.2 (22207–22215); §8.3.1 R5 (25218–25224); PDF pp.369, 374, 419.
- Philis stage: cells
- Automation recipe: the unit segment is shared across the set and the value ratio is expressed in unit counts. If the ratio is non-integer, use H08-43/H08-44.
- Beats hand layout because: every unequal-length choice must pass through an explicit sensitivity computation.
- Philis status: partial. Within one device all segments are equal (`kernel/cells/src/resistor.rs:82`, `:313-321`). Across devices of unequal value there is no support (see H08-07).

### H08-11 Capacitor area/periphery equality, non-integer units, antiparallel series
- Kind: formula / rule
- Statement: capacitors match against process bias only if their A/P ratios are equal. Use identical unit capacitors in parallel for integer ratios. For a non-integer ratio, add one non-unit capacitor with L2 = (C2/C1)(1 + sqrt(1 − C1/C2))·W1 and W2 = (C2/C1)(1 − sqrt(1 − C1/C2))·W1 (8.19/8.20, C2 ≥ C1). Alternatively keep W2 = W1 and add a notch or tang. Avoid series units; if unavoidable, build each from two antiparallel halves (top of one to bottom of the other). Unit arrays are customary for MOD and required for EXC. Example: 10×10 vs 10×20 µm with −0.1 µm bias gives 0.497 (0.6%).
- Source: §8.2.2; §8.3.2 C1; reftext 22218–22272, 25470–25484; PDF pp.374–375, 423.
- Philis stage: cells, annotator
- Automation recipe: in cap unitization, split C_i = n_i·C_u + r_i·C_u. If r_i ≠ 0, generate the residual unit from eqs 8.19/8.20 (or a notched unit). Check that A/P equals the unit's within the grid snap. For series topologies, emit antiparallel unit pairs.
- Beats hand layout because: the exact A/P-matched residual is computed and snapped to grid in every case.
- Philis status: partial. Binary banks use identical units (`kernel/cells/src/cap_array.rs:1-17`). There is no residual unit for non-integer ratios, and `capacitor.rs` draws one merged plate per device (`kernel/cells/src/capacitor.rs:1-2`).

### H08-12 Iso-dense print bias threshold for dummy width
- Kind: formula
- Statement: iso-dense bias is negligible for widths and spaces > w_min = λ/NA (8.21). For i-line with NA 0.6 that is ≈0.6 µm. It becomes significant below about 2× the minimum feature. Dummies need width ≥ w_min only.
- Source: §8.2.3 Photolithographic; reftext 22294–22318; PDF p.375.
- Philis stage: deck, cells
- Automation recipe: add `litho_wmin_nm` to the deck, defaulting to 2× the layer's min width. The MIN-class dummy width is max(min_width, litho_wmin). MOD and EXC use full width (H08-13).
- Beats hand layout because: dummy width comes from a stated lithography number instead of habit.
- Philis status: missing. Dummies are full body width (`kernel/cells/src/resistor.rs:162-187`).

### H08-13 Resistor array dummies by class
- Kind: rule
- Statement: MIN diffused or implanted resistors need no dummy. MIN deposited resistors need one min-width dummy at each end. MOD (any type) needs one full-width dummy at each end. EXC deposited resistors need multiple dummies spanning 5–10 µm (§8.2.3 says ≥10 µm, because etch effects extend ≥10 µm). All dummy-to-active spacings equal the active pitch. Body ends extend past the active region (measured from the inner edge of the innermost contact) by ≥3× the min drawn width for MOD and ≥5× for EXC (≥10 µm in §8.2.3). Partial segments extend to align with full ones. Diffused dummies are ≥3–4× junction depth wide (6–8 µm for a 2 µm junction), with heads and contacts like the actives, biased to the enclosing region.
- Source: §8.2.3; §8.3.1 R9; reftext 22312–22347, 22431–22438, 22514–22537, 25263–25273; PDF pp.375–378, 420.
- Philis stage: cells, verify
- Automation recipe: the generator reads the class. MIN gets one dummy of width max(w_min, litho), MOD one full-width dummy, EXC k = ceil(10 µm / pitch) dummies. Dummy spacing is the segment gap. Extend the body past the active region by 3× or 5× the min width. Verify: each array's end segments have a same-width neighbour at the array pitch and the dummy span meets the class.
- Beats hand layout because: the dummy count is computed from the class-required span (≥10 µm), which human layouts routinely shortcut to one dummy.
- Philis status: partial. There is one full-width poly dummy at each end, tied to GND, whenever n_dev > 1 or `dummy_required` is set (`kernel/cells/src/resistor.rs:162-187`). There is no class-dependent count or span and no body-extension rule.

### H08-14 Capacitor array dummy ring by class
- Kind: rule
- Statement: every arrayed matched capacitor gets dummies on all four sides. MOD under a shield needs dummy width ≤3× the minimum. With no shield, fringing reaches 10–30 µm, so dummies must be much wider. EXC needs exact copies of the unit on all sides. Dummy-to-active spacing equals the active spacing. Both dummy electrodes are tied to a node; with a common low-Z bottom plate, extend it under the dummies and tie their tops to the same node. The book's Fig 8.5 array has 6 active and 14 dummy units. Dummy rows and columns need only span the etch penetration (≤10 µm if measured).
- Source: §8.2.3 (22441–22482); §8.3.2 C7 (25550–25561); PDF pp.376–377, 424.
- Philis stage: cells
- Automation recipe: apply the ring to every capacitor unitization with ≥2 members, not only binary banks. Width comes from the class and shield presence. Ties go to the bottom-plate node if it is low-Z (H08-25), else to a dedicated GND pin.
- Beats hand layout because: the ring is guaranteed on every array and sized to the class.
- Philis status: partial. There is a ring of unit-copy dummies with shorted plates tied to C0's bottom plate or a GND bus for cap arrays (`kernel/cells/src/cap_array.rs:6-8`, `:402-404`, `:519-521`, `:556-557`). `capacitor.rs` (merged plates) has no dummies.

### H08-15 Dummy tie-node selection
- Kind: heuristic
- Statement: connect dummies, since floating poly can charge up and modulate its neighbours. Never tie them to noisy nodes, or to nodes that their capacitance would disturb. For diffused dummies, use the enclosing region's bias. For capacitor dummies, use the shield or low-Z bottom-plate node. Consult the designer.
- Source: §8.2.3; reftext 22340–22347, 22471–22482, 22535–22537; PDF pp.375–377.
- Philis stage: annotator, cells
- Automation recipe: the netrole classifier ranks candidate nets by quiet, low-Z reference (analog ground, then a quiet supply). A dummy pin is `GND` by default. The annotator resolves it to the quietest reference net of the block. Clock and digital nets are never chosen.
- Beats hand layout because: the tie net is chosen by computed net quietness rather than proximity.
- Philis status: partial. Dummies expose a `GND` pin (`kernel/cells/src/resistor.rs:187`; `kernel/cells/src/cap_array.rs:557`). The tie net is not selected by net quietness.

### H08-16 Same orientation, H-V bias and piezo axis
- Kind: rule
- Statement: all matched resistors share one orientation even when they are not adjacent (H-V lithography bias and stress). This applies from MIN up (R6). Choose the axis by material. N-type diffused on (100) is best along X/Y (<110>). P-type diffused on (100) is best at 45° (<100>) but usually drawn X/Y. Poly, (111) and thin film are orientation-independent, but R6 still requires one common orientation.
- Source: §8.2.3 (22351–22354); §8.2.8 (24450–24493); §8.3.1 R6 (25225–25231); PDF pp.375, 406–407, 419.
- Philis stage: dp, verify
- Automation recipe: treat every matched set, including sets drawn as separate cells, as orientation-locked. dp may rotate the set only as a whole, or not at all. Verify that every body's long axis is parallel across the set and that the current direction is balanced (H08-45).
- Beats hand layout because: the orientation invariant is enforced on every move, not only at review.
- Philis status: partial. dp never rotates a device in a multi-device group (`backend/dp/src/lib.rs:555-560`). Matched resistors are not grouped by the annotator, so they arrive as singleton cells and stay rotatable (`frontend/library/src/cellgen.rs:549-575`).

### H08-17 Topography and photoresist-flow keep-away
- Kind: rule
- Statement: tall prior topography, such as a poly-poly capacitor, perturbs resist thickness. MOS matching is affected within about 30 µm. R25: space matched poly resistors from poly-poly capacitors by at least the poly-2 width measured along the line from the resistor through the capacitor.
- Source: §8.2.3 (22362–22368); §8.3.1 R25 (25422–25428); PDF pp.375, 422.
- Philis stage: gp, dp, verify
- Automation recipe: tag large poly-poly or MIM plates as "topography" cells. The placement rule is gap(matched_poly_R, cap) ≥ that cap's extent along the joining line, or 30 µm for MOS sets. Use a hard constraint for MOD/EXC and a cost for MIN.
- Beats hand layout because: the distance is checked for every pair of matched set and topography cell.
- Philis status: missing.

### H08-18 Foreign diffusion spacing and sinker placement
- Kind: rule
- Statement: diffusion tails interact. For MOD/EXC diffused or implanted matched resistors, keep other diffusions ≥150% of the minimum spacing (R24). MIN may use the minimum spacing. Put long-tail diffusions (deep-N+ sinker) behind a head, not beside the body. Serpentine heads extend past the array and turn spacings are equal.
- Source: §8.2.3 Dopant Interactions (22498–22565); §8.3.1 R24 (25417–25421); PDF pp.377–379, 422.
- Philis stage: cells, gp, verify
- Automation recipe: in the deck, add a spacing multiplier to diffused-resistor bodies vs foreign diffusion, applied as a cell-halo keep-out scaled by class.
- Beats hand layout because: the halo is applied uniformly to every foreign diffusion.
- Philis status: missing for resistors. MOS WPE and OSE environment checks exist (`kernel/analog/src/placement/environment.rs:1-30`).

### H08-19 Well-proximity clearance for matched diffusions
- Kind: rule
- Statement: place matched diffusions ≥2–3 µm inside drawn well edges. Retrograde-well implant ions scatter >1 µm.
- Source: §8.2.3 WPE; reftext 22569–22575; PDF p.379.
- Philis stage: cells, verify
- Automation recipe: reuse `Surroundings.wpe_min_nm` for diffused resistors and diodes in wells.
- Beats hand layout because: the same clearance is checked on every device kind, not only transistors.
- Philis status: partial. It exists for MOS (`kernel/analog/src/placement/environment.rs:14-23`) and not for passives.

### H08-20 Ratiometric jumper and interconnect resistance
- Kind: rule / metric / algorithm
- Statement: jumper R adds to segment R. A 100 µm, 0.5 µm-wide, 50 mΩ/□ jumper adds 1% to a 1 kΩ segment, so this matters for segments ≤1 kΩ. Each jumper's R should be proportional to its segment's R, so equal segments get equal jumpers. Lengthen short jumpers with jogs, widen long ones, and insert vias in every jumper if any has a via, using multiple vias. R23 (EXC) requires the contribution of each metal layer and each via type to scale with the resistor ratio, with the via contribution a small fraction of the mismatch.
- Source: §8.2.4 (22590–22632, Fig 8.8); §8.3.1 R23 (25405–25416); PDF pp.380–381, 422.
- Philis stage: cells, dr, verify
- Automation recipe: metric per resistor = Σ_layer R_layer and Σ_viatype n_via·R_via along each series path, from the extracted internal routing. Check that |R_route(A)/R_route(B) − R_A/R_B| ≤ ε_sys·share and R_route ≪ allowed error·R. Generator: all inter-segment jumpers use the same construction (same layer, same length and via count), padded with jogs when needed.
- Beats hand layout because: every jumper's resistance is extracted and equalized, which by hand is done approximately at best.
- Philis status: missing. The resistor generator mixes li jumpers for adjacent columns and met1 plus two mcon for non-adjacent ones (`kernel/cells/src/resistor.rs:137-150`), so interdigitated devices get unequal jumper R. The cap array measures route length and via count per slot (`kernel/cells/src/cap_array.rs:505-546`, `route_spread` at `:70`).

### H08-21 Current-carrying leads and Kelvin taps
- Kind: rule
- Statement: leads carrying current into a matched array are short, direct and widened. Inspect them for IR drop injected by other circuitry. Kelvin connections are needed for EXC, and small sense currents must be accounted for.
- Source: §8.2.4 (22633–22636); §8.3.1 R23 (25411–25416); PDF pp.381, 422.
- Philis stage: gr, dr, verify
- Automation recipe: for resistor sets with op-point current, compute IR on each terminal route and require ΔV_lead/V_R ≤ the systematic budget. For EXC, route force and sense branches separately from the terminal (star at the pin).
- Beats hand layout because: IR drop is computed on every terminal route rather than eyeballed.
- Philis status: missing (see docs/LAYOUT-FUNDAMENTALS.md rows 41 and 55). There is an IR rule for routes (`kernel/analog/src/routing/ir.rs`) but no per-member ratiometric check.

### H08-22 Lead-capacitance matching per unit capacitor
- Kind: rule / algorithm
- Statement: lead C adds directly. A 1 µm metal is ≈0.13 fF/µm, so 100 µm is 13 fF = 1.3% of 1 pF. Make every unit's lead C equal: same lead width; spacing to other metal ≥2 µm, or ≥2–3× the ILD thickness per C10, else flank each lead with shield lines; if one lead crosses a metal, all do; exclude dummy metal; equalize with jogs or dead-end branches; verify by back-annotation. Required for MOD and EXC.
- Source: §8.2.4 (22640–22670, Fig 8.9); §8.3.2 C10 (25584–25595); PDF pp.381, 425.
- Philis stage: cells, dr, verify
- Automation recipe: extract C_lead per unit or per member from PEX. Check |C_lead(i)/n_i − C_lead(j)/n_j| ≤ ε·C_u. Fix in the generator (or dr) by adding dead-end stubs of computed length to short leads, which never lengthens the long lead.
- Beats hand layout because: stub lengths are computed from extracted capacitance rather than estimated.
- Philis status: partial. The cap array measures route length and via count and reports `route_spread` (`kernel/cells/src/cap_array.rs:67-70`, `:505-546`). It does not equalize, and nothing covers general capacitors.

### H08-23 Electrostatic shield over matched capacitors
- Kind: rule
- Statement: all MOD and EXC capacitors are shielded. The shield contains fringing (so the dummy ring can shrink), allows leads over the array and blocks outside fields. With no dummies, the shield extends 3–5 µm beyond the array. Tie it to a low-noise, low-Z node such as analog ground. Never route digital or noisy lines over it; route them ≥3–4 µm around the shielded area. A shield raises lead C but makes it predictable.
- Source: §8.2.4 (22671–22685); §8.3.2 C8 (25562–25570); PDF pp.381–382, 424.
- Philis stage: cells, gr, dr
- Automation recipe: generator option "top shield": a plate one metal above the top electrode, overhang 3–5 µm, pinned to a `SHIELD` net that the annotator binds to the quietest reference. gr and dr give digital/clock nets a hard keep-out of 3–4 µm around the shield polygon.
- Beats hand layout because: the shield and the keep-out around it are generated together and cannot be forgotten.
- Philis status: missing. The routing `Shield` is same-layer wire shielding only (`kernel/analog/src/routing/shield.rs:18-19`, "no top/bottom plates").

### H08-24 Same underlayer: moat-edge setback, no routing under capacitors
- Kind: rule
- Statement: matched deposited capacitors (C5) and poly resistors (R17) sit over the same field oxide. Poly over N-well vs P-well differs by >±1% without CMP. Never cross oxide steps. Keep more accurate resistors ≥5 µm and MOD capacitors ≥5–10 µm from moat edges; LOCOS beaks taper over microns and STI stresses radiate microns. For more accurate capacitors, nothing may be routed beneath them: remove the lower metal and poly or make them a contiguous solid plate.
- Source: §8.3.1 R17 (25351–25360); §8.3.2 C5 (25527–25539); PDF pp.421, 424.
- Philis stage: cells, gp, gr, verify
- Automation recipe: (a) a verify check that every member of a set has identical underlayers (the same well or none, no active edge within the setback). (b) gr/dr block all layers below a capacitor array's bottom plate within the array bbox, or the generator draws a solid tied plate there.
- Beats hand layout because: underlayer identity is checked geometrically for every member of every set.
- Philis status: missing. Nothing may be drawn between stacked plates, but the router is not told this (`kernel/cells/src/capacitor.rs:24-26`).

### H08-25 Bottom plate to low-Z node, well shield under the capacitor
- Kind: rule
- Statement: the bottom plate has a large parasitic to the substrate. Tie it to a low-Z node such as analog ground, or put an N-well (NBL plus deep-N+ in BiCMOS) under the capacitor as a shield. This matters most with switching-converter substrate noise. The same applies to deposited resistors: a grounded well or tank below acts as a lower shield.
- Source: §8.2.9 (24934–24940); §8.3.2 C6 (25540–25549); PDF pp.414, 424.
- Philis stage: annotator, cells
- Automation recipe: the netrole classifier decides which plate net is low-Z. The generator orients the capacitor so that the plate faces the substrate, else it adds an nwell-plus-tap shield under the plate tied to the quiet net.
- Beats hand layout because: every capacitor's orientation is decided from classified net impedance.
- Philis status: missing (docs/LAYOUT-FUNDAMENTALS.md row 30). Pins P and N carry no role (`kernel/cells/src/capacitor.rs:37`).

### H08-26 NBL shadow overlap (bipolar and BCD decks only)
- Kind: deck-requirement / rule
- Statement: the NBL shadow must not intersect EXC diffused resistors or MOD HSR. If the shift direction is unknown, overlap NBL on all sides. If its magnitude is unknown, overlap by ≥150% of the maximum epi thickness. STI processes are exempt.
- Source: §8.2.5 (22750–22790); §8.3.1 R18 (25361–25366); PDF pp.383–384, 421.
- Philis stage: deck, cells
- Automation recipe: deck fields `nbl_shift_nm` and `nbl_shift_dir`. Apply only if the deck declares a patterned-NBL process.
- Beats hand layout because: the rule cannot be forgotten on a process that needs it.
- Philis status: missing. It does not apply to the current CMOS STI decks.

### H08-27 Hydrogen-gettering control: metal over poly resistors
- Kind: rule
- Statement: metal over or near poly resistors getters hydrogen (mismatches up to 10%, with gradients microns from metal edges). Prefer folded-out jumpers over folded-in ones. Alternatively cover the array with a met1 plate, exposing only the heads, and fold met2 jumpers in over it (a met2 plate is less effective). Without a plate: no leads over matched segments, none even between them, and dummy-metal blocked on all layers. EXC poly (R21) needs a met2 field plate extending ≥10 µm beyond the active region in every direction, with no met1 across the active area beneath it. MOD poly needs a plate over as much of the body as possible, or all metal, including dummy fill, blocked from crossing active bodies.
- Source: §8.2.6 (22850–22925, Fig 8.12); §8.3.1 R21 (25392–25398); PDF pp.384–385, 422.
- Philis stage: cells, gr, dr, flow (fill)
- Automation recipe: per poly resistor set, the metal-over-body policy by class is {MIN: allowed per H08-28; MOD: plate or hard block; EXC: met2 plate with 10 µm overhang and a hard met1 block}. Fill already keeps out of matched cells. Extend the hard block to all routing layers over resistor bodies.
- Beats hand layout because: the chosen policy is enforced on all metal layers, including dummy fill that a human may never see.
- Philis status: partial. Fill keeps out of matched cells entirely (`frontend/library/src/fill.rs:16-17`). Jumpers run on li and met1 over heads, not bodies (`kernel/cells/src/resistor.rs:137-150`). There is no plate option and no hard route block over bodies (gr has only a soft cost, H08-28).

### H08-28 Leads over resistors: keep-out policy by class, sheet R and signal class
- Kind: rule
- Statement: conductivity modulation, hydrogenation and noise coupling all come from leads over resistors. Metal-1 over 2 kΩ/□ HSR gives 0.1%/V. The effect depends on ΔV, oxide thickness and overlap area. MIN resistors ≤500 Ω/□ may have unconnected low-frequency analog or static digital leads over them; never switching digital or HF analog. MOD: no unconnected lead over, and connected leads only if unavoidable, unless a field plate tied to low-Z intervenes. A lead may cross beside the head it connects to. Static logic lines may cross a shield; switching digital never, even shielded.
- Source: §8.2.9 (24840–24864, 24881–24898); §8.3.1 R21 (25382–25398); PDF pp.412–413, 422.
- Philis stage: gr, dr, verify
- Automation recipe: build a per-cell keep-out polygon over resistor bodies (from `Unit` rects). Net classes from netrole are digital-switching, digital-static, HF-analog, LF-analog and own-net. The cost matrix is {switching or HF over any matched resistor: forbidden; foreign net over MOD/EXC: forbidden unless shielded; foreign LF or static net over MIN ≤500 Ω/□: allowed at cost; own net: allowed only within the head region}.
- Beats hand layout because: the router applies this to every net and track, which is the rule most often broken by late ECO routing.
- Philis status: partial. gr charges a soft `KEEPOUT_COST` = 2.0 per node over a foreign matched cell (`backend/gr/src/lib.rs:748-757`, `:938-940`; dr fills it at `backend/dr/src/lib.rs:480-495`). There is no class or net-type policy, no hard block and no unit-level (body-only) mask.

### H08-29 Faraday shield over resistors; segmented shields
- Kind: rule
- Statement: a metal shield between the resistor and overlying leads, tied to the circuit reference node, blocks DC and attenuates LF but not digital edges. Overhang 2–3 µm. One common shield is fine unless poly sheet R is above ~500 Ω/□ or more than a few volts appear across the array; then use per-segment shields tied to each segment with 2–3 µm overlap, or reroute. Fig 8.30 shows the shield and dummies tied to ground.
- Source: §8.2.9; reftext 24881–24930; PDF pp.413–414.
- Philis stage: cells
- Automation recipe: a generator variant `shielded` draws a met1 plate over bodies with 2–3 µm overhang, heads exposed, pin `SHIELD`. When R_S > 500 Ω/□ or V_span > 3 V (from op-point), emit segmented shields per segment instead.
- Beats hand layout because: shield topology is chosen from computed sheet R and voltage span.
- Philis status: missing.

### H08-30 Field plates and split field plates (diffused or implanted resistors)
- Kind: rule
- Statement: field-plate any matched diffused or implanted resistor operating above 50% of the uppermost metal's thick-field threshold. Also field-plate every MOD one with sheet R ≥500 Ω/□ (charge spreading drift) and every EXC one. The plate ties to the tank or well containing the resistor; per-segment plates; gaps not channel-stopped. Split plates, with the gap halfway down each segment, are recommended for diffused R >1 kΩ/□ matching better than ±0.5% (dielectric absorption). They are unnecessary for base or poly below a few kΩ/□.
- Source: §8.2.9 Charge Spreading and Dielectric Absorption (24950–25091); §8.3.1 R20 (25372–25381); PDF pp.414–417, 421–422.
- Philis stage: cells, deck
- Automation recipe: applies only to decks with diffused or implanted resistor models (`res_implant` exists in sky130, gf180 and ihp decks). Generator variant with per-segment plates, split at mid-length when the thresholds hold.
- Beats hand layout because: the thresholds are evaluated for every diffused resistor from deck and op-point data.
- Philis status: missing.

### H08-31 Equal body (tank) bias and common-factor tank grouping
- Kind: rule / algorithm
- Statement: body modulation is about 0.1%/V for 160 Ω/□ base and several %/V for HSR. Matched resistors need equal resistor-to-body voltage. Resistors at different voltages get separate tanks, each tied to the positive end of its segment. With segment counts N1, N2, …, use groups of G segments per tank, where G is a common factor. Example: 4 and 8 segments give groups of 4, placed with R2's two groups either side of R1 to form a CC arrangement. Low-sheet material can share one tank.
- Source: §8.2.9 Voltage Modulation; reftext 24783–24836; PDF pp.412–413.
- Philis stage: annotator, cells
- Automation recipe: from the op-point, compute each segment's (V_body − V_segment). If the spread exceeds budget/(modulation coefficient), split into per-group wells. G = gcd of the segment counts (or the largest common factor meeting the budget). Order the groups CC. Needs a deck `body_mod_ppm_per_v` per model.
- Beats hand layout because: G and the group order are derived from the op-point voltages rather than decided by hand.
- Philis status: missing.

### H08-32 Thermal gradient mismatch metric (centroid separation)
- Kind: metric / formula
- Statement: ΔR = α·R·d_CC·∇T_CC (8.23), where α is the TC, d_CC the centroid separation and ∇T_CC the gradient along the centroid axis. Four levers: lower TC, lower gradient, separation perpendicular to the gradient, and d_CC → 0 (CC). It is valid under four assumptions: planar region, uniform contribution, linear in T, linear T(x).
- Source: §8.2.7 Centroids; reftext 23081–23139; PDF pp.387–389.
- Philis stage: dp, verify
- Automation recipe: for each matched set, compute each member's centroid from its units (eq 8.24/8.25, H08-33). Evaluate ∇T at the set centroid from the thermal field. Then Δ/R = α·(∇T · (c_A − c_B)). Compare against the systematic budget. This replaces "ΔT between cell centres".
- Beats hand layout because: gradient-times-offset is computed for every set under the actual power map.
- Philis status: partial. `ThermalGradient` prices |ΔT| between two targets at point centres, and a group reads as its hottest member (`kernel/analog/src/placement/thermal.rs:7-35`; `kernel/core/src/thermal.rs:57`). It is not unit-centroid-based and does not use TC.

### H08-33 Electrically weighted centroid of arbitrary segment networks
- Kind: algorithm
- Statement: X_R = Σ(∂R/∂R_i·X_i)/Σ(∂R/∂R_i), and likewise for Y (8.24/8.25). The weights are sensitivities, so series and parallel segments weigh differently.
- Source: §8.2.7 CC; reftext 23277–23290; PDF p.390.
- Philis stage: cells, dp, verify
- Automation recipe: `Unit.weight` should be ∂R/∂R_i rather than area: 1 for a segment in a pure series string, (R/R_i)² for a parallel branch, and the product of the factors along the path for nested series-parallel networks. Compute it once per unitization from the series-parallel topology.
- Beats hand layout because: exact centroids of mixed series-parallel networks are what humans approximate by symmetry.
- Philis status: partial. `CentroidGroup` uses weighted first moments of units (`kernel/analog/src/placement/cc.rs:6-12`, `:47-60`). Resistor units weigh body area (`kernel/cells/src/resistor.rs:113-121`), which is correct only for all-series equal segments.

### H08-34 CC pair-construction rules (pattern synthesis)
- Kind: algorithm
- Statement: build a CC array by placing segment pairs symmetrically about the axis. Each pair (a) belongs to one device, (b) has the same impact on its value (a series and a parallel segment differ) and (c) sits at equal distance from the axis. An optional single central segment is bisected by the axis. Examples: 25 kΩ = 2S + 2P gives SPPS or PSSP (not SSPP); 8.333 kΩ = 2Q ∥ + 3R ∥ gives QRRRQ or RQRQR; interleaved, SPRQRQRPS. If exact CC is impossible, minimize the centroid separation.
- Source: §8.2.7 CC; reftext 23248–23273; PDF p.390.
- Philis stage: cells
- Automation recipe: a pattern generator that takes multiset tokens tagged (device, role). It pairs identical tokens outside-in and allows ≤1 odd token per role at the centre. It is feasible iff each (device, role) count is even, except for at most one odd one overall. Fall back to the min-|Δcentroid| scatter (H08-39).
- Beats hand layout because: every legal pattern can be enumerated and scored, not just the first one found.
- Philis status: partial. `greedy_centroid` fills mirror pairs outside-in by remaining count (`kernel/cells/src/builder.rs:269-290`) without role tags (series vs parallel). The MOS CC order is at `kernel/cells/src/mosfet.rs:722-727`.

### H08-35 The four CC rules as checks (coincidence, symmetry, dispersion, compactness)
- Kind: check
- Statement: (1) COINCIDENCE: the centroids coincide, exactly if possible. This is paramount; reject patterns that fail it (ABBA > ABAB > AABB). (2) SYMMETRY: the array is symmetric about both the horizontal and vertical axes. (3) DISPERSION: split into as many sub-arrays as possible, each obeying (1) and (2), and then the whole array need not. (4) COMPACTNESS: the array and each sub-array are as compact as possible. EXC needs all four; dispersion is vital for large EXC arrays.
- Source: Table 8.4; reftext 23417–23447, 23449–23458, 23521–23531; §8.3.1 R8 (25252–25262); PDF pp.392, 420.
- Philis stage: cells, verify
- Automation recipe: per generated array, compute: coincidence |c_A − c_B|/extent ≤ tol; mirror-symmetry of the token grid about both axes (boolean plus asymmetry count); the number of maximal CC sub-arrays (dispersion index); and the bbox aspect and fill of each sub-array (compactness). Select variants lexicographically in that order, with class thresholds.
- Beats hand layout because: all four rules are scored on every variant, where a human usually checks only coincidence.
- Philis status: partial. Coincidence is checked (`kernel/analog/src/placement/cc.rs:14-18`, `COINCIDENCE_TOL` = 0.01 at `:86`). Symmetry, dispersion and compactness exist only implicitly in cap-array metrics (`kernel/cells/src/cap_array.rs:56-70`).

### H08-36 Quadratic residue metric (dispersion scoring)
- Kind: metric / formula
- Statement: expand the gradient effect about the common centroid: F(x) = c0 + c1(x−a) + c2(x−a)² + … (8.26). CC cancels c0 and c1. The residue is ≈ c2·Σ(x−a)², which is proportional to the square of the (sub-)array span. Splitting AABBBBAA into ABBAABBA quarters each sub-array's residue and halves the total. In 2-D, the number of cross-coupled pairs is the dispersion measure: ABAB/BABA × 2 has 9, ABBA/BAAB × 2 has 4.
- Source: §8.2.7; reftext 23462–23499, 23529–23531; PDF pp.392–393.
- Philis stage: cells, verify
- Automation recipe: for each member i, Q_i = Σ_u w_u·|r_u − r_c|² / Σ w_u, with r_c the array centroid. The residue metric is max_{i,j}|Q_i − Q_j| (µm²) times the deck quadratic coefficient if known, else reported raw. Add it to the variant score for every CC array (R, C, MOS).
- Beats hand layout because: second-order error is scored numerically on every array, and no human computes it.
- Philis status: implemented for capacitor arrays (`quad_um2` at `kernel/cells/src/cap_array.rs:60-62`). Missing for resistor and MOS arrays.

### H08-37 Priority sub-grouping for three or more devices
- Kind: heuristic
- Statement: when arraying three or more devices, give the most important pair its own CC sub-arrays. ABBACCABBA is best for A–B and BBACAACABB is best for A–C.
- Source: §8.2.7; reftext 23501–23503; PDF p.393.
- Philis stage: annotator, cells
- Automation recipe: the annotator emits pairwise matching weights (from sensitivity or class). The pattern generator first builds CC sub-arrays for the highest-weight pair, then places the other members symmetrically around them.
- Beats hand layout because: the choice is driven by a computed importance weight.
- Philis status: missing.

### H08-38 Dispersion vs random-area trade-off
- Kind: heuristic
- Statement: gaps between sections waste area. For a fixed footprint, more sections mean less active area and a higher random term; fewer sections mean a higher gradient term. The optimum is rarely computable. Large thermal or stress gradients favour dispersion. With negligible gradients, the simple ABBA or AB/BA is best, and CC then does no better than non-CC.
- Source: §8.2.7; reftext 23506–23513; PDF p.393.
- Philis stage: cells, flow
- Automation recipe: Philis can compute this optimum. Total σ² = σ_rand²(A_active(n_sections)) + (gradient·residue(n_sections))², where the gradient comes from the thermal field and deck stress data. Choose n_sections by minimizing it per variant. Report the chosen trade-off.
- Beats hand layout because: the book calls this optimum rarely computable by hand, and an automated flow can compute it.
- Philis status: partial. Cap-array variants span Spiral (least dispersion) to Chessboard (most) (`kernel/cells/src/cap_array.rs:28-41`), but selection does not combine the random and gradient terms.

### H08-39 Interdigitation pattern library and arbitrary-ratio scatter
- Kind: data-model / algorithm
- Statement: Table 8.2 (1-D, one symmetry axis): ABBA, ABABBABA, ABABABBABABA; ABCCBA…; ABCDDCBA…; ABA, ABAABA, ABAABAABA; ABABA…; AABAA…; AABCBAA…. Table 8.3 (2-D): AB/BA; ABBA/BAAB; ABA/BAB; ABCCBA/CBAABC; AAB/BAA and extensions. Table 8.5 optimum ratios: 1:1 ABBA; 2:1 ABA; 3:1 AABAABAA; 3:2 ABABA; 4:1 AABAA; 4:3 ABABABA; 5:1 AAABAAAABAAA; 5:2 AABABAA; 5:3 ABAABABAABABAABA; 5:4 ABABABABA. For arbitrary M:N, place M+N segments and scatter each device as uniformly as possible (9:5 gives ABAABAABABAABA); the centroid offset is then ≪ the array width.
- Source: Tables 8.2, 8.3, 8.5; reftext 23207–23245, 23324–23415, 23636–23713; PDF pp.389–392, 395–396.
- Philis stage: cells, verify (tests)
- Automation recipe: (a) Use the tables as golden tests: the generator's chosen pattern for each ratio must score ≥ the book's pattern on H08-35/36. (b) Arbitrary ratios use a min-discrepancy scatter: place A at positions round((k+½)·(M+N)/M) and mirror-balance, then score by coincidence.
- Beats hand layout because: patterns beyond Table 8.5 are optimized and scored instead of eyeballed.
- Philis status: partial. There is `greedy_centroid` (`kernel/cells/src/builder.rs:269-290`), but resistor groups cannot carry unequal counts (H08-07). There are no golden tests from Tables 8.2/8.3/8.5.

### H08-40 Higher-order cancelling arrays
- Kind: heuristic
- Statement: a four-segment array (Fig 8.17A, Lan et al.) cancels the linear term and most of the quadratic one. An octagonal array (Fig 8.17B, He et al.) cancels both, but it is not compact, costs area and needs non-Manhattan geometry, which some rule decks forbid. 2-D CC suffices in practice.
- Source: §8.2.7; reftext 23541–23573; PDF p.394.
- Philis stage: cells
- Automation recipe: keep it Manhattan. Offer the 4-segment rotational pattern as an optional variant only when the quadratic residue (H08-36) dominates and the area budget allows.
- Beats hand layout because: the variant is chosen only when the computed residue justifies its area.
- Philis status: missing (low priority).

### H08-41 Segment dimension floors by class
- Kind: rule
- Statement: width (R4) ≥150% of the min linewidth for MIN, ≥200% for MOD and ≥400% for EXC. Example: 0.5 µm min gives 0.8, 1 and 2 µm. The segmenting text says ≥150% always and ≥300% for highly accurate work; R4 is the rule list. Length (R10) ≥3× the min permitted for MIN, 5× for MOD and 10× for EXC. EXC poly total length is ≥1000× the grain diameter, or 200 µm if unknown, where total means the series path between the nodes of interest. TiSi2 poly needs a minimum area per segment (bimodal outliers at 3–5× the mean in string DACs; not an issue for Co or Ni silicide).
- Source: §8.2.7 Segmenting (23593–23618); §8.3.1 R4 (25205–25214), R10 (25274–25281); PDF pp.395, 419–420.
- Philis stage: cells, annotator
- Automation recipe: unit_w ≥ k_w(class)·w_min and unit_l ≥ k_l(class)·l_min, clamped by H08-06's random requirement. Deck fields `res_min_width` and `res_min_segment` exist; add `silicide_kind` and `res_min_area` if TiSi2.
- Beats hand layout because: the floors are applied uniformly from the class.
- Philis status: partial. There is one fixed floor (`res_min_width` and `res_min_segment`, `kernel/cells/src/resistor.rs:81`, `:296-300`, `:318-320`) and no class multiplier.

### H08-42 Sub-array (bank) count and 2-D arrangement of banks
- Kind: heuristic
- Statement: divide matched arrays into as many CC sub-arrays as segment-length limits allow, or until a sub-array's aspect approaches 1:1. With enough sub-arrays, array them in 2-D, which is more compact and more symmetric and partially suppresses the quadratic residue. Over-long arrays may be split into banks, each obeying all four CC rules. Long segments reduce jumper R but make spindly, gradient-sensitive arrays.
- Source: §8.2.7 (23626–23632); §8.2.4 (22598–22601); §8.3.1 R8 (25259–25262); PDF pp.380, 395, 420.
- Philis stage: cells
- Automation recipe: the resistor variant space is (segments, banks, bank grid). Each bank is a CC sub-array, and the bank grid is itself mirror-symmetric. Score by H08-35/36 and area.
- Beats hand layout because: bank counts and grids are enumerated, not settled after one attempt.
- Philis status: partial. Resistor variants pick segment counts by squareness (`kernel/cells/src/resistor.rs:313-335`) with a single row and no banks.

### H08-43 Non-integer ratio by per-resistor segment length (segmentation sensitivity)
- Kind: algorithm / formula
- Statement: give each resistor its own segment length, adjusting both ends equally so the axis is preserved. S = |N/R_N − M/R_M| (8.27), with M = round(N·R_M/R_N) (8.28). Sweep N and pick minima. Fig 8.19 (200k/146k) is best at N = 3, 8, 11, for example R_N = 8×18.25k and R_M = 11×18.18k. Dogbone segments or enclosing layers (silicide block) force embedded dummies between different lengths (Fig 8.18). R5 allows different lengths for MOD and EXC only with this sensitivity calculation.
- Source: §8.2.7 Segmenting; reftext 23725–23804; §8.3.1 R5; PDF pp.396–397, 419.
- Philis stage: cells, annotator
- Automation recipe: for a non-integer ratio, sweep N in 1..N_max subject to H08-41 floors. Compute M and S, keep the Pareto set (S, area, aspect) and emit the unit lengths per device. Reject the approach when the model's body uses a dogbone head (embedded dummies) unless the area allows.
- Beats hand layout because: the full N sweep is computed where a human builds a one-off spreadsheet.
- Philis status: missing.

### H08-44 Non-integer ratio by partial segments
- Kind: algorithm / formula
- Statement: all segments are R0 except one partial segment per resistor: R_M = M·R0 + j·R0 and R_N = N·R0 + k·R0, with 0 < j, k < 1. S = |(N+1)/(N+k) − (M+1)/(M+j)| (8.29), with M = trunc(R_M/R0), N = trunc(R_N/R0), j = R_M/R0 − M and k = R_N/R0 − N (8.30–8.33). Sweep R0. Fig 8.20 optima for 200k/146k are R0 = 10.34k or 10.64k, giving M = 19, N = 14, j = 0.342 (3.54k) and k = 0.120 (1.24k); the book misprints N as 19. These are the minima of eq 8.29 with j and k exchanged, not of eq 8.29 as printed (`ref-hastings-99` §2.1). Partial segments must be inset equally on both ends (R8) and their ends extended to align with full segments (R9). Avoid partial segments when possible.
- Source: §8.2.7; reftext 23808–23859; §8.3.1 R8, R9; PDF pp.397–398, 420.
- Philis stage: cells
- Automation recipe: a continuous sweep of R0 over the feasible range, where S is piecewise smooth. Take the global minimum, then draw partial segments centred on the array axis.
- Beats hand layout because: the global minimum over R0 is found, which a spreadsheet sweep can miss.
- Philis status: missing.

### H08-45 Thermoelectric cancellation
- Kind: rule / check
- Statement: E_T = S·ΔT_C (8.34), with S = 50–500 µV/°C for metal–Si contacts, independent of silicide and similar for Cu. 1 °C × 100 µV/°C = 0.1 mV, a 0.4% error in a bipolar mirror. CC cannot remove it. Series segments must alternate direction, half each way, with an even count for the best result (R11). For MIN serpentines, put both heads close together; identical serpentines point the same way, and non-identical ones get opposite heads to cancel misalignment (Fig 8.23C).
- Source: §8.2.7 Thermoelectric (23960–24046); §8.3.1 R11 (25282–25290); PDF pp.399–402, 420.
- Philis stage: cells, verify
- Automation recipe: per series string, Σ(direction) = 0. Verify from the `Unit.phi` sign per segment.
- Beats hand layout because: the invariant is guaranteed by construction and checkable per string.
- Philis status: implemented. Segment counts are 1 or even (`kernel/cells/src/resistor.rs:321`), direction alternates per segment (`:109-110`) and is encoded in `Unit.phi` (`:116-121`). A single-segment device (n=1) is the unpaired case R11 tolerates below EXC. There is no verify check.

### H08-46 Array of straight identical strips; serpentine only for MIN
- Kind: rule
- Statement: straight rectangular strips are preferred because they have the fewest geometry-induced errors (R5). Nested serpentines are acceptable only for MIN (R16). MOD needs arrays, even of dogbone segments. EXC widths allow plain rectangles.
- Source: §8.3.1 R5 (25218–25224), R16 (25345–25350); PDF pp.419, 421.
- Philis stage: cells
- Automation recipe: if class ≥ MOD, forbid serpentine variants of the generator for matched sets.
- Beats hand layout because: the class filter is applied to every generated variant.
- Philis status: implemented by construction. The resistor generator draws only straight strips (`kernel/cells/src/resistor.rs:101-160`). The deck has `res_serpentine_aspect` (`pdks/sky130.json`), which is unused for matched sets.

### H08-47 Placement relative to power devices
- Kind: rule
- Statement: power devices go on a die symmetry axis, making it a thermal axis. Matched arrays are aligned to an axis of the thermal field and kept as far as possible from heat sources; the gradient decays roughly exponentially from the power-device edge. One source: power at one end on the axis, matched devices on the same axis about halfway across the remaining die (§8.2.7), or about three-quarters of the way from centre to far edge for EXC (R13). Consider a 2:1 die. Two sources: one at each end with the matched devices centred and ≥0.5 mm separation. EXC needs TC <500 ppm/°C near power, and EXC with a power IC (≥1 W) is rarely feasible. MOD goes on the opposite side from multi-watt devices, interdigitated and on or near the power device's axis. MOD near small power devices must be interdigitated, on its axis, with array width ≤ power-device width and separation ≥1 µm per mW (½–¼ of that acceptable). MIN may sit within a few hundred µm of multi-watt devices if interdigitated. Risk triggers (R7) that require CC even for MOD: die >15 mm², power >250 mW, heat-sunk package, solder/sinter/eutectic mount, within 250 µm of die edge, within 250 µm of a major heat source.
- Source: §8.2.7 Placement (23878–23942); §8.2.8 (24687–24690); §8.3.1 R7 (25232–25245), R13 (25300–25326); PDF pp.398–400, 410, 419–421.
- Philis stage: gp, dp, annotator
- Automation recipe: (a) The annotator marks power devices from op-point P. (b) gp: a separation constraint d(set, power) ≥ max(k_class·1 µm/mW·P, class floor), plus an axis-alignment cost |y_set − y_axis(power)|. (c) The CC-required flag is derived from the R7 triggers, which is computable from die area, total power and the distance-to-edge and distance-to-source figures. (d) Check array width ≤ power-device width when near a small source.
- Beats hand layout because: all R7/R13 conditions are evaluated numerically and simultaneously for every set.
- Philis status: partial. There is a thermal field from per-device power (`kernel/core/src/thermal.rs:1-30`) and a pairwise ΔT rule (`kernel/analog/src/placement/thermal.rs:7-35`). There is no separation-per-mW, axis-alignment or R7-trigger logic.

### H08-48 Stress-gradient-aware die location
- Kind: rule
- Statement: the stress gradient is minimal at the die centre, rises toward the edges and is largest at the corners. Anomalies occur within about one die thickness of the edge (typically 250 µm). MIN resistors stay out of corners and ≥50–100 µm from edges (R12). MOD resistors stay in the interior, or at the middle of a side inset ≥100–250 µm (the middle of a longer side if peripheral). EXC resistors go near the centre. EXC arrays sit on a die symmetry axis, as do MOD monocrystalline resistors; thin film needs only to avoid edges and corners (R14). EXC capacitors sit near the centre and at least several hundred µm from the die sides (C12). On (100) dice the favoured spots are the die axes near the centre (Fig 8.27).
- Source: §8.2.8 (24579–24624); §8.3.1 R12 (25291–25299), R14 (25327–25334); §8.3.2 C12 (25601–25609); §8.4 (25623–25630); PDF pp.408–409, 420–421, 425–426.
- Philis stage: gp, dp, flow (top-level floorplan)
- Automation recipe: when Philis places at die level, add a per-set "stress field" cost proportional to the gradient magnitude of a radial stress model (centre-max compressive, corner shear). Encode hard setbacks by class from die edges and corners and an axis-attraction term for EXC. At block level, export these as floorplan requirements, meaning preferred block location and orientation, to the integrator.
- Beats hand layout because: the setbacks and axis attraction are applied to every set at once, including block-level exports.
- Philis status: missing. gp only seeds devices near the block centre (`backend/gp/src/mechanics.rs:358-365`). There is no die model.

### H08-49 Heat-vs-stress compromise location
- Kind: heuristic
- Statement: for a single heat source, the compromise is on the die axis through the source, about halfway from the source's edge to the far die edge (§8.2.7, §8.2.8), or about ¾ of the way from the die centre to the far edge for EXC on power ICs (R13).
- Source: §8.2.7 (23895–23903); §8.2.8 (24687–24690); §8.3.1 R13 (25304–25307); PDF pp.399, 410, 420.
- Philis stage: gp
- Automation recipe: rather than a fixed fraction, minimize the sum of the stress-gradient term (H08-48) and the thermal term (H08-32) over candidate locations along the axis, and report the chosen fraction.
- Beats hand layout because: the compromise is computed from the actual power map and die aspect instead of taken as ½ or ¾.
- Philis status: missing.

### H08-50 Die aspect ratio limits by package
- Kind: deck-requirement / check
- Statement: Table 8.8 gives preferred/maximum aspect: metal can and hermetic ceramic (epoxy) ≤2:1/any; plastic epoxy ≤10 mm² 1.5:1/3:1, >10 mm² 1.5:1/2:1; solder or eutectic ≤10 mm² 1.5:1/2:1, >10 mm² 1.5:1/1.5:1; CSP 1.5:1/2:1. Also: more than 1.5:1 is risky for solder or eutectic if the long side exceeds 3–4 mm. Larger and more elongated dice have higher stress.
- Source: §8.2.8 Table 8.8 (24625–24685); §8.2.7 (23934–23942); PDF pp.400, 409–410.
- Philis stage: flow
- Automation recipe: top-level only. Take a package descriptor (package type, attach, die area) and emit a warning when the chosen die outline exceeds the limits.
- Beats hand layout because: the check runs on every outline change.
- Philis status: missing. Philis is block-level; this matters only if it grows a die flow.

### H08-51 Piezoresistance shift and orientation selection
- Kind: formula / data-model
- Statement: ΔR = R(π_L·σ_L + π_T·σ_T + π_LT·τ_LT) (8.38). Table 8.7 values (1e-11 Pa⁻¹): P <100>/(100) (6.6, −1.1); P <110>/(100) (71.8, −66.3); P (111) (71.8, −22.8); P poly (24, −10); N <100>/(100) (−102, 53.4); N <110>/(100) (−31.2, −17.6); N (111) (−31.2, 29.7); N poly (−16, 9.5). Nichrome π_L = 0.89–1.3e-11. Layout X/Y is <110> on (100) wafers. Typical older packages give −100 to −200 MPa at the centre; poly at −110 MPa shifts 4%.
- Source: §8.2.8 Effects of Stress Upon Resistors; reftext 24384–24575; PDF pp.406–409.
- Philis stage: deck, verify
- Automation recipe: add deck fields per resistor model: `pi_l`, `pi_t` along layout X and Y, and `wafer_surface` (100)/(111). With a stress map (uniform or radial default), compute each member's ΔR/R. Mismatch is ΔR/R(A) − ΔR/R(B), which is zero for parallel, same-location members and nonzero for rotated or distant ones. This justifies H08-16 quantitatively.
- Beats hand layout because: orientation sensitivity is computed per material instead of assumed.
- Philis status: missing.

### H08-52 Horizontal-plus-vertical split arrays for orientation-sensitive resistors
- Kind: algorithm
- Statement: equal series H and V segments give π = (π_L + π_T)/2 (8.39) and insensitivity to the principal-stress direction. L-shaped segments are hard to arrange in CC. Better: two identical CC arrays, one horizontal and one vertical, with every resistor split equally between them. Worth it for P-type monocrystalline on (100) and N-type on (111).
- Source: §8.2.8; reftext 24694–24713; PDF p.410.
- Philis stage: cells
- Automation recipe: a variant for diffused resistor models flagged orientation-sensitive: two CC sub-arrays rotated 90°, placed as one macro, with each member's segments split 50/50.
- Beats hand layout because: the two-array construction is generated automatically for the models that need it.
- Philis status: missing (applies to diffused or implanted resistor models only).

### H08-53 Piezocapacitance: CC still required for capacitors
- Kind: formula / rule
- Statement: ΔC = C·ξ·(σx + σy) (8.40). ξ is 5.8e-13 Pa⁻¹ for dry oxide on (100) and 4.6e-13 on (111). ξ is independent of layout orientation and orders of magnitude below monocrystalline π. Capacitors tolerate worse die locations but still need CC, which also cancels oxide-thickness gradients. Titanate (high-k) dielectrics are piezoelectric and much more sensitive.
- Source: §8.2.8 Effects of Stress Upon Capacitors (24717–24759); §8.3.2 C12 (25606–25609); PDF pp.410–411, 425.
- Philis stage: cells, deck
- Automation recipe: capacitor sets are always CC-arrayed (C9) regardless of stress. Flag high-k dielectric models as stress-critical, which moves them up the H08-48 priority.
- Beats hand layout because: the stress-critical flag comes from the dielectric model, not memory.
- Philis status: partial. Binary banks are CC (`kernel/cells/src/cap_array.rs`). General matched capacitors are drawn CC only if a ≥2-capacitor unitization exists, which only `dac_banks` creates (`frontend/library/src/cellgen.rs:486-506`, `:756-786`).

### H08-54 Filler-induced random stress: spread-and-average or redundant selection
- Kind: heuristic
- Statement: mold filler particles cause random package shift (σ up to 2% with old crushed silica of 15–150 µm; modern compounds 2–3× better). Layout options: (1) split the device into several copies ≥50–100 µm apart and average them electrically; (2) build redundant matched arrays and pick the best one post-package. Both combine with low-stress mold and an overcoat (≥ filler diameter, 10–30 µm; 10 µm polyimide gives 3× less shift). Overcoats do not reduce gradient stress.
- Source: §8.2.8 Filler-Induced Stress; reftext 24225–24354; PDF pp.403–405.
- Philis stage: annotator, gp (EXC only)
- Automation recipe: an optional EXC-class strategy. The annotator allows a "distributed" topology in which n copies are constrained to a minimum mutual distance of 50–100 µm and a common centroid. This is the opposite of the proximity pull, so it applies only when the user enables package-stress mode.
- Beats hand layout because: the distributed topology can be generated and checked on request instead of hand-placed.
- Philis status: missing (niche).

### H08-55 Self-heating power-density limit in matched resistors
- Kind: rule / check
- Statement: power in matched resistors creates gradients. EXC should dissipate ≤1–2 µW/µm². MOD can dissipate substantially more. For deposited resistors, estimate the array temperature rise (eq 5.7 in ch.5); the resulting resistance shift must be a small fraction of the target mismatch.
- Source: §8.3.1 R22; reftext 25399–25404; PDF p.422.
- Philis stage: annotator, verify
- Automation recipe: from the op-point, P_seg = I²R_seg and density = P/A_body. Check against the class limit. Add each resistor's P to the thermal field as a source so that H08-32 sees self-heating.
- Beats hand layout because: the density is checked on every segment from the operating point.
- Philis status: missing. Thermal sources come from device power in the field (`kernel/core/src/thermal.rs:19-30`), and resistor self-heating is not checked (docs/LAYOUT-FUNDAMENTALS.md row 26).

### H08-56 Same material (model) for matched resistors
- Kind: rule / check
- Statement: resistors of different materials mismatch by ±20% or more, and still by ±5% or more over temperature after single-temperature trim. Even MIN requires the same material. Reference properties: thin film has TC <150 ppm/°C, k_A <0.5%·µm and no modulation. 500 Ω/□ N-poly (4 kÅ) has TC ≈1500 ppm/°C and k_A ≈2%·µm (adequate for MOD). 2 kΩ/□ HSR has TC ≈3000 ppm/°C and up to 2%/V modulation (struggles to reach MOD).
- Source: §8.3.1 R1; reftext 25164–25181; PDF p.418.
- Philis stage: annotator, verify
- Automation recipe: the matched-set key must include the device model. Sets mixing models are reported as unmatched. Model choice (poly vs HSR vs diffused) for a MOD/EXC set is advised from deck TC, k_A and modulation data.
- Beats hand layout because: the model check runs on every set, with material advice from deck data.
- Philis status: partial. The unitization class key is (kind, W, L) without the model (`backend/annotator/src/constraints.rs:30-33`), and the same holds for cap banks keyed by (plate, w, l) (`frontend/library/src/cellgen.rs:764-767`).

### H08-57 Capacitor unit geometry and array shape
- Kind: rule
- Statement: use square units for EXC. 2:1 or 3:1 rectangles are acceptable for MOD. Avoid odd shapes and extra corners, since OPC corners are not repeatable (C2). An optimum unit size exists because small units suffer periphery effects and large ones gradients; reported optima are 20×20 to 50×50 µm squares, used for EXC and often for MOD; MIN gains nothing from division (C3). Place units in compact rows and columns: 32 units as 4×8, or 5×7 with three unused; equal row pitch and equal column pitch, though they may differ from each other (C4). Cross-couple even two equal capacitors (2 sections each), and use more than 2 sections for large capacitors to raise dispersion (C9).
- Source: §8.3.2 C2–C4, C9; reftext 25485–25526, 25571–25583; PDF pp.423–425.
- Philis stage: cells, deck
- Automation recipe: unit = square of side s, with s from H08-03 area and clamped to [20, 50] µm for EXC (deck `cap_unit_side` as default). The grid is the near-square r×c ≥ n, with unused cells becoming dummies. For 2-member sets with equal values, use a cross-coupled 2×2 at minimum.
- Beats hand layout because: the unit side and grid shape are computed from the budget and the class.
- Philis status: partial. The cap array uses a near-square grid and deck `cap_unit_side` (`kernel/cells/src/cap_array.rs:132-136`, `:586-587`). There is no class-based unit sizing.

### H08-58 Dielectric and electrode choice for matched capacitors
- Kind: deck-requirement / heuristic
- Statement: junction capacitors are unsuitable. MOS capacitors biased several volts into accumulation or inversion are good for MOD. Poly-poly is limited by top-plate depletion (TC up to 50–100 ppm/°C). Metal or silicide electrodes are best. Thick homogeneous grown or LPCVD oxide beats thin or composite dielectrics (C11); ONO and TEOS relax in ~µs, which harms charge-redistribution converters. Dielectric absorption hurts long-time-constant integrators.
- Source: §8.2.9 Dielectric Absorption (25042–25101); §8.3.2 intro (25448–25459), C11 (25596–25600), C13 (25610–25617); PDF pp.416–417, 423, 425.
- Philis stage: deck, annotator
- Automation recipe: the deck tags each capacitor model with `dielectric_kind`, `electrode_kind` and `tc_ppm`. The annotator warns when an EXC set uses a composite, poly-electrode or MOS model, or an MOS capacitor biased near threshold (from the op-point).
- Beats hand layout because: the warning is raised from the op-point and model data rather than recalled.
- Philis status: missing.

### H08-59 Capacitors away from power devices (EXC)
- Kind: rule
- Statement: dielectric permittivity and volume changes nearly cancel, but capacitors with poly electrodes have TC up to 50–100 ppm/°C. Keep EXC capacitors, especially poly-electrode ones, away from power devices.
- Source: §8.3.2 C13; reftext 25610–25617; PDF p.425.
- Philis stage: gp
- Automation recipe: reuse H08-47 separation with a capacitor-specific TC (deck `tc_ppm`).
- Beats hand layout because: the separation follows from the model's TC.
- Philis status: missing.

### H08-60 Gate-doping block mask extension for poly resistors
- Kind: rule / deck-requirement
- Statement: where poly resistors use a gate-doping block mask, grain boundaries speed dopant diffusion. MIN uses the rule spacing; MOD and EXC use 150% of it.
- Source: §8.3.1 R19; reftext 25367–25371; PDF p.421.
- Philis stage: cells
- Automation recipe: the resistor recipe (npc/psdm/rpm-style enclosure) takes an enclosure multiplier of 1.5 for class ≥ MOD.
- Beats hand layout because: the multiplier is applied to every matched resistor of that class.
- Philis status: missing. Enclosures follow deck minima (`kernel/cells/src/resistor.rs:199-231`).

### H08-61 Matched-set identification and floorplan-first flow
- Kind: algorithm / flow
- Statement: matched resistor groups are designated by the designer, typically in schematic notes, and all members of a group go in one array (§8.2.7 Segmenting). The layout designer determines which components must match and with what accuracy, then floorplans the power devices and matched components before detailed layout. The most critical sets take the best locations (§8.4, R14).
- Source: §8.2.7 (23578–23582); §8.3.1 R14 (25331–25333); §8.4 (25623–25630); PDF pp.394, 421, 426.
- Philis stage: annotator, gp, flow
- Automation recipe: (a) The annotator recognizes passive matched sets from topology: resistor dividers and ladders (series chains between two reference nets); ratio sets (resistors sharing a node that feed a gain-defining op-amp or feedback path); equal-value resistors in symmetric branches of a matched FET block; capacitor sets sharing a top plate (not only binary). Emit a unitization and CentroidGroup for each. (b) gp ranks sets by class and assigns the best (lowest-gradient) locations first.
- Beats hand layout because: matched passives are found from topology even when the schematic carries no notes.
- Philis status: partial. Binary DAC capacitor banks are recognized (`frontend/library/src/cellgen.rs:486-506`, `:756-786`). Uncovered resistors each become a singleton unitization (`frontend/library/src/cellgen.rs:549-575`). The annotator catalog has no passive patterns (`backend/annotator/src/catalog.rs`; the only capacitor mention is the missing-budget note at `backend/annotator/src/lib.rs:134-135`).

### H08-62 Statistical validation protocol for benchmarks
- Kind: check / flow
- Statement: use ≥30 units spread over several wafers, the front, middle and back of the boat, and random die locations excluding the extreme edge; exclude misprocessed wafers; package as production. Report m_δ, s_δ and the 6σ limits. The 30-unit minimum is widely quoted but has little theoretical basis.
- Source: §8.1; reftext 21813–21843; PDF p.368.
- Philis stage: flow (benchmarks)
- Automation recipe: Monte Carlo in benchmarks draws ≥30 samples, stratified by the gradient-field orientation (θ in 45° steps, as the cap-array INL already does) and by random seeds. Report the 6σ band per set in the bench table.
- Beats hand layout because: the protocol is applied identically on every run.
- Philis status: partial. INL/DNL is taken worst-case over θ in 45° steps (`kernel/cells/src/cap_array.rs:63-66`). There is no per-set 6σ band.

---

## 4. Top-15 priorities for Philis

1. **Matched-passive recognition** (H08-61, H08-56). This is the precondition for every R and C rule. Today resistors fall through as singletons and only binary capacitor banks are grouped. Add divider, ratio and top-plate-set patterns keyed by model.
2. **Precision class plus random-share area budget** (H08-02, H08-03, H08-04). One field turns 25+38 qualitative rules into parameterized constraints. Add deck `k_a` and `k_p` per R and C model.
3. **Ratioed resistor arrays** (H08-07, H08-10, H08-39). The resistor generator ignores `dev_nf`, so no 2:1 or 9:4 divider can be drawn CC. Honour unit counts and series-parallel topology, with Table 8.5 patterns as golden tests.
4. **Four-rule CC scoring plus quadratic residue for resistor (and MOS) arrays** (H08-35, H08-36, H08-33). Reuse cap-array `quad_um2` and cc.rs coincidence. Make unit weights ∂R/∂R_i.
5. **Hard route keep-out over matched resistor and capacitor bodies, by class and net type** (H08-28, H08-27, H08-24). The current gr cost of 2.0 per node is soft. This is the rule most often broken late and the easiest to automate exhaustively.
6. **Ratiometric jumper R and equalized lead C** (H08-20, H08-22). Mixed li and met1 jumpers make interdigitated resistors unequal. Extend `route_spread` into a check and an equalizer (stubs and jogs).
7. **Class-scaled dummies for resistors and capacitors, including general capacitor sets** (H08-13, H08-14, H08-15). There is one fixed dummy today and none for merged-plate capacitors.
8. **Segment and unit dimension floors by class** (H08-41, H08-06, H08-57). These are simple multipliers on existing deck minima, solved against eq 8.14/8.12.
9. **Electrostatic shield plates for capacitor and resistor arrays tied to the quietest reference** (H08-23, H08-29, H08-25). None exist, and the routing `Shield` is same-layer only.
10. **Unit-centroid thermal metric ΔR = αR·d·∇T** (H08-32, H08-47, H08-55). This replaces point-ΔT with the book's equation, adds resistor self-heating sources and adds the R7 CC-required triggers.
11. **Orientation lock for matched passives drawn as separate cells** (H08-16, H08-51). dp locks only multi-device groups, and singleton matched resistors can rotate.
12. **Non-integer ratio optimizers** (H08-43, H08-44). These are closed-form sweeps that humans do in spreadsheets, and a direct win over hand layout.
13. **Dispersion vs random-area optimum** (H08-38, H08-42). The book says the optimum is rarely computable, and Philis has both terms available to compute it.
14. **Same underlayer, moat setback and no routing under capacitors** (H08-24). This is a geometric check plus a gr/dr layer block under the capacitor bbox.
15. **Die-level stress and heat floorplan export** (H08-48, H08-49, H08-47). Block-level Philis should at least emit preferred location and axis requirements for EXC sets. It needs a die model before it can enforce them.
