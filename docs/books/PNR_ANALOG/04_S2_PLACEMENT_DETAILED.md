# S2 -- Placement Engine: Full Standalone Specification (Analog-Enriched)

---

## 1. Problem Statement

Given a set of n movable cells V (from S0) and a set of nets E (from netlist), find positions v = (x_1,...,x_n, y_1,...,y_n) that minimize a weighted sum of wirelength, area, and performance degradation subject to symmetry, matching, LDE, thermal, and geometric constraints.

### 1.A Analog Considerations

The problem statement above correctly captures the multi-objective nature of analog placement. However, it implicitly assumes an **absolute-coordinate (flat) representation**, where cells are positioned by (x, y) coordinates in a continuous plane. This is the natural formulation for the gradient-based Nesterov solver described in Section 3, but it is not the only -- or always the best -- representation for analog placement.

**Flat vs. topological representations.** The literature identifies two fundamentally different ways to encode a placement [ALS 1.1]:

| Representation | Search Space | Overlap Handling | Symmetry | Best For |
|---|---|---|---|---|
| **Flat (absolute coordinates)** | R^{2n} -- very large | Quadratic overlap penalty driven to zero during optimization | Modeled as soft penalty (ePlace-A) or hard constraint (ILP) | Large mixed-signal blocks (>50 cells); gradient-based solvers |
| **Topological (sequence-pair, B*-tree)** | (n!)^2 or b_n * n! -- still large but structured | Overlaps are impossible by construction | Can be baked into the representation (symmetric-feasible codes) | Small analog subcircuits (4-20 devices); SA or deterministic enumeration |

The flat representation trades a larger search space for cheaper per-move evaluation and smoother gradients, making it well-suited for Nesterov-based global placement of moderately large circuits. The topological representation eliminates overlap entirely and can restrict the search to only symmetric-feasible codes, but its combinatorial explosion (336 B*-trees for n=4; 57,657,600 for n=8 [ALS 3.5]) makes it tractable only for small groups.

**The hybrid approach.** For analog circuits, the most effective strategy is hybrid: use topological representations for small symmetry groups and critical matched subcircuits (where symmetry must be exact and the search space is manageable), then embed these pre-placed groups into a larger flat placement for global optimization. This is precisely what the HB*-tree framework does [ALS 2.4], and it is what the spec's GP + ILP two-phase approach approximates with soft-then-hard symmetry enforcement.

**Signal-flow-driven placement.** Analog circuits have a natural signal flow: input stage to gain stage to output stage, with feedback paths. The problem statement's HPWL objective captures proximity but not directionality. A signal-flow placement arranges devices so that the geometric path from input to output follows the electrical signal path, keeping feedback loops short and minimizing parasitic coupling between stages at different signal levels [FOLD 5.3]. This is particularly important for:
- Operational amplifiers: input diff pair near one edge, output stage near the opposite edge, compensation cap between them
- Voltage references: bandgap core near die center (thermal symmetry), startup circuit peripheral
- Data converters: signal path monotonically flows through unit elements

**Sources:** [ALS 1.1.5, 1.1.6], [ALS 2.1], [ALS 3.1.2], [FOLD 5.3.2].

### 1.B Analog-Algorithm Interface

The algorithm must respect:

1. **Representation flexibility.** The placement engine must support both flat-coordinate and topological sub-placements. Symmetry groups identified by S0 may be placed using topological methods (B*-tree enumeration or SA with sequence-pairs) and then injected into the global placement as rigid or semi-rigid blocks.
2. **Signal-flow ordering.** If S0 provides a signal-flow DAG for the circuit, the global placement should bias device positions so that the geometric left-to-right (or bottom-to-top) ordering respects the electrical signal flow. This can be encoded as a soft ordering constraint in GP or a hard ordering constraint in the ILP.
3. **Hierarchy awareness.** The problem decomposition must reflect the circuit hierarchy (exact or virtual), not just cell count. A symmetry group of 6 devices is not "the same" as 6 unrelated devices for solver selection purposes.

---

## 2. Global Placement: The 7-Term Objective

```
min  W(v) + lambda*N(v) + tau*Sym(v) + eta*Area(v) + delta*LDE(v) + theta*Thermal(v) + alpha*Phi(G)
 v
```

### 2.1 Term 1: Wirelength -- W(v)

**Smoothing:** Weighted-Average (WA) function (better than LSE per Hsu et al. DAC 2011).

For each net e, approximate max_{i,j in e} |x_i - x_j| by:

```
WA_ex(v) = [Sum_i x_i*exp(x_i/gamma)] / [Sum_i exp(x_i/gamma)]
          - [Sum_i x_i*exp(-x_i/gamma)] / [Sum_i exp(-x_i/gamma)]
```

where gamma controls smoothing accuracy (smaller gamma -> more accurate but less smooth).

**Total wirelength:** W(v) = Sum_e [WA_ex(v) + WA_ey(v)]

**Gradient:** dW/dx_i for net e:

```
dWA_ex/dx_i = [exp(x_i/gamma)*(1 + x_i/gamma) * Sum_j exp(x_j/gamma) - exp(x_i/gamma) * Sum_j x_j*exp(x_j/gamma)]
              / [Sum_j exp(x_j/gamma)]^2
              - (similar term for the negative exponential part)
```

**Tuning:** gamma starts large (smooth, global search) and decreases over iterations (sharper, local refinement). Typical schedule: gamma = gamma_0 * 0.95^iteration, with gamma_0 ~ max(die_width, die_height) / 10.

### 2.1.A Analog Considerations: Wirelength Is Necessary but Not Sufficient

HPWL minimization captures a critical analog concern -- that connected devices should be close -- but misses several analog-specific aspects of "connectivity quality":

**1. Net criticality weighting.** Not all nets contribute equally to performance. Matched nets (diff pair gates, mirror gates) need balanced parasitics more than short total length. High-impedance nodes (cascode drains) need minimum parasitic capacitance more than minimum length. The WA model treats all nets identically unless weighted. The spec should support per-net weights w_e in the wirelength objective: W(v) = Sum_e w_e * [WA_ex(v) + WA_ey(v)], where w_e is derived from S0's net criticality tags [00_ANALOG_PRINCIPLES, Section 3.1].

**2. Matched net wirelength vs. parasitic equalization.** For matched net pairs, the wirelength objective should target symmetric pin-to-pin distances rather than minimum total wirelength. Asymmetric placement that appears HPWL-optimal may require costly parasitic equalization jogs during routing (serpentine jogs or dead-end stubs prescribed in `05_S3_ROUTING_DETAILED.md` Section 8.A), which intentionally increase wirelength. A placement that produces equal-length routing paths for both matched nets is superior to one with lower total HPWL but asymmetric pin access, even if the symmetric placement has higher aggregate wirelength. The wirelength weight w_e for matched nets should penalize |HPWL(net_a) - HPWL(net_b)| rather than sum(HPWL(net_a) + HPWL(net_b)).

**3. Signal-flow alignment.** HPWL measures the bounding box of a net but not whether devices are arranged to follow the signal path. Two placements with identical HPWL can differ dramatically in signal-flow quality. Consider a two-stage amplifier: placing the input pair between the load mirrors (HPWL-optimal) may interleave signal stages, while placing input pair on the left and loads on the right (slightly worse HPWL) produces a clean signal flow. A signal-flow penalty term can be added:

```
SF(v) = Sum_{(i,j) in signal_path} max(0, x_j - x_i - margin)^2
```

where (i,j) are directed signal-path edges and we penalize backward flow (j appearing to the left of i) [FOLD 5.3.2]. This term is cheap to compute and differentiable.

**4. Feedback path length.** Compensation networks (Miller caps, feedforward paths) create feedback loops. The total loop length directly affects phase margin through parasitic delay. A feedback-path penalty:

```
FB(v) = Sum_{loops} (perimeter of bounding box of loop devices)^2
```

keeps feedback devices compact.

**Sources:** [00_ANALOG_PRINCIPLES, Section 3], [FOLD 5.3.2].

### 2.1.B Analog-Algorithm Interface

1. The wirelength objective must accept per-net weights w_e from S0. Default w_e = 1; matched nets and high-impedance nets receive higher weights.
2. If S0 provides a signal-flow DAG, an optional signal-flow penalty SF(v) is added to the objective with weight sigma_SF. Default sigma_SF = 0 (disabled for circuits without clear signal flow).
3. Feedback loop device sets, if identified by S0, generate an optional FB(v) term with weight sigma_FB.

### 2.2 Term 2: Overlap -- N(v)

**Method:** Electrostatic potential energy (ePlace, Lu et al. TCAD 2015).

Model each cell as a charge distribution. The overlap energy is:

```
N(v) = Sum_x Sum_y [rho(x,y)]^2 * dx * dy
```

where rho(x,y) is the cell density at grid point (x,y), computed by distributing each cell's area across the bins it overlaps.

**Gradient:** Computed via 2D FFT of the density function (O(B*log B) where B is bin count). This is the key efficiency advantage of ePlace over bell-shaped overlap (NTUplace3).

**Tuning:** Weight lambda increases over iterations: lambda_{k+1} = lambda_k * (1 + Delta), where Delta is proportional to remaining overlap. Typical: Delta = 0.01 initially, increasing to 0.1.

### 2.2.A Analog Considerations: Why Topological Representations Avoid This Problem Entirely

The overlap penalty is a necessary artifact of the flat (absolute-coordinate) representation. It consumes a significant fraction of the solver's effort -- the lambda scheduling, FFT computation, and gradient balancing are all workarounds for allowing cells to overlap during optimization.

**Topological representations eliminate overlap by construction.** In a B*-tree, the preorder traversal packing algorithm produces a placement where every cell abuts its neighbors with zero overlap and zero dead space [ALS 2.1, Section 2.2.3]. In a sequence-pair, the longest-common-subsequence evaluation computes non-overlapping coordinates directly [ALS 1.3]. There is no overlap energy to compute and no lambda to schedule.

This is a major advantage for small subcircuits where the number of cells is small enough for topological enumeration or SA. The trade-off: topological representations cannot easily accommodate soft (penalty-based) symmetry -- symmetry is either exactly satisfied (symmetric-feasible codes) or violated (infeasible codes). The GP's approach of soft symmetry with gradual hardening is more flexible for large circuits.

**Practical implication for the hybrid approach:** Within symmetry groups placed by topological methods (B*-tree or sequence-pair), the overlap term N(v) is identically zero. The overlap penalty need only be computed for the global-level placement where these groups interact with each other and with non-symmetric cells.

**Sources:** [ALS 1.1.5], [ALS 2.2.3], [ALS 3.3].

### 2.2.B Analog-Algorithm Interface

1. When a symmetry group is placed as a topological block (B*-tree or sequence-pair), its constituent cells are removed from the overlap computation. The group is treated as a single rigid (or semi-rigid) macro in the global density map.
2. The density bin size must account for the fact that analog cells vary enormously in size (a compensation cap may be 100x the area of a bias transistor). Bin granularity should be set to approximately the median cell dimension, not the maximum.

### 2.3 Term 3: Symmetry -- Sym(v)

For a pair (i, j) symmetric about vertical axis at x_axis:

```
Sym_pair(v) = (y_i - y_j)^2 + (x_i + x_j - 2*x_axis)^2
```

For self-symmetric device r about axis x_axis:

```
Sym_self(v) = (x_r - x_axis)^2
```

**Total:** Sym(v) = Sum_{pairs} Sym_pair(v) + Sum_{self} Sym_self(v)

**Gradient:** dSym_pair/dx_i = 2*(x_i + x_j - 2*x_axis), dSym_pair/dy_i = 2*(y_i - y_j)

**Critical design choice:** Symmetry is SOFT in GP. Enforcing hard symmetry (y_i = y_j, x_i + x_j = 2*x_axis) in GP causes 17% area increase and 9% wirelength increase (ePlace-A Table I). Hard symmetry is deferred to ILP detail placement.

**Tuning:** tau starts moderate and increases: tau_0 ~ 1.0, tau_{k+1} = tau_k * 1.05.

### 2.3.A Analog Considerations: Symmetry Groups, Symmetry Islands, and the Symmetry Axis

The spec's Sym(v) formulation is correct for penalizing symmetry violation in a gradient-based solver. However, it treats symmetry as a pairwise constraint between individual devices. In analog layout, symmetry is a **group property** with several important structural characteristics that the pairwise formulation may miss:

**1. Symmetry groups and symmetry islands.** A symmetry group S = {(b_1, b_1'), (b_2, b_2'), ..., (b_p, b_p'), b_1^s, b_2^s, ..., b_q^s} consists of p symmetry pairs and q self-symmetric modules sharing a common symmetry axis [ALS 2.1, Definition]. A **symmetry island** is a placement of a symmetry group in which every module abuts at least one other module in the same group, forming a connected placement [ALS 2.1, Definition 2.1]. The symmetry island is strongly preferred because:
- The Pelgrom mismatch term S_P^2 * D_x^2 grows with device separation D_x [ALS 2.1, Section 2.2.2]
- A connected placement enables shared well regions and guard rings, reducing area [ALS 2.1]
- Symmetric routing is trivially achievable when devices abut

The pairwise Sym(v) penalty does not enforce compactness within a symmetry group. Two devices in a pair can satisfy Sym_pair = 0 while being arbitrarily far from the symmetry axis (and from each other), as long as they are equidistant from it. Adding a compactness term for each symmetry group:

```
Compact_group(v) = Sum_{(i,j) in same group} (x_i - x_j)^2 + (y_i - y_j)^2
```

or, more precisely, penalizing the deviation of the group's bounding box from the minimum achievable:

```
Island(v) = (bbox_width(group) * bbox_height(group) - Sum_{i in group} w_i * h_i)^2
```

drives the group toward a symmetry island configuration.

**2. Multiple symmetry groups with different axes.** Analog circuits often have multiple symmetry groups, each with its own symmetry axis. The spec's formulation correctly allows different x_axis values per group. However, in some circuits, multiple groups must share the same axis (e.g., a folded cascode amplifier where the input pair, cascode mirrors, and load mirrors all share a vertical symmetry axis). This alignment constraint:

```
Align(v) = Sum_{(k,l) in aligned groups} (x_axis_k - x_axis_l)^2
```

should be added when S0 specifies axis alignment [ALS 2.7.1].

**3. Mirror vs. perfect symmetry.** The spec assumes **mirror symmetry** (paired devices are flipped about the axis). In some cases, **perfect symmetry** (identical orientation, not mirrored) is required for stringent matching against anisotropic fabrication effects such as oblique-angle ion implantation [ALS 1.1.4]. Perfect symmetry is harder to route because terminals are not mirror-symmetric, requiring parasitic-matched (but not geometrically symmetric) wires. The symmetry type (mirror or perfect) should be specified per group by S0.

**4. Embedded symmetry groups.** A symmetry group can contain other symmetry groups [ALS 1.3.2]. For example, a fully differential amplifier has a top-level symmetry group containing two sub-groups (NMOS current mirror, PMOS load mirror), each of which is itself symmetric. The evaluation algorithm must process inner groups before outer groups, ordered by an embedding DAG [ALS 1.3.2]. The overall complexity for G groups is O(G * n * log(log(n))) for sequence-pair evaluation or O(n) for HB*-tree packing [ALS 2.5].

**Sources:** [ALS 1.1.4], [ALS 2.1], [ALS 2.3], [ALS 2.7.1], [ALS 1.3.2].

### 2.3.B Analog-Algorithm Interface

1. Each symmetry group must be associated with: (a) its member pairs and self-symmetric devices, (b) its symmetry type (mirror or perfect), (c) any axis-alignment constraints with other groups.
2. The GP objective should include a per-group compactness term to drive toward symmetry-island configurations. Weight: comparable to tau.
3. For circuits with embedded symmetry groups, the ILP (Section 4) must process groups inside-out, or the topological sub-placer must handle the embedding DAG.

### 2.4 Term 4: Area -- Area(v)

```
Area(v) = WA_{V,x}(v) * WA_{V,y}(v)
```

where WA_{V,x} approximates max_{i,j in V} |x_i - x_j| (total layout width) and similarly for height.

**Why this term matters:** ePlace-A Figure 2 shows omitting Area(v) causes >20% increase in both area and wirelength. In analog, area directly affects parasitics and thus performance -- unlike digital where area is secondary.

**Gradient:** Product rule: dArea/dx_i = dWA_{V,x}/dx_i * WA_{V,y} + WA_{V,x} * dWA_{V,y}/dx_i (second term is zero since WA_{V,y} doesn't depend on x).

**Tuning:** eta ~ 0.1-1.0. Higher eta -> more compact but potentially harder to route.

### 2.4.A Analog Considerations: Area Is Last Priority, Not First

Per the Analog Hierarchy of Needs [00_ANALOG_PRINCIPLES, Section 5], area is Priority 5 -- the lowest. The Area(v) term should never override matching correctness (Priority 1), isolation (Priority 2), parasitic minimization (Priority 3), or reliability (Priority 4). In practice, this means:

- eta should be lower than tau (symmetry), delta (LDE), and theta (thermal) throughout the optimization
- If area minimization causes symmetry groups to compress to the point where guard ring spacing is violated, the area term must yield
- Dummy devices, guard rings, and generous matched-device sizing all cost area -- this cost must be accepted [00_ANALOG_PRINCIPLES, Section 5, Priority 5]

The enhanced shape function approach from Plantage [ALS 3.4] provides an alternative perspective: instead of optimizing a single area value, compute the **Pareto front** of placements across different aspect ratios. This gives the designer (or the router) a choice of shapes rather than forcing a single compact solution. Each point on the Pareto front is an enhanced shape (w, h, alpha) where alpha is the B*-tree encoding [ALS 3.4.2].

**Sources:** [00_ANALOG_PRINCIPLES, Section 5], [ALS 3.4].

### 2.4.B Analog-Algorithm Interface

1. The weight eta must satisfy eta < min(tau, delta, theta) at all iterations to respect the Analog Hierarchy of Needs.
2. If the Plantage-style enhanced shape function approach is used for sub-blocks, the global placer receives a shape function (set of valid aspect ratios) per block, not a fixed rectangle.

### 2.5 Term 5: Layout-Dependent Effects -- LDE(v)

**Purpose:** Minimize WPE and LOD/STI stress mismatch between matched devices.

**WPE mismatch model:**

```
Delta_Vth_WPE(d) = Sum_k a_k * exp(-distance_to_well_edge_k / lambda_k)
```

where a_k, lambda_k are PDK WPE model parameters per well-edge orientation.

For a matched pair (i, j):

```
LDE_WPE(v) = (Delta_Vth_WPE(i) - Delta_Vth_WPE(j))^2
```

**LOD/STI stress model:**

```
Delta_Id_LOD(d) / Id = f(SA_d, SB_d) - f(SA_nom, SB_nom)
```

where SA, SB are source-side and drain-side distances from gate edge to OD edge, and f is the PDK's LOD correction function.

For a matched pair:

```
LDE_LOD(v) = (Delta_Id_LOD(i)/Id - Delta_Id_LOD(j)/Id)^2
```

**Total:** LDE(v) = Sum_{matched pairs} [LDE_WPE(v) + LDE_LOD(v)]

**Gradient:** Chain rule through the distance computation. For WPE, dLDE_WPE/dx_i depends on d(distance_to_well_edge)/dx_i, which is a step function smoothed by the exponential model. For LOD, the gradient depends on how placement changes SA/SB, which requires tracking the positions of neighboring devices (sweep-line algorithm, Ou et al. DAC 2015).

**Tuning:** delta ~ 10-100 (LDE mismatch should be weighted heavily for precision circuits). For general circuits, delta ~ 1.

### 2.5.A Analog Considerations: LDE Is a Cell-Generation Concern First, a Placement Concern Second

The LDE term in the GP objective models the placement-level contribution to mismatch (primarily WPE, which depends on distance to well edges). However, LOD/STI stress is primarily determined by the **intra-cell** layout -- the SA and SB values depend on how fingers are arranged within a device, not on inter-device placement.

**LOD equalization is best handled during cell generation (S1).** The cell generator should ensure that all matched devices have identical SA and SB by using:
- Identical dummy gate context at array edges [00_ANALOG_PRINCIPLES, Section 2.2]
- Moat extensions of at least 3 um beyond the last active transistor [00_ANALOG_PRINCIPLES, Section 2.2]
- Merged OD regions where possible

If S1 successfully equalizes SA/SB, the LDE_LOD term in GP becomes zero and can be dropped. The GP's LDE term should focus on WPE, which genuinely depends on placement (well edge distances change as cells move).

**WPE gradient accuracy.** The WPE model dVth = Sum a_k * exp(-d_k / lambda_k) is smooth and differentiable, making it suitable for gradient-based GP. However, the "distance to well edge" depends on the well boundary, which is itself determined by the placement of PMOS and NMOS device groups. During GP, the well boundary is not known precisely. A practical approach: assume the well boundary coincides with the NMOS/PMOS group boundary, and update this boundary estimate every N iterations.

**Sources:** [00_ANALOG_PRINCIPLES, Section 2.1, 2.2].

### 2.5.B Analog-Algorithm Interface

1. If S1 has equalized SA/SB for all matched devices, the LOD term may be dropped from the GP objective (delta_LOD = 0). The LDE term reduces to WPE only.
2. The WPE computation requires an estimate of well boundaries. The algorithm must provide this estimate (e.g., convex hull of PMOS device positions) and update it periodically.
3. The delta weight must satisfy delta >= tau (LDE mismatch is a matching concern, which is Priority 1).

### 2.6 Term 6: Thermal -- Thermal(v)

**Purpose:** Minimize thermal gradient across matched devices.

**Fast thermal model:** 2D Green's function convolution.

```
T(x, y) = Sum_d P_d * G(x - x_d, y - y_d)
```

where P_d is the power dissipation of device d (from S1 thermal tags) and G is the Green's function for heat conduction in the substrate:

```
G(dx, dy) ~ 1/(2*pi*k_th*t_sub) * ln(R_max / sqrt(dx^2 + dy^2))
```

For a matched pair (i, j):

```
Thermal(v) = (T(x_i, y_i) - T(x_j, y_j))^2
```

**Total:** Thermal(v) = Sum_{matched pairs} Thermal_pair(v)

**Gradient:** dThermal/dx_i via chain rule through the Green's function sum.

**Tuning:** theta ~ 0.1 for general circuits; theta ~ 10 for precision references and bandgap circuits where thermal mismatch is first-order.

### 2.6.A Analog Considerations: Isotherms, Power Devices, and Common-Centroid Cancellation

The thermal model is physically correct (Green's function for 2D heat conduction) and the pairwise penalty for matched devices is the right metric. Two additional analog concerns:

**1. Isotherm-aligned placement.** Matched devices should be placed along **isotherms** -- contours of constant temperature [00_ANALOG_PRINCIPLES, Section 4.3]. In the presence of a single dominant heat source (e.g., an output power stage), isotherms are approximately circular arcs centered on the heat source. Placing a matched pair along an isotherm means placing them at equal distance from the heat source, which the Thermal(v) term captures for a single source. For multiple heat sources, the isotherm shape is more complex, and the Green's function sum correctly handles this.

**2. Common-centroid cancellation.** A common-centroid layout cancels **linear** thermal gradients by construction [00_ANALOG_PRINCIPLES, Section 1.5, Rule 1]. If devices in a matched group are arranged in a common-centroid pattern, the first-order thermal mismatch is zero regardless of the thermal gradient direction. The Thermal(v) term should give credit for this: if a symmetry group uses common-centroid layout (as specified by S0), the thermal penalty for that group can be reduced or eliminated.

**3. Power device separation.** The thermal model requires P_d values for all devices. Power devices (output stages, regulators) dominate the thermal landscape. The spec should enforce a minimum separation between power devices and precision matched pairs [00_ANALOG_PRINCIPLES, Section 4.3]:
- Exceptional matching: power devices at opposite end of die, ~75% from center to far edge [00_ANALOG_PRINCIPLES, Section 1.7, Rule 14]
- The GP can model this as a repulsion force between power devices and matched groups

**Sources:** [00_ANALOG_PRINCIPLES, Section 1.5, 4.3], [ALS 2.1].

### 2.6.B Analog-Algorithm Interface

1. S0 must provide power dissipation estimates P_d for all devices (even rough estimates improve thermal placement).
2. If S0 flags a symmetry group as using common-centroid layout, the thermal penalty theta for that group can be reduced to theta/10.
3. Power devices identified by S0 generate repulsion forces against precision matched groups. Minimum separation: per S0's matching tier specification.

### 2.7 Term 7: Performance -- Phi(G)

**Purpose:** Directly optimize circuit performance during placement.

**Model:** GNN (Pooling with Edge Attention network from Li et al. ICCAD 2020) trained to predict P(FOM < threshold) from the circuit graph G including device types, positions, connections.

**Key difference from SA-based use:** ePlace-AP needs the gradient dPhi/dv, not Phi itself. Computed via TensorFlow/PyTorch autodiff through the trained GNN.

**Training:** >1000 samples generated by varying placement parameters, each labeled 0 (satisfactory) or 1 (unsatisfactory FOM). Cross-entropy loss.

**Gradient injection:** The GNN gradient is added to the total gradient at each Nesterov iteration. Since Phi is non-convex and noisy, alpha should be ramped up gradually: alpha_0 = 0, increasing to alpha_max after the geometric terms have roughly converged.

**Tuning:** alpha_max ~ 1.0-10.0, calibrated so that the performance gradient is comparable in magnitude to the wirelength gradient.

### 2.7.A Analog Considerations: Layout-Aware Sizing as the Outer Loop

The GNN-based performance model is a surrogate for SPICE simulation. An alternative -- or complement -- is **layout-aware sizing**, where the actual SPICE simulator is in the loop and device sizes are adjusted in response to layout parasitics [ALS 6.1].

**The layout-aware sizing loop.** ALS Chapter 6 presents a methodology where [ALS 6.1, Section 6.3.3]:

```
1. Optimization engine proposes design variables (W, L, bias currents)
2. Geometric Constraint (GC) module determines layout parameters (number of fingers, etc.)
3. Layout template is instantiated; parasitics extracted
4. SPICE simulation evaluates performance with extracted parasitics
5. Cost function evaluation drives next iteration
```

This loop is more accurate than a GNN surrogate but much slower (each iteration requires layout generation + extraction + simulation). For the PNR tool, the practical integration point is:

**Anticipatory geometric constraint satisfaction.** During placement, the GC module's shape function [ALS 6.4] can provide the set of valid (width, height) pairs for each device given its current sizing. The placer should use these shape functions rather than fixed rectangles, allowing the layout to adapt to the sizing variables. This is the link between placement and the sizing outer loop: the placer produces not a single placement but a **family of placements** parameterized by device shapes, and the sizing loop selects the best combination.

**Parasitic-aware iteration count.** Layout-aware sizing experiments show that including geometric and parasitic information in the sizing loop adds only ~20% to CPU time but prevents specification violations that would otherwise require redesign iterations [ALS 6.5.2]:
- Without layout info: PM = 63.4 deg (violates 65 deg spec) -- requires redesign
- With layout info (Approach B): PM = 65.0 deg (meets spec) -- no redesign needed

**Sources:** [ALS 6.1], [ALS 6.4], [ALS 6.5.2].

### 2.7.B Analog-Algorithm Interface

1. If a GNN performance model is available, the Phi(G) term proceeds as specified. If not, the performance feedback loop falls back to post-placement SPICE verification.
2. The placer must accept shape functions (sets of valid width/height pairs) for each device, not just fixed rectangles. When the sizing loop modifies a device's W/L, the placer receives an updated shape function.
3. The outer sizing loop communicates with the placer through an API that provides: (a) updated device dimensions, (b) parasitic budgets for critical nets, (c) FOM pass/fail status from the latest SPICE run.

---

## 3. GP Solver: Nesterov's Accelerated Gradient Descent

```
Algorithm:
  Initialize v^0 randomly (or from a constructive heuristic)
  Set u^0 = v^0
  For k = 0, 1, 2, ..., max_iter:
    Compute total gradient g = grad[W + lambda*N + tau*Sym + eta*Area + delta*LDE + theta*Thermal + alpha*Phi]
    v^{k+1} = u^k - step_size * g(u^k)
    u^{k+1} = v^{k+1} + (k/(k+3)) * (v^{k+1} - v^k)   [Nesterov momentum]
    
    Update penalty weights: lambda, tau (increase); gamma (decrease)
    
    Check stopping conditions:
      - Overlap ratio < 0.1% of total cell area
      - Wirelength change < 0.1% between iterations
      - Max iterations reached (typically 500-2000)
```

**Step size:** Adaptive, based on Lipschitz constant estimation or line search. ePlace uses a reference-point-based step-size adaptation.

### 3.A Analog Considerations: When Nesterov Is and Is Not the Right Solver

The Nesterov accelerated gradient descent is well-suited for the GP phase of large mixed-signal designs where the number of cells exceeds ~50 and the objective is smooth. For smaller analog subcircuits, alternative approaches may be more appropriate:

**Solver selection by circuit structure, not just cell count.** The spec mentions that "analog circuits are small enough (< 200 variables) that ILP solves in seconds" (Section 4.4). This is true for the ILP detail placement, but the selection of the GP solver should also consider circuit structure:

| Circuit Structure | Recommended GP Approach | Rationale |
|---|---|---|
| Single symmetry group, <= 8 devices | **Deterministic B*-tree enumeration** (Plantage) | Exhaustive search is tractable; produces Pareto front of aspect ratios [ALS 3.5] |
| Single symmetry group, 9-20 devices | **SA with S-F sequence-pairs** or **ASF-B*-tree** | Topological SA with symmetric-feasible codes [ALS 1.3, 2.3] |
| Multiple symmetry groups, 20-80 devices | **HB*-tree with SA** | Hierarchical framework handles multiple groups with O(n) packing [ALS 2.4, 2.5] |
| Mixed analog/digital, >80 devices | **Nesterov GP (this spec)** + topological sub-placement for symmetry groups | Gradient-based for global, topological for local symmetry |

For the first three cases, the Nesterov solver is overkill and may produce inferior results because:
- The soft symmetry penalty causes symmetry violations that must be corrected in detail placement, wasting optimization effort
- The overlap penalty causes density oscillations that do not occur in overlap-free topological representations
- The combinatorial structure of small placement problems is better exploited by enumeration or SA than by gradient descent

**Topological representations for small groups: data structures and complexity.**

The key data structures for topological placement are [ALS 1.1.6, 1.2]:

| Representation | Encoding | Evaluation Complexity | Symmetry Handling |
|---|---|---|---|
| **Sequence-pair** | Two permutations (alpha, beta) | O(n log log n) with Johnson PQ [ALS 1.3] | Symmetric-feasible subset; Property 1.1 [ALS 1.3] |
| **B*-tree** | Binary tree + contour | O(n) preorder packing [ALS 2.1] | ASF-B*-tree guarantees symmetry islands [ALS 2.3] |
| **HB*-tree** | Hierarchical B*-tree with hierarchy nodes | O(n) [ALS 2.4, Theorem 2.4] | Multiple symmetry groups; symmetry islands per group |
| **TCG** | Two directed graphs C_h, C_v | O(n^2) [ALS 1.4] | Equivalent to sequence-pair but slower |

The ASF-B*-tree (Automatically Symmetric-Feasible B*-tree) [ALS 2.3] deserves special attention:
- It operates on **representatives** (one half of each symmetry pair), halving the tree size
- It guarantees that every packing produces a **symmetry island** (Theorem 2.2 [ALS 2.3])
- There is a **unique correspondence** between compacted symmetric placements and ASF-B*-trees (Theorem 2.3 [ALS 2.3])
- Packing is O(n) -- faster than all other symmetric placement approaches [ALS 2.5, Table 2.2]

**The contour data structure.** All topological representations require tracking the border contour of the partial placement to compute coordinates. Four data structures achieve O(n log n) total time [ALS 1.2]:
- **Segment tree**: Best for SA (create once, reinitialize per iteration; 15-20% speedup [ALS 1.2.1])
- **Red-black interval tree**: Amortized O(log n) per step; few rotations [ALS 1.2.2]
- **1-3 deterministic skip list**: Top-down insert/delete; linked-list implementation [ALS 1.2.3]
- **Johnson's priority queue**: Integer keys; bucket + binary tree [ALS 1.2.4]

For B*-tree packing specifically, a simple doubly-linked list suffices for O(n) contour tracking [ALS 2.1].

**Sources:** [ALS 1.1.6], [ALS 1.2], [ALS 1.3], [ALS 1.4], [ALS 2.1], [ALS 2.3], [ALS 2.4], [ALS 2.5], [ALS 3.5].

### 3.B Analog-Algorithm Interface

1. **Solver selection.** The placement engine must implement a solver-selection function that examines each symmetry group's size, the total circuit size, and the hierarchy depth, then dispatches to the appropriate solver:
   - Groups with <= 8 devices: Plantage-style deterministic enumeration
   - Groups with 9-20 devices: SA with ASF-B*-tree
   - Global placement of >50 cells: Nesterov GP
   The selection is not by cell count alone but by circuit structure (number and nesting of symmetry groups).

2. **Topological sub-placement API.** The Nesterov GP must accept pre-placed symmetry groups as fixed or semi-rigid blocks. The API: `inject_block(group_id, block_outline, device_positions_relative_to_block_origin)`. The GP then places the block's origin, treating it as a single macro with a possibly rectilinear outline.

3. **Contour data structure.** When using topological placement for symmetry groups, the implementation should use the segment tree (for SA) or doubly-linked list (for B*-tree) contour structure. The segment tree should be created once and reinitialized per SA iteration [ALS 1.2.1].

---

## 4. Detailed Placement: ILP Formulation

### 4.1 Full Formulation

```
min  Sum_e [(x_bar_e - x_e) + (y_bar_e - y_e)] + mu * (H_tilde*W + W_tilde*H)/2
     -----------------------------------------------   -------------------------
                      HPWL                                       Area

subject to:
  (a) HPWL bounding box:  x_e <= x_hat_i <= x_bar_e,  y_e <= y_hat_i <= y_bar_e,  for all i in e, for all e in E
  (b) Boundary:           w_i/2 <= x_i <= W - w_i/2,  for all i in V
  (c) Flipping:           x_hat_i = x_i - w_i/2 + xpin_i*(1-fx_i) + (w_i-xpin_i)*fx_i
  (d) Non-overlap:        x_j + w_j/2 <= x_k - w_k/2,  for all (j,k) in P^H
  (e) Hard symmetry:      (x_q1+x_q2)/2 = x_axis = x_r,  for all (q1,q2) in S^p, r in S^s
  (f) Bottom alignment:   y_b1 - h_b1/2 = y_b2 - h_b2/2
  (g) Central alignment:  x_vc1 = x_vc2
  (h) Ordering:           x_o1 + w_o1/2 <= x_o2 - w_o2/2
  (i) Aspect ratio:       r_min <= W/H <= r_max  (linearized)
  (j) Guard ring space:   additional spacing for guard-ring-padded cells
  (k) Substrate isolation: min separation between noisy and sensitive blocks
  (l) Integer grid:       x_i in N,  x_bar_e, x_e in N,  W, H in N
  (m) Flipping binary:    fx_i, fy_i in {0, 1}
```

### 4.2 Matched-Device Flipping Constraint

Devices in the same matching group must flip identically:

```
fx_i = fx_j,   for all (i,j) in same matching group
```

This prevents the ILP from breaking matching to save wirelength.

### 4.3 Non-Overlap Pair Selection

From GP results, compute overlap (dx, dy) for each overlapping pair. If dx < dy, separate horizontally (pair goes in P^H); otherwise separate vertically (pair goes in P^V). This heuristic from the GP solution guides the ILP toward a similar topology.

### 4.4 Solver

Commercial: Gurobi or CPLEX (handles the mixed-integer constraints).
Open-source: CBC (COIN-OR) or SCIP.
Analog circuits are small enough (< 200 variables) that ILP solves in seconds.

### 4.A Analog Considerations: ILP Extensions for Analog Constraints

The ILP formulation correctly handles hard symmetry, non-overlap, and boundary constraints. Several analog-specific extensions are needed:

**1. Piecewise-linear minimum distance constraints (DTI, guard rings).** When deep trench isolation (DTI) is used, the minimum distance between transistors has a **forbidden zone** -- devices must be either directly abutting (sharing a DTI) or separated by at least d_DTI [ALS 3.3.2]. This makes the constraint non-convex:

```
d(i,j) <= x_range   OR   d(i,j) >= d_DTI
```

This cannot be expressed as a single linear constraint. The Plantage approach [ALS 3.3.2] introduces a binary range variable r_{ij} in {0, 1} per pair and formulates the constraint as:

```
d(i,j) - r_{ij} * beta <= s_max       (Range 1: abutting)
d(i,j) + (1 - r_{ij}) * beta >= d_DTI (Range 3: separated)
```

where beta is a sufficiently large constant. This converts the ILP to a **mixed-integer program (MIP)**, which is still tractable for analog circuit sizes.

**2. Common-centroid constraints.** For two groups of modules A and B forming a common-centroid pair [ALS 3.1.2]:

```
x_COG(A) = x_COG(B)
y_COG(A) = y_COG(B)
```

where x_COG(A) = Sum_{m in A} (area_m * x_m) / Sum_{m in A} area_m. For equal-sized unit elements, this simplifies to:

```
Sum_{m in A} x_m = Sum_{m in B} x_m
Sum_{m in A} y_m = Sum_{m in B} y_m
```

These are linear constraints that fit naturally into the ILP.

**3. Proximity constraints.** Devices within a proximity group must form a connected placement [ALS 2.1]. This can be modeled in the ILP as:

```
For all (i,j) in same proximity group: d(i,j) <= d_proximity_max
```

where d_proximity_max is the maximum allowed separation within the group.

**4. Variant selection.** If devices have multiple layout variants (different finger counts, aspect ratios), the ILP can include integer variables selecting among variants [ALS 3.1.1]:

```
v_i in {1, 2, ..., V_i}    (variant index for device i)
w_i = w_i(v_i)              (width depends on variant)
h_i = h_i(v_i)              (height depends on variant)
```

with variant matching constraints: v_i = v_j for matched devices. The Plantage approach enumerates all valid variant combinations during basic group enumeration [ALS 3.5.2].

**5. Well/implant density balance constraint.** The ILP should include a well density balance constraint: per-tile N-well density within [PDK.min_well_density, PDK.max_well_density]. Imbalanced well density causes implant dose non-uniformity, which creates systematic Vth variation across the die. For analog circuits, well density uniformity is particularly important near matched devices where even small doping gradients cause mismatch.

```
For each placement tile t:
  well_density(t) = area_of_N_well_in_tile(t) / area_of_tile(t)
  Constraint: PDK.min_well_density <= well_density(t) <= PDK.max_well_density
```

**6. Symmetry island enforcement.** To ensure a symmetry group forms a connected placement (symmetry island), the ILP can add connectivity constraints:

```
For each symmetry group S:
  For all (i,j) adjacent in S: d(i,j) = 0  (abutting)
```

This is stronger than the proximity constraint and ensures the Pelgrom separation term D_x is minimized [ALS 2.1, Section 2.2.2].

**Sources:** [ALS 3.3.2], [ALS 3.1.2], [ALS 2.1], [ALS 3.5.2].

### 4.B Analog-Algorithm Interface

1. The ILP solver must support MIP (mixed-integer programming) to handle DTI forbidden zones. Gurobi and CPLEX both support MIP natively.
2. Common-centroid constraints from S0 are added as linear equality constraints in the ILP.
3. Proximity group constraints from S0 are added as inequality constraints on pairwise distances.
4. If S0 provides variant options for devices, the ILP includes integer variant-selection variables with matching constraints.
5. Symmetry island enforcement (abutment) is optional but recommended for moderate and exceptional matching tiers.

---

## 5. Routability Feedback (S3->S2 Inner Loop)

### 5.1 Congestion Estimation

After GP converges:

```
1. Partition layout into BxB grid of tiles.
2. For each tile t:
   demand(t) = Sum_{nets passing through t} estimated_wire_density(net, t)
   capacity(t) = available_tracks(t) * available_layers(t)
   congestion(t) = demand(t) / capacity(t)
3. If max(congestion) > congestion_threshold (typically 1.5):
   Add congestion penalty to GP and re-run.
   Congestion penalty: Sum_t max(0, congestion(t) - 1)^2 integrated over cell positions.
```

### 5.2 Alternative: VAE-Based Routing Density Prediction

Run GeniusRoute's trained VAE on the current placement to predict routing density maps. Convert high-density regions into placement repulsion forces. This is more accurate than HPWL-based estimation but requires a trained model.

### 5.A Analog Considerations: Matched Routing Feasibility

Congestion estimation for analog circuits must go beyond capacity vs. demand. The critical routing concern is not "can all nets be routed?" but "can **matched nets** be routed with symmetric parasitics?"

**Symmetric routing feasibility.** A placement that is globally routable may still be unroutable for matched nets if the placement topology prevents symmetric wire paths. For each symmetry group, the router must be able to route paired nets with:
- Same metal layers [00_ANALOG_PRINCIPLES, Section 3.4]
- Same total wire length (within ~1-5% depending on matching tier) [00_ANALOG_PRINCIPLES, Section 3.4]
- Same via stacks [00_ANALOG_PRINCIPLES, Section 3.4]
- No other signals crossing between them (shielding) [00_ANALOG_PRINCIPLES, Section 3.4]

A symmetric placement enables symmetric routing; an asymmetric placement makes it impossible. The routability feedback should include a **symmetric routing feasibility check** for each symmetry group.

**High-impedance node routing.** Nodes identified as high-impedance by S0 need minimum-length routing on the lowest-capacitance metal layers. The congestion estimator should reserve routing resources on preferred layers for these nets.

**Source:** [00_ANALOG_PRINCIPLES, Section 3.4], [FOLD 5.3.3].

### 5.B Analog-Algorithm Interface

1. The congestion estimator must accept a list of matched net pairs and verify that symmetric routing paths exist.
2. High-impedance nets from S0 receive reserved routing capacity on low-capacitance layers.
3. If symmetric routing is infeasible for a symmetry group, the feedback loop triggers re-placement of that group (not global re-placement).

---

## 6. Performance-Driven Placement: ePlace-AP Extension

### 6.1 FOM Definition

```
FOM = Sum_i beta_i * z_tilde_i

where z_tilde_i = min(z_i/psi_i, 1) for metrics preferred large (gain, BW)
      z_tilde_i = min(psi_i/z_i, 1) for metrics preferred small (delay, offset)

and psi_i is the specification for metric z_i, beta_i is its weight (Sum beta_i = 1).
```

### 6.2 Results to Expect

From ePlace-AP (DATE 2022, GF 12nm, 10 circuits):

| Metric | SA baseline | ePlace-AP | Improvement |
|--------|-------------|-----------|-------------|
| Average FOM | 0.87 | 0.90 | +3.4% |
| Average area | 1.09x | 1.00x | -9% |
| Average HPWL | 1.02x | 1.00x | -2% |
| Average runtime | 3.09x | 1.00x | 3x faster |

### 6.A Analog Considerations: What FOM Metrics Matter for Analog

The FOM definition is general enough to accommodate analog metrics. The key analog-specific metrics that should be included:

| Metric | Direction | Typical Spec | Placement Sensitivity |
|---|---|---|---|
| DC gain | Maximize | >= 60-100 dB | Low (sizing-dominated) |
| Unity-gain frequency (UGF) | Maximize | >= 10-500 MHz | High (parasitic C on high-Z nodes) |
| Phase margin (PM) | Maximize | >= 45-65 deg | Very high (compensation network parasitics) |
| Input offset voltage | Minimize | <= 0.1-5 mV | Very high (matching, LDE, thermal) |
| CMRR | Maximize | >= 60-100 dB | Very high (parasitic symmetry) |
| PSRR | Maximize | >= 60-80 dB | High (supply routing parasitics) |
| Settling time | Minimize | <= 10-500 ns | High (parasitic C, feedback path length) |
| INL/DNL (DACs/ADCs) | Minimize | <= 0.5-1 LSB | Very high (unit element matching) |

Of these, **input offset voltage**, **CMRR**, and **INL/DNL** are the most placement-sensitive because they depend directly on device matching, which is dominated by layout effects.

**Sources:** [ALS 6.1], [00_ANALOG_PRINCIPLES, Section 5].

### 6.B Analog-Algorithm Interface

1. S0 must provide the FOM metrics, their directions (maximize/minimize), specification values, and weights.
2. Matching-dependent metrics (offset, CMRR, INL/DNL) should receive higher weights because they are most affected by placement.
3. Bandwidth and phase margin metrics should be linked to parasitic budgets on specific nets, enabling the placer to focus parasitic minimization on the critical nets.

---

## 7. Topological Placement Representations for Symmetry Groups

This section is new. It specifies the topological placement engine used for small symmetry groups, complementing the gradient-based Nesterov GP described in Sections 2-3.

### 7.1 Symmetric-Feasible Sequence-Pairs

A **sequence-pair** (alpha, beta) encodes a placement topology via two permutations of the cell names [ALS 1.3]:
- If alpha(d_1) < alpha(d_2) and beta(d_1) < beta(d_2): cell d_1 is to the **left** of cell d_2
- If alpha(d_1) < alpha(d_2) and beta(d_1) > beta(d_2): cell d_1 is **above** cell d_2

A sequence-pair is **symmetric-feasible (S-F)** if, for any two cells c_1, c_2 from any symmetry group, any cell c from a **different** group that appears between c_1 and c_2 in one sequence also appears between them in the other sequence [ALS 1.3, Property 1.1]. This is both necessary and sufficient for the existence of a valid symmetric placement.

**Search space reduction.** For a symmetry group of n = 2p cells, the number of S-F sequence-pairs is at most (2p)!, compared to (2p)!^2 for general sequence-pairs -- a reduction factor of (2p)! [ALS 1.3].

**Evaluation algorithm.** The evaluation proceeds in two phases [ALS 1.3.1]:

Phase 1 -- y-coordinates: Traverse sequence beta using Johnson's priority queue. For symmetric pairs (B_i, B_j), enforce y_i = y_j. May require up to Theta(p) iterations of Step 1y (in practice <= 3).

Phase 2 -- x-coordinates: Four traversals [ALS 1.3.1]:
1. Step 1x (Initialization): Right-to-left traversal of alpha; build embedding DAG; assign initial d-values
2. Step 2x (Sweep-to-the-right): Fix topological constraints
3. Step 3x (Sweep-to-the-left): Adjust for symmetry axis distance
4. Step 4x (Sweep-to-the-right again): Fix remaining symmetry violations

Overall complexity: O(p * n * log(n)) worst case; O(n * log(log(n))) with Johnson's priority queue [ALS 1.3].

**SA move set.** Moves must preserve symmetric-feasibility [ALS 1.3.3]:
- Interchange two cells in alpha -> interchange their symmetric counterparts in beta
- Move a cell in alpha -> move its symmetric pair in beta (range depends on first move)
- Rotations/mirroring affect both symmetric cells simultaneously
- Asymmetric cells have unrestricted moves

**Multiple symmetry groups.** For G groups, cells from different groups must not interleave in the sequences. Processing order follows an embedding DAG (inner groups before outer). Complexity: O(G * n * log(log(n))) [ALS 1.3.2].

### 7.2 ASF-B*-Tree and HB*-Tree

**B*-tree basics** [ALS 2.1, Section 2.2.3]. A B*-tree is an ordered binary tree where:
- Root = bottom-left corner module
- Left child of node n_i = lowest, adjacent module to the **right** of b_i
- Right child of node n_i = first module **above** b_i with same x-coordinate
- Coordinates computed by preorder traversal + contour update; O(n) packing time

**ASF-B*-tree (Automatically Symmetric-Feasible B*-tree)** [ALS 2.3]. Key idea: operate on **representatives** only -- one half of each symmetry pair, or one half of each self-symmetric module. Definitions:

- **Representative** of pair (b_j, b_j'): b_j^r = b_j [ALS 2.3, Def 2.2]
- **Representative** of self-symmetric module b_k^s: the right half (vertical symmetry) or top half (horizontal symmetry) [ALS 2.3, Def 2.3]
- **Boundary constraint (Property 2.1):** Self-symmetric representatives must lie on the rightmost branch (vertical) or leftmost branch (horizontal) of the tree [ALS 2.3]

Theorems:
- **Theorem 2.1:** ASF-B*-tree is symmetric-feasible [ALS 2.3]
- **Theorem 2.2:** Packing produces a **symmetry island** (connected symmetric placement) [ALS 2.3]
- **Theorem 2.3:** Unique correspondence between compacted symmetric placements and ASF-B*-trees [ALS 2.3]

Packing algorithm:
```
1. Traverse ASF-B*-tree in preorder
2. For each representative node:
   - Left child: x_j = x_parent + w_parent
   - Right child: x_k = x_parent
   - y from contour structure (doubly-linked list)
3. Mirror: x_sym = 2*x_bar - x_rep, y_sym = y_rep  (vertical axis)
4. Compute bottom contour of symmetry island from dual vertical contours
```
Complexity: O(n(S_i)) -- linear in group size [ALS 2.3, Theorem 2.4].

**HB*-tree (Hierarchical B*-tree)** [ALS 2.4]. Extends B*-tree with **hierarchy nodes** encapsulating entire symmetry groups:
- Each symmetry group S_i is a single hierarchy node n_{S_i} in the top-level tree
- Inside each hierarchy node: an ASF-B*-tree
- Non-hierarchy nodes: ordinary (non-symmetric) modules
- **Contour nodes** represent the rectilinear top contour of each symmetry island

HB*-tree packing: preorder traversal; when a hierarchy node is encountered, pack its internal ASF-B*-tree first, then continue with the global tree. Total packing time: O(n) [ALS 2.4, Theorem 2.4].

**SA perturbation operations** for ASF-B*-tree [ALS 2.5]:

| Op | Description | Constraint |
|---|---|---|
| Op1 | Rotate a module | Symmetry pairs: rotate both simultaneously |
| Op2 | Move a node | Self-symmetric: only along boundary branch |
| Op3 | Swap two nodes | If self-symmetric involved: one must stay on boundary branch |
| Op4 | Change representative | Swap which module in pair is representative |
| Op5 | Convert symmetry type | Vertical <-> horizontal (seldom applied) |

HB*-tree perturbation: prefer non-hierarchy node moves; hierarchy nodes cause large jumps in solution space [ALS 2.5].

**Complexity comparison (all approaches with symmetry)** [ALS 2.5, Table 2.2]:

| Approach | Perturbation | Packing |
|---|---|---|
| Sequence-pair + Johnson PQ | O(1) | O(m * n * log(log(n))) |
| TCG-S | O(n^2) | O(n^2) |
| **ASF-B*-tree + HB*-tree** | **O(log n)** | **O(n)** |

The ASF-B*-tree + HB*-tree combination achieves the best packing complexity of any symmetric placement approach.

### 7.3 Deterministic Enumeration (Plantage)

For very small groups (2-5 devices), exhaustive enumeration is feasible and produces provably optimal results [ALS 3.5]:

**Algorithm:**
```
function enumerateOnHierarchyLevelOf(element):
    if element is a basic group G_0:
        Enumerate all B*-trees for G_0 (336 for n=4)
        For each B*-tree, enumerate valid variant combinations
        For each (tree, variants) pair:
            Generate placement with constraints (Sect 4.A)
            If Pareto-optimal: store in enhanced shape function
    else:
        For each child: recurse
        Combine children's enhanced shape functions:
            - Horizontal addition: attach beta root to lowest-rightmost of alpha
            - Vertical addition: segment beta, attach to adequate nodes of alpha
        Prune suboptimal shapes
    return enhanced shape function
```

**Enhanced shape function** [ALS 3.4]: Each shape is a tuple (w, h, alpha) where alpha is the B*-tree. The Pareto front filters shapes dominated in both width and height. The output is a set of placements at different aspect ratios, not a single solution.

**Constraint preservation:** Both horizontal and vertical B*-tree addition preserve in-order and preorder traversal relationships, so constraints satisfied by input trees are automatically satisfied by the combined result [ALS 3.3].

**Experimental results** [ALS 3.5, Section 3.6]:
- Miller amplifier (13 modules): 35 Pareto-optimal placements, 14 s runtime
- Buffer amplifier (46 modules): 114 placements, 134 s runtime
- Area usage: 110-129% of module area (including well spacing)
- Compared to SA approaches: within 1-2% of best area, but deterministic and reproducible

### 7.A Analog-Algorithm Interface for Topological Sub-Placement

1. **Input.** For each symmetry group dispatched to topological placement:
   - Device list with (w, h) dimensions (or shape functions if variants exist)
   - Symmetry pairs and self-symmetric devices
   - Matching tier (minimal/moderate/exceptional) determining placement quality requirements
   - Proximity constraints (if any)

2. **Output.** An enhanced shape function: set of Pareto-optimal (w, h, device_positions) tuples representing valid placements at different aspect ratios.

3. **Integration with GP.** The GP selects one shape from the enhanced shape function (based on current area/aspect-ratio objectives) and treats the group as a rigid block at that shape. If the GP changes the target aspect ratio during optimization, a different shape can be selected without re-running the topological placer.

4. **Solver dispatch logic:**
   ```
   if group_size <= 5:
       use Plantage deterministic enumeration
   elif group_size <= 20:
       use SA with ASF-B*-tree (HB*-tree if nested groups exist)
   else:
       use Nesterov GP with soft symmetry, followed by ILP hardening
   ```

---

## 8. Hierarchy-Aware Placement Decomposition

This section is new. It specifies how the placement problem is decomposed along the circuit hierarchy.

### 8.1 The HSMPG Tree

The **Hierarchical Symmetry, Matching, and Proximity Group (HSMPG) tree** [ALS 3.2] captures the circuit's structure:
- Root = entire circuit
- Internal nodes = analog building blocks (differential pairs, current mirrors, cascode structures)
- Leaf nodes = individual devices (transistors, capacitors, resistors)

The HSMPG tree can be automatically generated from the netlist by [ALS 3.1.3]:
1. Identifying analog building blocks (current mirrors, diff pairs) via structural analysis
2. Detecting symmetry conditions via subgraph isomorphism
3. Clustering devices by device model and subcircuit functionality
4. Assigning constraints (symmetry, proximity, matching) per cluster

**Exact vs. virtual hierarchy** [ALS 2.1]. The layout design hierarchy may contain:
- **Exact hierarchy**: matches the circuit schematic hierarchy
- **Virtual hierarchy**: clusters added for layout purposes (e.g., grouping all NMOS devices that share a well, even if they are in different schematic subcircuits)

### 8.2 Bottom-Up Enumeration, Top-Down Assembly

The hierarchy guides placement in a bottom-up sweep [ALS 3.5]:

1. **Level 0 (basic groups):** For each leaf-level cluster (2-5 devices sharing a parent in the HSMPG tree), enumerate or SA-optimize placements. Store results as enhanced shape functions.

2. **Level 1 (building blocks):** Combine sibling groups' enhanced shape functions via horizontal and vertical addition. All permutation sequences are tried (ESF addition is not commutative). Prune suboptimal results.

3. **Level 2+ (subsystems):** Continue combining up the hierarchy. At each level, the number of items to combine is small (typically 2-4 children per parent).

4. **Top level:** The enhanced shape function of the root gives the Pareto front of complete circuit layouts.

**Why not pure bottom-up?** The optimal placement of a subcircuit may not lead to the globally optimal placement [ALS 2.1]. The HB*-tree framework addresses this by optimizing all levels simultaneously: the SA perturbs the HB*-tree (which contains all hierarchy nodes and their internal ASF-B*-trees) in a single annealing run [ALS 2.5]. This is a critical difference from the strict bottom-up approach of Plantage.

**When to use which:**

| Approach | When | Advantage | Disadvantage |
|---|---|---|---|
| Bottom-up Plantage | Small circuits (<50 devices), many variants | Deterministic, Pareto front, parallelizable | May miss global optimum |
| Simultaneous HB*-tree SA | Medium circuits (20-80 devices) | Global optimization of all levels | Stochastic, single solution |
| Nesterov GP + topological sub-placement | Large mixed-signal (>80 devices) | Scalable, gradient-efficient | Symmetry is approximate until ILP |

### 8.A Analog-Algorithm Interface

1. S0 provides the HSMPG tree (or equivalent hierarchy specification).
2. The placement engine traverses the HSMPG tree to identify basic groups and dispatches each to the appropriate solver (Section 7.A).
3. The hierarchy determines the order of combination: inner symmetry groups before outer ones, basic groups before composite groups.
4. The final output includes: (a) the complete placement with all device coordinates, (b) the enhanced shape function (if Plantage was used), (c) the topological encoding (B*-tree or sequence-pair) for each symmetry group, enabling efficient re-optimization if the outer loop changes device sizes.

---

## 9. Layout-Aware Sizing Feedback Interface

This section is new. It specifies how the placement engine interacts with the sizing outer loop.

### 9.1 The Sizing-Placement Feedback Loop

Layout-aware sizing [ALS 6.1] integrates placement into the device sizing optimization loop:

```
Outer loop (sizing optimizer):
  1. Propose design variables (W, L, bias currents, finger counts)
  2. GC module determines geometric parameters (shape functions for each device)
  3. Call placement engine with updated device dimensions
  4. Extract parasitics from placement
  5. SPICE simulation with parasitics
  6. Evaluate cost function (electrical specs + area + parasitic robustness)
  7. Update design variables and repeat
```

**Key insight from ALS 6.5.2:** Including geometric and parasitic information in the sizing loop adds ~20% to CPU time but prevents specification violations that would require full redesign iterations. Without layout info, post-extraction performance can degrade by 4% in phase margin and 2% in UGF -- enough to violate specs.

### 9.2 What the Placement Engine Must Provide

At each sizing iteration, the placement engine provides:
1. **Device coordinates** for all cells
2. **Wire length estimates** for all nets (from HPWL or Steiner tree)
3. **Parasitic estimates** for critical nets (R and C from wire length, layer, and width)
4. **Area** of the bounding rectangle
5. **Aspect ratio** of the bounding rectangle
6. **Symmetry quality metrics** (deviation from perfect symmetry for each group)
7. **LDE mismatch estimates** (WPE and LOD mismatch for each matched pair)

### 9.3 What the Placement Engine Must Accept

From the sizing optimizer:
1. **Updated device dimensions** (w_i, h_i) or shape functions {(w_i^k, h_i^k)} for each device
2. **Parasitic budgets** for critical nets (max allowed C, R per net)
3. **Matching tier updates** (if the optimizer changes the matching tier of a pair)
4. **Power dissipation updates** (if bias currents change, P_d changes, affecting thermal model)

### 9.A Analog-Algorithm Interface

1. The placement engine must support **incremental re-placement**: when a few device dimensions change, re-optimize locally without discarding the global placement. For topological sub-placements, this means re-running the ASF-B*-tree SA or selecting a different shape from the enhanced shape function.
2. The parasitic extraction interface must provide per-net R and C estimates, not just total HPWL. A simple model: R = R_sheet * length / width, C = C_per_unit_length * length, using metal layer parameters from the PDK.
3. The placement engine's API must be callable as a function (not just as a standalone tool) so it can be embedded in the sizing optimizer's inner loop.
4. For circuits where full re-placement per sizing iteration is too slow, a **cached placement** approach is used: the placement is fully optimized once, then incrementally adjusted as device dimensions change within a tolerance band. If dimensions change beyond the band, full re-placement is triggered.

---

## Summary of Solver Selection and Architecture

```
Input: Netlist with symmetry groups, device dimensions, hierarchy (HSMPG tree)

Phase 1: Decompose along hierarchy
  For each basic group in HSMPG tree:
    If group_size <= 5:  Plantage enumeration -> enhanced shape function
    If group_size <= 20: ASF-B*-tree SA -> enhanced shape function
    If group_size > 20:  Defer to Phase 2

Phase 2: Global placement
  If total_cells <= 80 and hierarchy is clear:
    HB*-tree SA (simultaneous optimization of all levels)
  Else:
    Nesterov GP with:
      - Symmetry groups from Phase 1 injected as rigid/semi-rigid blocks
      - 7-term objective (W + N + Sym + Area + LDE + Thermal + Phi)
      - Routability feedback (Section 5)

Phase 3: Detail placement
  ILP formulation (Section 4) with:
    - Hard symmetry constraints
    - MIP for DTI/forbidden-zone constraints
    - Common-centroid constraints
    - Proximity constraints
    - Variant selection (if applicable)

Phase 4: Verification
  - Symmetric routing feasibility check
  - Parasitic extraction and performance check
  - If performance fails: feed back to sizing outer loop (Section 9)

Output: Device coordinates, symmetry group encodings, enhanced shape functions
```

This architecture ensures that the strongest tools are applied where they are most effective: deterministic enumeration for small groups where exhaustive search is tractable, topological SA for medium groups where symmetry must be exact, and gradient-based optimization for large circuits where scalability matters. The hierarchy-aware decomposition prevents the combinatorial explosion that would make flat optimization of the entire circuit intractable.

**Sources cited throughout:**
- [ALS 1.1-1.4]: Balasa, "Device-Level Topological Placement with Symmetry Constraints," in Graeb (ed.), *Analog Layout Synthesis*, 2011.
- [ALS 2.1-2.5]: Lin and Chang, "Hierarchical Placement with Layout Constraints," in Graeb (ed.), *Analog Layout Synthesis*, 2011.
- [ALS 3.1-3.5]: Strasser, Eick, Graeb, Schlichtmann, "Deterministic Analog Placement by Enhanced Shape Functions," in Graeb (ed.), *Analog Layout Synthesis*, 2011.
- [ALS 6.1, 6.4]: Castro-Lopez, Roca, Fernandez, "Layout-Aware Sizing," in Graeb (ed.), *Analog Layout Synthesis*, 2011.
- [FOLD 5.3]: Lienig, "Primary Steps in Physical Design," in *Fundamentals of Layout Design for Electronic Circuits*, 2020.
- [00_ANALOG_PRINCIPLES]: Project-internal analog cross-cutting principles document.

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| COV-02 | Added well/implant density balance constraint (item 5) to Section 4.A ILP extensions | Checklist item 4.4 (well/implant density rules) had no concrete mechanism in the placement spec; imbalanced well density causes implant dose non-uniformity |
| CONTRA-01 | Added matched net wirelength vs. parasitic equalization note (item 2) to Section 2.1.A | Wirelength minimization objective conflicted with parasitic equalization jogs; placement should target symmetric pin-to-pin distances for matched nets rather than minimum total HPWL |
