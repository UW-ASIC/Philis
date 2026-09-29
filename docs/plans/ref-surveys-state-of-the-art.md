# State of the art in analog layout automation: performance-driven survey (ASP-DAC 2024) and performance-driven wire sizing (TODAES)

Sources:
- **[PDS]** P. Xu, J. Li, T.-Y. Ho, B. Yu, K. Zhu, "Performance-Driven Analog Layout Automation: Current Status and Future Directions (Invited Paper)", ASP-DAC 2024, pp. 679–685, DOI 10.1109/ASP-DAC58780.2024.10473859. Reftext `perf_driven_survey.txt`, lines 1–553 (all). PDF p1 = L1–67, p2 = L68–131, p3 = L132–196, p4 = L197–263, p5 = L264–339, p6 = L340–472, p7 = L473–553. The PDF is two-column and `pdftotext -layout` interleaves the columns on the same lines, so a cite such as "L99–125 (right col.)" names the column.
- **[WS]** Y. Li, Y. Lin, M. Madhusudan, A. Sharma, S. S. Sapatnekar, R. Harjani, J. Hu, "Performance-driven Wire Sizing for Analog Integrated Circuits" (Texas A&M / Univ. of Minnesota, the ALIGN group). The file is named `todaes22` and is cited as ACM TODAES. The extracted text does not print the venue or year. Reftext `todaes22.txt`, lines 1–1140 (all). PDF p1 = L1–40, p2 = L41–75, p3 = L76–108, p4 = L109–143, p5 = L144–196, p6 = L197–236, p7 = L237–291, p8 = L292–329, p9 = L330–384, p10 = L385–433, p11 = L434–474, p12 = L475–516, p13 = L517–559, p14 = L560–591, p15 = L592–639, p16 = L640–686, p17 = L687–730, p18 = L731–763, p19 = L764–812, p20 = L813–858, p21 = L859–887, p22 = L888–928, p23 = L929–971, p24–26 = L972–1067 (refs), p26–27 = L1070–1140 (appendix).
- The PDF pages were opened to read figures that the text extraction garbled: [PDS] p5 (Fig. 2 and Fig. 3) and [WS] p2–3 (the tables inside Fig. 1 and Fig. 2, and the Fig. 3 bars). Numbers taken from those images are marked "(figure image)".

## 1. Coverage

Read chunks, all with the Read tool:
- `perf_driven_survey.txt`: offset 1, limit 553 → L1–553 (the whole file).
- `todaes22.txt`: offset 1..400 → L1–400; offset 401..800 → L401–800; offset 801..1140 → L801–1140 (the whole file).
- PDF images: [PDS] p5; [WS] p2–3.

Headings in [PDS]:
- Abstract; Fig. 1 "Typical analog physical design flow" (L15–37)
- I. Introduction (L30–93, left col. p1–p2)
- II. Related Work (L95–116, left col.)
- II-A Conventional analog physical design (L118–125 left col.; L70–97 right col.)
- II-B Recent Developments in Performance-aware Analog Physical Design (L99–125 right col.; L134–182 left col.)
- II-C Machine Learning-Based Analog Performance Modeling (L184–190 left col.; L134–158 right col.)
- III. Case Study on MAGICAL-Generated Layouts (L160 right col.)
  - III-A Introduction to the Analog Performance Modeling Lifecycle (L162–190 right col.; L199–213 left col.)
  - III-B Exploring Model Transferability in Performance Modeling on OTA Designs (L215–258 left col.; Table I L199–212 right col.)
  - III-C Navigating the Multi-Objective Pitfall in Post-Layout Performance Optimization (L217–252 right col.)
- IV. Perspectives and Future Directions (L254–258 right col.; L302–304)
  - IV-A On Modeling Performance: Efficient Data Acquisition, Better transferability (L306–333 left col.); network architecture and pretraining (L282–310 right col.)
  - IV-B On Optimization Physical Design: Placement and Routing Representation, Multi-objective optimization (L312–333 right col.; L342–343)
- V. Conclusion (L345–358)
- References [1]–[81] (L360–547)
- Figures: Fig. 2 runtime breakdown on OTA1 (L266–279), Fig. 3 number of top-1 metrics (L283–299)

Headings in [WS]:
- Abstract (L1–20)
- 1 Introduction (L23–143); Fig. 1 (L65–66), Fig. 2 (L81), Fig. 3 (L101)
- 2 Problem Formulation (L146–184), Eq. 1–3
- 3 Overview of the Proposed Approach (L186–215)
- 4 GNN-Based Performance Model (L217)
  - 4.1 Notations and Background on GNN (L218–291), Eq. 4–10
  - 4.2 Circuit Graph and Features (L294–329), Fig. 4
  - 4.3 Wire Attention Graph Network (WAGN) (L340–433), Fig. 5, Eq. 11–17
  - 4.4 Handling Different Topologies and Model Complexity (L436–454)
  - 4.5 Training of WAGN Model (L459–474), Eq. 18–19
- 5 Performance-Driven Wire Sizing Optimization (L477)
  - 5.1 Optimization Strategy (L478–498), Eq. 20
  - 5.2 TensorFlow Training-Based Optimization (L501–528), Eq. 21, Fig. 6
  - 5.3 WAGN-guided Bayesian Optimization (L530–566), Algorithm 1
- 6 Experiments (L568–646); Fig. 7, Table 1, Table 2, Fig. 8
  - 6.1 Results on ML Performance Models (L647–711), Tables 3–5
  - 6.2 Results on Wire Sizing (L713–858), Fig. 9, Tables 6–10, Fig. 10
  - 6.3 Parameter Analysis for TF and BO-based Optimizations (L878–887), Figs. 11–12
  - 6.4 Comparison with a Manual Annotated Method (L909–920), Fig. 13, Table 11
- 7 Conclusion and Future Research (L924–942)
- References [1]–[50] (L945–1067)
- Appendix: Tables 12–13 (L1070–1140)

## 2. Section-by-section digest

### [PDS] Abstract and Fig. 1 (L15–37, p1)
- Optimizing circuit performance is the pivotal open challenge in automatic analog physical design. Performance is sensitive to the layout, and there is often no way to optimize it directly (L15–19).
- Emphasis is on black-box optimization of analog performance, plus a case study of the post-layout performance distribution of layouts generated by MAGICAL (L21–28).
- Fig. 1 flow: Netlist + Spec → Placement → Routing → Parasitic Extraction → Simulation → Performance. Performance-driven design must bring post-layout performance into placement and routing (L16–37 right col.).

### [PDS] I. Introduction (L30–93, p1–p2)
- Academic analog P&R conventionally optimizes proxy objectives (wirelength, area), which do not show performance or manufacturability capability (L39–44).
- Three generator families: template-based (BAG [2], LAYGO [3], [4]), which use user-defined patterns or floorplans; digital-P&R-based [5], [6]; analog-P&R-based (MAGICAL [7], [8] and ALIGN [9]), which are fully automated and treat physical design as optimization. The first two either limit the circuit architecture or need extensive manual effort (L47–58).
- Silicon demonstrations exist for template methods [11]–[15], digitally synthesized methods [16], and P&R methods [17], [18]. A general way to account for post-layout performance "remains an open question" (L40–44 right col.).
- Manual design relies on experience and trial and error. Templates need human effort. Automatic P&R "intrinsically optimize[s]" and can be fully automated and performance-driven (L45–55 right col.).
- No universal performance model (the analog analogue of static timing) is assumed to exist [19] (L75–79).

### [PDS] II. Related Work (L95–116)
- ILAC [20] (MOSAIC, simulated annealing for placement and routing) was an early optimization-based generator. In traditional approaches performance is reached only indirectly, through proxies (L96–106).
- The section is split into: A, conventional; B, performance-aware; C, ML performance modeling (L105–116).

### [PDS] II-A Conventional analog physical design (L118–125 left; L70–97 right, p2)
- Analog placement is treated as floorplanning that optimizes area and wirelength under analog constraints (L119–125, L70–71 right).
- Placement constraints: symmetry [20]–[22], symmetry islands [23], common-centroid [24], array-like regularity [25], [26], proximity [24], [27], boundary [28], [29] (L71–79 right).
- Routing constraints: symmetric pair routing [21], [30]–[40]; no routing over transistor active regions [31], [33]; power routing [41], [42]; shielding of critical nets [36] (L80–86 right).
- Exact routing [43], [44] requires equal wirelength of the pair on every layer so the wiring parasitics match precisely. Chen et al. [38] maximize the degree of symmetry without an explicit constraint (L86–91 right).
- Survey pointer [45]. These constraints and objectives "do not guarantee post-layout circuit performance" (L92–97 right).

### [PDS] II-B Performance-aware physical design (L99–125 right; L134–182 left, p2–p3)
- Wire-load minimization in routing: planar routing [35], and routing-aware incremental placement [50] that reduces via usage (L105–110 right).
- Monotonic current-flow placement [51]–[54]. Signal-flow-aware AMS placement [48] reports post-layout gains (L111–119 right).
- LDE-reducing placement for WPE, LOD and OSE [55]–[57]. WPE ties into well-island generation during placement [58]–[60]. Thermal-aware placement [61], [62] (L120–125 right; L134–140 left).
- Retargeting pulls geometric constraints from manual layouts into templates [63] but cannot handle unseen designs (L141–150).
- "Learn-from-human" methods: GeniusRoute, a VAE that predicts the routing regions designers chose and turns them into detailed-router guidance [37]; a GAN that learns manual well design for placement [60]; a VAE for primitive-cell synthesis [64]. Their limits are small datasets and no direct performance optimization (L151–168).
- Direct optimization [65]: layout generation plus simulation is treated as a black box, and Bayesian optimization (BO) tunes the **net weights**. This finds high-quality layouts, but simulation is slow, so it suits small circuits (L169–182).

### [PDS] II-C ML-based performance modeling (L184–190 left; L134–158 right, p3)
- Extraction and simulation inside the loop are expensive, and optimizing a neural-network output is cheaper (L185–188).
- [19]: MAGICAL generates tens of thousands of placements of one circuit. They are rasterized to images and a CNN predicts performance, with transfer across schematics. NAS/AutoML improves the accuracy further [66] (L189–190; L134–143 right).
- [67]: performance-driven placement from star-model wirelength estimation. It is inaccurate because it is not trained on real layout data (L145–150 right).
- A GNN performance model trained on simulated labels transfers across designs and is combined with SA placement that optimizes the predicted performance (cited as [46] in the text; the GNN paper is [80] in the list) (L151–158 right).

### [PDS] III-A Performance-modeling lifecycle (L162–190 right; L199–213 left, p3–p4)
- There are three stages: data acquisition (P&R, PEX, post-layout simulation), training, and inference. The layout is the input x and the simulated performance is the label y (L164–176 right).
- OTA case (Fig. 2): data collection is 92.89% of the lifecycle and training plus inference is 7.11% (L180–184 right). The Fig. 2(b) pie reads D:PEX+Post-Sim 75.42%, D:PNR 22.55%, Inference 0.11%, and a Train slice labeled "7%" (figure image). The pie and the text disagree. Fig. 2(a) shows about 360 min in total for OTA-1 (figure image).
- Obtaining one label (PEX + post-sim) costs about the same as 3–4 P&R iterations (L188–200).
- Remedies: parallelize PEX and post-sim (hardware-accelerated EDA [68], [69]) and use active learning [70] (L201–213).

### [PDS] III-B Transferability (L215–258 left; Table I L199–212 right, p4)
- Two scenarios: the same topology with different sizing, and different topologies. Modes: From Scratch (a small sample, cast as binary classification with balanced sampling [71]) and Transfer (fine-tune a pretrained model) (L219–240).
- Table I, accuracy from scratch vs transfer. OTA1: offset 95.54/91.67, CMRR 91.96/77.68, BW 96.43/95.54, DC gain 93.62/88.01, noise 91.96/79.14. OTA2: offset 81.35/65.39, CMRR 82.33/62.02, BW 80.71/72.14, DC gain 81.35/59.50, noise 88.80/69.29 (L199–212 right).
- Transfer loses 3–22% accuracy. Transfer across sizings is easier than across topologies (L241–251).
- Future directions: meta-learning for pretraining weights [72] and OOD detection [73] to decide when transfer is valid (L252–258; L214–215 right).

### [PDS] III-C Multi-objective pitfall (L217–252 right, p4; Fig. 3 p5)
- Common practice is a user-defined FOM, a weighted sum of post-layout metrics. It cannot express trade-offs between conflicting metrics. Pareto-optimal solutions are the alternative (L224–238).
- Weighted-BO vs MOBO [74] on four OTAs. MOBO is top-1 in almost all of offset, CMRR, BW and DC gain, and beats weighted-BO on 3–5 metrics for every design (L239–246).
- Fig. 3 top-1 metric counts, Weighted-BO vs MOBO: OTA1 1 vs 4, OTA2 2 vs 3, OTA3 0 vs 5, OTA4 1 vs 4 (figure image).
- Gradient-based multi-objective methods [75]–[77] (Fliege–Svaiter steepest descent, MGDA) are named as candidates (L247–252).

### [PDS] IV-A On modeling performance (L306–333 left; L282–310 right, p5)
- Efficient data acquisition: active learning selects which layouts to simulate (L307–320).
- Faster simulation: sparse LU with density-aware matrix multiplication [78], and an RL-based stochastic stepping policy for PTA [79] (L321–326).
- Transfer holds well across sizings of the same netlist and less well across topologies (L327–333).
- Inputs are multimodal: schematic, placement and routing. Prior work used a CNN for placement [19] and a GNN for topology [80]. A general multimodal network is suggested (L290–301 right).
- Pretrain-then-finetune (TAG [81]) raises data efficiency for layout-constraint and parasitic prediction (L302–310 right).

### [PDS] IV-B On optimization physical design (L312–333 right; L342–343, p5–p6)
- The representation of P&R for ML-driven optimization is overlooked. [80] uses randomized SA to generate placements that the model scores. [65] tunes net weights as a proxy knob, because they set net priority and so shape the layout (L313–324).
- Many analog metrics compete, so efficient multi-objective physical-design optimization is essential. MOBO beats weighted-BO (L325–343).

### [PDS] V. Conclusion and references (L345–547, p6–p7)
- Two priorities: efficient ML performance models, and better optimization algorithms in physical design (L347–358).
- The references name the canonical systems: MAGICAL [7] (ICCAD'19), [8] (TCAS-II'23), MAGICAL 1.0 silicon-verified with a 40-nm 1 GS/s ΔΣ ADC [17], 1 and 80 MS/s SAR ADCs in 40 nm [18]; ALIGN [9] (DAC'19); BAG [2]; LAYGO/LAYGO2 [3], [4]; an automated SerDes front end at 15 Gb/s and 1.96 pJ/bit in 16 nm [14]; a temperature-sensor generator verified on 64 instances in SkyWater 130 nm [16] (L360–467).
- Well-island generation and well-tap insertion [59] (TODAES'23); WellGAN [58]; RL-guided detailed routing [39]; AutoCRAFT [40]; silicon-proven AMS detailed routing [38]; exact route matching [43], [44]; LDE [55]–[57]; monotonic current [51]–[54], including pole-aware placement with wire crossings [53] (L475–547).

### [WS] Abstract and 1 Introduction (L1–143, p1–p4)
- Minimum-width wires have high resistance in recent nodes, so wire sizing strongly affects analog performance. The paper contributes a GNN performance model (WAGN), BO guided by WAGN, and TensorFlow-based optimization. Results: +11% performance, or 8.7× speedup, vs conventional BO (L9–15).
- Geometric or parasitic constraints alone are "either inadequate or overly tight". This is named as a main cause of the gap between manual and automatic layout (L30–34).
- 7-nm OTA (Fig. 1): widening L1/L2, the nets in series with the diff-pair sources, from 1× to 3× raises UGF from 688.5 to 972.0 MHz and gain from 23 to 25 dB. The Fig. 1 table (figure image) reads, for schematic / 1× / 2× / 3×: gain 26.2 / 23.0 / 24.5 / 25.0 dB; UGF 1223.0 / 688.5 / 881.4 / 972.0 MHz; BW 60.0 / 48.1 / 52.6 / 54.5 MHz; PM 90.45 / 92.2 / 91.4 / 91.1°; power 179.6 / 178.0 / 178.8 / 179.1 µW. Mechanism: series wire R degenerates the effective Gm of the diff pair (L46–53).
- Fig. 2, two-stage OTA: wire sizing alone improves FOM by 11.5%, and wire sizing after transistor sizing by 19% (L54–60). The Fig. 2 table (figure image) reads FOM 0.52 unsized, 0.58 wire-sized, 0.69 transistor-sized, 0.79 WS+TS (spec gain 40 dB, UGF 4.5 GHz, BW 65 MHz, PM 45°). 0.58/0.52 = +11.5%. 0.79/0.69 is only +14.5%, so the image does not reproduce the text's 19%.
- Earlier wire sizing targeted EM [22], [23], [38] and IR [42], or digital timing through Elmore [9], [13], [36]. Analog lacks a fast credible model (L68–75).
- Post-layout evaluation is up to about 10× slower than schematic (L91–96). Fig. 3 bars, total runtime in s (figure image): OTA pre-layout 1, post-layout 71; comparator pre-layout 20, post-layout 233.
- Parasitic-aware layout: capacitance sensitivity in placement [48]; coupling reduction in routing [16]; routing that prioritizes R-sensitive nets [35]; routing R minimized by balancing layer changes against wirelength [10]; wire detouring for parasitic mismatch (LEMAR [47]). None of these directly addresses performance (L115–120).
- Twelve testcases in 4 families. WAGN raises TPR from 77.1% to 89.5% vs PEA [20]. Performance improves 8–21% vs automated layout without sizing, and the method is integrated with an open-source router (L126–143).

### [WS] 2 Problem Formulation (L146–184, p5)
- Wire sizing is applied to a given global-routing solution. Widths are discrete integers because of multiple patterning: s ∈ Z^C, s_L ≤ s_i ≤ s_U, maximize FOM(s) (Eq. 1, L147–157).
- FOM = Σ_{i=1..M} w_i·z̃_i with Σ w_i = 1 (Eq. 2). z̃_i = min(z_i/φ_i, 1) for bigger-is-better metrics (Π+) and min(φ_i/z_i, 1) for smaller-is-better ones (Π−), where φ_i is the spec (Eq. 3, L158–184).

### [WS] 3 Overview (L186–215, p5–p6)
- Sizing runs after global routing and before detailed routing (L187–188).
- WAGN builds on PEA [20]. It adds global-routing features and multi-kernel attention (L189–201).
- Two optimizers: BO-WAGN, and TF (the widths become trainable parameters with multi-start) (L202–210).
- Every solution is discrete and realized in detailed routing. Analog circuits are rarely congested, so routability and area are rarely affected. **Two nets under a symmetry constraint always get the same width** (L212–215).

### [WS] 4.1 GNN background (L218–291, p6–p7)
- A graph is given by an adjacency A, node features X (n×d) and edge features E (n×n×p) (L219–226).
- Graph convolution is Z^(l) = σ(Φ_A X W) (Eq. 4). GAT attention is α_ij = softmax_row(τ_ij) with τ_ij = LeakyReLU(aᵀ[(W X_i)||(W X_j)]) (Eq. 5–7) (L228–266).
- DiffPool assignment is S^(l) = softmax_row[σ(D̃^−½ Ã D̃^−½ X W_pool)]; then X^(l+1) = SᵀZ and A^(l+1) = SᵀAS (Eq. 8–10) (L270–291).

### [WS] 4.2 Circuit graph and features (L294–329, p8)
- Nodes are device pins, IO pins and **Steiner nodes of the global route**. Edges are directed connections (L295–297).
- Node features: node type (PMOS, NMOS, cap, current source, GND, Steiner); **functional module** (bias current mirror, diff pair, active load); W, L and number of fins; pin location (L298–304).
- Edge features: horizontal and vertical distance, number of vias, wire-width options, pin shape length at both ends, and terminal type (S/D/G/Steiner) (L306–315).
- Edge direction encodes causality. For example, M3_S and M4_S control M2_D and not the reverse (Fig. 4) (L324–329).

### [WS] 4.3 WAGN (L340–433, p9–p10)
- The network is serial attention-pooling layers followed by an MLP. Each layer has 4 steps: attention construction and compression, graph convolution, node pooling, edge pooling (L341–344).
- Raw attention is α̂_ijk = τ_ij·E_ijk (Eq. 11). It is bidirectionally normalized (Eq. 12) and compressed from 3-D to 2-D with a trainable vector b (Eq. 13) (L345–384).
- New attention τ_ij = LeakyReLU(Σ_{r=1..3} B_r(W X_i, W X_j)) (Eq. 14) uses three kernels: Gaussian exp(−γ‖x−x′‖²), polynomial (xᵀx′+c)^d, and an MLP (Eq. 15). This gives +3.0% average accuracy over PEA (L387–413).
- The output is y = P(FOM < T), a soft classification of an unsatisfactory design (Eq. 17, L427–433).

### [WS] 4.4 Topologies and complexity (L436–454, p11)
- One model handles different topologies (cascode vs current-mirror OTA) and different Steiner-tree topologies, because the layer sizes {d_l, p_l, n_l} are independent of the input graph. By contrast, the GP model of [29] is limited to a single topology (L437–447).
- Per-layer cost is O(n_l²(n_l·p_l + d_l² + p_l²)). Inference is one forward pass (L448–454).

### [WS] 4.5 Training (L459–474, p11)
- There are N post-layout-simulated samples with label 1 (unsatisfactory) or 0. The loss is cross-entropy plus λ‖θ‖² (Eq. 18–19) (L460–474).

### [WS] 5.1 Optimization strategy (L478–498, p12)
- Solve min_s WAGN(s) under integer bounds (Eq. 20).
- Stage 1 relaxes to continuous values, then discretizes by **partial enumeration**: values near an integer are rounded, and the other nets try every up/down rounding combination, scored by WAGN.
- Stage 2 simulates the top 10 solutions and picks the best by Eq. 1 (L486–498).

### [WS] 5.2 TF optimization (L501–528, p12–p13)
- L_sizing = WAGN(s) + φ·Σ_j [ReLU(s_L − s_j) + ReLU(s_j − s_U)], with φ ≥ 0 (Eq. 21). The model weights θ* are frozen and the widths s are trained (L505–522).
- Multi-start gradient descent. The top solutions are simulated and the maximum-FOM one is kept (L523–528).

### [WS] 5.3 WAGN-guided BO (L530–566, p13–p14)
- Algorithm 1: sample N_s solutions with WAGN; fit a surrogate; iterate the acquisition optimum s* and evaluate it with WAGN; update; simulate the top set S_top; return the maximum simulated FOM (L544–554).
- The surrogate is a GP with an RBF kernel. Acquisition is Expected Improvement, optimized with bounded L-BFGS-B. Surrogate training costs O(N³) and inference O(N²), with N = N_s + N_iter (L557–566).

### [WS] 6 Experiments setup (L568–646, p14–p16)
- Hardware: a 64-core Xeon E5-2680 v2 with 256 GB. The C++ placer and router support symmetry, matching and common-centroid. Simulation uses HSPICE, and Spectre PSS/PAC for the SCF. PEX uses Calibre. Layout generation and simulation run in parallel (L579–591).
- Twelve testcases: 5 OTAs (5T, CC, CM, two two-stage), 3 comparators, 2 VCOs (4 and 6 repeated stages) and 2 SCFs, in ASAP7 and GF12 (L575–608).
- The specs are the schematic's own values (Table 1), so the goal is to approach schematic performance (L594–610).
- Each schematic gets 2000 layouts made by varying tool parameters and constraints, split 80/20 train/test. WAGN has 4 attention-pooling layers and 5 MLP layers (Table 2: nodes 12, 12, 6, 6; node features 11, 11, 5, 5; edge features 31, 31, 12, 12; MLP 32/16/8/4/1) (L610–625).
- Widths are 1×, 2× or 3× on a gridded router. 3× is the upper bound. 99% of solutions are routable, and an unroutable net is restored to its original width. 20 nets give over 3 billion combinations (L628–635).
- Width is realized as **parallel routes** (3× = three parallel tracks, Fig. 8). The parallel paths may differ slightly in length, with negligible effect (L636–645).

### [WS] 6.1 ML model results (L647–711, p16–p17)
- Metric definitions: TPR = TP/(TP+FN); FPR = FP/(FP+TN); accuracy; precision; AUROC (L648–666).
- Table 3, average of 12 testcases, as accuracy / precision / TPR / FPR / AUROC. WAGN 94.0 / 83.3 / 89.5 / 4.7 / 0.971; PEA 91.0 / 81.3 / 77.1 / 4.9 / 0.932; SVM 85.0 / 76.6 / 58.7 / 5.4 / 0.892; RF 83.9 / 76.6 / 47.4 / 4.5 / 0.881 (L672–676).
- Transfer means training on 80% of the source and fine-tuning on 10% of the target (12 pairs, Table 4). WAGN transfer: TPR 73.9% vs 61.0% fine-tune-only, a gain of 12.9%. PEA transfer gains only 4.6%. Transfer works only within one circuit type (L689–708).

### [WS] 6.2 Wire-sizing results (L713–858, p17–p20)
- Baselines: uniform 1×, 2× and 3×; BO1 with 14 simulations (runtime matched); BO2 with 110 simulations. Methods: BO-WAGN, BO-WAGN-transfer, TF, TF-transfer, each with 10 simulations (L714–730).
- BO-WAGN-transfer improves average FOM by 8–21% over uniform sizing. The other techniques are within 1%. BO1 at similar runtime is 11% worse. BO2 matches FOM but is 8.7× slower (Table 6 normalized: BO1 1.2×, BO2 8.7×, BO-WAGN WTC 2.6×, TF-transfer 1.3×) (L743–785).
- CM-OTA FOM: 1× 0.92, 2× 0.93, 3× 0.90; BO2 1.00; BO-WAGN-transfer 1.00. Wire area: 0.24, 0.63, 0.86, 0.57 and 0.51 µm² respectively (Table 7, L794–802).
- TS-OTA1 FOM: 1× 0.59, 3× 0.88, BO-WAGN 0.94 (gain goes from 18.25 dB at 1× to 36.23 dB). TS-OTA2: 1× 0.73, 3× 0.91, BO-WAGN 0.92, with wire area 0.59 vs 1.15 µm² at 3×. Comp1 offset: 1× 17.8 mV, 3× 13.4 mV, BO-WAGN-transfer 1.8 mV (Tables 8–10, L815–844).
- Optimized wire area is always below 3× and often near 2×. Extra routes fill white space without moving devices. **Optimized widths are spread about uniformly (≈1/3 of nets at each width)**, so different nets need different widths (L807–812).
- CC-OTA gain rises by about 3 dB and Comp2 offset falls by about 7 mV, with no degradation elsewhere (Fig. 10, L855–858).

### [WS] 6.3 Parameter analysis (L878–887, p21)
- The defaults are ξ = 100 runs (starts) and η = 10 simulated top solutions. TF's FOM rises with η and depends weakly on ξ. BO benefits more from ξ. Both parameters can be raised dynamically until FOM saturates (L879–887).

### [WS] 6.4 Manual-annotated comparison (L909–920; Table 11 L895–904, p22)
- Designers annotated each net as non-sensitive (1×), R-sensitive (3×, as wide as possible) or RC-sensitive (2×, neither too narrow nor too wide) (L910–916).
- BO-WAGN vs MA. TS-OTA1: FOM 0.94 vs 0.89, UGF 364.2 vs 329.4 MHz. TS-OTA2: FOM 0.92 vs 0.74, gain 46.79 vs 35.91 dB, wire area 0.59 vs 1.08 µm² (L897–904).
- Designers pick sensitive nets from topology, but the RC trade-off depends on the design (transistor and capacitor sizes) and is "hard for the designer to capture manually" (L917–920).

### [WS] 7 Conclusion (L924–942, p22–p23)
- Weaknesses: one model per circuit type; classification only, where regression would be more useful; poor scaling to ADC/DAC (L936–939).
- Future work: process variation, and joint or simultaneous transistor and wire sizing (L940–942).

### [WS] References and Appendix (L945–1140, p23–p27)
- References cover EM-aware routing [22], [23], [38]; IR-aware power routing [42]; parasitic-constraint generation [11] (Choudhury & Sangiovanni-Vincentelli); constraint generation [8] (Charbon); LEMAR length matching [47]; CC capacitor layout [24]; GCN-RL sizing [40] (L946–1067).
- Table 12 gives per-circuit WAGN vs PEA vs SVM vs RF. For example, CM-OTA WAGN TPR is 98.6% vs PEA 80.3%. Comparators are the hardest (WAGN TPR 82.0–83.8%) (L1073–1107).
- Table 13 gives per-pair transfer results. OTA pairs transfer well (TS-OTA2→TS-OTA1 TPR 97.9%). Comparator and VCO pairs transfer poorly (TPR 58–62%) (L1110–1140).

## 3. Actionable extraction

### SURV-01 Close the loop: post-layout simulation is the judge, and constraints are only proxies
- Kind: algorithm
- Statement: The flow is Netlist + Spec → Placement → Routing → PEX → Simulation → Performance ([PDS] Fig. 1). Proxy constraints "do not guarantee post-layout circuit performance". Simple geometric or parasitic constraints are "either inadequate or overly tight", and this is a main reason automatic layout trails manual layout ([WS] L30–34).
- Source: [PDS] Fig. 1, L15–37, L92–97 right col., p1–p2. [WS] §1, L26–34, p1.
- Philis stage: flow, verify.
- Automation recipe: every candidate layout (epoch) is extracted and simulated against the testbench specs. The spec miss ranks candidates above every proxy. The next step is to make the simulation drive the P&R knobs too (SURV-24, SURV-37), not only select among epochs.
- Beats hand layout because: a human simulates a few layouts. The tool can simulate every epoch and many variants, in parallel.
- Philis status: **implemented (selection only)**. `score_perf` runs ngspice on the extracted C plus per-terminal series R (frontend/library/src/lib.rs:878-894, :749-756; frontend/library/src/perf.rs:168). The spec miss is key position 2 of `LexKey` (frontend/library/src/lib.rs:916, :941-958). The simulation result does not feed back into the knobs of the next epoch.

### SURV-02 Generator taxonomy, and what "fully automated" must mean
- Kind: data-model
- Statement: There are three families. Template-based generators (BAG, LAYGO/LAYGO2) need user-defined patterns or floorplans. Digital-P&R-based flows limit the architecture or need manual effort. Analog P&R (MAGICAL, ALIGN) is fully automated and treats physical design as optimization. Template design "may require human efforts for designing and adjusting the layout templates" ([PDS] L47–58, L47–55 right col.).
- Source: [PDS] §I, L39–58, p1.
- Philis stage: flow.
- Automation recipe: stay in the P&R class. Hand-authored templates must not become required input. Generators in kernel/macroMaster are device generators, not floorplan templates.
- Beats hand layout because: no per-circuit human template work is needed.
- Philis status: **implemented**. It is P&R-class: annotator → gp → dp → gr → dr (frontend/library/src/lib.rs:336-339).

### SURV-03 Parity claims rest on post-layout metrics vs schematic, plus silicon
- Kind: metric
- Statement: Parity or superiority is claimed through (a) silicon tapeouts, such as MAGICAL 1.0 with a 40-nm 1 GS/s ΔΣ ADC [17], 1 and 80 MS/s SAR ADCs [18], and 64 SkyWater-130 temperature-sensor instances [16], and (b) post-layout simulation measured against the schematic. In [WS] the specs are the schematic's own values, and "the goal … is to approach the schematic design performance as much as possible" ([WS] L608–610). The metrics used are gain, UGF, BW, PM, CMRR, offset, noise, comparator evaluation and precharge delay, power, and VCO frequency range ([PDS] Table I L199–212; [WS] Table 1 L594–605). The vendored MAGICAL OTA benchmark records a manual-layout CMRR of 97 dB (benchmarks/competition/MAGICAL-CIRCUITS/benchmark_circuits/OTA/README.md:19).
- Source: [PDS] refs L353–367, p6; [WS] §6, Table 1, L594–610, p15.
- Philis stage: verify, flow (bench).
- Automation recipe: every bench fixture ships a testbench and specs, where min = schematic value × (1 − tol) and max accordingly. Report per metric the post-layout/schematic ratio and, where a reference exists, the manual-layout value (MAGICAL/ALIGN "Manual Layout" columns). A fixture "beats hand layout" when every metric's ratio ≥ the manual ratio and area ≤ manual area.
- Beats hand layout because: the claim becomes measured per metric, not asserted.
- Philis status: **missing in bench**. "no fixture ships a testbench" (benchmarks/src/bench.rs:140-141). ALIGN and MAGICAL circuits with manual-layout references are vendored under benchmarks/competition/ but not wired to perf scoring.

### SURV-04 Symmetry and symmetry islands
- Kind: rule
- Statement: Symmetric placement of matched cells [20]–[22]. A symmetry island [23] groups one symmetry group's devices into a connected placement region.
- Source: [PDS] §II-A, L71–75 right col., p2.
- Philis stage: annotator, gp, dp.
- Automation recipe: per stage, a shared axis. The island adds contiguity: the union of member bounding boxes must be connected, with no foreign cell between mirrored partners. As a cost, count foreign-cell area inside the group's hull.
- Beats hand layout because: exact integer mirror equality on every pair.
- Philis status: **partial**. `Symmetry` is exact (kernel/analog/src/placement/symmetry.rs:7-15) and `SymmetryGroup` shares one axis per stage (symmetry.rs:98-101). There is no island (contiguity) term; grep "island" finds only poly islands in cellgen.

### SURV-05 Common-centroid and array regularity
- Kind: rule
- Statement: Common-centroid layouts [24] and array-like module arrangements for regularity [25], [26].
- Source: [PDS] §II-A, L75–77 right col., p2.
- Philis stage: annotator, cells, dp.
- Automation recipe: centroid equality for matched groups, and row/column alignment for arrays (same pitch, same orientation).
- Beats hand layout because: exhaustive pattern enumeration is possible.
- Philis status: **implemented (CC)**, via `CentroidGroup` (kernel/analog/src/placement/cc.rs:28, "CommonCentroid" kind at :176). Regularity has no explicit rule (not audited).

### SURV-06 Proximity and boundary constraints
- Kind: rule
- Statement: Proximity [24], [27] and boundary constraints [28], [29] reduce wirelength and parasitics. Boundary means certain modules sit on the block boundary.
- Source: [PDS] §II-A, L77–79 right col., p2.
- Philis stage: annotator, gp, dp.
- Automation recipe: proximity as an edge-gap budget. Boundary: IO-connected cells (the pins of top-level ports) get cost = distance of the cell edge to the nearest die edge. As a hard constraint when the netlist declares a pin side.
- Beats hand layout because: short IO paths are guaranteed for every port, not only the ones a designer remembers.
- Philis status: **Proximity implemented** (kernel/analog/src/placement/proximity.rs:7-15). **Boundary missing**: no rule type (grep "Boundary" returns no .rs hits).

### SURV-07 Monotonic current-flow placement
- Kind: rule
- Statement: Enforce a monotonic direction of placement along current paths [51]–[54], VDD → load → signal devices → tail → GND. [53] adds pole awareness and wire-crossing minimization. The aim is fewer bends, crossings and vias on high-current paths.
- Source: [PDS] §II-B, L111–113 right col., p2; refs L520–534.
- Philis stage: annotator, gp, dp.
- Automation recipe: from the op-point, build per-supply DC current paths (the device chain through drain/source nets from supply to ground). For each path emit an ordering constraint y(dev_k) ≥ y(dev_{k+1}), or x for horizontal flow. Make it a cost term (a hinge on the order violation), weighted by path current. Detect paths with union-find over conducting D–S edges carrying op current above a threshold (not given in the source).
- Beats hand layout because: every path is ordered consistently, including in the tails and bias branches that designers skip.
- Philis status: **missing** (grep "current_flow|monoton" finds no placement rule).

### SURV-08 System signal-flow-aware placement
- Kind: heuristic
- Statement: Explicitly consider the critical signal flow in AMS placement [48], [49] (Zhu et al., ICCAD'20, TCAD'23). The survey says post-layout performance improves (no number given here).
- Source: [PDS] §II-B, L113–117 right col., p2.
- Philis stage: annotator, gp.
- Automation recipe: the annotator stages (one per top-level block, emit.rs header) get a topological order from input ports to output ports along gate←drain edges. gp adds a cost when stage centroid order is not monotone along the chosen axis.
- Beats hand layout because: the signal flow is consistent across all hierarchy levels automatically.
- Philis status: **missing**. Stages exist with axes (backend/annotator/src/emit.rs:1-16), but there is no inter-stage ordering.

### SURV-09 LDE-aware placement (WPE, LOD, OSE)
- Kind: rule
- Statement: Placement reduces layout-dependent effects: well proximity (WPE), length of diffusion (LOD), and oxide-to-oxide spacing (OSE) [55]–[57]. WPE ties into well-island generation [58]–[60].
- Source: [PDS] §II-B, L120–125 right col., L134–135, p2–p3.
- Philis stage: dp, cells, verify.
- Automation recipe: matched devices must see equal WPE and OSE distances (a budget), and LOD goes into the simulation deck.
- Beats hand layout because: the distances are measured on every matched pair, not eyeballed.
- Philis status: **partial**. `Surroundings`/`Environment` enforce equal WPE/OSE distances within `ENV_TOL` = 0.2 (kernel/analog/src/placement/environment.rs:5-30), built in frontend/library/src/lib.rs:806-875. The LOD stress term goes into post-layout simulation (frontend/library/src/perf.rs:25-27). There is no absolute-distance objective for unmatched sensitive devices.

### SURV-10 Well-island generation and well-tap insertion inside placement
- Kind: algorithm
- Statement: Well islands are generated during placement [58]–[60], and there is a generalized method for well-island generation plus well-tap insertion in AMS layouts [59] (TODAES'23). WellGAN [58] and GAN-guided well-aware placement [60] learn manual well designs.
- Source: [PDS] §II-B, L134–135, L159–162, p3; refs L475–482.
- Philis stage: dp, cells.
- Automation recipe: in dp, cost = Σ well perimeter near channels + number of well islands + tap-coverage violations, where each well point must be within the latch-up tap distance from the deck. Merge wells of same-bulk PMOS during annealing, not after.
- Beats hand layout because: the well count and tap coverage are checked for every configuration during search.
- Philis status: **partial**. `well_bridges` merges neighbouring same-bulk nwells after placement (kernel/cells/src/post_cell.rs:378-387). Guard rings merge within 2 µm (post_cell.rs:15-17, :26). Wells do not shape the placement cost.

### SURV-11 Thermal-aware placement with matching
- Kind: rule
- Statement: Thermal-driven symmetry and placement that keeps matching [61], [62].
- Source: [PDS] §II-B, L135–137, p3.
- Philis stage: dp.
- Automation recipe: matched pairs sit on one isotherm of the live power field.
- Beats hand layout because: the temperature field is computed, not guessed.
- Philis status: **implemented**. `ThermalGradient` uses the live field (kernel/analog/src/placement/thermal.rs:7-15).

### SURV-12 Analytical, SMT and RL engines used by leading tools
- Kind: algorithm
- Statement: The survey lists: analytical placement for analog [46] ("Are analytical techniques worthwhile?"); device-layer-aware analytical placement [47]; SMT routability-aware FinFET placement [26]; RL-guided detailed routing [39]; AutoCRAFT [40]; silicon-proven AMS detailed routing [38]. It gives no quantitative comparison.
- Source: [PDS] §II-B L101–103 right col.; refs L409–413, L479–486, L503–508.
- Philis stage: gp, dp, dr.
- Automation recipe: none new. These are the reference points for Philis's engines.
- Beats hand layout because: not given.
- Philis status: **implemented equivalent**. gp is momentum gradient descent on HPWL + density + analog cost (backend/gp/src/lib.rs:1-4), and dp is Metropolis annealing (backend/dp/src/lib.rs:84).

### SURV-13 Symmetric pair routing
- Kind: rule
- Statement: Symmetric routing of paired nets [21], [30]–[40] is the most widely adopted analog routing constraint.
- Source: [PDS] §II-A, L80–83 right col., p2.
- Philis stage: dr.
- Automation recipe: route one net and guide its partner onto the mirror image.
- Beats hand layout because: mirror-exact routing on every pair.
- Philis status: **implemented**. `mirror_guide` (backend/dr/src/lib.rs:1693-1709), per-node cost off the mirror (dr/src/lib.rs:1380), trials at :1452-1453.

### SURV-14 Exact route matching per layer; parasitic-matching detours
- Kind: check
- Statement: Exact routing [43], [44] requires "the pair of wire lengths be the same for every layer so that the wiring parasitics are precisely matched". LEMAR [47 in WS] uses wire detouring for parasitic mismatch.
- Source: [PDS] §II-A, L86–89 right col., p2; [WS] §1, L118–119, p4.
- Philis stage: dr, verify.
- Automation recipe: a per-layer route signature (length, area, via count). The mismatch is Σ_layer|a−b| over the mean. Repair by mirrored rip-up or by detouring the short side.
- Beats hand layout because: the check is exact on every layer.
- Philis status: **implemented (check), partial (repair)**. `Differential` compares per-layer length, area and cut signatures, or stack RC (kernel/analog/src/routing/differential.rs:8-27). The doc says dr repairs by mirrored rip-up. There is no length-equalizing detour pass.

### SURV-15 Maximize symmetry without an explicit constraint
- Kind: heuristic
- Statement: Chen et al. [38] (silicon-proven AMS detailed routing) maximize the degree of symmetry even when no symmetry constraint is declared.
- Source: [PDS] §II-A, L89–91 right col., p2.
- Philis stage: dr.
- Automation recipe: for every net pair whose terminal sets are mirror images about any placement axis (within one track; `mirror_guide` already tests this), apply the mirror guide as a soft cost. Weight it by min(sensitivity weights of the pair). Apply the same test for translation symmetry.
- Beats hand layout because: incidental symmetric pairs (bias lines, cascode gates) get symmetric routes for free.
- Philis status: **partial**. `mirror_guide` returns `None` unless terminals mirror (dr/src/lib.rs:1695-1709). It is invoked only for declared pairs (dr:1452 context). Not audited further.

### SURV-16 No routing over transistor active regions
- Kind: rule
- Statement: Forbid routing over active regions of transistors [31], [33].
- Source: [PDS] §II-A, L83–84 right col., p2.
- Philis stage: gr, dr.
- Automation recipe: every device's active and gate area becomes a keepout for foreign nets on metal layers ≤ M2, or on all layers when the victim is a matched or sensitive device. Hard for matched devices; cost for the others.
- Beats hand layout because: no stray crossing over any active area slips through.
- Philis status: **partial**. The gr keepout charges `KEEPOUT_COST` = 2.0 steps per node over a *matched* cell the net does not belong to (backend/gr/src/lib.rs:748-757). It is soft, applies to matched cells only, and does not reach dr (dr/src/lib.rs:473, :493).

### SURV-17 Reliability-driven power routing (EM and IR)
- Kind: rule
- Statement: Power routing optimization [41] (reliability-driven P/G routing), EM-aware and IR-drop-avoiding routing of multiport terminals [42]. Wire sizing for EM uses Black's equation ([WS] L68–71).
- Source: [PDS] §II-A, L84–85 right col.; [WS] §1, L68–72, p2.
- Philis stage: gr, dr, annotator.
- Automation recipe: per-segment current from the op-point sets the width through the deck EM limits. IR budgets apply per supply net.
- Beats hand layout because: the width follows current on every segment.
- Philis status: **implemented**. `Electromigration` (kernel/analog/src/routing/em.rs:92), `IrDrop` (kernel/analog/src/routing/ir.rs:19), IR budgets from op currents (frontend/library/src/lib.rs:282-292), dr EM width per segment (backend/dr/src/lib.rs:655-666).

### SURV-18 Shielding critical nets from net classification
- Kind: rule
- Statement: A shielding routing algorithm based on net classification [36].
- Source: [PDS] §II-A, L85–86 right col., p2.
- Philis stage: annotator, dr.
- Automation recipe: classify aggressors (clocks) and victims (sensitive nets), and shield victims with a reference net.
- Beats hand layout because: every victim–aggressor pair is found.
- Philis status: **implemented**. `Shield` (kernel/analog/src/routing/shield.rs:21) is emitted only when an aggressor exists (backend/annotator/src/extract.rs:87-95).

### SURV-19 Wire-load reduction: planar routing, via minimization, routing-aware incremental placement, R-aware layer balancing
- Kind: heuristic
- Statement: Planar routing [35], and routing-aware incremental placement refinement to reduce vias [50]. Routing wire R is minimized by balancing the number of layer changes against wirelength ([WS] ref [10]). R-sensitive nets are prioritized in routing ([WS] ref [35]).
- Source: [PDS] §II-B, L105–110 right col., p2; [WS] §1, L116–118, p4.
- Philis stage: gr, dr, dp.
- Automation recipe: a node cost per layer step and per via, scaled by the net's R-sensitivity S_R (SURV-37) and not only by its DC current. After routing, perform one incremental dp pass that moves cells to straighten nets with more than k vias (k not given in the source).
- Beats hand layout because: a quantitative via/R trade-off per net.
- Philis status: **partial**. gr prices `current·layer_r` per step and `current·via_r` per via (backend/gr/src/lib.rs:846-872), with `via_cost` in the graph (gr:514-531). R is priced only for current-carrying nets ("R is at the drawn wire width", gr:853-854), not for small-signal R-sensitive nets. There is no routing-driven incremental placement.

### SURV-20 Capacitance-sensitivity-weighted P&R
- Kind: formula
- Statement: Capacitance sensitivity is considered during placement ([WS] ref [48]) and coupling is reduced in routing ([WS] ref [16]). ([WS] notes that none of these addresses performance directly.)
- Source: [WS] §1, L115–117, p4.
- Philis stage: annotator, gp, gr.
- Automation recipe: per spec j, w_ij = ∓(∂f_j/∂C_i)/headroom_j, and Σ_i w_ij·C_i ≤ 1 as a shared budget. The weights also scale placement HPWL.
- Beats hand layout because: exact per-net shares of each spec's headroom.
- Philis status: **implemented**. Finite-difference ∂f/∂C in ngspice (frontend/library/src/perf.rs:194-228). `PerformanceBudget` rows (kernel/analog/src/routing/performance.rs:8-27). HPWL weights from them (backend/gp/src/lib.rs:137-153; frontend/library/src/lib.rs:294-296). Gap: computed only at the schematic point (`Parasitics::default()`, perf.rs:201).

### SURV-21 Learn-from-human: retargeting and generative guidance
- Kind: algorithm
- Statement: Retargeting pulls constraints from manual layouts into templates [63] and fails on unseen designs. GeniusRoute trains a VAE on manual layouts to predict designer routing regions, which become detailed-routing guidance [37]. A GAN learns manual well design [58], [60], and a VAE handles primitive-cell layout [64]. The limits are small datasets and no direct performance optimization.
- Source: [PDS] §II-B, L141–168, p3.
- Philis stage: gr, dp (guidance only).
- Automation recipe: skip until a corpus of manual layouts exists. The vendored ALIGN "Manual Layout" GDS could seed routing-region priors later.
- Beats hand layout because: it does not. It imitates hand layout, and the survey notes it lacks direct performance optimization.
- Philis status: **missing, by design** (no ML).

### SURV-22 Black-box BO over net weights to optimize post-layout performance
- Kind: algorithm
- Statement: [65] treats layout generation plus simulation as a black box. BO optimizes post-layout performance with **net weights as the input parameters**. It finds high-quality layouts but is slow, so it suits small circuits. Net weights "decide the priority of different nets and, therefore, impact the resulting layouts" ([PDS] L319–322 right col.).
- Source: [PDS] §II-B, L169–182, p3; §IV-B, L313–324 right col., p5.
- Philis stage: flow, gp, gr.
- Automation recipe: knobs k_n ∈ [0.25, 4] (a Philis choice, not from the source) multiply `net_weight[n]` for the M most sensitive nets (M ≤ 8). The outer loop runs epochs. The objective is the lex key with the spec miss first. Use coordinate or CMA search, or a GP-EI surrogate over k, for a budget of B epochs. Seed with k = 1, the current sensitivity weights.
- Beats hand layout because: it searches net priorities against simulated performance. A human cannot weigh 10+ nets jointly.
- Philis status: **partial**. The weights are derived once from sensitivities (backend/gp/src/lib.rs:137-153; frontend/library/src/lib.rs:296-316). Prices persist across epochs (frontend/library/src/lib.rs:336-339), but the weights are never tuned from simulated outcomes.

### SURV-23 ML placement-quality and performance predictors
- Kind: algorithm
- Statement: [19]: tens of thousands of MAGICAL placements are rasterized and a CNN predicts performance, with transfer across schematics. [66] adds AutoML/NAS. [67] uses star-model wirelength (inaccurate without layout data). The GNN [80] is used with SA to optimize predicted performance.
- Source: [PDS] §II-C, L187–190 left, L134–158 right, p3.
- Philis stage: flow (future).
- Automation recipe: out of scope now. The prerequisite is logging every epoch's (layout features, simulated metrics), which creates the dataset for free.
- Beats hand layout because: a cheap performance oracle inside the anneal.
- Philis status: **missing**. Epoch perf results are kept per epoch (frontend/library/src/lib.rs:892-893), but no dataset is exported.

### SURV-24 Simulation-budget economics of the performance loop
- Kind: metric
- Statement: Data collection takes 92.89% of the modeling lifecycle and training plus inference 7.11% (text). The Fig. 2 pie shows PEX+Post-Sim 75.42% and PNR 22.55% (figure image). One label (PEX + post-sim) costs about 3–4 P&R iterations. Post-layout evaluation is about 10× slower than schematic ([WS] Fig. 3: OTA 1 s → 71 s; comparator 20 s → 233 s, figure image). Parallelize PEX and post-sim, and select samples by active learning.
- Source: [PDS] §III-A, L177–213, p3–p4; Fig. 2, p5. [WS] §1, L91–96, Fig. 3, p3.
- Philis stage: flow, verify.
- Automation recipe: count simulation calls and PEX seconds per run in `RunStats`. Simulate only epochs whose (|V|, Θ) tie or beat the incumbent (a cheap pre-filter), and run candidate simulations concurrently (per-run deck files already exist).
- Beats hand layout because: a human cannot afford the simulation count the tool can run.
- Philis status: **partial**. The sensitivity runs are parallel, with one deck file per run (frontend/library/src/perf.rs:169-173, :201-210). Epoch scoring is one simulation per epoch (lib.rs:878-894). No simulation budget accounting was found.

### SURV-25 Weighted FOM vs Pareto: use multi-objective selection
- Kind: algorithm
- Statement: A weighted-sum FOM cannot express trade-offs between conflicting metrics. MOBO beats weighted-BO on 3–5 metrics per OTA. Top-1 counts, weighted vs MOBO: OTA1 1 vs 4, OTA2 2 vs 3, OTA3 0 vs 5, OTA4 1 vs 4. Pareto-optimal selection moves solutions toward the Pareto front. Gradient-based multi-objective methods (MGDA [77]) are future work.
- Source: [PDS] §III-C, L217–252 right col., p4; Fig. 3, p5; §IV-B, L325–343.
- Philis stage: flow.
- Automation recipe: keep an archive of non-dominated epochs over the vector (per-spec margin_j, Θ, extracted C, area). Dominance is only among epochs with |V| = 0. The final choice uses the declared policy: the current lex key, or the user's per-spec priority. Emit the front in the report so the designer picks.
- Beats hand layout because: the designer sees the whole trade-off surface, not one hand-iterated point.
- Philis status: **missing**. The key is a lexicographic scalarization with a summed spec miss (frontend/library/src/lib.rs:909-958; perf.rs:91-95). "Pareto" appears nowhere in the sources (grep).

### SURV-26 Transfer and OOD checks when reusing a model
- Kind: check
- Statement: Model accuracy drops 3–22% when transferred (Table I, e.g. OTA2 DC gain 81.35% → 59.50%). Transfer across sizings is easier than across topologies. Use OOD detection [73] to decide when transfer is valid. From scratch: binary classification with balanced sampling [71].
- Source: [PDS] §III-B, L219–258, Table I L199–212 right col., p4.
- Philis stage: flow (future ML).
- Automation recipe: applies only if a learned surrogate is added. A linear-sensitivity surrogate (SURV-20) has the same failure mode far from the schematic point. Re-derive sensitivities when the layout's simulated metrics differ from the linear prediction by more than the headroom.
- Beats hand layout because: not given.
- Philis status: **missing** (no learned model; sensitivities are not re-derived, perf.rs:201).

### SURV-27 Wire width is a first-order analog performance knob in modern nodes
- Kind: rule
- Statement: With minimum-width wires "the design is wire-resistance-constrained". In a 7-nm OTA, widening the diff-pair source nets L1/L2 from 1× to 3× gives UGF 688.5 → 972.0 MHz and gain 23.0 → 25.0 dB (schematic: 1223.0 MHz, 26.2 dB). Series R degenerates the effective Gm. The gain persists after transistor sizing: wire sizing alone +11.5% FOM, after transistor sizing +19% (text).
- Source: [WS] §1, L43–60, Figs. 1–2, p2–p3.
- Philis stage: annotator, dr.
- Automation recipe: tag as **R-critical** every net that carries a MOSFET source terminal of a gm device (diff pair, CS input, degenerated mirror): the source/tail nets, and nets in series with the signal current. They get the R-sensitivity measurement of SURV-28 and a candidate width > 1×.
- Beats hand layout because: every series-R net is identified and sized, not only those a designer suspects.
- Philis status: **missing**. dr keeps sensitive signal nets at the EM-need width, never wider: "a C-budgeted signal gains R it was not asked for at C it pays for (EM only)" (backend/dr/src/lib.rs:667-676).

### SURV-28 Per-net R and C sensitivity classification (what designers do by hand)
- Kind: algorithm
- Statement: Designers annotate each net as non-sensitive (1×), R-sensitive (3×, as wide as possible) or RC-sensitive (2×, neither too narrow for R nor too wide for C). The automatic method beats this annotation: TS-OTA1 FOM 0.94 vs 0.89; TS-OTA2 0.92 vs 0.74, with wire area 0.59 vs 1.08 µm². Reason: the "RC trade-offs and their impacts … are design-dependent … hard for the designer to capture manually".
- Source: [WS] §6.4, L909–920, Table 11 L895–904, p22.
- Philis stage: annotator (via perf), dr.
- Automation recipe: extend `perf::sensitivities` with a second finite difference per net. Add ΔR in series at the net's terminals through `Parasitics.series`, which already exists (perf.rs:22-24), giving ∂f_j/∂R_i. Then R-sensitive means |∂f/∂R|·R_1× ≥ τ·headroom, and C-sensitive likewise for ∂f/∂C·ΔC(1×→3×). τ is a Philis choice; the source gives none. Class → candidate widths: non-sensitive {1×}, R-only {2×, 3×}, RC {1×, 2×, 3×}, evaluated by the linear model w_R·R(w) + w_C·C(w) with R(w) ∝ 1/w and C(w) = area + fringe from the stack.
- Beats hand layout because: the per-net RC trade-off is computed from the actual device sizes, which the paper shows beats designer annotation.
- Philis status: **partial**. ∂f/∂C only (frontend/library/src/perf.rs:194-228). Series R reaches the simulation deck (frontend/library/src/lib.rs:749-756), but no ∂f/∂R is computed.

### SURV-29 Wire-sizing problem: discrete per-net widths, after gr and before dr
- Kind: data-model
- Statement: s ∈ Z^C, with s_L ≤ s_i ≤ s_U. Widths are discrete because of multiple patterning. Sizing runs after global routing and before detailed routing. **Symmetric nets always get equal width.** Analog circuits are rarely congested, so wider wires rarely hurt routability or area.
- Source: [WS] §2 Eq. 1, L147–157, p5; §3, L187–188, L212–215, p5–p6.
- Philis stage: gr, dr.
- Automation recipe: add `width_mult: Vec<u8>` per net (1..=3) to the dr config, set between gr and dr. Tie symmetric and differential partners to one value (`symmetric` set already exists in dr).
- Beats hand layout because: all nets are sized jointly.
- Philis status: **missing**. dr has only the global `fat_signal`/`fat_supply` targets (backend/dr/src/lib.rs:59-63, :126-129; frontend/library/src/elaborate.rs:434-435).

### SURV-30 Composite FOM with clipped relative performance
- Kind: formula
- Statement: FOM = Σ_{i=1..M} w_i·z̃_i, with Σw_i = 1. z̃_i = min(z_i/φ_i, 1) for i ∈ Π+ (gain, BW) and min(φ_i/z_i, 1) for i ∈ Π− (delay, offset). There is no reward beyond the spec.
- Source: [WS] §2 Eq. 2–3, L158–184, p5.
- Philis stage: flow.
- Automation recipe: Philis's normalized miss Σ max(0, (bound − v)/|bound|) is the complement form (0 when met). Keep it as the feasibility tier. Add the per-spec margin vector for SURV-25 so that over-achievement ranks too.
- Beats hand layout because: it gives an explicit, reproducible scoring policy.
- Philis status: **implemented (equivalent)**. `miss` (frontend/library/src/perf.rs:88-95) and the residual sum (perf.rs:185). The margin beyond the spec is discarded.

### SURV-31 Width realization by parallel tracks, with a routability fallback
- Kind: algorithm
- Statement: On gridded routing, 2× and 3× widths are realized as 2 or 3 **parallel routes** between the same pins, following the design rules. Path-length differences have negligible impact. The options are 1×, 2× and 3×, where 3× is "high enough for sufficient performance improvement and low enough for reducing routability problems". 99% of solutions are routable, and a net that is not is restored to its original width. The wire area of k× can exceed k× the 1× area.
- Source: [WS] §6, L628–645, Fig. 8, p15–p16; L733–735, p18.
- Philis stage: dr.
- Automation recipe: in dr, a net with width_mult k > 1 gets its tree routed k times on adjacent tracks, all pins shared, with the same-net spacing waived. Or fatten the trunk to k·w + (k−1)·s when width-dependent spacing allows (dr already has the deck's width-dependent spacing, dr/src/lib.rs:64-66). On failure, drop k by 1 and re-route. Record the fallback in the report.
- Beats hand layout because: the fallback is automatic and per net.
- Philis status: **partial**. Trunk fattening with width-dependent spacing exists (backend/dr/src/lib.rs:667-700), but for current (EM) or supply only, not by performance class.

### SURV-32 Different nets need different widths; uniform fattening is wrong
- Kind: heuristic
- Statement: After optimization the widths are spread about uniformly (≈1/3 of nets at each of 1×, 2× and 3×). Uniform 1×, 2× or 3× is 8–21% worse in FOM. Examples: CM-OTA 3× uniform scores FOM 0.90, below 1× uniform (0.92), and the optimized result is 1.00 at 0.51 µm² vs 0.86 µm²; for TS-OTA1, 1× gives gain 18.25 dB vs 36.23 dB optimized. Optimized wire area is always below 3× and often near 2×, and extra routes use white space without moving devices.
- Source: [WS] §6.2, L743–751, L794–812, Tables 7–8, p18–p20.
- Philis stage: dr.
- Automation recipe: never apply a global "fat signal" width to signal nets. Choose per net (SURV-28, SURV-33).
- Beats hand layout because: designers default to rules of thumb such as "make it wide".
- Philis status: **partial**. Philis already refuses blanket fattening of sensitive nets (dr/src/lib.rs:673-676). Unweighted signal nets still go toward `fat_signal` = 2× (elaborate.rs:434; backend/dr/src/lib.rs:678).

### SURV-33 Two-stage discrete optimization: relax, round by partial enumeration, simulate the top η
- Kind: algorithm
- Statement: Stage 1 solves the continuous relaxation with a surrogate. Fractional widths within a threshold of an integer are rounded directly. For the rest, enumerate all up/down roundings and score them with the surrogate. Stage 2 simulates the top η (default 10) and keeps the best simulated FOM. Multi-start with ξ = 100. Both ξ and η "can be dynamically increased during runtime till the saturation of FOM improvement".
- Source: [WS] §5.1, L486–498, p12; §6.3, L879–887, p21.
- Philis stage: flow, dr.
- Automation recipe: the surrogate is the linear R/C model of SURV-28, not a GNN: Δf_j ≈ Σ_i (∂f_j/∂R_i·ΔR_i(w_i) + ∂f_j/∂C_i·ΔC_i(w_i)). Minimize the summed spec miss over w ∈ {1, 2, 3}^C. With ≤ 20 R/RC-sensitive nets, run coordinate descent from ξ starts. Simulate the η best (in parallel threads, as sensitivities already do). Take the width vector with the best lex key. Stop when the best key does not improve over one η batch.
- Beats hand layout because: 3^20 > 3·10⁹ combinations ([WS] L633–635) are searched systematically.
- Philis status: **missing**.

### SURV-34 Gradient relaxation with a penalty for box bounds
- Kind: formula
- Statement: L_sizing = M(s) + φ·Σ_j [ReLU(s_L − s_j) + ReLU(s_j − s_U)], with φ ≥ 0 large (Eq. 21). s is the trainable variable and the model is frozen. Multi-start avoids local minima.
- Source: [WS] §5.2, L505–528, Fig. 6, p12–p13.
- Philis stage: flow.
- Automation recipe: with a linear surrogate, the relaxation is a box-constrained problem that projection solves exactly (clamp), so the penalty is unnecessary. Keep this as the reference form if a nonlinear surrogate arrives.
- Beats hand layout because: not given.
- Philis status: **missing** (not needed yet).

### SURV-35 Surrogate-guided Bayesian optimization, with simulation only for finalists
- Kind: algorithm
- Statement: Algorithm 1: N_s initial samples scored by the surrogate; a GP with RBF kernel; Expected Improvement acquisition optimized with L-BFGS-B (bounded); N_iter iterations scored by the surrogate; simulate the top set and return the maximum simulated FOM. Cost: O(N³) to train and O(N²) to infer, with N = N_s + N_iter. Results: BO1 (14 simulations, runtime-matched) is 11% worse in FOM. BO2 (110 simulations) matches FOM at 8.7× runtime. Hybrid surrogate plus 10 simulations wins.
- Source: [WS] §5.3, L530–566, p13–p14; §6.2, L714–730, L761–763, Table 6 L766–785, p17–p19.
- Philis stage: flow.
- Automation recipe: in the epoch outer loop (SURV-22), knob vectors are proposed by EI on a GP fitted to past (knobs → lex-key scalar), and only proposals are laid out and simulated. With ≤ 50 epochs, O(N³) is negligible.
- Beats hand layout because: sample efficiency. The same quality comes at 1/8.7 of the simulation budget.
- Philis status: **missing**.

### SURV-36 Circuit graph for performance reasoning: pins and Steiner nodes, directed causal edges
- Kind: data-model
- Statement: Nodes are device pins, IO pins and Steiner nodes of the global route. Node features: type, **functional module** (bias mirror, diff pair, active load), W, L, fins, pin location. Edge features: dx, dy, via count, width options, pin shape length, terminal types (S/D/G/Steiner). Edges are directed by causality: gate → drain, and source nets control drain nets, not the reverse.
- Source: [WS] §4.2, L294–329, Fig. 4, p8.
- Philis stage: annotator, gr.
- Automation recipe: build a directed "control graph" from the netlist: G → D and S → D edges per MOSFET, and net → terminal. Use it to (a) order stages for signal flow (SURV-08), (b) seed R-critical detection (SURV-27), (c) export features per epoch (SURV-23). Philis's annotator block kinds map directly onto the "functional module" feature.
- Beats hand layout because: the causal structure is captured for every device.
- Philis status: **partial**. There are functional modules (`BlockKind` DiffPair/CurrentMirror/Load/Stack, backend/annotator/src/block.rs:23-36) and an undirected hypergraph (pnr_core BipartiteHypergraph, used in kernel/analog/src/routing/differential.rs:5). There is no directed causal graph.

### SURV-37 WAGN performance classifier (architecture and accuracy)
- Kind: algorithm
- Statement: 4 attention-pooling layers plus 5 MLP layers. Multi-kernel attention τ_ij = LeakyReLU(Σ B_r) over Gaussian, polynomial and MLP kernels (Eq. 14–15). Raw attention α̂_ijk = τ_ij·E_ijk (Eq. 11), with normalization and compression (Eq. 12–13). DiffPool node and edge pooling. Output P(FOM < T) (Eq. 17). Loss: cross-entropy + λ‖θ‖² (Eq. 18). Averages over 12 circuits, as accuracy / TPR / FPR / AUROC: WAGN 94.0% / 89.5% / 4.7% / 0.971; PEA 91.0 / 77.1 / 4.9 / 0.932; SVM 85.0 / 58.7 / 5.4 / 0.892; RF 83.9 / 47.4 / 4.5 / 0.881. Per-layer cost O(n_l²(n_l·p_l + d_l² + p_l²)).
- Source: [WS] §4.3–4.5, L340–474, Tables 2–3, L617–676, p9–p16.
- Philis stage: flow (future).
- Automation recipe: reference design only if Philis ever adds a learned surrogate. It needs about 2000 layouts per schematic (L610–612), which is too costly per run.
- Beats hand layout because: not given.
- Philis status: **missing** (deliberately; the dataset cost is prohibitive per circuit).

### SURV-38 Knowledge transfer limits
- Kind: check
- Statement: Fine-tuning on 10% of target data: WAGN transfer TPR 73.9% vs 61.0% fine-tune-only (+12.9%), PEA +4.6%. Transfer works only within one circuit type (OTA→OTA). Comparators and VCOs transfer poorly (TPR 58–62%, Table 13). Cross-type transfer is "much more difficult" because the metrics differ.
- Source: [WS] §6.1, L689–708, Table 5, p16–p17; Table 13, L1110–1140, p27.
- Philis stage: flow.
- Automation recipe: this argues for per-circuit physics-based sensitivities (SURV-28) over pretrained models in a general tool.
- Beats hand layout because: not given.
- Philis status: **n/a** (Philis uses per-circuit simulation sensitivities, perf.rs:194-228).

### SURV-39 Wire sizing fixes offset and gain, not only speed
- Kind: metric
- Statement: Comp1 offset is 17.8 mV at 1× uniform, 13.4 mV at 3×, and 1.8 mV with BO-WAGN-transfer or BO2 (Table 10). CC-OTA gain rises about 3 dB. Comp2 offset falls about 7 mV without degrading other metrics (Fig. 10). CM-OTA BW goes from 10.2 MHz at 1× to 13.2 MHz optimized (Table 7).
- Source: [WS] §6.2, L837–858, Tables 7 and 10, p19–p20.
- Philis stage: dr, verify.
- Automation recipe: include offset (a DC testbench with symmetric inputs) in every comparator or amplifier spec set. Width asymmetry and series-R asymmetry show up there, and Philis's `Differential` RC check (SURV-14) plus equal widths on symmetric pairs (SURV-29) guard it.
- Beats hand layout because: offset caused by routing R is measured and optimized, not assumed zero.
- Philis status: **partial**. Offset is possible through a user testbench `.measure` (frontend/library/src/perf.rs:50-66). The bench has no testbench (SURV-03).

### SURV-40 Benchmark set and data protocol used to claim results
- Kind: metric
- Statement: 12 circuits. Five OTAs (5T, cascode, current-mirror, two two-stage), three StrongARM-type comparators (delay, precharge delay, power, offset), two VCOs (4 and 6 stages; power, f_max, f_min) and two switched-cap filters (PSS/PAC in Spectre), in ASAP7 and GF12. Specs are taken from the schematic (Table 1). 2000 layouts per schematic by varying tool parameters and constraints. Calibre PEX, HSPICE. 64-core parallel generation.
- Source: [WS] §6, L575–615, Table 1 L594–605, p14–p15.
- Philis stage: verify (bench).
- Automation recipe: mirror this family mix in Philis fixtures: OTA, comparator, VCO/ring, SC filter. Report per family the post-layout/schematic metric ratios (SURV-03). The comparator category is not given in the source beyond its metrics.
- Beats hand layout because: not given.
- Philis status: **partial**. OTA fixtures exist (frontend/library/tests/ota_cross_pdk.rs), and vendored MAGICAL has an ADC, a comparator, OTAs and buffers (benchmarks/competition/MAGICAL-CIRCUITS/benchmark_circuits/). They have no testbench-based scoring (benchmarks/src/bench.rs:140).

### SURV-41 Stated open problems: variation, joint sizing, scale
- Kind: heuristic
- Statement: The ML models are per circuit type, classification-only, and do not scale to ADC/DAC. Future work covers process variation and simultaneous transistor and wire sizing. From [PDS]: multimodal models, pretrain-then-finetune [81], active learning, faster SPICE [78], [79], and multi-objective gradient methods [75]–[77].
- Source: [WS] §7, L936–942, p23; [PDS] §IV, L306–343, p5–p6.
- Philis stage: flow.
- Automation recipe: variation belongs in Philis's matching budgets (Pelgrom) and the corner/Monte-Carlo testbench. Joint transistor-wire sizing is out of scope (Philis takes a sized netlist).
- Beats hand layout because: not given.
- Philis status: **n/a** (scope note).

## 4. Top-15 priorities for Philis

1. **Per-net R sensitivity** (SURV-27, SURV-28): add ∂f/∂R_i to `perf::sensitivities` through the existing `Parasitics.series`. It is the missing half of the RC trade-off, and [WS] shows it is worth 8–21% FOM.
2. **Per-net discrete width choice between gr and dr** (SURV-29, SURV-31, SURV-32): `width_mult ∈ {1, 2, 3}`, symmetric partners tied, fallback on routing failure. This replaces the fixed rule that sensitive signals stay narrow (dr/src/lib.rs:667-676).
3. **Two-stage width optimization** (SURV-33, SURV-35): linear R/C surrogate → coordinate search with rounding → simulate the top η = 10 in parallel. This gives BO2 quality at about 1/8.7 of the simulation budget.
4. **Bench with testbenches and a manual-layout reference** (SURV-03, SURV-40): ship specs derived from schematic values for OTA, comparator, VCO and SC-filter fixtures, and report post-layout/schematic and manual ratios. Without this, "beats hand layout" is unmeasured.
5. **Pareto archive of feasible epochs** (SURV-25, SURV-30): keep the non-dominated set over per-spec margins, Θ, C and area, and report the front. MOBO beat weighted scoring on 3–5 metrics per OTA.
6. **Close the loop on net weights** (SURV-22, SURV-35): an outer search over sensitivity-weight multipliers scored by post-layout simulation. Philis already has every piece except the feedback.
7. **Re-derive sensitivities at the layout point** (SURV-20, SURV-26): after the first feasible epoch, recompute ∂f/∂C and ∂f/∂R with the extracted parasitics. The linear model is only valid near the schematic point.
8. **Monotonic current-flow placement** (SURV-07): current paths from the op-point become an ordering cost in gp/dp, which cuts bends, vias and crossings on high-current branches.
9. **Signal-flow stage ordering** (SURV-08, SURV-36): a directed control graph orders the annotator stages, with a monotone stage order as a gp cost.
10. **Keep routing off active regions** (SURV-16): extend the gr keepout to every device's active/gate area for sensitive and matched victims, and carry it into dr.
11. **Opportunistic symmetry maximization** (SURV-15): apply `mirror_guide` as a soft cost to any net pair whose terminals mirror, not only to declared pairs.
12. **R-aware routing for small-signal nets** (SURV-19): price layer_r and via_r by S_R (from item 1), not only by DC current, so R-critical nets avoid via stacks and resistive layers.
13. **Well-aware placement** (SURV-10, SURV-09): put well-island count, well-edge proximity and tap coverage into the dp cost instead of only merging wells after placement.
14. **Boundary constraints for IO cells** (SURV-06): a new placement rule pulling port-connected cells to the die edge.
15. **Simulation budget accounting and parallel epoch scoring** (SURV-24): count simulation and PEX time, pre-filter epochs by (|V|, Θ) before simulating, and simulate candidates concurrently, since labels cost about 3–4 P&R iterations.
