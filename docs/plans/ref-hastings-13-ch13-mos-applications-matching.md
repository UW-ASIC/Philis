# Hastings ch.13 — Applications of MOS Transistors: power MOS and MOS matching

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson, 2023), Chapter 13 (13.1 Power MOS Transistors, 13.2 Matching MOS Transistors, 13.3 Rules for MOS Transistor Matching, 13.4 Summary, 13.5 Exercises).
Reftext file: `scratchpad/reftext/hastings.txt`, lines **38173–42876** (all read).
PDF: `docs/ref/The Art of Analog Layout 3ed 2023 ... .pdf`. Page convention used below: **PDF p.N = printed book page N−1** (checked: PDF p.644 shows printed 643). The reftext drops most inline math and numeric symbols, so every garbled equation/threshold that matters was re-read from the PDF (pages listed in §1).

---

## 1. Coverage

### Read chunks (Read tool, consecutive, no gaps)
The 2000-line chunk exceeded the tool's token cap, so 1000-line chunks were used:

| chunk | reftext lines |
|---|---|
| 1 | 38173–39172 |
| 2 | 39173–40172 |
| 3 | 40173–41172 |
| 4 | 41173–42172 |
| 5 | 42173–42876 |

PDF pages opened to recover garbled equations/numbers: 644–655, 657–667, 669–671, 686–697, 698–711, 712–717. Not opened (equations left as "not recovered"): PDF p.656 (fig. 13.11/13.12 only), 668, 672–685 (HV/DMOS figures; eqs 13.37–13.39), 718–722 (exercises).

### Every heading in the range (reftext line)
- Chapter 13 Applications of MOS Transistors (38173)
- 13.1 Power MOS Transistors (38195)
  - 13.1.1 Conduction Losses (38232) — On-Resistance (38270); Specific On-Resistance (38350); Computing Metallization Resistance (38380)
  - 13.1.2 Switching Losses (38770) — Gate Propagation Delay (38824)
  - 13.1.3 Safe Operating Area (38949) — Electrical SOA (38994); Electrothermal SOA (39112); The Spirito Effect (39177); Backgate Resistance (39245)
  - 13.1.4 CMOS Power Transistors (39289) — Metallization (39303); Alternate Layouts (39497); SenseFETs (39612)
  - 13.1.5 High-Voltage Transistors (39730) — Drain-Extended Transistors (40045); Field-Gapped Drain-Extended Transistors (40259); DMOS Transistors (40354)
- 13.2 Matching MOS Transistors (40642)
  - 13.2.1 Effects of Geometry upon Matching (40746) — Threshold Voltage Matching (40755); Transconductance Matching (40926); Orientation (40966)
  - 13.2.2 Diffusion, Implantation, Etch, and Trench Effects (41036) — Polysilicon Etch Rate Variation (41051); Diffusion Penetration of Polysilicon (41197); Diffusions near the Channel (41233); Well Proximity Effect (41265); Length of Diffusion Effects (41367); Stringers and Subthreshold Humps (41449); PMOS versus NMOS Transistors (41517)
  - 13.2.3 Bias-Dependent Mismatches (41535) — Channel-Length Modulation (41542); Hot-Carrier Generation (41587); Bias-Temperature Instability (41698)
  - 13.2.4 Hydrogenation (41729) — Dummy Metal and MOS Matching (41807)
  - 13.2.5 Gradients (41859) — Oxide Thickness Gradients (41921); Stress Gradients (41940); Temperature Gradients (42010)
  - 13.2.6 Common-Centroid Layout of MOS Transistors (42090)
- 13.3 Rules for MOS Transistor Matching (42321) — matching classes + rules 1–24
- 13.4 Summary (42674)
- Selected Bibliography (42702)
- 13.5 Exercises (42722)

---

## 2. Section-by-section digest

### Chapter 13 intro (L38173–38191)
- MOS: no gate current, near-ideal switches; enlarged CMOS for low-V high-I, DEMOS for 30–80 V on baseline CMOS, DMOS for high V and high I (L38180–38183).
- MOS match accurately "only if the details of device layout, orientation, and location are all considered" (L38187–38189).

### 13.1 Power MOS Transistors (L38195–38228)
- Bipolar power switches limited by minority-carrier recombination (400–800 V parts ≤100 kHz) and V_CE(sat) ≳ 250 mV (L38196–38205).
- Low-V MOS reach R_DS(on) < 1 mΩ; 400–800 V devices ~0.1 Ω; integrated power devices up to ~100 W / 100 V share a die with analog + logic (L38216–38228).

### 13.1.1 Conduction Losses (L38232–38266)
- Losses split into conduction (frequency-independent) and switching (∝ f) (L38239–38241).
- Eq 13.1 R_DS(on) = V_DS/I_D; eq 13.2 P_D = R_DS(on)·I_D²; 10 mΩ at 10 A dissipates 1 W (L38245–38266; PDF p.644–645).

#### On-Resistance (L38270–38346)
- Eq 13.3–13.5 (Shichman–Hodges linear region): R_ch ≅ 1/(k|V_GS−V_t|); k ∝ T^−1.5 (e⁻) / T^−1.2 (h⁺) (L38271–38299; PDF p.645).
- R_ch TC ≈ 5000 ppm/°C at 25 °C (≈+50% from 25 to 125 °C) (L38299–38303; PDF p.645).
- R_DS(on) = silicon (channel+source+drain) + metallization + package; metallization of a low-V power FET "can easily exceed its silicon resistance" (L38306–38321).
- Bondwires 20–75 µm diameter; dozens used for R and EM; metal/package TC 3000–4000 ppm/°C; total low-V R_DS(on) TC 4000–6000 ppm/°C (L38325–38334; PDF p.645).
- Procedure: simulate at worst-case process/V_GS/T, then add extracted metallization and package R (L38338–38346).

#### Specific On-Resistance (L38350–38376)
- Eq 13.6 R_SP = A_d·R_DS(on) (Ω·mm² or mΩ·mm²), measured with Kelvin connections excluding package (L38350–38371; PDF p.646).
- Metallization does not scale as 1/area: do not extrapolate area by more than 2–3× (L38374–38376).

#### Computing Metallization Resistance (L38380–38766)
- Vertical vs lateral MOSFETs; integrated are lateral; vertical backgrind 50–75 µm (L38381–38397; PDF p.646).
- Pitch limits: eq 13.7 P ≥ L_d + W_C + 2S_PC (silicon); eq 13.8 P ≥ W_C + 2E_MC + S_M1 (M1); eq 13.9 P ≥ W_V + 2E_M1V + S_M1 (via) (L38423–38477; PDF p.647–648).
- Fig 13.2 pattern: M1 per S/D finger, two perpendicular M2 buses; half of each finger has no lateral drop, the half under the opposite bus carries lateral current (L38449–38486).
- Vias directly over contacts need W plugs; with Al contacts vias are staggered between contacts (L38511–38515). Stacked fingers: eq 13.10 R_S12 = R_S1‖R_S2 (L38548–38558; PDF p.649).
- Rule of one-third, eq 13.11 R_M = R_S·W/(3W_M); valid for W ≤ d_pen (±1.5%) or ≤1.8·d_pen (±10%), eq 13.12 d_pen = R_Si·W_M/R_S (R_Si = silicon R of the finger) (L38572–38603; PDF p.650).
- Fig 13.2 fingers: eq 13.13 R_F = W·R_S1/(2W_M); eq 13.14 R_M1 = W·R_S1/(6N·W_M); eq 13.15 d_pen = N·R_Si·W_M/R_S1, compare W/2 (L38606–38652; PDF p.650). Example 50(100/0.7), R_Si 100 mΩ, 1 µm M1 at 50 mΩ/□ → d_pen 100 µm, R_M1 17 mΩ (L38656–38661; PDF p.651).
- Buses: eq 13.16 R = 2NPR_S2/W (one bus); eq 13.17 R_B = 4NPR_S2/(3W) (two buses); eq 13.18 d_pen = (R_Si+R_M1)·W/(4R_S2), compare N·P (L38683–38721; PDF p.651). Example P = 2 µm, 25 mΩ/□ → d_pen 117 µm, buses 100 µm, 33 mΩ; total 150 mΩ = 100 Si + 17 M1 + 33 M2 (L38725–38729; PDF p.652).
- Same-end termination has lowest R for long buses only by crowding current into near fingers; for bus length > ~2·d_pen use opposite-end termination (Fig 13.6B). Terminating both ends of each bus cuts bus R 4× and doubles d_pen (L38732–38753; PDF p.652).
- Non-standard patterns need test structures or FEA (e.g., Silicon Frontline R3D) (L38757–38760).

### 13.1.2 Switching Losses (L38770–38820)
- Eq 13.19 E_D = I_D·V_DS·t_s/2; eq 13.20 P_avg = I_D·V_DS·t_s·f_s with t_s = mean of on/off transitions (L38777–38812; PDF p.652–653).
- 10 A, 5 V, 10 ns at 1 MHz → 0.5 W (L38816–38817).

#### Gate Propagation Delay (L38824–38945)
- Eq 13.21 t_p(f) = k(f)·R·C, k = 1.0 (f=0.90), 1.3 (0.95), 2.0 (0.99); C_gate ≤ 2·C_ox for low-V CMOS (ratio 1.85:1 simulated, 5 V NMOS, 130 Å); f = 0.95 used (L38830–38843; PDF p.653–654).
- Eq 13.22 t_p ≈ 2.6·W²·R_S·ε_ox/t_ox (ε_ox = 3.5·10⁻¹⁷ F/µm) — ∝ W², independent of L; eq 13.23 W_max ≈ √(t_ox·t_pmax/(2.6·R_S·ε_ox)) (L38864–38895; PDF p.654).
- Example: 90 Å oxide, 10 Ω/□ silicided poly, t_pmax ≈ 1 ns → W_max ≈ 90 µm; contacting both finger ends doubles W_max; wider devices split into side-by-side banks (L38897–38925; PDF p.654–655).
- Over-wide gates turn on/off non-uniformly → local switching heat; rarely-switching devices may use wider fingers (L38941–38945).

### 13.1.3 Safe Operating Area (L38949–38992)
- Ideal SOA: V_DS(max) by breakdown, I_D(max) by electromigration, P_D(max) by T_J and package, pulses < ~10 ms above P_D(max) (L38957–38962; PDF p.655).
- Integrated devices lose SOA to electrical and electrothermal limits (L38981–38985).

#### Electrical SOA (L38994–39103)
- Impact ionization → backgate debiasing → parasitic bipolar → runaway in µs–ms (L38995–39004).
- BV_DII: pulsed, V_GS at max backgate current (≈½ max V_GS), point where I_D rises 20% above plateau (ref. at e.g. 2 V) (L39020–39029; PDF p.657).
- If BV_DII < BV_DSS, use BV_DII as rating if the device ever turns off while conducting significant current, even only under faults (L39055–39065; PDF p.658).
- Lower backgate resistance improves electrical SOA; backgate contacts on the ends of each source finger remove field intensification there (L39068–39071).
- Snapback (trigger/sustain); destructive local T 400–600 °C; avalanche energy rating; filamentation fixes (deeper LDD, field relief, adaptive RESURF) (L39075–39103; PDF p.658).

#### Electrothermal SOA (L39112–39173)
- Parasitic-bipolar thermal runaway, hot spot; stable hot spot "almost as objectionable" (L39113–39161).
- Fix: minimize effective backgate resistance via distributed backgate contacts; failures take hundreds of µs vs ns for electrical SOA (L39165–39173).

#### The Spirito Effect (L39177–39241)
- Eq 13.24 drain current rises with T if TC_Vt < TC_k·(V_GS−V_t)/(2V_t) (both TCs negative) — met by compact high-k devices at small overdrive (L39179–39203; PDF p.659).
- Eq 13.25 S = θ_JC·V_DS·dI_D/dT_J; eq 13.26 T_J − T_C = θ_JC·P_D; S > 1 → runaway; θ_JC practically not computable; low thermal R helps (L39204–39236; PDF p.660).

#### Backgate Resistance (L39245–39285)
- Sublayer (P+ substrate under P-epi, or NBL under N-well) suffices if ≤ ~10 Ω/□ (L39253–39258; PDF p.660).
- With sublayer: contact it (large substrate-contact areas; deep-N+ ring around PMOS over NBL, also a hole-blocking guard; extra deep-N+ stripes for large currents) (L39262–39267).
- Without sublayer (isolated P-tank NMOS, N-well PMOS without NBL; retrograde wells insufficient): distributed (if backgate = source and rules allow) or interdigitated backgate contacts (L39270–39285).

### 13.1.4 CMOS Power Transistors (L39289–39299)
- Core and I/O CMOS scale to power devices; concerns: metallization, alternate patterns, senseFETs (L39290–39294).

#### Metallization (L39303–39493)
- Design inputs: max R_DS(on), max continuous T_J, max I_D, lifetime or usage profile (fraction of life at each T/I) (L39309–39311; PDF p.661).
- Process inputs: metal layers, sheet R per layer (use max at max T), EM current densities; without a usage profile assume full current at max T for life, which "often grossly exaggerates" widths (L39315–39324).
- Silicon R at min V_GS, max T_J, worst corner (L39328–39330). Large devices split into sections at bondpads; buses assumed to terminate at pad centre (slightly conservative); total = parallel of sections (L39340–39345; PDF p.661–662).
- Iteration: guess metallization = ⅓ R_DS(on) (less for HV, more for low-V); size gate width; apportion metal (2LM: M1 fingers, M2 buses; 3LM: M1 fingers + M2‖M3 buses, or M1‖M2 fingers + M3 buses); compute eqs 13.14–13.18; iterate; check EM (L39361–39377; PDF p.662).
- Eq 13.27 finger peak current I_max ≅ I_D/N for N > 10, at the finger–bus junction; bus peak at exit or bondpad (L39377–39389; PDF p.662).
- Partial-width buses (Fig 13.16): eq 13.28 R_M1 ≅ (A·R_S12 + B·R_S1)/(3N·W_M); eq 13.29 d_pen = N·R_Si·W_M/R_S12; two EM peak points per finger (L39393–39439; PDF p.663).
- Diagonal device with trapezoidal buses: eq 13.30 R_B ≅ (2NPR_S2/(3D))·ln((C+D)/C); eq 13.31 d_pen = (R_Si+R_M1)(C+D)/(2R_S2); fill wasted space with bypass caps or substrate contacts (L39443–39493; PDF p.664).

#### Alternate Layouts (L39497–39608)
- Waffle, eq 13.32 (W/L)_w/(W/L)_c = (2S_d + 0.55L_d)/(L_d + S_d); better when S_d > 0.45·L_d; L_d = 1, S_d = 3 µm → +64% W/L (61% area) (L39497–39538; PDF p.665).
- Waffle defects: metallization dominates (gain halves if metal is ½ of R), 90° channel bends avalanche early (fillets/chamfers help), no backgate-contact provision (L39553–39573; PDF p.665–666).
- Bent-gate: 135° bends, dense, accepts distributed backgate contacts, diagonal contacts add ballasting; rule of one-third applies; M1 crosses poly (needs CMP for EM) (L39582–39608; PDF p.666).

#### SenseFETs (L39612–39726)
- Sense resistor: circuits resolve ≈ ±1 mV → ≥10 mV drop, 15–50 mV typical; 18 mV at 2 A → 9 mΩ (metal only; wide for EM); bondwire sense needs Au or PCC wire (L39613–39622; PDF p.666).
- Distributed senseFET ideal but hard to route; any deviation from regular gate pattern induces etch variation (L39642–39644; PDF p.667).
- Full-finger senseFET in the middle finger (bisects hot and cool regions); 100 sections → 49:1 (L39648–39653).
- Partial-finger (Fig 13.20): moat openings segment a drain finger; regularly spaced segments replicate temperature and metal debiasing; ~490:1 (1/10 finger); moat-over-poly rule may block it (L39656–39680; PDF p.667).
- Series senseFETs (Fig 13.21) reach large ratios with more gate area (less random mismatch); single dummy gate each side; moat-hole version keeps poly pitch uniform; example 5 × 10% segments in series → 4850:1 (L39687–39720; PDF p.668).
- LDMOS senseFETs need finger pairs; uniform finger spacing matters (L39723–39726).

### 13.1.5 High-Voltage Transistors (L39730–40041)
- Limits: punchthrough, avalanche, hot-carrier injection, all from drain depletion growth (L39749–39751).
- Eq 13.33 w_d ≈ √(2ε_Si·V_DS/(q·N_B)); eq 13.34 E_max ≈ √(2q·N_B·V_DS/ε_Si); E_crit ≈ 3·10⁵ V/cm; HCI occurs below it (L39772–39803; PDF p.669–670).
- Drift region + gate ending short of the junction keeps gate oxide thin (L39811–39819).
- Eq 13.35 R_SPD ≥ 6.0·10⁻⁹·BV_DSS^2.5 (NMOS), eq 13.36 R_SPD ≥ 1.6·10⁻⁸·BV_DSS^2.5 (PMOS), mΩ·mm², area of the drain–backgate junction facing the channel (L39840–39866; PDF p.671).
- RESURF (single/double/triple), dielectric RESURF (STI strips across drift), superjunction; RESURF BV more variable and sensitive to overlying metal/charge — field plate cures (L39881–40021). Eq 13.37 not recovered from PDF.

#### Drain-Extended Transistors (L40045–40245)
- Non-self-aligned drain extension (NSD in N-well); asymmetric DENMOS: only the extended terminal is the HV drain (L40046–40064).
- Poly-to-N-well spacing "must exactly equal the value specified by the device designer": too far → no conduction, too close → gate-oxide overstress (L40096–40104).
- Drawn L includes well outdiffusion → real L shorter, higher gm, short-channel effects (L40108–40111).
- Two gate fingers around one drain cancel poly/well misalignment; use an even number of sections with sources at both array ends (L40114–40136).
- Isolated DENMOS: outer drain fingers merge into the N-well isolation ring (L40140–40142). Symmetric DENMOS: both ends HV, self-aligned, long min drawn L (value garbled, not recovered) (L40159–40165).
- DEPMOS via channel-stop inversion: NChst = (NWell oversized by 3.0) xor Chstop (L40189–40214); shallow P-well in deep N-well; P-well drift in NBL-isolated tank (L40237–40245).

#### Field-Gapped Drain-Extended Transistors (L40259–40350)
- LOCOS field relief preferred (bird's beak); STI gives abrupt on-current transition (L40260–40264).
- Moat-to-well position set by 2-D simulation; tiny layout changes alter the device (L40276–40283).
- Field gap moves peak HCI below the surface; higher R_on; possible filamentation (L40300–40314).
- Field-gapped DEPMOS: NBL dilution by narrow strips + gate field plate (L40317–40322). Drain-centric annular layouts, rounded ends, elongated "sausage" fingers (L40330–40335).

#### DMOS Transistors (L40354–40636)
- DMOS channel = difference of boron and arsenic outdiffusion (self-aligned) (L40363–40449).
- Field-gap dimensions critical — implement exactly as specified (L40464–40467).
- Source-centric elongated fingers with rounded ends; alternating NSD/PSD plugs set backgate R; PSD plugs at finger ends; use the PCell, explode only to remove non-critical parts (e.g., outer drain fingers under a hole-blocking ring) (L40471–40503; PDF p.682).
- Drawn width = DWell perimeter (eqs 13.38/13.39, not recovered) (L40507–40536).
- LSD (source at substrate, no NBL, higher BV) vs HSD/isolated (NBL, lower BV) (L40541–40566). Adaptive RESURF via channel stop (L40575–40580). Separate backgate variant raises bipolar risk (L40589–40597). Graded channel of DMOS allows short channels at high V (L40625–40631).

### 13.2 Matching MOS Transistors (L40642–40742)
- Voltage matching (diff pairs) and current matching (mirrors) need different geometries (L40643–40645).
- Eq 13.40 ΔV_GS ≅ ΔV_t − V_gst1·(Δk/2k₂); eq 13.41 I_D2/I_D1 ≅ (1 − Δk/k₁)(1 + 2ΔV_t/V_gst1) (L40648–40685; PDF p.686).
- Little to gain from V_gst below ~100 mV (subthreshold); short devices waste area on S/D terminations (L40668–40674).
- Eq 13.42 s_V ≅ √(s_Vt² + (V_gst/2 · s_k/k)²); eq 13.43 s_I/I_D ≅ √((s_k/k)² + (2/V_gst · s_Vt)²); treating them as independent slightly overestimates (L40695–40722; PDF p.686–687).
- Vt-dominated: eq 13.44 s_V ≈ s_Vt; eq 13.45 s_I ≈ g_m·s_Vt (L40727–40742; PDF p.687).

### 13.2.1 Effects of Geometry upon Matching (L40746–40751)
- Large devices, long channels, same orientation all improve matching (L40747–40751).

#### Threshold Voltage Matching (L40755–40901)
- Eq 13.46 s_Vt = c_Vt/√(W_e·L_e), c_Vt in mV·µm by regression; assume units within ±6·s_Vt (L40756–40777; PDF p.687).
- Dopant fluctuation dominates; natural (unadjusted) devices match better (L40780–40783).
- c_Vt ≈ 1 mV·µm per nm of gate oxide (150 Å → ~15 mV·µm), rough rule (L40791–40799; PDF p.687–688).
- V_BS = 2 V on a 5 V device raises c_Vt ≈ 30%; avoid body bias on critical pairs (L40810–40813; PDF p.688).
- Eq 13.47 g_m = k·V_gst = √(2I_D·k) = √(2I_D·k′·W/L); eq 13.48 s_I = c_Vt·√(2I_D·k′)/L_e — "voltage matching depends upon area, current matching depends upon length" (L40817–40837; PDF p.688).
- Eq 13.49 s_Vt = c_Vt·√(1/(2W₁L₁) + 1/(2W₂L₂)); build ratioed devices from identical unit sections, series for the smaller, parallel for the larger; 20:1 mirror = 4 units in series vs 5 in parallel (Fig 13.41) (L40840–40864; PDF p.688–689).
- Keep both dimensions ≥ 1 µm to stay on Pelgrom (L40870–40873; PDF p.689). Pocket implants: eq 13.50 s_Vt = c_Vt/√(W_e·min(L_e, L_c)), L_c ≤ a couple of µm; current matching needs L > 5 µm, so use analog-friendly (no-pocket) devices (L40877–40901; PDF p.689).

#### Transconductance Matching (L40926–40962)
- Eq 13.51 s_k/k = c_k/√(W_e·L_e) (c_k in %·µm → divide by 100) (L40927–40938; PDF p.690).
- Eq 13.52 s_k/k = √(c_k²/(W_e·L_e) + c_kp1²/(W_e²·L_e) + c_kp2²/(W_e·L_e²)); edge terms matter below ~1 µm (L40942–40962; PDF p.690).

#### Orientation (L40966–41017)
- Mobility piezo-sensitivity is axis-dependent: rotated devices mismatch by several % in current; off-axis (tilted) wafers up to 5% even for nominally equivalent axes (L40967–40973; PDF p.690).
- Rotating a cell holding one of a matched pair by 90° at top level silently creates a 90° mismatch → keep matched devices in the same cell (L41002–41006; PDF p.691).
- Non-self-aligned channels (asymmetric DEMOS): devices must be superimposable (translation only, no rotation or reflection); even superimposable devices see tilted-implant S/D shift (L41009–41017; PDF p.691).

### 13.2.2 Diffusion, Implantation, Etch, and Trench Effects (L41036–41047)
- Neighbouring structures (poly, diffusions) shift L/W, Vt, k of matched devices (L41037–41047).

#### Polysilicon Etch Rate Variation (L41051–41187)
- Microloading: end gates of an array erode more → shorter L (L41052–41062; PDF p.692).
- Dummies: every active gate must face poly on both sides; dummy-to-active spacing = active-to-active spacing; dummy width = active width; same poly extension beyond moat; gate tied to keep it off (usually to backgate); outer S/D may stay uncontacted, but DRC/LVS may then flag it (L41077–41093; PDF p.692).
- Microloading decays exponentially, seldom significant beyond 3–5 µm → dummy L ≤ 3–5 µm unless active L < 3–5 µm (then dummy L = active L); very accurate matching considers up to 10 µm (multiple dummies for short L) (L41097–41104; PDF p.692).
- Half dummies (single S/D termination) for L > 3–5 µm and less accurate matching; min poly length = moat-over-poly overlap + poly-over-moat overhang; not narrower than 2–3 µm; may let moat/well edges approach the active gate (L41110–41121; PDF p.693).
- Corner rounding ≤ 1–2 µm: extend gates (incl. dummies) 1–2 µm beyond the moat, all identical; poly combs acceptable (except most accurate) if the interconnecting poly lies ≥ 1–2 µm from the moat (L41125–41132; PDF p.693).
- Block PG dummy-poly within 10–15 µm of very accurately matched transistors; no poly there except active and dummy gates (L41147–41154; PDF p.693).
- Poly-0 (double-poly capacitor bottom plate) topography disturbs resist spin: up to 12% mismatch at > 30 µm; place matched transistors ≥ 150 µm from poly-0 (L41158–41187; PDF p.693–694).

#### Diffusion Penetration of Polysilicon (L41197–41206)
- Under-annealed poly depletion or over-annealed dopant penetration along grain boundaries adds Vt mismatch — process-level, no layout remedy (L41198–41206; PDF p.694).

#### Diffusions near the Channel (L41233–41247)
- Deep diffusion tails (deep-N+ sinkers, wells) shift Vt and k: add "several microns" to minimum spacings; keep N-well away from matched NMOS; keep matched PMOS well inside their N-well; with multiple voltage rules add several µm to the lowest-voltage rule unless the actual well voltage needs more (L41234–41247; PDF p.695).

#### Well Proximity Effect (L41265–41356)
- Retrograde MeV well implants scatter off resist edges → heavier doping near well edges (L41266–41285; PDF p.695).
- Effects up to 3–5 µm, worst within 1–2 µm; 5% current mismatch at 1.8 µm spacing, 25% at 0.95 µm (L41302–41306; PDF p.696).
- Well edges ≥ 3–5 µm from matched active gates; end dummies push well edges away (removes end-vs-middle systematic mismatch) but side-edge doping still adds random mismatch → wider devices or move side well edges out (Fig 13.51) (L41314–41322; PDF p.696).
- NBL shadow (non-STI): pattern shift ≤ 150% of epi thickness; enlarge well/NBL so the shadow misses PMOS gates; absent in STI/CMP processes (L41342–41356; PDF p.697).
- LDMOS first-finger effect: outer fingers differ up to 10%; senseFET must not be an outer finger — add dummy fingers (L41359–41363; PDF p.697).

#### Length of Diffusion Effects (L41367–41445)
- STI densification oxide (2.2× Si volume) compresses channels: NMOS k down, PMOS k up by ≥ 10%; Vt shifts ≥ 10 mV; widening S/D (SA/SB) reduces it (L41368–41377; PDF p.697).
- Table 13.1 (10⁻¹¹ Pa⁻¹, 25 °C): ⟨100⟩: PMOS π_L −15, π_T 30; NMOS π_L −90, π_T 40. ⟨110⟩: PMOS 65, −50; NMOS −30, −20 (L41387–41401; PDF p.697).
- End devices see more transverse stress; stretch end moats 3–5 µm; for optimal matching moat ≥ 3–5 µm beyond the last active transistor; if L ≥ 3–5 µm one dummy suffices, else two or more; less accurate: 2–3 µm (L41411–41422; PDF p.697–698).
- OD-to-OD stress (trench width) — same fixes (L41442–41445; PDF p.698).

#### Stringers and Subthreshold Humps (L41449–41513)
- Low-Vt channel edges at STI; negligible if V_gst ≳ 100 mV, dominant near subthreshold; mirrors with stringers mismatch far more in subthreshold (L41450–41456; PDF p.698).
- Detect via subthreshold hump on log I_D–V_GS (L41464–41469). Fixes: poly fins over stringer regions (Fig 13.53B), annular gates (L41507–41513; PDF p.699).

#### PMOS versus NMOS Transistors (L41517–41531)
- PMOS often 30–50% more gm mismatch; some processes reversed — use measured data (L41518–41531; PDF p.699).

### 13.2.3 Bias-Dependent Mismatches (L41535–41538)
- Systematic mismatch from CLM, hot carriers, BTI (L41536–41538).

#### Channel-Length Modulation (L41542–41583)
- Eq 13.53 I_D2/I_D1 = 1 + λ·ΔV_DS/(1 + λ·V_DS1) ≈ 1 + λ·ΔV_DS; eq 13.54 λ ≈ 1/(B·L_e·√N_B), B ≈ 0.1–0.2 V·µm^½, valid for uniformly doped channels ≥ a few µm (L41543–41573; PDF p.699–700).
- Mirror L fell from 15–30 µm (20 V) to 5–10 µm (2 V); limit cascaded mirrors; use cascodes to equalize V_DS; source degeneration impractical for MOS (L41573–41583; PDF p.700).

#### Hot-Carrier Generation (L41587–41694)
- Impact ionization adds V_DS-dependent drain current (NMOS worse); no universal safe V_DS — use measured/simulated data (L41594–41606).
- Cascodes: eq 13.55 (W₁/L₁)/(W₂/L₂) = (W₃/L₃)/(W₄/L₄) ensures V_DS1 = V_DS2; with L₁ = L₂ and L₃ = L₄, eq 13.56 W₁/W₂ = W₃/W₄; cascode backgates tied to their own sources (L41610–41647; PDF p.700–701).
- HCI Vt drift accumulates "up to several tens of millivolts" between devices at different V_DS; also gm degradation — cascodes cure both (L41651–41694; PDF p.701–702).

#### Bias-Temperature Instability (L41698–41725)
- NBTI (PMOS, negative V_GS, high T), PBTI smaller; oxynitride/ONO raises both; BTI mismatch accrues with no drain current when matched devices sit at different V_GS (e.g., PMOS comparator inputs) — clamp diodes or NMOS input pair (L41699–41725; PDF p.702).

### 13.2.4 Hydrogenation (L41729–41798)
- H passivates interface traps (random Vt); metal blocks H diffusion; TiSi/TiW getter H (L41730–41781; PDF p.702–703).
- Metal-covered vs uncovered: up to 20% systematic current mismatch, any metal layer; full metal cover still leaves edge gradients and higher random mismatch (L41789–41794; PDF p.703).
- Best: no metal over active gates of critical pairs, minimal adjacent metal, identical surroundings; next best: metal plate over the active gate of all devices, upper metals route freely over it (L41794–41798).

#### Dummy Metal and MOS Matching (L41807–41840)
- Automatic fill can land on matched devices; use dummy-block pseudolayers, then hand-meet density (e.g., ≥ 25% in any 250 µm window; walking-window DRC samples windows, so instances can pass/fail differently) (L41808–41829; PDF p.703).
- Minimize count and size of block regions; cover less-critical pairs with metal plates instead (L41830–41832).
- Different surrounding fill (none over devices) → up to 1% current mismatch; identical custom fill out to several µm, similar out to ≥ 10 µm for critical pairs (L41835–41840; PDF p.703).

### 13.2.5 Gradients (L41859–41917)
- Effective position = active-area centroid (lies on every symmetry axis) (L41867–41877; PDF p.704).
- Eq 13.57 ΔP = α·P₁₂·d_CC·∇T_CC (gradient along the centroid axis times centroid distance) (L41897–41917; PDF p.704).

#### Oxide Thickness Gradients (L41921–41930)
- 250 Å dry oxide: 5% radial across 200 mm wafer ≈ 0.5 ppm/µm — negligible for properly laid out pairs (L41922–41930; PDF p.705).

#### Stress Gradients (L41940–42006)
- Stress barely moves Vt but moves k: eq 13.58 Δk = −k·(π_Lσ_L + π_Tσ_T + π_LTτ_LT)/(1 + …) ≈ −k·(π_Lσ_L + π_Tσ_T + π_LTτ_LT); π_LT ≈ 0 on usual axes; higher V_GS reduces NMOS ⟨100⟩ coefficients up to 50% (L41941–41970; PDF p.705).
- NMOS least stress-sensitive along ⟨110⟩ = layout X/Y; PMOS least along ⟨100⟩ = 45°, which editors/rules rarely support → diagonal not generally recommended (L41982–41999; PDF p.705–706).
- Avoid die edges/corners; die centre lowest gradient; on a die symmetry axis if possible; common centroid (L42003–42006; PDF p.706).

#### Temperature Gradients (L42010–42086)
- TC_Vt ≈ −2 mV/°C, bias-independent (L42011–42014; PDF p.706).
- Eq 13.59 bipolar offset is PTAT and trims across T; eq 13.60 s_Vio = √(s_Vt12² + (s_Id12/g_m1)² + (g_m3/g_m1·s_Vt34)² + (s_Id34/g_m1)²); trim at one T removes only ~⅔ of over-T drift; maximise g_m1, minimise g_m3 (L42022–42077; PDF p.706–707).
- Keep matched MOS away from power devices; on the symmetry axis through heat sources; common centroid (L42082–42086; PDF p.707).

### 13.2.6 Common-Centroid Layout of MOS Transistors (L42090–42315)
- CC cancels only the linear gradient; residue (mostly quadratic) grows with array size → compact + common-centroid (L42091–42095; PDF p.707).
- Pattern strings with S/D subscripts (e.g., ₛA… Fig 13.57 is ᴅAₛBᴅBₛAᴅ); tilted S/D implants and non-self-aligned layers make left- and right-drain sections differ (L42118–42140; PDF p.708).
- Orientation Φᵢ = +1 (current to the right) / −1 (left); eq 13.61 Φ = (1/N)·ΣΦᵢ; equal Φ (sign and magnitude) → no orientation mismatch, e.g. (3−1)/4 = (9−3)/12 = ½; ½ ≠ −½; 2-D arrays need both Φ_H and Φ_V equal (L42162–42187; PDF p.709).
- Table 13.2 five rules: coincidence, symmetry (both axes), dispersion, compactness, orientation (L42191–42210; PDF p.709).
- Table 13.3 common-source 1-D patterns, e.g. (ᴅAₛBᴅBₛA)ⁱᴅ and (ₛAᴅAₛBᴅBₛAᴅA)ⁱₛ (read from PDF p.710 figure). Without a common source insert gaps, or interdigitate along the width (common-gate mirrors) (L42215–42251; PDF p.710).
- 2-D AB/BA ≈ 60% of the residual gradient mismatch of 1-D ABBA with the same elements (FEA); square-ish units → 2-D arrays; cross-coupled pair ᴅAₛBᴅ/ᴅBₛAᴅ with half dummies (Fig 13.59) satisfies orientation (L42255–42296; PDF p.710–711).
- Cross-coupling only helps under a real gradient (test chips without heat sources show no difference) (L42287–42289). >2 segments: split into two groups (ₛAᴅAₛBᴅBₛ/ₛBᴅBₛAᴅAₛ) while roughly square, else several small cross-coupled subarrays; elaborate patterns hard to wire (L42305–42315; PDF p.711).

### 13.3 Rules for MOS Transistor Matching (L42321–42668)
- Classes (six-sigma, 10-year life, −40…125 °C, plastic package): **Minimal** ±10 mV or ±10% (bias mirrors with ≤ 3–4 mirrors from the reference resistor); **Moderate** ±3 mV or ±3% (general-purpose op-amp/comparator input pairs); **Exceptional** ±1 mV or ±1% (high-accuracy inputs; large CC arrays or trim; room-T trim removes ~⅔ of −40…125 °C drift; trim cannot fix thermal gradients) (L42333–42350; PDF p.712).
- Rules 1–11 (sizing/geometry): identical sections; area for voltage (Table 13.4); length for current (Table 13.5); avoid subthreshold for current matching (V_gst ≥ 100 mV all corners if stringers); avoid pocket implants unless short; thin oxide; same orientation (Φ equal); proximity; compactness (aspect ratio by class); 2-D CC; avoid submicron (L42360–42531; PDF p.712–714).
- Rules 12–24 (environment): dummies by class; low-stress die zones; distance from power devices; die symmetry axes; no contacts on active gate; metal-over-gate policy; dummy-metal blocking; deep-diffusion/well spacing; NBL shadow; gate extension +1–2 µm; metal gate straps; extraneous poly exclusion; diagonal PMOS only for exceptional + high stress (L42532–42668; PDF p.714–717). Full parameters in §3 (H13-42…H13-55).

### 13.4 Summary (L42674–42697)
- Integrated MOS power reaches ≤ 1 mΩ and tens of MHz switching (L42681–42686).
- MOS matching improves with thinner oxides and heavier backgates; competitive with bipolar for many uses (L42690–42693).

### Selected Bibliography (L42702–42716)
- El-Kareh & Hutter (HV/LDMOS), Croon/Sansen/Maes (deep-submicron matching), Pelgrom ADC ch.5 (matching) (L42704–42716).

### 13.5 Exercises (L42722–42871)
- 13.5–13.9 power FET metallization with thick Cu and bondwires; 13.7 requires every point within a given distance of a backgate contact (value garbled) (L42755–42794).
- 13.11: 100/10 cross-coupled pair 6σ = 5.7 mV → estimate for 1000/5 (Pelgrom √area scaling) (L42827–42829). 13.13 folded-cascode op-amp layout with C/C (cross-couple) and "~" (match as well as sizes allow) annotations and priority groups in parentheses (L42835–42855). 13.15 compute Φ for patterns; 13.16 stress Δk calc; 13.17 backgates to source "improve matching?" (L42860–42871).

---

## 3. Actionable extraction

Conventions: class ∈ {MIN, MOD, EXC} = Hastings minimal/moderate/exceptional; kind ∈ {V (voltage-matched), I (current-matched)}. "Deck" = pdks/*.json sidecar keys.

### H13-01 R_DS(on) budget and temperature scaling
- Kind: formula / metric
- Statement: R_DS(on) = R_Si + R_metal + R_pkg (eq 13.1). P = R_DS(on)·I_D² (13.2). R_ch ≅ 1/(k|V_GS−V_t|) (13.5) with TC ≈ +5000 ppm/°C at 25 °C (+50% at 125 °C); metal/package TC +3000–4000 ppm/°C; total low-V R_DS(on) TC 4000–6000 ppm/°C. Evaluate R_Si at min V_GS, max T_J, worst corner; metal sheet R at max T.
- Source: §13.1.1 On-Resistance, eqs 13.1–13.5; L38245–38346; PDF p.644–646.
- Philis stage: annotator, verify (PEX), flow.
- Automation recipe: detect power switches (annotator: device with W/L ≫ others, gate driven by a driver chain, drain to pad/inductor net, or user tag). Inputs: oppoint (V_GS, I_D), deck sheet R and TC per layer. After dr, extract R_metal of the S and D nets (PEX) at T_max; report R_Si + R_metal vs spec as a budget term in Θ.
- Beats hand layout because: every epoch reports the true metal share at T_max instead of a one-time hand estimate.
- Philis status: missing — no R_DS(on)/metallization metric (grep `rdson|on.?resist|one.?third` finds none in kernel/backend/frontend).

### H13-02 Specific on-resistance scaling limit
- Kind: heuristic / check
- Statement: R_SP = A_d·R_DS(on) (13.6), measured with Kelvin taps (package excluded). Do not extrapolate area more than 2–3× from a measured device.
- Source: §13.1.1 Specific On-Resistance; L38350–38376; PDF p.646.
- Philis stage: deck, annotator.
- Automation recipe: deck key `power_fet_rsp_mohm_mm2` with the reference area; annotator sizes the first-cut area A = R_SP/R_target and warns if A/A_ref ∉ [1/3, 3].
- Beats hand layout because: automatic early area estimate before placement.
- Philis status: missing.

### H13-03 Interdigitated power-FET pitch bounds
- Kind: formula / deck-requirement
- Statement: P ≥ L_d + W_C + 2·S_PC (13.7); P ≥ W_C + 2·E_MC + S_M1 (13.8); P ≥ W_V + 2·E_M1V + S_M1 (13.9, vias on the finger). Pitch = max of the three.
- Source: §13.1.1 Computing Metallization Resistance; L38423–38477; PDF p.647–648.
- Philis stage: cells.
- Automation recipe: in the MOS generator's `sd_and_pitch`, take the max of the three bounds from deck rules; for power FETs choose via-over-contact (W-plug decks) or staggered via (Al contacts) per deck flag.
- Beats hand layout because: minimum legal pitch computed exactly for every deck.
- Philis status: partial — cells computes an S/D pitch (`sd_and_pitch`, kernel/cells/src/mosfet.rs:262) and an M1 pitch (mosfet.rs:261); a via-on-finger bound (13.9) is not evident.

### H13-04 Stacked-metal fingers and via placement
- Kind: formula / rule
- Statement: stacking M1‖M2 with vias above every contact gives R_S12 = R_S1·R_S2/(R_S1+R_S2) (13.10); vias over contacts need W plugs; otherwise stagger vias between contacts. Vertical (contact/via) resistance is small versus lateral and usually ignored.
- Source: §13.1.1; L38511–38568; PDF p.648–649.
- Philis stage: cells, dr.
- Automation recipe: power-FET variant enumerates finger metal stacks (M1, M1‖M2, …) as cell variants; PEX/verify picks the lowest R_DS(on) meeting EM.
- Beats hand layout because: stack choice is searched, not guessed.
- Philis status: missing.

### H13-05 Rule of one-third for finger resistance with penetration-distance validity
- Kind: formula / check
- Statement: R_M = R_S·W/(3·W_M) (13.11), valid to ±1.5% if W ≤ d_pen, ±10% if W ≤ 1.8·d_pen, d_pen = R_Si·W_M/R_S (13.12). For the Fig 13.2 pattern (one S/D finger per gate, N sections, width W): R_M1 = W·R_S1/(6N·W_M) (13.14), d_pen = N·R_Si·W_M/R_S1 (13.15), compare W/2. Avoid fingers longer than 1.8·d_pen (non-uniform current).
- Source: §13.1.1; L38572–38661; PDF p.650–651 (example 50(100/0.7): 17 mΩ M1 on 100 mΩ Si).
- Philis stage: cells, verify.
- Automation recipe: closed-form cost for power-FET variant ranking (no FEA); hard limit W_finger/2 ≤ 1.8·d_pen emitted as a cell-enumeration filter.
- Beats hand layout because: all finger/width variants evaluated analytically per epoch.
- Philis status: missing.

### H13-06 Bus resistance and bus-termination topology
- Kind: formula / rule
- Statement: one bus R = 2NPR_S2/W (13.16); two buses R_B = 4NPR_S2/(3W) (13.17); d_pen = (R_Si + R_M1)·W/(4R_S2) (13.18), compare bus length N·P. Bus length should be ≤ ~2·d_pen; if longer, terminate the two buses at opposite ends (Fig 13.6B) for uniform current. Terminating both ends of each bus cuts R_B 4× and doubles d_pen (compare half the bus length).
- Source: §13.1.1; L38683–38753; PDF p.651–652.
- Philis stage: cells, gr/dr (power-net pin/exit selection).
- Automation recipe: power-FET cell exposes S and D bus pins at both ends; router cost prefers opposite-end exits when N·P > 2·d_pen; flag same-end exits as a current-crowding warning.
- Beats hand layout because: the topology choice is made numerically, avoiding the common intuitive mistake the book calls out.
- Philis status: missing.

### H13-07 Bondpad-sectioned power device
- Kind: algorithm
- Statement: with k pads per bus, current is zero on symmetry lines between pads; split into identical sections, compute each with buses terminating at pad centres (slightly conservative), total = parallel combination.
- Source: §13.1.4 Metallization, Fig 13.15; L39340–39345; PDF p.661–662.
- Philis stage: flow, verify.
- Automation recipe: R_metal estimator partitions at pad/bump pins, sums in parallel.
- Beats hand layout because: exact reuse of symmetry, no manual sectioning.
- Philis status: missing.

### H13-08 Power-FET metallization synthesis loop with EM checkpoints
- Kind: algorithm
- Statement: (1) assume metal = ⅓ of R_DS(on) (less for HV, more for low-V); (2) size silicon for the remainder; choose fingers for a convenient aspect ratio; (3) apportion layers (2LM: M1 fingers/M2 buses; 3LM: M1 fingers + M2‖M3 buses, or M1‖M2 fingers + M3 buses); (4) compute eqs 13.14–13.18; iterate; (5) EM: finger peak I_max ≅ I_D/N (N > 10) at the finger–bus junction (13.27), bus peak at exit/pad; EM widths from a usage profile (fraction of life per T/I), else assume max I at max T for life (overly pessimistic).
- Source: §13.1.4 Metallization; L39309–39389; PDF p.661–662.
- Philis stage: annotator (spec), cells, dr, verify.
- Automation recipe: inputs max R_DS(on), T_J, I_D, lifetime/usage profile (new spec fields); outer loop in flow over cell variants; EM rule instances at the two peak points with I_D/N.
- Beats hand layout because: iterates to a fixed point automatically across layer allocations; usage-profile EM avoids over-wide metal.
- Philis status: partial — per-net EM limits exist (kernel/analog/src/routing/em.rs:25-32, :92-98, deck `em_current_density_source` pdks/sky130.json); no power-FET finger/bus model, no usage profile.

### H13-09 Alternative power metallization patterns
- Kind: formula
- Statement: partial-width buses (Fig 13.16): R_M1 ≅ (A·R_S12 + B·R_S1)/(3N·W_M) (13.28), d_pen = N·R_Si·W_M/R_S12 (13.29); two EM peaks per finger (under the opposite bus edge and at its own bus). Diagonal device with trapezoidal buses (Fig 13.17): R_B ≅ (2NPR_S2/(3D))·ln((C+D)/C) (13.30), d_pen = (R_Si + R_M1)(C+D)/(2R_S2) (13.31); use the wasted triangle for bypass caps or substrate contacts.
- Source: §13.1.4; L39393–39493; PDF p.663–664.
- Philis stage: cells.
- Automation recipe: extra power-FET variants scored by the same closed forms; flow fills diagonal waste with decap/tap macros.
- Beats hand layout because: pattern choice by computed R and EM margin rather than habit.
- Philis status: missing.

### H13-10 Switching-loss metric
- Kind: formula / metric
- Statement: E_D = I_D·V_DS·t_s/2 (13.19); P_avg = I_D·V_DS·t_s·f_s (13.20), t_s = mean(turn-on, turn-off). Example 10 A, 5 V, 10 ns, 1 MHz → 0.5 W.
- Source: §13.1.2; L38770–38817; PDF p.652–653.
- Philis stage: flow (reporting), annotator (thermal source power).
- Automation recipe: feed P_avg into the device dissipation vector used by the thermal model so switching devices act as heat sources.
- Beats hand layout because: the thermal placement sees switching heat that a DC operating point misses.
- Philis status: partial — dissipation per device exists from DC oppoint only (kernel/core/src/layout.rs:33; frontend/library/src/oppoint.rs:14).

### H13-11 Maximum gate-finger width from gate RC delay
- Kind: formula / rule
- Statement: t_p(f) = k(f)·R·C, k = 1.0/1.3/2.0 for f = 0.90/0.95/0.99 (13.21); with C ≤ 2·C_ox, t_p ≈ 2.6·W²·R_S·ε_ox/t_ox (13.22, ε_ox = 3.5·10⁻¹⁷ F/µm); W_max ≈ √(t_ox·t_pmax/(2.6·R_S·ε_ox)) (13.23). Both-end gate contact → 2·W_max. Beyond that, split into banks. Example 90 Å, 10 Ω/□, 1 ns → 90 µm. Rarely switching devices exempt.
- Source: §13.1.2 Gate Propagation Delay; L38824–38945; PDF p.653–655.
- Philis stage: annotator (t_pmax from switching spec), cells (finger fold).
- Automation recipe: for devices tagged switching, add W_f ≤ W_max (×2 if the variant contacts both ends) to the finger-count filter; prefer the both-end variant.
- Beats hand layout because: W_max computed per deck oxide/poly sheet instead of a rule of thumb.
- Philis status: partial — finger folding already bounds finger width by deck `max_finger_width`, a gm-based gate-resistance floor `R□·W/(3·L·N²) ≤ 1/(5gm)` and the p2p resistance limit (frontend/library/src/cellgen.rs:600-640); a both-end contacted variant exists (`double_gate`, kernel/cells/src/mosfet.rs:44-47, :366). No switching-delay (13.23) bound.

### H13-12 Electrical/electrothermal SOA: backgate contact placement and BV rating
- Kind: rule / check
- Statement: lower backgate resistance extends SOA; place backgate contacts at the ends of each source finger (removes end field intensification); use distributed backgate contacts where backgate = source and rules allow, otherwise interdigitated. If BV_DII < BV_DSS, use BV_DII as the operating limit whenever the device can turn off under significant current (faults included).
- Source: §13.1.3; L39055–39071, L39165–39168, L39276–39285; PDF p.658–661.
- Philis stage: cells (power-FET taps), verify (ERC voltage check), deck (BV_DII).
- Automation recipe: power-FET generator inserts tap plugs at source-finger ends and periodically along source fingers (`backgate_tap_pitch` deck key); ERC compares oppoint/transient V_DS against deck BV_DII for devices whose drain current is large at turn-off.
- Beats hand layout because: tap coverage is enforced uniformly on every finger.
- Philis status: partial — generic tie distance `tie_max_dist_nm` 3000 (pdks/sky130.json:85) and guard rings per FET (backend/annotator/src/constraints.rs:65-75); no finger-end tap placement, no BV_DII rating.

### H13-13 Backgate sublayer tie strategy
- Kind: rule / deck-requirement
- Statement: a sublayer ≤ ~10 Ω/□ (P+ substrate under P-epi, NBL under N-well) provides backgate contact: then skip in-device taps and instead contact the sublayer (large substrate-contact areas; deep-N+ ring around PMOS over NBL, which also blocks hole injection; extra deep-N+ stripes for large currents). Without it (isolated P-tank, N-well without NBL, retrograde wells) use distributed/interdigitated backgate contacts.
- Source: §13.1.3 Backgate Resistance; L39253–39285; PDF p.660–661.
- Philis stage: deck, cells.
- Automation recipe: deck flag `backgate_sublayer_ohm_sq`; cells selects the tap strategy from it.
- Beats hand layout because: strategy tied to process data, not memory.
- Philis status: missing (deck has `p_epi_thickness`, `retrograde_pwell`, pdks/sky130.json:66-69, but no sublayer sheet R or strategy switch).

### H13-14 Spirito thermal-instability check
- Kind: check
- Statement: I_D rises with T if TC_Vt < TC_k·(V_GS−V_t)/(2V_t) (13.24); runaway if S = θ_JC·V_DS·dI_D/dT_J > 1 (13.25), T_J − T_C = θ_JC·P_D (13.26). θ_JC not reliably computable; low thermal resistance helps.
- Source: §13.1.3 The Spirito Effect; L39177–39241; PDF p.659–660.
- Philis stage: annotator (oppoint), flow (report).
- Automation recipe: from two oppoint temperatures compute dI_D/dT per power device; flag positive dI_D/dT with high V_DS·I_D; spread such devices (heat-spreading cost) and raise their priority in the thermal model.
- Beats hand layout because: detects a failure mode that is invisible in a nominal simulation.
- Philis status: missing.

### H13-15 Dense power layouts: waffle and bent-gate
- Kind: formula / heuristic
- Statement: waffle gain (W/L)_w/(W/L)_c = (2S_d + 0.55L_d)/(L_d + S_d) (13.32), > 1 if S_d > 0.45·L_d (L_d = 1, S_d = 3 µm → +64%). Metallization share halves the gain; 90° channel corners avalanche early (apply fillets/chamfers); no backgate-contact provision (latchup/debias risk). Bent-gate (135° bends) keeps density, allows distributed backgate contacts, adds ballasting; rule of one-third applies.
- Source: §13.1.4 Alternate Layouts; L39497–39608; PDF p.665–666.
- Philis stage: cells.
- Automation recipe: optional variant for switches in decks with CMP and a heavily doped substrate/sublayer; score with (13.32) × (1 − metal share).
- Beats hand layout because: selected only when the net gain after metallization is positive.
- Philis status: missing.

### H13-16 SenseFET generation and placement
- Kind: algorithm / rule
- Statement: sense ratio set by section counts; prefer the middle finger (bisects hot/cool regions); partial-finger segments spaced regularly along a drain finger (replicate temperature and metal debias); series segments for large ratios (e.g., 5 × 10% segments → 4850:1 with 100 sections); keep poly pitch uniform (moat holes rather than missing gates); dummy fingers so the senseFET is never an outermost finger (LDMOS first-finger effect up to 10%); one dummy gate each side of separate senseFETs. Sense-resistor alternative needs ≥ 10 mV (15–50 mV typical).
- Source: §13.1.4 SenseFETs; L39612–39726; §13.2.2 first-finger L41359–41363; PDF p.666–668, 697.
- Philis stage: annotator (pattern: small FET sharing G and S with a large FET, drain to a sense amp), cells.
- Automation recipe: annotator emits `SenseRatio{main, sense, ratio}`; cells builds the main FET with embedded sense segments at computed positions, forbidding end fingers.
- Beats hand layout because: segment positions derived from ratio and thermal symmetry, not trial.
- Philis status: missing (no senseFET pattern; grep `sense.?fet` none).

### H13-17 High-voltage device bounds (reference data)
- Kind: formula / deck-requirement
- Statement: w_d ≈ √(2ε_Si·V_DS/(q·N_B)) (13.33); E_max ≈ √(2q·N_B·V_DS/ε_Si) (13.34), E_crit ≈ 3·10⁵ V/cm; R_SPD ≥ 6.0·10⁻⁹·BV_DSS^2.5 (NMOS), 1.6·10⁻⁸·BV_DSS^2.5 (PMOS), mΩ·mm² (13.35–13.36). RESURF breakdown is sensitive to overlying metal/charge.
- Source: §13.1.5; L39772–39866, L39952–39958; PDF p.669–671.
- Philis stage: annotator (sanity), dr (no signal metal over RESURF drift unless the device has a field plate).
- Automation recipe: HV device classes carry a `no_route_over_drift` region in the cell; router treats it as a keepout.
- Beats hand layout because: overlying-metal hazard enforced on every HV device.
- Philis status: missing.

### H13-18 Drain-extended and DMOS layout constraints
- Kind: rule
- Statement: poly-to-well (DEMOS) and field-gap (DMOS) spacings must be exactly as specified (PCell values, not minimums); even number of gate sections with sources at both array ends cancels poly/well misalignment; isolated DENMOS outer drain fingers merge into the N-well isolation ring; drain-centric annular / rounded-end fingers avoid field crowding; DMOS: keep PSD plug pattern from the PCell, may explode only to remove outer drain fingers under a hole-blocking ring; LSD (source at substrate, no NBL, higher BV) vs HSD (NBL, lower BV).
- Source: §13.1.5 Drain-Extended / Field-Gapped / DMOS; L40096–40142, L40276–40283, L40330–40335, L40464–40503, L40541–40566; PDF p.674–684.
- Philis stage: cells, annotator (LSD/HSD from source net = substrate).
- Automation recipe: DEMOS/DMOS generators take "exact" keys from the deck; annotator selects HSD variant when source ≠ substrate net.
- Beats hand layout because: exact-dimension devices cannot be accidentally "optimised" by compaction.
- Philis status: missing (no DEMOS/DMOS generator in kernel/cells/src).

### H13-19 Voltage- vs current-matching classification and overdrive floor
- Kind: formula / data-model
- Statement: ΔV_GS ≅ ΔV_t − V_gst·Δk/(2k) (13.40); I_D2/I_D1 ≅ (1 − Δk/k)(1 + 2ΔV_t/V_gst) (13.41); s_V ≅ √(s_Vt² + (V_gst/2·s_k/k)²) (13.42); s_I/I_D ≅ √((s_k/k)² + (2·s_Vt/V_gst)²) (13.43); Vt-dominated: s_V ≈ s_Vt (13.44), s_I ≈ g_m·s_Vt (13.45). Voltage matching improves with lower V_gst but not below ~100 mV; current matching improves with higher V_gst.
- Source: §13.2; L40648–40742; PDF p.686–687.
- Philis stage: annotator, flow (report).
- Automation recipe: annotator tags each matched group kind V (diff pair: equal I_D, compare V_GS) or I (mirror: equal V_GS, compare I_D). With oppoint V_gst and deck A_VT/A_k, compute predicted σ per group and report it with the layout-induced systematic terms (gradient, LOD, WPE) as one offset budget.
- Beats hand layout because: quantitative per-pair offset prediction including layout terms, for every pair.
- Philis status: partial — diff pairs recognised and budgeted with Pelgrom distance term (kernel/analog/src/placement/matching_pair.rs:17-41; backend/annotator/src/emit.rs:1-27); no explicit V/I kind and no current-mismatch model.

### H13-20 Pelgrom area law with oxide and body-bias corrections
- Kind: formula
- Statement: s_Vt = c_Vt/√(W_e·L_e) (13.46), 99.9…% within ±6·s_Vt; c_Vt ≈ 1 mV·µm per nm of t_ox (e.g., 15 mV·µm at 150 Å; rough); V_BS = 2 V on a 5 V device raises c_Vt ≈ 30%; unequal devices s_Vt = c_Vt·√(1/(2W₁L₁) + 1/(2W₂L₂)) (13.49).
- Source: §13.2.1 Threshold Voltage Matching; L40756–40850; PDF p.687–689.
- Philis stage: deck, annotator, verify.
- Automation recipe: deck A_VT per polarity (fallback c_Vt = t_ox[nm]·1 mV·µm, flagged as estimate); annotator flags matched devices with nonzero V_BS (source ≠ bulk net or oppoint V_BS ≠ 0) and multiplies A_VT by a deck body-bias factor.
- Beats hand layout because: body-bias and unequal-size penalties are applied automatically.
- Philis status: partial — deck A_VT from MC (pdks/sky130.json:93-96, avt_n 9.5, avt_p 11.5 mV·µm) used by MatchingPair; no V_BS check (grep `vbs|body.?bias` none).

### H13-21 Current matching depends on L only
- Kind: formula / check
- Statement: s_I = c_Vt·√(2I_D·k′)/L_e (13.48) → relative s_I/I_D = c_Vt·√(2k′/I_D)/L_e; W does not help at fixed I_D. Required L therefore scales as I_D^−½ (derived from 13.48) from Table 13.5 values at 1 µA.
- Source: §13.2.1; L40817–40837; PDF p.688.
- Philis stage: annotator (sizing audit).
- Automation recipe: for I-kind groups, compute L_req = 6·c_Vt·√(2k′/I_D)/ε_class (ε = 0.10/0.03/0.01) using deck k′ and oppoint I_D; warn if L < L_req (P&R cannot fix; reports to the designer).
- Beats hand layout because: every mirror audited against its class, which humans skip.
- Philis status: missing.

### H13-22 Ratioed devices from identical unit sections (rule 1)
- Kind: rule / algorithm
- Statement: all sections of matched devices share W and L; ratio by count; the smaller device of a large ratio uses series units, the larger parallel units (20:1 = 4 series vs 5 parallel, Fig 13.41). Series units carry a small inherent mismatch → not for EXC.
- Source: §13.2.1 L40850–40864; §13.3 rule 1 L42360–42368; PDF p.688–689, 712.
- Philis stage: annotator (unitization), cells.
- Automation recipe: for a mirror group with members (W_i/L_i), find a unit (W_u, L_u) with L_i = s_i·L_u (series count s_i) and W_i = p_i·W_u (parallel count p_i), maximising unit area subject to integer s_i, p_i and the class's area/length requirement; emit one Unitization with per-member (s_i, p_i); cells draws series as Chain rows.
- Beats hand layout because: exhaustive integer search over unit choices; humans stop at the first workable one.
- Philis status: partial — unitization groups only identical (kind, W, L) classes and ratios by finger count (backend/annotator/src/constraints.rs:4-7, :42-63); different-L members are never unitized; series stack exists as `Pattern::Chain` (kernel/cells/src/lib.rs:37-40).

### H13-23 Short/narrow and pocket-implant limits (rules 5, 11)
- Kind: rule / check
- Statement: keep W and L ≥ 1 µm for Pelgrom validity (edge terms, 13.52); avoid submicron in MOD/EXC unless data show otherwise. Pocket-implant devices: s_Vt = c_Vt/√(W_e·min(L_e, L_c)) (13.50), L_c ≤ a couple of µm → for V-matching use L ≤ L_c and widen; for I-matching use analog (no-pocket) devices, since current matching typically needs L > 5 µm.
- Source: §13.2.1 L40870–40901, L40942–40962; rules 5, 11 L42473–42482, L42525–42531; PDF p.689–690, 713–714.
- Philis stage: deck (per model `has_pocket_implant`, `L_c`), annotator (warn).
- Automation recipe: annotator warns for MOD/EXC groups with W or L < 1 µm, or pocket-implant models with L > L_c in V-groups, or any pocket model in I-groups.
- Beats hand layout because: model-aware sizing audit.
- Philis status: missing (cellgen only warns below the deck's minimum legal channel, frontend/library/src/cellgen.rs:620-628).

### H13-24 Transconductance Pelgrom and polarity choice
- Kind: formula / data-model
- Statement: s_k/k = c_k/√(W_e·L_e) (13.51, c_k in %·µm ÷ 100); full form with peripheral terms (13.52). PMOS often 30–50% more gm mismatch than NMOS (process-dependent; use data).
- Source: §13.2.1 L40926–40962; §13.2.2 PMOS vs NMOS L41517–41531; PDF p.690, 699.
- Philis stage: deck.
- Automation recipe: deck keys `ak_n_pct_um`, `ak_p_pct_um`; used by H13-19 current-mismatch prediction.
- Beats hand layout because: gm term included in every I-mismatch estimate.
- Philis status: missing.

### H13-25 Orientation metric Φ, superimposability, and same-cell grouping (rule 7)
- Kind: metric / check / rule
- Statement: Φᵢ = +1 (S→D current right) or −1 (left); Φ = (1/N)·ΣΦᵢ (13.61); matched devices need equal Φ_H and Φ_V (sign and magnitude). Required equal for EXC and for any class with non-self-aligned channels (asymmetric DEMOS); equal or nearly equal for MOD self-aligned. Channels parallel (no 90° rotation): rotated devices mismatch by several %, up to 5% on tilted wafers. Non-self-aligned devices must be superimposable (translation only). Keep matched devices in one cell so a cell rotation cannot split them.
- Source: §13.2.1 Orientation L40966–41017; §13.2.6 L42162–42187; rule 7 L42490–42498; PDF p.690–691, 709, 714.
- Philis stage: cells, dp, verify.
- Automation recipe: per unit store φ = (φx, φy); verify computes Φ_H, Φ_V per matched member across all its units after placement (including separately placed members, after `orient` is applied) and fails MOD/EXC groups with unequal Φ; dp forbids rotations and mirror moves of matched devices.
- Beats hand layout because: Φ checked exactly on the final layout, including after top-level transforms, which is the error mode Hastings describes.
- Philis status: partial — units carry φ (kernel/core/src/units.rs:26-27; set from S/D side at kernel/cells/src/mosfet.rs:387); merged cells are accepted only if Σφ_a·n_b = Σφ_b·n_a (frontend/library/src/cellgen.rs:255-271); dp never rotates matched devices (backend/dp/src/lib.rs:555-560). No signoff Φ check for separately placed members.

### H13-26 Dummy-gate geometric equivalence (poly microloading)
- Kind: rule
- Statement: each active gate must see poly on both sides; dummy-to-active spacing = active pitch spacing; dummy width (moat-defined) = active width; dummy poly extension beyond moat = active extension; dummy gate tied off (to backgate); microloading reach 3–5 µm (up to 10 µm for very accurate), so dummy L may be capped at 3–5 µm unless active L < 3–5 µm (then dummy L = active L; add dummies until patterned poly extends the reach). Half dummies (one S/D termination) acceptable for L > 3–5 µm and lower accuracy; poly length ≥ moat-over-poly + poly-over-moat rules and ≥ 2–3 µm. Dummy outer S/D may be uncontacted, but add contacts if DRC/LVS flags them.
- Source: §13.2.2 Polysilicon Etch Rate Variation; L41059–41121; PDF p.692–693.
- Philis stage: cells, verify (LVS dummy handling).
- Automation recipe: MOS generator draws dummies with the same W, same poly extension, pitch spacing, tied to bulk; dummy L = active L if L_active < `microload_reach_nm` else max(`microload_reach_nm`, rule minimum); count per H13-47.
- Beats hand layout because: geometric equivalence is by construction, not by inspection.
- Philis status: partial — dummies drawn at the array's pitch, tied to the bulk rail and listed for LVS (kernel/cells/src/mosfet.rs:270-283, :484-485); count fixed per deck `dummy_gates_per_end` default 1 (mosfet.rs:50-55); code comments cite "Hastings §13.3 r9" (mosfet.rs:278, :291), but in the 3rd edition the dummy/LOD rule is rule 12.

### H13-27 Poly-0 topography keepaway
- Kind: rule
- Statement: double-poly processes: poly-0 (capacitor bottom plate) topography disturbs resist spin; mismatches up to 12% at distances > 30 µm; place matched transistors ≥ 150 µm from poly-0.
- Source: §13.2.2; L41158–41187; PDF p.693–694.
- Philis stage: deck, gp/dp.
- Automation recipe: deck key `poly0_keepaway_um` (only for decks with a poly-0 layer); placement constraint: matched MOS centroid distance to any poly-0 shape ≥ key (hard for MOD/EXC).
- Beats hand layout because: long-range keepaway enforced globally.
- Philis status: missing (not applicable to current decks unless a double-poly cap layer exists; no key).

### H13-28 Well proximity effect model and data
- Kind: data-model / rule
- Statement: WPE reach up to 3–5 µm, strongest within 1–2 µm; measured 5% current mismatch at 1.8 µm, 25% at 0.95 µm gate-to-well-edge. Fixes: well edges ≥ 3–5 µm from matched gates; end dummies push edges out; widen devices to dilute side-edge doping; move side well edges out into field.
- Source: §13.2.2 Well Proximity Effect; L41265–41322; PDF p.695–696. (Class thresholds in H13-53.)
- Philis stage: cells, gp/dp, verify.
- Automation recipe: per unit compute distance to nearest well edge on each side (both the device's own well and foreign wells for NMOS); add to units as `sc` (BSIM4 SCA-like); CentroidGroup compares the member means as it does LOD.
- Beats hand layout because: WPE asymmetry measured on the placed layout, including foreign wells that a cell-level halo cannot see.
- Philis status: partial — matched PMOS cells inflate N-well enclosure by `wpe_clearance_moderate` (kernel/cells/src/mosfet.rs:236-239, :626-630; pdks/sky130.json:86); no NMOS-to-foreign-well spacing at placement.

### H13-29 LOD / OD-to-OD stress and piezo data
- Kind: formula / rule / deck-requirement
- Statement: STI stress shifts k by ≥ 10% (NMOS down, PMOS up) and Vt by ≥ 10 mV; end devices of an array see more stress; stretch end moats 3–5 µm (2–3 µm for less accurate); with dummies, moat ≥ 3–5 µm past the last active gate; L ≥ 3–5 µm → one dummy per end suffices, else ≥ 2. Table 13.1 piezo coefficients (10⁻¹¹ Pa⁻¹): ⟨100⟩ PMOS −15/30, NMOS −90/40; ⟨110⟩ PMOS 65/−50, NMOS −30/−20 (π_L/π_T). OD-to-OD (trench width) fixed the same way.
- Source: §13.2.2 Length of Diffusion Effects; L41367–41445; PDF p.697–698.
- Philis stage: cells, dp (CentroidGroup LOD term), deck.
- Automation recipe: already modelled via SA/SB; add OD-to-OD term (distance to neighbouring OD across the trench) to units; class-dependent moat length (H13-47).
- Beats hand layout because: LOD mismatch computed in mV per placement using the model's KVTH0.
- Philis status: partial — units carry SA/SB (kernel/core/src/units.rs:28-32, :62-63), deck KVTH0 (pdks/sky130.json:90-92), CentroidGroup uses an LOD term (kernel/analog/src/placement/cc.rs:36-40), cells extend moats by `lod_moat_ext_moderate` = 3000 nm (kernel/cells/src/mosfet.rs:287-293; pdks/sky130.json:81). The tiered `lod_moat_ext_nm` [3000, 5000] (pdks/sky130.json:70-73) is not read by any Rust code; no OD-to-OD term.

### H13-30 NBL shadow keepout
- Kind: rule
- Statement: non-STI (LOCOS) processes with NBL: pattern shift ≤ 150% of epi thickness (use 150% of max epi if unknown, all sides if direction unknown); keep the shadow outside the active area of MOD/EXC transistors; STI/CMP processes have none.
- Source: §13.2.2 L41342–41356; rule 20 L42632–42637; PDF p.697, 716.
- Philis stage: deck, cells.
- Automation recipe: deck flag `nbl_shadow_shift_nm` (absent for STI decks); cells enlarges NBL/well by the shift around MOD/EXC matched PMOS.
- Beats hand layout because: applied only where the process needs it.
- Philis status: missing (not needed for sky130/gf180/ihp STI decks).

### H13-31 First-finger effect for multi-finger power/sense devices
- Kind: rule
- Statement: outer fingers of multi-finger LDMOS differ by up to 10% (implant scatter, microloading); never use an outer finger as the sense/matched element — add dummy fingers.
- Source: §13.2.2 L41359–41363; PDF p.697.
- Philis stage: cells.
- Automation recipe: senseFET/matched finger index constrained to [1 + n_dummy, N − n_dummy].
- Beats hand layout because: enforced by construction.
- Philis status: missing.

### H13-32 Stringer avoidance: overdrive floor (rule 4)
- Kind: check / rule
- Statement: if stringers are known/suspected (STI process, subthreshold hump), current-matched devices above MIN class must run at V_GS − V_t ≥ 100 mV in all corners, or use stringer-resistant layouts (poly fins across channel edges, annular gates).
- Source: §13.2.2 Stringers L41449–41513; rule 4 L42464–42472; PDF p.698–699, 713.
- Philis stage: annotator (oppoint), cells (fin variant).
- Automation recipe: oppoint reports V_gst (V_GS − V_th) per device; annotator flags I-kind MOD/EXC groups with V_gst < 100 mV (deck `stringer_prone`); cells offers a fin variant (poly extension across the channel edge, Fig 13.53B).
- Beats hand layout because: corner-wide overdrive check on every mirror.
- Philis status: missing — oppoint reads V_ds and V_dsat only (frontend/library/src/oppoint.rs:186, :205).

### H13-33 CLM systematic mismatch from ΔV_DS
- Kind: formula / check
- Statement: I_D2/I_D1 ≈ 1 + λ·ΔV_DS (13.53); λ ≈ 1/(B·L_e·√N_B), B ≈ 0.1–0.2 V·µm^½ (13.54, long uniform channels). Fixes: longer L, fewer cascaded mirrors, cascodes to equalize V_DS.
- Source: §13.2.3 Channel-Length Modulation; L41542–41583; PDF p.699–700.
- Philis stage: annotator (oppoint audit), flow (report).
- Automation recipe: for each mirror group, ε_CLM = |g_ds·ΔV_DS/I_D| from oppoint (use simulator g_ds instead of 13.54); report it alongside the random and layout terms; flag if ε_CLM exceeds the class budget.
- Beats hand layout because: systematic error quantified with the real bias point.
- Philis status: missing.

### H13-34 Hot-carrier equalization with cascodes
- Kind: rule / check
- Statement: matched devices at different V_DS drift apart by up to tens of mV (HCI Vt and gm shift); cascodes equalize and lower V_DS: (W₁/L₁)/(W₂/L₂) = (W₃/L₃)/(W₄/L₄) (13.55), with equal L: W₁/W₂ = W₃/W₄ (13.56); cascode backgates tied to their own sources so impact-ionization current cannot skew the ratio.
- Source: §13.2.3 Hot-Carrier Generation; L41587–41694; PDF p.700–702.
- Philis stage: annotator.
- Automation recipe: annotator recognises cascoded mirrors (existing catalog) and checks the width-ratio identity (13.55/13.56) and cascode bulk = source; reports violations.
- Beats hand layout because: netlist-level lifetime-mismatch audit runs on every design.
- Philis status: partial — cascode patterns recognised (backend/annotator/src/catalog.rs:261-280); no ratio/bulk checks.

### H13-35 BTI mismatch flag
- Kind: check
- Statement: BTI mismatch accrues without drain current when matched devices sit at different V_GS with at least one negative (typically PMOS comparator input pairs); cures: clamp diodes, NMOS input pair.
- Source: §13.2.3 Bias-Temperature Instability; L41698–41725; PDF p.702.
- Philis stage: annotator.
- Automation recipe: flag PMOS V-groups whose recognised block is a comparator (inputs may sit unbalanced).
- Beats hand layout because: surfaces a lifetime issue during layout.
- Philis status: missing.

### H13-36 Hydrogenation: metal coverage and surrounding-metal similarity
- Kind: rule / metric
- Statement: metal over vs not over a matched gate → up to 20% current mismatch (any layer); full coverage adds edge gradients and random mismatch; fill differences nearby → up to 1% even with no fill over the devices. Keep surrounding metal identical out to several µm and similar out to ≥ 10 µm for critical pairs.
- Source: §13.2.4; L41753–41840; PDF p.702–703. (Class rules in H13-51, H13-52.)
- Philis stage: gr/dr, flow (fill), verify.
- Automation recipe: metric per matched group: for each member, metal coverage fraction per layer inside a 10 µm halo of its active gates; residual = max pairwise difference; add to Θ as a cost; fill generation mirrors (copies) the fill pattern around members so the halo contents match.
- Beats hand layout because: surrounding-metal similarity measured numerically on the final GDS, which a human cannot do for every pair.
- Philis status: missing (no coverage-similarity metric).

### H13-37 Gradient mismatch and centroid distance
- Kind: formula / metric
- Statement: ΔP = α·P₁₂·d_CC·∇_CC (13.57) — mismatch ∝ centroid distance × gradient along the centroid axis; centroid = active-area centroid; wafer oxide gradient ≈ 0.5 ppm/µm (5% over 200 mm) — negligible for proper layouts.
- Source: §13.2.5; L41859–41930; PDF p.704–705.
- Philis stage: dp, verify.
- Automation recipe: already the CentroidGroup/ThermalGradient model; ensure centroids use gate-area-weighted active centroids.
- Beats hand layout because: exact centroid arithmetic over all units.
- Philis status: implemented — unit weights are gate area (kernel/core/src/units.rs:22-25); CentroidGroup gradient term (kernel/analog/src/placement/cc.rs:20-36); ThermalGradient prices ΔT on the live field (kernel/analog/src/placement/thermal.rs:7-15).

### H13-38 Stress-induced gm and orientation axes (rule 24)
- Kind: formula / rule
- Statement: Δk ≈ −k·(π_Lσ_L + π_Tσ_T + π_LTτ_LT) (13.58); NMOS least sensitive along ⟨110⟩ = layout X/Y; PMOS least along ⟨100⟩ = 45°. Diagonal PMOS only for EXC under significant stress (large die, solder/eutectic die attach), never with directional (pocket) implants, and only if the process permits.
- Source: §13.2.5 Stress Gradients L41940–42006; rule 24 L42655–42668; PDF p.705–706, 716–717.
- Philis stage: deck (π tables), verify.
- Automation recipe: keep Manhattan; use Table 13.1 only to weight stress-gradient sensitivity per polarity in the placement stress term; do not generate 45° devices.
- Beats hand layout because: polarity-aware stress weighting.
- Philis status: missing (no stress model beyond LOD; grep `stress|die.?edge` finds only LOD comments).

### H13-39 Offset composition and trim limits
- Kind: formula / metric
- Statement: s_Vio = √(s_Vt12² + (s_Id12/g_m1)² + (g_m3/g_m1·s_Vt34)² + (s_Id34/g_m1)²) (13.60); room-T trim removes only ~⅔ of the −40…125 °C MOS offset drift (bipolar trims nearly all); maximise g_m1, minimise g_m3.
- Source: §13.2.5 Temperature Gradients; L42057–42077; §13.3 L42343–42347; PDF p.707, 712.
- Philis stage: annotator, flow (report).
- Automation recipe: annotator composes the input-referred offset of recognised diff-pair + mirror-load stages from per-group σ (H13-19) and oppoint g_m; each group's budget is set so the composed offset meets the stage spec; the load mirror gets the class implied by its (g_m3/g_m1) weight.
- Beats hand layout because: class assignment for load mirrors is derived from the circuit, not guessed.
- Philis status: missing (MatchingPair budgets pairs individually, backend/annotator/src/emit.rs:43-83).

### H13-40 Five common-centroid rules as checks (Table 13.2)
- Kind: check
- Statement: (1) coincidence — centroids coincide (at least approximately); (2) symmetry about both horizontal and vertical axes; (3) dispersion — split large arrays into many small subarrays each satisfying 1–2 (then only subarrays need to); (4) compactness of the array or each subarray; (5) orientation — equal Φ.
- Source: §13.2.6 Table 13.2; L42191–42210; PDF p.709.
- Philis stage: cells (pattern generation), verify.
- Automation recipe: verify computes per group: centroid offset (nm), symmetry defect (units without a mirror partner of the same owner about each axis), subarray decomposition count, aspect ratio of each CC subarray, Φ_H/Φ_V; pass/fail by class.
- Beats hand layout because: all five scored numerically for every matched array.
- Philis status: partial — coincidence priced by CentroidGroup (kernel/analog/src/placement/cc.rs:20-40); Φ equality at cell merge (frontend/library/src/cellgen.rs:255-271); symmetry/dispersion/compactness not checked.

### H13-41 Pattern library and 2-D preference (rule 10)
- Kind: algorithm
- Statement: 1-D common-source patterns (Table 13.3, e.g., (ᴅAₛBᴅBₛA)ⁱᴅ, (ₛAᴅAₛBᴅBₛAᴅA)ⁱₛ); without a common source insert gaps or interdigitate along the width (natural for common-gate mirrors under one poly plate); 2-D AB/BA residual ≈ 60% of 1-D ABBA → prefer 2-D when units are near square; cross-coupled pair ᴅAₛBᴅ/ᴅBₛAᴅ with half dummies; for > 2 segments use two groups while roughly square, else several small cross-coupled subarrays (dispersion). Residual ∝ (subarray size)².
- Source: §13.2.6; L42215–42315; rule 10 L42517–42524; PDF p.710–711, 714.
- Philis stage: cells, dp (variant choice).
- Automation recipe: enumerate pattern variants {ABBA 1-D, AB/BA 2-D, k×(AB/BA) dispersed}; score each by predicted residual = c·(subarray diagonal)² × gradient + Φ equality + wiring cost; let dp pick the variant.
- Beats hand layout because: variant choice is quantitative and considers wiring cost jointly with residual.
- Philis status: partial — Pattern {Single, Cc1d (ABBA), Interdig, Chain} (kernel/cells/src/lib.rs:29-41) and a 2-row variant that turns a blocked pair into a cross-coupled quad (kernel/cells/src/mosfet.rs:32-35, :108-129); no dispersed multi-subarray variant, no width-direction interdigitation.

### H13-42 Matching class data model (the backbone of 13.3)
- Kind: data-model
- Statement: MIN = ±10 mV / ±10%; MOD = ±3 mV / ±3%; EXC = ±1 mV / ±1% (six-sigma, 10-year, −40…125 °C, plastic package). Defaults by function: MIN for bias mirrors (≤ 3–4 mirrors from the reference resistor), MOD for general-purpose op-amp/comparator input pairs, EXC for high-accuracy input stages. Rules 1–24 are parameterized by class and by kind (V/I).
- Source: §13.3; L42327–42350; PDF p.712.
- Philis stage: annotator (emit), all consumers.
- Automation recipe: add `MatchClass {Min, Mod, Exc}` and `MatchKind {Voltage, Current}` to every matching constraint (MatchingPair, CentroidGroup, Unitization). Annotator default: DiffPair → (V, Mod); load mirror → (I, class from H13-39 weight, default Mod); bias mirror → (I, Min); user override via netlist attribute or spec (offset in mV → class). Every class-dependent distance comes from a tiered deck array indexed by class (`wpe_clearance_nm[3]`, `lod_moat_ext_nm[…]`, etc.).
- Beats hand layout because: one explicit precision target per group drives every downstream geometric rule consistently.
- Philis status: missing — no class enum (grep `MatchClass|MatchLevel` none); code reads only the `_moderate` scalars (kernel/cells/src/mosfet.rs:293, :629) although the deck already holds tiered arrays `wpe_clearance_nm` [2000, 3000, 5000] and `lod_moat_ext_nm` [3000, 5000] (pdks/sky130.json:59-63, :70-73) that no Rust code reads.

### H13-43 Area requirement for voltage matching (rules 2, 6; Table 13.4)
- Kind: check / formula
- Statement: Table 13.4 required active area (µm²) for 12 / 5 / 3.3 / 1.8 V devices: MIN 423 / 56 / 19 / 5.8; MOD 4,700 / 630 / 220 / 64; EXC 42,000 / 5,600 / 1,900 / 580 (assumes c_Vt = 1 mV·µm per nm t_ox, E_max = 3.5 MV/cm (12 V), 4.0 (5 V), 4.5 (3.3, 1.8 V)). Consistent with A = (6·c_Vt/ΔV)², c_Vt = t_ox[nm]·1 mV·µm, t_ox = V_rating/E_max (derived check: 5 V EXC → (6·12.5/1)² = 5,625 µm²). Prefer thin-oxide devices (rule 6); quadrupling area halves mismatch.
- Source: §13.3 rules 2, 6; L42369–42407, L42483–42489; PDF p.712–714.
- Philis stage: annotator (audit), flow (report).
- Automation recipe: for V-groups compute A_req = (6·A_VT/ΔV_class)² with deck A_VT (not the t_ox rule when data exist); report W·L·m vs A_req; the layout cannot add area but can report and choose unit folding that keeps each unit ≥ 1 µm.
- Beats hand layout because: every pair audited against its class with process data.
- Philis status: missing (MatchingPair takes gate area as given and budgets only the distance term, kernel/analog/src/placement/matching_pair.rs:17-24).

### H13-44 Length requirement for current matching (rule 3; Table 13.5)
- Kind: check
- Statement: Table 13.5 channel length (µm) at I_D = 1 µA for 12 / 5 / 3.3 / 1.8 V: NMOS MIN 24 / 15 / 11 / 8.1, MOD 79 / 48 / 37 / 27, EXC 240 / 140 / 110 / 81; PMOS MIN 14 / 8.6 / 6.6 / 4.9, MOD 47 / 29 / 22 / 16, EXC 142 / 86 / 66 / 49 (Vt-dominated; k′ used not given). Scale with I_D^−½ (eq 13.48). Width does not help current matching at fixed I_D.
- Source: §13.3 rule 3; L42412–42459; PDF p.713.
- Philis stage: annotator (audit).
- Automation recipe: as H13-21 with deck k′; report L vs L_req per mirror.
- Beats hand layout because: exposes the "extraordinary lengths" requirement early, per mirror.
- Philis status: missing.

### H13-45 Proximity by class (rule 8)
- Kind: rule / constraint
- Statement: MIN: as close as possible; MOD: adjacent, preferably interdigitated so centroids approximately align; EXC: carefully constructed common-centroid layout, always.
- Source: §13.3 rule 8; L42499–42503; PDF p.714.
- Philis stage: annotator (constraint selection), cells, gp/dp.
- Automation recipe: MIN → Proximity budget; MOD → merged cell with Interdig/Cc1d (or adjacency hard constraint if separate cells); EXC → CentroidGroup hard (coincidence within grid) + 2-D pattern.
- Beats hand layout because: the constraint strength follows the class automatically.
- Philis status: partial — Proximity, CentroidGroup and merged interdigitated cells exist (backend/annotator/src/emit.rs:8-27) but are selected by circuit kind, not class.

### H13-46 Compactness: aspect-ratio limits by class and kind (rule 9)
- Kind: check / cost term
- Statement: current matching: MIN ≤ 10:1, MOD ≤ 3:1, EXC as square as possible; voltage matching: MIN ≤ 3:1, MOD and EXC square or nearly square. If the array is built from CC subarrays, only subarray aspect ratios count; an elongated area can still match well when divided into compact subarrays.
- Source: §13.3 rule 9; L42504–42516; PDF p.714.
- Philis stage: cells (variant filter), dp (variant choice).
- Automation recipe: per matched macro variant compute the aspect ratio of its minimal CC subarray; filter variants over the class limit (hard), cost toward 1:1.
- Beats hand layout because: exact limits applied per variant.
- Philis status: partial — finger folding aims each class row "closest to square" (frontend/library/src/cellgen.rs:600-613), independent of matching class and kind.

### H13-47 Dummies by class (rule 12)
- Kind: rule / constraint
- Statement: MIN: optional ("good idea if space permits"). MOD: generally; L < ~3 µm → one full dummy per end; L > ~3 µm → half dummies allowed, poly width ≤ 3 µm needed; STI: moat ≥ 5 µm past the last active gate (may make a full dummy with 2–3 µm channel the best choice). EXC: always; L < ~10 µm → one or more full dummies so the outer poly edge of the outermost dummy is ≥ 10 µm from the nearest active device; L > ~10 µm → half dummies with poly width ≥ 10 µm; STI: moat ≥ 10 µm past the outermost active gate (a full dummy with 8–10 µm gate length may be best). Dummy gates tied off. Proper dummies also cancel LOD.
- Source: §13.3 rule 12; L42532–42553; PDF p.714–715. (13.2.2 gives 3–5 µm moat extension and ≥ 2 dummies if L < 3–5 µm, L41411–41422.)
- Philis stage: cells, deck.
- Automation recipe: deck tiered keys `dummy_reach_nm` [0, 3000, 10000] and `lod_moat_ext_nm` [0, 5000, 10000]; generator: n_dummy = ceil((reach − poly extent already present)/(dummy L + spacing)), with dummy L = active L if active L < reach else one long dummy of L = reach (full) or half dummy if the deck allows; moat past last active = tier value.
- Beats hand layout because: the dummy count/length solves the reach inequality exactly per device and deck.
- Philis status: partial — annotator sets `dummy_required: true` for every unitization (backend/annotator/src/constraints.rs:61; frontend/library/src/cellgen.rs:502); generator draws `dummy_gates_per_end` (default 1, clamp 0–4; kernel/cells/src/mosfet.rs:50-55, :74) and moat `lod_moat_ext_moderate` 3 µm (mosfet.rs:293) — below Hastings' STI MOD value of 5 µm; no EXC reach rule, no half dummies.

### H13-48 Die stress zones and symmetry axes (rules 13, 15)
- Kind: rule / constraint
- Statement: leaded plastic package: lowest stress gradient from die centre halfway to the edges. MIN: not in corners, not within 50–100 µm of die edges; MOD: interior, or near the centre of one side inset ≥ 100–250 µm; EXC: interior, on a die symmetry axis. Bumps/pillars: gradients peak at bump perimeters, minima at bump centres and midway between bumps (symmetric about those axes); MOD may sit well inside a bump footprint or midway between bumps on their axis; EXC on the axis between bumps.
- Source: §13.3 rules 13, 15; L42554–42568, L42590–42595; PDF p.715.
- Philis stage: flow (block floorplan context), gp/dp.
- Automation recipe: optional die context (die outline, block origin, bump map); region costs per class: forbidden bands at edges/corners, attraction to die axes and bump-midlines for EXC groups. Without die context, report "unknown" (block-level run).
- Beats hand layout because: bump-map-aware placement is tedious by hand and exact here.
- Philis status: missing (no die/bump context; grep `die.?edge|bump` finds none).

### H13-49 Separation from power devices (rule 14)
- Kind: rule / constraint
- Statement: power IC = dissipates ≥ 1 W. EXC on power ICs: power device at one die end on an axis, matched devices at the other end on an axis, ≈ ¾ of the way from centre to the far edge (thermal vs stress compromise); consider 2:1–3:1 die; many small CC subarrays. MOD: away from multiwatt devices (opposite end), interdigitated or 2-D CC. MIN: within a few hundred µm of multiwatt devices if interdigitated or cross-coupled. Avoid locations near power-device corners (gradients neither horizontal nor vertical). EXC with even a 250 mW device needs large separation. MOD near small power devices: CC and ≥ 1 µm per mW (½–¼ of that usually suffices). MIN adjacent to small power devices if properly CC.
- Source: §13.3 rule 14; L42569–42589; PDF p.715.
- Philis stage: annotator (heat-source list from oppoint), gp/dp.
- Automation recipe: for each matched group and each device with P ≥ `heat_source_mw`, hard distance d ≥ k_class·P (µm/mW; k_MOD = 1, relax to 0.25–0.5 by deck flag; k_EXC larger or "opposite end"); penalize angular positions near the heater's corners; keep ThermalGradient isotherm cost.
- Beats hand layout because: every (group, heater) pair checked with actual dissipation.
- Philis status: partial — per-device dissipation and a thermal field with a ΔT budget exist (kernel/core/src/layout.rs:33; kernel/core/src/thermal.rs:20; kernel/analog/src/placement/thermal.rs:7-15); no µm/mW or class rule.

### H13-50 No contacts over active gate (rule 16)
- Kind: rule / check
- Statement: never place contacts over the active gate (poly ∩ moat) of matched transistors, even if DRC allows; for annular devices extend gate poly into the field for contacts.
- Source: §13.3 rule 16; L42596–42606; PDF p.715–716.
- Philis stage: cells, verify.
- Automation recipe: verify boolean: contact ∩ (poly ∩ diff) over matched devices = ∅.
- Beats hand layout because: exhaustive geometric check.
- Philis status: implemented in the generator (gate contacts on poly pads beyond the diffusion, kernel/cells/src/mosfet.rs:391-402); no explicit verify check.

### H13-51 Metal over active gates (rule 17)
- Kind: rule / constraint
- Statement: MIN: metal may cross only if the pattern over every section is identical; better, a metal field plate over the whole active area of all devices (M1 plate at min spacing from S/D metal; higher layers may cross the plate). MOD: no field plate and no metal leads crossing active areas. EXC: no field plate, no leads crossing; minimise unnecessary metal within 5–10 µm of the active area; leads passing within 5–10 µm must present a similar adjacent pattern to every section.
- Source: §13.3 rule 17; L42607–42617; §13.2.4 L41789–41798; PDF p.703, 716.
- Philis stage: gr, dr, verify.
- Automation recipe: cells export per-matched-device `active_gate` rects; gr/dr treat them as hard blockages on all layers for MOD/EXC (including the group's own nets), plus a 5–10 µm soft halo for EXC with a symmetry penalty (route crossing the halo must be mirrored for the partner); MIN: allow only if mirrored/identical per section, or insert an M1 plate. Verify: metal ∩ active_gate over MOD/EXC = ∅.
- Beats hand layout because: hard blockages and mirrored halo usage are enforced on every route, not just the ones a human remembers.
- Philis status: partial — gr charges a soft `KEEPOUT_COST` = 2 steps per node over a matched cell to nets that have no pin in it (backend/gr/src/lib.rs:748-757, :938-940); own nets can cross freely, there is no hard block and no verify check.

### H13-52 Dummy-metal (fill) blocking (rule 18)
- Kind: rule / deck-requirement
- Statement: MIN with metal field plates may take fill above the plate. Without a plate, draw dummy-block layers over the whole matched array enclosing all active areas; EXC: block extends 5–10 µm beyond the active gate area in all directions; include block layers for every metal under the protective overcoat; then meet density by custom-crafted identical fill around each member; keep blocked regions few and small (use plates on less-critical pairs).
- Source: §13.3 rule 18; L42618–42624; §13.2.4 L41816–41840; PDF p.703, 716.
- Philis stage: flow (fill), deck (block layer names), verify.
- Automation recipe: fill avoid region = matched array bbox grown by class halo (0 / 0 / 5–10 µm); emit foundry fill-block marker shapes over the same regions on every metal (and poly, H13-55) so downstream foundry fill honours them; synthesize symmetric custom fill around members to restore density (walking-window checked).
- Beats hand layout because: block regions and their compensating symmetric fill are generated together, avoiding the walking-window paradoxes Hastings describes.
- Philis status: partial — Philis' own fill skips matched-cell bboxes (frontend/library/src/lib.rs:416-420; frontend/library/src/fill.rs:44-52, :95) with no halo, no fill-block marker output for foundry fill, block-average density only (fill.rs:49).

### H13-53 Well / deep-diffusion spacing by class (rule 19, with WPE)
- Kind: rule / constraint
- Statement: EXC: well boundary to active gate ≥ 5–10 µm or ≥ 2× well junction depth, whichever is greater; similar for deep-N+ and other deep diffusions. MIN/MOD: layout rules, except WPE-prone low-voltage processes: MOD ≥ 3 µm, MIN ≥ 2 µm from drawn well boundary to active gate. (13.2.2: several µm beyond minimum; beyond the lowest-voltage rule when multiple voltage rules exist.)
- Source: §13.3 rule 19; L42625–42631; §13.2.2 L41233–41247, L41314–41322; PDF p.695–696, 716.
- Philis stage: cells (own well), gp/dp (foreign wells), verify.
- Automation recipe: tiered deck `wpe_clearance_nm` [MIN, MOD, EXC] with EXC = max(tier, 2·well_depth); cells applies it to the device's own well; placement adds a hard spacing between matched NMOS active gates and any foreign N-well shape (and between matched PMOS and their well edge) using the same tier; verify measures it.
- Beats hand layout because: foreign-well proximity is checked for every matched device after placement.
- Philis status: partial — cells inflates matched PMOS N-well by `wpe_clearance_moderate` = 3 µm only (kernel/cells/src/mosfet.rs:626-630); the tiered array [2000, 3000, 5000] and `n_well_depth` 2000 nm (pdks/sky130.json:59-63, :68) are unused; no NMOS-to-foreign-well rule.

### H13-54 Gate extension and metal gate straps (rules 21, 22)
- Kind: rule
- Statement: MOD/EXC: extend every gate (and dummy) 1–2 µm beyond the rule's poly overhang, all equal — no gate extends further than its neighbours. Connect gate fingers with metal, not a poly comb (poly combs acceptable for all but the most accurate if the connecting poly is ≥ 1–2 µm from the moat). MIN exempt.
- Source: §13.3 rules 21, 22; L42638–42647; §13.2.2 corner rounding L41125–41132; PDF p.693, 716.
- Philis stage: cells.
- Automation recipe: class-driven `gate_ext_extra_nm` (0 / 1000–2000 / 1000–2000); for MOD/EXC draw per-finger poly pads and join fingers in M1 (li where applicable) instead of a poly strap; if a poly strap is kept, place it ≥ 1–2 µm from the diffusion.
- Beats hand layout because: uniform extension by construction.
- Philis status: missing — gates use the deck poly extension only (`poly_ext`, kernel/cells/src/mosfet.rs:243) and fingers are joined by a poly strap of `poly_min_width` (mosfet.rs:417-420).

### H13-55 Extraneous poly exclusion (rule 23)
- Kind: rule / constraint
- Statement: EXC: no unconnected poly within 5–10 µm; block generated dummy poly within 5–10 µm (13.2.2 says 10–15 µm for very accurate matching). MOD: same idea out to 3–5 µm. MIN: arbitrary poly adjacent if end dummies exist; without dummies keep other poly ≥ 3–5 µm away.
- Source: §13.3 rule 23; L42648–42654; §13.2.2 L41147–41154; PDF p.693, 716.
- Philis stage: gp/dp (spacing to other devices' poly), flow (poly fill-block marker), verify.
- Automation recipe: per matched macro a class halo (0 or 3–5 µm if no dummies / 3–5 / 5–10 µm) in which no foreign poly may lie: implement as minimum macro-to-macro spacing measured from the matched active area to the neighbour's nearest poly; emit poly fill-block markers over the halo.
- Beats hand layout because: halo enforced against every neighbour and against foundry fill.
- Philis status: missing.

---

## 4. Top-15 priorities for Philis

1. **H13-42 MatchClass × MatchKind data model** — every 13.3 rule is parameterized by class; Philis already has tiered deck arrays that nothing reads (pdks/sky130.json:59-73). Unlocks H13-45…H13-55.
2. **H13-51 Metal over active gates (hard blockage for MOD/EXC, mirrored halo for EXC)** — up to 20% current mismatch (§13.2.4); today only a soft cost for foreign nets (backend/gr/src/lib.rs:748-757).
3. **H13-47 Class-driven dummies and moat extension** — current 1 dummy + 3 µm moat is below Hastings' STI MOD value (5 µm) and far from EXC (10 µm reach).
4. **H13-53 Well spacing by class incl. foreign wells at placement** — WPE data 5% at 1.8 µm, 25% at 0.95 µm; placement-level check missing.
5. **H13-25 / H13-40 Signoff Φ and CC-rule checks** — make orientation, coincidence, symmetry, compactness measurable in verify for all matched groups, not just merged cells.
6. **H13-52 Fill blocking with halo + foundry fill-block markers + symmetric compensating fill** — foundry fill otherwise lands over matched devices after Philis hands off.
7. **H13-54 Gate extension +1–2 µm and metal gate straps for MOD/EXC** — cheap generator change; removes poly-comb microloading.
8. **H13-22 Ratioed devices from unit sections (series/parallel)** — unitization currently cannot unify different-L members.
9. **H13-43 / H13-44 / H13-21 Sizing audit (area for V, length for I)** — report class feasibility before placement; layout cannot fix undersized devices.
10. **H13-46 Aspect-ratio limits per class/kind** — filter/cost on matched-cell variants.
11. **H13-41 Dispersed 2-D CC subarray variants** — AB/BA residual ≈ 60% of ABBA; residual ∝ subarray size².
12. **H13-49 Power-device separation (≥ 1 µm/mW, class-dependent)** — uses existing per-device dissipation.
13. **H13-36 Surrounding-metal similarity metric** — 1% mismatch from unequal nearby fill; numeric cost no human computes.
14. **H13-32 / H13-33 / H13-34 Oppoint audits (V_gst ≥ 100 mV, CLM ΔV_DS, cascode ratio/bulk)** — systematic mismatch terms from the bias point, reported with layout terms.
15. **H13-05 / H13-06 / H13-11 Power-FET metallization and gate-width models** — rule of one-third, bus termination, W_max from gate RC; needed before Philis can place/route integrated switches credibly.
