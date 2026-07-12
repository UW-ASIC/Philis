# ML Model Training Specifications

---

## 1. Overview of Learned Models

The engine uses three ML models. All augment classical algorithms -- none replaces a formal verifier.

| Model | Purpose | Architecture | Training | Stage |
|-------|---------|-------------|----------|-------|
| GNN symmetry extractor | Identify matching structures | GGRU on heterogeneous multigraph | Unsupervised | S1 |
| VAE routing guide | Predict routing probability maps | Convolutional VAE | Semi-supervised | S3 |
| GNN performance predictor | Estimate P(FOM < threshold) | PEA (Pooling with Edge Attention) | Supervised | S2 |

### 1.A Analog Considerations

**ML models are proxies, not ground truth.** In analog layout, the cost of a wrong prediction is not merely suboptimal performance -- it can be a non-functional circuit. A symmetry extractor that misses a matched pair produces a layout with systematic offset. A performance predictor that approves a bad placement wastes an entire PEX+SPICE iteration. A routing guide that ignores matching symmetry produces parasitic asymmetry.

**Key risk areas where ML predictions are dangerous:**

| Risk | Consequence | Mitigation |
|---|---|---|
| GNN misses a matched pair | Devices placed without symmetry constraint; systematic offset in silicon | Always validate GNN-extracted symmetry against a pattern-library fallback; flag unmatched differential pairs as warnings |
| GNN identifies a false symmetry | Over-constrains placement; wastes area; may create new asymmetries | Allow designer override; use confidence threshold (cosine similarity > 0.99 for device-level) |
| Performance predictor approves placement with poor matching | Saves one iteration but produces a failing circuit | Matching metrics (LDE mismatch, parasitic asymmetry) must be computed analytically, never delegated to the GNN predictor |
| VAE routing guide ignores matched-net symmetry | Routes matched nets on different layers or with different lengths | VAE output is guidance only; the A* router enforces symmetry as a hard constraint independent of VAE |
| Performance predictor optimizes area at cost of matching | Gradient drives placement toward compact but asymmetric arrangement | Matching penalty must dominate area penalty in GP objective, regardless of predictor gradient |

**The GNN performance predictor must NOT be the sole arbiter of matching quality.** The Pelgrom model, WPE model, and LOD model are deterministic and well-characterized. Their evaluation is cheap (O(N) per matched pair). There is no reason to approximate them with a neural network. The GNN predictor should focus on performance metrics that are expensive to compute (bandwidth, phase margin, gain) and leave matching evaluation to analytical models.

### 1.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Matching constraints (symmetry groups, LDE bounds) are never delegated to ML prediction; they use analytical models | All stages | Matching failures are deterministic and well-modeled; ML approximation adds risk without benefit |
| ML model outputs are always validated against physics-based checks before being acted upon | All stages | ML models can produce physically impossible predictions (e.g., suggesting a placement where WPE mismatch exceeds 50 mV) |
| The performance predictor gradient must never reduce the matching penalty weight | GP solver | Matching is priority #1; performance prediction is priority #3 (parasitic minimization) |

---

## 2. Model A: GNN Symmetry Extractor

**Source:** Chen et al., DAC 2021. Code: github.com/baloneymath/AncstrGNN.

### Architecture

```
Input: Heterogeneous multigraph G = (V, E)
       Node features: 18-dim (15 one-hot type + L + W + metal_layers)
       Edge types: 4 (gate, drain, source, passive)

Layer 1 (k=1):
  For each vertex v:
    h^(1)_v = GRU(h^(0)_v,  Sum_{u in N_in(v)}  W_{e_uv} * h^(0)_u)
  where W_{e_uv} is a learned linear transformation for edge type e_uv
  |W| = 4 matrices (one per edge type)

Layer 2 (k=2):
  h^(2)_v = GRU(h^(1)_v,  Sum_{u in N_in(v)}  W_{e_uv} * h^(1)_u)

Output: z_v = h^(2)_v in R^D,  D = 18
```

### Training

```
Loss: L(z_v) = -Sum_{u in N_in(v)} log(sigma(z_u^T * z_v))
              -Sum_{i=1}^B  E_{u_tilde~Neg(v)} log(1 - sigma(z_u_tilde^T * z_v))

where sigma = sigmoid, B = 5 negative samples per vertex.
Total loss: L_tot = Sum_{v in V} L(z_v)
Optimizer: Adam, lr = 0.001
Epochs: 200 (convergence typically at ~100)
```

### Training Data

- No labels needed (unsupervised).
- Collect circuits from any available source: open-source benchmarks (ALIGN, MAGICAL), internal designs, generated topologies.
- Minimum: 15-20 circuits for initial training. More circuits improve cross-topology generalization.
- All circuits converted to heterogeneous multigraphs.

#### 2.A Analog Considerations for Training Data

**The training circuits must include analog-specific structures.** The GNN learns structural similarity from the graph topology. If trained only on simple digital-like structures, it will fail to recognize:

| Analog Structure | Graph Signature | Why It Matters |
|---|---|---|
| Cascode current mirrors | Two stacked NMOS/PMOS pairs with shared gate nets | Must be recognized as a single matched group, not two independent pairs |
| Folded cascode OTA | Diff pair + cascode load + tail current source with cross-connected drains | System-level symmetry spans multiple hierarchy levels |
| Differential pair with active load | NMOS diff pair + PMOS current mirror load | The PMOS load pair must be matched to the NMOS diff pair symmetry axis |
| R-2R DAC ladder | Series/parallel resistor network with binary-weighted taps | All unit resistors must be identified as a single matching group |
| Bandgap reference | Matched PNP pair (or NPN pair) + resistor ratio network | Multiple matching groups with different tiers (NPN/PNP: exceptional, resistors: moderate) |
| Switched-capacitor integrator | Matched capacitor pair + switch transistors + opamp | Capacitor matching is exceptional; switch matching is minimal |

**Minimum training set should include:**
- At least 5 different OTA topologies (5T, folded cascode, telescopic, two-stage Miller, current-mirror OTA)
- At least 3 DAC/ADC structures (R-2R, binary-weighted capacitor, SAR logic)
- At least 2 bandgap references
- At least 2 comparators (StrongARM, double-tail)
- At least 2 voltage regulators (LDO, switching)

This ensures the GNN has seen the structural patterns that matter for analog symmetry extraction.

**The node feature vector should be enriched for analog.** The current 18-dimensional feature vector (15 one-hot type + L + W + metal_layers) does not capture:
- Device matching tier (which would help the GNN weight the importance of symmetry)
- Bias condition (devices at the same bias point are more likely to be matched)
- Current magnitude (high-current devices are power devices, not matched signal devices)

Adding these features would improve F1 score on analog circuits, particularly for distinguishing between matched signal pairs and unmatched power devices.

#### 2.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Training set must include at least 15 analog circuits spanning the topologies listed above | GNN training pipeline | Analog symmetry patterns differ from digital; insufficient coverage causes missed pairs |
| GNN-extracted symmetry must be validated against a pattern-library fallback for known topologies | S1 | Safety net against GNN failures on unfamiliar topologies |
| False negatives (missed matched pairs) are worse than false positives (extra symmetry constraints) | GNN threshold tuning | A missed pair produces a non-functional circuit; an extra constraint wastes area |

### Circuit Feature Embedding (for system-level)

```
For each subcircuit t:
  1. Build simplified directed graph G'_t (no parallel edges)
  2. Compute PageRank for all vertices in G'_t:
     PR(v) = (1-gamma)/|V_t| + gamma * Sum_{u in N_in(v)} PR(u)/|N_out(u)|
     gamma = 0.85 (damping factor)
  3. Select top-M vertices by PageRank (M=10, or |V_t| if smaller)
  4. Concatenate their feature vectors: z_t = [z_{v1}; z_{v2}; ...; z_{vM}]
```

### Similarity Thresholds

```
Device-level:  lambda_th = 0.99
System-level:  lambda_th = min(0.999, 0.95 + 0.95/(1 + |N_max_subcircuit|))

Classification: if cos(z_ti, z_tj) > lambda_th -> matched pair
```

### Retraining for New Process

Architecture is process-independent. Retrain on circuits from the target process:
- Collect 15+ circuits designed in the new PDK
- Run unsupervised training (~10 minutes on GPU)
- Verify F1 > 0.90 on a held-out test set

---

## 3. Model B: VAE Routing Guide

**Source:** Zhu et al., ICCAD 2019 (GeniusRoute).

### Architecture

```
ENCODER (shared across all stages):
  Input:  2 x 64 x 64 (channel 1: all pins, channel 2: target pins)
  Conv1:  5x5 kernel, 64 filters, stride 2 -> 32x32x64
  Conv2:  5x5 kernel, 128 filters, stride 2 -> 16x16x128
  FC:     16x16x128 -> 64
  mu, sigma:   FC 64 -> 32 each (latent dimension = 32)

DECODER (Stage 1, unsupervised):
  Input:   z in R^32 (sampled via reparameterization: z = mu + sigma*epsilon, epsilon~N(0,I))
  FC:      32 -> 16x16x64 (two parallel branches for 2-channel reconstruction)
  Deconv1: 4x4 kernel, 32 filters, stride 2 -> 32x32x32
  Deconv2: 4x4 kernel, 1 filter, stride 2 -> 64x64x1
  Output:  2 x 64 x 64 (reconstruction of input)

DECODER (Stage 2-3, supervised):
  Same as above but single branch -> 1 x 64 x 64 probability map
```

### Training Data

**Labeled data (per net type):**
- Differential nets: ~32 manual layouts x 4 augmentation = 128 samples
- Clock nets: ~42 manual layouts x 4 augmentation = 168 samples
- Power/ground: ~64 manual layouts x 4 augmentation = 256 samples

**Unlabeled data:** ~1590 layout-placement pairs x 4 augmentation = 6360 samples

**Data augmentation:** horizontal flip, vertical flip, 180-degree rotation.

**Source:** Manual layouts from experienced designers. Can include layouts from multiple circuit types (OTAs, comparators, ADCs). The model learns routing *strategy*, not circuit-specific patterns.

#### 3.A Analog Considerations for Training Data

**The training data must capture analog routing strategies, not just digital.** Analog routing differs from digital in fundamental ways:

| Analog Routing Strategy | What Training Data Must Show | Source |
|---|---|---|
| Matched net symmetry | Both nets of a matched pair routed as mirror images on the same metal layers with identical via stacks | [Hastings, Ch. 8, Rule 10; 00_ANALOG_PRINCIPLES, Section 3.4] |
| Parasitic equalization jogs | Dead-end branches or jogs added to the shorter net to equalize C | [Hastings, Ch. 8, Rule 10] |
| High-impedance node minimization | Cascode drain and opamp output nets routed with minimum length on lowest-C metal | [00_ANALOG_PRINCIPLES, Section 3.1] |
| Shielding | Ground metal interposed between noisy and sensitive signals | [Hastings, Ch. 15, Section 15.4.4] |
| Electrostatic shields at crossings | Metal-2 shield tied to quiet ground between noisy metal-3 and sensitive metal-1, with 2-5 um overhang | [Hastings, Ch. 15, Section 15.4.4] |
| Star nodes | All sensitive ground returns meeting at a single common point | [Hastings, Ch. 15, Section 15.4.4] |
| Kelvin connections | Separate force and sense leads for precision resistors | [Hastings, Ch. 15, Section 15.4.4] |
| No metal over matched gates | Routing avoids active gate regions of matched transistors | [Hastings, Ch. 13, Rule 17] |

**If training data does not show these strategies, the VAE will learn digital-style routing that is destructive for analog.** Specifically:
- Digital routing optimizes wirelength; analog routing sometimes adds wirelength to equalize parasitics
- Digital routing uses any available metal; analog routing avoids routing over matched gates
- Digital routing does not shield; analog routing requires shielding between noisy and sensitive nets

**Minimum analog training data:**
- At least 30 manually routed OTAs showing matched differential routing
- At least 20 manually routed ADC/DAC circuits showing capacitor array routing with shield planes
- At least 10 manually routed bias networks showing star-node ground returns
- All training layouts should be from experienced analog layout engineers, not auto-routed

**Additional net type for analog:** The current VAE trains separate decoders for differential, clock, and PG nets. An additional decoder should be trained for:
- **Matched analog nets** (gate connections of matched pairs, reference voltage distribution)
- **High-impedance nets** (cascode drains, opamp outputs)
- **Bias distribution nets** (current mirror gate buses)

#### 3.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| VAE output is guidance ONLY; the A* router enforces matching symmetry as a hard constraint independent of VAE | S3 router | The VAE may not learn perfect symmetry; hard constraints are the safety net |
| Training data must include analog-specific routing strategies (matched nets, shielding, star nodes) | VAE training pipeline | Without these, the VAE learns digital-style routing that degrades analog performance |
| VAE predictions that violate matching-gate keep-out zones must be overridden | S3 router | Hydrogenation blocking is not a soft penalty |
| VAE must be retrained when the routing strategy changes (e.g., adding shielding requirements) | VAE training pipeline | The model cannot learn strategies it has never seen |

### Three-Stage Training

```
Stage 1: Unsupervised feature extraction
  Objective: max log P(X|z) - D_KL[Q(z|X) || P(z)]
  Train on ALL data (labeled + unlabeled): 6360+ samples
  Learning rate: 0.001
  Epochs: 100
  Purpose: learn a general placement-feature encoder

Stage 2: Supervised decoder training
  Freeze encoder from Stage 1
  New decoder initialized randomly
  Separate model trained per net type (differential, clock, PG)
  Objective: min ||Y - Y_hat||_2  (Y = ground truth routing, Y_hat = prediction)
  Train on labeled data only: 128-256 samples per type
  Learning rate: 0.001
  Epochs: 200

Stage 3: Fine-tuning
  Unfreeze encoder, keep decoder from Stage 2
  Objective: max log P(Y|z) - D_KL[Q(z|X) || P(z)]
  Learning rate: 0.0001 (10x smaller)
  Epochs: 50
  Purpose: small joint adjustments for better accuracy
```

### Inference

```
Input: placement image (2-channel, 64x64)
Output: routing probability map (1-channel, 64x64)

For each net type needed:
  1. Preprocess placement into 2-channel image
  2. Run through trained encoder -> latent z
  3. Run through net-type-specific decoder -> probability map
  4. Pass probability map to A* router as guidance

Runtime: ~17 seconds total (dominated by model loading, not inference)
```

### Retraining for New Process/Circuit Family

- Retrain Stage 1 on unlabeled placement data from the new context
- Collect 30+ manual layouts per net type for Stages 2-3
- Full retraining: ~1 hour on GPU

---

## 4. Model C: GNN Performance Predictor

**Source:** Li et al., ICCAD 2020 (PEA network); used in ePlace-AP (Lin et al., DATE 2022).

### Architecture

```
Input: Circuit graph G = (V, E)
  Node features: device type, position (x,y), W, L, finger count
  Edge features: net connectivity, wire type

PEA (Pooling with Edge Attention) Network:
  Layer 1: Attention compression (reduce edge features)
  Layer 2: Graph convolution (aggregate neighbor features)
  Layer 3: Node pooling (compress graph)
  Layer 4: Edge pooling (compress edges)
  Flatten -> MLP -> output

Output: Phi(G) in [0, 1] = probability that FOM < threshold
```

#### 4.A Analog Considerations for Architecture and Features

**The node feature vector must include analog-specific features.** The current feature set (device type, position, W, L, finger count) does not capture the information that matters most for analog performance:

| Feature | Why It Matters for Analog | How to Compute |
|---|---|---|
| Matching group membership | Matched pairs dominate performance (offset, CMRR) | From S1 constraint file |
| Per-device WPE dVth (mV) | Systematic mismatch from well proximity | `dVth = sum a_k * exp(-d_k/lambda_k)` evaluated at current position |
| Per-device LOD f(SA,SB) | Systematic mismatch from STI stress | `f(SA,SB) = 1 + K1*(1/SA+1/SB) + K2*(1/SA^2+1/SB^2)` |
| Thermal gradient across matched pair (C) | Systematic Vth offset: -1 to -2 mV/C | From thermal model evaluated at current position |
| Net impedance at high-Z nodes (kOhm) | Parasitic C sensitivity: BW ~ 1/(2*pi*R*C) | From .op data |
| Current density in power nets (mA/um) | EM compliance | From .op data and wire widths |
| Asymmetric net features | One net of a matched pair is longer/has more vias | Computed from routing or estimated from placement |

**The FOM threshold must be analog-appropriate.** The current binary classification (FOM < threshold vs. FOM >= threshold) conflates all performance metrics into one. For analog, the performance predictor should predict multiple binary outcomes:

```
Phi_gain(G) = P(gain > spec_gain)
Phi_bw(G)   = P(BW > spec_BW)
Phi_pm(G)   = P(PM > spec_PM)
Phi_offset(G) = P(|offset| < spec_offset)
Phi_cmrr(G) = P(CMRR > spec_CMRR)
```

This allows the GP to compute separate gradients for each metric and identify which metric is the binding constraint.

**The performance predictor must NOT predict matching quality.** Matching quality is deterministic and well-modeled (Pelgrom + LDE). Using a neural network to approximate it introduces unnecessary noise and risk. The predictor should focus on metrics that are expensive to compute exactly (require full PEX + SPICE):
- Bandwidth (requires parasitic extraction)
- Phase margin (requires frequency-domain simulation)
- Gain (requires parasitic-loaded transconductance)
- Settling time (requires transient simulation)
- SNDR (requires FFT of transient simulation)

Leave these to analytical models:
- Offset (Pelgrom + LDE)
- CMRR (matched pair symmetry)
- PSRR (supply routing symmetry)
- INL/DNL (unit element matching)

#### 4.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Node features must include WPE dVth, LOD f(SA,SB), and thermal dT for matched devices | GNN feature extractor | These are the dominant analog performance factors; without them the predictor is blind to matching |
| The predictor must output per-metric probabilities, not a single FOM | GNN output layer | Different metrics have different remediation paths |
| The predictor must NOT be used for matching quality estimation | GP solver | Matching is deterministic and well-modeled; ML approximation adds risk |
| The predictor gradient for area must never override the analytical matching penalty | GP solver | Matching is priority #1 |

### Training Data

```
For each circuit topology:
  1. Generate >1000 placement variants by varying:
     - SA parameters (temperature, iterations)
     - Analytical placement parameters (weights)
     - Random perturbations of good placements
  
  2. For each placement:
     a. Route using A* router
     b. Extract parasitics (PEX)
     c. Simulate (SPICE)
     d. Compute FOM
     e. Label: 0 if FOM >= threshold, 1 if FOM < threshold
  
  3. Train PEA on labeled (placement, label) pairs
     Loss: binary cross-entropy
     Optimizer: Adam, lr = 0.001
     Epochs: 300
     Train/val split: 80/20
```

#### Training Data Analog Considerations

**The training data must capture the full range of analog placement quality, including matching failures.** If the training set only includes "reasonable" placements (from a matching-aware placer), the predictor will never learn what a bad matching configuration looks like. The training set should include:

| Placement Category | Fraction of Training Set | What It Teaches |
|---|---|---|
| Good placements (matching-aware, compact) | 30% | What a passing configuration looks like |
| Good matching, poor area | 20% | That matching quality matters more than area |
| Poor matching, good area | 20% | That compact but asymmetric placements fail |
| Random perturbations of good placements | 20% | Sensitivity to small position changes |
| Deliberately bad placements (orientation mismatch, WPE violation) | 10% | Hard failure modes that the predictor must reject |

**The training data must include analog-specific performance metrics as labels.** Beyond a single binary FOM, each training sample should be labeled with:
- Gain (dB)
- Bandwidth (MHz)
- Phase margin (degrees)
- Offset (mV) -- from Monte Carlo mismatch simulation
- CMRR (dB)
- PSRR (dB)
- Slew rate (V/us)
- Settling time (ns)

This enables multi-task learning where the predictor learns separate relationships between placement and each metric.

**Not just digital benchmarks.** If the training data comes from digital benchmarks (as many academic systems use), the predictor will learn relationships that do not hold for analog. Specifically:
- Digital: wirelength is the dominant proxy for timing. Analog: matching symmetry dominates offset/CMRR.
- Digital: area correlates with cost. Analog: area correlates with matching quality (larger devices match better).
- Digital: congestion predicts routing difficulty. Analog: parasitic asymmetry predicts performance degradation.

The training data must come from analog circuits with analog performance metrics.

### Use in Placement (Gradient Computation)

```
During ePlace-AP global placement:
  At each iteration k:
    1. Construct graph G from current positions v^k
    2. Forward pass: Phi(G) = PEA(G)
    3. Backward pass: compute dPhi/dv via autodiff (TensorFlow/PyTorch)
    4. Add alpha * (-dPhi/dv) to total gradient
       (negative because we want to MINIMIZE Phi, i.e., reduce failure probability)

Note: computing dPhi/dv is ~10x more expensive than computing Phi alone.
This is why ePlace-AP is only 3x faster than SA (vs. 55x for ePlace-A without performance).
```

#### Gradient Computation Analog Considerations

**The performance predictor gradient must be bounded relative to the matching gradient.** If the predictor gradient drives a matched pair apart (to improve predicted bandwidth, for example), the analytical matching penalty must dominate:

```
|alpha * dPhi/dv| <= 0.1 * |delta * dLDE/dv|  at all iterations
```

This prevents the predictor from "discovering" that moving a matched pair apart reduces parasitic loading (which is true) at the cost of destroying matching (which is catastrophic).

**Gradient clipping for stability.** The performance predictor gradient can be noisy, especially early in training or for out-of-distribution placements. Clip the predictor gradient to prevent it from dominating:
```
dPhi/dv = clip(dPhi/dv, -max_grad, max_grad)
where max_grad = 0.1 * mean(|dLDE/dv|) over all matched pairs
```

### Transfer Learning

The PEA model trained on one circuit topology (e.g., cascode OTA) can be fine-tuned for another (e.g., two-stage OTA) with ~100 new samples. Cross-topology accuracy drops 3-22% without fine-tuning (CUHK ASP-DAC 2024 survey).

---

## 5. Hardware Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| GPU | NVIDIA GTX 1080 (8 GB) | NVIDIA A100 (40 GB) |
| CPU | 4 cores, 16 GB RAM | 16 cores, 64 GB RAM |
| Storage | 10 GB for models + data | 100 GB for full training pipeline |
| Training time (all models) | ~4 hours | ~1 hour |
| Inference time (per circuit) | ~30 seconds | ~5 seconds |

### 5.A Analog Considerations

**The bottleneck for analog ML training is data generation, not model training.** Generating one training sample requires:
1. Placing the circuit (seconds to minutes)
2. Routing the circuit (seconds to minutes)
3. PEX extraction (minutes to hours)
4. SPICE simulation (minutes to hours)

For 1000 training samples per topology, this translates to days or weeks of computation. The training pipeline should:
- Run PEX + SPICE in parallel across multiple CPU cores
- Cache intermediate results (placement -> routing -> PEX -> SPICE) to avoid redundant computation
- Use coarse PEX (R+C only, no CC coupling) for initial training, fine PEX (R+C+CC) for final validation
- Consider using analytical parasitic estimates (from wire geometry) as a fast proxy for PEX during early training rounds

**Monte Carlo simulation for matching labels.** Computing offset, CMRR, and PSRR requires Monte Carlo mismatch simulation (100+ samples per placement variant). This multiplies the SPICE runtime by 100x. For practical training:
- Use the Pelgrom + LDE analytical model to estimate offset/CMRR for the bulk of training data (cheap, deterministic)
- Run full Monte Carlo only on a validation subset (10-20% of training data)
- Train the predictor on analytical matching estimates, validate on Monte Carlo results
