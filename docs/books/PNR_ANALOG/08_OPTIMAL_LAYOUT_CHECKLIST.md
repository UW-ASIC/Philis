# The Optimal Analog Layout: Complete Concern Checklist (Analog-Enriched)

This document catalogs every physical, electrical, manufacturability, and reliability concern that an "optimal" analog layout must address. Each concern is tagged with the pipeline stage(s) where it is handled, its priority tier, and what goes wrong if it's ignored.

The **Analog Enrichment Location** column points to the specific file and section heading in the `PNR_ANALOG/` enriched specs where that concern is addressed with textbook-backed analog domain knowledge.

---

## Tier 1 -- Non-Negotiable (failure to address = non-functional circuit)

| # | Concern | Stage | What happens if ignored | Analog Enrichment Location |
|---|---------|-------|------------------------|---------------------------|
| 1.1 | DRC compliance | S3, S4 | Fabrication failure; masks rejected by foundry | `05_S3_ROUTING_DETAILED.md` Section 11.A (OPC-friendly routing, manufacturability); `03_S2_S3_S4_INTERFACES.md` Section 1.A S4 (DRC as contract between designer and fab, dummy/false errors, runtime); `10_BENCHMARK_VALIDATION_PDK.md` Section 3.2.A (design rule robustness -- robust vs. non-robust rules) |
| 1.2 | LVS correctness | S4 | Wrong circuit is fabricated; opens, shorts, wrong connections | `03_S2_S3_S4_INTERFACES.md` Section 1.A S4 (zero topological errors non-negotiable; one error masks others); `03_S2_S3_S4_INTERFACES.md` Section 1.B S4 (LVS zero errors before PEX); `10_BENCHMARK_VALIDATION_PDK.md` Section 2.1.A (17-point Hastings checklist items 2-3) |
| 1.3 | Matching/symmetry for diff pairs | S0, S1, S2, S3 | Offset, CMRR, PSRR degradation; circuit may not function | `00_ANALOG_PRINCIPLES.md` Section 1 (full matching hierarchy: Pelgrom, systematic mismatch ranking, matching tiers, five CC rules, 25 resistor / 24 MOS / 13 capacitor matching rules); `01_S0_CELL_GENERATION.md` Section 4.A (five CC rules enforced in pattern selector, current vs. voltage matching sizing); `02_S1_CONSTRAINT_EXTRACTION.md` Section 3.A (matching tier assignment algorithm, what GNN misses); `04_S2_PLACEMENT_DETAILED.md` Section 1.1.A (LDE weight hierarchy, matched pair placement as primary objective); `05_S3_ROUTING_DETAILED.md` Section 3.A (matched net routing is hard constraint), Section 8.A (parasitic matching by construction: length/via/layer/coupling equalization) |
| 1.4 | Latch-up prevention (well taps) | S0 | Destructive latch-up; chip damage under normal operation | `00_ANALOG_PRINCIPLES.md` Section 4.6 (latchup condition B_NPN*B_PNP >= 1, triggers, guard ring types and efficiencies, design rules, P- substrate warning); `01_S0_CELL_GENERATION.md` Section 6.A (guard ring theory: 4 types with width formulas, completeness requirement, connection strategy, deep trench, well-tap spacing for analog, deep-N+ plug for BiCMOS); `02_S1_CONSTRAINT_EXTRACTION.md` Section 2.A (isolation domain triggers, merged device debiasing analysis, forward-biased junction analysis) |
| 1.5 | Connectivity (all nets routed) | S3 | Open circuits; non-functional blocks | `05_S3_ROUTING_DETAILED.md` Section 6.2 (constraint-aware A* routing algorithm); Section 7.A (tiered constraint relaxation policy -- never relax exceptional matching) |
| 1.6 | Supply/ground continuity | S3 | Devices without power; non-functional | `05_S3_ROUTING_DETAILED.md` Section 6.1.A (star vs. daisy-chain vs. mesh topology selection for power nets; star node rule for matched device returns); Section 10.A (IR drop analysis with resistive network solve); `00_ANALOG_PRINCIPLES.md` Section 4.2 (IR drop budget: < 5% Vsupply for analog) |

## Tier 2 -- Performance-Critical (failure = degraded specifications)

| # | Concern | Stage | What happens if ignored | Analog Enrichment Location |
|---|---------|-------|------------------------|---------------------------|
| 2.1 | Parasitic capacitance minimization | S2, S3 | Bandwidth reduction, gain degradation, speed loss | `00_ANALOG_PRINCIPLES.md` Section 3.1 (node sensitivity table), Section 3.2 (capacitance thresholds: C_budget = 1/(2*pi*f*R_node)); `05_S3_ROUTING_DETAILED.md` Section 6.3.A (Cost_layer_cap term), Section 6.6.A (capacitance-based wire sizing: min-width on highest metal for high-Z nodes), Section 12.A (layer assignment rules: high-Z on highest metal) |
| 2.2 | Parasitic resistance minimization | S3 | IR drop, noise increase, gain loss, EM risk | `00_ANALOG_PRINCIPLES.md` Section 3.3 (resistance thresholds, wire resistance model); `05_S3_ROUTING_DETAILED.md` Section 6.3.A (Cost_resistance term), Section 6.6.A (resistance-based wire sizing, IR drop sizing for power nets, source degeneration awareness) |
| 2.3 | Parasitic matching (R, C) for matched nets | S3 | Offset, gain error, INL/DNL degradation in ADCs | `00_ANALOG_PRINCIPLES.md` Section 3.4 (matched net routing requirements: same layers, identical via stacks, length matched to 1-5%, jogs/stubs to equalize C); `05_S3_ROUTING_DETAILED.md` Section 8.A (parasitic matching by construction: length equalization jogs, via count equalization, layer assignment matching, coupling environment matching, C equalization stubs); Section 3.A (matching tier determines R/C/via/length tolerance) |
| 2.4 | Multi-finger gate resistance | S0 | Noise increase (LNA), f_max degradation, dynamic errors | `01_S0_CELL_GENERATION.md` Section 3.A (finger count driven by matching granularity, LOD equalization, orientation chi, and Pelgrom sizing -- not just R_gate; practical MOS sizing tables from Hastings Ch. 13) |
| 2.5 | Common-centroid / interdigitation | S0, S1 | Systematic mismatch from process/thermal gradients | `00_ANALOG_PRINCIPLES.md` Section 1.5 (five CC rules: coincidence, symmetry, dispersion, compactness, orientation; 2D vs. 1D: 60% residual mismatch); `01_S0_CELL_GENERATION.md` Section 4.A (five CC rules enforced in pattern selector, 2D preference for exceptional, array aspect ratio constraints, ratioed pairs for BJT); `02_S1_CONSTRAINT_EXTRACTION.md` Section 7.A (five CC rules as constraints, 2D vs. 1D decision rule, pattern selection heuristics) |
| 2.6 | Dummy devices at array edges | S0 | Edge etch/stress mismatch; ratio errors in mirrors/DACs | `00_ANALOG_PRINCIPLES.md` Section 2.2 (LOD/STI stress equalization via dummies); `01_S0_CELL_GENERATION.md` Section 5.A (tier-specific dummy requirements: MOS per Hastings Ch. 13 Rule 12, resistor per Hastings Ch. 8 Rule 9, capacitor per Hastings Ch. 8 Rule 7; LOD/STI equalization mechanism; microloading and corner rounding; 2D capacitor array dummies) |
| 2.7 | Guard rings for noise isolation | S0 | Substrate noise coupling; spurious tones in ADCs | `00_ANALOG_PRINCIPLES.md` Section 4.4 (substrate noise attenuation model: 20-40 dB per ring, 40-60 dB deep N-well; substrate debiasing); `01_S0_CELL_GENERATION.md` Section 6.A (4 guard ring types with width formulas, completeness requirement, connection strategy, WPE displacement, deep trench isolation, well-tap spacing for analog) |
| 2.8 | Signal integrity (crosstalk/coupling) | S3 | Clock-to-signal coupling (GeniusRoute showed 5x offset worsening) | `00_ANALOG_PRINCIPLES.md` Section 3.4 (routing implications: never route clock/digital adjacent to high-Z analog, shielding); `05_S3_ROUTING_DETAILED.md` Section 6.3.A (Cost_coupling term with net-class-pair weights), Section 6.2.B (crosstalk exclusion matrix as hard constraints), Section 9.A (three shielding methods: lateral, vertical, all-around; 5 shield rules; coupling voltage estimation: dV = C*R*dV/dt); `02_S1_CONSTRAINT_EXTRACTION.md` Section 5.A.2 (crosstalk sensitivity matrix generation) |
| 2.9 | WPE mismatch | S0, S1, S2 | Vth mismatch 10-50 mV; bias point errors | `00_ANALOG_PRINCIPLES.md` Section 2.1 (WPE 4-edge model: dVth = sum a_k*exp(-d_k/lambda_k), quantitative impact at 0.25-5 um, layout knobs); `02_S1_CONSTRAINT_EXTRACTION.md` Section 4.A.1 (WPE extended model with tier-specific well distance requirements: minimal >= 2 um, moderate >= 3 um, exceptional >= 5 um); `04_S2_PLACEMENT_DETAILED.md` Section 2.5.A (WPE gradient accuracy, well boundary estimation) |
| 2.10 | LOD/STI stress mismatch | S0, S1, S2 | Drive current mismatch up to 13% (NMOS); bias errors | `00_ANALOG_PRINCIPLES.md` Section 2.2 (LOD correction function f(SA,SB), quantitative impact, dummy-based equalization); `01_S0_CELL_GENERATION.md` Section 5.A.2 (LOD/STI equalization via dummies: moat stretch, full dummy transistor); `02_S1_CONSTRAINT_EXTRACTION.md` Section 4.A.2 (LOD extended model with tier-specific dummy specs) |
| 2.11 | Thermal gradient across matched pairs | S2 | 1-2 mV/C systematic offset from dVth/dT | `00_ANALOG_PRINCIPLES.md` Section 4.3 (thermal gradient model: T(x,y) via Green's function, dVth/dT = -1 to -2 mV/C, Seebeck EMF, self-heating); `04_S2_PLACEMENT_DETAILED.md` Section 2.6.A (isotherm-aligned placement, CC thermal cancellation, power device separation: 75% from center to far edge for exceptional); `02_S1_CONSTRAINT_EXTRACTION.md` Section 4.A.5 (thermal proximity constraints with tier-specific dT tolerances: 2C/0.5C/0.1C); `03_S2_S3_S4_INTERFACES.md` Section 1.A S4 (thermal gradient validation at sign-off) |
| 2.12 | Orientation uniformity for matched devices | S0 | Mobility mismatch ~15%; gain and current errors | `00_ANALOG_PRINCIPLES.md` Section 1.3 (orientation mismatch ranked #1 systematic error at ~15% gm); `01_S0_CELL_GENERATION.md` Section 9.A (orientation chi metric, #1 systematic error, editing errors as common cause, directional-implant 90-degree rotation prohibition, PMOS 45-degree option, BJT orientation, resistor piezoresistivity); `02_S1_CONSTRAINT_EXTRACTION.md` Section 9.1 (orientation uniformity as HARD constraint, never softened) |
| 2.13 | Routability (placement anticipates routing) | S2 | Compact but unroutable placement; wasted iterations | `04_S2_PLACEMENT_DETAILED.md` Section 5.A (matched routing feasibility check: can matched nets be routed with symmetric parasitics?; explicit routing channel reservation: 10-20% of core area; choke point detection); `05_S3_ROUTING_DETAILED.md` Section 1.A (upstream dependency: routing cannot rescue poor placement) |
| 2.14 | Net shielding for sensitive signals | S3 | Coupling from clock/digital to high-impedance analog nodes | `05_S3_ROUTING_DETAILED.md` Section 9.A (three shielding methods: lateral, vertical, all-around; 5 rules: DC-connected shields, 5 um extension, regular ground connections, symmetric for matched pairs, right-angle crossings; shield insertion ordering); `02_S1_CONSTRAINT_EXTRACTION.md` Section 5.A.1 (high_impedance and compensation net classes with shielding requirements) |
| 2.15 | System signal flow regularity | S2 | Parasitic-induced feedback; stability and BW degradation | `04_S2_PLACEMENT_DETAILED.md` Section 1.2.A (signal flow is not optional for analog; sigpath extraction from netlist topology; meet-in-the-middle philosophy); Section 2.1.A (signal-flow penalty term: SF(v), feedback-path penalty: FB(v)); `00_MASTER_ARCHITECTURE.md` Section 2.A (signal flow must be preserved throughout pipeline) |

## Tier 3 -- Reliability (failure = premature device degradation or yield loss)

| # | Concern | Stage | What happens if ignored | Analog Enrichment Location |
|---|---------|-------|------------------------|---------------------------|
| 3.1 | Electromigration (wire current density) | S0, S3 | Wire opens/shorts after months/years; field failures | `00_ANALOG_PRINCIPLES.md` Section 4.1 (Black's equation, DC/AC J_max limits, Blech length immortality condition); `05_S3_ROUTING_DETAILED.md` Section 10.A (unidirectional vs. bidirectional J_max, via-below preference for Cu, Blech immortality exploitation, segment subdivision, reservoir effect, temperature derating with Black's law); Section 4.A (EM at every point along a lead, temperature derating, via placement at bends, Blech waivers); `01_S0_CELL_GENERATION.md` Section 8.A.4 (intra-cell EM sizing with Blech waivers) |
| 3.2 | IR drop on supply/ground | S3 | Headroom loss; bias point shifts; dynamic range reduction | `00_ANALOG_PRINCIPLES.md` Section 4.2 (IR drop budget: < 5% Vsupply for analog, resistive network model); `05_S3_ROUTING_DETAILED.md` Section 10.A (IR drop effects on matching: supply R must be equalized for matched pairs; Kelvin connections for exceptional); Section 6.6.A (IR drop wire sizing: W_IR formula); `02_S1_CONSTRAINT_EXTRACTION.md` Section 6.A.2 (analog IR drop budget 5% vs. digital 10%, matched-net IR drop constraint) |
| 3.3 | Antenna effect (plasma charging) | S3, S4 | Gate oxide damage during fabrication; yield loss | `00_ANALOG_PRINCIPLES.md` Section 4.5 (antenna rule, ratio formula, fixes: jumper, diode, shorter segments); `02_S1_CONSTRAINT_EXTRACTION.md` Section 6.A.4 (antenna ratio pre-computation per gate per layer); `03_S2_S3_S4_INTERFACES.md` Section 1.A S4 (antenna fixes must preserve symmetry for matched pairs); `00_MASTER_ARCHITECTURE.md` Section S3.A (antenna fixes applied symmetrically to matched net pairs) |
| 3.4 | Via reliability (multi-cut, redundancy) | S0, S3 | Single-via failure -> open circuit; yield and reliability | `05_S3_ROUTING_DETAILED.md` Section 11.A (via redundancy rules: >= 2 cuts for every connection, in-line placement, never at corners; via reliability is probabilistic); Section 6.6.A (via count matching for matched nets, via arrays at wide-lead transitions); `00_MASTER_ARCHITECTURE.md` Section S3.A (multi-cut vias by default on all signal paths) |
| 3.5 | ESD protection (at I/O pads) | S0 | Electrostatic damage during handling | `00_MASTER_ARCHITECTURE.md` Section S0.A (ESD protection cell generation: GGNMOS ballasting, silicide blocking, CDM proximity max 50 um, ECGR for ESD devices, ESD ground ring max 2 ohms); `02_S1_CONSTRAINT_EXTRACTION.md` Section 5.A.1 (ESD_bus net class: 2-ohm ring resistance, peripheral ring topology, CDM clamp proximity, chamfered corners); `05_S3_ROUTING_DETAILED.md` Section 6.6.A (ESD bus metal corner chamfering) |

## Tier 4 -- Manufacturability (failure = yield loss or fab complications)

| # | Concern | Stage | What happens if ignored | Analog Enrichment Location |
|---|---------|-------|------------------------|---------------------------|
| 4.1 | Metal density rules (min/max fill) | S4 | CMP non-uniformity; thickness variation; parametric yield loss | `03_S2_S3_S4_INTERFACES.md` Section 1.A S4 (density fill must respect matching: pseudolayer keep-outs over matched gates, 5 um extension for exceptional, symmetric fill patterns); `05_S3_ROUTING_DETAILED.md` Section 11.A (metal density tracking during routing, dummy fill blocking over matched gates, metal slotting for wide power) |
| 4.2 | Min-step / OPC compliance | S3 | Mask correction failure; lithographic errors | `05_S3_ROUTING_DETAILED.md` Section 11.A (OPC-friendly routing: avoid jogs < 2x min width, prefer straight runs, consistent wire widths, gradual tapers); `09_LITERATURE_SURVEY.md` Section 3.9 (OPC less aggressive for analog than digital, but submicron gates need mild OPC) |
| 4.3 | Double/multi-patterning rules (advanced) | S3 | Coloring conflicts; mask generation failure | `05_S3_ROUTING_DETAILED.md` Section 11.A (multi-patterning awareness: coloring assignment, coloring-aware routing for matched net parasitic symmetry; deferred to Phase 4) |
| 4.4 | Well/implant density rules | S2 | Implant dose non-uniformity | `04_S2_PLACEMENT_DETAILED.md` Section 4.A (ILP well/implant density balance constraint: per-tile N-well density within PDK.min/max; imbalanced density causes Vth variation near matched devices) |

## Tier 5 -- Optimization (failure = suboptimal but functional)

| # | Concern | Stage | What happens if ignored | Analog Enrichment Location |
|---|---------|-------|------------------------|---------------------------|
| 5.1 | Area minimization | S2 | Larger die; higher cost; potentially worse parasitics | `00_ANALOG_PRINCIPLES.md` Section 5 Priority 5 (area is last priority; dummy devices, guard rings, generous sizing all cost area -- accept this cost); `04_S2_PLACEMENT_DETAILED.md` Section 2.4.A (area is Priority 5; eta < min(tau, delta, theta); Pareto front via enhanced shape functions) |
| 5.2 | Wirelength minimization | S2, S3 | More parasitics; higher power in interconnect | `04_S2_PLACEMENT_DETAILED.md` Section 2.1.A (wirelength is necessary but not sufficient; per-net criticality weighting; matched nets target symmetric distances, not minimum HPWL); `05_S3_ROUTING_DETAILED.md` Section 6.6.A (RC product drops as 1/l^2: reducing line length is the single most effective RC optimization) |
| 5.3 | Via count minimization | S3 | More parasitic R; lower reliability | `05_S3_ROUTING_DETAILED.md` Section 12.B (via resistance 1-50 ohms per contact; single via difference = 1-10% R mismatch in matched pairs); Section 8.A (via count matching: insert compensating via jogs if asymmetric) |
| 5.4 | Aspect ratio / fixed outline | S2 | Packaging incompatibility; wasted die area | `04_S2_PLACEMENT_DETAILED.md` Section 4.1 (ILP aspect ratio constraint: r_min <= W/H <= r_max); Section 7.3 (Plantage enhanced shape functions: Pareto front of placements at different aspect ratios); `01_S0_CELL_GENERATION.md` Section 4.A.1 (array aspect ratio: <= 3:1 voltage matching, ~1:1 exceptional current matching) |
| 5.5 | Routing congestion balancing | S2, S3 | Local hotspots; routing detours; parasitic increases | `04_S2_PLACEMENT_DETAILED.md` Section 5.A (matched routing feasibility check; explicit routing channel reservation: 10-20% of core area; choke point detection); `05_S3_ROUTING_DETAILED.md` Section 7.A (rip-up priority: noisy first, matched last; tiered constraint relaxation) |
| 5.6 | Layout aesthetics / regularity | S3 | Harder to debug; less robust to process variation | `05_S3_ROUTING_DETAILED.md` Section 2.A (layout regularity captured implicitly by VAE model trained on expert layouts; Cost_violate term penalizes deviations from learned patterns) |
| 5.7 | Pelgrom-optimal unit device sizing | S0 | Over-designed (too large) or under-designed (too much mismatch) | `00_ANALOG_PRINCIPLES.md` Section 1.1 (Pelgrom model: sigma(dVth) = A_VT/sqrt(W*L), drain current mismatch equation); `01_S0_CELL_GENERATION.md` Section 4.A.4 (current vs. voltage matching: different sizing strategies; voltage matching scales W*L, current matching scales L primarily; Hastings Tables 13.5/13.6 with quantitative area/length requirements), Section 4.A.5 (drain current mismatch equation, subthreshold guard: Veff >= 100 mV) |

---

## Coverage Map: Concern -> Pipeline Stage

```
                    S0     S1     S2     S3     S4
                  Cell   Constr  Place  Route  Sign-off
                  -----  -----  -----  -----  -----
Matching          >>>>   >>>>   >>>>   >>>>   ....
Parasitics        >>..   ....   >>..   >>>>   >>>>
LDE (WPE/LOD)     >>>>   >>>>   >>>>   ....   >>>>
Thermal           ....   >>..   >>>>   ....   >>>>
EM / IR           >>..   >>..   ....   >>>>   >>>>
Substrate noise   >>>>   >>..   >>..   ....   ....
Signal integrity  ....   >>..   ....   >>>>   >>>>
DRC / Mfg         >>>>   ....   ....   >>>>   >>>>
LVS               ....   ....   ....   ....   >>>>
Antenna           ....   ....   ....   >>..   >>>>
Density / Fill    ....   ....   ....   ....   >>>>
Performance       ....   ....   >>>>   >>..   >>>>

>>>> = primary responsibility   >>.. = contributes   .... = not involved
```

---

## Enrichment Coverage Summary

All 29 concerns in the original checklist have at least one pointer to a specific section in the enriched PNR_ANALOG/ specs. Every concern is covered by analog domain knowledge from one or more of the three textbooks (Hastings AOAL, Lienig FOLD, Graeb ALS). No concern was left without a pointer.
