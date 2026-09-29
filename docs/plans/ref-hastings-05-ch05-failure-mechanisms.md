# Hastings, *The Art of Analog Layout* 3e — Chapter 5: Failure Mechanisms

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson, 2023), chapter 5 (book pp. 210–273).
Reftext file: `/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/hastings.txt`, lines **11865–15989** (read in full).
PDF: `ref/The Art of Analog Layout 3ed 2023 -- Ray Alan Hastings ....pdf`. PDF page = printed book page + 1 in this chapter (the printed page number line `NNN` in the reftext closes book page NNN). The `pdftotext -layout` output drops almost every inline number, unit and equation (they are MathML images), so the PDF pages were read to recover them (see §1).

Entry ID prefix: **H05-NN**. Stage keys: annotator | cells | gp | dp | gr | dr | verify | flow | deck.

---

## 1. Coverage

### 1.1 Read chunks (reftext, Read tool, consecutive)

| # | offset .. end | note |
|---|---|---|
| 0 | 11865 .. 13864 (limit 2000) | refused by the tool (38.8k tokens > 25k cap); re-read in 1000-line chunks below |
| 1 | 11865 .. 12864 | 5 intro, 5.1.1–5.1.4 (to eq. 5.21) |
| 2 | 12865 .. 13864 | 5.1.4 end, 5.1.5, 5.1.6, 5.2, 5.3 intro, 5.3.1, 5.3.2 (part) |
| 3 | 13865 .. 14864 | 5.3.2 end, 5.3.3–5.3.6, 5.4 intro, 5.4.1 |
| 4 | 14865 .. 15864 | 5.4.2–5.4.4, 5.5 (Table 5.5 to "Substrate influence") |
| 5 | 15865 .. 15989 | Table 5.5 end, bibliography, 5.6 exercises, "Chapter 6" header (line 15989) |

Every line 11865–15989 was read.

### 1.2 PDF pages read (numbers/equations the text lost)

PDF pp. 212–220, 221–229, 230–240, 241–246, 252–253, 256–266, 267–270, 273–274. Not opened: PDF 247–251 and 254–255 (their text survived except a few values; the ones used are cross-checked on 252), PDF 271–272 (Table 5.5 text survived intact).

### 1.3 Section / subsection headings in the range (reftext line)

- Chapter 5 Failure Mechanisms (11865); chapter intro (11867)
- 5.1 Electrical Overstress (11883)
  - 5.1.1 Self-Heating (11892) — Failure Mechanisms (11936) — Preventative Measures (12021) — Table 5.1 (12104)
  - 5.1.2 Filamentation (12160) — Failure Mechanisms (12174) — Figure 5.1 (12233) — Preventative Measures (12262)
  - 5.1.3 Electromigration (12278) — Failure Mechanisms (12286) — Preventative Measures (12353) — Figs 5.2 (12380), 5.3 (12464)
  - 5.1.4 Time-dependent Dielectric Breakdown (12528) — Failure Mechanisms (12566) — Fig 5.4 (12616) — Preventative Measures (12731)
  - 5.1.5 Electrostatic Discharge (ESD) (12870) — Figs 5.5 (12928), 5.6 (12974) — Failure Mechanisms (13000) — Preventative Measures (13037) — Fig 5.7 (13054)
  - 5.1.6 The Antenna Effect (13110) — Failure Mechanisms (13118) — Preventative Measures (13193) — Figs 5.8 (13204), 5.9 (13252)
- 5.2 Contamination (13268)
  - 5.2.1 Dry Corrosion (13277) — Failure Mechanisms (13297) — Preventative Measures (13347)
  - 5.2.2 Mobile Ion Contamination (13368) — Failure Mechanisms (13380) — Fig 5.10 (13388) — Preventative Measures (13420) — Fig 5.11 scribe seals (13515)
- 5.3 Surface Effects (13564)
  - 5.3.1 Hot-Carrier Injection in MOS Transistors (13585) — Failure Mechanisms (13606) — Fig 5.12 (13627) — Preventative Measures (13730) — Fig 5.13 (13745)
  - 5.3.2 Zener Walkout and Walkback (13808) — Fig 5.14 (13824) — Failure Mechanisms (13855) — Fig 5.15 (13883) — Preventative Measures (13931) — Fig 5.16 (13969)
  - 5.3.3 Avalanche-Induced Beta Degradation (13987) — Failure Mechanisms (14016) — Preventative Measures (14060)
  - 5.3.4 Negative-Bias Temperature Instability (14079) — Failure Mechanisms (14111) — Preventative Measures (14155)
  - 5.3.5 Parasitic Channels and Charge Spreading (14185) — Table 5.2 (14200) — Failure Mechanisms (14246) — Figs 5.17–5.19 — Preventative Measures for Standard Bipolar Designs (14417) — Figs 5.20–5.24 — Preventative Measures for CMOS and BiCMOS Designs (14627) — Fig 5.25 (14651)
  - 5.3.6 Substrate Influence (14711) — Failure Mechanisms (14720) — Preventative Measures (14731) — Fig 5.26 (14760)
- 5.4 Minority Carrier Injection (14789)
  - 5.4.1 Minority Carrier Injection (14798) — Figs 5.27 (14809), 5.28 (14871)
  - 5.4.2 Latchup (14889) — Fig 5.29 (14916)
  - 5.4.3 Debiasing (15024) — Thin Lightly Doped Layers Atop Heavily Doped Sublayers (15038) — Fig 5.30 (15080) — Thick Lightly Doped Substrates (15159) — Fig 5.31 (15186) — Thin Layers (15245) — Tables 5.3 (15293), 5.4 (15353)
  - 5.4.4 Guard Rings (15420) — Fig 5.32 (15477) — Electron-Collecting Guard Rings (15514) — Figs 5.33–5.35 — Electron-Blocking Guard Rings (15614) — Hole-Collecting Guard Rings (15622) — Figs 5.36, 5.37 — Hole-Blocking Guard Rings (15704) — Fig 5.38 (15714)
- 5.5 Summary (15753) — Table 5.5 (15761)
- Selected Bibliography (15871)
- 5.6 Exercises (15901) — 5.1–5.24

---

## 2. Section-by-section digest

### Chapter intro (L11867–11877; PDF 211)
- Design flaws found by QA after fabrication cost a design pass; some evade detection. Identify flaws as early as possible (L11867–11871).
- The chapter covers the layout-related mechanisms and their mitigation (L11874–11877).

### 5.1 Electrical Overstress (L11883–11888; PDF 212)
- EOS = excessive voltage/current. Four layout-dependent base mechanisms (self-heating, filamentation, electromigration, TDDB) explain ESD damage and the antenna effect.

### 5.1.1 Self-Heating (L11892–12151; PDF 212–215)
- Ambient ranges: commercial 0–70 °C; industrial −40–85 °C; military −55–125 °C; automotive −40 °C up to 85–150 °C. Analog/power junction temperatures up to 125 or 150 °C (L11892–11903; PDF 212).
- Eq. 5.1 `T_J = T_A + θ_JA·P_D`; small SMD packages without heatsinking have θ_JA > 150 °C/W. Eq. 5.2 `T_J = T_C + θ_JC·P_D`; power packages θ_JC < 10 °C/W (L11904–11932; PDF 212).
- Leakage doubles every ~8 °C (1 nA at 125 °C → 1 µA at ~205 °C); parametric shifts above ~175 °C, functional failure above ~200 °C; intrinsic temperature ≈ 325 °C at 10¹⁵ cm⁻³ and ≈ 450 °C at 10¹⁶ cm⁻³ (L11937–11945; PDF 212).
- Thermal shutdown ~25 °C above max Tj (e.g. 125 °C rating, 150 °C shutdown); 100,000 h (11.4 y) life at rated Tj, ~1,000 h at shutdown temperature (L11949–11959; PDF 212–213).
- Arrhenius eqs. 5.3–5.5: `R = k_r·e^(−Ea/kT)`, `t50 = A50·e^(Ea/kT)`, `t2 = t1·exp[(Ea/k)(1/T2 − 1/T1)]`, k = 8.62·10⁻⁵ eV/K; Ea usually 0.5–1.5 eV (L11963–12017; PDF 213).
- Eq. 5.6 (rectangular device L > W, smaller than die thickness ≈ 250 µm): `ΔT = ln(4L/W)/(π·κ·L) · P_D`, κ_Si ≈ 1.3 W/cm/°C; 25 µm square at 100 mW → 14 °C (L12021–12038; PDF 213).
- Deposited resistors: eq. 5.7 `ΔT = I²·t_ox·R_s/(κ·W²)`, κ_ox ≈ 0.011 W/cm/°C; eq. 5.8 `W_min = I_max·√(t_ox·R_s/(κ·ΔT))`; design ΔT ≈ 5 °C, max 50 °C; pulses < ~1 µs are adiabatic, eq. 5.9 `W_min = (I_max/t_R)·√(ρ·τ/(d·c_V·ΔT))`; c_V poly ≈ 1.66 J/°C/cm³, Al 2.42 J/°C/cm³ (L12044–12090; PDF 214).
- Table 5.1 bondwire max current (A), mold k = 0.65 W/m/°C, ΔT = 75 °C, lengths 1/2/5/10 mm: Au 25 µm 1.8/1.3/1.1/1.0; Au 33 µm 2.8/1.9/1.5/1.3; Au 50 µm 5.9/3.6/2.5/2.2; Cu 25 µm 2.2/1.5/1.2/1.1; Cu 33 µm 3.4/2.2/1.7/1.5; Cu 50 µm 7.2/4.3/2.9/2.5. Valid for DC and RMS AC ≥ a few kHz (L12093–12151; PDF 214–215).

### 5.1.2 Filamentation (L12160–12274; PDF 215–217)
- SiO₂ melts at 1600 °C, Si at 1400 °C; Al–Si contact fails at ~580 °C (eutectic). One molten filament destroys the die (L12160–12164; PDF 215).
- Intrinsic conduction localizes current; critical temperature of extrinsic Si > 350 °C, so another mechanism must heat a spot first (L12175–12185).
- Thermal runaway: V_BE tempco ≈ −2 mV/°C; +10–15 °C doubles I_C (footnote: ΔT ≈ 0.03·T_J); hot spots form in tens–hundreds of µs; Spirito effect is the MOS analogue (negative V_T tempco) (L12189–12209; PDF 215).
- Avalanche runaway (electrical filamentation): velocity saturation → space charge → avalanche injection → negative resistance → filament; holes in a few ns, filament < 1 ns; ESD can trigger it. Pulse-width test separates the two (L12215–12258; PDF 216).
- Cure: ballasting (series resistance, negative feedback); Zener zap uses filamentation deliberately (L12263–12274; PDF 216–217).

### 5.1.3 Electromigration (L12278–12519; PDF 217–220)
- Failures seen above ~1·10⁵ A/cm² in Al; voids, hillocks, dendrites (L12278–12290; PDF 217).
- Blech effect: Blech length ≈ 1200 A/cm ÷ J for pure Al on TiN; at 5·10⁵ A/cm² it is 24 µm; strongly process dependent. Nucleation-dominated failure without refractory barrier (L12294–12313; PDF 217).
- Black's law eq. 5.10 `t50 = A50·J⁻ⁿ·e^(Ea/kT)`; n = 2 nucleation-dominated, n = 1 growth-dominated (RBM + W vias, most Cu); n > 2 hints at thermal gradients. Ea Al ≈ 0.7 eV, Cu ≈ 0.8 eV, others 0.5–1.5 eV, high-tin solder ≈ 0.5 eV (L12316–12347; PDF 217).
- Cu-doped Al (0.5–4 %, 0.5 % usual) > 10× life; compressive overcoat helps; bamboo effect only for leads near Al grain size ≈ 0.25 µm; Cu EM at surfaces, cobalt cap; Cu ≥ 5× Al current at 105 °C, but above ~175 °C Al may be better (L12353–12413; PDF 218).
- Typical rule: J_max = 5·10⁵ A/cm² at 105 °C with Ea = 0.7 eV; eq. 5.11 `J2 = J1·exp[(Ea/(n·k))(1/T2 − 1/T1)]`; eq. 5.12 `W_min = I_max/(J·t_min)`, t_min = Al/Cu thickness only (L12416–12449; PDF 218–219).
- Contacts/vias: Al sidewall step coverage thins metal (10 kÅ at 50 % → 5 kÅ); only faces toward the current conduct: an end contact conducts through one face, a contact between two leads through two, and of two vias at a lead end only the front face of the front via (Fig 5.3); with RBM, ignore step coverage; W-plug: voiding where conventional current flows into the plug; RBM below-only plug is worse for upper→lower current (L12452–12490; PDF 219).
- AC > ~10 kHz seldom fails; pulsed eq. 5.13 `t_pulse = t_DC/D` for f < 1/τ, `t_DC/D²` for f > 1/τ, τ > 0.1 ms; 50 % duty → 4× life → an Al lead can be half as wide (L12504–12519; PDF 220).

### 5.1.4 Time-dependent Dielectric Breakdown (L12528–12866; PDF 220–224)
- Gate-oxide dielectric strength ≈ 11 MV/cm; Q_BD scales with gate area; TDDB = delayed rupture after leakage grows (L12528–12562; PDF 220).
- Direct tunneling through 15–20 Å oxides (no damage); trap-assisted ~30 Å; Fowler–Nordheim generates traps; AHI model eq. 5.14 `t50 = τ·e^(G/E)`, G ≈ 350 MV/cm; eq. 5.15 `V = E·t`; McPherson E-model eq. 5.16 `t50 = τ·e^(−γE)`, γ ≈ 2.5–3.5 cm/MV (L12566–12727; PDF 221–222).
- Max stress: dry oxide 300–500 Å ≈ 3.5–4 MV/cm; thinner oxides 4–4.5 MV/cm (for 100,000 h at 125 °C). Field crowds at sharp conductor edges; thicker oxides are somewhat more fragile (L12731–12743, 12809–12810; PDF 222–223).
- OVST: up to 2× max operating voltage, once, ~100 ms; tests a significant fraction of oxides (L12754–12764; PDF 223).
- Oxygen precipitates form above 1000 °C; deep-N+/NBL getter heavy metals within ~100 µm; do not use grown oxide over deep-N+ as a capacitor dielectric; STI cone defects make poly-over-STI weaker than poly-over-LOCOS (L12767–12801; PDF 223).
- Eqs. 5.17 `t2 = t1·e^(G(1/E2 − 1/E1))`; 5.18 `t2 = t1·e^(γ(E1 − E2))`; 5.19 (Weibull, β ≈ 2) `t2 = t1·(A1/A2)^(1/β)`; 5.20 (AHI) `E2 = 1/[1/E1 + ln(A2/A1)/(β·G)]`; 5.21 (McPherson) `E2 = E1 + ln(A1/A2)/(β·γ)`. Use them to run small/low-duty oxides above the global limit (L12809–12866; PDF 223–224).

### 5.1.5 Electrostatic Discharge (L12870–13106; PDF 224–228)
- Body charge ≥ 10 kV; 1 kV can destroy an unprotected IC; handling reduces static to a few hundred volts (L12870–12884; PDF 224).
- HBM 150 pF / 1.5 kΩ; 2 kV standard to 2010, 1 kV now accepted; MM 200 pF, ~750 nH, 200 V, obsolete; field-induced CDM: 0.38 mm FR4, >100 MΩ, 500 V (proposed 250 V); HMM = IEC 61000-4-2 circuit with 330 Ω, typically 8 kV, only for human-accessible pins (L12892–12989; PDF 224–226).
- Damage: oxide rupture or "walking wounded"; bipolar thin emitter oxide under pin-connected leads; silicided S/D removes ballast; filament leakage; resistor robustness ∝ volume (thin-film and poly fragile, wells robust); triggers HCI and beta degradation (L13000–13033; PDF 226).
- Protection network: one ESD device per pin to a reference node (usually substrate ring); any strike crosses ≤ 2 devices; 2 kV HBM ≈ 1.3 A peak; metallization between any two bondpads ≤ ~2 Ω; ESD devices adjacent to pads; common ring around the die; self-protecting power devices by process rule; multiple pads of one pin must be joined by wide on-die metal (CDM); add INP–INM clamp; multiple references need inter-reference clamps (≥ 3 devices per strike) (L13043–13106; PDF 227–228).

### 5.1.6 The Antenna Effect (L13110–13262; PDF 228–230)
- Plasma etch/ash charge injects through thin gate oxide, shifting V_T (PPID) (L13110–13114).
- Poly etch: peripheral antenna ratio = periphery / gate area under it, typical poly limit 100 µm⁻¹; ashing: areal ratio = area / gate area, typical poly limit 500 (L13127–13144; PDF 228).
- For metal-n: ratio per node (electrically connected geometries at that stage) = metal-n perimeter or area of the node / gate-oxide area under the node's poly (L13148–13157; PDF 228).
- Latent antenna: min-spaced neighbours clear late and act as one conductor; write rules with oversize–undersize; P+ poly gates may be more vulnerable (L13166–13179; PDF 228–229).
- Repairs: poly → metal jumper next to the gate (Fig 5.8); lower metal → higher metal jumper; top metal needs junctions (L13194–13200, 13240–13242; PDF 229–230).
- Junction bleed: NSD/substrate clamps negative; clamps positive only if max gate-oxide voltage ≤ ~150 % of NSD/substrate avalanche; avalanche cannot go much below 6 V, so ≤ 3.3 V processes need NSD/substrate **and** PSD/N-well junctions; PSD/N-well works by UV photocurrent at the N-well/substrate junction, needs no metal/poly shadowing and a large enough N-well; metal/diffusion antenna rules are checked only when a metal/gate rule fails (L13215–13230; PDF 229).
- Antenna diodes: NSD = NMoat in substrate; PSD = PMoat in a floating N-well; no metal/poly over it or within a few µm of its edges; dummy-fill block layers over it extended ≥ well junction depth (L13242–13248; PDF 230).

### 5.2 Contamination (L13268–13273; PDF 231)
- Plastic packages admit moisture/contaminants along leads and through bulk; two issues: dry corrosion and mobile ions.

### 5.2.1 Dry Corrosion (L13277–13364; PDF 231–232)
- Only trace water needed; PO openings (pads, probe pads, fuses) and die edges are ingress paths (L13277–13293).
- PSG > ~5 % P forms acids; BPSG and nitride PO resist; halides (Cl) attack Al; brominated flame retardants release Br above 250 °C; red-phosphorus mold compounds failed (L13298–13345; PDF 231).
- Layout measures: minimize number/area of PO openings; test pads on a separate test POR layer closed for production; metal overlaps pad openings on all sides; fuse openings minimal, nothing but the fuse in/next to them; Pd-coated Cu wire (L13348–13364; PDF 232).

### 5.2.2 Mobile Ion Contamination (L13368–13558; PDF 232–235)
- Most ions immobile in SiO₂ below ~500 °C; Li/Na/K mobile at room temperature; Na dominant (L13369–13371; PDF 232).
- Na drifts under gate bias → V_T shift; reversible by 200–250 °C unbiased bake, returns under bias (L13381–13416; PDF 232–233).
- Process cures: P in gate oxide (few-mV polarization "soakage"), P-doped poly gates, chlorinated oxidation; modern fabs < 1 mV shift (L13427–13467; PDF 233).
- Nitride/P-glass PO is the barrier; close test pads for production; place fuses well away from sensitive analog, especially matched MOS (L13469–13486; PDF 233).
- Scribe seal (Fig 5.11): continuous gap-free contact ring with PSD under and metal over (doubles as substrate ring), PO flap-down into the scribe street, continuous via rings between every adjacent metal pair (stackable if superimposed contacts/vias allowed); DI: deep-trench rings with large-radius corner fillets against BOX delamination (L13492–13558; PDF 234–235).

### 5.3 Surface Effects (L13564–13581; PDF 236)
- Interface traps or lateral charge motion; slow parametric shifts under bias; stop when unbiased; partially reversed by 150–300 °C unbiased bake (10 min to hours); low end reverses charge spreading, high end trap effects.

### 5.3.1 Hot-Carrier Injection (L13585–13804; PDF 236–239)
- Barrier: electron 3.1 eV, hole 4.8 eV; thermal mean energy ~30 meV at 25 °C (3/2·kT) (L13586–13590; PDF 236).
- CHC peaks at the drain–backgate junction; gate current 3–5 decades below backgate current; PMOS needs ~2× the field; gate current max at V_GS ≈ 40 % of V_DS; DAHC in avalanche adds (L13607–13685; PDF 236–238).
- Damage: dehydrogenation of interface bonds, electron trapping, bond breakage; NMOS linear g_m drops; V_T first down then up by tens to hundreds of mV (L13689–13726; PDF 238).
- Mitigation: characterize time to 10 % linear-g_m loss (or 10 mV V_T) vs V_DS, V_GS → duty-cycle contour (Fig 5.13, example NMOS: 100 %/10 %/1 %/0.1 % contours over V_DS 0–20 V, V_GS 0–6 V, 100 khr life); comparator pairs at unequal gate voltages develop offset → cascode to equalize V_DS; drain engineering (RESURF, LDD, DDD); deuterium anneal ≥ 10× slower; lengthening L by 0.5–2 µm buys a few volts of margin (L13736–13804; PDF 238–239).

### 5.3.2 Zener Walkout and Walkback (L13808–13983; PDF 239–242)
- Surface Zeners (> 6 V, avalanche) walk out by several hundred mV (Fig 5.14A 6.80 → ~6.89 V); BiCMOS NSD/DWell (0.7 µm) walk out 200–300 mV then back 300–500 mV; extraordinary cases several volts (L13814–13846; PDF 240).
- Mechanisms: hot holes trapped above the anode widen surface depletion; hydrogen compensation of boron (worst near 100 °C); closing PO test-pad openings trapped hydrogen; TiW/TiSi reduce walkout; 200–250 °C bake partially reverses (L13856–13922; PDF 240–241).
- Buried Zeners (breakdown ≥ 1 µm below surface) do not walk; 5–6 V reference diodes; emitter field plate (metal-1 to emitter, extended several µm past the drawn junction) still recommended against charge spreading; independently biased annular plate unproven (L13932–13983; PDF 241–242).

### 5.3.3 Avalanche-Induced Beta Degradation (L13987–14075; PDF 242–243)
- Vertical NPN far more susceptible than lateral PNP; poly-emitter worst; low-current β dropped ~80 % vs 12 % high-current in one discrete NPN; 250 °C/2 min bake reversed 60 %, 300 °C/5 min 95 % (L13988–13996; PDF 242).
- Interface recombination traps above E–B depletion; laterals punch through first; poly-emitter interface is H-passivated (L14017–14050; PDF 243).
- Never reverse-bias B–E beyond ~75 % of V_EBO; poly-emitter never beyond a couple of volts; pin-connected B–E junctions need ESD clamps or redesign (L14061–14075; PDF 243).

### 5.3.4 Negative-Bias Temperature Instability (L14079–14181; PDF 243–245)
- PMOS V_T shifts negative with negative gate bias at temperature, g_m drops; part recovers within seconds even at 25 °C; AC < DC shift; PBTI in NMOS (high-k) and some PMOS (L14085–14107; PDF 243–244).
- Worse with thin oxides, dual-doped poly (surface-channel PMOS), oxynitride; mitigations deuterium (small), fluorine (L14112–14165; PDF 244).
- Layout/circuit rule: find matched PMOS operated at different V_GS; larger ΔV_GS, longer time, higher T → larger mismatch; bias them identically or eliminate (L14176–14181; PDF 245).

### 5.3.5 Parasitic Channels and Charge Spreading (L14185–14698; PDF 245–255)
- Thick-field threshold V_TF per (conductor, backgate) pair raised by field oxide and channel-stop implants. Table 5.2 (20 V double-level-metal N-well CMOS; as printed): Poly NMOS TOX/N-well V_TF > 15 V; Poly PMOS TOX/P-epi < −15 V; Metal-1 NMOS TOX+MLO > 30 V; Metal-1 PMOS < −30 V; Metal-2 NMOS TOX+MLO+ILO > 50 V; Metal-2 PMOS < −50 V. The printed backgate column pairs NMOS with N-well, which looks swapped relative to the device polarity (L14193–14237; PDF 245).
- Six conditions: lightly doped backgate; opposite-type source; opposite-type drain; conductor/charge between; |V_GS| ≥ V_TF (with body effect); V_DS ≠ 0 (L14247–14256; PDF 245).
- Examples: bipolar PMOS (base → isolation under metal), bipolar NMOS (tank → tank over isolation), CMOS PMOS (PSD in N-well → P-epi under poly), CMOS NMOS (N-well → N-well under metal-1) (L14262–14299; PDF 246).
- Charge spreading: surface charge drifts laterally; µA leakages after long bias, bridges P-type regions; nitride PO worse; seen at < 10 V with metal V_TF > 42 V; Na amplification; in HV parts charges sit at PO/mold interface and vanish on decap; seen in < 40 V regulators (L14302–14403; PDF 246–248).
- Bipolar measures: BOI; protect every P region ≥ 75 % of the top-metal PMOS V_TF (≥ 25 % derating: 40 V → protect ≥ 30 V); channel stop overhang ≥ two-level misalignment + 2 × oxide thickness; field plate overhang = outdiffusion + misalignment + fringing (2 × oxide); highest-potential plate covers most; flanges / channel-stop bridges / overlapping metal-2 plates for gaps; partial plates extend well past the V_TF crossing point; lateral-PNP emitter plate overlapping the collector (β shifted > 30 % at < 5 V with V_TF > 40 V); field-plate all P regions above ~2/3 of max top-metal V_TF; inspect leads near P regions above ~2/3 of lowest-metal V_TF (L14417–14623; PDF 248–253).
- CMOS measures: reroute leads that bridge an HV P region to another P region, or move to a higher metal; thick-ox PMOS poly retracted inside the N-well by (depletion intrusion − outdiffusion + misalignment + 2·t_ox) ≈ N-well-over-PSD overlap + 1–2 µm; field plate = min-width poly/lower-metal strip under the lead tied ≥ N-well voltage; channel stop = min-width NMoat bisecting the lead; parasitic NMOS where a lead above NMOS V_TF crosses P-epi/P-well (L14628–14680; PDF 253–254).
- Modern CMOS top-metal V_TF > 50 V; HV charge spreading from mold-compound ions, inversion often > 200 V, but breakdown degrades far earlier; fix with highest-metal field plate, thick power Cu over PO, or semi-insulating PO (L14684–14698; PDF 254).

### 5.3.6 Substrate Influence (L14711–14783; PDF 254–255)
- DI handle acts as a gate through the BOX; floating handle charges and depletes the superficial silicon, lowering breakdown and causing leakage (L14712–14727).
- Tie the handle to the lowest-voltage pin: backside contact (backgrind removes oxide, conductive die attach, downbond or fused leadframe; double downbond detects delamination) or top-side through-silicon vias (+1 mask) (L14732–14783).

### 5.4 Minority Carrier Injection (L14789–14794; PDF 256)
- Parasitic bipolars form wherever minority carriers are injected near reverse-biased junctions; positive feedback → latchup.

### 5.4.1 Minority Carrier Injection (L14798–14885; PDF 256–257)
- Pin above supply → PSD/N-well PNP injects holes into substrate; pin below ground → NSD/P-epi NPN injects electrons into adjacent N-wells (Fig 5.27) (L14799–14826).
- Sources: cable hot-plug, supply sequencing, lightning, inductive kicks, capacitive coupling from fast nodes (L14830–14834).
- Rule: diffusions connected to pins may inject; latchup is usually triggered by negative transients sinking ≥ 100 µA; ESD clamps hold pins near −1 V, so 10 kΩ series usually prevents latchup: "diffusions connected to pins through less than 10 kΩ may trigger latchup"; low-current circuits failed even at 100 kΩ, so many use 50 or 100 kΩ (L14837–14849; PDF 256–257).
- A < 1 pF capacitor from a switching output pulled an NPN collector below ground and latched adjacent CMOS logic lacking guard rings/substrate contacts (Fig 5.28); Schottkies (especially PN-guard-ringed) inject into their cathodes (L14857–14885; PDF 257).

### 5.4.2 Latchup (L14889–15020; PDF 257–259)
- CMOS SCR: lateral NPN (NMOS source/P-epi/N-well) + lateral PNP (PMOS source/N-well/P-epi) with well resistance R1 and substrate resistance R2 (Fig 5.29) (L14901–14912).
- Conditions: both transistors forward/reverse active; β_N·β_P > 1 over some current range; supply sustains the current; persists until power cycled (L14928–14948; PDF 258).
- Triggers are majority-carrier drift drops: well debiasing (R1) and substrate debiasing (R2) (L14951–14956).
- Countermeasures: prevent injection; collect minority carriers; make them recombine; reduce well/substrate resistance (L14960–14972; PDF 259).
- Test: 100 mA for 50 ms, both polarities; ≥ 10 % supply increase = latchup (JESD78A); EMMI localizes (L14975–14994; PDF 259).
- Other SCRs: lateral PNP merged with vertical NPN in one tank; vertical NPN with an integrated field-plated Schottky (L15008–15020; PDF 259).

### 5.4.3 Debiasing (L15024–15416; PDF 259–264)
- Goal: distributed substrate/well/tank resistance ≤ a few ohms (L15025–15029).
- Thin light layer on heavy sublayer (P− epi on P+, N− epi on NBL, N-well to NBL, retrograde well): eq. 5.22 `R_V = ρ·t/A_dif` (A_dif ≫ t²); eq. 5.23 `ΣA_dif < ρ·t/R_V` (as printed); 100 mA and 0.3 V → R_V ≤ 3 Ω → 0.33 mm² at ρ = 10 Ω·cm, t = 10 µm; eq. 5.24 `R_V = ρ/(2π·r_dif)·tan⁻¹(2t/r_dif)`; eq. 5.25 `r_dif = √(A_dif/π)`; disperse array: A_dif = 10 µm², t = 10 µm, ρ = 10 Ω·cm → 13 % of one large equal-area contact, if spaced ≥ 2t; on P+ substrates fill unused area with substrate contact instead of bypass cap if substrate R > 1 Ω; annular contact near a peripheral injector, width benefit tapers beyond t/2; peripheral contacts only help if device size ≲ t. Rules: small contacts anywhere, fill unused area, ring majority injectors (L15038–15156; PDF 259–261).
- Thick lightly doped substrate: eq. 5.26 `R_SP = ρ/(2r_dif)·[1 − (2/π)·sin⁻¹(r_dif/d)]` (10 µm contacts on 10 Ω·cm: 1 kΩ far, 840 Ω at 20 µm c-c); eq. 5.27 annular `R_SP = ρ/(4(r_dif + s_dif + w_dif))·[1 − (2/π)·sin⁻¹(r_dif/(r_dif + s_dif))]` (10 µm dot + 5 µm ring: 110 Ω at 5 µm, 22 Ω at 100 µm spacing — farther is lower); valid to ~200 µm on a 250 µm post-backgrind die → no structure more than 100–200 µm from a contact; contacts within ~100 µm, as large as possible; "closer rings" intuition is wrong here. Rules: contacts within half the die thickness of all devices; make them large; contacts farther than half the die thickness help little (L15159–15241; PDF 261–262).
- Thin (isolated) layers (tank/well without buried layer, SOI, P-well on P−): eq. 5.28 `R_SP = ρ/(π·t)·ln(2d/r_dif)` (t < r_dif); eq. 5.29 `R_SP ≈ ρ/(2r_dif) + ρ/(π·t)·[ln(d/(2t)) − 0.116]` (t > r_dif); Table 5.3; eq. 5.30 annular `R_SP = ρ/(8π·t) + ρ/(2π·t)·ln((s_dif + r_dif)/r_dif) + ρ/(2π(s_dif + r_dif))` (t < r_dif, s_dif > t); Table 5.4; annular contact ≥ t/2 wide gathers most majority current, so always ring majority injectors; minority electrons travel ~800 µm in 10 Ω·cm P-Si, so scatter contacts; rules: annular contacts around majority injectors, as close as possible, width t/2, small contacts ≤ 20–50 t apart (the book heads this list "thick lightly doped substrate", evidently a typo) (L15245–15395; PDF 262–264).
- Standard bipolar: isolation sheet 10–20 Ω/□, vertical ≈ 3–5 kΩ/µm² (as printed); ring majority injectors with substrate contact, minimize metal gaps, scatter contacts (L15399–15416; PDF 264).

### 5.4.4 Guard Rings (L15420–15747; PDF 264–270)
- Minority carriers travel hundreds to thousands of µm; taps cannot stop them; guard rings collect (reverse-biased junction) or block (high-low junction): ECGR, EBGR, HCGR, HBGR (L15421–15446; PDF 264–265).
- Eq. 5.31 permeation `P = I_out/I_in = (A·D_H·τ_L/(W_H·V_L))·(N_L/N_H)`; example: 25 µm square tank, 10 µm deep at 3·10¹⁴, 5 µm NBL at 10¹⁸, τ_L ≈ 0.7 µs, D_H ≈ 4.4 cm²/s → P = 0.0018; effective if N_H/N_L > ~100; LV BiCMOS wells with surface N_L > 10¹⁸ defeat deep-N+ (L15454–15505; PDF 265–266).
- ECGR: all N layers; marginal in standard bipolar (electrons go under through the light substrate) → put adjacent to injector and wide; grounded ring of low-R layers survives 100–200 mA latchup tests only if every portion is strapped to ground by metal (gaps → debias → reinjection); supply-tied ring tolerates more debiasing (power dissipation at high V); ring metal sized for tens–hundreds of mA transients; with P+ substrate 10–100× electron reduction; improved substrate-returned ECGR claims > 10⁶ (one direction); ring segments along the die edge are useless → L/U rings ending at the die edge (L15514–15603; PDF 266–267).
- EBGR needs PBL/deep-P+ sinker; rare (L15614–15618; PDF 268).
- HCGR: P-iso ring in NBL tank; tie to tank potential (debias risk) or ground (limits tank V to P-iso/NBL breakdown); P-bar between lateral PNPs in one tank (ends extend into isolation); retrograde-well PSD HCGR ideally as wide as the well is deep, min width still helps (L15622–15700; PDF 268–269).
- HBGR: deep-N+ ring to NBL, must fully enclose (no gap); drawn NBL to the outer edge of deep-N+; more effective than ECGR in standard bipolar — prefer when space-limited; BiCMOS deep-N+ doping must exceed the N-well by ~100×; DTI-based HBGR: no gaps, NBL drawn beyond the trench outer edge (L15704–15747; PDF 269–270).

### 5.5 Summary (L15753–15866; PDF 270–272)
- Table 5.5 maps each mechanism → symptom → corrective actions (bold = designer-actionable). Layout-actionable items: antenna (jumpers, clamp diodes); beta degradation (limit E–B reverse bias); charge spreading (channel stops, field plates); corrosion (minimize PO openings); debiasing/injection (maximize and scatter contacts, contacts near injectors, guard rings); dielectric breakdown (thicker dielectrics, lower voltages, OVST provision, gettering, no grown oxide over deep-N+ for caps); EM (wider/parallel leads, more contacts/vias, more/larger bondwires); ESD (protection devices, no leads over thin emitter oxide); filamentation (ballast each finger, larger diffusion-to-contact overlap for intrafinger ballast); HCI (limit V_DS, LDD, longer L); mobile ions (minimize PO openings, scribe seals); parasitic channels (BOI, channel stops, field plates); self-heating (more bondwires, wider deposited resistors, diffused instead of deposited); substrate influence (TSV, fused leadframe/downbond); Zener walkout (buried Zeners).

### Selected Bibliography (L15871–15890; PDF 272)
- McPherson 2010 (Arrhenius/TTF), Troutman 1986 (latchup), Takeda et al. 1995 (HCI), White & Bernstein JPL 08-5 (EM/HCI/TDDB), Panasonic T04007BE.

### 5.6 Exercises (L15901–15983; PDF 273–274)
- Numeric exercises confirm the formula usage: TO-220 θ_JC = 4 °C/W (5.1); EM Ea 0.7 eV lifetime at 150 °C (5.2); poly R 3 mA, 20 Ω/□, 1.5 µm oxide, ΔT ≤ 5 °C (5.3); Al lead 1.5 A/100 ns ESD, 1 µm, c 0.9 J/g/°C, 2.7 g/cm³, 2.7 µΩ·cm, 25 → 300 °C (5.4); J 5.5·10⁵ A/cm² at 105 °C, 1.2 µm metal, 10 mA at 125 °C, then 500 mA peak at D = 0.1 (5.6–5.7); 200 Å oxide at 3.8 MV/cm, G = 400 MV/cm, β = 1.7, 1 mm² → 100 µm² (5.8–5.10); top-metal antenna limits areal 1000 and peripheral 400 µm⁻¹ with a 1 µm lead on 0.7 µm² of gate (5.11); channel stop and field plate overhangs with 0.8 µm misalignment, 1.5 µm outdiffusion, 1.2 µm oxide (5.15–5.16); N-well-over-poly with 0.8 µm oxide and 4 µm N-well/PSD overlap (5.20); disperse substrate contacts: 12 Ω·cm, 8 µm epi, 9 µm² contacts, target 3 Ω (5.21); failure-analysis matching (5.24).

---

## 3. Actionable extraction

### H05-01 Junction temperature from package thermal resistance
- Kind: formula + data-model
- Statement: `T_J = T_A + θ_JA·P_D` (eq. 5.1) or `T_J = T_C + θ_JC·P_D` (eq. 5.2). Ambient: commercial 0–70 °C, industrial −40–85 °C, military −55–125 °C, automotive −40 °C to 85–150 °C. Typical max T_J 125 or 150 °C. SMD without heatsink θ_JA > 150 °C/W; power packages θ_JC < 10 °C/W. Life assumption 100,000 h at max rated T_J; ~1,000 h at thermal-shutdown temperature (≈ T_J,max + 25 °C).
- Source: §5.1.1, eqs. 5.1–5.2; L11892–11959; PDF 212–213.
- Philis stage: flow, deck.
- Automation recipe: config fields `ambient_range` (grade enum), `theta_ja_c_per_w` (or θ_JC + T_C), `life_hours` (default 100,000). Total `P_D` = Σ op-point `power_uw`. Compute `T_J,max = T_A,max + θ·P_D` and use it as the temperature for every Arrhenius derating (H05-02, H05-11, H05-18). Output: one die temperature per corner, reported.
- Beats hand layout because: EM and oxide limits get evaluated at the real worst-case junction temperature for every net, not at the deck's rating temperature that designers usually quote.
- Philis status: partial. EM derating accepts one die temperature `temp_k` (`frontend/library/src/elaborate.rs:243-259`), fed from op-point `temp_c` (`frontend/library/src/oppoint.rs:120`); no θ_JA·P_D rise, no grade/ambient model.

### H05-02 Arrhenius acceleration (generic lifetime scaling)
- Kind: formula
- Statement: `R = k_r·e^(−Ea/kT)` (5.3); `t50 = A50·e^(Ea/kT)` (5.4); `t2 = t1·exp[(Ea/k)(1/T2 − 1/T1)]` (5.5); k = 8.62·10⁻⁵ eV/K; T in K (°C + 273); most mechanisms Ea = 0.5–1.5 eV.
- Source: §5.1.1; L11963–12017; PDF 213.
- Philis stage: flow, verify.
- Automation recipe: one shared helper `arrhenius_factor(T, T_ref, Ea)`; each reliability rule carries its own (Ea, T_ref). Report the lifetime factor per mechanism at T_J,max (H05-01).
- Beats hand layout because: consistent temperature scaling across all mechanisms instead of ad-hoc per-rule margins.
- Philis status: implemented for EM only (`kernel/analog/src/routing/em.rs:11-21`, `derate`); not generalized.

### H05-03 Leakage–temperature guard for high-impedance nodes
- Kind: check (heuristic)
- Statement: junction leakage doubles every ~8 °C (1 nA at 125 °C → 1 µA at ~205 °C); µA-level analog circuits shift parametrically above ~175 °C and fail functionally above ~200 °C; Si goes intrinsic at ~325 °C (10¹⁵ cm⁻³) to ~450 °C (10¹⁶ cm⁻³).
- Source: §5.1.1 Failure Mechanisms; L11937–11945; PDF 212.
- Philis stage: annotator, verify.
- Automation recipe: for nets whose op-point bias current is < 10× the extrapolated leakage `I_leak(T) = I_leak(T0)·2^((T−T0)/8)` of all junction area on the net (diffusion area from the drawn layout), flag. Input: diffusion area per net (from cells), deck leakage per area (not in these decks — deck requirement), T_J,max.
- Beats hand layout because: the tool can sum drawn junction area per high-impedance node exactly; humans do not.
- Philis status: missing (no leakage model; grep `leak` finds no rule).

### H05-04 Self-heating of a rectangular power device
- Kind: formula
- Statement: for a device L × W (L > W) smaller than the die thickness (~250 µm): `ΔT = ln(4L/W)/(π·κ·L)·P_D`, κ_Si ≈ 1.3 W/cm/°C. Example: 25 µm square at 100 mW → 14 °C.
- Source: §5.1.1 Preventative Measures, eq. 5.6; L12021–12038; PDF 213.
- Philis stage: gp, dp (thermal field), verify.
- Automation recipe: replace the self term of the thermal field with eq. 5.6 using the device's placed footprint (W = min side, L = max side); keep the point-source mutual term for other devices. Use the per-device T for EM derating of the wires attached to it.
- Beats hand layout because: every powered device's own hot spot is quantified and propagated to matching and EM checks on each move.
- Philis status: partial. Thermal field uses `P/(2π·k·r)` with the self term at `r = max(hw, hh)` (`kernel/core/src/thermal.rs:29-48`); k = 148 W/m/K (`thermal.rs:14`). Not eq. 5.6; no per-conductor temperature fed to EM (`elaborate.rs:246-247` ponytail).

### H05-05 Minimum width of deposited resistors from self-heating
- Kind: formula + rule
- Statement: `ΔT = I²·t_ox·R_s/(κ_ox·W²)` (5.7); `W_min = I_max·√(t_ox·R_s/(κ_ox·ΔT))` (5.8); κ_ox ≈ 0.011 W/cm/°C. Poly and thin-film resistors are EM-resistant, so self-heating is their only current limit. Design ΔT ≈ 5 °C (to avoid gradients that invalidate nearby EM calculations); never more than 50 °C (mechanical stress). Larger ΔT allowed for infrequent pulses.
- Source: §5.1.1, eqs. 5.7–5.8; L12044–12073; PDF 214.
- Philis stage: annotator (current), cells (width), verify.
- Automation recipe: for each poly/thin-film/metal resistor, take the op-point RMS current (device `id_ua`), deck `R_s` and field-oxide thickness under the resistor (deck requirement), κ_ox = 0.011 W/cm/°C, ΔT = 5 °C default → set `unit_w ≥ W_min` before unitization. Hard constraint in cells; report ΔT.
- Beats hand layout because: every resistor gets its width from its actual current; designers usually use a blanket width.
- Philis status: missing. Resistor width is `max(unit_w, res_min_width)` only (`kernel/cells/src/resistor.rs:81`).

### H05-06 Adiabatic pulse sizing (ESD / short pulses)
- Kind: formula
- Statement: for pulses shorter than ~1 µs, heat does not escape: `W_min = (I_max/t_R)·√(ρ·τ/(d·c_V·ΔT))` (5.9, as printed), t_R = conductor thickness, ρ resistivity, d density, c_V volumetric specific heat: poly ≈ 1.66 J/°C/cm³, Al ≈ 2.42 J/°C/cm³.
- Source: §5.1.1, eq. 5.9; L12076–12090; PDF 214. Exercise 5.4 (L15912–15914; PDF 273) applies it to an Al lead at 1.5 A/100 ns.
- Philis stage: dr, verify.
- Automation recipe: nets tagged as pin/ESD paths (H05-47) get a pulse spec (e.g. 2 kV HBM → 1.3 A, ~100 ns); router enforces the eq. 5.9 width on every segment and resistor on that path. Hard.
- Beats hand layout because: ESD path widths computed per segment, including jogs and vias, instead of a rule-of-thumb width.
- Philis status: missing.

### H05-07 Bondwire current capacity
- Kind: data-model (table)
- Statement: Table 5.1 (mold k 0.65 W/m/°C, ΔT 75 °C), max A at 1/2/5/10 mm: Au 25 µm 1.8/1.3/1.1/1.0; Au 33 µm 2.8/1.9/1.5/1.3; Au 50 µm 5.9/3.6/2.5/2.2; Cu 25 µm 2.2/1.5/1.2/1.1; Cu 33 µm 3.4/2.2/1.7/1.5; Cu 50 µm 7.2/4.3/2.9/2.5. DC and RMS AC ≥ few kHz.
- Source: §5.1.1, Table 5.1; L12093–12151; PDF 214–215.
- Philis stage: flow (top level only).
- Automation recipe: if Philis ever assembles a pad ring, pads per supply = ⌈I_supply / I_wire(length, diameter)⌉. Out of scope for block P&R.
- Beats hand layout because: n/a at block level.
- Philis status: missing / out of scope (no pad or bondwire model; grep `bondwire` none).

### H05-08 Ballasting against filamentation
- Kind: rule
- Statement: any positive-feedback current localization (intrinsic conduction above ~350 °C, thermal runaway, avalanche runaway in ns) is stopped by series resistance per current path ("ballasting"). Silicided S/D removes drain ballast, so silicided MOS in ESD paths are vulnerable. Table 5.5: ballast each finger; increase diffusion overlap of contact for intrafinger ballast.
- Source: §5.1.2 Preventative Measures, §5.1.5 Failure Mechanisms, Table 5.5; L12263–12266, 13011–13014, 15825–15826; PDF 216, 226, 271.
- Philis stage: cells.
- Automation recipe: for devices tagged pin-connected output/ESD or power (op-point current above a threshold), the mosfet generator draws a silicide-block region and a drain contact-to-gate spacing from the deck's ESD rules; per-finger source resistor for bipolar power arrays. Deck requirement: silicide-block layer and ESD spacing rules.
- Beats hand layout because: applied to every finger of every flagged device.
- Philis status: missing (grep `ballast` only hits an unrelated test comment at `kernel/analog/src/placement/symmetry.rs:349`).

### H05-09 Thermal runaway sensitivity of bipolar arrays
- Kind: heuristic
- Statement: V_BE tempco ≈ −2 mV/°C; +10–15 °C doubles I_C at constant V_BE (ΔT ≈ 0.03·T_J); hot spots form in tens–hundreds of µs; beta rolloff may stabilize them. Spirito effect: MOS analogue at very high g_m.
- Source: §5.1.2; L12189–12209; PDF 215.
- Philis stage: annotator, gp/dp.
- Automation recipe: for multi-finger/multi-emitter BJT arrays and high-g_m power MOS, bound the intra-array ΔT (from the thermal field) to ≪ 0.03·T_J and require emitter/source ballast (H05-08). Budget constraint on ΔT between fingers.
- Beats hand layout because: ΔT across fingers is computed for the actual placement.
- Philis status: partial. Pairwise ΔT rule exists (`kernel/analog/src/placement/thermal.rs:9-39`); no intra-array runaway criterion.

### H05-10 Black's law parameters per metal system
- Kind: formula + deck-requirement
- Statement: `t50 = A50·J⁻ⁿ·e^(Ea/kT)` (5.10). n = 2 nucleation-dominated (Al without barrier); n = 1 growth-dominated (Al with RBM + W vias, most Cu); n > 2 suggests thermal gradients. Ea: Al ≈ 0.7 eV, Cu ≈ 0.8 eV, others 0.5–1.5 eV, high-tin solder ≈ 0.5 eV. Cu carries ≥ 5× Al current at 105 °C but above ~175 °C Al may be better.
- Source: §5.1.3; L12316–12347, 12409–12413; PDF 217–218.
- Philis stage: deck.
- Automation recipe: deck sidecar field `(T_ref, Ea, n)` per metal; default by metal family if missing (Al 0.7 eV/n = 2, Cu 0.8 eV/n = 1), flagged as assumed.
- Beats hand layout because: the correct n and Ea change the allowed current at temperature by large factors.
- Philis status: implemented as optional deck data (`backend/verify/src/pdk.rs:23-35`, `derating: Option<(T_ref, Ea, n)>`); no family defaults.

### H05-11 EM current-density temperature derating
- Kind: formula
- Statement: `J2 = J1·exp[(Ea/(n·k))·(1/T2 − 1/T1)]` (5.11). Typical Cu-doped Al rule: 5·10⁵ A/cm² at 105 °C, Ea = 0.7 eV.
- Source: §5.1.3; L12416–12432; PDF 218–219.
- Philis stage: dr, verify.
- Automation recipe: derate every layer and cut limit to T_J,max (H05-01) or to the local conductor temperature (H05-04).
- Beats hand layout because: automatic and per corner.
- Philis status: implemented (`kernel/analog/src/routing/em.rs:11-21`; applied in `frontend/library/src/elaborate.rs:247-259`), die temperature only.

### H05-12 EM minimum lead width
- Kind: formula + check
- Statement: `W_min = I_max/(J·t_min)` (5.12); t_min counts only the Al/Cu, not TiW/TaN barrier layers. Remedies (Table 5.5): widen leads, parallel leads, more contacts/vias.
- Source: §5.1.3; L12435–12449, 15812–15815; PDF 219, 271.
- Philis stage: dr, verify.
- Automation recipe: per-segment current from the routed tree (sum of terminal currents on one side), width ≥ I/J per layer; hard rule.
- Beats hand layout because: every segment sized from its own tree current.
- Philis status: implemented. `Limit::width_nm` (`kernel/analog/src/routing/em.rs:41-58`), hard rule per routed net (`frontend/library/src/lib.rs:986-1025`); per-segment sizing in dr (`em.rs:79-83` doc; `backend/dr/src/lib.rs:657`).

### H05-13 Blech immortality
- Kind: formula
- Statement: leads shorter than the Blech length do not electromigrate; for pure Al on TiN, L_B ≈ 1200 A/cm ÷ J; at J = 5·10⁵ A/cm², L_B = 24 µm. Strongly process dependent. Analog EM rules usually ignore it.
- Source: §5.1.3; L12294–12301, 12416–12420; PDF 217–218.
- Philis stage: dr, deck.
- Automation recipe: allow `w < I/J` when `(I/w)·L < (jL)_B` from the deck; the domain L is the whole connected run on that layer between vias/branches.
- Beats hand layout because: short stubs are not over-widened.
- Philis status: implemented when the deck gives a Blech product (`kernel/analog/src/routing/em.rs:31-32, 53-55`; `backend/verify/src/pdk.rs:30-31`).

### H05-14 Contact/via EM: only faces toward the current count
- Kind: rule + algorithm
- Statement: current flows through the via/contact sidewall(s) facing the current. An end-of-lead contact conducts through one face; a contact between two leads through two faces; for a pair of vias at a lead end, only the front face of the front via conducts much (Fig 5.3). Sidewall metal is thinned by step coverage (10 kÅ at 50 % → 5 kÅ). With a refractory barrier, step coverage can be ignored. W plugs: current enters along the faces facing the current, through the full metal depth beneath those faces.
- Source: §5.1.3; L12452–12485; PDF 219.
- Philis stage: dr, verify.
- Automation recipe: when dr places a via array for current I, arrange the cuts in a row perpendicular to the current (all on the leading edge), not in depth along the lead. Count effective cuts = cuts on the leading row (plus both rows for a pass-through contact). Require `effective_cuts ≥ ⌈I/I_cut⌉`. Flag arrays whose extra rows sit behind the first row as "non-contributing".
- Beats hand layout because: via arrays are shaped by current direction; humans often draw square arrays and count all cuts.
- Philis status: partial. Cut count `n = ⌈I/I_cut⌉` treats every cut as equal (`kernel/analog/src/routing/em.rs:60-71`).

### H05-15 W-plug via polarity asymmetry
- Kind: deck-requirement + check
- Statement: voiding occurs where conventional current flows into a tungsten plug (electron wind leaves the Al there). A plug with refractory barrier below but not above is more susceptible for current from the upper metal to the lower metal. Asymmetry confirmed experimentally.
- Source: §5.1.3; L12482–12490; PDF 219.
- Philis stage: deck, dr.
- Automation recipe: deck field `via_em_direction_factor[up|down]`; dr uses the op-point current sign per via to pick the limit.
- Beats hand layout because: direction-aware sizing on every via.
- Philis status: missing.

### H05-16 Pulsed and AC electromigration
- Kind: formula
- Statement: pure AC above ~10 kHz rarely fails. Unidirectional pulses with duty D: `t_pulse = t_DC/D` for f < 1/τ, `t_DC/D²` for f > 1/τ (5.13), τ (vacancy recombination) > 0.1 ms. At 50 % duty the life quadruples; with n ≈ 2 an Al lead can be half as wide.
- Source: §5.1.3; L12504–12519; PDF 220. Exercise 5.7 (L15922–15923; PDF 273).
- Philis stage: annotator, dr.
- Automation recipe: per net, current waveform class from op-point/transient: DC, pulsed (I_peak, D, f), AC. Required width for equal life (derived here from eq. 5.13 with Black's `t ∝ J⁻ⁿ`; the book states only the n = 2, D = 0.5 → half-width case): `W ≥ I_peak·D^(1/n)/(J·t_min)` for f < 1/τ, and `W ≥ I_peak·D^(2/n)/(J·t_min)` for f > 1/τ (n = 2, D = 0.5, f > 1/τ → W halves, matching the text). AC nets above 10 kHz: exempt from EM (keep self-heating).
- Beats hand layout because: switched-capacitor/clock nets are sized from duty cycle, not peak.
- Philis status: missing ("DC average only" at `kernel/analog/src/routing/em.rs:90`).

### H05-17 Net voltage envelope (data model required by several checks)
- Kind: data-model
- Statement: TDDB (V = E·t across each oxide), parasitic-channel conditions (|V_lead − V_P| ≥ V_TF), HCI (V_DS, V_GS per device), BTI (V_GS mismatch), beta degradation (V_EB reverse) and antenna-diode clamping all need per-net DC voltage and, ideally, its min/max over operation.
- Source: implied by §5.1.4 (L12731–12743), §5.3.1 (L13736–13761), §5.3.3 (L14068–14070), §5.3.4 (L14176–14181), §5.3.5 (L14247–14256); PDF 222–253.
- Philis stage: flow (oppoint), annotator.
- Automation recipe: extend the op-point result with `net_v: Vec<f64>` (and optional `net_vmin/vmax` from corners/transient), and per-device `vgs, vds, vbs`. Default envelope for supplies from `OpConfig.vdd`.
- Beats hand layout because: all voltage-dependent reliability checks become exhaustive graph queries.
- Philis status: missing. `OpPoint` carries `power_uw`, `id_ua`, `headroom_mv`, `gm_us` only (`frontend/library/src/oppoint.rs:13-28`).

### H05-18 Oxide field limit (TDDB)
- Kind: check
- Statement: `V = E·t` (5.15). Max stress for 100,000 h at 125 °C: dry oxide 300–500 Å ≈ 3.5–4 MV/cm; thinner oxides 4–4.5 MV/cm. Dielectric strength ≈ 11 MV/cm (instant). Field crowds at sharp conductor edges; large safety margins needed.
- Source: §5.1.4; L12533, 12666–12680, 12741–12743, 12809–12810; PDF 220–223. Exercise 5.8 (L15924–15925).
- Philis stage: verify, annotator.
- Automation recipe: for every MOS gate (V_G − V_channel) and every capacitor/MOS-cap/poly-over-field crossing, `E = |ΔV|max / t_ox(deck)`; fail if E > E_max(t_ox). Also metal-over-metal lateral/vertical dielectric for HV nets. Hard.
- Beats hand layout because: every gate and capacitor plate pair is checked at its real voltage.
- Philis status: missing (no oxide-thickness or voltage data; grep `tddb`, `gate_ox` none).

### H05-19 TDDB time and area scaling
- Kind: formula
- Statement: AHI: `t2 = t1·e^(G·(1/E2 − 1/E1))` (5.17), G ≈ 350 MV/cm. McPherson: `t2 = t1·e^(γ(E1 − E2))` (5.18), γ ≈ 2.5–3.5 cm/MV (∝ 1/T). Weibull: `t2 = t1·(A1/A2)^(1/β)` (5.19), β ≈ 2. Field vs area: AHI `E2 = 1/[1/E1 + ln(A2/A1)/(β·G)]` (5.20); McPherson `E2 = E1 + ln(A1/A2)/(β·γ)` (5.21). Use to run small-area or low-duty oxides above the global limit.
- Source: §5.1.4; L12666–12702, 12809–12866; PDF 222–224. Exercises 5.9–5.10.
- Philis stage: verify.
- Automation recipe: in H05-18, scale E_max by each dielectric's area (sum per net-pair) and duty; large MOS caps get a lower limit, small gates a higher one.
- Beats hand layout because: area-aware limits for every capacitor; hand work uses one number.
- Philis status: missing.

### H05-20 Poly over STI is weaker than poly over LOCOS
- Kind: rule
- Statement: STI trench cone defects greatly reduce field-oxide integrity under poly; metal is not affected (extra MLO under it); poly over LOCOS can run at much higher voltage than poly over STI of similar thickness.
- Source: §5.1.4 Preventative Measures; L12795–12801; PDF 223.
- Philis stage: dr, verify.
- Automation recipe: in STI decks, forbid (hard) or cost high-voltage nets routed in poly over field; route HV crossings in metal.
- Beats hand layout because: layer choice per net by voltage, enforced everywhere.
- Philis status: missing.

### H05-21 Gettering and capacitor-dielectric placement
- Kind: rule + deck-requirement
- Statement: deep-N+ and NBL getter heavy metals; code them within ~100 µm of gate oxides to improve GOI where the process lacks oxygen precipitation (some DI processes mandate dummy deep-N+ strips near MOS). Do not use grown oxide over deep-N+ as a capacitor dielectric.
- Source: §5.1.4; L12767–12787; PDF 223.
- Philis stage: deck, cells, verify.
- Automation recipe: when a deck declares `getter_layer` and `getter_distance_um` (100 µm), check every gate within reach; insert strips in whitespace. Reject capacitor constructions of oxide over deep-N+.
- Beats hand layout because: coverage checked for every gate.
- Philis status: missing / not applicable to the four current decks (bulk CMOS).

### H05-22 ESD protection network topology
- Kind: check (algorithm)
- Statement: one ESD device per pin to a common reference node (usually the substrate ring); a pin that is the reference needs none; any pin-to-pin strike crosses ≤ 2 devices; extra clamp between differential inputs (INP–INM); multiple references (AGND/DGND) need clamps between references (a strike may then cross ≥ 3 devices, which must be larger). Self-protecting devices per process rules. Pads of one pin must be joined by wide on-die metal (CDM).
- Source: §5.1.5 Preventative Measures; L13043–13093; PDF 227.
- Philis stage: annotator, verify.
- Automation recipe: build a graph of pins and ESD devices (recognized by pattern); for each pin pair, shortest path through ESD devices; fail if a pin has no device to its reference domain, or a domain pair lacks an inter-reference clamp; flag differential input pairs without a cross clamp.
- Beats hand layout because: all pin pairs are enumerated.
- Philis status: missing at block level. The annotator recognizes a diode-connected ESD clamp pattern (`backend/annotator/src/catalog.rs:1811-1815`) but has no pin/pad role (`backend/annotator/src/netrole.rs:11-16`).

### H05-23 ESD current-path resistance
- Kind: check
- Statement: a 2 kV HBM strike peaks at ~1.3 A; to keep drops ≤ a couple of volts, the metallization resistance between any two bondpads must not exceed ~2 Ω. ESD devices sit adjacent to their pads with short wide leads; the reference node is a die-perimeter ring.
- Source: §5.1.5; L13065–13069; PDF 227.
- Philis stage: dr, verify.
- Automation recipe: for pin-tagged nets, compute least resistance pin → ESD device → reference ring with the router's resistor network; hard cap R ≤ 2 Ω·(1 kV/V_HBM target scaling is not given; use the 2 Ω/1.3 A pair). Widen or add parallel straps until met.
- Beats hand layout because: extracted path resistance, not estimated.
- Philis status: missing. A least-R port graph exists in the routing stack (`kernel/analog/src/routing/stack.rs:318-340`, `PortGraph::from`) that could compute it.

### H05-24 ESD qualification targets as flow config
- Kind: data-model
- Statement: HBM 150 pF/1.5 kΩ, 1 kV accepted since 2010, 2 kV often still demanded; MM (200 pF, ~750 nH, 200 V) obsolete; CDM 500 V (proposed 250 V); HMM 330 Ω, 8 kV typical, only human-accessible pins.
- Source: §5.1.5; L12892–12989; PDF 224–226.
- Philis stage: flow.
- Automation recipe: config `esd: {hbm_v, cdm_v, hmm_pins}` → peak currents for H05-06/H05-23 (HBM peak ≈ V/1.5 kΩ; 2 kV → 1.3 A given).
- Beats hand layout because: one source of truth for all ESD sizing.
- Philis status: missing.

### H05-25 ESD-sensitive layout details
- Kind: rule
- Statement: do not route pin-connected metal over thin emitter oxide unless it connects to that emitter (or use thick emitter oxide); resistors in pin paths fail by volume — thin-film and poly are fragile, deep lightly doped diffusions (wells) are robust; base–emitter junctions on pins need clamps (§5.3.3).
- Source: §5.1.5 Failure Mechanisms; L13004–13008, 13025–13028; §5.3.3 L14073–14075; PDF 226, 243.
- Philis stage: annotator, cells, gr/dr.
- Automation recipe: pin-tagged nets get a keep-out over thin-oxide regions of other devices (router obstacle); resistors in series with a pin: prefer well-resistor construction when the model allows, else check volume (H05-06).
- Beats hand layout because: keep-outs are enforced on every pin net.
- Philis status: missing.

### H05-26 Antenna ratios per node per etch stage
- Kind: check
- Statement: poly etch: peripheral ratio = periphery / gate area, typical limit 100 µm⁻¹; ashing: areal ratio = area / gate area, typical poly limit 500. For metal-n: node = everything electrically connected when metal-n is etched; ratio = metal-n perimeter (or area) of the node / gate-oxide area under the node's poly. Each layer has its own peripheral and areal limits. Example (Exercise 5.11): top metal areal 1000, peripheral 400 µm⁻¹, 1 µm lead to 0.7 µm² of gate → areal allows 700 µm, peripheral allows 2(L + 1) ≤ 280 → L ≤ 139 µm (derived here from the exercise data; the book gives no answer).
- Source: §5.1.6; L13127–13157; PDF 228; L15930–15933; PDF 273.
- Philis stage: dr, verify.
- Automation recipe: per gate net, at each etch stage, connected pieces of layers ≤ stage; exposed area or sidewall per the deck; divide by the gate area the piece reaches; hard.
- Beats hand layout because: exact per-stage connectivity for every gate net.
- Philis status: implemented. `Stack::antenna` (`kernel/analog/src/routing/stack.rs:270-316`), sidewall/peripheral mode via `antenna_sidewall_nm` (`frontend/library/src/elaborate.rs:265-288`), rule `kernel/analog/src/routing/antenna.rs:8-23`. Known simplification: a piece is charged the net's whole gate area (`stack.rs:280-282`).

### H05-27 Antenna repair by jumpers
- Kind: algorithm
- Statement: poly antenna: insert a short metal-1 jumper next to the gate so the long poly is split off (Fig 5.8). Lower-metal antenna: jumper through a higher metal near the gate. Top metal cannot be jumpered; it needs a junction (H05-29).
- Source: §5.1.6 Preventative Measures; L13194–13200, 13240–13242; PDF 229–230.
- Philis stage: dr.
- Automation recipe: rip up the violating net and reroute with the stage's layer penalized near the gate; jumper as close to the gate as possible.
- Beats hand layout because: automatic, then rechecked.
- Philis status: implemented (`backend/dr/src/lib.rs:1384` layer-steer cost; test `backend/dr/src/lib.rs:2393-2397`).

### H05-28 Junction bleed credit (which junctions protect)
- Kind: rule
- Statement: an NSD/substrate junction on the node clamps negative excursions; it clamps positive ones only if the max gate-oxide operating voltage ≤ ~150 % of the NSD/substrate avalanche voltage. Avalanche cannot be much below 6 V, so processes at ≤ 3.3 V need both NSD/substrate and PSD/N-well junctions (the latter works by UV photocurrent at the N-well/substrate junction, needs no metal/poly shadow and a large enough N-well). Many decks check metal/diffusion rules only when a metal/gate rule fails.
- Source: §5.1.6 Preventative Measures; L13215–13230; PDF 229.
- Philis stage: dr, verify, deck.
- Automation recipe: per node, credit bleed only from junctions reached at or below the current stage; for ≤ 3.3 V decks require both polarities (or the deck's diode rule); apply the deck's metal/diffusion ratio as the second test.
- Beats hand layout because: stage-accurate and polarity-accurate credit.
- Philis status: partial. Any contact to the deck's `diode_layer` zeroes the ratio at every stage (`kernel/analog/src/routing/stack.rs:286-290`, ponytail notes the stage simplification); no polarity or UV condition.

### H05-29 Antenna diode construction
- Kind: rule (cells) + check
- Statement: NSD diode = NMoat in the substrate (Fig 5.9A). PSD diode = PMoat inside a floating N-well (Fig 5.9B); it relies on UV reaching the N-well/substrate junction, so no metal or poly over it or within a few µm of its edges except its own lead; with dummy fill, draw fill-block layers over it extended beyond it by ≥ the well junction depth.
- Source: §5.1.6; L13242–13248; PDF 230.
- Philis stage: cells, dr, deck (fill).
- Automation recipe: diode generator emits the diode plus a keep-out/fill-block marker (size = diode + max(few µm, well depth)); router and fill honor it.
- Beats hand layout because: keep-outs cannot be forgotten.
- Philis status: partial. dr inserts a diode when jumpers fail (`frontend/library/src/elaborate.rs:322-330`; `frontend/library/tests/antenna_diode.rs:1-3`); no PSD/floating-well variant, no UV keep-out; metal fill has no block markers (`frontend/library/src/fill.rs:1-7`).

### H05-30 Latent antenna
- Kind: check
- Statement: min-spaced neighbours clear late in etch and act as one antenna even if they are separate nodes at the end; model by oversize–undersize (merge geometries closer than a threshold) before computing ratios.
- Source: §5.1.6; L13166–13172; PDF 228.
- Philis stage: verify, dr.
- Automation recipe: in `Stack::antenna`, union shapes of the same layer whose spacing ≤ `latent_merge_nm` (deck; default = min spacing) across nets at that stage; charge the union to every gate under it.
- Beats hand layout because: invisible to per-net checks; automation catches it.
- Philis status: missing (pieces are per-net only, `kernel/analog/src/routing/stack.rs:284-299`).

### H05-31 Protective-overcoat openings and fuses
- Kind: rule
- Statement: minimize number and area of PO openings; test pads on a separate test-POR layer closed for production; metal overlaps pad openings on all sides; fuse openings minimal, nothing but the fuse inside or adjacent; fuses well away from sensitive analog, especially matched MOS (mobile-ion ingress).
- Source: §5.2.1 L13348–13359; §5.2.2 L13475–13486; PDF 232–233.
- Philis stage: gp, verify.
- Automation recipe: treat fuse/PO-opening cells as aggressors with a keep-out radius to matched devices (radius not given in the source — config).
- Beats hand layout because: exhaustive distance check to every matched device.
- Philis status: missing.

### H05-32 Scribe seal / seal ring
- Kind: rule (chip level)
- Statement: continuous, gap-free contact ring around the active area with PSD under and metal over (doubles as the substrate ring); PO flap-down into the scribe street; continuous via rings between every adjacent metal pair (stacked if allowed); DI: deep-trench rings with large-radius fillets at corners.
- Source: §5.2.2; L13500–13558; PDF 234–235.
- Philis stage: flow (top level).
- Automation recipe: seal-ring generator from the deck's layer stack. Out of scope for block P&R.
- Beats hand layout because: n/a at block level.
- Philis status: missing / out of scope (grep `seal_ring` none).

### H05-33 Hot-carrier limits and matched-pair V_DS equalization
- Kind: check
- Statement: CHC injection needs V_DS above a critical value; gate current peaks at V_GS ≈ 0.4·V_DS; PMOS needs ~2× the NMOS field. Characterize allowed duty vs (V_DS, V_GS) for 10 % linear-g_m loss or 10 mV ΔV_T (Fig 5.13). Matched transistors at different gate/drain voltages (e.g. comparator input pairs) degrade differently → offset; equalize V_DS with cascodes.
- Source: §5.3.1; L13663–13666, 13736–13761; PDF 237–239.
- Philis stage: annotator, verify.
- Automation recipe: for every matched set from the annotator, compare op-point V_DS (and V_GS) across members; flag |ΔV_DS| above a config threshold (not given in the source) when V_DS exceeds the deck's HCI onset (deck requirement: HCI duty table or V_DS,max); report as a design finding (layout cannot fix it).
- Beats hand layout because: every matched group is screened at its op point.
- Philis status: missing (no per-device V_DS/V_GS in `frontend/library/src/oppoint.rs:13-28`; grep `hci`, `hot_carrier` none).

### H05-34 Channel-length margin for hot carriers
- Kind: heuristic
- Statement: HCI occurs only near the drain; lengthening L by 0.5–2 µm usually buys a few volts of margin. Alternatives: LDD/DDD, RESURF, deuterium anneal (≥ 10×).
- Source: §5.3.1; L13765–13804; PDF 239.
- Philis stage: annotator (sizing advice).
- Automation recipe: devices with V_DS above the deck's rated value get a finding recommending +0.5–2 µm L or an extended-drain model. Advisory.
- Beats hand layout because: consistent screening.
- Philis status: missing.

### H05-35 BTI-driven mismatch of matched PMOS
- Kind: check
- Statement: NBTI shifts PMOS V_T negative under negative gate bias at temperature (partly recovers in seconds, AC < DC). Matched PMOS at different V_GS mismatch over time; larger ΔV_GS, longer time and higher T make it worse. Bias identically or eliminate.
- Source: §5.3.4; L14085–14100, 14176–14181; PDF 243–245.
- Philis stage: annotator, verify.
- Automation recipe: for each matched PMOS group, flag unequal op-point V_GS (and stress duty) above a threshold (not given; config). Advisory.
- Beats hand layout because: every matched PMOS group is checked.
- Philis status: missing (grep `nbti` none).

### H05-36 Zener references: buried Zeners and emitter field plates
- Kind: rule
- Statement: surface Zeners (> 6 V) walk out several hundred mV (BiCMOS NSD/DWell: +200–300 mV then −300–500 mV; extraordinary: several volts). Buried Zeners (breakdown ≥ 1 µm deep) do not walk. Base–emitter Zeners: emitter field plate of metal-1 tied to the emitter, extended several µm past the drawn junction (move the base contact back if needed).
- Source: §5.3.2; L13814–13846, 13932–13960; PDF 240–242.
- Philis stage: annotator, cells.
- Automation recipe: diodes used as references (annotator pattern) with a surface-breakdown model → finding; the diode generator adds an emitter field plate when requested.
- Beats hand layout because: n/a beyond consistency.
- Philis status: missing (`kernel/cells/src/diode.rs` has no field plate; grep `field_plate` none).

### H05-37 Emitter–base reverse-bias limit
- Kind: check
- Statement: never reverse-bias a B–E junction beyond ~75 % of V_EBO; poly-emitter devices never beyond a couple of volts; pin-connected B–E junctions need ESD clamps. Low-current β can drop ~80 %.
- Source: §5.3.3; L13992–13993, 14068–14075; PDF 242–243.
- Philis stage: annotator, verify.
- Automation recipe: for every Npn/Pnp, op-point V_EB (reverse) ≤ 0.75·V_EBO(deck); flag BJTs whose base or emitter is a pin without a clamp (H05-47). Hard finding.
- Beats hand layout because: every BJT at every corner.
- Philis status: missing (BJTs exist, `kernel/core/src/netlist.rs:13-14`; no V_EB check).

### H05-38 Thick-field thresholds as deck data
- Kind: deck-requirement
- Statement: separate V_TF per (conductor layer, backgate type). Table 5.2 (20 V N-well CMOS): poly ±15 V, metal-1 ±30 V, metal-2 ±50 V (backgate column as printed pairs NMOS with N-well — apparently swapped). Modern CMOS/BiCMOS top-metal V_TF > 50 V. Standard-bipolar example metal V_TF 40–42 V.
- Source: §5.3.5; L14193–14237, 14359–14360, 14684–14686; PDF 245, 247, 254.
- Philis stage: deck.
- Automation recipe: sidecar table `thick_field_vt[layer][nwell|psub] = V`. Required before H05-39..H05-42 can run. Missing value → check reports "unknown", not pass.
- Beats hand layout because: n/a (enabler).
- Philis status: missing (grep `thick_field` none in `pdks/*.json`).

### H05-39 Parasitic-channel check (six conditions)
- Kind: check (algorithm)
- Statement: a parasitic MOS conducts when (1) a lightly doped backgate, (2) an opposite-type source and (3) drain exist, (4) a conductor or static charge lies between them, (5) |V_GS| approaches or exceeds V_TF (with body effect), (6) V_DS ≠ 0. CMOS cases: poly/metal from a PSD-in-N-well region over the well edge to P-epi (parasitic PMOS); metal from one N-well to another over P-epi (parasitic NMOS).
- Source: §5.3.5 Failure Mechanisms; L14247–14299; PDF 245–246.
- Philis stage: verify, dr.
- Automation recipe: geometric pass over the final layout: for each conductor segment on layer L crossing a well boundary or a lightly doped gap between two same-type regions (N-wells, or P regions), with net voltages V_lead, V_src, V_drn: if |V_lead − V_src| ≥ derate·V_TF(L, backgate) and V_src ≠ V_drn → violation. Uses H05-17 and H05-38. Hard in HV decks; cost term otherwise.
- Beats hand layout because: every crossing is enumerated with its real voltages.
- Philis status: missing.

### H05-40 Derating thresholds for protection and inspection
- Kind: rule
- Statement: standard bipolar: derate V_TF by ≥ 25 % (top-metal PMOS V_TF 40 V → protect any P region ≥ 30 V above substrate). Field-plate all P regions biased above an N-tank by > ~2/3 of the max N-epi V_TF of the uppermost metal; inspect for nearby leads all P regions above ~2/3 of the lowest-metal V_TF. CMOS: charge spreading seldom unless some part exceeds the top-metal V_TF.
- Source: §5.3.5; L14427–14431, 14602–14610, 14684–14686; PDF 248, 252, 254.
- Philis stage: verify, deck.
- Automation recipe: `derate = 2/3` (config; 0.75 for bipolar per the 25 % rule) used in H05-39 and to decide which regions need field plates.
- Beats hand layout because: uniform, quantitative trigger.
- Philis status: missing.

### H05-41 Parasitic-channel remedy ladder
- Kind: algorithm
- Statement: in order of ease: (a) reroute the lead so it does not bridge the HV P region to another P region, or move the lead away; (b) move it to a higher metal with larger V_TF; (c) insert a field plate: a minimum-width strip of poly or lower metal under the lead, tied to ≥ the N-well voltage, extending past misalignment and fringing (field plate is usually easier than a channel stop); (d) channel stop: a minimum-width NMoat strip bisecting the lead. Channel-stop overhang ≥ two-level misalignment + 2 × oxide thickness; field-plate overhang = outdiffusion + misalignment + 2 × oxide thickness.
- Source: §5.3.5; L14455–14460, 14493–14499, 14606–14610, 14634–14668; PDF 248–253. Exercises 5.15–5.16 (L15943–15949).
- Philis stage: dr (a, b), cells/dr (c, d).
- Automation recipe: on an H05-39 violation, dr first reroutes with the crossing forbidden on that layer (a/b); if impossible, emits a field-plate strip on the layer below, width = min width, length = crossing + 2·(misalign + 2·t_ox), tied to the well net.
- Beats hand layout because: automatic, minimal-area fix per crossing.
- Philis status: missing.

### H05-42 Retract poly gate leads into the N-well (thick-oxide PMOS)
- Kind: rule
- Statement: when a thick-oxide PMOS's |V_GS| exceeds the poly PMOS V_TF, its poly must not extend over the N-well edge into the isolation. Required N-well-over-poly overlap = depletion intrusion into the N-well − N-well outdiffusion + poly/N-well misalignment + 2·t_ox(under poly); simplified: N-well-over-PSD overlap + 1–2 µm.
- Source: §5.3.5 CMOS measures; L14640–14647; PDF 253. Exercise 5.20 (L15963–15966).
- Philis stage: cells, dr.
- Automation recipe: mosfet generator, for HV PMOS variants, draws the gate contact head inside the well with the computed overlap; router forbids poly crossing that well edge on the gate net.
- Beats hand layout because: applied to every HV PMOS automatically.
- Philis status: missing.

### H05-43 Field plating of HV resistors and lateral PNPs
- Kind: rule
- Statement: plate connected to the highest potential covers as much of the device as possible; flange gaps between plates (long narrow gaps), or bridge them with channel stops, or overlap with a metal-2 plate; partial plates extend well beyond where the voltage falls below V_TF; every lateral PNP gets an emitter plate over the exposed base overlapping the collector by misalignment (β shifted > 30 % at < 5 V without it); HV collector plate overhangs by misalignment + base outdiffusion.
- Source: §5.3.5 bipolar measures; L14493–14623; PDF 250–253.
- Philis stage: cells.
- Automation recipe: resistor generator option `field_plate: {net: high_terminal, overhang_nm}`; BJT (Pnp lateral) generator adds the emitter plate by default. Matched resistors need symmetric plating (source defers to §8.2.9).
- Beats hand layout because: consistent, symmetric plates on every instance.
- Philis status: missing.

### H05-44 Field plate as an electrostatic shield
- Kind: rule
- Statement: a field plate shields the silicon beneath from any charge or conductor above, so low-voltage metal-2 may cross metal-1 plates; a plate tied to a low-impedance node such as ground also blocks capacitive noise coupling from overlying leads into the silicon.
- Source: §5.3.5; L14507–14513; PDF 250.
- Philis stage: dr.
- Automation recipe: for sensitive resistors/diffusions (annotator), permit or require a grounded plate on the lowest metal beneath noisy crossings; bottom-plate variant of the existing shield rule.
- Beats hand layout because: every noisy crossing over a sensitive diffusion is found.
- Philis status: partial. Same-layer side shields exist (`kernel/analog/src/routing/shield.rs:8-19`, "no top/bottom plates").

### H05-45 High-voltage charge spreading
- Kind: rule
- Statement: in HV CMOS/BiCMOS, ions from the mold compound gather on the PO and move to HV regions; inversion often needs > 200 V, but charge over the drain–backgate junction lowers breakdown at far smaller voltages. Plate the device with the highest available metal if the oxide under it holds the voltage; thick power copper over the PO; else semi-insulating PO.
- Source: §5.3.5; L14689–14698; PDF 254.
- Philis stage: cells, deck.
- Automation recipe: drain-extended devices above a deck voltage get a top-metal plate over the drift region tied to the drain or source per deck guidance. Not relevant to the current decks' core devices.
- Beats hand layout because: n/a beyond consistency.
- Philis status: missing.

### H05-46 Substrate influence in dielectric isolation
- Kind: rule (package/flow)
- Statement: the DI handle must be tied to the lowest-voltage pin (ground for single supply); otherwise static charge depletes the superficial silicon, lowering breakdown and causing leakage. Backside contact (backgrind, conductive die attach, downbond or fused leadframe) or top-side TSVs (+1 mask).
- Source: §5.3.6; L14712–14783; PDF 254–255.
- Philis stage: flow, deck.
- Automation recipe: SOI decks require a declared handle net; ERC fails if absent.
- Beats hand layout because: n/a.
- Philis status: missing / not applicable to current decks.

### H05-47 Minority-carrier injector identification
- Kind: check (annotator classification)
- Statement: any diffusion connected to a pin can inject minority carriers (pin above supply → PSD/N-well/P-epi PNP; pin below ground → NSD/P-epi/N-well NPN). Latchup usually needs ≥ 100 µA sunk from a diffusion; ESD clamps limit pins to ~−1 V, so ≥ 10 kΩ series resistance usually prevents latchup: "diffusions connected to pins through less than 10 kΩ may trigger latchup"; conservative designers use 50 kΩ or 100 kΩ (100 kΩ has failed in very-low-current circuits over BOX). Also injectors: internal nodes capacitively coupled to switching outputs (< 1 pF triggered latchup), Schottky diodes (especially PN-guard-ringed), saturating lateral PNPs merged with other devices.
- Source: §5.4.1; L14837–14885; PDF 256–257.
- Philis stage: annotator (primary), gp/dp, cells.
- Automation recipe: add net role `Pin` (from the subcircuit port list) to the net classifier. Injector devices = devices with a drain/source/diode/collector diffusion connected to a `Pin` net through series resistance < R_inj (config, default 10 kΩ, conservative 50–100 kΩ), computed as the min resistance path through resistors in the netlist; plus devices on nets with a capacitor to a switching (clock/digital-output) net; plus Schottkies. Output: `injector[d]` with polarity (electron/hole). Feeds H05-54..H05-58 and the isolation rule as aggressors.
- Beats hand layout because: exhaustive, with series resistance computed through the netlist; humans miss capacitively coupled injectors.
- Philis status: missing. Net roles are Signal/Supply/Ground/Clock only (`backend/annotator/src/netrole.rs:11-16`); the isolation rule's aggressors are clock-net devices only (`backend/annotator/src/emit.rs:259-279`).

### H05-48 Latchup loop and SCR detection
- Kind: check
- Statement: latchup needs both parasitic transistors active, β_N·β_P > 1 over some current range, and a supply that sustains the current; triggered by well (R1) or substrate (R2) debiasing from majority-carrier drift. Any PNPN cross-section can latch (e.g. lateral PNP merged with vertical NPN in one tank; vertical NPN with integrated Schottky). Countermeasures: prevent injection, collect, recombine, reduce well/substrate resistance.
- Source: §5.4.2; L14928–14972, 15008–15020; PDF 258–259.
- Philis stage: verify.
- Automation recipe: from the final layout, list NMOS-source/P-sub/N-well/PMOS-source quadruples within a distance (deck latchup spacing; not given in this chapter) whose wells/substrate are not tapped within the distance of H05-51/H05-52; report each as an SCR candidate with its R1/R2 estimate.
- Beats hand layout because: every SCR candidate is enumerated with estimated resistances.
- Philis status: missing (grep `latchup` none); no latchup rule in the decks (`pdks/*.json`, grep none). Previously noted as a gap in `docs/LAYOUT-FUNDAMENTALS.md:137` (#60).

### H05-49 Latchup test as design target
- Kind: metric
- Statement: typical JESD78A-style test: 100 mA for 50 ms, both polarities, into each pin; latchup if any supply current rises by ≥ 10 %. Guard rings of low-R layers strapped to ground survive 100–200 mA tests.
- Source: §5.4.2 L14975–14981; §5.4.4 L15541–15554; PDF 259, 266.
- Philis stage: flow, verify.
- Automation recipe: config `latchup_test_ma = 100`; drives H05-50 (R_V target) and guard-ring strap sizing (H05-56: metal width for ≥ 100–200 mA transient via EM/self-heating).
- Beats hand layout because: ring and tap sizing derive from the test current.
- Philis status: missing.

### H05-50 Substrate-contact resistance for thin epi on a heavy sublayer
- Kind: formula + metric + algorithm
- Statement: `R_V = ρ·t/A_dif` (5.22, A_dif ≫ t²); required total contact area from `ΣA_dif < ρ·t/R_V` (5.23, as printed). Target: 100 mA latchup test, ≤ 0.3 V debias → R_V ≤ 3 Ω → 0.33 mm² at ρ = 10 Ω·cm, t = 10 µm. Small contacts: `R_V = ρ/(2π·r_dif)·tan⁻¹(2t/r_dif)` (5.24), `r_dif = √(A_dif/π)` (5.25); a disperse array of A_dif = 10 µm² contacts (t = 10 µm, ρ = 10 Ω·cm) has 13 % of the resistance of one large equal-area contact, if contacts are ≥ 2t apart. On P+ substrates fill unused area with substrate contacts instead of bypass capacitance if substrate resistance would exceed 1 Ω. Rules: place small contacts wherever convenient; fill unused area; consider ringing majority-carrier injectors (annular contact near the injector; width gain tapers above t/2; peripheral contacts only help devices not much larger than t).
- Source: §5.4.3 Thin lightly doped layers atop heavily doped sublayers; L15038–15156; PDF 259–261. Exercise 5.21 (L15967–15970; PDF 274).
- Philis stage: cells, dp (whitespace), verify.
- Automation recipe: metric `R_sub = 1/Σ(1/R_V,i)` over all substrate-tap shapes (eq. 5.24 per tap, treating taps spaced ≥ 2t as parallel); budget `R_sub ≤ V_debias/I_test`. After placement, fill whitespace with substrate-tap tiles (not only metal fill) until the budget is met. Deck requirement: epi ρ and t, sublayer type.
- Beats hand layout because: the resistance is computed and met, and whitespace is used for taps systematically.
- Philis status: missing. Fill is metal-only (`frontend/library/src/fill.rs:1-7`); the annotator notes decks lack a real epi thickness (`backend/annotator/src/emit.rs:249-255`).

### H05-51 Tap placement in a thick lightly doped substrate
- Kind: rule + formula + check
- Statement: `R_SP = ρ/(2r_dif)·[1 − (2/π)·sin⁻¹(r_dif/d)]` between two contacts (5.26): 10 µm contacts on 10 Ω·cm give 1 kΩ far apart, 840 Ω at 20 µm c-c. Annular contact around a dot: `R_SP = ρ/(4(r_dif + s_dif + w_dif))·[1 − (2/π)·sin⁻¹(r_dif/(r_dif + s_dif))]` (5.27): 10 µm dot + 5 µm ring → 110 Ω at 5 µm spacing, 22 Ω at 100 µm (a larger, farther ring is better). Valid while dimensions < substrate thickness (250 µm post-backgrind → up to ~200 µm). Rules: substrate contacts within half the die thickness of all devices (no structure more than 100–200 µm from a contact; "within about 100 µm"); make each as large as space permits; contacts farther than half the die thickness give little benefit; ringing "because closer is lower R" is flawed here.
- Source: §5.4.3 Thick lightly doped substrates; L15159–15241; PDF 261–262.
- Philis stage: verify, cells, dp.
- Automation recipe: check `max_device dist(device bbox, nearest substrate tap) ≤ d_tap` with d_tap = deck latchup rule or ½·die thickness (default 100 µm); cost term pulls tap-bearing cells or inserts tap tiles; prefer large taps over rings in bulk-substrate decks (sky130 is bulk p-substrate per `backend/annotator/src/emit.rs:252-254`).
- Beats hand layout because: distance to every device is verified and tap area is maximized from whitespace.
- Philis status: missing as a global check (`docs/LAYOUT-FUNDAMENTALS.md:69,137`, #17/#60); per-cell tap strip only (`kernel/cells/src/mosfet.rs:596-622`).

### H05-52 Tap placement in thin (isolated) layers: wells, tanks, SOI
- Kind: rule + formula
- Statement: t < r_dif: `R_SP = ρ/(π·t)·ln(2d/r_dif)` (5.28); t > r_dif: `R_SP ≈ ρ/(2r_dif) + ρ/(π·t)·[ln(d/(2t)) − 0.116]` (5.29). Table 5.3 (10 µm contacts, 10 Ω·cm; t = 1, 2, 5 µm, ∞): d = 20 µm → 6.6 k, 3.3 k, 1.3 k, 1.0 kΩ; 50 → 9.5 k, 4.7 k, 1.9 k, 1.0 k; 100 → 11.7 k, 5.9 k, 2.3 k, 1.0 k; 1000 → 19.1 k, 9.6 k, 3.8 k, 1.0 k. Annular (t < r_dif, s_dif > t): `R_SP = ρ/(8π·t) + ρ/(2π·t)·ln((s_dif + r_dif)/r_dif) + ρ/(2π(s_dif + r_dif))` (5.30). Table 5.4 (10 µm ring, 10 µm dot): s = 5 µm → 1.3 k, 710, 380, 110 Ω; 10 → 1.9 k, 980, 460, 100 Ω; 100 → 4.9 k, 2.4 k, 100 (as printed; out of trend), 22 Ω. On thin layers resistance rises with spacing (constriction); distant contacts barely help. An annular contact ≥ t/2 wide collects most majority current inside it; minority electrons travel ~800 µm in 10 Ω·cm P-Si before recombining, so scattered contacts are still needed. Rules: annular contacts around all majority-carrier injectors; as close as possible; width ≈ t/2; scatter small contacts ≤ 20–50 × t apart.
- Source: §5.4.3 Thin layers; L15245–15395; PDF 262–264.
- Philis stage: cells, verify.
- Automation recipe: for each N-well (t = well depth from the deck): (a) every PMOS source/diffusion that is an injector (H05-47) gets an annular well tap at min spacing, width ≥ t/2; (b) check max tap-to-tap spacing within the well ≤ k·t, k ∈ [20, 50] (config). Same for P-wells in twin-well/SOI decks.
- Beats hand layout because: spacing checked for every well; ring width derived from the well depth.
- Philis status: partial. Per-cell tap strip with a contact pitch (`kernel/cells/src/mosfet.rs:596-622`) and guard rings around matched devices (`backend/annotator/src/constraints.rs:66-84`); no well-depth-based spacing check.

### H05-53 Standard-bipolar isolation debiasing
- Kind: rule
- Statement: isolation sheet resistance 10–20 Ω/□, vertical ≈ 3–5 kΩ/µm² (as printed); minority carriers from tanks reach isolation laterally, then flow as majority carriers to contacts. Rules: ring majority-carrier injectors with as much substrate contact as possible; minimize gaps in the contact metallization around them; scatter contacts throughout.
- Source: §5.4.3; L15399–15416; PDF 264.
- Philis stage: cells (bipolar decks).
- Automation recipe: same machinery as H05-52 with isolation as the layer; only for bipolar-capable decks.
- Beats hand layout because: n/a beyond consistency.
- Philis status: missing / not applicable to current decks.

### H05-54 Guard-ring taxonomy and type selection
- Kind: data-model + rule
- Statement: substrate/well/tank contacts (majority-carrier) cannot stop minority carriers; minority-carrier guard rings either collect (reverse-biased junction) or block (high-low junction): ECGR = N-type region collecting electrons in P material; EBGR = P+/P− high-low junction; HCGR = P-type region collecting holes in N material; HBGR = N+/N− high-low junction. Injectors get collecting/blocking rings; victims get majority-carrier contacts plus distance.
- Source: §5.4.4; L15421–15446; PDF 264–265. Also §5.4.3 L15380–15385 (annular contacts cannot stop minority debiasing).
- Philis stage: annotator, cells (post_cell).
- Automation recipe: split the model into `TapRing {polarity = bulk type, net = bulk}` (majority-carrier, around victims and around majority injectors) and `MinorityGuard {kind: Ecgr|Hcgr|Hbgr|Ebgr, net: supply/ground}` around injectors (H05-47). ECGR around an electron injector in P-sub = N+ (and N-well/deep-N if available) ring tied to VDD or ground; HCGR around a hole injector inside an N-well = P+ ring tied to ground.
- Beats hand layout because: ring type follows the injected carrier by rule, for every injector.
- Philis status: partial, mislabeled. The annotator gives every matched-block NMOS an `Ecgr` and every PMOS an `Hcgr` tied to the device's **bulk** (`backend/annotator/src/constraints.rs:66-84`), and `post_cell` draws `Ecgr` as a p+ substrate tap and `Hcgr` as an n+ well tap (`kernel/cells/src/post_cell.rs:311-322`). Those are majority-carrier tap rings (Hastings's annular contacts), not the minority-carrier guard rings the type names denote; no ring is placed around injectors. Enum at `kernel/analog/src/cell.rs:25-33`.

### H05-55 High-low junction blocking effectiveness
- Kind: formula + deck-requirement
- Statement: permeation `P = I_out/I_in = (A·D_H·τ_L/(W_H·V_L))·(N_L/N_H)` (5.31). Example: 25 µm tank, 10 µm deep at 3·10¹⁴ cm⁻³ over a 5 µm NBL at 10¹⁸, τ_L ≈ 0.7 µs, D_H ≈ 4.4 cm²/s → P = 0.0018 (0.2 %). A high-low junction is effective if N_H/N_L > ~100. P− epi on P+ blocks electrons from the substrate; NBL/deep-N+ block holes; LV BiCMOS wells with surface N_L > 10¹⁸ make deep-N+ ineffective.
- Source: §5.4.4; L15454–15505; PDF 265–266.
- Philis stage: deck.
- Automation recipe: deck flags `blocks_electrons_to_substrate` (epi on P+), `nwell_blocks_holes` (NBL/retrograde, N_H/N_L ≥ 100); the ring selector uses them to choose blocking vs collecting rings and to decide whether rings (bulk) or scattered taps (epi) are right (H05-50 vs H05-51).
- Beats hand layout because: choice of ring style follows process physics per deck.
- Philis status: missing.

### H05-56 Electron-collecting guard ring (ECGR) construction
- Kind: rule
- Statement: include all available N-type layers (depth and doping); place immediately adjacent to the injector and make it as wide as practical (standard bipolar: electrons pass under through the light substrate); every portion strapped to ground by metal — metal gaps debias the ring and it reinjects; grounded low-R rings (emitter/NMoat/deep-N+/NBL) survive 100–200 mA latchup tests; without deep-N+, tie to a supply (tolerates more debias; power at high V); strap metal sized for tens to hundreds of mA transients without excessive heating; with a P+ substrate the ring reduces electron current 10–100×; a substrate-returned ECGR claims > 10⁶ in one direction; the ring segment along the die edge is useless → L or U shape ending at the die edge.
- Source: §5.4.4 ECGR; L15514–15603; PDF 266–267.
- Philis stage: cells (post_cell), dr, verify.
- Automation recipe: MinorityGuard generator: all N layers the deck allows in a ring; strap metal continuous (no gaps; router forbids crossing on the strap layer, crossings go over on a higher metal); strap width from the latchup test current (H05-49) via EM/self-heating; tie net = VDD unless deep-N+ is available; open side toward die edge allowed when the block is marked edge-adjacent.
- Beats hand layout because: continuity and strap width are verified.
- Philis status: missing (existing rings are bulk taps, see H05-54; ring on li only per `docs/LAYOUT-FUNDAMENTALS.md:135`).

### H05-57 Hole-collecting guard rings and P-bars
- Kind: rule
- Statement: HCGR = P-type ring inside the N region around a hole injector; tie to tank potential (risk of debias through a high-R base diffusion) or to ground (limits tank voltage to the P-iso/NBL breakdown; power at high V). In retrograde wells a PSD HCGR works; make it ideally as wide as the well is deep; even min width helps. P-bar: strip of base between two lateral PNPs sharing a tank, ends extending into isolation, stops cross-injection when one saturates (e.g. LPNP mirrors), and suppresses SCRs where an NPN collector is merged with a PNP base.
- Source: §5.4.4 HCGR; L15622–15700; PDF 268–269.
- Philis stage: cells, annotator.
- Automation recipe: for PMOS injectors in an N-well (drain on a pin): P+ ring inside the well around the device, tied to ground, width → well depth if space allows; for BJT arrays sharing a well, insert P-bars between members whose op point may saturate (V_CE small).
- Beats hand layout because: applied to every flagged hole injector.
- Philis status: missing.

### H05-58 Hole-blocking guard rings
- Kind: rule
- Statement: HBGR = N+/N− high-low wall (deep-N+ or deep trench) plus floor (NBL or radical retrograde well) completely enclosing the hole injector — any gap lets holes escape; drawn NBL extends to (or beyond) the outer edge of the deep-N+/trench so doping is full where they meet; effective only if its doping exceeds the N-well's by ~100× (older deep-well BiCMOS yes, newer shallow heavy wells maybe not); DTI-based HBGRs need no outdiffusion/depletion allowance. In standard bipolar HBGR beats ECGR for latchup; if space is limited, prefer HBGR.
- Source: §5.4.4 HBGR; L15704–15747; PDF 269–270.
- Philis stage: cells, deck, verify.
- Automation recipe: only for decks declaring deep-N+/NBL/DTI; ring generator with a closed-ring (no-gap) check and NBL-overhang rule.
- Beats hand layout because: gap-free enclosure verified.
- Philis status: missing / not applicable to current decks (`Hbgr` exists only as an enum value drawn as an n+ well tap, `kernel/cells/src/post_cell.rs:311-322`).

---

## 4. Top-15 priorities for Philis

Ranked for a block-level, constraint-aware analog P&R on bulk CMOS decks (sky130, gf180mcu, ihp_sg13g2) that must beat hand layout.

1. **H05-17** Per-net voltage and per-device V_GS/V_DS/V_EB in the op point — unblocks every voltage-dependent reliability check (H05-18, 33, 35, 37, 39, 42).
2. **H05-47** Pin role plus injector classification (pin diffusions < 10 kΩ, cap-coupled nodes, Schottkies) — gives substrate isolation and guard rings the real aggressors; today only clock nets are aggressors.
3. **H05-54** Split majority-carrier tap rings from minority-carrier guard rings. The current `Ecgr`/`Hcgr` rings are bulk taps around victims. Rings belong around injectors, with the ring type set by the injected carrier.
4. **H05-51 / H05-52** Global tap-distance check (half die thickness ≈ 100 µm in bulk; ≤ 20–50·t in wells), with tap insertion — closes the latchup gap already listed as #60 in LAYOUT-FUNDAMENTALS.
5. **H05-50 / H05-49** Substrate-contact resistance metric (R ≤ V_debias/I_test, e.g. 3 Ω for 100 mA/0.3 V) and tap fill in whitespace — a measured number no hand layout reports.
6. **H05-56 / H05-57** Guard-ring construction: continuous metal strap sized for 100–200 mA, all N layers for ECGR, P+ ring for hole injectors, L/U rings at the die edge.
7. **H05-14** Via-array geometry by current direction (leading-row effective cuts) — a small change to dr with a real EM gain over square arrays.
8. **H05-16** Pulsed/AC EM (duty cycle, f > 10 kHz exemption) — stops over-widening clock/switch nets and keeps matched routing compact.
9. **H05-33 / H05-35** Screen matched groups for unequal V_DS (HCI) and unequal PMOS V_GS (BTI) — constraint extraction the owner explicitly wants; long-term offset a human rarely checks.
10. **H05-05 / H05-06** Resistor (and ESD-path) width from self-heating: steady (ΔT ≤ 5 °C) and adiabatic pulse formulas.
11. **H05-28 / H05-29** Antenna junction credit by stage and polarity (both NSD and PSD at ≤ 3.3 V); PSD diode with UV keep-out and fill-block markers.
12. **H05-30** Latent antenna (oversize–undersize across nets) — cheap addition to the existing `Stack::antenna`.
13. **H05-18 / H05-19** Oxide-field check per gate and capacitor with area scaling (needs H05-17 and a t_ox per deck).
14. **H05-38 / H05-39 / H05-41** Thick-field thresholds in the deck, the six-condition parasitic-channel check, and the reroute → higher metal → field plate → channel stop fixes — needed once HV or thick-oxide devices (gf180 5 V/6 V, ihp HV) are placed.
15. **H05-01 / H05-04** Junction temperature from θ_JA·P_D plus eq. 5.6 self-heating, fed into per-net EM derating in place of one die temperature.
