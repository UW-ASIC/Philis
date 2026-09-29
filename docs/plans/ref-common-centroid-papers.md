# Common-Centroid Papers: CC Review, Constructive CC DAC P&R, Nth-Order Central Symmetry

Sources (three papers, read in full):

| Tag | Paper | reftext file | Lines read | PDF pages |
|---|---|---|---|---|
| REV | Karmokar, Madhusudan, Sharma, Harjani, Lin, Sapatnekar, "Common-Centroid Layout for Active and Passive Devices: A Review and the Road Ahead", ASP-DAC 2022, pp. 114–121 | `reftext/cc_review.txt` | L1–L623 (all) | 1–8 |
| DACP | Karmokar, Sharma, Poojary, Madhusudan, Harjani, Sapatnekar, "Constructive Placement and Routing for Common-Centroid Capacitor Arrays in Binary-Weighted and Split DACs", IEEE TCAD 42(9), 2023, pp. 2782–2795 | `reftext/cc_dac_constructive.txt` | L1–L1174 (all) | 1–14 |
| NTH | Dai, He, Xing, Chen, Geiger, "An Nth Order Central Symmetrical Layout Pattern for Nonlinear Gradients Cancellation", ISCAS 2005, pp. 4835–4838 | `reftext/nth_order.txt` | L1–L328 (all) | 1–4 |

The reftext directory is `/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/`. The text is two-column `pdftotext -layout` output, so one line often carries the left and right columns side by side, and line ranges below can interleave. Tables I–VI of DACP are images with no text (reftext holds only their captions). I read them from the PDF at 300 dpi (PDF p.11–13), and the numbers quoted from them are marked "(PDF image)".

PDF page to reftext line maps (from form-feed counts):
- REV: p1 L1–82, p2 L83–157, p3 L158–250, p4 L251–335, p5 L336–403, p6 L404–469, p7 L470–537, p8 L538–623.
- DACP: p1 L1–72, p2 L73–149, p3 L150–262, p4 L263–372, p5 L373–525, p6 L526–626, p7 L627–698, p8 L699–797, p9 L798–852, p10 L853–915, p11 L916–964, p12 L965–1021, p13 L1022–1091, p14 L1092–1174.
- NTH: p1 L1–61, p2 L62–153, p3 L154–245, p4 L246–328.

---

## 1. Coverage

Read chunks (Read tool, consecutive):
- `cc_review.txt`: offset 1 to end (L1–L623) in one read.
- `nth_order.txt`: offset 1 to end (L1–L328) in one read.
- `cc_dac_constructive.txt`: L1–L600, then L600–L1174.
- PDF pages opened to fix garbled equations and image tables: DACP p.1 (eq. 1), p.7–8 (Fig. 6, Algorithm 1), p.11–13 (Tables I–VI, Figs. 13–16; Table I rendered at 300 dpi); REV p.3 (eqs. 12–21, Fig. 3–4), p.5–7 (Figs. 6–10, eq. 24–25); NTH p.3–4 (Figs. 1–4, Table I).

Section headings in range:

**REV** (L1–623)
- Abstract (L12–26)
- I. Introduction (L28–73; eqs. 1–5)
- II. Modeling On-Chip Variations (L75–114; eqs. 6–7)
  - A. Modeling Systematic Variations (L116–145)
    - 1) Process Gradients (L117–142; eqs. 8–10)
    - 2) Layout-Dependent Effects (L144–152, L98–145 right column; eq. 11; Fig. 2)
  - B. Modeling Correlated Variations (L147–217; eqs. 12–19; Fig. 3)
  - C. Modeling Random Variations (L219–245, L183–201; eqs. 20–21)
- III. Common-Centroid Capacitor Layout (L202–216; Fig. 4)
  - A. Performance Modeling (L218–299; eqs. 22–23)
  - B. Unit Capacitor Structures (L301–330, L297–305; Fig. 5)
  - C. CC Capacitor Array Construction (L307–377; Figs. 6–7; eq. 24)
- IV. Common-Centroid Transistor Layout (L379–423, L406–407; Fig. 8)
- V. Is Common-Centroid Layout Always Necessary? (L424)
  - A. Impact of Layout on Performance (L425–503; eq. 25; Fig. 9)
  - B. Evaluation of Circuit-Level Performance (L516–531, L472–498; Fig. 10)
- VI. Future Directions and Conclusion (L500–522)
- References (L523–617)

**DACP** (L1–1174)
- Abstract, Index Terms (L15–31)
- I. Introduction (L35–147; eq. 1; Fig. 1)
- II. Background (L95)
  - A. Binary-Weighted DACs (L97–173; eqs. 2–3; Fig. 2)
  - B. Split DACs (L177–262; eqs. 4–10; Fig. 3)
  - C. Modeling Variations in DAC Capacitor Array (L271–370)
    - 1) Modeling Systematic Variation Due to Linear Gradient (L286–321; eqs. 11–12)
    - 2) Modeling Random Variations (L323–370; eqs. 13–16)
- III. Circuit-Level Metrics (L267–284; eqs. 17–18)
  - A. Errors in Linearity Metrics for Binary-Weighted DAC (L285–363; eqs. 19–24)
  - B. Errors in Linearity Metrics for Split DAC (L365–442; eqs. 25–31)
  - C. Capacitor 3-dB Frequency (L445–603; eqs. 32–36; Fig. 4)
  - D. Nonunit Capacitor Aspect Ratio (L607–616, L534–606; eqs. 37–45; Fig. 5)
- IV. Common-Centroid Placement and Routing (L619–648)
  - A. CC Placement for Binary-Weighted DAC (L650–786): 1) Placement Tradeoffs Between Wire Resistance and Dispersion; Spiral; Chessboard; BC approaches; Algorithm 1 (L702–735; Fig. 6)
  - B. Routing for Binary-Weighted DAC (L790–826): 1) Connected Unit Capacitor Group Formation (L748–761); 2) Bottom-Plate Routing (L763–795); 3) Top-Plate Routing (L813–826); Figs. 7–9
  - C. Layout Generation for Split DAC (L829–913): 1) Array Size Calculation (L835–847); 2) Placement (L848–878; Figs. 10–11); 3) Routing (L880–913, L860–881)
- V. Results and Discussion (L884–1043; Tables I–VI; Figs. 12–16)
- VI. Conclusion (L1045–1060)
- References and biographies (L1062–1174)

**NTH** (L1–328)
- Abstract (L10–17)
- I. Introduction (L19–53, right column L10–28)
- II. Gradient Modeling (L30–80, L94–147; eqs. 1–12)
- III. The Nth Order Central Symmetrical Layout Pattern and Nonlinear Gradients Cancellation (L83–222; eqs. 13–23; Figs. 1–3)
- IV. Comparison of Different Layout Patterns and Simulation Results (L225–266; Fig. 4; Table I L294–310)
- V. Conclusion (L268–293)
- References (L295–322)

---

## 2. Section-by-section digest

### REV Abstract (L12–26)
- CC layout cancels systematic variation, but it has to be engineered with routing parasitics, LDEs (active devices) and the performance impact of the layout in view (L20–23).
- The best CC layout depends on the choice of unit device and on the ratio of uncorrelated to systematic variation. Non-CC layouts are sometimes preferable (L23–26).

### REV I. Introduction (L28–73)
- Circuits are designed to be insensitive to absolute variation but remain sensitive to differential variation between devices (L29–40).
- CC with k elements, where device i has s_i units: the unit centroids of every device coincide. The 1D criterion is eq. (1), (1/s_A)Σx^A_i = (1/s_B)Σx^B_i (L41–57). The ABBA sequence meets it (L58).
- 2D: eqs. (2)–(3) require equal per-device mean x and mean y over all k devices (L59–71).
- Interdigitated (ABAB) layouts have no common centroid: A's centroid sits left of B's (L73–78, L28). CC matches better but costs more routing complexity and parasitics (L28–33).
- Rationale: under a linear gradient Δp = α·x, ΔP = αS_p·x (eqs. 4–5). The unit shifts in ABBA (−2α, +2α for A; −α, +α for B) cancel within each device (L34–64).
- A near-square aspect ratio minimises the maximum distance from the origin, which bounds systematic variation (L65–73).

### REV II. Modeling On-Chip Variations (L75–114)
- Variation is either systematic (predictable) or random (statistical). Global variation affects all like devices equally and causes no mismatch (process corners). Local variation (gradient plus spatially correlated random) causes mismatch and "does not significantly affect small arrays" (L75–94).
- Random variation is either uncorrelated (independent even for adjacent elements) or spatially correlated with a correlation distance. Uncorrelated variation has correlation distance 0 (L97–104).
- ΔP = g + u + s, σ_P² = σ_g² + σ_u² + σ_s² (eqs. 6–7, L105–114).

### REV II-A-1 Process Gradients (L117–142)
- Oxide-thickness gradient for caps: C*_k = Σ_{units} C_u·t0/t_k, t_k = t0 + γ(x_k cosθ + y_k sinθ), with γ the magnitude and 0 ≤ θ ≤ 180° the angle (eqs. 8–9, L117–132).
- Systematic metric: M_sys = max_{p≠q} |(C*_p/C*_q) − (C_p/C_q)| / (C_p/C_q) (eq. 10, L133–142).

### REV II-A-2 Layout-Dependent Effects (L144–152, L98–145)
- WPE: V_th shifts with the device's distance from the well edge. Mitigate by keeping well edges far away or by giving matched devices equal well spacing (L149–152, L98–102).
- LOD: ΔV_th ∝ 1/LOD = Σ_{i=1..n} [1/(SA_i + 0.5L_g) + 1/(SB_i + 0.5L_g)] over n unit cells (eq. 11, L103–114). Matched devices must have the same SA and SB (L115–117).
- OSE (OD spacing and width): keep OD width and spacing identical (L118–125). Gate pitch: keep poly pitch identical (L126–133).
- Identical unit cells cancel every LDE except LOD and WPE: uniform poly pitch, uniform OD width, row-based y-OSE and diffusion-shared x-spacing. The remaining effort goes into dummies and placement (L135–145).

### REV II-B Modeling Correlated Variations (L147–217)
- Caps: ρ_s(r) = ρ0^{r/l}, with 0 < ρ0 < 1 and l set by the process. l acts as a correlation distance, typically 1 mm (eq. 12, L147–161; exponent confirmed at PDF p.3 and by DACP eq. 13–14).
- C_p = pC_u, C_q = qC_u: ρ_pq = Cov(p,q)/(σ_pσ_q) (eq. 13), where σ_p² = σ_u²(p + 2S_p), Cov = σ_u²S_pq, S_p = Σ_{a<b∈p} ρ_ab and S_pq = Σ_{a∈p, b∈q} ρ_ab (L162–180).
- ρ_avg = (1/C(t,2)) Σ_{p<q} ρ_pq over t capacitors (eq. 14, L181–188).
- M_rand = max_{p≠q} var(C_p/C_q) (eq. 15), var(C_p/C_q) = A_f²/(C_u²q⁴WL)·[q²(p+2S_p) + p²(q+2S_q) − 2pqS_pq] (eq. 16, L190–202).
- Active devices: ρ_g(r) = 1, ρ_u(r) = 0, ρ_s(r) = e^{−(r/R_L)²}, with R_L the correlation distance. cov(P_i,P_j) = σ_g² + ρ_s(r)σ_s², and ρ(r) = (σ_g² + ρ_s(r)σ_s²)/σ_P² (eqs. 17–19, L203–217). In Fig. 3 ρ falls from about 0.8 to a floor of about 0.2 by roughly 5 mm (L179–180, PDF p.3 plot).

### REV II-C Modeling Random Variations (L219–245, L183–201)
- RDF and LER are uncorrelated and shrink with larger devices (L221–223).
- Pelgrom: σ²(ΔP) = σ_u² + σ_s², with σ_u² = A_P²/(WL) and σ_s² = S_P²r² (eq. 20, L229–245). The first term dilutes with area, the second with distance (L183–192).
- Caps: σ_u² = A_f²/(WL), where W and L are the unit cap's dimensions and A_f is a Pelgrom-like coefficient (eq. 21, L193–200).

### REV III Common-Centroid Capacitor Layout (L202–216)
- CC cap arrays serve charge-scaling DACs, SAR ADCs and SC filters (L203–207).
- Quality depends on the unit structure, the placement style and the routing. It must be translated into nonlinearity and power (L211–216).
- Fig. 4 names the parasitics C_TB, C_TS and C_BS of a charge-scaling DAC (L224–244).

### REV III-A Performance Modeling (L218–299)
- DNL_i = (V_OUT(i+1) − V_OUT(i) − V_LSB)/V_LSB (eq. 22). INL_i = (V_OUT(i) − V_OUT^ideal(i))/V_LSB (eq. 23) (L254–279).
- |DNL| or |INL| > 1 LSB can make the DAC non-monotonic or miss codes. A robust DAC keeps both within ±0.5 LSB (L263–268).
- Larger C_u reduces mismatch and parasitic impact but raises power and area. Lin et al. show that minimising routing-parasitic mismatch permits a smaller C_u (L286–299).

### REV III-B Unit Capacitor Structures (L301–330, L297–305)
- MOM beats MIM on cost and density in multi-metal nodes. The structures are finger, sandwich, pillar, mortise-tenon and vertical bars (L303–314; Fig. 5).
- Finger has the highest density, is the easiest, and is FinFET-friendly (gridded lower metals). In non-FinFET nodes it brings large C_TS (gain loss) and C_TB (finger to routing coupling) (L315–327).
- Sandwich, pillar and mortise-tenon enclose the top plate in the bottom plate, which eliminates C_TS and eases routing, at lower density. A parameterised mortise-tenon generator exists [28] (L328–330, L297–305).

### REV III-C CC Capacitor Array Construction (L307–377)
- First size the array as near-square, then fill the leftover slots with dummies. An outer dummy ring often suppresses edge (fringe) effects (L309–314).
- Heuristics: rectangles-and-circles [12]. Higher dispersion gives a higher correlation coefficient and lower variation [20]. A non-CC placement [37] raises correlation at the cost of systematic mismatch (L315–322).
- ILP [38] has three constraint sets: exclusivity (one unit per slot), ratio (exact unit counts) and routing (each unit picks one track in one of its four adjacent channels, each track spanning a whole channel) (L323–330).
- Chessboard [39–41]: C_N on one colour, C_{N−1} on the other, then alternate (Fig. 6). Partition-centering for C_2..C_{N−1} [42]. Hybrid chessboard for non-binary ratios that matches routing wirelength to capacitor ratio (PASTEL [43]). Chessboard needs many vias, which degrade f3dB. Block chessboard [44] recovers f3dB (L338–358).
- SA: [13] uses a pair sequence (pairs symmetric about the CC point, placed from the innermost circle outward) with 3 perturbation ops and adjacency feasibility, but ignores routing (L359–373). C-CBL [45] (grid based, SA over the global sequence pair) also ignores routing (L374–383). [46] runs simultaneous placement and global routing over a pair sequence ordered by nonincreasing distance from the CC point. Its routability analysis finds overlapping channel spans, minimises the maximum tracks per channel (target one), and routes trunk, then branch (BFS), then symmetric bridge wires (L384–361, Fig. 7).
- [27]: an MST over top plates plus a CP-sequence that encodes unit size, routing topology and pattern. A GA uses fitness Φ = (α·C_unit/C_unit,max + β·DNL_max/0.5 + γ·INL_max/0.5)^{−1}, set to ∞ penalty when DNL_max or INL_max > 0.5 LSB (eq. 24). A small ILP adds shields that keep ratios ideal (L362–377).

### REV IV Common-Centroid Transistor Layout (L379–423, L406–407)
- Cap-array algorithms do not transfer directly to transistors because of diffusion sharing and LDEs (L381–385).
- Constructive patterns exist [48, 49], with thermal awareness in [49]. McAndrew's [47] dispersion work is limited to two transistors. None of these routes or shares diffusion (L386–395).
- Long [2]: vertices are nodes and edges are S/D finger connections. The half diffusion graph M_half halves the edge counts. An Euler path on M_half gives half of the layout, which is reflected about the CC point. Odd unit counts put one cell at the layout edge [19] (L396–444; Fig. 8, M = [2,2,4,8,8], M_half = [1,1,2,4,4]).
- ALIGN [19]: avoids the expensive Euler enumeration, accounts for LDE and parasitic mismatch, and routes as an EM/IR-aware fishbone with optimised widths. At each step it places cells of the device with the largest Ratio (fraction of unplaced cells), alternating left and right of the CC point. If a device already occupies the column in another row, it gives priority to a different device (LOD). The procedure fills row by row and then reflects (L445–464, L406–407; Fig. 8(c)–(h)).
- FinFET gate misalignment [50]: the printed gate shifts, raising V_th and lowering I_D. Choosing the orientation of every FinFET in a CM or DP matches the current ratios [55]. A new current-ratio quality metric covers misalignment and parasitic R. Placement is diffusion-sharing aware and maximises dispersion. Routing is parasitic-aware MST (L408–423).

### REV V-A Impact of Layout on Performance (L425–503)
- LDE: in clustered and ID rows, SA/SB of the outer A equals SB/SA of the outer B, so LOD matches. In a CC row the inner B cells see a different LOD from the outer A cells, which causes mismatch (L428–437; Fig. 9).
- Routing parasitic mismatch: CC has unequal drain and source wire lengths between A and B, while ID and clustered do not. In FinFET nodes, unidirectional layers forbid detours and vias add large R jumps, so R matching is hard (L438–447).
- Parasitic and LOD mismatch matter more for small devices. For large devices, two rows with A and B swapped in the second row match both LOD and routing parasitics even for CC (L448–453).
- Effective G_m = g_m(v_in − i_ac·R_s)/v_in (eq. 25). The clustered pattern has the highest current through R_s and the lowest G_m. CC and ID see similar currents, but CC has R_s mismatch, so the ranking is ID > CC > clustered (L454–503).

### REV V-B Evaluation of Circuit-Level Performance (L516–531, L472–498)
- Cases are clustered, CC and optimised layouts at R_L = 10 µm and 1000 µm (L516–519).
- 5T-OTA: DP 46 µm/14 nm, NMOS CM 18.4 µm/14 nm, PMOS CM 2.3 µm/14 nm. The optimised layout uses CC for the DP (80 units/device) and the NMOS CM (40 units/device), in 4 rows that match LOD and parasitics, and clusters the small PMOS CM. The CC layout is worse than optimised because the PMOS CM is CC in one row with 4 units per device, which gives high parasitics and LOD mismatch (L520–531, L472–475).
- At R_L = 1000 µm the u term dominates. At R_L = 10 µm clustered is clearly worse. The optimised layout has the best σ and a good μ (L476–480).
- StrongARM: DP 6.1 µm, NMOS CCP 3.1 µm, PMOS CCP 1.6 µm. All blocks are small, so the optimised layout is clustered. CC creates X/Y capacitance mismatch and ID has higher parasitics at X/Y. The dynamic offset is nonlinear in V_th mismatch and parasitics. At R_L = 10 µm clustered degrades, and CC's μ and σ are similar to u-only but worse than optimised (L481–498).

### REV VI Future Directions (L500–522)
- Well-matched, low-capacitance, low-parasitic structures remain open, and so does the question of when CC beats non-CC (L504–509).
- FinFET and GAA: on-grid, fixed pitch and width, unidirectional lower metals. Turning or changing layer costs via R. MOM is preferred over MIM because via stacks to the upper metals are expensive. Stress calls for dummies [15, 60] (L510–522).

### DACP Abstract and I. Introduction (L15–147)
- Fast constructive CC placement and routing for binary-weighted and split charge-sharing DACs, targeting FinFET nodes with high wire and via R. f3dB degrades badly with parasitics. Via count is traded against dispersion to balance f3dB against INL/DNL (L15–29).
- An N-bit binary DAC has 2^N units, so area and power grow exponentially and settling slows. A split DAC separates an L-bit LSB array from an (N−L)-bit MSB array with an attenuation cap, C_A = (C_T^LSB / C_T^MSB)·C_u (eq. 1, PDF p.1). This value is generally not an integer multiple of C_u (L46–76).
- A unit bridge cap [14] gives a 1-LSB gain error. For MIM, non-unit caps keep the perimeter-to-area ratio [15] (L77–84).
- FinFET issues: higher per-unit wire and via R, a single direction per layer (every turn is a via), and MOM in the low layers is preferred because MIM needs via stacks. Detour-based matching [16, 17] pays heavy R penalties (L85–111).
- Contributions: a constructive spiral CC (few bends and vias, high f3dB), a block chessboard (BC) family between spiral and chessboard, the first split-DAC CC automation plus attenuation-cap sizing, and parallel wires for the critical bits to emulate wide wires under quantised widths (L112–147, L87–93).

### DACP II-A Binary-Weighted DACs (L97–173)
- C_k = n_kC_u with n_0 = 1 and n_k = 2^{k−1} for k ≥ 1. C_0 is always grounded. C_T = 2^N C_u (L115–123).
- C_ON(i) = Σ D_k(i)2^{k−1}C_u, and V_OUT^ideal(i) = V_REF·C_ON(i)/C_T = V_REF·Σ D_k(i)2^{k−N−1} (eqs. 2–3, L122–140).
- Routing parasitics per C_i are top-to-ground C_TS, top-to-bottom C_TB, bottom-to-ground C_BS, and bottom-to-bottom and top-to-top C_BB/C_TT. C_BB and C_TT are zero when both plates share a potential. In a binary DAC all top plates share one node (L142–157).
- C_TB accumulates into C_TB^ON or C_TB^OFF depending on the switch state. C_TS and C_TB alter V_OUT. C_BS does not affect linearity but loads V_REF and costs power and f3dB (L158–173).

### DACP II-B Split DACs (L177–262)
- The LSB array has n_k = 1 for k = 0 (C_0 grounded) and 2^{k−1} for k ≥ 1. The MSB array has n_k = 2^{k−1−L}, starting from C_u (L186–196).
- C_T^LSB = C_ON^LSB + C_OFF^LSB (with C_OFF^LSB including the +C_u of C_0), and C_T^MSB likewise (eqs. 4–5, L197–252).
- Superposition: V_OUT,ideal^LSB = V_REF·(C_ON^LSB/(C_T^LSB + C_A^MSB))·(C_A/(C_A + C_T^MSB)), with C_A^MSB = C_A·C_T^MSB/(C_A + C_T^MSB). V_OUT,ideal^MSB = V_REF·C_ON^MSB/(C_T^MSB + C_A^LSB), with C_A^LSB = C_A·C_T^LSB/(C_A + C_T^LSB). V_OUT^ideal is the sum (eqs. 6–10, L179–247).
- C_TS and C_TB of each sub-array alter the respective components (L253–262).

### DACP II-C Modeling Variations (L271–370)
- Units are identical cells on a gridded CC matrix. With the origin at the centre of an r×s array, x_k = (s_k − (s+1)/2)(W + S_h) and y_k = ((r+1)/2 − r_k)(H + S_v) (L271–284).
- Systematic: t_j = t0 + γ(x_j cosθ + y_j sinθ), where t0 is the MOM finger spacing at the centre. γ and θ vary randomly between arrays and are constant within one. C*_k = Σ_j C_u·t0/t_j (eq. 11). M_sys as in REV eq. 10 (eq. 12) (L286–321).
- Random: σ_u² = A_f²/(WH). ρ_AB = ρ_u^{D(A,B)} with D = √((x2−x1)² + (y2−y1)²)/L_c, and 0 < ρ_u < 1 (eqs. 13–14). σ_p², σ_q², Cov, S_p, S_q and S_pq are as in REV (eqs. 15–16) (L323–370).

### DACP III Circuit-Level Metrics and III-A Binary DAC Errors (L267–363)
- DNL(i) = (V_OUT(i) − V_OUT(i−1) − V_LSB)/V_LSB for 1 ≤ i ≤ 2^N − 1, with V_LSB = V_REF/2^N. INL(0) = 0 and INL(i) = (V_OUT(i) − V_OUT^ideal(i))/V_LSB (eqs. 17–18, L271–284).
- V_OUT(i) = V_REF·(C_ON(i) + ΔC_ON(i))/(C_T + ΔC_T) (eq. 19). ΔC_ON(i) = Σ D_k(i)ΔC_k + C_TB^ON (eq. 20). ΔC_T = Σ_{k=0..N} ΔC_k + C_TB^ON + C_TB^OFF + C_TS (eq. 21). ΔC_k^sys = C*_k − n_kC_u (eq. 22) (L285–321).
- 3σ model: σ²_{C_ON(i)} = Σ_j Σ_k D_j D_k Cov(j,k) and σ²_{C_T} = Σ_j Σ_k Cov(j,k). ΔC_ON = Σ D_kΔC_k + 3σ_{C_ON(i)} + C_TB^ON, and ΔC_T = ΣΔC_k + 3σ_{C_T} + C_TB^ON + C_TB^OFF + C_TS (eqs. 23–24, L323–363).

### DACP III-B Split DAC Errors (L365–442)
- Nonideal V_A(i) = V_REF·(C_ON^LSB + ΔC_ON^LSB)/(C_T^LSB + ΔC_T^LSB + C_A,nonideal^MSB) (eq. 25). V_OUT^LSB = V_A·(C_A + C_A^TB)/(C_A + C_A^TB + C_T^MSB + ΔC_T^MSB) (eq. 26). V_OUT^MSB = V_REF·(C_ON^MSB + ΔC_ON^MSB)/(C_T^MSB + ΔC_T^MSB + C_A,nonideal^LSB) (eq. 27) (L377–437).
- C_A,nonideal^X is the series combination of (C_A + C_A^TB) with (C_T^X + ΔC_T^X) (L443–448).
- The shifts carry C_TB^{X,ON}, C_TB^{X,OFF}, C_TS^X and 3σ terms over each sub-array's own covariance sums (eqs. 28–31, L449–442).

### DACP III-C Capacitor 3-dB Frequency (L445–603)
- f3dB is the maximum switching speed of the bottom plates. FinFET wire and via R dominate, and high-dispersion layouts need many vias (L445–469).
- f3dB = 1/(2(N+2)·ln 2·τ), where τ is the maximum over bits of the Elmore delay of the RC mesh to the units of C_i (eq. 32, L469–493).
- Split DAC, for bit k alone switched: C_A,eq^LSB = C_{T−k}^LSB + C_TB,T−k^{LSB} + (C_A + C_A^TB)(C_T^MSB + C_TB^MSB)/(C_A + C_A^TB + C_T^MSB + C_TB^MSB) (eq. 33). C_eq^LSB = (C_k^LSB + C_TB,k)·C_A,eq/(C_k + C_TB,k + C_A,eq) (eq. 34). eqs. 35–36 are the MSB mirror. R is R_k of the switched cap, and the minimum f3dB over all caps is the circuit's (L494–603).

### DACP III-D Nonunit Capacitor Aspect Ratio (L607–616, L534–606)
- The attenuation cap is not an integer multiple of C_u. It is conventionally 1–2× C_u in a rectangular box [15] (L607–616).
- MOM: C = N(l·t·ε/d), with l = w − c and H = Np, so C = (εt/(dp))·l·H (eqs. 37–39). Under ±δ edge shifts (l → l + 2δ, w → w + 2δ, H → H + 2δ), ΔC/C = 2δ(H + l)/(Hl) = 2δ(H + w − c)/(H(w − c)) (eqs. 40–45), i.e. perimeter-to-area as for MIM, with w − c in place of w (L534–612).
- Sizing: set the nonunit cap's relative error equal to the unit's and solve with eq. 39 for H and l (two equations, two unknowns) (L613–616).

### DACP IV and IV-A CC Placement for Binary-Weighted DAC (L619–786)
- The flow runs constructive placement first (mismatch, wirelength, RC, f3dB) and routing second (L619–648).
- Dispersion (the spread of each C_i's units) buys random matching. Reserved-direction FinFET layers turn every bend into a via, so the via count sets R and f3dB (L651–670).
- Spiral: C_0 and C_1 (one unit each, odd) sit diagonally opposite at the centre. Then all of C_2, then C_3, and so on follow a spiral from the centre, and every unit at (d1, d2) is paired with its reflection at (−d1, −d2). Units of one C_i line up in rows and columns, so vias are few (L671–696, L631–637).
- Chessboard [8]: for 6 bits, the 32 units of C_6 go on the black squares of 8×8, then C_5, and so on. Routing R is large (L638–644).
- BC: a full-chessboard inner core for C_0..C_k (small caps, whose many vias do not set f3dB) and outer corridors in blocks for C_{k+1}..C_N. In the 6-bit example (k = 4) the corridor is 2 cells wide. Since n6:n5 = 2:1, half of C_6 is clustered and the rest alternates with C_5 in chessboard (L645–667). Assume k and N−k even so that no dummies are needed (L672–679).
- Footnote 2: for uncorrelated variation the MSB's σ/μ is lower (μ = nμ_u, σ = √n·σ_u), so MSB caps matter less for accuracy. The ratio C_k/C_0 still depends on C_0, which varies strongly (L689–696).
- Algorithm 1 (PDF p.8): r_c = ⌈√(Σ_{i=0..k} C_i)⌉ and s_c = ⌈(Σ C_i)/r_c⌉. Fill the inner chessboard. For i = k+1..N step 2: r_i = ⌈√(Σ_{j=0..i+1} C_j)⌉, s_i = ⌈Σ C_j / r_i⌉, and (x_s, y_s) = (s_i/2 − s_c/2, r_i/2 − r_c/2). For p = 1..⌈(r_i/2)/b_s[i]⌉ and q = 1..⌈(s_i − s_c)/b_s[i]⌉, place a b_s×b_s block of C_i plus its diagonal-symmetric copy, alternating the start column (x_s = 0 or b_s by p parity) and advancing y_s by b_s. Put C_{i+1} in the leftover corridor cells, then set r_c ← r_i and s_c ← s_i (L702–786).

### DACP IV-B Routing for Binary-Weighted DAC (L790–826, L748–795)
- Minimise C_TB as in [9] by non-overlapped routing that separates top-plate and bottom-plate wires (L791–795).
- Connected groups: a graph whose nodes are units, with edges between neighbouring same-C_i units. BFS gives the connected components. Branch wires follow the BFS tree, and every connection is mirrored to the diagonally symmetric location (L748–761; Fig. 7).
- Bottom plates go to switches and drivers clustered outside the array, and the terminals exit at the bottom. Wires are of three kinds: branch (within a group, or group to trunk), trunk (vertical tracks joining disjoint groups) and bridge (joining trunks at the periphery). The steps are channel selection, track assignment and routing [18] (L763–789; Fig. 8).
- Critical bits use multiple parallel wires, which give parallel vias at every direction change under quantised FinFET widths (L790–795; Fig. 9(c)).
- Top plate: a graph of all units with N/S/E/W edges weighted by spacing. Because vertical spacing < horizontal (channel) spacing, the MST connects each column with branch wires and then adjacent columns. This minimises C_TS (L813–826; Fig. 9(d)).

### DACP IV-C Layout Generation for Split DAC (L829–913)
- Array size is near square. Ratios are [1:1:2:…:2^{L−1}:1:2:…:2^{N−L−1}]. Dummies D_C = r·s − (Σ n_i + 2), where the +2 is the two slots of the nonunit C_A (L835–847).
- Placement: put the two slots of C_A near the centre, symmetric (smaller than C_u, so the most perturbed by routing C). Then the odd-count caps: C_0 and a dummy diagonal near the centre, C_1 at the next spiral slot and C_6 at its symmetric slot. Then alternate LSB and MSB (C_2, C_7, C_3, C_8, …) along the spiral with reflected pairs. The alternative (C_0, C_1, C_2 and a dummy on the inner 4, C_A on the next ring) showed no significant difference (L848–843, L810–834; Fig. 10).
- BC for split: the complete chessboard holds the small caps and BC the larger ones, with equal block sizes for same-sized LSB/MSB caps (e.g. C_4 and C_9) placed diagonally symmetric. More dispersion means better INL/DNL and more routing parasitics (L844–878; Fig. 11).
- Routing: bottom plates as in the binary DAC. C_A's two slots join during group formation if adjacent, otherwise through an extra vertical track. Top plates: BFS joins adjacent groups, and an MST over the remaining groups of each side array minimises C_TS^LSB and C_TS^MSB. C_A connects to the top plates of adjacent units of each side (L880–913, L860–881).

### DACP V Results and Discussion (L884–1043; Tables from PDF p.11–13)
- Implemented in Python and evaluated on a commercial 12 nm process, N = 6–10. The methods compared are [1] (Lin 2013), [8] (chessboard), S (spiral) and BC. Wire pitch is 64 nm. R = r·l, C = c·l and Cc = c_c(s)·l_overlap. γ = 10 ppm, ρ_u = 0.9, L_c = 1 mm, A_f² = 0.85%·1 fF, C_u = 5 fF (L884–911).
- Table I (8-bit, PDF image): ΣC_TS is 0.40 ([1]), 0.38, 0.38 and 0.38 fF. ΣC_wire is 12.3, 36.6, 5.8 and 18.5 fF. ΣC_BB is 17.6, 73.3, 6.1 and 15.8 fF. (ΣN_V, ΣL µm) is (60, 1042), (295, 3012), (60, 514) and (234, 1535). Critical bit (R_V, R_total) is (0.3, 4.1), (4.1, 14.0), (0.002, 0.16) and (0.02, 0.67) kΩ. For S the only vias are at the input connection (L949–956).
- Table II (8-bit, PDF image): area is 2641, 2541, 2509 and 2541 µm². {|DNL|, |INL|} is {0.00, 0.08}, {0.01, 0.06}, {0.05, 0.08} and {0.02, 0.07} LSB. f3dB is 56, 16, 1411 and 339 MHz. At 10-bit: [8] {0.05, 0.30} at 0.8 MHz, S {0.22, 0.31} at 81 MHz, BC {0.11, 0.30} at 29 MHz. All are below 0.5 LSB (L934–945).
- With parallel wires, k = 2 gives more than 2× f3dB (a 2×2 mesh with four vias). The gain lies between 2× (wire-dominated) and 4× (via-dominated), with diminishing returns as k grows. Chessboard stays via-bottlenecked (L996–1013; Fig. 15). [8] needs 5 vertical tracks per channel against 2 for the spiral (L931–957).
- Table III (PDF image): at γ = 100 ppm, 10-bit S gives {DNL, INL} = {4.93, 2.56} LSB while [8] gives {0.06, 0.34}. 8-bit S gives {0.27, 0.17}. The t0/t nonlinearity grows with γ and defeats CC's linear cancellation (L998–1007). Table IV: the dependence on ρ_u is non-monotonic (L982–997). No measured ρ or γ exists for caps. The values are inherited from older papers (L1014–1019, L982–985).
- Table V (split, 2 parallel wires; C_u = 5 fF for 6–12 bit, 10 fF for 14 bit): 8-bit S area is 362 µm² at f3dB 19977 MHz. Fig. 16(c) compares 8-bit binary and split: area 2509 against 362 µm², f3dB 1411 against 19977 MHz, |DNL| 0.05 against 0.06, |INL| 0.08 against 0.04. Split-DAC critical-bit R is higher, so it uses 2 parallel wires against 4 for binary (L1009–1087).
- Table VI (PDF image): spiral runs 0.02, 0.04, 0.12, 0.35 and 1.11 s, and BC 0.03, 0.05, 0.19, 0.38 and 2.25 s, for 6–10 bit, in Python (L1031–1043, L1088–1089).

### DACP VI Conclusion (L1045–1060)
- Routing-conscious constructive placement followed by routing optimises systematic and random mismatch for both DAC types. The paper claims the first models that trade linearity against f3dB and the first automated split-DAC CC. Spiral and BC trade wire parasitics against dispersion (L1046–1060).

### NTH Abstract and I. Introduction (L10–53)
- A pattern with 2^n unit cells per device cancels gradient mismatch up to order n between two devices (L10–17).
- Systematic mismatch can equal random mismatch [7]. Once area has cut the random part, systematic dominates, and larger area worsens the gradient effect (L38–49).
- CC cancels only linear gradients. Circular symmetry [6] cancels nonlinear ones but is area-inefficient and needs diagonal placement (right column L10–17).

### NTH II. Gradient Modeling (L30–80, L94–147)
- p1(x,y) = G1 + C with G1 = g_{1,0}x + g_{0,1}y. p_n(x,y) = Σ_{i=1..n} G_i + C, with G_i = Σ_{j=0..i} g_{j,i−j}x^j y^{i−j} (eqs. 1–4, L32–72).
- A unit cell is represented by the parameter at one point S (unit is small, so the intra-cell gradient is negligible). Device P = Σ_{i=1..m} p_n(x_i, y_i) (eq. 5). Ideal matching is P_A = P_B (eq. 6) (L74–93).
- Moving the expansion centre to (x0, y0) re-expresses the top-order term about (x0, y0) and adds only lower-order terms (eqs. 7–12, L94–80): the centre of the nth-order term can be put anywhere.

### NTH III. The Nth-Order Pattern (L83–222)
- n = 1 is any CC pattern (Fig. 1: ABBA, and 2×2 AB/BA) (L90–93).
- n > 1 is two (n−1)st-order patterns symmetric about a new centre C_n. For odd n, each device is centrally symmetric about C_n by itself. For even n, one copy has A and B interchanged, so A's cells are centrally symmetric to B's about C_n (L94–119).
- Figs. 2–3 (PDF p.3): 2nd order in 1D is ABBABAAB, and in 2D the 2×4 ABBA/BAAB. 3rd order is the 2×8 ABBABAAB/BAABABBA and the 4×4 ABBA/BAAB/BAAB/ABBA (L154–180).
- n = 1: P_A = P_B iff x_cA = x_cB and y_cA = y_cB (eqs. 13–16, L131–207).
- m must be even for n > 1. Odd n: pairs A_i and A_{m−i} are point-symmetric about C_n, so (x−x_Cn)^j (y−y_Cn)^{n−j} sums to 0 per device (eq. 17) and the nth-order term vanishes in both P_A and P_B (eqs. 18–20). Even n: (x_Ai − x_Cn)^j(y_Ai − y_Cn)^{n−j} = (x_Bi − x_Cn)^j(y_Bi − y_Cn)^{n−j} (eq. 21), so the nth-order terms are equal in A and B (eqs. 22–23). By induction all orders 1..n cancel (L209–214, L154–203).
- General corollary: if each device is centrally symmetric by itself, all odd orders cancel. If one device is centrally symmetric to the other, all even orders cancel (L216–222).

### NTH IV. Comparison and Simulation (L225–266; Fig. 4; Table I)
- Patterns compared at equal total area: 1st–5th order central symmetric (Fig. 4(a)–(e): 2×2, 2×4, 4×4, 8×4, 8×8), 2nd-order circular symmetry (f) and hexagonal tessellation (g) (L228–239; PDF p.4).
- Table I, mismatch % by highest gradient order 1–5: (a) 0, 2.77, 5.22, 7.43, 10.39; (b) 0, 0, 0.24, 0.87, 1.70; (c) 0, 0, 0, 0.01, 0.068; (d) 0, 0, 0, 0, 0.0023; (e) all 0; (f) 0, 0, 0, 0.026, 0.18; (g) 0, 0, 0.26, 0.50, 2.24 (L294–310). The gradient magnitudes used are not given.
- Hexagonal tessellation cancels only up to 2nd order. 2nd-order circular symmetry cancels up to 3rd because each device is self-centrally-symmetric (L249–261).

### NTH V. Conclusion (L268–293)
- The nth-order form uses 2^n cells per device, cancels up to nth order (proved and simulated), and is area-efficient with flexible placement (L268–293).

---

## 3. Actionable extraction

### CC-01 Centroid-coincidence criterion for k devices (1D and 2D)
- Kind: check / metric
- Statement: for devices j = 1..k with s_j units at (x_i^j, y_i^j), CC holds iff (1/s_1)Σx^1 = … = (1/s_k)Σx^k and likewise for y. It cancels any linear (first-order) gradient exactly. Interdigitation (ABAB) does not satisfy it.
- Source: REV §I eqs. 1–3, L41–71; NTH eqs. 13–16, L131–207; REV p.1, NTH p.3.
- Philis stage: cells (pattern generation), dp (rule), verify.
- Automation recipe: inputs are the unit positions per device from `Layout.units`. Compute the per-device unit mean; the metric is max_{j,l} ‖c_j − c_l‖ / array extent. Treat it as a hard constraint inside generated arrays and as a cost term when devices are separate cells. Apply to every annotator matched group (DP, CM, cap bank, resistor ratio).
- Beats hand layout because: it is evaluated exactly on the drawn units of every group, including ratioed k-device groups, which humans only eyeball.
- Philis status: implemented. `CentroidGroup` coincidence and gradient checks are at kernel/analog/src/placement/cc.rs:14-24 and :90-121, with the fixed 1% tolerance at cc.rs:80-86. The per-slot `lin_um` of the cap array is at kernel/cells/src/cap_array.rs:56-59 and :369-375.

### CC-02 Gradient-order cancellation by moment matching (general metric)
- Kind: metric / check
- Statement: model p(x,y) = Σ_{i=1..n} Σ_{j=0..i} g_{j,i−j} x^j y^{i−j} + C (NTH eqs. 3–4). Device parameter P = Σ_units p(x_u, y_u) (eq. 5). Then P_A − P_B = Σ_{i,j} g_{j,i−j}·(M_A(j,i−j) − M_B(j,i−j)), where M_D(a,b) = Σ_{u∈D} x_u^a y_u^b. A layout therefore cancels every polynomial gradient up to order n, for any coefficients, iff all mixed moments with a+b ≤ n are equal between devices (per unit, for ratioed devices: M_D/s_D). This is derived directly from NTH eqs. 3–6. The paper proves the sufficient construction (CC-04), and this is the exact necessary-and-sufficient check. For n = 2 it needs three equalities (x², xy, y²), not one.
- Source: NTH §II eqs. 1–6, L32–93; §III L131–207; PDF p.1–3.
- Philis stage: cells (pattern choice), dp (rule), verify (report).
- Automation recipe:
  ```
  fn cancel_order(units: &[(dev, x_nm, y_nm)], a: Dev, b: Dev, nmax: u8) -> (u8, Vec<f64>) {
      // centre on the array centroid, scale by extent Lx = max(|x|,|y|) for conditioning
      let norm = |v| v / L;
      let mut resid = vec![];
      for n in 1..=nmax {
          let r_n = (0..=n).map(|j| {
              let m = |d| mean over u∈d of norm(x)^j * norm(y)^(n-j);   // per-unit mean (ratio-safe)
              (m(a) - m(b)).abs()
          }).fold(0.0, f64::max);
          resid.push(r_n);
      }
      let order = resid.iter().take_while(|&&r| r < 1e-9).count() as u8;
      (order, resid)   // order = highest fully cancelled gradient order
  }
  ```
  For k > 2 devices, take the maximum over device pairs, or compare each device against the group mean. Report `order` per group. The cost is Σ_n w_n·r_n·L^n, where w_n is the deck's nth-order gradient coefficient when present. Without deck data it is a tie-breaker: prefer the higher order, then the smaller r_{order+1}.
- Beats hand layout because: a human checks centroids by eye, while this proves, for each candidate pattern, the exact gradient order cancelled and the residual of the first uncancelled order.
- Philis status: partial. The cap array computes only |⟨r²⟩_slot − ⟨r²⟩_array| (kernel/cells/src/cap_array.rs:60-62, :374), which misses x²−y² and xy. cc.rs:84-85 notes "a curved field would want the second moments too". No general order metric exists.

### CC-03 Odd/even central-symmetry theorem (pattern classifier)
- Kind: rule / check
- Statement: if every device's units are centrally symmetric about one point by themselves, all odd-order gradient terms cancel. If device A's units are the point reflection of device B's about a point, all even-order terms cancel. A plain CC pattern with self-symmetric devices (e.g. ABBA) cancels orders 1, 3, 5… but not order 2 unless the second moments also match.
- Source: NTH §III, L216–222 (eqs. 17–23, L154–197); PDF p.3.
- Philis stage: cells, verify.
- Automation recipe: for a unit grid, test S_self = ∀u∈D: reflect(u) ∈ D, and S_swap = ∀u∈A: reflect(u) ∈ B, about the array centre and about sub-block centres. Label the pattern "odd-cancelling", "even-cancelling" or both (both is possible only hierarchically, per CC-04). This is a cheap pre-filter before CC-02.
- Beats hand layout because: it classifies every generated variant instantly, including hierarchical symmetries a designer would not spot.
- Philis status: partial. Cap-array tests assert point symmetry (kernel/cells/src/cap_array.rs:642) and the BJT has point-symmetric slots (kernel/cells/src/bjt.rs:279), but there is no classifier.

### CC-04 Nth-order central-symmetric pattern construction (two matched devices)
- Kind: algorithm
- Statement: P_1 is any CC pattern with 2 units per device (2×2 [[B,A],[A,B]] or 1×4 ABBA). P_n = P_{n−1} ⊕ R_n, where R_n is the 180° rotation (point reflection) of P_{n−1} placed adjacent to it. For even n, the labels A↔B are swapped in R_n. The new centre C_n is the midpoint of the combined block. P_n uses 2^n units per device and cancels gradients of orders 1..n. Measured mismatch (Table I, % at highest gradient order 1–5) is 1st order 0 / 2.77 / 5.22 / 7.43 / 10.39, 2nd order 0 / 0 / 0.24 / 0.87 / 1.70, 3rd order 0 / 0 / 0 / 0.01 / 0.068, 4th order 0 / 0 / 0 / 0 / 0.0023, and 5th order all 0. The gradient magnitudes are not given. Hexagonal tessellation is worse than 3rd order (0.26% at order 3). Circular symmetry cancels to 3rd order but is area-inefficient and needs non-Manhattan placement.
- Source: NTH §III L90–119 (definition), eqs. 17–23, Figs. 1–4 (PDF p.3–4), Table I L294–310.
- Philis stage: cells (mosfet, finfet, resistor, capacitor, cap_array general set).
- Automation recipe:
  ```
  fn central_sym(n: u8, p1: Grid<Dev>) -> Grid<Dev> {
      let mut p = p1;                                   // 1st order, 2 units/device
      for k in 2..=n {
          let mut r = p.rotate180();                     // point reflection about p's own centre
          if k % 2 == 0 { r.swap_labels(A, B); }
          // concatenate along the shorter side to stay near square (Fig. 4: 2x2,2x4,4x4,8x4,8x8)
          p = if p.rows >= p.cols { p.hcat(&r) } else { p.vcat(&r) };
      }
      p
  }
  ```
  Correctness: in the vertical stack of P (rows 0..h−1) over R, cell (r,c) maps under point reflection about the block centre to (2h−1−r, w−1−c), and R[h−1−r][w−1−c] = rot180(P)[h−1−r][w−1−c] = P[r][c]. For MOS rows, a row of units is one cell: rotate180 reverses finger order and mirrors the row about the new centre line (Philis `stack_on_tap` already mirrors). When the matched pair has 2^n·u units, pick n = ⌊log2(units/2)⌋. Enumerate n = 1..n_max as variants and let CC-02 and the parasitic cost (CC-40) pick among them.
- Beats hand layout because: designers rarely go beyond ABBA/cross-quad. The generator reaches order 3–5 at no extra area whenever the unit count is a power of two.
- Philis status: partial. The 1D ABBA order is at kernel/cells/src/mosfet.rs:720-751. The 2-row mode for 2 devices swaps labels in row 2 (mosfet.rs:151-156), which gives ABBA/BAAB, the 2nd-order Fig. 4(b) pattern, when the row is palindromic. Orders ≥ 3 (4 rows) are missing, and finfet.rs has no matched pattern at all (kernel/cells/src/finfet.rs:89-96).

### CC-05 Nonlinear oxide-gradient capacitor model, worst angle
- Kind: formula
- Statement: C*_k = Σ_{j∈C_k} C_u·t0/t_j with t_j = t0 + γ(x_j cosθ + y_j sinθ), and γ, θ ∈ [0°, 180°) fixed per array but random between arrays. The t0/t nonlinearity matters. At γ = 100 ppm the 10-bit spiral reaches |DNL| 4.93 and |INL| 2.56 LSB against 0.06/0.34 for chessboard (DACP Table III, PDF image). CC cancels only the linear part.
- Source: REV eqs. 8–9, L117–132; DACP eq. 11, L286–310; Table III L968–1007 (PDF p.12).
- Philis stage: cells (metrics), verify.
- Automation recipe: evaluate every array variant under the exact t0/t form. Sweep θ over [0°, 180°) in steps of 180°/(4·max(r,s)) or golden-section on the worst-case INL. Sweep γ ∈ {10, 100 ppm}, or the deck value when present. Report the worst |INL| and |DNL|. This is a cost term for variant choice and a check against a spec (CC-12).
- Beats hand layout because: hand layout assumes linear cancellation, and this exposes the γ at which a low-dispersion (spiral) array fails.
- Philis status: partial. `metrics()` uses the linearised ε = g(x cosθ + y sinθ) + q·r² over 4 angles in 45° steps (kernel/cells/src/cap_array.rs:376-381).

### CC-06 Maximum systematic ratio mismatch M_sys
- Kind: metric
- Statement: M_sys = max_{p≠q} |C*_p/C*_q − C_p/C_q| / (C_p/C_q) under the gradient of CC-05, worst over θ.
- Source: REV eq. 10, L133–142; DACP eq. 12, L311–321.
- Philis stage: cells, verify.
- Automation recipe: O(k²) per angle over the k capacitors of any ratioed bank (DAC, SC gain caps, resistor ratio via the same form with R). Report it with INL/DNL. It is the right metric for non-DAC ratio banks (SC filters) where INL/DNL is undefined.
- Beats hand layout because: it gives a quantitative worst-pair ratio error for arbitrary ratio sets.
- Philis status: missing. There is no M_sys; cap_array.rs:63-67 reports only INL/DNL for binary banks.

### CC-07 Variation decomposition g + u + s and the correlation-distance regime
- Kind: data-model
- Statement: ΔP = g + u + s and σ_P² = σ_g² + σ_u² + σ_s² (REV eqs. 6–7). Global variation g causes no mismatch. Local correlated variation s decays with distance, and small arrays are barely affected. For active devices ρ(r) = (σ_g² + e^{−(r/R_L)²}σ_s²)/σ_P². REV evaluates R_L = 10 µm and 1000 µm. At 1000 µm the u term dominates every pattern, and at 10 µm clustering is clearly worse.
- Source: REV §II L75–114, eqs. 17–19 L203–217, Fig. 3 (PDF p.3); §V-B L476–480.
- Philis stage: deck, annotator (budget split), dp.
- Automation recipe: the deck carries σ_u (A_VT), σ_s and R_L per device type. The annotator's matching budget uses σ_u²/(WL) for area and σ_s²(1 − e^{−(r/R_L)²}) for the separation term in place of the linear S·D. When R_L is not given, keep the Pelgrom linear form (CC-11).
- Beats hand layout because: the tool knows when distance matters (R_L small relative to array size) and when area is the only lever.
- Philis status: partial. The linear Pelgrom S·D form is at kernel/analog/src/placement/matching_pair.rs:74-76. There is no R_L or σ_s in the deck (pdks/*.json carries only `avt_*`).

### CC-08 Capacitor spatial-correlation model and default parameters
- Kind: formula / deck-requirement
- Statement: ρ_ab = ρ_u^{D(a,b)} with D = Euclidean distance / L_c, 0 < ρ_u < 1. The paper's defaults are ρ_u = 0.9, L_c = 1 mm, γ = 10 ppm, A_f² = 0.85%·1 fF, C_u = 5 fF, and wire pitch 64 nm (12 nm process). No measured ρ exists for caps; the values are inherited. σ_u² = A_f²/(W·H).
- Source: REV eq. 12 L147–161 and eq. 21 L193–200; DACP eqs. 13–14 L323–345 and L909–911; L1014–1019.
- Philis stage: deck.
- Automation recipe: add optional deck keys `cap_af_pct_sqrt_ff` (A_f), `cap_rho_u`, `cap_corr_len_um` (L_c), `ox_gradient_ppm` (γ) and `mos_corr_len_um` (R_L), each with a `_source` string like the existing `avt_*_source`. When absent, metrics that need them report "unknown", and the paper defaults are used only as labelled sweep points, never as silent constants.
- Beats hand layout because: the deck makes the matching model explicit and reproducible per process.
- Philis status: missing. The deck has only `avt_n_mv_um` and `avt_p_mv_um` (pdks/sky130.json, gf180mcu.json, ihp_sg13g2.json). cap_array.rs:353-355 notes "No deck carries these".

### CC-09 Correlation coefficient ρ_pq and ρ_avg (quantified dispersion)
- Kind: metric
- Statement: for caps of p and q units, S_p = Σ_{a<b∈p} ρ_ab, S_q likewise, and S_pq = Σ_{a∈p, b∈q} ρ_ab. σ_p² = σ_u²(p + 2S_p) and Cov = σ_u²S_pq, so ρ_pq = S_pq / √((p + 2S_p)(q + 2S_q)). ρ_avg = (2/(t(t−1)))·Σ_{p<q} ρ_pq over t caps. Higher dispersion gives a higher ρ and lower ratio variance [20]. A non-CC placement can raise ρ at the cost of systematic mismatch [37].
- Source: REV eqs. 13–14, L162–188; §III-C L318–322; DACP eqs. 15–16, L347–370.
- Philis stage: cells (variant metric), dp (cost for arrays of separate cells).
- Automation recipe: O(U²) over U units (U ≤ 1024 means ≤ 10⁶ pair terms, negligible). Precompute ρ_ab from unit coordinates with the CC-08 deck values, or with a unitless sweep ρ_u ∈ {0.5, 0.9, 0.99} when absent. Maximise ρ_avg as the dispersion objective in place of the farthest-point heuristic. This replaces the undefined "dispersion degree"; REV gives no formula for dispersion (L388–391).
- Beats hand layout because: dispersion becomes a number optimised per array instead of a visual judgement.
- Philis status: missing. The greedy farthest-point `spread()` stands in for dispersion (kernel/cells/src/cap_array.rs:259-280), and no ρ is computed.

### CC-10 Random ratio mismatch M_rand (closed form)
- Kind: metric
- Statement: M_rand = max_{p≠q} var(C_p/C_q), with var(C_p/C_q) = A_f²/(C_u²q⁴WL)·[q²(p + 2S_p) + p²(q + 2S_q) − 2pq·S_pq]. This is the first-order delta method on eqs. 13 and 21.
- Source: REV eqs. 15–16, L190–202; PDF p.3.
- Philis stage: cells, verify.
- Automation recipe: reuse S_p, S_q and S_pq from CC-09. For a DAC the critical pairs are (C_k, C_0) (DACP footnote 2, L694–696). Report √M_rand in % and compare with the spec budget. It is a hard constraint when a ratio-accuracy spec exists.
- Beats hand layout because: it gives an exact variance per ratio pair, including the correlation benefit of the placement.
- Philis status: missing.

### CC-11 Pelgrom random plus distance mismatch for transistors and caps
- Kind: formula
- Statement: σ²(ΔP) = A_P²/(WL) + S_P²·r² (REV eq. 20). For caps σ_u² = A_f²/(WL) (eq. 21). The first term dilutes with area, the second with proximity.
- Source: REV §II-C L219–245, L183–200.
- Philis stage: annotator (budgets), dp.
- Automation recipe: already the basis of `MatchingPair`/`CentroidGroup` for MOS. Extend it to caps and resistors with A_f and A_R deck keys (CC-08).
- Beats hand layout because: it computes a distance budget per group from the deck.
- Philis status: implemented for MOS (kernel/analog/src/placement/matching_pair.rs:74-76, cc.rs:20-23). Missing for caps.

### CC-12 DAC linearity check: DNL/INL definition and ±0.5 LSB spec
- Kind: check
- Statement: DNL(i) = (V(i) − V(i−1) − V_LSB)/V_LSB and INL(i) = (V(i) − V_ideal(i))/V_LSB for i = 1..2^N − 1, with V_LSB = V_REF/2^N. |DNL| or |INL| > 1 LSB risks non-monotonicity or missing codes. The robust spec is ±0.5 LSB.
- Source: REV eqs. 22–23, L254–284, L263–268; DACP eqs. 17–18, L271–284.
- Philis stage: verify, flow (spec).
- Automation recipe: default hard limit of 0.5 LSB for any recognised DAC bank, overridable by the spec file. Report max |DNL| and |INL| (3σ, CC-13) per bank in signoff.
- Beats hand layout because: every code is checked under gradient, random variation and extracted parasitics.
- Philis status: partial. `inl_lsb`/`dnl_lsb` are computed under gradient only (kernel/cells/src/cap_array.rs:63-67, :383-390), but only in tests (cap_array.rs:699, :786). There is no signoff check.

### CC-13 Nonideal DAC transfer with parasitics and 3σ random terms
- Kind: algorithm
- Statement: V_OUT(i) = V_REF·(C_ON(i) + ΔC_ON(i))/(C_T + ΔC_T), where
  - ΔC_ON(i) = Σ_k D_k(i)ΔC_k^sys + 3σ_{C_ON(i)} + C_TB^ON(i),
  - ΔC_T = Σ_{k=0..N} ΔC_k^sys + 3σ_{C_T} + C_TB^ON + C_TB^OFF + C_TS,
  - σ²_{C_ON(i)} = Σ_j Σ_k D_j D_k Cov(j,k) and σ²_{C_T} = Σ_j Σ_k Cov(j,k), with Cov(j,j) = σ_u²(n_j + 2S_j) and Cov(j,k) = σ_u²S_jk.
- Source: DACP §III-A eqs. 19–24, L285–363; PDF p.4–5.
- Philis stage: cells (variant metric), verify (post-route with extracted C_TS and C_TB).
- Automation recipe:
  ```
  cov[j][k] = σu² * (if j==k { n_j + 2*S_j } else { S_jk })           // from CC-09 sums
  dsys[k]  = Σ_{u∈C_k} Cu*t0/t(u) − n_k*Cu                              // CC-05, worst θ
  for code i in 0..2^N:
      on  = Σ_k D_k(i)*(n_k*Cu + dsys[k]); var_on = Σ_jk D_j D_k cov[j][k]
      ctb_on = Σ_k D_k(i)*CTB_k; ctb_off = Σ_k (1−D_k(i))*CTB_k
      V[i] = VREF*(on + 3*sqrt(var_on) + ctb_on) / (CT + Σdsys + 3*sqrt(Σcov) + ctb_on + ctb_off + CTS)
  DNL/INL per CC-12
  ```
  The cost is O(2^N·N²), which for N = 10 is about 10⁵ multiply-adds. Pre-route, take C_TB and C_TS from the generator's own geometry (Philis draws the array). Post-route, take them from PEX.
- Beats hand layout because: the effect on each code of every parasitic and of correlated mismatch is computed exhaustively.
- Philis status: missing. Only the deterministic gradient part exists (cap_array.rs:376-391).

### CC-14 DAC parasitic taxonomy and which metric each one hurts
- Kind: rule
- Statement: C_TS (top-to-substrate) gives gain error and alters V_OUT. C_TB (top-to-bottom, per bit) gives code-dependent nonlinearity. C_BS (bottom-to-substrate) does not affect linearity but loads V_REF (power, f3dB). C_BB and C_TT are zero between plates at equal potential; all top plates are one node in a binary DAC. Minimising routing-parasitic mismatch allows a smaller C_u, and hence lower power and area.
- Source: DACP §II-A L142–173; REV Fig. 4 L224–244 and L286–299.
- Philis stage: annotator (net roles: DAC top plate = high-Z sensitive; bottom plates = driven), gr/dr (costs), verify.
- Automation recipe: tag the top-plate net as `Sensitive` (penalise any C to ground or to bottom-plate nets). Tag bottom-plate nets as `Driven` (R matters for f3dB, C_BS only for power). The router's per-net cost picks C versus R weighting from the tag.
- Beats hand layout because: every coupling is attributed to a metric instead of minimising "parasitics" generically.
- Philis status: partial. The cap array keeps C_TB structurally absent (kernel/cells/src/cap_array.rs:12-17). The annotator has no DAC-plate net roles (grep "dac" in backend/annotator finds only the current-steering cell, catalog.rs:1739-1743).

### CC-15 3-dB switching frequency from Elmore delay
- Kind: formula / algorithm
- Statement: f3dB = 1/(2(N+2)·ln 2·τ), with τ = max over bits k of the Elmore delay of the bottom-plate RC network to the units of C_k. The circuit f3dB is the minimum over caps. For the split DAC, C_eq comes from eqs. 33–36 (the series C_A path to the other side array).
- Source: DACP §III-C eqs. 32–36, L445–603; PDF p.5–6.
- Philis stage: cells (variant metric), dr (critical-bit sizing), verify.
- Automation recipe: build the bottom-plate route of C_k as an RC tree (segment R = r·l, via R per cut / n_parallel, C = c·l + unit plate C). The Elmore delay to node v is τ_v = Σ_{e on path(v)} R_e·C_downstream(e). τ_k = max_{u∈C_k} τ_u, and f3dB is as above. For a mesh (parallel wires) use the tree with R/n, or a small nodal solve. Report per bit, and rank bits by τ to drive CC-30.
- Beats hand layout because: the critical bit is identified quantitatively, and the wiring effort goes only there.
- Philis status: missing (no Elmore or f3dB anywhere; grep `elmore|f3db` finds nothing).

### CC-16 Split-DAC electrical model and attenuation-cap value
- Kind: formula
- Statement: C_A = (C_T^LSB/C_T^MSB)·C_u (eq. 1). The unit counts are [1, 1, 2, …, 2^{L−1} | 1, 2, …, 2^{N−L−1}]. Superposition gives V = V^LSB + V^MSB with C_A^MSB = C_A·C_T^MSB/(C_A + C_T^MSB) and C_A^LSB = C_A·C_T^LSB/(C_A + C_T^LSB) (eqs. 6–10). The nonideal versions (eqs. 25–31) add C_A^TB to C_A and the per-array ΔC_T. The unit-bridge alternative [14] costs a 1-LSB gain error. Split versus binary at 8 bits: area 362 against 2509 µm², f3dB 19977 against 1411 MHz, comparable INL/DNL (Fig. 16(c), PDF image).
- Source: DACP eq. 1 L60–76 (PDF p.1); §II-B L177–262; §III-B L365–442; Fig. 16 L1046–1087.
- Philis stage: annotator (recognise the structure), cells, verify.
- Automation recipe: recognition means two binary-weighted cap banks on two different top-plate nets joined by one capacitor whose value is not an integer multiple of the unit. Check C_A ≈ (C_T^LSB/C_T^MSB)·C_u within 1%, and otherwise warn about a gain error. Evaluate INL/DNL with eqs. 25–31.
- Beats hand layout because: the attenuation-cap error and its parasitics are folded into the linearity check automatically.
- Philis status: missing. `bits()` accepts only the single-bank [1,1,2,…] form (kernel/cells/src/cap_array.rs:123-130), and `dac_banks` groups by one top plate (frontend/library/src/cellgen.rs:756-785).

### CC-17 Nonunit MOM capacitor sizing (perimeter-to-area matched)
- Kind: algorithm
- Statement: MOM finger cap C = (εt/(dp))·l·H with l = w − c (c is the end gap) and H = N·p. Under edge bias δ, ΔC/C = 2δ(H + w − c)/(H(w − c)) (eq. 45). Size the nonunit cap (C_A, typically 1–2× C_u in a rectangle) so that its ΔC/C equals the unit's: (H + l)/(H·l) = (H_u + l_u)/(H_u·l_u) and l·H = C·dp/(εt). This gives two equations in (l, H). Snap H to a multiple of p and l to the grid, then re-solve l.
- Source: DACP §III-D eqs. 37–45, L534–616; PDF p.6.
- Philis stage: cells (capacitor generator).
- Automation recipe: let A = C·dp/(εt) and k = (H_u + l_u)/(H_u·l_u). Then H + l = k·A and H·l = A, so H and l are the roots of z² − kA·z + A = 0. Take H ≥ l, snap, and verify. Draw C_A as two half-slots (DACP L844–847). The same method applies to MIM with w in place of w − c ([15], L81–84).
- Beats hand layout because: the ratio error of a non-integer cap is matched to the unit's edge sensitivity exactly, not just by area.
- Philis status: missing (no nonunit sizing; kernel/cells/src/capacitor.rs draws merged plates).

### CC-18 Array sizing, coordinates, dummies, outer ring
- Kind: algorithm / data-model
- Statement: choose r×s as close to square as possible (a square minimises the maximum distance from the centroid). Fill the remaining slots with dummies; for a split DAC, D_C = r·s − (Σn_i + 2). Add an outer dummy ring against edge effects. Unit coordinates are x_k = (s_k − (s+1)/2)(W + S_h) and y_k = ((r+1)/2 − r_k)(H + S_v).
- Source: REV L65–73 and L309–314; DACP L276–284 and L835–847.
- Philis stage: cells.
- Automation recipe: r = ⌈√U⌉ and s = ⌈U/r⌉, or for a power of two dims(m) = (2^{⌊m/2⌋}, 2^{⌈m/2⌉}). Enumerate `tall` for odd m. Tie the dummies' plates to a quiet rail, never to a member's bottom plate (it would load that member alone).
- Beats hand layout because: it is consistent and exact.
- Philis status: implemented. See `dims` (kernel/cells/src/cap_array.rs:194-198), `general_assign` near-square with fill dummies (:140-144, :155-161), the dummy ring tied to C0 or a separate GND bus (:7-8, :402-405).

### CC-19 Spiral CC placement (binary-weighted)
- Kind: algorithm
- Statement: C_0 and C_1 (one unit each, not CC-able) go diagonally opposite at the centre. Then for b = 2..N, walk a spiral outward from the centre and place each unit of C_b at the first free slot together with its point reflection (d1, d2) → (−d1, −d2). Units of one cap align in rows and columns, so the S array needs vias only at the input connection. It has the best f3dB and the worst INL/DNL.
- Source: DACP §IV-A L671–696 (Fig. 6(a), PDF p.7); results L949–956.
- Philis stage: cells.
- Automation recipe: order the cells by (Chebyshev ring scaled to aspect, angle), then assign pairs greedily as above.
- Beats hand layout because: it is constructive, runs in under a second (Python: 1.11 s for 10 bit, Table VI), and is exact.
- Philis status: implemented (kernel/cells/src/cap_array.rs:317-341).

### CC-20 Chessboard CC placement (max dispersion)
- Kind: algorithm
- Statement: C_N takes every black square (2^{N−1} of 2^N). C_{N−1} takes half of the white squares in chessboard fashion. The process alternates colours down to C_2. C_0 and C_1 go at the centre. It has the best INL/DNL and the most vias (5 vertical tracks per channel against 2 for the spiral at 8 bits). Its f3dB is 16 MHz against 1411 for the spiral (8-bit, Table II).
- Source: REV §III-C L338–346 (Fig. 6); DACP L638–644, L931–957; Table II (PDF p.11).
- Philis stage: cells.
- Automation recipe: at each level b, take the remaining cells, colour them by the (r + c) parity of the remaining sub-lattice, and give C_b one colour class in reflected pairs.
- Beats hand layout because: it is exact and exhaustive.
- Philis status: partial. C_N is on the black squares, but lower bits use the farthest-point `spread` instead of alternating colours (kernel/cells/src/cap_array.rs:232-257).

### CC-21 Block-chessboard (BC) placement (Algorithm 1)
- Kind: algorithm
- Statement: the inner full chessboard holds C_0..C_k (small caps: good dispersion, and their via R does not set f3dB). Outer corridors hold (C_i, C_{i+1}) pairs for i = k+1..N step 2, with C_i in b_s×b_s blocks in chessboard order plus its diagonal-symmetric copy, and C_{i+1} in the leftovers. Assume k and N−k even. The pseudocode follows the PDF p.8 listing.
  ```
  rc = ceil(sqrt(Σ_{i≤k} n_i)); sc = ceil(Σ_{i≤k} n_i / rc); chessboard(C0..Ck, rc×sc centred)
  for i in (k+1..=N).step_by(2):
      ri = ceil(sqrt(Σ_{j≤i+1} n_j)); si = ceil(Σ_{j≤i+1} n_j / ri)
      (xs, ys) = (si/2 − sc/2, ri/2 − rc/2)
      for p in 1..=ceil((ri/2)/bs[i]):          // block rows, upper half only
          for q in 1..=ceil((si − sc)/bs[i]):   // blocks across the row
              place bs×bs block of Ci at (xs.., ys..) and at its point reflection
              advance xs to next block start
          xs = if p % 2 == 0 { 0 } else { bs[i] }  // alternate block phase
          ys += bs[i]
      fill corridor(ri×si minus rc×sc) leftovers with C_{i+1}
      (rc, sc) = (ri, si)
  ```
  8-bit results: f3dB 339 MHz, {DNL, INL} = {0.02, 0.07} LSB, NV = 234 against spiral 60 and chessboard 295.
- Source: DACP §IV-A L645–786, Algorithm 1 L702–735 (PDF p.8); Table I/II (PDF p.11).
- Philis stage: cells.
- Automation recipe: enumerate k ∈ {2..N−2} with N−k even and b_s ∈ {1, 2, …}, deduplicate identical assignments, and evaluate each with CC-13/15.
- Beats hand layout because: the whole (k, b_s) family is evaluated, where a human draws one.
- Philis status: implemented with a simplification. The corridor is ordered by block parity and mirrored instead of walking the upper-half blocks explicitly (kernel/cells/src/cap_array.rs:282-306, :343-348; enumeration :95-102).

### CC-22 Handling odd unit counts and singletons
- Kind: rule
- Statement: devices with odd unit counts cannot be CC. Place them as close to the centre as possible. Binary DAC: C_0 and C_1 diagonal at the centre. Split DAC: C_0 opposite a dummy near the centre, and C_1 and C_6 (the odd-count MSB unit) as a symmetric pair. Transistors: one cell at the layout edge (ALIGN [19]). An alternative placement (C_0, C_1, C_2 and a dummy on the inner four) showed no significant difference.
- Source: DACP L674–678, L822–834; REV L439–441.
- Philis stage: cells.
- Automation recipe: collect the odd-count devices. If the grid has an odd cell count, put one at the centre. Pair the rest across the centre two at a time, and if one is left over, pair it with a dummy. Report the residual centroid offset (CC-01) of each odd device as its known systematic error.
- Beats hand layout because: the pairing rule is deterministic and reports its residual error.
- Philis status: implemented (kernel/cells/src/cap_array.rs:151-171 general set; :329-331 spiral C0/C1; :243-253 chessboard C0/C1).

### CC-23 Split-DAC CC placement
- Kind: algorithm
- Statement: (1) C_A's two half-slots go symmetric near the centre, because they are smaller than C_u and the most perturbed by routing C. (2) The odd-count caps go as in CC-22. (3) Along the spiral, alternate LSB and MSB caps: C_2, C_7, C_3, C_8, …, each unit with its reflection. The BC variant uses a chessboard for the small caps and BC for the large ones, with equal block sizes for same-sized LSB/MSB caps (e.g. C_4 and C_9).
- Source: DACP §IV-C-2, L848–878, L810–843; Figs. 10–11 (PDF p.9–10).
- Philis stage: cells.
- Automation recipe: extend `CapArray` with a `split: Option<L>` slot layout. The slot table becomes [C0..C_L | C_{L+1}..C_N | C_A, C_A]. The ordered sequence interleaves the LSB and MSB slot lists and is fed to the existing reflected-pair assigners.
- Beats hand layout because: split DACs have no prior automated CC generator (DACP claim, L123–125).
- Philis status: missing.

### CC-24 Pattern selection by spec (INL/DNL against f3dB against area)
- Kind: heuristic / flow
- Statement: spiral gives the best f3dB and the worst INL/DNL, chessboard the opposite, and BC sits between. At 10 bits: S {0.22, 0.31} LSB at 81 MHz, BC {0.11, 0.30} at 29 MHz, CB {0.05, 0.30} at 0.8 MHz. S fails at large gradients (γ = 100 ppm: 10-bit {4.93, 2.56}). Runtimes are small (≤ 2.25 s in Python for 10 bits), so exhaustive variant evaluation is affordable.
- Source: DACP Table II–III (PDF p.11–12), L934–945, L998–1007; Table VI L1031–1043.
- Philis stage: cells (enumerate), dp (variant choice), flow.
- Automation recipe: for each DAC bank, enumerate all variants (spiral, CB, BC(k, b_s), tall or wide). Compute {3σ INL/DNL at γ_nom and γ_max (CC-13), f3dB (CC-15), area}. Drop variants that violate a spec (|INL|, |DNL| ≤ 0.5 LSB, f3dB ≥ f_s·margin). Among the rest, pick the lexicographic best (area, then −f3dB margin), or hand the Pareto set to dp as alternative macros.
- Beats hand layout because: a designer commits to one style, while the tool proves which style meets the spec at minimum area.
- Philis status: partial. Variants are enumerated (kernel/cells/src/cap_array.rs:94-115) and drawn (frontend/library/src/cellgen.rs:801-806), but `metrics()` is called only in tests (cap_array.rs:699, :786), so selection is not spec-driven.

### CC-25 Allocate dispersion to LSBs and low via count to MSBs
- Kind: heuristic
- Statement: MSB relative variation is lower (σ/μ ∝ 1/√n), so MSBs matter less for accuracy but carry the largest RC and set f3dB. LSBs (and C_0, which every ratio C_k/C_0 depends on) need dispersion and centrality.
- Source: DACP footnote 2 L689–696; L672–677.
- Philis stage: cells.
- Automation recipe: in any constructive or annealed array placement, weight the dispersion cost of C_b by 1/√n_b and the via or bend cost by n_b (or by τ_b from CC-15).
- Beats hand layout because: the effort goes where the error comes from.
- Philis status: partial. It is implicit in the BC core (cap_array.rs:343-348), with no weighting.

### CC-26 Connected unit-capacitor groups (BFS) with mirrored branches
- Kind: algorithm
- Statement: a graph with a node per unit and an edge between 4-neighbours of the same C_i. BFS gives the connected components (groups) and a BFS tree per group. Each tree edge becomes a branch wire, and each branch is mirrored to the diagonally symmetric location.
- Source: DACP §IV-B-1, L748–761 (Fig. 7, PDF p.8).
- Philis stage: cells (in-array routing).
- Automation recipe: `for each slot s: comps = bfs_components(units of s, 4-adjacency)`, then draw the BFS tree edges as same-layer plate-to-plate straps (no vias). The group count per bit is the via or trunk demand and feeds the variant metric.
- Beats hand layout because: the minimum branch count comes by construction, and the wiring stays symmetric.
- Philis status: partial. The cap array routes each unit's bottom plate over a met2 branch to a per-bit met1 track right of its column (kernel/cells/src/cap_array.rs:12-15) instead of merging adjacent same-bit units.

### CC-27 Bottom-plate routing: branch, trunk and bridge wires with track minimisation
- Kind: algorithm
- Statement: bottom plates exit at the array bottom to a clustered switch and driver bank. Branch wires join units within groups or groups to trunks. Trunk wires are vertical channel tracks joining disjoint groups. Bridge wires join trunks at the periphery, symmetrically. The steps are channel selection, track assignment and routing. [46] also runs a routability analysis: find overlapping channel spans between groups, then minimise the maximum tracks per channel with a target of one. More tracks per channel mean a wider pitch and more C_BB (Table I, 8-bit: C_BB 73.3 fF for [8] against 6.1 fF for S).
- Source: DACP §IV-B-2 L763–789 (Fig. 8, PDF p.8); REV L350–361 (Fig. 7); Table I (PDF p.11).
- Philis stage: cells (array-internal), gr/dr (exit to switches).
- Automation recipe: for each channel c, demand(c) = the number of bits whose groups need a trunk in c. Assign trunks to minimise max_c demand(c): a greedy interval-graph colouring over column spans, where the chromatic number equals the maximum overlap. Widen the channel pitch only where demand(c) > 1. Place the bridges below the array in bit order.
- Beats hand layout because: the channel width is minimised globally with a guaranteed optimum for interval colouring.
- Philis status: partial. There is one per-bit track per column channel plus met2 buses below the array (cap_array.rs:12-15), so channel demand equals the bit count rather than the minimum.

### CC-28 Non-overlapped top/bottom routing (kill C_TB)
- Kind: rule
- Statement: route top-plate and bottom-plate wires so they never overlap or run adjacent. C_TB then becomes negligible (Table I omits it for this reason).
- Source: DACP §IV-B L791–795, L937–938.
- Philis stage: cells, dr.
- Automation recipe: in the array, reserve disjoint layer and track sets for TOP and BOT. In dr, forbid top-plate-net segments within the coupling distance of any bottom-plate net (a hard spacing class), or cost them by c_c(s)·l_overlap.
- Beats hand layout because: it holds structurally rather than by inspection.
- Philis status: implemented inside the array (kernel/cells/src/cap_array.rs:15-17, "ARR-03"). There is no dr-level rule for plates leaving the macro.

### CC-29 Top-plate MST to minimise C_TS
- Kind: algorithm
- Statement: a graph of all units with N/S/E/W edges weighted by the spacing between neighbours. The MST is the minimum top-plate wire. When the vertical spacing is below the horizontal (channel) spacing, the MST connects each column with vertical branches and adjacent columns with one horizontal branch each. For a split DAC, BFS first joins adjacent groups, an MST over the remaining groups of each side array follows, and C_A lands on adjacent unit top plates of each side.
- Source: DACP §IV-B-3 L813–826 (Fig. 9(d)); §IV-C-3 L860–881, L898–913.
- Philis stage: cells, dr.
- Automation recipe: run Kruskal over unit adjacency with the physical spacing as weight, then draw the tree on the TOP layer. Metric: C_TS = c_layer × tree length plus the plates' own bottom C.
- Beats hand layout because: minimum top-plate length is guaranteed, so C_TS and gain error are minimised.
- Philis status: partial. TOP is strapped on met3 per column and joined above the array (kernel/cells/src/cap_array.rs:12-15, :562-570); the shape is similar, but there is no MST and no C_TS metric.

### CC-30 Parallel-wire routing of critical bits
- Kind: algorithm
- Statement: in quantised-width (FinFET) layers, route the critical bit with k parallel wires, which gives parallel vias at every turn. For k = 2 the f3dB gain exceeds 2× and lies between 2× (wire-dominated) and 4× (via-dominated: a 2×2 mesh with four vias). The gain diminishes as k grows because wire C rises. Iterate: after the MSB is paralleled, the next bit may become critical. Increase column spacing if the channel lacks room. Binary 8-bit used 4 wires and split used 2.
- Source: DACP L790–795 (right column), L949–953, L996–1013 (Fig. 15, PDF p.12), L1051–1058.
- Philis stage: cells (array routes), dr (width/multi-track), verify.
- Automation recipe:
  ```
  loop {
      k* = argmax_k τ_k (CC-15)
      if f3dB >= target or wires[k*] == k_max { break }
      wires[k*] += 1   // add a parallel track + via array at each bend
      if channel(k*) full { widen column pitch by one track }
      recompute τ
  }
  ```
  Generalise to dr: a net with an R budget gets n parallel minimum-width tracks when the layer's widths are quantised, instead of a wide wire.
- Beats hand layout because: the wire count is the minimum that meets the frequency target, per bit.
- Philis status: missing. dr has no multi-track routing of one net (the grep found only the shield parallel track at backend/dr/src/lib.rs:1552-1575).

### CC-31 Via count and critical-bit R as first-class metrics
- Kind: metric
- Statement: with reserved-direction layers every bend is a via. ΣN_V and ΣL correlate with R. f3dB depends only on the critical bit's R_V and R_total. At 8 bits S has R_total = 0.16 kΩ, BC 0.67 and CB 14.0.
- Source: DACP L655–670, L938–956; Table I (PDF p.11).
- Philis stage: cells, dr, verify.
- Automation recipe: report per-net via count and R_total (wire plus cuts over parallel count) for every array bit and every matched pair net. Use them in the variant cost and the dr cost.
- Beats hand layout because: every net is counted, including the ones nobody checks.
- Philis status: partial. The cap array counts via1 cuts per slot (kernel/cells/src/cap_array.rs:71-72, :368). `Differential` compares via counts between pos and neg (kernel/analog/src/routing/differential.rs:12-15).

### CC-32 Attenuation-cap placement and routing (split DAC)
- Kind: algorithm
- Statement: C_A is drawn as two half-size slots near the centre. If they are adjacent, join them during group formation; otherwise assign an extra vertical track. Connect C_A to adjacent top plates of each side array, which keeps the LSB/MSB top-plate wiring minimal.
- Source: DACP §IV-C-3 items 2–3, L898–913, L876–881.
- Philis stage: cells.
- Automation recipe: part of the CC-23 generator. Place the C_A slots adjacent (same row, centre columns) whenever the grid allows, so no extra track is needed.
- Beats hand layout because: it follows directly from the construction.
- Philis status: missing.

### CC-33 Unit capacitor structure choice
- Kind: rule
- Statement: MOM over MIM in multi-metal and FinFET nodes (cost, density, no via stack). Finger MOM has the highest density and is FinFET-friendly but exposes C_TS and C_TB. Sandwich, pillar and mortise-tenon enclose the top plate in the bottom (C_TS eliminated, easier routing, lower density). A parameterised mortise-tenon generator exists [28].
- Source: REV §III-B L301–330, L297–305 (Fig. 5); DACP L102–111.
- Philis stage: cells, deck.
- Automation recipe: expose the unit style as a variant axis {plate stack, finger, enclosed}. Where the top-plate node is `Sensitive` (CC-14), prefer the enclosed style unless area is binding. C_u comes from the deck's per-layer coupling (stack.rs).
- Beats hand layout because: the style is chosen per bank against its C_TS budget.
- Philis status: partial. There is one style, BOT met1 plus TOP met2 inset plus met3 strap (kernel/cells/src/cap_array.rs:12-13). There are no finger or enclosed variants.

### CC-34 Parasitic model for array evaluation
- Kind: data-model
- Statement: a segment of length l has R = r·l and C = c·l. The coupling between adjacent segments at spacing s is c_c(s)·l_overlap. Per-unit r, c and c_c come from the foundry per layer.
- Source: DACP L899–908.
- Philis stage: cells (metrics), gr/dr.
- Automation recipe: use the existing stack model for r, c and c_c(s) in the in-generator metrics (CC-13/15), so pre-route choices match post-route PEX.
- Beats hand layout because: estimates are consistent across the generator, the router and signoff.
- Philis status: partial. Lateral coupling per gap is at kernel/analog/src/routing/stack.rs:15; `metrics()` does not use it.

### CC-35 LOD (STI stress) equality for matched devices
- Kind: formula / check
- Statement: ΔV_th ∝ Σ_{i=1..n}[1/(SA_i + 0.5L_g) + 1/(SB_i + 0.5L_g)]. Matched devices need equal sums. In a single CC row, the inner cells of B see different SA/SB from the outer cells of A, which is inherent LOD mismatch. Clustered and ID rows match. The fix is two rows with A and B swapped in the second row, or dummies.
- Source: REV eq. 11 L103–117; §V-A L428–437, L448–453.
- Philis stage: cells, dp (rule), verify.
- Automation recipe: compute the per-device Σ LOD term from the drawn diffusion ends. The check is |ΔV_th,LOD| ≤ η·σ_rand using the deck's KVTH0. When it fails, offer the 2-row swapped variant (CC-04 n = 2) or add end dummies.
- Beats hand layout because: the stress term is computed, not guessed.
- Philis status: implemented (kernel/analog/src/placement/cc.rs:37-41, :123-141; kernel/cells/src/mosfet.rs:288).

### CC-36 WPE, OSE and gate-pitch rules
- Kind: rule
- Statement: WPE: keep well edges far from matched devices, or equal for all of them. OSE: identical OD width and OD spacing. Gate pitch: identical poly pitch. Identical unit cells in rows with diffusion sharing cancel every LDE except LOD and WPE.
- Source: REV §II-A-2 L149–152, L98–102, L118–145.
- Philis stage: cells, dp (rule), deck.
- Automation recipe: WPE check: for each matched group, compute d_well(device) as the minimum distance from the channel to each well edge on 4 sides. Require max − min ≤ tol, or all d ≥ the deck's `wpe_min_dist` (a deck key; value not given here). OSE and pitch hold by construction for unit-cell generators; assert them in the cell self-check.
- Beats hand layout because: the equal-distance condition is checked on every matched group.
- Philis status: missing for WPE (grep `wpe|well.proximity` returns nothing). OSE and pitch are implicit in unit generators.

### CC-37 ALIGN ratio-greedy 2D CC placement of transistor arrays (LOD-aware)
- Kind: algorithm
- Statement: input a multiplicity vector M (units per device) and a row width. Build M_half = M/2 (odd M: one unit of that device is placed at the edge). Loop: Ratio_d = unplaced_d / M_half_d. Pick the device with maximum Ratio that can share diffusion with the current row end. If that device already occupies this column in another row, prefer another device (LOD). Place its cells alternately left and right of the CC point. When the row is full, move to the next row. Finally reflect the half layout about the CC point. Routing is a fishbone with EM/IR-optimised widths.
- Source: REV §IV L445–464, L406–407, Fig. 8(c)–(h) (PDF p.6); L439–441.
- Philis stage: cells (mosfet, finfet).
- Automation recipe:
  ```
  half = M.map(|m| m/2); U = M.map(|m| m%2)         // odd leftovers → edge cells
  grid = rows × cols_half; place U units at the outer boundary first
  while any half[d] > placed[d]:
      ratio[d] = (half[d] - placed[d]) / half[d]
      cand = devices sorted by ratio desc, filter(can_share_diffusion(row_end, d))
      d = first in cand not already in this column (other rows); else cand[0]
      put d at next slot, alternating left/right of the row's CC column
      if row full: next row
  full = half ∪ reflect180(half)                      // CC by construction
  ```
  Score the candidates with CC-02 (moment order) and CC-35 (LOD).
- Beats hand layout because: it handles k-device ratioed banks (e.g. [2,2,4,8,8]) in 2D with LOD balancing, which is tedious by hand.
- Philis status: partial. `greedy_centroid` (kernel/cells/src/builder.rs:271-292) is 1D, fills both ends by most-remaining, and is not LOD-aware. `centroid_sequence` covers equal counts only (kernel/cells/src/mosfet.rs:720-751). There is no multi-row ratio placement.

### CC-38 Diffusion-sharing half graph and Euler path
- Kind: algorithm
- Statement: vertices are S/D nets and edges are fingers (edge multiplicity = finger count). Halve the edge counts to get M_half. An Euler path on M_half is a diffusion-shared ordering of half the fingers. Reflect it about the CC point to get the full row. [2] enumerates Euler paths, which is expensive; [19] replaces the enumeration with the greedy of CC-37.
- Source: REV §IV L396–444 (Fig. 8(b)).
- Philis stage: cells.
- Automation recipe: use Hierholzer's algorithm (O(E)) on the M_half multigraph, choosing at each step the edge whose device has the highest Ratio (combining with CC-37). An odd vertex count greater than 2 means breaks are needed, so insert dummies or gaps and report them.
- Beats hand layout because: the diffusion-sharing order is found in linear time.
- Philis status: partial. Boundaries are forced to land on source regions (kernel/cells/src/mosfet.rs:724-728). There is no general Euler path.

### CC-39 Fishbone EM/IR-aware routing of CC transistor arrays
- Kind: algorithm
- Statement: route each device's S/D terminals as a fishbone (spine plus ribs) whose wire widths are optimised against EM and IR drop [19]. For CC arrays, a parasitic-aware MST is an alternative [50].
- Source: REV L448–452 and L422–423.
- Philis stage: cells, dr.
- Automation recipe: spine on the upper layer along the row, ribs per finger on the lower layer. Size the spine width from the device current (op point) with the existing EM `Limit::width_nm` and via cuts `Limit::cuts`, and make the spine R equal across matched devices.
- Beats hand layout because: widths are sized from real currents with equal R.
- Philis status: partial. EM sizing helpers exist (kernel/analog/src/routing/em.rs:48, :65), but there is no fishbone template in the MOS array.

### CC-40 When not to use CC: CC against interdigitated against clustered
- Kind: heuristic
- Statement: CC in one row has inherent LOD mismatch and S/D wire mismatch (R_s). Interdigitated rows give the best G_m for a DP (equal R_s), CC comes second and clustered is worst. Small devices (StrongARM, 1.6–6.1 µm/14 nm; PMOS CM 2.3 µm/14 nm) do best clustered. Large devices (DP 80 units/device, CM 40) do best as multi-row CC (4 rows) that matches LOD and parasitics. With R_L = 1000 µm the u term dominates and pattern barely matters for σ. With R_L = 10 µm clustering hurts.
- Source: REV §V L425–503, L516–531, L472–498.
- Philis stage: annotator (group intent), cells (enumerate), dp (select).
- Automation recipe: offer {clustered, ID (pair-level AABB with a shared source), 1-row CC, multi-row swapped CC, nth-order} for every matched group. Score each with σ_mismatch = √(A²/(WL_unit·n) + σ_s²·f(pattern, R_L)) + |LOD term| + |ΔR_s·I|·g_m (CC-41) + parasitic C at sensitive nodes (StrongARM X/Y). Keep the minimum. Do not force CC by class.
- Beats hand layout because: the choice between CC and non-CC is computed per instance, which the review says is an open problem for designers (L507–509).
- Philis status: partial. Single, Cc1d, split-gate and 2-row variants are enumerated (kernel/cells/src/mosfet.rs:84-131). "Interdig is never offered" for MOS (mosfet.rs:87-88) because of drain shorts, so the pair-level interdigitated DP option is missing. Selection comes from rule costs, not the metric above.

### CC-41 Effective G_m with source-interconnect resistance
- Kind: metric
- Statement: G_m = g_m(v_in − i_ac·R_s)/v_in, where R_s is the parasitic R from each source to the AC ground (the point where the small-signal currents cancel). Clustered patterns accumulate current through R_s. In CC, A and B see different R_s.
- Source: REV eq. 25, L454–465; L491–503 (Fig. 9, PDF p.7).
- Philis stage: cells, verify (post-route).
- Automation recipe: per DP variant, build the source rail as a resistor ladder with one current injection per unit (+I_UA for A units, −I_UB for B units). Locate the node where the net current is 0 (AC ground). Compute v_s at each unit and G_m,A and G_m,B. Report the G_m loss and the A/B G_m mismatch; the mismatch is an input-offset contribution through g_m. The op point supplies I_U.
- Beats hand layout because: this quantifies an effect designers only reason about qualitatively.
- Philis status: missing.

### CC-42 FinFET gate-misalignment-aware orientation
- Kind: rule
- Statement: gate misalignment shifts the printed gate, which raises V_th and lowers I_D. Choosing the orientation of every FinFET unit in a CM or DP makes the current ratios match exactly [50, 55]. Placement stays diffusion-sharing aware and dispersion-maximising, and routing uses a parasitic-aware MST.
- Source: REV §IV L408–423.
- Philis stage: cells (finfet), deck.
- Automation recipe: treat each unit's orientation (S-left or S-right) as a binary variable. Require that for each device the count of each orientation is equal (or proportional to the ratio), so the misalignment shift cancels per device. Add the constraint to the CC-37 assignment. The misalignment sensitivity itself is not given in REV (deck key needed).
- Beats hand layout because: it enforces orientation balance per device automatically.
- Philis status: missing (kernel/cells/src/finfet.rs has no pattern or orientation logic).

### CC-43 Stochastic and ILP CC placement for arbitrary ratios (fallback)
- Kind: algorithm
- Statement:
  - Pair-sequence SA [13, 46]: the placement is a list of symmetric unit pairs in nonincreasing distance from the CC point. Perturbing it gives a different CC layout with the same dimensions; three ops increase dispersion, and a repair step restores adjacency feasibility for non-integer ratios.
  - C-CBL [45]: grid based; SA over the global sequence pair.
  - ILP [38]: exclusivity, exact ratio counts, and each unit choosing one track in one of its four adjacent channels.
  - GA over a CP-sequence [27] with fitness Φ = (α·C_unit/C_unit,max + β·DNL_max/0.5 + γ·INL_max/0.5)^{−1}, set to ∞ penalty when DNL or INL exceeds 0.5 LSB, plus a shield ILP that keeps ratios ideal.
- Source: REV §III-C L323–377 (eq. 24, PDF p.5).
- Philis stage: cells (general sets), dp.
- Automation recipe: keep the constructive generators as seeds. For general (non-binary, non-power-of-two) sets, run SA over the pair sequence with the move set {swap two pairs, swap within a device across rings, rotate a ring}. The cost is w1·(1 − ρ_avg) + w2·M_sys + w3·Σ τ-or-vias + ∞·[violates 0.5 LSB], and the CC property is kept by construction.
- Beats hand layout because: it gives a near-optimal array for ratios that have no textbook pattern.
- Philis status: partial. `general_assign` is greedy (ring order or farthest-point) with no optimisation (kernel/cells/src/cap_array.rs:140-192).

### CC-44 Parasitic ratio matching (wirelength proportional to capacitance) and ratio-preserving shields
- Kind: rule
- Statement: hybrid chessboard [43] (PASTEL) matches the routing wirelength per bit to the capacitor ratio. Any parasitic C then scales with C_k and preserves ratios. A shield ILP [27] places shields so that the parasitic-inclusive ratios stay ideal.
- Source: REV L349–353 and L375–377.
- Philis stage: cells, dr.
- Automation recipe: metric L_k/n_k = constant over bits (max/min − 1 → 0). Add dummy wire length to low bits, or reroute, until the spread is ≤ tol, trading against f3dB (CC-15). Where the top-plate route passes over bottom-plate routes, prefer shields tied to a quiet net, and apportion the shield coverage per bit in proportion to n_k.
- Beats hand layout because: ratio-preserving parasitics hold exactly on all bits.
- Philis status: partial. `route_spread` measures bottom-route length per unit, max/min − 1 (kernel/cells/src/cap_array.rs:68-70, :392-394), but it is not optimised. `Shield` is one-sided and has no plate awareness (kernel/analog/src/routing/shield.rs:18-19).

---

## 4. Top-15 priorities for Philis

1. **CC-02 / CC-03**: replace the ⟨r²⟩ proxy and the 1% coincidence tolerance with the exact moment-order metric (max cancelled gradient order plus the first uncancelled residual) for every matched group. It is cheap and pure, and it fixes a real blind spot: equal ⟨r²⟩ does not cancel the x²−y² or xy gradients.
2. **CC-04**: nth-order central-symmetric generator (the recursive rotate180-and-swap construction) for 2-device MOS, FinFET, resistor and cap pairs. It extends the existing 2-row ABBA/BAAB to 4 or more rows (order 3–5) at no area cost, and the NTH Table I shows the residual collapsing from 5.22% to 0 at 3rd order.
3. **CC-08 / CC-09 / CC-10**: deck keys (A_f, ρ_u, L_c, γ, R_L) plus the ρ_pq/ρ_avg/M_rand metrics. They make dispersion a number and replace the farthest-point heuristic's implicit objective.
4. **CC-13 / CC-12**: the full DAC INL/DNL evaluator (nonlinear t0/t gradient, 3σ correlated random terms, C_TB and C_TS), with a ±0.5 LSB signoff check. It turns the cap array from "pattern-correct" into "spec-proven".
5. **CC-24**: drive DAC variant choice by the spec using the metrics, which today run only in tests (cap_array.rs:699, :786). This is cheap once #4 exists and gives the S/BC/CB trade from the paper automatically.
6. **CC-15**: Elmore-based f3dB per bit and the critical-bit identification. It is the missing half of the DACP trade-off and is needed by #7.
7. **CC-30**: parallel-wire routing of critical bits, in the array and as a dr option for quantised-width layers. It gives more than 2× f3dB for k = 2 in FinFET nodes (DACP Fig. 15).
8. **CC-40 / CC-41**: per-instance CC against ID against clustered selection with the G_m/R_s metric. The review shows CC loses for small devices, and Philis currently never offers the ID DP.
9. **CC-37 / CC-38**: 2D ratio-greedy, LOD-aware, Euler-path CC placement for k-device transistor banks. The current 1D greedy cannot handle [2,2,4,8,8]-style mirror banks in multiple rows.
10. **CC-16 / CC-17 / CC-23 / CC-32**: split-DAC recognition, attenuation-cap sizing (perimeter-to-area quadratic), placement and routing. 8-bit split versus binary is 362 against 2509 µm² at 14× the f3dB.
11. **CC-29 / CC-14**: top-plate MST with a C_TS metric, plus annotator net roles for the DAC top and bottom plates, so that gr/dr cost C versus R per plate correctly.
12. **CC-05 / CC-06**: exact t0/t gradient with a fine θ sweep, plus M_sys for non-DAC ratio banks (SC filters and gain caps), where INL/DNL does not apply.
13. **CC-36**: WPE equal-distance check for matched groups. It is one of only two LDEs that unit cells do not cancel, and it is absent today.
14. **CC-27 / CC-26**: BFS group merging and interval-colouring trunk assignment. They minimise channel tracks (C_BB 6.1 against 73.3 fF at 8 bits) and cut array width.
15. **CC-42 / CC-43**: FinFET orientation balancing, and a pair-sequence SA fallback for arbitrary-ratio arrays seeded by the constructive patterns.
