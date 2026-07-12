# S2 -- Placement Engine: Detailed Specification

---

## 1. Global Placement (NLP)

### 1.1 Objective Function

```
min  W(v) + lambda*N(v) + tau*Sym(v) + eta*Area(v) + delta*LDE(v) + theta*Thermal(v) + alpha*Phi(G)
```

| Term | Function | Source | Differentiable? |
|------|----------|--------|-----------------|
| W(v) | Weighted-Average HPWL | ePlace (Lu et al.) | Yes |
| N(v) | Electrostatic potential energy (overlap) | ePlace | Yes (FFT gradient) |
| Sym(v) | Sum (y_i - y_j)^2 + (x_i + x_j - 2x_axis)^2 per pair | ePlace-A (Lin et al.) | Yes |
| Area(v) | WA_x(v) * WA_y(v) bounding box | ePlace-A | Yes |
| LDE(v) | WPE Vt-shift + LOD/STI current mismatch | Ou et al. DAC 2015 | Yes (sweep-line) |
| Thermal(v) | Delta-T across matched pairs (Green's function) | New | Yes |
| Phi(G) | GNN performance predictor (P(FOM < threshold)) | ePlace-AP (Lin et al.) | Yes (autodiff) |

**Solver:** Nesterov's accelerated gradient descent (from ePlace). Penalty weights lambda, tau, eta increase over iterations to drive solution toward feasibility. Symmetry is SOFT in GP (hard symmetry in GP increases area 17%, wirelength 9% -- ePlace-A Table I).

#### 1.1.A Analog Considerations

**Weight hierarchy enforcement.** The penalty weights must respect the analog hierarchy of needs (see `00_ANALOG_PRINCIPLES.md` Section 5):

```
delta (LDE matching)  >= 100 * eta (Area)
tau   (Symmetry)      >=  50 * eta (Area)
theta (Thermal)       >=  10 * eta (Area)
alpha (Performance)   >=   5 * eta (Area)
```

This ensures that the optimizer never trades matching quality for area. In practice, this means the optimizer will produce layouts that are 10-30% larger than a matching-unaware optimizer but meet analog specifications.

**LDE term must include all systematic mismatch sources.** The current formulation includes WPE and LOD/STI. The following additional terms should be modeled:

| Mismatch Source | Equation | Typical Magnitude | How to Include |
|---|---|---|---|
| WPE | `dVth = sum a_k * exp(-d_k/lambda_k)` | 10-50 mV within 5 um of well edge | Compute per matched pair; penalize `|dVth_i - dVth_j|` |
| LOD/STI | `f(SA,SB) = 1 + K1*(1/SA+1/SB) + K2*(1/SA^2+1/SB^2)` | Up to 13% Id | Penalize `|f_i(SA,SB) - f_j(SA,SB)|` |
| Thermal gradient | `dT = |T(x_i,y_i) - T(x_j,y_j)|` via Green's function | 1-2 mV/C * dT | Penalize `dT` for matched pairs |
| Mechanical stress gradient | Piezoresistivity: `dR/R = pi_L*sigma_L + pi_T*sigma_T` | Several % gm | Penalize distance from die center for exceptional pairs [Hastings, Ch. 8, Ch. 13] |

**Pelgrom-aware device sizing check.** At each GP iteration, verify that each matched pair's area (W*L) is sufficient for the target matching tier:
```
sigma(dVth) = A_VT / sqrt(W*L)
Required W*L >= (A_VT / sigma_target)^2
```
If a pair's devices are undersized, flag for S0 re-generation in the outer loop.

#### 1.1.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| LDE weight (delta) must be >= 100x area weight (eta) at all GP iterations | GP solver | Matching is the #1 priority; area is #5 |
| LDE term must compute per-matched-pair mismatch, not an aggregate sum | GP solver | One badly mismatched pair can sink the circuit; average LDE is misleading |
| Thermal term must use per-device power from S1 constraints (not uniform power assumption) | GP solver | Power devices dissipate 100-1000x more than bias devices |
| At convergence, report per-matched-pair LDE mismatch for downstream verification | GP solver | S4 needs this for the matching report |

### 1.2 System Signal Flow (Optional)

If a sigpath file is provided (or automatically extracted), add a signal-flow regularity term that penalizes deviation from a "schematic-like" layout -- forward signal path left-to-right, feedback bottom-to-top (MAGICAL ICCAD 2020, Zhu et al.).

#### 1.2.A Analog Considerations

**Signal flow is not optional for analog.** While the spec marks this as optional, analog circuits strongly benefit from schematic-like placement [Lienig, Section 4.3.2; Hastings, Ch. 15, Section 15.2]. If no sigpath file is provided, S1 should attempt to extract one from the netlist topology by identifying:
- Input pads (placed on the left or top)
- Output pads (placed on the right or bottom)
- Bias networks (placed centrally, accessible from all signal paths)
- Feedback paths (routed separately from forward paths)

This preserves the meet-in-the-middle design philosophy where the layout reflects the schematic topology [Lienig, Section 4.3.2].

### 1.3 Routability Estimation (S3->S2 Feedback)

After GP converges, estimate per-region congestion:
- Partition layout into tiles.
- For each tile: demand = sum of HPWL-based wire density passing through. Capacity = available tracks x layers.
- If demand/capacity > congestion_threshold in any tile, add congestion penalty and re-run GP.
- Alternative: run GeniusRoute VAE to predict routing density; feed as additional GP term.

#### 1.3.A Analog Considerations

**Routing channels must be explicitly reserved.** Unlike digital placement where routing resources are distributed uniformly, analog placement requires explicit routing channels [Hastings, Ch. 15, Sections 15.2 and 15.4.3]:
- Primary routing channels should hold space for 10-20% of all top-level signals
- Even the narrowest feeder channel should allow at least 5 signals
- Channel width: `W_C = N * P_r + S_m` where N is the number of signals, P_r is the routing pitch, and S_m is the metal spacing [Hastings, Ch. 15, Section 15.4.2]

**Choke points are expensive to fix late [Hastings, Ch. 15, Section 15.2].** The congestion estimator must flag narrow routing channels between abutting blocks. If a choke point is not discovered until routing has begun, significant rework may be required.

---

## 2. Detailed Placement (ILP)

### 2.1 Formulation (from ePlace-A)

```
min  Sum_e HPWL_D(e) + mu * Area_D(v)

subject to:
  Non-overlap:      x_j + w_j/2 <= x_k - w_k/2,  for all (j,k) in overlap pairs
  Hard symmetry:    (x_q1 + x_q2)/2 = x_axis,  y_q1 = y_q2
  Self-symmetry:    x_r = x_axis
  Bottom alignment: y_b1 - h_b1/2 = y_b2 - h_b2/2
  Central alignment: x_vc1 = x_vc2
  Ordering:         x_o1 + w_o1/2 <= x_o2 - w_o2/2
  Device flipping:  x_hat_i = x_i - w_i/2 + xpin_i*(1-fx_i) + (w_i - xpin_i)*fx_i
  Aspect ratio:     W/H in [r_min, r_max]
  Integer grid:     x_i in N
```

#### 2.1.A Analog Considerations

**Additional ILP constraints for analog:**

```
  WPE equalization:    |d_well_i - d_well_j| <= epsilon_wpe,  for all matched pairs (i,j)
      where d_well is the distance from device center to nearest well edge

  Thermal proximity:   |T(x_i,y_i) - T(x_j,y_j)| <= dT_max,  for all matched pairs
      dT_max = 0.5 C for moderate, 0.1 C for exceptional

  Die-center proximity: sqrt((x_i - x_center)^2 + (y_i - y_center)^2) <= R_max
      R_max = die_diagonal/4 for exceptional, die_diagonal/2 for moderate

  Power device separation: sqrt((x_matched - x_power)^2 + (y_matched - y_power)^2) >= D_min
      D_min = 1 um/mW for exceptional [Hastings, Ch. 8, Rule 13]

  Guard ring reservation: bounding_box inflated by guard_ring_width on all sides
      Guard ring area must be allocated BEFORE area optimization

  Substrate isolation:  min_distance(analog_block, digital_block) >= isolation_spacing
      For deep N-well isolation: spacing per PDK rules
```

**Linked flipping for matched groups.** The ILP must link flipping variables for all devices in a matching group:
```
  fx_i = fx_j,  for all (i,j) in same matching group
  fy_i = fy_j,  for all (i,j) in same matching group
```
Flipping one device but not its partner destroys orientation matching [Hastings, Ch. 13].

#### 2.1.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| WPE equalization constraint must be HARD for moderate and exceptional matching tiers | ILP solver | WPE mismatch cannot be corrected post-placement |
| Thermal proximity constraint must be HARD for exceptional, SOFT for moderate | ILP solver | 1 C gradient = 1-2 mV offset; acceptable for moderate but not exceptional |
| Flipping variables must be linked within matching groups | ILP solver | Independent flipping destroys orientation matching |
| Guard ring area must be reserved as a non-negotiable area overhead | ILP solver | Guard rings cannot be added as an afterthought |

### 2.2 Solver Selection by Problem Size

| Cell count | Solver | Rationale |
|------------|--------|-----------|
| < 8 | Enumerative (exhaustive sequence-pair) | Globally optimal for tiny blocks |
| 8-20 | ILP (Gurobi/CPLEX) | Exact solution, handles all constraints |
| 20-100 | Analytical GP (ePlace-A) + ILP detail | Scalable GP, exact legalization |
| > 100 | SA with analytical warm-start | SA handles very large search spaces |

### 2.3 Flipping Rules

- Device flipping (horizontal/vertical) is a binary variable in the ILP.
- **NEVER flip a device in a matching group** unless its matched partner is flipped identically. The ILP constraint links flipping variables for matched pairs.
- Flipping reduces wirelength by 3-5% (ePlace-A Table IV).

---

# S3 -- Routing Engine: Detailed Specification

---

## 1. Routing Pipeline

```
Input: Placed layout + constraint file + (optional) reference manual layouts for training
       |
  +---------------------+
  | VAE routing guide    |  <- trained on manual layouts (per net type)
  | generation           |     Output: 64x64 probability maps per net class
  +----------+----------+
             |
  +---------------------+
  | Symmetry constraint  |  <- Edmonds' blossom maximum-weight matching
  | allocation           |     Output: per-net-pair symmetry variant assignment
  +----------+----------+
             |
  +---------------------+
  | Pin access           |  <- preferred direction sets per pin
  | assignment           |     (avoid active-region DRC violations)
  +----------+----------+
             |
  +---------------------+
  | Pin clustering       |  <- split symmetric nets into L/R subsets,
  |                      |     form max fully-symmetric clusters
  +----------+----------+
             |
  +---------------------+
  | Constraint-aware     |  <- A* search with hybrid priority queue
  | A* routing           |     Cost = wire + via + history + violate + compete
  | + rip-up/reroute     |     Symmetry: simultaneous dual-side pathfinding
  +----------+----------+
             |
  +---------------------+
  | Post-route           |
  | verification         |
  |  - Parasitic match   |  <- |R_A - R_B| < threshold for matched pairs
  |  - EM check          |  <- J < J_max per segment; widen if exceeded
  |  - IR drop           |  <- resistive network solve; reinforce if needed
  |  - Shielding insert  |  <- grounded adjacent tracks for sensitive nets
  |  - Min-step fix      |  <- metal patching for OPC compliance
  +----------+----------+
             |
  Output: Routed layout
```

### 1.A Analog Considerations

**Net routing order must prioritize analog-critical nets.** The current priority formula:
```
PR_i = 0.1 * HPWL_i + 2 * |P_i| + 100 * d_i + 50 * z_i
```
should be augmented with net-type-specific terms:

```
PR_i = 0.1 * HPWL_i + 2 * |P_i| + 100 * d_i + 50 * z_i
     + 200 * is_matched(i) + 150 * is_high_impedance(i) + 100 * is_feedback(i)
```

Route in this order:
1. Matched differential nets (route simultaneously as pairs)
2. High-impedance sensitive nets (shortest possible path on lowest-C metal)
3. Feedback/compensation nets (minimize added parasitic C)
4. Current bias lines (low resistance, shielded)
5. Power/ground (wide, multi-via, EM-compliant)
6. General signals
7. Noisy digital (last, with shielding from sensitive nets)

This order ensures that the most constraint-sensitive nets get first pick of routing resources [Hastings, Ch. 15, Section 15.4.3: "route wide signals first"].

**Matched net routing protocol.** For every matched net pair, the router must:
1. Route both nets simultaneously (dual-side A* pathfinding)
2. Use the same metal layer(s) for both nets
3. Use identical via stacks (same number of cuts, same via type)
4. Match total wire length to within ~5% (moderate) or ~1% (exceptional)
5. After routing, compute `|R_A - R_B|` and `|C_A - C_B|`
6. If mismatch exceeds tolerance, add jogs or dead-end branches to the shorter/lower-C net to equalize [Hastings, Ch. 8, Rule 10]
7. Maintain lead-to-adjacent-metal spacing >= 2-3x interlayer dielectric thickness for matched nets [Hastings, Ch. 8]

### 1.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Matched nets must be routed simultaneously (not sequentially) | A* router | Sequential routing of matched nets prevents optimal parasitic equalization |
| Net priority must include analog-specific terms (matched, high-Z, feedback) | Priority queue | Analog-critical nets need first pick of routing resources |
| Post-route parasitic matching verification is a HARD gate: if matched nets fail tolerance, rip-up and re-route | Post-route verifier | Parasitic asymmetry directly creates offset |

## 2. Net Routing Order (Priority Queue)

```
PR_i = 0.1 * HPWL_i + 2 * |P_i| + 100 * d_i + 50 * z_i
```

(See 1.A above for analog-augmented version.)

## 3. A* Cost Function

```
Cost = Cost_wire                    (wirelength)
     + Cost_via                     (via penalty)
     + Cost_history                 (negotiation-based congestion)
     + Cost_violate                 (routing outside VAE-predicted region)
     + Cost_compete                 (routing in region demanded by other nets)
```

### 3.A Analog Considerations

**Additional cost terms for analog routing:**

```
Cost_analog = Cost_matched_gate_keepout     (penalty for routing over matched transistor gates)
            + Cost_noise_coupling           (penalty for routing noisy net adjacent to sensitive net)
            + Cost_parasitic_budget         (penalty for exceeding net's parasitic C budget)
            + Cost_shielding_absence        (penalty for unshielded sensitive-to-noisy crossing)
```

**Cost_matched_gate_keepout:** Infinite penalty for routing any metal over the active gate region of a matched transistor. This prevents hydrogenation blocking mismatch (up to 20% Id mismatch [Hastings, Ch. 13]).

**Cost_noise_coupling:** For a noisy net routing adjacent to a sensitive net:
```
Coupling_penalty = C_coupling * R_victim * dV/dt_aggressor
```
where `C_coupling` is the estimated coupling capacitance (proportional to parallel run length / spacing), `R_victim` is the victim net impedance, and `dV/dt_aggressor` is the estimated slew rate [Hastings, Ch. 15, Section 15.4.4].

**Cost_parasitic_budget:** For each net with a parasitic C budget (from S1):
```
If C_accumulated > C_budget: penalty = 1000 * (C_accumulated - C_budget) / C_budget
```

### 3.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Matched-gate keep-out zones must be treated as infinite-cost routing blockages | A* cost function | Hydrogenation blocking is not tradeable |
| Noise coupling cost must be evaluated for every noisy-to-sensitive net adjacency | A* cost function | `dV = C * R * dV/dt` can produce volt-level interference [Hastings, Ch. 15] |
| Parasitic budget violations must trigger rip-up, not just soft penalty | Post-route verifier | Budget exceedance degrades BW/PM which may not be recoverable |

## 4. Wire Sizing and Via Rules

```
For each net n:
  w_min(n) = max(PDK.min_width, I_max(n) / J_max(layer))
  N_cuts(n) = max(1, ceil(I_max(n) / I_max_per_cut))
  
  If n.class == "power" or n.class == "ground":
    w_min(n) = max(w_min(n), PDK.power_min_width)
    N_cuts(n) = max(N_cuts(n), 4)
  
  The router uses NDR (non-default rules) per net:
    custom width, custom spacing, preferred layers.
```

### 4.A Analog Considerations

**EM rules apply at every point along a lead [Hastings, Ch. 15, Section 15.4.4].** A lead cannot be necked down even briefly. Minimum width for electromigration:
```
W_min = I_max / (J_max * t_min)
```

**Temperature derating (Black's Law) [Hastings, Ch. 15, Section 15.4.4]:**
```
D = exp[(Ea/k) * (1/T_d - 1/T_r)]^(1/n)
```
where Ea = 0.5 eV (pure Al), ~0.7 eV (Cu-doped Al), ~0.9 eV (Cu). A 25 C temperature increase from 100 C to 125 C reduces the safe current to 58% of the 100 C rating (for aluminum with n=2).

**Via placement at bends [Hastings, Ch. 15, Section 15.4.4]:** Current crowds toward the inside corner at 90-degree bends. Via arrays at corners see drastically nonuniform current. The router should:
- Replace 90-degree bends with pairs of 135-degree angles
- Place via arrays in straight sections of leads, not at bends
- Fill wide-lead layer transitions with as many vias as possible

**Blech length (immortality condition) [00_ANALOG_PRINCIPLES, Section 4.1]:**
```
J_critical * L < (J*L)_Blech ~ 2000-5000 A/cm for Cu
```
Short intra-cell straps below the Blech length are "immortal" and can be granted EM waivers. The router should automatically identify and exempt such segments.

### 4.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Wire width must satisfy EM at every point, including necked-down regions | Router | EM is a worst-case-at-every-point constraint, not an average |
| Via arrays must be placed in straight lead sections, not at bends | Router | Current crowding at bends causes early EM failure [Hastings, Ch. 15] |
| Temperature-derated J_max must be used (worst-case operating temperature) | Router | MTTF halves per ~10 C increase [Black's equation] |
| Blech-immortal segments should be automatically identified and exempted from EM width requirements | Router | Avoids unnecessary area overhead for short intra-cell straps |

---

# S4 -- Sign-off & Verification: Detailed Specification

---

## 1. Verification Sequence

```
Step 1: DRC (Calibre nmDRC)
  -> If violations: feed back to S3 (inner loop 2)
  -> Must be clean before proceeding

Step 2: LVS (Calibre nmLVS)
  -> Graph isomorphism: extracted netlist === schematic
  -> Any error is a hard failure -- debug and fix in S3

Step 3: Antenna check
  -> Per net, per layer: antenna_ratio = metal_area / gate_area
  -> If ratio > PDK limit:
     Fix 1: route jumper to higher metal (break the antenna)
     Fix 2: insert protection diode at gate (reverse-biased to substrate)
  -> Fixes must not break symmetry

Step 4: Density fill
  -> Per layer, per tile: compute metal density
  -> If density < PDK.min_density: insert dummy fill metal
  -> Keep-out zones: sensitive nets (from S1 net classification),
     high-impedance nodes, capacitor top plates
  -> If density > PDK.max_density: flag (rare, usually routing is sparse)

Step 5: PEX (Calibre PEX or StarRC)
  -> R + C + CC extraction (full parasitic netlist)
  -> This is the most expensive step (minutes to hours for large circuits)

Step 6: Post-layout SPICE simulation (Cadence Spectre)
  -> Evaluate all performance metrics on the extracted netlist
  -> Compare each metric to spec:
     metric_margin = (metric_value - spec_value) / spec_value

Step 7: Decision
  -> All margins >= 0: PASS -> output GDSII
  -> Any margin < 0:
     If |margin| < 10%: tighten constraints, re-run S2->S4
     If |margin| 10-30%: adjust GP weights, re-run S2->S4
     If |margin| > 30%: exit to circuit design (re-sizing needed)
```

### 1.A Analog Considerations

**Thermal gradient validation for matched pairs.** For each matched pair, S4 must verify that the thermal gradient across the pair does not exceed the tolerance from S1's LDE bounds:

```
For each matched pair (device_i, device_j):
  dT = |T(device_i) - T(device_j)|    (from S2's thermal_map)
  dT_tolerance = LDE_bound.thermal_budget_C    (from S1's constraint file)
  
  If dT > dT_tolerance:
    FLAG: thermal gradient violation for matched pair
    Report dT and dT_tolerance in the matching report
    Trigger outer loop re-placement with increased theta (thermal weight)
```

This validation closes the gap where S2 computes a thermal map during placement but S4 does not check thermal gradients at sign-off.

**DRC is the contract between layout designer and fabrication process [Hastings, Ch. 3, Section 3.2].** Design rules encode the minimum dimensions and spacings that photolithography, etching, diffusion, and depletion physics can reliably achieve. Violating a design rule means the fabricated circuit may not function correctly. DRC runtime can reach up to one week for modern designs; most companies require completion in less than one day [Lienig, Section 5.4.5].

**Dummy/false errors [Hastings, Ch. 3; Lienig, Section 5.4.1].** Both abstractions in the two-stage constraint mapping (physics -> formal rules -> tool formats) introduce approximations. When a technological constraint cannot be exactly modeled, a safety margin is added. Layouts may fail DRC despite meeting the true manufacturing requirement. Experienced engineers may waive these, but doing so demands deep understanding of the underlying technology.

**Antenna fixes must preserve symmetry.** When antenna violations occur on one net of a matched pair, the fix (jumper or protection diode) must be applied to BOTH nets of the pair, even if only one violates. Otherwise the fix introduces parasitic asymmetry that degrades matching.

**Density fill must respect matching [Hastings, Ch. 13, Rules 17-18].** The density fill engine must:
- Use pseudolayers to define keep-out zones over matched transistor gates
- Exceptional: keep-out extends 5 um beyond active gate in all directions
- Ensure the metal fill pattern is symmetric around matched devices
- Asymmetric fill causes up to 1% mismatch from hydrogenation blocking effects

**Post-layout simulation should include [Lienig, Section 5.4.3; Hastings, Ch. 15, Section 15.5]:**
- Nominal corner simulation (TT, 25 C)
- Process corners (FF, SS, SF, FS at -40 C, 25 C, 125 C) for Phase 3+
- Monte Carlo mismatch analysis (100+ samples) for Phase 3+
- Worst-case performance across corners defines the true margin

**Hierarchical verification [Lienig, Section 5.4.6].** Memory blocks and IP elements should be compared hierarchically to reduce LVS runtime. Analog blocks and macro cells may use flat representation. This can drastically reduce debugging time.

### 1.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Antenna fixes must be applied symmetrically to matched net pairs | S4 antenna fixer | Asymmetric fixes introduce parasitic mismatch |
| Density fill must use pseudolayer-defined keep-out zones for matched devices | S4 fill engine | Hydrogenation blocking: up to 20% Id mismatch for metal over gate; up to 1% from asymmetric fill [Hastings, Ch. 13] |
| LVS must be zero topological errors before proceeding to PEX | S4 flow | One LVS error can mask others [Hastings, Ch. 15, Section 15.5] |
| Post-layout simulation must report matching-specific metrics (offset, CMRR, PSRR) separately | S4 report | Matching failures require different remediation than performance failures |
| Phase 3+ must include Monte Carlo mismatch simulation | S4 SPICE | Nominal simulation does not capture mismatch effects |

## 2. Performance Report Format

```json
{
  "circuit": "telescopic_OTA",
  "pdk": "TSMC40nm",
  "status": "PASS",
  "metrics": [
    {"name": "DC_gain", "spec": 40.0, "unit": "dB", "achieved": 42.3, "margin_pct": 5.75},
    {"name": "UGB", "spec": 500.0, "unit": "MHz", "achieved": 487.0, "margin_pct": -2.6},
    {"name": "phase_margin", "spec": 60.0, "unit": "deg", "achieved": 72.1, "margin_pct": 20.2},
    {"name": "offset", "spec": 1.0, "unit": "mV", "achieved": 0.3, "margin_pct": 70.0},
    {"name": "CMRR", "spec": 60.0, "unit": "dB", "achieved": 85.2, "margin_pct": 42.0}
  ],
  "drc_violations": 0,
  "lvs_errors": 0,
  "antenna_fixes": 2,
  "fill_tiles_modified": 47,
  "total_runtime_s": 182.4,
  "iterations": {"inner_loop_1": 1, "inner_loop_2": 0, "outer_loop": 0}
}
```

### 2.A Analog Considerations

**The performance report must include a matching section:**

```json
{
  "matching_report": [
    {
      "pair": ["M1_diff_p", "M2_diff_n"],
      "tier": "moderate",
      "estimated_dVth_mV": 0.82,
      "estimated_dId_pct": 0.34,
      "wpe_dVth_mV": 0.12,
      "lod_dId_pct": 0.08,
      "parasitic_dR_ohm": 0.5,
      "parasitic_dC_fF": 0.02,
      "orientation_chi_match": true,
      "dummy_coverage": "full",
      "cc_pattern": "CC_2D",
      "distance_D_um": 3.4,
      "guard_ring": "P_PLUS, complete"
    }
  ],
  "thermal_map": {
    "max_dT_across_matched_pair_C": 0.3,
    "worst_pair": ["M1_diff_p", "M2_diff_n"],
    "power_device_nearest_um": 120.0
  },
  "parasitic_budget_report": [
    {
      "net": "vout_p",
      "class": "high_impedance",
      "C_budget_fF": 5.2,
      "C_actual_fF": 3.8,
      "margin_pct": 26.9
    }
  ]
}
```

---

# Interface & Data Model Specification

---

## Stage-to-Stage Data Flow

```
                    Cell library              Constraint file
Input --------> S0 ----------------> S1 ----------------------> S2
              |                 |                         |
              | cell metadata   | pattern recommendations  | placed layout
              | (feedback)      | (feedback to S0)        | (pin positions)
              |                 |                         |
              +-----------------+                         v
                                                         S3
                                                         |
                                                         | routed layout
                                                         | (GDSII geometry)
                                                         v
                                                         S4
                                                         |
                                                         | verified GDSII
                                                         | + performance report
                                                         v
                                                       Output
```

## Key Data Structures

### Cell Record (S0 output)
```
CellRecord:
  name: string
  device_type: enum (NMOS, PMOS, NCAP, PCAP, RES, ...)
  W_eff: float (um)
  L_eff: float (um)
  finger_count: int
  finger_width: float (um)
  pattern_type: enum (SINGLE, INTERDIGITATED, CC_1D, CC_2D, CLUSTERED)
  dummy_count: int (per edge)
  guard_ring_type: enum (P_PLUS, N_PLUS, DEEP_NWELL, COMBINED)
  guard_ring_width: float (um)
  bounding_box: Rectangle (includes guard ring inflation)
  pins: map<string, list<(layer, Rectangle)>>
  orientation: enum (DEFAULT, ROTATED_90, ROTATED_180, ROTATED_270)
  internal_parasitics:
    R_gate: float (Ohm)
    C_drain: float (fF)
    C_source: float (fF)
  matching_group_id: string (nullable)
```

#### Cell Record Analog Enrichment

The following fields must be added to support analog-specific requirements:

```
  matching_tier: enum (NONE, MINIMAL, MODERATE, EXCEPTIONAL)
  orientation_chi: float (orientation metric, 0 = perfectly balanced)
  dummy_type: enum (NONE, HALF, FULL, EXTENDED)
  dummy_moat_extension_um: float (distance moat extends beyond last active gate)
  sa_sb_values: {SA: float, SB: float} (gate-to-OD-edge distances for LOD equalization)
  wpe_distance_to_well_edge_um: list<float> (left, right, top, bottom)
  metal_over_gate_blocked: bool
  thermal_power_mW: float (from DC operating point)
  bias_current_mA: float (per-device DC bias current Id from .op data; needed by S3 for EM wire sizing and by S1 for parasitic budget computation)
  overdrive_voltage_mV: float (per-device overdrive voltage Vgs-Vth from .op data; needed by S1 for Pelgrom current mismatch calculation sigma^2(dId)/Id^2 = 4*sigma^2(dVth)/(Vgs-Vth)^2 + sigma^2(d_beta/beta), and for subthreshold guard Rule 4: avoid Veff < 100 mV)
  current_flow_direction: enum (LEFT_TO_RIGHT, RIGHT_TO_LEFT, BIDIRECTIONAL)
  seebeck_cancellation: bool (true if even segments with antiparallel current flow)
  kelvin_connected: bool (for precision resistors)
  esd_protection_type: enum (NONE, GGNMOS, GCNMOS, BTNMOS, ACTIVE_FET, PNP, MVSCR)
```

### Constraint Record (S1 output)

#### Constraint Record Analog Enrichment

The constraint file must include the following analog-specific fields beyond the original specification:

```
ConstraintRecord:
  symmetry_groups: list<SymmetryGroup>
    SymmetryGroup:
      axis: SymmetryAxis
      pairs: list<MatchedPair>
        MatchedPair:
          device_a: string
          device_b: string
          matching_type: enum (MIRROR, CROSS, SELF, PARTIAL)
          matching_tier: enum (MINIMAL, MODERATE, EXCEPTIONAL)
          max_dVth_mV: float (from Pelgrom + tier)
          max_dId_pct: float (from Pelgrom + overdrive)
  
  net_classifications: list<NetClassification>
    NetClassification:
      net_name: string
      net_class: enum (DIFFERENTIAL, CLOCK, POWER, GROUND, SENSITIVE_ANALOG,
                       HIGH_IMPEDANCE, MATCHED, FEEDBACK, BIAS, NOISY_DIGITAL, GENERAL)
      parasitic_C_budget_fF: float (nullable; computed from f_target and R_node)
      parasitic_R_budget_ohm: float (nullable; for supply and bias nets)
      shielding_required: bool
      preferred_layers: list<string> (e.g., ["M3", "M4"] for lowest-C routing)
      max_coupling_fF: float (nullable; max allowed coupling to noisy nets)
  
  thermal_tags: list<ThermalTag>
    ThermalTag:
      device_name: string
      power_mW: float
      is_power_device: bool
      min_distance_to_matched_um: float (1 um/mW for exceptional)
  
  lde_bounds: list<LDEBound>
    LDEBound:
      matched_pair: (string, string)
      min_well_edge_distance_um: float (from WPE model)
      max_sa_sb_mismatch_um: float (from LOD model)
      max_wpe_dVth_mV: float
      max_lod_dId_pct: float
  
  current_flow_directions: list<CurrentFlowTag>
    CurrentFlowTag:
      device_name: string
      direction: enum (LEFT_TO_RIGHT, RIGHT_TO_LEFT, UP_TO_DOWN, DOWN_TO_UP)
      
  bias_currents: list<BiasCurrentTag>
    BiasCurrentTag:
      device_name: string
      Id_mA: float (DC bias current from .op data)
      Vgs_minus_Vth_mV: float (overdrive voltage from .op data)

  esd_requirements: list<ESDRequirement>
    ESDRequirement:
      pad_name: string
      protection_type: enum (PRIMARY, SECONDARY, CDM_CLAMP)
      max_trigger_voltage_V: float
      ecgr_required: bool
```

### Placed Layout (S2 output)
```
PlacedLayout:
  cells: list<PlacedCell>
    PlacedCell:
      cell_ref: CellRecord
      position: (x, y) in grid units
      flipped_h: bool
      flipped_v: bool
  symmetry_axes: list<SymmetryAxis>
    SymmetryAxis:
      axis_x: float
      groups: list<SymmetryGroup>
  bounding_box: Rectangle
  estimated_congestion_map: 2D array (optional)
```

#### Placed Layout Analog Enrichment

```
  lde_mismatch_report: list<LDEMismatchEntry>
    LDEMismatchEntry:
      pair: (string, string)
      wpe_dVth_mV: float
      lod_dId_pct: float
      thermal_dT_C: float
      mechanical_stress_region: enum (CENTER, EDGE, CORNER)
  
  thermal_map: 2D array (temperature in C at each grid point)
  
  metal_keepout_zones: list<Rectangle> (populated from S0's hydrogenation_safe_zone metadata during placement; consumed by S4's density fill engine as keep-out zones for dummy metal generation)

  guard_ring_coverage: list<GuardRingEntry>
    GuardRingEntry:
      device: string
      ring_type: enum (P_PLUS, N_PLUS, DEEP_NWELL, COMBINED)
      complete: bool
      width_um: float
```

### Routed Layout (S3 output)
```
RoutedLayout:
  placed_layout: PlacedLayout
  routes: list<Route>
    Route:
      net_name: string
      net_class: enum (DIFFERENTIAL, CLOCK, POWER, GROUND, SUBSTRATE, GUARD_RING,
                       MATCHED_PAIR, HIGH_IMPEDANCE, COMPENSATION, ESD_BUS,
                       SENSITIVE, GENERAL)
      segments: list<Segment>
        Segment:
          layer: string
          start: (x, y)
          end: (x, y)
          width: float
      vias: list<Via>
        Via:
          layer_from: string
          layer_to: string
          position: (x, y)
          cuts: int
  symmetry_degree: float (d_SYM metric)
  parasitic_estimates: map<net_name, {R: float, C: float}>
```

#### Routed Layout Analog Enrichment

```
  metal_keepout_zones: list<Rectangle> (carried forward from PlacedLayout; consumed by S4's density fill engine as keep-out zones for dummy metal generation to prevent hydrogenation blocking of matched devices)

  matched_net_parasitic_comparison: list<MatchedNetParasiticEntry>
    MatchedNetParasiticEntry:
      net_a: string
      net_b: string
      R_a_ohm: float
      R_b_ohm: float
      C_a_fF: float
      C_b_fF: float
      dR_pct: float
      dC_pct: float
      within_tolerance: bool
  
  shielding_report: list<ShieldingEntry>
    ShieldingEntry:
      sensitive_net: string
      shielded: bool
      shield_type: enum (ADJACENT_GROUND, ELECTROSTATIC_PLATE, NONE)
      unshielded_crossings: int
  
  em_compliance: list<EMEntry>
    EMEntry:
      net: string
      segment_id: int
      J_actual_mA_per_um: float
      J_max_mA_per_um: float
      compliant: bool
      blech_immortal: bool
  
  parasitic_budget_compliance: list<ParasiticBudgetEntry>
    ParasiticBudgetEntry:
      net: string
      C_budget_fF: float
      C_actual_fF: float
      within_budget: bool
```

---

## Data Gaps: What the Current Interfaces Drop

The following data is required by analog practice but not currently carried by the interface data structures:

| Missing Data | Where Needed | Why It Matters | Resolution |
|---|---|---|---|
| Per-device DC bias current (Id) | S3 wire sizing, S4 EM check | Cannot size wires without knowing current; cannot check EM | Add to ThermalTag or create separate BiasCurrentTag in S1 output |
| Per-net impedance (R_node) | S1 parasitic budget computation, S3 noise coupling check | `C_budget = 1/(2*pi*f*R_node)`; `dV = C*R*dV/dt` | Compute from .op data in S1; add to NetClassification |
| Per-device overdrive voltage (Vgs-Vth) | S1 matching tier inference, Pelgrom current mismatch | `sigma^2(dId)/Id^2 = 4*sigma^2(dVth)/(Vgs-Vth)^2 + sigma^2(d_beta/beta)` | Extract from .op data; add to CellRecord metadata |
| Matching tier per pair | S0 cell generation, S2 constraint strength | Different tiers require different CC patterns, dummy counts, area budgets | Add to MatchedPair in ConstraintRecord |
| ESD protection requirements per pad | S0 ESD cell generation, S4 compliance check | Every pin except substrate ground needs primary protection [Hastings, Ch. 14] | Add ESDRequirement to ConstraintRecord |
| Current flow direction per device | S0 Seebeck cancellation, S3 matched routing | Even number of segments with antiparallel flow cancels thermoelectric EMF [Hastings, Ch. 8] | Add to CellRecord and ConstraintRecord |
| Package/die stress model | S2 mechanical stress placement | PMOS is ~2x more stress-sensitive than NMOS [Hastings, Ch. 13] | Add package CTE and die position to input specification |
| Keep-out pseudolayer for density fill | S4 fill engine | Dummy metal over matched gates causes up to 20% Id mismatch | Add metal_over_gate_blocked flag to CellRecord; generate pseudolayer |

---

## Implementation Priority Order

For a team building this engine, the recommended implementation order:

```
Phase 1 -- Minimum viable pipeline (proof of concept):
  S0: Single-finger cells with guard rings (no CC/interdigitation)
  S1: Pattern-library constraint extraction (no GNN)
  S2: SA placement with soft symmetry
  S3: A* routing with mirror symmetry only
  S4: DRC + LVS only
  -> Target: DRC/LVS-clean layout for a 5T-OTA

Phase 2 -- Matching by construction:
  S0: Multi-finger decomposition + ABBA interdigitation + dummies
  S1: GNN symmetry extraction
  S2: ePlace-A analytical placement (GP + ILP)
  S3: All four symmetry variants + pin clustering
  S4: DRC + LVS + PEX + SPICE
  -> Target: Post-layout performance within 15% of schematic for OTAs

Phase 3 -- Physics-aware optimization:
  S0: CC 2-D arrays + Pelgrom sizing + passive array generator
  S1: LDE sensitivity + net classification + electrical constraints
  S2: LDE + thermal + performance terms in GP
  S3: VAE routing guidance + parasitic matching + EM/IR verification
  S4: Full sign-off with antenna + density fill
  -> Target: Post-layout performance within 5% of schematic

Phase 4 -- Advanced node and production:
  S0: FinFET template-and-grid (LAYGO-style)
  S1: LLM-assisted constraint generation (LLANA-style)
  S2: Routability feedback loop + multi-candidate Pareto generation
  S3: RL-guided net ordering + performance-driven routing (PARoute2-style)
  S4: Process-corner and Monte Carlo verification
  -> Target: Silicon-proven on a FinFET ADC tape-out
```

### Implementation Priority Analog Considerations

**Phase 1 must still produce functional analog layouts.** Even the minimum viable pipeline must:
- Generate guard rings (latchup is destructive)
- Enforce orientation uniformity within matching groups
- Route power/ground with EM-compliant widths
- Pass both DRC and LVS (zero errors)

Without these, the Phase 1 output is not a valid analog layout, merely a geometric exercise.

**Phase 2 is the critical phase for analog.** The transition from Phase 1 to Phase 2 is where the engine becomes useful for real analog circuits. The key additions are:
- Multi-finger decomposition with ABBA interdigitation (cancels linear gradients)
- Dummy devices (equalizes edge effects)
- GNN symmetry extraction (automates constraint identification)
- Four symmetry routing variants (handles all matching topologies)
- PEX + SPICE simulation (validates that matching is preserved through layout)

**Analog-specific fields must be added in Phase 2, not Phase 3.** The enriched data structures defined above (matching tier, parasitic budgets, thermal tags, LDE bounds) are needed as soon as the engine targets matching. Deferring them to Phase 3 means Phase 2 produces layouts without proper matching constraints.

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| GAP-01 | Added `bias_current_mA` field to CellRecord Analog Enrichment and `BiasCurrentTag` to ConstraintRecord | Per-device DC bias current (Id) was missing from S0 output; S3 needs it for EM wire sizing, S1 for parasitic budget computation |
| GAP-02 | Added `overdrive_voltage_mV` field to CellRecord Analog Enrichment and `Vgs_minus_Vth_mV` to BiasCurrentTag | Per-device overdrive voltage (Vgs-Vth) was missing; S1 needs it for Pelgrom current mismatch calculation |
| GAP-03 | Added `net_class: enum` field to the Route record in RoutedLayout | Net classification did not flow from S1 to S3 as structured input; S3 had to re-parse S1's constraint file to determine net class for each route |
| GAP-04 | Added `metal_keepout_zones: list<Rectangle>` to both PlacedLayout and RoutedLayout analog enrichments | S0's hydrogenation_safe_zone rectangles were not carried through placement/routing to S4's density fill engine |
| GAP-07 | Added thermal gradient validation step to S4 verification sequence in Section 1.A | S2 computed a thermal map but S4 did not validate thermal gradient across matched pairs at sign-off |
