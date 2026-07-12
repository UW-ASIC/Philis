# Literature Survey and Interdisciplinary Cross-Pollination Map

---

## 1. Core Papers by Pipeline Stage

### S0 -- Cell Generation

| Paper | Venue | Key contribution |
|-------|-------|------------------|
| Pelgrom, Duinmaijer, Welbers, "Matching Properties of MOS Transistors" | JSSC 1989 | Foundational area law: sigma^2(dVth) = A^2_VT/(W*L) |
| Scott, Lutze, Rubin, Nouri, Manley, "NMOS Drive Current Reduction Caused by Transistor Layout and Trench Isolation Induced Stress" | IEDM 1999 | First report of 13% STI drive-current reduction |
| Han, Bae, Chang, Wang, Nikolic, Alon, "LAYGO" | TCAS-I 2021 | Template-and-grid layout engine for FinFET; silicon-proven 7 GS/s SAR ADC in 16nm |
| Chang, Han, Bae, Wang, Nikolic, Alon, "BAG2" | CICC 2018 | Process-portable generator framework for analog layout |
| Karmokar, Sapatnekar et al., "CC Placement+Routing for Binary-Weighted and Split DACs" | TCAD 2023 | Co-optimized CC capacitor arrays trading f_3dB vs INL/DNL |
| Sapatnekar et al., "Common-Centroid Layouts: Advantages and Limitations" | DATE 2021 | Critical analysis: CC helps gradient but not random mismatch; routing overhead can hurt |

**Textbook foundations for S0:**
- [Hastings, Ch. 8] provides the 25 resistor matching rules, 13 capacitor matching rules, and the three matching tiers (minimal, moderate, exceptional) with quantitative targets for area, width, and accuracy. These rules are the primary source for parameterizing the S0 cell generator's matching constraints.
- [Hastings, Ch. 13] provides the 24 MOS transistor matching rules including the five rules of common-centroid layout (coincidence, symmetry, dispersion, compactness, orientation), dummy device requirements per tier, and the orientation metric chi = (1/N) * sum(chi_i).
- [Hastings, Ch. 14] provides guard ring design rules: guard the injector not the victim, HCGR + HBGR combined achieves >99% efficiency, ECGR width >= P-epi thickness. Also ESD protection device layout rules (GGNMOS ballasting, silicide blocking, CDM clamp proximity).
- [Hastings, Ch. 4] provides BiCMOS device structures (CDI NPN, lateral PNP, LDMOS, poly resistor types) essential for process-specific cell generators.
- [Hastings, Ch. 15, Section 15.1] provides die area estimation formulas: cell area `A_c = P_c * sum(A_i)` with packing factors per process type, and component area formulas for resistors, capacitors, MOS transistors.
- [Lienig, Section 6.6.6] provides the three-tier matching concept classification (normal, higher, highest) with specific layout requirements per tier.

### S1 -- Constraint Extraction

| Paper | Venue | Key contribution |
|-------|-------|------------------|
| Chen, Zhu, Liu, Tang, Sun, Pan, "Universal Symmetry Constraint Extraction with GNNs" | DAC 2021 | Unsupervised GNN on heterogeneous multigraph; F1>0.95 system-level |
| Kunal et al., "A General Approach for Identifying Hierarchical Symmetry Constraints" | ICCAD 2020 | Multi-axis, nested symmetry; approximate matching via neural tensor network |
| Kunal, Madhusudan et al., "GANA: GCN-based Automated Netlist Annotation" | DATE 2020 | GCN sub-circuit recognition (100% accuracy on 6 types) |
| Liu et al., "S3DET: System Symmetry Constraints with Graph Similarity" | ASP-DAC 2020 | Spectral method (Laplacian eigenvalues + K-S test) for system-level |
| Eick et al., "Comprehensive Generation of Hierarchical Placement Rules" | TCAD 2011 | Pattern library + signal flow traversal (pre-ML era) |
| Chen et al., "LLM-Enhanced Bayesian Optimization for Analog Layout Constraint Generation" (LLANA) | arXiv 2024 | LLM few-shot constraint generation with BO |
| Liu et al., "LayoutCopilot" | TCAD 2025 | LLM multi-agent: natural language -> layout tool commands |

**Textbook foundations for S1:**
- [Hastings, Ch. 13, Table 13.2] ranks systematic mismatch sources by impact magnitude (orientation > WPE > LOD > thermal > mechanical stress > hydrogenation > process gradients > etch-rate > Seebeck), providing the priority order for constraint sensitivity analysis.
- [Lienig, Section 5.2] describes netlist generation from schematic entry, emphasizing that the topological arrangement in a schematic bears no relation to the geometrical properties of the final circuit layout -- the constraint extractor must infer geometric intent from electrical topology.
- [Lienig, Section 4.3.2] describes the meet-in-the-middle design style for analog: bottom-up device generation + top-down system composition. The constraint extractor must support hierarchical extraction consistent with this methodology.

### S2 -- Placement

| Paper | Venue | Key contribution |
|-------|-------|------------------|
| Lin, Li, Fang et al., "Are Analytical Techniques Worthwhile for Analog IC Placement?" | DATE 2022 | ePlace-A/AP: 55x speedup, 12% WL reduction vs SA; first performance-driven analytical |
| Lu et al., "ePlace-MS" | TCAD 2015 | Electrostatics-based placement with FFT gradient; Nesterov solver |
| Xu et al., "Device Layer-Aware Analytical Placement" | ISPD 2019 | NTUplace3-based analog placement with device-layer overlap |
| Ou et al., "Layout-Dependent-Effects-Aware Analytical Analog Placement" | TCAD 2016 | First to model WPE+LOD+OSE analytically in placement |
| Li et al., "A Customized GNN Model for Guiding Analog IC Placement" | ICCAD 2020 | PEA network for placement quality prediction |
| Basso et al., "Effective Analog ICs Floorplanning with R-GCN and RL" | DATE 2025 | RL + R-GCN with knowledge transfer across topologies |
| Lin, Chang et al., "Analog Placement Based on Symmetry-Island Formulation" | TCAD 2009 | ASF-B*-tree; foundational symmetry-island concept |
| Balasa, Lampaert, "Symmetry within the Sequence-Pair" | TCAD 2000 | Sequence-pair extended for analog symmetry |

**Textbook foundations for S2:**
- [Hastings, Ch. 15, Section 15.2] describes the complete floorplanning methodology for analog ICs: mirror-image placements for identical blocks, pin proximity, routing channel reservation (~20% of core area), choke point avoidance, and power lead planning. The floorplan is the most consequential single drawing in the entire design process.
- [Hastings, Ch. 8, Rules 7, 12-15] provide quantitative placement rules: matched devices within "a few hundred um" for minimal, "immediately adjacent" for moderate, "always common-centroid" for exceptional. Low stress-gradient region means interior of die, >=200 um from edges, >=500 um from corners for moderate.
- [Hastings, Ch. 13, Rules 8-10, 13-15] provide additional MOS placement rules: compact layout aspect ratio <=3:1 for voltage matching, ~1:1 for exceptional current matching. Die center for low stress-gradient locations. Opposite end of die from power devices for exceptional.
- [Lienig, Section 6.6.4] describes thermal placement: power transistors create temperature differences of several tens of Kelvin across a chip; matched devices must be placed along isotherms.

### S3 -- Routing

| Paper | Venue | Key contribution |
|-------|-------|------------------|
| Zhu, Liu, Lin et al., "GeniusRoute" | ICCAD 2019 | VAE-guided routing; competitive with manual layouts |
| Chen, Zhu, Liu et al., "Toward Silicon-Proven Detailed Routing for AMS Circuits" | ICCAD 2020 | Four symmetry variants; pin clustering; silicon-proven (TSMC 40nm ADC tape-out) |
| Xu et al., "PARoute2: Enhanced Analog Routing via Performance-Drive Guidance" | TCAD 2025 | 3D GNN with cost-aware distance; 75% DC gain improvement vs GeniusRoute |
| Chen et al., "RL-Guided Detailed Routing for Custom Circuits" | ISPD 2023 | Sign-off quality RL routing across AMS+digital |
| Ou et al., "Nonuniform Multilevel Analog Routing with Matching Constraints" | TCAD 2014 | ILP for symmetry+CC+topology+length matching |
| Ou, Chang, "Electromigration and Parasitic-Aware ILP Router" | DAC 2018 | Joint EM+parasitic ILP routing |
| Martins et al., "EM-Aware and IR-Drop Avoidance Routing in Analog Multiport Terminal Structures" | DATE 2014 | EM-aware analog routing with Calibre validation (UMC 130nm) |

**Textbook foundations for S3:**
- [Hastings, Ch. 15, Section 15.4] provides the complete manual routing methodology: via technologies (aluminum, tungsten plug, single/dual damascene), routing pitches (`P_LTL = W_m + S_m`, `P_VTV = W_v + 2*M_ov + S_m`), channel routing rules, noise coupling equation (`dV = C * R * dV/dt`), star nodes, Kelvin connections, electrostatic shielding, and high-current lead sizing.
- [Hastings, Ch. 15, Section 15.4.4] describes noisy/sensitive signal separation: never route noisy signals adjacent to sensitive ones; cross at right angles; insert electrostatic shields with 2-5 um overhang; insert series resistors in slow digital lines to reduce slew rate.
- [Hastings, Ch. 8, Rules 10, 23] describe matched routing requirements: match lead capacitance using jogs/stubs; metal and via resistance contributions should scale proportionally to desired resistance ratios; use Kelvin connections for exceptional matching.
- [Lienig, Section 5.4.5] describes antenna rules: allowable ratio of poly/metal area to gate area, one ratio per interconnect layer. Sometimes a ratio of poly/metal perimeter to gate area is also required since charges are preferentially collected at conductor edges.

### S4 -- Sign-off

| Paper | Venue | Key contribution |
|-------|-------|------------------|
| Chen, Liu, Tang et al., "MAGICAL 1.0: Fully-Automated AMS Layout Verified with 40nm Delta-Sigma ADC" | CICC 2021 | Silicon-proven full flow; SNDR 67.4 dB (MAGICAL) vs 65.6 dB (manual) |
| Zhu, Chen, Liu, Pan, "Tutorial and Perspectives on MAGICAL" | TCAS-II 2022 | Comprehensive flow description; post-layout simulation results |

**Textbook foundations for S4:**
- [Hastings, Ch. 15, Section 15.5] provides the 17-point final layout checklist: DRC, LVS (zero topological errors non-negotiable), EM compliance, antenna, ESD, latchup, dummy metal block zones, noisy/sensitive separation, bondpads, Kelvin connections, corner exclusion, scribe seals, substrate contacts, die symbolization, archiving.
- [Hastings, Ch. 3, Section 3.2] describes design rule construction from physics: minimum feature size (Abbe criterion `CD_min ~ lambda/(2*NA)`), process bias, linewidth control (~10% of min feature), mask alignment (~20% of min feature combining as ~160% for two alignment errors), outdiffusion (~80% of junction depth lateral), depletion region width (voltage-dependent, add 50% safety margin).
- [Lienig, Section 5.4] describes the two-stage constraint mapping (physics -> formal rules -> tool formats) and the concept of dummy/false DRC errors when safety margins cause valid layouts to fail.
- [Lienig, Section 5.4.3] describes functional verification via simulation: simulator types (analog/SPICE, digital/Verilog, mixed-mode), the fundamental limitation that simulation cannot guarantee correctness, and the three levels of mixed-mode simulation.
- [Lienig, Section 5.5] describes layout post processing: chip finishing (seal ring, filler structures), reticle layout (test patterns, alignment marks), layout-to-mask preparation (RET/OPC, fracturing). OPC is less aggressive for analog than digital, but submicron analog processes still need at least mild OPC on gate and moat masks [Hastings, Ch. 3, Section 3.3.4].

### Full-Flow Systems

| System | Lead Group | Key paper(s) |
|--------|-----------|--------------|
| MAGICAL | UT Austin (David Z. Pan) | ICCAD 2019, CICC 2021, TCAS-II 2022 |
| ALIGN | UMN/TAMU/Intel (Sapatnekar, Harjani, Hu) | DAC 2019, ISPD 2023 |
| BAG/BAG2 | UC Berkeley (Alon, Nikolic) | ICCAD 2013, CICC 2018 |
| LAYGO/LAYGO2 | UC Berkeley | TCAS-I 2021 |
| AutoCRAFT | UT Austin | ISPD 2022 |
| FALCON | USC/Infineon (Avestimehr) | NeurIPS 2025 (arXiv 2505.21923) |

---

## 2. Textbook References

| Book | Author(s) | Key chapters for this engine |
|------|-----------|------------------------------|
| Design of Analog CMOS Integrated Circuits | Razavi | Ch.19: Layout and Packaging |
| The Art of Analog Layout | Hastings | Ch.3: Design rules/PG; Ch.4: Processes (std bipolar, BiCMOS); Ch.7: Matching R/C; Ch.8: Matching rules (25 resistor, 13 capacitor); Ch.12: MOS matching/CC; Ch.13: Guard rings, 24 MOS matching rules; Ch.14: ESD protection; Ch.15: Die assembly (area estimation, floorplanning, routing, final checklist) |
| CMOS: Circuit Design, Layout, and Simulation | Baker | Ch.2-5: Wells, metals, active, passive; Ch.28.6: Mixed-signal layout |
| Analog Design Essentials | Sansen | Matching/mismatch/offset sections |
| CMOS IC Layout: Concepts, Methodologies, and Tools | Clein | Methodology and tool perspective |
| Fundamentals of Layout Design for Electronic Circuits | Lienig | Ch.4: Design flow, models, styles (full-custom for analog, meet-in-the-middle); Ch.5: Physical design steps (netlist generation, verification, post-processing); Ch.6: Matching (three-tier classification, five CC rules); Ch.7: Substrate noise, latchup |

---

## 3. Interdisciplinary Cross-Pollination Map

### 3.1 Operations Research -> Analog P&R

| OR technique | Applied to | How | Key reference |
|-------------|-----------|-----|---------------|
| Integer Linear Programming | Detailed placement | Single-stage area+HPWL minimization with hard constraints | ePlace-A (Lin DATE 2022) |
| Mixed-Integer LP | Hierarchical placement | Symmetry, matching, proximity constraints | Xu et al. ISPD 2017 |
| SMT (Satisfiability Modulo Theories) | Constraint placement | Z3 solver for symmetry-breaking, grouping, double-patterning | Multiple (de Moura & Bjorner TACAS 2008) |
| Combinatorial optimization | Net ordering | NP-hard net-order selection in routing | RL-based (ScienceDirect 2025) |
| Edmonds' blossom algorithm | Symmetry allocation | Maximum-weight matching for net-pair symmetry assignment | Chen ICCAD 2020 |

### 3.2 Computational Geometry -> Analog P&R

| Technique | Applied to | How |
|-----------|-----------|-----|
| Sequence-pair | Placement | Topological encoding for SA-based placement (Balasa TCAD 2000) |
| B*-tree / ASF-B*-tree | Placement | Linear-time packing with symmetry islands (Lin TCAD 2009) |
| Corner stitching | Placement | Near-constant-time spacing/legality checks (Ousterhout TCAD 1984) |
| Voronoi diagrams | Routing | Obstacle-aware routing skeletons |
| Hanan grid | Routing | Rectilinear Steiner tree construction (ALIGN bulk router) |

### 3.3 Machine Learning -> Analog P&R

| ML technique | Applied to | How |
|-------------|-----------|-----|
| Graph Neural Networks (GCN, GGRU, PEA) | Constraint extraction, placement quality prediction | Chen DAC 2021; Li ICCAD 2020 |
| Variational Autoencoders | Routing guidance | GeniusRoute (Zhu ICCAD 2019) |
| Reinforcement Learning | Placement, routing, net ordering | Basso DATE 2025; Chen ISPD 2023; Liao 2020 |
| Diffusion models | Placement | Simultaneous parallel placement (Lee et al. arXiv 2024) |
| Large Language Models | Constraint generation, interactive layout | LLANA 2024; LayoutCopilot 2025 |
| Transfer learning | Cross-topology generalization | Placement quality prediction; Liu DATE 2020 |

### 3.4 Robotics Path Planning -> Analog Routing

| Technique | Applied to | How |
|-----------|-----------|-----|
| A* search | Detailed routing | Core pathfinding algorithm in all analog routers |
| Maze routing | Detailed routing | Breadth-first variant of A* for analog (CMU ANAGRAM II, 1991) |
| Negotiation-based routing | Congestion management | PathFinder (McMurchie & Ebeling 1995) adapted for analog |
| RRT/RRT* | Multi-net routing (potential) | Sampling-based planning for high-dimensional obstacle-dense spaces |

### 3.5 Photonic IC Layout -> Analog P&R

| Technique | Applied to | Key insight for analog |
|-----------|-----------|----------------------|
| Bending-aware wirelength | PIC placement (Apollo) | Analog: parasitic-aware wirelength models |
| Curvy waveguide routing | PIC routing (APR) | Analog: variable-width/non-Manhattan routing |
| Routing-coupled-to-placement | PIC co-optimization | Analog: routing parasitics dominate -> must co-optimize |

### 3.6 Formal Verification -> Analog P&R

| Technique | Applied to | How |
|-----------|-----------|-----|
| Graph isomorphism | LVS | Extracted netlist === schematic (Gemini II / SubGemini) |
| Boolean/BDD DRC | Design rule checking | Geometric DRC rule compilation (Lyra, Magic) |
| SMT solving | Constraint placement + verification | Same Z3 backend generates and verifies (de Moura & Bjorner) |

### 3.7 Compiler Theory -> Analog P&R

| Technique | Applied to | Analogy |
|-----------|-----------|---------|
| Polyhedral optimization | Placement scheduling | ILP-like affine scheduling <-> ILP placement |
| Pass pipeline | Flow orchestration | RTL->place->route <-> compiler optimization passes |
| Register allocation | Net-to-track assignment | Graph coloring for routing-track assignment |

### 3.8 PCB Layout Automation -> Analog P&R

| Technique | Applied to | Key insight |
|-----------|-----------|-------------|
| Generate-many-score-select | Candidate evaluation | Generate multiple P&R candidates, score against physics/DRC, let designer pick |
| Constraint-aware RL | Placement/routing | Treat clearance/DRC as hard constraints, not soft penalties |
| Physics-driven validation | Post-route check | Validate against physics (SI, PI, thermal) not just geometry |

### 3.9 Analog Process Physics -> Analog P&R (Textbook Cross-References)

| Physics Domain | Key Reference | How It Informs the Engine |
|---|---|---|
| Design rule construction from process physics | [Hastings, Ch. 3, Section 3.2] | Explains WHY design rules exist (minimum feature size, linewidth control, mask alignment, outdiffusion, depletion width); essential for understanding which rules have margin vs. which are at the physical limit |
| Junction isolation and tank structures | [Hastings, Ch. 4, Section 4.1] | Standard bipolar fabrication sequence; junction isolation consumes more area than oxide isolation; NBL spacing from isolation required to avoid low-voltage avalanche |
| BiCMOS process integration | [Hastings, Ch. 4, Section 4.3] | CDI NPN (N-well as collector), twin-well architecture, deep-N+ sinker requirements, three poly resistor types (LSR/MSR/HSR), LDMOS layout rules |
| Dielectric isolation | [Hastings, Ch. 4, Section 4.3] | Wafer bonding + DTI provides superior minority carrier injection protection and substrate noise isolation; termination scheme B is best area-vs-reliability tradeoff |
| Pattern generation and OPC | [Hastings, Ch. 3, Section 3.3] | Analog uses less OPC than digital; never create exposed acute interior angles; process size adjusts handled by PG deck not designer; address unit constraints on coding grid |
| ESD protection devices and strategies | [Hastings, Ch. 14, Section 14.4] | GGNMOS, GCNMOS, BTNMOS, Active FET, MVSCR -- layout rules for each; pad-based vs. rail-based networks; CDM protection must be local; ground ring width computation |
| Verification methodology | [Lienig, Section 5.4] | Two-stage constraint mapping; formal vs. functional vs. geometric verification; PEX became critical below 0.5 um node; hierarchical LVS for runtime |
| VLSI design flow for analog | [Lienig, Section 4.1] | Analog uses layout generators or manual drawing, not standard cells; geometric representations created in real time; meet-in-the-middle design methodology |

---

## 4. Key Venues

**Tier 1 conferences:** DAC, ICCAD, ISPD, DATE, ASP-DAC, CICC, ISSCC, ISCAS
**Tier 1 journals:** IEEE TCAD, IEEE TVLSI, IEEE JSSC, IEEE TCAS-I/II, IEEE D&T
**Emerging:** NeurIPS/ICML (for ML-EDA), arXiv (for preprints)
