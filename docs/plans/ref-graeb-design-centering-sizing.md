# Graeb — Analog Design Centering and Sizing (Springer 2007): study for Philis

Source: Helmut E. Graeb, *Analog Design Centering and Sizing*, Springer, 2007 (ISBN 978-1-4020-6003-8). PDF: `docs/ref/Analog Design Centering and Sizing -- Helmut E_ Graeb -- 2007 ... .pdf`.
Reftext file: `scratchpad/reftext/graeb_centering.txt` (pdftotext -layout), **lines 1–8560 (entire file) read**.
Page convention: "PDF p.N" = 1 + number of form feeds before the cited line (the book's printed page is roughly PDF p. − 18…20; printed numbers appear in the running heads quoted in the reftext).

What this book is and is not: it is the mathematical core of tolerance design (parameter/performance spaces, tolerance classes, worst-case analysis, Monte-Carlo and geometric yield analysis, worst-case-distance-based yield optimization). It contains **no layout content** and **no list of sizing rules**; it defines the sizing-rule constraints only as `c(x_d) ≥ 0` ("technological and structural requirements concerning DC properties of transistors", citing Graeb et al. ICCAD 2001 [58]). Every layout mapping below is therefore Philis-side reasoning built on the book's formulas; the formulas and numbers are the book's.

---

## 1. Coverage

### Read chunks (Read tool, consecutive, no gaps)
A 2000-line chunk exceeded the tool's token cap, so smaller chunks were used:

| chunk | reftext lines |
|---|---|
| 1 | 1–1200 |
| 2 | 1201–2300 |
| 3 | 2301–3400 |
| 4 | 3401–4500 |
| 5 | 4501–5600 |
| 6 | 5601–6700 |
| 7 | 6701–7700 |
| 8 | 7701–8560 |

PDF page opened: p.64 (to confirm the §3.9 wording "Local variations decrease with increasing gate area and increasing distance", which the reftext reproduces correctly; see erratum note in §2, §3.9). All equations used below were legible in the reftext.

### Every heading in the range (reftext line)
- Front matter: title/copyright (1–48); Contents (49–215); List of Figures (216–478); List of Tables (479–519); Preface (520–589)
- Chapter 1 Introduction (590): 1.1 Integrated Circuits (597); 1.2 Analog Circuits (694); 1.3 Analog Design (763); 1.4 Analog Sizing (1011)
- Chapter 2 Tolerance Design: Example (1191): 2.1 RC Circuit (1198); 2.2 Performance Evaluation (1221); 2.3 Performance-Specification Features (1231); 2.4 Nominal Design/Performance Optimization (1243); 2.5 Yield Optimization/Design Centering (1320)
- Chapter 3 Parameters & Tolerances, Performance & Specification (1504): 3.1 Parameters (1512); 3.2 Parameter Tolerances (1575); 3.3 Range-Parameter Tolerances (1597); 3.4 Statistical Parameter Distribution (1684); 3.5 Univariate Normal Distribution (1724); 3.6 Multivariate Normal Distribution (1830); 3.7 Transformation of Statistical Distributions (2059); 3.8 Generation of Normally Distributed Sample Elements (2224); 3.9 Global and Local Parameter Tolerances (2293); 3.10 Performance Features (2339); 3.11 Numerical Simulation (2365); 3.12 Performance-Specification Features (2424)
- Chapter 4 Analog Sizing Tasks (2472): 4.1 Sensitivity-Based Analysis (2479) — 4.1.1 Similarity of Performance Features (2566); 4.1.2 Similarity of Parameters (2590); 4.1.3 Significance of Parameters (2601); 4.1.4 Adjustability of Performance Features (2613); 4.1.5 Multiple-Objective Behavior (2628); 4.1.6 Exercise (2639); 4.1.7 Sensitivity-Based Analysis of Tolerance Objectives (2650). 4.2 Performance-Sensitivity Computation (2657) — 4.2.1 Simulator-Internal Computation (2663); 4.2.2 Finite-Difference Approximation (2682). 4.3 Scaling Parameters and Performance Features (2706) — 4.3.1 Scaling to a Reference Point (2716); 4.3.2 Scaling to the Covered Range of Values (2728); 4.3.3 Scaling by Affine Transformation (2747); 4.3.4 Scaling by Equalization of Sensitivities (2758). 4.4 Nominal Design (2808). 4.5 Multiple-Objective Optimization (2817) — 4.5.1 Smaller or Greater Performance Vectors (2863); 4.5.2 Pareto Point (2942); 4.5.3 Pareto Front (2967); 4.5.4 Pareto Optimization (3024). 4.6 Single-Objective Optimization (3035) — 4.6.1 Vector Norms (3100); 4.6.2 Performance Targets (3126). 4.7 Worst-Case Analysis and Optimization (3145) — 4.7.1 Worst-Case Analysis (3159); 4.7.2 Worst-Case Optimization (3293). 4.8 Yield Analysis, Yield Optimization/Design Centering (3363) — 4.8.1 Yield (3370); 4.8.2 Acceptance Region Partitions (3462); 4.8.3 Yield Partitions (3580); 4.8.4 Yield Analysis (3615); 4.8.5 Yield Optimization/Design Centering (3644); 4.8.6 Tolerance Assignment (3774); 4.8.7 Beyond 99.9% Yield (3807)
- Chapter 5 Worst-Case Analysis (3878): 5.1 Classical Worst-Case Analysis (3929) — 5.1.1 Classical Worst-Case Parameter Vectors (4005); 5.1.2 Classical Worst-Case Performance Values (4043); 5.1.3 Discrete Parameters (4052); 5.1.4 Corner Worst Case (4083). 5.2 Realistic Worst-Case Analysis (4094) — 5.2.1 Realistic Worst-Case Parameter Vectors (4189); 5.2.2 Realistic Worst-Case Performance Values (4213). 5.3 Yield/Worst-Case Distance – Linear Performance Feature (4226). 5.4 General Worst-Case Analysis (4284) — 5.4.1 General Worst-Case Parameter Vectors (4421); 5.4.2 General Worst-Case Performance Values (4442). 5.5 Yield/Worst-Case Distance – Nonlinear Performance Feature (4475) — 5.5.1 Yield Approximation Accuracy (4546); 5.5.2 Realistic Worst-Case Analysis as Special Case (4704). 5.6 Exercise (4718)
- Chapter 6 Yield Analysis (4746): 6.1 Statistical Yield Analysis (4771) — 6.1.1 Monte-Carlo Analysis (4858); 6.1.2 Importance Sampling (4912); 6.1.3 Yield Estimation Accuracy (4966). 6.2 Tolerance Classes (5151) — 6.2.1 Tolerance Interval (5161); 6.2.2 Tolerance Box (5215); 6.2.3 Tolerance Ellipsoid (5317); 6.2.4 Single-Plane-Bounded Tolerance Region (5425); 6.2.5 Corner Worst Case vs. Realistic Worst Case (5645). 6.3 Geometric Yield Analysis (5701) — 6.3.1 Problem Formulation (5713); 6.3.2 Lagrangian Function (5858); 6.3.3 First-Order Optimality Condition (5894); 6.3.4 Second-Order Optimality Condition (5950); 6.3.5 Worst-Case Range-Parameter Vector (6013); 6.3.6 Worst-Case Statistical Parameter Vector (6042); 6.3.7 Worst-Case Distance (6079); 6.3.8 Geometric Yield Partition (6143); 6.3.9 Geometric Yield (6187); 6.3.10 General Worst-Case Analysis/Geometric Yield Analysis (6276); 6.3.11 Approximate Geometric Yield Analysis (6317). 6.4 Exercise (6327)
- Chapter 7 Yield Optimization/Design Centering (6362): 7.1 Statistical-Yield Optimization (6385) — 7.1.1 Acceptance-Truncated Distribution (6386); 7.1.2 Statistical Yield Gradient (6454); 7.1.3 Statistical Yield Hessian (6536); 7.1.4 Solution Approach to Statistical-Yield Optimization (6579); 7.1.5 Tolerance Assignment (6619); 7.1.6 Deterministic Design Parameters (6628). 7.2 Geometric-Yield Optimization (6780) — 7.2.1 Worst-Case-Distance Gradient (6808); 7.2.2 Solution Approaches to Geometric-Yield Optimization (6921); 7.2.3 Least-Squares/Trust-Region Solution Approach (6996); 7.2.4 Min-Max Solution Approach (7156); 7.2.5 Linear-Programming Solution Approach (7174); 7.2.6 Tolerance Assignment, Other Optimization Parameters (7207)
- Appendix A Expectation Values (7216): A.1 (7222) … A.11 Exercises (7357)
- Appendix B Statistical Estimation of Expectation Values (7370): B.1 (7376) … B.7 Exercises (7499)
- Appendix C Optimality Conditions of Nonlinear Optimization Problems (7513): C.1 (7525), C.2 (7561), C.3 (7592), C.4 (7696), C.5 (7724), C.6 (7826), C.6.1 (7877), C.7 Bounding-Box-of-Ellipsoids Property (7896)
- References [1]–[128] (7941–8436); Index (8437–8560)

---

## 2. Section-by-section digest

### Front matter (1–589)
- Contents, figure and table lists (49–519) confirm the book has 7 chapters + 3 appendices; Table 6 (WCA types), Tables 7–8 (MC accuracy), 9–13 (tolerance classes), 14–15 (op-amp WCD example) are the numeric anchors (484–519).
- Preface: the worst-case problem "stands at the interface between process technology and design technology"; algorithms are those in the commercial tool WiCkeD [88] (525–532).
- Ch.5 develops three WCA types (classical, realistic, general), Ch.6 statistical (MC) and geometric yield analysis, Ch.7 the two corresponding yield-optimization approaches; derivatives for sensitivity-based optimization are derived (569–585).

### 1.1 Integrated Circuits (597–692)
- Yield ("percentage of ICs that pass the production test") and reliability are the targets process technology must keep constant while scaling (634–635).
- Relative process variations grow and "cannot be compensated by circuit design alone"; statistical methods move from analog into digital design (674–692).

### 1.2 Analog Circuits (694–761)
- Analog uses ~20% of IC area but ~40% of design effort and ~50% of re-spins (EDACafé 2005) (700–704).
- Hierarchical circuits (OTA-C filter from OTAs) can be simulated flat; PLL-scale circuits need behavioral models (741–761).

### 1.3 Analog Design (763–1009)
- Design levels (system, architecture, circuit, device, process), partitioning, and views (behavior, structure, geometry) (Table 1, 785–792).
- Synthesis = behavioral→structural→geometric; three analog synthesis phases: structural synthesis, parametric synthesis (sizing), layout synthesis (placement and routing) (906–917).
- Top-down synthesis must be accompanied by bottom-up analysis/verification; inaccurate models cause repeated design loops (918–921, 991–996).
- Behavioral block models must carry "bottom-up modeling of the performance capabilities" to avoid unrealistic block requirements (944–946).

### 1.4 Analog Sizing (1011–1190)
- Sizing = nominal design (no tolerances; optimize nominal performance subject to constraints) then tolerance design (1014–1019).
- Manufacturing variation = global (equal for all transistors, e.g. oxide thickness) + local (independent per transistor, e.g. threshold voltage); operating conditions = intervals (supply, temperature) (1020–1027).
- Worst-case optimization minimizes worst-case deviation and thereby performance sensitivity; yield optimization maximizes the fraction meeting spec; statistical (MC) vs geometric (acceptance-region approximation) approaches (1030–1062).
- Design centering: max yield when the center of gravity of the pdf equals that of the truncated pdf (1078–1085); yield > 99.9% is hard for statistical optimization, inherent for geometric (1098–1102).
- Number of simulations is the CPU-cost measure (1135–1141). Table 4 (op-amp): initial yield 0%, after nominal design 89%, after yield optimization 99.9%, achieved by trading transit-frequency margin (20→18 MHz) for phase margin (68°→72°) (1154–1184).
- Nominal design first because tolerance/yield in the loop multiplies simulation count (1185–1190).

### 2.1–2.3 RC Circuit, evaluation, specification (1198–1241)
- τ = R·C, A = R + C, normalized dimensionless R, C (1201–1215); τ is nonlinear in parameters although the circuit is linear (1227–1229).
- Spec: 0.5 ≤ τ ≤ 2.0, A ≤ 4.0; each bound is one "performance-specification feature" (1232–1241).

### 2.4 Nominal Design (1243–1318)
- min A s.t. τ = 1.25 → R = C = √(5/2) ≈ 1.118, A = √5 ≈ 2.236 (1251–1260).
- Centering performance values ≠ centering parameter values because of nonlinearity; centering x in parameter space would give R = C ≈ 1.061, τ = 1.125 (1303–1318).

### 2.5 Yield Optimization/Design Centering (1320–1502)
- σR = 0.2, σC = 0.8, ρ = 0; joint normal pdf (5) (1324–1335).
- Parametric yield vs catastrophic yield loss (spot defects) distinguished; "yield" = parametric in the book (1369–1378).
- Yield after nominal design 58.76% (6); after centering Y(0.569, 2.016) = 74.50% (7) (1379–1380, 1495–1497).
- Nominal design misses max yield because of (a) nonlinearity skewing equal performance margins in parameter space and (b) skewed parameter spread from variances/correlations (1436–1487).

### 3.1 Parameters (1512–1573)
- Parameters = simulator inputs; performance features = simulator outputs (1513–1526).
- Design parameters x_d (W, L), statistical parameters x_s (oxide thickness, Vth, channel-length reduction), range parameters x_r (supply, temperature: e.g. −40…125 °C; no probability attached) (1528–1573).
- In ICs x_d and x_s are separate sets, which makes statistical yield optimization expensive (1554–1560).

### 3.2 Parameter Tolerances (1575–1595)
- Two tolerance types: ranges (operating conditions, intervals) and distributions (manufacturing) (1576–1591).
- Extracting statistical parameter distributions needs dedicated optimization and consistent multi-level measurement (1592–1595).

### 3.3 Range-Parameter Tolerances (1597–1682)
- Tolerance regions: box (8), polytope (9), ellipsoid (10), nonlinear (11) (1602–1628).
- Box is the prevalent range-parameter form; closed regions reflect physical/resource limits; operating range width is priced (−40…125 °C costs more than 0…70 °C) (1644–1659).
- Ellipsoid regions coincide with Gaussian level sets and bridge range and statistical parameters (1672–1678).

### 3.4 Statistical Parameter Distribution (1684–1722)
- pdf/cdf relations (13)–(15) (1693–1719).

### 3.5 Univariate Normal Distribution (1724–1828)
- pdf (16), cdf via φ0/erf (17)–(19) (1726–1748).
- Table 5: cdf at −3σ…+4σ = 0.1%, 2.2%, 15.8%, 50%, 84.1%, 97.7%, 99.8%, 99.99% (1775–1779).
- Univariate: unique relation between tolerance interval and yield, Y = cdf(x_U) − cdf(x_L) (22) (1809–1815).

### 3.6 Multivariate Normal Distribution (1830–2057)
- pdf (23) with β²(x_s) = (x_s − x_s,0)ᵀC⁻¹(x_s − x_s,0) (24) (1834–1838).
- C = Σ·R·Σ (26)–(29); |ρ| < 1 assumed so C is positive definite (33) (1892–1949).
- Level sets are ellipsoids; correlation tilts/narrows them (Fig. 22) (1987–2025).
- Bounding-box-of-ellipsoids (37): union over all correlations of β-ellipsoids = box ±βσ_k (2026–2042).
- Closed-form cdf only for R = I (38) (2043–2057).

### 3.7 Transformation of Statistical Distributions (2059–2222)
- Non-normal physical parameters must be transformed to normal ones; transformed parameters lose physical meaning (2066–2075).
- Change of variables (39)–(41); uniform→any via inverse cdf (43); lognormal (46) for positive quantities; χ² with 1 dof (50) describes ellipsoidal regions (2076–2215).

### 3.8 Generation of Normally Distributed Sample Elements (2224–2291)
- Three steps: uniform pseudo-random z; y = cdf⁻¹_N(z); x_s = A·y + x_s,0 with C = A·Aᵀ (56)–(58) (2226–2273).
- Cholesky or eigen-decomposition A = VΛ^½ (60); eigen preferred for ill-conditioned C from highly correlated parameters (2277–2291).

### 3.9 Global and Local Parameter Tolerances (2293–2337)
- Global = inter-chip/wafer, affects all transistors equally (oxide thickness, channel-length reduction); local = intra-chip, per transistor, statistically independent (2297–2308).
- "Local variations decrease with increasing gate area and increasing distance between transistors [95]" (2308–2309, confirmed on PDF p.64). **Erratum for Philis use:** the cited Pelgrom model [95] has mismatch *increasing* with distance (σ² ∝ D²); treat "decrease … distance" as a wording error, keep "decrease with gate area".
- Pairs such as current mirrors need identical behavior and are "very sensitive to mismatch" (2310–2313).
- 100 transistors → 1 global Vth + 100 local Vth parameters (2319–2325); block covariance (61): C = diag(C_glob, Σ_loc), Σ_loc diagonal (2326–2333).
- The simulator-level transistor parameter may be x_d,k + x_s,glob,l + x_s,loc,m (2334–2337).

### 3.10 Performance Features (2339–2363)
- Performance features = simulation outputs (gain, phase margin, slew rate, PSRR) (2343–2350); x → f (62) (2353–2363).

### 3.11 Numerical Simulation (2365–2422)
- Circuit DAE (63); simulation x → (u_n, i_Z) (64) then post-processing → f (65); DC, AC, TR features (2366–2395).
- One simulation costs minutes to days; performance models/macromodels pay off only if reused enough; break-even not investigated (2398–2422).

### 3.12 Performance-Specification Features (2424–2471)
- Spec feature φ_PSF,µ(f) ≥ 0 (66); prevalent form bounds f_i − f_L,i ≥ 0, f_U,i − f_i ≥ 0 (68)–(69); acceptance box A_f (70) (2433–2466).
- Verification against spec may be needed after structural synthesis "prior to the layout design phase" and after production test (2428–2431).

### 4.1 Sensitivity-Based Analysis (2479–2564) and 4.1.1–4.1.7 (2566–2656)
- Sensitivity matrix S = ∂f/∂xᵀ = J ∈ R^{nf×nx} (71); Δf = S·Δx (72) (2485–2548).
- 4.1.1 Similarity of performance features: cos φ(f_i, f_j) of gradients (73); 0 = independently tunable, ±1 = inseparable (2566–2588).
- 4.1.2 Similarity of parameters: cos φ(x_k, x_l) between columns (74) (2590–2599).
- 4.1.3 Significance: ‖∇f(x_k)‖ ranks parameters for equal Δx (75)–(76) (2601–2611).
- 4.1.4 Adjustability: ‖∇f_i(x)‖ ranks performance features; max change along the gradient, Δf_i,max = ‖∇f_i‖² (77)–(79) (2613–2626).
- 4.1.5 Multiple-objective behavior: Δf|Δf_i,max = S·∇f_i(x) (80) (2628–2637). 4.1.6 Exercise S = [[1,2],[−1,1]] (2639–2647).
- 4.1.7 The same analysis applies to tolerance objectives: worst-case performance values and worst-case distances (2650–2655).

### 4.2 Performance-Sensitivity Computation (2657–2704)
- Simulator-internal via chain rule (82): ~10% of one simulation per parameter; requires model derivatives (2663–2680).
- Finite differences (83): 100% of one simulation per parameter; Δx_k "large enough to surmount numerical noise and small enough to compare to the gradient" (2682–2704).

### 4.3 Scaling (2706–2806)
- Improper scaling degrades condition and makes solvers fail (2707–2711).
- Reference-point scaling (85)–(86); covered-range scaling to [0,1] (87) with carefully chosen bounds (2716–2745).
- Affine whitening x' = A⁻¹(x − x_s,0) (88): independent, values "most probably" in [−3, 3], condition number 1 (2747–2756).
- Equalizing rows/columns of S to unit length (90)–(91) equalizes parameter/feature visibility; implementations must use scaled variables (2758–2806).

### 4.4 Nominal Design (2808–2815)
- Optimizes performance without tolerances; the same multi/single-objective machinery applies to tolerance objectives (2809–2815).

### 4.5 Multiple-Objective Optimization (2817–3033) and 4.5.1–4.5.4
- min f(x_d) s.t. c(x_d) ≥ 0 (92); **c(x_d) ≥ 0 = "technological and structural requirements concerning DC properties of transistors … for a proper function and robustness [58]"; they "determine the achievable performance feature values of a given circuit structure"** (2824–2834).
- Statistical/range parameters may be fixed at worst-case values during nominal design (2835–2839); mixed objective/constraint form (93) (2841–2851).
- 4.5.1 Vector order (94)–(99); superior set bounded by l∞ contour (2863–2935).
- 4.5.2 Pareto point = min over x_d of weighted l∞ distance from reference point f_RP (101); f_RP = individual minima (102) (2942–2964).
- 4.5.3 Pareto front (103), may be discontinuous/nonconvex (2967–3022). 4.5.4 NBI and goal attainment suit Pareto optimization; weighted sum only for convex regions (3024–3033).

### 4.6 Single-Objective Optimization (3035–3143), 4.6.1–4.6.2
- Performance centering: meet spec with maximum margin; scalarization adds nonlinearity and must be "defined very carefully" (3055–3061).
- Practical form (104): min ‖f_I − f_I,target‖ s.t. f_L ≤ f ≤ f_U, c(x_d) ≥ 0; targets are realistic goals and act as scaling (3066–3087).
- 4.6.1 Weighted l1 (105), l2 (106), l∞ (107); l2 differentiable, favored for gradient methods (3100–3124).
- 4.6.2 Two-sided bound → target at interval center (108); one-sided → from experience and sensitivities, updated during optimization (3126–3143).

### 4.7 Worst-Case Analysis and Optimization (3145–3361), 4.7.1–4.7.2
- Worst-case deviation ∝ sensitivity; worst-case optimization = minimize sensitivity without nominal drift (3152–3157).
- WCA (109)–(110): min/max f_i over x_s ∈ T_s(Y_L), x_r ∈ T_r; 1 ≤ n_WC ≤ 2n_f (3159–3192); T_s for a yield Y_L chosen heuristically in the multivariate case (3196–3199).
- Output mapping (112): worst-case parameter vectors and performance values per bound (3202–3218). WC vectors may coincide or cluster → fewer vectors needed (3271–3275); WC can lie inside T if the function is unimodal there (3276–3284).
- WC vectors need process data *and* concrete circuit and performance features (3285–3291).
- 4.7.2 Worst-case optimization (113) = worst-case analysis in a loop; four nested loops (optimization → WCA → sensitivity → simulation), Fig. 30; simplifications: WCA once per step or every µth step; sensitivity once per WCA step or once per WCA (3293–3352). Nominal design can already include linearized worst-case assumptions (3353–3361).

### 4.8 Yield Analysis / Design Centering (3363–3877), 4.8.1–4.8.7
- 4.8.1 Y = prob{∀x_r ∈ T_r: f_L ≤ f(x_d, x_s, x_r) ≤ f_U} (114); integrals over A_s (116) or A_f (117) (3370–3457).
- 4.8.2 Partition A_f and A_s per spec feature (118)–(125); A_s,L,i requires the bound for **all** range-parameter vectors (123); non-acceptance regions (126)–(133) (3462–3577).
- 4.8.3 Acceptance functions δ (134)–(136); Y = E{δ} (137), partitions Y_L,i, Y_U,i (138)–(139); partitions rank spec features by impact on robustness; **the smallest partition is an upper bound on overall yield** (3580–3613).
- 4.8.4 Yield analysis mapping (140); WC values become bounds of a yield analysis and a yield becomes the input of a WCA (3615–3642).
- 4.8.5 max Y (141) (single objective) or max partitions (142) (multi-objective, cheaper); yield optimization picks one point of the Pareto front (Fig. 34) (3644–3702). Simplifications: yield analysis every kth step, variable accuracy (3703–3715). Two mechanisms: reshape A_s by x_d (Fig. 36) or move x_s,0 (143) (Fig. 37a); max yield ≠ max inscribed tolerance region (3740–3759).
- 4.8.6 Tolerance assignment (144) with det C = const to avoid trivial C = 0 (3774–3789); also w.r.t. range-parameter bounds and spec bounds; select largest tolerances that keep yield (discrete resistors ±1%/±10%) (3792–3805).
- 4.8.7 Yield is a very weak optimum above 99.9% (premature termination); remedy: inflate C := a·C, a > 1, once Y > 99% (145); or max det C s.t. Y = 50% (146)/(147) (50% is the most sensitive point); geometric approach has no weak optimum (3807–3877).

### Chapter 5 intro (3878–3927)
- Table 6: classical = box + linear (uniform/unknown distributions, discrete or range parameters); realistic = ellipsoid + linear (normal, IC transistor parameters); general = ellipsoid + nonlinear (3903–3911).
- Classical WC vectors give "exaggerated robustness" for ICs because correlations are ignored (3916–3921); linearization is generally insufficient → general WCA (3922–3927).

### 5.1 Classical WCA (3929–4092), 5.1.1–5.1.4
- Box (148), linear model (149), LP (150)–(151); worst case usually at a box corner (3930–3974); KKT (152)–(157) (3976–4003).
- 5.1.1 x_r,WL,k = x_r,L,k if ∂f/∂x_k > 0, x_r,U,k if < 0, undefined if 0 (158); upper analogous (159); zero gradient → keep nominal in practice (4005–4032).
- 5.1.2 f_WL/U = f0 + ∇fᵀ(x_WL/U − x0) (160) (4043–4050).
- 5.1.3 Tested discrete parts have truncated distributions (price classes); classical WCA is adequate for them (4052–4081).
- 5.1.4 Corner worst case: each relevant statistical parameter at a multiple of σ in the deteriorating direction (slow/fast, slow-slow, slow-fast, fast-fast) (4083–4092).

### 5.2 Realistic WCA (4094–4224), 5.2.1–5.2.2
- Ellipsoid (161), linear model (162), problem (163)–(164); KKT (165)–(170) (4094–4187).
- 5.2.1 x_s,WL − x_s,0 = −(β_W/σ_f̄)·C·∇f (171)–(172), upper with + (173)–(174); σ²_f̄ = ∇fᵀC∇f (175) (4189–4211).
- 5.2.2 f_WL = f0 − β_W·σ_f̄ (176)–(177), f_WU = f0 + β_W·σ_f̄ (178)–(179) (4213–4224).

### 5.3 Yield/WCD – linear (4226–4282)
- Linear f is normal N(f0, σ²) (180); Y_U = Y_L = ∫_{−∞}^{β_W} φ (182)–(183); two-sided Y = ∫_{−β_W}^{β_W} φ (184) (4228–4261).
- β_W = "worst-case distance", measured in performance standard deviations; β_W = 3 three-sigma design, 6 six-sigma design (4266–4281).

### 5.4 General WCA (4284–4473), 5.4.1–5.4.2
- Nonlinear f over ellipsoid (185)–(187); WC touches ellipsoid tangentially; linearization at the WC point (188) (4285–4350).
- Solutions not generally unique; **mismatch-sensitive performances have semidefinite Hessians: the nominal lies on a ridge constant along equal changes of two local parameters; typically two WC vectors in opposite directions** (4355–4368). Solve by SQP (4369–4373).
- KKT (189)–(193); same form as realistic WCA but with the gradient at each WC point (194)–(198) (4375–4440); f_WL = f̄0^(WL) − β_W·σ_f̄(WL) (200)–(203) (4442–4473).

### 5.5 Yield/WCD – nonlinear (4475–4716), 5.5.1–5.5.2
- Linearized f at the WC point is normal (204); approximate partitions Ȳ via β_W (205)–(207) (4477–4544).
- 5.5.1 Error from approximating the acceptance region by a tangent plane (208)–(213); overestimate for one bound, underestimate for the other (Fig. 44) (4546–4633).
- Error weighted by pdf, exact at the WC point; **practical error ≈ 1%–3% absolute yield**; duality: the WC-point tangent is the best tangential approximation (glb or lub); error shrinks with yield, negligible above 99.9% (4634–4685).
- Two-sided Ȳ_i by (184) is a valuable approximation even with non-parallel gradients (4686–4692); interior WC with unimodal f → 100% (4696–4702).
- 5.5.2 Realistic WCA = first iteration of general WCA; optimizer can detect linearity and stop (4704–4716).

### 5.6 Exercise (4718–4745)
- f = x1·x2, x0 = [1,1], C = diag(0.2², 0.8²), 0.5 ≤ f ≤ 2.0: classical (3σ box), realistic (β_W = 3), general WCA (4719–4745).

### 6.1 Statistical Yield Analysis (4771–5149), 6.1.1–6.1.3
- MC: accuracy independent of number of parameters, needs many simulations; geometric: more efficient but cost ∝ n parameters (4758–4769).
- Ŷ = n_ok/n_MC (219); partition estimators (220); pseudo-random sequences are not strictly i.i.d. (4782–4819).
- 6.1.1 MC integration with indicator h_A and sampling pdf (221)–(222) (4858–4910).
- 6.1.2 Importance sampling (223)–(224): weight δ by pdf/pdf_MC; ideal spread ~50% in/out (4912–4964).
- 6.1.3 σ²_Ŷ = Y(1−Y)/n_MC (225), estimator (226); maximal at Y = 50% (227)–(229) (4966–5035). Table 7 (Y = 85%): n = 10/50/100/500/1000 → σ_Y = 11.3/5.0/3.6/1.6/1.1% (5051–5055). Normal approx valid if #pass > 4, #fail > 4, n ≥ 10 (5060–5071). k_γ: 90% 1.645, 95% 1.960, 99% 2.576, 99.9% 3.291 (5086–5091). n_MC ≈ Y(1−Y)k²_γ/ΔY² (232); Table 8: 85% ± 1% at 99% needs 8,461 (5097–5129). Accuracy ∼ √n, complexity ∼ n, independent of nonlinearity and #parameters (5141–5149).

### 6.2 Tolerance Classes (5151–5699), 6.2.1–6.2.5
- 6.2.1 Interval ↔ Y_I (233)–(235); Table 9: ±1σ 68.3%, ±2σ 95.5%, ±3σ 99.7%; one-sided 3σ 99.9% (5161–5213).
- 6.2.2 Box ↔ Y_B (236)–(241), numerical except R = I; Table 10 (±3σ): n = 3…10 → 99.1…97.0% (ρ = 0), 99.3…98.7% (ρ = 0.8); box is the worst case when correlations are unknown (5215–5315).
- 6.2.3 Ellipsoid ↔ Y_E via χ²_{n} (242)–(248); Table 11 (β_W = 3): n = 2 98.9%, 3 97.1%, 4 93.9%, 5 89.1%, 6 82.6%, 7 74.7%, 8 65.8%, 9 56.3%, 10 46.8%, 15 12.3%; good for sampling, bad as a tolerance class (5317–5423).
- 6.2.4 Single-plane-bounded region (249)–(259): tied to one spec feature, yield independent of n, variances and correlations; β_W signed (+ satisfied, − violated); Table 12: β = −1…3 → 15.9…99.9%; touches the β-ellipsoid (260)–(262); geometrically optimal approximation for one spec feature (5425–5643).
- 6.2.5 β_W,B ≥ β_W (263); Table 13 (±3σ box, "βW = 3σf"): n = 2: 4.2/5.1/6.7/13.4σ_f for ρ = 0/0.3/0.6/0.9; n = 4, ρ = 0.9: 19.0σ_f — "excessive" (5645–5698).

### 6.3 Geometric Yield Analysis (5701–6325), 6.3.1–6.3.11
- 6.3.1 WC point = max-pdf point on the other side of the spec border (264)–(265); mismatch-sensitive features give two nearly symmetric WC vectors per mismatch-producing pair (5757–5778); nested formulation with range parameters (266)–(274) (5779–5856).
- 6.3.2–6.3.3 Lagrangians (275)–(280); KKT (281)–(287); constraint active: λ > 0, f(x_WC) = bound (288)–(289) (5858–5948).
- 6.3.4 Second-order condition (290): ellipsoid curvature must exceed acceptance-border curvature; spurious stationary points and extra local minima; "suitable measures have to be taken" for mismatch parameters (5950–6010).
- 6.3.5 Range-parameter WC vector by gradient sign at the WC point (291)–(292); initialize at nominal gradient, monitor gradients instead of including in SQP — saves cost because performance is often "plain" in range parameters (6013–6040).
- 6.3.6–6.3.7 WC statistical vector (293)–(296) (= general WCA); β_W² (297); β_W = (f̄(x_s,0) − f_L)/√(∇fᵀC∇f) (300)/(301); a β change has two parts: margin (numerator) and sensitivity (denominator) (6042–6141).
- 6.3.8 Ȳ_L/U,i = Φ(±β_W) (302); accuracy summary: 1–3% absolute, better at high yield, better when border curvature is weak, suitable beyond 99.9%, best tangent (6143–6185).
- 6.3.9 Table 14 op-amp: gain ≥65 dB (76 dB, β 2.5), f_T ≥30 MHz (67 MHz, β 7.7), PM ≥60° (68°, β 1.8), SR ≥32 V/µs (67, β 6.3), power ≤3.5 µW (2.6 µW, β 1.1), Ȳ = 82.9%; WCDs make heterogeneous margins comparable (6187–6246). Overall yield from intersection of single-plane regions via MC on the linear models at no simulation cost; confirmed within 2% by simulation MC (6247–6254). k = 1…7 SQP steps; complexity ∼ k·n_f·n_xs; ~50 parameters → a whole geometric yield optimization costs about one MC analysis (6257–6274).
- 6.3.10 General WCA (β → f_WC) and geometric yield analysis (f_bound → β) are inverse mappings (303)–(304) (6276–6315).
- 6.3.11 Approximate geometric yield analysis = first step only (sensitivity at nominal) (6317–6325).

### 6.4 Exercise (6327–6361)
- Geometric yield analysis of f = x1x2 (305)–(309) and of f = ¼x1²x2² with f ≤ 1, x0 = 0, C = I (310)–(313) (checking second-order conditions) (6328–6361).

### 7.1 Statistical-Yield Optimization (6385–6778), 7.1.1–7.1.6
- 7.1.1 Truncated pdf_δ = δ·pdf/Y (314); mean x_s,0,δ and covariance C_δ (315)–(316) and MC estimators (317)–(319) (6386–6452).
- 7.1.2 ∇Y(x_s,0) = Y·C⁻¹(x_s,0,δ − x_s,0) (324); optimum when truncated mean = nominal (325): "design centering"; neither performance centering nor max inscribed ellipsoid is a centered design in this sense (6454–6511).
- 7.1.3 ∇²Y = Y·C⁻¹[C_δ + (x_δ − x0)(x_δ − x0)ᵀ − C]C⁻¹ (330); necessary: C_δ − C negative semidefinite (331) (6536–6577).
- 7.1.4 Newton step on quadratic yield model (332)–(335); fixes for indefinite Hessian (gradient only, flip negative eigenvalues, diagonal bias); step length from yield-estimator variance (6579–6617).
- 7.1.5 ∇Y(C) = ½∇²Y(x_s,0) (336) (6619–6626).
- 7.1.6 For deterministic design parameters MC gives no gradient; FD needs n_xd MC runs (337) — prohibitive; marginal-distribution estimator (339)–(342) with per-sample line searches (343)–(344) and gradient (345)–(346) needs ≥ 2n_MC line searches + 2n_MC sensitivity analyses — "may still be prohibitive" (6628–6778).

### 7.2 Geometric-Yield Optimization (6780–7215), 7.2.1–7.2.6
- Increase β if inside, decrease if outside the partition (6781–6785); WCD gradient has the same form and cost for statistical and deterministic parameters (6797–6806).
- 7.2.1 Linear extension in x_d (347)–(348) → β(x_d) (349)–(350); ∇β(x_s,0) = ±∇f(x_s,WC)/σ_f̄ (351)/(353), ∇β(x_d) = ±∇f(x_d)/σ_f̄ (352)/(354); via Lagrange factor ∇β² = λ (355)–(358) (6808–6919).
- 7.2.2 Multi-objective max α_i·β_i (359) with α_i = ±1 (360) (6921–6955). Table 15: after geometric-yield optimization β = 4.2/4.5/3.9/3.9/4.2 (gain/f_T/PM/SR/power), Ȳ 82.9% → 99.9%; gain nominal unchanged (76 dB) but β 2.5 → 4.2 by reducing sensitivity; 3.9σ is a Pareto point between PM and SR (6956–6994).
- 7.2.3 LSQ to β targets (361) (3 for three-sigma, 6 for six-sigma); Gauss–Newton (365), Levenberg–Marquardt trust region (366)–(368); step chosen at the bend of the progress-vs-step Pareto curve, verified by extra simulations (6996–7154). Applies to nominal design with performance targets too (7151–7154).
- 7.2.4 max min α_i·β_i (369) = l∞; maximal inscribed ellipsoid; similar but not equal to center-of-gravity centering (7156–7172).
- 7.2.5 LP (370): max β s.t. β_WC + J(x_d − x_d,µ) ≥ β·1, linearized c ≥ 0; sequence of LPs; any piecewise-linear A_s approximation works; simplicial [38], ellipsoidal methods [1,123] (7174–7205).
- 7.2.6 β derivatives w.r.t. σ, ρ, spec bounds and range bounds exist [122] (7207–7215).

### Appendix A Expectation Values (7216–7369)
- E, moments, mean, variance, covariance, correlation, covariance matrix (A.1)–(A.11) (7222–7300).
- Linear transforms (A.12)–(A.13); translation formula (A.14); **Gaussian error propagation V{aᵀz + b} = aᵀCa = Σa_k²σ_k² when ρ = 0 (A.15)** (7304–7341). Standardization (A.16) (7344–7356). Exercises (7357–7369).

### Appendix B Statistical Estimation (7370–7512)
- Unbiased mean estimator (B.1) and covariance estimator with n−1 (B.5); bias, consistency (B.7)–(B.9); estimator quality (B.10)–(B.11); V{Ê} = V/n (B.14)–(B.15); calculation formulas (B.16)–(B.18); exercises (7376–7512).

### Appendix C Optimality Conditions (7513–7940)
- Unconstrained first/second-order conditions (C.1)–(C.9) with definiteness classes (Fig. C2) (7525–7625).
- Constrained: Lagrangian (C.11), KKT (C.17)–(C.21), second-order on unconstrained stationary directions (C.25)–(C.26) (7696–7875).
- C.6.1 Lagrange factor = sensitivity of the optimum to the constraint bound (C.28) (7877–7894). C.7 proof sketch of (37): |x_k*| = β·σ_k for any correlation (C.29)–(C.36) (7896–7940).

### References and Index (7941–8560)
- Key follow-ups for Philis: [6] Antreich/Graeb/Wieser, WCD-driven analysis and optimization, TCAD 1994 (7965–7967); [58] Graeb/Zizala/Eckmueller/Antreich, "The sizing rules method", ICCAD 2001 (8164–8166); [95] Pelgrom 1989 (8299–8300); [122] Wieser PhD (WCD derivatives) (8409–8410); [38] Director/Hachtel simplicial design centering (8091–8092); [8]/[10] yield-prediction centering (7973–7982).
- Index is a pointer list only (8437–8560).

---

## 3. Actionable extraction

Notation used in recipes: device `i` (FET), its local threshold shift ΔV_i with σ_i = A_VT/√(W·L·m) (Pelgrom; deck `avt_n/p_mv_um`); spec `j` with bound `b_j`; post-layout metric `f_j`; `s_ji = ∂f_j/∂V_i`; net `n`, `g_jn = ∂f_j/∂C_n`.

### GRAEB-01 Three parameter classes for layout evaluation (design / statistical / range)
- Kind: data-model
- Statement: Every simulator input is exactly one of: design parameter x_d (tuned by the tool), statistical parameter x_s (pdf, from manufacturing), range parameter x_r (interval, no pdf; operating conditions, e.g. −40…125 °C). Spec must hold for all x_r in T_r and is scored statistically over x_s.
- Source: §3.1, §3.2; reftext L1522–1573, L1576–1591; PDF p.46–48.
- Philis stage: flow, annotator (config), verify (perf).
- Automation recipe: extend `perf::PerfConfig` with `range: Vec<RangeParam{name, lo, hi, nominal}>` (VDD, TEMP, bias current) and `statistical: {local_vth: bool, global: Option<…>}`. x_d for layout = placement coords/orientation/variant, wire widths/layers, dummies — never varied statistically. Output: explicit parameter vector per evaluation.
- Beats hand layout because: a human checks one corner by eye; the tool evaluates every layout decision against the full declared operating box and process spread.
- Philis status: missing — `OpConfig` carries one `corner: String` (frontend/library/src/oppoint.rs:103–107); `PerfConfig` has only testbench + specs (frontend/library/src/perf.rs:59–69); no range or statistical parameters.

### GRAEB-02 Layout-induced systematic shifts are mean shifts of local statistical parameters
- Kind: data-model / formula
- Statement: Simulator-level transistor parameter = x_d,k + x_s,glob,l + x_s,loc,m (sum of deterministic, global, local parts). Yield changes either by reshaping the acceptance region via x_d or by moving the statistical means x_s,0 (143).
- Source: §3.9 L2334–2337 (PDF p.64); §4.8.5 eq.(143), Fig. 37a, L3742–3759 (PDF p.98).
- Philis stage: annotator, dp, verify.
- Automation recipe: represent each layout systematic effect as ΔV_sys,i added to device i's local Vth mean: gradient ΔV = S_VT·(position − centroid) (deck `svt_uv_per_um`), LOD ΔV = kvth0·Δ(1/(SA+L/2)+1/(SB+L/2)) (already measured per device, `lod_inv_um`), thermal ΔV = TC·ΔT (deck `vt_tc_uv_per_k`). Feed Δf_j,sys = Σ_i s_ji·ΔV_sys,i into the spec's worst-case distance (GRAEB-07). Constraint form: budget term inside β.
- Beats hand layout because: a human applies "keep matched devices close" uniformly; this converts every systematic shift into its actual effect on each spec, device by device.
- Philis status: partial — LOD enters simulation as equivalent SA/SB (frontend/library/src/perf.rs:18–27, 137); gradient and thermal are per-pair ratio checks (kernel/analog/src/placement/matching_pair.rs:17–37, 66–78; backend/annotator/src/emit.rs:94–99), not mapped to spec impact.

### GRAEB-03 Covariance structure: global block + independent local diagonal
- Kind: data-model
- Statement: x_s = [x_glob; x_loc], C = [[C_glob, 0],[0, Σ_loc]], Σ_loc = diag(σ_loc,1 … σ_loc,n) (61); n transistors add n local Vth parameters. Local variation shrinks with gate area. (Source erratum: the book also writes "decrease with … increasing distance" (L2308–2309); the cited Pelgrom [95] model grows with distance — Philis must keep σ² ∝ D² for the gradient term.)
- Source: §3.9 eq.(61); L2297–2333; PDF p.63–64.
- Philis stage: annotator, verify.
- Automation recipe: build Σ_loc from deck A_VT and each device's W·L·m (same computation as `Pelgrom::new`); global part from the model library's statistical section if the deck declares one ("not given" otherwise — do not fabricate). Treat distinct fingers of one device as one local parameter unless the generator splits them into separately placed units.
- Beats hand layout because: gives every device an explicit σ, so budgets follow real device areas rather than a uniform rule of thumb.
- Philis status: partial — σ_rand = A_VT/√(WL) computed (backend/annotator/src/emit.rs:67–90); deck A_VT measured by MC of the installed models (pdks/sky130.json:93–96; benchmarks/characterize_mismatch.py:34); no C matrix assembled; no global parameters.

### GRAEB-04 Specification as per-bound acceptance partitions
- Kind: data-model
- Statement: Spec = box A_f = {f | f_L ≤ f ≤ f_U} (70); split into one partition per bound, A_f,L,i = {f_i ≥ f_L,i}, A_f,U,i = {f_i ≤ f_U,i} (119)–(121); every bound is a separate "performance-specification feature" with its own yield partition and worst-case distance.
- Source: §3.12 eq.(66)–(70) L2433–2471 (PDF p.66–67); §4.8.2 eq.(118)–(125) L3474–3515 (PDF p.91–92).
- Philis stage: flow (perf), verify.
- Automation recipe: treat `Spec{min,max}` as up to two independent features; report and score each separately (β_L,j, β_U,j).
- Beats hand layout because: a two-sided spec (e.g. offset window, UGF window) gets a margin on each side instead of one lumped miss.
- Philis status: partial — `Spec {metric, min, max}` (frontend/library/src/perf.rs:51–56); `miss` sums both sides into one number (perf.rs:88–95).

### GRAEB-05 Spec must hold for every operating condition (∀ x_r ∈ T_r)
- Kind: rule / check
- Statement: Y = prob{∀x_r ∈ T_r : f_L ≤ f(x_d, x_s, x_r) ≤ f_U} (114); A_s,L,i = {x_s | ∀x_r ∈ T_r: f_i ≥ f_L,i} (123). A post-layout pass at one corner is not a pass.
- Source: §4.8.1 eq.(114) L3370–3376 (PDF p.89); §4.8.2 eq.(122)–(124), L3517–3521 (PDF p.92); §6.3.1 eq.(266)–(269) L5779–5812 (PDF p.147–148).
- Philis stage: flow, verify.
- Automation recipe: `score_perf` evaluates each spec at its own worst range corner (GRAEB-26), the spec's metric = worst over those runs. Unmeasured → full miss (existing rule).
- Beats hand layout because: every epoch is checked over the whole operating box; humans usually re-simulate only the final layout at a few corners.
- Philis status: missing — per-epoch perf runs once at the configured corner (frontend/library/src/lib.rs:878–895; oppoint.rs:103–107).

### GRAEB-06 Performance standard deviation by Gaussian error propagation
- Kind: formula
- Statement: For a linear(ized) performance f̄ = f0 + ∇fᵀ(x_s − x_s,0) with x_s ~ N(x_s,0, C): σ²_f̄ = ∇fᵀ·C·∇f (175); with ρ = 0, σ²_f̄ = Σ_k (∂f/∂x_k)²·σ_k² (A.15). Units: those of f.
- Source: §5.2.1 eq.(175) L4205–4211 (PDF p.111); App. A eq.(A.13), (A.15) L7313–7341 (PDF p.183).
- Philis stage: flow (perf), annotator.
- Automation recipe: at the schematic operating point, one simulation per FET with a DC source of +Δ_i in series with its gate (equivalent to a Vth shift, first order) → s_ji = (f_j(+Δ_i) − f_j0)/Δ_i, in parallel like `perf::sensitivities`. σ_f,j = √(Σ_i s_ji²·σ_i²). Cost: n_FET + 1 simulations once per run. Output: σ_f,j per spec and the per-device contribution shares s_ji²σ_i²/σ²_f,j.
- Beats hand layout because: tells, per spec, which devices' mismatch actually matters and by how much — a human guesses from topology.
- Philis status: missing — only ∂f/∂C per net exists (frontend/library/src/perf.rs:189–223).

### GRAEB-07 Signed worst-case distance per spec feature (the layout robustness metric)
- Kind: metric
- Statement: β_W = (f̄(x_s,0) − f_L)/√(∇fᵀC∇f) for a lower bound, (f_U − f̄(x_s,0))/√(∇fᵀC∇f) for an upper bound (300)/(301)/(257); positive if satisfied at nominal, negative if violated (258)/(259); unit = σ of the linearized performance. β_W = 3 → three-sigma, 6 → six-sigma design (4272–4274). A β change has two parts: margin (numerator) and sensitivity (denominator) (6130–6141).
- Source: §5.3 L4266–4281 (PDF p.112); §6.2.4 eq.(257)–(259) L5503–5556 (PDF p.141); §6.3.7 eq.(297)–(301) L6079–6141 (PDF p.154–155).
- Philis stage: flow (lex key), verify, dp/gr (cost).
- Automation recipe: β_L,j = (f_j,post − b_L,j)/σ_f,j, β_U,j = (b_U,j − f_j,post)/σ_f,j, with f_j,post from `score_perf` (parasitics + LOD already simulated) and σ_f,j from GRAEB-06. Where post-layout simulation is too expensive in the inner loop, use f_j,post ≈ f_j0 + Σ_n g_jn·C_n + Σ_i s_ji·ΔV_sys,i (existing budget linearization + GRAEB-02). Report β per bound; score min β.
- Beats hand layout because: puts parasitic loss, systematic mismatch and process spread on one scale (σ), per spec; a human compares dB against MHz by feel.
- Philis status: missing — epoch spec score is Σ normalised miss, 0 once met (frontend/library/src/perf.rs:73–77, 88–95; lib.rs:911–957).

### GRAEB-08 β ↔ yield partition conversion
- Kind: formula
- Statement: Ȳ_L/U,i = Φ(β_W) if satisfied at nominal, Φ(−|β_W|) if violated (302), Φ = standard normal cdf; two-sided Ȳ_i = ∫_{−β}^{β}φ (184). Table 12 / Table 9: β = −1 → 15.9%, 0 → 50.0%, 1 → 84.1%, 2 → 97.7%, 3 → 99.9%; symmetric ±3σ → 99.7%. Independent of the number of parameters and of correlations (single-plane class).
- Source: §5.3 eq.(182)–(184) L4237–4261 (PDF p.112); §6.2.4 Table 12 L5583–5591 (PDF p.143); §6.3.8 eq.(302) L6155–6170 (PDF p.155–156); §6.2.1 Table 9 L5190–5198 (PDF p.134).
- Philis stage: verify (report), flow.
- Automation recipe: report per spec bound: β, Φ(β) as ppm-level partition yield; overall approximate yield per GRAEB-35. Use `erfc` from a tiny local function (no new dependency).
- Beats hand layout because: states the manufacturing consequence of each layout choice in yield terms.
- Philis status: missing.

### GRAEB-09 WCDs make heterogeneous margins comparable → use them in epoch selection
- Kind: metric / rule
- Statement: "it is not possible to judge whether a performance safety margin of 11 dB for the gain is better or worse than … 37 MHz for the transit frequency, [but] a worst-case distance of 2.5 for the gain means less robustness than … 7.7" (Table 14). Nominal gain unchanged (76 dB) while β rose 2.5 → 4.2 by sensitivity reduction (Table 15).
- Source: §6.3.9 L6196–6200, Table 14 L6207–6216 (PDF p.156–157); §7.2.2 Table 15 L6967–6994 (PDF p.174).
- Philis stage: flow (lex key).
- Automation recipe: replace LexKey slot `spec miss` with (a) count of spec features with β < 0 (hard) then (b) −min_j β_j capped at `beta_target` (declared policy; book examples 3 and 6). Epochs with every β ≥ beta_target tie and fall through to Θ, C, area.
- Beats hand layout because: the winner is the most robust layout, not merely one that passes at nominal.
- Philis status: missing — LexKey = (|V|, Σ miss, Θ, C, area) (frontend/library/src/lib.rs:911–957); normalization by the bound's magnitude (perf.rs:88–95) is unit-arbitrary.

### GRAEB-10 Smallest yield partition bounds total yield → bottleneck spec drives effort
- Kind: heuristic
- Statement: Each added spec feature usually costs yield; min_i Y_i ≥ Y, i.e. the smallest partition is an upper bound on overall yield. Design centering must raise the smallest β first.
- Source: §4.8.3 L3609–3613 (PDF p.94); §6.3.9 L6242–6248 (PDF p.158); §7.2.4 L7157–7158 (PDF p.179).
- Philis stage: gp, dp, gr (weights), flow.
- Automation recipe: per epoch, j* = argmin_j β_j; multiply the net weights (`gp` HPWL weights, `PerformanceBudget` rows) of spec j* by a factor that grows as β_j* approaches beta_target; leave others at 1. Report j* as the "limiting spec".
- Beats hand layout because: effort follows the spec that actually limits yield, which a human rarely identifies before signoff.
- Philis status: partial — gp sums positive sensitivities over all specs uniformly (backend/gp/src/lib.rs:120–126); no bottleneck selection.

### GRAEB-11 Signed objective across the feasibility boundary
- Kind: metric
- Statement: max α_i·β_i with α_i = +1 if the nominal is inside the partition, −1 if outside (359)–(360); a violated spec's |β| measures violation and must be decreased until it switches sign, then maximized. Continuous and meaningful on both sides of the bound.
- Source: §7.2.2 eq.(359)–(360) L6921–6948 (PDF p.173); §7.2 L6781–6785 (PDF p.170).
- Philis stage: flow, gr (budget rows).
- Automation recipe: keep budget rows for specs already missed at the schematic, with objective "do not decrease β further" (w_n = g_jn/σ_f,j, allowance 0) instead of dropping them.
- Beats hand layout because: layout never silently worsens a spec that is already short.
- Philis status: missing — a spec with no headroom at the schematic gets no row (frontend/library/src/perf.rs:241–248; lib.rs:218–222).

### GRAEB-12 Scalarization: l∞ (min-max β) for robustness, l2 for gradients, weighted sum only if convex
- Kind: rule
- Statement: A Pareto point is the min of the weighted l∞ distance from the reference point of individual minima (101)–(102); l1 and l∞ are non-differentiable at kinks, l2 is preferred for gradient methods (105)–(107); weighted sum reaches only convex fronts (L3031–3033). Geometric yield optimization: max min α_i β_i (369) = l∞, the largest inscribed tolerance ellipsoid.
- Source: §4.5.2 L2942–2964 (PDF p.78); §4.5.4 L3024–3033 (PDF p.80); §4.6.1 L3100–3124 (PDF p.82); §7.2.4 L7156–7172 (PDF p.179).
- Philis stage: flow, dp (anneal cost).
- Automation recipe: epoch selection uses min β (l∞); the dp annealer, which needs smooth deltas, uses Σ_j softplus(beta_target − β_j)² (l2 on shortfalls) so moves that help any short spec register.
- Beats hand layout because: the scalarization is declared and reproducible.
- Philis status: partial — lexicographic policy declared and cites Graeb ch.1 (frontend/library/src/lib.rs:924–929; kernel/analog/src/placement/utilization.rs:11).

### GRAEB-13 Least-squares to β targets with Levenberg–Marquardt trust region
- Kind: algorithm
- Statement: min_{x_d} ‖β(x_d) − β_target‖₂ (361); linear model β̄(r) = β(x_d,µ) + J·r (362); Gauss–Newton JᵀJr = −Jε (365); trust region ‖r‖ ≤ Δ → (JᵀJ + λI)r = −Jε (368); sweep λ from 0 to ∞, pick the step at the bend of the progress-vs-length curve, verify with simulations (L7076–7081, 7133–7140).
- Source: §7.2.3 eq.(361)–(368) L6996–7154; PDF p.174–178.
- Philis stage: dp (post-anneal refinement), flow.
- Automation recipe: x_d = small continuous layout knobs with known derivatives: device x/y offsets within a symmetry group, wire widths of budgeted nets, dummy/spacing adjustments. J rows = ∇β_j(x_d) from GRAEB-15. Solve the tiny normal equations (n_knobs ≲ 50) with a hand-written Cholesky; accept only if post-layout β confirms (trust-region shrink otherwise).
- Beats hand layout because: moves toward all spec targets at once with a quantified step, instead of trial-and-error edits.
- Philis status: missing.

### GRAEB-14 LP design centering over linearized WCDs
- Kind: algorithm
- Statement: max_{x_d, β} β s.t. β_WC(x_d,µ) + J(x_d − x_d,µ) ≥ β·1 and c(x_d,µ) + ∇c(x_d − x_d,µ) ≥ 0 (370); iterate with re-linearization; any piecewise-linear acceptance approximation fits.
- Source: §7.2.5 eq.(370) L7174–7205; PDF p.179–180.
- Philis stage: dp (legalization / compaction LP).
- Automation recipe: if dp already solves an LP for legalization, add variable β and the rows above with x_d = cell coordinates; legality rows are the c(x_d) ≥ 0 set. Otherwise use it only for finalists.
- Beats hand layout because: legalization that maximizes the minimum spec robustness rather than just removing overlaps.
- Philis status: missing.

### GRAEB-15 WCD gradient w.r.t. layout (design) parameters: ∇β = ∇f/σ_f
- Kind: formula
- Statement: ∇β_W(x_d) = ±∇f(x_d,µ)/√(∇f(x_s,WC)ᵀ·C·∇f(x_s,WC)) (352)/(354) — same form and same cost as for statistical means (351)/(353); via Lagrange factor ∇β² = λ (355)–(358). Sign + when the partition is satisfied at nominal.
- Source: §7.2.1 eq.(347)–(358) L6808–6919; PDF p.170–173.
- Philis stage: gr, dp, gp (weights).
- Automation recipe: net weight in σ units w_jn = g_jn/σ_f,j (aF⁻¹ in units of σ); device-move weight = s_ji·∂ΔV_sys,i/∂position/σ_f,j. These replace `/headroom` normalization so a net's cost is "σ of spec j consumed per aF".
- Beats hand layout because: one consistent currency (σ) for every net and move across all specs.
- Philis status: partial — weights are ∓(∂f/∂C)/headroom (frontend/library/src/perf.rs:225–256; kernel/analog/src/routing/performance.rs:8–27).

### GRAEB-16 Reserve β_target·σ_f of each margin for process variation
- Kind: rule
- Statement: Worst-case value f_WL = f0 − β_W·σ_f̄, f_WU = f0 + β_W·σ_f̄ (177)/(179); so the layout-spendable margin of a lower-bound spec is (f0 − f_L) − β_target·σ_f, not f0 − f_L. (Derived: book gives the formula; the budget split is Philis's application.)
- Source: §5.2.2 eq.(176)–(179) L4213–4224 (PDF p.111); §6.3.7 L6130–6141 (PDF p.155).
- Philis stage: flow (perf budget rows), gr.
- Automation recipe: in `budget_rows`, headroom_stat = headroom − beta_target·σ_f,j; if ≤ 0 emit a row with allowance 0 (GRAEB-11) and report "statistically infeasible at schematic".
- Beats hand layout because: the router cannot spend margin the process needs; hand layout routinely consumes it unknowingly.
- Philis status: missing — headroom = f0 − lo or hi − f0 (frontend/library/src/perf.rs:241–245).

### GRAEB-17 Realistic worst-case parameter vector and one verifying simulation per spec bound
- Kind: algorithm
- Statement: x_s,WL − x_s,0 = −(β_W/σ_f̄)·C·∇f (171)–(172), x_s,WU − x_s,0 = +(β_W/σ_f̄)·C·∇f (173)–(174); worst-case vectors may coincide or cluster across specs, which reduces their number (L3271–3275).
- Source: §5.2.1 L4189–4204 (PDF p.111); §4.7.1 L3268–3275 (PDF p.87).
- Philis stage: verify (finalists), flow.
- Automation recipe: for each spec bound and the winning epoch: ΔV_i = ∓β_target·σ_i²·s_ji/σ_f,j; simulate the extracted circuit with these gate offsets; spec passes robustly if f_j(x_WC) meets the bound. Deduplicate vectors with cosine > 0.99 across specs. Cost: ≤ 2·n_spec simulations.
- Beats hand layout because: a targeted "3σ mismatch corner" per spec, built from the actual layout, replaces generic corners.
- Philis status: missing.

### GRAEB-18 General (nonlinear) WCA by SQP; realistic WCA is its first step
- Kind: algorithm
- Statement: min/max f(x_s) s.t. (x_s − x_s,0)ᵀC⁻¹(x_s − x_s,0) ≤ β_W² (186)–(187), solved by SQP; the first iteration from nominal sensitivities equals the realistic WCA (L4712–4716, L6317–6325); k = 1…7 iterations, complexity ∼ k·n_f·n_xs; ~50 parameters → full geometric-yield optimization ≈ cost of one MC analysis; accuracy 1–3% absolute yield.
- Source: §5.4 L4284–4373 (PDF p.113–115); §5.5.2 L4704–4716 (PDF p.123); §6.3.9 L6257–6274 (PDF p.159); §6.3.11 L6317–6325 (PDF p.160–161).
- Philis stage: verify (finalists).
- Automation recipe: iterate GRAEB-17: re-linearize at x_WC (new s_ji at that point, n_FET sims), recompute x_WC, stop when |Δβ| < 0.05 or 7 iterations. Use only on the final 1–3 epochs.
- Beats hand layout because: nonlinear worst cases (e.g. a device leaving saturation at a 3σ shift) are found, not missed by linear extrapolation.
- Philis status: missing.

### GRAEB-19 Mismatch-sensitive specs have two opposite worst-case vectors per pair
- Kind: check / algorithm
- Statement: Mismatch-sensitive performances have semidefinite second derivatives: nominal on a ridge, constant for equal changes of two local parameters, deteriorating otherwise; "very regularly … two worst-case parameter vectors in 'opposite' directions" per mismatch-producing pair; solvers must handle it; additional local minima exist (L6003–6010).
- Source: §5.4 L4358–4368 (PDF p.114–115); §6.3.1 L5772–5778 (PDF p.147); §6.3.4 L6003–6010 (PDF p.152).
- Philis stage: verify, annotator.
- Automation recipe: for specs whose s_j has pairs with s_ji ≈ −s_jk (GRAEB-20), evaluate the WC search from both ±(e_i − e_k) starting directions and keep the worse. Layout systematic offset ΔV_sys,i − ΔV_sys,k shifts the ridge: it lowers β on one side and raises it on the other — score min(β_L, β_U) so a layout-induced offset is never averaged away.
- Beats hand layout because: catches the side on which the layout's own systematic offset adds to the random one.
- Philis status: missing.

### GRAEB-20 Sensitivity-based extraction of matching constraints (parameter similarity)
- Kind: algorithm
- Statement: Similarity of parameters cos φ(x_k, x_l) = ∇f(x_k)ᵀ∇f(x_l)/(‖∇f(x_k)‖·‖∇f(x_l)‖) (74); ±1 = inseparable effect, 0 = independent. Combined with GRAEB-19: cos ≈ −1 with equal magnitudes ⇒ the specs depend on the *difference* of the two parameters (a mismatch-producing pair).
- Source: §4.1.2 eq.(74) L2590–2599 (PDF p.70); §5.4 L4360–4367 (PDF p.115).
- Philis stage: annotator (constraint extraction).
- Automation recipe: columns v_i = [s_1i σ_i/σ_f,1, …, s_Ji σ_i/σ_f,J] (scaled, GRAEB-25). For same-type FET pairs (i,k) with cos(v_i, v_k) < −0.9 and ‖v_i‖/‖v_k‖ ∈ [0.8, 1.25] (Philis thresholds, not from the book) emit a MatchingPair with priority ‖v_i‖ (fraction of σ²_f explained); pairs found by the pattern catalog but with ‖v‖ ≈ 0 get relaxed (low weight). Output: extracted pairs with a quantitative weight.
- Beats hand layout because: finds every pair the circuit is actually sensitive to (including across blocks and ratioed mirrors) and ranks them; a pattern catalog or a human finds only named structures.
- Philis status: missing — matching comes from structural patterns only (backend/annotator/src/catalog.rs; pattern.rs:24–29) and `is_diff_pair` topology (kernel/analog/src/placement/matching_pair.rs:82–95).

### GRAEB-21 Parameter significance ranks nets/devices for placement and routing priority
- Kind: heuristic
- Statement: x_k more significant than x_l ⇔ ‖∇f(x_k)‖ > ‖∇f(x_l)‖ for equal Δx (76); requires scaled variables.
- Source: §4.1.3 eq.(75)–(76) L2601–2611; PDF p.70–71.
- Philis stage: gp, gr (net order), dr.
- Automation recipe: significance of net n = ‖[g_1n/σ_f,1, …, g_Jn/σ_f,J]‖₂ (σ-scaled); route nets in descending significance, give the top decile shielding/short-path priority; device significance = ‖[s_ji σ_i/σ_f,j]_j‖ for placement centrality.
- Beats hand layout because: the ordering is computed from the circuit, not from net names.
- Philis status: partial — per-net weights from positive sensitivities summed over specs, mean-normalized (backend/gp/src/lib.rs:120–126); not σ-scaled; router net order not checked here.

### GRAEB-22 Performance similarity detects conflicting specs over the same nets
- Kind: heuristic
- Statement: cos φ(f_i, f_j) = ∇f_iᵀ∇f_j/(‖∇f_i‖‖∇f_j‖) (73); ≈ −1: one parameter change helps one spec and hurts the other; multiple-objective side effect Δf|Δf_i,max = S·∇f_i (80).
- Source: §4.1.1 eq.(73) L2566–2588 (PDF p.70); §4.1.5 eq.(80) L2628–2637 (PDF p.71).
- Philis stage: flow (report), gr.
- Automation recipe: compute cos over the net-capacitance gradients g_j·; report spec pairs with cos < −0.5 as "trade-off specs"; keep signed weights in budgets (a net that helps one spec must not be minimized blindly).
- Beats hand layout because: exposes where adding C on a net is a real trade rather than pure loss.
- Philis status: partial — signs kept in PerformanceBudget (kernel/analog/src/routing/performance.rs:15–16); no similarity report.

### GRAEB-23 Sensitivity computation: cost and finite-difference step
- Kind: rule
- Statement: simulator-internal sensitivities cost ~10% of one simulation per parameter; finite differences cost one full simulation (100%) per parameter (83); Δx_k must be "large enough to surmount numerical noise and small enough to compare to the gradient".
- Source: §4.2.1–4.2.2 L2663–2704; PDF p.72–73.
- Philis stage: flow (perf).
- Automation recipe: per net, step = max(10% of the net's schematic node C, solver-noise floor); per device, step = σ_i. Use central differences (±Δ, one extra run) on the top-significance nets to check linearity; if forward and backward slopes differ > 20%, halve the step once. Parallel runs as today.
- Beats hand layout because: sensitivities that drive every budget are numerically trustworthy.
- Philis status: partial — forward difference with a fixed 10 fF for every net (frontend/library/src/lib.rs:215–216; perf.rs:200–222).

### GRAEB-24 Scaling: whiten statistical parameters, equalize sensitivities
- Kind: rule
- Statement: scale by reference point (85) or covered range to [0,1] (87); whitening x' = A⁻¹(x − x_s,0) makes parameters independent, "most probably" in [−3, 3], condition number 1 (88); normalizing rows/columns of S to unit length (90)–(91) equalizes visibility; "Scaled variables have to be used in an implementation."
- Source: §4.3 L2706–2806; PDF p.73–75.
- Philis stage: flow, annotator, dp.
- Automation recipe: compute all β work in whitened coordinates (divide ΔV_i by σ_i); normalize each net-weight row by its norm before summing across specs (fixes the "summed over specs" mixing of units in gp).
- Beats hand layout because: no single spec or large net dominates the optimizer by units alone.
- Philis status: partial — rule residuals normalized by budget (`over`, kernel/analog/src/rule.rs:308–318); gp sums raw 1/aF across specs (backend/gp/src/lib.rs:121–126).

### GRAEB-25 Classical worst case for range parameters: corner by gradient sign
- Kind: algorithm
- Statement: For box range parameters and a linear model, x_r,WL,k = x_r,L,k if ∂f/∂x_r,k > 0, x_r,U,k if < 0, undefined (stay nominal) if 0 (158); upper WC reversed (159); f_WL/U = f0 + ∇fᵀ(x_WL/U − x_r,0) (160). Table 6: classical WCA suits range (operating) parameters and unknown distributions.
- Source: §5.1 eq.(148)–(160) L3929–4050 (PDF p.104–107); Table 6 L3903–3911 (PDF p.103).
- Philis stage: flow (perf).
- Automation recipe: once per run at the schematic: n_r + 1 simulations (nominal + one per range parameter) give ∂f_j/∂x_r; per spec bound pick the corner by sign; per epoch simulate only the distinct corners (≤ 2·n_spec, usually far fewer than 2^{n_r}); the spec's post-layout value = its own worst-corner result.
- Beats hand layout because: exhaustive operating-box coverage at near-nominal cost, every epoch.
- Philis status: missing.

### GRAEB-26 Range-parameter worst case: initialize at nominal, monitor gradients
- Kind: heuristic
- Statement: The worst-case range vector at the statistical WC point uses the gradient there (291)–(292); initialize from nominal gradients, keep it out of the SQP, update only when monitored gradient signs flip — saves cost because performance is often "plain" in range parameters.
- Source: §6.3.5 L6013–6040; PDF p.153.
- Philis stage: flow.
- Automation recipe: cache the per-spec corner choice from GRAEB-25; recheck the sign of ∂f/∂x_r only for finalists (one extra run per range parameter) and re-select corners if a sign flipped.
- Beats hand layout because: corner coverage stays correct without per-epoch corner sweeps.
- Philis status: missing.

### GRAEB-27 Corner (box) worst case exaggerates robustness — never stack per-device ±kσ corners
- Kind: rule
- Statement: β_W,B ≥ β_W (263): a ±3σ box corner corresponds to β_W,B = 4.2σ_f (n = 2, ρ = 0) up to 19.0σ_f (n = 4, ρ = 0.9) (Table 13), "excessive"; the box ignores correlations (Fig. 22d, (37)). Realistic (ellipsoid/single-plane) worst case is appropriate for normally distributed IC parameters.
- Source: §6.2.5 L5645–5698, Table 13 L5660–5669 (PDF p.144–145); §5.1.4 L4083–4092 (PDF p.108); Table 6 L3903–3911.
- Philis stage: annotator (budgets), verify.
- Automation recipe: combine independent mismatch contributions of several devices in root-sum-square through σ_f (GRAEB-06), never as Σ|kσ_i|; treat foundry ss/ff corners as range-type corners (GRAEB-25), not as statistical 3σ statements.
- Beats hand layout because: avoids both over-design (wasted area from stacked corners) and under-design.
- Philis status: partial — matching budgets are per pair ratios (kernel/analog/src/placement/matching_pair.rs:17–25); multi-member groups use the tightest member (backend/annotator/src/emit.rs:217–222); no stacked-corner logic found.

### GRAEB-28 Tolerance class choice: per-spec single-plane budgets, not "all devices within kσ"
- Kind: rule
- Statement: Box yield depends on n and ρ (Table 10: ±3σ, n = 3…10: 99.1…97.0% at ρ = 0); ellipsoid yield collapses with n (Table 11: β = 3 gives 98.9% at n = 2, 46.8% at n = 10, 12.3% at n = 15); single-plane (per spec bound) yield is independent of n, variances and correlations (Table 12) and is the geometrically optimal per-spec approximation.
- Source: §6.2.2 Table 10 L5272–5315 (PDF p.135–136); §6.2.3 Table 11 L5369–5423 (PDF p.138–139); §6.2.4 L5574–5643 (PDF p.141–144).
- Philis stage: annotator, flow.
- Automation recipe: any requirement "every matched device within kσ" must be re-expressed per spec bound through σ_f; tolerance targets are β per spec bound.
- Beats hand layout because: budgets do not silently change meaning as the circuit grows.
- Philis status: missing (no σ-based targets yet); flagged as a design rule for GRAEB-07/16.

### GRAEB-29 Bounding box of ellipsoids: conservative screen when correlations are unknown
- Kind: check
- Statement: For fixed β, the union over all correlations of the β-ellipsoids is exactly the box |x_k − x_0,k| ≤ β·σ_k (37); a tolerance box is the worst case if correlations are unknown (L5307–5309); proof via max|x_k| s.t. xᵀC⁻¹x = β² ⇒ |x_k*| = βσ_k (C.29)–(C.36).
- Source: §3.6 eq.(37) L2026–2042 (PDF p.58); §6.2.2 L5307–5315 (PDF p.136); App. C.7 L7896–7940 (PDF p.196–197).
- Philis stage: verify, annotator.
- Automation recipe: when spatial correlation between devices' local parameters is unknown (e.g. S_VT is a proxy, pdks/sky130.json:97–98), bound the gradient-term contribution by the box (worst correlation) and mark the check "conservative"; when the deck supplies correlation, use the ellipsoid.
- Beats hand layout because: a guaranteed-safe bound is used exactly where data is missing, and labelled as such.
- Philis status: partial — unknown gradient coefficient makes the check "unknown" rather than conservative (kernel/analog/src/placement/matching_pair.rs:22–24, 72–78).

### GRAEB-30 Monte-Carlo yield estimator and its variance
- Kind: algorithm
- Statement: Ŷ = n_ok/n_MC (219); partitions Ŷ_L/U,i = n_ok,L/U,i/n_MC (220); σ²_Ŷ = Y(1−Y)/n_MC (225), estimator Ŷ(1−Ŷ)/(n_MC − 1) (226); max variance at Y = 50%; accuracy ∼ √n_MC, cost ∼ n_MC, independent of #parameters and nonlinearity.
- Source: §6.1 eq.(219)–(229) L4771–5035 (PDF p.125–130); L5141–5149 (PDF p.133).
- Philis stage: verify (final signoff), benchmarks.
- Automation recipe: optional final step: run the extracted winner with the model library's mismatch section (sky130 `tt_mm`, `MC_MM_SWITCH=1`, as used by benchmarks/characterize_mismatch.py:34) for n_MC seeds; report Ŷ, per-bound partitions and σ_Ŷ; flag disagreement with Φ(β) beyond 3%.
- Beats hand layout because: a statistical certificate of the delivered layout, not a claim.
- Philis status: missing in flow — MC exists only for deck characterization (benchmarks/characterize_mismatch.py:74–103).

### GRAEB-31 Monte-Carlo sample size and confidence
- Kind: rule / deck-requirement
- Statement: n_MC ≈ Y(1−Y)·k_γ²/ΔY² (232); k_γ = 1.645 (90%), 1.960 (95%), 2.576 (99%), 3.291 (99.9%); Table 8 (Y = 85%): ±10% needs 35/49/85/139, ±5% 139/196/339/553, ±1% 3,451/4,899/8,461/13,810 samples; Table 7 (Y = 85%): n = 1000 → σ_Y = 1.1%. Normal approximation valid when #pass > 4, #fail > 4, n ≥ 10. Accuracy ×F costs ×F² samples.
- Source: §6.1.3 L5036–5133, Tables 7–8 L5051–5106; PDF p.130–132.
- Philis stage: verify, flow (config).
- Automation recipe: `mc: {confidence, half_width}` config → n_MC from (232) using the WCD-predicted yield as prior Y; refuse to report a yield whose sample violates the >4/>4/≥10 condition (report a one-sided bound instead).
- Beats hand layout because: statistically sized signoff instead of an arbitrary "100 runs".
- Philis status: missing.

### GRAEB-32 Why MC cannot drive the layout loop: prefer WCD in the loop
- Kind: rule
- Statement: MC for ±1% at 99% confidence needs thousands of simulations (Table 8); geometric yield analysis needs k·n_f·n_xs sensitivity runs (k = 1…7) and for ~50 parameters a whole yield optimization costs about one MC analysis; statistical yield gradients w.r.t. deterministic design parameters cost n_xd MC runs (337) or ≥ 2n_MC line searches + 2n_MC sensitivity analyses (346) — "prohibitive"; WCD gradients are cheap and identical in form for design parameters.
- Source: §6.1.3 L5126–5138 (PDF p.132); §6.3.9 L6257–6274 (PDF p.159); §7.1.6 L6628–6778 (PDF p.167–170); §7.2 L6797–6806 (PDF p.170).
- Philis stage: flow.
- Automation recipe: in-loop metric = WCD (GRAEB-07) with schematic σ_f; MC only for final signoff (GRAEB-30).
- Beats hand layout because: statistical robustness is optimized during layout at affordable cost.
- Philis status: missing (neither).

### GRAEB-33 Cheap joint-yield estimate: MC on the linearized acceptance region
- Kind: algorithm
- Statement: The intersection of the single-plane-bounded regions of all spec bounds approximates A_s (Fig. 55); "A Monte-Carlo analysis using the approximate parameter acceptance region … can be performed at no additional simulation cost"; confirmed within 2% by simulation MC (Table 14 example).
- Source: §6.3.9 L6247–6254, Fig. 55 L6228–6233; PDF p.158.
- Philis stage: verify (report), flow.
- Automation recipe: with whitened parameters t ~ N(0, I) (GRAEB-34), sample 10⁵ vectors in-process; a sample passes if for every bound j: sign_j·(f_j,post + Σ_i s_ji σ_i t_i − b_j) ≥ 0. Ŷ_joint costs microseconds; report it beside min β.
- Beats hand layout because: joint yield (correlated specs) per epoch at zero simulator cost.
- Philis status: missing.

### GRAEB-34 Sample generation for statistical parameters
- Kind: algorithm
- Statement: z_k ~ U(0,1) → y_k = cdf⁻¹_N(z_k) → x_s = A·y + x_s,0 with C = AAᵀ (56)–(58); Cholesky or eigen A = VΛ^½ (59)–(60); eigen-decomposition preferred when C is ill-conditioned by highly correlated parameters.
- Source: §3.8 L2224–2291; PDF p.62–63.
- Philis stage: verify.
- Automation recipe: for the diagonal local Σ_loc, A = diag(σ_i) (no factorization); only a correlated global block needs Cholesky. Use the existing seeded RNG; Box–Muller or inverse-cdf for normals.
- Beats hand layout because: reproducible statistical checks (seeded).
- Philis status: missing.

### GRAEB-35 Importance sampling for high-yield estimation
- Kind: algorithm
- Statement: Y = E_{pdf_MC}{δ·pdf/pdf_MC} (223), Ŷ = (1/n)Σ δ(x^µ)·pdf(x^µ)/pdf_MC(x^µ) (224); a sampling distribution spreading samples ~50/50 across the acceptance border improves the estimate.
- Source: §6.1.2 L4912–4964; PDF p.128–129.
- Philis stage: verify.
- Automation recipe: for finalists with β ≥ 3 (few MC failures), shift the sampling mean to the WC point x_WC (GRAEB-17) and reweight by the likelihood ratio; report Ŷ with its estimator variance.
- Beats hand layout because: ppm-level yield evidence without millions of simulations.
- Philis status: missing.

### GRAEB-36 Yield saturates above 99.9% — rank by β (or inflate tolerances)
- Kind: heuristic
- Statement: Yield is a very weak optimum above 99.9%; optimization terminates prematurely; remedy: when Y > 99%, set C := a·C (a > 1) and continue (145), or max det(a·C) s.t. Y = 50% (147); the geometric (WCD) approach has no weak optimum.
- Source: §4.8.7 L3807–3877; PDF p.100–102.
- Philis stage: flow (lex key).
- Automation recipe: never rank epochs by MC pass rate; rank by β (GRAEB-09). If a pass-rate score is ever used, evaluate it at inflated σ (a chosen so the best epoch's pass rate < 99%).
- Beats hand layout because: keeps improving robustness past the point where "all MC runs pass".
- Philis status: n/a (no yield score yet); design rule for GRAEB-09.

### GRAEB-37 Statistical yield gradient from one MC: attribute yield loss to layout mean shifts
- Kind: algorithm
- Statement: ∇Y(x_s,0) = Y·C⁻¹·(x_s,0,δ − x_s,0) (324), x_s,0,δ = mean of passing samples (317); optimum when x_s,0,δ = x_s,0 (325) ("center of gravity"); Hessian (330); necessary C_δ − C negative semidefinite (331).
- Source: §7.1.1–7.1.3 L6386–6577; PDF p.162–166.
- Philis stage: verify (report), annotator (feedback).
- Automation recipe: from the signoff MC (GRAEB-30) compute ∇Y per device local Vth; predicted yield change from layout systematics ΔY ≈ ∇Yᵀ·ΔV_sys (GRAEB-02). Report devices with the largest |∇Y_i·ΔV_sys,i| as "layout-induced yield loss" and feed them back as tighter matching/LOD constraints on the next run.
- Beats hand layout because: a quantitative attribution of yield loss to specific layout-induced shifts.
- Philis status: missing.

### GRAEB-38 Tolerance assignment → allocate the layout's mismatch share per pair
- Kind: algorithm
- Statement: Tolerance assignment tunes variances/correlations (and range/spec bounds) for yield with det C = const (144), or maximizes tolerance volume at fixed yield (146); goal "select the largest possible tolerance intervals without affecting the yield in order to save production costs" (L3798–3801); ∇Y(C) = ½∇²Y (336); WCD derivatives w.r.t. σ, ρ and bounds exist [122] (L7207–7215).
- Source: §4.8.6 L3774–3805 (PDF p.99–100); §7.1.5 L6619–6626 (PDF p.167); §7.2.6 L7207–7215 (PDF p.180).
- Philis stage: annotator (budget generation).
- Automation recipe: per device i, the layout's allowed systematic shift is a "tolerance". Choose allowances a_i (mV) maximizing Σ_i a_i (loosest layout constraints → less area, more freedom) s.t. for every spec bound Σ_i |s_ji|·a_i ≤ (β_schem,j − β_target)·σ_f,j (a small LP; greedy by |s_ji| is adequate). Convert a_i into MatchingPair D_max = a/S_VT, CommonNode ΔR_max = a/I_D, LOD/thermal limits.
- Beats hand layout because: tight constraints only where the circuit is sensitive, loose elsewhere; a flat η over-constrains insensitive pairs and under-constrains critical ones.
- Philis status: partial — flat η = 0.3 (backend/annotator/src/emit.rs:46–52) or derived from a global `offset_sigma_mv` (backend/annotator/src/netrole.rs:74; emit.rs:58–65); CommonNode ΔR from the same allowance (frontend/library/src/lib.rs:786–797).

### GRAEB-39 Nested-loop simplifications and sensitivity refresh cadence
- Kind: heuristic
- Statement: Worst-case/yield optimization nest four loops (optimization → WCA/yield analysis → sensitivity → simulation); simplifications: analysis once per step or every µth/kth step; sensitivity once per WCA step or once per WCA; nominal design first because tolerance design multiplies simulations; the transition nominal → worst-case is smooth if nominal design includes linearized worst-case assumptions.
- Source: §4.7.2 L3316–3361, Fig. 30 (PDF p.88–89); §4.8.5 L3697–3715, Fig. 35 (PDF p.97); §1.4 L1185–1190 (PDF p.37).
- Philis stage: flow.
- Automation recipe: schematic s_ji, g_jn once per run (as now); recompute g_jn and s_ji around the extracted incumbent every µ epochs (µ = 4, Philis choice) or when |predicted − simulated| Δf_j > 30% of σ_f,j; full WC/SQP only for finalists.
- Beats hand layout because: the linear models steering the loop stay calibrated to the layout actually produced.
- Philis status: partial — sensitivities computed once from the schematic (frontend/library/src/lib.rs:196–232); never refreshed.

### GRAEB-40 Sizing-rule constraints c(x_d) ≥ 0: DC feasibility of recognized building blocks
- Kind: rule
- Statement: Nominal design's constraints c(x_d) ≥ 0 "basically describe technological and structural requirements concerning DC properties of transistors that have to be fulfilled for a proper function and robustness [58]"; they "determine the achievable performance feature values of a given circuit structure". Specific rules and thresholds are **not given** in this book (see [58]).
- Source: §4.5 L2831–2834 (PDF p.76); §4.6 L3072–3075 (PDF p.81); ref [58] L8164–8166.
- Philis stage: annotator, flow (oppoint), gr/dr (IR budgets).
- Automation recipe: per recognized block (catalog match), attach DC constraints evaluated at the operating point: every device in a mirror/pair/cascode saturated (V_DS − V_DSsat ≥ margin), matched devices at similar V_DS, overdrive above a floor (Hastings row 3: V_ov ≥ 100 mV, docs/LAYOUT-FUNDAMENTALS.md:55). Layout-induced IR drop and ΔV_DS consume these margins → the block's constraint becomes an IR/ΔV budget on the nets feeding it; failing constraints at the schematic are reported as "structure infeasible" (layout cannot fix). Thresholds come from [58] or the user, not this book.
- Beats hand layout because: block-level electrical preconditions are checked for every instance and every layout, with the IR spend accounted.
- Philis status: partial — structural pattern + size-match recognition (backend/annotator/src/pattern.rs:24–29; catalog.rs:279–281 same-L cascode); saturation headroom per device/net feeds IR budgets (frontend/library/src/oppoint.rs:19–22, 177–187; lib.rs:283); no per-block DC rule set.

### GRAEB-41 Performance targets: interval center for two-sided specs; update targets during optimization
- Kind: rule
- Statement: With both bounds, target = ½(f_L + f_U) (108) (maximum margin); with one bound, target from experience and the sensitivity matrix, updated during optimization; single-objective form min ‖f_I − f_I,target‖ s.t. spec, c ≥ 0 (104). Nonlinearity makes performance-centered ≠ parameter-centered (RC example: R = C ≈ 1.118 vs 1.061) and yield-centered ≠ performance-centered (58.76% → 74.50%).
- Source: §4.6 L3066–3087 (PDF p.81); §4.6.2 eq.(108) L3126–3143 (PDF p.82–83); §2.4–2.5 L1251–1318, L1379–1497 (PDF p.39–44).
- Philis stage: flow (perf).
- Automation recipe: for a two-sided spec, score in β terms per side (GRAEB-04/07), which centers statistically; where β is unavailable, use distance from the interval center normalized by half-width.
- Beats hand layout because: layout does not push a windowed metric to one edge while "passing".
- Philis status: partial — two-sided `Spec` supported, scored only when violated (frontend/library/src/perf.rs:88–95).

### GRAEB-42 Pareto archive of feasible epochs
- Kind: data-model
- Statement: f > f* / f < f* order (94)–(95); Pareto optimal ⇔ no superior vector (100); yield optimization selects one point of the Pareto front determined by spec and tolerances (Fig. 34, L3672–3687); 3.9σ in Table 15 is "a Pareto point concerning the worst-case distances" of PM and SR (L6984–6990).
- Source: §4.5.1–4.5.3 L2863–3022 (PDF p.76–80); §4.8.5 L3672–3687 (PDF p.95); §7.2.2 L6983–6990 (PDF p.174).
- Philis stage: flow.
- Automation recipe: keep a bounded archive of feasible epochs non-dominated in (min β, per-spec β vector, area, extracted C); the lex winner is reported with the archive so the user can pick another trade-off.
- Beats hand layout because: shows the real layout trade-offs instead of one hand-picked compromise.
- Philis status: missing — single incumbent via `key_lt` (frontend/library/src/lib.rs:929–939; multi-start reduce lib.rs:189).

### GRAEB-43 Guaranteed ("datasheet") values per spec after layout
- Kind: metric
- Statement: f_WL = f0 − β_W·σ_f̄, f_WU = f0 + β_W·σ_f̄ (177)/(179); yield also depends on the spec bounds, and choosing "the best possible performance specification that can be guaranteed with a certain yield" is a valid optimization target (L3802–3805); general WCA maps β → f_WC, geometric yield analysis maps f bound → β (303)–(304).
- Source: §5.2.2 L4213–4224 (PDF p.111); §4.8.6 L3802–3805 (PDF p.100); §6.3.10 L6276–6315 (PDF p.159–160).
- Philis stage: verify (report).
- Automation recipe: report per spec: nominal post-layout value, σ_f, β, and f at β_target (e.g. "UGF ≥ x MHz at 3σ"); diff against the schematic's same numbers to show what layout cost.
- Beats hand layout because: the layout's statistical cost per spec is stated in the spec's own units.
- Philis status: missing.

### GRAEB-44 Accuracy of the WCD yield approximation and its validation
- Kind: check
- Statement: Error comes from replacing the curved acceptance border by the tangent at the WC point; exact at the WC point where the pdf is largest on the error region; typically 1–3% absolute yield; smaller at higher yield, negligible above 99.9%; better when the border is less curved than the equidensity contours; duality: the WC-point tangent is the greatest lower / least upper bound among tangents; two-sided Ȳ_i by (184) not identical to the region of Fig. 44c unless gradients are parallel.
- Source: §5.5.1 L4546–4702 (PDF p.118–123); §6.3.8 L6171–6185 (PDF p.156).
- Philis stage: verify, benchmarks.
- Automation recipe: benchmark check on fixtures (ota, bgr_core): compare Φ(β) (linear, GRAEB-07), SQP β (GRAEB-18) and MC Ŷ (GRAEB-30); assert |Φ(β) − Ŷ| ≤ 3% + 3σ_Ŷ; otherwise flag the spec as "nonlinear — use SQP β".
- Beats hand layout because: the robustness metric itself is validated per circuit.
- Philis status: missing.

### GRAEB-45 Multiple local worst-case points; second-order check
- Kind: check
- Statement: Stationary points of the geometric yield problem need the second-order condition: tolerance-ellipsoid curvature must exceed the acceptance-border curvature (290); other stationary points and local minima exist (Fig. 53); a deterministic optimizer from x_s,0 usually finds the global one, but "there is no guaranty" and "suitable measures have to be taken", notably for mismatch-producing parameters.
- Source: §6.3.4 L5950–6010; PDF p.151–152.
- Philis stage: verify.
- Automation recipe: start the WC search (GRAEB-18) from the realistic WC point and from its mirror for each mismatch pair (GRAEB-19); keep the smallest β; record how many distinct WC points were found.
- Beats hand layout because: no hidden second failure mode on the other side of a pair.
- Philis status: missing.

### GRAEB-46 Transform non-normal parameters before WCD/MC
- Kind: data-model
- Statement: Parameters bounded below (e.g. oxide thickness) are skewed; transform to normal (39)–(41); lognormal (46): ln z normal, suitable for strictly positive quantities; inverse-cdf mapping (43).
- Source: §3.7 L2059–2222; PDF p.58–61.
- Philis stage: verify.
- Automation recipe: if Philis ever treats parasitic spread (e.g. sheet resistance, via resistance) statistically, model it as lognormal and run WCD in ln-space; for Vth use normal (deck A_VT fit already assumes σ ∝ 1/√WL).
- Beats hand layout because: statistical treatment of strictly positive parasitics without negative-resistance artifacts.
- Philis status: n/a — no statistical parasitics yet.

### GRAEB-47 Fail is never pass; four outcomes of a check
- Kind: rule
- Statement: Verification against the specification may happen "after structural synthesis prior to the layout design phase" and after production test (L2428–2431); a circuit is "in full working order" only if all spec features hold (L2443–2445). (The book states the acceptance definition; the "unknown never passes" policy is Philis's application.)
- Source: §3.12 L2424–2446; PDF p.66–67.
- Philis stage: flow, verify.
- Automation recipe: keep: a spec feature whose β cannot be computed (missing A_VT, failed simulation) counts as β = −∞ for feasibility and is reported "unknown", never as satisfied.
- Beats hand layout because: missing evidence never passes silently.
- Philis status: implemented for rules and perf — FAIL_FLOOR and default residual (kernel/analog/src/rule.rs:138–143, 292–306); unmeasured metric = full miss (frontend/library/src/perf.rs:73–77, 91–92).

---

## 4. Top-15 priorities for Philis

1. **Per-spec σ_f from Vth sensitivities + deck A_VT** — one-time n_FET+1 runs at the schematic; the foundation of every statistical item (GRAEB-06, GRAEB-03).
2. **Signed worst-case distance per spec bound, post-layout** — one σ-based robustness number per spec using the already-simulated post-layout metrics (GRAEB-07, GRAEB-04, GRAEB-08).
3. **Epoch selection on min β (capped at β_target) instead of Σ normalised miss** — rewards margin, fixes unit-arbitrary normalization (GRAEB-09, GRAEB-11, GRAEB-12, GRAEB-36).
4. **Reserve β_target·σ_f before handing headroom to the router; weights in σ units** — stops layout from spending the process margin (GRAEB-16, GRAEB-15, GRAEB-11).
5. **Map layout systematic shifts (gradient, LOD, thermal) to spec impact via s_ji** — replaces per-pair heuristics with spec-level effect (GRAEB-02).
6. **Sensitivity-derived allocation of the layout's mismatch allowance (replace flat η = 0.3)** — tight where sensitive, loose elsewhere; less area (GRAEB-38).
7. **Sensitivity-based matched-pair extraction (parameter similarity ≈ −1)** — constraint extraction beyond the pattern catalog, with quantitative priority (GRAEB-20, GRAEB-19).
8. **Range-parameter worst-case corners by gradient sign, per spec** — operating-box coverage every epoch at near-nominal cost (GRAEB-25, GRAEB-05, GRAEB-26).
9. **Bottleneck-spec weighting in gp/gr** — effort goes to the spec limiting yield (GRAEB-10, GRAEB-21).
10. **Finite-difference step scaling and linearity check** — trustworthy sensitivities under all budgets (GRAEB-23, GRAEB-24).
11. **Sensitivity refresh around the extracted incumbent every µ epochs / on prediction error** — keeps the linear guides calibrated (GRAEB-39).
12. **Finalist verification at the worst-case vector (both signs for mismatch pairs), SQP if nonlinear** — catches nonlinear and one-sided failures (GRAEB-17, GRAEB-18, GRAEB-45).
13. **Zero-cost joint-yield estimate per epoch + optional MC signoff with statistically sized n** — yield evidence for the delivered layout (GRAEB-33, GRAEB-30, GRAEB-31, GRAEB-44).
14. **Per-block DC feasibility (sizing-rule) checks with IR consumption** — electrical preconditions of recognized blocks checked every layout; thresholds from [58]/user (GRAEB-40).
15. **Report guaranteed 3σ values per spec and a Pareto archive of feasible epochs** — states the layout's statistical cost and the real trade-offs (GRAEB-43, GRAEB-42).
