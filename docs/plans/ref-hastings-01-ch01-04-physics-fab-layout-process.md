# Hastings, *The Art of Analog Layout* 3e — Ch. 1–4 (device physics, fabrication, layout & design rules, representative processes): what an automated analog P&R must apply

- Source: A. Hastings, *The Art of Analog Layout*, 3rd ed., Pearson 2024 (ISBN 978-0-13-803839-7).
- Reftext: `/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/hastings.txt`, lines **1–11865** (front matter, TOC, preface, acknowledgments, chapters 1–4 up to the `Chapter 5` header at L11865). PDF pages 1–211.
- Many numbers are typeset as images and are blank in the reftext. Where a number mattered I opened the PDF page (Read tool) and give it with "(PDF p.N)". A value that is blank in the text and was not checked on the PDF is marked "not given".
- Philis status checks are quick greps against the working tree (file:line) and against the GPurify deck pinned by `Cargo.lock:678` (rev `8df8c09`, `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/pdks/sky130.deck`, cited as `sky130.deck:N`).
- Prefix: **H01-NN**.

## 1. Coverage

Consecutive Read chunks of the reftext (every line read; a 2001–4000 request exceeded the tool's token cap and was re-read in ≤ 1200-line chunks; the last chunk ran past the range into the start of Ch.5 at L11900 only to confirm the boundary):
`1–2000`, `2001–3200`, `3201–4400`, `4401–5600`, `5601–6800`, `6801–7900`, `7901–9000`, `9001–10100`, `10101–11200`, `11201–11900`.

PDF pages opened to recover values typeset as images (PDF page = 1 + number of form feeds before the reftext line; in this range PDF page N carries printed page N−1, e.g. PDF p.140 = printed p.139): 41–43, 46, 82, 85, 92–94, 103, 124, 138–143, 157, 171, 172, 177–179, 181–185, 187–189, 195, 200–207. Every number below that is not legible in the reftext was read on one of these pages; anything else blank in the reftext is marked "not given".

Headings in range (reftext line):
- Front matter L1–114; Contents L115–1340; Preface to the Third Edition L1346; Resources for Instructors L1453; Acknowledgments L1473.
- **Ch.1 Device Physics** L1488: 1.1 Semiconductors L1513; 1.1.1 Generation and Recombination L1648; 1.1.2 Extrinsic Semiconductors L1851; 1.1.3 Diffusion and Drift L2025; 1.2 PN Junctions L2136; 1.2.1 Depletion Regions L2224; 1.2.2 PN Diodes L2285; 1.2.3 Zener Diodes L2479; 1.2.4 Schottky Diodes L2524; 1.2.5 Ohmic Contacts L2649; 1.3 Bipolar Transistors L2699; 1.3.1 Beta L2852; 1.3.2 I-V Characteristics L2917; 1.4 MOS Transistors L2975; 1.4.1 Threshold Voltage L3238; 1.4.2 I-V Characteristics L3370; 1.5 JFET Transistors L3439; 1.6 Summary L3564 (Selected Bibliography L3604); 1.7 Exercises L3650.
- **Ch.2 Semiconductor Fabrication** L3720: 2.1 Silicon Manufacture L3748; 2.1.1 Crystal Growth L3769; 2.1.2 Wafer Manufacture L3818; 2.1.3 The Crystal Structure of Silicon L3844; 2.2 Patterning L3931; 2.2.1 Photoresists L3940; 2.2.2 Exposure L3977; 2.2.3 Development L4034; 2.3 Oxide Growth and Removal L4063; 2.3.1 Oxide Growth and Deposition L4073; 2.3.2 Oxide Removal L4210; 2.3.3 Other Effects of Oxide Growth and Removal L4304; 2.3.4 LOCOS L4401; 2.4 Diffusion and Ion Implantation L4490; 2.4.1 Diffusion L4541; 2.4.2 Other Effects of Diffusion L4649; 2.4.3 Ion Implantation L4753; 2.5 Silicon Deposition and Etching L4936; 2.5.1 Epitaxy L4953; 2.5.2 Poly Deposition L5081; 2.5.3 Silicon etching L5115; 2.6 Isolation L5173; 2.6.1 Junction Isolation L5183; 2.6.2 Dielectric Isolation L5256; 2.6.3 Wafer Bonding L5349; 2.7 Interconnection L5413; 2.7.1 Aluminum L5461; 2.7.2 Refractory Barrier Metal L5527; 2.7.3 Silicidation L5597; 2.7.4 Tungsten Plugs L5684; 2.7.5 Dielectrics L5729; 2.7.6 Copper L5859; 2.8 Assembly L5999; 2.8.1 Mount and Bond L6060; 2.8.2 Packaging L6181; 2.9 Summary L6278; 2.10 Exercises L6326.
- **Ch.3 Layout** L6402: 3.1 Layout Editors L6511; 3.1.1 Coordinates L6519; 3.1.2 The Grid L6555; 3.1.3 Shapes L6572 (Rectangles L6592, Polygons L6616, Circles L6668, Paths L6688); 3.1.4 Hierarchy L6765 (Transformations L6798, Pins L6840, Parameterized Cells L6889); 3.1.5 Interchange File Formats L6966 (CIF L6982, GDSII L7016, OASIS L7077); 3.2 Design Rules L7117; 3.2.1 Geometric Operations L7152 (OR L7159, AND L7192, NOT L7224, ANDNOT L7260, XOR L7291, Size Adjust L7321); 3.2.2 Rule Checks L7363 (Width L7384, Spacing L7443, Overlap L7510, Overhang L7553); 3.2.3 Design Rule Construction L7587 (Minimum Feature Size L7596, Process Bias L7652, Linewidth Control L7680, Mask Alignment L7701, Outdiffusion L7741, Depletion Region Width L7764, Example L7824, Scalable Design Rules L7991); 3.3 Pattern Generation L8067; 3.3.1 Optical Pattern Generation L8083; 3.3.2 Advances in Photolithography L8233; 3.3.3 MEBES L8314; 3.3.4 Optical Proximity Correction L8363; 3.4 Summary L8422; 3.5 Exercises L8472.
- **Ch.4 Representative Processes** L8545: 4.1 Standard Bipolar L8584; 4.1.1 Essential Features L8607; 4.1.2 Fabrication Sequence L8646 (Starting Material L8659, N-Buried Layer L8669, Epitaxial Deposition L8693, Isolation Diffusion L8726, Deep-N+ Sinker L8749, Base Implant L8790, Emitter Diffusion L8818, Contact OR L8857, Metallization L8864, Protective Overcoat L8872); 4.1.3 Available Devices L8899 (NPN L8924, PNP L9031, Resistors L9188, Capacitors L9309); 4.1.4 Process Extensions L9336 (Up-Down Isolation L9343, Double-Level Metal L9377, High-Sheet Resistors L9393); 4.2 Poly-Gate CMOS L9438; 4.2.1 Essential Features L9492; 4.2.2 Fabrication Sequence L9550 (Starting Material L9560, Epitaxial Growth L9569, N-Well Diffusion L9590, Inverse Moat L9627, Channel Stop Implants L9661, LOCOS Processing and Dummy Gate Oxidation L9696, Threshold Adjust L9723, Polysilicon Deposition and Patterning L9764, Source/Drain Implants L9799, Contacts L9845, Metallization L9865, Protective Overcoat L9884); 4.2.3 Available Devices L9912 (NMOS L9920, PMOS L10090, Substrate PNP L10131, Resistors L10169, Capacitors L10261); 4.2.4 Process Extensions L10291 (LDD Transistors L10299, High-Voltage Drain-Extended Transistors L10368); 4.3 Analog BiCMOS L10412; 4.3.1 Essential Features L10440; 4.3.2 Fabrication Sequence L10522 (Starting Material L10542, Etched Alignment Marker L10552, N-Buried Layer L10560, Epitaxial Growth L10567, N-Well Implant L10603, P-Well Implant L10616, Deep-N+ Sinker L10643, Base Implant L10678, Inverse Moat L10706, LOCOS Processing and Dummy Gate Oxidation L10747, Gate Oxidation L10754, Polysilicon Deposition, Doping, and Patterning L10762, Source/Drain Implants L10800, Silicidation L10848, Contacts L10871, Metallization L10882); 4.3.3 Available Devices L10919 (NMOS L10927, PMOS L11027, DMOS L11074, NPN L11167, PNP L11254, Resistors L11299, Capacitors L11370); 4.3.4 Process Extensions L11382 (3.3 V CMOS Transistors L11402, Dielectric Isolation L11524); 4.4 Summary L11728; 4.5 Exercises L11743.

## 2. Section-by-section digest

### Front matter, Contents, Preface, Acknowledgments (L1–1487)
- TOC (L115–1340) maps the rest of the book; Appendix C "Sample Layout Rules" (L1333) is the rule set chapter-4 exercises use (L11744).
- Preface: STI has "largely supplanted LOCOS", DRC rulesets "now contain thousands of rules", OPC "more complex" (L1351–1353).
- Preface: the new Ch.3 covers editors, interchange formats, mask alignment and scalable rules (L1366–1369); Ch.8 adds WPE, centroid equations for arbitrary arrays, a quantified rule of dispersion, "segmentation sensitivity", piezoresistive coefficients (L1393–1399); Ch.13 adds WPE, length-of-diffusion, and a quantitative orientation measure replacing "chirality" (L1429–1431). These are later-chapter topics; they are out of this document's range.
- Acknowledgments (L1473–1482): no technical content.

### Ch.1 intro (L1488–1507)
- Motivation only: solid-state devices are doped crystals; Ch.2 covers manufacture (L1503–1507).

### 1.1 Semiconductors (L1513–1646)
- Metallic, ionic, covalent bonding; silicon forms four covalent bonds (L1546–1610).
- Table 1.1: Si bandgap 1.1 eV, Ge 0.7 eV, diamond 5.2 eV (L1687–1712).

### 1.1.1 Generation and Recombination (L1648–1849)
- Holes are mobile vacancies; hole mobility < electron mobility, so electron-conduction devices switch faster (L1757–1760).
- Light < 1100 nm generates carriers; camera flashes have upset chip-scale-packaged ICs (L1783–1793).
- Indirect-bandgap Si recombines via SRH traps; gold/Fe/Ni are recombination centres; lifetimes ns–hundreds of µs (L1823–1847).

### 1.1.2 Extrinsic Semiconductors (L1851–2021)
- Donors P, As, Sb; acceptor B (Table 1.2, L1969–1982). Majority/minority carriers defined (L1910–1916).
- Counterdoping inverts type; modern devices are nested counterdoped regions (L1988–1994).
- Compound semiconductors; only Ge-doped Si is used in volume ICs (L2017–2021).

### 1.1.3 Diffusion and Drift (L2025–2130)
- Diffusion from concentration gradients; drift under field (L2058–2094).
- Ohm's law holds to about 5 kV/cm; above it hot carriers and velocity saturation make resistors non-ohmic (L2103–2114).
- Mobility falls with temperature, doping, and near the surface (L2118–2124).

### 1.2 PN Junctions (L2136–2222)
- Junction equilibrium: diffusion balanced by drift across the depletion field; built-in potential (value not given in text) (L2183–2215).

### 1.2.1 Depletion Regions (L2224–2281)
- Depletion extends almost entirely into the lightly doped side (100× further for a 100× doping ratio) (L2254–2260). Spacing margins belong on the lightly doped side.

### 1.2.2 PN Diodes (L2285–2475)
- Contact potentials cancel only at uniform temperature; a gradient produces a Seebeck voltage (L2305–2315).
- Leakage doubles about every 8 °C; ICs are designed for Tj max 125–150 °C (L2379–2383; PDF p.41).
- IC diodes at µA drop 0.6–0.65 V at 25 °C; V_F falls ≈ 2 mV/°C (0.65 V at 25 °C → 0.78 V at −40 °C, 0.45 V at 125 °C) (L2461–2475; PDF p.43).

### 1.2.3 Zener Diodes (L2479–2520)
- Avalanche critical field ≈ 200 kV/cm at 1e15 cm⁻³ to ≈ 2 MV/cm at 1e18 cm⁻³; heavier doping → lower BV (L2488–2501; PDF p.43).
- Avalanche BV rises with T (7 V junction: about +4 mV/°C) (L2502–2503; PDF p.43); Zener (tunnelling) dominates below ≈ 5 V (L2506–2520).

### 1.2.4 Schottky Diodes (L2524–2645)
- Metal/lightly doped N rectifies; PtSi or PdSi anodes in ICs (L2576–2580). Majority-carrier device, faster than PN (L2610–2615).

### 1.2.5 Ohmic Contacts (L2649–2693)
- Ohmic contact to lightly doped silicon needs a heavily doped same-type region under the contact (L2673–2678); contact resistance < 50 Ω·µm² possible (PDF p.46).
- Seebeck coefficient of a contact 0.1–1.0 mV/°C; circuits matched to a mV degrade under self-heating gradients (L2681–2688; PDF p.46).

### 1.3 Bipolar Transistors (L2699–2850)
- BJT = two close junctions; E and C doped differently, swapping them degrades performance (L2742–2749).
- Four regions (Table 1.3, L2760–2781).

### 1.3.1 Beta (L2852–2913)
- Integrated NPN β ≈ 150; CMOS-process NPN may be < 10 (L2853–2857). β ∝ 1/Gummel number (L2861–2869).
- Emitter injection efficiency > 0.995; reverse β < 5 for forward β 150 (L2884–2895). β falls at low current (depletion recombination) and high current (high-level injection) (L2899–2904).

### 1.3.2 I-V Characteristics (L2917–2964)
- Early effect; avalanche limits V_CE; process designers derate; punchthrough (L2938–2959).

### 1.4 MOS Transistors (L2975–3207)
- MOS capacitor: accumulation, depletion, inversion; threshold (L2997–3081).
- Source/drain identity depends on bias: the more positive terminal of an NMOS is the drain; identity can swap as voltages move (L3105–3107).
- Linear, saturation, channel-length modulation, subthreshold; subthreshold current negligible 200–300 mV below V_t (L3162–3201).

### 1.4.1 Threshold Voltage (L3238–3366)
- Enhancement vs depletion devices (Table 1.6, L3239–3281).
- Asymmetric transistors (e.g. lightly doped drain): swapping S and D lowers the voltage rating and can be catastrophic (L3312–3321).
- Backgate modulation (body effect) shifts V_t (L3329–3336); backgate doping and gate material (N+ vs P+ poly ≈ 1 V shift) set V_t (L3340–3346); oxides < 9 nm tunnel; high-k (L3349–3355); interface charge (L3361–3366).

### 1.4.2 I-V Characteristics (L3370–3433)
- Long-channel devices break down by avalanche and snap back through the parasitic S–B–D bipolar (L3397–3404); short-channel devices punch through without snapback (L3407–3410).
- PMOS has somewhat less than half the NMOS transconductance (L3430–3433).

### 1.5 JFET Transistors (L3439–3558)
- Junction-gated depletion device; pinchoff voltage; symmetric vs asymmetric JFETs (L3440–3532).

### 1.6 Summary and bibliography (L3564–3643)
- Recap of junction, BJT, MOS and JFET operation (L3571–3598). Bibliography only after L3604.

### 1.7 Exercises (L3650–3714)
- Leakage vs temperature (1.5), V_F vs temperature (1.15), S/D swap breakdown (1.9), junction capacitance vs voltage (1.18) (L3659–3694).

### Ch.2 intro (L3720–3742)
- History; Ch.3 covers mask-data generation, Ch.4 three representative flows (L3740–3742).

### 2.1 Silicon Manufacture (L3748–3765)
- Metallurgical → semiconductor-grade polysilicon → single crystal (L3753–3765).

### 2.1.1 Crystal Growth (L3769–3814)
- Czochralski; oxygen precipitates getter heavy metals (L3803–3809).

### 2.1.2 Wafer Manufacture (L3818–3840)
- Flats/notches identify crystal orientation; rough backside getters (L3826–3840).

### 2.1.3 The Crystal Structure of Silicon (L3844–3925)
- (100) has the lowest surface-state charge → MOS; (111) used for bipolar and suppresses parasitic PMOS (L3897–3902).

### 2.2 Patterning / 2.2.1 Photoresists (L3931–3973)
- Photolithography patterns blanket processes; positive resists dominate because negative resists swell (L3935–3973).

### 2.2.2 Exposure (L3977–4030)
- Contact → projection → steppers with 2X/5X/10X reticles (L4010–4021). Analog fabs mostly I-line (365 nm); 248/193 nm plus immersion for < 50 nm (L4025–4030).

### 2.2.3 Development (L4034–4057)
- Windows; oxide and nitride as high-temperature masks (L4052–4057).

### 2.3 Oxide Growth and Removal / 2.3.1 (L4063–4197)
- Dry vs wet oxide; Table 2.1 times; (111) oxidizes faster (L4076–4130). PSG/BPSG reflow (L4173–4180).
- Modern dice look drab because of planarization and "dense patterns of dummy metal figures" (L4194–4197).

### 2.3.2 Oxide Removal (L4210–4300)
- Wet etch isotropic with undercut; RIE anisotropic gives linewidth control (L4231–4270).

### 2.3.3 Other Effects of Oxide Growth and Removal (L4304–4378)
- Oxide steps: silicon consumed ≈ 45 % of grown oxide thickness (L4314–4321).
- Segregation: boron suckup, phosphorus plow (L4342–4351). Dopant-enhanced oxidation, e.g. thick field over N+ sinkers (L4373–4378).

### 2.3.4 LOCOS (L4401–4484)
- Pad oxide + nitride mask; lateral oxidant diffusion forms the bird's beak (L4402–4411). Field vs moat regions (L4437–4439).
- Kooi effect: nitride residue under the bird's beak thins gate oxide at moat edges; dummy (sacrificial) gate oxidation fixes it (L4443–4451).
- LOCOS isolates MOS devices > ≈ 0.6 µm; fully recessed LOCOS ≈ 0.4 µm; smaller needs STI (L4480–4484; PDF p.82).

### 2.4 Diffusion and Ion Implantation (L4490–4531)
- Planar process: junctions terminate under oxide (passivated) (L4502–4509).

### 2.4.1 Diffusion (L4541–4645)
- Predeposition + drive; B and P fast, As and Sb slow (Table 2.2) (L4554–4598). POCl₃ still used for deep N+ sinkers (L4640–4645).

### 2.4.2 Other Effects of Diffusion (L4649–4745)
- Practical junction depth limit ≈ 15 µm (PDF p.85). Lateral outdiffusion ≈ 80 % of junction depth (L4661–4667).
- Boron suckup can invert a lightly counterdoped surface (L4694–4696). Emitter push, NBL push, oxidation-enhanced diffusion (deeper junctions under LOCOS field than under moat) (L4699–4732). Profiles are not fully predictable (L4744–4749).

### 2.4.3 Ion Implantation (L4753–4930)
- Dose ∝ beam current × time → better control (L4803–4805); range, straggle, retrograde and chained implants (L4808–4820).
- Poly-masked implants self-align S/D (L4867–4875).
- Implants are tilted to avoid channeling; tilted implants restricted to one direction produce two transistor classes (horizontal vs vertical) with different characteristics (L4899–4911).

### 2.5 Silicon Deposition and Etching (L4936–4949)
- Mono vs poly deposition; isotropic vs anisotropic etch (L4937–4949).

### 2.5.1 Epitaxy (L4953–5071)
- CMOS: lightly doped P epi on heavily doped P substrate (L5005–5009). NBL of As or Sb; Sb for less lateral autodoping (L5012–5019).
- NBL shadow and pattern shift; CMP after STI erases the shadow, so STI processes use etched alignment markers (L5048–5071).

### 2.5.2 Poly Deposition (L5081–5111)
- Grains 0.03–0.3 µm (PDF p.92); grain boundaries leak, so PN junctions are not normally made in poly (L5083–5086).
- Lightly doped poly resistors reach tens of MΩ; diffused resistors a few hundred kΩ (L5089–5093).

### 2.5.3 Silicon etching (L5115–5163)
- RIE STI allows MOS widths < 0.1 µm (PDF p.93); deep trenches with aspect > 10 for DI and TSVs (L5128–5142).
- V-grooves 35.26° from vertical; 270° corners facet on {311}, fixed by small mask protrusions (L5157–5161; PDF p.93).

### 2.6 Isolation (L5173–5179)
- Junction isolation (cheap) vs dielectric isolation (niche: rad-hard, HV) (L5174–5179).

### 2.6.1 Junction Isolation (L5183–5251)
- All isolation junctions must stay reverse-biased: P-substrate below the lowest voltage on any isolated N region (L5202–5207).
- A forward-biased isolation junction injects minority carriers that reach other devices → parametric glitches or destruction (latchup) (L5211–5215).
- Wells: a 40 V process needs a 6–10 µm well so depletion stays out of the top 2–3 µm (L5218–5225; PDF p.94).
- Well (counterdoped, graded) vs tank (epi, uniform) (L5241–5251).

### 2.6.2 Dielectric Isolation (L5256–5343)
- SOS, shape-back, SIMOX: islands on insulator suffer no substrate injection (L5264–5325).

### 2.6.3 Wafer Bonding (L5349–5407)
- Bond + thin/cleave; DI returning for HV ICs and fully depleted MOS (L5350–5407).

### 2.7 Interconnection (L5413–5455)
- FEOL/BEOL (L5414–5418). Analog processes: 3–4 metals, the top often thick for current (L5444–5448).
- Poly is a "free" routing layer, but its resistance forbids routing current-carrying signals through significant lengths (L5451–5455).

### 2.7.1 Aluminum (L5461–5523)
- Al alloys into P silicon; Ohmic to N needs heavy doping (L5486–5492). Contact spiking; Al-Si (L5496–5505).
- Electromigration near 1e6 A/cm²; a fraction of a percent of Cu improves EM resistance ×10 (L5513–5523).

### 2.7.2 Refractory Barrier Metal (L5527–5593)
- TiW barrier stops erosion; step coverage at contacts and vias; RBM thinning at sidewalls is not an EM hazard (L5527–5593).

### 2.7.3 Silicidation (L5597–5680)
- Clad poly and clad S/D; salicide self-aligns (L5601–5635).
- Silicided processes need a silicide-block mask for poly resistors above a few kΩ (L5614–5617).
- TiSi₂ resistivity rises in narrow lines (C49→C54 needs large grains); Ni and Co silicides do not (L5646–5656).
- Silicided poly helps digital density; "most automated routers are not optimized to properly employ poly routing" (L5671–5680).

### 2.7.4 Tungsten Plugs (L5684–5725)
- CVD W fills sub-µm cuts; metal over a plug needs overlap on only two sides; contacts and vias can stack; plugs forbid arbitrary cut sizes/shapes (L5720–5725).

### 2.7.5 Dielectrics (L5729–5857)
- DLM stack: MLO, ILO, six masks (poly, contact, M1, via, M2, POR) (L5734–5748).
- SOG and resist etch-back; CMP; dishing over "hundreds of microns" of recessed area; cure = dummy geometries "automatically generated" (L5770–5792).
- Modern stack: STI, TiSi₂, W plugs, TiN ARC; 7th mask = silicide block for unsilicided resistors (L5809–5825). Compressive nitride PO (L5840–5848).

### 2.7.6 Copper (L5859–5993)
- Cu: 35 % less resistance than Al and an order of magnitude better EM (L5865–5866); TaN barrier; single/dual damascene (L5869–5937).
- Power copper and bond-over-active-circuitry; CTE stress grows with Cu thickness and die size (L5941–5978). Cu pillars (L5982–5989).

### 2.8 Assembly (L5999–6056)
- Probing on pads through PO openings; backgrinding; scribe streets hold test structures but limited metal/oxide (L6026–6056).

### 2.8.1 Mount and Bond (L6060–6177)
- Cu leadframe CTE mismatch stresses the die; Alloy 42 for low stress (L6064–6069).
- Conductive epoxy "cannot be trusted to provide low-resistance electrical connectivity" to the backside (L6090–6092).
- Ball-bond pads ≈ 2–3× wire diameter; one-mil wire carries a large fraction of an amp; bond placement rules are verified by tools (L6170–6177).

### 2.8.2 Packaging (L6181–6268)
- Mold compound ≈ 90 % silica to lower CTE and stress (L6241–6246); RoHS; final test (L6252–6268).

### 2.9 Summary / 2.10 Exercises (L6278–6396)
- Photolithographic mass patterning is the key technology (L6293–6296). Exercises: outdiffusion width (2.7), silicide choice vs linewidth (2.13) (L6352–6379).

### Ch.3 intro (L6402–6505)
- Rubylith → digitizers → Calma GDS/GDSII → MEBES; editors (Virtuoso, IC Station) mature since the 1980s; tools added autoplace/route, verification, extraction (L6415–6505).

### 3.1 Layout Editors / 3.1.1 Coordinates (L6511–6551)
- Right-handed Cartesian, user units; integer storage via DBU/UU (L6520–6537).
- Converting DBU/UU is safe only to an integer multiple of the old ratio; mil→µm (×25.4) rounds (L6540–6545). Tech file owns units (L6549–6551).

### 3.1.2 The Grid (L6555–6568)
- Grid increment must be an integer multiple of the coding increment; layout rules expect every coordinate on the minimum grid (L6565–6568).

### 3.1.3 Shapes (L6572–6761)
- Layer numbers vs names (L6581–6588). Rectangles (L6592–6612).
- Polygons: Manhattan preferred in low-voltage CMOS; higher-voltage processes use non-orthogonal shapes to avoid field-intensifying corners (L6635–6639); never nonsimple polygons (L6643–6647); acute angles cause DRC errors and decomposition failures (L6662–6664).
- Circles: side count a multiple of 4 (32 or 64) for H/V symmetry; vertices fall off grid; matched circles should be on-grid polygons (L6668–6684).
- Paths: width an integer multiple of twice the coding increment (example: 0.05 µm rules with 0.35 µm metal → 0.025 µm increment; PDF p.124) (L6720–6726); non-orthogonal paths lose width in rounding (L6728–6732); flush/extended/rounded ends; zero-width paths (L6740–6761).

### 3.1.4 Hierarchy (L6765–6962)
- Cells, instances; editing a shared cell edits every instance (copy before editing) (L6771–6787).
- Eight orthogonal transforms R0/R90/R180/R270/MX/MY/MX90/MY90; magnification and any-angle rotation cause rounding and are usually forbidden (L6798–6823). Arrays rotate about the lower-left instance (L6827–6836).
- Pins = shape + layer + name (+direction) (L6880–6885).
- Pcells: stretch/include/repeat/modify/border-repeat/reference-point (L6902–6910); a properly designed Pcell always passes DRC (L6919–6922); exploded Pcells lose linkage; upgrade translators force a schematic-vs-layout choice (L6937–6950); use custom layout only when the Pcell would significantly degrade performance (L6956–6962).

### 3.1.5 Interchange File Formats (L6966–7111)
- CIF: 100 DBU/UU, named layers, nonsimple allowed (L6994–7012).
- GDSII: 32-bit coordinates, layers/datatypes 0–63 (commonly 0–255; Cadence 0–32767), boundaries ≤ 200 vertices (Cadence 8000), path types 0–3 (0 and 2 universally recognized), structure names ≤ 32 chars [A-Za-z0-9_?$] (avoid ? and $), any-angle/magnification discouraged (L7036–7073).
- OASIS: delta encoding, repetitions, no layer names (L7078–7111).

### 3.2 Design Rules (L7117–7148)
- Rule deck; DRC loads, derives layers, checks, emits diagnostics (L7135–7140). Flat vs hierarchical verification (L7144–7148).

### 3.2.1 Geometric Operations (L7152–7359)
- OR (union), AND (distributes over OR), NOT (dark field), ANDNOT (A−B), XOR (L7159–7304).
- Size adjust: oversize/undersize move every edge; sequences are not invertible (Fig. 3.14: oversize then undersize fills a notch; undersize then oversize deletes a narrow arm) (L7321–7359).

### 3.2.2 Rule Checks (L7363–7568)
- Merge touching/overlapping shapes (across hierarchy) before checking (L7376–7380).
- Width: perpendicular and vertex-to-vertex; arcs give false errors; diagonal paths lose up to 1.4 dbu, DRC relaxes 2 dbu; `exact` = only a square of that size passes (L7384–7439).
- Spacing: between layers or within one; touching violates unless `overlap okay`; intra- vs inter-figure (L7443–7506).
- Overlap (= enclosure): unenclosed inner shape errors unless `overhang okay`; naked cuts need `CONT must touch METAL`; `exact` for fixed devices (L7510–7549). Overhang (= extension) (L7553–7568).

### 3.2.3 Design Rule Construction (L7587–7990)
- Abbe estimate: min feature ≈ λ/(2·NA); 365 nm, NA 0.6 → 0.3 µm; a wise process uses 0.5 µm (PDF p.138) (L7596–7606). CD monitor figures (L7630–7632).
- Process bias ≤ 10 % of film thickness; corrected by process size adjusts in PG (e.g. +0.05 µm per side); bias is geometry dependent (L7652–7676; PDF p.139).
- Linewidth control ≈ 10 % of min feature; adjacent identical devices match "far better than linewidth control suggests" (L7680–7697).
- Mask alignment ≈ 20 % of min feature; two alignment errors ≈ 160 % of one (L7701–7737).
- Outdiffusion 80 % of x_j (5 µm well spreads 4 µm; 10 µm drawn → 18 µm; PDF p.140); dilution below ≈ 2·x_j; keep min width ≥ x_j (L7741–7754).
- Table 3.1 depletion widths (background, x_j, V): e.g. 1e15/1 µm → 3/4/5/8 µm at 5/10/20/40 V; 1e16/1 µm → 1.3/1.5/2 µm at 5/10/20 V; add ≥ 50 % margin (L7764–7820; PDF p.140).
- Worked rule derivation (0.8 µm stepper, 0.1 µm linewidth control, 0.2 µm alignment; PDF pp.141–143): NDIF width 2.0; NDIF space 5.8 µm @10 V, 7.8 µm @20 V; CONT 1.0 exact; CONT space 1.0; NDIF ovl CONT 0.4; METAL w/s 1.8; METAL ovl CONT 0.4; POR width 10.0; POR space 4.2; METAL ovl POR 1.4 (L7824–7986).
- Scalable λ rules; optical shrinks; analog does not scale; drawn vs silicon µm confusion caused ≈ 20 % capacitance errors; modern rules are silicon µm (L7991–8061).

### 3.3 Pattern Generation / 3.3.1 (L8067–8224)
- PG deck combines coding (drawn) layers into generated mask layers; pseudolayers carry DRC information without affecting masks (L8110–8128).
- Process size adjusts, dark/clear field choice (Table 3.3), reticle scale and address unit (L8134–8210).
- Rectangular decomposition cannot fill acute interior angles; even chip art must obey design rules (L8214–8224).

### 3.3.2 Advances in Photolithography (L8233–8310)
- Process-control plugs → scribe-street test structures; composed reticles and MPWs; scanners (L8241–8310).

### 3.3.3 MEBES (L8314–8359)
- Raster e-beam writers; trapezoid decomposition fills acute angles (L8336–8346).

### 3.3.4 Optical Proximity Correction (L8363–8416)
- Dog-ear corner OPC; rule-based OPC depends on neighbour spacing and corner distance (L8370–8403).
- Analog: mild OPC on gate and possibly moat; OPC on damascene metal (L8412–8416).

### 3.4 Summary / 3.5 Exercises (L8422–8539)
- Good layout designers understand their process and every mask (L8434–8438). Exercises on grids, derived-rule spacing from Table 3.1, alignment references (L8473–8539).

### Ch.4 intro (L8545–8578)
- Three archetypes: standard bipolar, poly-gate CMOS, analog BiCMOS (L8575–8578).

### 4.1 Standard Bipolar / 4.1.1 Essential Features (L8584–8622)
- NPN-optimized; junction isolation with N-epi tanks bounded by P+ isolation (L8607–8622).

### 4.1.2 Fabrication Sequence (L8646–8880)
- (111) substrate cut off-axis (L8659–8664); NBL with shadow for alignment (L8669–8675); epi thickness sets V_CEO; pattern shift (L8693–8701).
- Isolation mask aligned to the NBL shadow with a deliberate offset (L8726–8731).
- Deep-N+ with ≈ 25 % overdrive; NBL kept inside isolation or N+/P+ avalanches at ≈ 30 V (L8749–8770; PDF p.157).
- Base implant; base-over-isolation raises the NMOS thick-field threshold without a channel stop (L8790–8800). Emitter, contact OR, AlCuSi metal, compressive nitride PO (L8818–8880).

### 4.1.3 Available Devices (L8899–9317)
- NPN: base width set by diffusions, not lithography; NBL + deep-N+ lower R_C; CB-shorted diode; emitter-base Zener (L8924–9027).
- Substrate PNP: collector = substrate; place substrate contacts adjacent to limit debiasing; no NBL (L9031–9068).
- Lateral PNP: drawn base width ≥ ≈ 2× base x_j; NBL blocks substrate injection; surface recombination limits β (L9126–9181).
- Base resistor: tank tied to the more positive end; NBL under it prevents vertical punchthrough (L9203–9207). Emitter and pinch resistors; pinch resistors "notoriously variable" (L9223–9254). Junction capacitor: biased, variable, used for compensation (L9309–9317).

### 4.1.4 Process Extensions (L9336–9418)
- Up-down isolation halves outdiffusion; DLM removes crossunder tunnels; HSR implant (L9343–9418).

### 4.2 Poly-Gate CMOS / 4.2.1 Essential Features (L9438–9546)
- Self-aligned gates removed overlap capacitance (L9461–9468). (100) silicon; channel stops stop parasitic field MOS under metal (L9502–9521).
- Sidewall spacers ("zero drain overlap") (L9524–9531).
- V_t: digital 0.8–0.9 V at 5 V; 1.8 V process ≈ 0.65 V ± 100 mV; analog ≈ 0.7 V, tolerating tiny leakage at V_GS = 0 (L9540–9546; PDF p.171).

### 4.2.2 Fabrication Sequence (L9550–9906)
- 4/3 µm analog CMOS, 9 masks (PDF p.171). P+ substrate with 5–10 µm P− epi for latchup immunity (L9561–9586; PDF p.172).
- N-well x_j ≈ 5 µm for 20 V (PDF p.172); PMOS on different supplies go in separate N-wells (L9609–9614).
- Inverse moat = color-reversed moat (L9627–9657). Channel stops raise both thick-field thresholds above the maximum operating voltage (L9661–9691).
- LOCOS, Kooi, dummy gate oxide (L9696–9719); V_t adjust (0.7 V both) (L9723–9761); gate poly 20–30 Ω/□ (L9764–9779).
- Backgate contacts need NSD/PSD under them (L9852–9855); BPSG reflow; silicided contacts; PO (L9859–9906).

### 4.2.3 Available Devices (L9912–10288)
- NMOS: backgate contacts adjacent to NMOS pin the epi potential and improve latchup immunity (L9923–9926); PSD tap may abut NSD source only at the same potential (L9927–9929).
- Coding: NMoat/PMoat → PSD = PMoat oversized 1.0 (1 µm misalignment), NSD likewise, Moat = NMoat+PMoat, InvMoat = Moat\; clipping of intersecting PSD/NSD needs care (L9946–9966; PDF p.178).
- Vtadj = Moat; NChst = NWell oversized 3.0 (outdiffusion); moat overlaps the channel by several tenths of a µm so the bird's beak does not change channel dimensions (L9970–9985).
- Only square contacts of one size; bigger contacts = arrays (L9989–9992). Drawn L across poly, drawn W = poly perimeter touching the source (L9998–10002).
- Hot electrons: blocking rating (switches) vs lower operating rating (saturated analog devices) (L10006–10017). Table 4.4: L 4/3 µm, 400 Å, V_t ±0.7 V, natural 0/−1.4 V, 50/20 µA/V², V_GS 15 V, V_DS blocking 15 V, operating NMOS 7 V / PMOS 15 V (PDF p.179).
- PMOS: well outdiffusion is large; merging PMOS with a common backgate in one well saves area; backgate ≥ source (L10090–10113). Natural PMOS |V_t| > 1 V (L10124–10127).
- Substrate PNP β 50–100 (4/3 µm) or 10–20 (5 V CMOS); scribe-street substrate contacts handle 10–20 mA; beyond that fill unused die area with substrate contacts (L10150–10160; PDF p.181). Lateral PNP β < 1 without NBL (L10163–10165).
- Poly resistor 20–30 Ω/□, min width 2 µm, ≥ 50 V to substrate, low parasitic C, poor heat conduction (self-annealing drift), fuse behaviour, bad for ESD (L10169–10197; PDF p.181). NSD/PSD/N-well resistors; well tied to the more positive end; N-well resistors notoriously variable, field plates help (L10203–10227).
- Gate-oxide capacitor 0.86 fF/µm² (400 Å), ±20 %, well must stay ≥ 1 V above poly (L10261–10273; PDF p.183).

### 4.2.4 Process Extensions (L10291–10406)
- 400 Å, 3 µm: SDD NMOS 5–10 V, PMOS 15–20 V; LDD NMOS 12–15 V; spacer drift ≈ 0.5 µm (L10299–10324; PDF p.183).
- Spacer LDD is symmetric; an N−S/D block mask lets short low-voltage NMOS shrink L by 0.5–1.0 µm (L10350–10364; PDF p.184).
- Drain-extended NMOS: N-well drift (BV > 40 V), asymmetric; 15 V gate oxide 300–400 Å handles ≈ 20 V, so a field-relief bird's beak thickens oxide over the drift; STI makes field relief harder (L10368–10402; PDF p.185).

### 4.3 Analog BiCMOS / 4.3.1 Essential Features (L10412–10504)
- 25–30 masks, > 50 components (L10432–10436). CDI NPN; NBL: low R_C, no vertical punchthrough, blocks holes to substrate, enables isolated NMOS (L10452–10460).
- Deep-N+ kept mainly for minority-carrier guard rings (L10476–10479).
- LDMOS: DMOS mask implants As and B through one opening; L set by drive; field-oxide step placement sets voltage; a Pcell can target any voltage within junction BV limits (L10493–10504).

### 4.3.2 Fabrication Sequence (L10522–10900)
- Baseline 160 Å 5 V (L_min 0.7 µm) + 80 Å 3.3 V core (0.35 µm); LDMOS to 60 V; 18 / 21 / 25 masks (L10522–10538; PDF pp.187–188).
- P+ substrate + first epi + NBL + second 6 µm epi (PDF p.188); etched alignment markers in the scribe (L10552–10556).
- NBL doping ≥ 50× the N-well bottom doping to limit hole permeation (L10585–10591).
- Wells are a 3.3/5 V compromise; lateral PNP β ≈ 20; a 5/1.8 V process may need four wells; thick-field threshold ≈ 20 V without channel stops (L10603–10639; PDF p.189).
- Deep-N+ must penetrate the NBL; DTI replaces it for lateral hole blocking (L10643–10659).
- Moat = NMoat + PMoat + Base, because base in moat avoids OED variability (L10714–10745).
- PMOS needs P+ poly (buried-channel leakage otherwise); dual-doped poly via Ngate mask (L10762–10780).
- Silicide shorts P/N poly junctions; silicide block is baseline because analog needs MΩ (L10848–10867). W plugs with TiN; Ti/TiN/AlCu/TiN-ARC metal; BPSG + nitride PO (L10871–10900).

### 4.3.3 Available Devices (L10919–11378)
- NMOS PG: PSD/NSD = Moat oversized 0.3; PMSD = PSD; NMSD = NSD; InvMoat = (PMoat+NMoat)\ (L10935–10939; PDF p.194). PWell = (NWell oversized 4.0)\ is derived, not drawn (L10963–10972). Table 4.6: 0.7 µm, 160 Å, ±0.7 V, 100/40 µA/V², V_GS = V_DS = 7 V (PDF p.195).
- PMOS: NBL in the N-well reduces lateral backgate R and blocks hole injection (L11028–11031); NGate = (PSD incremented by 3.0)\ — 3 µm because dopant outdiffuses fast along poly grain boundaries (L11055–11070).
- LDMOS: bird's beak under the gate where the vertical field peaks; bird's-beak-to-NSD distance trades R_on vs hot electrons; PSD plugs at the ends suppress end breakdown (L11103–11119).
- NPN β ≈ 50; deep-N+ avoids quasi-saturation; NPN ≥ 10× CMOS area (L11188–11203). Lateral PNP: NBL depletion stop; PSD ring encloses E and C; β > 50 (L11273–11295).
- Resistors LSR 5, MSR 200, HSR 1000 Ω/□; min width 0.35 µm; variability ±20/±20/±30 %; heads silicided because W plugs need silicide; NGate = (((PSD − NWELL) incremented by 3.0) + HSR)\ (L11299–11328; PDF p.201).

### 4.3.4 Process Extensions (L11382–11723)
- 3.3 V extension (3 masks), shared wells and LDD; etch-and-regrow gives 80 Å and 160 Å oxides; 3.3 V V_t 0.6 V (L11382–11442; PDF p.202).
- LVMOS drawing layer generates LVGOX = (AllMoat*LVMOS) oversized 0.2, NVT and PVT = (N/PMoat*Poly*LVMOS) oversized 0.2 (L11447–11458). Table 4.10: 0.35 µm, 80 Å, ±0.6 V, 180/70 µA/V², V_GS and V_DS 3.6 V (PDF pp.203–204). Core devices have smaller voltage margin than I/O devices (L11474–11476).
- DI by bonding + DTI saves outdiffusion/depletion area (L11524–11536); termination A (N-well beyond trench, more area), B (N-well ends mid-trench, NBL short of it: recommended), C/D (junctions on the sidewall: low BV, leakage, β loss) (L11533–11586).
- DI NPN: large-radius bends at trench corners avoid etch/planarity problems; BV to other devices set by sidewall oxide (L11697–11708).

### 4.4 Summary / 4.5 Exercises (L11728–11859)
- Layout decides reliability (L11735–11737). Exercises build every device of the chapter; Ex. 4.19 lists a DTI rule set (width exactly, centerline corner radius exactly, spacings to NBL/DEEPN/BASE/PMOAT/NMOAT, N-well into DTI exactly) (L11817–11854).

## 3. Actionable extraction

### H01-01 Isolation-junction reverse-bias invariant (well/substrate bias ERC)
- Kind: check
- Statement: Every isolation junction must stay reverse-biased at all times: a P substrate below the lowest voltage on any isolated N region; an N-well at or above the highest voltage on any P region inside it; a PMOS backgate ≥ its source. A forward-biased isolation junction injects minority carriers that cause glitches or latchup.
- Source: §2.6.1 L5202–5215 (PDF p.94); §4.2.3 PMOS L10110–10113 (PDF p.180); §1.4.1 L3329–3336.
- Philis stage: annotator, verify, flow
- Automation recipe: Build a well/tank graph from placed geometry (each N-well polygon, its tap net, every P diffusion inside it; the substrate tie net). From the op-point (node voltages at each simulated corner and over transients when a testbench is given) check `V(well_tap) ≥ max V(p-diff inside)` and `V(sub) ≤ min V(any n-region)`, margin ≥ 0 V (hard). Pre-layout the annotator can already assert `bulk(PMOS) = highest supply of its source network` and flag PMOS whose bulk is a signal net. Output ERC violations with the offending device pair.
- Beats hand layout because: a human checks the obvious supply ties; the tool checks every well against every bias corner and transient, including wells shared by devices of different source potentials.
- Philis status: partial. GPurify runs `nwell.4 floating_well` and `supply_short(ntap, ptap)` (sky130.deck:467–468) but `well_bias` is "not configured" because the n-well is not a net (sky130.deck:663–667). Guard rings tie to the device bulk (backend/annotator/src/constraints.rs:73–74). The op-point stores only per-device I_D, V_DS headroom, g_m (frontend/library/src/oppoint.rs:13–28): no node voltages.

### H01-02 Voltage-dependent diffusion/well spacing from depletion width
- Kind: formula
- Statement: Spacing between two diffusions of the same polarity in an opposite-type background = outdiffusion of both (0.8·x_j each) + depletion width of each at its own reverse bias (Table 3.1, × 1.5 safety) + linewidth control of both. Table 3.1 (peak 10³× background, 25 °C): background 1e15, x_j 1 µm: 3/4/5/8 µm at 5/10/20/40 V; x_j 5 µm: 5/6/7/10; 1e16, x_j 1 µm: 1.3/1.5/2/— ; x_j 5 µm: 2/2.5/3/4; 1e17, x_j 1 µm: 0.5/0.6/—/—; x_j 5 µm: 1/1.2/—/— ("—" = avalanche). Worked example: NDIF–NDIF 5.8 µm at 10 V, 7.8 µm at 20 V. Depletion goes into the lightly doped side.
- Source: §3.2.3 Outdiffusion/Depletion Region Width/Example L7741–7889 (PDF pp.140–141, Table 3.1); §1.2.1 L2254–2260; Ex. 3.6 L8501–8506.
- Philis stage: deck, annotator, gp, dp, dr, verify
- Automation recipe: Give each net a voltage range (supplies from the netlist, signal nets from op-point min/max). For each pair of same-type well/tank/diffusion shapes on different nets, compute required space `s(V1,V2) = 2·0.8·x_j + 1.5·(W(V1) + W(V2)) + 2·LWC` with `W(V)` interpolated from a deck table keyed by background doping and x_j, and take `max(deck_min, s)`. Use it as a hard pairwise spacing in legalization (dp) and as a DRC check. Where the deck gives per-voltage rules (sky130 `nwell.7`, `nwell.8`, `hv.nwell.1`), use those instead of the formula.
- Beats hand layout because: hand layout applies the worst-case HV spacing everywhere or forgets it; the tool applies the exact spacing per net pair, saving area on low-voltage pairs and never under-spacing HV pairs.
- Philis status: missing. The deck states net-voltage spacings are not expressible (sky130.deck:643–645); no net-voltage attribute exists in Philis (grep for voltage domains in backend/annotator, backend/dr, backend/gr, kernel/analog finds none).

### H01-03 Design-rule derivation model and deck sanity check
- Kind: formula
- Statement: Rules are sums of error terms: minimum feature F (Abbe: F ≈ λ/(2·NA); 365 nm, NA 0.6 → 0.3 µm, used reliably at 0.5 µm); linewidth control ≈ 0.1·F per feature; mask alignment ≈ 0.2·F per alignment, two alignments ≈ 1.6× one; outdiffusion 0.8·x_j; depletion ×1.5. Worked example inputs: 0.8 µm stepper, LWC 0.1 µm per feature, alignment 0.2 µm, NDIF x_j 1.0 µm into 1e16 cm⁻³ B epi. Results: NDIF width = 2·x_j = 2.0 µm (dilution); NDIF space = 2·0.8 + depletion (1.5·1.5 ≈ 2 µm each at 10 V → 4; 3 µm each at 20 V → 6) + LWC 0.2 = 5.8 µm (10 V) / 7.8 µm (20 V); CONT width 1.0 µm exact (Al fill, not lithography, limits it); CONT space = F + LWC of both = 0.8 + 0.2 = 1.0 µm; NDIF ovl CONT = 0 + LWC 0.2 + alignment 0.2 = 0.4 µm; METAL width/space = 2F (thick film over topography) + LWC 0.2 = 1.8 µm; METAL ovl CONT = 0 + LWC of two layers 0.2 + alignment 0.2 = 0.4 µm; POR width 10.0 µm (pads > 50 µm); POR space 4.2 µm (terms listed: wet overetch ≈ 1.0 µm, LWC 0.2 µm, min PO ≈ 2 µm; the stated total implies overetch counted on both openings); METAL ovl POR = 1.0 + 0.2 + 0.2 = 1.4 µm.
- Source: §3.2.3 L7596–7986 (PDF pp.138–143).
- Philis stage: deck, verify
- Automation recipe: Encode the example rule set as a unit-test fixture of a small `derive_rule(terms)` helper used only where a deck lacks a rule that Philis needs (e.g. a generator dimension such as implant-over-tap). Also run a deck lint: flag decks whose `linewidth_control_nm` is empty or whose enclosure < measured LWC + alignment.
- Beats hand layout because: the tool can fill missing sidecar dimensions from process physics consistently instead of guessed constants, and can explain every generated clearance.
- Philis status: missing. `linewidth_control_nm` is an empty object in the sidecars (pdks/sky130.json:75; pdks/generic_finfet.json:81) and has no Rust consumer (grep finds no `linewidth_control` in backend/kernel/frontend).

### H01-04 Outdiffusion and dilution geometry for wells and diffusions
- Kind: formula
- Statement: Fabricated junction edge = drawn edge + 0.8·x_j (5 µm well: +4 µm per side; 10 µm drawn → 18 µm). Widths < ≈ 2·x_j suffer dilution (doping drop up to 5× in dilution patterns); keep min width ≥ x_j when dilution is unwanted. Practical x_j limit ≈ 15 µm. A 40 V process needs 6–10 µm wells; a 20 V CMOS N-well is ≈ 5 µm.
- Source: §2.4.2 L4661–4667 (PDF p.85); §3.2.3 Outdiffusion L7741–7754 (PDF p.140); §2.6.1 L5218–5225 (PDF p.94); §4.2.2 N-Well Diffusion L9590–9594 (PDF p.172).
- Philis stage: cells, deck, gp
- Automation recipe: When the deck lacks an explicit well-to-diffusion enclosure or a well-edge keep-out, derive `enc ≥ 0.8·n_well_depth` and use the fabricated (outdiffused) well outline, not the drawn one, when measuring WPE distances and well-to-well spacing. Reject well stripes narrower than x_j unless intentional.
- Beats hand layout because: measuring from the fabricated junction instead of the drawn edge keeps well-proximity and spacing budgets honest for every device.
- Philis status: partial. `n_well_depth` is listed as a sidecar key (backend/verify/src/pdk.rs:1058; pdks/sky130.json:68) but has no consumer; enclosures come from the deck (`well_enclosure` pdks/sky130.json:43).

### H01-05 Merge same-backgate PMOS into one well
- Kind: heuristic
- Statement: Any number of PMOS can share a well when their backgates tie to the same potential; because the deep well outdiffuses a lot, merging saves substantial area.
- Source: §4.2.3 PMOS L10090–10094 (PDF p.180); §4.3.3 PMOS L11031.
- Philis stage: annotator, gp, dp, cells
- Automation recipe: Annotator groups PMOS by bulk net (well groups). Placement adds a cost term rewarding adjacency of same-group PMOS (and forbids the widest-layer clearance between them), then the generator draws one well per connected same-group cluster; different-group wells keep the (voltage-dependent, H01-02) well spacing.
- Beats hand layout because: global placement can trade well sharing against matching and wirelength across all PMOS at once rather than row by row.
- Philis status: partial. Placement separates all cells by the widest device-layer spacing "so wells and implants of neighbouring cells never merge" (frontend/library/src/lib.rs:526–528); only post-placement `well_bridges` fill gaps up to twice the well spacing between facing same-bulk PMOS (kernel/cells/src/post_cell.rs:376–386; frontend/library/src/lib.rs:608–610).

### H01-06 Well partitioning by supply/bulk; source-tied wells; body-effect identity for matched devices
- Kind: rule
- Statement: In N-well CMOS, PMOS referenced to different supplies occupy separate N-wells; the P substrate ties to the negative ground. The PMOS well is a free design variable (any voltage ≥ source) that designers use to remove body effect. Body effect (backgate modulation) shifts V_t with V_SB.
- Source: §4.2.2 N-Well Diffusion L9609–9614 (PDF p.172); §4.2.3 PMOS L10110–10113; §1.4.1 L3329–3336.
- Philis stage: annotator, cells, gp
- Automation recipe: From the netlist, the well key of a PMOS is its bulk net; for NMOS without an isolated P-well (deep N-well) the bulk must be the substrate net, otherwise flag "needs isolated P-well (DNW)". For matched pairs require identical bulk net and identical source net (or equal op-point V_SB) — a hard constraint for matching; a pair with bulk tied to different sources must use two wells placed symmetrically.
- Beats hand layout because: the tool enforces well identity for every matched group and every supply domain automatically, and catches NMOS bulks that silently need isolation.
- Philis status: partial. Guard-ring type and bulk tie are derived per device (backend/annotator/src/constraints.rs:67–74); NPN isolation requires `dnwell` (kernel/cells/src/bjt.rs:31–34); no NMOS-bulk-not-substrate check or V_SB equality check found.

### H01-07 Backgate/substrate contacts next to devices; tap distance (latchup)
- Kind: rule
- Statement: Place substrate (backgate) contacts immediately adjacent to NMOS to pin the epi surface potential and improve latchup immunity; epi on P+ substrate lowers substrate resistance for the same reason; backgate contacts need NSD/PSD under them for ohmic contact.
- Source: §4.2.3 NMOS L9923–9926 (PDF p.177); §4.3.3 NMOS L10963–10966 (PDF p.195); §4.2.2 Starting Material L9561–9563, Epitaxial Growth L9582–9586 (PDF pp.171–172); Contacts L9852–9855.
- Philis stage: cells, verify, dp
- Automation recipe: Hard check: every point of every NMOS/PMOS active area within `tie_max_dist` of a tap of its own bulk net (use the deck value; sky130 sidecar 3000 nm, IHP 20000 nm); dp adds a penalty when a move pushes a device past it without a shared tap. Emit the tap row as part of the cell (already done) and a separate tap cell when a cell has none.
- Beats hand layout because: exhaustive per-device distance checks and automatic tap insertion remove the most common latchup review finding.
- Philis status: partial. MOSFET cells carry a bulk tap rail (kernel/cells/src/mosfet.rs:2, 494–499); `tie_max_dist_nm` is in the sidecar (pdks/sky130.json:85; pdks/ihp_sg13g2.json:76) but only listed as a known key (backend/verify/src/pdk.rs:1068), never checked; the deck says latch-up "needs a guard-ring width and tap distance that [S1] does not state" (sky130.deck:669–670).

### H01-08 Substrate-contact area proportional to injected substrate current
- Kind: rule
- Statement: Devices that inject into the substrate (substrate PNP collector current; any forward-biased isolation junction) need adequate substrate contact to avoid debiasing. A typical CMOS die's scribe-street substrate contacts handle 10–20 mA; beyond that, unused die area should be filled with substrate contacts. Substrate PNP contacts go adjacent to the transistor.
- Source: §4.2.3 Substrate PNP L10150–10160 (PDF p.181); §4.1.3 PNP L9056–9060 (PDF p.162); §2.6.1 L5211–5215.
- Philis stage: annotator, cells, flow
- Automation recipe: Annotator tags injectors (vertical/substrate PNP, diodes to substrate, devices whose op-point shows a forward-biased junction) with their current from the op-point. Emit a requirement: substrate-tap area near the injector ≥ I_inj / J_tap (J_tap from the deck or a default), placed between injector and sensitive devices. After routing, fill free block area with substrate taps when total injection exceeds the block budget.
- Beats hand layout because: contact area is sized from simulated current instead of habit, and placed on the injector-to-victim path.
- Philis status: partial. Guard rings exist per device polarity (backend/annotator/src/constraints.rs:68–71; kernel/cells/src/post_cell.rs:1–5) and injector/victim spacing exists for clock nets (backend/annotator/src/emit.rs:258–263; kernel/analog/src/placement/isolation.rs:7–10); no current-proportional contact sizing.

### H01-09 Butted source–tap only at equal potential
- Kind: rule
- Statement: When an NMOS source ties to substrate potential, abutting the PSD substrate contact against the NSD source gives a very compact layout. PSD and NSD at different potentials must not abut: the N+/P+ junction leaks excessively and breaks down at a very low voltage.
- Source: §4.2.3 NMOS L9926–9929 (PDF p.177); Fig. 4.25 caption L9835–9836.
- Philis stage: cells, verify
- Automation recipe: In the MOSFET generator, when `net(S) == net(B)` for an end source region, emit a butted-tap variant (tap abutting the source diffusion, implants split at the shared edge) as an extra placement alternative; otherwise keep the separate tap row. Add an ERC: any touching N+/P+ diffusion pair must be on the same net.
- Beats hand layout because: the tool evaluates the butted variant for every eligible device and keeps it only where it wins area without violating matching symmetry.
- Philis status: missing (MOSFET cells use a separate tap strip with its own implant spacing, kernel/cells/src/mosfet.rs:494–499; no butting option found).

### H01-10 Implant enclosure of active and PSD/NSD clipping
- Kind: rule
- Statement: Source/drain implants are generated from moat drawings oversized for misalignment (4/3 µm CMOS: PSD = PMoat oversized 1.0 µm; BiCMOS: 0.3 µm) so the implant covers the moat opening; intersecting PSD/NSD must be clipped (needs a careful algorithm, §12.2.1).
- Source: §4.2.3 NMOS L9946–9966 (PDF p.178); §4.3.3 NMOS L10935–10958 (PDF p.194).
- Philis stage: cells
- Automation recipe: Generators draw implants as `active ⊕ enc(implant, active)` from the deck; where an N-implant and P-implant region of the same cell meet (tap next to diffusion), split along the midline between the two actives, each keeping its enclosure, and verify implant-to-opposite-active spacing.
- Beats hand layout because: implants are derived deterministically from the active shapes, so they are always consistent after any regeneration.
- Philis status: implemented via deck enclosures (kernel/cells/src/mosfet.rs:496–498; kernel/cells/src/bjt.rs:90–96).

### H01-11 Moat/field-edge effects: bird's beak, Kooi, OED; moat past the channel
- Kind: heuristic
- Statement: LOCOS bird's beak encroaches under the moat edge; Kooi nitride residue thins gate oxide at moat edges; oxidation-enhanced diffusion drives junctions deeper under field oxide than under moat. Hence the moat must overlap the channel by at least several tenths of a µm so the bird's beak does not change channel dimensions, and BiCMOS base regions are placed inside moat to avoid OED variability.
- Source: §2.3.4 L4402–4451, L4480–4484 (PDF pp.80–82); §2.4.2 L4729–4732; §4.2.3 L9982–9985 (PDF p.178); §4.3.2 Inverse Moat L10714–10745.
- Philis stage: cells, dp
- Automation recipe: Keep every matched device's gate a fixed distance from any active edge in both axes (identical for all members); use the deck's gate-to-active-edge and LOD values; prefer dummy fingers so the outermost real gate never sits next to the moat edge.
- Beats hand layout because: identical edge environment is enforced for every member of every matched group, not only the ones a reviewer inspects.
- Philis status: implemented in part: dummies are required for matched groups (backend/annotator/src/constraints.rs:61); the generator extends moat by `lod_moat_ext_moderate` for multi-device cells (kernel/cells/src/mosfet.rs:293); OSE/WPE skew is scored (kernel/analog/src/placement/environment.rs:13–16).

### H01-12 Tilted implants create orientation-dependent devices → matched devices share orientation
- Kind: rule
- Statement: Implanters tilt the beam to avoid channeling; a tilted implant can be restricted to one direction, yielding "two classes of transistors, one aligned vertically and the other horizontally, which have different device characteristics".
- Source: §2.4.3 L4899–4911 (PDF p.89).
- Philis stage: dp, verify
- Automation recipe: Hard constraint: all members of a matched group share gate orientation and current direction; forbid R90/R270 relative turns inside a group; report orientation spread in the match report.
- Beats hand layout because: enforced for every group through every placement move.
- Philis status: implemented ("Grouped (matched) devices never turn, and no move introduces a mirror", backend/dp/src/lib.rs:555–560; `quarter_turn` stays within the reflection class, backend/dp/src/lib.rs:541–552).

### H01-13 Drawn vs effective dimensions and units
- Kind: data-model
- Statement: Drawn L = poly width across the moat; drawn W = perimeter of poly touching the source (equals drain side for a simple device); PG size adjusts make drawn ≈ effective. Rules and capacitances must be in silicon µm; the old 90 % drawn/silicon shrink caused ≈ 20 % capacitance errors.
- Source: §4.2.3 L9998–10002 (PDF p.179); §4.3.3 L10977–10982; §3.2.3 Scalable Design Rules L8035–8061 (PDF p.143).
- Philis stage: cells, verify, annotator
- Automation recipe: Keep one unit system (nm silicon); compute W for non-rectangular gates (annular, bent) as source-side perimeter and feed that to LVS/sizing; never scale geometry.
- Beats hand layout because: no manual unit conversion; extracted W/L always consistent with generated geometry.
- Philis status: implemented (1 dbu = 1 nm, backend/verify/src/pdk.rs:4); only rectangular gates are generated.

### H01-14 Symmetric vs asymmetric MOS and bias-determined source/drain
- Kind: rule
- Statement: Source/drain identity follows bias (NMOS drain = more positive terminal) and may swap. Asymmetric devices (selectively blocked LDD, drain-extended MOS, LDMOS) must never have S and D interchanged — the rating drops, possibly catastrophically. Spacer-LDD devices are symmetric. Drain-extended NMOS uses N-well as drift and NSD as source.
- Source: §1.4 L3105–3107 (PDF p.54); §1.4.1 L3312–3321 (PDF p.57); §4.2.4 L10350–10353, L10377–10381 (PDF pp.184–185).
- Philis stage: annotator, cells, verify
- Automation recipe: Annotator reads the device model's symmetry flag (deck recipe) and the op-point terminal voltages; for asymmetric models it pins the drift side to the netlist drain and forbids finger flips/mirrors that swap S/D; for symmetric devices it may swap to share diffusion. LVS must compare terminals with S/D swap disabled for asymmetric models.
- Beats hand layout because: every device's S/D orientation is checked against bias, not just the power devices someone remembers.
- Philis status: missing. Device recipes cover resistors only (backend/verify/src/pdk.rs:1073–1088); the MOSFET generator pairs fingers on shared drains by netlist terminal (kernel/cells/src/mosfet.rs:22–27), not by bias.

### H01-15 Hot-carrier operating vs blocking voltage rating
- Kind: check
- Statement: NMOS have a blocking rating (switches; junction breakdown/punchthrough) and a lower operating rating (devices held in saturation, set by hot-electron degradation). Example 4/3 µm process: NMOS blocking 15 V, operating 7 V; PMOS 15/15 V. Plain 400 Å, 3 µm devices: NMOS 5–10 V, PMOS 15–20 V; LDD NMOS 12–15 V. BiCMOS 5 V and 3.3 V devices with LDD have one rating (7 V; 3.6 V).
- Source: §4.2.3 L10006–10026, Table 4.4 (PDF pp.179); §4.2.4 L10299–10312 (PDF p.183); §4.3.3 L10982–10984, Table 4.6 (PDF p.195); Table 4.10 (PDF pp.203–204).
- Philis stage: annotator, verify
- Automation recipe: Per device model store V_GS,max, V_DS,block, V_DS,op. From the op-point: if the device is saturated (headroom ≥ 0) require |V_DS| ≤ V_DS,op, else ≤ V_DS,block; always |V_GS| ≤ V_GS,max. Report violations before layout (it is a sizing/flavor error the layout cannot fix) and use them to pick the device flavor (H01-16).
- Beats hand layout because: it is checked on every device at every simulated corner.
- Philis status: missing (no voltage-rating data or check; op-point has V_DS headroom only, frontend/library/src/oppoint.rs:19–22).

### H01-16 Device flavors and oxide marker layers (3.3 V core vs 5 V I/O, natural V_t)
- Kind: deck-requirement
- Statement: A dual-oxide process marks thin-oxide devices with a drawing layer (LVMOS) that generates LVGOX = (AllMoat*LVMOS) oversized 0.2, NVT = (NMoat*Poly*LVMOS) oversized 0.2, PVT likewise. 5 V: 160 Å, L_min 0.7 µm, V_t ±0.7 V, 100/40 µA/V², 7 V; 3.3 V: 80 Å, 0.35 µm, ±0.6 V, 180/70 µA/V², 3.6 V. Core devices have smaller voltage margin than I/O devices. Natural devices are made by blocking the V_t-adjust (NMOS ≈ 0 V, PMOS ≈ −1.4 V). Quad-well processes exist for 5/1.8 V.
- Source: §4.3.2 L10522–10538 (PDF pp.187–188); §4.3.4 L11402–11476 (PDF pp.202–204); §4.2.3 L10021–10023, L10124–10127; §4.3.2 P-Well Implant L10628–10634.
- Philis stage: deck, cells, annotator
- Automation recipe: For each FET model the PDK sidecar gets a recipe `{model, marker layers (hvi/thkox/dualgate/lvtn/hvtp/natural), marker enclosure of diff/poly, L_min, ratings}`; the MOSFET generator draws the markers around the cell with the deck enclosure; placement treats marker regions like wells (same-marker cells may share, different markers keep marker spacing). The annotator selects the flavor from the netlist model name and checks ratings (H01-15).
- Beats hand layout because: flavor, markers and spacing come from one table for every device; an I/O device can never be drawn without its oxide marker.
- Philis status: missing. The deck recognizes `nfet_g5v0d10v5`/`pfet_g5v0d10v5` via `hvi` (sky130.deck:196, 201) and has `hvi.*` rules (sky130.deck:424–427), but Philis draws no `hvi` (no hit in kernel/cells) and FET recipes do not exist (backend/verify/src/pdk.rs:1073–1088 covers resistors).

### H01-17 Thick-field threshold: parasitic field MOS under leads
- Kind: check
- Statement: Metal leads over field oxide form parasitic MOS transistors; channel stop implants (or base-over-isolation, or well doping in low-voltage processes) raise the metal-1 thick-field thresholds above the maximum operating voltage (BiCMOS example ≈ 20 V without channel stops).
- Source: §4.2.1 L9510–9521 (PDF p.171); §4.2.2 Channel Stop Implants L9661–9691 (PDF p.173); §4.1.2 Base Implant L8798–8800; §4.3.2 P-Well Implant L10636–10639 (PDF p.189).
- Philis stage: deck, dr, verify
- Automation recipe: Deck gives V_T,field per metal layer. For each net with |V| > V_T,field (from net voltage ranges), flag routes crossing field between two same-type diffusions/wells on different nets (the parasitic channel path); the router adds a cost to such crossings and can drop a grounded field plate/channel-stop-equivalent (ch.5 methods) when unavoidable.
- Beats hand layout because: every HV net segment is checked against every field gap it crosses.
- Philis status: missing (no field-threshold data; low priority for ≤ 5 V designs).

### H01-18 High-voltage MOS generators (drain-extended MOS, LDMOS)
- Kind: algorithm
- Statement: Drain-extended NMOS: N-well drift, NSD plug drain, NSD source outside the well (asymmetric); gate oxide handles ≈ 20 V for 300–400 Å, so oxide is thickened only over the drift using the LOCOS bird's beak (field relief); STI's abrupt step makes field control harder. LDMOS: DMOS implant (As+B) through one opening, L set by drive; bird's beak placed under the gate where the vertical drain-gate field peaks; bird's-beak-to-NSD distance trades R_on vs lateral field/hot electrons; PSD plugs at DMOS ends suppress end breakdown; a Pcell can target any voltage within N-well/DMOS-backgate and N-well/substrate breakdown.
- Source: §4.2.4 L10368–10402 (PDF pp.184–185); §4.3.1 L10493–10504 (PDF p.187); §4.3.3 DMOS L11074–11119 (PDF pp.196–197).
- Philis stage: cells
- Automation recipe: Parametric generator `(V_DS target) → drift length, field-plate/STI overlap, drain plug spacing, end plugs`, with the voltage→dimension table from the deck; annotator selects it for asymmetric HV models (H01-14).
- Beats hand layout because: voltage-optimized drift geometry per device instead of one fixed HV cell.
- Philis status: missing (no HV FET generator; sky130 deck lists `denmos.*`, `extd.*` as "left out", sky130.deck:647–660).

### H01-19 Buried layers, deep-N+, DTI termination (BCD/BiCMOS decks)
- Kind: deck-requirement
- Statement: NBL must stay inside isolation (else N+/P+ avalanches at ≈ 30 V); NBL doping ≥ 50× the N-well bottom; deep-N+ must penetrate the NBL; NBL shadow pattern shift needs mask offsets; with DTI, terminate the N-well mid-trench and keep NBL short of the trench (scheme B); junctions on the trench sidewall (C, D) risk low BV/leakage/β loss; use large-radius bends at trench corners; DI removes substrate injection and saves area.
- Source: §4.1.2 L8766–8770 (PDF p.157); §4.3.2 L10585–10591, L10643–10659 (PDF pp.188–189); §2.5.1 L5048–5071; §4.3.4 DI L11524–11586, L11697–11708 (PDF pp.204–207); Ex. 4.19 L11817–11854.
- Philis stage: deck, cells, gp
- Automation recipe: When a deck has NBL/DTI layers, generators draw termination scheme B by default; placement uses the existing DTI banding (share or isolate) and adds rounded trench corners at cell corners.
- Beats hand layout because: termination choice and trench corners are applied uniformly and cost-evaluated.
- Philis status: partial (DTI share/isolate banding rule exists, kernel/analog/src/placement/dti.rs:7–13; no NBL/DTI termination generation; current PDKs lack NBL).

### H01-20 Poly resistor: isolation, heat, fusing
- Kind: rule
- Statement: Poly resistors on field oxide are dielectrically isolated (≥ 50 V to substrate, can sit above/below the rails) with low parasitic capacitance, but oxide conducts heat poorly: enough power permanently shifts resistance (self-annealing) and extreme power melts/cracks poly before a diffused resistor would fail; unsuitable for pulsed power/ESD.
- Source: §4.2.3 Resistors L10169–10197 (PDF p.181).
- Philis stage: annotator, cells, verify
- Automation recipe: From the op-point compute per-resistor dissipation; require power density `P/(W·L) ≤ p_max` from the deck (or segment width ≥ w(P)); flag poly resistors on nets that the annotator classifies as ESD/pad paths.
- Beats hand layout because: every resistor segment is sized from its simulated power, not rule-of-thumb widths.
- Philis status: missing (resistor generator has no power-driven sizing; thermal model uses device power for placement only, kernel/core/src/thermal.rs:1–4).

### H01-21 Silicided-process resistor recipes
- Kind: deck-requirement
- Statement: Silicided poly is only a few Ω/□; resistors need a silicide-block over the body while heads stay silicided (W plugs need silicide). BiCMOS example: LSR 5 Ω/□, MSR 200 Ω/□, HSR 1 kΩ/□, min width 0.35 µm, variability ±20/±20/±30 %; HSR uses a dedicated mask to block the gate implant; NGate derivation includes HSR.
- Source: §2.7.3 L5614–5617; §2.7.5 L5821–5825; §4.3.2 Silicidation L10857–10859; §4.3.3 Resistors L11299–11328 (PDF pp.200–201).
- Philis stage: cells, deck
- Automation recipe: Resistor recipe per model = layers + block enclosure + head length; generator places the block over all bodies (dummies included) and silicided heads.
- Beats hand layout because: identical block/head geometry on every segment and dummy.
- Philis status: implemented (resistor recipes, backend/verify/src/pdk.rs:1073–1088; salicide block over every body incl. dummies, kernel/cells/src/resistor.rs:214–220).

### H01-22 Diffused and well resistors: isolation bias and variability
- Kind: rule
- Statement: A diffused resistor's tank/well must be tied to its more positive end (or a higher node) to stay reverse-biased; NBL under base resistors prevents vertical punchthrough; NSD/PSD resistors break down at about 20 V (Table 4.5); N-well resistors are "notoriously variable" (doping, outdiffusion, voltage modulation, surface effects) — field plates help; pinch resistors are the most variable.
- Source: §4.1.3 Resistors L9203–9254 (PDF pp.165–166); §4.2.3 Resistors L10203–10227 (PDF p.182).
- Philis stage: annotator, cells
- Automation recipe: If a diffused/well resistor model is used, the annotator emits `well_net = argmax(V(R+), V(R−))` (from op-point) and the generator ties the well there; prefer poly when the precision class demands.
- Beats hand layout because: the well tie follows the actual bias, including circuits where the "positive end" flips with operating mode.
- Philis status: missing (poly resistors only, kernel/cells/src/resistor.rs:3).

### H01-23 MOS (gate-oxide) and junction capacitors: bias polarity and parasitics
- Kind: rule
- Statement: Gate-oxide capacitor (poly over N-well): 0.86 fF/µm² at 400 Å, ±20 %, but only while the well stays ≥ 1 V above the poly; otherwise capacitance drops sharply; large bottom-plate junction parasitic and series resistance. Junction capacitors must stay reverse-biased, vary with bias/temperature, and are fit mainly for compensation.
- Source: §4.2.3 Capacitors L10261–10273 (PDF p.183); §4.1.3 Capacitors L9309–9317 (PDF p.167); §4.3.3 L11370–11378.
- Philis stage: annotator, cells, verify
- Automation recipe: For MOS caps check `V(well) − V(poly) ≥ 1 V` at every op-point corner (hard), connect the well (bottom plate, large parasitic) to the low-impedance node, and put the poly (top plate) on the sensitive node.
- Beats hand layout because: bias validity and plate orientation are checked for every MOS cap automatically.
- Philis status: missing (capacitor generator covers MOM/MIM metal stacks only, kernel/cells/src/capacitor.rs:11–20).

### H01-24 Bipolar devices available in CMOS/BiCMOS
- Kind: data-model
- Statement: N-well CMOS offers only the substrate PNP (emitter PSD, base N-well, collector = substrate; β 50–100 in 4/3 µm, 10–20 in 5 V CMOS); lateral PNP without NBL has β < 1. BiCMOS lateral PNP with NBL: β > 50 (≈ 20 with the 3.3/5 V well). Lateral PNP drawn base width ≥ ≈ 2× base junction depth. CDI NPN β ≈ 50, needs deep-N+ to avoid quasi-saturation, ≥ 10× CMOS area. CMOS NPN β may be < 10.
- Source: §4.2.3 Substrate PNP L10131–10165 (PDF pp.180–181); §4.1.3 L9131–9135; §4.3.2 N-Well Implant L10609–10612; §4.3.3 NPN/PNP L11188–11295; §1.3.1 L2853–2857.
- Philis stage: annotator, cells
- Automation recipe: Annotator maps BJT models to available constructions; substrate PNPs get collector = substrate net forced and substrate taps adjacent (H01-08); reject lateral PNP in processes without a buried layer.
- Beats hand layout because: impossible or poor constructions are rejected before placement.
- Philis status: partial (BJT generator: PNP units share the substrate collector band, kernel/cells/src/bjt.rs:58; NPN requires dnwell, kernel/cells/src/bjt.rs:31–34).

### H01-25 Contact and via standardization
- Kind: rule
- Statement: Etch rate depends on opening size and shape, so processes allow only square contacts of one size; larger contacts are arrays of minimum contacts. W plugs: metal overlap needed on only two sides, contacts/vias may stack, but arbitrary cut sizes are forbidden.
- Source: §4.2.3 L9989–9992 (PDF p.178); §2.7.4 L5720–5725 (PDF p.104); §3.2.2 Width `exact` L7428–7439; §3.2.3 Example CONT width 1.0 exact L7895–7903.
- Philis stage: cells, dr
- Automation recipe: Cuts only on a lattice of the exact cut size; enclosure may be asymmetric (two-sided) per deck; stack vias where the deck allows; identical cut arrays on matched devices.
- Beats hand layout because: cut counts and positions are identical across matched devices and maximal for EM and resistance.
- Philis status: implemented (cut lattice `2·grid`, kernel/cells/src/builder.rs:141–143; two-sided enclosure handling, backend/verify/src/pdk.rs:1091–1100; deck exact size `licon.1 size == 170nm x 170nm`, sky130.deck:331).

### H01-26 Grid and database-unit discipline
- Kind: rule
- Statement: Coordinates are integers in DBU; grid increment is an integer multiple of the coding increment; every coordinate on the minimum grid; path (and centred-shape) widths are integer multiples of twice the coding increment so edges land on grid; DBU conversion is safe only by integer multiples.
- Source: §3.1.1 L6534–6551; §3.1.2 L6565–6568; §3.1.3 Paths L6720–6726 (PDF p.124).
- Philis stage: cells, gp, dp, dr, flow
- Automation recipe: Snap all generator rectangles, cell origins and route centrelines to the grid; centre-referenced objects use a `2·grid` lattice; assert in debug builds.
- Beats hand layout because: off-grid errors are impossible by construction.
- Philis status: implemented (grid-snapped Builder, kernel/cells/src/builder.rs:8–9; `2·grid` cut lattice, builder.rs:141–143; origins snap, backend/gp/src/lib.rs:158; deck `x.1b off_grid`, sky130.deck:447).

### H01-27 Geometry primitives: Manhattan, no acute/nonsimple, circles, HV corner rounding
- Kind: rule
- Statement: Low-voltage CMOS allows only orthogonal shapes; never nonsimple polygons; no acute angles (DRC errors, PG decomposition failures); diagonal paths lose up to 1.4 dbu of width; circles approximated with side counts that are multiples of 4 (32/64), matched circles as on-grid polygons; HV processes use non-orthogonal shapes to avoid field-intensifying corners; trench corners get large-radius bends.
- Source: §3.1.3 L6635–6684, L6728–6732 (PDF p.123–124); §3.2.2 L7414–7425, L7496–7506; §4.3.4 L11704–11706.
- Philis stage: cells, dr, verify
- Automation recipe: Keep shapes as axis-aligned rectangles; if octagonal inductors or HV rounded corners are added, generate polygons with 8k sides whose vertices lie on grid, identical for matched instances.
- Beats hand layout because: no hand-drawn diagonal errors; symmetric polygonization for matched round structures.
- Philis status: implemented for Manhattan (`Shape` is a layer + `Rect`, kernel/core/src/geom.rs:69–72; deck `x.3a angle`, sky130.deck:448); HV rounding N/A.

### H01-28 Orthogonal (D4) orientations only
- Kind: rule
- Statement: Use only the eight Manhattan transformations; magnification and any-angle rotation cause rounding and are usually disallowed.
- Source: §3.1.4 Transformations L6798–6823 (PDF p.125); §3.1.5 GDSII L7062–7067.
- Philis stage: gp, dp, emit
- Automation recipe: Placement orientation variable ∈ D4; emit SREF with angle ∈ {0,90,180,270} and mirror flag only.
- Beats hand layout because: n/a (parity with hand layout).
- Philis status: implemented (`Orient` = D4, kernel/core/src/geom.rs:18–31).

### H01-29 Hierarchy and Pcell discipline
- Kind: heuristic
- Statement: Editing a shared cell changes every instance; copy before local edits. A properly designed Pcell always passes DRC and embodies good practice; exploded Pcells lose linkage; custom layout only where the Pcell would significantly degrade performance.
- Source: §3.1.4 L6784–6787, L6919–6962 (PDF pp.125–128).
- Philis stage: cells, emit
- Automation recipe: Each generator variant = one unique cell; generators self-check DRC; emit GDS hierarchy (one structure per unique variant, SREF per placement) so external hierarchical DRC/LVS runs faster and diffs are readable.
- Beats hand layout because: generators are DRC-clean by construction and regenerated, never hand-patched.
- Philis status: partial (generator self-check tests, kernel/cells/tests/cell_selfcheck.rs:40–68; GDS is flat: "flat shapes → one `TOP` structure", frontend/library/src/gds.rs:1–2, 9).

### H01-30 Pins and labels in the exchange data
- Kind: data-model
- Statement: A layout pin = polygon + layer + name (+ direction); GDSII carries pins only as properties attached to shapes or as text.
- Source: §3.1.4 Pins L6880–6885 (PDF p.126); §3.1.5 GDSII L7036–7039.
- Philis stage: emit
- Automation recipe: Write TEXT records (layer/datatype = the deck's pin/label purpose) at each top-level pin and each net label needed by external LVS; write a pin-direction property when known.
- Beats hand layout because: labels are generated from the netlist, never mistyped.
- Philis status: missing (GDS writer emits only BOUNDARY elements, no TEXT records, frontend/library/src/gds.rs:1–25).

### H01-31 GDSII format limits
- Kind: deck-requirement
- Statement: 32-bit signed coordinates; layers/datatypes 0–63 originally (0–255 common, 0–32767 Cadence); ≤ 200 vertices per boundary/path (8000 Cadence); last vertex repeats the first; structure names ≤ 32 characters from [A-Za-z0-9_?$] (avoid ? and $); path types 0 and 2 universally recognized; any-angle/magnification risky.
- Source: §3.1.5 GDSII L7036–7073 (PDF pp.128–129).
- Philis stage: emit
- Automation recipe: When hierarchy is emitted, sanitize and hash-shorten structure names to ≤ 32 legal chars; keep rectangles as 5-point boundaries; no PATH elements.
- Beats hand layout because: n/a (portability).
- Philis status: implemented for the current flat writer (rect BOUNDARY only, single name `TOP`, frontend/library/src/gds.rs:9, 20).

### H01-32 Derived-layer algebra; non-invertible sizing
- Kind: algorithm
- Statement: OR, AND (distributive over OR), NOT (dark field), ANDNOT, XOR, oversize/undersize. Oversize-then-undersize closes notches (does not restore the shape); undersize-then-oversize deletes narrow arms.
- Source: §3.2.1 L7152–7359 (PDF pp.130–133).
- Philis stage: cells, verify
- Automation recipe: Use closing (grow then shrink by s/2) to merge same-net wells/implants separated by < s into notch-free regions; use opening to find sub-min-width slivers; never assume a size pair is identity.
- Beats hand layout because: notch and sliver cleanup is global and exact.
- Philis status: implemented (deck boolean/derived layers, sky130.deck:104–207; well bridging as a closing between facing wells, kernel/cells/src/post_cell.rs:376–386).

### H01-33 DRC check semantics the deck must express
- Kind: deck-requirement
- Statement: Merge before checking; width measured perpendicular and vertex-to-vertex; spacing intra- and inter-figure with `overlap okay`; overlap (enclosure) including partial enclosure errors and `must touch` for naked cuts; overhang (extension); `exact` widths/overlaps for fixed devices.
- Source: §3.2.2 L7363–7568 (PDF pp.134–137).
- Philis stage: verify, deck
- Automation recipe: Philis inner loops use the same semantics as the deck (vertex-to-vertex corner spacing in its own clearance math) so the router and legalizer never produce what sign-off rejects.
- Beats hand layout because: in-loop checks match sign-off semantics exactly.
- Philis status: implemented in GPurify (e.g. `enclosure(... opposite)`, `size(...) ==`, sky130.deck:331–336; DRC/ERC classification, backend/verify/src/checker.rs:337–347).

### H01-34 Layer taxonomy: coding, generated, pseudo (marker) layers
- Kind: data-model
- Statement: Coding layers are drawn; generated layers are produced by the PG deck (masks); pseudolayers are drawn but only inform DRC/recognition (e.g. voltage-recognition layers); examples: NMoat/PMoat → Moat/NSD/PSD; LVMOS → LVGOX/NVT/PVT; HSR/SiBlk markers.
- Source: §3.3.1 L8110–8128 (PDF p.145); §4.2.3 L9946–9960; §4.3.4 L11447–11458.
- Philis stage: deck, cells
- Automation recipe: Sidecar roles classify each layer as drawn, derived (never drawn by Philis) or marker; generators draw only drawn and marker layers; tests assert no generator writes a derived layer.
- Beats hand layout because: consistent layer usage across all generators and PDKs.
- Philis status: partial (roles map, backend/verify/src/pdk.rs:52–56; derived layers live in the deck, sky130.deck:104–207; no explicit drawn/derived/marker classification).

### H01-35 Derived-layer formulas as deck test fixtures
- Kind: formula
- Statement: PSD = PMoat oversized 1.0 (4/3 µm) or 0.3 (BiCMOS); NSD likewise; Moat = NMoat + PMoat (+ Base in BiCMOS); InvMoat = Moat\; Vtadj = Moat; NChst = NWell oversized 3.0 (well outdiffusion); PWell = (NWell oversized 4.0)\; NGate = (PSD incremented by 3.0)\ → (((PSD − NWELL) incremented by 3.0) + HSR)\ (3 µm because dopant diffuses fast along poly grain boundaries); LVGOX/NVT/PVT oversized 0.2.
- Source: §4.2.3 L9954–9978 (PDF p.178); §4.3.2 L10740–10742; §4.3.3 L10935–10939, L10972, L11062–11070, L11318 (PDF pp.194–201); §4.3.4 L11455–11458 (PDF p.203).
- Philis stage: deck, verify
- Automation recipe: Use these as regression fixtures for the deck-language derived-layer engine and as the pattern for any Philis-side derived layer (e.g. computing a PWell keep-out around wells when placing NMOS).
- Beats hand layout because: n/a (verification infrastructure).
- Philis status: n/a in Philis (deck responsibility, GPurify).

### H01-36 Process bias is geometry dependent → identical geometry for matched devices
- Kind: heuristic
- Statement: Etch bias ≤ 10 % of film thickness is corrected by PG size adjusts, but bias depends on geometry (large openings etch faster), so one adjust cannot correct all sizes; separate rules exist for larger vias.
- Source: §3.2.3 Process Bias L7652–7676 (PDF p.139).
- Philis stage: annotator, cells
- Automation recipe: Matched devices use the same unit geometry (same finger W/L, same contact arrays, same head lengths) and the same variant; only counts differ for ratios.
- Beats hand layout because: variant identity is enforced for every matched group.
- Philis status: implemented (`same_variant_required: true` for matched groups, backend/annotator/src/constraints.rs:60).

### H01-37 Linewidth control vs local matching
- Kind: metric
- Statement: Linewidth control (worst-case random CD variation) ≈ 10 % of min feature and dominates narrow resistors and short-channel MOS spread, but "similar devices placed adjacent to one another will match far better than linewidth control suggests" because the contributing factors are common to neighbours.
- Source: §3.2.3 Linewidth Control L7680–7697 (PDF p.139).
- Philis stage: annotator, gp, dp
- Automation recipe: Score matching by a local-mismatch model (A_VT/√(WL), deck value) plus distance-dependent gradient terms; never budget matching from the absolute tolerance; keep matched members adjacent.
- Beats hand layout because: quantitative mismatch per pair drives the placement.
- Philis status: implemented (A_VT from Monte Carlo in the sidecar, pdks/sky130.json:93–96; proximity/matching rules in kernel/analog/src/placement/proximity.rs, matching_pair.rs).

### H01-38 Mask-alignment error budget and translation invariance
- Kind: heuristic
- Statement: Worst-case alignment ≈ 20 % of min feature; two alignment errors combine to ≈ 160 % of one; spacing/overlap/overhang rules between two layers carry that margin. Derivation (not stated in these chapters; Hastings treats orientation in Ch.13): a misalignment shifts all shapes of one mask by the same vector, so translated copies of a device see identical overlap errors while mirrored copies see opposite-sign errors in any mask-offset-sensitive dimension.
- Source: §3.2.3 Mask Alignment L7701–7737 (PDF pp.139–140).
- Philis stage: dp, cells
- Automation recipe: For matched groups prefer translated copies (same orientation) over mirrored copies unless a common-centroid arrangement needs mirroring; if mirroring is used, keep the mirrored set balanced (equal counts per orientation within each member).
- Beats hand layout because: orientation balance is counted exactly per member.
- Philis status: implemented in part (matched devices keep seeded orientation, no moves introduce mirrors, backend/dp/src/lib.rs:553–558).

### H01-39 OPC and neighbourhood identity
- Kind: heuristic
- Statement: Rule-based OPC adds corner corrections and spacing variations that depend on the distance to adjacent geometry; analog processes apply mild OPC on gate (and moat) layers and on damascene metal.
- Source: §3.3.4 L8370–8416 (PDF pp.149–150).
- Philis stage: cells, dp
- Automation recipe: Matched gates have identical poly neighbourhoods within the OPC interaction range (dummy poly at the same pitch on both sides); report asymmetric neighbours as a matching violation.
- Beats hand layout because: neighbourhood symmetry is checked for every matched gate.
- Philis status: partial (dummies required, backend/annotator/src/constraints.rs:61; no explicit OPC-range neighbour check).

### H01-40 CMP dishing and automatic dummy fill
- Kind: algorithm
- Statement: CMP dishes large recessed areas (effect visible over hundreds of µm); cure = arrays of unconnected metal/poly dummy geometries generated automatically; all generated shapes (including chip art) must obey design rules.
- Source: §2.7.5 L5783–5792 (PDF p.105); §2.3.1 L4194–4197; §3.3.1 L8219–8224.
- Philis stage: flow, verify
- Automation recipe: Post-route fill per deck density window, keeping out sensitive nets and matched devices (or filling them symmetrically); fill poly/active too where decks have density rules.
- Beats hand layout because: fill is density-exact and symmetric by construction.
- Philis status: implemented for metal (frontend/library/src/fill.rs:1–17; deck `m1..m4.density`, sky130.deck:500–503); poly/diff fill not present.

### H01-41 Poly as interconnect and dual-doped poly junctions
- Kind: rule
- Statement: Even low-resistivity poly has many times metal's resistance; avoid routing current-carrying signals through significant poly length; automated routers are not optimized for poly. In dual-doped poly, a P/N poly junction forms wherever the Ngate edge crosses poly; silicide shorts it; without silicide it obstructs current.
- Source: §2.7 L5451–5455 (PDF p.99); §2.7.3 L5671–5680; §4.3.2 Silicidation L10848–10854 (PDF p.193).
- Philis stage: dr, cells
- Automation recipe: Poly only inside generated cells (gate straps, short jumpers); router layers start at the first metal/local interconnect; any unsilicided poly (resistor body) must not straddle an N/P poly doping edge.
- Beats hand layout because: n/a (policy enforced).
- Philis status: implemented (routing metals exclude poly: pdks/sky130.json:22–29, pdks/gf180mcu.json:36–43).

### H01-42 Metallization and electromigration facts
- Kind: data-model
- Statement: EM becomes critical near 1e6 A/cm²; a fraction of a percent Cu in Al improves EM ×10; Cu has 35 % less resistance than Al and ×10 EM; RBM sidewall thinning is not an EM hazard; analog processes thicken the top metal for current; W plugs enable stacked vias.
- Source: §2.7.1 L5513–5523 (PDF p.100); §2.7.2 L5588–5593; §2.7.6 L5865–5866 (PDF p.107); §2.7 L5444–5448.
- Philis stage: dr, verify
- Automation recipe: Width/cut counts per segment from per-layer J_max × derating; prefer thick top metal for supply trunks; stack vias.
- Beats hand layout because: every segment is sized from its current.
- Philis status: implemented (EM limits per layer/cut, backend/verify/src/pdk.rs:23–35; deck EM rules, sky130.deck:491–495).

### H01-43 Temperature coefficients for a thermal-offset metric
- Kind: formula
- Statement: Diode/BJT V_F falls ≈ 2 mV/°C; avalanche BV rises ≈ +4 mV/°C (7 V junction); contact Seebeck coefficient 0.1–1.0 mV/°C (higher for lightly doped silicon), so millivolt-matched circuits degrade under self-heating gradients; junction leakage doubles every ≈ 8 °C; Tj,max 125–150 °C.
- Source: §1.2.2 L2379–2383, L2468–2475 (PDF pp.41, 43); §1.2.3 L2502–2503 (PDF p.43); §1.2.5 L2681–2688 (PDF p.46).
- Philis stage: gp, dp, annotator, verify
- Automation recipe: For each matched pair compute ΔT between members from the power map (existing superposition model) and report an equivalent input offset: BJT/diode `ΔV = 2 mV/°C · ΔT`; MOS `ΔV = TC_Vt · ΔT` (deck); for signal paths whose two contacts sit at different temperatures add `S · ΔT_contacts` (S from the deck, default range 0.1–1.0 mV/°C). For leakage-sensitive nodes (sampling capacitors, high-Z bias) penalize proximity to heat sources: leakage ×2^(ΔT/8 °C).
- Beats hand layout because: thermal symmetry becomes a number in µV per pair, optimized against every heat source at once.
- Philis status: partial (ΔT superposition, kernel/core/src/thermal.rs:1–14; MOS V_t tempco key, pdks/sky130.json:127; no diode/BJT V_F, Seebeck or leakage terms).

### H01-44 Package and assembly stress sources
- Kind: data-model
- Statement: Cu leadframes have a CTE far from silicon and stress the die during thermal cycling (Alloy 42 for low-stress parts); mold compound is ≈ 90 % silica to lower CTE; thick power copper adds CTE stress growing with thickness and die size; conductive-epoxy die attach is not a reliable electrical backside contact.
- Source: §2.8.1 L6061–6069, L6084–6092 (PDF p.112); §2.8.2 L6241–6246 (PDF p.114); §2.7.6 L5973–5978 (PDF p.110).
- Philis stage: flow, gp
- Automation recipe: Accept an optional block location on the die (centre distance, axis) and bias matched groups toward the block's stress-symmetry axis; never rely on backside contact for substrate biasing (always top-side taps).
- Beats hand layout because: die-level stress context becomes an explicit placement input.
- Philis status: missing (no die-location or package-stress input).

### H01-45 Threshold dependencies relevant to matching and device choice
- Kind: data-model
- Statement: V_t depends on V_SB (body effect), backgate doping, gate material (N+ vs P+ poly ≈ 1 V shift), oxide thickness, and oxide/interface charge; PMOS gates need P+ poly or a buried channel raises subthreshold leakage; subthreshold current is negligible 200–300 mV below V_t; analog processes target ≈ 0.7 V V_t and tolerate tiny leakage at V_GS = 0.
- Source: §1.4.1 L3329–3366 (PDF p.57); §1.4 L3194–3201; §4.2.1 L9540–9546 (PDF p.171); §4.3.2 L10762–10768.
- Philis stage: annotator
- Automation recipe: Matched-pair equality set = {model, W, L, nf, bulk net, V_SB (op-point), gate-poly type}; flag any difference as a mismatch source before layout.
- Beats hand layout because: silent V_SB or flavor differences inside a "matched" pair are caught automatically.
- Philis status: partial (size/model equality via pattern matching, backend/annotator/src/pattern.rs:106; no V_SB comparison).

### H01-46 Passive-device tolerance and breakdown tables as recipe data
- Kind: deck-requirement
- Statement: Table 4.5 (poly-gate CMOS; variability quoted at 15 µm width): poly 20 Ω/□, min drawn width 2 µm, BV > 100 V, ±30 %; PSD 50 Ω/□, 3 µm, 20 V, ±20 %; NSD 30 Ω/□, 3 µm, 20 V, ±20 %; N-well 2 kΩ/□, 5 µm, 40 V, ±40 %. Table 4.9 (analog BiCMOS poly): LSR 5 Ω/□, MSR 200 Ω/□, HSR 1 kΩ/□, min width 0.35 µm each, ±20/±20/±30 %; higher sheet → higher variability, so HSR is a compromise value. Gate-oxide capacitor: 0.86 fF/µm² at 400 Å, ±20 %, valid only with the well ≥ 1 V above the poly.
- Source: §4.2.3 Resistors, Table 4.5 L10203–10259 (PDF p.182); §4.3.3 Resistors, Table 4.9 L11299–11354 (PDF pp.200–201); §4.2.3 Capacitors L10262–10273 (PDF p.183).
- Philis stage: deck, annotator, cells, verify
- Automation recipe: Per resistor/capacitor recipe the sidecar carries `{sheet Ω/□ (or fF/µm²), tolerance %, min width, breakdown V, bias condition}`. The annotator picks the recipe for each schematic value by (a) precision class of the device's circuit role (divider/matched ratio → lowest tolerance; startup/bias → any), (b) `max |V|` across the body and to its isolation from the op-point ≤ BV (hard), (c) area = value/sheet × width². The generator then sizes width from tolerance and power (H01-20). PEX reads the same sheet value.
- Beats hand layout because: every resistor gets the area/tolerance/voltage trade computed, not the designer's habitual layer.
- Philis status: partial. Recipes are keyed by schematic model (pdks/sky130.json:137; backend/verify/src/pdk.rs:1073–1076) and sheet resistance comes from deck PEX (backend/verify/src/pdk.rs:677–686); `sheet_tolerance` is an empty object (pdks/sky130.json:74) with no consumer; no breakdown field.

### H01-47 Linewidth-dependent silicide resistance
- Kind: data-model
- Statement: TiSi₂ resistivity rises sharply in leads narrower than about 1 µm because the C49→C54 phase change needs room for larger grains; Ni and Co silicides do not show this. Silicided poly remains much more resistive than metal.
- Source: §2.7.3 L5646–5656, L5671 (PDF p.103); Ex. 2.13 L6376–6379.
- Philis stage: verify (PEX), cells
- Automation recipe: When a deck declares a Ti silicide, PEX takes silicided poly/active sheet resistance from a width-keyed table instead of one value; generators keep gate straps and poly jumpers at or above the knee width where resistance matters (gate resistance of wide RF/low-noise devices).
- Beats hand layout because: narrow-strap resistance is extracted instead of assumed.
- Philis status: missing (one `sheet_res_ohm_sq` per layer, backend/verify/src/pdk.rs:677–686); whether any shipped PDK uses Ti silicide was not checked.

### H01-48 Light sensitivity of bare and chip-scale-packaged dice
- Kind: heuristic
- Statement: Photons with wavelength below about 1100 nm generate electron-hole pairs in silicon (visible light 390–700 nm); camera flashes have made chip-scale-packaged ICs malfunction. The chapter states the failure only; it gives no layout remedy.
- Source: §1.1.1 L1780–1793.
- Philis stage: annotator, dr
- Automation recipe (inference, not from the source): when the flow is told the part is CSP/bare die, tag leakage-sensitive nodes (sampling/hold capacitors, bias nodes carrying ≤ µA per the op-point) and cover their junction areas with top metal tied to a quiet reference; report uncovered sensitive junction area. Cost = added capacitance on the shielded node.
- Beats hand layout because: sensitive junctions are enumerated from the netlist and op-point instead of remembered.
- Philis status: missing (the shield rule covers route coupling only, kernel/analog/src/routing/shield.rs:1; no package input).

## 4. Top-15 priorities for Philis

1. **H01-16** FET flavor recipes with oxide/V_t marker layers (hvi, dual-gate, lvtn/hvtp): today any 5 V/thick-oxide model is drawn without its marker, so LVS/DRC of I/O devices cannot pass; deck already recognizes them.
2. **H01-01** Well/substrate reverse-bias ERC from op-point node voltages (add node voltages to `OpPoint`): the deck cannot check well bias, and forward-biased wells are latchup and injection failures.
3. **H01-07** Enforce `tie_max_dist_nm` (key already in the sidecars, unused): cheap exhaustive latchup tap-distance check plus dp penalty.
4. **H01-02** Net voltage ranges + voltage-dependent well/diffusion spacing (Table 3.1 formula or deck per-voltage rules): required for any > core-voltage design; the deck explicitly cannot express it.
5. **H01-15** Hot-carrier operating vs blocking voltage check per device from the op-point: catches wrong-flavor devices before layout; also drives H01-16 flavor choice.
6. **H01-06** Well partitioning by bulk/supply and V_SB identity for matched pairs; flag NMOS bulks that need an isolated P-well.
7. **H01-05** Well sharing as a placement objective instead of post-placement bridging (placement currently forbids merges via widest-layer clearance): large area win for PMOS-heavy blocks.
8. **H01-09** Butted source–tap variant when S = B, plus the N+/P+ abutment ERC: area win per device, cheap to add as a generator alternative.
9. **H01-43** Thermal-offset metric in µV (V_F −2 mV/°C, Seebeck, V_t TC, leakage ×2/8 °C): turns existing ΔT data into circuit-level numbers.
10. **H01-30 / H01-29** GDS TEXT pin labels and hierarchical SREF output: external LVS and hierarchical DRC need them.
11. **H01-23** MOS capacitor generator with bias-polarity check and plate orientation: common analog component currently unsupported.
12. **H01-14** Asymmetric-device model (drift side pinned, S/D swap forbidden) and bias-derived S/D identity.
13. **H01-08** Injector-current-proportional substrate contact sizing and placement on the aggressor–victim path.
14. **H01-20 / H01-46** Power-driven poly resistor sizing, ESD-path flagging, and recipe choice from tolerance/breakdown tables (`sheet_tolerance` is empty today).
15. **H01-03 / H01-04** Rule-derivation lint (empty `linewidth_control_nm`, unused `n_well_depth`) and outdiffused well outlines for WPE/spacing measurements.
