# Graeb (ed.), *Analog Layout Synthesis: A Survey of Topological Approaches* — Chapters 4–7

Source: H. E. Graeb (ed.), *Analog Layout Synthesis: A Survey of Topological Approaches*, Springer 2011.
Chapter 4 "Routing Analog Circuits" (G. Dündar, A. Unutulmaz), Chapter 5 "Analog Layout Retargeting" (H. Said, M. Dessouky, R. El-Adawi, H. Abbas, H. Shahein), Chapter 6 "Closing the Gap Between Electrical and Physical Design: The Layout-Aware Solution" (R. Castro-López, E. Roca, F. V. Fernández), Chapter 7 "Constraint-Driven Design Methodology: A Path to Analog Design Automation" (G. Jerke, J. Lienig, J. B. Freuer), plus the book's index.

Reftext file: `/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/balasa_graeb_survey.txt`, lines **8167–14994** (end of file), read in full.
PDF: `ref/Analog Layout Synthesis : A Survey of Topological Approaches -- ... .pdf`. PDF page = 1 + number of form feeds up to the line. Offsets: chapter 4 PDF = book + 13 (book p.149 = PDF p.162); chapters 5–7 and index PDF = book + 10 (book p.205 = PDF p.215).

Entry ID prefix: **BAL2-NN**. Stage keys: annotator | cells | gp | dp | gr | dr | verify | flow | deck.

---

## 1. Coverage

### 1.1 Read chunks (Read tool, consecutive)

The tool refused 2000-line and 1200-line chunks in dense regions (token cap), so the range was read in these consecutive chunks:

| # | offset .. end | content |
|---|---|---|
| 1 | 8167 .. 9366 | ch.4 opening through 4.5.3.1 (eq. 4.7) |
| 2 | 9367 .. 10466 | 4.5.3.1 end through 4.8 and ch.4 refs 1–26 |
| 3 | 10467 .. 11566 | ch.4 refs 27–102, ch.5 up to Theorem 5.1 proof |
| 4 | 11567 .. 12666 | Theorem 5.1 end through 6.3.2 (part) |
| 5 | 12667 .. 13766 | 6.3.2 end through 7.3 |
| 6 | 13767 .. 14416 | 7.4 through 7.6.1 (part) |
| 7 | 14417 .. 14994 | 7.6.1 end, 7.6.2–7.7, glossary, ch.7 refs, index |

Every line 8167–14994 was read. PDF pages opened to fix garbled equations: PDF p.186 (eqs. 4.6–4.7), p.190 (eq. 4.9), p.260 (eqs. 6.3–6.4), p.290 (eq. 7.1).

### 1.2 Section and subsection headings in the range (reftext line)

- 8167 Chapter 4 Routing Analog Circuits (abstract 8175)
- 8189 4.1 Introduction
- 8261 4.2 Basic Routing Algorithms; 8275 4.2.1 Maze Router; 8319 4.2.2 Line Expansion Routers; 8339 4.2.3 Channel Routing; 8421 4.2.4 Steiner Routing
- 8472 4.3 Representations; 8483 4.3.1 Layout Representations; 8490 4.3.1.1 Grid-Based Representations (8499 Uniform Grid Representation; 8530 Nonuniform Grid Representation); 8605 4.3.1.2 Tile Based Representations; 8627 4.3.1.3 Topological Representations; 8666 4.3.2 Connectivity Representation; 8738 4.3.3 Rule Representations
- 8793 4.4 Routing Issues and Techniques for Analog Circuits; 8806 4.4.1 Net Splitting; 8827 4.4.2 Symmetric Routing; 8912 4.4.3 Crosstalk and Shielding
- 8982 4.5 Routing Strategies for Analog Circuits; 8990 4.5.1 Digitally Inspired Early Routing Strategies; 9146 4.5.2 Routing Based on Cost Minimization; 9248 4.5.3 Routing Based on Parasitic Bounds; 9257 4.5.3.1 Constraint Generation and Sensitivity Calculation; 9382 4.5.3.2 Routers Based on Parasitic Bounding (9481 A* Algorithm); 9591 4.5.4 Integrated Placement and Routing; 9686 4.5.5 Global/Detailed Routing; 9800 4.5.6 Template Based Approaches; 9896 4.5.6.1 A Simple Template Script; 10087 4.5.7 Other Routing Strategies
- 10144 4.6 Specialized Analog Routers; 10152 4.6.1 Routing for RF Circuits; 10177 4.6.2 Routing for Analog Arrays
- 10301 4.7 Manufacturability and Yield Issues in Routing
- 10376 4.8 Conclusions; 10405 References (ch.4, [1]–[102])
- 10681 Chapter 5 Analog Layout Retargeting (abstract 10690)
- 10707 5.1 Introduction; 10766 5.2 Previous Work
- 10799 5.3 Analog Layout Retargeting Flow; 10830 5.3.1 Layer Mapping; 10844 5.3.2 Device Recognition; 10859 5.3.3 Constraint Generation
- 10884 5.4 Layout Compaction Methodologies: Background; 10895 5.4.1 The Constraint-Graph Approach; 10978 5.4.2 Linear Programming: The Simplex Method; 11173 5.4.3 Graph-Based Simplex Methods
- 11211 5.5 Multivariable Constraint-Graph Based Simplex Method; 11221 5.5.1 Basic Coefficient Constraint-Graph; 11377 5.5.2 Multivariable Graph-Based Simplex Algorithm (11386 Gain Equation; 11401 Graph Mathematics); 11470 5.5.2.1 Updating the Basic Coefficient Constraint-Graph; 11510 5.5.2.2 Dealing with Loops; 11573 5.5.2.3 Initial Basic Feasible Solution; 11635 5.5.3 Complexity Analysis (11661 Multivariable Graph-Based Algorithm Complexity)
- 11777 5.6 Layout Constraints Revisited (11788 Constraint Number Reduction; 11819 Complex Design-Rules; 11840 Edge Order; 11881 Nanometer Process Effects)
- 11902 5.7 Practical Retargeting; 11912 5.7.1 Symmetry Enforcement; 11942 5.7.2 Device Aspect Ratio; 11975 5.7.3 Layout Hierarchy; 12009 5.7.4 Device Replacement; 12026 5.7.5 Layout Parasitics
- 12042 5.8 Examples; 12146 5.9 Conclusion; 12174 References (ch.5, [1]–[36])
- 12263 Chapter 6 Closing the Gap Between Electrical and Physical Design (abstract 12272)
- 12293 6.1 Introduction; 12432 6.2 Previous Work; 12434 6.2.1 Circuit Sizing and Layout Generation; 12490 6.2.2 Previous Approaches to Layout-Aware Sizing
- 12572 6.3 Selected Approach to Layout-Aware Sizing; 12574 6.3.1 Sizing; 12652 6.3.2 Layout Generation and Parasitic Extraction; 12701 6.3.3 Putting It All Together
- 12752 6.4 Geometrically Constrained Sizing; 12769 6.4.1 Floorplan Sizing; 12860 6.4.2 Proposed Approach; 13008 6.4.3 Experimental Results
- 13094 6.5 Layout-Aware Sizing of AMS Circuits; 13096 6.5.1 Completing the Layout-Aware Sizing Methodology with Parasitic Extraction; 13212 6.5.2 Experimental Results of Layout-Aware Sizing
- 13269 6.6 Conclusions; 13295 References (ch.6, [1]–[38])
- 13398 Chapter 7 Constraint-Driven Design Methodology (abstract 13407)
- 13422 7.1 Introduction; 13514 7.2 Problem Description; 13516 7.2.1 The Design Problem; 13603 7.2.2 Analog Vs. Digital Design Automation
- 13667 7.3 Constraint Classification and Representation
- 13770 7.4 Components of a Constraint-Driven Design Flow; 13825 7.4.1 Constraint Management; 13900 7.4.2 Constraint Derivation; 13935 7.4.3 Constraint Transformation; 13978 7.4.4 Constraint Sensitivity Analysis; 14079 7.4.5 Constraint Verification
- 14189 7.5 Constraint Engineering; 14204 7.5.1 Constraint Programming; 14287 7.5.2 The Constraint Engineering System
- 14404 7.6 Impact Analysis; 14411 7.6.1 Impact on Design Flow; 14449 7.6.2 Impact on Design Methods; 14514 7.6.3 Impact on Design Algorithms
- 14578 7.7 Outlook; 14614 Glossary; 14709 References (ch.7, [1]–[32])
- 14786 Index (book index, pp. 299–302)

---

## 2. Section-by-section digest

### Chapter 4 — Routing Analog Circuits

**Abstract (8175–8185).** Reviews maze → sophisticated routers, data representations, analog-specific issues, strategies, RF and array routers, manufacturability; ends with open problems.

**4.1 Introduction (8189–8257, PDF 162–163).**
- Routing connects module terminals and ports; analog performance depends critically on routing parasitics, so it needs more care than digital (8191–8197).
- Routing quality is bounded by the earlier steps: device merging reduces the wires needed and their parasitics (Fig. 4.1a vs 4.1b) (8198–8250).
- Conclusion drawn: routing must be integrated with device generation and placement as much as possible (8246–8248).

**4.2 Basic Routing Algorithms (8261–8272).**
- Analog routing algorithms are the digital/PCB ones, tailored: analog has fewer paths but more constraints per path (8266–8272).

**4.2.1 Maze Router (8275–8316, PDF 164).**
- Lee's algorithm (extension of Moore to uniform grids): number grid points outward from source until the wavefront hits the target, backtrace (8277–8284, Fig. 4.2).
- Guaranteed to find a path if one exists, and it is shortest; time O(nm) on an n×m grid, memory per grid point (8285–8291).
- Speed/memory improvements in [3]–[7] (8292–8293).

**4.2.2 Line Expansion Routers (8319–8335, PDF 165).**
- Search space of line segments instead of grid points: much less run time and memory, fewer bends, but no shortest-path guarantee (8321–8326).
- Becomes cumbersome for complex routing; Hightower [9] may miss a solution that exists (8327–8331).

**4.2.3 Channel Routing (8339–8418, PDF 165–166).**
- Channel = horizontal area with fixed top/bottom pins, height computed by the router (8341–8345).
- VCG (directed, net x above y where pins overlap horizontally) and HCG (undirected, overlapping spans) fully represent the instance (8368–8379).
- Left-Edge algorithm when VCG has no edges; Constrained Left-Edge for acyclic VCG; cyclic VCG needs [12]–[14] (8379–8385).
- Not preferred in analog tools because parasitic effects are poorly estimated (8385–8387).

**4.2.4 Steiner Routing (8421–8468, PDF 167–168).**
- Pairwise routing makes result depend on net/pair order (net ordering problem) (8423–8428).
- Optimal multi-terminal routing is NP-complete (Steiner problem); rectilinear version MRST; rectilinear MST is at worst 1.5× the MRST length (Hwang [16]) (8428–8442).
- Heuristics: approximate Steiner point + maze routing to it; or route one pair and use the found path as a new wavefront source (8444–8449).
- Extensions: diagonal routing [17], multiple layers, performance criteria; ISPD 2007 global routing contest [18] (8450–8468).

**4.3 Representations (8472–8479).**
- Algorithm performance depends on data representation; covers layout, connectivity and rule representations (8474–8479).

**4.3.1 Layout Representations (8483–8487).**
- Three traditional options: grid-based, tile-based, topological (8485–8487).

**4.3.1.1 Grid-Based Representations (8490–8497).**
- Simple, suited to area routing; uniform (since Lee) and nonuniform (since the 1980s) (8492–8496).

*Uniform Grid Representation (8499–8527, PDF 168–169).*
- Fine grid: dense layouts but excessive memory; coarse grid: less memory but larger layouts and cannot represent wires whose size is comparable to the grid (8501–8508, Fig. 4.6).
- Analog needs variable-width routing and constraint observance, unlike digital; "route on uniform grid then compact" improvements are called unsatisfactory (8509–8527).

*Nonuniform Grid Representation (8530–8602, PDF 169–171).*
- More memory-efficient; can be finer in congested regions; faster parasitic evaluation than tiles because neighbourhood information is in the regular structure (8532–8537).
- Built by extending component boundaries to the layout boundary (Fig. 4.7a); fewer cells if extension stops at component boundaries [19] (Fig. 4.7b) (8538–8542).
- Grid graph G(V,E): V = empty cells, E = adjacency (overlapping boundaries) (8543–8589).
- Channel intersection graph: vertices at channel intersections, edges = channel segments; for line-expansion routing (8589–8595, Fig. 4.8).
- Minimum spacing folded into the graph by inflating each component by its layer's min space, except the source and target terminals being routed (8596–8602, Fig. 4.7d).

**4.3.1.2 Tile Based Representations (8605–8624, PDF 171).**
- Corner stitching [20]: stores empty space and components with corner pointers; suits line expansion (ANAGRAM II) (8607–8612).
- Efficient when block sizes differ widely (capacitors much larger than transistors); more complex; may need escape points (non-corner points) which enlarge the tile graph and shorten paths (8613–8624, Fig. 4.9).

**4.3.1.3 Topological Representations (8627–8662, PDF 171–172).**
- Slicing trees, sequence pair, O-trees express relative positions and symmetry well (8629–8634).
- Multi-SP [26] and channeled-BSG [27] proposed for simultaneous P&R, but they express block and wire vicinity only partially, so they are poor for parasitic extraction; better for placement than routing (8634–8662).

**4.3.2 Connectivity Representation (8666–8734, PDF 172–173).**
- Rip-up and reroute changes physical connectivity often; re-extracting each iteration is too slow, so a dedicated structure is needed (8668–8672).
- Requirements: net splitting (route high-current paths separately), fast connectivity service, fast nearest connectable component (8679–8684).
- Model: circuit C = {N_i}; net N_i = {S_ij} subnets; subnet S_ij = {G_ijk} groups of layout components. Same group = physically connected; same subnet different group = connection pending (8685–8734, Fig. 4.10).
- A subnet covers the wires of a net with similar current densities (footnote 3, 8725–8727).

**4.3.3 Rule Representations (8738–8789, PDF 174–175).**
- Router time depends on the service time of design and electrical rules (8740–8745).
- Two-level hash table keyed by layer1 then layer2 returns a rule in O(1), e.g. `get_capacitance_between(layer1, layer2)`; example value "Capacitance 50fF/mm2" (8745–8789, Fig. 4.11).

**4.4 Routing Issues and Techniques for Analog Circuits (8793–8802).**
- Parasitics cannot be eliminated, only reduced: split high-current from low-current paths, route symmetrically (8796–8802).

**4.4.1 Net Splitting (8806–8823, PDF 175).**
- Current densities in one net may differ by an order of magnitude (8808–8810).
- R_wire = l·R_sh/w (eq. 4.1); V_drop = R_wire·I on a shared segment moves the branch's "ground" off ground, time-varying (8811–8821).
- Fixes: separate the branch from the high-current path [28] (Fig. 4.12b) or widen the high-current wire (Fig. 4.12c) (8821–8823).

**4.4.2 Symmetric Routing (8827–8909, PDF 175–177).**
- Symmetric routing applies only when the terminals are symmetric about an axis (8872–8875).
- Early tools [30] ignored nonsymmetric obstacles: route one half, mirror (8876–8878).
- ANAGRAM II evolves both halves simultaneously; an obstacle on either side blocks both (8880–8898). ROAD mirrors obstacles onto the other side, routes one side, mirrors the result (8898–8900, Fig. 4.13b).
- Terminals on opposite sides with crossing paths: a "connector" crossing structure [31] matches R and C of both nets; residual coupling to other nets may differ (8901–8909, Fig. 4.14).

**4.4.3 Crosstalk and Shielding (8912–8979, PDF 177–178).**
- Extraction dimensionality 1-D, 2-D, 2.5-D, 3-D [32] (8914–8916).
- 1-D: C_1D = A·C_α + S·C_β (eq. 4.2): overlap area A times area capacitance plus overlap perimeter S times fringe per length (8917–8923).
- 2-D: C_2D = C_1D + l·C/d (eq. 4.3), l = parallel overlap length, d = separation, C = coupling per length; "commonly used in routing, due to its simplicity" (8924–8960).
- 2.5-D precomputes fringing from cross-sections; 3-D matches layout to a parameterised library; accurate but too slow inside path search (8961–8966).
- RF: RLC interconnect models [33], EM simulation (8967–8970).
- First add space; if impossible, insert a shield; ROAD can add shield lines; three shield types: same layer, different layer, bulk (substrate contacts); a shield must be tied to a DC potential or ground (8971–8979, Fig. 4.16).

**4.5 Routing Strategies for Analog Circuits (8982–8986).**
- Historical order: digital-like routers → cost-minimisation → parasitic-bound → integrated P&R (8984–8986).

**4.5.1 Digitally Inspired Early Routing Strategies (8990–9142, PDF 179–182).**
- Two-step global/detailed routing, heuristic costs used feedforward (Fig. 4.17) or in a weak feedback loop on intermediate variables (overlap areas), so performance is not guaranteed (8996–9053).
- Constraints considered: interconnect area, crosstalk, crossovers and vias, wiring density; driving goal was crosstalk minimisation (9017–9021).
- A-priori cost estimates allow net ordering: critical nets first take privileged locations and become obstacles for later nets (9054–9060).
- MIGHTY/OPASYN: order by net function (input, output, ...) and rip-up-reroute; feasible because analog has few nets (9061–9067).
- ILAC: four classes — sensitive, noisy, noncritical signal, power. Order: power, sensitive, noncritical, noisy last; power and noncritical nets act as shields between critical and noisy nets. Global maze router handles coupling, undesired crossings, planarity of power nets and congestion; channel size from number of nets; scan-line incremental channel router with stretchable spacing minimises distance, layer switches, channel growth, and running beside noisy/sensitive nets (9068–9084).
- SALIM: channels from the slicing tree; electrical constraints (low-R supplies, few crossings, priority for sensitive nets, symmetry) mostly supplied as user rules; gridless variant routes symbolic zero-width tracks first, meets crosstalk/R/C/symmetry, then design rules; connection order equals placement order (9085–9103).
- SLAM: critical-net analysis assigns per-node distance constraints with priority (highest priority → shortest wires) and prioritised pairwise spacing (sensitive–noisy highest) (9104–9112).
- [44]: slicing tree to one module per box, hierarchical channel graph, approximate RST up the hierarchy with pseudo pins; noisy nets routed crossing-free with traffic-light routing and terminal grouping (9113–9123).
- [45,46]: AI design plan; routes ordered by sensitivity, distance, knowledge-base constraints, straightness; option paths kept for alternatives; virtual grid → stick diagram (precursor of templates) (9126–9142).

**4.5.2 Routing Based on Cost Minimization (9146–9244, PDF 182–184).**
- Area routing with a multi-objective cost; cost terms are C to ground, inter-node C, wire R, but without an explicit link to performance constraints (9148–9162).
- ANAGRAM: line-expansion router on a uniform grid graph; route cost eq. 4.4 = Cost(P) + MaterialCost(C) + ParasiticCost(C) + RoutabilityCost(C) + CostToTarget(C,T) (9163–9181).
- Partial paths kept in a heap, cheapest popped and expanded to the next feature; search complexity quadratic in circuit size; effective for distributed terminals (capacitor perimeters, multi-transistor modules) and multi-terminal nets (9181–9194).
- Wire classes neutral (supply, bias), noisy (high swing: clock, outputs), sensitive; coupling penalised against already-routed wires; rip-up-reroute removes crosstalk violations due to ordering. Weakness: no link to performance specs; congestion relies on aggressive rip-up (9194–9207).
- ANAGRAM II: arbitrary rectangles instead of grid segments, over-the-device and arbitrary-width wiring, guaranteed symmetric differential wiring; RoutabilityCost = cost of ripping up neighbours, trading detour vs. removal (9208–9229).
- [30]: modified Lee with per-net user-weighted cost of distance, layer resistivity, layer-to-bulk capacitance, proximity to existing wires, congestion (current plus estimate from a fast first-attempt search of the remaining wires); corner clean-up by backtrace; preconnected pins (stacked transistors); symmetry; scheduler from symmetry, user and congestion data, plus rip-up (9230–9244).

**4.5.3 Routing Based on Parasitic Bounds (9248–9255, PDF 184).**
- ROAD and ANAGRAM III route to keep parasitics within acceptable bounds from designers or sensitivity analysis (Fig. 4.18 flow: netlist + performance constraints + sensitivities → parasitic constraint generation → router) (9250–9294).

**4.5.3.1 Constraint Generation and Sensitivity Calculation (9257–9379, PDF 184–186).**
- Constraint generator maps high-level performance specs to parasitic bounds [50–52]; routing = constraint-driven optimisation, objective = chip area, constraints = performance (9259–9303).
- Parasitics considered: line R, C, L, line-to-line C and L; bounding constraints and matching constraints; parasitics that collectively cause negligible degradation are dropped; the rest are "critical parasitics" (9303–9311).
- Constraints also usable to modify VCGs in channel routing for mixed-signal [53,54] (9313–9314).
- S_ij = ∂W_i/∂p_j at p_j = 0 (eq. 4.5). Perturbation is slow and error-prone (especially transient: difference of two close numbers); direct or adjoint sensitivity is faster and more accurate (9314–9333).
- Linearised constraints (eqs. 4.6–4.7, verified on PDF p.186): Σ_j S⁺_ij p_j ≤ ΔW⁺_i,max for W_i ∈ W⁺ and Σ_j S⁻_ij p_j ≤ ΔW⁻_i,max for W_i ∈ W⁻, with S⁺_ij = S_ij if S_ij ≥ 0 else 0, S⁻_ij = −S_ij if S_ij ≤ 0 else 0 (9334–9360).
- Infinitely many bound sets satisfy these; simplify by thresholding or ICA; heuristics: sensitivity graphs [55], PARCAR flexibility values [31]: flexibility from estimated min/max of each parasitic, refined as the layout evolves, used in a QP; less flexible parasitics get more effort (9361–9379).

**4.5.3.2 Routers Based on Parasitic Bounding (9382–9479, PDF 187–189).**
- [56]: constraint generator finds critical parasitics and bounds; for differential circuits matching constraints from matched node pairs with worst-case sensitivities including mismatch; sensitivities become weights of an A* maze router whose path cost is the performance impact, not the raw parasitic (9384–9397).
- Bounds are generated so that all parasitics within bounds ⇒ all performances within bounds; so a violated performance implies at least one parasitic over its bound. Naive raise/lower weight updates can oscillate; [56] gives a heuristic update (9397–9406).
- ROAD: A* maze router on a 3-D nonuniform grid with dynamic allocation; local refinement limits size and aspect ratio of grid rectangles; every new wire refines the grid; over-the-device routing allowed but sensitive modules can be excluded; terminals/blockages are arbitrary geometry (9407–9431).
- ROAD cost = weighted sum of local area crowding, resistance, capacitance to bulk, cross capacitance; weight ∝ sensitivity / maximum allowed variation of that performance (9431–9438).
- ROAD schedule: more constraints on a net → higher priority; symmetric nets very high priority. Loop: route, extract, estimate degradation; if violated raise weights of the most sensitive parasitics and reroute; when all are at maximum, placement must be regenerated with a wider variation range for those parasitics (9438–9449).
- ROAD net splitting by a heuristic estimating low-current parts; shields are a last resort built after all wires; NDL command language sets weights, symmetry, net-splitter nets (9449–9463).
- Crosstalk-aware area routers use a four-term cost (material, crosstalk penalty, routability, cost-to-target); proximity checks at each partial-path head are unavoidable overhead; ANAGRAM III prunes parasitically non-viable partial paths, finding the cheapest path that meets hard parasitic bounds without trading wirelength against crosstalk (9464–9478).

*A\* Algorithm (9481–9587, PDF 189–191).*
- Worst case O(n²) like Lee–Moore, average much better [57–59] (9483–9487).
- f(x) = g(x) + h(x) (eq. 4.8); h must not overestimate; Manhattan distance when only length counts (9493–9500).
- Analog: h = resistive cost of the Manhattan path to target; g(x) = A·(G_{x−1} + ΔG_x) (eq. 4.9), ΔG_x rows = Σ_j c_ijn·ΔC_ij + r_n·ΔR per performance metric n, A = weight vector; r_n = 0 → capacitive only, c_ijn = 0 → resistive only (9501–9540, verified PDF p.190).
- Open set as red-black tree of (f, list of nodes with that f) to handle ties; closed set as a hash set (9541–9587, Figs. 4.20–4.21).

**4.5.4 Integrated Placement and Routing (9591–9682, PDF 191–193).**
- Sequential P&R uses a rough wirelength estimate, so the placement is not optimal for global routing; analog's few nets make combined P&R affordable and help enforce parasitic constraints (9593–9609).
- Granularity: from routing between placement iterations to routing as placement of wires (9609–9614).
- KOAN/ANAGRAM II [61]: annealing manipulates devices and wires; wires always electrically connected; cost includes DRC violations, crosstalk, net area (9615–9631).
- RACHANA [62]: schematic graph (assumes the drawn schematic has the best distribution), constraint-driven module variants, floorplan picks variants, then place-a-module/route-its-connections interleaving; gridless multilayer router (9632–9642).
- GELSA [63,64]: approximate routing at each SA step on slicing structures; cost has global symmetry about virtual axes and local symmetry of groups (9645–9655).
- [65]: sequence pair for blocks and wire rectangles with wire-connectivity order conditions and H/V constraint graphs for compaction (9656–9661).
- [66]: hybrid GA global placement with HPWL, then fast simulated reannealing with min-Steiner-tree global routing per intermediate solution using net sensitivity and channel congestion; global-route cost scores the placement (9662–9676).
- [67]: tiles moved, swapped, routed, resized under tabu search with constraint transformation; superior to sequential flow (9677–9682).

**4.5.5 Global/Detailed Routing (9686–9796, PDF 193–196).**
- ALADIN: global routing inside placement; estimators HPWL, center-of-mass, complete graph, MST all Manhattan-based and unreliable; min-Steiner-tree estimator on a rectilinear channel graph solved with Dijkstra; channel weight = channel length plus congestion (number of nets through it) (9688–9729).
- Compaction-based constructive detailed routing: order heuristic starting from symmetric and more central modules; intra-module wiring dense around the module boundary with a ring router; module compacted toward the reference; inter-module connections maze-routed in a small scope; no routing-area estimate needed (9729–9744).
- PARSY [72]: symmetric routing of net bundles; corners cause length differences; aggressors affect bundle members differently; choose routing area, add extra paths and adjustment sections (9745–9753).
- Yılmaz/Dündar [73,74]: channel intersection graph; backtracking maze global routing exploring all solutions but pruning branches that exceed performance bounds using predicted node-to-node/ground C and sensitivities; local routing with channel operations push/propagate/peak/pop and a refinement to reduce level transitions; switchbox routing (≤3×3) enumerated exhaustively for coupling (9754–9796).

**4.5.6 Template Based Approaches (9800–9893, PDF 196–198).**
- Designers want control and incremental change; probabilistic generators give different layouts per run; templates with pre-extracted parasitics give reproducible updates (9802–9821).
- [75]: template + electrical parameters + shape constraint; exhaustive floorplan area optimisation picks device shapes; compaction transforms the template; routing only stretches/compacts (9822–9840).
- BALLISTIC [76] layout language (relative placement and wiring commands; a few hundred lines per medium cell); CAIRO [77,78] C-function superset with relative routing to reference points (9841–9853).
- [79,80]: hierarchical parameterised templates for retargeting; interconnect parameterisation considers design rules and current density (9854–9869).
- LAYGEN [81,82]: adapt template routing to a new placement (scale path, move start to new start pin, set stop to new stop pin; nets split into 2-pin wires), then genetic optimisation with cost of DRC violations, connectivity, wirelength, inter-net distance; one gene per net; routing validation costlier than placement (9870–9893).

**4.5.6.1 A Simple Template Script (9896–10084, PDF 198–200).**
- LDS: positions as equations on element boundaries (top, bottom, left, right); each line is horizontal or vertical only (9898–9907, Fig. 4.22).
- Extracted from a placement via H/V constraint graphs; a simple "right edge sees left edge" graph can overlap after resizing (Fig. 4.23b); extended graph adds edges from an element's right boundary to every element with higher x and nothing in between, guaranteeing no overlap for any sizes; compaction removes slack; overhead is small (9908–9946).
- Spacing constraints only between same-layer shapes of different nets, per layer (9946–9952).
- Wire paths: a 3-wire path fixes order; a 4-wire "dynamic" path lets the path flip orientation (Figs. 4.24c,d); constraints solved by LP (9953–9960).
- Demo: two-stage opamp template resynthesised with enlarged M3–M4 and narrowed M6/M7 without overlaps or min-width/spacing violations (9997–10005, Fig. 4.25).

**4.5.7 Other Routing Strategies (10087–10140, PDF 203–204).**
- Probabilistic routing [83]: resistor array per routing area with resistances from probability incl. constraints; simulate, iteratively remove low-probability resistors; [84] per-grid-point probabilities converted to priorities, with symmetry (10089–10103).
- x–y routing [85]: two adjacent layers, horizontals on one, verticals on the other, stacked vias; phases generate candidates with different via counts, mostly inside the net bounding box; multi-terminal nets via MST (t−1 two-terminal edges); compatibility graph (edge = incompatible), reduced to an independent set, one candidate per net (10104–10127).
- Constraints: parasitics, geometry, symmetry, wire widths, layer confinement, keep-outs, parallel distance and coupled length vs width; all nets considered together rather than net-by-net; extended to 45° [86] (10127–10140).

**4.6 Specialized Analog Routers (10144–10148).** RF and analog arrays need different routers.

**4.6.1 Routing for RF Circuits (10152–10174, PDF 204).**
- CYCLONE [87,88] uses LAYLA for P&R, implying no special RF routing (10156–10159).
- CORAL [89]: A* area routing; inductive/capacitive crosstalk modelled as degradation of Z0 and loss (or discrete RLC at low frequency); analytical models fitted to 2-D/3-D solver data (10160–10166).
- Two phases: constraint-based routing, then refinement expanding all nets simultaneously to enforce fixed interconnect lengths without new parasitic violations (10167–10174).

**4.6.2 Routing for Analog Arrays (10177–10297, PDF 205–207).**
- Regular arrays: flash ADCs, cellular neural networks, current-steering DACs (10179–10184).
- Mondriaan [91,92]: channels across/between cells; ground/bias/supply by abutment; cells flipped to share lines; most connections go to edge pins, unlike FPGAs; metric is equal R/C/matching, not delay (10187–10202).
- Algorithm: vertical wires per column; IO pins pick free vertical wires, unplaced cells placed in free slots; column scan propagates connectivity; horizontal wire count from vertical wires needing connection; bus and tree generators (10202–10214).
- FPAA routing (FAAR [93]): fixed switches; four subproblems (2-terminal with 0–3 global wires, multi-terminal with maximal sharing, multiple nets = NP-complete compatibility not addressed, performance bound as max switches/crossings with rip-up on violation) (10215–10285).
- [94] segmentation + buffer insertion; [95] hexagonal reconfigurable CABs; [96] simulator-in-the-loop, parasitics may form part of a filter (10286–10297).

**4.7 Manufacturability and Yield Issues in Routing (10301–10372, PDF 207–209).**
- Among routings meeting specs, prefer higher yield/manufacturability/testability (10303–10308).
- [97]: after performance-driven line-expansion routing, if margin remains, rip-up/reroute nets to improve yield/testability until margin is used; partial paths exceeding margin are removed from the heap; expected bridging faults added to cost: dielectric pinholes (critical area = inter-level overlap), photolithographic defects (same-level parallel conductors; critical area vs defect size × size distribution); testability by supply-current monitoring with Monte Carlo (10309–10335).
- Current-driven routing for EM [98–101]: currents by simulation (user patterns or Monte Carlo); per-segment widths; greedy Steiner points from three terminals summing currents; terminal-to-terminal sequence fixed beforehand lets KCL fix currents before detailed routing; current-density verifier (10337–10360).
- [102]: area-first terminal tree built bottom-up (min-current terminal joins nearest neighbour, width set per pair), then simulated annealing (10361–10372).

**4.8 Conclusions (10376–10401).**
- Analog routing still open; move toward one-step layout generation; RF interconnects are devices; deeper-submicron rules and yield need new algorithms; templates expected to spread (10386–10401).

**References ch.4 (10405–10680).** [1]–[102]; key: ROAD [19], KOAN/ANAGRAM II [21,61], Malavasi constraints [31], Choudhury constraint generation [50–54], ANAGRAM III [49], ALADIN [68,69], PARSY [72], Lampaert yield routing [97], current-driven routing [98–102].

### Chapter 5 — Analog Layout Retargeting

**Abstract (10690–10703).** Retargeting preserves the source layout's knowledge via compaction; a graph-based simplex supports symmetry, hierarchy, cell replacement; linear-like for simple constraints, simplex-like for complex.

**5.1 Introduction (10707–10763, PDF 215–216).**
- Analog IP migration is manual and redesigned per node; designers accept automation for migration more than for new design (10730–10750).
- Compaction keeps source-layout characteristics; chapter outline (10751–10763).

**5.2 Previous Work (10766–10796, PDF 216–217).**
- Digital hard-IP migration [1,2]; analog adds matching/symmetry [3]; behaviour tied to physical design [4] (10768–10774).
- Designers distrust optimisation-based retargeting that re-invents the layout [5]; [6] reused only relative block positions and aspect ratios; compaction-based [7,8] keeps floorplan/placement/routing (10777–10796).

**5.3 Analog Layout Retargeting Flow (10799–10826, PDF 217–218).**
- Inputs: source netlist + layout, target netlist with new sizes, target rules, optional user constraints; modules: Layer Mapping, Constraint Generation, Device Recognition, Compaction (10801–10816, Fig. 5.1).
- The industrial DRC/LVS tool (Calibre) is reused for process files and performance (10816–10826).

**5.3.1 Layer Mapping (10830–10841).** One-to-many/many-to-one layer map per process pair; adjacent contacts/vias merged; unavailable devices replaced (10832–10841).

**5.3.2 Device Recognition (10844–10856).** Netlist migration [10] changes sizes; LVS-style recognition links layout devices (including fingered and matched) to netlist elements (10846–10856).

**5.3.3 Constraint Generation (10859–10880, PDF 219).**
- Target rules translated to layout constraints using the DRC engine in an internal mode; not one-to-one across processes (10861–10870).
- Constraint count drives compaction efficiency; remove redundancy; new device sizes become dimension constraints (10871–10880).

**5.4 Layout Compaction Methodologies: Background (10884–10891).** Minimise area under design rules; constraint graphs and LP [11,12] (10886–10891).

**5.4.1 The Constraint-Graph Approach (10895–10974, PDF 219–221).**
- 1-D; constraint x_j − x_i ≥ d_ij (eq. 5.1); vertex per edge coordinate; edge weight d_ij; source v0 at x = 0; end vertex vn (10897–10936, Fig. 5.2).
- Min-distance-only graph is a DAG; longest path from v0 gives each minimal coordinate (10937–10946).
- Drawbacks: everything pushed left ("long wires", Fig. 5.3) which hurts shape, parasitics and the other-direction pass [15,16]; only two-variable constraints, whereas symmetry x_b − x_a = x_d − x_c (eq. 5.2) and hierarchy need more (10947–10974).

**5.4.2 Linear Programming: The Simplex Method (10978–11169, PDF 222–226).**
- 1-D compaction as two LPs; min f s.t. f_i(x) ≤ d_fi, x ≥ 0 (eqs. 5.3–5.4); slacks (5.5); matrix form min cᵀx s.t. Ax = d (5.6) (10980–11029).
- BFS definition; basic/nonbasic split (5.7); x_B = B⁻¹d − B⁻¹N x_N (5.8); f = zᵀx_N + c_Bᵀ B⁻¹d (5.9); z = c_N − (B⁻¹N)ᵀ c_B (5.10) (11030–11077).
- Steps: entering = most negative z; step vector Δx_B = B⁻¹N a_e (5.15); ratio test t ≤ x_i/Δx_i, leaving = min ratio (5.16–5.18); update, repeat (11078–11158, Fig. 5.4).
- Cost dominated by B⁻¹; LU helps for sparse B; pure simplex too slow for industrial layouts vs graph methods (11159–11169).

**5.4.3 Graph-Based Simplex Methods (11173–11207, PDF 226–227).**
- Exploit that most constraints are two-variable distances [16,19–21]: Marple (distance only + wirelength objective), Onozawa (up to three variables), Wang & Lai (reduced core matrix ∝ number of multivariable constraints; no general initial BFS; objective with one variable) (11175–11203).

**5.5 Multivariable Constraint-Graph Based Simplex Method (11211–11217).** Graph method supporting any linear objective and constraints (11213–11217).

**5.5.1 Basic Coefficient Constraint-Graph (11221–11374, PDF 227–230).**
- Nodes for every variable (incl. slacks) and every constraint; weighted arcs variable→constraint = coefficient; bias node x_bias = 1 carries constants (11228–11247).
- Supernode (c_j, x_k): constraint solved for its basic variable, arc weight = ∂x_k/∂x_i (11249–11261); symmetry example c3: x2 − x1 − x4 + x3 = 0 (11284–11291, Fig. 5.5c).
- Worked example eqs. 5.25–5.27 with f = x8 − x5 − x0 (11336–11374, Fig. 5.8).

**5.5.2 Multivariable Graph-Based Simplex Algorithm (11377–11467, PDF 231–233).**
- *Gain Equation (11386–11399):* ∂x_j/∂x_i = Σ path gains (eq. 5.28) when loop-free (Mason's rule).
- *Graph Mathematics (11401–11438):* Rule 1 x_B = ∂x_B/∂x_bias (5.29–5.30); Rule 2 z = ∂f/∂x_N (5.31); Rule 3 Δx_B = −∂x_B/∂x_e (5.32).
- Same 8 steps as simplex, done by graph connection values (11439–11467).

**5.5.2.1 Updating the Basic Coefficient Constraint-Graph (11470–11507, PDF 233–234).** Find path entering→leaving variable, detach and re-associate supernodes along it, renormalise so the in-supernode coefficient is 1; only arcs touching updated nodes change (11472–11507, Figs. 5.9–5.10).

**5.5.2.2 Dealing with Loops (11510–11569, PDF 235–236).** Eliminate loops by successive substitution (example yields x6 = 7 + (4/3)s7 + (4/3)s8 + (1/3)s6 — signs lost in text, not needed); nonbasic nodes never in loops; Theorem 5.1: every loop contains at least one nondistance constraint (distance-only loop would make B singular) (11512–11569).

**5.5.2.3 Initial Basic Feasible Solution (11573–11632, PDF 236–237).**
- Longest path gives the start; satisfied multivariable constraints take their slack as basic; unsatisfied or slack-less (symmetry) get an artificial variable with big-M penalty in f (11589–11602).
- Example: x4 − x3 = x2 − x1 with longest-path values 10, 30, 10, 50 → a1 = 20 (eqs. 5.35–5.36) (11603–11619).
- Equality x_j − x_i = K (5.37) for exact device dimensions and contact/via sizes handled the same way (11620–11629).

**5.5.3 Complexity Analysis (11635–11773, PDF 238–241).**
- Simplex iteration O(R³) (5.38); longest path O(N + E) ≈ O(N) (5.39) (11643–11658).
- Graph method: connection values O(k + k_f + u + p·v) (5.40); loop elimination per constraint O(p·(u+v)) (5.41), total O(v·p·(u+v)) (5.42) (11661–11713).
- Cases: distance only → linear O(k + k_f + u) (5.43); u ≫ v → linear plus O(v·p·u) (5.44–5.45); u ≪ v, p ≈ N → O(v²), O(v³) = simplex (5.46–5.47); only nodes affected by the entering variable are recomputed (11715–11773).

**5.6 Layout Constraints Revisited (11777–11786).** Constraint number and kind drive cost; covers reduction and 1-D issues (11779–11786).
- *Constraint Number Reduction (11788–11816):* merge each contact/via array into one large cut before compaction, size it to keep at least the source cut count, regenerate minimum cuts after (Fig. 5.12).
- *Complex Design-Rules (11819–11837):* polygon-based and multivariable rules; conditional rules (wide-metal spacing), conjunctive/context rules depending on another layer [26–28]; DRC engine converts them to edge form.
- *Edge Order (11840–11879):* min-distance constraints freeze edge order; after resizing, obsolete constraints block moves (Fig. 5.13); fixes: iterate H/V compaction regenerating constraints, or 2-D compaction (NP-complete, heuristics [29]); tools use 1-D.
- *Nanometer Process Effects (11881–11899):* STI stress and WPE [4] change device performance; migration preserves them only if handled in the source; compaction minimises wells, so detect non-minimum wells and treat specially; objective favouring closeness to source [2].

**5.7 Practical Retargeting (11902–11909).** Symmetry, aspect ratio, hierarchy, replacement, parasitics all fit the multivariable method (11904–11909).

**5.7.1 Symmetry Enforcement (11912–11939, PDF 244).** Relative accuracy needs symmetry; detected manually or by multilevel detection from the extracted netlist [3]; Okuda [12] solves a reduced LP with revised simplex and converts to distance constraints; general method needs no such split (11914–11939).

**5.7.2 Device Aspect Ratio (11942–11971, PDF 245–246).** Keeping fingers/unit counts wastes corners after size changes (Fig. 5.14b); 2-D compaction would fix it; [31] changes device realisation (finger count) under the same relative placement (Fig. 5.14d) (11943–11971).

**5.7.3 Layout Hierarchy (11975–12005, PDF 246).** Hierarchical compaction [17]: intracell constraints once per cell using coordinates relative to cell origin; edge = x_cell^top ± x_cell^level1 … ± x_edge (eq. 5.49); [32] trims variables (11977–12005).

**5.7.4 Device Replacement (12009–12022, PDF 246–247).** Replace devices with target PDK Pcells (e.g. finger change, missing poly resistor); [34] cell-swap on hierarchical LP (12011–12022).

**5.7.5 Layout Parasitics (12026–12038, PDF 247).** [35] nonlinear optimisation bounds parasitics during retargeting; careful source layouts plus closeness objective usually keep matched parasitics (12028–12038).

**5.8 Examples (12042–12143, PDF 247–250).**
- Miller and folded-cascode opamps, 0.35 µm → 0.18 µm with symmetry; graph method 2–4× faster per iteration than revised simplex (lp_solve), ≥10× total thanks to a good initial BFS (12044–12084, Table 5.1).
- Table 5.1: Miller X 521 vars/1,831 constraints: 2.517 vs 4.555 ms/iter, 0.375 vs 3.594 s; Y 521/2,067: 1.164 vs 5.175 ms, 0.234 vs 4.265 s; folded-cascode X 2,431/8,599: 9.270 vs 21.492 ms, 8.232 vs 91.345 s; Y 2,443/11,495: 7.260 vs 28.297 ms, 7.391 vs 118.141 s (12063–12071).
- Areas: Miller 6,060 → 2,800 µm²; folded cascode 10,270 → 4,445 µm² with an empty region caused by irrelevant edge constraints (12085–12091).
- VCO 130 nm → 65 nm: edge-order locked space (Fig. 5.16b) until constraints deleted manually (12092–12102).
- Astable oscillator (~500 devices, 0.6 → 0.25 µm): Table 5.2 X 14,495 edges, 71,197 min constraints, 2,857 symmetry/equal, 6,750 iterations; Y 14,469 / 75,494 / 2,676 / 6,540 (12103–12142).

**5.9 Conclusion (12146–12164).** Compaction with the right objective preserves design knowledge; graph simplex tends to linear or to simplex depending on constraints (12148–12164).

**References ch.5 (12174–12262).** [1]–[36], incl. Calligrapher [2], multilevel symmetry [3], Drennan proximity [4], Okuda symmetry compaction [12], Wang–Lai [20], lp_solve [36].

### Chapter 6 — The Layout-Aware Solution

**Abstract (12272–12289).** Parasitic-aware sizing is incomplete without geometry (area, aspect ratio); layout-aware sizing combines simulation-based optimisation, procedural (template) layout, exhaustive geometric evaluation, parasitic estimation.

**6.1 Introduction (12293–12428, PDF 253–256).**
- IC complexity grows 58%/yr vs productivity 21%/yr (12312–12314); remedies: reuse, hierarchical synthesis, avoiding iterations (12319–12323).
- Over-estimating parasitics wastes power/area; under-estimating risks failure (12350–12352); area and parasitics are coupled (12359–12361).
- Features 1–5 of the method: simulator accuracy, global optimisation, parasitics inside sizing, template speed, realistic area incl. routing, guard rings, separations (12375–12389).
- Two techniques: geometrically constrained sizing (GPs such as number of folds) and parasitic-aware sizing; different foldings change junction capacitances (12397–12421).

**6.2 Previous Work (12432).**

**6.2.1 Circuit Sizing and Layout Generation (12434–12486, PDF 256–257).**
- Knowledge-based sizing is fast but inaccurate and hard to port (12438–12448); layout knowledge: rule-based vs template-based (12451–12463).
- Optimisation-based: equations or simulation; statistical (escapes local minima) vs deterministic (12466–12480).
- Optimisation-based layout cost: area, net length, penalties for mismatch and crosstalk; general but complex, lower quality, slow (12481–12486).

**6.2.2 Previous Approaches to Layout-Aware Sizing (12490–12569, PDF 257–259).**
- Table 6.1 compares [16],[25]–[29] and this work by engine, evaluation, estimation, layout generation, geometric constraints (12498–12522).
- [25] slice-level LP for geometry but only near optimum; [26] layout only in fine-tuning, folds only; [27] no geometry; [28] templates + symbolic, ~20% faster, small-signal only; [29] equations (12526–12569).

**6.3 Selected Approach (12572).**

**6.3.1 Sizing (12574–12648, PDF 259–261).**
- Knowledge-based sizing deviations "several hundred percent" in worst cases (12578–12582).
- Simulator-in-the-loop SA with coarse-grid pre-exploration, adaptive temperature, move amplitude tied to temperature, embedded C design knowledge (12595–12606).
- min y_oi(x) s.t. y_rj ≥ Y_rj and/or ≤ Y_rj (6.1–6.2); infeasible cost ψ = max_j[−w_j log(y_rj/Y_rj)] (6.3); feasible ψ = Φ(y_oi) = −Σ_i −w_i log|y_oi| (6.4) (verified PDF p.260) (12607–12637).
- SA phase then Powell local optimisation; HSPICE evaluator (12638–12648).

**6.3.2 Layout Generation and Parasitic Extraction (12652–12698, PDF 261–262).**
- Trial-and-error parasitic correction is unsystematic; degradation often from several parasitics combined (12654–12663).
- Optimisation-based layout too slow for the sizing loop; templates take "a few seconds", carry full geometry, encapsulate expertise (12666–12679).
- Footnote: a template costs 1.5–2× the time of manual layout of the block; symmetries coded into templates (12682–12689).

**6.3.3 Putting It All Together (12701–12748, PDF 262–263).**
- Inputs: netlist + specs, binary slicing tree template, geometric goals, process data (12725–12732).
- Per iteration: new design vector → GC module enumerates geometric parameter combinations, returns min area and area loss meeting goals → instance extracted, parasitics added → simulate → cost (12733–12748).

**6.4 Geometrically Constrained Sizing (12752–12760).** Use geometry during sizing to meet aspect ratio and area and get parasitic estimates (12754–12760).

**6.4.1 Floorplan Sizing (12769–12856, PDF 264–266).**
- Floorplan sizing: choose each component's GP values (fingers) minimising φ(W,H) under a fixed template placement (12771–12782).
- Area loss: unused area not required by rules (e.g. n-well spacing) relative to total (12783–12792).
- Slicing floorplans: easier relative placement, more compact, easier routing evaluation, easier constrained sizing; slicing tree with m nonleaf and n = m + 1 leaves (12793–12812).
- Table 6.2 maps tiles to devices, GPs (fingers, capacitor X dim) and building blocks (12823–12841); shape function = Pareto-optimal (w,h) list per leaf (12853–12856).

**6.4.2 Proposed Approach (12860–13004, PDF 266–269).**
- Alternatives: Stockmeyer [36] or LP [25]; prior analog uses apply it after sizing only (12862–12872).
- Goals: minimise area and area loss; complete geometry for parasitics; user geometric constraints (aspect ratio, max W/H) (12875–12882).
- Each leaf's shape list from generators (folded transistor, diff pair, cascode, mirror, cap array, folded resistor) including styles (common-centroid, interdigitated) (12883–12912).
- Bottom-up combine keeping Pareto pairs sorted h_i > h_{i+1}, w_i < w_{i+1}, list length ≤ Π l_i; template slice separations added; complexity O(d·l_T) (12914–12931).
- Floorplan-sizing matrix: W, H, area, aspect ratio, area loss = W·H − Σ W_i·H_i − routing area (12932–12940); example 3,000 points (12940).
- Top-down: drop rows violating AR ∈ [AR − ΔAR, AR + ΔAR] or max/min W, H; pick min area; propagate to leaves → GPs (12958–12978).
- Wire width self-adapts to current per the layer's max current density (footnote 5, 12944–12946); symmetries live in the template, not in GC (footnote 6, 12984–12986).
- Area also in the cost (6.3); area loss optional objective (12979–13004).

**6.4.3 Experimental Results (13008–13091, PDF 269–271).**
- Opamp of Fig. 6.3, GPs = folds and unit capacitor side; load 50 kΩ, 5 pF; objectives area, power, area loss (13013–13035).
- Table 6.4: Exp#1 AR [0.91, 1.1] → 1.00 (165.7 × 165.85 µm); Exp#2 AR [1.95, 2.05] → 1.96 (229.0 × 116.7); Exp#3 W < 150 → 143.2 (H 171.1); Exp#4 W < 300, H < 150 → 186.9 × 138.1 (13054–13060).
- Table 6.5: power 3.18/3.21/4.93/2.98 mW; area 27,481/26,724/24,502/25,811 µm²; area loss 1.94/3.06/0.93/0.57% (13063–13067).
- Table 6.6: 1,708/5,778/6,462/4,824 iterations; 409.92/1380.9/1563.8/1157.76 s on a 1.3 GHz Pentium IV (13077–13091).

**6.5 Layout-Aware Sizing of AMS Circuits (13094).**

**6.5.1 Completing the Methodology with Parasitic Extraction (13096–13189, PDF 272–273).**
- Accurate parasitics need all GP values; GC runs before extraction (13099–13107).
- MOS extraction geometric (measure AS/AD/PS/PD) or analytical (functions of fingers, style, process); interconnect numerical, analytical-geometrical (A–G), lookup; A–G fast and accurate, lookup storage explodes with parameters (13108–13128).
- Approach A: geometric MOS + 3-D A–G interconnect every iteration (13129–13148). Approach B: analytical diffusion equations per style [38] + template sampling lookup (13150–13163).
- Sampling m points per n parameters gives mⁿ instances; two steps: ~100 instances to find critical parasitics by simulation, then dense sampling only of parameters affecting them; table built once per template (13163–13189).

**6.5.2 Experimental Results of Layout-Aware Sizing (13212–13266, PDF 274–276).**
- Exp I no layout info (area = Σ W·L), Exp II approach B, Exp III approach A; load 100 kΩ, 8 pF (13216–13228).
- Table 6.7: Exp I UGF 91.8 MHz nominal, 89.8 after extraction vs goal 90; PM 67.6° → 63.4° vs goal 65 — fails after layout; Exp II 105.7 (105.4) MHz, 65.4° (65.0°); Exp III 106.6 MHz, 66.2°; areas 195.8 × 358.8 (AR 0.55), 173.8 × 191.25 (0.91), 189.6 × 193.05 µm (0.98) (13198–13208, 13229–13237).
- Exp I layout has large empty areas (13239–13242).
- Table 6.8: 2116/2437/3044 iterations, 507.8/612.31/880.3 s; text states approach A costs ~15% more CPU than B and instancing + extraction is 17% of sizing time (13243–13260) — the table ratio (880.3/612.31) is larger than 15%; the text's claim is quoted as given.
- Conclusion: approach A is fast enough for cells of a few tens of devices (13262–13265).

**6.6 Conclusions (13269–13291).** Layout-aware sizing minimises electrical/physical iterations; future: hierarchy and multi-objective optimisation for template selection (13271–13291).

**References ch.6 (13295–13397).** [1]–[38], incl. Castro-López [5,30], Stockmeyer [36], Naiknaware stack generation [38], Lampaert book [24].

### Chapter 7 — Constraint-Driven Design Methodology

**Abstract (13407–13418).** Capture rules governing analog layout as constraints, and produce layouts that satisfy them; representation, management, derivation, transformation, verification.

**7.1 Introduction (13422–13510, PDF 279–281).**
- Most analog constraints are specified manually and often implicit (13431–13435).
- Evolution: polygon pushing → schematic-driven layout → constraint-driven design → analog design automation; "analyzing" and "verifying" are a precondition for "synthesizing" (13473–13494, Fig. 7.1).

**7.2 Problem Description (13514).**

**7.2.1 The Design Problem (13516–13599, PDF 281–282).**
- Design = sequential removal of degrees of freedom by functional transformations (spec → netlist → floorplan → placement → wired layout → mask) (13518–13531).
- Analog steps overlap in time (Fig. 7.2) (13531–13580).
- A must-fulfil objective is a constraint; a may-fulfil constraint is an objective (13594–13599).

**7.2.2 Analog Vs. Digital Design Automation (13603–13663, PDF 282–283).**
- Few devices but a richer constraint set per object (13605–13613); many constraints unknown at the start (13614–13617).
- One-algorithm-per-constraint-type fails for linked constraints (13618–13625); no unified constraint description; DRC/LVS leave a verification gap (13626–13651).
- Device generation, preplacement and global routing happen together during analog floorplanning (13653–13660).

**7.3 Constraint Classification and Representation (13667–13766, PDF 283–285).**
- Simple (independent variables) vs complex (dependent) constraints; linked to design objects (cell, instance, net, terminal) (13671–13676, Fig. 7.3).
- Categories: technology, functional (electrical), design methodology (geometry), commercial (13677–13713).
- Implicit vs explicit; implicit (e.g. "diff pair must be symmetric") unusable by automation (13715–13729).
- Type with a unit (IR-drop in V, delay in s) (13730–13736).
- Uniform representation via CLP; IR-drop example: split into functional irDrop(P1,P2,V) and arithmetic V < 0.1; add structural netPin(N,P1), netPin(N,P2), starShaped(N); unification couples them (13738–13766).

**7.4 Components of a Constraint-Driven Design Flow (13770–13821, PDF 285–286).** Management, derivation, transformation, sensitivity analysis, verification around the flow (Fig. 7.4).

**7.4.1 Constraint Management (13825–13897, PDF 286–288).**
- Store constraints, synchronise with design objects, semantic integrity across abstraction, hierarchy, fast API (13827–13835).
- Over-constraint detection via verification; CSP resolution by propagation, relaxation, backtracking; weight-based conflict resolution degrades when many weights are equal (13836–13867).
- Assignment in existing hierarchy, across object extent, in a virtual hierarchy; permanent or temporary (13868–13874).
- Top-down, bottom-up, mixed (Fig. 7.5); shielding constraint from I/O pad down; skip metal resistors so all physically connected nets get it; bottom-up costs one constraint per flattened instance (13875–13897).

**7.4.2 Constraint Derivation (13900–13932, PDF 288).**
- From objectives via derivation rules, logic deduction, expert knowledge (13906–13912).
- Transformation is indirect derivation (13915–13917); deduction lets a rule for MOS apply to BJT via the class "transistor" (13918–13926); expert knowledge remains unstructured (13927–13932).

**7.4.3 Constraint Transformation (13935–13975, PDF 288–289).**
- Higher ↔ lower level with transformation rules; choice of rule reduces global freedom; inverse transformation needed for verification (13937–13948).
- Simple → independent equations; complex → coupled equations; lower constraints affecting higher ones cause iterations (13949–13961).
- Example: pad-to-terminal IR budget + known current → placement constraints + routing constraints; with fixed placement, routing constraints in length, layer, width (13963–13975).

**7.4.4 Constraint Sensitivity Analysis (13978–14075, PDF 289–291).**
- Two modules: parameter→output sensitivity; relative distance to constraints (13981–13985).
- Local derivative or statistical (sampling, Bayesian, Monte Carlo) (13988–13994).
- d_l(x) = exp(x_l − x) − 1, d_u(x) = exp(x − x_u) − 1 (eq. 7.1, verified PDF p.290); 0 = at bound, > 0 violation, < 0 satisfied (13995–14004).
- Use: prioritise sensitive or violating parameters; weakly coupled groups → independent subproblems (14005–14021).
- Wire near a heat source: V_IR = i·ρ·l/(w·d)·(1 + TK1·(T − T_ref)); widen w if w ≈ w1 (high sensitivity), w loses influence for large w (14022–14075, Fig. 7.6).

**7.4.5 Constraint Verification (14079–14186, PDF 291–293).**
- Checks fulfilment and mutual conflicts (Fig. 7.7: V_IR,1 ≤ V_IR,2 ∧ w1 ≤ w2 with fixed lengths is infeasible) (14081–14111).
- Tools: Calibre PERC, Virtuoso IC 6.1 (limited) (14113–14115); one-algorithm-per-type verification does not scale (14116–14124).
- Meta-verification: decompose complex checks into independent sub-checks run by existing tools; CLP detects conflicts (14126–14139).
- Rule ordering: short-running sub-checks first, stop at the first violation (14150–14161); rule-set effort comparable to DRC/LVS decks (14162–14166).
- Static (signoff) vs dynamic (what-if inside algorithms) CSP; dynamic suffers data-access latency (14167–14186).

**7.5 Constraint Engineering (14189–14200, PDF 294).** Constraint programming plus CES integrating management, derivation, transformation, verification.

**7.5.1 Constraint Programming (14204–14284, PDF 294–295).**
- Domain-specific solvers (boolean, real, linear, polynomial) (14208–14218).
- CLP chosen for stateless declarative constraints (14223–14232).
- CHR [12]: propagation (A ≤ B ∧ B ≤ C ⇒ A ≤ C), simplification (A ≤ 5 ∧ A ≤ 7 ⇒ drop A ≤ 7), simpagation (14273–14284).

**7.5.2 The Constraint Engineering System (14287–14400, PDF 296–298).**
- Middleware with plug-ins; Tool Integration Kits translate tool data to/from CLP and back-annotate (14294–14312).
- Built-in linear arithmetic solver (simplex); custom solvers via CHR or native plug-ins (14313–14327).
- Example rule `starShapedIRDrop(P1,P2,V,Virmax) :- starShaped(N), netPin(N,P1), netPin(N,P2), irDrop(P1,P2,V), V > Virmax.` queried with Virmax 0.1; irDrop delegated to an external IR tool (14333–14364).
- CSA call with ranges T 218–448 K, w 0.18–2.0 µm returning normalised sensitivities (14365–14377); GUI of runsets (Fig. 7.9) (14378–14395); rules reused across designs and processes (14396–14400).

**7.6 Impact Analysis (14404–14408).**

**7.6.1 Impact on Design Flow (14411–14446, PDF 299).**
- Components must be available at all stages; breaks reduce verification coverage (14419–14426).
- All tools must understand the representation or convert it (14427–14433); constraint verification complements DRC/LVS; back-annotation of results limits iterations (14434–14441); low-level and high-level consistency (14442–14446).

**7.6.2 Impact on Design Methods (14449–14510, PDF 299–301).**
- Designers must formalise constraints (effort) (14457–14461); which constraints at which step is clear only for next-step effects (14462–14470).
- Over-constraining earlier can block later optimisation; root-cause analysis then iterate or override (14471–14483).
- Remove degrees of freedom gradually; reuse structural information (templates with constraints) rather than frozen IP (14490–14510).

**7.6.3 Impact on Design Algorithms (14514–14574, PDF 301–302).**
- Special-purpose algorithms lack standard data interfaces; a standard interface layer enables modular reuse (14522–14542).
- CSA-driven dynamic task partitioning; prioritise sensitive parameters; dynamic hierarchy of functional transformations; new constraint types without low-level changes (14543–14574).

**7.7 Outlook (14578–14605, PDF 302–303).**
- Needed: completeness checks for constraint sets and verification rules, automatic rule optimisation (14582–14588).
- CSA scales to "a few thousand design variables", enough for blocks of several hundred devices (14591–14596); gradual freedom removal and dissolving step boundaries (14599–14605).

**Glossary (14614–14705).** Definitions of constraint, assignment, derivation, CES, CHR, CLP, management, CSP, CSA, transformation, meta-verification, over-constraint, propagation, simplification, simpagation, TIK, unification, etc.

**References ch.7 (14709–14785).** [1]–[32], incl. Freuer meta-verification [11], Frühwirth CHR [12], Malavasi constraint transformation/management [21,22], Nassaj [26].

**Index (14786–14994).** Book index pp. 299–302; no new technical content (confirms page locations, e.g. A* pp. 174/176/191, constraint graph p. 209, Stockmeyer p. 256).

---

## 3. Actionable extraction

### BAL2-01 Steiner growth by multi-source wavefront from the partial tree
- Kind: algorithm
- Statement: route a multi-terminal net by connecting one pair, then using the whole found path as the new wavefront source for the next terminal. The rectilinear MST is at worst 1.5× the minimum rectilinear Steiner tree (Hwang [16]), so growing from the tree (Steiner points allowed) beats terminal-to-terminal MST routing.
- Source: §4.2.4, Fig. 4.5; lines 8423–8449; book p.154 / PDF 167.
- Philis stage: gr, dr
- Automation recipe: seed Dijkstra/A* with every node of the current tree at cost 0, target nearest unconnected terminal first, repeat.
- Beats hand layout because: every net gets a near-minimal tree consistently, including nets a human would route pin-to-pin.
- Philis status: implemented — `backend/gr/src/lib.rs:874-879` ("repeated multi-source Dijkstra from the growing tree, targets nearest-first").

### BAL2-02 A* with an admissible analog heuristic
- Kind: algorithm
- Statement: f(x) = g(x) + h(x) (eq. 4.8); h must not overestimate. For length-only cost h = Manhattan distance; analog h = resistive cost of the Manhattan path to the target; g(x) = A·(G_{x−1} + ΔG_x) with ΔG_x,n = Σ_j c_ijn·ΔC_ij + r_n·ΔR (eq. 4.9). Worst case O(n²) like Lee–Moore; average much faster. Ties in f handled by a list per f value.
- Source: §4.5.3.2 "A* Algorithm", eqs. 4.8–4.9, Figs. 4.19–4.21; lines 9481–9587; book pp.176–178 / PDF 189–191 (eq. 4.9 read from PDF 190).
- Philis stage: gr, dr
- Automation recipe: h(v) = manhattan(v, nearest target) × min over layers of per-step base cost (plus min via cost × layer difference). With multi-target searches use the min over remaining targets (or a bounding-box lower bound). Keep `BinaryHeap` (duplicates are fine; no red-black tree needed). All added costs (history, overuse, penalty, Elec) are ≥ 0, so h = lower bound on base cost stays admissible.
- Beats hand layout because: faster search buys more rip-up/reroute trials and more epochs within the same run time.
- Philis status: missing — plain Dijkstra, `backend/gr/src/lib.rs:816-848` ("Stamp-based Dijkstra", "Dijkstra stays exact").

### BAL2-03 Obstacle inflation by layer min-space, terminals exempt; nonuniform refinement
- Kind: data-model
- Statement: build the routing graph with each obstacle inflated by its layer's minimum spacing, except the source and target terminals being routed (otherwise they are unreachable). A nonuniform grid built by extending component boundaries is finer where congested and keeps neighbourhood info for fast parasitic evaluation; ROAD refines locally around each new wire and limits cell size and aspect ratio.
- Source: §4.3.1.1, Fig. 4.7d; lines 8530–8602; book pp.156–158 / PDF 169–171. ROAD grid: §4.5.3.2 lines 9418–9431.
- Philis stage: gr, dr
- Automation recipe: when blocking foreign metal on the track graph, block tracks within `spacing(layer, width)` of foreign shapes; never block the current net's own pins/shapes. For off-track pins, add pin-aligned tracks (nonuniform refinement) instead of refining the global pitch.
- Beats hand layout because: DRC-clean by construction on every net, without manual pin-access tuning.
- Philis status: partial — uniform pitch track graph (`backend/dr/src/lib.rs:317-321`, `:517`), width-dependent spacing only at fattening (`backend/dr/src/lib.rs:64-65`); pin access handled separately (`backend/dr/src/lib.rs:612`).

### BAL2-04 Net → subnet → group connectivity model
- Kind: data-model
- Statement: C = {N_i}; N_i = {S_ij} (subnet = wires of the net with similar current density); S_ij = {G_ijk} (group = physically connected components). Same group ⇒ connected; same subnet, different group ⇒ connection pending. Enables fast connectivity queries during rip-up and "nearest unconnected component" search within a subnet.
- Source: §4.3.2, Fig. 4.10, footnote 3; lines 8666–8734; book pp.159–161 / PDF 172–173.
- Philis stage: dr, annotator
- Automation recipe: keep a union-find per net over routed shapes (groups); add a subnet tag per terminal (from BAL2-06) so a sense branch and a force branch of the same net are routed as separate trees joined at one point.
- Beats hand layout because: current-class separation is enforced on every net, not only the ones a designer remembers.
- Philis status: partial — union-find exists (`kernel/core/src/unionfind.rs`), per-net routed trees in `RouteHot`; no subnet level.

### BAL2-05 Layer-pair rule table (O(1) electrical/design rule lookup)
- Kind: deck-requirement
- Statement: rules keyed by (layer1, layer2) in a two-level hash return values such as capacitance between two layers ("50 fF/mm²" example) in constant time; router speed depends on rule service time.
- Source: §4.3.3, Fig. 4.11; lines 8738–8789; book pp.161–162 / PDF 174–175.
- Philis stage: deck, gr, dr, verify
- Automation recipe: add to each PDK JSON a matrix `overlap_af_um2[l1][l2]` and `fringe_af_um[l1][l2]` (inter-layer crossing capacitance), stored as a dense `Vec<f32>` indexed `l1·n + l2`.
- Beats hand layout because: crossover capacitance becomes a priced quantity instead of a guess.
- Philis status: partial — per-layer ground area/fringe and same-layer lateral terms exist (`kernel/analog/src/routing/stack.rs:13-16`, `:48-61`; `backend/verify/src/pdk.rs:708`); no layer-pair table.

### BAL2-06 Net splitting of high-current paths (force/sense separation)
- Kind: rule
- Statement: R_wire = l·R_sh/w (eq. 4.1); a low-current branch tapped from a segment carrying current I sees V_drop = R_wire·I, time-varying. Fix: route the branch separately back to the source point (Fig. 4.12b) or widen the shared high-current segment (Fig. 4.12c). Current density in one net can differ by an order of magnitude. ROAD splits nets by a heuristic estimating low-current parts.
- Source: §4.4.1, eq. 4.1, Fig. 4.12; lines 8806–8823; book pp.162–163 / PDF 175. ROAD: lines 9449–9450.
- Philis stage: annotator, dr
- Automation recipe: inputs = op-point terminal currents (already in `Bias.currents`). For each supply/ground/bias net, classify terminals as force (|I| > k·median, e.g. output-stage sources) or sense (bias-mirror sources, reference gates, Kelvin taps). Emit a subnet split: sense terminals form a tree joined to the force tree only at the net's root terminal (pad/strap). Budget: V_drop on shared segments ≤ fraction of the sense device's headroom (reuse `net_headroom_mv`). Verify with terminal-resolved R (`Stack::terminal_resistance_ohm`).
- Beats hand layout because: every high/low current junction is found from the op-point and checked numerically, not by eye.
- Philis status: missing — `IrDrop` charges the whole current to the worst path (`kernel/analog/src/routing/ir.rs:8-17`); fattening widens trunks (`backend/dr/src/lib.rs:637-678`), no split.

### BAL2-07 Symmetric routing with mirrored obstacles (joint search)
- Kind: algorithm
- Statement: symmetric routing applies when terminals are symmetric about an axis. ANAGRAM II grows both halves together (an obstacle on either side blocks both); ROAD mirrors all nonsymmetric obstacles onto the other half, routes one half, mirrors the path.
- Source: §4.4.2, Fig. 4.13; lines 8827–8900; book pp.162–164 / PDF 175–176.
- Philis stage: gr, dr
- Automation recipe: for a `Differential` pair with mirror-symmetric pins: build the blocked-node set B ∪ mirror(B), route net A on it, emit mirror(route A) for net B. Only fall back to independent routing if the mirrored route is illegal.
- Beats hand layout because: exact geometric symmetry on every matched pair, including via counts and layer changes.
- Philis status: partial — initial routing ignores the field (`backend/gr/src/lib.rs:738-742`, `plain`), repair copies or mirror-guides trees after the fact (`backend/dr/src/lib.rs:1392-1398`, `:1440-1452`).

### BAL2-08 Crossover "connector" for symmetric nets that must cross the axis
- Kind: heuristic
- Statement: when paired terminals lie on opposite sides and the two symmetric paths must cross, route both through a symmetric crossing structure (connector, Fig. 4.14b) so R and C of both nets match; coupling to other nets may still differ.
- Source: §4.4.2, Fig. 4.14; lines 8901–8909; book p.164 / PDF 177.
- Philis stage: dr
- Automation recipe: detect `Differential` pairs whose pins are mirror-swapped across the axis (cross-coupled latches, chopper swaps). Insert a crossing cell at the axis: each net changes layer once, symmetric about the axis centre (net A: M_k→M_{k+1} left of axis, net B: same right of axis), then mirror-route both halves to the crossing.
- Beats hand layout because: automatic RC-balanced crossings for every swapped pair.
- Philis status: missing — no crossing handling in `kernel/analog/src/routing/differential.rs` or `backend/dr/src/lib.rs` (grep "cross" finds none).

### BAL2-09 1-D/2-D capacitance model incl. inter-layer crossings
- Kind: formula
- Statement: C_1D = A·C_α + S·C_β (eq. 4.2): A = overlap area, S = overlap perimeter, C_α per area, C_β fringe per length. C_2D = C_1D + l·C/d (eq. 4.3): l = parallel run, d = gap, C = coupling per length. 2-D is the router standard; 2.5-D/3-D too slow inside search.
- Source: §4.4.3, eqs. 4.2–4.3, Fig. 4.15; lines 8912–8966; book pp.164–165 / PDF 177–178.
- Philis stage: gr, dr, verify, deck
- Automation recipe: extend `CouplingBudget` and the router's `Elec` cost with crossing terms: for shapes on different layers overlapping in plan view, C = A·overlap_af_um2[l1][l2] + S·fringe_af_um[l1][l2] (table from BAL2-05). In the router, price a node crossing an occupied node on an adjacent layer with `cross_c[l]·(weight + sens)`.
- Beats hand layout because: every crossover of a sensitive node is counted and budgeted.
- Philis status: partial — lateral l·C/d same layer (`kernel/analog/src/routing/coupling.rs:20-30`, `:61-62`; test "different layers do not couple" `:168-172`); ground area+fringe (`kernel/analog/src/routing/stack.rs:48-61`).

### BAL2-10 Shield policy: space first, shield last, tie to DC; three shield geometries
- Kind: rule
- Statement: reduce coupling by spacing first; if impossible add a shield between critically coupled nets; shields must connect to a DC potential or ground. Types: same layer, different layer (plate between), bulk (substrate contacts). ROAD builds shields after all wires, as a last resort.
- Source: §4.4.3, Fig. 4.16; lines 8971–8979; §4.5.3.2 lines 9453–9456; book pp.165, 175 / PDF 178, 188.
- Philis stage: annotator, dr
- Automation recipe: emit `Shield` only when `CrosstalkExclusion`/`CouplingBudget` still fail after the keep-away rip-up round; choose reference = quietest low-impedance net (Ground over Supply; never a Signal/Clock). Add a plate variant (reference metal on layer ±1 under/over the victim run) and a substrate-tap row variant for bulk coupling.
- Beats hand layout because: shields are placed only where the numbers say spacing cannot meet the budget, saving area and C elsewhere.
- Philis status: partial — `Shield` same-layer two-sided (`kernel/analog/src/routing/shield.rs:8-21`, ponytail "same-layer only"); shields routed last (`backend/gr/src/lib.rs:244-245`); insertion `backend/dr/src/lib.rs:585-594`.

### BAL2-11 Net classes including "noisy" (high swing) derived electrically
- Kind: heuristic
- Statement: ILAC: sensitive, noisy, noncritical signal, power. ANAGRAM: neutral (supply, bias), noisy (high-swing: clocks, outputs), sensitive. Classes drive ordering, spacing and shielding.
- Source: §4.5.1 lines 9068–9074; §4.5.2 lines 9196–9200; book pp.167, 170 / PDF 180, 183.
- Philis stage: annotator
- Automation recipe: keep name rules, add electrical classification: noisy = net with large-signal swing (transient or AC gain from input > 1 at the op-point: stage outputs, drains of switches, clock-driven nodes); sensitive = high-impedance nodes (gate-only loads with no DC path except through a high-R/leakage: comparator inputs, integrator virtual grounds, bias gates). Use the existing op-point and pattern catalog (switch patterns, output stages).
- Beats hand layout because: every node is classified, including those with misleading names.
- Philis status: partial — `NetClass` has Signal/Clock/Supply/Ground/Sensitive/Substrate (`kernel/analog/src/metadata.rs:18-25`), assigned by name (`backend/annotator/src/netrole.rs:8-28`); no "noisy" class for high-swing outputs.

### BAL2-12 Net routing order
- Kind: heuristic
- Statement: route critical nets first so they take privileged tracks. ILAC: power, sensitive, noncritical, noisy last (power/noncritical then separate critical from noisy). ROAD: more constraints ⇒ higher priority; symmetric nets very high; shields after all wires. [30]: scheduler from symmetry, user and congestion data.
- Source: §4.5.1 lines 9054–9074; §4.5.2 lines 9240–9244; §4.5.3.2 lines 9438–9456; book pp.167, 171, 175 / PDF 180, 184, 188.
- Philis stage: gr, dr
- Automation recipe: tier key (symmetric, hard, budget, free), then descending constraint count per net, then sensitivity weight, then pins. Put noisy nets (BAL2-11) last within the free tier.
- Beats hand layout because: the order is computed from the full constraint set per net.
- Philis status: implemented (mostly) — `backend/gr/src/lib.rs:240-270` (symmetric → hard → budget → free, weight, pin count, shields last). Constraint count per net not used; noisy-last not available.

### BAL2-13 Prioritised node distance and sensitive–noisy spacing (SLAM)
- Kind: rule
- Statement: critical-net analysis gives each node a distance (length) constraint with priority (highest ⇒ shortest connection) and each node pair a prioritised spacing; sensitive–noisy pairs get the highest priority and large spacing.
- Source: §4.5.1 lines 9104–9112; book p.168 / PDF 181.
- Philis stage: annotator, gr, dr
- Automation recipe: for each (sensitive, noisy) pair from BAL2-11 emit `CrosstalkExclusion` with spacing = k × min spacing where k grows with (noisy swing × victim sensitivity); for each sensitive net emit a length/C budget. Priority = product used in routing order.
- Beats hand layout because: pairwise separation is computed for all O(n²) pairs.
- Philis status: partial — `CrosstalkExclusion` self-extracted with spacing raised to the victim class (`backend/annotator/src/extract.rs:8`, `kernel/analog/src/routing/crosstalk.rs:10-19`); pairs not keyed on a noisy class.

### BAL2-14 Additive routing cost with cost-to-target (ANAGRAM eq. 4.4)
- Kind: formula
- Statement: Cost = Cost(P) + MaterialCost(C) + ParasiticCost(C) + RoutabilityCost(C) + CostToTarget(C, T): accumulated cost, incremental length + layer/via, incremental parasitic to each nearby net, crowding, lower-bound remainder. Partial paths in a heap; effective for distributed terminals (capacitor perimeters).
- Source: §4.5.2, eq. 4.4; lines 9163–9194; book pp.169–170 / PDF 182–183.
- Philis stage: gr, dr
- Automation recipe: already the node cost; add CostToTarget via BAL2-02; for distributed terminals (cap-array top plates, multi-finger drains) keep all pin shapes as targets.
- Beats hand layout because: all four terms are traded per step, not per net.
- Philis status: partial — material + parasitic + congestion present (`backend/gr/src/lib.rs:842-870`, `:874-879`); no cost-to-target.

### BAL2-15 Routability as rip-up cost (detour vs remove)
- Kind: algorithm
- Statement: ANAGRAM II's RoutabilityCost = cost of ripping up neighbouring nets to advance; explicit trade between detouring and removing a cheap obstacle and rescheduling it.
- Source: §4.5.2 lines 9223–9229; book p.171 / PDF 184.
- Philis stage: gr, dr
- Automation recipe: negotiated congestion (present overuse × p_fac + history) is the equivalent; keep.
- Beats hand layout because: global convergence to a legal routing without manual ordering.
- Philis status: implemented (different mechanism) — PathFinder `backend/gr/src/lib.rs:1015-1020`; history persists across epochs (`frontend/library/src/lib.rs:336-342`).

### BAL2-16 Sensitivity-derived parasitic bounds, conservative sign split
- Kind: formula
- Statement: S_ij = ∂W_i/∂p_j at p = 0 (eq. 4.5). Σ_j S⁺_ij p_j ≤ ΔW⁺_i,max (W_i ∈ W⁺) and Σ_j S⁻_ij p_j ≤ ΔW⁻_i,max (W_i ∈ W⁻), with S⁺ = max(S, 0), S⁻ = max(−S, 0) (eqs. 4.6–4.7). Parasitics p_j: line R, C to ground, line-to-line C, L, mutual L; plus matching constraints on matched-node pairs using worst-case sensitivities with mismatch. Perturbation sensitivity is slow and error-prone (difference of close numbers, transient noise); direct/adjoint methods preferred.
- Source: §4.5.3.1, eqs. 4.5–4.7; lines 9257–9360; book pp.171–173 / PDF 184–186 (eqs. from PDF 186); matching: lines 9387–9390.
- Philis stage: flow, annotator, dr, verify
- Automation recipe: (1) keep one row per spec side, but sum only the adverse-direction terms (S⁺ for a ceiling-limited metric, S⁻ for a floor) so that a net whose C "helps" earns no credit (the linear model cannot be trusted to cancel). (2) Extend the parasitic vector beyond ground C: series R of current-carrying nets, coupling C between net pairs (i,j) listed by `CouplingBudget`, and matched-pair ΔC (C_pos − C_neg) for differential specs (offset, CMRR). (3) Replace per-net perturbation runs with an AC adjoint where the simulator allows; otherwise use a central difference with a step chosen from solver tolerance. Output: `PerformanceBudget` rows with per-parasitic weights; hard.
- Beats hand layout because: every parasitic is tied to a quantitative spec margin; a human only knows "keep it short".
- Philis status: partial — forward-difference ground-C sensitivity per net, 10 fF step (`frontend/library/src/perf.rs:189-222`; `frontend/library/src/lib.rs:215-216`); rows keep signs and grant credit (`kernel/analog/src/routing/performance.rs:8-17`), which is less conservative than eqs. 4.6–4.7; no R, coupling or matching parasitics.

### BAL2-17 Critical-parasitic screening and flexibility-based bound allocation
- Kind: heuristic
- Statement: drop parasitics whose max estimate × sensitivity is collectively negligible; the rest are critical. Bounds are non-unique; PARCAR assigns each parasitic a flexibility from its estimated min/max range, refined as the layout evolves, and allocates bounds with a QP so less flexible parasitics get tighter effort.
- Source: §4.5.3.1 lines 9305–9311, 9361–9379; book pp.172–173 / PDF 185–186.
- Philis stage: flow, annotator
- Automation recipe: estimate p_j^min (Steiner length of the placed pins × C per µm) and p_j^max (HPWL × detour factor 2) after placement; critical iff S_ij·p_j^max > ε·ΔW_i,max (ε = 0.02). Use flexibility (p_max − p_min)/p_max to set router weight = S / flexibility.
- Beats hand layout because: effort is concentrated on the few parasitics that actually move a spec.
- Philis status: partial — only nets classed Signal/Sensitive/Clock are sensed (`frontend/library/src/lib.rs:209-213`); no screening threshold or flexibility.

### BAL2-18 Router weight ∝ sensitivity / allowed variation
- Kind: formula
- Statement: path cost = its effect on performance, weights from sensitivity; contribution of a parasitic is proportional to its sensitivity and inversely proportional to the maximum variation allowed for that performance (ROAD); cost terms local crowding, R, C to bulk, cross C.
- Source: §4.5.3.2 lines 9390–9397, 9431–9438; book pp.174–175 / PDF 187–188.
- Philis stage: gr, dr
- Automation recipe: weight_n = Σ_i |S_in| / ΔW_i,max, normalised; already fed to `Elec.weight`.
- Beats hand layout because: every step of every net is priced in spec units.
- Philis status: implemented — `backend/gr/src/lib.rs:842-870`; weights from `perf_rows` (`frontend/library/src/lib.rs:295-316`).

### BAL2-19 Weight escalation loop and route-to-placement feedback
- Kind: algorithm
- Statement: after weighted routing, extract and estimate degradation; if a spec is violated, raise the weights of the most sensitive parasitics and reroute; when all are at maximum, the placement must be regenerated with a wider variation range for those parasitics. Naive ± weight updates oscillate; use a damped update.
- Source: §4.5.3.2 lines 9397–9406, 9441–9449; book pp.174–175 / PDF 187–188.
- Philis stage: flow, gp, dp, dr
- Automation recipe: per epoch, for every budget row with residual > 0 after dr's repair rounds, multiply the placement HPWL weight of its top-k contributing nets by (1 + residual), capped, with exponential smoothing (β = 0.5) to avoid oscillation; carry into the next epoch's `gp::place`/`dp::place` `net_weight`. Stop raising when capped; report the spec as placement-limited.
- Beats hand layout because: the placement learns from measured routing parasitics instead of a designer's re-floorplan.
- Philis status: partial — dr doubles `p_fac` per violated rule (`backend/dr/src/lib.rs:32`, `:1392-1405`) and reroutes IR-violating nets (`:527-534`); placement net weights are computed once per run (`frontend/library/src/lib.rs:296`) and placement prices settle on placement rules only (`backend/gp/src/lib.rs:196`, `:325`).

### BAL2-20 Prune partial paths that already violate a hard parasitic bound
- Kind: algorithm
- Statement: ANAGRAM III prunes parasitically non-viable partial paths during search, finding the cheapest path that meets hard parasitic bounds without trading wirelength against crosstalk. The yield router [97] similarly drops a partial path from the heap when its exact added degradation exceeds the remaining margin. [73,74] prune backtracking branches exceeding performance bounds using predicted C and sensitivities; switchboxes ≤ 3×3 are enumerated exhaustively.
- Source: §4.5.3.2 lines 9469–9478; §4.7 lines 10316–10320; §4.5.5 lines 9763–9771, 9793–9796; book pp.175–176, 182–183, 194–195 / PDF 188–189, 195–196, 207–208.
- Philis stage: gr, dr
- Automation recipe: carry per-label accumulated (C_ground, C_coupling to each budgeted victim, R) with the search state; discard a label when any exceeds the net's remaining budget (budget − already-routed share). This is resource-constrained shortest path; with a single resource (C) a label per node suffices.
- Beats hand layout because: hard bounds are guaranteed by construction instead of repaired afterwards.
- Philis status: missing — budgets enter only as costs and post-route repair (`backend/dr/src/lib.rs:1392-1405`).

### BAL2-21 Placement scored by an in-loop global route estimate
- Kind: algorithm
- Statement: analog has few nets, so global routing can run inside placement. Options range from routing between placement iterations to wires as placement objects (KOAN anneals devices and wires; cost DRC violations, crosstalk, net area). ALADIN/[66]: HPWL-based global search, then SA where each state gets a min-Steiner-tree global route over a channel graph weighted by length + congestion (nets per channel), solved with Dijkstra; the route cost scores the placement. Resistor-array probabilistic routing [83,84] estimates channel usage including symmetry.
- Source: §4.5.4 lines 9591–9682; §4.5.5 lines 9688–9729; §4.5.7 lines 10089–10103; book pp.178–181, 190 / PDF 191–194, 203.
- Philis stage: dp, gp
- Automation recipe: in dp's anneal, for budgeted/sensitive nets replace HPWL by a rectilinear Steiner estimate (RSMT via 1-Steiner or FLUTE-like table) times per-layer C, plus a RUDY-style congestion map term; evaluate incrementally for moved cells only.
- Beats hand layout because: placement is judged by the parasitics the routes will actually have.
- Philis status: partial — epoch loop routes between placements (`frontend/library/src/lib.rs:336-380`, `:572-625`); dp uses HPWL (`backend/dp/src/lib.rs:242-243`); no congestion term (grep finds none in gp/dp).

### BAL2-22 Symmetric coupling across aggressors (bundles)
- Kind: check
- Statement: symmetric nets can still get unequal parasitics from coupling to other nets (4.4.2) and from corners in bundles; each aggressor affects each bundle member differently. PARSY chooses routing area and adds extra paths and adjustment sections to balance.
- Source: §4.4.2 lines 8905–8909; §4.5.5 lines 9745–9753; book pp.164, 182 / PDF 177, 195.
- Philis stage: verify, dr
- Automation recipe: for each `Differential` (pos, neg), per aggressor net a: ΔC_a = |C(pos,a) − C(neg,a)| (lateral + crossing from BAL2-09); check Σ_a ΔC_a·S_a ≤ budget (S_a = aggressor swing weight, 1 for noisy class). Repair: price aggressor tracks adjacent to only one side; or reroute the aggressor along the axis.
- Beats hand layout because: every aggressor is balanced numerically; hand layout checks only the pair's own length.
- Philis status: missing — `Differential` compares own RC only, "no coupling term" (`kernel/analog/src/routing/differential.rs:9-27`).

### BAL2-23 Constructive detailed-routing order: symmetric and central modules first
- Kind: heuristic
- Statement: ALADIN orders detailed wiring starting from symmetric modules and more central modules; intra-module wiring is packed around the module boundary (ring router) before inter-module routing in a small scope.
- Source: §4.5.5 lines 9731–9744; book p.181 / PDF 194.
- Philis stage: dr
- Automation recipe: within a tier, break ties by distance of the net's bbox centre to the block centroid (central first).
- Beats hand layout because: minor; consistent ordering.
- Philis status: missing (low value; tier order exists `backend/gr/src/lib.rs:240-270`).

### BAL2-24 Relative-order template extraction (extended constraint graph, LDS)
- Kind: data-model
- Statement: extract horizontal/vertical constraint graphs from a placement: the simple graph (right edge sees left edge) can overlap after resizing; the extended graph adds an edge from an element's right boundary to every element with larger x and nothing in between, guaranteeing no overlap for any resizing. Spacing constraints only between same-layer shapes of different nets. Paths coded with 4 wires stay flexible (orientation may flip). Solved by LP, then compaction removes slack.
- Source: §4.5.6.1, Figs. 4.22–4.25; lines 9896–10005; book pp.185–189 / PDF 198–200.
- Philis stage: dp, flow
- Automation recipe: after an epoch's best layout, extract the extended H/V visibility graph over cells; when a later epoch swaps variants (sizes change) or the design is re-sized, re-legalise by longest path on this graph (BAL2-30) instead of re-annealing, keeping the winning topology.
- Beats hand layout because: a proven topology is kept exactly while every size changes, in milliseconds.
- Philis status: missing — no stored relative-order graph; dp re-anneals (`backend/dp/src/lib.rs:1-2`).

### BAL2-25 Warm-start routes from a template/previous solution (LAYGEN)
- Kind: heuristic
- Statement: adapt template routing to a new placement: split nets into 2-pin wires, scale each path, move its start to the new start pin, set its end to the new stop pin; then optimise (GA) with cost of DRC violations, connectivity, wirelength and inter-net distance.
- Source: §4.5.6 lines 9870–9893; book pp.184–185 / PDF 197–198.
- Philis stage: gr, dr
- Automation recipe: seed each epoch's route trees from the previous best epoch translated by cell displacement; rip-up only nets whose pins moved.
- Beats hand layout because: incremental change keeps good routing and cuts run time per epoch.
- Philis status: partial — negotiation history persists across epochs (`frontend/library/src/lib.rs:338-342`), trees do not.

### BAL2-26 Candidate routes + compatibility graph selection (x–y routing)
- Kind: algorithm
- Statement: generate candidate routes per two-terminal edge (MST of the net, t−1 edges) in phases by via count, mostly inside the net bbox; compatibility graph (edge = conflict); reduce to an independent set, one candidate per net. Supports widths, layer confinement, keep-outs, parallel distance and coupled length vs width; all nets considered together; extends to 45°.
- Source: §4.5.7 lines 10104–10140; book pp.190–191 / PDF 203–204.
- Philis stage: dr
- Automation recipe: for the small set of symmetric/critical nets only: enumerate L/Z/U-shapes per 2-pin edge on two layers, build conflict graph (shorts, spacing, coupled-length limits), solve maximum-weight independent set exactly by branch-and-bound (few dozen candidates).
- Beats hand layout because: jointly optimal critical nets instead of order-dependent ones.
- Philis status: missing.

### BAL2-27 Exact-length RF lines: constructive route then simultaneous refinement
- Kind: algorithm
- Statement: CORAL models coupling as Z0 and loss degradation with models fitted to 2-D/3-D solver data; phase 1 constraint-based A* routing; phase 2 refinement expands all nets simultaneously to enforce fixed interconnect lengths without creating new parasitic violations.
- Source: §4.6.1 lines 10160–10174; book p.191 / PDF 204.
- Philis stage: dr, annotator
- Automation recipe: for nets marked RF with a target length (e.g. from inductor/transmission-line specs) add a meander-insertion pass after routing, checking coupling budgets after each insertion.
- Beats hand layout because: exact lengths with automatic coupling checks.
- Philis status: missing (inductor generator exists, `kernel/cells/src/inductor.rs`; no length-target rule).

### BAL2-28 Array-style placement and routing by abutment and column wires (Mondriaan)
- Kind: algorithm
- Statement: regular arrays (flash ADC, current-steering DAC, CNN): supply/bias by abutment, cells flipped to share lines; decide vertical wires per column, assign free vertical wires to IO pins and place connected cells in free slots, scan columns to propagate connectivity, then horizontal wire count = vertical wires needing connection; bus and tree generators. Metric is equal R/C/matching, not delay.
- Source: §4.6.2 lines 10187–10214; book p.192 / PDF 205.
- Philis stage: cells, gp, dr
- Automation recipe: when the annotator finds an array of ≥ 8 identical unit cells (DAC current sources, comparator banks), generate the array as one macro with abutted rails and channel wiring, then treat it as one cell in gp.
- Beats hand layout because: tree/bus generators give equal-length wiring to every unit.
- Philis status: partial — constructive cap arrays with routing metrics (`kernel/cells/src/cap_array.rs:62`, `:789`); no generic unit-cell array generator.

### BAL2-29 Yield-aware rerouting within leftover performance margin
- Kind: algorithm
- Statement: after a successful performance-driven route with margin left, rip up and reroute nets to minimise expected bridging faults until the margin is consumed or no improvement; faults = dielectric pinholes (critical area = inter-level overlap) and photolithographic defects (same-level parallel conductors: critical area vs defect size combined with the defect-size distribution); partial paths exceeding the remaining margin are dropped.
- Source: §4.7 lines 10301–10335; book pp.194–195 / PDF 207–208.
- Philis stage: dr, verify
- Automation recipe: post-route pass: for each same-layer parallel pair compute critical area A_c = run·f(gap) with f from a 1/x³ defect density (x0 = min spacing) — the distribution itself is not given in this source; spread wires (increase gap) and reduce crossings on nets with remaining budget headroom (`worst_usage` < 1), accept if no rule gets worse.
- Beats hand layout because: spare margin is systematically converted into yield.
- Philis status: missing (no critical-area code; grep "critical area" finds none).

### BAL2-30 Constraint-graph compaction by longest path
- Kind: algorithm
- Statement: 1-D compaction with constraints x_j − x_i ≥ d_ij (eq. 5.1); vertices = edge coordinates, source v0 at 0, sink vn; min-distance-only graph is a DAG; longest path from v0 gives the minimal coordinate of every element; O(N + E), practically O(N) (eq. 5.39).
- Source: §5.4.1, eq. 5.1, Fig. 5.2; lines 10895–10946; §5.5.3 lines 11650–11656; book pp.209–211, 228 / PDF 219–221, 238.
- Philis stage: dp
- Automation recipe: after dp, build the extended visibility graph (BAL2-24) per axis with d_ij = hw_i + hw_j + clearance(i, j) (abutment groups get 0/shared-diffusion distances); pinned cells as fixed-coordinate constraints (two edges to the source). Longest path (topological order) gives a legal, compact layout; alternate x/y passes regenerating the graph each pass (BAL2-34).
- Beats hand layout because: deterministic, minimal-area, DRC-legal spacing in O(N + E) after every placement change.
- Philis status: missing — O(n²) pairwise relaxation with rollbacks (`backend/dp/src/legalize.rs:9-16`, ponytail "a constraint graph + LP compaction if blocks reach thousands of devices").

### BAL2-31 Long-wire problem and closeness objective
- Kind: heuristic
- Statement: pure longest path pushes every element left, stretching wires (Fig. 5.3), increasing parasitics and hurting the other-axis pass; fix with an objective minimising wire lengths [15,16]. Objectives privileging closeness to the source layout keep parasitics and WPE-related spacing [2].
- Source: §5.4.1 lines 10947–10956; §5.6 lines 11897–11899; §5.7.5 lines 12033–12038; book pp.211, 234, 237 / PDF 221, 244, 247.
- Philis stage: dp
- Automation recipe: after the longest-path pass, run the reverse (longest path from the sink) to get max coordinates; place each non-critical element at the point in [min, max] closest to its pre-compaction position (or its net-weighted median), which is an LP with difference constraints (solvable as min-cost flow dual or by BAL2-32).
- Beats hand layout because: compaction does not silently lengthen sensitive wires.
- Philis status: missing.

### BAL2-32 LP compaction with symmetry and equality constraints (big-M start)
- Kind: algorithm
- Statement: min f(x) s.t. f_i(x) ≤ d_fi, x ≥ 0 (eqs. 5.3–5.4). Multivariable constraints: symmetry x_b − x_a = x_d − x_c (eq. 5.2); exact dimensions x_j − x_i = K (eq. 5.37); hierarchy x_cell^top ± … ± x_edge (eq. 5.49). Start: longest-path coordinates; each unsatisfied or slack-less constraint gets an artificial variable a_k with penalty M·a_k (M huge) so it reaches 0 (example x4 − x3 = x2 − x1 with 10, 30, 10, 50 ⇒ a1 = 20, eqs. 5.35–5.36). Every constraint-graph loop contains at least one nondistance constraint (Theorem 5.1).
- Source: §5.4.2 lines 10978–11169; §5.5.2.3 lines 11573–11632; §5.7.1 lines 11912–11939; §5.7.3 lines 11975–12005; book pp.212–216, 226–227, 234, 236 / PDF 222–226, 236–237, 244, 246.
- Philis stage: dp
- Automation recipe: variables = cell x (resp. y) centres + one axis variable per symmetry group; constraints: separations (BAL2-30 graph), mirror pairs x_a + x_b − 2·axis = 0 and self-symmetric x_c − axis = 0, same-row y_a − y_b = 0, fixed cells; objective = Σ net-weighted (x_max − x_min) per net (auxiliary variables) + ε·bbox. Philis blocks are tens to a few hundred cells; a dense bounded-variable simplex is adequate.
- Beats hand layout because: exact symmetry and minimal area simultaneously, every time.
- Philis status: missing — symmetry restored by projection inside legalisation sweeps (`backend/dp/src/legalize.rs:9-13`).

### BAL2-33 Compaction problem sizes and solver choice
- Kind: metric
- Statement: Table 5.1: Miller opamp 521 variables, 1,831 (X) / 2,067 (Y) constraints: graph simplex 0.375 / 0.234 s vs revised simplex 3.594 / 4.265 s; folded cascode 2,431 / 2,443 variables, 8,599 / 11,495 constraints: 8.232 / 7.391 s vs 91.345 / 118.141 s (3.0 GHz, 512 MB). Table 5.2: ~500-device oscillator 14,495 edges, 71,197 min constraints, 2,857 symmetry/equal constraints, 6,750 iterations (X). Complexity: distance-only O(k + k_f + u) (eq. 5.43); u ≫ v adds O(v·p·u) loop elimination (eq. 5.45); worst O(v³) (eq. 5.47).
- Source: §5.5.3, §5.8, Tables 5.1–5.2; lines 11635–11773, 12042–12142; book pp.228–240 / PDF 238–250.
- Philis stage: dp
- Automation recipe: Philis compacts cells, not polygon edges: expect < 500 variables and < 5,000 constraints per axis, far below the book's cases, so a plain simplex is fast enough; no graph-simplex needed.
- Beats hand layout because: n/a (sizing guidance).
- Philis status: n/a.

### BAL2-34 Edge-order locking: regenerate constraints between passes
- Kind: heuristic
- Statement: min-distance constraints freeze edge order; after resizing, obsolete constraints (e.g. between A and D in Fig. 5.13) block area recovery; fix by iterating H/V compaction, regenerating constraints each pass, until no area gain; 2-D compaction is NP-complete. Example VCO 130 → 65 nm needed manual constraint deletion (Fig. 5.16).
- Source: §5.6 "Edge Order" lines 11840–11879; §5.8 lines 12088–12102; book pp.232–233, 238 / PDF 242–243, 248.
- Philis stage: dp
- Automation recipe: in BAL2-30, rebuild the visibility graph after each axis pass from current coordinates; stop when bbox area decreases < 0.5%.
- Beats hand layout because: removes dead space without human inspection.
- Philis status: missing (depends on BAL2-30).

### BAL2-35 Contact/via array merge during compaction
- Kind: heuristic
- Statement: merge each contact/via array into one large cut before compaction, size it to keep at least the source cut count, regenerate minimum-size cuts afterward; greatly reduces constraint count.
- Source: §5.6 "Constraint Number Reduction" lines 11788–11816, Fig. 5.12; book pp.231–232 / PDF 241–242.
- Philis stage: dr, cells
- Automation recipe: keep vias abstract (one "via region" per junction) until final geometry; expand into arrays at emit time with count ≥ EM requirement.
- Beats hand layout because: n/a (efficiency).
- Philis status: implemented in spirit — via arrays created at the end of fattening (`backend/dr/src/lib.rs:717`, `:2556`).

### BAL2-36 Conditional/context rules via the signoff DRC engine
- Kind: deck-requirement
- Statement: rules are becoming polygon-based and conditional (wide-metal spacing) or conjunctive (depend on another layer); the foundry DRC engine can generate edge constraints from them, keeping constraint generation consistent with signoff.
- Source: §5.3.3 lines 10861–10870; §5.6 "Complex Design-Rules" lines 11819–11837; book pp.209, 232 / PDF 219, 242.
- Philis stage: deck, dp, dr, verify
- Automation recipe: derive every spacing the router and compactor use from the same deck object the signoff checker (GPurify) uses; add width-and-run-length dependent spacing to the router's blockage (BAL2-03) not only at fattening.
- Beats hand layout because: no rule drift between the generator and signoff.
- Philis status: partial — width-dependent spacing in dr config (`backend/dr/src/lib.rs:64-65`); dp consults the live DRC oracle (`frontend/library/src/lib.rs:338-339`).

### BAL2-37 Preserve intentional non-minimum spacing (WPE/STI) under compaction
- Kind: rule
- Statement: STI stress and well proximity change transistor performance; compaction minimises wells and destroys deliberate well extensions; detect wells not at minimum size and treat them specially; closeness objective helps.
- Source: §5.6 "Nanometer Process Effects" lines 11881–11899; book pp.233–234 / PDF 243–244.
- Philis stage: cells, dp
- Automation recipe: when compaction (BAL2-30) is added, emit well/diffusion halo distances from the cell generator as lower bounds (never as default min spacing), and keep matched-group halos equal on all sides (equality constraints).
- Beats hand layout because: halos are kept consistently on all matched devices after any resize.
- Philis status: partial — matched groups get a WPE halo in the generator (`kernel/cells/src/mosfet.rs:626-630`); no compactor yet to protect it.

### BAL2-38 Aspect-ratio control by device realisation swap at fixed relative placement
- Kind: heuristic
- Statement: after size changes, keeping finger counts wastes corners (Fig. 5.14b); changing device realisations (finger count) under the same relative placement recovers area (Fig. 5.14d) without 2-D compaction [31].
- Source: §5.7.2 lines 11942–11971; book pp.235–236 / PDF 245–246.
- Philis stage: dp, cells
- Automation recipe: variant-reshape move in the anneal; combine with BAL2-39 Pareto pruning.
- Beats hand layout because: all variant combinations are explored under the same topology.
- Philis status: implemented — dp variant-reshape moves (`backend/dp/src/lib.rs:2`, `:580-605`); variants from `cellgen` (`frontend/library/src/cellgen.rs:152-186`).

### BAL2-39 Pareto shape functions and Stockmeyer floorplan sizing
- Kind: algorithm
- Statement: each leaf block has a shape function = Pareto-optimal (w, h) list from its variants (finger counts, styles such as common-centroid or interdigitated, 90° rotations). Bottom-up over the slicing tree, combine children (horizontal: w add, h max; vertical: h add, w max) keeping only Pareto pairs sorted h_i > h_{i+1}, w_i < w_{i+1}; list length ≤ Π l_i; complexity O(d·l_T). Top-down: drop points outside AR ∈ [AR − ΔAR, AR + ΔAR] or max/min W, H; pick min area; propagate to leaves to get each block's parameters. Results: AR [0.91, 1.1] → 1.00; [1.95, 2.05] → 1.96; W < 150 µm → 143.2; area loss 0.57–3.06% (Tables 6.4–6.5).
- Source: §6.4.1–6.4.3, Tables 6.2, 6.4–6.5; lines 12769–13067; book pp.254–261 / PDF 264–271.
- Philis stage: cells, flow, dp
- Automation recipe: (1) per cell, drop dominated variants (both w and h ≥ another variant's, same pin quality) before the variant odometer — shrinks the search space multiplicatively. (2) Add optional die aspect-ratio / max W / max H goals to `Config`; enforce as a placement-tier hard rule on the footprint bbox. (3) For hierarchical blocks, compute the block's composite shape function from its cells for use by the parent.
- Beats hand layout because: every shape combination is evaluated exactly; humans try two or three.
- Philis status: missing — variants tried by a mixed-radix odometer without dominance pruning (`frontend/library/src/cellgen.rs:395-415`); no aspect-ratio goal (grep finds none in `frontend/library/src/lib.rs`).

### BAL2-40 Area-loss metric
- Kind: metric
- Statement: area loss = W·H − Σ_i W_i·H_i − routing area (unused area not demanded by rules), reported as a percent of W·H; used as an optional objective to obtain compact realisations (0.57–3.06% in Table 6.5).
- Source: §6.4.1 lines 12783–12792; §6.4.2 lines 12936–12940, 12999–13004; Table 6.5 lines 13063–13067; book pp.254, 257, 259, 261 / PDF 264, 267, 269, 271.
- Philis stage: verify, flow
- Automation recipe: report area loss per winning layout: bbox − Σ cell areas − routed metal area outside cells; track in benchmarks.
- Beats hand layout because: quantified compactness per run.
- Philis status: partial — `Utilization` floor rule (`kernel/analog/src/placement/utilization.rs:6-12`) uses cell area vs footprint, without routing area.

### BAL2-41 Log-ratio spec cost for infeasible and feasible points
- Kind: formula
- Statement: infeasible: ψ(x) = max_j [−w_j·log(y_rj/Y_rj)] (eq. 6.3); feasible: ψ(x) = Φ(y_oi) = −Σ_i −w_i·log(|y_oi|) (eq. 6.4). Log ratios make specs of different units comparable; the max focuses on the worst violated spec.
- Source: §6.3.1, eqs. 6.1–6.4; lines 12607–12637; book p.250 / PDF 260 (read from PDF).
- Philis stage: flow, verify
- Automation recipe: keep lexicographic key; inside the perf residual, use Σ or max of |log(v/bound)| for violated specs instead of linear relative excess, which saturates oddly near zero bounds.
- Beats hand layout because: n/a (scoring robustness).
- Philis status: partial — linear relative miss (`frontend/library/src/perf.rs:91-95`).

### BAL2-42 Layout-inclusive evaluation inside the optimisation loop
- Kind: metric
- Statement: without layout information, a sized opamp met specs nominally but failed after extraction (UGF 91.8 → 89.8 MHz vs 90; PM 67.6° → 63.4° vs 65°) and had large empty areas (AR 0.55); with layout-aware sizing all specs held after extraction (e.g. 105.7 → 105.4 MHz, 65.4° → 65.0°) at AR 0.91–0.98. Full 3-D A–G extraction per iteration was reported fast enough for cells of a few tens of devices (Table 6.8: 880.3 s vs 612.31 s with lookup; text: ~15% more CPU, extraction 17% of total).
- Source: §6.5.2, Tables 6.7–6.8; lines 13212–13266; book pp.264–266 / PDF 274–276.
- Philis stage: flow, verify
- Automation recipe: keep extracting and simulating the winning candidates each epoch; this is the evidence that schematic-only margins are insufficient.
- Beats hand layout because: every candidate is judged post-layout, not only the final one.
- Philis status: implemented — per-epoch post-layout perf scoring of improving epochs (`frontend/library/src/lib.rs:51-55`, `:359-360`).

### BAL2-43 Analytical diffusion area/perimeter per variant
- Kind: formula
- Statement: MOS diffusion areas and perimeters as functions of W, L, finger count, layout style (single, stacked, interdigitated) and process [38]; different foldings change junction capacitance, so geometry choice feeds parasitics.
- Source: §6.1 lines 12418–12421; §6.5.1 lines 13108–13114, 13150–13156; book pp.246, 262–263 / PDF 256, 272–273.
- Philis stage: cells, flow
- Automation recipe: each `cellgen` variant reports its drawn AS/AD/PS/PD (sum of source/drain diffusion shapes per terminal); pass them into the perf netlist for sensitivity runs and into the variant score (drain C of a sensitive node penalised by its sensitivity weight).
- Beats hand layout because: the finger/style choice minimises junction C on sensitive nodes exactly.
- Philis status: missing — no AS/AD/PS/PD per variant found (grep over `frontend`, `kernel/cells`).

### BAL2-44 Two-step sampling to find critical parasitics
- Kind: heuristic
- Statement: sampling n layout parameters at m points gives mⁿ instances; instead generate ~100 instances, simulate to find parasitics with non-negligible impact, then densely sample only parameters driving those; the table is built once per template.
- Source: §6.5.1 lines 13163–13189; book p.263 / PDF 273.
- Philis stage: flow
- Automation recipe: in the first epochs, correlate per-net extracted C with spec deltas across epochs (regression over epochs already run) to identify nets that matter; restrict sensitivity runs and budgets to them.
- Beats hand layout because: data-driven identification of the parasitics that matter.
- Philis status: partial — schematic-point sensitivities per net (`frontend/library/src/perf.rs:189-222`); no post-layout screening.

### BAL2-45 Typed, explicit, categorised constraint records
- Kind: data-model
- Statement: constraints relate design variables; simple (independent) vs complex (coupled through shared variables); categories technology, functional (electrical), design methodology (geometry), commercial; explicit vs implicit (implicit ones are unusable by automation); each constraint type has a unit (IR drop V, delay s). A must-fulfil objective is a constraint; a may-fulfil constraint is an objective.
- Source: §7.2.1 lines 13594–13599; §7.3 lines 13667–13736; book pp.272–274 / PDF 282–284.
- Philis stage: annotator, flow, verify
- Automation recipe: add to each emitted rule a provenance record: category, unit, origin (catalog pattern id, spec id, deck value, user), and derivation parent (for transformed rules). Report rules grouped by category in metadata.
- Beats hand layout because: every constraint is explicit and traceable; implicit designer knowledge becomes checkable.
- Philis status: partial — uniform `Rule`/`RuleBatch` trait with tier (hard/budget/cost) (`kernel/analog/src/rule.rs:9-123`, `:136-205`; `kernel/analog/src/requirements.rs:19-23`); no category/provenance fields.

### BAL2-46 Complex constraints as conjunctions; pin-to-pin IR on star nets
- Kind: check
- Statement: split a constraint into structural, functional and arithmetic parts coupled by shared variables: starShaped(N) ∧ netPin(N,P1) ∧ netPin(N,P2) ∧ irDrop(P1,P2,V) ∧ V < 0.1 V; query returns every pin pair violating it.
- Source: §7.3 lines 13738–13766, Fig. 7.3; §7.5.2 lines 14333–14364; book pp.274–275, 286–287 / PDF 284–285, 296–297.
- Philis stage: verify, annotator
- Automation recipe: extend `IrDrop` to pin-pair form: for each (P1, P2) with a drop budget (e.g. sense pin vs force pin of a mirror), V = Σ terminal currents × terminal-resolved R from `Stack::terminal_resistance_ohm`; report the violating pairs.
- Beats hand layout because: all pin pairs are checked, not only the obvious rail ends.
- Philis status: partial — IR is whole-net worst-path (`kernel/analog/src/routing/ir.rs:8-17`); terminal-resolved R exists (`kernel/analog/src/routing/stack.rs:147-153`).

### BAL2-47 Hierarchical constraint assignment along physical connectivity
- Kind: rule
- Statement: assign constraints top-down, bottom-up or both in the hierarchy; across instances such as metal resistors so every physically connected net segment gets the constraint (e.g. shielding from an I/O pad to a subcell terminal). Bottom-up assignment creates one constraint per flattened instance.
- Source: §7.4.1 lines 13868–13897, Fig. 7.5; book pp.277–278 / PDF 287–288.
- Philis stage: annotator
- Automation recipe: when a net class or budget is assigned, propagate it through series metal resistors and zero-ohm links (resistor devices whose layer is metal) to the connected nets.
- Beats hand layout because: no segment of a constrained net is forgotten.
- Philis status: partial — flat annotation; `retarget(cell_of)` remaps rules to merged cells (`kernel/analog/src/rule.rs:82`); no propagation through metal resistors found.

### BAL2-48 Over-constraint detection and root-cause report
- Kind: check
- Statement: constraint verification must detect mutually conflicting constraints (over-constraint, a CSP), e.g. V_IR,1 ≤ V_IR,2 ∧ w1 ≤ w2 with fixed lengths and equal current is infeasible (Fig. 7.7). Resolution by propagation, relaxation or backtracking; weight-based resolution fails when many weights are equal. An earlier over-constraint can block later optimisation; a root-cause analysis decides to iterate or to override.
- Source: §7.4.1 lines 13836–13867; §7.4.5 lines 14081–14111; §7.6.2 lines 14471–14483; book pp.276–277, 281–282, 290 / PDF 286–287, 291–292, 300.
- Philis stage: annotator, flow
- Automation recipe: pre-flight before the epoch loop: (a) a device in two symmetry groups with different partners or axes; (b) proximity(a,b) max distance < isolation(a,b) min distance; (c) budget below its physical floor (e.g. C budget < C of the minimum Steiner tree of the pins at min width after placement; IR budget with current × R_min of the shortest path above the limit); (d) ordering/alignment constraints forming a positive cycle in a difference-constraint graph (Bellman–Ford); (e) EM width requirement > max track width available. Report each conflict with the rules and their origin (BAL2-45), and mark budgets as relaxed rather than letting Lagrange prices saturate.
- Beats hand layout because: infeasible requirements are named up front instead of after hundreds of epochs.
- Philis status: missing — no conflict detection (grep "over-constrain|conflict|infeasib" finds only LVS label conflicts and stall handling); unsatisfiable budgets are capped by `LAMBDA_MAX` (`backend/gp/src/lib.rs:47-49`).

### BAL2-49 Constraint derivation by rules and class deduction
- Kind: heuristic
- Statement: derive constraints from objectives via fixed derivation rules, logic deduction (a rule written for MOS applies to BJT because both are "transistor"), or expert knowledge; deduction gives the abstraction needed for reuse.
- Source: §7.4.2 lines 13900–13932; book p.278 / PDF 288.
- Philis stage: annotator
- Automation recipe: key catalog rules on device-class traits (matched-pair-capable, current-carrying, gate-like input) rather than concrete device kinds, so BJT/FinFET/MOS all get matching and routing rules from one definition.
- Beats hand layout because: uniform application across device technologies.
- Philis status: partial — pattern catalog derives rules (`backend/annotator/src/catalog.rs`, `backend/annotator/src/extract.rs:8-16`); class abstraction not verified here.

### BAL2-50 Constraint transformation with inverse for verification (IR split example)
- Kind: algorithm
- Statement: higher-level constraints become lower-level ones by transformation rules (several may apply; the choice removes freedom); an inverse transformation must exist for verification. Example: pad-to-terminal IR budget with known current becomes placement constraints (distance) plus routing constraints (length, layer, width); with placement fixed, only routing constraints remain.
- Source: §7.4.3 lines 13935–13975; book pp.278–279 / PDF 288–289.
- Philis stage: annotator, gp, dp, dr, verify
- Automation recipe: for each IR/C budget, split it into a placement share (e.g. 50%: max Manhattan distance between force pins = share / (I·R_per_µm at max width)) emitted as a placement-tier proximity budget, and the routing share (the rest); verify with the inverse (sum of shares ≤ original) at signoff.
- Beats hand layout because: budgets are allocated across steps before the steps run.
- Philis status: partial — spec → routing `PerformanceBudget` (`frontend/library/src/perf.rs:225-260`), headroom → IR budgets (`frontend/library/src/lib.rs` `Bias.net_headroom_mv`); no placement-share transformation.

### BAL2-51 Constraint sensitivity analysis: relative distance and coupling groups
- Kind: formula
- Statement: d_l(x) = exp(x_l − x) − 1, d_u(x) = exp(x − x_u) − 1 (eq. 7.1): 0 at the bound, > 0 violation, < 0 satisfied. Combined with parameter-to-output sensitivities (local derivatives or sampling/Monte Carlo), it prioritises sensitive or violating parameters and finds weakly coupled groups that can be solved independently (dynamic task partitioning). Scales to a few thousand design variables.
- Source: §7.4.4 lines 13978–14021; §7.6.3 lines 14543–14558; §7.7 lines 14591–14596; book pp.279–280, 291–292 / PDF 289–290, 301–302 (eq. 7.1 read from PDF 290).
- Philis stage: flow, dp, dr
- Automation recipe: report each rule's margin as normalised distance (Philis `headroom`/`usage` already); use the interaction graph of rules sharing cells/nets to split repair work: rules in different connected components can be repaired in parallel threads.
- Beats hand layout because: margins are quantified per constraint; independent repairs run concurrently.
- Philis status: partial — headroom/usage/residual/criticality per rule (`kernel/analog/src/rule.rs:17-43`, `:153-166`); margins reported (`frontend/library/src/metadata.rs:141`, `benchmarks/src/bench.rs:99`); no coupling-group partitioning.

### BAL2-52 Temperature-dependent wire resistance in IR checks
- Kind: formula
- Statement: V_IR = i·ρ·l/(w·d)·(1 + TK1·(T − T_ref)); near a heat source, choose between moving the wire (changing T) and widening it; widening dominates when w ≈ w1 (high sensitivity), loses influence for large w. Example ranges T 218–448 K, w 0.18–2.0 µm.
- Source: §7.4.4 lines 14022–14075, Fig. 7.6; §7.5.2 lines 14365–14377; book pp.280–281, 287 / PDF 290–291, 297 (formula read from PDF 290).
- Philis stage: dr, verify, deck
- Automation recipe: evaluate R per segment at the local temperature from the placement's thermal map (`Layout::refresh_temps`) using the deck's metal TCR; apply in `IrDrop` and router R pricing; EM already derates with temperature.
- Beats hand layout because: hot-spot IR is computed, not guessed.
- Philis status: missing for IR — `IrDrop` uses deck R at nominal temperature (`kernel/analog/src/routing/ir.rs:30-37`); EM limits are temperature-derated (`kernel/analog/src/routing/em.rs:12`, `:23`).

### BAL2-53 Meta-verification: decompose, cheap checks first, short-circuit
- Kind: check
- Statement: complex constraint checks are decomposed into independent sub-checks served by existing tools; order sub-checks from short- to long-running and stop at the first violation; rule-set development effort is comparable to DRC/LVS decks. Static verification (signoff, constant data) vs dynamic (what-if inside algorithms; latency matters).
- Source: §7.4.5 lines 14126–14186; book pp.282–283 / PDF 292–293.
- Philis stage: verify, dp, dr
- Automation recipe: in the epoch key computation, evaluate cheap geometric rules first and skip expensive ones (PEX + simulation) when the epoch cannot beat the best key (already done for perf); extend the same gate to DRC/LVS on dominated epochs.
- Beats hand layout because: n/a (throughput).
- Philis status: implemented (partly) — perf scoring only when the epoch's violation count can win (`frontend/library/src/lib.rs:359-360`); rules expose `known`/`applicable` (`kernel/analog/src/rule.rs:50-60`).

### BAL2-54 Constraint simplification and propagation before solving (CHR)
- Kind: algorithm
- Statement: propagation derives new constraints (A ≤ B ∧ B ≤ C ⇒ A ≤ C); simplification removes dominated ones (A ≤ 5 ∧ A ≤ 7 ⇒ drop A ≤ 7); simpagation combines both.
- Source: §7.5.1 lines 14273–14284; book p.285 / PDF 295.
- Philis stage: annotator
- Automation recipe: after extraction, merge duplicate rules on the same object set keeping the tightest bound (two `CrosstalkExclusion` on the same pair → max spacing; two C budgets on one net → min); derive transitive ordering/alignment constraints for BAL2-48(d).
- Beats hand layout because: n/a (fewer, consistent constraints; faster scoring).
- Philis status: missing — no rule-level dedupe found (grep `dedup` only in cell generators).

### BAL2-55 Gradual removal of degrees of freedom across overlapping steps
- Kind: heuristic
- Statement: analog design steps overlap (device generation, preplacement and global routing happen together during floorplanning); degrees of freedom should be removed gradually, keeping them available for optimisation as long as possible; standard algorithm interfaces over shared design + constraint data let modular algorithms be composed; templates carrying constraints are reusable structure.
- Source: §7.2.1 lines 13518–13580; §7.2.2 lines 13653–13660; §7.6.2 lines 14490–14510; §7.6.3 lines 14522–14574; §7.7 lines 14599–14605; book pp.271–273, 290–293 / PDF 281–283, 300–303.
- Philis stage: flow
- Automation recipe: keep variant choice open through dp (done), then allow dr to request a variant change of a cell whose pins block a hard routing rule (escalation already exists at assignment level); expose rules through the one `Rule` interface to every stage (done).
- Beats hand layout because: late decisions use actual routing evidence.
- Philis status: partial — variant reshape in dp (`backend/dp/src/lib.rs:580-605`), assignment escalation after stalls (`frontend/library/src/cellgen.rs:395-415`, `frontend/library/src/lib.rs:384`); unified `Rule` trait (`kernel/analog/src/rule.rs:1-9`).

---

## 4. Top-15 priorities for Philis

1. **BAL2-48** Over-constraint pre-flight with root-cause report — cheap, prevents hundreds of wasted epochs on infeasible rule sets and replaces silent `LAMBDA_MAX` saturation.
2. **BAL2-16** Conservative (sign-split) performance budgets extended to R, coupling C and matched ΔC — current signed rows can certify a layout the linear model only thinks is safe; coverage of R/coupling is missing.
3. **BAL2-30 / BAL2-32** Constraint-graph longest-path + small LP compaction with exact symmetry — deterministic legal, minimal, exactly symmetric placement instead of O(n²) relaxation with projection.
4. **BAL2-19** Route-to-placement feedback of violated budgets into next-epoch net weights (damped) — closes ROAD's outer loop; today placement never learns from routing residuals.
5. **BAL2-22** Differential coupling symmetry per aggressor — hand layouts balance own RC; unequal aggressor coupling is a common offset/CMRR killer that Philis does not check.
6. **BAL2-09 (+BAL2-05)** Inter-layer crossing capacitance in coupling budgets and router cost — crossovers of sensitive nodes are currently free.
7. **BAL2-06 (+BAL2-04, BAL2-46)** Net splitting / force–sense separation and pin-pair IR — op-point currents already exist; this turns them into Kelvin-correct supply/bias routing.
8. **BAL2-20** Hard-bound pruning of partial paths — guarantees budgets by construction and reduces repair rounds.
9. **BAL2-21** Steiner/C-aware route estimate in the dp anneal for budgeted nets — placement judged by real parasitics, not HPWL.
10. **BAL2-39** Pareto pruning of variants plus aspect-ratio/max-W/H goals — multiplicative search-space reduction and a missing user-facing constraint.
11. **BAL2-02** A* heuristic in gr/dr — pure speed; more epochs and repair trials per unit time.
12. **BAL2-08 (+BAL2-07)** Symmetric routing from the start with mirrored obstacles, and crossover connectors for axis-swapped pairs — exact symmetry without post-hoc repair.
13. **BAL2-43** Analytical AS/AD/PS/PD per variant feeding sensitivity and variant scoring — finger/style choice driven by junction C on sensitive nodes.
14. **BAL2-11 (+BAL2-13)** Electrical "noisy" net class from op-point swing/topology — enables ILAC/SLAM ordering and noisy–sensitive spacing on every net regardless of names.
15. **BAL2-29** Yield-aware wire spreading within leftover margins — converts unused spec headroom into fewer bridging faults, something hand layout never quantifies.
