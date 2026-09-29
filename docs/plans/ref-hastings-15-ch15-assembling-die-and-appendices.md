# Hastings 3e, Chapter 15 (Assembling the Die) and Appendices A–E: study for Philis

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson, 2023), Chapter 15 "Assembling the Die", Appendix A (acronyms), Appendix B (Miller indices), Appendix C (sample layout rules), Appendix D (mathematical derivations), Appendix E (rule of one-third).
Reftext file: `/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/hastings.txt`, lines **46303–52119** (Chapter 15 heading through the last line before the Index). PDF page numbers below are PDF page indices (`1 + count of \f` before the line); the printed folio is PDF page − 1.

The text extraction dropped every equation, most numeric values set in math, and every table cell set in math. All such values below were read from the rendered PDF pages listed in §1. A value that is neither in the text nor legible in the render is marked "not given".

---

## 1. Coverage

### 1.1 Read chunks (Read tool, consecutive, no gaps)

| Chunk | offset..end (reftext lines) |
|---|---|
| 1 | 46303..47402 |
| 2 | 47403..48502 |
| 3 | 48503..49602 |
| 4 | 49603..50702 |
| 5 | 50703..51802 |
| 6 | 51803..52122 (range ends at 52119, "Index") |

(A first attempt at a 2000-line chunk exceeded the tool's token limit and returned nothing; chunks were then cut to 1100 lines.)

Rendered PDF pages consulted for equations, tables and numbers lost in extraction: 782–793, 798–806, 812–824, 837–844, 846–876. Not rendered: 794–797 (floorplan figures, text fully extracted), 807–811 (bump/pillar process; numeric bump sizes therefore "not given" here), 825–835 (checklist tail, exercises, acronyms, Miller indices; text fully extracted).

### 1.2 Every heading in the range

- Chapter 15 Assembling the Die (46303)
- 15.1 Die Area Estimation (46338)
  - 15.1.1 Partitioning and Populating the Database (46349); Partitioning (46407); Populating a Design (46456)
  - 15.1.2 Computing Cell Areas (46530); Resistors (46558); Capacitors (46580); Bipolar Transistors (46606); MOS Transistors (46621); Power Transistors (46636); Table 15.1 (46689)
  - 15.1.3 Estimating Die Areas (46739)
  - 15.1.4 Cost Estimation (46896)
- 15.2 Floorplanning (46976); Table 15.2 (46989); Figs 15.4–15.8
- 15.3 Constructing the Padring (47267)
  - 15.3.1 Scribe Streets (47279)
  - 15.3.2 Scribe Seals and Ground Rings (47373)
  - 15.3.3 Bondpads (47506)
  - 15.3.4 Solder Bumps and Copper Pillars (47797)
- 15.4 Manual Top-Level Interconnection (48039)
  - 15.4.1 Vias (48068); Table 15.3 (48122)
  - 15.4.2 Routing Pitches and Grids (48163); Table 15.4 (48243)
  - 15.4.3 Channel Routing (48351); Table 15.5 (48452)
  - 15.4.4 Special Routing Techniques (48502): Star Nodes and Kelvin Connections (48509); Noisy Signals and Sensitive Signals (48568); High-Voltage Signals (48691); High-Current Signals (48738)
- 15.5 Final Layout Checklist (48849), items 1–17
- 15.6 Conclusion (49003)
- 15.7 Exercises (49015), 15.1–15.12
- Appendix A Table of Acronyms (49094)
- Appendix B The Miller Indices of a Cubic Crystal (49561)
- Appendix C Sample Layout Rules (49656); C.1 Standard Bipolar Rules (49667), Tables C.1–C.4; C.2 Polysilicon-Gate CMOS Rules (49933), Tables C.5–C.8
- Appendix D Mathematical Derivations (50448): Eq 5.7 (50456); Eq 5.9 (50504); Eq 5.32 (50583); Eq 7.6 (50680); Eq 8.27 (50757); Eq 8.29 (50812); Table 8.7 (50869); Eq 8.40 Supplementary Information (50910), Table 8.45; Eq 9.14 (51050); Eq 10.2 (51102); Eq 10.8 (51139); Eq 10.9 (51192); Table 10.3 (51250), Table 10.T1; Section 11.1.2 (51345); Eq 11.8 (51405); Eq 11.9 (51454); Eqs 12.26–12.27 (51492); Eq 13.24 (51563); Eq 13.38 (51608); Eq 13.39 (51661); Eq 14.6 (51707)
- Appendix E The Rule of One-Third (51753): One Singly Terminated Source/Drain Finger (51762); Two Source/Drain Fingers Terminated on the Same End (51924); Two Source/Drain Fingers Terminated on Opposite Ends (51994)

---

## 2. Section-by-section digest

### Chapter 15 introduction (46303–46336, PDF 781)
- Layout starts from a die-area computation: analog cells by quick autoplacement or by summing component areas; power MOS by on-resistance/metal-resistance computation or preliminary layout; logic by gate count (46305–46310).
- Die area = Σ subsections + routing + bondpads + scribe seals + scribe streets + a safety margin (46309–46310).
- A floorplan shows die outline with scribe and corner exclusion zones, ground ring, bondpads, ESD, guard rings, sized polygons for major blocks, and key metal (power, buses); it considers matched-device placement, noise coupling, metal resistance and inter-block signal flow (46313–46318).
- Build order: scribe seal + ground ring → ESD + bondpads + guard rings → power devices → analog cells → logic; then top-level routing, final verification (46322–46332).
- Designs with hundreds–thousands of top-level signals benefit from autorouting; a few dozen are "better hand-routed" (46329–46331).

### 15.1 Die Area Estimation (46338–46347, PDF 782)
- Estimates within ±20% are "routinely achievable" (46339–46340; value from PDF 782 render).
- Prerequisite: a partitioned, populated design database (46344–46345).

### 15.1.1 Partitioning and Populating the Database (46349–46528, PDF 782–784)
- Cell/instance/parent/child/leaf hierarchy; few designs need more than 10–12 levels (46350–46388).
- LVS assumes layout hierarchy equals schematic hierarchy but tolerates layout-only cells (power-device portions, ground rings) (46401–46403).
- Partition by simulation units; a schematic should fit a B-size sheet, ≈50–100 components per cell (46414–46421).
- Matched components that occupy an array must reside in the same cell (46429–46431).
- Cell names: lower-case letter first, then lower-case/digits/underscore, ≤32 characters (GDSII) (46435–46452); pin names similarly (46464–46474).
- Pin direction ERC: a net connected only to inputs is floating; a net connected to two outputs is shorted (46495–46506).
- Name suffixes carry semantics: `b`/`z` negative logic, `1p5`/`3p3` voltage domain (46510–46514).
- Early schematics should carry realistic values for area-dominant parts: large caps, high-value resistors, matched components, power devices (46521–46526).

### 15.1.2 Computing Cell Areas (46530–46737, PDF 784–787)
- Eq 15.1 `A_cell ≅ P_f(cell) · ΣA`; P_f: standard bipolar SLM 1.5–3.0, DLM bipolar 1.3–2.5, two-metal CMOS/BiCMOS 1.2–1.8; more metal layers add little for analog (46538–46554).
- Eq 15.2 resistor area `A ≅ 1.2·R·W_r·(W_r+S_r)/R_S`; example 500 kΩ of 200 Ω/□ poly, W = S = 2 µm → 24,000 µm² (46564–46577).
- Eq 15.3 capacitor `A ≅ 1.1·C/C_a`; Eq 15.4 `C_a = 0.885·ε_r/t` (pF/µm², t in Å); 200 Å oxide → 0.017 pF/µm²; 100 pF → 6400 µm² (46580–46603).
- Bipolar areas: lay out representative devices (min NPN, matched single/four-emitter NPN, min lateral PNP, min substrate PNP) (46606–46618).
- Eq 15.5 MOS `A ≅ 1.3·W_g·(L_g + S_gg)` (46621–46633).
- Eq 15.6 power MOS `A ≅ R_SP/(R_DS(on) − R_P)`; R_DS(on) varies ≈±25% over process and rises ≈50% from 25 °C to 125 °C, so worst case ≈2× nominal; large power devices are estimated standalone (near-perfect packing) (46636–46662).
- Table 15.1 worked bandgap example: 95,400 µm² components, P_f = 2 → 0.19 mm² (46665–46737; values from PDF 787).

### 15.1.3 Estimating Die Areas (46739–46893, PDF 787–790)
- Core-limited vs pad-limited dice; staggered inner pad ring possible after assembly review (46740–46748).
- Eq 15.7 `A_C ≅ R_f·P_f(die)·ΣA_cell + P_f(die)·ΣA_pwr`; P_f(die) 1.1–1.2 for 20–30 moderate cells, <1.05 for tight-pack (46764–46781).
- Channel vs maze routing; parasitic coupling as small as 1 fF can cause malfunction; parasitic extraction and back-annotation "extremely helpful" (46785–46790).
- Routing factors: SLM ≤1.2 for 50–100 components; 1.5-layer metal 1.2–1.4; DLM 1.1–1.3 (maze+channel ≈1.1; pure maze near unity if cells use little M2 and long M2 runs one direction); TLM 1.1–1.2; QLM maze routing easy if cells avoid M3/M4 (46793–46834).
- Obstacles to maze routing: dummy-metal block regions over matched devices, power devices using all metals, autorouted logic (46829–46831). Full shielding of a signal costs three routing layers (46831–46833).
- Eq 15.8 `A_die ≅ (√A_C + 2W_pr + W_s)²`; saw scribe 50–75 µm, laser 25 µm; bondpad width 2–3× wire diameter (25 µm wire → 50–75 µm); seal+ground ring 20–50 µm; guard rings ≥25 µm; padring 100–150 µm (46837–46851).
- Eq 15.9 `P_min ≅ (N_p + 4)(W_p + S_p) + 4W_s`; add 8× corner distance if a corner-to-pad rule exists; Eq 15.10 `P = P_min/(4√A_die)`; P > 1 = pad-limited; Eq 15.11 `A_die = P_min²/16` (46854–46892).

### 15.1.4 Cost Estimation (46896–46970, PDF 790–791)
- Eq 15.12 `N_d = η·π·d²/(4·A_die)`, η 0.85–0.95; 10 mm² on 200 mm, η = 0.95 → 2980 dice (46904–46919).
- Eq 15.13 `C_d = (C_m + C_p)/(N_d·Y_p)`, probe yield 0.8–0.95 (46927–46940).
- Eq 15.14 `C_f = (C_d + C_a)/Y_a`, Y_a > 0.95 (46944–46955); Eq 15.15 `GPM = (S − C_f)/S·100%`, feasibility ≈≥50% (46958–46970).

### 15.2 Floorplanning (46976–47262, PDF 792–797)
- Inputs: per-cell area, die area, pad list and order (Table 15.2 worksheet: 1.33 mm² die with P_f = 1, R_f = 1.2; 25 µm PCC wire; 75 µm pad; 100 µm padring; 75 µm scribe) (46983–47087; values PDF 792).
- Die edge rounded to mask-vendor increment; smallest leadframe that fits with 10–20% X/Y margin; oversized mountpads delaminate (47089–47103).
- Identical blocks as mirror-image placements of one cell for similar electrical characteristics; block placement follows pad adjacency; elongating the amplifier improves matching by separating sensitive input circuitry from output-stage heat (47136–47145).
- Routing reserve (20% of core) drawn as explicit strips (47160–47162).
- Pads start adjacent to lead fingers; bondwires must not intersect or closely approach; assembly verification tools (47166–47180); symmetric pad placement of the two amplifiers; power pads at top/bottom (47195–47198).
- Substrate ground ring = lowest potential (vss in dual-supply parts) (47201–47206).
- High-current leads: EM sets a lower bound on width, resistance often forces wider; mark each lead's DC current and max resistance on the floorplan (47210–47214); vcc in M2 over the bias cell with M1 crossings (47217–47220).
- Complex floorplans resemble jigsaw puzzles; careless block placement creates channel choke points (Fig 15.8) (47241–47261).

### 15.3 Constructing the Padring (47267–47276, PDF 798)
- Padring = scribe streets, scribe seal, ground ring, bondpads, ESD, guard rings; bump/pillar dice put ESD at the periphery or next to bumps (47268–47275).

### 15.3.1 Scribe Streets (47279–47370, PDF 798–799)
- Backgrind: 200 mm wafer ≈725 µm, 300 mm ≈775 µm, dice ≈250 µm, down to 50 µm (47280–47284).
- Saw blade 20–30 µm, kerf margins 15–25 µm; scribe <50 µm impossible, 75 µm reasonable; stealth-dicing damage zone ≈6 µm (47297–47312).
- Test structures live in scribe; composed reticle = main die (0,0)–(X_die,Y_die) arrayed into a scribe frame by a scribe PG deck (47321–47369).

### 15.3.2 Scribe Seals and Ground Rings (47373–47503, PDF 799–801)
- Seal: every metal layer joined by continuous contact/via strips, optional PO flap-down; blocks moisture, mobile ions, cracks (47374–47382).
- Seal over a substrate-contact diffusion becomes the ground ring; total metal resistance through ESD devices between any two bondpads must not exceed a value, "often two ohms" (47385–47391).
- Plastic-package stress: die edges high, corners highest (shear); larger dice worse; metal extrusion shorts and diagonal cracks (47394–47410).
- Corner exclusion zones: typical for plastic dice with longer side >2.5 mm; d_c ≥ 5% of the longer die dimension (5 mm die → 250 µm); nothing active, not even the ground ring; fabs differ on anti-delamination structures and seal routing (47413–47438).
- Anti-delamination slots: long axis orthogonal to the shear-stress gradient (parallel to die edge, or to the exclusion-zone diagonal); slot W_S × L_S, max gaps S_L and S_W, alternate columns offset, adjacent-layer slots offset; drawn on a slot layer, `FINAL_MET1 = MET1 − MET1SLOT` (47441–47489).
- Eq 15.16 `R_slot/R_unslot ≅ [W·L_S/((S_L+L_S)(W − N_S·W_S))] + S_L/(S_L+L_S)` (47491–47502).

### 15.3.3 Bondpads (47506–47793, PDF 801–806)
- History of wedge/ball bonding, purple plague, EFO (47507–47551).
- Fine pitch: 60 µm with 20 µm wire, 50 µm possible; wedge ≤40 µm; typical analog 25 µm wire at 125 µm pitch; Cu/PCC wire now dominant (47548–47559).
- Pad = POR square over top metal, metal overlapping POR (47567–47572).
- Typical rules: opening 3× wire diameter (75 µm), pitch 5× (125 µm); circular metal-exclusion zone of diameter ≈50 µm more than the opening; leads within 25 µm of the pad ≥10 µm wide; pad-1 marks (corner notches ≈10 µm deep, octagon, circle) (47587–47612).
- Under-pad structures: SLM forbade active circuitry; DLM M1/M2 sandwich pad; BOAC schemes (omit next-to-top metal, merged top metals, thick top metal, floating mesh) (47627–47684).
- BOAC guidelines: topmost metal for the pad; few topmost vias under pad; minimize next-to-top density and keep its lines thin; dense lower-level via arrays encouraged (47688–47702).
- Matched circuitry: avoid under thin-Al BOAC pads; under thick metal (power Cu), place matched devices well inside or well away from the thick-metal edge; never let a thick-metal edge cross matched devices (47704–47713).
- Eq 15.17 `R_w = 4ρL/(πD²)`; wires 12.5–50 µm, Al to 250 µm; 4N Au ρ = 2.2–2.4 µΩ·cm, 2N 3.0–3.3, Cu 1.7; 1 mm 25 µm 4N Au = 47 mΩ; TCR Au 3700 ppm/°C, Cu 4000 ppm/°C; +37% from 25 to 125 °C; 2–3 wires per package pin (47719–47740).
- Probe pads: no ESD needed; blade cards ≤88 probes at ≥100 µm pitch; epoxy-ring ≤2000 probes at ≥50 µm; POR ≥ max(2× tip, 0.75× pitch); 38 µm tip → 75 µm opening; 25/18 µm tips → 65 µm, possibly 50 µm; two offset ranks separated by ≥ min pitch (47748–47778).

### 15.3.4 Solder Bumps and Copper Pillars (47797–48033, PDF 806–810)
- C4 history; CSP, laminate/MCM, bump-on-leadframe (47798–47851).
- Lead-free solder carries ≈2% of Al's current density: solder joints are an EM concern (47855–47861).
- Bump-on-I/O, bump-on-PO (RDL), bump-on-polymer (BCB); stress limits array size; copper pillars finer pitch, oval pillars for high current; Cu ≈5× the thermal conductivity of solder, so pillars can be placed to draw heat from power devices (47864–48014). Numeric bump dimensions: not given here (pages not rendered).

### 15.4 Manual Top-Level Interconnection (48039–48064, PDF 811)
- Small analog tops still hand-routed faster than autorouted (48040–48043); SLM needs hand-crafted devices and tunnels; DLM cheaper above ≈50 components (48046–48053).
- Maze routing saves 5–10% area but is slower, harder to modify, and more prone to unexpected coupling; channel routing emphasized (48061–48064).

### 15.4.1 Vias (48068–48160, PDF 811–812)
- Al vias: fixed size, arrays of min vias for high current, not stackable, symmetric overlap of both metals (48069–48096).
- W-plug vias stackable, top-metal overlap optional/asymmetric; single/dual damascene: both overlaps optional, stackable (Table 15.3) (48100–48146).
- Stress voiding is probabilistic per via; fabs demand via pairs; adjacent vias protect each other beyond redundancy (48148–48153).

### 15.4.2 Routing Pitches and Grids (48163–48347, PDF 812–815)
- Eq 15.18 `P_LTL = W_m + S_m`; Eq 15.19 `P_VTV = W_v + 2E_mv + S_m` (E_mv = smallest opposite-side overlap when asymmetric); Eq 15.20 `P_VTL = (W_v+W_m)/2 + E_mv + S_m` (staggered vias congest channel intersections, so via-to-via preferred); Eq 15.21 `W_C = N·P_R + S_m` (48175–48233).
- Table 15.4: M1/M2 width 0.3, space 0.3, VIA1 0.25 exact, overlaps 0.05 (two sides)/0.1 → P_LTL 0.6 µm, P_VTV 0.65 µm (48238–48281).
- Grid consistency: a 0.35 µm path puts edges on a 0.025 µm grid; with a 0.05 µm coding increment the width must go to 0.4 µm and P_VTV to 0.7 µm (48285–48290).
- Multi-layer channels with different rules: use the larger rule on all layers if the difference is small (48293–48298).
- Routing grid = integer multiple of coding increment (0.1 µm here); consistent vs inconsistent (centerline-only, 0.7 µm) grids (48302–48313).
- Octagonal (45°) routing packs tighter along a corner-exclusion diagonal; off-grid vertices; some rules require wider diagonal segments; then compute the pitch from the diagonal width (48317–48333).

### 15.4.3 Channel Routing (48351–48499, PDF 815–817)
- Poly jumpers: unsilicided 20–50 Ω/□, silicided <5 Ω/□; static digital signals tolerate poly (48352–48360).
- Die-wide alternating preferred directions per metal layer (48363–48366).
- Route wide leads first; widths should consume integer multiples of the routing pitch, Eq 15.22 `W_N = N·P_R + S_m` (P_R 0.7, S_m 0.3 → 1.0, 1.7, …, 0.7N + 0.3 µm); start at channel edges, keep min spacing, no jogs (jogs propagate congestion) (48387–48409).
- At wide-lead layer changes fill the full overlap with as many vias as possible; many designers do this at branches too (48411–48414).
- Overload a full channel via non-preferred metal or poly; route metal-only nets first, poly-tolerant last (48429–48441).
- Table 15.5 signal classes (HV, HC, HV+HC, sensitive analog, noisy digital, static digital poly-OK, other metal-only); air wires hurt (48445–48488).
- Primary channels near die centre hold 10–20% of top-level signals each; taper toward edges ("watercourses"); feeders ≥5 signals; widths cannot grow once routing starts (48490–48498).

### 15.4.4 Special Routing Techniques (48502–48843, PDF 817–822)
- Star/Kelvin: 10 squares of 30 mΩ/□ (0.3 Ω) × 1 mA = 0.3 mV between two matched emitters' ground taps; star node C makes ground drops common-mode (48509–48538). Kelvin force/sense; match sense-lead currents and resistances so residual drops cancel (48540–48549).
- Noise: capacitive interference; Eq 15.23 `ΔV = C·R·dV/dt`; <0.1 ns edges ⇒ >1 GV/s; 10 fF into 100 kΩ → 1 V (48574–48595).
- Noisy = fast slew, or switching when others cannot tolerate it (48599–48601); eight sensitive categories (48604–48616); suffix `_n` / `_s` (48618–48621).
- No noisy-over-sensitive; cross at right angles; evaluate with Eq 15.23 or back-annotation; selective back-annotation of only noisy→critical-node C, rest lumped to ground (48625–48635).
- Shield plate on intermediate layer tied to the sensitive node's own reference, extended 2–3 µm past the intersection; in DLM route one signal in poly (48638–48667).
- Series resistors (few tens of kΩ) on static digital control lines: 100 kΩ with 1 pF → ≈10 MV/s (48670–48679).
- No adjacent-layer or same-layer adjacency between noisy and sensitive; lateral C can equal vertical C; interpose a quiet line (static digital, supply, ground) (48683–48687).
- High voltage: ILO ≈2 MV/cm = 200 V/µm nominal, but a drawn 0.25 µm space may hold only ≈25 V (microloading, OPC, damascene taper); low-k weaker; voltage-dependent spacing rules exist but DRC needs ΔV between the two leads, not absolute voltage; `_hv` flagging; Calibre voltage-aware DRC works best for digital (48691–48729).
- High current: EM must hold at every point (no neck-down); resistance averages along the lead (widen where room exists) (48739–48744).
- J_max (MIL-M-38510) = 5×10⁵ A/cm² under glass; Eq 15.24 `W_min = I_max/(J_max·t)`: 50 mA, 1 µm metal → 10 µm (48748–48772).
- Eq 15.25 `D = exp[(E_a/(n·k))(1/T_j − 1/T_0)]`, k = 8.62×10⁻⁵ eV/K; E_a 0.5 eV pure Al, ≈0.7 eV Cu-doped Al, n ≈2 for Al; 105→125 °C gives D = 0.58 (25 mA → 15 mA) (48776–48791). Cu: higher J_max at 85–105 °C, n ≈1, parity with Al ≈175 °C (48804–48809).
- 90° bends crowd current at the inside corner: replace by two 135° angles; via arrays at a 90° corner load the inside-corner via most, so put arrays in straight segments (48818–48824). Dynamic EM tools with Blech-length rules exist (48839–48843).

### 15.5 Final Layout Checklist (48849–48998, PDF 823–825)
- 1 DRC diagnostics reviewed/waived with process engineering; 2 zero LVS topological errors (never waivable); 3 LVS parametric errors reviewed (e.g., guard-ring diode areas at hierarchy boundaries); 4 power-wire EM compliance via power plots for pinch points and too-few vias; 5 resistance of crucial high-current leads computed (rule of one-third or FEA); 6 zero antenna violations (jumpers/diodes); 7 ESD: resistance between ESD devices, CDM clamps close to every pin-connected gate oxide and at domain crossings with wide direct ground/power; 8 latchup: any diffusion connected to a vulnerable pin directly or through < ≈50 kΩ needs minority-carrier guard rings (substrate pins exempt; positive-supply pins see no below-ground transients, negative-supply pins no above-ground); 9 dummy-metal block over matched devices; 10 noisy vs sensitive separation or shielding; 11 bondpads/probe pads sized/located, keep-away, lead juncture width, pad-1 mark; 12 Kelvin/star physically correct, including "Kelvin at the bondpad"; 13 corner exclusion and slotting; 14 scribe seal joined to ground metal and scribe width; 15 on P+ substrate dice fill empty area with substrate contacts vs bypass caps; 16 symbolization, mask-work notice; 17 archival (layout DB, libraries, DRC/LVS/PG decks, GDSII, layer maps, checksums, two copies) (48856–48997; 50 kΩ from PDF 824).

### 15.6 Conclusion (49003–49009, PDF 826)
- Die assembly is "largely art and not science" requiring judgement (49004–49009). An automated tool must replace this judgement with explicit metrics (the gap Philis targets).

### 15.7 Exercises (49015–49089, PDF 827–828)
- Area estimates (15.1–15.3), die area/pad counts incl. 1.5:1 aspect (15.4–15.5), cost/GPM (15.6–15.7), pinout/floorplan/high-current routing (15.8–15.10), channel width for 12 leads with App. C rules (15.11), EM width for 360 mA 50% duty at 0.7 eV (15.12). Exercise 15.12 implies duty-cycle-averaged DC current for EM (49080–49088).

### Appendix A Table of Acronyms (49094–49556, PDF 829–833)
- Glossary (0TC … WLP). Relevant vocabulary: BOAC, CDM, CTE, EBGR/ECGR/HBGR/HCGR guard-ring types, LoD, WPE, PPID, POR, RDL, UBM, TDDB (49095–49555).

### Appendix B Miller Indices (49561–49651, PDF 834–835)
- Reciprocal-intercept rule, equivalent planes {…} and directions ⟨…⟩; "there is no such thing as a {100} wafer" (49593–49650). Needed to interpret the (100)/(111) piezo coefficients in App. D.

### Appendix C Sample Layout Rules (49656–50443, PDF 836–844)
- Intro: 1980s/1990s-representative, not a real process (49657–49661).
- C.1 standard bipolar, 30 V; Table C.1 electricals (beta 100/200/300; V_EBO 6.4–7.2 V; V_CBO ≥40 V; V_CEO ≥30 V; base 130/160/190 Ω/□; emitter 5/7/10 Ω/□; pinched base 1.5/3/4.5 kΩ/□; HSR 1.6/2/2.4 kΩ/□; metal 25/30/35 mΩ/□; thick-field ≥35 V); 8 coding layers; BOI auto-generated; 1 µm coding increment; Tables C.2–C.4 rules 1–44 (49667–49930).
- C.2 10 V N-well poly-gate CMOS; Table C.5 (NMOS V_t 0.5/0.7/0.9 V, k 50/70/90 µA/V²; PMOS V_t −0.9/−0.7/−0.5 V, k 17/25/33; poly-1 20/30/40 Ω/□; poly-2 450/600/750 (40/50/60 PSD-doped); NSD 24/30/36; PSD 40/50/60; N-well 1.4/2/2.6 kΩ/□; base 400/500/600; M1/M2 32/40/48 mΩ/□; C_ox 0.85/0.95/1.05 fF/µm²; poly-poly 1.3/1.5/1.7 fF/µm²; NPN beta 40/80/120; V_EBO 7/8/9 V); 11 masks; derived layers (NMOAT→MOAT+NSD, PMOAT→MOAT+PSD, CHST from NWELL+MOAT); 0.25 µm increment; Table C.6 rules 1–41 with conditional notes; Table C.7 poly-2 rules 42–52; Table C.8 BiCMOS rules 53–76 (49933–50443; numbers PDF 837–844). Full rule-class enumeration in H15-44.

### Appendix D Mathematical Derivations (50448–51750, PDF 845–869)
- Eq 5.7 resistor self-heating `ΔT = I²·t_ox·ρ/(κ·W²)` (from `ΔT = P·t/(κA)`, `R = ρL/(t_R W)`) (50456–50501).
- Eq 5.9 adiabatic pulse width `W_min = (I_max/t_R)·√(ρτ/(d·c_H·ΔT))` (50504–50581).
- Eq 5.32 high-low junction recombination: equations illegible in both text and render (PDF 847–848 blank); only the variable definitions are given (50583–50677).
- Eq 7.6 fringing: `C_F = 2εR·ln(2eπR/d)` (disc over plane); `C_F/P ≈ (ε/π)·ln(eP/d)`; parallel discs `C_F ≈ (εP/π)·ln(2eP/t)`; strip-thickness correction `C_C/L ≈ (ε/π)·ln(1+2t_e/t+2√(t_e/t + t_e²/t²))` → `(ε/π)·ln(1+4t_e/t)` for t_e ≫ t; final `C_F ≈ (εP/π)[ln(2eP/t) + ½·ln(1+4t_e/t)]` (50680–50754).
- Eq 8.27 segmentation sensitivity `S = |N/R_N − M/R_M|` from `R = αR_0 + β` (50757–50809).
- Eq 8.29 with partial segments `S = |(N+1)/(N+k) − (M+1)/(M+j)|`, 0 < j,k < 1 (50812–50866).
- Table 8.7 poly piezoresistance from gauge factors; E_poly ≈169 GPa, ν ≈0.22; P-type G_L ≈42 (G_T not legible), N-type G_T ≈15 (G_L not legible); formula operators and resulting π values not legible (50869–50906).
- Eq 8.40 piezocapacitance: isotropic constitutive relations, thin-film dielectric strain, dielectrostriction `Δε = 2M₁₂(σ_xd+σ_yd)`, `ΔC = C(ε_xd+ε_yd−ε_zd+Δε/(ε_rε_0))`, `ξ = ((1−ν_Si)/E_Si)[1 + (1/(1−ν_d))(2M₁₂E_d/(ε_rε_0) + ν_d)]`; Table 8.45 E/ν; M₁₂(oxide) = −2.1×10⁻²² m²/V²; ξ = 4.6×10⁻¹³ and 5.8×10⁻¹³ Pa⁻¹ (both printed "(111)") (50910–51046).
- Eq 9.14 `V_CE(sat) = V_T·ln[(β_F/β_R)·(β_R+β_force+1)/(β_F−β_force)]` (51050–51099).
- Eq 10.2 emitter debiasing `ΔV_BE = L·R_S·I_E/(2W)` (51102–51136).
- Eq 10.8 `s_IC2/IC1 ≅ k_A/√A_E`; Eq 10.9 `s_ΔVBE = k_A·V_T/√A_E`, with `s_A = k_A√(A_E/2)`, `ΔV_BE = V_T ln(A_E1/A_E2)` (51139–51247).
- Table 10.3 piezojunction: `ΔI_S/I_S = −(B₁σ'₁₁+B₂σ'₂₂)cos²φ − (B₂σ'₁₁+B₁σ'₂₂)sin²φ`; circular lateral on (100): `−((ζ₁₁+ζ₁₂)/2)(σ'₁₁+σ'₂₂)`; on (111): `−((2ζ₁₁+4ζ₁₂+ζ₄₄)/6)(σ'₁₁+σ'₂₂)`; Table 10.T1 (10⁻¹¹ Pa⁻¹): PNP ζ₁₁ 8.9, ζ₁₂ 14.3, ζ₄₄ 103.5; NPN −28.4, 43.4, 13.1 (51250–51341).
- §11.1.2 thermalization distance `x_t ≤ (3µ/q)√(3kT·m_eff)`: ≤140 nm electrons, ≤60 nm holes (51345–51402).
- Eq 11.8 disc with uniform top injection, peripheral extraction `R = R_S/(4π)`; Eq 11.9 annulus `V = J·R_S·r₁·ln(r₂/r₁)` (51405–51489).
- Eqs 12.26–12.27 annular MOSFET `L = (B−A)/2`, `W/L = 2π/ln(B/A)`, `W = π(B−A)/ln(B/A)` (51492–51559).
- Eq 13.24 `TC_ID = k(V_GS−V_t)[TC_k(V_GS−V_t)/2 − TC_Vt]`; TC_ID < 0 if `TC_Vt < TC_k·(V_GS−V_t)/(2V_t)` (as printed) (51563–51605).
- Eq 13.38 `ΔV_GS = ΔV_t − V_gst1(√(1+Δk/k₂) − 1)` and its first-order form; Eq 13.39 `I_D2/I_D1 ≅ (k₂/k₁)(1 + 2ΔV_t/V_gst1)` for ΔV_t ≪ V_gst1 (51608–51704).
- Eq 14.6 HBM conductor cross-section `A = √(ρτI_pk²/(2C_VΔT))`, τ = 1.5 kΩ × 150 pF = 225 ns (51707–51750).

### Appendix E The Rule of One-Third (51753–52118, PDF 870–876)
- Singly terminated finger: distributed RG line, `R = R_S/W_M`, `G = 1/(W·R_Si)`, `γ = √(RG)`, `R_DS(on) = (γ/G)·coth(γW)`; two Taylor terms → `R_M ≅ R_S·W/(3W_M)` (one-third of end-to-end); penetration distance `d_pen = R_Si·W_M/R_S`; error <1.5% for W ≤ d_pen, <10% for W ≤ 1.8·d_pen (51762–51921).
- Two fingers terminated on the same end: `γ = √(2RG)`, `R_M ≅ 2R_S·W/(3W_M)`, `d_pen = R_Si·W_M/(2R_S)` (51924–51991).
- Two fingers on opposite ends: `R_DS(on) = (γ/2G)[csch(γW)+coth(γW)] + RW/2`, `R_M ≅ 2R_S·W/(3W_M)`, `d_pen = R_Si·W_M/(2R_S)`; error 1.3% at W = d_pen, 7.7% at 1.8·d_pen (51994–52113).

---

## 3. Actionable extraction

Philis status checks are quick greps, not an audit. "Deck" status refers to the GPurify decks Philis loads (`~/.cargo/git/checkouts/gpurify-*/8df8c09/pdks/*.deck`, referenced from `pdks/*.json`).

### H15-01 Block area estimate and packing factor as a placement target
- Kind: formula / metric
- Statement: `A_cell ≅ P_f·ΣA_device`. P_f ranges: two-metal CMOS/BiCMOS 1.2–1.8; DLM standard bipolar 1.3–2.5; SLM bipolar 1.5–3.0; more metal layers add little for analog. Tight-pack die-level P_f < 1.05; 20–30 loosely fitted cells 1.1–1.2.
- Source: §15.1.2 Eq 15.1; §15.1.3 Eq 15.7 text; reftext 46538–46554, 46773–46781; PDF 784, 788.
- Philis stage: flow, gp, verify (report).
- Automation recipe: after `cells` picks variants, compute `ΣA` from generated footprints; report achieved `P_f = bbox_area/ΣA` per block and per epoch; seed gp's initial outline at `P_f ≈ 1.2–1.3` (two-metal CMOS lower end) and let the utilization budget tighten it. Cost form: budget term on P_f (already Θ-ranked via Utilization).
- Beats hand layout because: every epoch reports P_f exactly, so area is traded against Θ and PEX quantitatively, not by the designer's "tight vs hasty" judgement.
- Philis status: partial — `Utilization` floor `footprint ≤ Σ cell area / u_min` (kernel/analog/src/placement/utilization.rs:6-13); gp bin-overflow target (backend/gp/src/lib.rs:167, 316-320). No P_f report against Hastings' ranges.

### H15-02 Component-level area estimators for pre-generation budgeting
- Kind: formula
- Statement: resistor `A ≅ 1.2·R·W_r(W_r+S_r)/R_S` (1.2 covers dummies, heads); capacitor `A ≅ 1.1·C/C_a`, `C_a[pF/µm²] = 0.885·ε_r/t[Å]` (200 Å SiO₂ → 0.017 pF/µm²); MOS `A ≅ 1.3·W_g(L_g+S_gg)`; power MOS `A ≅ R_SP/(R_DS(on) − R_P)` with R_DS(on) worst case ≈2× nominal (±25% process, +50% 25→125 °C). Bipolar: tabulate laid-out reference devices.
- Source: §15.1.2 Eqs 15.2–15.6; reftext 46564–46662; PDF 784–786.
- Philis stage: annotator, cells.
- Automation recipe: in `cells`, before generating all variants, estimate each device's area with these formulas using deck sheet R / cap density (`cell.cap_density_ff_um2`, sheet values in `pdks/*.json`); prune variants whose generated area exceeds the estimate by > 2× (flags a bad generator choice); power MOS sized from an R_DS(on) spec at worst-case temperature (2× nominal).
- Beats hand layout because: consistent estimates for every device and every variant; the worst-case R_DS(on) factor is applied mechanically.
- Philis status: missing as estimators (grep "packing|area estimate" finds none); cells generate real geometry directly (kernel/cells/src/*.rs).

### H15-03 Routing factor by metal count and routing style
- Kind: data-model / metric
- Statement: `A_C ≅ R_f·P_f(die)·ΣA_cell + P_f(die)·ΣA_pwr`. R_f: SLM ≤1.2 (50–100 components); 1.5-layer 1.2–1.4; DLM channel 1.1–1.3, maze+channel ≈1.1, pure maze ≈1.0 (requires cells with minimal M2 and one-direction long M2); TLM channel 1.1–1.2, maze near 1.0 (cells with little/no M3); QLM maze easy if cells avoid M3/M4. Maze saves 5–10% area but raises coupling risk.
- Source: §15.1.3 Eq 15.7; §15.4; reftext 46764–46834, 48061–48064; PDF 788–789, 811.
- Philis stage: gp, gr, flow.
- Automation recipe: record `R_f = (routed block area)/(placed-only area)` per epoch; set gp whitespace reservation from the routing layer count available above the cells' own metal (cells in Philis use li/M1; routing on higher layers ⇒ target R_f ≈1.0–1.1); if gr overflows, grow whitespace toward R_f 1.2 before failing.
- Beats hand layout because: R_f becomes a measured, optimized quantity per design instead of a rule-of-thumb reserve.
- Philis status: missing (no routing-factor metric; gr capacity fixed at 6 per gcell, backend/gr/src/lib.rs:31-33).

### H15-04 Die, padring and cost model (chip-level only)
- Kind: formula
- Statement: `A_die ≅ (√A_C + 2W_pr + W_s)²` (W_s: saw 50–75 µm, laser 25 µm; padring 100–150 µm typical); `P_min ≅ (N_p+4)(W_p+S_p) + 4W_s` (+8× corner-to-pad distance if ruled); `P = P_min/(4√A_die)`; P > 1 ⇒ pad-limited and `A_die = P_min²/16`. Cost: `N_d = ηπd²/(4A_die)` (η 0.85–0.95), `C_d = (C_m+C_p)/(N_dY_p)`, `C_f = (C_d+C_a)/Y_a`, `GPM = (S−C_f)/S`.
- Source: §15.1.3 Eqs 15.8–15.11; §15.1.4 Eqs 15.12–15.15; reftext 46837–46970; PDF 789–791.
- Philis stage: flow (top-level only).
- Automation recipe: if Philis ever assembles a full die, compute core-vs-pad limitation up front and pick the smaller of the two die sizes; for blocks, skip.
- Beats hand layout because: closed-form; no advantage beyond speed.
- Philis status: missing; out of scope for block-level P&R (no pad/padring code: grep "padring|bondpad" empty).

### H15-05 Hierarchy constraints: matched arrays in one cell, cell size, layout-only cells
- Kind: rule / data-model
- Statement: matched components occupying an array must reside in the same cell; ≈50–100 components per cell; LVS tolerates layout-only cells (ground rings, power-device parts).
- Source: §15.1.1; reftext 46401–46431; PDF 782–783.
- Philis stage: annotator (block extraction), cells.
- Automation recipe: annotator never splits a matched group across `block::leaves`; any matched group produces exactly one generator cell (cap_array, cc, bjt array). Guard rings and dummies are emitted as layout-only children so LVS hierarchy matching is not broken.
- Beats hand layout because: guaranteed by construction instead of by partitioning skill.
- Philis status: partial — matched groups are generated as one cell (kernel/cells/src/cap_array.rs, kernel/analog/src/placement/cc.rs); annotator groups by kind (backend/annotator/src/lib.rs:110-117). No explicit "never split" assertion found.

### H15-06 Net-name and pin semantics as annotator inputs
- Kind: heuristic / data-model
- Statement: suffixes `_n` noisy, `_s` sensitive, `_hv` high-voltage, `b`/`z` negative logic, `1p5`/`3p3` voltage domain; pin directions give ERC: only-inputs ⇒ floating, two outputs ⇒ shorted.
- Source: §15.1.1; §15.4.4; reftext 46495–46514, 48618–48621, 48711–48713; PDF 783–784, 819–821.
- Philis stage: annotator, verify.
- Automation recipe: extend `netrole::classify_nets` with suffix rules (case-insensitive, SPICE is case-insensitive): `_n` → Noisy, `_s` → Sensitive, `_hv` → HighVoltage, `<d>p<d>` → voltage domain V (used by H15-36); names override inference only upward (a name can add sensitivity, never remove an inferred one). ERC: GPurify `multiple_drivers`, `floating_gate` rules already cover the two direction checks.
- Beats hand layout because: the tool reads designer intent that Hastings says "few layout designers" can infer (48618), and never forgets a tagged net.
- Philis status: partial — clock by name only (backend/annotator/src/netrole.rs:22-50); deck ERC `multiple_drivers(;` and `floating_gate(;` exist in sky130.deck.

### H15-07 Mirror-image placement of identical blocks
- Kind: rule
- Statement: two instances of the same block (e.g., dual op-amp) are placed as mirror images of one cell to give similar electrical characteristics and simpler symmetric routing to symmetric pads.
- Source: §15.2, Fig 15.5–15.6B; reftext 47136–47145, 47195–47198; PDF 794–795.
- Philis stage: annotator, gp, dp.
- Automation recipe: annotator detects repeated identical sub-blocks (same pattern, same sizes, disjoint nets) and emits a block-level symmetry constraint (mirror axis between them) plus identical generator variant choice; dp moves them as a mirrored pair.
- Beats hand layout because: identity of the two copies (variant, orientation, routing) is enforced exactly, including routing parasitics.
- Philis status: partial — device-level symmetry exists (kernel/analog/src/placement/symmetry.rs); block-level mirrored replication not found.

### H15-08 Thermal floorplanning: separate heat sources from sensitive/matched circuitry
- Kind: heuristic / constraint
- Statement: elongate/arrange so that sensitive input circuitry sits far from output-stage heat; matched devices go on isotherms (App. D Eq 13.24 converts ΔT to ΔI, see H15-58).
- Source: §15.2; reftext 47143-47145; PDF 794.
- Philis stage: gp, dp.
- Automation recipe: from op-point power per device (`P = I_D·V_DS`), compute the thermal field (pnr_core thermal) and (a) penalize ΔT across each matched pair (existing), (b) add a distance/ΔT budget between power devices (P > threshold) and the block's input pair.
- Beats hand layout because: the field is computed for every candidate placement, not guessed.
- Philis status: implemented for pairs — `ThermalGradient` prices ΔT from the live field (kernel/analog/src/placement/thermal.rs:7-24); field model kernel/core/src/thermal.rs:9-13. Die temperature from op-point only; no self-heating feedback (frontend/library/src/elaborate.rs:246-247).

### H15-09 Routing channel reservation, capacity sizing and choke-point detection
- Kind: algorithm / heuristic
- Statement: reserve routing area explicitly (20% of core in the example); primary channels near the die centre carry 10–20% of all top-level signals each; channels taper toward edges and along feeders; every feeder ≥5 signals; channel widths cannot grow once routing starts; careless block placement creates choke points.
- Source: §15.2 Fig 15.8; §15.4.3; reftext 47160–47162, 47241–47261, 48490–48498; PDF 794, 796, 817.
- Philis stage: gp, gr.
- Automation recipe: after global placement, run gr on a gcell grid whose capacity is derived per layer from `W_C = N·P_R + S_m` (H15-20); compute per-gcell demand from a quick probabilistic routing estimate (RUDY) before dp; add a gp penalty for gcells with demand > capacity and a floor of 5 tracks on any gcell boundary between blocks. Report choke points (min-cut capacity between the two halves of the netlist).
- Beats hand layout because: capacity is checked for every placement move, so choke points are removed before routing, which Hastings says is otherwise found too late.
- Philis status: partial — PathFinder negotiated congestion (backend/gr/src/lib.rs:1-9) with fixed `gcell_capacity: 6` (backend/gr/src/lib.rs:31-33); gp uses density overflow, not routing demand (backend/gp/src/lib.rs:167, 316-331).

### H15-10 High-current lead annotation: current and maximum resistance per net
- Kind: data-model / check
- Statement: every high-current lead carries its DC current and (if appropriate) its maximum allowed resistance; keep such leads short; EM gives the width floor, resistance usually forces wider.
- Source: §15.2; checklist item 5; reftext 47210–47214, 48887–48893; PDF 795, 823.
- Philis stage: annotator, gp, dr, verify.
- Automation recipe: annotator derives per-net DC current from the op-point (terminal currents) and a resistance budget `R_max = ΔV_allow/I`; gp weights HPWL by current (short high-current nets); dr widens to meet R_max; verify reports measured R vs R_max.
- Beats hand layout because: every net gets a current and a budget, and every routed net is measured, not only the ones someone remembered.
- Philis status: implemented in part — `IrDrop` budgets from op-point currents (frontend/library/src/lib.rs:286-288; kernel/analog/src/routing/ir.rs:8-19); EM rules from worst terminal current (frontend/library/src/lib.rs:1005-1027); gp HPWL weights come from capacitance budgets, not current (backend/gp/src/lib.rs:121-128).

### H15-11 Package and bondwire resistance in IR and Kelvin budgets
- Kind: formula
- Statement: `R_w = 4ρL/(πD²)`; 4N Au 2.2–2.4 µΩ·cm, 2N Au 3.0–3.3, Cu 1.7; 1 mm × 25 µm 4N Au = 47 mΩ; TCR Au 3700 ppm/°C, Cu 4000 ppm/°C (+37% from 25 to 125 °C); 2–3 wires per pin. Power-MOS sizing subtracts package R_P (Eq 15.6).
- Source: §15.3.3 Eq 15.17; reftext 47719–47740; PDF 805.
- Philis stage: annotator (budget), flow.
- Automation recipe: when a block pin is marked as a pad connection, subtract `R_w(T)` from the IR budget of that net and decide whether a Kelvin sense (H15-34) is needed: if `I·R_w` exceeds the net's accuracy budget, require a separate sense pin.
- Beats hand layout because: the off-chip share of the error budget is computed rather than ignored.
- Philis status: missing (no package model; grep "bondwire" empty).

### H15-12 Die-edge, corner and package-stress keep-outs for matched devices
- Kind: rule / constraint
- Statement: plastic-package stress is high at die edges and maximal (shear) at corners, larger dice worse; corner exclusion zones (typical when the longer die side >2.5 mm): `d_c ≥ 0.05 × longer die dimension` (5 mm → 250 µm) with nothing active inside, not even the ground ring.
- Source: §15.3.2 Fig 15.10; reftext 47394–47438; PDF 799–800.
- Philis stage: gp, dp, verify, deck.
- Automation recipe: when the block's die position is known (or the block is the die), add a placement budget: matched groups keep ≥ a deck-given distance from die edges (default: outside the outer 10% band, "not given" by Hastings as a number) and never in corner triangles of leg `0.05·max(W,H)`; deck gets `corner_exclusion(frac: 0.05, min_die: 2.5mm)`.
- Beats hand layout because: exact geometric compliance and a stress-aware ranking of placements, not a visual check.
- Philis status: missing (grep "die.?edge|corner.?exclu" empty).

### H15-13 Thick-metal edges and bondpads versus matched devices
- Kind: rule / constraint
- Statement: avoid matched analog circuitry under thin-Al BOAC bondpads (stress gradients unpredictable); with thick top metal (power Cu), place matched devices well within or well away from the thick-metal geometry; never let a thick-metal edge intersect matched devices.
- Source: §15.3.3; reftext 47704–47713; PDF 804.
- Philis stage: gp, dp, dr, verify.
- Automation recipe: data model marks layers as `thick` (deck thickness > k × M1, or deck flag); dr forbids thick-layer shapes whose edge crosses a matched cell's bbox (or budgets the distance to the edge); gp keeps matched cells out of pad keep-outs. Check: for each matched cell, `min(dist(edge_thick, cell))` either 0-with-full-containment or ≥ d_min.
- Beats hand layout because: every thick-metal edge is checked against every matched device; humans route power last and miss this.
- Philis status: missing (no thick-metal/matched-cell interaction; grep "thick" in routing finds none).

### H15-14 Anti-delamination slotting and slotted-metal resistance
- Kind: deck-requirement / formula
- Statement: slots of W_S × L_S with max gaps S_L (length) and S_W (width), alternate columns offset, adjacent-layer slots offset; slot long axis parallel to the die edge (or to the corner-zone diagonal); `R_slot/R_unslot ≅ W·L_S/((S_L+L_S)(W − N_S·W_S)) + S_L/(S_L+L_S)`, N_S = average slots across the lead.
- Source: §15.3.2 Fig 15.11 Eq 15.16; reftext 47441–47502; PDF 800–801.
- Philis stage: deck, dr, verify.
- Automation recipe: if a deck declares a slotting rule (max unslotted width), dr emits slot shapes on the slot layer for wide leads, oriented per the rule, and IR/EM use `R_slot` from Eq 15.16 and the reduced cross-section `W − N_S·W_S` for current density.
- Beats hand layout because: slot pattern and its resistance penalty are generated and accounted for exactly.
- Philis status: missing (grep "slot" finds only unrelated grid slots in kernel/cells/src/bjt.rs:62-63; sky130.deck lists no slot rule).

### H15-15 ESD path resistance and ground-ring width
- Kind: check / deck-requirement
- Statement: total metal resistance through ESD devices between any two bondpads must not exceed a limit, "often two ohms"; the substrate metal (ground ring) is widened to meet it.
- Source: §15.3.2; checklist item 7; reftext 47385–47391, 48898–48904; PDF 799, 823.
- Philis stage: verify, dr.
- Automation recipe: for every pad pair, compute the resistance of pad→clamp→ring→clamp→pad paths from extracted metal (PEX R); fail if > deck limit; dr widens the ring/branches until satisfied.
- Beats hand layout because: all N² pad pairs are checked, not the few a reviewer samples.
- Philis status: partial — deck has topological ESD (`esd_topological(pad; clamps: …)`, sky130.deck; gf180mcu.deck `ESD.clamp_path`); no resistance check.

### H15-16 Bondpad and probe-pad geometry rules
- Kind: deck-requirement
- Statement: pad opening ≈3× wire diameter, pitch ≈5× (25 µm wire → 75 µm, 125 µm); metal-exclusion circle ≈50 µm larger in diameter than the opening; leads within 25 µm of a pad ≥10 µm wide; pad-1 identification. Probe pads: POR ≥ max(2× tip, 0.75× pitch); blade cards ≥100 µm pitch, ≤88 probes; epoxy ring ≥50 µm; dual ranks offset and separated by ≥ min pitch. "All numerical values are merely examples".
- Source: §15.3.3; reftext 47587–47612, 47748–47778; PDF 802–806.
- Philis stage: deck, cells, verify.
- Automation recipe: a pad cell generator parameterized by deck `pad` rules; verify keep-out and lead-widening rules as width-within-distance checks.
- Beats hand layout because: rule-exact pads; low value for block P&R.
- Philis status: partial — deck `pad.2 space(pad) >= 1270nm` (sky130.deck), IHP `Pad.a1`, `Pad.d` (ihp_sg13g2.deck); no pad generator.

### H15-17 Bond-over-active (BOAC) placement rules
- Kind: rule / deck-requirement
- Statement: topmost metal for the pad; few topmost vias under it; minimize next-to-top metal density and keep its lines thin; dense lower-level via arrays encouraged; with thick top metal lower layers can be used freely.
- Source: §15.3.3; reftext 47666–47713; PDF 804.
- Philis stage: dr, gp.
- Automation recipe: pad keep-out region with per-layer density caps (next-to-top ≤ deck value, topmost vias forbidden) consumed by dr as blockage/cost; matched devices excluded (H15-13).
- Beats hand layout because: per-layer density under the pad is enforced numerically.
- Philis status: missing.

### H15-18 Via stacking and enclosure model per via technology
- Kind: deck-requirement / data-model
- Statement: Al vias not stackable (VIA2 not on VIA1; VIA2 on contact and VIA3 on VIA1 usually allowed), symmetric enclosure both metals; W-plug vias stackable, lower-metal enclosure mandatory (two sides), upper optional/asymmetric; single/dual damascene: stackable, both enclosures optional/asymmetric (Table 15.3). Same-size vias only; high current uses arrays of minimum vias.
- Source: §15.4.1 Table 15.3; reftext 48068–48146; PDF 811–812.
- Philis stage: deck, dr.
- Automation recipe: deck states `stackable(via_i, via_j)` and `asymmetric_enclosure` per cut; dr's via-stack builder refuses unstackable pairs (offsets them) and uses the narrow enclosure along the wire direction.
- Beats hand layout because: exact exploitation of asymmetric enclosure gives the tighter P_VTV of Eq 15.19 on every via.
- Philis status: partial — `asymmetric_enclosure … min_one_side` read in backend/verify/src/pdk.rs:354-358, 994, 1098; stackability not modelled (grep "stack" in pdk.rs not checked in depth; no `stackable` rule kind in sky130.deck).

### H15-19 Redundant (paired) vias against stress voiding
- Kind: rule
- Statement: void formation per via is probabilistic; many fabs demand pairs of vias rather than singles; adjacent vias protect each other beyond redundancy.
- Source: §15.4.1; reftext 48148–48153; PDF 812.
- Philis stage: dr, verify.
- Automation recipe: dr places ≥2 cuts at every layer change where space allows (always on power, bias and matched-signal nets; opportunistically elsewhere), then `max(2, ⌈I/I_cut⌉)`; report single-cut count as a metric.
- Beats hand layout because: doubling is applied everywhere space permits, and the residual single-cut list is explicit.
- Philis status: partial — EM cut count `n = ⌈I/I_cut⌉` (kernel/analog/src/routing/em.rs:59-60); no minimum of 2 found (grep "redundan|min_cuts" empty).

### H15-20 Routing pitch formulas and track lattice
- Kind: formula
- Statement: `P_LTL = W_m + S_m`; `P_VTV = W_v + 2E_mv + S_m` (E_mv = smallest opposite-side enclosure); `P_VTL = (W_v+W_m)/2 + E_mv + S_m` (staggered vias, congests intersections); channel width `W_C = N·P_R + S_m`. Example (Table 15.4): P_LTL 0.6 µm, P_VTV 0.65 µm.
- Source: §15.4.2 Eqs 15.18–15.21, Table 15.4; reftext 48175–48281; PDF 812–814.
- Philis stage: gr, dr, deck.
- Automation recipe: per layer compute P_VTV from deck width, space (incl. EOL) and cut size + min-one-side enclosure; build a per-layer track lattice; use W_C for gcell capacity (H15-09).
- Beats hand layout because: minimum legal pitch per layer, computed from the deck.
- Philis status: implemented with one pitch for all layers — `routing_pitch = wire_width + max(route_spacing)` over the stack (backend/verify/src/pdk.rs:423-433); wire width = via pad extent (frontend/library/src/elaborate.rs:390-399), i.e. via-to-via pitch; per-layer pitch is a noted ponytail (pdk.rs:426-428).

### H15-21 Grid consistency of path edges
- Kind: rule / check
- Statement: path edges must land on the coding grid: a width that is an odd multiple of the increment puts centreline-defined edges off grid (0.35 µm path on 0.05 µm grid → edges on 0.025 µm), so widen (to 0.4 µm; P_VTV → 0.7 µm). Routing grid must be an integer multiple of the coding increment.
- Source: §15.4.2; reftext 48285–48313; PDF 814.
- Philis stage: dr, cells.
- Automation recipe: all wire widths are snapped to `2·grid` multiples when drawn by centreline; lattice pitch is a multiple of grid; verify `off_grid` rule.
- Beats hand layout because: no rounding-induced DRC failures.
- Philis status: implemented in part — rect-based geometry and `snap_cut` in cells (kernel/cells/src/mosfet.rs:308); deck `off_grid(;` rule in sky130.deck.

### H15-22 Octagonal (45°) routing and diagonal width rules
- Kind: deck-requirement / algorithm
- Statement: 45° routing packs signals along diagonal edges; path vertices fall off grid; some rules require wider diagonal segments; use the diagonal width for the pitch.
- Source: §15.4.2 Fig 15.23; reftext 48317–48333; PDF 814–815.
- Philis stage: dr, deck.
- Automation recipe: optional; only needed for EM bend relief (H15-47) and corner zones. Deck: `angle(allowed: …)` plus `diagonal_width`.
- Beats hand layout because: n/a (low priority).
- Philis status: missing in router (Manhattan L-jogs, backend/dr/src/lib.rs:1072-1074); deck allows 45° (`x.3a angle(; allowed: [0deg, 45deg, 90deg, 135deg])`, sky130.deck).

### H15-23 Die-wide preferred direction per metal layer
- Kind: rule
- Statement: each routing metal runs perpendicular to the one below, enforced across the entire die; long M2 runs in one direction inside cells enable maze routing over them.
- Source: §15.4.3; §15.1.3; reftext 48363–48366, 46813–46824; PDF 815, 789.
- Philis stage: gr, dr, cells.
- Automation recipe: lattice with alternating directions; cells restricted to the lowest layers so upper layers stay routable over them (R_f → 1).
- Beats hand layout because: uniform.
- Philis status: implemented — "even layers horizontal, odd vertical" (backend/gr/src/lib.rs:513-514; backend/dr/src/lib.rs:175).

### H15-24 Wide-lead widths as integer track multiples
- Kind: formula / rule
- Statement: wide leads should consume integer multiples of the routing pitch so several minimum-width leads can replace the wide lead where it does not run the full channel: `W_N = N·P_R + S_m`, N integer. Example: P_R = 0.7 µm, S_m = 0.3 µm → 1.0, 1.7, …, (0.7N + 0.3) µm.
- Source: §15.4.3 Eq 15.22; reftext 48387–48400; PDF 816.
- Philis stage: dr.
- Automation recipe: when dr fattens a trunk (IR/EM), quantize its width to the Eq 15.22 series `w = N·pitch + space` (not an arbitrary value), so the lattice stays consistent and a partially-used wide trunk can be swapped for N lattice wires.
- Beats hand layout because: widening never strands partial tracks.
- Philis status: partial — dr fattens trunks to "the widest width (≤ its net's cap) that keeps Θ" (backend/dr/src/lib.rs:637-705); quantization to track multiples not found.

### H15-25 Channel fill order and jog avoidance
- Kind: heuristic / algorithm
- Statement: route wide (power/ground) leads first; place first signals at channel edges and work inward at the chosen minimum spacing; avoid jogs (they propagate congestion laterally); route metal-only signals before poly-tolerant ones; overload a full channel with non-preferred metal or poly for static signals.
- Source: §15.4.3; reftext 48387–48441; PDF 816–817.
- Philis stage: gr, dr.
- Automation recipe: net ordering key = (wide/supply first, then sensitive, then others, poly-tolerant static last); PathFinder bend penalty inside channels; allow non-preferred-direction segments only at a cost.
- Beats hand layout because: negotiated congestion revisits ordering automatically.
- Philis status: partial — "Supply nets widen first" (backend/dr/src/lib.rs:640); PathFinder negotiation (backend/gr/src/lib.rs:1-9); no explicit class-ordered channel fill.

### H15-26 Fill the overlap with vias at wide-lead layer changes and branches
- Kind: rule
- Statement: where wide leads change layers, extend both leads fully across each other and fill the rectangle with as many vias as fit; do the same at branches off wide leads (free, the area is already consumed).
- Source: §15.4.3 Fig 15.25; reftext 48411–48414; PDF 816.
- Philis stage: dr.
- Automation recipe: at any layer change of a net whose width > 1 track, compute the overlap rectangle and place the maximum via array under deck `via_array_spacing`; count ≥ EM need.
- Beats hand layout because: maximal cut count everywhere, lowering R and EM stress.
- Philis status: partial — EM-driven cut counts (kernel/analog/src/routing/em.rs:59-60); `via_array_spacing` read (backend/verify/src/pdk.rs:492-497; frontend/library/src/elaborate.rs:424). "Fill the overlap" policy not found.

### H15-27 Net-class taxonomy for routing (Table 15.5)
- Kind: data-model
- Statement: signal classes: high-voltage (label voltage), high-current (label width), HV+HC (both), sensitive analog, noisy digital, static digital (may route in poly), other (must route in metal).
- Source: §15.4.3 Table 15.5; reftext 48445–48488; PDF 817.
- Philis stage: annotator, gr, dr.
- Automation recipe: extend `NetClass` with `HighVoltage{v_max}`, `HighCurrent{i_dc}`, `Noisy{slew}`, `StaticDigital` (poly-OK, shield-lead candidate); annotator infers from op-point (V, I), device types (logic inverters driving analog switch gates = static digital), names (H15-06).
- Beats hand layout because: every net is classified; classes drive rules automatically.
- Philis status: partial — `NetClass {Signal, Clock, Supply, Ground, Sensitive, Substrate}` (kernel/analog/src/metadata.rs:18-26); no HV/HC/static-digital classes.

### H15-28 Sensitive-net detection (eight categories)
- Kind: heuristic
- Statement: sensitive: inputs to high-gain amplifiers and precision comparators; ADC inputs; precision reference outputs; analog grounds of accurate circuits; high-value resistor networks; very low-voltage signals; bias current lines into low-current accurate circuits; any very-low-current circuitry.
- Source: §15.4.4; reftext 48604–48616; PDF 819.
- Philis stage: annotator.
- Automation recipe: mark Sensitive if (a) net drives gates of a matched diff pair or comparator input pair (exists); (b) net is a bandgap/reference output (catalog pattern); (c) ground/return net of a matched block; (d) internal node of a resistor chain with R > threshold (e.g. node impedance > 100 kΩ, Hastings' 1 V example uses 100 kΩ); (e) op-point |V| small and node impedance high; (f) gate/drain of a bias mirror with I < 1 µA (threshold "not given"; set by config); (g) any node whose small-signal impedance `R_node` × allowed ΔV bound yields a coupling budget < some aF (see H15-29).
- Beats hand layout because: exhaustive detection over all nets with op-point data.
- Philis status: partial — Sensitive = gates of devices in sensitive blocks (backend/annotator/src/lib.rs:119-128; block kinds backend/annotator/src/block.rs:60).

### H15-29 Coupling budget from injected-noise physics
- Kind: formula / metric
- Statement: `ΔV = C·R·dV/dt` for aggressor slew `dV/dt` coupling through `C` into a victim of resistance `R`; 1 fF can matter; 10 fF into 100 kΩ at 1 GV/s → 1 V.
- Source: §15.4.4 Eq 15.23; reftext 48582–48595; PDF 819.
- Philis stage: annotator (budgets), verify (scoring).
- Automation recipe: per victim v and aggressor a: `C_max(v,a) = ΔV_allow(v) / (R_node(v) · SR(a))`, with `R_node` from the op-point small-signal (1/g at the node, or DC resistance to a low-impedance node), `SR(a)` from netclass (clock: VDD/t_edge; static digital: VDD/(R_series·C_line), H15-31), `ΔV_allow` from the offset/accuracy budget (existing `offset_sigma_mv`). Budget sum over aggressors = CouplingBudget. Scoring: report `Σ_a C(v,a)·SR(a)·R(v)` in mV.
- Beats hand layout because: coupling limits become per-pair numbers derived from the circuit instead of a blanket "keep apart".
- Philis status: partial — CouplingBudget sums coupling onto a victim (kernel/analog/src/routing/coupling.rs:1-33; emitted backend/annotator/src/extract.rs:78-84), budgets from gate area/class margins (backend/annotator/src/extract.rs:108-114), not from R·SR; no slew model (grep "slew|dv_dt" empty).

### H15-30 No noisy-over-sensitive on adjacent layers; cross at right angles
- Kind: rule / cost
- Statement: noisy signals must not run above/below (adjacent layer) or beside sensitive ones; a necessary crossing is made at right angles to minimize overlap area; lateral C can equal vertical C in dense stacks.
- Source: §15.4.4; checklist 10; reftext 48625–48629, 48683–48687, 48925–48931; PDF 819–820, 824.
- Philis stage: gr, dr, verify.
- Automation recipe: coupling evaluation must include inter-layer overlap C (`C = ε·A_overlap/t_ild` + fringe) between shapes on adjacent layers; gr/dr cost: forbid parallel overlap of a (noisy, sensitive) pair on adjacent layers, allow orthogonal crossings with cost ∝ overlap area; verify counts parallel-overlap length.
- Beats hand layout because: every overlap is enumerated and priced; a human checks only obvious ones.
- Philis status: missing for vertical coupling — lateral same-layer only (kernel/analog/src/routing/coupling.rs:20-24, 61-62; test "different_layers_do_not_couple_laterally" coupling.rs:168-172); keep-away pairs push nets apart in gr (backend/gr/src/lib.rs:204, 284-290).

### H15-31 Slew limiting of static digital control lines
- Kind: heuristic (design feedback)
- Statement: series resistance of a few tens of kΩ in the gate driving a static control line cuts slew and injection; 100 kΩ with 1 pF line → ≈10 MV/s; not for clocks/data buses; resistor value depends on line length (compute, back-annotate, or add C).
- Source: §15.4.4; reftext 48670–48679; PDF 820.
- Philis stage: annotator (report), verify.
- Automation recipe: after routing, for each static-digital aggressor violating H15-29, report the series R needed from `SR ≈ V_swing/(R_s·C_line)` (the 100 kΩ × 1 pF → 10 MV/s example): `R_s = V_swing/(SR_target·C_line)` with extracted C_line and SR_target from H15-29.
- Beats hand layout because: exact R from extracted C per line.
- Philis status: missing.

### H15-32 Electrostatic shield plates and shield leads
- Kind: rule / algorithm
- Statement: (a) plate: square of intermediate metal between a noisy and a sensitive crossing, tied to a quiet low-impedance node, ideally the sensitive node's own reference (e.g., a reference's analog ground), extending 2–3 µm beyond the intersection; in DLM route one signal in poly under M1-plate. (b) lead: run a quiet line (static digital, supply, ground) between parallel noisy and sensitive lines. (c) full shielding in QLM: signal on M3 with M3 side shields and M2/M4 plates (three routing layers).
- Source: §15.4.4 Fig 15.28; §15.1.3; reftext 48638–48687, 46831–46833; PDF 819–820, 789.
- Philis stage: annotator, dr.
- Automation recipe: annotator picks the shield reference per victim as the net the victim is referenced to (for a reference output: its ground; for a diff-pair input: the pair's tail/ground domain), not a global ground; dr inserts plates at every unavoidable crossing with a noisy net (extent = overlap + 2–3 µm), side leads for parallel runs; full coax for top-sensitivity nets when ≥4 metals are free.
- Beats hand layout because: shielding is inserted exactly where the coupling analysis says it pays, and its added C is traded against the victim's C budget.
- Philis status: partial — `Shield` requires reference metal on both sides, same layer (kernel/analog/src/routing/shield.rs:8-27); "ponytail: one-sided and same-layer only (no top/bottom plates); tie impedance is not measured" (shield.rs:18-19); reference is always the ground net and only when a clock exists (backend/annotator/src/extract.rs:87-104).

### H15-33 Star nodes for matched returns
- Kind: rule / algorithm
- Statement: matched devices whose emitters/sources return to a line carrying other current see `ΔV = I·R` between their tap points (10 squares × 30 mΩ/□ = 0.3 Ω, 1 mA → 0.3 mV); returning both to one point C makes the drops common-mode.
- Source: §15.4.4 Fig 15.26; reftext 48509–48538; PDF 817–818.
- Philis stage: annotator, dr, verify.
- Automation recipe: annotator emits `Star{net, members, root}` for any matched group whose shared terminal net also carries current from other devices (from op-point: net current ≠ Σ members' currents); dr routes each member's branch from a single root shape (tree with degree ≥ members at the root, no member branch sharing current with foreign branches); verify: the resistance from root to each member terminal equal within ΔR budget, and no foreign current path through a member branch.
- Beats hand layout because: the star is guaranteed topologically and the branch resistances are balanced to a numeric budget.
- Philis status: partial — `CommonNode` balances routed R from net centre to each member's source (kernel/analog/src/routing/common_node.rs:10-20; emitted frontend/library/src/lib.rs:798; backend/dr/src/lib.rs:560); star topology (no foreign current in branches) not enforced (grep "star" empty).

### H15-34 Kelvin (force/sense) connections
- Kind: rule / check
- Statement: force leads carry current; sense leads carry negligible current and connect at the resistor terminals; in precise circuits match the two sense-lead currents and resistances so their drops cancel; "Kelvin at a bondpad" must be physically at the pad; separate schematic names via metal resistors prevent force/sense shorts but do not guarantee location.
- Source: §15.4.4 Fig 15.27; checklist 12; reftext 48540–48549, 48940–48944; PDF 818, 824.
- Philis stage: annotator, dr, verify.
- Automation recipe: detect current-sense resistors (low-R resistor whose two terminals also drive the inputs of an amplifier/comparator); split each terminal net into force and sense sub-nets that meet only at the resistor terminal (a "tap point" constraint); dr routes sense branches from the terminal shape itself; verify extracts that the sense branch has no shared segment with the force branch and that the two sense resistances match within budget.
- Beats hand layout because: the tap location is verified geometrically, which Hastings says schematic tricks cannot ensure.
- Philis status: missing (grep "kelvin" empty).

### H15-35 Selective parasitic back-annotation for noise
- Kind: algorithm
- Statement: full extraction makes simulation slow; instead insert only the coupling capacitances between noisy nets and pre-selected critical nodes, and lump all other capacitance at those nodes to ground.
- Source: §15.4.4; reftext 48629–48635; PDF 819.
- Philis stage: verify (PEX scoring), flow.
- Automation recipe: in the PEX scoring step, extract the coupling matrix only for (noisy × sensitive) pairs plus ground C per sensitive node; feed H15-29's ΔV metric; the full netlist only for final signoff.
- Beats hand layout because: fast enough to run every epoch, so noise enters the objective rather than a final review.
- Philis status: partial — PEX in signoff via GPurify (backend/verify/src/lib.rs); selective coupling extraction not found.

### H15-36 Voltage-aware spacing (high-voltage nets)
- Kind: rule / deck-requirement
- Statement: ILO allowed field ≈2 MV/cm (200 V/µm nominal) but a drawn 0.25 µm space may hold only ≈25 V (microloading, OPC, damascene taper); low-k porous dielectrics weaker; required spacing depends on the voltage difference between the two adjacent leads, not on either lead's absolute voltage; flagging `_hv` nets is a crude substitute.
- Source: §15.4.4; App C Table C.6 note 4; reftext 48691–48729, 50099, 50130; PDF 820–821, 841.
- Philis stage: annotator, dr, verify, deck.
- Automation recipe: annotator computes each net's voltage range [V_min, V_max] over op-points (DC + corners; supply domains from names H15-06); for every pair of adjacent shapes, `ΔV = max|V_a − V_b|` over ranges; spacing = deck table `space(layer, ΔV)`; dr uses it as a per-pair spacing; verify runs it on extracted geometry. Deck grammar: `voltage_space(met1; table: [(5V, 0.14um), (20V, …)])`.
- Beats hand layout because: exact pairwise ΔV instead of a conservative per-net HV flag, so area is not wasted on two HV nets at the same potential (Hastings' stated problem, 48718–48722).
- Philis status: missing — sky130.deck omits "hv.* and vhvi.* (net propagation of high voltage)" (sky130.deck:654); no voltage spacing in Philis (grep "voltage.?(aware|dependent)" finds nothing relevant).

### H15-37 EM at every point, resistance on average
- Kind: rule
- Statement: EM must hold at every point of a lead (no neck-downs around obstructions); resistance averages, so a short narrow section costs little R and widening where room exists lowers R.
- Source: §15.4.4; reftext 48739–48744; PDF 821.
- Philis stage: dr, verify.
- Automation recipe: EM as a hard per-segment width check (min over segments); IR as a path integral (widen opportunistically); both on the same net.
- Beats hand layout because: every segment of every net is checked.
- Philis status: implemented — hard per-segment EM width (kernel/analog/src/routing/em.rs:43-57; backend/dr/src/lib.rs:150); IR budget on worst path (kernel/analog/src/routing/ir.rs:8-19); dr widening (backend/dr/src/lib.rs:637-670).

### H15-38 EM minimum width and temperature derating
- Kind: formula
- Statement: `W_min = I_max/(J_max·t)` (MIL-M-38510 J_max = 5×10⁵ A/cm² for Al under glass; 50 mA on 1 µm metal → 10 µm); derating `D = exp[(E_a/(n·k))(1/T_j − 1/T_0)]`, k = 8.62×10⁻⁵ eV/K, E_a 0.5 eV (pure Al), ≈0.7 eV (Cu-doped Al), n ≈2 (Al), ≈1 (Cu); 105 → 125 °C gives 0.58; Cu/Al parity ≈175 °C. Exercise 15.12 uses duty-cycle averaging (360 mA at 50%).
- Source: §15.4.4 Eqs 15.24–15.25; §15.7 Ex 15.12; reftext 48748–48809, 49080–49088; PDF 821–822, 828.
- Philis stage: dr, verify, deck.
- Automation recipe: already coded; the gap is data: decks must carry `(T_ref, E_a, n)` per EM rule, and the op-point temperature (or local T from the thermal field) must feed `derate` per conductor.
- Beats hand layout because: derating per conductor at its own temperature.
- Philis status: implemented but inert — `derate()` (kernel/analog/src/routing/em.rs:12-21) applied when the deck provides `derating` (backend/verify/src/pdk.rs:32-34; frontend/library/src/elaborate.rs:242-258); no deck has these fields (ihp_sg13g2.deck:789-790 "no activation energy or current exponent"; sky130 EM rules lack them).

### H15-39 Current crowding at bends and via arrays in straight segments
- Kind: rule
- Statement: inside corners of 90° bends see much higher current stress; replace a 90° interior angle by two 135° angles; a via array at a 90° corner loads the inside-corner via most (Fig 15.29A #1) and the outside-corner least; place arrays in straight segments where each column carries roughly equal current.
- Source: §15.4.4 Fig 15.29; reftext 48818–48833; PDF 822.
- Philis stage: dr, verify.
- Automation recipe: for nets whose segment current density exceeds, e.g., 50% of the EM limit (threshold not given), (a) chamfer 90° corners of wide wires with a 45° notch fill (requires H15-22), (b) move layer-change via arrays off corners onto a straight run (extend the lower wire past the bend by the array length), (c) EM check derates the via array by a corner factor if left at a bend (factor not given by Hastings; use FEA or a conservative 1/n_columns per inside via).
- Beats hand layout because: applied to every high-current bend, not just ones a reviewer spots.
- Philis status: missing — Manhattan L jogs with overrun corners (backend/dr/src/lib.rs:1029-1030, 1072-1074).

### H15-40 Dynamic/AC EM with Blech
- Kind: check
- Statement: tools simulate current waveforms and check widths, and apply Blech rules to short leads; hard for analog because waveforms are hard to determine.
- Source: §15.4.4; checklist 4; reftext 48839–48843, 48879–48886; PDF 822–823.
- Philis stage: verify.
- Automation recipe: EM current per net = max(|I_DC|, I_avg from a transient over the op-point's operating mode, I_rms for Joule heating); Blech immortality `j·L < (jL)_B` per diffusion domain.
- Beats hand layout because: automatic waveform-based currents per segment.
- Philis status: partial — Blech in `Limit::width_nm` (kernel/analog/src/routing/em.rs:30-57); currents are DC op-point worst terminal (frontend/library/src/lib.rs:1005-1017).

### H15-41 Power plots: pinch points and via counts
- Kind: check
- Statement: generate views of power and high-current nets and inspect for unexpected pinch points and inadequate via counts; FEA (R3D) for power-transistor metal.
- Source: §15.5 item 4; reftext 48879–48886; PDF 823.
- Philis stage: verify, visualizer.
- Automation recipe: for each supply/high-current net, compute min width along each current path and cut count per layer change vs `⌈I/I_cut⌉`; render a heat map (visualizer) of `J/J_max` per shape; list top pinch points in the report.
- Beats hand layout because: numeric J/J_max per shape instead of visual inspection.
- Philis status: partial — EM rule evaluation per net (kernel/analog/src/routing/em.rs:92-123); no per-shape J/J_max map found.

### H15-42 LVS topological vs parametric; waivers
- Kind: check / metric
- Statement: final LVS must have zero topological errors (never waivable: one error can mask others); parametric errors may be waived after review (e.g., guard-ring diode areas across hierarchy); DRC false diagnostics waived only after process review.
- Source: §15.5 items 1–3; reftext 48856–48878; PDF 823.
- Philis stage: verify, flow.
- Automation recipe: epoch ranking counts topological LVS errors as infinite (reject), parametric as scored; waivers only via an explicit sidecar list with reasons, reported in every run.
- Beats hand layout because: consistent policy and a machine-readable waiver ledger.
- Philis status: partial — sidecar waivers `cell.waivers` rule id → reason (backend/verify/src/checker.rs:51-53); topological vs parametric LVS split not verified here.

### H15-43 Latchup exposure of pin-connected diffusions
- Kind: check
- Statement: any diffusion connected to a vulnerable pin directly or through < ≈50 kΩ can trigger latchup; substrate pins are not vulnerable; positive-supply pins see no below-ground transients, negative-supply pins no above-ground; I/O pins both; protect with minority-carrier guard rings, else add substrate/well contacts to minimize debiasing. CDM: clamps near every pin-connected gate oxide, and at every supply/ground domain crossing.
- Source: §15.5 items 7–8; reftext 48898–48918; PDF 823–824.
- Philis stage: annotator, cells, verify.
- Automation recipe: graph walk from each block pin through resistors with cumulative R < 50 kΩ to diffusion terminals; classify pin polarity exposure; require an electron- or hole-collecting guard ring (ECGR/HCGR) of the right type around each exposed device; for gates reachable from a pin, require a clamp within a distance budget.
- Beats hand layout because: exhaustive reachability instead of review.
- Philis status: partial — annotator assigns guard rings by device polarity (backend/annotator/src/constraints.rs:8-11, 69); no pin-reachability latchup check; decks lack pad latchup geometry (ihp_sg13g2.deck:792).

### H15-44 Deck rule classes needed for analog P&R (Appendix C)
- Kind: deck-requirement
- Statement: Appendix C (Tables C.2–C.8) uses these classes; each must be expressible in the deck:
  1. Minimum width per layer (e.g., CMOS NWELL 5.0, NMOAT/PMOAT 3.0, POLY1 2.0, METAL1 2.0, METAL2 2.0, POR 4.0 µm; bipolar NBL/TANK/DEEPN 8, BASE/EMIT/METAL1 6, POR 10 µm).
  2. Exact size for cuts ("CONT width 1.0 µm exact", "VIA width 1.0 µm exact", bipolar CONT 4 µm) with object-class exceptions ("except for bondpads", note 2).
  3. Same-layer spacing (NWELL 15.0, NMOAT 5.5, PMOAT 5.5, POLY1 2.0, CONT 2.0, M1 2.0, VIA 2.0, M2 2.0, POR 4.0 µm; bipolar TANK 6, BASE 14, EMIT 6, CONT 4, M1 4, POR 10 µm).
  4. Inter-layer spacing (NMOAT–NWELL 9.5, PMOAT–NWELL 7.0, PMOAT–NMOAT 4.0, POLY1–MOAT 2.0, CONT–MOAT 1.0, CONT–POLY1 2.0, POR–VIA 2.0; bipolar BASE–DEEPN 18, BASE–EMIT 12, EMIT–CONT 6 µm).
  5. Enclosure/overlap (NWELL⊃NMOAT 1.0, NWELL⊃PMOAT 2.0, MOAT/POLY1⊃CONT 1.0, M1⊃CONT 1.0, M1⊃VIA 1.0, M2⊃VIA 1.0, M2⊃POR 2.0; bipolar TANK⊃NBL 22, TANK⊃DEEPN 24, TANK⊃BASE 22, TANK⊃EMIT 18, BASE⊃EMIT 4, M1⊃POR 4 µm), plus asymmetric two-side enclosure (Table 15.4).
  6. Extension/overhang (POLY1 over MOAT endcap 1.0; MOAT over POLY1 S/D extension 3.0; BASE overhang HSR 2; HSR into BASE 2; NSD/PSD overhang POLY2 1.5; PSD overhang BASE 1.0 µm).
  7. Exact-distance / no-touch rules ("VIA spacing to CONT 1.0 µm exact", "VIA must not touch CONT", note 3) = stacking prohibition.
  8. Voltage-conditional spacing ("For different voltages 5.5 µm; for same voltage 3 µm", note 4) — needs net voltage data.
  9. Region-conditional spacing (NMOAT–NMOAT 5.5 µm only outside NWELL; 4.0 µm inside, note 5).
  10. Connectivity-conditional abutment ("PMOAT allowed to abut NMOAT if the two are connected by METAL1", note 1) — butted taps.
  11. Containment-conditional rules (BiCMOS rules 67, 71–73 apply "both PSD and NSD contained by BASE").
  12. Derived layers / mask generation (NMOAT → MOAT+NSD; PMOAT → MOAT+PSD; CHST from NWELL and MOAT; BOI auto; BASE drawing → MOAT+BASE masks; slot layers `FINAL = MET − SLOT`, Eq 15.16 context).
  13. Optional process modules as separable rule sets (HSR, Schottky, Poly-2, BiCMOS).
  14. Coding increment/grid (bipolar 1 µm; CMOS 0.25 µm).
  15. Process electricals (Tables C.1, C.5: sheet R min/mean/max, cap per area, V_t, k, breakdown voltages, thick-field thresholds) — for sizing, PEX, IR/EM, voltage-spacing.
  Plus chapter-15 classes: wide-metal spacing, EM current density per layer and per cut with (T_ref, E_a, n), antenna ratios, density windows, pad/probe-pad rules, slotting, corner exclusion, diagonal widths, via stacking, voltage-dependent spacing, ESD path resistance, latchup distance.
- Source: App C Tables C.2–C.8 and notes; §15.3–15.4; reftext 49725–50443; PDF 837–844.
- Philis stage: deck, verify, dr, cells.
- Automation recipe: audit each GPurify deck for each class; add missing grammar: `voltage_space`, EM derating fields, `stackable`, `slot`, `corner_exclusion`, `esd_resistance`, `latchup_distance`.
- Beats hand layout because: a complete machine-readable deck is the precondition for every rule-exact automation step.
- Philis status: partial — sky130.deck has width, space, notch, wide_space, enclosure (incl. asymmetric), extension, size (exact), area, hole_area, angle, forbidden/inside/contains, antenna, em_current_density, density_cmp, esd_topological, ir_drop, off_grid, floating_well, supply_short, multiple_drivers (counted from `grep ^rule sky130.deck`); via_array_space in gf180mcu/ihp; missing: voltage-conditional spacing (sky130.deck:654 omits hv.*), EM derating fields, stacking, slotting, corner exclusion, ESD resistance.

### H15-45 Resistor self-heating width floor
- Kind: formula
- Statement: temperature rise of a thin-film resistor over oxide: `ΔT = I²·t_ox·ρ/(κ·W²)` (κ = oxide thermal conductivity, t_ox = oxide thickness under the resistor). Invert: `W ≥ I·√(t_ox·ρ/(κ·ΔT_max))`.
- Source: App D Eq 5.7 (5.7.1–5.7.4); reftext 50456–50501; PDF 846.
- Philis stage: annotator, cells, verify.
- Automation recipe: for each resistor segment (and each narrow metal/poly route carrying current), compute ΔT from op-point current; for matched resistors, the two ΔT must agree (feeds ThermalGradient as a local heat source); set cells' minimum width so ΔT ≤ budget (e.g., 1 K; budget not given).
- Beats hand layout because: self-heating mismatch is computed per segment.
- Philis status: missing (frontend/library/src/elaborate.rs:246-247: local self-heating not fed back).

### H15-46 Adiabatic pulse and ESD conductor sizing
- Kind: formula
- Statement: adiabatic heating by a pulse of duration τ: `W_min = (I_max/t_R)·√(ρτ/(d·c_H·ΔT))` (d density, c_H specific heat). HBM strike (τ = 1.5 kΩ × 150 pF = 225 ns, `I(t) = I_pk·e^(−t/τ)`): conductor cross-section `A = √(ρ·τ·I_pk²/(2·C_V·ΔT))` (C_V volumetric specific heat).
- Source: App D Eqs 5.9, 14.6; reftext 50504–50581, 51707–51750; PDF 847, 869.
- Philis stage: dr, verify.
- Automation recipe: nets classified ESD (pad-to-clamp) get a width floor `A/t_metal` per layer from the HBM level (I_pk = V_HBM/1.5 kΩ) and the allowed ΔT (not given; e.g., melting margin set by deck).
- Beats hand layout because: every ESD branch is sized from the same physical rule.
- Philis status: missing (no ESD net class; `ESD_DIODE_CLAMP` pattern exists, backend/annotator/src/catalog.rs:1811-1815).

### H15-47 Fringing capacitance of plates vs perimeter
- Kind: formula
- Statement: `C_F ≈ (εP/π)·[ln(2eP/t) + ½·ln(1 + 4t_e/t)]` for parallel plates of perimeter P, separation t, electrode thickness t_e (disc-derived, crude); `C_F/P ≈ (ε/π)·ln(eP/d)` for a plate at height d above a plane.
- Source: App D Eq 7.6 (7.6.1–7.6.7); reftext 50680–50754; PDF 850.
- Philis stage: cells, verify.
- Automation recipe: capacitor-array unit model `C = C_a·A + C_F(P)`; ratio error between units of different P/A; choose unit shapes with equal perimeter-to-area ratio for ratioed caps; use as a PEX sanity check on generated caps.
- Beats hand layout because: ratio error from fringing is computed per variant rather than eyeballed.
- Philis status: partial — PEX fringe per layer from deck (kernel/analog/src/routing/parasitic.rs:9-12); unit-cap P/A matching in cap_array not checked here.

### H15-48 Resistor segmentation sensitivity
- Kind: formula / metric
- Statement: with process bias `R = αR_0 + β` per segment, ratio error of an N-segment R_N vs M-segment R_M is `(β/α)·(R_M N − R_N M)/(R_N R_M)`; segmentation sensitivity `S = |N/R_N − M/R_M|` (zero when segment counts scale with resistance). With one partial segment each (jR_0, kR_0; 0<j,k<1): `S = |(N+1)/(N+k) − (M+1)/(M+j)|`.
- Source: App D Eqs 8.27, 8.29; reftext 50757–50866; PDF 851–852.
- Philis stage: cells, dp, verify.
- Automation recipe: in `cells` resistor variant enumeration, for every ratioed resistor group compute S for each candidate unit value R_0 and segment split; prefer S = 0 (integer unit multiples); when partial segments are unavoidable pick j,k minimizing S; report S in the bench table.
- Beats hand layout because: exhaustive search over unit sizes for zero segmentation sensitivity.
- Philis status: missing — resistor generator enumerates `feasible_segments` and patterns (kernel/cells/src/resistor.rs:12-41) without an S metric.

### H15-49 Poly piezoresistance coefficients
- Kind: formula / data-model
- Statement: π_L and π_T derived from gauge factors G_L, G_T with E ≈169 GPa and ν ≈0.22 (polysilicon); P-type G_L ≈42; N-type G_T ≈15; the operator form of 8.T7.1/8.T7.2 and the resulting π values are not legible in the source render ("not given").
- Source: App D Table 8.7 derivation; reftext 50869–50906; PDF 853.
- Philis stage: cells, dp.
- Automation recipe: store π_L, π_T per resistor material in the sidecar when the PDK gives them; orient matched poly resistors identically (common-orientation constraint already standard); score stress sensitivity `ΔR/R = π_L σ_L + π_T σ_T` with a die-stress map (H15-12).
- Beats hand layout because: stress sensitivity quantified per orientation.
- Philis status: missing (grep "piezo" empty).

### H15-50 Piezocapacitive coefficient of oxide capacitors
- Kind: formula
- Statement: `ΔC/C = ξ·(σ_x + σ_y)` with `ξ = ((1−ν_Si)/E_Si)·[1 + (1/(1−ν_d))·(2M₁₂E_d/(ε_rε_0) + ν_d)]`; Table 8.45: ⟨110⟩ on (100) Si E = 169 GPa, ν = 0.064; ⟨100⟩ on (100) 130 GPa, 0.279; any on (111) 169 GPa, 0.262; oxide 75 GPa, 0.17; M₁₂(oxide) = −2.1×10⁻²² m²/V², ε_r = 3.9; ξ = 4.6×10⁻¹³ Pa⁻¹ and 5.8×10⁻¹³ Pa⁻¹ (the text labels both "(111) silicon"; which applies to (100) is not given unambiguously).
- Source: App D Eq 8.40 supplementary, Table 8.45; reftext 50910–51046; PDF 854–855.
- Philis stage: dp, verify.
- Automation recipe: stress-gradient cost for matched cap arrays: `ΔC_mismatch/C ≈ ξ·Δ(σ_x+σ_y)` across the array's centroids; with a die-stress field (edge/corner model or FEA), common-centroid residuals get a physical weight.
- Beats hand layout because: gives a physical number to "keep caps away from stress".
- Philis status: missing.

### H15-51 Piezojunction effect for bipolar orientation
- Kind: formula
- Statement: `ΔI_S/I_S = −(B₁σ'₁₁ + B₂σ'₂₂)cos²φ − (B₂σ'₁₁ + B₁σ'₂₂)sin²φ`; (100): `B₁ = (ζ₁₁+ζ₁₂+ζ₄₄)/2`, `B₂ = (ζ₁₁+ζ₁₂−ζ₄₄)/2`; circular lateral device on (100): `−((ζ₁₁+ζ₁₂)/2)(σ'₁₁+σ'₂₂)`; (111): `B₂ = (ζ₁₁+5ζ₁₂−ζ₄₄)/6`, circular: `−((2ζ₁₁+4ζ₁₂+ζ₄₄)/6)(σ'₁₁+σ'₂₂)`. Table 10.T1 (×10⁻¹¹ Pa⁻¹): PNP ζ₁₁ 8.9, ζ₁₂ 14.3, ζ₄₄ 103.5; NPN ζ₁₁ −28.4, ζ₁₂ 43.4, ζ₄₄ 13.1. φ = current direction relative to the wafer-flat axes.
- Source: App D Table 10.3 derivation, Table 10.T1; reftext 51250–51341; PDF 860–861.
- Philis stage: cells, dp, verify.
- Automation recipe: BJT generator and dp keep matched BJTs at equal φ and equal stress; stress sensitivity score for bandgap cores `ΔV_BE ≈ V_T·ΔI_S/I_S`.
- Beats hand layout because: orientation/stress sensitivity computed per device.
- Philis status: missing.

### H15-52 Bipolar emitter-area mismatch and ΔV_BE
- Kind: formula / metric
- Statement: `s_A = k_A√(A_E/2)` (peripheral term k_P = 0); `s_IC2/IC1 ≅ k_A/√A_E`; `ΔV_BE = V_T·ln(A_E1/A_E2) ≈ V_T(A_E1/A_E2 − 1)`; `s_ΔVBE = k_A·V_T/√A_E`. Distribution of ΔV_BE is skewed for normal A_E (not quantified).
- Source: App D Eqs 10.8, 10.9; reftext 51139–51247; PDF 858–859.
- Philis stage: annotator (sizing check), verify (Θ weight).
- Automation recipe: for matched BJT groups, annotator computes σ of the collector-current ratio from k_A (sidecar) and emitter area; the placement budget for gradient residuals is set to a fraction η of this random σ (same scheme as MOS Pelgrom in Philis).
- Beats hand layout because: gradient budgets scaled to the random floor per device.
- Philis status: partial — MOS Pelgrom budgets exist (kernel/analog/src/placement/matching_pair.rs:25; backend/annotator/src/emit.rs:50); BJT k_A path not found.

### H15-53 Emitter-lead debiasing
- Kind: formula
- Statement: a lead collecting total current I_E uniformly along length L (width W, sheet R_S) develops `ΔV_BE = L·R_S·I_E/(2W)` end to end.
- Source: App D Eq 10.2; reftext 51102–51136; PDF 857.
- Philis stage: cells, dr.
- Automation recipe: BJT generator sizes emitter/base finger metal so `ΔV_BE ≤ budget` (e.g., ≤0.1·V_T; budget not given); same formula for MOS source fingers (see H15-57).
- Beats hand layout because: every finger's debiasing computed.
- Philis status: missing (grep "debias" empty).

### H15-54 Distributed contact/ring resistance of discs and annuli
- Kind: formula
- Statement: disc of radius r_A with uniform top injection extracted at the periphery: `R = R_S/(4π)` (independent of radius); annulus with radial current: `V = J·R_S·r₁·ln(r₂/r₁)`, i.e., `R = (R_S/2π)·ln(r₂/r₁)`.
- Source: App D Eqs 11.8, 11.9; reftext 51405–51489; PDF 863–864.
- Philis stage: cells, verify.
- Automation recipe: resistance models for circular/annular devices, guard rings and substrate contacts (tap resistance feeding latchup/debiasing checks, H15-43).
- Beats hand layout because: closed forms for every ring/annular generator output.
- Philis status: missing.

### H15-55 Annular (circular) MOSFET equivalent W/L
- Kind: formula
- Statement: channel between inner diameter A (source) and outer diameter B (drain): `L = (B−A)/2`, `W/L = 2π/ln(B/A)`, `W = π(B−A)/ln(B/A)`; valid in saturation if the pinched-off region is short relative to L.
- Source: App D Eqs 12.26–12.27; reftext 51492–51559; PDF 865.
- Philis stage: cells.
- Automation recipe: if Philis adds an annular/ELT MOS generator (radiation-hard or HV drain), size it from W/L with this mapping and report the LVS W as `π(B−A)/ln(B/A)`.
- Beats hand layout because: exact equivalent width for LVS/SPICE.
- Philis status: missing (no annular MOS in kernel/cells/src/mosfet.rs).

### H15-56 Converting ΔV_t and Δk into offset and current mismatch
- Kind: formula / metric
- Statement: saturated pair at equal I_D: `ΔV_GS = ΔV_t − V_gst1·(√(1+Δk/k₂) − 1)`, first order (binomial `√(1+x) ≈ 1 + x/2`, 13.38.5) → `ΔV_GS ≈ ΔV_t − V_gst1·Δk/(2k₂)` (the printed final 13.38 form is not legible enough to confirm the factor 2; 13.38.4 is exact). At equal V_GS: `I_D2/I_D1 ≅ (k₂/k₁)(1 + 2ΔV_t/V_gst1)` for ΔV_t ≪ V_gst1.
- Source: App D Eqs 13.38, 13.39; reftext 51608–51704; PDF 867–868.
- Philis stage: annotator, verify.
- Automation recipe: when scoring a pair's gradient residual (ΔV_t and Δβ/β from placement), convert to the circuit-relevant error: diff pairs (voltage-matched) → ΔV_GS; mirrors (current-matched) → ΔI/I = Δk/k + 2ΔV_t/V_gst. Use op-point V_gst per device.
- Beats hand layout because: gradient cost weighted by each pair's bias point, not uniformly.
- Philis status: partial — per-pair offset σ and η budgets (backend/annotator/src/emit.rs:50; kernel/analog/src/placement/matching_pair.rs:25); mirror-vs-pair conversion by V_gst not verified here.

### H15-57 Rule of one-third for finger metallization resistance
- Kind: formula
- Statement: distributed model per unit width `R = R_S/W_M`, `G = 1/(W·R_Si)` (R_Si = on-resistance without metal), `γ = √(RG)`; one singly terminated finger: `R_DS(on) = (γ/G)·coth(γW)` ≈ `R_Si + R_S·W/(3W_M)`, so `R_M ≅ R_S·W/(3W_M)`; valid within 1.5% for `W ≤ d_pen = R_Si·W_M/R_S`, 10% for `W ≤ 1.8·d_pen`, else use the coth form. Two fingers (same end or opposite ends): `γ = √(2RG)`, `R_M ≅ 2R_S·W/(3W_M)`, `d_pen = R_Si·W_M/(2R_S)`; opposite-end exact form `R_DS(on) = (γ/2G)[csch(γW) + coth(γW)] + RW/2`, error 1.3% at W = d_pen, 7.7% at 1.8·d_pen. (E.14 prints `γW = R_S W/(R_Si W_M)`, which is `(γW)²` from E.1, E.2, E.7; the "< 1" test is unaffected.)
- Source: App E Eqs E.1–E.34; checklist item 5; reftext 51753–52113, 48887–48893; PDF 870–876, 823.
- Philis stage: cells, dr, verify.
- Automation recipe: in the mosfet generator, choose finger metal width W_M so `R_M ≤ ε·R_Si` (e.g. ε from the device's gm·R budget), and pick termination style (same vs opposite ends) — both give 2/3, but opposite ends balance current along the finger; IR/common-node rules add `R_S·W/(3W_M)` per terminal instead of treating a finger as a point.
- Beats hand layout because: closed-form metal resistance per finger for every variant, fed to scoring.
- Philis status: missing (grep "one.third" empty; terminal resistance via `Stack::terminal_resistance_ohm`, kernel/analog/src/routing/common_node.rs:17-18, treats pins in parallel).

### H15-58 Drain-current temperature coefficient for thermal-mismatch weighting
- Kind: formula
- Statement: with `V_t(T) = V_t(T_nom)[1 + TC_Vt(T−T_nom)]`, `k(T) = k(T_nom)[1 + TC_k(T−T_nom)]`: `TC_ID = k(V_GS−V_t)·[TC_k(V_GS−V_t)/2 − TC_Vt]`; TC_ID < 0 when `TC_Vt < TC_k·(V_GS−V_t)/(2V_t)` (as printed). Near the zero-TC bias a pair is insensitive to ΔT.
- Source: App D Eq 13.24; reftext 51563–51605; PDF 866.
- Philis stage: annotator, dp.
- Automation recipe: ThermalGradient budget per pair = allowed ΔI/I ÷ |TC_ID/I_D| (from op-point V_gs, V_t and model TCs); pairs biased near zero TC get looser ΔT budgets.
- Beats hand layout because: ΔT limits scaled by each pair's actual temperature sensitivity.
- Philis status: partial — fixed `max_delta_mc` per pair (kernel/analog/src/placement/thermal.rs:7-24); not derived from TC_ID.

### H15-59 Selected device formulas without direct P&R use (recorded for completeness)
- Kind: formula
- Statement: `V_CE(sat) = V_T·ln[(β_F/β_R)(β_R + β_force + 1)/(β_F − β_force)]` (Eq 9.14); thermalization distance `x_t ≤ (3µ/q)√(3kT·m_eff)` ≤140 nm (electrons, µ = 1400 cm²/V·s, m_eff = 0.26m₀), ≤60 nm (holes, 470, 0.39m₀) (§11.1.2); Eq 5.32 (high-low junction recombination) not legible.
- Source: App D; reftext 51050–51099, 51345–51402, 50583–50677; PDF 856, 862, 847–848.
- Philis stage: none (cells only if BJT switches or hot-carrier spacing are generated).
- Automation recipe: none now.
- Beats hand layout because: n/a.
- Philis status: n/a.

### H15-60 Substrate-contact fill of empty area
- Kind: heuristic
- Statement: on P+ substrate dice, fill unused area with substrate contacts; in logic-heavy designs trade against bypass capacitance.
- Source: §15.5 item 15; reftext 48954–48958; PDF 824.
- Philis stage: flow (post-route fill).
- Automation recipe: after metal fill, fill empty active-free regions with tap arrays tied to the substrate net (or decap tied to supplies where supply IR/decap budget is short), respecting matched-cell keep-outs.
- Beats hand layout because: maximal fill with electrical accounting.
- Philis status: missing — post-route metal fill only (frontend/library/src/fill.rs:1-18).

### H15-61 Dummy-metal block over matched devices
- Kind: rule / check
- Statement: CMP dummy metal can disturb matching (notably MOS); cover unprotected matched devices with dummy-metal block layers; dummy-metal blocks also obstruct maze routing.
- Source: §15.5 item 9; §15.1.3; reftext 48919–48924, 46829–46831; PDF 824, 789.
- Philis stage: flow, verify.
- Automation recipe: fill excludes matched cells; emit the deck's fill-block layer over them so foundry fill honors it.
- Beats hand layout because: automatic, complete.
- Philis status: implemented for Philis' own fill — matched cells excluded entirely (frontend/library/src/fill.rs:17-18, 90); emitting a foundry fill-block layer not found.

---

## 4. Top-15 priorities for Philis

1. **H15-30 + H15-29** — add inter-layer (overlap + fringe) coupling to `CouplingBudget` and derive per-victim budgets from `ΔV = C·R·dV/dt` with op-point node impedance and aggressor slew; today only same-layer coupling exists, so a clock crossing a reference on the next layer is invisible to Θ.
2. **H15-32** — shield plates at unavoidable noisy/sensitive crossings, with the reference chosen per victim (its own reference net), not a global ground; extend `Shield` beyond same-layer side leads.
3. **H15-33** — enforce star topology (no foreign current through matched members' return branches) on top of `CommonNode` balance; this is the offset mechanism of Fig 15.26.
4. **H15-34** — Kelvin force/sense detection and a tap-point routing constraint with geometric verification; completely missing and a classic hand-layout error (checklist 12).
5. **H15-36** — voltage-aware spacing from net voltage ranges (pairwise ΔV), plus deck grammar; decks currently drop hv.* rules, so HV analog designs are unchecked.
6. **H15-12 + H15-13** — stress keep-outs: matched devices away from die edges/corners, under-pad areas and thick-metal edges; add a stress map that later feeds H15-49/50/51.
7. **H15-38** — populate EM derating data (T_ref, E_a, n) in decks and feed per-conductor temperature; the code exists but is inert.
8. **H15-39** — EM current crowding: chamfer high-current 90° bends and move via arrays off corners onto straight segments.
9. **H15-19 + H15-26** — minimum two cuts per layer change where space allows, and fill the full wide-lead overlap with the maximum via array.
10. **H15-57** — rule-of-one-third finger metal resistance in the MOS generator and in IR/common-node scoring (fingers are not points).
11. **H15-48** — segmentation sensitivity `S` in resistor variant selection (prefer integer unit multiples; minimize S with partial segments).
12. **H15-28 + H15-27 + H15-06** — broaden net classification (eight sensitive categories, HV, high-current, static digital, name suffixes); every downstream rule keys off classes.
13. **H15-09 + H15-20** — per-layer pitch and routing-demand-aware placement (channel capacity from `W_C = N·P_R + S_m`) to kill choke points before routing.
14. **H15-56 + H15-58** — weight gradient and thermal residuals by each pair's bias (ΔV_GS vs ΔI/I conversion; TC_ID-scaled ΔT budgets) so Θ reflects circuit error, not geometry.
15. **H15-43** — pin-reachability latchup/CDM check (diffusions within <50 kΩ of a pin need the right guard ring; clamps near pin-connected gates).
