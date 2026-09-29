# Hastings ch.12 — Field-Effect Transistors: extraction for Philis

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson 2023), Chapter 12 "Field-Effect Transistors" (printed pp. 573–641).
Reftext: `scratchpad/reftext/hastings.txt`, lines **34124–38173** (read in full). PDF pages below are PDF indices (= printed page + 1), computed as 1 + form-feeds before the line. Garbled equations and numbers were checked against the PDF (Read tool, pages 577–588, 598–601, 610–621, 628–629, 642).

Scope note: this chapter has **no LOD or WPE material** (Hastings covers those in ch.13 §13.2). Its only stress-related remark is that silicon is isotropic unless placed under anisotropic stress (L36719–36721). Extended-drain devices appear only as LDD/DDD drift regions (L36328–36384) and as the drain-extended NMOS inside the EPROM cell (L37419–37420, L37459–37461). Their construction is in ch.13 §13.1.5.

## 1. Coverage

Read chunks (Read tool, offset..end): 34124..34823, 34824..36123, 36124..36823, 36824..37523, 37524..38123, 38124..38173. Every line of 34124–38173 was read.

Section and subsection headings in the range:
- Ch.12 intro (L34124–34175)
- 12.1 MOS Transistor Operation (L34181–34328)
  - 12.1.1 Device Transconductance (L34330–34458)
  - 12.1.2 Threshold Voltage (L34462–34808)
  - 12.1.3 Additional Modeling Considerations (L34817–35202): Channel-Length Modulation (L34830); Velocity Saturation (L34889); Short-Channel Effect (L34942); Narrow-Channel Effect (L34973); Subthreshold Conduction (L34982); Gate-induced Drain Leakage (L35162)
  - 12.1.4 Breakdown in MOS Transistors (L35206–35294)
- 12.2 Constructing CMOS Transistors (L35300–35344)
  - 12.2.1 Coding the MOS Transistor (L35348–35464)
  - 12.2.2 Wells and Tanks (L35468–35752)
  - 12.2.3 Channel Stop Implants (L35756–35856)
  - 12.2.4 Threshold Adjust Implants (L35860–36039)
  - 12.2.5 Multiple Gate Oxides (L36043–36098)
  - 12.2.6 Scaling the Transistor (L36102–36285)
  - 12.2.7 Drain Engineering of CMOS Transistors (L36291–36497)
  - 12.2.8 Variant CMOS Layouts (L36502–36833): Serpentine Transistors (L36671); Annular Transistors (L36696)
  - 12.2.9 Backgate Contacts (L36837–37047)
- 12.3 Nonvolatile Memory (L37053–37161)
  - 12.3.1 The Floating-Gate Transistor (L37165–37412)
  - 12.3.2 Single-Poly EPROM (L37415–37502)
  - 12.3.3 Single-Poly EEPROM (L37506–37623)
- 12.4 The JFET Transistor (L37629–37652)
  - 12.4.1 Modeling the JFET (L37656–37797)
  - 12.4.2 JFET Construction (L37799–38022): Epi-FET (L37807); N-well JFET (L37864); Double-Diffused JFET (L37898); Ion Implanted JFET (L37952)
- 12.5 Summary (L38028–38033); Selected Bibliography (L38038–38067)
- 12.6 Exercises 12.1–12.20 (L38073–38168)

## 2. Section-by-section digest

**Ch.12 intro (L34124–34175, PDF 574–575)**
- JFETs came first (1952) because thin gate dielectrics were hard to make. JFET op-amps are still sold for their lower noise than MOSFET amplifiers (L34137–34141).
- Early MOS devices had poor Vt control and ESD-fragile oxides (L34149–34154). The chapter covers self-aligned poly-gate MOS, digital CMOS, floating-gate NVM and JFETs. HV/power MOS and MOS matching are in ch.13 (L34171–34175).

**12.1 MOS Transistor Operation (L34181–34328, PDF 575–577)**
- Shichman–Hodges model: cutoff, linear and saturation regions (Table 12.1). Saturation current is I_D = (k/2)·V_gst² (Eq. 12.3), with V_gst = V_GS − V_t (Eq. 12.1) (L34194–34271).
- Source and drain are **defined by bias, not by terminal labels**. When V_DS < 0 the roles swap (L34273–34279).
- Small-signal g_m depends on k, and therefore on W and L (Eq. 12.4). Pushing W/L too high drives the device into subthreshold, where g_m/I_D saturates. MOS g_m is always below bipolar g_m at equal current (L34296–34311).

**12.1.1 Device Transconductance (L34330–34458, PDF 577–579)**
- k = k'·(W_e/L_e) (Eq. 12.6). W_e = W_d + W_b and L_e = L_d + L_b (Eqs. 12.7–12.8), with signed process biases that "seldom exceed a few tenths of a micron" (L34337–34365; PDF 577).
- k' = μ·ε0·εr / t_ox (Eq. 12.9). The capacitance-equivalent t_ox is about 0.4 nm thicker than physical because of a quantum effect, plus poly depletion (L34366–34382).
- Effective mobility (Eqs. 12.10–12.11): μn ≈ 540/(1+((V_GS+V_t)/(0.54·t_ox))^1.85) and μp ≈ 180/(1−((V_GS+1.5V_t)/(0.34·t_ox))), with t_ox in nm (PDF 578; L34390–34408).
- Oxide strength is about 10 MV/cm. Derate to ≤3.5 MV/cm for t_ox > 30 nm and to ≤5 MV/cm for t_ox ≤ 15 nm (L34417–34421; PDF 578).
- A PMOS needs about 3× the NMOS W/L for equal k (L34430–34432).
- Mobility goes as ∝T^−1.7 for electrons and ∝T^−1.5 for holes. k at 150 °C is about 60 % of k at 25 °C (L34436–34441; PDF 578).

**12.1.2 Threshold Voltage (L34462–34808, PDF 579–584)**
- Enhancement vs depletion devices, NMOS/PMOS sign conventions (Fig. 12.2; L34462–34491).
- Swapping N+ and P+ poly shifts Vt by about 1.2 V (Table 12.2; L34501–34552; PDF 580).
- Vt vs backgate doping and t_ox (Table 12.3). Thick-field thresholds come from 10 kÅ field oxide: 10^16 cm^−3 gives 13.7 V and 10^17 cm^−3 gives 47.5 V (L34555–34619; PDF 580–581).
- **Backgate modulation**: V_BS raises |Vt| (Table 12.4, NMOS 0.17/0.43/0.61/0.89 V at V_BS 0/−1/−2/−4 V) and "also increases the random variation of the threshold voltage" (L34627–34633; PDF 581–582).
- Oxide charges Qf, Qot, Qm, Qit. Incomplete hydrogen anneal raises random Vt variation and 1/f noise. HCI regenerates traps: NMOS Vt rises and drain current falls, while PMOS can move either way (L34654–34738).
- Achievable Vt control, a combined budget, and a floor on |Vt| to avoid subthreshold leakage (L34754–34759). The full Vt equation is Eqs. 12.12–12.15 (L34762–34802).
- Vt temperature coefficient is about −1.5 mV/°C at 100 Å oxide and −2.5 mV/°C at 500 Å (L34806–34808; PDF 584).

**12.1.3 Additional Modeling Considerations (L34817–35202, PDF 584–590)**
- CLM: I_D = (k/2)V_gst²(1+λV_DS) (Eq. 12.16), and r_o is inversely proportional to λ (Eq. 12.17). Output resistance scales about linearly with drawn L if backgate doping is uniform. **Analog lengths of 20–40 µm are not uncommon.** Pocket-implanted devices break this scaling (L34830–34885; PDF 584–585).
- Velocity saturation: ΔL = |V_DS/(E_c·L_e)| (Eq. 12.18), with E_c = 1.1·10^4 V/cm for electrons. At V_DS = 1 V it adds 1 % at L = 10 µm and 100 % at L = 1 µm. V_sat is given by Eq. 12.19 (L34889–34938; PDF 585).
- SCE lowers |Vt| for L below a few µm. DITS/DIBL matter mainly for L < 2 µm, and punchthrough is the extreme case (L34942–34969).
- The narrow-channel effect raises |Vt| for W below a few µm. Analog designers seldom go that narrow (L34973–34978; PDF 586).
- Subthreshold: I_D = I_0·e^((V_GS−V_X)/(nV_T)) (Eq. 12.20), n = 1 + ε_Si·t_ox/(ε_ox·w_d) (Eq. 12.21), S = 2.30·n·V_T (Eq. 12.23). S is at least 60 mV/dec, typically 80, and about 90 at 0.15 µm. Minimum Vt is about 4S ≈ 300 mV, which leads to NMOS Vt of 550–700 mV and an extra ~200 mV for power devices. Subthreshold circuits "do not function well" much above 70 °C (Table 12.5; L34982–35160; PDF 586–588).
- GIDL is exponential in the drain–gate field and in temperature, and worst in cutoff at high V_DS. Antenna charging damage and HCI raise GIDL (L35162–35202).

**12.1.4 Breakdown (L35206–35294, PDF 589–591)**
- BV_DSS is set by avalanche (abrupt, preferred) or punchthrough (soft, leaky) (Fig. 12.6; L35206–35223).
- Snapback: impact-ionisation holes debias the backgate and turn on the parasitic NPN. The trigger voltage and sustain voltage are defined, and operating above the sustain voltage is risky (Fig. 12.7; L35240–35255).
- BV_DII is measured at V_GS ≈ half the maximum V_GS, as the V_DS that raises I_D by about 20 %. A pass or power device with BV_DII below BV_DSS risks hot shorts. Power devices need BV_DII ≥ max operating voltage, which "requires backgate resistance be minimized", for example with integrated backgate contacts (L35271–35284).
- Gate-oxide breakdown can be instantaneous or slow (TDDB) (L35292–35294).

**12.2 Constructing CMOS Transistors (L35300–35344, PDF 591–592)**
- Self-aligned poly-gate NMOS in P-epi on P+. NSD/PSD implants do not penetrate poly or field oxide, and PSD contacts the backgate. Metal gates are back only with high-k dielectrics (L35300–35344).

**12.2.1 Coding (L35348–35464, PDF 592–595)**
- Three coding styles: mask layers, NMoat/PMoat, and Active+Tap (Fig. 12.9). The deck derives masks: PSD = PMoat oversized 1.0, NSD = NMoat oversized 1.0 (L35348–35409).
- Oversizing overlaps abutting NSD and PSD, so they must be clipped: PSD = PSD not common to NMoat, NSD = NSD not common to PMoat. "Even the most sophisticated clipping routines sometimes produce questionable results" (L35413–35442).
- Active+Tap coding avoids clipping. Its code is SD = Active⊕1.0, NSD = (SD−Tap−NWell) + (SD∩Tap∩NWell), and so on (L35445–35457).

**12.2.2 Wells and Tanks (L35468–35752, PDF 594–600)**
- P-substrate processes dominate (negative ground). The N-well, P-well, twin-well, triple-well and quad-well flows are described. Most current analog processes are twin-well with dual gate oxides (L35468–35621).
- A generated P-well is the drawn N-well inverted and undersized by 2.0. Each well voltage class gets its own spacing: NwellLV oversized 2.0, NwellHV oversized 3.0 (L35558–35583; PDF 596–597).
- Wells cannot isolate NMOS backgates. Isolated P-tanks can be built four ways: deep N-well to NBL (high vertical R), deep-N+ sinker, up-down isolation, or DTI. The isolated NMOS has five terminals, and the isolation must be biased at or above the backgate (L35625–35688).
- The doubly isolated PMOS has seven terminals. Backgate-to-isolation must stay ≤ 5–10 V, or about 20 V with PBL. Shallow N-well breakdown is 20–30 V; deep isolation handles 40–80 V (L35692–35718; PDF 599).
- **Tank grouping.** Identify every isolated NMOS that can inject electrons before layout. Devices whose drains share one pin may share a tank. Devices on different pins should not, unless proven safe. Never merge fast-switching ("noisy") isolated NMOS with sensitive analog devices, because the result is parasitic rectification (bandgaps are notorious). Scatter many tank contacts, observe the maximum device-to-contact distance, and optionally ring each device (L35722–35752; PDF 599–600).

**12.2.3 Channel Stop Implants (L35756–35856, PDF 600–601)**
- 10^17 cm^−3 under 10 kÅ field oxide gives a thick-field Vt of about 50 V, which is margin for a 30 V process. Channel stops come in pairs, and their junction limits N-well/P-epi breakdown, typically above 30 V for a 15 V process (L35763–35793).
- Channel-stop mask generation: NwellOS = Nwell⊕4.0, MoatOS = Moat⊕1.0, PChst = NwellOS − MoatOS (L35809–35827).
- Each conductor has its own thick-field Vt: **poly lowest**, then M1, M2 and so on, and summaries quote only M1. An accidental "poly stub" past a well can invert the silicon. **Derate published thick-field Vt by ≥ 30 %**: a 40 V M1 PMOS thick-field threshold allows at most about 25 V between M1 and the N-well beneath (L35842–35856; PDF 601).

**12.2.4 Threshold Adjust Implants (L35860–36039, PDF 601–605)**
- Nominal |Vt| must be ≥ 0.55 V, with a target range from 0.60 V (L35861–35869; PDF 601). Natural vs adjusted thresholds are shown in Table 12.6 (L35873–35918).
- The **NatVt** coding layer must be drawn around the gate of each natural transistor and slightly overlap the channel for misalignment and outdiffusion (L35922–35936; Fig. 12.17). Exercise 12.11 gives rules: NATVT width 4 µm, overlap of GATE 2 µm, spacing to POLY 4 µm, spacing to NATVT 2 µm (L38125–38142; PDF 642).
- Dual-doped poly uses PVT/NVT/PPoly/NPoly masks. PPoly/NPoly junctions are diodes unless silicided. Silicide speeds dopant diffusion, so these junctions must sit well away from gates. The DRC must check that every poly lead receives NPoly or PPoly, because near-intrinsic poly opens if the silicide breaks (L35952–36032).
- Other Vt options (depletion, etc.) are coded like NatVt with their own layer (L36036–36039).

**12.2.5 Multiple Gate Oxides (L36043–36098, PDF 605–606)**
- Staged oxidation uses a poly-1 gate for thin oxide and poly-2 for thick. Etch-and-regrow uses an extra mask. Both draw a **Moat-2 layer around the gate region** of thick-oxide devices (L36049–36083; Fig. 12.20).

**12.2.6 Scaling (L36102–36285, PDF 606–608)**
- Constant-voltage vs constant-field scaling (Table 12.7). Hot carriers above 200 kV/cm drove the change to constant-field scaling and dual-gate processes (L36136–36214).
- Optical shrinks and lambda rules ended when FEOL and BEOL scaling split. **Analog must never be shrunk without resimulation**: resistors do not change (ignoring biases) while capacitances scale as 1/S² (L36218–36240; PDF 607).
- Analog poly-gate CMOS is expected to stop around the 65 nm node. Oxides thinner than 30 Å tunnel too much for analog (L36244–36285).

**12.2.7 Drain Engineering (L36291–36497, PDF 608–612)**
- SDD with sidewall spacers; LDD (NSD/PSD plus NMSD/PMSD, i.e., four masks); DDD (As+P, NMOS only); BCLDD PMOS in single-doped poly (L36291–36424).
- **Pocket/halo implants** suppress DIBL but make r_o scale as about √L instead of L. Mismatch also deviates from Pelgrom 1/√L scaling. "Accurate current mirrors cannot be constructed using pocket-implanted transistors without resorting to resistive degeneration" (L36425–36471; PDF 610–611).
- Pockets can be blocked by two extra masks, or by directional tilted implants shot left and right only. In the latter case, devices with horizontal L get pockets and vertical-L devices do not. **A block may be reflected or rotated by 180° but not by 90°** (L36480–36487; PDF 611).

**12.2.8 Variant CMOS Layouts (L36502–36833, PDF 612–618)**
- W/L below about 10 fits one section. Above that, divide into identical parallel sections. Shared S/D saves area and cuts junction capacitance by up to 50 % (L36503–36508).
- Notation N(W/L). Lay out exactly as specified when annotated. Otherwise it may be refolded (N·W_f = W, e.g. 1000/0.5 → 20(50/0.5)), but tell the designer. **ESD robustness depends on per-finger width**, so 20/0.5 → 2(10/0.5) may halve it (L36525–36538; PDF 612).
- **Matched devices must use equal section widths**. Example: M1 100/0.5 and M2 2(100/0.5) become 4(25/0.5) and 8(25/0.5) with 25 µm sections (L36541–36546; PDF 612).
- An odd section count gives equal S and D finger counts. An even count gives one extra S or D. PCells offer "minimize the drain" or "minimize the source", and designers prefer minimizing the drain. Very high-speed designs split small transistors into two sections to cut drain capacitance (L36550–36564).
- **Abutting (butting) backgate contacts** sit next to a source when backgate = source. Odd sections allow one end. Even sections with minimized drain allow both ends. Minimized source allows none (L36568–36574; PDF 613, Fig. 12.25).
- Merged transistors of unequal width share S/D through a notched moat. The poly-to-moat spacing S_PM grows the shared region, but the result is still smaller and lower in capacitance than two devices. A separate backgate contact can then serve several devices (L36577–36599; Fig. 12.26).
- NAND example: series devices share an uncontacted internal node, and gates are brought closer with 90° or 45° jogs (L36602–36610; PDF 613). Standard-cell rows abut their wells into one contiguous well. Tapless libraries use tap cells at intervals. Pins are a name plus geometry and are case-sensitive for LVS (L36626–36667; PDF 614–615).
- **Serpentine** long-L device: each 90° bend adds W/2, so L = 2L_X + L_Y + W (Fig. 12.29, PDF 615). Serpentines match only identical copies and are used for trickle current sources (L36671–36677).
- **Annular** device: minimum C_D/W, since interdigitation already halves C_D/W. Circular: W = π(B−A)/ln(B/A), L = (B−A)/2, approximately W ≈ ½π(A+B). Square: W ≈ 2(C+D), L ≈ (D−C)/2. Elongated circular: W ≈ π(B−A)/ln(B/A) + 2U. Elongated square: W ≈ 2V + C + D. Square corners cause early avalanche at high voltage. Where contacts over active gate are banned, use the elongated gate crossing onto field (W = 2W_S) or a tic-tac-toe grid gate (W ≈ 4W_S). Identical copies match (Eqs. 12.24–12.31; L36696–36833; PDF 616–618).

**12.2.9 Backgate Contacts (L36837–37047, PDF 618–622)**
- Every MOS needs a backgate contact; otherwise the parasitic PNP/NPN SCR can latch. Trigger is about 0.65 V at 25 °C and 0.4 V at 150 °C, with rising β at high temperature. JESD78 test pulses are ±100 mA to ±250 mA (L36838–36855; PDF 618).
- Latchup conditions: β12·β21·(1−ηc12)(1−ηc21) < 1 (Eq. 12.32, unconditional immunity), or I_T·R_B2·(1−ηc12)·(β12/(β12+1)) < V_trig (Eq. 12.33, conditional immunity). Guard rings reduce η terms and backgate contacts reduce R_B, so each helps the other (L36866–36899; PDF 619).
- Backgates are normally connected so parasitic junctions never forward-bias. Leakage bleeders that forward-bias the backgate diode need guard rings above a few hundred µA (L36903–36911).
- Over a heavily doped sublayer (P+ substrate, or a well touching NBL), vertical conduction dominates, and a large distant substrate-contact area beats a small adjacent contact (L36914–36922).
- **Backgate pinning**: place contacts on the victim's side facing the injector, across all direct paths, as close to the victim as possible. A ring of majority-carrier contacts satisfies Eq. 12.33, while minority-carrier guard rings satisfy Eq. 12.32 (L36926–36973; PDF 620–621, Fig. 12.33).
- Without a sublayer (P-epi on P−, N-well without NBL, isolated tanks), contact every transistor. Butting contacts apply only if backgate = source; otherwise keep NMoat–PMoat spacing (Fig. 12.34). **Maximum distance from any point in the transistor to the nearest backgate contact is typically 25–250 µm.** Exemptions: the backgate touches NBL, or the device cannot inject (always linear) (L36977–37008; PDF 621). Exercise 12.16 uses 50 µm (L38155–38157; PDF 642).
- Interdigitated backgate strips are large. **Distributed backgate contacts** (plugs inside source fingers) add a little source R but save much area, and the plugs must survive outdiffusion and misalignment (L37011–37021; Fig. 12.35).
- Isolated NMOS is immune to CMOS latchup but can still suffer lateral NPN action and snapback at high voltage, which distributed contacts mitigate. NMOS over P+ with a deep-N+ electron-collecting ring resists latchup (L37037–37047).

**12.3 Nonvolatile Memory (L37053–37161, PDF 622–624)**
- Analog products need only a few bytes of NVM, for trim and configuration, in baseline processes. Probe-pad fuse PROM is limited to about 10 bits. Laser links are slow. Transistor-blown fuses need large transistors. OTP EPROM is common for analog trim (L37053–37135).

**12.3.1 Floating-Gate Transistor (L37165–37412, PDF 624–628)**
- Carriers must surmount the oxide interface by heat, UV, hot carriers or Fowler–Nordheim tunneling. Retention follows Arrhenius. UV erase needs a UV-transparent overcoat (L37165–37195).
- FAMOS PMOS is programmed by avalanche and acts as a normally open switch (L37197–37235).
- Double-poly EPROM: V_FG coupling (Eq. 12.34). Programming raises Vt by Q_FG/C_CG (Eqs. 12.35–12.36). It needs V_GS ≈ V_DS, and its high current forces an external programming pin (L37239–37322).
- FLOTOX (L37326–37357). Endurance above 100 k cycles, often characterized at 1 k (L37360–37373).
- Tunneling current follows Eq. 12.37. Program/erase voltage is about 250 % of the maximum operating voltage (5 V oxide → 12.5 V). Tunnel oxide must be at least 60 Å. The voltage should be ramped (L37377–37406; PDF 628).

**12.3.2 Single-Poly EPROM (L37415–37502, PDF 628–630)**
- The cell is a single PMOS with a floating gate and needs gate oxide ≥ 60 Å. A 120 Å, 5 V oxide programs at about 7.5 V, and the drain-extended NMOS takes that voltage (L37415–37420; PDF 628).
- Programming current is about 100 µA per cell, so 128 cells need more than 10 mA. Read voltage is at most half the operating voltage (a 5 V device programs at about 8 V and reads at about 2 V). Latch the read result (L37451–37480; PDF 629).
- Redundancy: two EPROM transistors in parallel whose **gates must not connect** (L37484–37489).
- **Avoid metal over EPROM transistors**: it changes retention through capacitance and blocks UV erase (L37492–37502; PDF 629).

**12.3.3 Single-Poly EEPROM (L37506–37623, PDF 630–632)**
- The cell is a tunnel capacitor (minimum PMOS in its own N-well), a control capacitor (enlarged PMOS in its own N-well) and a sense NMOS, all on one floating gate. **C_C/C_T ≥ 20** (L37518–37524).
- Program, read and erase biasing (Fig. 12.41). The program voltage should be a slow ramp for endurance (L37539–37572).
- Latched EEPROM gives two-cell redundancy (L37586–37597). **Core cell layouts must exactly match the reliability-qualified reference layouts** (L37614–37617; PDF 632).

**12.4 JFET (L37629–37652, PDF 633)**
- JFET inputs give low offset, bias current and noise. Ion-implanted JFETs (1974) fixed offset (L37629–37638).

**12.4.1 Modeling the JFET (L37656–37797, PDF 633–635)**
- S/D are defined by bias. The gate junction must stay reverse-biased. Pinch-off voltage is Eq. 12.38. Shockley linear and saturation currents are Eqs. 12.39–12.41 (L37656–37797).

**12.4.2 JFET Construction (L37799–38022, PDF 635–639)**
- The ideal |V_P| is small and controllable. Without process extensions it usually exceeds a few volts (L37799–37803).
- Epi-FET = epi pinch resistor. Empirical R(V) model and R0 with width, length and corner corrections (Eqs. 12.42–12.43). It is minimum-width, often serpentined, and has contacts on the pinch plate tied to substrate. Rounded bends give little benefit (L37807–37860; PDF 635–636).
- N-well JFET: the PMoat pinch plate must extend much farther into isolation than an epi-FET's, because the well outdiffuses. A minimum-width channel lowers V_P and may need widening (L37864–37894; PDF 636).
- Double-diffused PJFET: V_P varied by tens of mV across a die, and common-centroid "could only do so much" (L37898–37943; PDF 637).
- Ion-implanted PJFET: gate contacts sit inside an emitter diffusion, not the shallow N implant, because of spiking. Multi-finger devices look like MOSFETs. An **annular JFET separates top gate and back gate** for low input C and leakage, sized with the annular MOS equations (L37952–38007; PDF 638–639).

**12.5 Summary / Bibliography / 12.6 Exercises (L38028–38168, PDF 639–642)**
- Ch.13 covers extended-voltage, power and DMOS devices (L38028–38033). Bibliography: Troutman on latchup, JESD78E, Tsividis, Takeda (L38038–38067).
- Exercises cover guard rings around the inverter PMOS/NMOS (12.7), isolated NMOS with a butting contact (12.8), N(W/L) layouts with butting contacts (12.10), NatVt rules (12.11), stackable standard cells (12.12), a serpentine PMOS (12.13), annular devices (12.14–12.15), a 5000/3 PMOS with interdigitated contacts at ≤ 50 µm (12.16), and EEPROM/epi-FET layouts (12.17–12.20) (L38073–38168).

## 3. Actionable extraction

### H12-01 Bias-defined source/drain roles
- Kind: data-model
- Statement: Which MOS terminal is the source depends on bias. If V_DS < 0 (NMOS) the roles swap, and the Shichman–Hodges equations apply to the swapped device (L34273–34279). JFETs work the same way (L37678–37681).
- Source: §12.1, §12.4.1; L34273–34279, L37678–37681; PDF 576, 633.
- Philis stage: annotator, cells.
- Automation recipe: Read V_DS sign per device from the op-point (and, where available, per corner or transient). A device whose |V_DS| changes sign (pass gates, switches, bleeders) is tagged `bidirectional`. For those devices, the "minimize drain" choice (H12-29) and drain-side rules (GIDL H12-09, halo H12-26) apply to **both** diffusions. Output a per-device `role_of_terminal` map that cells and routing use instead of netlist pin names.
- Beats hand layout because: a human trusts labels. Automation checks every device against actual bias.
- Philis status: partial. The generator fixes S/D by pin label (`kernel/cells/src/mosfet.rs:324-342`). The op-point extracts `vds` (`frontend/library/src/oppoint.rs:186`, `:238`), but nothing re-labels terminals.

### H12-02 Effective width/length with process biases; ratioed devices need an identical unit
- Kind: formula
- Statement: k = k'·W_e/L_e, W_e = W_d + W_b, L_e = L_d + L_b, and k' = μ·ε0·εr/t_ox (Eqs. 12.6–12.9). Biases are "a few tenths of a micron" and negligible only for large devices (L34337–34366; PDF 577).
- Source: §12.1.1 Eqs. 12.6–12.9; L34330–34382; PDF 577–578.
- Philis stage: annotator, cells.
- Automation recipe: For any ratioed pair (a mirror with ratio N, or a DAC branch), the ratio error from biases is ΔR/R ≈ W_b·(1/W_f,a − 1/W_f,b) when finger widths differ. Force identical unit fingers so W_b cancels (ties to H12-28). Report the predicted bias error when the unit cannot be shared.
- Beats hand layout because: the error is computed exactly per ratio instead of being guessed.
- Philis status: partial. One `unit_w` per unitization class (`kernel/cells/src/builder.rs:245-263`, `backend/annotator/src/constraints.rs:3-7`). Devices of different W fall into different classes.

### H12-03 Temperature coefficients for thermal-gradient matching
- Kind: formula
- Statement: Mobility goes as ∝T^−1.7 for electrons and ∝T^−1.5 for holes, and k(150 °C) ≈ 0.6·k(25 °C) (L34436–34441; PDF 578). Vt TC is about −1.5 mV/°C for 100 Å oxide and −2.5 mV/°C for 500 Å (L34806–34808; PDF 584).
- Source: §12.1.1, §12.1.2; PDF 578, 584.
- Philis stage: annotator, gp, dp (thermal rule).
- Automation recipe: For a matched pair in a thermal field, ΔV_GS,offset = TC_Vt·ΔT + (V_gst/2)·(ΔI/I from the mobility term). The mobility term is Δk/k ≈ −1.7·ΔT/T (NMOS) or −1.5·ΔT/T (PMOS). Include the mobility term alongside the Vt term when converting the thermal budget into allowed ΔT between members.
- Beats hand layout because: every matched pair gets a numeric ΔT budget from its own bias, not a blanket "keep away from heat".
- Philis status: partial. Vt TC is used (`backend/annotator/src/emit.rs:179`, `backend/annotator/src/netrole.rs:98`, `pdks/gf180mcu.json:18`). The mobility TC is missing.

### H12-04 Backgate bias raises Vt and random Vt variation
- Kind: check
- Statement: V_BS ≠ 0 shifts Vt (Table 12.4: NMOS 0.17 → 0.43 → 0.61 → 0.89 V at V_BS = 0/−1/−2/−4 V) and "also increases the random variation of the threshold voltage" (L34627–34633; PDF 581–582).
- Source: §12.1.2 Table 12.4; L34627–34648.
- Philis stage: annotator, cells, verify.
- Automation recipe: From the netlist (B vs S nets) and the op-point, compute V_SB per device. For each matched set: (a) if the members' V_SB differ, flag a hard mismatch source unless each member gets a private well tied to its own source; (b) if V_SB ≠ 0 on precision pairs, emit an advisory with σ inflation "not given" numerically in this chapter. For PMOS pairs whose bulk ≠ source, offer a generator variant with a source-tied private N-well.
- Beats hand layout because: V_SB is checked on every matched set automatically.
- Philis status: missing (no V_SB, body-effect or bulk handling in annotator or analog; grep `vsb|body.effect|backgate`: 0 hits).

### H12-05 Interface traps and HCI shift Vt over lifetime
- Kind: heuristic
- Statement: Incomplete hydrogen anneal raises random Vt variation and 1/f noise (L34687–34696). HCI raises NMOS Vt and lowers I_D, while PMOS can shift either way (L34704–34709). Antenna charging raises GIDL (L35190–35192).
- Source: §12.1.2; L34687–34709; PDF 582.
- Philis stage: annotator, gr/dr (antenna).
- Automation recipe: For matched sets, compare |V_DS| and I_D across members from the op-point. Unequal stress means unequal HCI ageing. Flag pairs whose |V_DS| differ by more than a tolerance (the tolerance is a design choice; the source gives no number). Treat gate nets of matched pairs as antenna-critical and use a stricter ratio, e.g. require diode protection early.
- Beats hand layout because: stress symmetry is verified numerically. (The equal-stress rule is an inference from the stated mechanism.)
- Philis status: missing for stress symmetry. Antenna routing exists (`kernel/analog/src/routing/antenna.rs:13-16`).

### H12-06 Long-L output-resistance devices and pocket-free assumption
- Kind: heuristic
- Statement: r_o scales about linearly with drawn L when backgate doping is uniform, and "lengths of 20–40 µm are not uncommon" (L34880–34885; PDF 585). With pocket implants, r_o ∝ √L instead (L36452–36458).
- Source: §12.1.3 CLM Eqs. 12.16–12.17; §12.2.7; PDF 584–585, 611.
- Philis stage: annotator, cells.
- Automation recipe: Classify devices with L ≥ 10·L_min as `long_channel`. For them: (1) offer a serpentine or folded-channel variant when W/L < 1 (H12-35); (2) if the deck flags halo implants, warn that r_o gain from L is only √L.
- Beats hand layout because: the tool selects the compact long-L structure automatically.
- Philis status: missing (no long-L classification and no serpentine MOS; `backend/dr/src/lib.rs:942` mentions serpentine only for routing stubs).

### H12-07 Minimum finger width for matched devices (narrow-channel effect)
- Kind: rule
- Statement: The narrow-channel effect raises |Vt| for W below "a few microns" (L34973–34978). SCE matters for L below a few µm and DITS for L < 2 µm (L34942–34964; PDF 585–586).
- Source: §12.1.3; L34942–34978.
- Philis stage: annotator, cells.
- Automation recipe: When refolding (H12-27/28), never choose unit finger width below `W_nce_min`, a deck value. The source says only "a few µm", so the exact number is not given and should be a deck key. The same limit bounds how many fingers a matched device may be split into.
- Beats hand layout because: the limit is enforced uniformly on every refold.
- Philis status: missing (no refold exists; no deck key).

### H12-08 Subthreshold (weak inversion) device tagging
- Kind: check
- Statement: Subthreshold region: V_FB ≤ V_GS ≤ V_t (Table 12.5). I_D = I_0·e^((V_GS−V_X)/(nV_T)) (Eq. 12.20), and S = 2.30·n·V_T with S ≥ 60 and typically 80 mV/dec (Eq. 12.23). Subthreshold circuits do not work well much above 70 °C (L35092–35160; PDF 588).
- Source: §12.1.3 Subthreshold; PDF 586–588.
- Philis stage: annotator, gp/dp (thermal).
- Automation recipe: Extend the op-point parse to V_GS and V_th, since today only id, vds, vdsat and gm are parsed. Tag devices with V_GS < V_th as `weak_inversion`. In weak inversion I_D mismatch is exponential in ΔV_t (ΔI/I = ΔV_t/(nV_T)), so raise the matching tier of weak-inversion pairs, and put them in the cold region of the thermal map with a hard ΔT budget.
- Beats hand layout because: the operating region is known per device instead of assumed.
- Philis status: missing (`frontend/library/src/oppoint.rs:238` shows id, vds, vdsat and gm only; no region classification).

### H12-09 GIDL-sensitive off-state devices
- Kind: heuristic
- Statement: GIDL is exponential in drain–gate field and in temperature. It is worst in cutoff at high V_DS and high temperature, and HCI or antenna damage raises it (L35162–35202; PDF 589).
- Source: §12.1.3 GIDL.
- Philis stage: annotator, gp, gr/dr.
- Automation recipe: Tag devices that sit in cutoff with |V_DG| near the maximum (sample switches, bleeders, hold-capacitor switches). For them, apply the strictest antenna ratio on the gate net and keep them out of hot spots (thermal rule).
- Beats hand layout because: leakage-critical switches are identified from bias automatically.
- Philis status: missing.

### H12-10 Snapback / BV_DII: low backgate resistance for pass and power devices
- Kind: rule
- Statement: Avalanche holes debias the backgate and trigger the parasitic NPN (snapback). BV_DII is measured at V_GS ≈ ½V_GS,max. Pass or power devices need BV_DII ≥ max operating voltage, which "requires backgate resistance be minimized", e.g. with integrated backgate contacts (L35240–35284; PDF 590–591). Isolated HV NMOS gets distributed contacts (L37042–37046).
- Source: §12.1.4; §12.2.9.
- Philis stage: annotator, cells.
- Automation recipe: Identify pass or power devices: large W, source or drain on a supply or output pin, and V_DS near V_max from the op-point or pin spec. Force the distributed-backgate-contact variant (H12-44) and bound the maximum in-device distance to a contact more tightly than the general rule.
- Beats hand layout because: the rule is applied to every qualifying device, not only those a designer remembers.
- Philis status: missing.

### H12-11 Gate-oxide field limit ERC
- Kind: check
- Statement: Oxide strength is about 10 MV/cm. Operate at ≤ 3.5 MV/cm for t_ox > 30 nm and ≤ 5 MV/cm for t_ox ≤ 15 nm (L34417–34421; PDF 578). Tunnel/EEPROM program at about 10 MV/cm, while thin oxides operate at about 4 MV/cm maximum (L37386–37389).
- Source: §12.1.1, §12.3.1.
- Philis stage: verify, deck.
- Automation recipe: Per device oxide class (deck: t_ox, V_GS,max), check the op-point and pin ranges: |V_GS|, |V_GD| and |V_GB| ≤ E_max·t_ox. Also check that thin-oxide gates driven from high-voltage nets are flagged.
- Beats hand layout because: every gate is checked, not just the obvious ones.
- Philis status: missing (grep `gate_ox|thick_ox`: 0 hits).

### H12-12 Coding-layer derivation and S/D implant clipping
- Kind: deck-requirement
- Statement: PSD = PMoat⊕1.0 and NSD = NMoat⊕1.0 (the oversize must exceed one-layer misalignment). Clip with PSD = PSD not common to NMoat and NSD = NSD not common to PMoat. With Active+Tap coding: SD = Active⊕1.0, SDTap = SD∩Tap, SDNoTap = SD−Tap, NSD = (SDNoTap−NWell)+(SDTap∩NWell), PSD = (SDNoTap∩NWell)+(SDTap−NWell) (L35399–35457; PDF 593–595).
- Source: §12.2.1.
- Philis stage: cells, deck, verify.
- Automation recipe: The generator draws implants explicitly (nsdm/psdm). When two cells abut NMOS diffusion to PMOS tap (butting contacts H12-30, merged cells), run the clipping rule on the combined implant geometry inside the placer's merge step. Add a DRC-style self-check that implant overlap exists only where the deck permits it.
- Beats hand layout because: clipping is exact on every abutment. (Hastings: hand or deck clipping "sometimes produce questionable results".)
- Philis status: partial. The annotator keeps NMOS/PMOS apart because abutment merges implants (`backend/annotator/src/lib.rs:43-45`). The generator draws implant enclosures (`kernel/cells/src/mosfet.rs:573-592`).

### H12-13 Well spacing per voltage class
- Kind: deck-requirement
- Statement: A generated P-well is the drawn N-well inverted and undersized by 2.0. Each N-well voltage level needs its own spacing (NwellLV⊕2.0, NwellHV⊕3.0) (L35558–35583; PDF 596–597).
- Source: §12.2.2.
- Philis stage: deck, gp/dp, verify.
- Automation recipe: The deck carries `well_space_by_voltage[class]`. The annotator assigns each well (bulk net) a voltage class from the pin or op-point maximum. The placer uses a pairwise well-to-well gap equal to the rule for the higher of the two classes. Same-net, same-class wells may merge (H12-33).
- Beats hand layout because: spacing is exact per class instead of using the worst case everywhere.
- Philis status: missing (a single `NWELL.2` rule is used in tests: `frontend/library/tests/ota_cross_pdk.rs:56`).

### H12-14 Isolated-NMOS (P-tank) generator and isolation-bias ERC
- Kind: data-model
- Statement: An isolated NMOS in a P-tank (deep N-well to NBL, deep-N+, up-down or DTI) has five terminals, and "the isolation terminal must be biased at or above the backgate" (L35625–35688; PDF 598–599). A deep N-well has high vertical resistance, and a deep-N+ plug reduces it for latchup immunity (L35652–35658).
- Source: §12.2.2, Fig. 12.14.
- Philis stage: cells, annotator, verify.
- Automation recipe: When an NMOS bulk net is not the substrate net, (1) select the isolated-NMOS generator: dnwell ring and tank taps, plus a deep-N+ plug if the deck has one; (2) add ERC V(iso) ≥ V(bulk) at the op-point and across pins; (3) connect iso by default to the highest supply or to the bulk, per the text (L35686–35688).
- Beats hand layout because: tank construction and bias checks are generated automatically for every non-substrate NMOS body.
- Philis status: missing for MOS. dnwell is used only for NPN isolation (`kernel/cells/src/bjt.rs:34`, `:129-196`).

### H12-15 Doubly isolated PMOS and isolation voltage limits
- Kind: check
- Statement: The seven-terminal doubly isolated PMOS limits backgate-to-isolation to 5–10 V, or about 20 V with PBL. Shallow N-well breakdown is 20–30 V and deep isolation 40–80 V (L35692–35718; PDF 599).
- Source: §12.2.2, Fig. 12.15.
- Philis stage: verify, deck.
- Automation recipe: The deck carries `bv_well_iso`, `bv_nwell_sub` and `bv_iso_sub`. ERC compares these with the max |ΔV| between the corresponding nets over all pin states.
- Beats hand layout because: a numeric check runs on every well.
- Philis status: missing.

### H12-16 Tank grouping by injection source
- Kind: algorithm
- Statement: Identify, before layout, every isolated NMOS that can inject into its tank. Devices whose drains connect to the same pin may share a tank. Devices on different pins should not, "unless one can demonstrate that this is not an issue" (L35722–35735; PDF 599–600).
- Source: §12.2.2.
- Philis stage: annotator, gp.
- Automation recipe: For isolated NMOS, compute `inj_key = set of external pins reachable from the drain through ≤ 1 device` (a direct pin connection is the case the text names). Partition devices by (bulk net, inj_key). Each partition is a tank group: a `SameTank` constraint within the partition and `DifferentTank` across partitions. Emit these as hard constraints to gp.
- Beats hand layout because: the partition is exhaustive and derived from connectivity.
- Philis status: missing.

### H12-17 Noisy vs sensitive tank and well separation (parasitic rectification)
- Kind: rule
- Statement: Fast-switching isolated NMOS inject noise through drain–backgate capacitance and "should never be merged" into a tank with sensitive analog. Parasitic rectification shifts DC points of nonlinear nodes, and bandgaps are "notoriously vulnerable" (L35739–35745; PDF 600).
- Source: §12.2.2.
- Philis stage: annotator, gp.
- Automation recipe: Net roles tag switching nets (clock, PWM, digital) and sensitive nets (bandgap core, high-impedance, reference). A device is noisy if its drain is on a switching net. For well or tank sharing, forbid a shared bulk region between noisy and sensitive devices (a hard `DifferentWell` constraint), in addition to distance.
- Beats hand layout because: every well merge is checked against net roles.
- Philis status: partial. There is a distance-only noisy→sensitive rule (`kernel/analog/src/placement/isolation.rs:7-18`) but no well or tank separation constraint.

### H12-18 Tank contacts scattered with a maximum distance
- Kind: rule
- Statement: "Numerous tank contacts should be scattered throughout every P-type tank", with a maximum distance from an isolated NMOS to the nearest backgate contact. Designers often contact or ring every isolated transistor (L35748–35752; PDF 600).
- Source: §12.2.2.
- Philis stage: cells, dp, verify.
- Automation recipe: Same as H12-43 but with a tank-specific distance from the deck, plus a requirement of at least one contact per isolated device.
- Beats hand layout because: the distance check is exact.
- Philis status: missing.

### H12-19 Thick-field (parasitic channel) voltage check per conductor
- Kind: check
- Statement: Each conductor has its own thick-field Vt, poly lowest, then M1, M2 and so on. Process summaries quote only M1. Derate published thick-field Vt by at least 30 %: a 40 V M1 PMOS threshold allows at most about 25 V between M1 and the N-well beneath (L35842–35856; PDF 601). The field threshold is about 50 V at 10^17 cm^−3 under 1 µm oxide (L35763–35766).
- Source: §12.2.3.
- Philis stage: deck, gr, dr, verify.
- Automation recipe: The deck carries `thick_field_vt[layer][well_type]`. For each routed segment over a well or field region of type T, check |V(net) − V(well net)| ≤ 0.7·Vt_tf[layer][T]. The router treats violating overlaps as keep-outs and prefers higher metals for high-voltage nets.
- Beats hand layout because: every overlap is computed and there is no reliance on "M1 is fine".
- Philis status: missing.

### H12-20 Poly stub / poly routing across wells
- Kind: rule
- Statement: Poly has the lowest thick-field threshold. A poly gate "inadvertently" extended beyond a well (a poly stub) can invert the underlying silicon. Poly routing is used in low-metal-count products (L35845–35849; PDF 601).
- Source: §12.2.3.
- Philis stage: cells, gr, verify.
- Automation recipe: Generator: gate end-caps must not extend across a well boundary. Router: forbid poly routes over an opposite-type well, or over a well at a different potential, when |ΔV| > 0.7·Vt_tf[poly]. Verify: add a geometric check "poly ∩ (well boundary ± ext) on a net with |V − V_well| > limit".
- Beats hand layout because: stubs are caught automatically.
- Philis status: missing.

### H12-21 Native and special-Vt marker layers around gates
- Kind: rule
- Statement: The NatVt layer is coded around the gate of each natural transistor and slightly overlaps the channel for misalignment and lateral outdiffusion (L35930–35936; PDF 602). Example rules: NATVT width 4 µm, overlap of GATE 2 µm, spacing to POLY 4 µm, NATVT-to-NATVT 2 µm, where GATE = POLY ∩ (NMOAT ∪ PMOAT) (L38125–38142; PDF 642). Other Vt options are coded the same way (L36036–36039).
- Source: §12.2.4, Fig. 12.17, Exercise 12.11.
- Philis stage: cells, gp/dp, deck.
- Automation recipe: Device model → `vt_flavor`, and the deck maps flavour → (layer, enc_gate, width, space_poly, space_same). The generator draws the marker with enc_gate around every gate, including dummies. The placer treats same-flavour neighbours as mergeable, since markers at a spacing below `space_same` merge. Otherwise it keeps the marker-to-foreign-poly spacing.
- Beats hand layout because: spacing and merging are optimized with the marker rules.
- Philis status: partial. Layer roles `hvtp` and `lvtn` exist in the deck role list (`backend/verify/src/pdk.rs:1212`), but the MOS generator never draws them (grep `lvtn|hvtp` in `kernel/cells/src`: 0 hits).

### H12-22 Dual-doped poly: PPoly/NPoly junction placement and full-coverage DRC
- Kind: deck-requirement
- Statement: PPoly/NPoly junctions form diodes unless silicided. Silicide accelerates dopant diffusion, so junctions "must be spaced well away from gate regions". The DRC should check that all poly receives NPoly or PPoly (L36016–36032; PDF 604).
- Source: §12.2.4.
- Philis stage: cells, verify.
- Automation recipe: When a gate strap joins NMOS and PMOS gates (inverter-style shared poly), place the doping split at a deck distance from both gates. Add a verify check that `poly ⊆ npoly ∪ ppoly` where the deck has these layers.
- Beats hand layout because: every shared-poly net is checked.
- Philis status: missing.

### H12-23 Thick-oxide marker (Moat-2) around gates
- Kind: rule
- Statement: Thick-gate devices are marked by a Moat-2 geometry drawn around the gate region. It defines the thick-oxide threshold-adjust region and, in etch-and-regrow, the protected oxide (L36078–36083; PDF 605–606).
- Source: §12.2.5, Fig. 12.20.
- Philis stage: cells, gp/dp, deck.
- Automation recipe: The device model maps to an oxide class, and the deck maps oxide class → (marker layer, enclosure, spacing). The generator draws the marker. The placer clusters same-oxide devices so markers merge and the thin-to-thick transition spacing is paid once per cluster.
- Beats hand layout because: clustering is global instead of per device.
- Philis status: missing (grep `thick_ox|thkox|dual_gate`: 0 hits).

### H12-24 Never reuse analog layout by shrink: resimulate post-layout
- Kind: rule
- Statement: Analog does not scale in unison. Resistors are unchanged (ignoring biases) while capacitances diminish by 1/S², so "Analog circuits must never be scaled down without thoroughly resimulating" (L36237–36240; PDF 607).
- Source: §12.2.6.
- Philis stage: flow, verify.
- Automation recipe: Cross-PDK retargeting must re-extract and resimulate, not scale geometry.
- Beats hand layout because: the tool can resimulate every epoch.
- Philis status: implemented. PEX is part of signoff and epoch ranking (`frontend/library/src/lib.rs:5`, `:337`, `:910`).

### H12-25 Halo/pocket-implant processes: analog consequences
- Kind: deck-requirement
- Statement: Pocket implants make r_o ∝ √L rather than L, and mismatch does not follow Pelgrom 1/√L. "Accurate current mirrors cannot be constructed using pocket-implanted transistors without resorting to resistive degeneration" (L36452–36471; PDF 611).
- Source: §12.2.7, Fig. 12.24.
- Philis stage: deck, annotator.
- Automation recipe: The deck flag `halo: none|blanket|directional(axis)` controls three things. (1) The Pelgrom-based σ in the annotator should not extrapolate with 1/√(WL) for halo devices; use a measured table per L. (2) For precision mirrors in halo processes, emit an advisory for degeneration (a design change, reported not applied). (3) For a directional halo, see H12-26.
- Beats hand layout because: the σ model follows the actual process.
- Philis status: missing (grep `halo|pocket` hits are only well or ring halos, `kernel/cells/src/mosfet.rs:626-630`, `kernel/cells/src/post_cell.rs:291`).

### H12-26 Directional halo: forbid 90° rotation of devices or blocks
- Kind: rule
- Statement: With tilted pocket implants shot only left and right, horizontal-L devices get pockets (digital) and vertical-L devices do not (analog). "A block can safely be reflected or rotated by 180°, but it cannot be rotated by 90°" (L36480–36487; PDF 611).
- Source: §12.2.7.
- Philis stage: gp, dp, cells, deck.
- Automation recipe: Where the deck says `halo = directional(axis)`: analog devices must have their channel-length axis ⟂ to the implant axis, and digital devices ∥. The generator draws gates in the required orientation. dp removes R90, R270, Mx90 and Mx270 from the move set for **all** devices, not only matched ones. Verify checks gate orientation against device class.
- Beats hand layout because: there is no accidental 90° turn of a block.
- Philis status: partial. Matched devices never rotate (`backend/dp/src/lib.rs:555-560`), but unmatched cells get quarter-turn moves (`backend/dp/src/lib.rs:562-575`; `Orient::R90` in `kernel/core/src/geom.rs:22-33`). There is no deck flag.

### H12-27 Finger count policy (N(W/L) notation, ESD exception)
- Kind: rule
- Statement: Use one section for W/L below about 10. Above that, divide into identical parallel sections (L36503–36508). An annotated N(W/L) "should be laid out exactly as specified". Otherwise refold with N·W_f = W (1000/0.5 → 20(50/0.5)) and communicate the change to the designer. ESD robustness depends on per-finger width, so 20/0.5 → 2(10/0.5) "might halve its ESD robustness" (L36525–36538; PDF 612).
- Source: §12.2.8, Fig. 12.25.
- Philis stage: annotator, cells.
- Automation recipe: (1) If the schematic gives nf or m, lock it. (2) If nf = 1 and W/L > 10 and the device is not ESD, enumerate nf ∈ {divisors giving W_f ≥ W_nce_min (H12-07)} as generator variants, with the LVS reference emitting the matching nf card. (3) If the device matches an ESD pattern, or its drain is on a pad net, lock the per-finger width. (4) Log each refold in the report for designer sign-off.
- Beats hand layout because: the aspect/parasitic search covers all legal foldings.
- Philis status: partial. nf is taken from the schematic and never refolded (`kernel/cells/src/mosfet.rs:75-78`). An ESD clamp pattern exists (`backend/annotator/src/catalog.rs:1811-1815`), but there is no per-finger lock and no refold enumeration.

### H12-28 Common unit finger width for matched/ratioed devices
- Kind: algorithm
- Statement: "Matched transistors must use sections of the same width". For 100/0.5 and 2(100/0.5), choose 25 µm sections, giving 4(25/0.5) and 8(25/0.5) (L36541–36546; PDF 612).
- Source: §12.2.8.
- Philis stage: annotator, cells.
- Automation recipe: For a matched set with total widths W_i and equal L, the candidate units are u = gcd(W_i)/k for integer k, subject to u ≥ W_nce_min and u ≤ W_max_finger (deck or aspect). Pick u by cost = aspect error + area + (number of fingers)·C_parasitic weight. Emit a single unitization per matched set with dev_nf_i = W_i/u. Report it to the designer as a schematic change. Unequal L cannot be unitized this way; keep L equal (a precondition).
- Beats hand layout because: the optimal u is found exactly, instead of the designer's first guess.
- Philis status: partial. Unitization is per (kind, W, L) class (`backend/annotator/src/constraints.rs:3-7`, `:28-40`), so ratioed matched devices of different W land in different classes and cannot share one unit.

### H12-29 Odd/even sections and "minimize drain" choice
- Kind: heuristic
- Statement: An odd section count gives equal S and D finger counts. An even count gives one extra S or D, and the extra electrode has more junction capacitance. PCells offer minimize-drain or minimize-source, and designers "generally prefer to minimize the drain". High-speed designs split small transistors into two sections to cut drain capacitance (L36550–36564; PDF 612–613).
- Source: §12.2.8, Fig. 12.25.
- Philis stage: annotator, cells.
- Automation recipe: Per device, choose outer electrodes to minimize C on the higher-impedance or higher-|dV/dt| node from the op-point and net role. The drain is the default. For a source on a high-impedance node (source follower output, cascode internal node), minimize the source instead. For devices with nf = 1 on fast nets, offer nf = 2 as a variant (C_D roughly halves).
- Beats hand layout because: the choice follows node impedance for every device.
- Philis status: partial. A single device always starts with S at region 0, so even nf gives source-outer, i.e. minimized drain (`kernel/cells/src/mosfet.rs:324-328`). Multi-device rows start with D outer. There is no per-node choice.

### H12-30 Abutting (butting) backgate contacts
- Kind: rule
- Statement: When backgate = source, a backgate contact strip can abut the source. This saves the NMoat–PMoat spacing and "more effectively pin[s] the backgate". With odd sections it can sit at one end only; with even sections and minimized drain, at either or both ends; with minimized source, at neither (L36568–36574, L36982–36986; PDF 613, 621). If backgate ≠ source, keep NMoat–PMoat spacing (Fig. 12.34B).
- Source: §12.2.8 Fig. 12.25; §12.2.9 Fig. 12.34.
- Philis stage: cells.
- Automation recipe: Generator variant `tap: row_strip | butt_end(left|right|both)`. It is legal only if the bulk net equals the source net of the outer finger. It uses an abutted diff/tap with implant clipping (H12-12). Score variants on area and on max distance-to-tap (H12-43).
- Beats hand layout because: every legal tap style is enumerated and scored.
- Philis status: missing. The generator always draws a tap strip along one side of the row (`kernel/cells/src/mosfet.rs:567-572`; `docs/LAYOUT-FUNDAMENTALS.md:69`).

### H12-31 Merged devices sharing S/D (notched moat)
- Kind: algorithm
- Statement: Transistors sharing a source or drain net merge even with unequal widths by using a notched moat. The poly-to-moat spacing S_PM enlarges the shared region slightly, but the shared electrode area and capacitance are less than two separate ones. A separate backgate contact nearby can serve other devices (L36577–36599; Fig. 12.26; PDF 613).
- Source: §12.2.8.
- Philis stage: annotator, cells, dp.
- Automation recipe: The annotator finds pairs of non-matched devices of the same polarity and bulk that share an S or D net (diffusion-sharing graph). Offer merged macros as alternatives (gp/dp `variants`). Where W differs, draw a notched diff with S_PM clearance. Score by area and shared-node C.
- Beats hand layout because: all sharing candidates are enumerated. This is the classic Euler-path diffusion-sharing search, applied exhaustively.
- Philis status: partial. Sharing exists only within one matched group row (`kernel/cells/src/mosfet.rs:324-326`), requires equal unit_w, and has no notched moat. There is no cross-group merge.

### H12-32 Series stacks without internal contacts; gate jogs
- Kind: heuristic
- Statement: In series devices (NAND M3/M4) "the drain of M3 simultaneously acts as the source of M4", so the internal node needs no contacts. Gates can be brought closer with short 90° or 45° jogs (L36602–36610; PDF 613).
- Source: §12.2.8, Fig. 12.27.
- Philis stage: cells.
- Automation recipe: For series pairs whose internal net has no other connection, the chain variant omits contacts on the internal diffusion and uses the poly-to-poly spacing only. This minimizes the internal node C (cascode nodes).
- Beats hand layout because: every series-stack candidate is detected and drawn this way automatically.
- Philis status: partial. A Chain style exists for series stacks (`kernel/cells/src/mosfet.rs:80-86`, `:331-336`). Whether internal regions are left uncontacted was not verified.

### H12-33 Shared/contiguous wells across abutting cells; tap cells
- Kind: rule
- Statement: Standard cells abut so their wells "overlap to form a single contiguous region", avoiding well-to-well spacings. Tapless libraries place tap cells at intervals (L36626–36644; PDF 614).
- Source: §12.2.8, Figs. 12.27–12.28.
- Philis stage: gp, dp, cells.
- Automation recipe: dp: same-bulk-net PMOS cells whose wells lie within the well spacing get their wells merged (bridged) instead of spaced, which reduces area. Add `WellMerge` attraction between same-bulk PMOS in gp. Tap cells: if cells are drawn tapless, insert tap cells so the max distance-to-tap (H12-43) holds.
- Beats hand layout because: global well-merge optimization.
- Philis status: partial. Bridged wells merge in the geometry output (`frontend/library/src/lib.rs:99`), but there is no cross-cell well-sharing objective (`docs/LAYOUT-FUNDAMENTALS.md:41`, `:114`).

### H12-34 Pins: name plus geometry, case-sensitive
- Kind: check
- Statement: A pin is a signal name plus a geometry. Pin names must match the schematic, case-sensitively, for LVS (L36660–36667; PDF 615).
- Source: §12.2.8.
- Philis stage: verify.
- Automation recipe: A pre-LVS check that the set of top-level pin labels equals the schematic port set, compared exactly. A case-only mismatch is reported as a distinct error.
- Beats hand layout because: the check is trivial and exhaustive.
- Philis status: not checked (status not assessed).

### H12-35 Serpentine long-channel MOS generator
- Kind: formula
- Statement: A moat strip folded under a poly plate. Each 90° bend adds W/2 to the drawn length, so L = 2L_X + L_Y + W for Fig. 12.29. Serpentines "will not match very accurately unless they use identical layouts". They are used for trickle current sources (L36671–36677; PDF 615–616).
- Source: §12.2.8 Serpentine Transistors, Fig. 12.29.
- Philis stage: cells, annotator.
- Automation recipe: Generator `SerpentineMos { segments, seg_len }` solves L = Σ straight segments + (n_bends)·W/2 for the target L, with a near-square aspect (Exercise 12.13). Offer it only for devices not in matched sets, or for matched sets whose members use the identical macro. The LVS reference must report the device as W/L with that L.
- Beats hand layout because: exact L and optimal aspect are computed, with no hand counting of bends.
- Philis status: missing.

### H12-36 Annular (enclosed) MOS generator and W/L formulas
- Kind: formula
- Statement: Circular: W = π(B−A)/ln(B/A) and L = (B−A)/2, with approximation W ≈ ½π(A+B), which overestimates slightly. Square: W ≈ 2(C+D), L ≈ (D−C)/2. Elongated circular: W ≈ π(B−A)/ln(B/A) + 2U, L ≈ (B−A)/2. Elongated square: W ≈ 2V + C + D, L ≈ (D−C)/2. Annular gives minimum C_D/W, while interdigitation already halves C_D/W. Square corners cause early avalanche at high voltage. The elongated form is not a C_D/W gain but is common for HV devices (Eqs. 12.24–12.31; L36696–36807; PDF 616–618).
- Source: §12.2.8 Annular Transistors, Figs. 12.30–12.31.
- Philis stage: cells, annotator.
- Automation recipe: Offer an annular variant for devices whose drain is a high-speed or high-impedance node with the source on a low-Z rail (the text's condition, L36703–36704), and for HV devices (circular preferred). Compute W and L from the formulas and emit these into the LVS reference. Matching relies on identical copies (L36745–36747).
- Beats hand layout because: exact W/L from closed forms, plus automatic choice when C_D matters.
- Philis status: missing.

### H12-37 Annular MOS with gate contacts over field
- Kind: formula
- Statement: If contacts over active gate are banned, use an elongated gate whose ends cross the moat onto field (W = 2W_S, W_S measured between moat edges), or a tic-tac-toe grid gate (W ≈ 4W_S) with four S/D regions. Identical copies match (L36809–36818; Fig. 12.32; PDF 618).
- Source: §12.2.8.
- Philis stage: cells, deck.
- Automation recipe: Choose between H12-36 and H12-37 using the deck flag `contact_over_active_gate`. Emit W from the formulas.
- Beats hand layout because: the correct form is selected per deck automatically.
- Philis status: missing. The regular MOS generator already places gate contacts over field (`kernel/cells/src/mosfet.rs:343-367`).

### H12-38 Latchup immunity metric (Eqs. 12.32–12.33)
- Kind: metric
- Statement: Unconditional immunity requires β12·β21·(1−ηc12)(1−ηc21) < 1 (Eq. 12.32). Conditional immunity requires I_T·R_B2·(1−ηc12)·β12/(β12+1) < V_trig (Eq. 12.33). V_trig ≈ 0.65 V at 25 °C and ≈ 0.4 V at 150 °C. I_T is the test current (JESD78: ±100 mA to ±250 mA). Guard rings reduce the η terms and backgate contacts reduce R_B (L36838–36899; PDF 618–619).
- Source: §12.2.9.
- Philis stage: verify, dp (cost term).
- Automation recipe: For each injector (a device with drain or source on an external pin) and each complementary victim within a radius, estimate β12 from spacing, η from the guard rings present (deck-calibrated η per ring type), and R_B2 from the victim's backgate path. That path is the tap distance × sheet R, or vertical R if a sublayer exists (H12-41). Compute the Eq. 12.33 margin with I_T from the deck (JESD78 class) and V_trig at T_max. Hard constraint: margin > 0. Soft cost: −log(margin).
- Beats hand layout because: a quantitative, per-pair latchup margin replaces rules of thumb.
- Philis status: missing (grep `latchup`: 0 hits; `docs/LAYOUT-FUNDAMENTALS.md:137`).

### H12-39 Guard rings where injection is possible, not everywhere
- Kind: rule
- Statement: Guard rings "require so much room that they can be placed only around a few devices — usually those that potentially inject minority carriers". Backgate contacts go on every device (L36896–36899). A bleeder whose backgate diode can conduct more than a few hundred µA needs guard rings; below that they are "generally not necessary" (L36906–36911; PDF 619).
- Source: §12.2.9.
- Philis stage: annotator, cells.
- Automation recipe: Injector detection: (a) devices with a terminal on an external pin (can be pulled beyond the rails); (b) devices whose source–backgate diode forward-biases in some mode (bleeders), with I_diode > 200 µA as a deck threshold ("a few hundred µA" in the text); (c) inductive or switching drivers. Rings go on injectors (minority-carrier type) and backgate contacts on all devices. Victims near injectors get pinning taps (H12-42).
- Beats hand layout because: rings are placed where needed, saving area elsewhere.
- Philis status: partial. The annotator requests a guard ring on every FET of every non-glue block regardless of injection (`backend/annotator/src/constraints.rs:64-84`), which over-applies rings and costs area.

### H12-40 Latchup-temperature derating
- Kind: formula
- Statement: V_trig ≈ 0.65 V at 25 °C and ≈ 0.4 V at 150 °C, and parasitic β rises with temperature, so latchup margins must be evaluated at T_max (L36844–36847; PDF 618).
- Source: §12.2.9.
- Philis stage: verify.
- Automation recipe: Use V_trig(T) linearly interpolated between the two quoted points in H12-38 (interpolation is an engineering choice; only the two points are given).
- Beats hand layout because: margins are evaluated at worst-case temperature on every pair.
- Philis status: missing.

### H12-41 Sublayer-aware backgate contact policy
- Kind: rule
- Statement: Over a heavily doped sublayer (NMOS over a P+ substrate, or PMOS in an N-well touching NBL), backgate current flows vertically. "Unless the transistor is very small, vertical conduction ... dominates", and a large far substrate-contact area beats a small adjacent one (L36914–36922). Without a sublayer (P-epi on P−, N-well without NBL, isolated tanks), contact every transistor (L36977–36981; PDF 619–621).
- Source: §12.2.9.
- Philis stage: deck, annotator, cells, verify.
- Automation recipe: The deck carries `substrate: p_plus_epi | bulk | soi` and `nwell_touches_nbl`. If there is a sublayer, relax per-device tap distance (H12-43 exempt) but require total substrate-contact area per region and pinning taps near injectors (H12-42). If there is none, enforce per-device taps and max distance.
- Beats hand layout because: the rule set follows the process instead of habit.
- Philis status: partial. Epi thickness is read for isolation, with a nominal default (`backend/annotator/src/emit.rs:248-256`). No substrate-type policy exists for taps.

### H12-42 Backgate pinning between injector and victim
- Kind: algorithm
- Statement: Add backgate contacts to the victim on the side facing the injector, across "all direct paths from the injector to the victim", and "as close to the victim as possible" (L36960–36964; Fig. 12.33; PDF 620).
- Source: §12.2.9.
- Philis stage: dp, cells (post_cell), verify.
- Automation recipe: For each (injector, victim) pair from H12-39 with complementary polarity or a shared substrate, compute the segment set between facing edges. Require a tap strip (victim-bulk net, victim type) that intersects every straight line between the two bboxes, placed ≤ d_pin from the victim edge (deck). This can be a partial ring on the facing sides. In post_cell, draw it as a partial ring. The cost is the uncovered fraction of the facing edge.
- Beats hand layout because: coverage of all paths is geometric and exact.
- Philis status: missing. Rings are full enclosures per device (`backend/annotator/src/constraints.rs:74-83`); there is no injector/victim pinning.

### H12-43 Maximum distance to the nearest backgate contact
- Kind: check
- Statement: The maximum allowed distance from any point in a transistor to the nearest backgate contact "usually falls somewhere between 25 and 250 µm", depending on backgate sheet R and expected current. It can be ignored if the backgate contacts NBL or the device cannot inject (always linear). When exceeded, add a second contact on the other side (L37002–37008; PDF 621). Exercise 12.16 uses 50 µm (L38155–38157; PDF 642).
- Source: §12.2.9, Exercise 12.16.
- Philis stage: deck, cells, dp, verify.
- Automation recipe: The deck provides `max_bg_contact_dist_um` (per well type). The generator computes max over the diffusion of the distance to its own tap. If that exceeds the limit, switch to a two-sided tap, interdigitated strips (H12-44) or distributed plugs (H12-44). Post-placement verify runs a distance transform over the final GDS: every diff point must lie within the limit of a same-bulk tap. Exempt devices flagged linear-only by the op-point or on NBL.
- Beats hand layout because: exact distance-field verification over the final GDS.
- Philis status: missing. A tap strip exists on one side only, with no distance check (`kernel/cells/src/mosfet.rs:567-572`; `docs/LAYOUT-FUNDAMENTALS.md:69`, `:137`).

### H12-44 Interdigitated vs distributed backgate contacts
- Kind: algorithm
- Statement: Interdigitated backgate strips threaded through the transistor reduce distance but "substantially increase the size". Distributed backgate contacts, small plugs in holes inside source fingers, slightly raise source R but "greatly reduce the area". They can go on every source finger or on a few at regular intervals. Plugs must stay connected after outdiffusion and misalignment (L37011–37021; Fig. 12.35; PDF 621–622).
- Source: §12.2.9.
- Philis stage: cells, deck.
- Automation recipe: Two generator variants for large devices: (a) `bg_interdigit every k fingers`; (b) `bg_distributed in source fingers every k`, legal only when bulk = source net and the deck has `tap_plug_min` (size and enclosure). Choose k as the smallest value meeting H12-43. Resize W to compensate the source-R increase if the op-point shows source-degeneration sensitivity.
- Beats hand layout because: k is chosen optimally for the distance rule.
- Philis status: missing.

### H12-45 Frozen reference layouts for NVM (and qualified cells)
- Kind: rule
- Statement: EEPROM performance "depends upon subtle aspects of its layout". The core devices "must exactly match those subjected to reliability testing" (L37614–37617; PDF 632).
- Source: §12.3.3.
- Philis stage: cells, annotator, flow.
- Automation recipe: The annotator recognises NVM cells (floating-gate nets: gates with no DC path) and hard-IP subcircuits by name. cells imports them as fixed macros (no regeneration, no stretch, no rotation beyond the allowed set). The router treats their interior as blocked.
- Beats hand layout because: it prevents accidental modification.
- Philis status: missing (no NVM recognition; fixed macros may exist via `macros` in `library::run`, not assessed).

### H12-46 No metal over EPROM/floating-gate devices
- Kind: rule
- Statement: "Layout designers should generally avoid placing metal above EPROM transistors". Metal adds capacitance that alters retention and blocks UV erase (used after wafer probe) (L37492–37502; PDF 629).
- Source: §12.3.2.
- Philis stage: gr, dr.
- Automation recipe: Mark floating-gate device bboxes as all-metal keepout zones for routing and fill.
- Beats hand layout because: there are no inadvertent over-routes.
- Philis status: missing. The verify `floating_gate` ERC exists (`backend/verify/src/checker.rs:145-177`) and would also need an NVM exemption.

### H12-47 NVM programming-rail current budget and EPROM redundancy
- Kind: rule
- Statement: About 100 µA per single-poly EPROM cell during programming, so 128 cells draw more than 10 mA (L37461–37464). Redundant EPROM pairs: two floating-gate transistors in parallel whose "gate electrodes ... must not connect" (L37484–37489; PDF 629).
- Source: §12.3.2, Fig. 12.39.
- Philis stage: gr/dr (EM), verify.
- Automation recipe: The VPP net gets an EM current budget of N_simultaneous × 100 µA, or the deck or designer value. LVS/ERC: redundant floating gates must be separate nets, so flag any short.
- Beats hand layout because: EM sizing is derived from the programming mode automatically.
- Philis status: missing (EM rule exists, `kernel/analog/src/routing/em.rs`, but there is no mode-based current).

### H12-48 Single-poly EEPROM capacitor ratio generator
- Kind: formula
- Statement: Tunnel capacitor (PMOS, own N-well, minimum size), control capacitor (PMOS, own N-well, enlarged) and sense NMOS share one floating gate. C_C/C_T is "typically ... at least 20" (L37518–37524; PDF 630). Exercise 12.18 uses 20× gate area (L38158–38163).
- Source: §12.3.3, Fig. 12.40.
- Philis stage: cells.
- Automation recipe: Size the control gate area A_C ≥ 20·A_T (ignoring fringing, as in the exercise), with separate N-wells for the tunnel and control capacitors. Treat it as a fixed macro after first qualification (H12-45).
- Beats hand layout because: the ratio is guaranteed by construction.
- Philis status: missing.

### H12-49 JFET device kind and generators (N-well JFET, annular JFET)
- Kind: data-model
- Statement: N-well JFET: N-well channel with a PMoat pinch plate that "must extend out into the isolation a much greater distance" than an epi-FET's. A minimum-width channel lowers V_P and may need widening (L37864–37894). Ion-implanted JFET gate contacts sit inside an emitter diffusion, not the shallow N implant, because of contact spiking (L37975–37976). Multi-finger JFETs resemble multifinger MOSFETs (L37996–37998). An annular JFET separates the top gate from the back gate, so the input connects to the top gate for lower C and leakage, sized with Eqs. 12.24–12.31 (L38001–38007; PDF 636–639).
- Source: §12.4.2, Figs. 12.44–12.47.
- Philis stage: annotator, cells, verify.
- Automation recipe: Add `DeviceKind::Jfet{n,p}` with terminals D, G, S and optional back gate. The generator offers a finger variant and an annular variant, and the pinch-plate extension comes from the deck. Recognise JFET input pairs as matched (common-centroid, H12-50).
- Beats hand layout because: the same matching machinery as MOS applies to JFETs.
- Philis status: missing (`kernel/core/src/netlist.rs:7-17` has no JFET kind).

### H12-50 JFET pairs need centroid layout (die-level pinch-off gradients)
- Kind: rule
- Statement: Double-diffused PJFET pinch-off "varying by tens of millivolts from one side of the die to the other". Common-centroid "could only do so much" (L37938–37943; PDF 637).
- Source: §12.4.2 Double-Diffused JFET.
- Philis stage: annotator, gp.
- Automation recipe: JFET differential pairs get the highest matching tier (common-centroid plus compactness). The gradient magnitude "tens of mV per die" can be entered as a deck gradient for the σ model.
- Beats hand layout because: the gradient is modeled quantitatively.
- Philis status: missing.

### H12-51 Epi-FET / pinched resistor model and contacts
- Kind: formula
- Statement: R = f(R0, V1, V2, empirical depletion factor) (Eq. 12.42). R0 = R_s,eff·(L_d + ΔL + corner term)/(W_d + ΔW) with N_90 corners (Eq. 12.43, exact form garbled in text; see PDF 635). Epi-FETs are minimum width, often serpentined. Contacts are placed over the pinch plate tied to substrate to limit debiasing, and rounded bends give "little or no benefit" (L37807–37860; PDF 635–636).
- Source: §12.4.2 Epi-FET.
- Philis stage: cells.
- Automation recipe: If Philis gains bipolar-process support, the pinched-resistor generator uses the corner-count R0 formula like serpentine resistors and adds pinch-plate substrate contacts. Otherwise this is low priority.
- Beats hand layout because: the resistance is computed exactly, corners included.
- Philis status: missing.

## 4. Top-15 priorities for Philis

1. **H12-43 + H12-41: Max backgate-contact distance check (25–250 µm deck value) with sublayer exemption.** Philis has no latch-up distance check today, and it is a signoff blocker for real tapeouts.
2. **H12-38 + H12-40 + H12-39: Quantitative latchup margin (Eqs. 12.32/12.33) with injector detection.** This replaces blanket per-FET guard rings (`constraints.rs:64-84`) with rings only where injection exists. That saves area and adds a hard safety metric no hand layout computes.
3. **H12-28: Common unit finger width for ratioed matched devices.** Ratioed mirrors and DACs currently fall into different unitization classes. A gcd-based unit restores W-bias cancellation (H12-02).
4. **H12-30: Butting backgate contact variant (+ H12-12 implant clipping).** This is the standard compact tap for bulk = source devices and gives better pinning than a side strip.
5. **H12-27 + H12-07: Refold enumeration with ESD lock and narrow-channel floor.** The generator can then search aspect and parasitics instead of freezing the schematic nf.
6. **H12-26: Directional-halo orientation constraint (no 90° turns) via a deck flag.** It is cheap to add (dp move-set filter), and in those processes an accidental quarter turn turns an analog transistor into a digital one.
7. **H12-42: Backgate pinning taps between injector and victim.** This is the analog-CMOS substitute for minority guard rings, placed geometrically on every facing side.
8. **H12-31: Cross-group diffusion sharing with notched moat.** It gives area and junction-C reduction on non-matched devices (Euler-style merge search).
9. **H12-29: Minimize-drain/source selection by node impedance.** It lowers C on high-Z nodes and costs one variant bit.
10. **H12-16 + H12-17 + H12-14: Isolated-NMOS tanks, grouped by injection pin and noisy/sensitive role.** This unlocks deep-N-well NMOS with correct tank partitioning.
11. **H12-44: Distributed or interdigitated backgate contact variants for large and power devices (+ H12-10).** They are needed to satisfy H12-43 on wide devices and to protect against BV_DII/snapback.
12. **H12-19 + H12-20: Thick-field voltage check per conductor (30 % derate) and poly-stub rule.** It catches parasitic channels on HV routes that DRC decks do not check.
13. **H12-21 + H12-23: Vt-flavour and thick-oxide marker drawing with same-flavour clustering.** Without them, devices using lvtn/hvtp or thick oxide are not drawable at all.
14. **H12-36/H12-37 + H12-35: Annular and serpentine MOS generators with closed-form W/L.** Annular gives minimum C_D/W for fast nodes and suits HV devices; serpentine gives compact long-L trickle sources.
15. **H12-08 + H12-04: Op-point region tagging (weak inversion, V_SB ≠ 0).** Weak-inversion pairs and body-biased pairs need a stricter matching tier and thermal budget, and this ties into the H12-03 mobility TC.
