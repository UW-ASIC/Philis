# Charbon, Gharpurey, Miliozzi, Meyer, Sangiovanni-Vincentelli — *Substrate Noise: Analysis and Optimization for IC Design* (Kluwer 2001)

- Source PDF: `ref/Substrate Noise : Analysis and Optimization for IC Design -- Edoardo Charbon; ... .pdf`
- Reftext: `scratchpad/reftext/charbon_substrate.txt` (5063 lines, `pdftotext -layout`, pages split by form feed).
- Range read: **lines 1–5063 (entire file)**.
- Page convention: "p.N" is the printed book page; "PDF M" is the PDF page (M = N + 23 for the body; checked against the form-feed count, e.g. L3209 → PDF 122 = p.99).
- pdftotext dropped almost every displayed equation and every figure/table number. The equations, figure axes and table values quoted below were read from the PDF pages listed in §1. Values marked "read from figure" are plot readings (±2 dB / ±10 %), not printed numbers.
- Philis tree state: working tree on `main` at `dad330c` plus the uncommitted changes. Code citations are `path:line`.

---

## 1. Coverage

### 1.1 Consecutive Read chunks (reftext)

| Chunk | Lines | Note |
|---|---|---|
| 1 | 1–1200 | first attempt at 1–2000 exceeded the tool token limit; re-read in smaller chunks |
| 2 | 1201–2400 | |
| 3 | 2401–3600 | |
| 4 | 3601–4350 | 3601–5063 in one call exceeded the limit; split |
| 5 | 4351–5063 | end of file (index) |

Every line 1–5063 was read. The references (4642–4930) and index (4931–5063) were read quickly.

### 1.2 PDF pages opened to recover equations, figures and tables

PDF 35–41 (ch.2 eqs 2.1–2.11, Fig 2.2–2.6), 47–51 (eqs 3.6–3.19, Fig 3.3–3.5), 58–64 (eqs 4.1–4.12, Table 4.1), 70–83 (ch.5 eqs 5.1–5.10, Tables 5.1–5.5, Fig 5.2–5.10), 86–93 (eqs 6.3–6.9, Fig 6.2–6.9), 98–106 (Tables 6.1–6.7, Fig 6.13–6.18), 109–117 (eqs 7.1–7.4, Fig 7.1–7.5), 122–152 (all of ch.8: Fig 8.1–8.26, eqs 8.1–8.2, Table 8.1).

### 1.3 Every heading in the range

- Front matter: title/copyright (L1–56), Contents (L57–131), List of Figures (L133–311), List of Tables (L313–353), dedication (L354–357), Acknowledgments (L359–377), Foreword by P. R. Gray (L379–411)
- **Ch.1 Introduction** (L413–677): 1 Substrate Noise Problem (L467); 2 Analyzing Substrate Noise Transport (L511); 3 Optimization in the Presence of Substrate Noise (L565); 4 Design Practices (L629); 5 Book Organization (L651)
- **Ch.2 Noise Coupling Mechanisms** (L678–936): 1 Substrate Noise Transmission (L695); 2 Substrate Injection Mechanisms (L732); 3 Substrate Reception Mechanisms (L835); 4 Delay Effect (L874)
- **Ch.3 Analysis and Simulation** (L938–1327): 1 Substrate Macromodels (L956); 2 Electromagnetic Formulation (L973); 3 Boundary Element Methods (L1068); 4 Green's Function Computation (L1121); 5 Computational Techniques (L1212)
- **Ch.4 Substrate Modeling** (L1329–1753): 1 Switching Noise and Noise Signatures (L1347); 2 Use of Noise Signatures (L1396); 3 Multi-Port Substrate Models (L1423); 4 Generating Noise Signatures (L1510) — 4.1 Creating DDMs (L1583), 4.2 Iterative Refinement Process (L1644); 5 Case Study (L1696); Notes (L1750)
- **Ch.5 Constraint Generation** (L1755–2112): 1 Local Noise Generators (L1774); 2 Worst-Case Sensitivities (L1842); 3 Case Study (L1891) — 3.1 PLL Architecture (L1903), 3.2 Constraint Generation (L1993), 3.3 Constraint Enforcement (L2029)
- **Ch.6 Optimization Techniques** (L2113–2748): 1 Substrate-Aware Placement (L2152) — 1.1 Incremental Substrate Evaluation (L2205), 1.2 Modified Placement Algorithm (L2283); 2 Template-Based Extraction (L2392); 3 Case Study (L2574) — 3.1 Physical Design (L2603), 3.2 Trend Analysis and Technology Scaling (L2674), 3.3 Accelerated Extraction and Technology Selection (L2713); Notes (L2731)
- **Ch.7 Impact of Substrate on Performance** (L2749–3196): 1 Feedback (L2788); 2 Localized Potential Shifts (L2868); 3 Thermal Noise (L2898); 4 Substrate Losses (L2920) — 4.1 On-Chip Passives (L2924), 4.2 Active Circuits (L2993), 4.3 On-Chip Coupling of Spurious Signals (L3026), 4.4 A Circuit Application for Substrate Resistance (L3161); Notes (L3186)
- **Ch.8 Physical Design Guidelines** (L3197–3959): 1 Characterizing Conduction (L3209); 2 Characterizing Isolation (L3250) — 2.1 Low-Resistivity Substrates (L3268), 2.2 High-Resistivity Substrates (L3320); 3 Improving Isolation Using Differential Circuits (L3353) — 3.1 Low-Resistivity (L3365), 3.2 High-Resistivity (L3401); 4 Effects of Load Impedance (L3429); 5 Impact of Guard Rings (L3465) — 5.1 Low-Resistivity (L3494), 5.2 High-Resistivity (L3529); 6 Guard Rings in Single-Ended Circuits (L3541) — 6.1 Grounding Schemes (L3552), 6.2 Optimization of Guard Ring Widths (L3606); 7 Guard Rings in Differential Circuits (L3734); 8 Dual Guard Rings (L3787); 9 Buried Substrate Shields (L3813); Notes (L3946)
- **Ch.9 Conclusion** (L3960–3990)
- **App. A Boundary Element Method Derivations** (L3992–4052): 1 Nonzero Depth Contact Calculation (L3998); 2 Scaling Coefficient of Induction Matrix (L4022)
- **App. B Sensitivity Analysis** (L4054–4155)
- **App. C Convergence of Modified Placement Algorithms** (L4157–4210): 1 Modification of Search Space (L4163); 2 Substrate-Aware Placement (L4186)
- **App. D Measurement of Substrate Noise** (L4211–4641): 1 DC Measurements (L4234) — 1.1 Test-Chip Description (L4247), 1.2 Measurement Procedure (L4262), 1.3 Simulation Procedure (L4286), 1.4 Discussion of Results (L4291); 2 Small-Signal High-Frequency On-Wafer Measurements (L4343); 3 Time-Domain Measurements (L4446); 4 Frequency Domain Measurements (L4521); Notes (L4635)
- References [1]–[85] (L4642–4930); Index (L4931–5063)

---

## 2. Section-by-section digest

### Front matter (L1–411)
- Contents, list of 70+ figures and 17 tables; authors from UC Berkeley, TI, Conexant (L14–26, L57–353).
- Foreword: package lead impedance, thermal effects and substrate interactions are the main unforeseen interactions; substrate is the hardest to characterize; the need is a compromise between computational cost and accuracy plus substrate-insensitive design practice (L384–409).

### Ch.1 Introduction (L413–677)
- **1 Problem** (L467–509): substrate noise = intrinsic (thermal noise of the substrate resistance, usually ignored) + switching noise (digital current pulses injected by impact ionization and capacitive coupling) (L468–477). The current path is set by the relative position of injector, pickup, *other contacts*, doping profile and backplate potential (L481–485). Noise is a superposition of many sources each attenuated and delayed differently (L488–490). Reception by capacitive coupling and body effect; digital gates get Vt-modulated delays (L497–502).
- **2 Transport analysis** (L511–563): two families — direct PDE solution (FD/FEM, 3D cubes, versatile, handles lateral resistivity variation, costly, needs non-uniform grids) and integral-equation (BEM over contact surfaces, fewer unknowns, suited to incremental/ECO/optimization flows); the book uses the integral form (L537–563).
- **3 Optimization** (L565–623): early practice = large contact-dominated separation areas (bottom-up) (L567–569); coarse-grid FD + AWE placers need grid/layout alignment and can mislead (L581–593); analytical impedance networks are of unclear accuracy under small layout changes (L595–600); third approach = sensitivities of contact-to-contact impedances to layout moves (L602–603). Fig 1.3: VCO noise-sensitivity vs direction of divider displacement; Fig 1.4: noise concentrates near the high-frequency divider (L604–613). Sensitivities also select technology and evaluate migration without re-extraction (L615–623).
- **4 Design practices** (L629–649): qualitative practices suited to semi-automated flows; select process for design and architecture for process.
- **5 Organization** (L651–676): ch.2 sources, ch.3 transport, ch.4 acceleration, ch.5–6 substrate-aware design/optimization, ch.7 circuit implications, ch.8 physical-design guidelines.

### Ch.2 Noise Coupling Mechanisms (L678–936)
- **1 Transmission** (L695–730): switching currents injected at various depths, picked up near and far (L697–699). Two substrate types (Fig 2.2, PDF 35): *high-resistivity* = uniformly doped 20–50 Ω·cm (400 µm, with a 1 µm p 0.1 Ω·cm surface layer); *low-resistivity* = ~10 µm epi of 10–15 Ω·cm on a p+ bulk of ~1 mΩ·cm (300 µm, 1 µm p 1 Ω·cm surface) (L705–710). Low-res substrates are preferred for latch-up; high-res substrates respond better to guard rings and separation; all substrates are resistive below ~5 GHz (L710–715). Injected spectrum is wide, concentrated near 1/(average gate delay); impact ionization gives a DC component because it only produces positive current (L722–730).
- **2 Injection** (L732–833): vertical npn couples through collector–substrate junction C_js (eq 2.1) and a parasitic pnp in saturation (L733–755); lateral pnp through base–substrate C; vertical pnp's collector *is* the substrate → large injection unless a low-impedance drain is adjacent (L759–763). MOS inject through S/D junction C and impact ionization (eqs 2.2–2.4); impact ionization dominates NMOS substrate noise to at least 100 MHz (L764–791). PMOS ionization is small; a PMOS well that is not AC-grounded becomes a large injector via the well–substrate C (L799–807). Reverse-biased junction leakage adds DC drift current (L808–813). Distributed resistor injection eq 2.5; poly resistors have much smaller substrate C than diffused ones (L814–825). p+ taps/rings themselves inject if their potential bounces (L826–833).
- **3 Reception** (L835–872): capacitive sensing for bipolar, capacitors, resistors, interconnect; a lateral pnp base in a gain stage must be shielded or low-Z (L836–841). MOS body effect (eqs 2.6–2.7): g_mb/g_m typically 0.1–0.3, so the body-to-drain gain is only 14–20 dB below gate-to-drain; body effect dominates at low/medium frequency, capacitive pickup above ~1 MHz (L843–872).
- **4 Delay effect** (L874–936): digital delay t50% = R_EFF·C_G (2.8), R_tr = (L/W)/(µC_OX(V_DD−V_T)) (2.9), t50% ∝ 1/(√(φ_f+V_sb) − √φ_f) (2.10); substrate-under-wire coupling is modelled like interconnect crosstalk with C_c = C_s (Fig 2.6) and a closed-form peak (2.11) (L882–923); empirical crosstalk delay model ΔD ∝ L^α/S^β, α≈2, β≈1 [22] (L929–936).

### Ch.3 Analysis and Simulation (L938–1327)
- **Intro** (L945–954): full contact-to-contact impedance matrix is often too costly and unnecessary; partial extraction plus macromodels.
- **1 Macromodels** (L956–970): equivalent circuits fitted to measured/simulated two- or three-contact experiments (inject current at one, measure all others incl. backplate, sweep geometry) [11,12,23,24]; alternative numerical/semi-analytical PDE solutions.
- **2 EM formulation** (L973–1066): continuity equation (3.1); resistive approximation valid when dielectric relaxation time ε/σ ≪ time scale of interest, ~2–5 GHz except very low-resistivity material (L980–984). Laplace (3.2) with Dirichlet contacts (equipotential, subdivide if not), Neumann edges, current continuity σ1∂Φ/∂n = σ2∂Φ/∂n at interfaces (3.3) (L996–1008). Extract one admittance column by setting one contact to 1 V (L1009–1014). Integral methods competitive only if resistivity varies in one dimension or piecewise with few regions (L1030–1035). Green's theorem form (3.5), J = σ∇Φ (3.6).
- **3 BEM** (L1068–1119): resistance problem mapped to capacitance problem: C_ij = Q_j/Φ_i (3.7), R_ij = Y_ij⁻¹ = −(1/σε)Φ_i/Q_j (3.8); susceptance ≪ conductance up to 4–5 GHz so impedances are real (L1086–1088); Φ̄ = PQ, Q = cΦ̄, c = P⁻¹ (3.11); C_i = Σ_j c_ij, C_ij = −c_ij (3.12) → all mutual and ground resistances (PDF 48).
- **4 Green's function** (L1121–1210): layered-media Green's function avoids discretizing boundaries; series of cosines (3.13); p_ij closed form (3.16); the 64-term sum becomes a 2-D DCT (3.18) computed **once per substrate stack**, cost O(P̃Q̃ log P̃Q̃); moving contacts changes only P (size 50–5000); non-abrupt profiles by z-discretization (Fig 3.5); implemented in SUBRES (L1187–1210, PDF 50–51).
- **5 Computational techniques** (L1212–1327): FD 7-point stencil (3.19), sparse; direct factorization suffers 3-D fill; Krylov (CG, GMRES) with ILU, multigrid or multiresolution preconditioners (L1219–1248). BEM panels: dense Z (3.21), one solve per extracted column (L1249–1272); physically based approximate inverses [15,54,26] lack error control (L1277–1282); FCT tables + low-rank updates effective inside optimization loops (L1284–1290); precorrected FFT/DCT, hierarchical SVD compression (IES3), multilevel/moment-matching multigrid, wavelet bases give near-O(N) matvecs with user tolerance, exploiting that the potential is complex only near the source (L1291–1327).

### Ch.4 Substrate Modeling (L1329–1753)
- **1 Noise signatures** (L1347–1394): per-circuit substrate noise signature (current waveform injected) and supply noise signature (ripple) depend on implementation, technology and input vectors (L1358–1366). White/pink or clock-synchronous single-source models underestimate harmful components (L1373–1383). Parameterized multi-port models are the most used [23,11,12] (L1384–1388). SUBWAVE: characterize library cells once, combine with switching activity (L1389–1394).
- **2 Uses** (L1396–1421): floorplanning allocates isolated areas; minimum distance from noise spectral energy vs tolerated spurious energy [61]; guard rings tuned to problematic spectra when area is short (L1397–1405); noise spectra as IP interface data; noise-limited synthesis; EMC; fault fingerprints (L1408–1421).
- **3 Multi-port models** (L1423–1508): ΔK = Σ_ℓ (∂K/∂T_ℓ)ΔT_ℓ with ∂K/∂T_ℓ = Σ_ij (∂K/∂Y_ij)(∂Y_ij/∂T_ℓ) (4.1, PDF 58); Y_ii = Σ_j c_ij, Y_ij = −c_ij (4.2); ∂Q/∂T = −P⁻¹(∂P/∂T)Q (4.4); ∂c_ij/∂T = (1/Φ_j)∂Q_i/∂T (4.5); ∂p_ij/∂T via DCT (4.6). Storage example: 10 technology parameters, 1024×1024 grid → 41.9 MB, 1 µm resolution on 1×1 mm (L1502–1508).
- **4 Generating signatures** (L1510–1581): three phases: dynamic delay models (DDM) per gate/macro/interconnect parameterized by supply ripple, slope, fanout; event-driven simulation → switching traces s_n(t) = Σ_k A_k δ(t−t_k) (4.7); I_sub = Σ_n s_n(t) * i_sub,n(t) (4.8), I_dd likewise (4.9) (PDF 61).
- **4.1 DDMs** (L1583–1642): single-gate current pulse 1–100 µA, width 0.5–5× typical delay (L1588–1590, PDF 62); ΔV_DD and ΔV_GND effects are not separable by superposition but smooth → 2-D polynomial fit (4.10) with a sensitivity window (L1611–1628); injection pattern fitted to i(t) = A e^{−t/τ} cos(ωt−φ) (4.11); Table 4.1: A = 100 µA, τ = 6.3 ns, φ = 0, ω printed as "1.3 sec⁻¹" (units as printed) (PDF 63–64).
- **4.2 Iterative refinement** (L1644–1694): five-step loop; traces match when same count, direction and |Δt| < tolerance (4.12); REL_TOL 200, ABS_TOL 2000, TOL 3 time units = 0.3 ps (L1687–1694).
- **5 Case study** (L1696–1752): matches Verilog-XL on ISCAS85; <5 iterations when convergent; two oscillating non-convergent cases (C432 and C7552, pattern #2) driven by many simultaneous switchings (L1705–1723). Signatures depend strongly on stimulus → parametric or super-model over stimulus sets (L1729–1748).

### Ch.5 Constraint Generation (L1755–2112)
- **Intro** (L1762–1772): a bound on a precise parasitic guides a placer/router at decision time and localizes the cause of a spec violation.
- **1 Local noise generators** (L1774–1840): G_n(t, Π) models all substrate noise at node n; Π = [Wᵀ Gᵀ T V0]ᵀ (process, layout, temperature, local substrate potential). Sensitivity vector S (5.1), ΔK_i = Sᵀ·ΔΠ (5.2), bound ΔK_i ≤ ΔK̄_i (5.3–5.4); pick bounds by *maximize Flexibility(Π_bound) s.t. Sᵀ·ΔΠ ≤ ΔK̄_i, Π_min ≤ Π_bound ≤ Π_max* (PDF 71). Constraints are process-independent and generated once per circuit; process-dependent extraction enforces them during layout (L1828–1833). The analog section is small relative to distance to digital sources, so one waveform with per-node delays τ_m is assumed (L1834–1840).
- **2 Worst-case sensitivities** (L1842–1889): split parasitics into linear Π′ and nonlinear Π″; worst-case sensitivity = max over Π″ in its feasibility interval (5.5–5.6); ΔK_i = trace([S̄]·[ΔΠ′]) (5.8). Local generators act as antennas: evaluated without knowing injector positions; after extraction V0 = α·(source potential) with isolation factor α, which tells analog–digital placement and whether guard rings are needed (L1878–1889).
- **3 Case study / 3.1 PLL** (L1891–1991): top-down flow: behavioral models → block constraints → sizing with parasitic bounds → physical design enforcing them → extraction/verification (L1892–1901). F_out = F_ref·n/(mk); VCO F_out = F_0 + K_0 ΔV (5.9); specs F_min ≤ F_out ≤ F_max, J_rms ≤ J̄_rms, J_pp ≤ J̄_pp; finite-difference sensitivities (5.10) (PDF 74–75). Local generators at every schematic node: bulk-coupled ones model Vt modulation, capacitively coupled ones model direct coupling (L1963–1967). Worst case = impulsive, synchronous with the cell; sensitivity to V0 ≈ 2/3 (constant over 0–300 mV); ring-VCO jitter peaks when generator delay equals one inverter delay (Fig 5.3: ~280 ps at 0 delay to ~450 ps at 0.5 ns, V0 = 300 mV, read from figure) (L1978–1984). Loop filter sensitivity much lower → no constraints for it (L1985–1991).
- **3.2 Constraint generation** (L1993–2027): Table 5.1 VCO bounds: K_0 40 MHz/V ± 4 MHz/V, F_0 100 (printed "MHz/V") ± 10 MHz, J_rms ≤ 0.05 %, J_pp ≤ 1 % (PDF 76). Power-minimizing sizing raises parasitic sensitivity; add Sᵀ·ΔΠ_bound ≤ ΔK̄_i to the sizing problem (PDF 78).
- **3.3 Enforcement** (L2029–2111): VCOGEN floorplan: abut delay cells (min RC), multiple folding (contain mismatch), low cell aspect ratio (systematic mismatch 1-D), fully differential (coupled noise → common mode), mirror-symmetric delay cell (balanced branches reduce thermal and substrate noise effects) (L2031–2047). Wire R/C enforcement loop with R = ρL/W, C = C0·W·L (PDF 80). Substrate enforcement: chip-level technology-aware model, compute required isolation factor α (= R3/R2 in the Fig 5.7 model) to meet the V0 bound → minimum distance from each source (L2063–2073). Eight clusters, one contact per cluster sized to the total injecting area, SUBRES resistances to the VCO contact, SPICE with a fitted source (L2079–2085). Tables 5.2–5.5: period 6–20 ns ±10 %, pp jitter ≤ 100 ps; constraints C_out± 12.34 fF (extracted 11.60), C_x 15.84 fF (1.12), V0|VCO 110 mV (108 mV); sizes W_p 36 vs 25 µm with/without parasitic constraints; CPU substrate sensitivity 2545 s, interconnect sensitivity 3256 s, constraint generation 115 s, layout 43 s (PDF 81–83).

### Ch.6 Optimization Techniques (L2113–2748)
- **Intro** (L2120–2146): substrate is global — a local move changes the whole network; FD approaches recompute the whole workspace each iteration; integral methods exploit the locality of incremental changes.
- **1 Substrate-aware placement** (L2152–2203): objects, absolute constraints C^A and relative constraints C^R, Manhattan orientations NO_ROTATE…MIRROR_YX, legal configurations, minimize f(S) (NP-hard) (L2155–2197). Iterative improvement best exploits progressive extraction (L2201–2203).
- **1.1 Incremental evaluation** (L2205–2281): moving one contact changes one row/column of P → Sherman–Morrison c′ = c + δc, δc = −c_{·r}(c δP_{r·})/(1 + δP_{r·}c_{·r}) (6.3), O(N²) per moved partition (PDF 86). Gradient-based method: [c]_k = [c]_0 + Σ[δc]_n (6.4); ∇_vY = [A B]ᵀ, A = ∂Y/∂v_x, B = ∂Y/∂v_y (6.5); finite differences on the DCT grid (6.6); [Y]_n ≈ [Y]_0 + [∇_vYᵀ]_0 Δv (6.7). Accuracy 1 % within 5 grid steps, 10 % up to 1/10 of chip size (L2278–2281, PDF 88).
- **1.2 Modified placement** (L2283–2390): PUPPY-A simulated annealing with analog constraints; two remedies — distance (conventional moves) or guard-ring alternative implementations in an extended search space; book implements distance only (L2285–2298). Required: injector models (waveform + location), compact transport model, absorption/performance model (L2299–2312). Steps: (1) bounds Π_bound for critical nodes n_c (once per chip), (2) resistive network G_S(V,E) (Fig 6.3), (3) violations (L2319–2336). Heuristic Fig 6.6: per temperature exact Sherman–Morrison; per move gradient estimate (6.8); exact re-evaluation only when Π_j ≥ Π_j_bound at a critical node; the precise violation drives the cost like [70]; otherwise the substrate term is dropped (L2343–2387, PDF 90–91). Convergence retained (App. C).
- **2 Template-based extraction** (L2392–2572): reuse a pre-extracted template's c; update only contacts displaced relative to the template; Y_actual = Xᵀ c_actual X (6.9); worst case O(N³), identical template → zero cost (PDF 93). Template choice maximizes exactly overlapping contacts N_o; improved criterion prunes conductances whose cumulative performance effect is below a fraction of the allowed degradation and deletes nodes left with ≤1 conductance (Fig 6.10) (L2442–2449). Weighted extraction inaccuracy ε_K (6.15) bound (6.16), template selection (6.17), solved exhaustively per template; 2-D Taylor estimate of δc for small displacements (6.18–6.19), one "pole" contact per partition, ≤50 % error accepted (L2533–2572).
- **3 Case study** (L2574–2602): 140 MHz RAMDAC (3 DACs, PLL, digital), low-resistivity substrate like Fig 2.2(b); Table 6.1 PLL specs: jitter ΔT/T ≤ 0.007, phase margin ≥ 45° (PDF 98).
- **3.1 Physical design** (L2603–2668): jitter set by VCO → sensitivity model to peak-to-peak noise at ~85 critical VCO locations; PARCAR derived minimal constraints (2545 s) (L2604–2622). Three dividers are the injectors; SUBWAVE models; peak-to-peak by DC analysis on ± peak injector currents (resistive substrate) (L2629–2639). Divider n placed far from CP/VCO/LPF; divider k insensitive so placed freely; divider m traded noise vs interconnect C (L2655–2666). Table 6.2: manual 5637×6481 λ² (no jitter estimate); parasitic-aware 406.74 s, 6765×6528 λ², est. jitter 0.1; substrate+parasitic-aware 885.20 s, 7322×7716 λ², est. jitter 0.005 (PDF 101).
- **3.2 Trend analysis** (L2674–2711): Table 6.3 injectors: divider 152, PFD 23; receptors: VCO 85, CP 46, LPF 5 (PDF 102). Jitter sensitivity to R13 (divider→VCO cell path carries ~20 % of divider noise reaching the VCO) vs epi doping, contact depth, epi thickness (Fig 6.18); Table 6.4 CPU ~2.9–4.0 ks per trend, P of 1244×1244 inverted in 1525 s (PDF 104).
- **3.3 Accelerated extraction** (L2713–2729): Table 6.5 10×10 grid full 73.8 s vs sensitivity-based 9.8 s, max error 3 %; 2,500 contacts full not completed vs 57 min, 5 % (PDF 105). Table 6.6–6.7: contact-depth variance → R mean/variance; technology T2 chosen for higher probability of meeting all R bounds (0.927 vs 0.923) (PDF 105–106).
- **Notes** (L2731–2747): polygon centre = centre of mass of the approximating rectangle; rough max/min substrate conductances from two contacts at chip edges or in close proximity (L2742–2744).

### Ch.7 Impact of Substrate on Performance (L2749–3196)
- **Intro** (L2756–2782): three effects: feedback within a circuit, interference between circuits (tonal/wideband noise), lossy substrate (passive Q, thermal noise).
- **1 Feedback** (L2788–2866): bond-pad C_sbp 0.5–1 pF, R_sbp from several Ω to several kΩ; collector–substrate C_s1, C_s2 in series via R_s12 appear as a Miller C at Q1's collector: 0.2 pF each with Q2 gain 10 → 1 pF (L2792–2810, PDF 109); bandwidth 1.9 → 1.8 GHz in a series-series amplifier (L2813–2815). Mitigations: guard ring near Q1's collector, grounded backplate, differential implementation with close devices (costly), feedback (L2816–2831). MOS: Miller via substrate, DC feedback via body effect and g_db (L2832–2836). Substrate feedback limits max open-loop gain (Barkhausen) and raises |S12|, lowering MSG = |S21/S12|; MSG = g_m/(ω[C_µ + (C_s1‖C_sbp)/{1 + ωR_s1bp(C_s1‖C_sbp)}]) (7.1, PDF 111).
- **2 Localized potential shifts** (L2868–2896): ΔC_j = −(C_j/n)(ΔV_bias/ψ0)/(1 + V_bias/ψ0) (7.2) shifts poles; ΔI_d = g_mb ΔV_bias (7.3) shifts gain; local substrate contacts near sensitive devices fix the local potential, especially in high-res substrates (PDF 111–112).
- **3 Thermal noise** (L2898–2918): substrate resistances are noise generators; LNA NF degraded unless the input pad is shielded (buried n-well to supply or lower metal, Fig 7.3); output thermal noise peaks when reactive and resistive parts of the pad impedance are similar; very high-resistivity substrate also helps.
- **4.1 Passives** (L2924–2991): substrate loss lowers inductor and capacitor Q near self-resonance; connect the top plate to the active node and the bottom plate to the reference (L2936–2944); AC-coupling bottom-plate substrate R can help single-ended (L2945–2947); two adjacent differential AC-coupling caps have bottom plates coupled by R12 missing from single-ended "brittle" models → attenuation 1/(1+r), r = 0.2 → 1.6 dB loss (L2954–2981, PDF 115); same pitfall for device models reused in close differential placement (L2982–2986).
- **4.2 Active circuits** (L2993–3024): input/output matching gain 1/(1−|S11|²), 1/(1−|S22|²); substrate loss (series R–C) lowers |S11| (7.4); worst for cascodes with high output impedance.
- **4.3 Spurious coupling** (L3026–3159): seven interaction classes: PA→LNA (>100 dB power ratio, substrate not frequency selective), digital harmonics (n-th harmonic 20·log(n) dB below fundamental; 10 MHz 3 V clock: fundamental 1.91 V, ~GHz harmonic 19.1 mV, −40 dB), low-frequency noise upconversion by second-order distortion with a jammer (differential circuits help), beat products/frequency plan, DC offsets from clock/LO leakage into samplers/mixers, frequency pulling by PA, oscillatory loops across high-gain stages (direct conversion 80–100 dB at one frequency) (L3052–3146, PDF 117–119). Junction/interconnect caps in digital circuits push HF energy *into* the substrate; isolated device capacitors shunt it to circuit ground (L3073–3080). Circuit remedies: frequency plan with digital harmonics, stagger ADC output-buffer enable vs sampling (L3151–3156).
- **4.4 ESD** (L3161–3184): ggNMOS relies on substrate current forward-biasing the parasitic npn; substrate resistance is critical and nonlinear (conductivity modulation).
- **Notes** (L3186–3195): feedback case when transistors are close, have large substrate C and few/small substrate contacts in high-res substrates.

### Ch.8 Physical Design Guidelines (L3197–3959)
- **1 Conduction** (L3209–3248): single-contact impedance vs width: high-res weak (logarithmic) dependence because the surface layer spreads current (Fig 8.2: ~600 Ω at 10 µm to ~90 Ω near 700 µm, read from figure); low-res near-inverse with area (Fig 8.3: ~3–5 kΩ at 10 µm to ~2 Ω at 1000 µm, epi 1–15 Ω·cm) (PDF 123). Two 20 µm contacts: R12 rises monotonically with distance; in high-res R10 to backplate is non-monotonic (surface flow when close, backplate flow when far, rises again near die edge) (Fig 8.4–8.5, PDF 124–125).
- **2 Isolation** (L3250–3266): I = 20·log(V_rcv/V_inj); depends on C_sub, backplate impedance, frequency and receiver load; nominal 50 Ω load (PDF 124–125).
- **2.1 Low-res** (L3268–3318): 50 µm contacts in a 1000 µm die, ρ_surface 1 Ω·cm, ρ_epi 15 Ω·cm, ρ_bulk 10 mΩ·cm; L_gnd 5/3/1/0 nH, 1 GHz and 100 MHz, C_sub 0.3 and 0.8 pF (PDF 125–128). Floating backplate: isolation ~flat with distance (R12 ≪ X_Csub). With L_gnd 1–5 nH isolation stops improving past a critical distance of 2.5–5× epi thickness (10 µm) at 1 GHz (Su et al.: 4×); only L_gnd = 0 keeps improving (L3289–3296, PDF 127). Z′12(jω) = R10 + R20 − jR10R20/(ωL_gnd) (8.1); HF isolation degrades ~linearly with contact area; larger C_sub degrades isolation (L3302–3318).
- **2.2 High-res** (L3320–3351): ρ_epi 0.1 Ω·cm, ρ_bulk 20 Ω·cm; weak dependence on backplate L (surface conduction); taps more efficient than in low-res; isolation keeps improving with distance even with finite backplate L; weaker area dependence (PDF 127–130).
- **3 Differential** (L3353–3363): noise appears as common mode; differential noise orders of magnitude smaller.
- **3.1 Low-res** (L3365–3399): two 200×10 µm² receiver contacts at centre, 50×50 µm² sliding injector (PDF 130–131). Injector moved along y (the pair's symmetry axis) → perfect balance, infinite differential isolation; along x (pair axis) → worst case (L3382–3386). Isolation falls rapidly with distance and is insensitive to backplate L, because the p+ bulk makes backplate resistance independent of contact position (L3387–3399). Fig 8.9(b): 1 GHz ≈ −35 dB near → ≈ −120 dB at 400 µm; 100 MHz → ≈ −153 dB (read from figure).
- **3.2 High-res** (L3401–3427): smaller differential improvement — the nearer contact shields the farther one (L3406–3409). 5 % mismatch of receiver substrate C (low-res) collapses differential isolation, worst for distant contacts (Fig 8.10b: 1 GHz ≈ −80 dB with mismatch vs ≈ −120 dB without; 100 MHz ≈ −123 vs ≈ −150 dB, read from figure) (L3410–3427, PDF 133).
- **4 Load impedance** (L3429–3463): at 90 µm, L_gnd 3 nH, C_sub 0.8 pF: internal node voltage is load-independent while the load pole 1/(2πR_load C_sub) is above the noise frequency; isolation then scales linearly with R_load (100 MHz); pole ≈ 200 MHz at 1 kΩ, 1 GHz at 200 Ω (PDF 134–135). Fig 8.11: 1 GHz ≈ −57 → −27 dB, 100 MHz ≈ −103 → −67 dB over 10 Ω–1 kΩ (read from figure).
- **5 Guard rings** (L3465–3488): majority-carrier heavily doped surface ring = Faraday shield / current sink; raises effective R12; L_gr = ring bond-wire + pin inductance, L_gnd = backplate; ring useful only if X_gnd and X_gr are small (PDF 135).
- **5.1 Low-res** (L3494–3527): 10 µm ring between two 2500 µm² contacts: only 7–10 dB improvement, weak function of L_gr; big gains only by lowering backplate L (ring sinks only the surface current); best practice = excellent backplate ground (PDF 136–138).
- **5.2 High-res** (L3529–3539): rings very effective; lowering L_gr improves isolation by large amounts (Fig 8.13b: 1 GHz ≈ −45 dB at L_gr 5 nH vs ≈ −65 dB at 0 nH; 100 MHz ≈ −95 vs ≈ −103 dB, read from figure) (PDF 137–138).
- **6.1 Grounding** (L3552–3599): ring around receiver (Fig 8.15, a = 50, d = 10, b = 100 µm). External ground via an independent bond-wire is best when the signal is referenced to an external ground; a ring tied to the internal ground with large L_gr is *worse than no ring*; internal ground is right only when the signal is measured against internal ground, where isolation becomes independent of L_gr (L3583–3598). Width 2→34 µm helps only at L_gr = 0 → use thin rings. Fig 8.17(a): no ring −55 dB (100 MHz), −27 dB (1 GHz) (PDF 139–142).
- **6.2 Width optimization** (L3606–3732): model with ring bond-wire noise V_inj1 (Fig 8.19–8.20); V2 = R_l{(1/R12 + jωL/(R1grR2gr + jωL(R1gr+R2gr)))V_inj + (R1gr/(R1grR2gr + jωL(R1gr+R2gr)))V_inj1} (8.2, PDF 143). Wider w raises R12 and lowers R1gr, R2gr; LF isolation set by R12 (wide good); HF by R1gr + R2gr; ring loses advantage above ω ≈ R1grR2gr/(L(R1gr+R2gr)); V_inj1 appears scaled by R_l/R2gr (L3636–3650). Small d (ring close to receiver) raises R12 area-efficiently but lowers R2gr (more V_inj1); ring around injector is symmetric for V_inj and suppresses V_inj1 (L3651–3658). Ideal: ring around the noisy circuit with its bond-wire among the analog bond-wires (L3669–3671). Width sweep 4–32 µm step 2 µm, C_sub 0.8 pF, L_gr 0–8 nH (PDF 144–146). Thin rings best at ~1 GHz; shallow optimum at 100 MHz (15 µm at 2 nH, Fig 8.21a); **rule: minimum-width ring with a very good ground** (L3695–3704). Grounded backplate adds 5–10 dB → surface rings are the primary ground path in high-res (L3704–3708). Ring around injector vs receiver: same V_inj isolation, ~20 dB better vs V_inj1 around the injector (L3709–3720). Example: 1 mA at 1 GHz, M = 1 nH → V_inj1 = 6.3 mV → 1.41 mV at receiver (receiver ring) vs 0.14 mV (injector ring); 1 V injector → 5.623 mV; crossover 35 mA (injector ring) vs 3.5 mA (receiver ring); no ring 22 mV (note 4) (L3721–3732, L3955–3956, PDF 147). Text (L3684–3686) says Fig 8.21 = no backplate and Fig 8.22 = grounded, but the figure captions and the list of figures (L283–288) say Fig 8.22 is the floating-backplate case; the captions are consistent with the "backplate adds 5–10 dB" statement (no-ring values −53/−25 dB in 8.21a vs −38/−21 dB in 8.22a).
- **7 Differential + ring** (L3734–3785): ring = nearby ground plane → makes coupled noise common mode; ring width 10–48 µm step 2 µm, L_gr 0/4/8 nH (Fig 8.24); differential-mode isolation improves slightly with width and is independent of L_gr; common-mode isolation worsens with width for L_gr > 0 → thin rings; mismatch degrades (PDF 149–150). Fig 8.25: DM ≈ −92…−100 dB at 1 GHz, ≈ −130…−140 dB at 100 MHz (read from figure).
- **8 Dual rings** (L3787–3811): rings around both, grounded separately (tied together they act as one ring); 50 µm contacts, b = 300 µm, 10 µm rings, 5 nH each, C_sub 0.8 pF; Table 8.1: none −62/−34 dB, receiver ring −99/−45 dB, two rings −130/−57 dB (100 MHz/1 GHz) (PDF 151). Use two rings if two pins are available.
- **9 Buried shields** (L3813–3944): (a) p+ buried Faraday shield — near-ideal with ideal ground, very sensitive to ground-path impedance, floats with bond-wire L at HF; (b) SOI/dielectric — good LF, degrades with frequency (series C), improve by thicker oxide; unpatterned; (c) buried n+ well to highest potential — depletion (dielectric) + Faraday, less sensitive to both non-idealities; hard under PMOS n-well or npn buried collector (depths up to several µm) (L3826–3866, PDF 152–153). Models Fig 8.28–8.30; shield resistivity > 0 lets coupling through even with perfect ground; isolation ∝ 1/ground-path impedance (L3867–3886). High-conductivity n+ shield: T-attenuator, near-ideal LF, degrading with frequency; low-doped shield: series junction C, better HF (L3923–3936). **Place the shield around the smaller-area circuit** (usually analog) — smaller C, larger reactance vs ground impedance (L3937–3944, PDF 156).
- **Notes** (L3946–3958): load = impedance looking into the device from the substrate incl. feedback; not placing the ring avoids V_inj1 altogether and a ring can increase noise if V_inj1 is large; R_l smaller than all other resistors assumed.

### Ch.9 Conclusion (L3960–3990)
- Total integration makes substrate interaction a first-order problem; the book offers analysis, acceleration for optimization loops, measured validation, and packaging-aware guidelines to size protective structures.

### App. A (L3992–4052)
- **A.1** nonzero-depth contacts: modified (3.10)/(3.16) with contact depth (Fig A.1), reduces to (3.16) at zero depth (L3998–4019).
- **A.2** coefficient-of-induction scaling: admittance from c via a diagonal operator and unity vectors (A.4–A.9) (L4022–4052).

### App. B Sensitivity analysis (L4054–4155)
- Recursive formulas for ∂Γ_N/∂T, ∂β_N/∂T for doping levels and layer thicknesses (B.1–B.9); sensitivity of k_mn and p_ij (B.10–B.11); contact-depth sensitivity via DCT of ∂k_mn/∂c (B.12–B.13); precompute DCTs for several depths (L4151–4155).

### App. C Convergence (L4157–4210)
- **C.1**: SA with a dynamically modified search space remains a homogeneous, regular, recurrent Markov chain → convergence per Romeo [73] and Hajek [74] (L4163–4183).
- **C.2**: substrate (and electrothermal) resistive networks are linear, storage-free and bounded → estimates bounded → convergence retained, also when cells carry alternative guard-ring implementations [82] (L4186–4210).

### App. D Measurement (L4211–4641)
- **D.1 DC** (L4234–4341): 20 ohmic p+ contacts + 4 rings (inner solid, outer rings of parallel taps), gold backside contact grounded (L4247–4260); 0.1 V bias (1 V showed weak nonlinearity) (L4264–4271); V_j = f(R_ij, R_j0) (D.1); full admittance matrix measured (L4280–4284). Match < 15 % for voltage ratio > 0.5, good > 0.1, degrades below 0.1 with systematic error from the approximate multi-layer profile; contact resistances also missing (L4293–4327).
- **D.2 HF two-port** (L4343–4444): S → Y parameters vs quasi-static FEM; quasi-static valid to several GHz (L4360–4368); sweep distance and port size to calibrate a simulator (L4369–4374). Concentric disc/ring ports make coupling a function of distance only (L4380–4385). Pfost: ring wiring inductance kills isolation above 10 GHz; closely spaced ring ~22 dB at ~10 GHz only because very close to injector with tiny ground Z (L4391–4403); probe floor −40…−45 dB (L4403–4409). Joardar [81]: ~40 dB from rings around both devices; buried n+ better than SOI (L4409–4417). On-wafer tests cannot show package ground-path L effects (L4418–4423).
- **D.3 Time domain** (L4446–4519): Su [11] low-res: isolation independent of distance beyond ~4× epi; p+ ring with dedicated ground near the sensor ~20 % improvement; distant ring none; ring tied to the substrate-bias pad *increases* noise (package L of the substrate contact); minority (n-type) rings no effect (current flows in p+ bulk) (L4465–4485). Makie-Fukuda [84]: statistical comparator measurement (L4490–4519).
- **D.4 Frequency domain** (L4521–4641): Gharpurey [85]: on-chip heterodyne downconversion separates substrate from package/board coupling; 1.5 GHz differential VCO injector, n-well bottom-plate sensors, mixers U (upper reference), F (floor), M (6 sensors) (L4533–4604). Coupling −59…−47 dB below VCO; open VCO outputs worst; all sensors within 3 dB (low-res → distance-insensitive); the mixer with a ring tied to its own substrate tie was consistently *worse* (shared substrate node with ESD cells) (L4605–4632). Note: absolute isolation numbers are only comparable within one known cross-section (L4636–4641).

### References and index (L4642–5063)
- 85 references; key ones for P&R: [7] Stanisic power-distribution synthesis, [8] WRIGHT substrate-aware placement, [10] Voronoi macromodels, [11] Su measurements, [12] Joardar crosstalk model, [13] Charbon TCAD'99, [29] Gharpurey–Meyer, [44] Charbon thesis, [70–71, 82] PUPPY-A, [72] Choudhury constraint generation, [81] Joardar isolation.

---

## 3. Actionable extraction

Entries are grouped: deck/data model (01–05), injector/receptor physics for the annotator (06–14), constraint generation (15–20), placement algorithms (21–28), guard rings / taps / shields (29–42), differential and matching (43–46), circuit-level parasitics a P&R tool can see (47–52), extraction and verification (53–58).

### SUB-01 Substrate stack is a deck input; rules branch on substrate kind
- Kind: deck-requirement
- Statement: Substrates fall in two electrical classes with opposite layout remedies. High-resistivity bulk: uniformly doped 20–50 Ω·cm (Fig 2.2: 400 µm, with a 1 µm 0.1 Ω·cm surface layer). Low-resistivity epi: ~10 µm epi of 10–15 Ω·cm on a ~1 mΩ·cm p+ bulk (300 µm, 1 µm 1 Ω·cm surface). High-res responds to guard rings and distance; low-res responds mainly to backplate grounding. Both are resistive below ~5 GHz.
- Source: ch.2 §1, Fig 2.2; ch.8 Fig 8.1; L705–715; p.12 (PDF 35), p.99 (PDF 122).
- Philis stage: deck, annotator, cells, verify.
- Automation recipe: add a `substrate` object to `pdks/*.json`: `kind` ∈ {`bulk`, `epi_on_pplus`, `soi`}, `layers: [{t_um, rho_ohm_cm}]` top-down, `backplate` ∈ {`grounded`, `floating`}, `l_gnd_nh` (package/down-bond), `l_ring_nh` (ring pin + bond-wire), `dnwell: bool`. Load into `ProcessNumbers` (`backend/annotator/src/netrole.rs:102-104`, replacing the bare `epi_nm`) as `substrate: Option<SubstrateStack>`; `SubstrateStack` lives in `pnr_core` next to `thermal.rs` so annotator, cells, dp and verify share it. Every rule below reads `kind` first. Missing stack → rules become cost-only pulls and the check reports "unknown", as the epi fallback does today (`backend/annotator/src/lib.rs:137-139`).
- Beats hand layout because: the tool applies the correct remedy class per PDK automatically; hand designers carry folklore ("guard rings always help") that is wrong in epi substrates (§8.5.1).
- Philis status: partial. Only `epi_nm` exists (`netrole.rs:102-104`); the four decks carry `p_epi_thickness: 3000` (`pdks/sky130.json:66`, `gf180mcu.json:68`, `ihp_sg13g2.json:63`), which `emit.rs:250-256` documents is a guard-ring depth default, not an epi thickness; sky130 is noted as bulk p-substrate (`emit.rs:253-255`). No kind, resistivity, backplate or inductance fields.

### SUB-02 Quasi-static resistive model is valid to 2–5 GHz; peak noise by DC analysis
- Kind: rule
- Statement: Neglect the displacement term of ∇·(σ∇Φ) + ∂/∂t ∇·(ε∇Φ) = 0 (3.1) when ε/σ ≪ time scale of interest: valid up to ~2–5 GHz except very low-resistivity material; substrate susceptance ≪ conductance up to 4–5 GHz, so contact impedances are real. With a resistive substrate, peak-to-peak substrate voltage at every node = DC analysis on the positive and negative peak injector currents. Confirmed by two-port measurements valid "up to several GHz".
- Source: ch.3 §2 L980–984 (PDF 45), §3 L1086–1088 (PDF 48); ch.6 §3.1 L2636–2639 (PDF 99); App. D.2 L4363–4368 (PDF 175).
- Philis stage: flow, dp, verify.
- Automation recipe: model the substrate as a linear resistor network (plus device-to-substrate capacitors when computing isolation in dB, SUB-33). Placement cost uses peak injector current × transfer resistance; no transient simulation in the loop. Gate: if the design's highest signal/clock frequency (from netlist/op-point metadata) exceeds 2 GHz, mark the estimate "approximate".
- Beats hand layout because: allows evaluating every aggressor–victim pair per move instead of a few pairs by intuition.
- Philis status: missing (no substrate network exists). The pattern exists for thermal: linear superposition refreshed per epoch (`kernel/core/src/thermal.rs:1-4,22`).

### SUB-03 Contact network data model G_S(V,E) with ground conductances
- Kind: data-model
- Statement: Map a placement to a fully connected graph whose vertices are substrate contacts (device junction footprints, taps, rings, backplate as node 0) and whose edges carry conductances Y_ij; Y_ii (to backplate) = Σ_j c_ij, Y_ij = −c_ij where c = P⁻¹ is the coefficient-of-induction matrix. The two-contact special case is the three-element model R10, R20, R12 (Fig 3.3).
- Source: ch.3 §3 eqs 3.11–3.12 (PDF 48), Fig 3.3 (PDF 49); ch.4 eq 4.2 (PDF 59); ch.6 §1.2 Fig 6.3, L2327–2334 (PDF 89).
- Philis stage: data-model (pnr_core), dp, verify.
- Automation recipe: `pnr_core::substrate::Network { contacts: Vec<Contact{target: Target, rect, kind: Injector|Receptor|Tap|Ring}>, g: Vec<f32> /* dense N×N, N small */ }`. Contacts come from cells (junction rects per device; one per placed cell for the cluster abstraction SUB-26) and from `post_cell` rings/taps. Build Y from a transfer model (SUB-31/32 cheap, SUB-53/54 exact). Victim node voltage = solve (Y + Y_load) V = I_inj with the backplate node tied through L_gnd for AC.
- Beats hand layout because: every contact including taps and rings participates; hand analysis considers one injector–victim pair and ignores shunting by third contacts (L481–485).
- Philis status: missing.

### SUB-04 Isolation definition and two-port macromodel (closed form per pair)
- Kind: formula
- Statement: I = 20·log(V_rcv/V_inj). Circuit (Fig 8.6a): V_inj → C_sub1 → node 1; R10, R20 from nodes 1, 2* to backplate node, backplate to ground through L_gnd; R12 between 1 and 2*; 2* → C_sub2 → receiver node with R_load to ground. With small C_sub reactance, small load and R12 ignored: Z′12(jω) = R10 + R20 − jR10R20/(ωL_gnd) (8.1). The load matters linearly while the load pole 1/(2πR_load C_sub) is above the noise frequency (1 kΩ with 0.8 pF → ≈ 200 MHz; 200 Ω → 1 GHz).
- Source: ch.8 §2 L3250–3266, Fig 8.6(a) (PDF 126), eq 8.1 L3302–3306 (PDF 127), §4 L3438–3462 (PDF 134–135).
- Philis stage: verify, dp (cost), annotator (budget).
- Automation recipe: per aggressor–victim pair solve the 3-node circuit (nodes 1, 2*, backplate) analytically at the aggressor's frequency f (clock frequency from netlist or the deck default), inputs R10(A1), R20(A2), R12(d) from SUB-31/32, C_sub from SUB-07, R_load = |Z| of the victim node (SUB-12), L_gnd from the deck. Output isolation dB and margin vs the victim budget (SUB-16). O(1) complex arithmetic per pair; all pairs per epoch.
- Beats hand layout because: quantitative, per-pair, frequency- and package-aware; a human cannot evaluate O(injectors × victims) pairs.
- Philis status: missing. `Isolation` scores edge gap only (`kernel/analog/src/placement/isolation.rs:22-37`).

### SUB-05 Backplate impedance dominates isolation in epi substrates
- Kind: rule
- Statement: In low-resistivity (epi on p+) substrates isolation is "very sensitive" to backplate impedance; at a fixed spacing the variation over L_gnd 0–5 nH is very large (Fig 8.6b at 1 GHz: plateau ≈ −47 dB at 5 nH, ≈ −62 dB at 1 nH, still falling to ≈ −112 dB at 400 µm for 0 nH; read from figure). A floating backplate makes isolation distance-independent (≈ −22 dB at 1 GHz, C_sub 0.3 pF). Guard rings in these substrates give only 7–10 dB; "the most effective way of improving isolation is to provide a very good ground contact to the backplate".
- Source: ch.8 §2.1 L3281–3318 (PDF 125–128); §5.1 L3510–3527 (PDF 136–138).
- Philis stage: deck, verify, flow (report).
- Automation recipe: deck field `backplate` and `l_gnd_nh` (SUB-01). For `epi_on_pplus`, verify reports isolation with the deck L_gnd and flags victims whose budget is only met at L_gnd = 0 as "needs backside/down-bond ground" (a package action, outside P&R). Placement stops spending area beyond the saturation distance (SUB-31).
- Beats hand layout because: tells the designer which decisions the layout cannot fix, instead of adding rings and area that do nothing.
- Philis status: missing.

### SUB-06 Device injector taxonomy (what injects, how much)
- Kind: heuristic
- Statement: vertical npn: collector–substrate C_js (2.1) plus a parasitic pnp when the npn saturates. Lateral pnp: base–substrate C. Vertical pnp: the substrate *is* the collector → large injection "unless low impedance draining is provided in immediate proximity". NMOS: S/D junction C and impact ionization (dominant to ≥100 MHz). PMOS: small ionization; the n-well injects through its large well–substrate C if not AC-grounded. Resistors: poly ≪ diffused in substrate C. n-diffusions: reverse-bias C. p+ taps/rings: inject any bounce on their net over their whole extent. Reverse-biased junctions add DC leakage drift current.
- Source: ch.2 §2, Fig 2.4; L732–833; pp.12–16 (PDF 35–39).
- Philis stage: annotator.
- Automation recipe: `annotator::substrate::injector_weight(dev, op, net_classes) -> f32` per device: 0 for devices on only DC nets; NMOS on a Clock net or with large |dV/dt| drain (switching: comparator/latch outputs, `catalog.rs` LATCH_HALF at `backend/annotator/src/catalog.rs:416`) → high; vertical PNP (from `DeviceKind`/bjt model) → high and requires an adjacent tap (SUB-37); PMOS → weight scaled by well tap resistance (SUB-36). Output feeds SUB-15/16.
- Beats hand layout because: exhaustive classification of every device; humans mark "the digital block" and miss analog switching devices (latches, choppers, sampling switches).
- Philis status: partial. Aggressors = devices on a `NetClass::Clock` net that are not sensitive (`backend/annotator/src/emit.rs:272-273`); no device-type or op-point weighting; no vertical-PNP rule.

### SUB-07 Junction and distributed capacitance to substrate (injection and pickup)
- Kind: formula
- Statement: abrupt junction per unit area C_js = √(qε/(2(ψ0+V_cs)) · N_C N_S/(N_C+N_S)) (2.1). Distributed resistor with one end at AC ground: I = √(jωC/R) · tanh(√(jωRC)·ℓ/2) · V_in (2.5), C and R per unit length, ℓ length. Bias shift changes C_j by ΔC_j = −(C_j/n)(ΔV_bias/ψ0)/(1 + V_bias/ψ0) (7.2).
- Source: eq 2.1 L742–749 (PDF 36); eq 2.5 L819–825 (PDF 39); eq 7.2 L2870–2878 (PDF 111).
- Philis stage: cells, annotator, verify.
- Automation recipe: C_sub per contact = C_j,area·A + C_j,sw·P from the deck's junction capacitances (area/perimeter) and the drawn diffusion/well geometry that cells already produce; well contacts use the n-well–substrate C. Resistor injection: evaluate (2.5) at the aggressor frequency with deck per-square R and C of the resistor layer; prefer the lower-C layer (poly) for resistors on sensitive nets when the netlist leaves the choice open.
- Beats hand layout because: the C_sub term in SUB-04 is computed from real geometry per device rather than a rule-of-thumb constant (the book uses fixed 0.3/0.8 pF).
- Philis status: missing (no junction C per contact; verify PEX via GPurify does not emit substrate nodes; only a diode substrate marker is referenced at `backend/verify/src/lib.rs:484`).

### SUB-08 Impact-ionization current from the operating point
- Kind: formula
- Statement: I_impact ≈ (A/B)·l·E_m·I_d·e^{−B/E_m} = C1(V_ds − V_dsat)·I_d·e^{−C2/(V_ds−V_dsat)} (2.3); small-signal g_db = ∂I_sub/∂V_D = C2·I_sub/(V_ds−V_dsat)² (2.4), which lowers r_o. C1, C2 are material constants (numbers not given). Dominant NMOS substrate-noise mechanism up to at least 100 MHz; PMOS much smaller (lower hole ionization coefficient).
- Source: ch.2 §2 eqs 2.2–2.4, L768–801 (PDF 38).
- Philis stage: annotator, deck.
- Automation recipe: op-point already gives `id`, `vds`, `vdsat` per device (`frontend/library/src/oppoint.rs:184-186,204-205,437-446`). Add deck constants `ii_c1_per_v`, `ii_c2_v` (BSIM `alpha0/beta0`-equivalent; "not given" in the source, so absent → rank by I_d·(V_ds−V_dsat) only and mark the estimate relative). Injector current for SUB-15: NMOS I_sub from (2.3); switching devices use the pulse model of SUB-10.
- Beats hand layout because: identifies high-V_ds, high-current NMOS (cascode bottoms, output stages) as injectors even in a purely analog block, which humans rarely treat as aggressors.
- Philis status: partial (op-point data present, not used for injection).

### SUB-09 Body effect reception gain → victim ranking
- Kind: formula
- Statement: V_t = V_t0 + (√(2qεN_A)/C_ox)(√(2φ_f+V_sb) − √(2φ_f)) (2.6); g_mb/g_m = √(2qεN_A)/(2C_ox√(2φ_f+V_sb)) (2.7), typically 0.1–0.3, so the body-to-drain gain is only 14–20 dB below gate-to-drain. Body effect dominates reception at low/medium frequency; capacitive pickup becomes significant above ~1 MHz. DC shift: ΔI_d = g_mb·ΔV_bias (7.3).
- Source: ch.2 §3 eqs 2.6–2.7, L843–872 (PDF 40); ch.7 §2 eq 7.3, L2889–2893 (PDF 112).
- Philis stage: annotator.
- Automation recipe: `victim_weight(dev) = (g_mb/g_m)·|A_v,node→output|` where g_mb/g_m comes from (2.7) with deck N_A/C_ox (or parse `gmb` in the ngspice `show m` block: add `gmb` to the column list at `oppoint.rs:238`), and the gain from the block role (input pair, bias rail = offset-critical). Devices whose source is not tied to bulk (V_sb ≠ 0) and devices in `Sensitive` blocks rank first.
- Beats hand layout because: every MOS gets a numeric sensitivity; the human intuition "only input pairs matter" misses tail and bias devices whose body is the substrate.
- Philis status: missing (`sensitive` = device in a sensitive block, boolean: `backend/annotator/src/lib.rs:121-128`).

### SUB-10 Switching injection waveform (noise signature) per aggressor
- Kind: formula
- Statement: I_sub(t) = Σ_n s_n(t) * i_sub,n(t), s_n(t) = Σ_k A_k δ(t − t_k), A_k = ±1 (4.7–4.8); per-cell injection fitted as i(t) = A·e^{−t/τ}·cos(ωt − φ) (4.11), Table 4.1 example A = 100 µA, τ = 6.3 ns, φ = 0 (ω printed "1.3 sec⁻¹"). Single-gate pulse 1–100 µA, width 0.5–5× the gate delay. Spectrum concentrates near 1/(average gate delay) with a DC part from impact ionization. White/pink or clock-synchronous single-source models underestimate harmful components.
- Source: ch.2 §1 L722–730 (PDF 35); ch.4 §1 L1373–1383, §4 eqs 4.7–4.9 (PDF 61), §4.1 L1588–1590 (PDF 62), eq 4.11 + Table 4.1 (PDF 63–64).
- Philis stage: annotator.
- Automation recipe: analog P&R has few digital gates; model each clocked/switching device as one pulse source: peak = I_d at the op-point (or 100 µA default when unknown), repetition = clock frequency, τ = deck default. Peak current feeds the DC peak analysis (SUB-02). For mixed-signal tops, accept an external per-block "noise signature" (peak current + spectrum) as IP data (L1408–1413).
- Beats hand layout because: gives each aggressor a magnitude, so distance and ring decisions scale with actual injection instead of "digital = bad".
- Philis status: missing.

### SUB-11 Harmonic content of clocks and HF shunting
- Kind: formula / rule
- Statement: for an ideal rectangular pulse train the n-th harmonic is 20·log(n) dB below the fundamental; a 10 MHz, 3 V rail-to-rail clock has a 1.91 V fundamental and a ~GHz harmonic of 19.1 mV (−40 dB). Junction and interconnect capacitances in digital circuits convey HF energy into the substrate; capacitors electrically isolated from the substrate (decoupling between the digital rails) shunt HF energy to circuit ground.
- Source: ch.7 §4.3 item 2, L3063–3080 (PDF 117).
- Philis stage: annotator, cells.
- Automation recipe: aggressor spectrum lines at k·f_clk with amplitude V/k; a victim whose band (from its block role, e.g. RF/IF) contains a harmonic gets the tighter budget. For clocked blocks, request local decoupling capacitors between the clocked supply and ground built from isolated-plate structures (MOM/MIM), placed adjacent to the aggressor.
- Beats hand layout because: frequency-plan-aware budgets per victim instead of a blanket "keep away".
- Philis status: missing (no decap generation in `frontend/library/src` or `kernel/cells/src`).

### SUB-12 Victim node impedance scales received noise
- Kind: rule
- Statement: received voltage = substrate internal-node voltage × voltage division of R_load vs C_sub; isolation scales linearly with R_load (+20 dB/decade) while the load pole is above the noise frequency (Fig 8.11 at 100 MHz: ≈ −103 dB at 10 Ω to ≈ −67 dB at 1 kΩ, 90 µm spacing, L_gnd 3 nH, C_sub 0.8 pF; read from figure). "Load" = impedance looking into the device from the substrate including feedback. Lateral pnp base in a gain stage must be shielded or connected to a low-impedance node. High-Z nodes are the worst victims.
- Source: ch.8 §4, L3429–3463 (PDF 134–135), note 1 L3947–3950; ch.2 §3 L838–841 (PDF 39).
- Philis stage: annotator.
- Automation recipe: R_load per net from the op-point small-signal output resistance (≈ 1/(g_ds,up + g_ds,down) for a gain node; 1/g_m for a diode node) or class defaults: bias rail (gates-only, `classify.rs:165`) → high Z; supply/ground → low Z. Victim weight ∝ |Z_node| (SUB-15).
- Beats hand layout because: ranks victims by electrical exposure; bias rails and high-Z gain nodes get protection first.
- Philis status: missing.

### SUB-13 Substrate feedback inside one amplifier (Miller via substrate)
- Kind: rule / formula
- Statement: collector(drain)–substrate capacitances of two stages coupled through R_s12 act as a Miller capacitance: C_s1 = C_s2 = 0.2 pF with a second-stage gain of 10 → 1 pF at the first stage's output; measured BW 1.9 → 1.8 GHz. Worst when devices are close, have large substrate C and few substrate contacts (high-res). Max stable gain with substrate feedback: G = g_m/(ω[C_µ + (C_s1‖C_sbp)/{1 + ωR_s1bp(C_s1‖C_sbp)}]) (7.1). Substrate feedback can cause oscillation (Barkhausen) and raises |S12|. Mitigations: guard ring near the output device, grounded backplate, differential implementation with close devices.
- Source: ch.7 §1 L2788–2866, eq 7.1 (PDF 109–111), note 1 L3187–3190.
- Philis stage: annotator, dp, cells.
- Automation recipe: for each amplifier block with stage gain |A| > threshold (block role + op-point g_m·r_o), emit a `SubstrateFeedback{out_dev, in_dev, c_max_ff}` budget: C_Miller = (C_s,out series C_s,in)·|A| via the substrate path; enforce by (a) spacing between the stage's output and input devices, (b) a ring (or taps) around the output device when the deck is `bulk`. Verify computes the effective Miller C from the SUB-03 network.
- Beats hand layout because: the loop is invisible in a schematic simulation without a substrate model (L2799–2803); the tool checks every high-gain stage.
- Philis status: missing.

### SUB-14 Well AC grounding (PMOS well as injector and victim)
- Kind: rule / check
- Statement: PMOS in an n-well are shielded only if the well is "locally AC-grounded"; a well whose potential moves relative to the substrate becomes a large injector through the well–substrate capacitance. Local substrate contacts fix the local potential near sensitive devices, reducing ΔC_j (7.2) and ΔI_d = g_mb ΔV_bias (7.3).
- Source: ch.2 §2 L802–807 (PDF 39); ch.7 §2 L2882–2896 (PDF 111–112).
- Philis stage: cells, verify.
- Automation recipe: check per well: R(well tap → rail) ≤ R_max and max distance from any point of an active device in the well to a tap ≤ d_tap (deck). Per sensitive device: at least one substrate/well tap within d_tap. Sensitive-block wells: tie to a quiet supply net, not the digital supply.
- Beats hand layout because: automatic per-device tap proximity and resistance check.
- Philis status: partial. MOS cells draw a tap strip (`kernel/cells/src/mosfet.rs:488-499`) and every MOS in a non-glue block requests a ring tied to its bulk (`backend/annotator/src/constraints.rs:66-84`); no well-resistance or tap-distance check.

### SUB-15 Local noise generators and constraint generation (bounds with maximum flexibility)
- Kind: algorithm
- Statement: G_n(t, Π) models all substrate noise at node n; Π = [Wᵀ Gᵀ T V0]ᵀ. ΔK_i = (S^{K_i}_Π)ᵀ·ΔΠ (5.2); require ΔK_i ≤ ΔK̄_i (5.3). Choose bounds by: maximize Flexibility(Π_bound) s.t. (S^{K_i}_Π)ᵀ·ΔΠ ≤ ΔK̄_i, Π_min ≤ Π_bound ≤ Π_max. Constraints are process-independent and computed once per circuit; placement enforces them with process-dependent extraction. Bounds are generated only for a subset of critical nodes n_c chosen by cumulative parasitic impact.
- Source: ch.5 §1, eqs 5.1–5.4 and the boxed problem, L1774–1840 (PDF 70–72); ch.6 §1.2 L2319–2326 (PDF 89).
- Philis stage: annotator.
- Automation recipe: per victim device v with performance share ΔK̄ (offset budget in mV from `cfg.offset_sigma_mv`, `backend/annotator/src/lib.rs:130-131`, or a noise/spur budget), sensitivity ∂K/∂V0 ≈ g_mb/g_m × input-referral (SUB-09), set V0_max(v) = ΔK̄_share / |∂K/∂V0|. "Flexibility" = distribute the block's ΔK̄ among its victims proportionally to 1/weight so that the cheapest-to-protect victims get the tightest share (a linear program with one constraint reduces to proportional allocation). Emit `SubstrateBudget{victim, v0_max_uv}` into `Requirements.budget`.
- Beats hand layout because: converts system specs into per-node numeric substrate-voltage limits; hand layout has none.
- Philis status: missing (budgets exist for mismatch/thermal/routing, not substrate voltage).

### SUB-16 Isolation factor α → per-pair minimum distance
- Kind: algorithm
- Statement: after extraction the local substrate potential relates to the source potential through an isolation factor α (V0 = α·V_n); in the SUBRES model (Fig 5.7) α = R3/R2 ratio. Given each source's injected signal, compute the α required to meet the V0 bound and derive the minimum distance from each noise source, usable for automated or hand layout. Also indicates whether guard rings are needed.
- Source: ch.5 §2 L1885–1889 (PDF 73); §3.3 L2063–2073 (PDF 80).
- Philis stage: annotator, dp.
- Automation recipe: α_req(a, v) = V0_max(v) / (I_peak(a)·R_ground(a)). Invert the kind-specific transfer model (SUB-31/32) for d_min(a, v). If d_min exceeds the saturation distance (epi) or a deck area budget, emit a ring requirement instead (SUB-39) and re-evaluate with the ring model (SUB-38). Replace the fixed `min_distance_nm = 4·epi` (`backend/annotator/src/emit.rs:274`) by d_min(a, v), clamped to [0, d_sat].
- Beats hand layout because: per-pair distances sized to actual injection and tolerance; a uniform keep-out wastes area on weak pairs and under-protects strong ones.
- Philis status: partial (uniform distance, `emit.rs:247-287`).

### SUB-17 Worst-case sensitivity for nonlinear parasitics and timing alignment
- Kind: algorithm
- Statement: split parasitics into linear Π′ and nonlinear Π″; worst-case sensitivity S̄ = max over Π″ ∈ I of S^{K_i}_{Π′} (5.5–5.6); total ΔK_i = trace([S̄]·[ΔΠ′]) (5.8). Worst excitation = impulsive generator synchronous with the victim's switching; for a ring VCO the pp jitter peaks when the delay between generators equals one inverter delay (Fig 5.3: ≈280 ps → ≈450 ps at 0.5 ns, V0 = 300 mV, read from figure); sensitivity to V0 ≈ 2/3, constant over 0–300 mV.
- Source: ch.5 §2 eqs 5.5–5.8 (PDF 72–73); §3.1 L1968–1984 (PDF 75–77).
- Philis stage: annotator.
- Automation recipe: when the op-point loop can run perturbation sims (`frontend/library/src/oppoint.rs` drives ngspice), compute ∂K/∂V0 per victim by finite difference (5.10) with a bulk-injected source, at the worst-case phase (synchronous impulse). Otherwise use SUB-09 linear estimates. Store S̄ in the victim record.
- Beats hand layout because: quantifies worst-case alignment that a designer cannot reason about across many nodes.
- Philis status: missing.

### SUB-18 Cluster-contact abstraction for early floorplan estimates
- Kind: heuristic
- Statement: partition the layout into clusters (8 in the PLL case), give each a single contact whose size equals the total injecting area (devices, interconnect), extract resistances from each cluster contact to the victim contact, and simulate with the fitted source waveform. Rough max/min substrate conductances come from two contacts at chip edges or in close proximity.
- Source: ch.5 §3.3 L2079–2085 (PDF 81); ch.6 note 2 L2742–2744 (PDF 106).
- Philis stage: gp, dp.
- Automation recipe: in gp and early dp temperatures, one contact per placed cell (bbox, area = sum of device diffusion areas) — matches `Layout::edge_gap`/`bbox` granularity (`kernel/core/src/layout.rs:170`). Refine to per-device junction contacts only for verify.
- Beats hand layout because: cheap enough to evaluate every move.
- Philis status: missing.

### SUB-19 Wire R/C constraint enforcement loop
- Kind: algorithm
- Statement: R = ρL/W, C = C0·W·L. Set W = W_min, L = L_min; loop: if R < R_max exit, else W += ΔW (minimum grid increment λ); while C < C_max. If C > C_max or R > R_max → infeasible; else constraint enforced.
- Source: ch.5 §3.3 algorithm box, L2048–2062 (PDF 80).
- Philis stage: gr, dr.
- Automation recipe: per net with both an R bound and a C bound (from `extract::routing`), size wire width in `dr` with this loop before committing a route; report infeasible pairs to the flow as budget conflicts.
- Beats hand layout because: minimal width meeting both bounds on every constrained net.
- Philis status: partial. `dr` has a single `wire_width` (`backend/dr/src/lib.rs:43,152`); no per-net R/C-driven width loop.

### SUB-20 Constraint-aware sizing uses larger devices to cut parasitic sensitivity
- Kind: heuristic
- Statement: power-minimizing sizing shrinks devices and raises sensitivity to layout parasitics; adding the parasitic-bound constraint to sizing produced W_p 36 vs 25 µm and L_n 4 vs 4.6 µm (Table 5.4) and met constraints (C_out± 12.34 fF bound vs 11.60 fF extracted; V0|VCO 110 mV vs 108 mV).
- Source: ch.5 §3.2 L2001–2027, Tables 5.3–5.4 (PDF 78, 82).
- Philis stage: flow (outside P&R; feedback to the sizer).
- Automation recipe: export per-net parasitic bounds and per-victim V0 limits in the solution report so an external sizer can close the loop. No P&R action.
- Beats hand layout because: closes the sizing–layout loop with numbers.
- Philis status: missing (out of scope; noted).

### SUB-21 Incremental substrate update: Sherman–Morrison
- Kind: algorithm
- Statement: moving one contact changes row r and column r of P; c′ = P′⁻¹ = c + δc, δc = −c_{·r}(c δP_{r·})/(1 + δP_{r·} c_{·r}) (6.3), O(N²) per moved contact partition; apply twice (row and column) for a symmetric update.
- Source: ch.6 §1.1 eq 6.3, L2205–2219 (PDF 86).
- Philis stage: dp, verify.
- Automation recipe: keep c (N ≤ a few hundred contacts in analog blocks) per epoch; on a dp move of cell k, update the P rows/columns of k's contacts and apply rank-1 updates; revert on reject by storing δc or recomputing. Use only when N is moderate; otherwise SUB-22.
- Beats hand layout because: exact coupling updates at O(N²) per move make substrate cost affordable in annealing.
- Philis status: missing.

### SUB-22 Gradient-based move estimate of the conductance matrix
- Kind: algorithm
- Statement: ∇_vY = [A B]ᵀ with A = ∂Y/∂v_x, B = ∂Y/∂v_y (6.5), from finite differences on the grid (6.6); [Y]_n ≈ [Y]_0 + [∇_vYᵀ]_0·Δv (6.7). The Green's function has no high-frequency content, so the linearization is accurate: 1 % within 5 grid steps, 10 % for moves up to 1/10 of the chip size.
- Source: ch.6 §1.1 eqs 6.4–6.7, L2229–2281 (PDF 87–88).
- Philis stage: dp.
- Automation recipe: at each dp temperature start (or epoch start), compute per-contact gradients of its row of Y by central differences of the transfer model (SUB-31/32), store as `grad: Vec<[f32;2]>` per edge. Per move: Y_est = Y0 + ∇·Δv for edges touching moved contacts; solve victim voltages only for critical victims (SUB-23). Refresh exactly when |Δv| > 5 grid steps or a victim nears its bound.
- Beats hand layout because: global substrate effects are tracked per move at near-zero cost.
- Philis status: missing. Analogous per-epoch refresh exists for temperature (`kernel/core/src/thermal.rs:1-4`, `Layout::refresh_temps` at `thermal.rs:53`).

### SUB-23 SA heuristic: exact per temperature, estimated per move, exact only on violation
- Kind: algorithm
- Statement: foreach temperature T_k: evaluate network exactly (Sherman–Morrison); repeat: select move m_k; estimate network change (gradient); foreach critical node j: G_j = estimate cumulative noise; if Π_j ≥ Π_j_bound → evaluate network exactly; evaluate cost; accept/reject; until equilibrium (Fig 6.6). If a violation occurs, the precise violation drives the annealing cost like [70]; otherwise the substrate contribution is dropped from the cost. At high temperature contact reshuffling is large (Fig 6.5 resistances swing ~7–14 kΩ), at low temperature small (read from figure).
- Source: ch.6 §1.2, Fig 6.4–6.6, L2336–2387 (PDF 89–92).
- Philis stage: dp.
- Automation recipe: in `backend/dp` add a `SubstrateNoise{victim, v0_max_uv}` rule type implementing `Rule` like `Isolation` (`kernel/analog/src/placement/isolation.rs:20-37`), whose `cost` reads a per-epoch cached network + gradient estimate; `satisfied`/`residual` use the exact solve. Cost = Σ_victims max(0, V_v − V0_max)² × weight; zero when all under bound (matches "substrate term dropped"). Epoch Θ residual includes it; best-epoch selection is unchanged.
- Beats hand layout because: Table 6.2 — parasitic-only placement predicted jitter 0.1 vs spec ≤ 0.007; substrate+parasitic-aware placement predicted 0.005, at 1.28× the area of the parasitic-only result (7322×7716 vs 6765×6528 λ²; derived) and 885 s vs 407 s CPU. The manual layout (5637×6481 λ²) had no jitter estimate.
- Philis status: missing (only the distance proxy `Isolation`).

### SUB-24 Placement problem form with absolute/relative constraints and orientation set
- Kind: data-model
- Statement: objects with centre and perimeter vertices; absolute constraints C^A bound a centre/perimeter to reference points; relative constraints C^R relate pairs/groups; Manhattan orientations NO_ROTATE, ROTATE_90/180/270, MIRROR_X, MIRROR_Y, MIRROR_YX; minimize f(S) s.t. C^A, C^R (NP-hard); iterative improvement best exploits progressive substrate extraction. Polygon centre = centre of mass of the approximating rectangle.
- Source: ch.6 §1, L2152–2203 (PDF 85–86), note 1 L2732–2735.
- Philis stage: dp.
- Automation recipe: already the Philis model; add orientation as a substrate lever: device orientation changes junction position relative to injectors (SUB-43). No new structure.
- Beats hand layout because: n/a (baseline).
- Philis status: implemented (placement rules and anneal in `backend/dp`; rule trait `kernel/analog/src/rule.rs`).

### SUB-25 Extended search space: alternative implementations incl. a guard-ring variant
- Kind: algorithm
- Statement: substrate remedies in placement are (1) distance via conventional moves, (2) alternative module implementations including one with a guard ring, chosen by the annealer from an extended search space. SA convergence is preserved when cells carry several guard-ring implementations or swap implementations [82].
- Source: ch.6 §1.2 L2287–2296 (PDF 88); App. C.2 L4207–4210 (PDF 169).
- Philis stage: dp, cells.
- Automation recipe: add a discrete per-victim/aggressor `ring` branch bit like `DtiBand.branch` (`kernel/analog/src/placement/dti.rs:9-30`, flipped by dp as a discrete move); the committed side changes the cell footprint (ring halo from `post_cell::ring_halo`, `kernel/cells/src/post_cell.rs:291`) and the transfer model (SUB-38). Seed from SUB-16 (ring when d_min unaffordable).
- Beats hand layout because: the tool trades area of a ring against distance per pair with the actual isolation numbers.
- Philis status: missing (rings are unconditional per MOS; `constraints.rs:66-84`).

### SUB-26 Template/cached sub-network reuse with sensitivity pruning
- Kind: algorithm
- Statement: reuse a pre-extracted template c; update only contacts displaced vs the template; Y_actual = Xᵀ c_actual X (6.9); cost 0 for an identical template, O(N³) worst case. Prune conductances whose cumulative effect on performance is below a fraction of the allowed degradation; remove nodes left with ≤ 1 conductance (Fig 6.10). Weighted extraction inaccuracy ε_K bounds the set D of contacts needing full extraction (6.15–6.17); a single pole contact per partition with ≤ 50 % error was acceptable.
- Source: ch.6 §2, L2392–2572, eqs 6.9–6.19 (PDF 92–97).
- Philis stage: cells, verify.
- Automation recipe: cache each generated macro's internal contact sub-network (keyed by macro hash, like repeated unit cells in cap arrays/mirrors); at chip level only inter-macro edges are computed. Prune edges with |∂K/∂Y_ij|·Y_ij,max < 0.1·ΔK̄ (fraction is a tunable; "a fraction" in the source).
- Beats hand layout because: makes full-chip substrate verification tractable for repeated analog arrays.
- Philis status: missing.

### SUB-27 Trend analysis across technologies/PDKs
- Kind: metric
- Statement: ΔK = Σ_ℓ (∂K/∂T_ℓ)ΔT_ℓ with ∂K/∂T_ℓ = Σ_ij (∂K/∂Y_ij)(∂Y_ij/∂T_ℓ) (4.1); ∂Y via ∂P/∂T and (4.4–4.5). Used for yield (process imperfection), technology selection (Table 6.7: probability of meeting all six resistance bounds 0.923 for T1 vs 0.927 for T2), and migration without re-extraction. Sensitivity-based extraction: 10×10 grid 9.8 s vs 73.8 s, 3 % max error; 2,500 contacts 57 min vs full extraction not completed.
- Source: ch.4 §3 eqs 4.1–4.6 (PDF 58–59); ch.6 §3.2–3.3, Tables 6.4–6.7 (PDF 102–106).
- Philis stage: verify, flow.
- Automation recipe: low priority. Philis runs the same netlist on four PDKs (`frontend/library/tests/ota_cross_pdk.rs`); report per-PDK isolation margins from SUB-04 side by side instead of analytic ∂/∂T.
- Beats hand layout because: cross-PDK substrate risk visible before layout.
- Philis status: missing.

### SUB-28 Ring efficacy depends on substrate kind
- Kind: rule
- Statement: low-res (epi on p+): a 10 µm ring between two 2500 µm² contacts improves isolation only 7–10 dB and is a weak function of L_gr (rings sink only the surface current); Su: p+ ring near the sensor ~20 % improvement, a distant ring none, minority (n-type) rings no effect. High-res: rings are very effective and lowering L_gr gives large improvement (Fig 8.13b at 1 GHz ≈ −45 dB with L_gr 5 nH vs ≈ −65 dB with 0 nH; read from figure).
- Source: ch.8 §5.1–5.2 L3494–3539 (PDF 136–138); App. D.3 L4465–4485 (PDF 177–178).
- Philis stage: annotator, cells.
- Automation recipe: ring policy by `substrate.kind`: `bulk` → emit isolation rings for victims/aggressors per SUB-16/SUB-39; `epi_on_pplus` → emit only latch-up rings and taps, never isolation rings, and put the effort in the backplate report (SUB-05); never emit minority-carrier (n-well) rings for isolation in epi.
- Beats hand layout because: avoids area spent on ineffective rings and targets rings where they work.
- Philis status: missing (ring requests are substrate-agnostic, `backend/annotator/src/constraints.rs:66-84`).

### SUB-29 Minimum-width guard rings with a low-impedance ground
- Kind: rule
- Statement: at high frequency (~1 GHz) thin rings beat wide rings; at 100 MHz there is a shallow optimum (15 µm at 2 nH in Fig 8.21a); width 2→34 µm improves isolation only when L_gr = 0 and worsens it otherwise. "The best rule for sizing guard rings is to use minimum width rings and ensure a very good ground connection to the ring." Differential circuits: thin rings too (common-mode isolation worsens with width when L_gr > 0).
- Source: ch.8 §6.1 L3583–3587 (PDF 140), §6.2 L3695–3704 (PDF 146–147), §7 L3766–3771 (PDF 149).
- Philis stage: cells.
- Automation recipe: ring band = deck minimum width, cut rows added only to meet the ring resistance budget (already so); add a "ground quality" budget: R + jωL of the ring's connection to its pad/rail (SUB-30) instead of widening.
- Beats hand layout because: hand layouts often widen rings believing wider is better.
- Philis status: implemented for width: `band()` uses the minimum ring width and adds cut rows only while `cut_ohm / cuts` exceeds `max_ring_resistance_mohm` (`kernel/cells/src/post_cell.rs:346-367`). Ring-to-pad impedance is not modeled (`post_cell.rs:351-353` ponytail note).

### SUB-30 Ring ground connection: dedicated, quiet, correctly referenced
- Kind: rule / check
- Statement: if the victim signal is referenced to an external ground, connect the ring through an independent bond-wire (Fig 8.16a); a ring on the internal ground with large L_gr is worse than no ring; an internally grounded ring is excellent only when the signal is measured against that internal ground (then L_gr-independent). Rings tied to the shared substrate-bias/ESD substrate node *increase* received noise (Su; Gharpurey). A tap/ring whose net bounces injects over its whole extent. Keep ring inductance small (X_gr small at the frequency of interest).
- Source: ch.8 §5 L3482–3488 (PDF 135), §6.1 L3569–3598 (PDF 139–140); ch.2 §2 L826–832 (PDF 39); App. D.3 L4473–4482, D.4 L4614–4629 (PDF 177–182).
- Philis stage: annotator, gr, verify.
- Automation recipe: GuardRingRequirement gains `role: RingRole{Latchup, Victim, Aggressor}` and the ring's `connection_net` is chosen by role: Victim → the victim block's own reference ground net if the netlist distinguishes it (e.g. AGND/VSSA from `netrole`), else a dedicated `*_ring` net exported as a top-level pin; Aggressor → its own dedicated net/pin; never a net shared with ESD or digital supply. gr: route ring nets as star connections to their pin (no shared segment with signal ground). verify: flag rings whose connection net is shared with a Clock-class device's source or with >N unrelated devices.
- Beats hand layout because: the connection error is invisible in DRC/LVS and routinely made by hand; the tool checks every ring.
- Philis status: missing. Every ring connects to its device's bulk net (`backend/annotator/src/constraints.rs:83`); `docs/LAYOUT-FUNDAMENTALS.md:135-136` already lists this gap (#58, #59).

### SUB-31 Distance scaling in epi-on-p+ substrates: saturation at 2.5–5× epi thickness
- Kind: formula / rule
- Statement: with finite backplate inductance (1–5 nH at 1 GHz) isolation becomes independent of distance beyond a critical distance of 2.5–5× the epi thickness (10 µm epi); Su et al. measured 4×; only L_gnd = 0 keeps improving. High-frequency isolation worsens ~linearly with contact area (R to backplate ∝ 1/area; C_sub reactance ∝ 1/area). Single-contact R in low-res ≈ inverse with area (Fig 8.3: ~3–5 kΩ at 10 µm to ~2 Ω at 1000 µm; read from figure). Differential receivers in low-res: excellent common-mode conversion because R to backplate is position-independent (SUB-43).
- Source: ch.8 §1 L3226–3228 (PDF 123), §2.1 L3289–3313 (PDF 127), §3.1 L3392–3399 (PDF 132); App. D.3 L4465–4468.
- Philis stage: annotator, dp.
- Automation recipe: transfer model for `epi_on_pplus`: R_i0(A) = ρ_epi·t_epi/A_eff (inverse area; A_eff = A + fringe from deck-fitted constant), R12(d) growing until d_sat = k·t_epi with k from deck (default 4), constant beyond. Isolation rule cost goes to zero at d ≥ d_sat. Victim contact area is a lever: smaller victim junction area → better HF isolation.
- Beats hand layout because: stops wasting area past d_sat and redirects effort (area reduction of victim junctions, backplate ground).
- Philis status: partial. `Isolation` with `min_distance_nm = 4·epi` implements the saturation distance (`backend/annotator/src/emit.rs:247-249,274`; `kernel/analog/src/placement/isolation.rs:7-11`); no area term, no L_gnd; applied to every deck including bulk ones.

### SUB-32 Distance scaling in high-resistivity bulk: keeps improving, surface-dominated
- Kind: formula / rule
- Statement: in high-res substrates a large fraction of current flows at the surface; isolation keeps improving with distance even with finite backplate impedance, and is weakly dependent on backplate inductance (Fig 8.8(a), C_sub 0.3 pF, 100 MHz grounded: ≈ −67 dB at 0 µm to ≈ −80 dB at 400 µm; read from figure). Single-contact R depends only weakly (logarithmically) on contact size (Fig 8.2: ~600 Ω at 10 µm to ~90 Ω near 700 µm; read from figure). R12 rises monotonically with distance; R10 is non-monotonic and rises near the die edge.
- Source: ch.8 §1 L3209–3248 (PDF 122–125); §2.2 L3320–3351 (PDF 127–130).
- Philis stage: annotator, dp.
- Automation recipe: transfer model for `bulk`: R12(d) monotonic increasing (fit a + b·ln(d) or a + b·d on deck calibration points; no fit constants are given in the source), R_i0 weakly dependent on area. Isolation cost keeps a slope at all distances (no plateau): cost ∝ max(0, V_v − V0_max) with V_v computed, not a fixed keep-out. Add a die-edge penalty term for victims (higher R to backplate near the edge means less shunting).
- Beats hand layout because: continuous trade-off between distance and area instead of a single keep-out number.
- Philis status: partial/wrong-model. sky130 is bulk (`backend/annotator/src/emit.rs:253-255`), yet the plateau rule is applied with a nominal 2.5 µm epi (`emit.rs:256,274`).

### SUB-33 Guard-ring equivalent circuit and its closed-form received voltage
- Kind: formula
- Statement: ring = current sink between contacts: model (Fig 8.12d/8.20) with R12, R1gr, R2gr, ring node GR to ground through L_gr (+ ring bond-wire noise V_inj1), R10, R20 to backplate through L_gnd. With small device-to-substrate reactances and small R_l: V2 = R_l·{(1/R12 + jωL/(R1grR2gr + jωL(R1gr+R2gr)))·V_inj + (R1gr/(R1grR2gr + jωL(R1gr+R2gr)))·V_inj1} (8.2). Ring advantage lost above ω ≈ R1grR2gr/(L(R1gr+R2gr)); LF isolation set by R12; HF by R1gr + R2gr; ring-noise term ∝ R_l/R2gr.
- Source: ch.8 §5 Fig 8.12 (PDF 136), §6.2 eq 8.2 and L3636–3650 (PDF 143–144).
- Philis stage: verify, dp.
- Automation recipe: in the SUB-04 pair evaluator, when a ring encloses the victim or the aggressor, add node GR with R1gr, R2gr from the transfer model (ring treated as a contact of its drawn area, shortest distance to each contact) and L_gr from the deck (`l_ring_nh`) or computed from the ring's routed connection. Report the ring's "useful up to" frequency R1grR2gr/(2πL(R1gr+R2gr)); flag rings whose useful frequency is below the aggressor clock.
- Beats hand layout because: tells when a ring helps, is neutral, or hurts, per pair and per frequency.
- Philis status: missing.

### SUB-34 Ring placement: around the injector vs around the receiver
- Kind: rule
- Statement: isolation from the substrate injector is nearly the same whether the ring surrounds the injector or the receiver (Fig 8.23a: ≈ −98 dB at 100 MHz, ≈ −47 dB at 1 GHz vs no ring ≈ −60/−32 dB, b = 250, w = 4, a = 50, d = 10 µm; read from figure); isolation from ring bond-wire noise V_inj1 is ~20 dB better with the ring around the injector. Example: adjacent bond-wire carrying 1 mA at 1 GHz, M = 1 nH → V_inj1 = 6.3 mV → 1.41 mV at the receiver (receiver ring) vs 0.14 mV (injector ring); 1 V injector → 5.623 mV; no ring 22 mV; V_inj1 dominates above 3.5 mA (receiver ring) or 35 mA (injector ring). Ideal: ring around the noisy circuit, its bond-wire placed among the analog bond-wires.
- Source: ch.8 §6.2 L3651–3671, L3709–3732 (PDF 144–148), note 4 L3955–3956.
- Philis stage: annotator, cells.
- Automation recipe: default ring role = Aggressor (ring the injector cluster) when the aggressor is a compact set of cells; ring the victim when there are many dispersed aggressors or the aggressor cluster is large. Both when SUB-35 applies.
- Beats hand layout because: the choice is made per pair from numbers, and ring-pin noise is accounted.
- Philis status: missing (rings around every MOS, role-blind; `constraints.rs:66-84`).

### SUB-35 Dual guard rings, separately grounded
- Kind: rule
- Statement: rings around both injector and receiver, each on its own bond-wire (5 nH), 50 µm contacts, 300 µm apart, 10 µm rings, C_sub 0.8 pF: no ring −62 dB / −34 dB; receiver ring only −99 / −45 dB; two rings −130 / −57 dB (100 MHz / 1 GHz). Tied together they act as one large ring. Use two rings when two package pins are available.
- Source: ch.8 §8, Fig 8.26, Table 8.1, L3787–3811 (PDF 150–151).
- Philis stage: annotator, cells, gr.
- Automation recipe: when SUB-16's required isolation exceeds the single-ring estimate and the flow config allows another pin, emit Victim and Aggressor rings on distinct nets (`*_ring_a`, `*_ring_v`); gr must not merge them (the ring merge in `post_cell` already merges only same `connection_net` and type, `kernel/cells/src/post_cell.rs:79-85`).
- Beats hand layout because: pin budget vs isolation trade-off decided per pair.
- Philis status: missing.

### SUB-36 Ring gap vs ring width: area-efficient ring
- Kind: heuristic
- Statement: to raise R12 while keeping ring width w small, make the ring-to-receiver distance d small; small d lowers R2gr and increases coupling of ring noise V_inj1; if V_inj1 is significant, both w and d must be large.
- Source: ch.8 §6.2 L3651–3654 (PDF 144).
- Philis stage: cells.
- Automation recipe: ring gap = deck clearance (current `ring_gap`, `kernel/cells/src/post_cell.rs:299-302`) when the ring net is dedicated and quiet; add `extra_gap_nm` proportional to the expected ring-net noise when the ring shares a pin with other circuitry (from SUB-30 classification).
- Beats hand layout because: gap chosen by ring-net quality rather than habit.
- Philis status: partial (minimum clearance only).

### SUB-37 Low-impedance drain next to strong injectors (vertical PNP, noisy wells)
- Kind: rule
- Statement: a vertical pnp injects its collector current into the substrate "unless low impedance draining is provided in immediate proximity of the device". Local substrate contacts set the substrate potential near a device.
- Source: ch.2 §2 L759–763 (PDF 38); ch.7 §2 L2882–2888 (PDF 111–112).
- Philis stage: cells, annotator.
- Automation recipe: for vertical PNP devices (and any device with I_sub from SUB-08 above a threshold) require a substrate tap ring or tap strip adjacent (within deck minimum spacing) tied to a dedicated ground net (SUB-30), sized for R ≤ R_max (existing `band()`).
- Beats hand layout because: applied to every such device automatically.
- Philis status: partial. BJT cells draw rings/wells for isolation (`kernel/cells/src/bjt.rs:5,31-34,129-196`); no injector-driven drain rule.

### SUB-38 Buried shields: choice and placement
- Kind: rule
- Statement: (a) p+-in-p buried Faraday shield: near-ideal isolation with an ideal ground, very sensitive to ground-path impedance, degrades at HF with bond-wire L; nonzero shield resistivity lets coupling through. (b) SOI/dielectric: excellent LF, degrades with frequency (series C); thicker oxide helps. (c) Buried n+ well tied to the highest potential: dielectric (depletion) plus Faraday; less sensitive to both non-idealities; hard under PMOS n-wells or npn buried collectors. Joardar: buried n+ better than SOI. Place the shield around the circuit with the smaller area (usually the analog section) so its C is small relative to the ground-path impedance.
- Source: ch.8 §9 L3813–3944, Fig 8.27–8.30 (PDF 151–156); App. D.2 L4414–4417 (PDF 176).
- Philis stage: cells, annotator, deck.
- Automation recipe: when the deck has `dnwell` (sky130 lists it, `backend/verify/src/pdk.rs:1212`) and a sensitive NMOS group needs more isolation than distance+ring give (SUB-16), emit a triple-well requirement: DNW under the victim NMOS group with the enclosing n-well ring tied to a quiet supply, and an isolated p-well tap to the victim's ground. Reuse the DNW drawing in `kernel/cells/src/bjt.rs:129-196`. Apply to the smaller of victim vs aggressor groups.
- Beats hand layout because: automatic selection and correct placement (smaller circuit) of the strongest structure.
- Philis status: missing for MOS (DNW only for NPN isolation, `kernel/cells/src/bjt.rs:31-34`).

### SUB-39 Guard rings as nearby ground planes improve differential isolation
- Kind: rule
- Statement: in high-res substrates a ring around a differential receiver acts as an equipotential plane that makes coupled noise common mode; differential-mode isolation improves slightly with ring width and is independent of L_gr (Fig 8.25: DM ≈ −92…−100 dB at 1 GHz, ≈ −130…−140 dB at 100 MHz; read from figure); common-mode isolation worsens with width at L_gr > 0. Mismatch degrades it. Differential injectors and receivers behave likewise.
- Source: ch.8 §7 L3734–3785, Fig 8.24–8.25 (PDF 147–150).
- Philis stage: annotator, cells.
- Automation recipe: one shared thin ring around both halves of a matched differential pair (not one ring per device); the existing merge of same-net same-type rings (`kernel/cells/src/post_cell.rs:1-5,79-85`) is the mechanism; require that the pair's ring be a single loop (no separate halves).
- Beats hand layout because: guaranteed common ring geometry and symmetric distance to it.
- Philis status: partial (rings merge when same net/type and close, `post_cell.rs:15-17,79-85`; not required for pairs).

### SUB-40 Bond-pad shield for low-noise inputs
- Kind: rule
- Statement: substrate resistances under an LNA input pad add thermal noise amplified by the input device; shield the input pad with a low-impedance shield (buried n-well to supply, or a lower metal layer); output noise from the pad network peaks when its reactive and resistive parts are similar; bond-pad C 0.5–1 pF, R_sbp several Ω to several kΩ.
- Source: ch.7 §1 L2792–2796 (PDF 109); §3 L2898–2918, Fig 7.3 (PDF 112–113).
- Philis stage: cells (pad frame), deck.
- Automation recipe: out of current scope (Philis places no pads). When a pad generator exists: pads on nets classified as low-noise inputs get a grounded lower-metal or n-well shield.
- Beats hand layout because: applied uniformly to every sensitive pad.
- Philis status: missing / out of scope.

### SUB-41 Tap density near sensitive devices (local potential pinning)
- Kind: rule
- Statement: local substrate contacts set the substrate potential "quite effectively in its immediate vicinity, especially in high-resistivity substrates", reducing junction-capacitance modulation (7.2) and bias shifts (7.3); "surrounding sensitive nodes of an amplifier by substrate contacts reduces undesirable changes in bandwidth". Taps are more efficient in high-res substrates (surface current).
- Source: ch.7 §2 L2882–2896 (PDF 111–112); ch.8 §2.2 L3342–3344 (PDF 130).
- Philis stage: cells, verify.
- Automation recipe: check `max_dist_to_tap(dev) ≤ d_tap_sensitive` for sensitive devices (tighter than the latch-up rule) with `d_tap_sensitive` a deck/config value (not given in the source); the MOS cell tap strip normally satisfies it; flag merged/stacked cells whose inner devices exceed it.
- Beats hand layout because: exhaustive check.
- Philis status: partial (tap strip per MOS cell, `kernel/cells/src/mosfet.rs:488-499`; per-MOS rings, `constraints.rs:66-84`; no distance check).

### SUB-42 Contact size is a lever that depends on substrate kind
- Kind: formula / heuristic
- Statement: high-res: tap/contact R to ground depends weakly (logarithmically) on size — enlarging one tap buys little; low-res: R ≈ inverse with area — enlarging taps helps, but HF isolation of *victim* junctions worsens ~linearly with their area.
- Source: ch.8 §1 L3209–3228, Fig 8.2–8.3 (PDF 122–123); §2.1 L3307–3313 (PDF 127).
- Philis stage: cells.
- Automation recipe: tap sizing: in `bulk` use more, distributed taps near victims rather than bigger taps; in `epi_on_pplus` large-area backplate-tied taps near the aggressor. Victim junction area enters SUB-04 via C_sub and R_i0.
- Beats hand layout because: the right lever per PDK automatically.
- Philis status: missing.

### SUB-43 Orient differential receivers so the dominant injector lies on the symmetry axis
- Kind: rule / metric
- Statement: with two identical receiver contacts, an injector moved along the pair's symmetry axis (perpendicular bisector, y in Fig 8.9c) keeps the circuit perfectly balanced — infinite differential isolation; an injector on the line through both contacts (x) is the worst case. In low-res substrates differential isolation falls rapidly with distance and is backplate-insensitive; in high-res the near contact shields the far one, so differential gain is smaller.
- Source: ch.8 §3.1 L3382–3399, Fig 8.9 (PDF 131–132); §3.2 L3401–3409 (PDF 132).
- Philis stage: dp, annotator.
- Automation recipe: for each matched pair (a, b) with symmetry axis (emitted per stage, `backend/annotator/src/emit.rs:139,158`) and each aggressor cluster k, add cost term `|Z_ka − Z_kb| · I_k` (from the SUB-03 network), or the geometric proxy `|d(k,a) − d(k,b)|/(d(k,a)+d(k,b))`. dp then prefers stage orientations/positions that put aggressors on the pair's mirror axis.
- Beats hand layout because: every pair against every aggressor, continuously, during annealing.
- Philis status: missing (`Symmetry` enforces mirror positions only, `kernel/analog/src/placement/symmetry.rs:7-14`).

### SUB-44 Substrate-parasitic matching inside differential pairs
- Kind: check / rule
- Statement: a 5 % mismatch of the receiver pair's substrate capacitances collapses differential isolation, most for distant injectors (Fig 8.10b, 1 GHz: ≈ −80 dB with mismatch vs ≈ −120 dB without; 100 MHz ≈ −123 vs ≈ −150 dB; read from figure). Mismatches turn differential circuits single-ended. Brittle single-ended device models are wrong in close differential placement (output sees smaller substrate impedance).
- Source: ch.8 §3.2 L3410–3427 (PDF 132–134); §7 L3775; ch.7 §4.1 L2982–2986 (PDF 115).
- Philis stage: dp, verify, cells.
- Automation recipe: for every matched pair require equal junction area and perimeter to substrate (same cell variant — already required), equal distance to the nearest tap/ring edge (|Δd_tap| ≤ grid), equal well-edge distance, and equal exposure to each aggressor (SUB-43). verify: compute C_sub and R_to_ring per half from drawn geometry and report the relative mismatch; budget ≤ a fraction of the 5 % that the source shows is damaging (exact limit not given).
- Beats hand layout because: exact geometric equality checks on every pair; hand layouts commonly ring one half closer than the other.
- Philis status: partial. `MatchingPair` covers random/gradient VT mismatch (`kernel/analog/src/placement/matching_pair.rs:28-37`) and same-variant/dummy requirements exist (`backend/annotator/src/constraints.rs:60-61`); no substrate-exposure equality.

### SUB-45 Mirror-symmetric, fully differential, abutted module floorplan
- Kind: heuristic
- Statement: VCOGEN floorplan: abut all delay cells (minimum RC), fold the ring multiple times (contain technological mismatch), keep cell aspect ratio low (systematic mismatch reduced to one dimension), fully differential (capacitively coupled noise becomes common mode), mirror-symmetric delay cell (balances rise/fall and cancels thermal and substrate noise effects between branches). Cell symmetry makes both differential outputs share one constraint (Table 5.3).
- Source: ch.5 §3.3 L2029–2047 (PDF 79), L2104–2107 (PDF 82).
- Philis stage: annotator, dp.
- Automation recipe: already the Philis direction (symmetry per stage, matching pairs); add: for ring-oscillator/delay-line patterns (repeated identical stages), emit abutment + folded row placement template and shared symmetry axis for all stages.
- Beats hand layout because: n/a (codifies expert practice).
- Philis status: partial (symmetry/matching rules exist; no ring-oscillator folding template).

### SUB-46 Worst excitation for differential isolation is along the pair axis; balance improves with distance only in low-res
- Kind: metric
- Statement: report differential and common-mode isolation separately; differential isolation (low-res) ≈ −35 dB near to ≈ −120 dB at 400 µm at 1 GHz for identical receivers; high-res improvement is smaller due to the surface-current shielding of one contact by the other.
- Source: ch.8 §3 L3353–3409, Fig 8.9–8.10 (PDF 130–133).
- Philis stage: verify.
- Automation recipe: verify's substrate report lists, per matched pair and aggressor, DM and CM isolation from the SUB-03 network (two receivers), not only single-ended values.
- Beats hand layout because: DM/CM split per pair is not something hand review produces.
- Philis status: missing.

### SUB-47 Capacitor plate orientation and adjacent AC-coupling capacitors
- Kind: rule / check
- Statement: when one side of a capacitor is a reference node, connect the top plate to the active node and the bottom plate to the reference (LC tanks, bypass) — avoids substrate current and keeps Q. For single-ended AC coupling the bottom-plate substrate R can help. Two adjacent differential AC-coupling capacitors have their bottom plates coupled by R12: if the bottom-plate C is a ratio r of the main C, V_out = V_in/(1+r); r = 0.2 → 1.6 dB loss not predicted by single-ended models.
- Source: ch.7 §4.1 L2924–2981, Fig 7.4 (PDF 112–115).
- Philis stage: annotator, cells, verify.
- Automation recipe: annotator assigns plate orientation: bottom plate (`N` pin, `kernel/cells/src/capacitor.rs:37`) to the lower-impedance net (Supply/Ground/bias < signal), swapping `P`/`N` in the instance when the netlist allows. verify: for capacitors on complementary nets of a differential pair placed within d_couple, include an R12 between their bottom-plate substrate nodes in the extracted netlist and report the attenuation 1/(1+r).
- Beats hand layout because: every capacitor is oriented correctly; the differential bottom-plate coupling is caught in verification.
- Philis status: missing (`docs/LAYOUT-FUNDAMENTALS.md:87` row 30 lists it as absent).

### SUB-48 Substrate losses at matched RF nodes
- Kind: formula
- Statement: matching gains 1/(1−|S11|²) and 1/(1−|S22|²); a series R–C substrate loss gives |S11| = √(((R−Z0)² + 1/(ωC)²)/((R+Z0)² + 1/(ωC)²)) (7.4), approaching 1 only for R → 0 or R → ∞; cascodes with very high output impedance lose the most gain.
- Source: ch.7 §4.2 L2993–3024, eq 7.4 (PDF 115–116).
- Philis stage: verify.
- Automation recipe: low priority; for nets classified RF input/output, report the substrate R–C of the pad/device junction from the network and the implied |S11|.
- Beats hand layout because: quantitative loss check.
- Philis status: missing / out of scope.

### SUB-49 Spur classes a mixed-signal placement must budget
- Kind: heuristic
- Statement: PA→LNA (power ratio > 100 dB; substrate path is not frequency selective), in-band digital harmonics, low-frequency noise upconverted by second-order distortion with a jammer (differential circuits help), beat products in any nonlinearity (frequency plan must include substrate spur levels), DC offsets from clock/LO leakage into samplers/mixers, oscillator frequency pulling by PA, oscillatory loops across high-gain stages (80–100 dB at one frequency in direct conversion). Circuit-side remedy example: stagger ADC output-buffer enable vs sampling.
- Source: ch.7 §4.3 L3026–3159 (PDF 116–119).
- Philis stage: annotator.
- Automation recipe: net-role tags `LO`, `sampling_clock`, `sampler_input`, `mixer_input` (from `netrole`) raise the victim weight of sampler/mixer inputs against the corresponding clock/LO aggressors (DC-offset class); high-gain chains get the SUB-13 feedback check across their full gain.
- Beats hand layout because: systematic pairing of known aggressor–victim classes.
- Philis status: missing (NetClass has Clock and Sensitive only, `kernel/analog/src/metadata.rs:18-26`).

### SUB-50 Floorplan: isolated areas and spectrum-driven minimum distances
- Kind: heuristic
- Statement: allocate well-isolated areas to noisy circuits; compute minimum distances from the noise spectral energy of the aggressor and the maximum spurious energy tolerated by sensitive circuits; where space is lacking, design guard rings for the problematic part of the spectrum. Early practice separated sections with large contact-dominated areas.
- Source: ch.4 §2 L1396–1405 (PDF 57); ch.1 §3 L567–569 (PDF 28).
- Philis stage: gp.
- Automation recipe: in gp, seed aggressor clusters and victim clusters on opposite sides of the region (or opposite ends of the die for top-level) using SUB-16 distances as soft separation; this is SUB-16 applied at gp granularity.
- Beats hand layout because: separation sized to numbers, not rules of thumb.
- Philis status: partial (`Isolation` is a placement cost that gp/dp see; `emit.rs:283-286`).

### SUB-51 Delay effect and substrate-as-aggressor crosstalk under wires
- Kind: formula
- Statement: t50% = R_EFF·C_G (2.8); R_tr = (L/W)/(µC_OX(V_DD − V_T)) (2.9); t50% ∝ 1/(√(φ_f + V_sb) − √φ_f) (2.10). Substrate under a victim wire is an aggressor with C_c = C_s (Fig 2.6); peak response t_p = τ1 if τ1 = τ2 else (τ2τ1/(τ2−τ1))·ln(τ1/τ2); v1(t_p) = (C_c/(C1+C_c))e⁻¹ if τ1 = τ2 else (C_c/(C1+C_c))(τ1/τ2)^{τ1/(τ2−τ1)}, τ1 = (R1‖R2)(C1+C_c) (2.11). Empirical crosstalk delay ΔD ∝ L^α/S^β, α ≈ 2, β ≈ 1 [22].
- Source: ch.2 §4 eqs 2.8–2.12, L874–936 (PDF 40–42).
- Philis stage: gr (crosstalk cost shape), verify.
- Automation recipe: use the L²/S shape for the crosstalk exclusion cost between a victim and an aggressor route (parallel run length squared over spacing) instead of a pure spacing threshold. Sensitive routes over a quiet well or shield (see `docs/LAYOUT-FUNDAMENTALS.md:138` row 61).
- Beats hand layout because: run-length-aware crosstalk on every net pair.
- Philis status: partial. `CrosstalkExclusion` is spacing-based (`kernel/analog/src/routing/crosstalk.rs:13`); Σ-coupling per victim exists (`backend/annotator/src/extract.rs:78-96`).

### SUB-52 ESD devices depend on substrate resistance
- Kind: rule
- Statement: ggNMOS ESD protection turns on by substrate current forward-biasing the parasitic npn; turn-on depends heavily on substrate resistance, which must be modeled including nonlinear conductivity modulation.
- Source: ch.7 §4.4 L3161–3184 (PDF 119–120).
- Philis stage: cells, verify.
- Automation recipe: do not add isolation taps/rings inside or adjacent to ESD NMOS cells beyond the deck's ESD rule; exclude ESD cells from ring merging; treat their substrate node as shared noisy (SUB-30).
- Beats hand layout because: prevents a substrate-isolation step from silently degrading ESD.
- Philis status: missing (no ESD cell class).

### SUB-53 Layered Green's function via DCT (exact surface-contact extraction)
- Kind: algorithm
- Statement: layered-media Green's function G = G0 + Σ_m Σ_n f_mn C_mn cos(mπx/a)cos(mπx′/a)cos(nπy/b)cos(nπy′/b) (3.13), C_mn = 0/2/4 by index; p_ij closed form (3.16) with k_mn = a²b²f_mn C_mn/(m²n²π⁴); the sum becomes a 2-D DCT K(p̃, q̃) (3.18) computed once per substrate stack at O(P̃Q̃ log P̃Q̃); only P (50–5,000) is recomputed and inverted when contacts move; non-abrupt profiles via z-discretization. Contacts equipotential, subdivided if not. Admittance column: set one contact to 1 V, others and backplate to 0.
- Source: ch.3 §2–4, eqs 3.1–3.18, L973–1210 (PDF 45–51); App. A L3998–4052.
- Philis stage: verify.
- Automation recipe: `verify::substrate::extract(stack, contacts) -> Y`: precompute the DCT table per deck stack (cache file per PDK); p_ij by table lookup; invert P (dense LU, N ≤ 5000); Y from (3.12). Use for signoff and for calibrating the cheap models of SUB-31/32 (fit their constants per deck from synthetic two-contact sweeps, as the book's macromodels are fitted from measured/simulated two- and three-contact experiments, L963–967).
- Beats hand layout because: signoff-grade substrate coupling for every contact pair.
- Philis status: missing.

### SUB-54 Finite-difference resistive mesh with Krylov solver (alternative extractor)
- Kind: algorithm
- Statement: 7-point stencil ∂²Φ/∂x² ≈ (Φ_{i+1} + Φ_{i−1} − 2Φ_i)/Δ_i² (3.19); sparse; direct factorization fills badly in 3-D; use CG/GMRES with incomplete factorization or multigrid preconditioners; non-uniform grid fine near contacts, coarse far away. Handles lateral resistivity variation (wells, DNW, trenches) that the layered Green's function cannot.
- Source: ch.1 §2 L537–551 (PDF 26–27); ch.3 §5 L1212–1248 (PDF 51–52).
- Philis stage: verify.
- Automation recipe: if SUB-53 is too heavy to implement first: 3-D node grid over the block (e.g. 2 µm lateral near contacts, geometric growth to 50 µm; 5–10 z layers matching the stack), conductances from layer ρ, well/DNW regions as different σ, Dirichlet contacts; solve per aggressor column with preconditioned CG (SPD system). Output Y between contacts. Grid parameters are proposals, not source values.
- Beats hand layout because: includes wells and trenches that the analytic models ignore.
- Philis status: missing.

### SUB-55 Accelerated matrix–vector products for large contact counts
- Kind: algorithm
- Statement: dense BEM Z (3.21) needs one solve per extracted column; accelerate matvecs with FCT when contacts are uniform and dense; precorrected FFT/DCT, hierarchical interpolation + SVD compression (IES3), multilevel moment-matching multigrid, wavelet-like bases (also preconditioners) give near-O(N) with user-controlled tolerance, because the potential is complex only near the source.
- Source: ch.3 §5 L1249–1327 (PDF 52–54).
- Philis stage: verify.
- Automation recipe: not needed for analog blocks (N in the hundreds); note as the scale-up path if Philis handles mixed-signal tops with thousands of contacts (Table 6.5: 2,500 contacts required acceleration).
- Beats hand layout because: n/a (scalability).
- Philis status: missing (not required now).

### SUB-56 Signoff substrate report: per-pair isolation, margins, remedies
- Kind: check / metric
- Statement: constraint violations localize the cause of a system-level spec violation; final verification extracts parasitics and checks specs. Isolation must be reported relative to a known cross-section; absolute numbers are not comparable across technologies.
- Source: ch.5 intro L1762–1767 (PDF 70), §3 L1892–1901 (PDF 73); App. D note 1 L4636–4641 (PDF 183).
- Philis stage: verify, flow.
- Automation recipe: add a `substrate` section to `verify::signoff` output (`backend/verify/src/lib.rs:45`): per aggressor–victim pair: distance, isolation dB (SUB-04/33), victim V_peak vs V0_max (SUB-15), DM/CM for pairs (SUB-46), ring role and ring-net check (SUB-30), and the suggested remedy (distance / ring / DNW / backplate). Feed the count of failing pairs into the epoch's Θ residual so best-epoch selection sees it.
- Beats hand layout because: a machine-checked substrate signoff that hand flows lack.
- Philis status: missing.

### SUB-57 Calibration of substrate models against measured or reference data
- Kind: check
- Statement: DC test chip: 20 p+ contacts and 4 rings, 0.1 V bias (1 V showed nonlinearity), voltage-ratio and admittance experiments; simulation matched within 15 % for ratios > 0.5, good for > 0.1, degraded below 0.1 with systematic error from the approximate profile and missing contact resistances. HF two-port: sweep distance and port size to calibrate and validate; concentric disc/ring ports remove boundary effects; probe floor −40…−45 dB; on-wafer tests miss package inductance.
- Source: App. D.1 L4234–4341 (PDF 170–174); D.2 L4343–4444 (PDF 174–177).
- Philis stage: verify, deck.
- Automation recipe: per deck, a synthetic calibration: run SUB-53/54 on two-contact sweeps (sizes 10–1000 µm, distances 3–1000 µm, matching the book's sweeps) and fit the SUB-31/32 constants; store in the deck `substrate.fit`. Accept the fit only if the model is within 15 % for coupling ratios > 0.1 (the book's measured-accuracy level).
- Beats hand layout because: every PDK gets fitted coefficients; nobody hand-derives them.
- Philis status: missing.

### SUB-58 Stimulus dependence and iteration of switching models
- Kind: heuristic
- Statement: noise signatures depend strongly on stimulus statistics (fast random vs slow stimuli); with varied stimuli build a parametric statistical model or a super-model of per-stimulus models and verify against each. Iterative delay/noise refinement converges in < 5 iterations when activity is moderate and can oscillate with many simultaneous switchings.
- Source: ch.4 §5 L1705–1748 (PDF 65–67).
- Philis stage: annotator.
- Automation recipe: for clocked aggressors, evaluate the substrate check at the worst-case (all synchronous, SUB-17) and nominal activity; report both. Do not iterate delay–noise coupling inside P&R.
- Beats hand layout because: explicit worst/nominal corners.
- Philis status: missing.

---

## 4. Top-15 priorities for Philis

1. **Deck substrate stack + kind switch** (SUB-01, SUB-05): every other substrate rule is wrong without it; today sky130 (bulk) is scored with an epi plateau at a nominal 2.5 µm (`emit.rs:250-256,274`). Add `substrate{kind, layers, backplate, l_gnd_nh, l_ring_nh, dnwell}` to `pdks/*.json` and `ProcessNumbers`.
2. **Kind-specific transfer model replacing the fixed 4×epi keep-out** (SUB-31, SUB-32, SUB-42): plateau at d_sat for epi, monotone distance/area model for bulk; the cost keeps a slope where distance still helps.
3. **Injector and victim weights in the annotator** (SUB-06, SUB-08, SUB-09, SUB-10, SUB-12): op-point data (`oppoint.rs:184-205`) already carries id/vds/vdsat/gm; add gmb and node impedance; aggressors beyond `NetClass::Clock`.
4. **Per-victim substrate-voltage budgets and per-pair minimum distances** (SUB-15, SUB-16, SUB-18): converts specs into V0_max and α per pair; cluster contacts keep it cheap.
5. **Guard-ring net selection and role** (SUB-30): rings tied to the device bulk (`constraints.rs:83`) can inject; dedicated/quiet ring nets, star-routed, checked in verify. Highest return for the least code.
6. **Substrate-kind ring policy and ring location** (SUB-28, SUB-34, SUB-35): no isolation rings in epi-on-p+, rings around the injector by default in bulk, dual rings when pins allow; stop ringing every MOS for isolation.
7. **Two-port and ring closed-form isolation evaluator** (SUB-04, SUB-33): O(1) per pair, frequency and package aware; the core of both the dp cost and the signoff report.
8. **dp substrate cost with epoch-exact / move-estimated evaluation** (SUB-23, SUB-22, SUB-03): follow the thermal superposition pattern (`thermal.rs`) with gradients; exact only near violations. The book's PLL: jitter 0.1 → 0.005 vs spec 0.007.
9. **Differential pairs: injector on the symmetry axis and equal substrate exposure** (SUB-43, SUB-44, SUB-39): 5 % substrate-C mismatch costs ~40 dB of differential isolation; one shared thin ring per pair.
10. **Signoff substrate report fed into Θ** (SUB-56, SUB-46): per-pair isolation dB, V_peak vs budget, DM/CM, remedy; failing pairs count toward the epoch residual.
11. **Capacitor plate orientation and differential AC-coupling bottom-plate coupling** (SUB-47): cheap annotator rule plus a verify check (1.6 dB at r = 0.2).
12. **Substrate feedback check in high-gain stages** (SUB-13): Miller via substrate and Barkhausen risk; spacing or ring on the output device.
13. **DNW triple-well option for sensitive NMOS groups** (SUB-38): deck has `dnwell`; reuse the NPN DNW drawing; place around the smaller circuit.
14. **Ring-in-search-space branch** (SUB-25): discrete ring bit per pair, like `DtiBand.branch`, so dp trades ring area against distance.
15. **Exact extractor for calibration and signoff** (SUB-53 or SUB-54, SUB-57, SUB-21): layered DCT Green's function or FD mesh + CG; fit the cheap models per deck; Sherman–Morrison when N is moderate.
