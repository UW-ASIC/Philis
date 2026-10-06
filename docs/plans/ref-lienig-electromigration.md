# Lienig & Thiele, *Fundamentals of Electromigration-Aware Integrated Circuit Design* (Springer 2018): study for Philis

- Source: J. Lienig, M. Thiele, *Fundamentals of Electromigration-Aware Integrated Circuit Design*, Springer 2018, ISBN 978-3-319-73557-3. PDF: `docs/ref/Fundamentals of Electromigration-Aware Integrated Circuit -- Jens Lienig,Matthias Thiele ... .pdf`.
- Reftext: `scratchpad/reftext/lienig_em.txt` (7770 lines).
- Range read: lines 1–7770, the whole file (front matter, Chapters 1–5, all reference lists, index).
- Page convention: "PDF p." = 1 + number of form feeds before the line. Book page = PDF page − 12 (book p.1 = PDF p.13).

---

## 1. Coverage

### 1.1 Read chunks (Read tool, consecutive, no gaps)

The first 2000-line attempts exceeded the tool's token limit and returned nothing, so the file was re-read in 1000-line chunks:

| # | offset..end | content |
|---|---|---|
| 1 | 1..1000 | cover, foreword, preface, contents, Ch. 1, Ch. 2 start (2.1) |
| 2 | 1001..2000 | 2.1–2.4.4 |
| 3 | 2001..3000 | 2.4.4–2.6.3 |
| 4 | 3001..4000 | 2.6.3–2.6.4, Ch. 2 refs, Ch. 3 up to 3.3.3 |
| 5 | 4001..5000 | 3.3.3–3.7, Ch. 3 refs |
| 6 | 5001..6000 | Ch. 3 refs end, Ch. 4 4.1–4.5.2 |
| 7 | 6001..6900 | 4.5.2–4.9.2 |
| 8 | 6901..7770 | 4.9.2–4.10, Ch. 4 refs, Ch. 5, index |

PDF pages opened to check equations the text extraction garbled: p.42 (Eq. 2.6–2.8), p.66 (Eq. 2.22), p.88 (Eq. 3.5b–3.7), p.94 (Eq. 3.11–3.12), p.96 (Eq. 3.13–3.19), p.100 (Eq. 3.21–3.23), p.101 (Eq. 3.24–3.26), p.120 (Eq. 4.2–4.3), p.123 (Eq. 4.6–4.9).

### 1.2 Every heading in the range (reftext line)

- Front matter: Foreword L92; Preface L162; Contents L257.
- **Ch. 1 Introduction** L369: 1.1 Development of Semiconductor Technology L392; 1.2 Interconnect Development L519; 1.3 The Rise of Electromigration L605; 1.4 Motivation and Structure of This Book L766; References L907.
- **Ch. 2 Fundamentals of Electromigration** L945: 2.1 Introduction L975; 2.2 Electromigration Quantification Options L1201; 2.3 Design Parameters L1270; 2.3.1 Technology L1281; 2.3.2 Environment L1344; 2.3.3 Design L1411; 2.4 Electromigration Mechanisms L1457; 2.4.1 Crystal Structures and Diffusion Mechanisms L1466; 2.4.2 Barriers of Copper Metallization L1631; 2.4.3 Frequency Dependency of Electromigration L1765; 2.4.4 Mechanical Stress L1972; 2.5 Interaction of Electromigration With Thermal and Stress Migration L2057; 2.5.1 Thermal Migration L2093; 2.5.2 Stress Migration L2163; 2.5.3 Mutual Interaction of EM, Thermal and Stress Migration L2282; 2.5.4 Differentiation of EM, Thermal and Stress Migration L2440; 2.6 Migration Analysis Through Simulation L2564; 2.6.1 Simulation Techniques L2578; 2.6.2 Atomic-Flux Simulation L2910; 2.6.3 Simulation of Mechanical Stress L2990; 2.6.4 Void-Growth Simulation L3064; References L3142.
- **Ch. 3 Integrated Circuit Design and Electromigration** L3285: 3.1 Design Flow of Integrated Circuits L3326; 3.2 Electromigration-Aware Design Flows L3448; 3.2.1 Analog Design L3501; 3.2.2 Digital Design L3597; 3.3 Determination of Currents L3717; 3.3.1 Current Types L3745; 3.3.2 Terminal Currents L3790; 3.3.3 Segment Currents L3975; 3.4 Determination of Current-Density Limits L4145; 3.4.1 Mission Profile L4166; 3.4.2 Application-Robust Current-Density Limits L4247; 3.5 Current-Density Verification L4487; 3.5.1 Methodology L4517; 3.5.2 Current-Required Wire and Via Sizes L4587; 3.5.3 Net Terminal Connections L4687; 3.5.4 Simulation Methods for Electromigration Processes L4703; 3.5.5 Current-Density Simulation L4770; 3.6 Post-Verification Layout Adjustment L4842; 3.7 Design Options for Electromigration Avoidance L4901; References L4969.
- **Ch. 4 Mitigating Electromigration in Physical Design** L5060: 4.1 Overview of Presented Measures and Effects L5087; 4.2 Bamboo Effect L5161; 4.2.1 Fundamentals L5163; 4.2.2 Applications L5259; 4.3 Critical Length Effects L5325; 4.3.1 Fundamentals L5327; 4.3.2 Applications L5615; 4.3.3 Linked Segments L5772; 4.4 Via-Below and Via-Above Configurations L5823; 4.4.1 Fundamentals L5825; 4.4.2 Parameters L5871; 4.4.3 Applications L5923; 4.5 Reservoirs L5947; 4.5.1 Fundamentals L5949; 4.5.2 Sources and Sinks L5971; 4.5.3 Reservoir Types L6007; 4.5.4 Applications L6044; 4.6 Multiple Vias L6114; 4.6.1 Fundamentals L6116; 4.6.2 Current Distribution L6186; 4.6.3 Vias With Reservoirs L6217; 4.6.4 Geometrical Configuration L6234; 4.6.5 Applications L6320; 4.7 Frequency-Dependent Effects L6373; 4.7.1 Self-Healing and Rising Frequencies L6375; 4.7.2 Applications L6434; 4.8 Materials for Classical Metal Routing L6482; 4.8.1 Interconnect L6522; 4.8.2 Dielectric L6578; 4.8.3 Barrier L6639; 4.9 New Materials and Technologies L6729; 4.9.1 Carbon-Based Solutions L6740; 4.9.2 CNT Properties L6833; 4.9.3 Applications L6912; 4.10 Summary L6967; References L7014.
- **Ch. 5 Summary and Outlook** L7318: 5.1 Summary of Electromigration-Inhibiting Measures L7338; 5.2 Outlook: Segment Lengths L7465; 5.3 Outlook: Library of Electromigration-Robust Elements L7523; 5.4 Outlook: New Technologies L7573; 5.5 Electromigration-Aware Design: Driven by Constraints L7596; References L7651; Index L7657.

---

## 2. Section-by-section digest

### Front matter (L1–368, PDF p.1–12)
- The cover diagram (L22–53) already shows the analog EM flow used later as Fig. 3.2: floorplanning → placement → current-driven routing → current-density verification → (violations → current-driven layout decompaction) → EM-robust design.
- Preface (L170–179): 1 FIT is one failure in 10⁹ device-hours, about 114,000 years.
- Acknowledgement (L241–244) credits G. Jerke (Bosch) for the application-robust current-density limits of Sect. 3.4.

### 1 Introduction (L369–388, PDF p.13)
- Two opposing trends motivate the book. Required current densities rise because cross-sections shrink faster than currents do. At the same time the tolerable density limits fall (L379–386).

### 1.1 Development of Semiconductor Technology (L392–515, PDF p.13–15)
- Table 1.1 (L456–486) gives ITRS values for Cu at 105 °C, 2016→2028. Maximum tolerable DC-equivalent current density falls 3.0 → 0.2 MA/cm². The density needed to drive four inverter gates rises 1.81 → 5.35 MA/cm². M1 half-pitch shrinks 28.3 → 7.1 nm at aspect ratio 2.0–2.2.
- The derived rows use T = A/R·W, A = W·T and I = J·A (L482–484).

### 1.2 Interconnect Development (L519–601, PDF p.16–17)
- Moving from Al to Cu changed the failure modes: Cu resists migration better, but other diffusion paths now dominate, which forces barrier layers (L526–533).
- Low-k dielectrics are less stiff, so their interconnects are more EM-prone. The lower Young's modulus gives less mechanical load to stop extrusion (L540–547).
- TSVs remove routing area and cause mechanical tension near themselves (L597–601).

### 1.3 The Rise of Electromigration (L605–762, PDF p.17–20)
- Failure modes are opens (voids) and shorts (hillocks, whiskers). They appear after weeks to years (L611–616).
- Eq. 1.1: J = I/A (L652–657).
- Trends from ITRS (L735–758): currents halve about every 5 years. Cross-sections halve in about 3 years. Permissible J halves every 3 years. Required J doubles about every 8 years.

### 1.4 Motivation and Structure (L766–903, PDF p.20–23)
- Analog design already routinely uses current-dependent routing and widens highly loaded tracks. Digital design cannot widen all wires (L832–840).
- EM measures pay off most in physical design, especially routing. Later fixes are less effective, but currents are not exact until a topology exists (L852–859).

### 2 Fundamentals of Electromigration (intro L945–971, PDF p.25)
- The chapter covers physics (2.1), quantification (2.2), parameters (2.3), mechanisms (2.4), coupling with thermal and stress migration (2.5) and simulation (2.6).

### 2.1 Introduction (L975–1197, PDF p.25–30)
- Two forces act on the ions: F_field (usually negligible because of screening) and F_wind (electron momentum transfer, dominant). Material moves cathode → anode, with the electron flow (L1014–1028).
- Flux divergence causes damage at inhomogeneities: line ends, direction changes, layer changes, cross-section changes, material or lattice changes, existing damage, temperature gradients and stress gradients (L1044–1061).
- Line depletion: electrons flow via→line. Via depletion: electrons flow line→via. A larger line-width/via-width ratio loads the via more (L1065–1113, Fig. 2.3).
- A positive feedback loop drives failure: void → higher local J → Joule heating → faster diffusion → bigger void (L1114–1120, Fig. 2.4).
- Solid-state EM happens at j > 10⁴ A/cm². Electrolytic (wet) EM happens at < 100 °C, needs moisture and is excluded from the book (L1128–1197).

### 2.2 Quantification Options (L1201–1266, PDF p.30–31)
- Black's equation (Eq. 2.1): MTF = A·j⁻²·exp(Ea/kT). Later versions replace the exponent 2 with n. n = 1 means void-growth-limited failure; n = 2 means nucleation-limited (L1216–1233).
- Al: Ea ≈ 0.7 eV with n = 2 (grain boundary). Cu: Ea = 0.9 eV (surface) with n = 1.1–1.3 (L1241–1245).
- Black does not model the change of mechanism at steep current rise. It covers only linear lines, with no bends, layer changes or material transitions, and A and Ea are technology-specific (L1248–1259).
- The Li/Tan Eyring-based model has more parameters, but they are physically determinable (L1260–1266).

### 2.3 Design Parameters (L1270–1277, PDF p.31)
- Parameters fall into three groups: technology (materials), environment (temperature) and design (current density).

### 2.3.1 Technology (L1281–1340, PDF p.31–33)
- Ea depends on the diffusion site: bulk > grain boundary, and surface. Al is grain-boundary-dominated (≈0.7 eV). Cu is boundary- or surface-dominated (0.8–1.2 eV) (L1299–1321).
- Stiff dielectrics (high Young's modulus) help stress migration counteract EM (L1322–1329).
- Technology fixes the geometry rules (width, spacing, overlap, density, via size) that shape EM behavior (L1330–1340).

### 2.3.2 Environment (L1344–1407, PDF p.33–34)
- Automotive ambient can reach 175 °C (L1348–1351).
- Cu: +10 K requires more than a 50 % current cut for the same MTF. −5 K allows about +25 % j (L1362–1367).
- Fig. 2.7 (Al): going from 25 to 125 °C requires about a 90 % cut in J_max (L1399–1403).
- Joule heating causes temperature gradients, and those cause thermal migration (L1368–1373).

### 2.3.3 Design (L1411–1453, PDF p.34–35)
- Design sets J through I and A, so width must be adapted to current (L1413–1420).
- Direction and layer changes raise local J. Fig. 2.8 compares 90°, 135° and 150° bends: 90° bends must be avoided, and 135° is markedly better (L1421–1445).
- Shorter lines benefit from the Blech length (L1428–1432). Current reversal lets damage partly heal (L1448–1453).

### 2.4 EM Mechanisms (L1457–1462, PDF p.35)
- The mass flux follows the electron flow. Subsections cover material, frequency and stress.

### 2.4.1 Crystal Structures and Diffusion (L1466–1627, PDF p.35–38)
- Structure types: amorphous, polycrystalline, near-bamboo, bamboo and monocrystalline (Fig. 2.9). In practice interconnects are polycrystalline (L1468–1534).
- Eq. 2.2 (diffusion): ∂c/∂t = D·∂²c/∂x². Eq. 2.3 (drift velocity): v = (D/kT)·e·z*·ρ·j. Eq. 2.4: D = D_v + δ·D_b/d (L1539–1570).
- Table 2.1, Ea in eV (L1582–1588): Al bulk 1.2, grain boundary 0.7, surface 0.8. Cu bulk 2.3, grain boundary 1.2, surface 0.8.
- Two-thirds of Al failures sit at transitions between very different grain sizes [AR70]. Near-bamboo triple points and blocking grains are divergence sites (L1600–1626).

### 2.4.2 Barriers of Copper Metallization (L1631–1761, PDF p.39–41)
- In the (dual) Damascene process, the metal liner (Ta/TaN) conducts current and keeps some conductance through a void. The dielectric cap (SiN, SiCN) sits on top (L1637–1731).
- Suppressing one diffusion path makes another one dominant (L1737–1748).
- CMP damages the top surface, so voids typically form at the top of a line. This is why via-above is worse (Sect. 4.4) (L1749–1761).

### 2.4.3 Frequency Dependency (L1765–1968, PDF p.41–45)
- Self-healing (checked on PDF p.42), Eq. 2.6: J_net = J_fwd − J_back = J_fwd·(1 − γ).
- Eq. 2.7: MTF_AC = A/(r·j⁺ − γ(1 − r)·j⁻)ⁿ·exp(Ea/kT).
- Eq. 2.8: γ = (r·j⁺/j_DC − s·MTF_DC/MTF_AC)/((1 − r)·j⁻/j_DC). r is the duty factor and s is fitted iteratively (L1785–1813).
- In Cu, MTF_AC/MTF_DC rises by ×500 over 10–10⁴ Hz [TCH93] (L1814–1817, Fig. 2.13). The lifetime stays finite at high f because of thermal migration (L1848–1851).
- Supply nets carry mostly DC, so signal and supply nets must be treated differently (L1856–1861).
- Shono et al.: for a waveform with zero DC but time-asymmetric pulses, the minimum lifetime is at duty factor ≈ 0.4 (L1863–1883).
- Skin effect: Eq. 2.9 δ = √(2ρ/(ωμ)); Eq. 2.10 j ≈ j_S·exp(−d/δ); Eq. 2.11 is the cylindrical Bessel approximation. Cu δ ≈ 9.4 mm at 50 Hz, scaling as 1/√f. The critical frequency is 90 GHz for W = t = 0.45 µm and about 35 THz at 22 nm (L1885–1968).

### 2.4.4 Mechanical Stress (L1972–2053, PDF p.45–47)
- Stress sources are (i) cooling from about 500 °C deposition with CTE mismatch, (ii) uneven layer growth (the larger effect) and (iii) EM vacancy redistribution (L1974–1989).
- Stress-free temperature ≈ 250 °C. α_Cu = 16.5·10⁻⁶ K⁻¹, α_SiO2 = 0.5·10⁻⁶ K⁻¹ (L1995–2002).
- Eq. 2.12: σ/E = ε = α·ΔT. Eq. 2.13: σ = (α_SiO2 − α_Cu)·ΔT/(1/E_SiO2 + 1/E_Cu). With E_Cu = 117 GPa, E_SiO2 = 70 GPa and ΔT = 200 K this gives ≈ 140 MPa tensile (L2003–2021).
- Tensile stress creates voids. Past a compressive threshold, extrusions (hillocks, whiskers) form, and those are irreversible (L2023–2053).

### 2.5 Interaction with Thermal and Stress Migration (L2057–2089, PDF p.47–48)
- Thermal migration (TM) moves material down the temperature gradient. Stress migration (SM) moves it from compressive to tensile regions.
- The Blech effect is the equilibrium between EM and SM. The Soret effect is the equilibrium between TM and chemical diffusion (L2085–2089).

### 2.5.1 Thermal Migration (L2093–2159, PDF p.48–49)
- Gradient sources are Joule heating, nearby hot transistors, and cooling through TSVs or heat sinks combined with poor conduction (L2130–2137).
- TM is weak inside ICs because the metals are pure and the conductivities high. It is strong in solder: 10 K across a 100 µm bump gives 1000 K/cm (L2153–2159).

### 2.5.2 Stress Migration (L2163–2243, PDF p.49–51)
- SM is also called stress voiding (SIV). Its drivers are CTE mismatch, EM itself and packaging. TSVs raise the stress and make it less uniform (L2165–2202).
- Eq. 2.14: σ = E·ε (Hooke). Vacancy supersaturation nucleates voids, and the void resets local stress (L2208–2243).

### 2.5.3 Mutual Interaction (L2282–2436, PDF p.52–55)
- SM opposes EM, and TM's direction is independent of current (L2285–2293). j → T via Joule heating, T → σ via CTE, and T and σ → D (Fig. 2.20, L2298–2329).
- Eqs. 2.15–2.17 give the fluxes: J_E = (c/kT)·D₀·exp(−Ea/kT)·z*eρj; J_T = −(cQ*/kT²)·D₀·exp(−Ea/kT)·∇T; J_S = (cΩ/kT)·D₀·exp(−Ea/kT)·∇σ. Eq. 2.18: J_a = J_E + J_T + J_S (L2383–2412).
- Eq. 2.19 (1-D, as printed): J_a = (Dc/kT)·ρjz*e + (Dc/kT)·Ω·∂σ/∂x. Eq. 2.20: D = D₀·exp(−Ea/kT). Setting net flux to zero is the prevention principle (L2418–2436).

### 2.5.4 Differentiation of EM/TM/SM (L2440–2553, PDF p.55–57)
- Damage cannot be told apart by appearance, only by location. EM sits at high J and current crowding at bends and vias. TM sits near hotspots and can shift toward cool spots. It shows in AC lines (L2443–2503).
- TSVs: α_Si ≈ 3·10⁻⁶ K⁻¹ versus α_Cu ≈ 16.5·10⁻⁶ K⁻¹. Keep active devices out of a keep-out zone to limit stress-induced mobility change (L2542–2550).

### 2.6 Migration Analysis Through Simulation (L2564–2574, PDF p.58)
- The simulation hierarchy is current density → atomic flux → stress → void growth.

### 2.6.1 Simulation Techniques (L2578–2907, PDF p.58–64)
- Methods: analytical; quasi-continuous ("power blurring", usable full-chip for temperature); lumped (one element per segment, fast, no spatial resolution); meshed (FEM/FVM/FDM) (L2580–2667).
- FEM on all nets does not scale (Fig. 2.28). Filtering critical nets [JL10] will not be enough for digital designs (L2669–2681).
- Proposal [TBL17]: FEM-verify a library of routing patterns in advance, then build layouts only from them (Fig. 2.29) (L2736–2747).
- Partition models where the boundary conditions are homogeneous: straight wire away from vias and branches, with "appendices" added to via-only patterns. Diffusion barriers are natural cut points for flux (L2753–2778).
- The maximum error at the cut interface is 3 % (Figs. 2.30–2.32). One library characterization can be faster than one full FEM run (L2779–2857).

### 2.6.2 Atomic-Flux Simulation (L2910–2986, PDF p.64–66)
- The flux has units of atoms/(m²·s) or kg/(m²·s). Eq. 2.21 (atomistic): v = W_p·E_b·(1/m) (L2912–2937).
- Quasi-static flux divergence locates critical regions (Fig. 2.34, via depletion). Microstructure needs probabilistic treatment [COS11] (L2945–2986).

### 2.6.3 Simulation of Mechanical Stress (L2990–3060, PDF p.66–67)
- Korhonen (Eq. 2.22, checked on PDF p.66): ∂σ/∂t = ∂/∂x[(D_a·B·Ω/kT)·(∂σ/∂x − z*eρj/Ω)] (L3001–3011).
- The steady state is linear stress in a short wire (immortal, Fig. 2.36). Crossing σ_critical starts nucleation and is equivalent to a small resistance rise. The spread is large, so the treatment must be probabilistic (L3046–3060).

### 2.6.4 Void-Growth Simulation (L3064–3138, PDF p.67–69)
- A transient simulation is required. Two options: mesh-geometry modification [OO01] or deletion of elements past a flux-divergence limit [WDY03], plus surface-tension modelling (L3099–3138).

### 3 IC Design and Electromigration (intro L3285–3315, PDF p.73)
- Knowledge of the currents is the prerequisite. The key parameter is the maximum permissible J, derived from mission profiles.

### 3.1 Design Flow of ICs (L3326–3445, PDF p.74–76)
- Standard physical-design steps: partitioning, chip planning (floorplan plus power routing), placement, CTS, global and detailed routing, timing closure. Verification covers DRC, LVS, parasitic extraction, antenna and ERC.

### 3.2 EM-Aware Design Flows (L3448–3498, PDF p.76–77)
- Constraints are the simulated currents plus the permissible J. Place blocks to minimize current flows and size widths to the currents (L3457–3462).
- For multi-terminal nets, choosing Steiner points can reduce segment currents [LJ03, LKL+12] (L3463–3470).
- Digital net classes differ in EM susceptibility: power is DC, clock and signal are AC, and so they need different limits (L3492–3498).

### 3.2.1 Analog Design (L3501–3593, PDF p.77–79)
- Analog currents range from nA (sensors) to several A (power). Power and signal currents are of the same order, so analog cannot filter signal nets out (L3521–3532).
- Fig. 3.2 flow: current characterization, current propagation through hierarchy, current-aware PCells, current-aware route planning, current-density verification, and on violation current-driven layout decompaction (L3533–3561).
- Current-driven routing is correct by construction. It needs (1) current-density-correct terminal connections and (2) a planned topology that minimizes segment currents. The primary goal is minimum routing area (L3537–3572).

### 3.2.2 Digital Design (L3597–3713, PDF p.79–82)
- Three net classes: power (DC-dominant, high current), clock (nearly symmetric AC, high RMS because of fan-out) and signal (asymmetric pulsed AC, low RMS) (L3642–3648).
- Filtering must never drop critical nets (L3649–3654). Commercial tools use three global limits: peak, average and RMS (L3703–3711).

### 3.3 Determination of Currents (L3717–3741, PDF p.82)
- Use transient SPICE or quasi-static methods. For each node build a vector of RMS, min/max mean and peak, because the worst cases occur in different operating modes. The dimensioning current is a frequency-dependent weighting (L3719–3733).

### 3.3.1 Current Types (L3745–3786, PDF p.82–83)
- RMS applies for f < 1 Hz, analog DC nets and reliability-critical long pulses. It is conservative (no self-healing) and includes Joule self-heating, which often limits the top metals (L3759–3773).
- Average applies for f > 1 Hz, digital and analog signal nets. It includes self-healing and is the basis for Black-law lifetime (L3774–3778).
- Peak applies to single events (ESD, short power-stage pulses), i.e. EOS rules (L3779–3782).
- Evaluate all three together and pick whichever needs the most vias or the widest wire per segment, layer and temperature (L3783–3786).

### 3.3.2 Terminal Currents (L3790–3971, PDF p.83–87)
- One max-|equivalent| value per terminal fails at Steiner points when terminals have reversed worst cases (Fig. 3.4) (L3796–3827).
- Use lower and upper bounds (avg±, RMS±, peak±). RMS⁻ is computed from the zero and negative parts of the waveform and multiplied by −1 (L3828–3836).
- Model 1 (Eq. 3.1): per terminal, KCL-consistent current vectors at every terminal's min and max instants, up to m pairs. It needs all waveforms and over-sizes (L3842–3897).
- Model 2 (Eq. 3.2): per terminal, time slots S_x with [i_min, i_max]. Slots map to operating phases and mission profiles (L3898–3916).
- Model 3 (Eqs. 3.3–3.4): per terminal n, matrices L_n and U_n of size O (current types) × P (operating phases). Storage is 2·N·O·P values; a 3-pin DC net with O = 3, P = 1 gives 6·N values. Sparse storage is possible (L3917–3971).

### 3.3.3 Segment Currents (L3975–4141, PDF p.87–91)
- Cut the segment to split terminals into LHS and RHS sets. Eq. 3.5a/b: sum the L and U bounds on each side (L3980–4016).
- Eq. 3.6 (PDF p.88): i_W,op = max{min(|i_L,LHS,op|, |i_U,RHS,op|), min(|i_L,RHS,op|, |i_U,LHS,op|)}. Eq. 3.7: i_W,o = max_p(i_W,op) (L4017–4035).
- Example: VT1 in [−1, +2] mA and VT2 in [−3, 0] mA give i_W = 2 mA (L4036–4043).
- The method also works when terminal currents do not satisfy KCL. Summing annotated equivalents is always ≥ the equivalent of the vector sum, i.e. safe (L4044–4052).
- Cyclic conflict: the topology decides the currents and the currents should decide the topology (Fig. 3.9). The fix is a current-driven wire-planning step that minimizes interconnect area rather than length (Fig. 3.10, [LJ03]). Detailed routing then reduces to point-to-point with known widths (L4053–4141).

### 3.4 Determination of Current-Density Limits (L4145–4162, PDF p.91)
- The limit depends on the application: temperature and loads come from a mission profile.

### 3.4.1 Mission Profile (L4166–4243, PDF p.91–93)
- Standard profile classes are consumer, automotive, industry, medical and aerospace. Operating states give stimuli, stimuli give terminal currents, and those give layout sizes (L4169–4182).
- Any change to the temperature profile or phase durations requires re-verifying the same layout. The layer chain is characteristic (char) → reference (ref) → effective (eff) rules (Fig. 3.11) (L4185–4238).

### 3.4.2 Application-Robust Current-Density Limits (L4247–4484, PDF p.93–98)
- Eq. 3.8: MTF = t50 = A·j⁻ⁿ·exp(Ea/kT). Eq. 3.9: j = i/A_wire (L4249–4275).
- Eq. 3.10: t_life,ref = AF_TF·AF_T·AF_q·t50,char. The typical target is 10 years at T_ref = 378 K (105 °C); 15 years gives AF_TF = 1.5 (L4283–4298).
- Eq. 3.11: AF_T = exp[(Ea/(n·k))·(1/T_ref − 1/T_char)]. Eq. 3.12: AF_q = S1/exp[norminv(q_ref)·σ], with 1 < S1 ≤ 10 and σ the lognormal sigma. Characterization runs at > 473 K (L4299–4325; PDF p.94).
- Eq. 3.13: j_ref = j_char(T_char)/(AF_T·AF_q·AF_TF). Eq. 3.17: q_eff ≪ 1/m, where m = number of routed interconnect segments (L4401–4422; PDF p.96).
- Eq. 3.18: 1/T_eff = 1/T_ref − (k/Ea)·ln(t_life,eff/t_life,ref). Eq. 3.19: t_life,eff = Σ_s t_life,s·exp[(Ea/(n·k))·(1/T_ref − 1/T_s)]. Assumes no self-heating. Compute T_eff per layer (Ea and n are layer-specific) and use the highest (L4423–4469).
- Eq. 3.20: j_eff = j_ref(T_ref)/(AF_T(T_ref, T_eff)·AF_q(q_ref, q_eff)·AF_TF) (L4470–4476).

### 3.5 Current-Density Verification (L4487–4513, PDF p.98)
- Check that every worst-case J ≤ the application-robust limit. Violations show up in via arrays and in wires (Fig. 3.14).

### 3.5.1 Methodology (L4517–4585, PDF p.99–100)
- Inputs: current bounds, geometry, layer thickness, temperature (average or field map) and per-layer limits (L4519–4526).
- Fast method: compare the required cross-section per segment and via with the drawn one. Precise method: FEM (L4527–4539).
- FEM flow [JL04]: (1) check terminals; (2) de-select nets whose Σ worst-case terminal currents < I_max of the minimum-sized metallization; (3) compute J per element against the layer limit; (4) drop dummy errors such as corner spots (L4540–4574).

### 3.5.2 Current-Required Wire and Via Sizes (L4587–4660, PDF p.100–101)
- Eqs. 3.21–3.23: w_nom = max(i_w,eq/(j_eff,eq·h_nom), i_w,peak/(j_eff,peak·h_nom), w_min_process).
- Eq. 3.24: w_eff = w_nom·(h_nom/h_min) + Δw + w_etch.
- Eq. 3.25: n_via = ceil(i_w,eq/i_single_via(T_ref)·f(T_eff)·g(H)). g(H) = 1 for homogeneous flow and > 1 otherwise (from FEM).
- Eq. 3.26 (PDF p.101): f(T_eff) = exp(−(Ea/(n·k·T_ref))·(1 − T_ref/T_eff)), usually with 1 ≤ n ≤ 2, and f = 1 when T_eff = T_ref.
- **Source defect:** with Eq. 3.26 as printed, f < 1 when hotter, so Eq. 3.25 as printed gives a hotter via fewer cuts. f is the allowed-current factor and must divide. Philis already does this (`kernel/analog/src/routing/em.rs:60-63`).

### 3.5.3 Net Terminal Connections (L4687–4699, PDF p.102)
- Analog pins vary in shape, and the connection point changes the current load inside the pin. Compute the ampacity of each pin region and remove regions whose ampacity is below the arriving wire current as candidate access points (Fig. 3.15, U-shaped pin with regions carrying 3, 2, 1 and 0.5 mA).

### 3.5.4 Simulation Methods (L4703–4766, PDF p.102–104)
- EM is stochastic, so treat simulated results as random variables or apply safety factors (L4708–4717).
- The method list is repeated. FEM and FDM suit the diffusion/heat equation, and multi-physics FEM gives thermal and stress fields (L4719–4766).

### 3.5.5 Current-Density Simulation (L4770–4838, PDF p.104–105)
- Usually needs 3-D. 2.5-D is adequate only for coarse structures. Symmetry can at best halve IC structures (L4772–4783).
- Eq. 3.27: j = σ·E. Eq. 3.28: E = −grad Φ. Boundary conditions: fixed potential on one face and fixed j on another (Fig. 3.16). The matrix is sparse and linear (L4784–4818).
- Fast mode computes only the driving-force fields (j, T, σ) to locate risk. Growth rate and TTF additionally need the critical void volume (L4831–4838).

### 3.6 Post-Verification Layout Adjustment (L4842–4897, PDF p.105–106)
- Current-driven decompaction [JLS04] avoids another P&R iteration. Steps: (1) decompose into segments whose ends are sources or sinks; (2) size wires and via arrays from the located currents; (3) add support polygons at bends and terminals where widening is impossible or insufficient; (4) decompact while preserving topology.

### 3.7 Design Options for EM Avoidance (L4901–4965, PDF p.107–108)
- Five levers: limit J, limit T, layout modifications, material modifications, new materials (L4908–4915).
- Global widening conflicts with scaling (L4916–4924). Thermal vias and wires lower T by only a few K (L4925–4949).
- The book focuses on layout modifications (L4954–4965).

### 4 Mitigating EM in Physical Design (intro L5060–5083, PDF p.111)
- Each measure is a local layout change that raises the approved J limit.

### 4.1 Overview (L5087–5157, PDF p.111–113)
- Table 4.1 (L5147–5157) maps each effect to its design parameters. Bamboo: W, material, technology. Blech: L, material, technology. Via effects: L, material, technology. Reservoir: L, W, f, material, technology. Via configuration: L, W, technology. Self-healing: f. Passivation and immunity: material, technology.

### 4.2.1 Bamboo Fundamentals (L5163–5255, PDF p.113–115)
- MTF against width is non-monotonic (Fig. 4.1). It falls in the near-bamboo regime and rises below about half the grain size. A thermal self-heating limit bounds how narrow a wire can go (L5174–5223).
- Slotting or cheesing wide lines (Fig. 4.2: rectangular or octagonal cutouts) is mainly a CMP measure, but it creates parallel bamboo strips (L5224–5232).
- The effect is strong in Al and weak in Cu, where surface diffusion dominates (L5245–5249).

### 4.2.2 Bamboo Applications (L5259–5321, PDF p.115–116)
- Al < 2 µm; Cu < 1 µm with a 3 h anneal at 400 °C. Effective only if w/D50 < 0.5 (L5261–5268).
- Scanned laser annealing reached 4 µm grains, versus 0.13 µm at 275 °C for 24 h (L5269–5273). Most of these processes are not manufacturable (L5276–5281).

### 4.3.1 Critical Length Fundamentals (L5327–5611, PDF p.116–123)
- Definition of a segment: the conductor between two vias or contacts or between in-plane branches, with no branch inside; it is a graph edge with an anode and a cathode end (L5370–5377, Fig. 4.5).
- Steady state: linear stress. If σ_critical is not reached there is no damage (L5402–5412).
- Eq. 4.1: (jL)_Blech = Ω·Δσ/(e·z*·ρ) (L5413–5429).
- Voids form before extrusions, because lines start in tension and the tensile threshold is lower (L5432–5438).
- Void-growth saturation, Eq. 4.2 (PDF p.120): (jL)_sat < (ρ/A)/(ρ_l/A_l)·(ΔR_fail/R)·2ΩB/(ez*ρ) (L5439–5485).
- The jL² criterion [LDP+09], Eq. 4.3: V_sat = e·z*·ρ·A_Cu·jL²/(2ΩB). It is less restrictive for segments < 20 µm and proportional to the absolute ΔR, i.e. the IR drop (L5492–5510).
- A void spans the full cross-section at V_void = H²·W. Saturated growth below that makes the line immortal (L5535–5550).
- Eq. 4.4: ΔR_sat = V_sat·ρ_b/(A_Cu·A_b). Eq. 4.5 is its inverse (L5563–5580).
- Measured jL: Al 420–3800 A/cm [Sch85], Cu 375–3700 A/cm [Tho08]. For Cu this corresponds to about 5–100 µm (L5597–5609).

### 4.3.2 Critical Length Applications (L5615–5768, PDF p.123–126)
- [Set09] rules (Eqs. 4.6–4.9, PDF p.123): I_max(5 ≤ L ≤ 10) ~ (W/L)·S; I_max(L < 5) ~ (W/5)·S; I_max(2 < W < 20) ~ W·√W·S; I_max(W ≥ 20) ~ W·S. S is a user de-rating factor. The units of L and W are not given (L5621–5645).
- Limit segment length for high currents (L5646–5649). The book's numbers are valid only for simple two-terminal segments (L5650–5653).
- Mean segment lengths (Figs. 4.11–4.12) usually satisfy Blech, but the exceeded share of routing grows to about 5 % by 2026 (L5662–5735).
- The lowest 4–6 thin layers are the critical ones (L5742–5750).
- Routers restrict segment length. Long nets can be split by layer changes, weighing the added via risk (L5759–5768).

### 4.3.3 Linked Segments (L5772–5819, PDF p.126–127)
- Neighboring segments act like reservoirs and emit flux, so the whole net must be analyzed. Isolated-segment conclusions can be reversed in a tree [CCT+06] (L5774–5787).
- Factors: length under test, connected length, current ratio, embedding and external stress (L5793–5798).
- The steady-state model gives no guarantee, and dynamic models are uneconomic. Existing models are inadequate (L5799–5819).

### 4.4.1 Via-Below/Via-Above Fundamentals (L5825–5867, PDF p.127–128)
- Via-above (downstream): the void forms at the top surface right under the via, so a small void fails the line. Via-below (upstream) tolerates more volume loss (Figs. 4.13–4.14).
- Under growth saturation, jL can be ×10 larger for via-below (L5863–5867).

### 4.4.2 Parameters (L5871–5919, PDF p.128–129)
- jL = 375 A/cm (via-above) versus up to 3700 A/cm (via-below). At 5·10⁵ A/cm² these are 7.5 µm and 74 µm (L5873–5907).
- More via-above segments exceed the critical length (Fig. 4.15) (L5908–5916).

### 4.4.3 Applications (L5923–5943, PDF p.129–130)
- Configure critical segments via-below. Apply separate jL values for via-below and via-above (L5932–5934).
- Via-above: overlap the metal liners at the via and the line beneath [MS13] (L5941–5943).

### 4.5.1 Reservoir Fundamentals (L5949–5967, PDF p.130)
- Non-current-carrying extensions supply material, so larger voids are tolerated and jL rises. In a net, neighboring segments also act as reservoirs.

### 4.5.2 Sources and Sinks (L5971–6003, PDF p.130–131)
- A source (at the cathode) raises TTF and can make a line immortal. A sink (at the anode) lowers the stress build-up, enlarges the balanced void and raises the failure probability.

### 4.5.3 Reservoir Types (L6007–6040, PDF p.131–132)
- End-of-line: enlarged via overlap, stays on grid, compatible with double and triple patterning.
- Side reservoir: a 2-D branch that intercepts a travelling void, but may break double-patterning rules [MGL+11].
- Gaps between multiple vias form reservoirs too.

### 4.5.4 Applications (L6044–6110, PDF p.132–133)
- A reservoir is a pure source only for unidirectional current. With AC or low-k dielectrics, avoid reservoirs (L6049–6054).
- Average ×5 permissible Blech length at the same j [HRM08] (L6055–6058). TTF scales with reservoir area [NSMK01]. End-of-line length has an optimum [LNW10, TF12] (L6062–6096).

### 4.6.1 Multiple Vias Fundamentals (L6116–6182, PDF p.133–135)
- Redundancy protects against mask shift, missing polygons, particles, shallow etch and incomplete fill (Figs. 4.21–4.22). Via arrays (to lower J) are distinct from redundant vias (for yield) (L6132–6160).
- Trade-off: footprint and reservoirs (L6145–6149).

### 4.6.2 Current Distribution (L6186–6199, PDF p.135)
- Parallel vias give a 2–4× increase in TTF, but only if the current splits evenly (L6195–6199).

### 4.6.3 Vias With Reservoirs (L6217–6230, PDF p.136)
- Reservoirs sit between the vias and in overlaps. Whether they help depends on current direction. According to [MIM+07], the reservoir effect is the main reason redundant vias improve TTF.

### 4.6.4 Geometrical Configuration (L6234–6279, PDF p.136–137)
- At a direction change, the inside-curve vias are overloaded. In a series of vias along the wire, the via on the shortest path carries the most current (Figs. 4.25–4.27).
- Via pitch trade-off: a larger pitch gives larger reservoirs but lower dielectric rigidity and worse distribution. Redundant vias can only go where routing capacity is spare (L6266–6279).

### 4.6.5 Applications (L6320–6371, PDF p.138–140)
- Redundant vias are standard post-layout for yield, added only where they enlarge nothing and change no topology; current distribution is ignored (L6339–6344).
- For uniform stress, place vias at minimum spacing on the perpendicular to the bisector between the connected wires (L6352–6354).
- Better still, route one wire "around the corner" so both wires share an orientation and the array lies in line with the current (Fig. 4.29). This is possible only without strict preferred directions (L6355–6359).

### 4.7.1 Self-Healing and Rising Frequencies (L6375–6430, PDF p.140–141)
- Eq. 4.10 repeats Eq. 2.6. Using average rather than RMS accounts for self-healing (L6378–6388).
- MTF_AC/MTF_DC has a plateau across current clock ranges and falls in the THz range from the skin effect (Fig. 4.30). Thermal migration rises at high f because RMS heating stays constant (L6389–6430).

### 4.7.2 Applications (L6434–6478, PDF p.141–142)
- Power nets are the most sensitive. Clock and signal nets differ in current and symmetry, and clock nets have more sinks and higher currents (L6436–6473).
- A single global limit wastes resources. Use at least two limits, one for DC or f < 10 kHz and one for AC, with more steps possible between 10 Hz and 10 kHz (L6450–6478).

### 4.8 Materials for Classical Metal Routing (L6482–6518, PDF p.142–143)
- Metal, dielectric and barrier all matter. Blocking one diffusion path shifts dominance to another.

### 4.8.1 Interconnect (L6522–6574, PDF p.143–144)
- Table 4.2 (resistivity in µΩ·cm, Ea for void migration in eV): Al 2.44/0.61; Ag 1.47/0.66; Cu 1.54/0.70; Au 2.03/0.75; W 4.84/1.89 (L6547–6554).
- W is EM-robust but too resistive except for contacts. Au poisons Si (L6561–6568).

### 4.8.2 Dielectric (L6578–6635, PDF p.144–145)
- Table 4.3 (ε_r, E in GPa): aerogel 1.1–2.2/0.001; polyimide 2.7/3.7; SiO2 3.9/300; FR epoxy 5/20; Si3N4 7.5/297; Al2O3 9.5/264; Si 11.7/99 (L6592–6601).
- **Source inconsistency:** Sect. 2.4.4 uses E_SiO2 = 70 GPa (L2006).
- Low-k lowers back-stress. Compensate with local metal structures or reinforcement, or lower the limits (L6622–6635).

### 4.8.3 Barrier (L6639–6725, PDF p.146–147)
- Table 4.4 (Cu interface Ea in eV): Ta 2.1; Ta/TaN 1.4; SiN or SiCxNyHz 0.7–1.1; SiN on Cu(Ti) 1.3; CoWP 1.9–2.4; SiCxHy 0.9 (L6672–6680).
- The cap/top interface is the weak one. Metallic caps or self-aligned Mn/Ru barriers help (L6683–6706).

### 4.9 New Materials (L6729–6737, PDF p.148)
- No better metal than Cu is expected soon.

### 4.9.1 Carbon-Based Solutions (L6740–6829, PDF p.148–150)
- Candidates: GNR, CNF and CNT. SWCNT and MWCNT exist. Chirality types are armchair, zigzag and chiral; one-third are metallic, and MWCNTs are almost always metallic.

### 4.9.2 CNT Properties (L6833–6908, PDF p.150–152)
- Single CNTs carry up to 10¹⁰ A/cm² [BS06], with ballistic transport up to 1 µm. A Cu line of 100×50 nm carries up to 50 µA; a 1 nm CNT carries 20–25 µA (L6838–6847).
- Table 4.5: Cu max J < 1·10⁷ A/cm²; CNT > 1·10⁹; Cu-CNT composite > 6·10⁸ A/cm². Thermal conductivity: Cu 385 W/(m·K); CNT 3000–10,000 (L6848–6887).
- Measured MWCNT resistivity is above the model predictions (L6897–6901).

### 4.9.3 CNT Applications (L6912–6963, PDF p.152–153)
- CNT arrays suit vias. Contact resistance and packing density are the obstacles; the likely near-term form is a hybrid of CNT vias with composite lines.
- CNT bond energy ≥ 3.6 eV versus about 1 eV for Cu, but the tube fails thermally once the maximum is exceeded (L6955–6960).

### 4.10 Summary (L6967–7010, PDF p.154)
- Bamboo needs surface diffusion disabled. Length, via and reservoir effects are the near-term levers.
- Via arrays in power nets need careful geometry. Frequency effects serve for net classing only.
- Skin effect matters for analog HF at ≥ 45 GHz [YZZ+11] (L7000–7004).

### 5.1 Summary of EM-Inhibiting Measures (L7338–7461, PDF p.161–164)
- Guidelines (L7436–7454): use the Blech length; use reservoirs in power nets but not in signal nets; prefer via-below; restrict jL more on via-above segments; place multi-vias in line.
- Combining the measures is estimated to raise the permissible J by ×10 in current nodes (L7456–7461).

### 5.2 Outlook: Segment Lengths (L7465–7519, PDF p.164–165)
- Fig. 5.1 assumes a maximum mechanical stress of 100 MPa. Blech lengths shrink faster than segment lengths. About 5 % of routing exceeds Blech by 2026 and will need reservoirs (L7499–7519).

### 5.3 Outlook: Library of EM-Robust Elements (L7523–7570, PDF p.165–166)
- A pattern generator produces EM-verified routing elements and routing uses only those. Verification then reduces to checking interactions, with no FE runs (Fig. 5.2).

### 5.4 Outlook: New Technologies (L7573–7591, PDF p.166)
- CNTs are thermally destroyed above 10⁹ A/cm². EM persists in Cu-CNT composites. New routing constraints are expected.

### 5.5 EM-Aware Design: Driven by Constraints (L7596–7647, PDF p.167–168)
- The flow moves from constraint-correct (verify after the fact) to constraint-driven (algorithms governed by constraints). Only EM-robust elements are allowed (Fig. 5.3).

### Index (L7657–7770)
- Checked for missed topics. No terms appear that are not covered above.

---

## 3. Actionable extraction

The Philis status lines are based on quick greps. Current EM surface in Philis:
- Deck EM limits per layer come from `backend/verify/src/pdk.rs:519-558`, including `derating` at `:542`.
- Temperature derating is `derate` in `kernel/analog/src/routing/em.rs:16-21`.
- Width and cut sizing are `Limit::width_nm` and `Limit::cuts` (`em.rs:48-70`).
- The route-tier rule is the tree-independent `Electromigration` rule (`em.rs:92-150`).
- `dr` computes per-edge DC branch currents (`backend/dr/src/lib.rs:888-897`) and sizes each segment and via cut from them (`dr/lib.rs:149-153`, `:749`). It reports `em cuts` (`:883`) and `em underwidth` (`:1853`).
- Op-point currents are DC only (`frontend/library/src/oppoint.rs:89-100`), with one die temperature (`frontend/library/src/elaborate.rs:242-261`).

### EM-01 Current density and cross-section model
- Kind: data-model
- Statement: J = I/A (Eq. 1.1). A = W·T with T = (A/R)·W. The limit applies per layer at the nominal thickness h_nom (Eqs. 3.9, 3.21). A deck limit in µA/µm of width already folds thickness in.
- Source: Eq. 1.1, L652–657, PDF p.18; Table 1.1 footnote, L482–484, PDF p.15; Eq. 3.9, L4273–4275, PDF p.93.
- Philis stage: deck, dr, verify.
- Automation recipe: keep the limit per layer as current per unit width, or as J together with h_nom and h_min. The h_min is needed for EM-13. Convert decks given in MA/cm² with I_max/W = J·h.
- Beats hand layout because: every segment is sized from a number, not from a rule-of-thumb width.
- Philis status: implemented as µA/µm plus µA/cut (`em.rs:26-30`, `pdk.rs:539-541`). Thickness is not carried separately.

### EM-02 Black's equation as the lifetime metric
- Kind: formula / metric
- Statement: MTF = t50 = A·j⁻ⁿ·exp(Ea/kT). n = 1 means void-growth-limited and n = 2 nucleation-limited failure. Al: Ea ≈ 0.7 eV, n = 2. Cu: Ea = 0.9 eV (surface), n = 1.1–1.3. The equation is valid only for straight lines with no bends, layer changes or material transitions, and not under steep current rise.
- Source: Sect. 2.2 Eq. 2.1, L1216–1259, PDF p.30–31; Eq. 3.8, L4249–4261, PDF p.93.
- Philis stage: verify, flow (report).
- Automation recipe: per segment, after `dr`, compute the relative lifetime MTF/MTF_limit = (j_limit/j)ⁿ·exp[(Ea/k)(1/T − 1/T_ref)]. Report the minimum over segments as an "EM lifetime margin" metric in the bench table, next to the pass/fail budget. Use it as a tie-breaker cost term, not a hard constraint.
- Beats hand layout because: a quantitative worst-segment margin across every net instead of "wide enough".
- Philis status: partial. The temperature form is used in `derate` (`em.rs:11-21`), but no MTF or margin is reported.

### EM-03 Temperature derating of current limits
- Kind: formula
- Statement: at constant MTF, j_max(T)/j_max(T_ref) = exp[(Ea/(n·k))·(1/T − 1/T_ref)]. Cu: +10 K requires a current cut of more than 50 %. Al: −5 K gives about +25 % j, and 25 → 125 °C requires about a 90 % cut. Automotive ambient reaches up to 175 °C.
- Source: Sect. 2.3.2, L1346–1367, Fig. 2.7 L1399–1403, PDF p.33–34; Eq. 3.11, L4299–4304, PDF p.94.
- Philis stage: flow, deck.
- Automation recipe: derate every layer and cut limit to the conductor temperature (EM-04). Ea and n come from the deck, with fallback per EM-42.
- Beats hand layout because: designers rarely re-derive limits for 125–175 °C mission temperatures.
- Philis status: implemented (`em.rs:16-21`, used at `elaborate.rs:255-257`). It never credits temperatures below T_ref (`em.rs:14,17`). It is a no-op when the deck has no `reference_temperature`/`activation_energy_ev`/`current_exponent` (`pdk.rs:542`). Only GPurify's `generic_finfet.json` deck carries `activation_energy_ev`/`blech_limit` values (grep of `~/.cargo/git/checkouts/gpurify-*/pdks`).

### EM-04 Per-segment conductor temperature (thermal map input)
- Kind: algorithm
- Statement: verification needs temperature data, either the average chip temperature or a temperature field. Thermal simulation accounts for gradients near heat sources and sinks (Sect. 3.5.1). Wire temperature is dominated by active-device dissipation plus ambient (Sect. 3.7).
- Source: L4519–4526, PDF p.99; L4932–4937, PDF p.107.
- Philis stage: flow, dr, verify.
- Automation recipe:
  - Compute the device-power temperature rise map with the existing thermal model (`kernel/core/src/thermal.rs:22`, `rises_mc`).
  - Evaluate it at each segment's midpoint and set T_seg = T_die + ΔT_map.
  - Derate that segment's limit with EM-03 before sizing it in `dr`.
  - Budget-check and report per segment.
- Beats hand layout because: a human applies one worst-case temperature to the whole block. Automation over-sizes only near the hot devices.
- Philis status: missing. The flow uses one die temperature (`elaborate.rs:245-246` ponytail note, `lib.rs:280`).

### EM-05 Joule self-heating and the electrothermal loop
- Kind: rule / algorithm
- Statement: voids raise J, J raises Joule heating, heating raises T, and T accelerates void growth (Fig. 2.4). RMS-based sizing includes self-heating, which often limits current density in the top metal layers because they conduct heat poorly (Sect. 3.3.1). Narrowing a wire to reach the bamboo regime is bounded by a self-heating width limit (Fig. 4.1). Numeric self-heating limits are not given.
- Source: L1114–1120, Fig. 2.4, PDF p.29–30; L3769–3773, PDF p.83; Fig. 4.1, L5200–5223, PDF p.114.
- Philis stage: dr, verify, deck.
- Automation recipe:
  - The deck should provide a per-layer RMS or self-heating limit (µA/µm_rms), or a thermal resistance to substrate.
  - Size each segment to the RMS current against that limit (EM-12, third term).
  - Optionally compute ΔT_self = I_rms²·R_seg·θ_seg, feed it into EM-04, and iterate twice.
- Beats hand layout because: the coupling is rarely computed by hand, and top-metal power straps are exactly where it bites.
- Philis status: missing (no `self.?heat|joule` except the ponytail note at `elaborate.rs:245-246`).

### EM-06 Three current types and when each governs
- Kind: rule
- Statement:
  - RMS applies for f < 1 Hz, analog DC nets and reliability-critical long pulses. It is conservative (no self-healing) and includes Joule heating.
  - Average applies for f > 1 Hz, digital and analog signal nets. It includes self-healing and is the basis for Black-law lifetime.
  - Peak applies to single pulses (ESD, short power-stage pulses) and is checked against EOS rules.
  - All three are evaluated at once. Per segment, the type needing the widest wire or the most vias (for the layer and temperature) decides.
- Source: Sect. 3.3.1, L3756–3786, PDF p.82–83.
- Philis stage: annotator (op-point), dr, verify, deck.
- Automation recipe:
  - Extend the op-point step from DC to a transient bench per operating phase. Record per terminal i_avg, i_rms and i_peak.
  - Carry three limits per layer: j_avg, j_rms and j_peak.
  - Size with EM-12 as w = max over the three types.
  - ESD pad and clamp nets get peak-only sizing from the deck's ESD current.
- Beats hand layout because: humans usually size for one "DC" current. The max-of-three rule catches RMS-dominated clock-like nets and ESD paths automatically.
- Philis status: partial. DC average only (`em.rs:90`; `oppoint.rs:89-100`). No `rms`/`peak` fields exist anywhere.

### EM-07 Terminal current bounds instead of a single equivalent value
- Kind: data-model
- Statement: a single max-|equivalent| current per terminal produces violations at Steiner-point segments when two terminals have reversed worst cases (Fig. 3.4). Store lower and upper bounds (avg⁻/avg⁺, RMS⁻/RMS⁺, peak⁻/peak⁺). RMS⁻ is computed from the zero and negative parts of the waveform and multiplied by −1, so RMS⁻ ≤ 0.
- Source: Sect. 3.3.2, L3790–3836, Fig. 3.4 L3813–3817, PDF p.83–84.
- Philis stage: annotator/flow (op-point), dr.
- Automation recipe: replace `pin_ua: Vec<Vec<(String, i32)>>` (`backend/dr/src/lib.rs:55`) with a per-pin bounds record `{lo: [avg, rms, peak], hi: [avg, rms, peak]}` per operating phase. Fill it from the transient waveforms (EM-06). With DC only, set lo = hi = i_dc.
- Beats hand layout because: the reversal case (a node that sources in one mode and sinks in another, e.g. a bidirectional bias line or charge-pump node) is invisible when sizing by hand.
- Philis status: missing. One DC value per pin (`dr/lib.rs:55,227-235`) and per net (`oppoint.rs:89-100`).

### EM-08 Terminal current models (vector, time-slot, phase matrix)
- Kind: data-model
- Statement:
  - Model 1 (Eq. 3.1): per terminal, KCL-consistent snapshot vectors at each terminal's min and max instants, i.e. up to m pairs (2m values) for an m-terminal net. It stores all waveforms and over-sizes.
  - Model 2 (Eq. 3.2): i_terminal = [[S1, i_min1, i_max1], …, [Sn, i_minn, i_maxn]] per time slot or operating phase. It links to mission profiles.
  - Model 3 (Eqs. 3.3–3.4): matrices L_n and U_n of size O types × P phases per terminal. Storage is 2·N·O·P values; with O = 3 and P = 1 that is 6·N. Storage can be sparse.
- Source: Sect. 3.3.2, L3837–3971, Figs. 3.5–3.6, PDF p.84–87.
- Philis stage: annotator, flow.
- Automation recipe: adopt Model 3. `[[f32; P]; O]` lo/hi per pin, with P from the testbench operating phases (e.g. normal, standby, hot). It is the smallest model that still supports EM-09 and EM-16.
- Beats hand layout because: sizing per operating phase avoids both undersizing (missed modes) and the over-engineering of a worst-of-all-time value.
- Philis status: missing.

### EM-09 Worst-case segment current from LHS/RHS bound sums
- Kind: algorithm
- Statement:
  - Cutting segment s splits the terminals into LHS and RHS. i_L,LHS = Σ_LHS i_L,n,op, and likewise i_U,LHS, i_L,RHS, i_U,RHS (Eq. 3.5).
  - i_W,op = max{min(|i_L,LHS,op|, |i_U,RHS,op|), min(|i_L,RHS,op|, |i_U,LHS,op|)} (Eq. 3.6).
  - i_W,o = max_p i_W,op (Eq. 3.7).
  - Example: [−1, +2] mA against [−3, 0] mA gives 2 mA.
  - It also works for non-KCL terminal data. Summing annotated equivalents is ≥ the true vector-sum equivalent, so it is safe.
- Source: Sect. 3.3.3, L3975–4052, Fig. 3.7, Eq. 3.6 checked on PDF p.88.
- Philis stage: dr (sizing), verify.
- Automation recipe: in `branch_currents` (`dr/lib.rs:897`) keep the DFS subtree sums, but sum lo and hi separately per type and phase. Take the edge current as Eq. 3.6, then the max over phases, and size with EM-12 per type. This is O(edges·O·P) per net.
- Beats hand layout because: it is exact over every tree edge and every mode. Humans check only the trunk.
- Philis status: partial. DC KCL subtree sums per edge (`dr/lib.rs:888-897`). When the net's currents do not sum to zero, the edge carries the larger side (`:891-893`). There are no lo/hi bounds and no min/max form. Cycles are cut to a spanning tree (`:895-896`).

### EM-10 Current-driven net topology (wire planning) minimizing routing area
- Kind: algorithm
- Statement: topology and segment currents depend on each other (Fig. 3.9). Resolve this with a wire-planning step that builds the routing tree and estimates segment currents together. The objective is minimum interconnect area rather than length: keep current-intensive segments short, and choose Steiner points to reduce segment currents. Afterwards, detailed routing is point-to-point with known widths.
- Source: Sect. 3.2, L3463–3470, PDF p.76–77; Sect. 3.3.3, L4053–4141, Figs. 3.9–3.10, PDF p.89–91; [LJ03].
- Philis stage: gr.
- Automation recipe:
  - For nets with a known current (op-point) above the criticality threshold (EM-20), build the Steiner tree by minimizing Σ_edges L_e·w(i_W,e), where w() is EM-12, instead of Σ L_e.
  - Heuristic: start from the RSMT (rectilinear Steiner minimal tree). Repeatedly try moving or merging Steiner points and re-attaching terminals, recompute edge currents (EM-09), and accept moves that lower the area.
  - Terminals with large opposite currents (source and sink) should be joined by the shortest path. Small loads branch off it.
  - Hand `dr` the per-edge widths.
- Beats hand layout because: exhaustive topology search on area-weighted cost versus a human's single guess.
- Philis status: partial. `gr` prices current × layer resistance per step (`backend/gr/src/lib.rs:850-930`), which biases each net's path. There is no Steiner or topology optimization on area (`steiner` has 0 hits in `kernel/`, `backend/` and `frontend/`).

### EM-11 Current-aware placement
- Kind: heuristic / cost term
- Statement: "place the function blocks at locations so as to minimize current flows" (Sect. 3.2).
- Source: L3457–3462, PDF p.76.
- Philis stage: gp, dp.
- Automation recipe: add Σ_nets I_net·HPWL_net (I from the op-point; µA·µm) as a weighted wirelength term in `gp`/`dp`, normalized by the heaviest net. The effect is to pull high-current device pairs (output stages, bias mirrors feeding large loads, supply taps) together. Use the same current vector that `gr` already receives.
- Beats hand layout because: a joint optimization with matching and symmetry constraints instead of intuition.
- Philis status: missing (no current or `_ua` hits in `backend/gp/src/lib.rs`).

### EM-12 Nominal wire width from equivalent and peak currents
- Kind: formula
- Statement: w_nom(T_eff) = max{i_w,eq/(j_eff,eq(T_eff)·h_nom), i_w,peak/(j_eff,peak(T_eff)·h_nom), w_min_process} (Eqs. 3.21–3.23). i_w,eq is the RMS or average worst-case segment current (EM-06, EM-09).
- Source: Sect. 3.5.2, L4587–4617, PDF p.100.
- Philis stage: dr, cells (straps).
- Automation recipe: `Limit` gains `ua_per_um_rms` and `ua_per_um_peak`. `width_nm` returns the max over the three types. `em_width` already enforces w ≥ wire_width and grid snapping (`dr/lib.rs:149-153`).
- Beats hand layout because: exact per-edge widths with no blanket overdesign.
- Philis status: partial. Only the equivalent-DC term (`em.rs:48-58`), with the w_min term through `dr/lib.rs:151`. The peak term is missing.

### EM-13 Effective width with process margins
- Kind: formula
- Statement: w_eff(T_eff) = w_nom(T_eff)·(h_nom/h_min) + Δw + w_etch (Eq. 3.24). The terms are the thickness spread (nominal against minimum), width variation and etch loss.
- Source: L4618–4625, PDF p.101.
- Philis stage: deck, dr.
- Automation recipe: deck fields `h_nom`, `h_min`, `dw` and `w_etch` per metal. When missing, set h_nom/h_min = 1 and the margins to 0, and emit a "no process margin" note. Apply the formula in `width_nm`.
- Beats hand layout because: margins are applied uniformly and cannot be forgotten.
- Philis status: missing.

### EM-14 Via-array cut count, temperature factor and inhomogeneity factor g(H)
- Kind: formula
- Statement: n_via = ceil(i_w,eq/i_single_via(T_ref)·f(T_eff)·g(H)) (Eq. 3.25). f(T_eff) = exp(−(Ea/(n·k·T_ref))·(1 − T_ref/T_eff)) with 1 ≤ n ≤ 2 (Eq. 3.26). g(H) = 1 for homogeneous flow and > 1 for inhomogeneous flow (FEM). As printed, f < 1 when hotter, so it is the allowed-current factor. It must divide: n_via = ceil(i_eq·g(H)/(i_single(T_ref)·f(T_eff))).
- Source: L4626–4660, Eqs. 3.25–3.26 checked on PDF p.101.
- Philis stage: dr.
- Automation recipe: keep `Limit::cuts` with the derated per-cut limit and multiply i by g(H). Take g(H) from the via-array geometry class (EM-39): 1.0 for arrays in line with the current, and a larger per-PDK factor from pattern characterization (EM-46) for L-turn arrays. Values are not given in the book.
- Beats hand layout because: the cut count follows current and temperature and accounts for crowding.
- Philis status: partial. The cut count uses the corrected sign (`em.rs:60-70`, used at `dr/lib.rs:749`). g(H) is missing.

### EM-15 Mission-profile scaling of limits (characteristic → reference → effective)
- Kind: formula / deck-requirement
- Statement:
  - t_life,ref = AF_TF·AF_T·AF_q·t50,char (Eq. 3.10).
  - AF_TF = target lifetime/10 years. Example: 15 years gives AF_TF = 1.5, with the reference at 10 years and T_ref = 378 K.
  - AF_T = exp[(Ea/(n·k))·(1/T_ref − 1/T_char)] (Eq. 3.11).
  - AF_q = S1/exp[norminv(q_ref)·σ], with 1 < S1 ≤ 10 (Eq. 3.12).
  - j_ref = j_char/(AF_T·AF_q·AF_TF) (Eq. 3.13). j_eff = j_ref/(AF_T(T_ref, T_eff)·AF_q(q_ref, q_eff)·AF_TF) (Eq. 3.20).
  - Characterization runs at > 473 K.
  - Caution: as printed these equations mix time factors and current-density factors. A form derived consistently from Eq. 3.8 is j_ref = j_char·exp[(Ea/(n·k))·(1/T_ref − 1/T_char)]·(t50,char/(AF_q·t_life,ref))^(1/n). Check against [JK14] before relying on the printed direction.
- Source: Sect. 3.4.2, L4247–4476, Eqs. 3.10–3.13 and 3.20, PDF p.93–97 (checked on PDF p.94 and p.96).
- Philis stage: deck, flow.
- Automation recipe: the config gains `mission: {years, phases: [(hours, T_K)], q_target}`. Compute T_eff per layer (EM-16) and scale each layer's deck limit before EM-03. Default: the deck's own reference, i.e. no scaling.
- Beats hand layout because: the same layout is re-qualified for consumer (10 years at 105 °C) and automotive profiles by recomputing, not by redrawing.
- Philis status: missing.

### EM-16 Effective temperature of a multi-phase mission profile
- Kind: formula
- Statement: 1/T_eff = 1/T_ref − (k/Ea)·ln(t_life,eff/t_life,ref) (Eq. 3.18). t_life,eff = Σ_s t_life,s·exp[(Ea/(n·k))·(1/T_ref − 1/T_s)] (Eq. 3.19, printed with n in the exponent). T_s is the mid-temperature of the bin; the example is 100 h at 373 K for the 368–378 K bin. Compute per layer (Ea and n are layer-specific) and use the highest T_eff for layout and verification. Self-heating is assumed negligible.
- Source: L4423–4469, Fig. 3.13, PDF p.96–97.
- Philis stage: flow, deck.
- Automation recipe: fold the `mission.phases` table into one T_eff per layer, then feed it to EM-03 instead of `op.temp_c`.
- Beats hand layout because: automotive profiles have 5–10 temperature bins, which humans collapse to the maximum. That is either over-conservative or wrong when Ea differs per layer.
- Philis status: missing (`op.temp_c` is a single temperature, `lib.rs:280`).

### EM-17 Error quantile scales with the number of segments
- Kind: formula / metric
- Statement: q_eff ≪ 1/m, where m = number of routed interconnect segments (Eq. 3.17). More interconnects require a smaller quantile and so a longer t_life,ref and a stricter j.
- Source: L4386–4389, L4418–4428, PDF p.95–96.
- Philis stage: flow, verify.
- Automation recipe: after routing, count m (segments, i.e. edges of all net trees). Set q_eff = 1/(10·m); the factor 10 is a Philis choice, not from the book. Tighten j_eff through AF_q (EM-15). This is small for analog (m ~ 10²–10³), but it makes the limit design-aware.
- Beats hand layout because: statistical correctness scales with design size automatically.
- Philis status: missing.

### EM-18 Re-verify on reuse or on mission change
- Kind: check / flow
- Statement: re-validate current density whenever the temperature profile or the phase durations change, even with an identical layout (Sect. 3.4.1).
- Source: L4185–4194, PDF p.92.
- Philis stage: flow, verify.
- Automation recipe: persist the per-segment (layer, width, length, i_W bounds) table next to the GDS. A mission change re-runs only EM-15, EM-16 and EM-20, with no P&R.
- Beats hand layout because: seconds instead of a manual audit.
- Philis status: missing.

### EM-19 Pin-region ampacity and access-point filtering
- Kind: check / algorithm
- Statement: different connection positions on an analog pin produce different current loads inside it. Compute the ampacity of each pin region and exclude regions below the arriving wire current as connection points (Fig. 3.15, U-shaped pin, regions of 3/2/1/0.5 mA).
- Source: Sect. 3.5.3, L4687–4699, PDF p.102.
- Philis stage: cells (export), dr (access).
- Automation recipe:
  - Each generator exports every pin rect split into regions, with ampacity = region width × layer limit, plus the cut count × cut limit of contacts under the region.
  - `dr` access-point selection drops regions whose ampacity is below the segment's i_W (EM-09).
  - If none remain, flag the pin (cells must widen the strap), or allow a multi-point connection that splits the current.
- Beats hand layout because: every pin of every device is checked, including the inner fingers of wide multi-finger MOSFET straps.
- Philis status: missing (no current or ampacity terms in `kernel/cells/src`).

### EM-20 Verification flow with net criticality filter
- Kind: check / algorithm
- Statement: (1) verify the net terminals; (2) de-select a net if Σ worst-case terminal currents < the maximum permitted current of the minimum-size wire on the minimum layer; (3) compute J against the layer limit; (4) remove dummy errors such as corner spots. The layout is cut into independent segments carrying the worst-case currents, and each is checked once.
- Source: Sect. 3.5.1, L4540–4574, PDF p.99–100.
- Philis stage: annotator (classification), verify.
- Automation recipe: in the annotator, mark a net EM-critical when Σ|i_W| over its terminals > I_max(min wire, min layer). Critical nets get EM constraints in gr and dr, and the rest skip them. Analog note: power and signal currents are of the same order in analog (L3529–3531), so filter on current, never on net role.
- Beats hand layout because: nothing is filtered on a guess, and the non-critical majority costs nothing.
- Philis status: partial. The rule is "unknown" without an op current (`em.rs:86-88`), and GPurify's ERC EM rules are armed by design intent (`backend/verify/src/lib.rs:318-335`). There is no explicit criticality classification.

### EM-21 Early (netlist-level) EM-critical net estimation
- Kind: heuristic
- Statement: "Estimation of EM-critical nets based on netlist" [JL10] computes worst-case bounds on segment currents from the terminal bounds before layout, splitting nets into critical and non-critical sets. Only critical nets need special handling during layout generation.
- Source: Sect. 3.2.2, L3696–3701, Fig. 3.3, PDF p.81; L3680–3683.
- Philis stage: annotator.
- Automation recipe: without a topology, i_W,max ≤ min(Σ|positive bounds|, Σ|negative bounds|) over all terminals; this bound is derived from Eq. 3.6. If it fits the minimum wire, the net is non-critical. Otherwise emit an `Electromigration` constraint and a width hint for gr capacity reservation.
- Beats hand layout because: the classification is exhaustive and provably safe.
- Philis status: missing.

### EM-22 Net classes with separate limits (DC/power, clock, signal)
- Kind: rule / deck-requirement
- Statement: power nets carry DC and are the most sensitive. Clock nets carry near-symmetric AC with high RMS (many sinks). Signal nets carry asymmetric pulsed AC with low RMS. Use at least two limits: one for DC or f < 10 kHz and one for AC above that, with further steps possible between 10 Hz and 10 kHz. A single global limit wastes routing resources.
- Source: Sect. 3.2.2, L3642–3650, PDF p.80; Sect. 4.7.2, L6436–6478, PDF p.141–142; Sect. 4.10, L6995–6999, PDF p.154.
- Philis stage: annotator, deck, dr.
- Automation recipe: map `NetRole::{Supply, Ground}` and DC-bias nets to the DC class, and `NetRole::Clock` and switching nets (from the transient) to the AC class. Per class, use a multiplier on the deck limit, or the deck's own `electromigration` average versus RMS limits.
- Beats hand layout because: signal nets are not over-widened, which frees area and capacitance for matching-critical routes.
- Philis status: partial. `NetRole {Signal, Supply, Ground, Clock}` exists (`backend/annotator/src/netrole.rs:11-15`), but EM limits are per layer only (`em.rs:98`).

### EM-23 AC lifetime and self-healing model
- Kind: formula
- Statement:
  - J_net = J_fwd·(1 − γ) (Eq. 2.6/4.10).
  - MTF_AC = A/(r·j⁺ − γ(1 − r)·j⁻)ⁿ·exp(Ea/kT) (Eq. 2.7).
  - γ is empirical (Eq. 2.8) and r is the duty factor.
  - In Cu, MTF_AC/MTF_DC rises up to ×500 over 10–10⁴ Hz [TCH93]. For zero-DC asymmetric pulses the minimum lifetime is at r ≈ 0.4 [SKSY90]. Self-healing is negligible for supply nets and low-rate lines.
- Source: Sect. 2.4.3, L1765–1883, Fig. 2.13, PDF p.41–44 (checked on PDF p.42); Sect. 4.7.1, L6375–6395, PDF p.140.
- Philis stage: flow (equivalent current), verify.
- Automation recipe: for AC-class nets (EM-22), compute j_eq = r·j⁺ − γ(1 − r)·j⁻ from the transient. Use the deck's γ if present, otherwise γ = 0 (conservative) or the average current (L6386–6388).
- Beats hand layout because: it credits AC nets exactly instead of guessing.
- Philis status: missing.

### EM-24 Frequency regimes: plateau, thermal migration and skin effect
- Kind: heuristic / formula
- Statement:
  - The MTF_AC/MTF_DC ratio has a plateau across present switching frequencies. At high f, thermal migration dominates because RMS heating stays constant (Fig. 4.30).
  - Skin depth is δ = √(2ρ/(ωμ)) (Eq. 2.9), with j ≈ j_S·exp(−d/δ) (Eq. 2.10). Cu δ ≈ 9.4 mm at 50 Hz, scaling as 1/√f.
  - The critical frequency is 90 GHz for W = t = 0.45 µm and about 35 THz at 22 nm. Skin effect matters for analog HF at ≥ 45 GHz [YZZ+11].
- Source: L1885–1968, PDF p.44–45; L6396–6430, PDF p.140–141; L7000–7004, PDF p.154.
- Philis stage: annotator (RF net flag), dr.
- Automation recipe: for nets tagged RF with f ≥ 45 GHz, use an effective cross-section ≈ perimeter·δ when δ < min(W, t)/2. The perimeter·δ approximation is derived from Eq. 2.10, not given in the book. Otherwise ignore.
- Beats hand layout because: it is applied only where it matters.
- Philis status: missing. Low priority for Philis's current targets.

### EM-25 Avoid 90° bends on current-critical wires
- Kind: rule
- Statement: a 90° corner has significantly higher current density than 135° or 150° bends, so 90° bends must be avoided on analog wires with high current (Fig. 2.8). Current crowding at bends and vias is the main EM indicator (Sect. 2.5.4).
- Source: Sect. 2.3.3, L1421–1445, Fig. 2.8, PDF p.34–35; L2452–2454, PDF p.55.
- Philis stage: dr, cells.
- Automation recipe: for EM-critical segments (EM-20), replace each L-corner with a 45° chamfer, making two 135° bends. Chamfer leg = w, clipped to DRC; diagonal edges must be legal in the deck. If the deck forbids non-Manhattan shapes, add an inner-corner fill ("support polygon", EM-27). Score it as a crowding factor in EM-14 and EM-46.
- Beats hand layout because: applied to every bend of every critical net.
- Philis status: missing. `dr` draws flush L-corners (`dr/lib.rs:1029`, `:1074`), and there are no diagonal, chamfer or octagon hits in `dr`.

### EM-26 Flux-divergence inhomogeneities to minimize
- Kind: heuristic / check
- Statement: damage concentrates at line ends, direction changes, layer changes, cross-section changes, lattice or material changes, existing damage or tolerances, temperature gradients and stress gradients.
- Source: Sect. 2.1, L1044–1061, PDF p.26–27.
- Philis stage: gr, dr.
- Automation recipe: on EM-critical nets, add a per-event cost (bend, via, width step) to the router's cost function. Merge collinear width steps and prefer one width per edge. Count the events per net in the report.
- Beats hand layout because: the counts are minimized globally rather than locally.
- Philis status: partial. `gr` prices vias through `via_r·current` (`gr/lib.rs:850-851`). Bends and width steps carry no EM cost.

### EM-27 Support polygons at bends and terminals
- Kind: algorithm
- Statement: add support polygons at critical corners (bends) and around net terminals when widening is not applicable (terminals) or not sufficient (corner hotspots).
- Source: Sect. 3.6, L4891–4894, Fig. 3.17, PDF p.106.
- Philis stage: dr.
- Automation recipe:
  - At each L-corner of a critical segment, add a right-triangle or square fill in the inner corner of size ≈ w_eff. The book gives no dimension.
  - At each pin connection, add a flared pad whose width grows from the wire width to the pin-region width.
  - Keep all fills DRC-clean against other nets.
- Beats hand layout because: it is systematic.
- Philis status: missing.

### EM-28 Current-driven post-route decompaction (repair loop)
- Kind: algorithm
- Statement: (1) decompose into segments whose ends are sources or sinks; (2) compute the current and size wires and via arrays; (3) add support polygons; (4) decompact while preserving topology. This avoids repeated P&R cycles.
- Source: Sect. 3.6, L4842–4897, PDF p.105–106.
- Philis stage: dr, flow.
- Automation recipe: after `dr`, for each `em underwidth` or `em cuts` violation, widen in place. If spacing blocks it, push the neighboring wires (shove) while preserving topology. Only if that fails, feed back to the epoch loop as it does today.
- Beats hand layout because: exact and fast local repair.
- Philis status: partial. Violations become budget violations (`dr/lib.rs:883`, `:1853`) and are resolved by the epoch loop. There is no local decompaction.

### EM-29 Via vs line depletion and the via-to-line width ratio
- Kind: rule
- Statement: electron flow via → line causes line depletion; line → via causes via depletion (via voiding). As the line/via width ratio rises, the via carries more current for the same line current density.
- Source: Sect. 2.1, L1065–1113, Fig. 2.3, PDF p.27–28.
- Philis stage: dr, verify.
- Automation recipe: size the via array to the full segment current (EM-14), never to the line's current density times via area. Check: cut count × I_cut ≥ i_W for every via stack of a critical net.
- Beats hand layout because: a widened wire landing on a single via is caught every time.
- Philis status: implemented for DC (`dr/lib.rs:749`, `em.rs:65-70`).

### EM-30 Blech immortality per segment
- Kind: formula / rule
- Statement: (jL)_Blech = Ω·Δσ/(e·z*·ρ) (Eq. 4.1). A segment is the conductor between two vias or contacts or between in-plane branches, i.e. a graph edge. Below (jL)_Blech no voids form. Measured jL: Cu 375–3700 A/cm, Al 420–3800 A/cm; for Cu this is about 5–100 µm at typical j. Fig. 5.1 assumes a maximum mechanical stress of 100 MPa.
- Source: Sect. 4.3.1, L5327–5429, L5597–5609, PDF p.116–123; L7499–7503, PDF p.164.
- Philis stage: dr, verify, deck.
- Automation recipe:
  - For each tree edge compute j·L with L = the edge length between vias or branch points. If j·L < (jL)_B for its via configuration (EM-31), the edge is immortal and may be sized to the Blech bound instead of I/J.
  - Unit bridge: (I/w)·L [µA] ≤ 100·(jL)_B[A/cm]·t[µm], since 1 A/cm = 100 µA/µm.
  - Also prefer shorter segments on critical nets (EM-33).
- Beats hand layout because: every segment is credited by the physics instead of a blanket width.
- Philis status: partial. Implemented with a conservative domain equal to the net's whole run on the layer (`em.rs:42-58`; `em.rs:109-117` sums all same-layer shapes). It is active only when the deck gives `blech_limit` (`pdk.rs:541`), which the shipped sky130/ihp/gf180 decks appear not to.

### EM-31 Via-above vs via-below configuration
- Kind: rule / deck-requirement
- Statement:
  - CMP leaves the top surface weak, so voids form at the top.
  - Via-above (downstream, the line contacted from above) fails with a small void under the via. Via-below (upstream, contacted from below) tolerates more void volume.
  - Critical jL: 375 A/cm via-above versus up to 3700 A/cm via-below, i.e. 7.5 µm versus 74 µm at 5·10⁵ A/cm². Under growth saturation, jL is ×10 larger for via-below.
  - Configure critical segments as via-below. Apply separate jL per configuration and restrict jL more on via-above segments.
  - For via-above, overlap the metal liners at the via and the line below.
- Source: Sect. 4.4, L5823–5943, Figs. 4.13–4.15, PDF p.127–130; Sect. 5.1, L7442–7449, PDF p.163–164.
- Philis stage: dr (layer assignment), deck, verify.
- Automation recipe:
  - Know each edge's DC electron-flow direction; electrons flow opposite to the conventional current.
  - Label the cathode (electron-entry) end of each edge "via-above" if it lands on a cut above the layer and "via-below" if on a cut below.
  - Use `blech_above` or `blech_below` from the deck. With only one value given, treat it as via-above, the conservative choice.
  - In layer assignment for critical DC nets, prefer stacks where the high-j segment is entered from below.
- Beats hand layout because: few designers track electron direction per via.
- Philis status: missing (via-above/below appears only in cells and macro contexts, not in routing; `em.rs` has a single `blech`).

### EM-32 Void-growth saturation and the jL² / ΔR criterion
- Kind: formula / check
- Statement:
  - (jL)_sat < (ρ/A)/(ρ_l/A_l)·(ΔR_fail/R)·2ΩB/(e·z*·ρ) (Eq. 4.2).
  - V_sat = e·z*·ρ·A_Cu·jL²/(2ΩB) (Eq. 4.3). It is less restrictive for L < 20 µm and proportional to the absolute ΔR, i.e. the IR drop.
  - ΔR_sat = V_sat·ρ_b/(A_Cu·A_b) (Eq. 4.4).
  - A void spans the full cross-section at V = H²·W. Saturated void volume below that means immortal.
- Source: L5439–5580, Eqs. 4.2–4.5 checked on PDF p.120, PDF p.119–122.
- Philis stage: verify, deck.
- Automation recipe: when the deck gives B, Ω, z*, ρ_b and liner area (none of these values is in the book), compute V_sat per edge and ΔR_sat. Compare ΔR_sat·I with the net's IR-drop budget (EM-45). A segment passes if the void saturates below H²·W or the ΔR·I increase stays within budget.
- Beats hand layout because: EM is tied to the circuit's real tolerance (IR drop) instead of a fixed density.
- Philis status: missing.

### EM-33 Length-dependent current rules (fallback when a deck has no physics)
- Kind: rule
- Statement: I_max(5 ≤ L ≤ 10) ~ (W/L)·S; I_max(L < 5) ~ (W/5)·S; I_max(2 < W < 20) ~ W·√W·S; I_max(W ≥ 20) ~ W·S (Eqs. 4.6–4.9, [Set09]). S is a user de-rating factor. These are proportionalities; the units of L and W are not given. Limit segment length for high currents. Long nets may be split into shorter segments by layer changes, weighing the added via risk.
- Source: Sect. 4.3.2, L5615–5649, L5759–5768, PDF p.123–126 (checked on PDF p.123).
- Philis stage: deck, dr.
- Automation recipe: use these only when the deck states such a rule, because the constants and units must come from the foundry. The general heuristic is to cap edge length on critical nets at L_B(j) from EM-30 and insert a jog or layer change when an edge exceeds it and the via cost is lower.
- Beats hand layout because: automatic segmentation.
- Philis status: missing.

### EM-34 Linked segments: the tree, not the edge, is the unit
- Kind: rule / algorithm
- Statement: neighboring segments act like reservoirs and emit flux, so isolated-segment conclusions can be wrong or even reversed [CCT+06]. Analyze the whole net, or at least each segment's neighbors: length under test, connected length, current ratio, embedding and external stress. Existing models are inadequate.
- Source: Sect. 4.3.3, L5772–5819, PDF p.126–127.
- Philis stage: verify.
- Automation recipe (derived from the Korhonen steady state, EM-44, not from the book):
  - For each DC tree, solve the steady-state hydrostatic stress. Along each edge, dσ/dx = z*eρj/Ω. Stress is continuous at the nodes, and Σ σ_i·L_i·A_i = 0 (volume conservation, blocking boundaries at vias).
  - This is a linear system in one unknown offset per tree.
  - Pass if max tensile σ < σ_crit, using 100 MPa if the deck gives nothing (Fig. 5.1 assumption).
  - This replaces the whole-run domain of EM-30 with a tree-exact check.
- Beats hand layout because: no designer does a tree-level stress solve.
- Philis status: partial/conservative. The whole same-layer run is used as the Blech domain (`em.rs:106-117`).

### EM-35 Reservoirs (end-of-line and side) for DC nets only
- Kind: rule / algorithm
- Statement:
  - A source reservoir at the cathode raises TTF and can make a line immortal. A sink at the anode lowers TTF.
  - End-of-line reservoir: an enlarged via overlap, on grid and double-patterning compatible. Side reservoir: a 2-D branch that intercepts travelling voids, but may violate double-patterning rules.
  - Average ×5 permissible Blech length at the same j [HRM08]. TTF scales with reservoir area. End-of-line length has an optimum (value not given).
  - Only with unidirectional current. Avoid reservoirs with AC or with low-k. Gaps between multiple vias also form reservoirs.
- Source: Sect. 4.5, L5947–6110, Figs. 4.16–4.19, PDF p.130–133; Sect. 5.1, L7438–7441, PDF p.163.
- Philis stage: dr, cells.
- Automation recipe: for DC-class supply and bias nets (EM-22), extend the metal past the cathode-end via by a deck-given `eol_reservoir` length. Use 0 if the deck gives none, since the optimum is not given. Never add one on AC-class nets. Credit jL by the deck factor.
- Beats hand layout because: applied consistently per current direction.
- Philis status: missing (0 hits for `reservoir`).

### EM-36 Multiple vias: redundancy and via arrays
- Kind: rule
- Statement:
  - Redundant vias protect against mask shift, missing polygons, particles, shallow etch and incomplete fill (Figs. 4.21–4.22).
  - Parallel vias give a 2–4× increase in TTF if the current splits evenly.
  - Standard practice adds redundant vias post-layout wherever no area growth or topology change results. Via arrays in power nets reduce J and static IR drop.
  - Trade-off: footprint and inter-via reservoirs. Reservoirs between vias hurt AC nets.
- Source: Sect. 4.6.1–4.6.3 and 4.6.5, L6116–6230, L6339–6348, PDF p.133–139.
- Philis stage: dr.
- Automation recipe: after EM sizing, add yield-redundant cuts on every single-cut via whose overlap allows a second cut. Skip AC-class nets if the deck flags low-k.
- Beats hand layout because: exhaustive.
- Philis status: partial. `dr` fills each wire overlap with as many cuts as fit and flags `em cuts` when fewer than needed (`dr/lib.rs:740-790`). Whether redundancy for yield is applied on every via was not checked.

### EM-37 Via-array geometry for uniform current
- Kind: rule / algorithm
- Statement:
  - At a direction change, vias on the inside curve are overloaded. Vias in series along the wire axis: the via on the shortest path carries the most current. The same holds for 2-D arrays (Figs. 4.25–4.27).
  - Place vias at minimum spacing on the perpendicular to the bisector between the connected wires.
  - Ideally route one wire "around the corner" so both share an orientation and the array lies in line with the current, giving equal path lengths (Fig. 4.29).
  - Via pitch trade-off: larger pitch gives bigger reservoirs but lower dielectric rigidity and worse distribution.
- Source: Sect. 4.6.4–4.6.5, L6234–6362, Figs. 4.25–4.29, PDF p.136–140; Sect. 5.1, L7407–7410, PDF p.163.
- Philis stage: dr.
- Automation recipe: at a layer-change corner of a critical net, the current `dr` fills the full overlap (`dr/lib.rs:757-786`). Replace that with a diagonal row of cuts at minimum pitch perpendicular to the bisector, or extend one wire past the corner so the two wires overlap collinearly, then place the array along that overlap. Set g(H) for EM-14 from the chosen pattern.
- Beats hand layout because: the crowding-optimal pattern is applied on every corner via.
- Philis status: missing. The overlap fill is current-direction agnostic (`dr/lib.rs:757-786`).

### EM-38 Slotting wide high-current straps
- Kind: rule / deck-requirement
- Statement: slotting (cheesing) of wide lines is done mainly to satisfy CMP maximum-width and density rules. It creates parallel bamboo-like strips. The bamboo effect needs w/D50 < 0.5. It is strong in Al (< 2 µm) and weak in Cu unless surface diffusion is suppressed. Resistance rises.
- Source: Sect. 4.2, L5224–5268, Fig. 4.2, PDF p.114–115; Sect. 4.10, L6974–6981, PDF p.154.
- Philis stage: dr, cells, deck.
- Automation recipe: when an EM-sized width exceeds the deck's maximum metal width, split the strap into parallel slotted strips per the deck's slot rules. Recompute the width from EM-12 with the effective (non-slot) width.
- Beats hand layout because: automatic slotting instead of DRC-driven after-the-fact fixes.
- Philis status: missing (no slotting or max-width logic in routing; wide-metal spacing only, e.g. `dr/lib.rs:685`).

### EM-39 Bamboo width window (non-monotonic MTF against width)
- Kind: heuristic
- Statement: MTF falls as width shrinks in the near-bamboo regime, then rises below about half the grain size. A self-heating width limit bounds it from below (Fig. 4.1). The effect needs annealing, and its benefit in Cu is small unless surface diffusion is blocked.
- Source: L5174–5223, L5261–5268, PDF p.113–115.
- Philis stage: deck.
- Automation recipe: none in P&R unless a deck publishes a width-dependent limit table. If one does, `width_nm` must search the table rather than invert a linear I/J.
- Beats hand layout because: n/a (technology lever).
- Philis status: n/a.

### EM-40 Thermomechanical stress estimate
- Kind: formula / data-model
- Statement: σ = (α_SiO2 − α_Cu)·ΔT/(1/E_SiO2 + 1/E_Cu) (Eq. 2.13). With α_Cu = 16.5·10⁻⁶ K⁻¹, α_SiO2 = 0.5·10⁻⁶ K⁻¹, E_Cu = 117 GPa, E_SiO2 = 70 GPa and ΔT = 200 K this gives ≈ 140 MPa tensile. The stress-free temperature is ≈ 250 °C. Note: Table 4.3 lists E_SiO2 = 300 GPa, inconsistent with Sect. 2.4.4.
- Source: Sect. 2.4.4, L1995–2021, PDF p.45–46; Table 4.3 L6592–6601, PDF p.145.
- Philis stage: deck, verify.
- Automation recipe: this sets the initial σ offset for the EM-34 tree stress solve (tensile start makes voids more likely). The deck supplies α, E and the stress-free T; otherwise use these values.
- Beats hand layout because: n/a (model input).
- Philis status: missing.

### EM-41 Low-k and dielectric rigidity de-rating
- Kind: rule / deck-requirement
- Statement: low-k dielectrics have lower Young's modulus, which means less back-stress, shorter effective Blech lengths, reservoirs that hurt, and a higher extrusion risk. Compensate by adding metal structures near the anode and cathode or by local reinforcement, or else lower the limits.
- Source: L540–547, PDF p.16; Sect. 4.5.4, L6052–6061, L6097–6110, PDF p.132–133; Sect. 4.8.2, L6622–6635, PDF p.145.
- Philis stage: deck, dr.
- Automation recipe: a deck flag `low_k: bool` disables reservoir credits (EM-35) and restricts the Blech credit of EM-30 to deck-supplied values. It applies no numeric de-rating by default, because the book gives no factor.
- Beats hand layout because: consistent handling per process.
- Philis status: missing.

### EM-42 Material parameter defaults for derating when the deck is silent
- Kind: deck-requirement / data-model
- Statement:
  - Ea (eV), Table 2.1: Al bulk 1.2, grain boundary 0.7, surface 0.8; Cu bulk 2.3, grain boundary 1.2, surface 0.8.
  - Black parameters: Cu Ea 0.9 eV with n 1.1–1.3; Al Ea 0.7 eV with n = 2.
  - Void-migration Ea, Table 4.2: Al 0.61, Ag 0.66, Cu 0.70, Au 0.75, W 1.89. Resistivity (µΩ·cm): 2.44/1.47/1.54/2.03/4.84.
  - Cu/barrier interface Ea, Table 4.4: Ta 2.1, Ta/TaN 1.4, SiN or SiCxNyHz 0.7–1.1, SiN on Cu(Ti) 1.3, CoWP 1.9–2.4, SiCxHy 0.9.
  - Via temperature scaling uses 1 ≤ n ≤ 2.
- Source: L1241–1245, PDF p.31; Table 2.1 L1582–1588, PDF p.38; Table 4.2 L6547–6554, PDF p.144; Table 4.4 L6672–6680, PDF p.146; L4655–4656, PDF p.101.
- Philis stage: deck.
- Automation recipe: when the deck gives a current limit plus its reference temperature but no Ea or n, fall back per metal family: Cu Ea = 0.9, n = 1.2 (midpoint of 1.1–1.3); Al Ea = 0.7, n = 2; W contacts per deck only. Emit a provenance note in the report. With no reference temperature at all, keep today's no-derate behavior and warn.
- Beats hand layout because: derating is never silently skipped.
- Philis status: missing. No derating without deck fields (`elaborate.rs:255-257`).

### EM-43 Coupled EM/TM/SM flux model
- Kind: formula
- Statement:
  - J_E = (c/kT)·D₀e^(−Ea/kT)·z*eρj. J_T = −(cQ*/kT²)·D₀e^(−Ea/kT)·∇T. J_S = (cΩ/kT)·D₀e^(−Ea/kT)·∇σ.
  - J_a = J_E + J_T + J_S (Eqs. 2.15–2.18). 1-D: J_a = (Dc/kT)·ρjz*e + (Dc/kT)·Ω·∂σ/∂x (Eq. 2.19 as printed).
  - D = D₀·exp(−Ea/kT) (Eq. 2.20). Prevention means driving the net flux to zero.
  - TM is weak inside ICs (pure metals, high conductivity) and matters in solder: 10 K across 100 µm gives 1000 K/cm.
- Source: Sect. 2.5.3, L2383–2436, PDF p.54–55; L2153–2159, PDF p.49.
- Philis stage: verify.
- Automation recipe: use only as the theory behind EM-34. For IC wiring, drop J_T unless EM-04 finds gradients above a deck threshold (not given).
- Beats hand layout because: n/a (theory).
- Philis status: missing.

### EM-44 Korhonen stress evolution and nucleation criterion
- Kind: formula
- Statement: ∂σ/∂t = ∂/∂x[(D_a·B·Ω/kT)·(∂σ/∂x − z*eρj/Ω)] (Eq. 2.22, PDF p.66). The steady state is a linear stress profile (Fig. 2.36). σ > σ_critical means void nucleation, equivalent to a small resistance rise. The spread is large, so probabilistic handling is required.
- Source: Sect. 2.6.3, L2990–3060, PDF p.66–67.
- Philis stage: verify.
- Automation recipe: the steady state (∂σ/∂t = 0) gives the per-edge gradient used in EM-34. The transient form is not needed in P&R.
- Beats hand layout because: n/a (theory).
- Philis status: missing.

### EM-45 Tie EM damage to the IR-drop budget (absolute ΔR)
- Kind: check / metric
- Statement: the absolute resistance change ΔR_sat, through its voltage drop, is a better failure measure than relative ΔR, because transistors fail when an IR drop is exceeded [LDP+09]. A critical void volume is one that either breaks IR-drop function or causes thermal damage.
- Source: L5503–5518, PDF p.120–121.
- Philis stage: verify, flow.
- Automation recipe: for supply and bias nets that have an `IrDrop` constraint (`frontend/library/src/lib.rs:288`), give EM the IR-drop headroom. Allowed ΔR_fail per segment = (max_drop − present drop)/i_W. Feed it to EM-32. Nets with a large IR-drop margin get a jL² credit, and tight nets get none.
- Beats hand layout because: EM margin and IR margin are co-optimized instead of each being padded separately.
- Philis status: missing. `IrDrop` exists; the EM coupling does not.

### EM-46 Pre-characterized routing-pattern library (crowding factors)
- Kind: algorithm
- Statement:
  - Separate FEM from verification: FEM-verify a library of routing elements with parametric models, build layouts only from verified elements, and check the combined layout against current-density limits through those models.
  - Partition at homogeneous-current cuts (straight wire away from vias and branches); via-only patterns need wire appendices. Diffusion barriers bound the flux.
  - The maximum error at the cut interface is 3 %. One library run can beat one full-layout FEM run.
  - This leads to constraint-driven routing that uses only EM-robust elements.
- Source: Sect. 2.6.1, L2736–2857, Figs. 2.29–2.32, PDF p.61–63; Sect. 5.3, L7523–7570, PDF p.165–166; Sect. 5.5, L7596–7647, PDF p.167–168.
- Philis stage: dr, verify, deck.
- Automation recipe:
  - `dr` emits only a few primitives: straight run, L-corner (flush), T-junction, cut stack, cut array.
  - Once per PDK, solve 2-D Laplace (j = σE, E = −∇Φ; Eqs. 3.27–3.28) on each primitive with a small finite-difference solver. Store the peak/average current-density ratio (crowding factor) per primitive and aspect ratio.
  - Use that factor as g(H) (EM-14) and as a width multiplier on corners in EM-12.
  - Verification then multiplies the per-segment j by the pattern factor, with no full-layout field solve.
- Beats hand layout because: every corner and via gets a quantified crowding factor, which a human never computes.
- Philis status: missing.

### EM-47 Current-density field simulation (for residual hotspots)
- Kind: algorithm
- Statement:
  - j = σ·E, E = −grad Φ (Eqs. 3.27–3.28). Boundary conditions: fixed potential on one face and fixed j on another. The system is sparse and linear.
  - 2-D or 2.5-D is adequate only for coarse structures; 3-D is otherwise needed. Symmetry can at best halve IC structures.
  - A lumped model (one element per segment) is fast but has no spatial resolution. For power and ground nets, meshed methods are affordable.
  - Discard "dummy" corner-singularity errors.
- Source: Sect. 2.6.1, L2580–2670, PDF p.58–60; Sect. 3.5.5, L4770–4838, PDF p.104–105; L4572–4574, PDF p.100.
- Philis stage: verify.
- Automation recipe: keep the lumped tree solve (EM-09) for sizing. Optionally run a 2-D finite-difference solve per critical net's metal polygons on the manufacturing grid to find hotspots. Ignore single-cell corner singularities (cap at the EM-46 factor).
- Beats hand layout because: a quantitative map of every high-current net.
- Philis status: partial. Lumped DC tree only (`dr/lib.rs:888-897`). Meshes (loops) are cut to trees (`:895-896`), which misestimates current in parallel straps.

### EM-48 Net-level design lever priority (what automation can change)
- Kind: heuristic
- Statement: the five levers are limiting J, limiting T, layout modifications, material modifications and new materials. Only layout modifications are in the designer's hands. Thermal vias and wires lower T by only a few K, yet −5 K is worth about +25 % j (Al). Combining the Chapter 4 measures is estimated at ×10 permissible J.
- Source: Sect. 3.7, L4908–4965, PDF p.107–108; L1365–1367, PDF p.33; Sect. 5.1, L7456–7461, PDF p.164.
- Philis stage: flow.
- Automation recipe: order the EM work as (1) exact currents and sizing (EM-06 to EM-14), (2) geometry measures (EM-25, EM-30, EM-31, EM-35, EM-37), (3) temperature placement (EM-04, EM-11: keep high-current conductors away from hot devices). Thermal wires are a last resort.
- Beats hand layout because: the ×10 headroom goes to area and parasitics instead of being wasted as blanket width.
- Philis status: n/a (planning).

---

## 4. Top-15 priorities for Philis

1. **Per-terminal current bounds and a transient-based avg/RMS/peak op-point** (EM-06, EM-07, EM-08). Everything else in EM sizing depends on this. Today there is one DC number per pin (`dr/lib.rs:55`, `oppoint.rs:89-100`).
2. **Eq. 3.6 min/max segment current in `branch_currents`** (EM-09). This makes reversed-current and Steiner-segment sizing safe. It is a small change to the existing subtree sums at `dr/lib.rs:897`.
3. **Max-of-three width and cut sizing with RMS and peak limits** (EM-12, EM-14, EM-05). It catches RMS-dominated top-metal and ESD paths. It needs deck fields for j_rms, j_peak and the self-heating limit.
4. **Per-segment temperature from the thermal map, then per-segment derating** (EM-04, EM-03). This replaces the single die temperature (`elaborate.rs:245`) and couples to thermal-aware placement Philis already has.
5. **Deck fallback Ea/n with provenance, plus deck-requirement fields** (EM-42, EM-13, EM-31). Without Ea, n and Blech values in the sky130/ihp/gf180 decks, derating and Blech credit are silently off.
6. **Net classes (DC versus AC) with separate limits** (EM-22, EM-23). Uses the existing `NetRole` (`netrole.rs:11-15`) and stops over-widening AC signal nets.
7. **Criticality filter on current, not role** (EM-20, EM-21). It confines EM constraints to nets that need them and keeps analog power and signal nets covered, since both carry currents of the same order.
8. **Current-driven Steiner topology in `gr` minimizing Σ L·w(i)** (EM-10). This is the book's core analog-routing method and is absent today (0 `steiner` hits).
9. **Per-segment Blech domain with a via-above/via-below jL split** (EM-30, EM-31). This replaces the whole-run domain (`em.rs:109-117`) with the graph-edge definition and applies the ×10 via-below credit.
10. **Via-array geometry aligned to current direction, with g(H)** (EM-37, EM-14). The current overlap fill (`dr/lib.rs:757-786`) overloads the inner-corner cuts.
11. **Pin-region ampacity and access-point filtering** (EM-19). This is an analog-specific intrinsic-reliability check that no Philis stage does, and it needs a `cells` → `dr` interface.
12. **No 90° corners on critical nets: 45° chamfer or support polygons** (EM-25, EM-27). The Fig. 2.8 current-crowding rule is cheap to add at the L-corner emitters (`dr/lib.rs:1029`, `:1074`).
13. **Mission-profile limits (T_eff, AF factors, q_eff ∝ 1/m)** (EM-15, EM-16, EM-17, EM-18). Re-qualifying for automotive becomes a recompute. Implement from Black's law directly because of the printed-equation inconsistencies.
14. **Current-weighted placement term** (EM-11). It shortens high-current nets before routing, so fewer segments need widening or Blech credit.
15. **Pattern-library crowding factors plus a tree stress (Korhonen steady-state) check** (EM-46, EM-34, EM-44). This turns the conservative whole-run Blech rule into an exact tree immortality check and gives quantified corner and via crowding without a full field solve.

Source defects to keep in mind while implementing:
- Eq. 3.25's use of f(T) (already corrected in `em.rs:60-63`).
- The mixing of time and current-density factors in Eqs. 3.10–3.13, 3.19 and 3.20 (EM-15, EM-16).
- E_SiO2 = 70 GPa (Sect. 2.4.4) versus 300 GPa (Table 4.3) (EM-40).
