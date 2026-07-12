# S3 -- Routing Engine: Full Standalone Specification (Analog-Enriched)

---

## 1. Problem Statement

Given a placed layout M, nets N, symmetry pairs N_SP, self-symmetry nets N_SS, net types {N^T}, design rules, and routing guidance R from trained VAE models, route all nets and optimize post-layout performance.

Analog routing differs fundamentally from digital: performance is continuous and sensitive to routing parasitics, so wirelength minimization is necessary but not sufficient. Current balancing, voltage drop, signal coupling, and routing aesthetics (regular patterns encoding hard-to-formalize expertise) must all be considered.

### 1.A Analog Considerations

Analog routing is the stage where many hard-won placement gains can be destroyed. The textbook literature identifies three fundamental differences from digital routing [ALS 4.1]:

1. **Parasitic sensitivity.** Analog circuit performance is critically dependent on layout parasitics (wire resistance, coupling capacitance, substrate coupling). A few femtofarads of unintended coupling or a few hundred milliohms of unintended resistance can shift bias points, degrade bandwidth, or introduce offset. Digital circuits are largely immune to these effects.

2. **Fewer nets, more constraints.** Analog ICs typically have fewer routing paths than digital ICs, but each path carries significantly more constraints: matched lengths, symmetric routing, shielding, current density requirements, layer confinement, and parasitic budgets.

3. **Upstream dependency.** Routing quality is strongly affected by all preceding synthesis steps -- device generation, folding, merging, placement, and shaping. A sophisticated router cannot rescue a poor placement [ALS 4.1, Fig. 4.1]. Device merging alone can dramatically reduce routing overhead for even a simple differential input stage.

4. **Interconnect as performance limiter.** As feature sizes shrink, interconnection becomes the performance bottleneck rather than the transistors. The RC delay of wiring -- governed by metal resistivity, interlevel dielectric permittivity, lead geometry, and parasitic capacitance -- dominates signal propagation in advanced nodes [Hastings, Ch. 2].

5. **Node-type sensitivity hierarchy.** Not all nodes are equally sensitive [00_ANALOG_PRINCIPLES, Section 3.1]:

| Node Type | Why It Matters | Typical Sensitivity |
|---|---|---|
| High-impedance nodes (cascode drains, opamp outputs) | Parasitic C directly degrades bandwidth, phase margin | BW ~ 1/(2*pi*R_out*C_parasitic) |
| Matched nets (diff pair gates, mirror gates) | Asymmetric parasitics create offset | Must be matched to within ~1% of each other |
| Feedback paths (compensation nodes) | Extra C shifts pole/zero locations | Can destabilize feedback loop |
| Supply/ground rails | R causes IR drop; L causes ground bounce | Budget: < 5% of V_supply |
| Clock distribution | Parasitic R*C causes skew | Matched routing required |

### 1.B Analog-Algorithm Interface

The routing engine MUST accept and respect the following analog-specific inputs:

| Input | Source | Mandatory |
|---|---|---|
| Net classification (sensitive, noisy, noncritical, power, matched) | S1 constraint extraction or designer | Yes |
| Parasitic budgets per net (max R, max C, max coupling) | Sensitivity analysis or designer | Yes for critical nets |
| Matched net pairs with matching tolerance (R_match, C_match) | S1 symmetry extraction or designer | Yes |
| Shielding requirements per net | S1 or designer | Yes for sensitive nets |
| Current per net (for EM/IR sizing) | S1 or designer | Yes for power nets |
| Layer confinement constraints (e.g., high-Z nodes on upper metals only) | Designer or auto-derived | Recommended |
| Power net topology preference (star, daisy-chain, mesh) | Designer | Recommended |

---

## 2. Sub-Block 1: VAE Routing Guide Generation

**Architecture (GeniusRoute, Zhu et al. ICCAD 2019):**

| Component | Stage 1 (unsupervised) | Stage 2-3 (supervised) |
|-----------|----------------------|----------------------|
| Input | 2 x 64 x 64 image | 2 x 64 x 64 image |
| Encoder | conv 5x5x64, conv 5x5x128, FC/64 | Same (fixed from Stage 1) |
| Latent | 32 dimensions | 32 dimensions |
| Decoder | FC/16x16x64, deconv 4x4x32, deconv 4x4x1 | Same (single-channel output) |
| Output | 2 x 64 x 64 reconstruction | 1 x 64 x 64 probability map |

**Input channels:**
- Channel 1: Pin locations of the entire design (all nets)
- Channel 2: Pin locations of the nets of interest (target net class)

**Pin extraction:** M1 shapes overlapping with contact window (CO) are pins. MOM capacitors and external ports labeled manually.

**Data augmentation:** Flip horizontal, flip vertical, rotate 180 degrees -- 4x expansion.

**Training procedure:**
1. Stage 1: Unsupervised VAE on all unlabeled data (~6000+ augmented samples). Objective: maximize log P(X|z) - D_KL[Q(z|X) || P(z)].
2. Stage 2: Freeze encoder, train new decoder on labeled data per net type (~128-256 augmented samples each). Objective: minimize ||Y - Y_hat||_2.
3. Stage 3: Fine-tune entire model with reduced learning rate. Objective: maximize log P(Y|z) - D_KL[Q(z|X) || P(z)].

**Separate models for:** differential nets, clock nets, power/ground nets.

**Image preprocessing:** Gaussian blur with 17x17 kernel on routing, 5x5 on placement. This removes exact metal shapes and encourages learning routing regions, not wire geometry.

**Output:** Per-net-type probability map r in [0,1]^{64x64} indicating likelihood each region should carry the target nets.

### 2.A Analog Considerations

The VAE routing guide implicitly learns analog routing patterns from expert layouts. However, several analog concerns require explicit attention:

**What the VAE captures well:**
- Region assignment (which areas of the die carry which net types)
- Shielding corridors (expert layouts consistently place power/ground between sensitive and noisy regions)
- Symmetric routing regions (the VAE learns that matched nets occupy mirror-image areas)

**What the VAE cannot capture:**
- Exact parasitic matching (the Gaussian blur intentionally removes geometric detail)
- Current-dependent wire widths (the image representation is binary: presence/absence of routing)
- Layer-specific routing (the 2D image projection collapses the 3D metal stack)
- Via count constraints (vias are not visible in the blurred representation)

**Layout regularity.** Layout regularity and aesthetics (checklist item 5.6) are captured implicitly by the VAE model trained on expert layouts. The VAE learns the regular routing patterns that experienced layout engineers produce, and the Cost_violate term penalizes deviations from these learned patterns. No explicit regularity metric is needed beyond the VAE guidance.

**Implication for the algorithm:** The VAE guide provides region-level steering but MUST NOT be the sole driver of routing decisions for matched nets or high-impedance nodes. The A* cost function (Section 6) must override VAE guidance when parasitic constraints demand it. Specifically, Cost_violate should be attenuated (lower weight on the b/(2^(r/c)) term) for nets with explicit parasitic budgets.

### 2.B Analog-Algorithm Interface

- The VAE guide is a soft constraint: it steers routing toward learned-good regions but never overrides hard parasitic or matching constraints.
- For matched net pairs, the VAE guide for one net in the pair should be the mirror of the other. The training procedure should enforce this by augmenting matched-pair training data with explicit mirror pairs.
- For shielded nets, the VAE guide should indicate routing corridors that leave room for shield tracks on adjacent grid positions.

---

## 3. Sub-Block 2: Symmetry Constraint Allocation

**When constraints are not designer-specified**, automatically determine which nets should be routed symmetrically and what symmetry variant to use.

**Algorithm (Chen et al. ICCAD 2020, Algorithm 1):**

```
AllocateSymConstraints(Layout L, Nets N):
  Initialize undirected graph G = (V, E)
  For each net n_i: add vertices v_i and v'_i to V

  For each net pair (n_i, n_j):
    If i == j:  # self-symmetry check
      psi_i = max over all pin-pair midpoints of fraction of pins
           that are symmetric about that midpoint
      lambda_i = the axis achieving psi_i
      If psi_i > 0: add edge (v_i, v'_i, weight=psi_i)
    Else:  # inter-net symmetry check
      psi_ij = max over all cross-net pin-pair midpoints of fraction
             of pins that have symmetric counterparts
      lambda_ij = the axis achieving psi_ij
      If psi_ij > 0: add edge (v_i, v_j, weight=2*psi_ij)

  S_sym = MaxWeightMatching(G)  # Edmonds' blossom algorithm
  Return S_sym with assigned symmetry variants
```

**Symmetry variant assignment** (based on pin geometry relative to axis):
- Mirror: all pins of both nets on the same side of the axis
- Cross: pins appear on both sides (naive mirroring would self-intersect)
- Self: matching within a single net
- Partial: some pin pairs lack symmetric counterparts; bonded with another type

### 3.A Analog Considerations

Symmetry constraint allocation is the routing engine's first line of defense for matching. Several analog concerns refine the basic algorithm:

**Matched net routing is not optional.** For matched nets (differential pair gates, current mirror gates), symmetric routing is not a nice-to-have -- it is a functional requirement. Asymmetric routing parasitics create systematic offset. A clock-to-signal coupling asymmetry of even a few femtofarads can cause 5x offset worsening [00_ANALOG_PRINCIPLES, Section 3.4]. The algorithm must therefore treat matched-net symmetry constraints as hard constraints that are never relaxed (see Section 7 for relaxation policy).

**Net ordering for matched pairs.** The ILAC and MIGHTY routers established that net ordering critically affects routing quality [ALS 4.4]. For analog circuits, the ordering should be:

1. **Matched signal net pairs** (highest priority -- they are the most constrained and require the best routing resources)
2. **Sensitive signal nets** (need low-parasitic, shielded paths)
3. **Power/ground nets** (wide, can serve as shields between sensitive and noisy nets)
4. **Noncritical signal nets**
5. **Noisy/digital nets** (lowest priority -- routed last, around everything else)

This ordering differs from the PNR spec's original priority queue (Section 6.1) which uses HPWL and pin count as primary factors. The analog ordering ensures that matched nets get the best routing resources and that power/noncritical nets naturally interpose between sensitive and noisy nets, providing incidental shielding [ALS 4.4, ILAC].

**Parasitic matching is bidirectional.** The symmetry constraint allocator identifies WHICH nets should be symmetric, but not HOW tightly their parasitics must match. The algorithm should annotate each symmetry pair with a matching tier:

| Tier | R matching | C matching | Via count matching | Length matching | Source |
|---|---|---|---|---|---|
| Moderate | < 5% | < 10% | Same count | < 5% | [00_ANALOG_PRINCIPLES, Section 3.4] |
| Exceptional | < 1% | < 2% | Identical stack | < 1% | [Hastings, Ch. 8] |

### 3.B Analog-Algorithm Interface

- The symmetry allocator MUST annotate each matched pair with a matching tier (moderate or exceptional).
- Matched net pairs with exceptional tier MUST NOT have their symmetry constraints relaxed during rip-up-and-reroute (Section 7).
- The net priority queue (Section 6.1) MUST route matched pairs before all other nets, regardless of HPWL or pin count.

---

## 4. Sub-Block 3: Pin Access Assignment

For each pin, determine valid access points on the routing grid and preferred access directions.

```
For each pin p:
  access_points = grid points covered by p on the same metal layer

  For each access point acs:
    preferred_directions = {}
    For each neighboring grid point v adjacent to acs:
      If v is on a different net's pin: skip
      Create candidate pattern (wire segment or via from acs to v)
      Check candidate against:
        - Spacing rules (parallel run, EOL)
        - Min-step rules
        - Active region overlap
      If no DRC violation:
        Compute overlap_area with active region (OD)
        If overlap_area is minimal:
          Add direction(acs->v) to preferred_directions

    acs.D_pref = preferred_directions
```

**For pins without same-layer grid coverage:** find access via cross-layer via (rare in analog due to large device dimensions; common only for small digital cells in mixed-signal).

### 4.A Analog Considerations

Pin access in analog routing has several constraints not present in digital:

**No routing over matched device gates.** Metal routing over active gate regions of matched transistors causes hydrogenation blocking, which can produce up to 20% systematic Id mismatch [Hastings, Ch. 13]. The pin access algorithm must mark grid points above active gate regions of matched devices as forbidden for all nets except the device's own gate connection. This requires cross-referencing the placement data to identify matched device boundaries.

**Contact resistance awareness.** At the metal-semiconductor interface, contact resistance is non-negligible. Typical single-contact resistance values are in the single to double-digit ohm range [FOLD, 2.8.4]. For matched nets, pin access should prefer access points that result in identical contact/via stacks for both nets in a matched pair.

**Via enclosure at pin access.** The access via must satisfy via enclosure rules on both the lower and upper metal layers. For tungsten-plug and damascene via technologies [Hastings, Ch. 15]:

| Technology | Lower Metal Overlap | Upper Metal Overlap | Stackable |
|---|---|---|---|
| Aluminum vias | Mandatory (symmetric) | Mandatory (symmetric) | No |
| Tungsten plug | Mandatory (2 sides) | Optional | Yes |
| Single damascene | Optional | Optional | Yes |
| Dual damascene | Optional | Optional | Yes |

The pin access algorithm must use the correct overlap rules for the process technology and widen the metal at access points accordingly.

### 4.B Analog-Algorithm Interface

- The pin access algorithm MUST flag grid points above matched device active gate regions as routing-forbidden zones.
- Pin access for matched net pairs MUST produce access patterns with identical via stacks (same number of cuts, same enclosure geometry).
- The algorithm MUST account for via enclosure widening when computing grid point availability near pins.

---

## 5. Sub-Block 4: Pin Clustering

**Purpose:** Group pins of symmetric nets into clusters that can be routed symmetrically, even when perfect global symmetry is impossible.

```
For symmetric net pair (n_i, n_j) with axis lambda:
  Split pins into left/right subsets:
    P^l_i = {p in P_i : x_p < lambda}
    P^r_i = {p in P_i : x_p > lambda}
    (similarly for n_j)

  Find max fully-symmetric parts between (P^l_i, P^r_j) and (P^r_i, P^l_j):
    For each pin in P^l_i, check if it has a symmetric counterpart in P^r_j
    Group matched pins into symmetry clusters c_k

  Each cluster c_k has a corresponding cluster c_k' in the partner net.

For self-symmetric net n_i with axis lambda:
  Split into P^l_i, P^r_i as above.
  Pins on the axis go into both subsets.
  Find max fully-symmetric parts -> self-symmetry clusters.

For normal nets: no clustering (skip).
```

**Routing strategy per cluster type:**
- Mirror-symmetric cluster: route one side, mirror to the other -- perfectly symmetric
- Cross-symmetric cluster: route one side, mirror with axis-crossing -- requires obstacle union
- Remaining pins (not in any cluster): route freely after clusters are done

### 5.A Analog Considerations

Pin clustering for symmetric nets must account for the physical reality that perfect mirror symmetry in routing produces perfect parasitic matching ONLY if the electromagnetic environment is also symmetric. Two additional concerns:

**Coupling asymmetry.** Even if two matched nets are routed with perfect geometric symmetry, their coupling to a THIRD net may differ. The structure in a connector (the crossover bridge) provides much better matching than simple crossing of symmetric wires [ALS 4.4.2, Fig. 4.14b]. For cross-symmetric clusters, the algorithm should prefer crossover structures that equalize coupling from nearby aggressors.

**Cluster-to-cluster ordering.** Within a symmetric net pair, multiple clusters may exist. The order in which clusters are routed affects the obstacle landscape for subsequent clusters. Route the cluster with the tightest matching requirement first, giving it the most routing freedom.

### 5.B Analog-Algorithm Interface

- For cross-symmetric clusters, the routing algorithm MUST use crossover bridge structures rather than simple wire crossings to equalize coupling from nearby aggressors.
- Clusters within a symmetric pair MUST be ordered by matching tier (exceptional before moderate) for routing priority.

---

## 6. Sub-Block 5: Constraint-Aware A* Routing

### 6.1 Net Priority Queue

```
PR_i = alpha*HPWL_i + beta*|P_i| + gamma*d_i + delta*z_i

Constants: alpha=0.1, beta=2, gamma=100, delta=50

Rationale:
- gamma is highest: symmetric nets are hardest to route, must go first
- delta escalates priority for repeatedly-failing nets
- beta prioritizes multi-pin nets (more constrained)
- alpha is a tiebreaker favoring shorter nets
```

### 6.1.A Analog Considerations: Net Ordering

The priority queue formulation above captures structural routing difficulty but does not encode analog-specific net sensitivity. The literature establishes a clear net ordering for analog circuits [ALS 4.4, ILAC, MIGHTY, SLAM]:

**Analog net ordering (overrides the generic priority queue for analog mode):**

```
Class 1: Matched signal net pairs    (route first, best resources)
Class 2: Sensitive signal nets       (low-parasitic paths, shielding needed)
Class 3: Power/ground nets           (wide leads, serve as natural shields)
Class 4: Noncritical signal nets     (fill remaining routing resources)
Class 5: Noisy/digital nets          (route last, around everything else)
```

Within each class, the original priority formula (HPWL, pin count, symmetry, failure count) still applies as a secondary sort.

**Why this ordering matters:** When ILAC routes power and noncritical nets before noisy nets, the power/noncritical routing physically interposes between sensitive and noisy regions, providing incidental shielding without explicit shield track insertion [ALS 4.4]. This reduces the number of explicit shield wires needed (Section 9) and conserves routing resources.

**Star vs. daisy-chain vs. mesh for power nets.** Power net topology must be selected before routing begins [Hastings, Ch. 15]:

| Topology | When to Use | IR Drop | EM Risk | Area |
|---|---|---|---|---|
| **Star** | Matched device returns (diff pair, mirror); precision bias lines | Best (zero differential drop at star point) | Moderate (each branch carries only its own current) | Highest (dedicated branch per device) |
| **Daisy-chain** | Sequential current distribution (e.g., resistor string biasing) | Worst (drops accumulate along chain) | Worst (first segment carries total current) | Lowest |
| **Mesh/Grid** | High-current distribution (VDD, VSS power grid) | Good (redundant paths average out drops) | Best (current splits across multiple paths) | Moderate |

**The star node rule [Hastings, Ch. 15]:** For matched devices whose performance depends on identical supply/ground potentials, all sensitive leads must return to a single common point (the star point). Since both leads connect to the same node, ground current cannot generate any differential between them. Voltage drops elsewhere cause both devices to vary in unison (common mode), to which most circuits are highly immune.

**Kelvin connections [Hastings, Ch. 15]:** For precision resistor measurement or matched current sensing, separate force leads (carrying the large current) from sense leads (connecting to the low-current measurement circuit). Because sense leads carry negligible current, almost no voltage drops occur along them. For extreme accuracy, match the currents flowing through both sense leads and route them with equal resistance so that any residual voltage drops cancel.

### 6.1.B Analog-Algorithm Interface

- The net priority queue MUST support an analog-mode override that sorts by net class (matched > sensitive > power > noncritical > noisy) before applying the HPWL/pin-count/symmetry tiebreaker.
- Power nets with star-node requirements MUST be annotated before routing. The router MUST implement star topology by routing all branches from a single Steiner point (the star point), not by daisy-chaining device connections along a trunk.
- Kelvin connections MUST be implemented as separate force and sense subnets within the same logical net, routed with distinct paths that converge only at the measurement point.

### 6.2 Routing Algorithm

```
ConstraintAwareRoute(net n_i, clusters C_i):
  # Phase 1: Union obstacles on both sides of symmetry axis
  obstacles = union(obstacles_left, mirror(obstacles_left))

  R_i = {}, P_rest = P_i

  # Phase 2: Route clusters symmetrically
  For each cluster c_k in C_i:
    If c_k not yet routed:
      r_k = A_star_search(c_k, obstacles, guidance_map, design_rules)
      If r_k is legal:
        R_i = R_i + {r_k, Mirror(r_k)}
        P_rest = P_rest \ c_k \ SymCluster(c_k)

  # Phase 3: Route remaining pins (non-symmetric)
  r_rest = A_star_search(P_rest, obstacles + R_i)
  R_i = R_i + {r_rest}

  Return R_i
```

### 6.2.A Analog Considerations: Routing-by-Construction for Matching

The algorithm above routes one side and mirrors to produce symmetric routing. This is the correct approach for producing matched routes BY CONSTRUCTION rather than routing both sides independently and checking afterward [ALS 4.4.2]. However, several refinements are needed:

**Obstacle mirroring with nonsymmetric blockages.** When a blockage exists only on one side of the symmetry axis, it must be reflected to the other side before routing, so that the routed path avoids it on both sides. This is the ROAD router's approach [ALS 4.4.2] and is already implemented in Phase 1 above. The analog consideration is that this obstacle union must also include:
- Shield tracks already placed (from previously routed sensitive nets)
- Guard ring metal (from placement)
- Metal-over-gate exclusion zones (from Section 4.A)

**Cross-symmetric routing.** When pins appear on both sides of the axis, naive mirroring causes self-intersection. The crossover bridge structure (two wires cross the axis, then connect to their respective targets on the opposite side) provides better parasitic matching than simple crossing [ALS 4.4.2, Fig. 4.14b]. The A* search for cross-symmetric clusters must include a mandatory axis-crossing waypoint.

**Net splitting for current density.** Within a single net, current densities in different portions may vary by an order of magnitude [ALS 4.4.1]. The router must support net splitting: separating high-current paths from low-current paths so that voltage drops on high-current segments do not propagate to sensitive low-current nodes. This is implemented by the subnet structure in the connectivity representation [ALS 4.3.2]:

```
Net N_i = {Subnet S_i1 (high-current), Subnet S_i2 (low-current), ...}
```

Each subnet is routed with appropriate wire width and via count. The subnets share a common node but are physically separated so that IR drop in S_i1 does not affect S_i2.

### 6.2.B Analog-Algorithm Interface

- The obstacle union in Phase 1 MUST include shield tracks, guard ring metal, and metal-over-gate exclusion zones in addition to standard routing obstacles.
- The A* router MUST enforce S1's crosstalk exclusion matrix as hard minimum-spacing constraints between the specified net pairs, in addition to the soft Cost_coupling penalty. For each `(net_a, net_b, min_spacing)` tuple in S1's `crosstalk_exclusions` list, routing of net_a within `min_spacing` of net_b (or vice versa) is treated as a hard DRC-like violation, not merely a soft cost increase.
- For cross-symmetric clusters, the A* search MUST include a mandatory axis-crossing waypoint to enable crossover bridge structures.
- The connectivity representation MUST support net splitting into subnets with independent wire width and via count requirements.

### 6.3 A* Cost Function

```
Cost(node) = Cost_wire(node)
           + Cost_via(node)
           + Cost_history(node)
           + Cost_violate(node)
           + Cost_compete(node)

Cost_wire = segment_length * layer_cost[layer]
  (layer_cost reflects sheet resistance: lower layers = higher cost)

Cost_via = via_penalty * num_layer_changes
  (via_penalty ~ 2-5x a typical wire segment, since vias add R and C)

Cost_history = history_map[node]
  (incremented each time a net is ripped up at this node; drives negotiation)

Cost_violate = a + b / 2^(r^n_{i,j} / c)
  where r^n_{i,j} is the routing probability at this location for this net type
  (high probability -> low cost; low probability -> high cost)
  Constants: a ~ 1, b ~ 10, c ~ 0.3

Cost_compete = max(d * (r_bar_{i,j} - r^n_{i,j}), 0)
  where r_bar_{i,j} is the average probability across all net types
  (penalizes routing in regions "claimed" by other net types)
  Constant: d ~ 5
```

### 6.3.A Analog Considerations: Parasitic-Aware Cost Terms

The A* cost function above is primarily a congestion and guidance-driven cost. For analog routing, it must be extended with parasitic-aware terms derived from the sensitivity-based routing paradigm [ALS 4.5.3]:

**Extended analog cost function:**

```
Cost_analog(node) = Cost_wire(node)
                  + Cost_via(node)
                  + Cost_history(node)
                  + Cost_violate(node)
                  + Cost_compete(node)
                  + Cost_coupling(node)      [NEW]
                  + Cost_resistance(node)     [NEW]
                  + Cost_layer_cap(node)      [NEW]

Cost_coupling = sum over neighboring nets j:
  coupling_weight[j] * C_coupling(node, j)

  where:
    C_coupling is estimated using the 2D extraction model [ALS 4.4.3]:
      C_2D = C_1D + (delta_l * C_p) / d

    coupling_weight[j] depends on the net classification pair:
      sensitive-to-noisy: weight = 100  (strongly penalized)
      sensitive-to-sensitive: weight = 50  (moderately penalized, esp. if unmatched)
      any-to-power: weight = 1  (power is quiet, low penalty)
      noncritical-to-noncritical: weight = 0  (no penalty)

Cost_resistance = R_sensitivity[net] * delta_R(node)

  where:
    delta_R = R_sheet[layer] * segment_length / wire_width
    R_sensitivity is derived from sensitivity analysis or set by tier:
      High-impedance node: R_sensitivity = 0 (resistance is irrelevant;
        capacitance dominates -- use Cost_layer_cap instead)
      Power/ground: R_sensitivity = 100 (IR drop is critical)
      Matched pair: R_sensitivity = 50 (resistance matching)

Cost_layer_cap = C_sensitivity[net] * C_substrate(node)

  where:
    C_substrate is the parasitic capacitance to substrate, which depends
    on the metal layer:
      C_substrate = epsilon_0 * epsilon_r * (wire_width * segment_length) / d_to_substrate
    Lower metal layers have smaller d_to_substrate -> higher C_substrate
    For high-impedance nodes, use upper metal layers (larger d) to minimize
    parasitic capacitance [FOLD, 7.3.2]
```

**Layer cost refinement.** The original layer_cost reflects sheet resistance only. For analog, layer cost must be a vector encoding multiple properties:

| Layer | R_sheet (typical) | C_to_substrate (relative) | Analog Role |
|---|---|---|---|
| Metal-1 | Highest | Highest (closest to substrate) | Device-local connections only; avoid for long runs |
| Metal-2 | Moderate | Moderate | General signal routing |
| Metal-3 | Moderate | Lower | Shield planes, signal routing |
| Metal-4+ (if available) | Lowest (often thicker) | Lowest | Power distribution, high-impedance node routing (min-C), clock distribution |
| Top metal (thick Cu) | Very low | Lowest | High-current power (ampere-scale); non-standard design rules [FOLD, 2.8.4] |

**High-impedance node routing rule:** High-Z nodes (cascode drains, opamp internal nodes) should be routed on the HIGHEST available metal layer to minimize parasitic capacitance to substrate. The layer_cost for these nets should strongly penalize lower metals:

```
layer_cost_highZ[layer] = base_cost * (max_layer - layer + 1)^2
```

This produces a strong quadratic penalty for routing high-Z nets on lower layers.

**Source degeneration in routing metal.** For nodes where the routing wire is in the signal path of an amplifier (e.g., source connections of cascode devices), the wire resistance acts as source degeneration, reducing transconductance. The resistance penalty for such nets should be elevated:

```
R_sensitivity[source_degen_net] = 200  (very high -- even small R matters)
```

A 1-ohm routing resistance in a source leg with gm = 10 mS causes ~1% gm degradation. At 10 ohms, the degradation is ~9%.

**Crosstalk extraction model.** The cost function uses the 2D extraction model from [ALS 4.4.3]:

```
C_1D = A * C_area + S * C_perimeter

  where:
    A = overlap area between wires on adjacent layers
    S = perimeter of the overlap region
    C_area = capacitance per unit area (process-dependent)
    C_perimeter = fringing capacitance per unit length

C_2D = C_1D + (delta_l * C_p) / d

  where:
    delta_l = parallel run length between wires on the same layer
    C_p = coupling capacitance per unit length
    d = separation between wire edges
```

For modern damascene processes, lateral coupling C_lat is MORE significant than vertical coupling C_vert because narrow, tall wire cross-sections have greater sidewall area [FOLD, 7.3.3].

### 6.3.B Analog-Algorithm Interface

- The A* cost function MUST include Cost_coupling, Cost_resistance, and Cost_layer_cap terms for analog mode.
- The coupling weight table MUST be configurable per net-class pair (sensitive-noisy, sensitive-sensitive, etc.).
- High-impedance nets MUST have a layer cost that strongly penalizes lower metal layers.
- The crosstalk extraction model MUST use at minimum the 2D model (C_1D + lateral fringing). For RF applications, inductive coupling and 2.5D/3D models may be required [ALS 4.4.3].

### 6.4 Heuristic

For nets WITHOUT routing guidance: standard Manhattan distance heuristic.
For nets WITH routing guidance: heuristic set to ZERO (effective Dijkstra/maze routing), because the guidance cost landscape already directs the search. Setting a non-zero heuristic can fight the guidance.

### 6.5 Multi-Pin Net Decomposition

Each multi-pin net is decomposed into 2-pin subnets via Minimum Spanning Tree on pin positions. Routed sequentially, each new subnet connecting to the existing partial Steiner tree.

### 6.5.A Analog Considerations: EM-Aware Net Topology

The standard Minimum Spanning Tree decomposition minimizes total wirelength but may NOT minimize electromigration stress. For nets carrying significant current (power nets, bias nets), the net topology affects current distribution and thus EM reliability [FOLD, 7.5.4]:

| Topology | Wire Length | EM Stress | When to Use |
|---|---|---|---|
| RSMT (Steiner tree) | Lowest | Highest (current accumulates at trunk) | Low-current signal nets |
| Trunk tree | Moderate | Moderate | Moderate-current nets |
| Current-optimized (split paths) | Highest | Lowest (current distributed) | High-current power nets |
| Star (all branches from one point) | Moderate-high | Low (each branch independent) | Matched device returns |

For power nets above a current threshold (e.g., I_net > 1 mA), the MST decomposition should be replaced with a current-optimized topology that splits current across multiple paths. For matched device returns, the star topology is mandatory (Section 6.1.A).

### 6.5.B Analog-Algorithm Interface

- Multi-pin net decomposition MUST select topology based on net current:
  - I_net < 0.1 mA: RSMT (minimize wirelength)
  - 0.1 mA < I_net < 1 mA: Trunk tree
  - I_net > 1 mA: Current-optimized split paths
  - Matched returns: Star topology (mandatory)

### 6.6 Wire Width and Via Handling

```
For each net n:
  wire_width = max(PDK.min_width, S1.min_wire_width[n])
  via_cuts = max(1, S1.min_via_cuts[n])

  During A* search:
    - Grid edges are filtered by minimum spacing for the net's wire width
    - Via nodes generate multi-cut via structures
    - For power/ground: enforce NDR (non-default rules) per PDK
```

### 6.6.A Analog Considerations: Wire Sizing Beyond EM

The original wire sizing is purely EM-driven (minimum width to keep J < J_max). Analog wire sizing must also consider:

**1. Resistance-based sizing.** For nodes where wire resistance degrades circuit performance (not just reliability), wire width must be set to keep R below a functional budget:

```
W_resistance = R_sheet * L / R_budget

where:
  R_sheet = sheet resistance of the layer (Ohm/sq)
  L = estimated wire length (from HPWL or global route)
  R_budget = maximum acceptable resistance for this net
```

Example: A current mirror gate connection with R_budget = 10 ohms on Metal-2 (R_sheet = 0.05 Ohm/sq) over 100 um requires W >= 0.05 * 100 / 10 = 0.5 um. This may exceed the min-width for EM.

**2. Capacitance-based sizing.** For high-impedance nodes, wider wires INCREASE parasitic capacitance. The wire should be at MINIMUM width on the HIGHEST available metal layer:

```
For high-Z nodes:
  wire_width = PDK.min_width  (never wider)
  preferred_layer = highest available metal  (max d_to_substrate)
```

The RC product for a wire scales as [FOLD, 7.3.2]:

| Action | Effect on R | Effect on C | Effect on RC |
|---|---|---|---|
| Reduce wire length l | Proportional decrease | Proportional decrease | Drops as 1/l^2 (quadratic improvement) |
| Increase wire width w | Decreases (proportional to 1/w) | Increases (C_plate only, not C_fringe) | Net decrease (R reduction dominates) for long wires |
| Use upper metal layers | Depends on layer R_sheet | Decreases (larger d) | Generally decreases |

The quadratic benefit of reducing line length makes wire length minimization the single most effective RC optimization [FOLD, 7.3.2].

**3. IR drop sizing for power nets.** For power/ground nets, wire width must keep IR drop below the budget:

```
W_IR = R_sheet * L * I_max / V_IR_budget

where:
  V_IR_budget = 0.05 * V_supply  (analog: 5% budget)
                0.10 * V_supply  (digital: 10% budget)
```

Example: For 1.0V analog supply, V_IR_budget = 50 mV. A power line carrying 10 mA over 500 um on Metal-2 (R_sheet = 0.05 Ohm/sq) requires W >= 0.05 * 500 * 0.01 / 0.05 = 5.0 um.

**4. Via count for current and matching.**

Via resistance is significant and variable. For matched nets, via count matching is essential [00_ANALOG_PRINCIPLES, Section 3.3]:

```
For each via transition in a matched net pair:
  via_cuts_A must equal via_cuts_B
  via_stack_A must use identical layers as via_stack_B
```

For current-carrying nets, the minimum number of via cuts is:

```
num_cuts = ceil(I_max / I_max_per_cut)

where I_max_per_cut is process-dependent (typically 0.1-0.5 mA/cut)
```

**5. ESD bus metal corner chamfering [FOLD 7.4].** ESD bus metal corners must be chamfered (45-degree bevel) to prevent current crowding during ESD events. Outer corners of Metal1 interconnects near ESD devices are particularly susceptible to local field peaks that cause premature metal failure under high transient currents. The router must automatically apply 45-degree chamfers to all corners of nets classified as `ESD_bus`.

**7. Metal width tapering near vias.** At via transitions, the metal must widen to accommodate the via enclosure rules [Hastings, Ch. 15]:

```
W_at_via = V + 2 * OL_min

where:
  V = drawn via width
  OL_min = minimum metal overlap of via
```

If W_at_via > wire_width, the router must insert a local width taper (widening) at the via location. This is standard in digital routers but particularly important in analog because:
- The taper metal adds parasitic capacitance (counted in C matching)
- Asymmetric tapers between matched nets create parasitic mismatch
- For matched nets, tapers must be geometrically identical

**8. Via arrays at wide-lead transitions.** When wide leads change layers, extend both metal leads fully across each other and fill the overlap with as many vias as possible [Hastings, Ch. 15]. Place via arrays in straight lead segments, not at corners, to avoid current crowding [Hastings, Ch. 15].

**9. Corner bend guidelines.** 90-degree corner bends create current crowding that increases EM stress [FOLD, 7.5.4], [Hastings, Ch. 15]. Replace 90-degree bends with pairs of 45-degree bends (135-degree turns) for high-current leads. The current-density distribution at 135-degree bends is much more uniform than at 90-degree bends [FOLD, 7.5.4, Fig. 7.35].

### 6.6.B Analog-Algorithm Interface

- Wire width MUST be computed as: max(W_EM, W_resistance, W_IR, PDK.min_width), where each component is net-type-dependent.
- For high-impedance nodes, wire width MUST be set to PDK.min_width and routed on the highest available metal layer.
- Via count for matched net pairs MUST be identical. The router MUST track via count per matched net and insert dummy vias if needed to equalize.
- Metal tapers at via locations MUST be geometrically identical for matched net pairs.
- High-current leads (I > 1 mA) MUST use 135-degree bends (two 45-degree segments) instead of 90-degree bends.
- Via arrays MUST be placed in straight lead sections, never at corners.

---

## 7. Sub-Block 6: Negotiation-Based Rip-Up and Reroute

```
For iteration = 1 to max_reroute_iterations:
  For each net in priority order:
    Route net using A* with current obstacle map
    If routing fails or causes DRC violation:
      Record failure in history_map (increment cost)
      Rip up the net (remove its routing)
      Re-add to priority queue with z_i incremented

  If all nets routed with 0 DRC violations: break

  If a net fails > constraint_relaxation_threshold times:
    Relax its symmetry constraint (waive to normal routing)
    Log the relaxation in the constraint report
```

**Typical parameters:** max_reroute_iterations = 50, constraint_relaxation_threshold = 5.

### 7.A Analog Considerations: Constraint Relaxation Policy

The constraint relaxation mechanism (waiving symmetry after repeated failures) is dangerous for analog circuits. Relaxing matching constraints converts a routable-but-hard problem into an easy-but-broken circuit.

**Tiered relaxation policy:**

```
For a failing net with matching tier T:
  If T == exceptional:
    NEVER relax matching constraint.
    Instead: increase max_reroute_iterations, rip up lower-priority nets
    to free routing resources, or flag for placement adjustment.

  If T == moderate:
    Relax only after exhausting all alternatives:
    1. First: try rerouting on different layers (layer reassignment)
    2. Second: try rerouting with relaxed length matching (5% -> 10%)
    3. Third: try rerouting with relaxed C matching (10% -> 20%)
    4. Only after all above fail: waive to normal routing with warning

  If T == none (normal net):
    Standard relaxation after constraint_relaxation_threshold failures.
```

**Rip-up priority for analog.** When freeing routing resources for a failing matched net, the rip-up priority should be:
1. Noisy/digital nets (lowest analog value, easiest to reroute elsewhere)
2. Noncritical signal nets
3. Power nets (only if alternative power paths exist)
4. Never rip up other matched or sensitive nets

### 7.B Analog-Algorithm Interface

- Exceptional-tier matched nets MUST NOT have their symmetry constraints relaxed. The router MUST escalate to placement adjustment if routing is infeasible.
- Moderate-tier matched nets MUST exhaust layer reassignment and tolerance relaxation before waiving symmetry.
- Rip-up priority MUST follow the inverse of the analog net ordering (noisy first, matched last).

---

## 8. Sub-Block 7: Parasitic Matching Verification and Correction

After routing all symmetric pairs:

```
For each matched net pair (n_a, n_b):
  R_a = sum over segments in n_a: length * R_sheet[layer] / width
  R_b = sum over segments in n_b: length * R_sheet[layer] / width

  C_a = sum over segments in n_a: length * width * C_unit[layer]
  C_b = sum over segments in n_b: length * width * C_unit[layer]

  If |R_a - R_b| > R_match_threshold:
    Add length-equalization constraint to the shorter net
    Reroute with additional detour to equalize

  If |C_a - C_b| > C_match_threshold:
    Flag for review (harder to fix -- may need placement adjustment)
```

**Thresholds:** R_match < 5% of total, C_match < 10% of total (application-dependent).

### 8.A Analog Considerations: Producing Matched Routes by Construction

The verification-then-correction approach above is a safety net, but the router should PRODUCE matched routes by construction rather than fixing them afterward [ALS 4.4.2]. The mirror-and-route approach in Section 6.2 achieves geometric symmetry, but parasitic matching requires additional measures:

**1. Length matching by construction.** When the mirror-route approach produces a geometrically symmetric route, the lengths are automatically matched (to the extent that the obstacle landscape is symmetric). However, when obstacles force asymmetric detours, explicit length equalization is needed. The technique: add serpentine jogs or dead-end stubs to the shorter net [Hastings, Ch. 8, Rule 10]. These jogs equalize both R and C simultaneously (unlike simple wire lengthening, which would change R/C ratio if done on a different layer).

```
Length equalization algorithm:
  delta_L = |L_a - L_b|
  shorter_net = argmin(L_a, L_b)

  Insert serpentine jog on shorter_net:
    jog_length = delta_L / 2  (each U-turn adds 2 * jog_length)
    jog_width = same as wire_width (maintains R/C ratio)
    jog_layer = same layer as adjacent segments (maintains C matching)

  Verify: |L_a_new - L_b_new| < L_match_tolerance
```

**2. Via count matching.** Matched nets must have identical via stacks. If the mirror route requires an extra via on one side (due to asymmetric obstacles), add a compensating dummy via on the other side. The dummy via connects to the same net (creating a zero-length via jog) but equalizes the via resistance contribution:

```
Via count equalization:
  V_a = count(vias in n_a, grouped by via type)
  V_b = count(vias in n_b, grouped by via type)

  For each via type t:
    delta_V = |V_a[t] - V_b[t]|
    If delta_V > 0:
      net_with_fewer = argmin(V_a[t], V_b[t])
      Insert delta_V compensating via jogs on net_with_fewer
      (up to layer above, immediately back down)
```

**3. Layer assignment matching.** Both nets in a matched pair must use the same metal layers for corresponding segments. If net A uses Metal-2 for a segment while net B uses Metal-3 for the corresponding (mirrored) segment, the different C_unit values create a parasitic mismatch even if lengths are identical.

```
Layer assignment verification:
  For each segment pair (s_a, s_b) where s_b = mirror(s_a):
    If layer(s_a) != layer(s_b):
      FLAG: layer mismatch in matched pair
      CORRECTIVE ACTION: reassign both segments to the same layer
```

**4. Coupling environment matching.** Even with perfect R, C, and length matching, asymmetric coupling to third-party nets creates mismatch. The ANAGRAM router identifies this as a limitation of pure geometric matching [ALS 4.4.2]. The corrective measure: after routing all nets, compute coupling capacitance from each matched net to all neighboring nets. If coupling is asymmetric, insert shield tracks on the side with higher coupling (see Section 9).

**5. Parasitic capacitance equalization using jogs/stubs.** Hastings Ch. 8, Rule 10 prescribes using jogs or dead-end branches to equalize parasitic capacitance between matched nets. The algorithm:

```
C equalization:
  delta_C = |C_a - C_b|
  If delta_C > C_match_threshold:
    net_with_less_C = argmin(C_a, C_b)
    stub_length = delta_C / (C_unit[layer] * wire_width)
    Insert dead-end stub of stub_length on net_with_less_C
    (stub connects to the net but carries no signal current)
```

### 8.B Analog-Algorithm Interface

- The router MUST produce matched routes by construction (mirror-route, same-layer, same-via-count) as the primary mechanism. Post-route verification is a safety net, not the primary matching strategy.
- Length equalization jogs MUST be on the same metal layer as adjacent segments to maintain C matching.
- Via count equalization MUST insert compensating via jogs (up-and-back) to equalize via resistance.
- Layer assignment for matched segments MUST be identical. The router MUST NOT assign different layers to corresponding segments of a matched pair.
- After all routing is complete, coupling environment verification MUST check that each matched net has symmetric coupling to neighboring nets. Asymmetric coupling is corrected by shield insertion (Section 9).

---

## 9. Sub-Block 8: Shielding Insertion

```
For each net classified as "sensitive" (from S1):
  Identify the routing layers used by the net
  On each layer:
    Reserve adjacent tracks (one track on each side)
    Route grounded shield wires on the reserved tracks
    Connect shield wires to VSS at regular intervals

  Shield insertion happens AFTER main signal routing,
  BEFORE power/ground routing (so PG routing can connect to shields).
```

### 9.A Analog Considerations: Shielding Methods and Rules

The basic shielding insertion above addresses only same-layer lateral shielding. The literature describes three distinct shielding methods [ALS 4.4.3], [FOLD, 7.3.3]:

**1. Same-layer (lateral) shielding.** A grounded shield wire on the same metal layer between the aggressor and victim. This absorbs lateral coupling C_lat. In modern damascene processes, C_lat dominates C_vert for narrow, tall wires [FOLD, 7.3.3], making lateral shielding the most important method.

**2. Different-layer (vertical) shielding.** A grounded metal plane on the layer above and/or below the sensitive wire. This absorbs vertical coupling C_vert. Vertical shielding is particularly important when sensitive signal wires must cross over or under noisy wires on adjacent layers.

**3. All-around shielding.** Combination of lateral and vertical shielding for maximum protection. Used for the most critical analog signals (precision reference voltages, high-gain amplifier inputs).

**Shield insertion rules:**

```
Rule 1: Shield wires MUST be connected to a DC potential (ground or
  the sensitive net's reference node). A floating shield WORSENS
  coupling because it acts as a capacitive voltage divider
  [ALS 4.4.3], [Hastings, Ch. 15].

Rule 2: Shield wires MUST extend beyond the sensitive net by at least
  5 um on each end to capture fringing fields [Hastings, Ch. 15].

Rule 3: Shield connections to ground must occur at regular intervals
  (every 50-100 um) to keep the shield at a low impedance at
  frequencies of interest.

Rule 4: For matched net pairs, shielding must be symmetric. If one
  net in a matched pair has a shield track on its left side, the
  corresponding net must have a shield track on its right side
  (maintaining mirror symmetry).

Rule 5: When sensitive and noisy signals must cross, minimize
  intersection area by crossing at RIGHT ANGLES. Insert a
  grounded metal plane (electrostatic shield) between them on an
  intermediate metal layer, extending ~5 um beyond the
  intersection area [Hastings, Ch. 15].
```

**Shield insertion ordering relative to other routing:**

```
1. Route matched signal nets (highest priority)
2. Route sensitive signal nets (with shield track reservation)
3. Insert shield tracks for sensitive nets
4. Route power/ground nets (can connect to shield tracks as needed)
5. Route noncritical signal nets
6. Route noisy/digital nets (last -- routed around shields)
```

This ordering ensures that power/ground nets can be routed through shield tracks (connecting to them), providing additional shielding benefit without extra area [ALS 4.4, ILAC].

**Coupling voltage estimation.** The voltage disturbance induced by capacitive coupling is [Hastings, Ch. 15]:

```
delta_V = C_coupling * (dV/dt) * R_node

Example: A digital signal switching in < 0.1 ns produces slew rates
  exceeding 10 V/ns. Such a signal coupling across 10 fF into a
  100 kOhm node generates a 1 V disturbance.
```

This quantifies why shielding is non-optional for high-impedance analog nodes near digital signals.

### 9.B Analog-Algorithm Interface

- The shielding insertion algorithm MUST support three modes: lateral only, vertical only, and all-around.
- Shield wires MUST be connected to a DC potential at intervals not exceeding 100 um. Floating shields are forbidden.
- Shields MUST extend 5 um beyond the sensitive net endpoints.
- For matched net pairs, shield geometry MUST be mirror-symmetric about the symmetry axis.
- Shield insertion MUST occur after matched net routing but before power/ground and noisy net routing.
- The router MUST never route a noisy signal adjacent to, above, or below a sensitive signal without an intervening shield [Hastings, Ch. 15].

---

## 10. Sub-Block 9: EM/IR Verification and Correction

### EM Check

```
For each wire segment s:
  J_s = I_max[net] / (width_s * thickness[layer])

  If J_s > J_max[layer]:
    If length_s < Blech_length[layer]:
      Grant EM waiver (short segment, immortal)
    Else:
      Widen segment: new_width = I_max[net] / (J_max[layer] * thickness)
      If widening violates spacing to neighbors:
        Add parallel path on adjacent layer
      Re-check routing DRC after widening

For each via v:
  I_via = I_max[net]
  If I_via > I_max_per_cut * num_cuts:
    Increase num_cuts or add parallel vias
```

### IR Drop Analysis

```
For power and ground nets:
  Build resistive network:
    Each wire segment -> resistor: R = R_sheet * length / width
    Each via -> resistor: R = R_via / num_cuts
    Each device pin -> current source: I = DC_bias_current[device]

  Solve Kirchhoff's equations (sparse linear system)

  For each device pin:
    IR_drop = |V_supply - V_pin|
    If IR_drop > IR_budget (typically 5-10% of V_supply):
      Add metal reinforcement (wider wire or additional via stack)
      Re-solve and verify
```

### 10.A Analog Considerations: EM/IR for Analog Circuits

The EM/IR verification above handles the basic reliability and voltage-drop concerns. Several analog-specific refinements are needed:

**1. Unidirectional vs. bidirectional current.** Power and analog bias lines carry UNIDIRECTIONAL (DC) current and are the MOST susceptible to electromigration. Digital signal and clock lines carry bidirectional current and have lower EM risk due to reversed, compensatory material migration [FOLD, 7.5.1]. The EM check must use different J_max limits:

| Wire Type | Current Direction | J_max (Al) | J_max (Cu) |
|---|---|---|---|
| Power/analog bias | Unidirectional (DC) | ~1 mA/um | ~5 mA/um |
| Digital signal/clock | Bidirectional (AC) | ~5 mA/um (RMS) | ~25 mA/um (RMS) |

**2. Via configuration awareness.** In copper dual-damascene interconnects, via-below configurations are preferred for EM robustness. Voids nucleate on the top surface of copper, and via-below structures allow higher permissible void volumes before failure. Via-above configurations are more susceptible because the void forms directly at the via connection point [FOLD, 7.5.4].

The router should prefer via-below configurations for high-current nets:

```
For high-current net transitions between layers:
  If current flows upward (from lower to higher metal):
    Prefer via-below configuration (current enters via from below)
  If current flows downward:
    Prefer via-above configuration (less ideal, but unavoidable)
    Compensate with additional via cuts (redundancy)
```

**3. Blech length exploitation.** Any interconnect shorter than the Blech length is "EM immortal" because stress migration compensates the electromigration flow [FOLD, 7.5.4]:

```
(J * L)_Blech ~ 2000-5000 A/cm for Cu (process-dependent)

For a segment with J = 5 mA/um = 5 * 10^4 A/cm:
  L_Blech = 5000 / (5 * 10^4) = 0.1 cm = 1000 um = 1 mm
```

Short intra-cell straps below the Blech length can be granted EM waivers without risk. The router should compute J*L for each segment and waive EM checks for segments below the Blech threshold.

**Segment subdivision for EM immortality.** Long interconnects can be made EM-immune by subdividing into segments below the Blech length via additional vias [FOLD, 7.5.4]. The tradeoff: more vias consume routing resources and add resistance. Segment length balancing (shifting existing vias without adding new ones) is a lower-cost alternative that reduces hydrostatic stress in the highest-risk segment.

**4. Reservoir effect.** Enlarged via overlaps provide extra metal material for migration, preventing void growth [FOLD, 7.5.4]. For analog power nets, specify minimum via overlap that exceeds the design-rule minimum:

```
For power nets:
  via_overlap = max(OL_min * 1.5, OL_min + 0.1 um)
```

Caveat: Reservoirs can have an adverse effect in nets with current-flow reversals, as they reduce beneficial stress migration in those cases [FOLD, 7.5.4].

**5. IR drop effects on matching.** For matched devices, IR drop in supply/ground routing creates systematic errors in matched ratios [Hastings, Ch. 8, Rule 23]. The metal and via resistance contributions should scale proportionally to desired device ratios. For a 1:1 current mirror:

```
R_supply_to_device_A MUST equal R_supply_to_device_B

If |R_supply_A - R_supply_B| > 1% of total R_path:
  FLAG: supply resistance mismatch in matched pair
  CORRECTIVE ACTION: equalize by widening the narrower path or
    adding parallel supply connections
```

Use Kelvin connections (separate sense and force leads) for exceptional matching [Hastings, Ch. 8].

**6. Temperature derating.** EM limits must be derated for temperature using Black's law [Hastings, Ch. 15]:

```
Derating factor D = exp[(E_a / k_B) * (1/T_ref - 1/T_op) * (1/n)]

where:
  E_a = 0.5 eV (pure Al), 0.7 eV (Cu-doped Al), 0.9 eV (Cu)
  k_B = 8.62 * 10^-5 eV/K
  n = 2 (Al), 1 (Cu)
  T_ref = reference temperature (typically 298K = 25C)
  T_op = maximum operating temperature

Example: Lead rated for 25 mA at 25C, operating at 125C:
  D = exp[(0.7 / 8.62e-5) * (1/298 - 1/398) * (1/2)]
  D ~ 0.58
  Derated capacity: 25 * 0.58 = 14.5 mA
```

A lead must meet EM rules at its worst-case operating temperature, not room temperature.

### 10.B Analog-Algorithm Interface

- EM checks MUST use different J_max limits for unidirectional (DC) and bidirectional (AC) current.
- The router MUST prefer via-below configurations for high-current nets in copper processes.
- Blech length waivers MUST be computed using J*L product, not just J alone.
- Via overlaps for power nets SHOULD exceed the design-rule minimum to provide reservoir margins.
- For matched devices, supply/ground IR drop MUST be equalized between matched branches. The IR drop analysis MUST flag asymmetric supply resistance in matched pairs.
- EM limits MUST be derated for the maximum operating temperature using Black's law.

---

## 11. Post-Processing: Min-Step Fix

```
For each routed net:
  Merge all connected metal shapes into a polygon per layer
  Traverse polygon edges clockwise

  For each pair of adjacent edges with length < minStep:
    Determine if concave jog or convex jog:
      Clockwise orientation of 3 ordered points -> convex
      Counter-clockwise -> concave

    Add patch metal to eliminate the short edges:
      Concave: fill the concavity
      Convex: extend the protrusion

    Verify patch doesn't create new DRC violations
```

### 11.A Analog Considerations: OPC-Friendly Routing and Manufacturability

**Multi-patterning awareness (advanced nodes).** For advanced nodes requiring double or multi-patterning (typically <= 20 nm), the router must assign coloring to metal shapes and avoid coloring conflicts within the same net and between adjacent nets. Coloring-aware routing ensures that matched net pairs receive compatible color assignments so that their parasitic properties remain symmetric after decomposition. This capability is deferred to Phase 4 (advanced node).

The min-step fix addresses one DRC concern, but analog routing must also consider broader manufacturability issues [ALS 4.7]:

**1. OPC-friendly routing.** Optical proximity correction (OPC) is used in advanced lithography to compensate for pattern distortion. Routing patterns that are "OPC-friendly" print more reliably:

- Avoid jogs shorter than 2x the minimum metal width (these create hard-to-correct lithographic features)
- Prefer straight runs over serpentines when possible
- Maintain consistent wire widths along a route (width changes create OPC hot spots)
- When width changes are necessary (e.g., at via tapers), use gradual tapers rather than abrupt steps

**2. Metal density uniformity.** CMP planarization quality depends on local metal density [FOLD, 2.8.4], [Hastings, Ch. 2]. Routing should aim for uniform metal density across the die. The consequences of non-uniform density:

- **Too little metal:** Dummy fill is inserted, which can interfere with matched device matching (hydrogenation blocking) [Hastings, Ch. 13]
- **Too much metal:** Slots must be cut in wide interconnects, and spacings may need to increase

For analog circuits, the critical constraint is that dummy metal fill must be BLOCKED over matched device active gate regions. The router should track metal density during routing and flag regions that will require aggressive dummy fill near matched devices.

**3. Metal slotting.** Very wide power interconnects (W > ~10 um in advanced nodes) require slots to satisfy density rules and prevent dishing [Hastings, Ch. 15]. The slotting algorithm:

```
For wide metal segments with W > W_slot_threshold:
  Insert rectangular slots at regular intervals:
    slot_width = PDK.slot_width (typically 1-2 um)
    slot_spacing = PDK.slot_spacing (typically 5-10 um)
    slot_offset_from_edge >= PDK.slot_edge_clearance

  Verify: remaining metal width >= W_min at all cross-sections
  Verify: current density in remaining metal < J_max
```

**4. Via enclosure and redundancy.** Via reliability is probabilistic -- each via has a small but nonzero chance of failing due to thermal cycling-induced voiding [Hastings, Ch. 15]. Many fabs mandate pairs of vias rather than single vias. Multiple vias in close proximity have a protective effect beyond mere redundancy -- void formation in one via may relieve stress on adjacent vias.

```
Via redundancy rules:
  - Use at least 2 vias for every inter-layer connection (even if
    one via meets electrical requirements)
  - Place redundant vias in-line with the current direction so all
    current paths have equal length [FOLD, 7.5.4]
  - Never place via arrays at corners -- current crowding at inside-
    corner vias creates localized EM stress [Hastings, Ch. 15]
```

**5. Photolithographic defect avoidance.** Photolithographic defects can cause shorts between wires on the same routing level. The expected number of faults for two parallel conductors depends on their separation [ALS 4.7]. For critical analog nets:

```
Minimum spacing for sensitive analog nets:
  S_analog = max(S_min, 2 * S_min)  (use 2x minimum spacing when area allows)
```

After routing all nets, use available whitespace to extend wire spacing for improved isolation [FOLD, 7.3.3].

### 11.B Analog-Algorithm Interface

- The min-step fix MUST avoid creating jogs shorter than 2x minimum metal width.
- The router MUST track metal density and flag regions near matched devices that will require dummy fill. Dummy metal blocking pseudolayers MUST be generated over matched device active gates (extending >= 5 um in all directions) [Hastings, Ch. 13].
- Wide power metal (W > W_slot_threshold) MUST be slotted per PDK rules, with verification that slotted metal still meets EM requirements.
- Every inter-layer connection MUST use at least 2 via cuts (redundant vias).
- Via arrays MUST be placed in-line with the current direction, never at corners.
- Sensitive analog nets SHOULD use >= 2x minimum spacing when routing resources allow.

---

## 12. Sub-Block 10: Practical Metal Stack Knowledge

This section encodes practical interconnect knowledge from [Hastings, Ch. 2, Ch. 15], [FOLD, 2.8, 7.3, 7.5], and [AOAL, Ch. 15] that the routing algorithm must reference when making layer assignment and wire sizing decisions.

### 12.A Metal Stack Properties and Layer Assignment

**Typical analog metal stack (3-5 layers):**

| Layer | Typical R_sheet | Typical Thickness | C_to_substrate (relative) | Primary Analog Use |
|---|---|---|---|---|
| Metal-1 | 0.07-0.10 Ohm/sq | 0.3-0.5 um | 1.0 (baseline, closest to substrate) | Device-local connections; short intra-cell wiring |
| Metal-2 | 0.05-0.07 Ohm/sq | 0.3-0.6 um | 0.5-0.7 | General signal routing (horizontal) |
| Metal-3 | 0.03-0.05 Ohm/sq | 0.5-0.8 um | 0.3-0.5 | General signal routing (vertical); shield planes |
| Metal-4 | 0.02-0.04 Ohm/sq | 0.5-1.0 um | 0.2-0.3 | Power distribution; high-Z node routing (min-C) |
| Top thick metal | 0.005-0.01 Ohm/sq | 2.0-4.0 um | 0.1-0.2 | High-current power (ampere-scale); bondpad connections |

**Key physical relationships [FOLD, 7.3]:**
- Lower metals have HIGHER C_to_substrate (smaller d) and generally HIGHER R_sheet
- Upper metals are typically THICKER (lower R_sheet) and FARTHER from substrate (lower C)
- Fringe-field capacitance per unit length C'_fringe often EXCEEDS plate capacitance C'_plate for narrow wires (W < 2 um for M1, W < 4 um for M2) [FOLD, 7.3.2]
- In modern damascene processes, lateral coupling C_lat is MORE significant than vertical coupling C_vert due to narrow, tall wire cross-sections [FOLD, 7.3.3]

**Layer assignment rules for the router:**

```
Rule 1: HIGH-IMPEDANCE NODES -> highest available metal layer
  (minimize parasitic C to substrate; BW ~ 1/(2*pi*R_out*C_parasitic))
  Use minimum wire width on the highest layer.

Rule 2: POWER/GROUND -> highest and thickest metal layers
  (minimize R for IR drop; thick metals handle high current)
  Use wide wires, multi-cut vias.

Rule 3: MATCHED NET PAIRS -> same layer for corresponding segments
  (equalize C_unit and R_sheet between matched segments)
  Must use identical via stacks.

Rule 3a (precedence): When Rule 1 and Rule 3 conflict (one net of a
  matched pair is high-Z but the other is not), Rule 3 prevails: both
  nets are routed on the same layer, chosen as the highest layer that
  satisfies both nets' routing constraints. Matching is Priority 1;
  parasitic minimization for a single net is Priority 3.

Rule 4: SENSITIVE SIGNALS -> middle metal layers with shield planes
  above and below on adjacent layers
  (vertical shielding from grounded planes; lateral shielding from
  grounded wires on same layer)

Rule 5: NOISY/DIGITAL SIGNALS -> any available layer, MAXIMALLY
  separated from sensitive signals (different layer preferred;
  if same layer, maximum lateral spacing)

Rule 6: Metal-1 RESERVED for device-local connections
  (highest C_to_substrate; highest R_sheet; used for poly-to-M1
  contacts and short device-to-device straps within a cell)

Rule 7 (RF applications): INTEGRATED INDUCTOR KEEP-OUT ZONE
  For circuits containing integrated planar spiral inductors, the
  router must enforce an inductor keep-out zone of at least half the
  inductor outer diameter around each inductor. No routing metal,
  dummy fill, or active circuitry may be placed within this zone.
  Dummy metal and poly fill must be blocked within this zone. Do not
  place junctions beneath inductors (eddy currents in the junction
  depletion region degrade inductor Q). Do not place circuitry in
  the empty center of spiral inductors. Keep unconnected metal at
  least half the inductor width away from the spiral turns
  [Hastings, Ch. 7.2]. This capability is deferred to Phase 4
  (advanced node / RF).
```

### 12.B Contact and Via Resistance Rules of Thumb

Via and contact resistance contributes significantly to total path resistance, especially in multi-layer via stacks [Hastings, Ch. 15]:

| Connection Type | Typical Resistance (per cut) | Notes |
|---|---|---|
| Contact (M1 to silicon) | 5-50 Ohm/contact | Highly process-dependent; silicided contacts much lower |
| Via-1 (M1 to M2) | 1-10 Ohm/via | Tungsten-plug or copper, depending on process |
| Via-2+ (upper vias) | 0.5-5 Ohm/via | Generally lower R for upper vias (larger geometry) |
| Stacked via (M1 to M4) | Sum of all via R's | 10-50 Ohm total for a 4-layer stack with single cuts |

**Implications for matched nets:**
- A single via cut difference between matched nets contributes 1-10 ohms of resistance mismatch
- For a matched pair with total path R of 100 ohms, one extra via creates 1-10% resistance mismatch
- Via count matching is therefore essential for moderate and exceptional matching tiers

**Implications for power nets:**
- Multi-cut vias reduce per-transition resistance: R_via = R_per_cut / num_cuts
- For a 10 mA current through a single 5-ohm via: P = I^2 * R = 0.5 uW, V_drop = 50 mV
- Multiple cuts are needed for both EM and IR drop reasons

### 12.C Interconnect Material Properties

**Aluminum vs. Copper [Hastings, Ch. 2], [FOLD, 2.8]:**

| Property | Aluminum (Al/AlCu) | Copper (Cu) |
|---|---|---|
| Bulk resistivity | 2.65 uOhm*cm | 1.68 uOhm*cm |
| Relative resistivity | 1.0x (baseline) | ~0.63x (35% lower) |
| EM activation energy | 0.5-0.7 eV | 0.9 eV |
| J_max (DC) | ~1 mA/um | ~5 mA/um |
| Current exponent n | ~2 | ~1 |
| Patterning | Subtractive (dry etch) | Damascene (trench fill + CMP) |
| Diffusion barrier needed | No (self-passivating oxide) | Yes (Ta/TaN on all sides) |
| Via material | Tungsten plug or Al | Tungsten (to silicon) or Cu |

**Copper process implications for routing:**
- Cu requires diffusion barriers (TaN) on all sides, including a nitride cap on top. This consumes effective wire cross-section, so actual R_sheet is slightly higher than bulk resistivity would predict.
- Cu CMP creates dishing in large metal areas -> density rules and dummy fill required [Hastings, Ch. 2]
- Cu dual-damascene allows stacked vias (all Cu vias), enabling more vertical routing flexibility
- Cu via-below configurations are preferred for EM (voids nucleate on top Cu surface) [FOLD, 7.5.4]

**Polysilicon as emergency routing [Hastings, Ch. 2]:**
- Silicided poly: < 10 Ohm/sq -- usable for short connections (< 50 um) carrying < 0.1 mA
- Unsilicided poly: 20-40 Ohm/sq -- usable only for gate connections and non-current-carrying signals
- NEVER route current-carrying signals through significant lengths of poly
- Poly routing is invisible to most automatic routers and should be used only as a manual congestion-relief measure

### 12.D Thermal and Migration Considerations for Routing

**Thermal migration.** Temperature gradients cause net diffusion from hot to cold regions [FOLD, 7.5.2]. Layout mitigation:
- Distribute high-power transistors over a large die area to avoid hot spots
- Route along isothermal lines where feasible -- stay along regions of equal temperature until leaving the high-gradient zone
- Add thermal wires and thermal vias (extra metal purely as heat conductors) to reduce thermal gradients

**Stress migration.** Mechanical stress gradients (from CTE mismatch between metal, dielectric, and substrate) drive atomic diffusion [FOLD, 7.5.3]. Layout mitigation:
- Normalize material distribution (uniform metal density)
- Use keep-out zones around TSVs (for 3D-ICs)
- Wider wires experience less stress migration than narrow wires

**Coupled migration.** EM, TM, and SM are closely coupled processes. Reducing EM (via wider wires, shorter segments, lower current density) also reduces SM (which counteracts EM) and TM (via reduced Joule heating) [FOLD, 7.5.5]. The most effective single mitigation is reducing current density.

---

## 13. Expected Results

From the ICCAD 2020 silicon-proven router (TSMC 40nm):

| Benchmark | #Devices | #Nets | WL (nm) | Vias | d_SYM | DRV | Runtime (s) |
|-----------|----------|-------|---------|------|-------|-----|-------------|
| COMP | 16 | 12 | 138,400 | 19 | 0.95 | 0 | 0.11 |
| OTA1 | 25 | 18 | 386,800 | 38 | 0.88 | 0 | 1.71 |
| OTA2 | 34 | 26 | 523,400 | 79 | 0.70 | 0 | 0.30 |
| ADC1 | 206 | 127 | 2,686,600 | 175 | 0.62 | 0 | 2.70 |
| ADC2 | 153 | 109 | 3,327,600 | 184 | 0.69 | 0 | 18.82 |

Compared to prior art: 2.5x higher symmetry degree, 3.6x fewer vias, 0 DRC violations (vs. 83-550), 24x faster runtime. ADC2 was taped out and silicon-proven.

### 13.A Analog Considerations: Additional Metrics for Analog Quality

The benchmark results above evaluate wirelength, via count, symmetry degree, and DRC violations. For analog routing quality, additional metrics are needed:

| Metric | Definition | Target |
|---|---|---|
| R_match | max |R_a - R_b| / avg(R_a, R_b) over all matched pairs | < 5% (moderate), < 1% (exceptional) |
| C_match | max |C_a - C_b| / avg(C_a, C_b) over all matched pairs | < 10% (moderate), < 2% (exceptional) |
| Via_count_match | max |V_a - V_b| over all matched pairs | 0 (exceptional), <= 1 (moderate) |
| IR_drop_max | max IR_drop at any device pin | < 5% of V_supply (analog) |
| EM_violations | count of segments with J > J_max and L > L_Blech | 0 |
| Shield_coverage | fraction of sensitive net length with adjacent shields | > 90% |
| Coupling_max | max C_coupling between any sensitive-noisy pair | < C_budget from sensitivity analysis |
| Layer_mismatch | count of matched-pair segment pairs on different layers | 0 |

These metrics should be reported alongside the existing benchmarks to evaluate analog routing quality.

---

## Appendix: Source Cross-Reference

| Topic | Primary Sources |
|---|---|
| Net ordering (matched first, noisy last) | [ALS 4.4, ILAC/MIGHTY]; [00_ANALOG_PRINCIPLES, Section 5] |
| Symmetric routing (mirror-route, obstacle union) | [ALS 4.4.2, ROAD/ANAGRAM II]; [00_ANALOG_PRINCIPLES, Section 3.4] |
| Shielding methods (lateral, vertical, all-around) | [ALS 4.4.3]; [FOLD, 7.3.3]; [Hastings, Ch. 15] |
| Crosstalk extraction (1D, 2D, 2.5D models) | [ALS 4.4.3]; [FOLD, 7.3.3] |
| Sensitivity-based parasitic bounds | [ALS 4.5.3, ROAD/ANAGRAM III] |
| Parasitic matching (length, via count, layer) | [00_ANALOG_PRINCIPLES, Section 3.4]; [Hastings, Ch. 8] |
| Star nodes and Kelvin connections | [Hastings, Ch. 15]; [00_ANALOG_PRINCIPLES, Section 3.4] |
| Power net topology (star, daisy-chain, mesh) | [Hastings, Ch. 15] |
| Wire sizing (EM, resistance, capacitance, IR) | [FOLD, 7.3, 7.5]; [Hastings, Ch. 15]; [00_ANALOG_PRINCIPLES, Section 4] |
| Metal stack properties and layer assignment | [FOLD, 2.8, 7.3]; [Hastings, Ch. 2, Ch. 15] |
| Via resistance and redundancy | [Hastings, Ch. 15]; [FOLD, 2.8.4, 7.5.4] |
| Electromigration mitigation (Blech, reservoirs, topology) | [FOLD, 7.5]; [Hastings, Ch. 15]; [00_ANALOG_PRINCIPLES, Section 4.1] |
| OPC-friendly routing and manufacturability | [ALS 4.7]; [FOLD, 2.8.4] |
| Metal density and dummy fill | [FOLD, 2.8.4]; [Hastings, Ch. 2, Ch. 13] |
| Corner bends and current crowding | [FOLD, 7.5.4]; [Hastings, Ch. 15] |
| Net splitting for current density | [ALS 4.4.1] |
| Template-based routing | [ALS 4.5.6] |
| Integrated placement and routing | [ALS 4.5.4] |
| Contact/via technology (Al, W-plug, damascene) | [Hastings, Ch. 2]; [FOLD, 2.8] |
| Thermal and stress migration in routing | [FOLD, 7.5.2, 7.5.3, 7.5.5] |
| Hydrogenation blocking by metal | [Hastings, Ch. 13]; [00_ANALOG_PRINCIPLES, Section 2.4] |

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| GAP-05 | Added hard minimum-spacing enforcement of S1's crosstalk exclusion matrix to Section 6.2.B | S1's crosstalk exclusion matrix was not consumed by S3's obstacle union as hard constraints; only the soft Cost_coupling penalty existed |
| COV-01 | Added multi-patterning awareness note to Section 11.A | Checklist item 4.3 (double/multi-patterning rules) had no concrete mechanism; deferred to Phase 4 |
| COV-04 | Added layout regularity note to Section 2.A | Checklist item 5.6 (layout aesthetics/regularity) had no explicit comment; clarified that VAE model captures this implicitly |
| CONTRA-02 | Added Rule 3a (precedence) to Section 12.A after Rule 3 | Rule 1 (high-Z on highest metal) and Rule 3 (matched nets on same layer) could conflict; clarified that Rule 3 prevails for matched nets |
| MISSING-05 | Added Rule 7 (integrated inductor keep-out zone) to Section 12.A | Inductor keep-out zones and RF routing constraints were not addressed; deferred to Phase 4 |
| MISSING-08 | Added ESD bus metal corner chamfering rule (item 5) to Section 6.6.A | FOLD 7.4 ESD bus chamfering requirement was missing; prevents current crowding during ESD events |
