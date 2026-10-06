# Lampaert, Gielen, Sansen: Analog Layout Generation for Performance and Manufacturability (1999), study for Philis

Source: K. Lampaert, G. Gielen, W. Sansen, *Analog Layout Generation for Performance and Manufacturability*, Kluwer/Springer 1999 (LAYLA). The PDF is `docs/ref/Analog Layout Generation for Performance and -- Koen Lampaert ... .pdf`.
Reftext: `scratchpad/reftext/lampaert.txt`, **lines 1–7750 (whole file)**. PDF page numbers follow the task rule `1 + count(\f)` and were checked against the page images; book page = PDF page − 12 or − 13.
Entry prefix: `LAMP-NN`. "Philis status" gives file:line evidence from a short grep of the tree as of 2026-09-28. It is not a code audit.

---

## 1. Coverage

**Read chunks (Read tool, consecutive, every line):**
- 1–1200 (a first 1–2000 request went over the tool's 25k-token cap, so the file was re-read in chunks of 1100–1200 lines)
- 1201–2300
- 2301–3400
- 3401–4500
- 4501–5600
- 5601–6700
- 6701–7750 (end of file; the bibliography 7125–7750 was read in full)

**PDF pages opened because equations were garbled in the text:** 37–39 (eqs 2.2–2.11), 50 (2.29–2.32), 53–54 (2.35–2.39), 107 (4.9–4.12), 112–113 (4.19–4.28), 115–116 (4.32–4.35), 118 (4.37–4.40), 146 (5.4–5.8), 151 (5.12–5.14), 153–155 (5.16–5.26), 157–159 (5.28–5.34).

**Every heading in the range:**
Front matter: Abstract (95–124); Contents, List of Tables, List of Figures (125–395).
Ch 1 Introduction (396): 1.1 Mixed-signal Design Methodology (411); 1.2 A Hierarchical Performance-Driven Design Strategy (495); 1.3 Physical Design Tools for Mixed-signal ICs (667): 1.3.1 Circuit Level Layout Generation (669), 1.3.2 System Level Layout Generation (686), 1.3.2.1 Floorplanning (692), 1.3.2.2 Block Level Layout Assembly (707), 1.3.3 Layout Extraction and Verification (716), 1.3.4 Scope (743); 1.4 Layout Styles (764): 1.4.1 Full-Custom (771), 1.4.2 Semi-Custom (784), 1.4.3 Scope (840); 1.5 Existing Tools for Analog Layout (850): 1.5.1 The Analog Macro-Cell Layout Style (862), 1.5.2 Implementations of the Macro-Cell Layout Style (945), 1.5.2.1 Procedural Tools (946), 1.5.2.2 Template Based (976), 1.5.2.3 Rule Based (1008), 1.5.2.4 Algorithmic Approaches (1030; "Heuristic Approaches" 1045, "Performance Driven Approaches" 1089), 1.5.3 Situation of This Work (1112); 1.6 Overview of LAYLA (1151): 1.6.1 Circuit Analysis (1169), 1.6.2 Device Generation (1179), 1.6.3 Placement (1190), 1.6.4 Routing (1200); 1.7 Summary and Conclusions (1216).
Ch 2 Performance Driven Layout (1238): 2.1 Introduction (1243); 2.2 Problem Formulation (1263); 2.3 Previous Work (1421): 2.3.1 Digital (1430), 2.3.1.1 Net-Based (1442), 2.3.1.2 Path-Based (1460), 2.3.2 Analog (1467), 2.3.2.1 Berkeley Tools (1468), 2.3.2.2 KOAN/ANAGRAM III (1517), 2.3.2.3 GELSA (1526), 2.3.3 Discussion (1536); 2.4 A Direct Performance Driven Layout Strategy (1575): 2.4.1 Modeling Performance Degradation (1601), 2.4.2 Generation of Performance Sensitivities (1623), 2.4.2.1 Perturbation (1641), 2.4.2.2 Direct (1655), 2.4.2.3 Adjoint (1676), 2.4.2.4 Discussion (1723), 2.4.3 Modeling of Layout Parasitics (1745); 2.5 Interconnect Parasitics (1759): 2.5.1 Interconnect Modeling (1788), 2.5.2 Equivalent Circuit Model (1799), 2.5.3 Parasitic Extraction (1888), 2.5.3.1 Parasitic Capacitance (1892), 2.5.3.2 Parasitic Resistance (1984); 2.6 Device Parasitics (2011); 2.7 Mismatch (2066): 2.7.1 Mismatch Model (2088), 2.7.2 Layout Rules for Optimum Matching (2150); 2.8 Thermal Effects (2229): 2.8.1 Operating Temperature (2240), 2.8.2 Thermal Analysis (2352), 2.8.3 Discussion (2464); 2.9 Substrate Coupling (2476): 2.9.1 Injection, Reception, Transmission (2487), 2.9.2 Modeling (2516), 2.9.3 Layout Measures (2561), 2.9.3.1 Physical Separation (2562), 2.9.3.2 Guard Rings (2595); 2.10 Summary (2607).
Ch 3 Module Generation (2639): 3.1 (2643); 3.2 Problem Formulation (2661); 3.3 Module Generation Strategies (2757): 3.3.1 Fixed Library (2763), 3.3.2 Dynamic Merging (2792), 3.3.3 Simultaneous Placement and Module Optimization (2813), 3.3.4 Discussion (2833); 3.4 Transistor Stacking Algorithms (2854); 3.5 Procedural Module Generation (2973); 3.6 Technology Independence (3000); 3.7 Examples (3051): 3.7.1 MOS Transistor (3066), 3.7.1.1 MOS Definition (3067), 3.7.1.2 Parameters (3081), 3.7.1.3 Examples (3160), 3.7.2 Cascode MOS Transistor Pair (3237); 3.8 Summary (3247).
Ch 4 Placement (3275): 4.1 (3279); 4.2 Problem Formulation (3302); 4.3 Overview (3443); 4.4 Previous Work (3474): 4.4.1 CP (3490), 4.4.2 FDP (3498), 4.4.3 PbP (3508), 4.4.4 QO (3518), 4.4.5 SE (3527), 4.4.6 SA (3541), 4.4.7 Discussion (3611); 4.5 SA for Analog Performance Driven Placement (3678): 4.5.1 Placement Representation (3705), 4.5.1.1 Flat (3738), 4.5.1.2 Slicing (3748), 4.5.1.3 Discussion (3787), 4.5.2 Device Representation (3825), 4.5.3 Interconnect Area Estimation (3944); 4.6 Handling Analog Constraints in SA (4033); 4.7 Move Set (4132); 4.8 Cost Function (4398); 4.9 Estimating Performance Degradation (4460): 4.9.1 Interconnect Parasitics (4503), 4.9.1.1 Net Topology Estimation (4510), 4.9.1.2 Interconnect Parasitics Extraction (4614), 4.9.2 Device Mismatch (4718), 4.9.3 Thermal Effects (4768), 4.9.3.1 Unit Source (4799), 4.9.3.2 Device Profile (4841), 4.9.3.3 Placement Profile (4851); 4.10 Dynamic Interconnect Area Estimation (4874); 4.11 Annealing Schedule (4913); 4.12 Experimental Results (5020): 4.12.1 Comparator (5027), 4.12.2 Opamp1 (5193), 4.12.3 Opamp2 (5267), 4.12.4 Opamp3 (5309); 4.13 Summary (5391).
Ch 5 Routing (5411): 5.1 (5415); 5.2 Problem Formulation (5425); 5.3 Overview (5479); 5.4 Classification (5503): 5.4.1 Routing Strategy (5511), 5.4.2 Routing Model (5543), 5.4.3 Search Strategies (5593); 5.5 Previous Work in Area Routing (5633): 5.5.1 Maze (5639), 5.5.2 Line-Search (5668), 5.5.3 Line-Expansion (5696), 5.5.4 Discussion (5714); 5.6 A Grid-Less Maze Routing Algorithm (5734): 5.6.1 Routing Model (5778), 5.6.1.1 Corner Stitching (5783), 5.6.1.2 Routing Cells (5865), 5.6.2 Source Region Expansion (5917), 5.6.3 Path Expansion (5965); 5.7 Cost Function (6065): 5.7.1 Actual Path Cost (6099), 5.7.2 Predictor Term (6181), 5.7.3 Symmetric Routing (6199), 5.7.4 Multi-Terminal Nets (6231); 5.8 Net Scheduling (6282): 5.8.1 Pre-Routing (6297), 5.8.2 Performance Driven Routing (6307), 5.8.3 Manufacturability (6351); 5.9 Estimating Yield and Testability (6378): 5.9.1 Yield Modeling (6405), 5.9.2 Testability (6619); 5.10 Experimental Results (6695): 5.10.1 Opamp1 (6702), 5.10.2 Opamp2 (6765), 5.10.3 CPU Times (6790); 5.11 Summary (6807).
Ch 6 Implementation (6830): 6.1 (6834); 6.2 Implementation (6839): 6.2.1 Source Code (6840), 6.2.2 Interface to Electronic Design Frameworks (6902); 6.3 Use of LAYLA in an Industrial Environment (6922): 6.3.1 Link to Schematic Capture (6928), 6.3.2 Link to Simulation (6942), 6.3.3 Back-Annotation (6986); 6.4 Results (6996).
Ch 7 General Conclusions (7011); Suggestions for Future Work (7082). Bibliography (7125–7750).

---

## 2. Section-by-section digest

### Front matter (1–395)
- The abstract states the thesis. LAYLA drives placement and routing directly by performance constraints, and "performance degradation … is evaluated at runtime using predetermined sensitivities", "without intermediate parasitic constraint generation step" (107–118, PDF 5).
- A new fault-detectability criterion is combined with a yield model and integrated into performance-driven routing (119–124).
- The TOC and lists (125–395) index 6 tables of results (4.1–4.6, 5.1–5.3).

### Ch 1 Introduction
- **1.1 (411–493, PDF 14–16).** Mixed-signal flow: high-level spec → high-level design → synthesis → layout. The analog subsystem includes ADC/DAC, clocks and control logic ("mixed A/D subsystem").
- **1.2 (495–663, PDF 16–20).**
  - Hierarchy: system / module / circuit / device.
  - Between two levels: top-down architecture selection → sizing → verification, then bottom-up layout generation, extraction and verification.
  - AMGIE offers standard-cell, parameterized-cell and full-custom styles.
- **1.3.1 (669–683).** The circuit-level generator takes the netlist and specifications plus aspect-ratio and pin constraints. It must keep "the performance degradation induced by their combined effects" within specs. The key effects are interconnect parasitics (including crosstalk), mismatch and thermal.
- **1.3.2–1.3.2.2 (686–713).**
  - The floorplanner fixes position, aspect ratio and terminal positions under bounded degradation, including interconnect, thermal and substrate effects. Its estimated routing parasitics feed synthesis of the lower-level blocks.
  - Block assembly must control the same effects.
- **1.3.3–1.3.4 (716–761).**
  - DRC, extraction, LVS and ERC; ERC covers EM, self-heat and floating nodes.
  - Scope: circuit level, 10–100 devices.
  - Substrate noise was studied but not implemented. Extraction tools were commercial.
- **1.4–1.4.3 (764–847, PDF 22–24).** Full-custom versus semi-custom (standard cell, MPGA, FPGA, sea-of-gates). The placer adapts to semi-custom styles by constraining the move set.
- **1.5.1 (862–898).** Macro-cell style: module recognition → variants per module → placement with variant selection → routing → optional compaction.
- **1.5.2.1–1.5.2.3 (946–1026).**
  - Procedural generators give high quality but cost a lot of code per topology.
  - Templates (Philips "design by example" with slicing trees and a river router) must be redone per circuit type and per technology.
  - Rule-based tools (ALSYN: min-cut plus Stockmeyer) are only as good as their rule set.
- **1.5.2.4 (1030–1110, PDF 28–30).**
  - Heuristic tools (ILAC) use four net classes: sensitive, noisy, non-critical, supply. The classes set routing priority and coupling penalties (1060–1066).
  - KOAN/ANAGRAM II adds dynamic merging, over-device routing, crosstalk avoidance and symmetric wiring.
  - Performance-driven tools (Choudhury, Malavasi, Charbon, Felt; Basaran's bounded router) map specs to parasitic bounds.
- **1.5.3 (1112–1148).**
  - LAYLA removes the constraint-generation step and handles interconnect, mismatch and thermal effects together.
  - Several equally valid layouts exist in all but the tightest cases, so the spare freedom goes to yield and testability.
- **1.6–1.6.4 (1151–1213, PDF 31–33).**
  - Inputs: netlist, specification file, technology file.
  - The circuit analyzer drives a commercial simulator to get the sensitivities of every performance to every parasitic.
  - Generators run in interface mode (bbox plus terminals) and then layout mode.
  - The SA placer controls interconnect, mismatch and thermal degradation.
  - The router works in two phases: performance-driven routing, then rip-up for yield and testability.
- **1.7 (1216–1237).** Summary of the above.

### Ch 2 Performance-driven layout
- **2.1 (1243–1260, PDF 34).** A traditional layout needs extraction and verification loops, and the extracted list gives "no clue" which parasitic caused a violation. Performance-driven layout aims to meet the specs by construction.
- **2.2 (1263–1418, PDF 34–38).**
  - A spec is an interval `P ∈ [P_min, P_max]` (2.1). P depends on design parameters, process parameters and layout parasitics.
  - Process spread gives `[P_proc^min, P_proc^max]` (2.2).
  - Layout margins: `ΔP_lay ∈ [P_min − P_proc^min, P_max − P_proc^max]` (2.3–2.7). These are the inputs of the layout tool.
- **2.3.1–2.3.1.2 (1430–1464).** Digital timing-driven layout is either net-based (net-length bounds via zero-slack or convex programming, which over-constrains and gives no convergence recipe) or path-based (optimize performance directly).
- **2.3.2.1 (1468–1515, PDF 39–40).** Berkeley: sensitivities plus QP give parasitic bounds that maximize the layout flexibility sum `f_j = a_j + b_j·x + c_j·x²` (2.8–2.11). Applied to channel routing, area routing, placement and compaction. Iterations are needed when a bound fails.
- **2.3.2.2–2.3.2.3 (1517–1533).** Basaran's router prunes partial paths whose accumulated parasitic exceeds its bound. GELSA runs SA with slicing plus in-loop global routing but is restricted to slicing.
- **2.3.3 (1536–1572, PDF 40–41).**
  - One fixed set of parasitic bounds over-constrains the tools and forces re-iteration.
  - The "flexibility" criterion cannot be quantified: it ignores terminal area, distance, obstacles and previously routed nets.
- **2.4 (1575–1598, PDF 41–42).** The direct method has three steps: geometry → parasitic values → linear sensitivity model → penalty when ΔP leaves the margin. The tool either yields a correct layout or flags the specs as infeasible, with no iterations.
- **2.4.1 (1601–1620).** `ΔP_j,lay = Σ_i S_xi^Pj · x_i`, with `S = ∂P_j/∂x_i` (2.12–2.13).
- **2.4.2–2.4.2.4 (1623–1742, PDF 42–45).**
  - Perturbation (2.15): N+1 solves, and a small Δh is unstable.
  - Direct method (2.16–2.17): reuse the Newton LU, one solve per parameter.
  - Adjoint method (2.18–2.24): `Mᵀx' = −d`, two solves in total for one output function.
  - N_p × N_x sensitivities are needed. HSPICE had only DC sensitivities and the SPICE3 direct method was not robust, so LAYLA used perturbation.
- **2.4.3 (1745–1756).** Extraction is too slow for inner loops and needs finished layouts, so fast geometric models are used on partial layouts.
- **2.5–2.5.2 (1759–1850, PDF 45–48).**
  - Parasitics load poles and zeros (bandwidth, phase margin, slew) and couple nets (noise, PSRR, stability). "There is no concept of a noise margin for analog".
  - Exact T/Π line models (2.25–2.28) reduce to lumped models when |γD| ≪ 1. L and G are neglected when G₀ ≪ ωC₀ and ωL₀ ≪ R₀.
  - LAYLA's model: an n-terminal net has n resistors, one ground C and N−1 coupling Cs.
- **2.5.3.1 (1892–1980, PDF 49–50).** Numerical extraction (FD, FEM, BEM, multipole) is too slow. Geometric model: area `ε/d·A` (2.29), lateral `F(d)·l` with `F = C₀ + C₁/d + C₂/d² + C₃/d³ + C₄/d⁴` (2.30–2.31), fringe `C_F0·l·(e^{−x₂/x₀} − e^{−x₁/x₀})` (2.32).
- **2.5.3.2 (1984–2008).** Resistance: decompose along equipotentials, `R = ρ□·l/w` (2.33), then series and parallel combination.
- **2.6 (2011–2062, PDF 51–52).** Gate C is fixed by sizing. The junction C `A·C_j/(1−V/φ)^m + P·C_jsw/(1−V/φ)^m_sw` (2.34) is reducible by folding, merging and abutment.
- **2.7–2.7.1 (2066–2147, PDF 52–54).**
  - Kinget: the ratio Speed·Accuracy²/Power is fixed by technology mismatch.
  - Pelgrom `σ²(ΔP) = A_P²/(WL) + S_P²·D²` (2.35), specialized to V_T0, K and β (2.36–2.38).
  - Bastos short-channel model for V_T (2.39).
- **2.7.2 (2150–2207, PDF 54–55).**
  - Rules: same structure, same temperature (isotherms), same shape and size (fingers; resistor L, W and bends), common-centroid or 2-axis symmetry, same orientation and current direction, same surroundings (dummies).
  - Under package stress, finger and interdigitated-finger β fluctuation reaches up to 5× the model. Quad and waffle layouts show no systematic mismatch.
- **2.8–2.8.1 (2229–2349, PDF 56–58).**
  - V_T, K_P, BJTs and passives are temperature-sensitive.
  - The failure rate doubles for about every 10 °C.
  - Degradation comes from absolute shifts (reference drift) and from gradients (offset, drift, dc feedback).
- **2.8.2 (2352–2461, PDF 59–61).**
  - N-layer conduction model with an isothermal bottom and adiabatic top (2.40–2.45). The surface T is a double Fourier series (2.46), with coefficients from the recursion (2.47–2.49).
  - Edges at least one structure thickness from the boundary behave as infinite.
- **2.8.3 (2464–2473).** The number of series terms scales with chip/source size, which is too slow for layout loops. The incremental scheme is in Ch 4.
- **2.9–2.9.1 (2476–2511, PDF 61–62).**
  - Injectors: junction and collector caps, capacitor bottom plates, varying wells, impact ionization.
  - Receivers: capacitive coupling and the body effect (low frequency).
  - The resistive substrate model is valid up to 2 GHz. Above 4–5 GHz Maxwell's equations are needed.
- **2.9.2 (2516–2539).** Substrate as an n-port resistive network (3-D mesh, single epi node, FD, BEM). Speed-ups: lumped guard-ring models, precomputed impedances, Genderen's common node with neighbour-only direct resistances.
- **2.9.3.1–2.9.3.2 (2561–2604, PDF 63–64).**
  - Low-impedance epi substrate: separation beyond 4× the effective epi thickness (≈40 µm) gives no improvement.
  - High-impedance substrate: coupling falls almost linearly with separation.
  - Guard rings help only when close to the sensitive circuit and biased from dedicated package pins.
- **2.10 (2607–2638).** Module generation fixes device-level parasitics, placement sets their minimum values and routing sets the final values. The distance between matched devices is traded against the other effects.

### Ch 3 Module generation
- **3.1–3.2 (2643–2747, PDF 66–68).**
  - Partition the circuit into modules. Each module has variants and shapes, and the placer picks one.
  - Junction C (3.1). Two transistors sharing a node can share diffusion if they have the same type and the same bulk potential (2740–2743).
- **3.3.1 (2763–2789).** Fixed procedural sub-circuit generators (diff pair, mirror, cascode) hard-code sharing. Mapping the circuit onto them is non-trivial and sharing opportunities are lost. Each generator must be maintained per process.
- **3.3.2 (2792–2810).** KOAN dynamic merging finds every sharing option during placement and costs it, but burdens the placer. Some structures, such as interdigitated cascodes, cannot be built by merging.
- **3.3.3 (2813–2829).** Charbon: a stack generator enumerates alternative module sets, and SA swaps between them.
- **3.3.4 (2833–2850, PDF 70).** LAYLA combines dedicated generators (interdigitated cascodes, common-centroid pairs) with dynamic merging. The total junction C drops in proportion to the overlap area and perimeter, "promoted specifically for sensitive nets".
- **3.4 (2854–2970, PDF 70–72).**
  - Diffusion graph: nets are vertices, transistors are S–D edges.
  - Steps: split by type and bulk → fold → split by width → per-subgraph stacking.
  - Malavasi's path enumeration plus clique search is exponential.
  - Basaran's Euler-trail method with a supervertex is linear, handles symmetry and matching, and is optimal for a class of circuits.
- **3.5 (2973–2997).** Interface mode returns abstractions for all variants. Layout mode draws the chosen variant.
- **3.6 (3000–3048, PDF 73–74).** Symbolic geometric parameters (width, spacing, enclosure) and electrical parameters (sheet R, cap, max current density), plus device definitions that map generic layers to process layers (with a 'void' layer).
- **3.7 (3051–3058).** Library contents: MOS, capacitors, resistors, inductors, transformers and matched pairs. Ported to 20+ processes from 5 foundries, about 1 h per port.
- **3.7.1.1–3.7.1.2 (3066–3157, PDF 75–77).**
  - MOS definition: gate, S/D and guard-ring layers, masks and contact types.
  - Parameters: W, L, fingers, slices (bulk-contact max-distance rule), current (widen S/D wires and add vias), aspect ratio (derive fingers and slices), type, technology file, mode, format.
- **3.7.1.3 (3160–3211, PDF 77–78).** Current-sized wires (3.2–3.5). Wide source wires lengthen gate wires and add gate R (noise, RC), so metal2 is used for the source.
- **3.7.2 (3237–3244, PDF 81).** In a cascode pair the shared S/D has no contact, so the poly lines can sit at minimum distance and the node C drops. Merging cannot produce this structure.
- **3.8 (3247–3274).** Summary.

### Ch 4 Placement
- **4.1 (3279–3299, PDF 83).** Placement sets matched distances and the thermal profile, and it fixes the minimum achievable interconnect parasitics. All three must be handled together.
- **4.2 (3302–3440, PDF 83–86).**
  - Symmetry: couples with identical variants and mirrored orientation, self-symmetric devices on the axis, one axis per group.
  - Matching: equal orientation; equal variants, or unit devices for a ratio; distance chosen by performance.
  - Performance constraints: interconnect, mismatch, thermal.
  - Geometry: aspect ratio, fixed height, terminal positions.
- **4.3 (3443–3471, PDF 86–87).**
  - Margins come from specs versus the sized values including statistical spread (e.g. PM ≥ 60°).
  - Simulations give sensitivities and the operating point (branch currents, node voltages).
  - Output: placement plus the final degradation and "an identification of the most important contributions".
- **4.4.1–4.4.6 (3474–3607, PDF 87–90).** Surveys constructive, force-directed, min-cut (KL/FM), quadratic, genetic and SA placement. SA uses Metropolis acceptance `exp(−ΔC/T)` and a Boltzmann stationary distribution (4.1–4.3).
- **4.4.7 (3611–3674, PDF 90–92).** Requirements: pick variant, position and orientation together; rectilinear terminals that cannot be reduced to points; 20–30 devices; sizes spanning orders of magnitude; arbitrary constraints; decisions based on full-layout degradation. SA or SE fits. SA was chosen for maturity and batch mode.
- **4.5–4.5.1.3 (3678–3822, PDF 92–95).** Flat representation over slicing: symmetry lives in the move set, absolute coordinates allow parasitic estimation, non-slicing topologies are denser, and overlap is exploitable for merging. Channel-routing advantages are irrelevant for analog.
- **4.5.2 (3825–3941, PDF 95–97).**
  - Device: minimum overlapping cover (MOC, O(n⁴) approximation) plus a bbox prefilter, bulk net and thermal profile.
  - Terminal: disjoint rectangle partition (a MOC would overcount C), bbox, layer, net.
  - Legal overlap requires same net, same layer and same bulk.
- **4.5.3 (3944–4030, PDF 97–99).** Routing space is added as a per-side border `Δint_x = static + dynamic` (4.4). The static part is the sum of the widths of the wires on that side (4.5).
- **4.6 (4033–4129, PDF 99–101).**
  - Hard constraints go in the move set: symmetry, equal orientation and variant, hard height.
  - Penalties: performance, soft geometry, and overlap (kept soft to find beneficial overlaps). Hard penalties need large weights.
- **4.7 (4132–4395, PDF 101–105).**
  - Groups: independent (translate, swap), symmetry (symmetric translate, swap, flip, axis shift), orientation (equal or mirrored), shape (equal variants).
  - Orientations are precomputed as variants, so a reorientation is a pointer switch.
  - The construction rules for groups follow from the constraints.
- **4.8 (4398–4457, PDF 106–107).** `C = αC_area + βC_AR + γC_overlap + δC_perf` (4.6–4.8). The performance and AR weights decrease linearly and the overlap weight increases after each inner loop.
- **4.9 (4460–4499, PDF 107).** Per-spec penalty (4.11), summed over specs (4.12).
- **4.9.1.1 (4510–4612, PDF 108–110).** Estimators: semi-perimeter O(n), MST O(n²), centre of mass, Steiner (NP-complete), source-to-sink, complete graph. MST with edge-to-edge Manhattan distance (Cohn 94) was chosen as the most accurate.
- **4.9.1.2 (4614–4715, PDF 110–112).** `C_x = C_w + C_t` (4.13–4.17), coupling from MST segments in parallel or overlapping (4.18–4.20), segment R (4.21), `ΔP_j` (4.22).
- **4.9.2 (4718–4765, PDF 112–113).** σ(V_T0), σ(β) (4.23–4.24). `ΔP_j = Σ_k |S|·3σ` (4.25) splits into a constant area term and a distance term `Σ S_Dk·D_k` (4.26–4.27).
- **4.9.3–4.9.3.3 (4768–4871, PDF 113–116).**
  - Direct evaluation of (4.28) takes about 30 s per evaluation for a 2-layer model.
  - A DCT gives the unit-source profile on a P×Q grid (4.29–4.31).
  - Variant profile = sum of the unit sources over the fractured active area.
  - Placement profile = superposition.
  - `ΔP_j = Σ S_ΔTk·ΔT_k` (4.32).
- **4.10 (4874–4910, PDF 116).** Dynamic routing space by distance to the placement centre (4.34), or by distance to each net's bbox centre (4.35). The second is "consistently and significantly better".
- **4.11 (4913–5017, PDF 117–119).**
  - Homogeneous Markov chains.
  - `T₀` from `P₀ = exp(−ΔC⁺/T₀)` with P₀ = 0.6.
  - Stop when the last costs of consecutive chains lie within an interval.
  - Chain length by the Catthoor quasi-equilibrium test.
  - `T_{k+1} = αT_k`, α ∈ [0.8, 0.95], larger for long chains.
- **4.12.1 (5027–5082, PDF 119–120).** Comparator: performance-driven vs off gives offset 3.7 vs 6.9 mV (spec < 5) and delay 2.8 vs 5.4 ns (spec < 5), at 106 vs 93 s CPU. Pair distances are chosen by sensitivity (Table 4.2).
- **4.12.2 (5193–5223, PDF 123–124).** Opamp1: all specs met in one pass in 83 s (Table 4.3). The remaining margin is left for routing.
- **4.12.3 (5267–5274, PDF 125).** Fully differential Opamp2 in 163 s. All degradations stay in margin (Table 4.4).
- **4.12.4 (5309–5330, PDF 127).** Class-AB Opamp3, 140 mW. Temperature-difference sensitivities come from simulation. Pairs were placed symmetric about the output devices, and the total thermal offset is 1.012 mV (Tables 4.5–4.6).
- **4.13 (5391–5409).** First placer to handle all three effects together and directly. Performance gains at a small CPU cost.

### Ch 5 Routing
- **5.1–5.2 (5415–5475, PDF 131–132).** Routing fixes the final parasitics. Requirements: RC performance constraints; symmetric routing even for non-symmetric placements; variable width for EM; yield and testability; n-layer use.
- **5.3 (5479–5494).** Circuit analysis → pre-route to rank nets → performance-driven routing (line expansion) → yield/testability loop while margin remains.
- **5.4.1 (5511–5540, PDF 133–134).** Area routing, one net at a time, is chosen. Global/detailed decomposition loses the global view needed for symmetry, matching and crosstalk, and analog cells have only 10–20 nets.
- **5.4.2 (5543–5590, PDF 134–135).** A grid wastes area (worst-case pitch), forbids variable width and forces terminals onto the grid, so the model is gridless with unreserved layers.
- **5.4.3 (5593–5630).** DFS, BFS, best-first and A*. Heuristic search is used with a predictor aimed at the best performance.
- **5.5.1–5.5.4 (5639–5731, PDF 136–138).** Lee (BFS, shortest), Soukup (DFS plus BFS, not shortest), Hadlock (A*), Mikami–Tabuchi, Hightower, line expansion (Heyns). The adopted router is a gridless hybrid called a "grid-less maze router".
- **5.6 (5734–5772).** Loop: pop the best partial path (A*) → expand → DRC check → target check → insert.
- **5.6.1.1 (5783–5863, PDF 139–141).** Linked list O(n), bins, corner-stitched tiles (hint tile, near O(1) in practice). Two tile planes per layer (max horizontal and max vertical strips). Vias go into both layers. User no-route areas are supported.
- **5.6.1.2 (5865–5914).** Regular cells (x, y, L, W) and via cells (x, y, L_b, L_t, W_b, W_t) carry back pointers. Paths are rebuilt by following them.
- **5.6.2 (5917–5963, PDF 142–143).**
  - Source cells sit on the shrunk source box perimeter at process resolution, plus via cells up and down.
  - Width `W_i = I/I_max,i` (5.1), `N_via = I/I_max,ij` (5.2), `W_ij = [⌈√N_via⌉ − 1]·sep + 2·max(ovl_i, ovl_j)` (5.3).
  - Width belongs to the segment.
- **5.6.3 (5965–6062, PDF 143–145).** A regular cell expands into 3 same-layer cells and 3+3 via cells (centre, left and right alignment). A via cell expands like a source region. Every new cell is DRC-checked. Fibonacci heap, insert O(log n).
- **5.7–5.7.1 (6065–6178, PDF 146–148).** `PathCost = actual + predictor` (5.5–5.8). The segment cost is the spec-summed ΔP from R (5.9) and from ground, lateral, fringe and overlap C within an influence distance d (Arora models) (5.10–5.11).
- **5.7.2 (6181–6196).** The predictor is the same estimate on the minimum Manhattan path. It is optimal iff it is a lower bound.
- **5.7.3 (6199–6228, PDF 148–149).** Route one net of a symmetric pair and mirror it. Each new cell is DRC-checked on both sides of the axis. This works for partially symmetric placements.
- **5.7.4 (6231–6279, PDF 149–150).** Prim-like and Kruskal-like multi-terminal construction, with similar quality.
- **5.8–5.8.2 (6282–6348, PDF 150–152).** Three phases. Pre-route every net alone, ignoring DRC and constraints. Impact factor `F_k = (Σ_j ΔP_j^k/ΔP_j)/N_s` (5.12–5.14). Rip up and reroute in increasing F_k order, so the most critical nets keep their ideal routes.
- **5.8.3 (6351–6375, PDF 152).** With leftover margin, reroute nets and prune any partial path that would violate a spec. The added term is `T_manuf = Σ_{i≠k} λ_ki·Ψ_ki` (5.15).
- **5.9–5.9.1 (6378–6616, PDF 152–157).**
  - Defect → fault mapping via critical area (Stapper). λ, Poisson yield (5.16–5.20).
  - Pinhole: overlap area times density (5.21–5.23).
  - Photolithographic short and open critical areas (5.24–5.25) with a 1/x³ defect size distribution (5.26).
  - Closed forms `L·x₀²·D/(2s)`, `L·x₀²·D/(2w)` (5.27–5.31).
- **5.9.2 (6619–6692, PDF 157–159).** Specification-based vs fault-based testing. Monte-Carlo mean and covariance; χ² acceptance ellipsoid (5.32–5.33); Mahalanobis fault distance `d_0,k` (5.34); `Ψ = 1/d`. The test signature is the IPS RMS plus harmonics.
- **5.10–5.10.3 (6695–6803, PDF 159–164).** Opamp1: TER 0.11 → 0.01 % while staying in spec. Opamp2: TER 0.21 → 0.03 %. CPU split is about 33 % placement, 38–40 % routing and 27–29 % yield (Table 5.3).
- **5.11 (6807–6829).** Summary of the three phases.

### Ch 6 Implementation
- **6.1–6.2.1 (6834–6899, PDF 165–166).** About 115 kLOC of C++: data structures 6k, parser 11k, technology 6k, geometry 5k (MST and tile planes), circuit analysis 4k, generators 36k, placement 26k, router 21k. Generators derive from one base class.
- **6.2.2 (6902–6919).** Interfaces to Falcon (AMPLE), DFII (SKILL) and ROSE. Layout is exported as extension-language command files.
- **6.3.1 (6928–6939, PDF 167).** Layout constraints (generator properties, matching and symmetry groups, port locations) are annotated on the schematic symbols. Cross-probing between layout and schematic.
- **6.3.2–6.3.3 (6942–6992, PDF 167–168).** Each generator has a simulation model with its layout parasitics (Fig. 6.2: resistor and capacitor models). Layout parasitics are back-annotated so designers can re-simulate early.
- **6.4 (6996–7010, PDF 168–169).** Manual layout time splits into module generation 50 %, placement 12.5 %, routing 25 % and verification 12.5 %. LAYLA gave a 5–7× productivity gain over 20+ chips.

### Ch 7 General conclusions (7011–7124, PDF 170–172)
- Recaps the direct strategy, hybrid generators plus merging, the three-effect placer and the manufacturability router.
- Future work:
  - Wire and mutual inductance for RF.
  - A 10–15 % area penalty versus manual layout, caused by the place/route separation and by overestimated routing area. Remedies: simultaneous P&R (too complex, per Cohn 94), global routing inside placement, or performance-aware compaction.
  - Use layout-derived parasitic estimates in sizing.

### Bibliography (7125–7750)
- Key primary sources to chase:
  - Choudhury 90b/93 and Charbon 93 (constraint generation).
  - Basaran 93 (bounded router) and 96 (O(n) stacking).
  - Cohn 91/94 (KOAN/ANAGRAM).
  - Arora 96 (capacitance models).
  - Lee 88/89 (thermal).
  - Stapper 83/84 (critical area).
  - Catthoor 88 (SAMURAI schedule).
  - Gielen 94 (IPS test).
  - Pelgrom 89 and Bastos 95/96 (mismatch).

---

## 3. Actionable extraction

### LAMP-01 Layout margin from the spec and the process spread
- **Kind:** formula
- **Statement:**
  - For each spec `P ∈ [P_min, P_max]` (2.1), with process interval `[P_proc^min, P_proc^max]` (2.2), layout may spend `ΔP_lay ∈ [ΔP_lay^min, ΔP_lay^max]`, where `ΔP_lay^min = P_min − P_proc^min` (2.6) and `ΔP_lay^max = P_max − P_proc^max` (2.7).
  - The first bound is ≤ 0 (a floor spec) and the second ≥ 0 (a ceiling spec). Either may be ±∞.
  - The nominal value alone is not the reference. The worst process corner is.
- **Source:** §2.2, Fig. 2.2, eqs 2.1–2.7; reftext L1305–1418; PDF 35–38.
- **Philis stage:** flow, annotator.
- **Automation recipe:**
  - Input: specs `{metric, min, max}` (`perf::Spec`).
  - Run the testbench at the deck's process corners, or N Monte-Carlo runs; take the min and max of each metric as P_proc.
  - Emit one budget row per finite bound. A two-sided spec gives two rows: a floor row with sign −1 and a ceiling row with sign +1.
  - Headroom ≤ 0 means infeasible before layout (LAMP-49).
- **Beats hand layout because:** a hand layout designer budgets parasitics against the nominal value or by feel. Automation spends exactly the corner-safe margin on every spec.
- **Philis status:** partial.
  - `budget_rows` measures headroom at the nominal schematic point only (`frontend/library/src/perf.rs:239-245`).
  - With both bounds set, `match (spec.min, spec.max) { (Some(lo), _) => … }` keeps only the floor and drops the ceiling (`perf.rs:241-244`).
  - No process spread is used.

### LAMP-02 Linear degradation model over every parasitic class
- **Kind:** formula / data-model
- **Statement:**
  - `ΔP_j,lay = Σ_{i=1..N_x} S_xi^Pj · x_i`, with `S_xi^Pj = ∂P_j/∂x_i` (2.12–2.13).
  - Valid because layout keeps each P near its nominal value (small perturbations).
  - The parasitic vector x includes:
    - ground C per net;
    - coupling C per net pair (4.22);
    - series R per net segment (4.21);
    - ΔV_T and Δβ per matched pair (4.25);
    - distance per matched pair (4.27);
    - ΔT per matched pair (4.32);
    - junction area and perimeter via terminal C (4.16).
- **Source:** §2.4.1 eqs 2.12–2.13; L1601–1620; PDF 42. Also eqs 4.22, 4.25, 4.27, 4.32.
- **Philis stage:** annotator, flow, gp, dp, gr, dr.
- **Automation recipe:**
  - Extend `perf::Sensitivity::d_per_af` (per spec × net, ground C only) to a per-spec vector over typed parasitics: `Ground(net)`, `Couple(net, net)`, `Series(net)`, `PairVt(k)`, `PairBeta(k)`, `PairDist(k)`, `PairDT(k)`.
  - Each placement or routing tier evaluates Σ S·x from its own estimator.
- **Beats hand layout because:** a human cannot hold N_p × N_x trade-offs in mind. A linear model makes every placement move priced in spec units.
- **Philis status:** partial.
  - Only ground C sensitivities (`perf.rs:189-223`) feed a linear row (`kernel/analog/src/routing/performance.rs:8-17`).
  - Mismatch and thermal limits are per-pair constants, not sensitivities (`backend/annotator/src/emit.rs:83-100`).

### LAMP-03 Sensitivity generation: perturbation, direct, adjoint
- **Kind:** algorithm
- **Statement:**
  - Perturbation (2.15): solve once per parameter. The finite difference becomes unstable when Δh is tiny, and the cost is N_x + 1 solves.
  - Direct (2.16–2.17): `M·∂x⁰/∂h = −∂f/∂h`, reusing the LU factors of the Newton Jacobian. One back-substitution per parameter.
  - Adjoint (2.18–2.24): solve `Mᵀx' = −d` once. Then `∂φ/∂h = x'ᵀ·∂f/∂h` for every h, so two systems in total per output function φ.
  - All three apply to DC, AC and transient.
  - LAYLA used perturbation because the simulators lacked a robust alternative.
- **Source:** §2.4.2–2.4.2.4; L1623–1742; PDF 42–45.
- **Philis stage:** flow (`library::perf`), annotator.
- **Automation recipe:**
  - Keep perturbation, which is simulator-agnostic, but:
    - use central differences (±Δ) to cancel the first error term;
    - choose Δ per class: ~5–10 % of the expected parasitic, not a fixed 10 fF;
    - add parasitic classes beyond ground C.
  - Coupling: perturb `C(net_a, net_b)` only for candidate pairs (nets whose pins lie within a reach radius after gp), to avoid N².
  - Series R: insert a small series resistor at the driver pin of current-carrying nets.
  - ΔV_T,k: add a DC source ±δ in series with one gate of pair k.
  - Δβ_k: scale the multiplier (m) of one device by 1 ± ε.
  - ΔT_k: set one instance's `temp` / `dtemp` to T ± δ.
  - Run all jobs in parallel threads, as now.
  - If the simulator exposes a native small-signal sensitivity analysis (ngspice documents `.sens`; not verified in this study), use it for AC and DC metrics.
- **Beats hand layout because:** the ranking of which nets and pairs matter is measured, not guessed. Designers typically protect only the nets they already know.
- **Philis status:** partial. There is a one-sided perturbation, ground C only, one run per net in parallel (`perf.rs:200-223`), with Δ = 10 000 aF (`frontend/library/src/lib.rs:216`).

### LAMP-04 Direct performance-driven P&R, with no parasitic-bound generation
- **Kind:** algorithm / flow
- **Statement:**
  - Price every intermediate place or route solution by ΔP computed from its geometry through the sensitivities.
  - Penalize only the part outside the margins. Do not convert specs into one fixed set of per-parasitic bounds.
  - The fixed-bound conversion over-constrains the tools, rejects valid layouts and forces iteration.
  - The direct method either yields a correct layout or flags the specs as infeasible, without iteration.
- **Source:** §2.3.3 and §2.4; L1536–1598; PDF 40–42; Fig. 2.4.
- **Philis stage:** gp, dp, gr, dr, flow.
- **Automation recipe:**
  - Mirror `analog::routing::PerformanceBudget` (`RuleBatch<Routes>`) with a placement-tier `RuleBatch<Layout>`, "PlacementPerfBudget". It uses the same spec rows and weights, but x comes from the placement estimators LAMP-06, LAMP-07, LAMP-08 and LAMP-12.
  - Residual `(used − 1)⁺` goes to Θ (lexicographic, as in dp's gate). `criticality` weights the moves.
- **Beats hand layout because:** a human cannot re-evaluate every spec after every move. The annealer does this thousands of times per second.
- **Philis status:** partial.
  - The routing tier is direct, with shared per-spec margin rows (`kernel/analog/src/routing/performance.rs:8-67`).
  - Placement uses only per-net HPWL weights derived from sensitivities (`backend/gp/src/lib.rs:121-153`, `backend/gp/src/mechanics.rs:114-118`). No spec is evaluated during placement.

### LAMP-05 Berkeley constraint-generation QP (flexibility model)
- **Kind:** algorithm
- **Statement:**
  - Choose bounds `x_j,bound` to maximize `Σ_j f_j` subject to `Σ_j S_j·x_j,bound ≤ ΔP_lay` for every spec.
  - Flexibility of a bound: `f_j = a_j + b_j·x + c_j·x²` (2.8), with `c_j = −1/(x_j,max − x_j,min)²` (2.9), `b_j = −2c_j·x_j,max` (2.10), `a_j = −b_j·x_j,min − c_j·x_j,min²` (2.11).
  - So f = 0 at x_min and f is maximal at x_max.
  - Example given: < 0.1 pF on a wire is "almost impossible".
  - The book rejects this method for driving P&R (LAMP-04). It is still the right tool where per-net caps are structurally needed.
  - How x_min and x_max are estimated is not given.
- **Source:** §2.3.2.1 eqs 2.8–2.11, Fig. 2.3; L1467–1515; PDF 39–40. Critique at L1557–1572.
- **Philis stage:** annotator (per-net `c_budget_af`, `max_coupling_af`), gr.
- **Automation recipe:**
  - Only for consumers that need a per-net number (net-class caps, `CouplingBudget` per-pair caps, gr pruning).
  - Take x_min = the MST lower-bound C at the current placement (LAMP-06) and x_max = the C at k·HPWL (calibrate k).
  - Solve the separable concave QP with linear constraints. N ≤ a few hundred nets, so an active-set method or KKT water-filling is enough.
  - Re-solve after each placement epoch. The shared rows (LAMP-04) stay the judge.
- **Beats hand layout because:** caps are derived from specs instead of from class folklore.
- **Philis status:** missing. `c_budget_af` is a class heuristic (`backend/annotator/src/classify.rs:18-29`), acknowledged at `backend/gp/src/lib.rs:131-136`.

### LAMP-06 Per-spec penalty term and total performance cost
- **Kind:** formula
- **Statement:**
  - `C_perfDegr,i` equals:
    - `ΔP_lay,i^min − ΔP_lay,i` when `ΔP_lay,i < ΔP^min`;
    - `0` when `ΔP^min ≤ ΔP_lay,i ≤ ΔP^max`;
    - `ΔP_lay,i − ΔP^max` when `ΔP_lay,i > ΔP^max` (4.11).
  - The total is `C_perfDegr = Σ_i C_perfDegr,i` (4.12).
  - The penalty is linear in the violation.
- **Source:** §4.9 eqs 4.9–4.12; L4460–4499; PDF 107.
- **Philis stage:** dp, gp, gr, dr.
- **Automation recipe:**
  - Normalize each spec by its headroom so that specs in different units sum (Philis already does this: `w_i = ∓(∂f/∂C_i)/headroom`).
  - Keep both sides (LAMP-01).
  - Hard tier: `violations = used > 1`. Budget tier: residual `(used − 1)⁺`.
- **Beats hand layout because:** it tells the tool exactly which spec is being overspent and by how much.
- **Philis status:** partial. `PerformanceBudget::residual = (used − 1)⁺` (`performance.rs:30-45`) is one-sided and routing-tier only.

### LAMP-07 Net topology during placement: MST with edge-to-edge Manhattan distance
- **Kind:** algorithm
- **Statement:**
  - Estimator options:
    - semi-perimeter, O(n);
    - MST, O(n²) (Prim or Kruskal);
    - centre of mass, O(n²);
    - Steiner tree (NP-complete; heuristics O(n log n)–O(n²));
    - source-to-sink, O(n);
    - complete graph, O(n²).
  - LAYLA uses the MST for both length and geometry: the geometry is needed for coupling estimation.
  - The distance metric is Manhattan, edge to edge between terminal rectangles (Cohn 94), which is "the most accurate for analog layouts".
  - Terminals are real rectangles, not device centres.
- **Source:** §4.9.1.1, Figs 4.13–4.14; L4510–4612; PDF 108–110.
- **Philis stage:** gp (analytic, keep HPWL for gradients), dp (SA cost), annotator (lower-bound C).
- **Automation recipe:**
  - In dp's cost (not gp's gradient), per net, compute a Prim MST over pin rectangles with distance `max(0, |dx| − (w_a + w_b)/2) + max(0, |dy| − (h_a + h_b)/2)`.
  - Store the MST segments as L-shapes with a deterministic elbow for LAMP-08.
  - Recompute only the nets touching moved cells (incremental). Nets have ≤ ~20 pins, so O(n²) per net is fine.
- **Beats hand layout because:** it gives an accurate per-net wire estimate for every trial move, not a gut feel for "short enough".
- **Philis status:** missing. gp and dp use pin HPWL (`backend/gp/src/lib.rs:1`, `:224`); no MST was found by grep.

### LAMP-08 Placement-time parasitic extraction and ΔP
- **Kind:** formula / algorithm
- **Statement:**
  - Ground C: `C_x = C_w,x + C_t,x` (4.13), with `C_w = Σ_segments L·W·C_av`. C_av is the area-weighted average cap per unit area over the routing layers (4.14–4.15). `C_t = Σ terminal caps` (4.16–4.17).
  - Coupling: `C_xy = Σ_i Σ_j [L_p,ij·C_c(D_p,ij) + C_ov(A_ov,ij)]` over MST segment pairs that run in parallel (length L_p, gap D_p) or cross (area `A_ov = W_i·W_j`) (4.18–4.20).
  - Series R: `R_x,i = ρ□,av·(L/t_x)/W_wire,i` (4.21), where W comes from the terminal current.
  - Result: `ΔP_j = Σ_k [S_Ck·C_k + Σ_i S_Rk,i·R_k,i + Σ_{i≠k} ½·S_Cki·C_ki]` (4.22). The ½ counts each coupling once from each side.
- **Source:** §4.9.1.2, Fig. 4.15; L4614–4715; PDF 110–112.
- **Philis stage:** dp (cost), annotator (terminal C from generator junction data).
- **Automation recipe:**
  - For each net after LAMP-07:
    - `C_w` = the segment lengths times the net's expected width times the deck's layer-averaged `area_af_um2 + 2·fringe_af_um/W` (the deck already carries `pex` per layer: `kernel/analog/src/routing/stack.rs:12-17`);
    - coupling from segment pairs with `lateral·run/gap` (`stack.rs:253-258`) plus crossings;
    - `C_t` from the macro pin junction area at the operating-point bias.
  - Feed the result into the LAMP-04 placement budget.
  - Cache per-net partial sums and update only the nets of moved cells.
- **Beats hand layout because:** coupling between specific net pairs (e.g. an input to a supply, which kills PSRR, L1783–1785) is priced before any wire exists.
- **Philis status:** missing. No placement-tier C or coupling estimate exists. Coupling is priced only in gr and dr (`backend/gr/src/lib.rs:841-872`).

### LAMP-09 Mismatch: one offset budget shared over all matched pairs, weighted by sensitivity
- **Kind:** formula / algorithm
- **Statement:**
  - `σ²(V_T0) = A_VT0²/(WL) + S_VT0²·D²` (4.23) and `σ²(β) = A_β²/(WL) + S_β²·D²` (4.24).
  - `ΔP_j = Σ_{k=1..m} (|S_ΔVT0,k|·3σ(V_T0)_k + |S_Δβ,k|·3σ(β)_k)` (4.25), summed over the m matched pairs.
  - Split: `ΔP_j = ΔP_j,area + ΔP_j,distance` (4.26). The area term is fixed after sizing. `ΔP_j,distance = Σ_k S_Dk·D_k` (4.27) is recomputed every move.
  - The placer does not get a per-pair distance cap. It trades distance where sensitivity is low.
  - Comparator evidence (Table 4.2, µm / µV·µm⁻¹ / mV):

    | Pair | Distance (µm) | Sensitivity (µV/µm) | Degradation (mV) |
    |---|---|---|---|
    | M1–M2 | 60 | 12 | 0.720 |
    | M3–M5 | 70 | 2.9 | 0.203 |
    | M4–M6 | 70 | 2.9 | 0.203 |
    | M7–M8 | 118 | 0.2 | 0.024 |
    | M11–M12 | 119 | 1.93 | 0.230 |
    | M13–M14 | 38 | 3.8 | 0.145 |
    | M15–M16 | 40 | 3 | 0.120 |
    | **Total** | | | **1.645** |

  - The algorithm "selectively minimizes the distances for the most sensitive transistor pairs".
- **Source:** §4.9.2 eqs 4.23–4.27; L4718–4765; PDF 112–113. Table 4.2 at L5068–5082; PDF 120.
- **Philis stage:** annotator (sensitivities, area term), dp/gp (budget rule), verify (report).
- **Automation recipe:**
  1. Annotator: for each matched pair k, get `S_ΔVT,k` and `S_Δβ,k` on the offset or other metric (LAMP-03). Use analytic input-referred gains (`gm_k/gm_in`, already partly in `bias()` via `gm_us`) when simulation is unavailable.
  2. `ΔP_area = Σ_k 3(|S_VT|·A_VT + |S_β|·A_β)/√(WL)_k`. If `ΔP_area ≥ margin`, flag a sizing infeasibility (LAMP-49).
  3. Emit one `OffsetBudget` `RuleBatch<Layout>`: `Σ_k S_Dk·D_k ≤ margin − ΔP_area`, with `S_Dk = 3(|S_VT|·S_VT,proc + |S_β|·S_β,proc)`.
     - This is the book's linear form. A quadrature form `√(Σ(…)²)` is the exact σ sum if preferred.
     - D_k is the centroid distance, as common-centroid units reduce it to ≈ 0.
  4. Keep the existing per-pair checks as fallback when no sensitivities exist.
- **Beats hand layout because:** a human puts all "matched" pairs as close as possible. The budget lets insensitive pairs (M7–M8 at 0.2 µV/µm) sit 118 µm apart to make room for the ones that matter, which is provably within spec.
- **Philis status:** partial.
  - There is a per-pair Pelgrom allowance `D ≤ η·A/(S·√WL)`. η comes from a per-pair `offset_sigma_mv` or the fixed 0.3 (`backend/annotator/src/emit.rs:46-52`, `:83-90`; `kernel/analog/src/placement/matching_pair.rs:17-28`).
  - There is no circuit-level shared offset budget and no sensitivity weighting.

### LAMP-10 Mismatch model coefficients needed in the deck
- **Kind:** deck-requirement
- **Statement:**
  - `σ²(ΔP) = A_P²/(WL) + S_P²·D²` (2.35), for V_T0, K and β (2.36–2.38).
  - Full β form: `σ²(β)/β² = A_W²/(W²L) + A_L²/(WL²) + A_μ²/(WL) + A_Cox²/(WL) + S_β²·D²`, approximately `A_β²/(WL) + S_β²·D²`.
  - Short-channel V_T (Bastos 95): `σ²(V_T0) = A_1VT²/(WL) + A_2VT²/(WL²) − A_3VT²/(W²L) + S_VT²·D²` (2.39). The 1/√area law fails for short L.
  - These values are "a minimum value of the achievable accuracy". Layout rules (LAMP-11) are needed to reach them.
- **Source:** §2.7.1–2.7.2 eqs 2.35–2.39; L2088–2157; PDF 53–54.
- **Philis stage:** deck, annotator.
- **Automation recipe:**
  - Add per-device-type deck keys `A_VT, S_VT, A_beta, S_beta` (and optionally A1–A3 VT).
  - The annotator selects the (2.39) form when L < L_short (deck key).
  - Missing values stay unknown, never guessed.
- **Beats hand layout because:** matching is budgeted in mV from process data, not by a rule of thumb.
- **Philis status:** partial. A_VT and S_VT are used (`emit.rs:83-90`, `s_over_a` from `svt_uv_per_um`). β and short-channel terms are absent.

### LAMP-11 Matching layout rules as checks
- **Kind:** rule / check
- **Statement:** Matched devices must have:
  - the same structure (e.g. never poly-poly against metal-poly caps);
  - the same temperature (placed on isotherms);
  - the same shape and size: the same finger count; for resistors, the same L, W and number of bends, not just the same square count;
  - common-centroid or 2-axis symmetry. Measured: quad and waffle layouts show no systematic mismatch. Under package stress, finger and interdigitated-finger β fluctuation reaches up to 5× the model (piezoresistive);
  - the same orientation, with current flow parallel and in the same direction (Fig. 2.12);
  - the same surroundings (dummies, Fig. 2.13).
- **Source:** §2.7.2; L2150–2207; PDF 54–55; Figs 2.10–2.13.
- **Philis stage:** cells, annotator, verify.
- **Automation recipe:**
  - Verify-time check per matched group:
    - identical macro variant id, nf and unit count;
    - identical orientation;
    - the same S→D direction (Φ);
    - resistor bends equal;
    - dummies on both ends;
    - identical neighbour layer set within a halo (the same-surroundings check: compare the multiset of foreign shapes within r of each member after mirroring).
  - Prefer a 2-axis (quad) style for pairs flagged as stress-sensitive.
- **Beats hand layout because:** the checks are exhaustive and cover every matched group, not a spot check.
- **Philis status:** mostly implemented.
  - Same current direction: `currents_run_alike` (`frontend/library/src/cellgen.rs:255-262`).
  - Matched devices never rotate: `rotatable` (`backend/dp/src/lib.rs:555-560` (`rotatable` at :558)).
  - Common-centroid rule (`kernel/analog/src/placement/cc.rs`).
  - Dummies (`backend/annotator/src/lib.rs:33`).
  - A resistor-bend equality check and a same-surroundings neighbourhood check were not found.

### LAMP-12 Joint reorientation of matched groups (equal and mirrored orientation groups)
- **Kind:** algorithm (move set)
- **Statement:**
  - An equal-orientation group rotates all members together.
  - A mirrored-orientation group (a symmetric couple) keeps mirror symmetry.
  - Each device's layouts for all orientations are generated at initialization, so a reorientation is a variant pointer switch.
  - Shape groups change the variant of identical devices together.
- **Source:** §4.7 (orientation and shape groups; construction rules 1–3); L4348–4395; PDF 104–105.
- **Philis stage:** dp.
- **Automation recipe:**
  - `try_group_rotate(group)`: apply the same quarter turn to every member about its own centre and swap hw/hh. For a symmetric couple, apply mirrored turns (member b gets the axis-mirrored orientation).
  - Accept through `Sa::trial`.
  - The existing reshape (`try_reshape`) should likewise change all members of a shape group together.
- **Beats hand layout because:** matched sets can still use rotation to shorten wires without breaking equal orientation.
- **Philis status:** partial. Matched devices are frozen in orientation (`backend/dp/src/lib.rs:555-560` (`rotatable` at :558)). There is no group rotate.

### LAMP-13 Thermal: distributed-source profiles plus superposition and a sensitivity-weighted ΔT budget
- **Kind:** algorithm / formula
- **Statement:**
  - Chip model: N layers with conductivity k_j and thickness L_j, isothermal bottom, adiabatic top.
  - Surface T of a w×h source with flux Q₀: `T(x, y) = (4Q₀/k_N)·Σ_n Σ_m τ_N(n, m)·[sin(nπw/L_x)/((1+δ_n0)nπ)]·[sin(mπh/L_y)/((1+δ_m0)mπ)]·cos(nπx/L_x)·cos(mπy/L_y)` (2.46/4.28).
  - Coefficients: `τ₁ = tanh(γL₁)`; `τ_N = [k_{N−1}·tanh(γL_N) + k_N·τ_{N−1}] / [k_{N−1} + k_N·τ_{N−1}·tanh(γL_N)]`; γ from (2.49) (2.47–2.49).
  - Too slow in loops: about 30 s per 2-layer evaluation.
  - Fast scheme:
    1. Unit-source profile on a P×Q grid via DCT, `T = Σ k_nm·cos·cos` (4.29–4.31).
    2. Each variant's active area is fractured into unit sources, and its profile is the shifted sum scaled by power density, precomputed once per variant (Fig. 4.16).
    3. The placement profile is a superposition.
  - Result: `ΔP_j = Σ_k S_ΔTk^Pj·ΔT_k` (4.32).
  - The boundary can be treated as infinite if sources are at least one structure thickness from the die edge.
- **Source:** §2.8.2 eqs 2.40–2.49, L2352–2461, PDF 59–61; §4.9.3 eqs 4.28–4.32, L4768–4871, PDF 113–116.
- **Philis stage:** deck, annotator, dp.
- **Automation recipe:**
  - Deck: add die thickness and layer k values (Si 148 W/m·K is already hard-coded).
  - Precompute a unit profile T_u(r) on a radial or 2-D grid once per run: a 2-layer image-source or Fourier model, or keep `P/(2πkr)` with the finite-thickness correction.
  - Per macro variant, fracture the gate area of each finger (not the bbox) into unit cells weighted by P_finger, giving a per-variant matrix.
  - During the epoch refresh, superpose the matrices shifted to the placements.
  - Replace per-pair `max_delta_mc` with a budget row `Σ_k S_ΔTk·ΔT_k ≤ margin`, with S_ΔTk from LAMP-03.
- **Beats hand layout because:** it gives quantitative isotherm placement for smart-power and class-AB stages. The designer's "keep pairs away from the output stage" becomes an exact trade.
- **Philis status:** partial.
  - Point-source superposition `ΔT = P/(2πkr)` with a per-device floor (`kernel/core/src/thermal.rs:1-40`), refreshed per epoch (`backend/dp/src/lib.rs:345-346`).
  - Per-pair limit derived from `η·σ_rand/TC` (`emit.rs:93-101`) in `ThermalGradient` (`kernel/analog/src/placement/thermal.rs:7-22`).
  - No sensitivity weighting and no distributed sources.

### LAMP-14 Thermal-offset sensitivity by device-temperature simulation; symmetric placement about the heat source
- **Kind:** heuristic / metric
- **Statement:**
  - For a class-AB opamp (140 mW, mostly in output devices M14–M17), simulations with different device temperatures give the offset sensitivity to each matched pair's ΔT.
  - The placer puts the sensitive pairs symmetric about the output transistors.
  - Table 4.6:

    | Pair | Sensitivity (mV/deg) | ΔT (deg) | Degradation (mV) |
    |---|---|---|---|
    | M1a–M1b | 1.6 | 0.153 | 0.245 |
    | M8a–M8b | 3.7 | 0.084 | 0.319 |
    | M13a–M13b | 6.0 | 0.074 | 0.448 |
    | **Total** | | | **1.012** |

  - Offset after placement is 2.3 mV against a 1.3 mV nominal. The spec is "< 5 mV" in the text but "< 3" in Table 4.5, an inconsistency in the source.
  - The thermal profile tool can also score hand layouts interactively.
- **Source:** §4.12.4, Tables 4.5–4.6, Figs 4.24–4.26; L5289–5330; PDF 127–129.
- **Philis stage:** annotator (sensitivity), dp (rule), verify (report).
- **Automation recipe:**
  - Detect hot devices: P_i > p·max(P) from `bias()` power.
  - For each matched pair, add a soft isotherm term that rewards `|d(a, hot) − d(b, hot)| → 0` for each hot source, or simply rely on LAMP-13 ΔT.
  - Report a Table-4.6-style breakdown.
- **Beats hand layout because:** all pairs are protected against all sources at once, with numbers.
- **Philis status:** partial. TC comes from deck dV_T/dT, not from simulation (`emit.rs:93-101`), and there is no per-pair report.

### LAMP-15 Hot-spot reliability rule
- **Kind:** rule
- **Statement:**
  - The failure rate of micro-electronic devices doubles for roughly every 10 °C increase.
  - Hot spots from local dissipation are a long-term reliability concern, in addition to thermally induced performance degradation.
- **Source:** §2.8.1; L2250–2318; PDF 56–57.
- **Philis stage:** verify, dp.
- **Automation recipe:**
  - Report the peak ΔT from the LAMP-13 map.
  - Feed the local conductor temperature into EM derating, which already takes T.
  - Optionally add a soft peak-temperature term that spreads hot devices.
- **Beats hand layout because:** EM limits get derated per wire from the real map, not with a global worst case.
- **Philis status:** partial. EM derating exists (`kernel/analog/src/routing/em.rs:9-22`). Wiring it to the placement thermal map was not verified.

### LAMP-16 Substrate coupling: separation depends on substrate type; guard-ring conditions
- **Kind:** rule / deck-requirement
- **Statement:**
  - Low-impedance substrate (epi on p+ or n+ bulk): the bulk acts as a single node. Beyond 4× the effective epi thickness (≈ 40 µm) more separation gives no improvement.
  - High-impedance (uniform lightly doped) substrate: coupling falls almost linearly with separation, so more distance keeps helping.
  - Guard rings (grounded substrate contacts enclosing a region) are effective only when placed very close to the sensitive circuit and biased through dedicated package pins.
  - Injectors: S/D junction caps, collector–substrate caps, MOM bottom plates, wells at varying potential, impact ionization.
  - Receivers: capacitive coupling, and the body effect, which matters even at low frequency.
  - A resistive substrate model is adequate up to 2 GHz. At 4–5 GHz and above it needs Maxwell's equations.
  - Modelling: an n-port resistive network, or Genderen's common substrate node plus neighbour-only direct resistances.
- **Source:** §2.9–2.9.3.2; L2476–2604; PDF 61–64; Figs 2.15–2.16.
- **Philis stage:** deck, annotator, gp, dp, cells.
- **Automation recipe:**
  - Deck: `substrate: {kind: epi|bulk, t_epi_nm}`.
  - Epi: keep the threshold rule `d ≥ 4·t_epi` as hard or budget.
  - Bulk: a separation cost that is linear in distance (reward) between noisy and sensitive devices, with no threshold.
  - Guard rings around sensitive devices: minimum ring-to-device gap, and the ring net must be a dedicated pin (an ERC check that the ring net is not shared with noisy returns).
- **Beats hand layout because:** the correct rule is applied for the actual substrate. A fixed rule of thumb is wrong for one of the two substrate types.
- **Philis status:** partial.
  - The epi rule exists: `ISOLATION_EPI_MULTIPLE = 4` (`backend/annotator/src/emit.rs:250-274`; `kernel/analog/src/placement/isolation.rs:7`).
  - There is no high-impedance-substrate mode and no dedicated-pin check for guard rings (not found by grep).

### LAMP-17 Interconnect equivalent-circuit validity checks (RC vs RLC)
- **Kind:** check / data-model
- **Statement:**
  - Exact T/Π models use `Z₀ = √((R₀ + jωL₀)/(G₀ + jωC₀))` and `γ = √((R₀ + jωL₀)(G₀ + jωC₀))` (2.25–2.26).
  - For |γD| ≪ 1, series impedance ≈ (R₀ + jωL₀)D/2 and shunt ≈ 1/((G₀ + jωC₀)D) (2.27–2.28), so the segment is lumped.
  - L and G can be dropped when G₀ ≪ ωC₀ and ωL₀ ≪ R₀. The book's typical values at ω = 2π·100 MHz are L₀ = 10⁻¹⁵, R₀ = 0.01, G₀ = 10⁻¹², C₀ = 10⁻¹⁶; the units are garbled in the text.
  - LAYLA's model: an n-terminal net has n series R, one C to ground and a C to every other node.
  - The future-work section says RF needs self and mutual wire inductance (L7082–7093).
- **Source:** §2.5.2, Figs 2.6–2.7, eqs 2.25–2.28; L1799–1850; PDF 46–48. §7, L7082–7093.
- **Philis stage:** verify, annotator.
- **Automation recipe:**
  - For nets the annotator marks as RF or high-frequency (a spec with frequency f), compute per routed segment `ωL₀·len` vs `R₀·len`, using the per-layer L₀ from the deck, and `|γ|·len`.
  - Warn when either ratio exceeds 0.1. Inductance extraction is then required, and the RC score is not trustworthy.
- **Beats hand layout because:** it states explicitly when the parasitic model stops being valid.
- **Philis status:** RC with a coupling matrix is implemented (`frontend/library/src/perf.rs:1-8`). Wire inductance and the validity check are missing.

### LAMP-18 Geometric capacitance and resistance models for inner loops
- **Kind:** formula / deck-requirement
- **Statement:**
  - Area: `C_A = (ε/d)·A_xy` (2.29).
  - Lateral: `C_L = F(d)·l_xy` with `F(d) = C₀ + C₁/d + C₂/d² + C₃/d³ + C₄/d⁴`, fitted per technology (2.30–2.31).
  - Fringe to another layer: `C_F = C_F0·l_xy·(e^{−x₂/x₀} − e^{−x₁/x₀})` (2.32), where x₂ and x₁ are the distances to the near and far edges of the other conductor. C_F0 and x₀ are fitted.
  - Resistance: decompose into rectangles along equipotentials, `R_i = ρ□·l_i/w_i` (2.33), then series and parallel combination.
  - Numerical field solvers are accurate but too slow for layout loops.
- **Source:** §2.5.3.1–2.5.3.2; L1892–2008; PDF 49–51; Fig. 2.8.
- **Philis stage:** deck, gr, dr, dp.
- **Automation recipe:**
  - Deck `pex` per layer: add `lateral_poly: [C0..C4]` and `fringe_to: {layer: (CF0, x0)}`, fitted offline to foundry PEX tables.
  - `LayerStack::lateral_af` becomes `F(gap)·run` when present, else the current `ε·t/gap`.
  - Cross-layer fringe uses (2.32) for over and under neighbours.
- **Beats hand layout because:** coupling on close tracks is right to within the fit, instead of a 1/d idealization.
- **Philis status:** partial.
  - Area plus fringe ground C per perimeter (`kernel/analog/src/routing/stack.rs:44-56`).
  - Lateral `ε0·k·t·run/gap` (`stack.rs:15-17`, `:253-258`).
  - No F(d) polynomial and no exponential inter-layer fringe.

### LAMP-19 Junction capacitance priced on sensitive nets (folding, merging, abutment)
- **Kind:** formula / heuristic
- **Statement:**
  - `C_jSBt = A_s·C_j/(1 − V/φ_j)^{m_j} + P_s·C_jsw/(1 − V/φ_j)^{m_jsw}` (2.34 = 3.1).
  - It is reduced by folding (a shared S/D between two gates), merging diffusion between electrically connected devices, and abutment.
  - Two MOS devices with a common S/D node can share diffusion iff they are the same type with the same bulk potential. Series devices share one node; parallel devices share two.
  - Gate area C is fixed by sizing and cannot be reduced in layout.
- **Source:** §2.6, Fig. 2.9, L2011–2062, PDF 51–52; §3.2, Fig. 3.2, L2725–2747, PDF 67–68.
- **Philis stage:** cells, annotator, dp.
- **Automation recipe:**
  - For each MOS variant, the generator reports per-terminal (A_s, P_s). The annotator evaluates `C_j` at the operating-point junction voltage (`oppoint`).
  - dp adds `S_C(net)·C_t(net)` via LAMP-08, so a variant with the drain on the inside (even nf) wins when the drain net is sensitive.
- **Beats hand layout because:** the variant choice per device is priced by the actual node sensitivity.
- **Philis status:** partial.
  - Fingers share S/D, and series stacks are drawn as chains with shared junctions (`frontend/library/src/cellgen.rs:94-110`; `kernel/cells/src/mosfet.rs:928`).
  - There is a merged vs apart solve for matched pairs (`frontend/library/src/lib.rs:169-181`).
  - Junction C is not in any placement cost term.

### LAMP-20 Dynamic diffusion merging in placement (legal overlap)
- **Kind:** algorithm / rule
- **Statement:**
  - Terminal overlap is legal iff the terminals are on the same net, on the same layer, and their devices have the same bulk potential (Cohn 91).
  - Legal overlap lowers the net's junction C in proportion to the overlap area and perimeter, which lowers the cost. It is "promoted specifically for sensitive nets".
  - Illegal overlap must reach zero.
  - Overlap is kept as a cost term, not forbidden in the move set, so that beneficial overlaps can be found.
  - Some structures still need dedicated generators: interdigitated cascodes and common-centroid pairs.
- **Source:** §3.3.2, §3.3.4, L2792–2850, PDF 69–70; §4.5.2, L3892–3908, PDF 96; §4.6 overlap, L4117–4124.
- **Philis stage:** dp, cells, verify.
- **Automation recipe:**
  - Give each MOS macro variant "edge diffusion" pins (the S/D stripe on each end: layer diff, net, bulk net).
  - In dp, an abutment move snaps device b so that its edge diffusion coincides with a's. It is allowed only if the nets, layer and bulk are equal and both edges carry the dummy/diffusion-end rule compatibility.
  - Credit `S_C(net)·ΔC_j(overlap)`.
  - The legalizer treats such overlap as legal. cellgen draws the merged diffusion.
- **Beats hand layout because:** every sharing opportunity is evaluated by the node's sensitivity, not only the obvious ones.
- **Philis status:** missing. Every overlap is a defect (`backend/dp/src/legalize.rs:2`). Merging happens only pre-placement in cellgen for grouped members.

### LAMP-21 Transistor stacking by Euler trail (Basaran 96), linear time
- **Kind:** algorithm
- **Statement:**
  1. Build a diffusion graph G(V = nets, E = transistors linking their S and D nets).
  2. Split it into subgraphs of the same type and the same bulk net.
  3. Fold large transistors into parallel fingers.
  4. Split into connected subgraphs with equal channel width.
  5. Per subgraph: if it is not Eulerian (connected with all degrees even), add a supervertex joined to every odd-degree vertex. Run an Euler-trail algorithm that honours symmetry and matching and uses a constraint-driven cost.
  - Every added edge becomes a diffusion gap. The method is linear in the number of transistors and gives the minimum-cost stack set for a class of circuits.
  - The Malavasi/Wimer alternative enumerates all paths by DP and then finds cliques; it is exponential.
- **Source:** §3.4, Fig. 3.3; L2854–2915; PDF 70–72.
- **Philis stage:** annotator, cellgen.
- **Automation recipe:**
  - Hierholzer per subgraph. Edge weights: prefer continuing through nets with high S_C (a merge saves their junction C).
  - For symmetric pairs, run on the half-graph and mirror, requiring that mirrored edges get mirrored positions in the trail.
  - Output: chains that cellgen already knows how to draw (`series_order`).
  - Cost: `Σ S_C·C_j` saved minus a gap penalty.
- **Beats hand layout because:** it finds the optimal sharing over the whole circuit in O(E), which is hard by eye beyond about 10 devices.
- **Philis status:** missing. Series-chain recognition exists only for grouped members (`frontend/library/src/cellgen.rs:103-106`, `series_order`; `backend/annotator/src/block.rs:30-52` Stack). There is no Euler partition.

### LAMP-22 MOS generator: slices, bulk-contact distance, current-sized straps, gate-R-aware source routing
- **Kind:** formula / rule
- **Statement:**
  - Fingers: fold the device into nf parallel parts.
  - Slices: no part of the transistor may be more than a specified distance (a foundry or designer rule) from a bulk contact. Very wide devices are split into slices with rows of bulk contacts between them.
  - Current: if the current exceeds a process threshold, widen the S/D wires and add vias:
    - metal2 slice connector `W₂ = I/I_max,2` (3.2), with I_max in A/m;
    - metal1 finger connector `W₁ = I/(n_s·I_max,1)` (3.3);
    - `N_via = I/(n_s·I_max,via12)` (3.4);
    - metal1 on diffusion `W₁₁ = I/(n_s·⌊n_f/2⌋·I_max,1)` (3.5).
  - Aspect ratio: derive nf and slices from a target aspect ratio under the bulk-contact and high-current rules.
  - Wide source wires lengthen the gate wires, which adds gate R (noise, RC). Routing the source in metal2 lets the gate sit close to the active area.
- **Source:** §3.7.1.2–3.7.1.3, Figs 3.5–3.7, eqs 3.2–3.5; L3081–3211; PDF 75–78.
- **Philis stage:** cells, annotator.
- **Automation recipe:**
  - Pass per-terminal DC current from `bias()` into the MOS generator. Size the internal S/D straps and cut arrays with (3.2–3.5) using the deck `max_density` and `max_current_per_cut`.
  - Enforce the max distance to a tap per finger.
  - Choose a source-strap layer that keeps the gate lead short when the finger poly R check trips.
- **Beats hand layout because:** EM-correct device-internal straps are produced by construction on every device.
- **Philis status:** partial.
  - Tap strip (`kernel/cells/src/mosfet.rs:567`); `tap_pitch_nm: 2_000` (`backend/annotator/src/constraints.rs:79`).
  - Finger poly-R check (`mosfet.rs:58`).
  - EM sizing exists only in routing (`backend/dr/src/lib.rs:149`, `:663`). No current-sized internal straps were found.

### LAMP-23 Dedicated cascode-pair generator (contactless shared node)
- **Kind:** rule / cells
- **Statement:**
  - Two MOS devices connected source-to-drain in cascode need no contacts on the shared S/D, so the poly gates can sit at minimum distance.
  - This substantially reduces the internal node C.
  - This interdigitated structure cannot be formed by merging during placement.
- **Source:** §3.7.2, Fig. 3.8; L3237–3244; PDF 81.
- **Philis stage:** cells.
- **Automation recipe:** in the series-chain generator, when the internal node has no external connection (only the two devices), emit it contactless at min poly-to-poly spacing over diffusion.
- **Beats hand layout because:** the minimum-C cascode node is applied every time it is legal.
- **Philis status:** partial / unknown. The chain shares the junction (`kernel/cells/src/mosfet.rs:928`). Whether the internal contact is omitted was not verified.

### LAMP-24 Technology independence of generators
- **Kind:** data-model
- **Statement:**
  - Generators are written in symbolic geometric parameters (min width, spacing, enclosure) and electrical parameters (sheet R, cap per area, max current density), plus device definitions that map generic layers to process layers, with 'void' for absent layers.
  - Result: 20+ processes from 5 foundries, about 1 hour per port, with no source changes.
- **Source:** §3.6, §3.7, Example 3.1; L3000–3058; PDF 73–75.
- **Philis stage:** deck, cells.
- **Automation recipe:** keep all rules in `pdks/*.json`, and treat any literal nm in the generators as a bug. Add a porting test: build every generator variant on every deck.
- **Beats hand layout because:** one generator library runs on every deck.
- **Philis status:** implemented in design (deck JSONs, `kernel/macroMaster`). Hard-coded constants remain, e.g. `tap_pitch_nm: 2_000` at `backend/annotator/src/constraints.rs:79`.

### LAMP-25 Generator interface mode and layout mode, with variants selected by the placer
- **Kind:** data-model
- **Statement:**
  - Interface mode returns, for every possible variant, only what the placer needs: bbox or cover plus terminal geometry.
  - Layout mode draws the selected variant.
  - Variants and shapes are chosen during placement, not before.
- **Source:** §1.6.2, L1179–1187; §3.2, L2661–2684; §3.5, L2973–2997; PDF 32, 66, 73.
- **Philis stage:** cells, gp, dp.
- **Automation recipe:** keep `VariantSpace`, plus per-variant electrical side data (junction A and P per pin, thermal matrix: LAMP-13, LAMP-19) so that reshape moves are priced electrically.
- **Beats hand layout because:** variants are chosen jointly with placement.
- **Philis status:** implemented (`gp::VariantSpace`; `try_reshape`, `backend/dp/src/lib.rs:584`). Electrical side data per variant is missing.

### LAMP-26 Placement constraint semantics (symmetry, matching, ratio)
- **Kind:** rule
- **Statement:**
  - A couple is placed symmetric about an axis, with identical variants and mirrored orientations.
  - A self-symmetric device sits on the axis.
  - Members of one symmetry group share one axis. There can be several groups.
  - A matching group has equal orientations. At 1:1 the variants are equal. At other ratios the members are built from equal unit devices according to the ratio.
  - The distance between matched devices is chosen by its performance effect (LAMP-09).
- **Source:** §4.2, Figs 4.1–4.2, L3316–3354, PDF 83–84; §4.6, L4068–4090, PDF 100.
- **Philis stage:** annotator, dp.
- **Automation recipe:** already modelled. Make sure ratioed matching groups always unitize (integer units) and that the couple's mirrored orientation is enforced in the variant choice.
- **Beats hand layout because:** these constraints hold exactly by construction.
- **Philis status:** implemented. Symmetry groups (`backend/dp/src/lib.rs:442`), projection (`:371-400`), unitization (`PLAN.md:103`), and a symmetry-group comment citing Lampaert (`backend/annotator/src/emit.rs:200`).

### LAMP-27 Hard constraints by construction; overlap and performance as penalties
- **Kind:** heuristic
- **Statement:**
  - Hard constraints are best enforced in the move set. The price is a more complex move set, but no CPU is wasted on illegal states.
  - A constraint in the cost is traded off and is not guaranteed to reach zero.
  - Performance cannot be maintained by construction, so it must be a penalty, and special care must drive it to zero.
  - Hard geometry (max height, standard-cell rails) goes in the move set. Soft geometry (target aspect ratio) goes in the cost.
  - Overlap stays soft so that beneficial overlap can be explored.
- **Source:** §4.6; L4033–4129; PDF 99–101.
- **Philis stage:** dp.
- **Automation recipe:** keep Philis's lexicographic gate (violations ≺ Φ ≺ Θ ≺ PEX). Put performance rows in the Θ tier with violation counting when a spec is exceeded (not only a residual), so they dominate PEX.
- **Beats hand layout because:** hard constraints are never traded away.
- **Philis status:** implemented through a different mechanism: projection of hard batches plus a lexicographic gate (`backend/dp/src/lib.rs:1-8`, `:371-406`).

### LAMP-28 Symmetric move set: translate, swap along the axis, flip, axis shift
- **Kind:** algorithm
- **Statement:**
  - Independent group: translate one device to a random position, or swap the centres of two devices.
  - Symmetry group:
    - symmetric translation: a couple moves while staying mirrored, and a self-symmetric device slides on the axis;
    - symmetric swap: two symmetric units exchange their coordinates along the axis direction, and their coordinates perpendicular to the axis stay the same;
    - symmetric flip: the two members of a couple exchange centres;
    - symmetric shift: move the whole axis perpendicular to itself, needed when there are several groups.
- **Source:** §4.7, Figs 4.9–4.10; L4199–4346; PDF 102–104.
- **Philis stage:** dp.
- **Automation recipe:**
  - Add `try_sym_swap(g, u1, u2)`, which exchanges the along-axis coordinate of two units of group g.
  - Add a self-symmetric slide.
  - Check that `try_pair_swap` covers the flip.
  - Keep `try_group_shift` (axis shift).
- **Beats hand layout because:** the search explores symmetric orderings exhaustively.
- **Philis status:** partial. There are group shift, pair expand and pair swap (`backend/dp/src/lib.rs:311-326`, `:473-535`). The along-axis swap of two units was not found.

### LAMP-29 Cost function with scheduled weights, aspect ratio and geometric constraints
- **Kind:** formula / heuristic
- **Statement:**
  - `C = α·C_area + β·C_aspectRatio + γ·C_overlap + δ·C_perfDegr` (4.6).
  - `C_area` is the bbox area. `C_AR = |AR − AR_desired|` (4.7). `C_overlap = Σ_{i<j} overlap_area_ij` (4.8).
  - After each inner loop, the performance and AR weights decrease linearly from max to min and the overlap weight increases from min to max: configuration first, legality last.
  - A floorplanner or user may impose a target aspect ratio, a fixed height and fixed terminal positions (L3435–3440, L4103–4115).
- **Source:** §4.8, L4398–4457, PDF 106–107; §4.2 geometric constraints, L3435–3440, PDF 86.
- **Philis stage:** dp, gp, flow.
- **Automation recipe:**
  - Add `cfg.aspect_ratio: Option<f32>` (soft budget row `|AR − AR*|/AR*`) and `cfg.max_height_nm` (hard clamp of the move region).
  - Add optional fixed pin sides for I/O nets (pin cells pinned in `fixed`).
- **Beats hand layout because:** blocks meet top-level floorplan constraints without manual reshaping.
- **Philis status:** partial. There are no aspect-ratio or height terms in gp or dp (no grep hits for aspect or height in `backend/gp`, `backend/dp`). Overlap is handled by the gate plus the legalizer.

### LAMP-30 Flat representation and a device model of covers and terminal partitions
- **Kind:** data-model
- **Statement:**
  - A flat (absolute-coordinate) representation beats slicing for analog. Symmetry lives in the move set, parasitics need absolute coordinates, non-slicing topologies are denser (especially with widely varying sizes), and overlap is exploitable.
  - Device: minimum overlapping cover (MOC; general NP-hard, O(n⁴) approximation after Wu 90) plus a cover bbox prefilter, the bulk net and a thermal profile.
  - Terminal: a disjoint rectangle partition (a MOC would overcount C), bbox, layer and net.
  - Terminals are never reduced to points.
- **Source:** §4.5.1–4.5.2, Figs 4.5–4.7; L3705–3941; PDF 92–97.
- **Philis stage:** gp, dp.
- **Automation recipe:**
  - Keep flat.
  - Add optional multi-rectangle covers for L- or U-shaped macros (rings, cap arrays with a tap corner), so that overlap and legalization stop reserving the full bbox.
  - Use pin rectangles, not centres, in LAMP-07.
- **Beats hand layout because:** L- and U-shaped devices can nest tightly.
- **Philis status:** flat is implemented (`Layout` x/y with hw/hh). Devices are single bboxes. Multi-rectangle covers are missing.

### LAMP-31 Per-edge routing-space allocation from terminal currents and net geometry
- **Kind:** formula / algorithm
- **Statement:**
  - Each device's cover rectangles are expanded outward per side x ∈ {l, b, r, t}: `Δint_x = Δint_x,st + Δint_x,d` (4.4/4.33).
  - Static part: `Δint_x,st = Σ_{i=1..T_x} W_i` (4.5), summed over the terminals on side x. W_i is sized from terminal current. It is fixed during placement.
  - Dynamic part, option 1: `k₁ − k₂·√((x_e − x_c)² + (y_e − y_c)²)` relative to the placement centre (4.34).
  - Dynamic part, option 2: `Σ_{i=1..N} (k₁,i − k₂,i·√((x_e − x_c,i)² + (y_e − y_c,i)²))` relative to each net's pin-bbox centre, with k from the net's wire width (4.35). Option 2 is "consistently and significantly better".
  - Numeric k values are not given.
  - Too little space makes the layout unroutable. Too much loses density.
- **Source:** §4.5.3, Fig. 4.8, L3944–4030, PDF 97–99; §4.10, L4874–4910, PDF 116.
- **Philis stage:** gp, dp, flow.
- **Automation recipe:**
  - Per macro side, `halo_st = Σ` (EM width from `em::Limit::width_nm` + spacing) over the pins on that side.
  - Per epoch, `halo_d = Σ_nets max(0, k₁,i − k₂,i·dist(edge centre, net bbox centre))`.
  - Clamp `halo_d ≥ 0` (the book's form can go negative far away).
  - Calibrate k₁ and k₂ per deck from gr overflow in previous epochs: fit k so that the predicted demand matches the gcell overflow.
  - Use the halos as the per-side clearance in overlap and legalize instead of the single `clearance`.
- **Beats hand layout because:** it attacks the 10–15 % area penalty that LAYLA itself measured against manual layout (L7094–7106, LAMP-47).
- **Philis status:** missing. There is one uniform `clearance` (`backend/gp/src/lib.rs:155-162`), gp `UTILIZATION = 0.4` (`:167`), and only guard-ring halos are reserved (`frontend/library/src/lib.rs:1195-1199`).

### LAMP-32 Annealing schedule (Otten T₀, Catthoor chain length, adaptive α)
- **Kind:** algorithm
- **Statement:**
  - `P₀ = exp(−ΔC⁺/T₀)`, so `T₀ = −ΔC⁺/ln P₀` (4.37–4.38). ΔC⁺ is the mean cost increase measured over random moves. P₀ defaults to 0.6.
  - Stop when the last costs of consecutive Markov chains lie within a user interval for a user number of chains.
  - Chain length L_k: stop the chain when the running estimate of the denominator of the stationary distribution `q_i(T) = exp(−(C_i − C_opt)/T)/Σ_j exp(−(C_j − C_opt)/T)` (4.39) stops changing (Catthoor 88).
  - Cooling `T_{k+1} = αT_k` (4.40), with α ∈ [0.8, 0.95]. A long chain signals a critical region and uses a high α; a short chain uses a small α.
  - Convergence theory: infinite chains, conditions on the A and G matrices, and T → 0 (4.36).
- **Source:** §4.11 eqs 4.36–4.40; L4913–5017; PDF 117–119.
- **Philis stage:** dp.
- **Automation recipe:** what remains untested is the adaptive-α part: α = 0.95 when an epoch's accept count needs the full `MOVES_PER_CELL·n` to settle, and 0.8 when the cost moves little. Measure on ota, rc_filter and bjt_mirror over seeds, as the earlier test did.
- **Beats hand layout because:** adaptive cooling spends effort where the landscape is critical.
- **Philis status:**
  - Evaluated and not adopted. The fixed schedule is `ALPHA = 0.93` and `MAX_ITERS = 220`, with `T₀ = 0.02·mean|ΔPEX|` (`backend/dp/src/lib.rs:21-27`, `:276-289`).
  - The comment records that Lampaert's `T₀ = −ΔC⁺/ln 0.6` with a 10-flat-chains stop gave "ota C −12 % but rc_filter C +7 %, bjt_mirror area +22 %; not adopted" (`:296-303`).
  - The adaptive-α and quasi-equilibrium chain length are untried.

### LAMP-33 A/B protocol: performance-driven vs not
- **Kind:** metric
- **Statement:**
  - Comparator (delay spec < 5 ns, offset spec < 5 mV): performance-driven placement gave offset 3.7 mV and delay 2.8 ns. The same tool with the performance mechanism off gave 6.9 mV and 5.4 ns, violating both specs.
  - CPU 106 s vs 93 s (SPARC 10).
  - "Nominal" values are from sizing without layout parasitics.
- **Source:** §4.12.1, Tables 4.1–4.2, Figs 4.17–4.19; L5020–5082; PDF 119–120.
- **Philis stage:** benchmarks.
- **Automation recipe:**
  - For every fixture with `performance` specs, run the flow twice: with performance rows and with rows empty (HPWL weights 1).
  - Report per-spec measured values, misses, area and CPU.
  - Record the delta as the headline "constraint-awareness gain".
- **Beats hand layout because:** it gives quantitative proof that performance-awareness pays.
- **Philis status:** unknown. `benchmarks/src/bench.rs` was not audited for a perf on/off A/B.

### LAMP-34 Reserve margin for routing at placement time
- **Kind:** heuristic
- **Statement:**
  - After placement, "the remaining performance margins … are needed for the subsequent routing phase (some parasitics may be different than estimated during placement)".
  - Opamp1 (Table 4.3): GBW > 225 MHz, nominal 228, after placement 226. PM > 60°, 61 → 60.2. Offset < 5 mV, 4.5 → 4.8. Av and slew unchanged.
  - Opamp2 (Table 4.4): Av > 100 dB, 107 → 104; GBW > 200 MHz, 205 → 202; PM > 70°, 77 → 74; CMRR 78 dB and PSRR 86 dB, with ∞ nominal.
- **Source:** §4.12.2–4.12.3, Tables 4.3–4.4; L5193–5274; PDF 123–125.
- **Philis stage:** dp, flow.
- **Automation recipe:**
  - Placement-tier rows (LAMP-04) use `budget_place = (1 − r)·margin`, with r the fraction reserved for routing.
  - Calibrate r per spec from the ratio of routed ΔP to estimated ΔP over past epochs. Default r = 0.2 mirrors the existing thermal margin.
- **Beats hand layout because:** routing starts with a guaranteed margin instead of whatever is left over.
- **Philis status:** partial. `THERMAL_MARGIN_PCT = 20` (`backend/annotator/src/emit.rs:43-44`) covers thermal only. There are no performance rows at placement to reserve from.

### LAMP-35 Degradation attribution report
- **Kind:** metric
- **Statement:** The placer outputs the final layout plus "information about the performance degradation in this final layout and an identification of the most important contributions to this degradation", so the designer can see failing specs and the critical effects.
- **Source:** §4.3; L3466–3471; PDF 87. Tables 4.2 and 4.6 are examples.
- **Philis stage:** verify, flow (metadata).
- **Automation recipe:** for every spec j, emit a sorted list of contributions:
  - per net k, `ΔP_j^k = Σ S·p` (5.13);
  - per coupling pair;
  - per matched pair, distance term (Table 4.2 style);
  - per pair, ΔT term (Table 4.6 style).
  - Each entry carries its share of the headroom. Write it to the metadata JSON and to the visualizer.
- **Beats hand layout because:** it explains exactly why a spec failed. Human post-layout debugging has "no clue which parasitics are responsible" (L1254–1257).
- **Philis status:** partial. Metadata stores per-spec measured values and the miss (`frontend/library/src/lib.rs:432-437`). There is no per-parasitic attribution.

### LAMP-36 Routing model: area routing, gridless, variable width
- **Kind:** heuristic / data-model
- **Statement:**
  - For analog cells with about 10–20 nets, single-phase area routing (net by net over the whole area) is preferred. Global/detailed decomposition "loses the global view", which hampers symmetry, matching and crosstalk control.
  - A gridded model wastes area (the pitch is the worst case over all layers), cannot do variable width and forces terminals onto the grid.
  - A reserved-layer model (e.g. VH) restricts the solution space.
  - LAYLA therefore uses gridless routing with unreserved layers.
- **Source:** §5.4.1–5.4.2, Fig. 5.2; L5511–5590; PDF 133–135.
- **Philis stage:** gr, dr.
- **Automation recipe:** a gridless rewrite is not needed. Instead:
  - reserve the EM width during the search (block neighbour tracks when the width exceeds one track) rather than fattening afterwards;
  - allow off-lattice pin access, which the L-shaped jogs already do;
  - keep gr, but run dr over the whole die (already true) with symmetric pairs co-routed (LAMP-40).
- **Beats hand layout because:** wide wires are never squeezed into gaps that were planned for minimum width.
- **Philis status:** different design.
  - Two-phase gr + dr on a capacity-1 track lattice (`backend/dr/src/lib.rs:1-12`).
  - Width is fattened after the search: "R is at the drawn wire width (fattening happens after the search)" (`backend/gr/src/lib.rs:852-853`).

### LAMP-37 Current-driven wire width and via arrays per segment
- **Kind:** formula
- **Statement:**
  - `W_i = I/I_max,i` (5.1), with I_max,i in A/m of layer i.
  - `N_via = I/I_max,ij` (5.2).
  - Via cell width `W_ij = [⌈√N_via⌉ − 1]·viaSeparation_ij + 2·max(viaOverlap_i, viaOverlap_j)` (5.3), for a square array. As printed, the formula has no cut-size term. Read viaSeparation as the cut pitch.
  - Width is a property of a segment, not of the whole net.
- **Source:** §5.6.2 eqs 5.1–5.3; L5941–5963; PDF 143.
- **Philis stage:** dr.
- **Automation recipe:**
  - Keep per-segment sizing from branch currents.
  - Check that the via-array extent uses ⌈√N⌉ columns at the cut pitch plus 2× the max enclosure of the two layers.
  - Add a test that the drawn via array matches (5.3) for N = 1, 2, 5, 9.
- **Beats hand layout because:** every segment is EM-correct and none is over-widened.
- **Philis status:** implemented. `DetailedCfg::em_width` per segment (`backend/dr/src/lib.rs:149`, `:663`), per-cut current limits (`kernel/analog/src/routing/em.rs:24-30`), and the doc at `dr/src/lib.rs:2356`.

### LAMP-38 A* router with a performance cost and an admissible predictor
- **Kind:** algorithm
- **Statement:**
  - `PathCost(P_n) = PathCost_act(P_n) + PathCost_pred(P_n)` (5.5).
  - `PathCost_act(P_n) = Cost_act(c_n, c_{n−1}) + PathCost_act(P_{n−1})`, with `PathCost_act(P₀) = 0` (5.6–5.7).
  - `PathCost_pred(P_n) = Cost_pred(c_n)` depends only on the head cell (5.8).
  - The segment cost is `Σ_j ΔP_j` over specs (5.11).
  - The predictor applies the same ΔP estimator to a minimum Manhattan path from the head to the target, as in placement §4.9.1.
  - The search returns a minimum-cost path iff the predictor is a lower bound.
  - Partial paths live in a Fibonacci heap: insert O(log n), space O(n).
- **Source:** §5.7–5.7.2 eqs 5.4–5.11; L6065–6196; PDF 146–148. Heap at L6058–6062.
- **Philis stage:** gr, dr.
- **Automation recipe:**
  - In `route_net` / `Dij`, add `h(node) = manhattan_steps(node, nearest target) × min_step_cost`. `min_step_cost` is the base step cost plus the cheapest layer's `weight·layer_c` and zero coupling, which keeps it admissible.
  - Use a bounding-box distance to the target tree for multi-target search.
  - Expected result: fewer expanded nodes at the same optimum.
- **Beats hand layout because:** the router gets the same quality in less time, which frees CPU for more epochs.
- **Philis status:** partial. Dijkstra over non-negative electrical costs, with no predictor (`backend/gr/src/lib.rs:816-848`, `:874-882`).

### LAMP-39 Segment cost from R, ground C and coupling C within a region of influence
- **Kind:** formula
- **Statement:**
  - For a new segment (c_{n−1} → c_n) on layer L_n with length l_n and width w_n: `R_i = ρ□,Ln·l_n/w_n` (5.9).
  - Capacitances are computed within an extension distance d around the segment: overlap (to ground, or coupling when the overlapped shape belongs to another net), lateral and fringe, using Arora 96 models. A larger d is more accurate but slower.
  - `ΔP_j = S_Ci·C_i + S_Ri·R_i + Σ_{k≠i} ½·S_Cki·C_ki` (5.10). `Cost_act = Σ_{j=1..N_s} ΔP_j` (5.11).
- **Source:** §5.7.1, Fig. 5.10; L6099–6178; PDF 146–148.
- **Philis stage:** gr, dr.
- **Automation recipe:**
  - Price coupling with the per-pair sensitivity `S_Cki`, from LAMP-03 couple perturbations, instead of the sum of self-weights. The pairs are sparse.
  - Price series R with `S_R` where a spec is R-sensitive (a gm-degenerated source, a filter net), not only through current.
  - Price via C, noted as missing in the code.
- **Beats hand layout because:** the router knows which specific aggressor–victim pairs cost spec, so it detours only where that pays.
- **Philis status:** partial. `Elec` = `weight·layer_c + beside_c·Σ(sens)` + `current·layer_r` (`backend/gr/src/lib.rs:841-872`). Coupling uses the summed self-weights, R is priced by current, and via C is not priced (`:852`).

### LAMP-40 Hard symmetric routing: route one net, mirror, DRC-check both sides
- **Kind:** algorithm
- **Statement:**
  - A symmetric net pair is routed in one step. Only one net is searched. Each candidate cell is DRC-checked on the routed side, then mirrored about the axis and DRC-checked again.
  - A cell is accepted only if it is legal on both sides.
  - The partner is then generated by mirroring.
  - This guarantees symmetric routes even for partially symmetric placements (Fig. 5.11: a detour around an obstacle present only on the other side).
- **Source:** §5.7.3; L6199–6228; PDF 148–149.
- **Philis stage:** dr, gr.
- **Automation recipe:**
  - For a `Differential` pair with a symmetry axis, search net A over nodes n such that both n and mirror(n) are free for their respective nets (joint occupancy test).
  - Commit A and mirror(A) together.
  - Fall back to the current copy/guide approach only when the terminals are not mirror images.
- **Beats hand layout because:** the parasitics of the two nets are identical by construction.
- **Philis status:** partial.
  - The tree is copied when a translation or mirror maps the terminals exactly. Otherwise one side is rerouted along the other's mirror image with a soft `GUIDE_COST = 1.0` (`backend/dr/src/lib.rs:1380-1381`, `:1400-1405`).
  - There is no joint mirrored legality in the search.

### LAMP-41 Multi-terminal net construction, Prim-like or Kruskal-like
- **Kind:** algorithm
- **Statement:**
  - Prim-like: pick a random source, connect the terminal closest in Manhattan distance, merge it into the source, and repeat.
  - Kruskal-like: connect the closest terminal pair, unify, and repeat.
  - Both gave similar quality.
- **Source:** §5.7.4, Fig. 5.12; L6231–6279; PDF 149–150.
- **Philis stage:** gr, dr.
- **Automation recipe:** none needed.
- **Beats hand layout because:** not a differentiator.
- **Philis status:** implemented as repeated multi-source Dijkstra from the growing tree (`backend/gr/src/lib.rs:874-882`).

### LAMP-42 Three-phase net scheduling with the impact factor F_k
- **Kind:** algorithm
- **Statement:**
  1. Pre-route every net alone, ignoring DRC and other nets. This is fast because DRC and performance evaluation dominate runtime.
  2. From the pre-route parasitics p_i, compute:
     - `ΔP_j = Σ_i S_i^j·p_i` (5.12);
     - the net-k share `ΔP_j^k = Σ_{i ∈ net k} S_i^j·p_i` (5.13);
     - `F_k = (Σ_{j=1..N_s} ΔP_j^k/ΔP_j)/N_s ∈ [0, 1]` (5.14).
  3. Rip up and reroute in **increasing** F_k. The highest-impact nets keep their ideal pre-routed paths, and low-impact nets detour to clear DRC and improve performance.
  4. Continue with the manufacturability phase (LAMP-43).
- **Source:** §5.8–5.8.2 eqs 5.12–5.14; L6282–6348; PDF 150–152.
- **Philis stage:** gr, dr.
- **Automation recipe:**
  - Before negotiated routing, run `route_net` per net on an empty graph (no history or present cost).
  - From `PerformanceBudget` weights × routed C (and coupling from LAMP-39), compute ΔP_j^k and F_k.
  - Replace `impact(a) = weight` in `order_by_priority` with F_k.
  - In PathFinder, scale each net's present-congestion cost by `1/(ε + F_k)`, so that low-impact nets yield contested nodes.
- **Beats hand layout because:** routing order becomes an explicit, spec-derived priority instead of an arbitrary sequence.
- **Philis status:** partial. `order_by_priority` uses symmetry, hard, budget and free tiers, then the net's own sensitivity weight. The ponytail note says it is "not Lampaert's F_k from a pre-route of every net" (`backend/gr/src/lib.rs:240-270`).

### LAMP-43 Manufacturability phase: spend leftover margin on yield and testability
- **Kind:** algorithm
- **Statement:**
  - Entered only if performance-driven routing succeeded and margin remains.
  - Nets are rerouted until the margin is consumed or no improvement is found.
  - During this phase, partial paths that would violate a spec are pruned from the heap (hard), so no spec can be broken.
  - The cost adds `T_manuf = Σ_{i=1..n, i≠k} λ_ki·Ψ_ki` (5.15): the expected bridging faults between the rerouted net k and each net i (λ, LAMP-44) times their undetectability (Ψ, LAMP-45).
- **Source:** §5.8.3 eq 5.15; L6351–6375; PDF 152.
- **Philis stage:** dr, flow.
- **Automation recipe:**
  - After an epoch is DRC/LVS-clean and every `PerformanceBudget` has `used < 1`, loop over nets in decreasing Σ_i λ_ki. Reroute with base + μ·(critical-area cost of the neighbours' shapes, LAMP-44), rejecting any node whose inclusion would push any row's `used` above 1.
  - Keep the reroute if Σλ drops.
  - Default Ψ = 1 (yield only) until LAMP-45 exists.
- **Beats hand layout because:** leftover spec margin is turned into yield (wire spreading targeted at critical-area pairs) with a guarantee that no spec is lost.
- **Philis status:** missing (no critical-area or yield term found).

### LAMP-44 Critical-area yield model (shorts, opens, pinholes)
- **Kind:** formula / metric / deck-requirement
- **Statement:**
  - Fault rate for fault i and defect mechanism j: `λ_ij = ∫₀^∞ A_ij(x)·D_j(x) dx` (5.16), with `λ_i = Σ_j λ_ij` (5.17).
  - Poisson sub-yield `Y_i = e^{−λ_i}` (5.18). Circuit yield `Y = Π Y_i = e^{−λ}` (5.19). `P_i = 1 − e^{−λ_i}` (5.20).
  - Dielectric pinholes: critical area = the inter-level overlap area `A_ov,jk` (5.21) and `D_ph(x) = D̄_ph` (5.22), so `λ = A_ov,jk·D̄_ph` (5.23).
  - Photolithographic short between parallel wires (length L, gap s): `A(x) = 0` for x < s and `L·(x − s)` for x ≥ s (5.24).
  - Open in a wire of width w: `A(x) = 0` for x < w and `L·(x − w)` for x ≥ w (5.25).
  - Defect size distribution: `D(x) = x·D̄_pd/x₀²` for x ≤ x₀ and `x₀²·D̄_pd/x³` for x ≥ x₀ (5.26). x₀ is below the optical resolution, so the minimum feature exceeds x₀.
  - Closed forms: short `λ = L·x₀²·D̄_pd/(2s)` (5.27–5.28); open `λ = L·x₀²·D̄_pd/(2w)` (5.29).
  - Per net pair: `λ_s = A_ov,jk·D̄_ph + L_jk·x₀²·D̄_pd/(2s)` (5.30).
  - Per net: `λ_o = L_j·x₀²·D̄_pd/(2w)` (5.31).
  - Numeric D̄ and x₀ values are not given.
- **Source:** §5.9–5.9.1, Figs 5.13–5.18; L6378–6616; PDF 152–157.
- **Philis stage:** deck, verify, dr.
- **Automation recipe:**
  - Deck: `defects: {d_ph_per_um2, d_pd_per_um2, x0_nm}` per layer. If absent, report relative λ with D = 1.
  - In verify: for each same-layer shape pair of different nets, run-length L at gap s gives `L·x₀²/(2s)`. For each cross-layer overlap of different nets, add `A_ov`. For each wire, `L/(2w)` for opens.
  - Report λ_short, λ_open and Y.
  - In dr (LAMP-43), use the per-node increment of these terms as a cost.
- **Beats hand layout because:** wire spreading and widening can be optimized quantitatively everywhere.
- **Philis status:** missing.

### LAMP-45 Testability weight from the Mahalanobis fault distance
- **Kind:** metric / algorithm
- **Statement:**
  - Response vector φ ∈ ℝ^m, e.g. the RMS and the first m−1 harmonics of the supply current I_PS.
  - Monte-Carlo gives the fault-free mean μ₀ and covariance Σ₀.
  - Accept the circuit iff `(φ − μ₀)ᵀΣ₀⁻¹(φ − μ₀) < χ²_m(α)` (5.32–5.33).
  - For short-fault k, `d_0,k = (μ₀ − μ_k)ᵀΣ₀⁻¹(μ₀ − μ_k)` (5.34), and `Ψ_ki = 1/d_0,k`: a small distance means hard to detect.
  - With n nodes there are n(n−1)/2 possible shorts.
- **Source:** §5.9.2 eqs 5.32–5.34; L6619–6692; PDF 157–159.
- **Philis stage:** flow (perf), dr.
- **Automation recipe:**
  - Optional, and costly: one simulation per candidate short, restricted to the net pairs with a significant λ from LAMP-44.
  - Output Ψ per pair to the LAMP-43 cost.
- **Beats hand layout because:** layout is tuned for testability, which hand layout does not do.
- **Philis status:** missing. Low priority.

### LAMP-46 Routing-phase evidence (performance kept, yield and testability improved)
- **Kind:** metric
- **Statement:**
  - Opamp1 (UGBW > 1 MHz, PM > 60°): Stage 1 gives UGBW 1.013 MHz and PM 61.04°. Stage 2 gives 1.010 MHz and 60.09°. Test error rate 0.11 % → 0.01 %.
  - Opamp2 (UGBW > 7 MHz, PM > 70°): 7.13 → 7.06 MHz, 70.3 → 70.2°, TER 0.21 % → 0.03 %.
  - CPU on an HP 712:

    | Step | Opamp1 (s) | Opamp1 (%) | Opamp2 (s) | Opamp2 (%) |
    |---|---|---|---|---|
    | Performance-driven placement | 184 | 33 | 263 | 33 |
    | Performance-driven routing | 210 | 38 | 321 | 40 |
    | Yield/testability | 163 | 29 | 213 | 27 |
    | **Total** | **557** | | **797** | |

- **Source:** §5.10, Tables 5.1–5.3; L6702–6803; PDF 159–164.
- **Philis stage:** benchmarks.
- **Automation recipe:** track the per-stage wall time and the spec-margin consumption per stage in the bench table, so that LAMP-43 gains are shown next to their margin cost.
- **Beats hand layout because:** it gives evidence to report.
- **Philis status:** n/a (a reference number).

### LAMP-47 The 10–15 % area penalty and its remedies
- **Kind:** heuristic
- **Statement:**
  - LAYLA's layouts showed a 10–15 % area penalty against manual layout.
  - The cause is the separation of place and route, and routing area that is overestimated to avoid unroutable placements. Experts place with routing in mind.
  - Remedies:
    - simultaneous place and route (Cohn 94: too complex beyond small circuits);
    - a global router inside the placement loop;
    - post-route compaction that respects performance, symmetry and matching.
- **Source:** §7, Suggestions for Future Work; L7094–7114; PDF 171–172.
- **Philis stage:** flow, gp, dp, gr.
- **Automation recipe:**
  1. Feed gr gcell overflow per region from epoch e into the LAMP-31 halos for epoch e+1 (a closed loop).
  2. Run gr's cheap pricing (`GroupPrice {hpwl, overflow, reachable}`, `backend/gr/src/lib.rs:94-100`) inside dp on sampled moves.
  3. Add a final symmetry-preserving 1-D compaction (x then y), with the performance rows as constraints.
- **Beats hand layout because:** it closes the one gap where LAYLA lost to humans.
- **Philis status:** partial. The epoch loop re-runs P&R (`frontend/library/src/lib.rs:176-190`), and gr prices group variants. There is no routing-demand feedback into placement spacing and no compaction.

### LAMP-48 Net classes for priority and penalties (ILAC)
- **Kind:** heuristic
- **Statement:**
  - Four classes: **sensitive** (high-impedance nodes), **noisy** (outputs, clocks), **non-critical signal**, **supply**.
  - They drive routing priority and coupling penalties in placement and routing.
  - The book notes this heuristic cannot guarantee specs, which is why LAYLA uses sensitivities.
- **Source:** §1.5.2.4; L1060–1066; PDF 29. Critique at L1045–1053.
- **Philis stage:** annotator.
- **Automation recipe:** keep the classes as the fallback when no simulator or specs exist. When sensitivities exist, derive the class from them: sensitive = top quantile of Σ_j |S_C|/headroom, and noisy = large dV/dt from the op-point or transient.
- **Beats hand layout because:** nets are classified the same way every time.
- **Philis status:** implemented (`backend/annotator/src/classify.rs:2-29`, `NetClass::Sensitive`, and so on).

### LAMP-49 Pre-layout infeasibility flag
- **Kind:** check
- **Statement:**
  - A direct performance-driven tool "will either yield a correct layout or will flag the specifications as being impossible to meet, without iterations".
  - The mismatch area term ΔP_area is known right after sizing (4.26).
  - So a spec whose area term, or whose process spread alone, already exceeds the margin can be flagged before placement.
- **Source:** §2.4, L1592–1598, PDF 42; §4.9.2, L4755–4756, PDF 113.
- **Philis stage:** flow, annotator.
- **Automation recipe:**
  - Before placement, for every spec, compute `headroom_corner` (LAMP-01) and `ΔP_area` (LAMP-09).
  - If `headroom_corner ≤ 0`, or if `ΔP_area ≥ headroom_corner`, emit a hard "sizing-infeasible" finding with the numbers and skip layout for that spec, or for the whole run in strict mode.
- **Beats hand layout because:** days of layout on an unrealizable sizing are avoided.
- **Philis status:** partial. `budget_rows` drops specs without nominal headroom and the caller logs "no headroom at the schematic" (`perf.rs:246-248`; `frontend/library/src/lib.rs:219-222`). There is no area-term or corner check.

### LAMP-50 Layout-aware device models and parasitic back-annotation
- **Kind:** data-model / flow
- **Statement:**
  - Each generator has a matching simulation model that includes its layout-dependent parasitics: junction caps for MOS, and resistor and capacitor parasitic networks to the substrate (Fig. 6.2).
  - Back-annotating the layout parasitics lets designers re-simulate within minutes.
  - Constraints (matching and symmetry groups, generator properties, port locations) live as schematic properties, with cross-probing.
  - Reported productivity: 5–7× over manual layout on more than 20 mixed-signal and RF chips.
  - Manual effort splits as module generation 50 %, placement 12.5 %, routing 25 %, verification 12.5 %.
- **Source:** §6.3–6.4; L6922–7010; PDF 167–169.
- **Philis stage:** flow, cells, verify.
- **Automation recipe:**
  - Emit per-variant parasitic subcircuits (resistor bottom-plate C, MOM or MIM substrate C, well C) into the perf deck along with the extracted wiring C. That already exists for stress (SA/SB).
  - Accept user-annotated constraints in the netlist (`injected`) as overrides.
- **Beats hand layout because:** layout-dependent parasitics are visible to sizing before layout exists.
- **Philis status:** partial.
  - An equivalent SA/SB stress card (`frontend/library/src/perf.rs:29-32`).
  - Extracted C is injected into the simulation (`perf.rs:16-27`, `:97-150`).
  - Injected constraints (`frontend/library/src/lib.rs:161`).
  - Per-variant passive substrate parasitic models were not verified.

---

## 4. Top-15 priorities for Philis

1. **A placement-tier performance budget.** Add a direct performance cost in dp using MST topology, estimated ground and coupling C and R, and ΔP from sensitivities, sharing spec rows with routing. Placement fixes the minimum parasitics (L3285–3287), yet Philis places on weighted HPWL only. (LAMP-04, 06, 07, 08, 34)
2. **A shared offset budget over all matched pairs, weighted by sensitivity.** This replaces per-pair η caps with `Σ S_Dk·D_k ≤ margin − ΔP_area`. It is the book's clearest measured win (comparator 6.9 → 3.7 mV, Table 4.2). (LAMP-09, 10, 49)
3. **Broader sensitivity extraction.** Add coupling pairs, series R, ΔV_T/Δβ per pair and ΔT per pair, with central differences and class-specific Δ. Every other priority depends on these numbers. (LAMP-02, 03)
4. **Corner-aware, two-sided margins.** `ΔP_lay = [P_min − P_proc^min, P_max − P_proc^max]`, one row per bound. This fixes the dropped ceiling of two-sided specs and the nominal-only headroom. It is a small change with large correctness value. (LAMP-01, 06, 49)
5. **Pre-route impact factor F_k for net ordering and congestion yielding.** This removes the ponytail heuristic in `order_by_priority`, and the most critical nets keep their ideal routes. (LAMP-42)
6. **Per-edge routing-space halos from terminal currents and net geometry, calibrated from gr overflow.** This targets the 10–15 % area gap LAYLA measured against humans. (LAMP-31, 47)
7. **Hard mirrored co-routing for differential pairs** (joint two-side legality in the search). It gives equal parasitics by construction, where the current soft guide cost does not. (LAMP-40)
8. **Thermal budget with sensitivity-weighted ΔT and distributed per-finger sources**, replacing fixed per-pair ΔT caps derived from TC. This matters for class-AB and power stages. (LAMP-13, 14, 15)
9. **Junction C in the cost, dynamic diffusion abutment on sensitive nets and Euler-trail stacking.** This is a density and performance win that hand layouters apply inconsistently. (LAMP-19, 20, 21, 23)
10. **Per-pair coupling sensitivities and S_R pricing in the router, plus via C.** This makes gr and dr cost match eq. 5.10 instead of summed self-weights. (LAMP-39)
11. **Manufacturability phase.** Spend leftover margin on critical-area yield (wire spreading and widening), with the spec guaranteed by pruning. This is a quantitative advantage no hand layout has. (LAMP-43, 44)
12. **Degradation attribution report** per spec, per net, per matched pair and per thermal pair (Tables 4.2 and 4.6 style) in metadata. It makes failures actionable and the benchmarks explainable. (LAMP-35, 33)
13. **Matched-group joint rotation and the symmetric along-axis swap in dp.** These recover search freedom that is currently frozen for matched sets. (LAMP-12, 28)
14. **Deck fidelity for inner-loop parasitics:** a lateral `F(d)` polynomial, exponential inter-layer fringe, β mismatch coefficients, substrate type and epi thickness. The estimators are only as good as these. (LAMP-18, 10, 16)
15. **An A\* predictor in `route_net`/`Dij`**: Manhattan distance times the minimum step cost, which is admissible. It is a pure speed-up that funds more epochs and starts. (LAMP-38)
