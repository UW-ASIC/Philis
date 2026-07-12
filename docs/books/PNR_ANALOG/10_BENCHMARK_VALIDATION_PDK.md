# Benchmark, Validation Plan, and PDK Abstraction

---

## 1. Benchmark Circuit Suite

### 1.1 Phase 1 Benchmarks (Minimum Viable Pipeline)

| Circuit | Devices | Nets | Complexity | Key metrics | Source |
|---------|---------|------|-----------|-------------|--------|
| 5T-OTA | 5 | 6 | Trivial | Gain, UGB, PM | Standard |
| Telescopic OTA | 8 | 10 | Easy | Gain, UGB, PM, CMRR | Standard |
| Simple current mirror | 4 | 4 | Trivial | Current ratio, Rout | Standard |
| NMOS diff pair (standalone) | 2 | 4 | Trivial | Offset, gm matching | Standard |

**Success criteria:** DRC-clean, LVS-pass for all. Post-layout gain within 20% of schematic for OTAs.

#### 1.1.A Analog Considerations

**Even Phase 1 benchmarks should validate matching.** A DRC/LVS-clean layout that has 15% current mismatch in the current mirror or 10 mV offset in the diff pair is not a successful analog layout. Phase 1 success criteria should include:

| Circuit | Analog Pass Criterion | Rationale | Source |
|---|---|---|---|
| Simple current mirror | Current ratio within 5% of target (i.e., current mismatch < 5%) | A current mirror that does not mirror current is non-functional | [Hastings, Ch. 13]: minimal matching = 2-5% current mismatch |
| NMOS diff pair | Estimated sigma(dVth) < 15 mV (6-sigma, minimal tier) | A diff pair with > 15 mV offset is marginal for most applications | [Hastings, Ch. 13]: minimal MOS matching = 5-15 mV voltage mismatch |
| 5T-OTA | Post-layout offset < 5x schematic offset (with Monte Carlo mismatch) | The layout should not dramatically worsen the circuit's inherent offset | [Hastings, Ch. 8]: matching by construction |
| All circuits | Orientation chi identical for all matched devices | Orientation mismatch causes ~15% gm error, the #1 systematic mismatch source | [Hastings, Ch. 13] |
| All circuits | Guard rings present on all potential latchup injectors | Latchup is destructive | [Hastings, Ch. 14]; [Lienig, 7.1.3] |

### 1.2 Phase 2 Benchmarks (Matching by Construction)

| Circuit | Devices | Nets | Complexity | Key metrics | Source |
|---------|---------|------|-----------|-------------|--------|
| Folded cascode OTA | 12-16 | ~15 | Medium | Gain, UGB, PM, offset, CMRR | Standard |
| Two-stage Miller OTA | 10-14 | ~12 | Medium | Gain, UGB, PM, slew rate | Razavi Ch.9 |
| StrongARM comparator | 10-16 | ~12 | Medium | Delay, offset, noise | ALIGN benchmark |
| Current mirror OTA | ~20 | ~15 | Medium | Gain, BW, CMRR, offset | ALIGN benchmark |
| Switched-cap filter (1st order) | ~30 | ~20 | Hard | Gain, BW, settling time | ALIGN benchmark |

**Success criteria:** DRC/LVS clean. Post-layout performance within 15% of schematic on all primary metrics. Offset < 2x manual layout offset.

#### 1.2.A Analog Considerations

**Phase 2 must validate common-centroid effectiveness.** The key question Phase 2 answers is: "Does the matching-by-construction approach actually produce matched devices?" The following additional validation steps are required:

| Validation Step | What It Checks | How to Measure |
|---|---|---|
| CC pattern verification | Are matched devices in a valid CC arrangement (coinciding centroids, symmetric about both H and V axes)? | Compute centroids of matched devices; verify coincidence within 1 um |
| Dummy device verification | Are dummy devices present at all array edges? Are they OD-merged with active devices? | Geometric check: dummy poly present at array boundaries; moat continuous from first dummy to last dummy |
| WPE equalization verification | Are matched devices at equal distance from well edges? | Compute `|dVth_WPE(i) - dVth_WPE(j)|` for each matched pair; should be < 1 mV for moderate matching |
| LOD equalization verification | Are SA and SB identical for all active devices in a matched array? | Compute `|SA_i - SA_j|` and `|SB_i - SB_j|`; should be < 0.1 um for moderate matching |
| Parasitic symmetry verification | Are matched nets routed on the same layers with the same via stacks? | Extract R and C for each net in a matched pair; compute `|R_a - R_b|/R_avg` and `|C_a - C_b|/C_avg`; should be < 5% |
| Orientation verification | Do all devices in a matching group have identical orientation chi? | Compute chi = (1/N)*sum(chi_i) for each multi-finger device; verify match |

**Monte Carlo offset simulation.** For each OTA benchmark, run 100-point Monte Carlo mismatch simulation and report:
- Mean offset (mV)
- Sigma offset (mV)
- 3-sigma offset (mV)
- Comparison to manual layout offset (if available)

The 3-sigma offset is the true quality metric, not the nominal simulation result.

### 1.3 Phase 3 Benchmarks (Physics-Aware)

| Circuit | Devices | Nets | Complexity | Key metrics | Source |
|---------|---------|------|-----------|-------------|--------|
| 5-bit SAR ADC | ~100 | ~50 | Hard | SNDR, SFDR, INL, DNL | ALIGN/MAGICAL |
| 2nd-order CT Delta-Sigma modulator | ~200 | ~120 | Very hard | SNDR, SFDR, power | MAGICAL (ADC1) |
| R-2R DAC | ~30 | ~20 | Medium | INL, DNL, settling | ALIGN |
| 10-tap FIR equalizer | ~100 | ~50 | Hard | Eye opening, BER | ALIGN |
| LDO regulator | ~20 | ~15 | Medium | Ripple, load reg, PSRR | ALIGN |
| VCO (ring or LC) | ~15 | ~10 | Medium | Freq, phase noise, power | ALIGN |
| 4x4 MIMO receiver | ~200 | ~100 | Very hard | Gain, NF, B1dB | ALIGN (TSMC65) |

**Success criteria:** DRC/LVS/antenna clean. Post-layout within 10% of schematic. Performance competitive with manual (within 5% on primary metrics). Runtime < 60 min for largest circuits.

#### 1.3.A Analog Considerations

**Phase 3 benchmarks must validate matching in data converters.** The SAR ADC, Delta-Sigma modulator, R-2R DAC, and FIR equalizer all contain precision matched elements (unit capacitors, unit current sources, or unit resistors). The pass criterion for these circuits is NOT just SNDR/SFDR/INL/DNL within spec -- it is that the matching of unit elements is maintained through the layout process.

| Circuit | Matching Validation | Pass Criterion | Source |
|---|---|---|---|
| 5-bit SAR ADC | Unit capacitor matching | INL < 0.5 LSB, DNL < 0.5 LSB (implies capacitor matching < 0.5/2^5 = 1.6%) | [Hastings, Ch. 8]: exceptional capacitor matching |
| Delta-Sigma modulator | Integrator capacitor ratio matching; OTA offset | SNDR > spec; OTA offset < 1 mV (moderate matching) | [Hastings, Ch. 8, Ch. 13] |
| R-2R DAC | Resistor ratio matching (R and 2R segments) | INL < 0.5 LSB; all R segments from identical unit resistors; 2R from two unit R in series | [Hastings, Ch. 8]: 25 resistor matching rules |
| LDO regulator | Feedback resistor ratio matching; error amplifier offset | Load regulation < spec; PSRR > spec | [Hastings, Ch. 8] |

**Thermal validation for Phase 3.** The LDO regulator and power-handling circuits create thermal gradients. Validate that:
- The error amplifier diff pair is placed along an isotherm (temperature difference < 0.5 C)
- The power transistor is placed far from precision analog (>= 1 um/mW for exceptional) [Hastings, Ch. 8, Rule 13]
- The bandgap reference (if present) is at the coolest die location

**Substrate noise validation for mixed-signal Phase 3 circuits.** The SAR ADC and Delta-Sigma modulator have both analog and digital sections. Validate that:
- Analog and digital domains have separate supply domains [00_ANALOG_PRINCIPLES, Section 4.4]
- Guard rings provide >= 20 dB attenuation between digital switching noise and analog inputs
- Deep N-well isolation used for critical analog blocks where available (40-60 dB isolation)

### 1.4 Phase 4 Benchmarks (Advanced Node)

Target: FinFET circuits from LAYGO2/AutoCRAFT benchmarks (GlobalFoundries 12nm, ASAP7, or commercial FinFET PDK).

#### 1.4.A Analog Considerations

**FinFET matching differs from planar.** In FinFET processes:
- W is quantized (integer number of fins), so Pelgrom area scaling must use quantized W
- LDE effects are amplified due to smaller geometries [00_MASTER_ARCHITECTURE.md Section 8]
- Grid-based placement (LAYGO-style) constrains CC pattern options
- Self-heating is first-order: fin structures have poor thermal conductivity

---

## 2. Validation Methodology

### 2.1 Per-Circuit Validation Protocol

```
For each benchmark circuit:

  Step 1: SCHEMATIC BASELINE
    Simulate the schematic (pre-layout) to establish baseline metrics.
    Record: all performance metrics, DC operating points, noise.

  Step 2: AUTOMATED LAYOUT
    Run the full pipeline (S0->S1->S2->S3->S4).
    Record: runtime per stage, constraint report, iteration counts.

  Step 3: PHYSICAL VERIFICATION
    DRC: 0 violations required.
    LVS: 0 errors required.
    Antenna: all fixed.
    Density: all tiles within bounds.

  Step 4: POST-LAYOUT SIMULATION
    PEX extraction (R+C+CC).
    SPICE simulation on extracted netlist.
    Record: all performance metrics.
    Compare to schematic baseline:
      degradation_pct = (metric_schematic - metric_postlayout) / metric_schematic * 100

  Step 5: MANUAL COMPARISON (where available)
    Compare automated layout metrics to manual layout metrics.
    Record: area comparison, performance comparison, runtime comparison.
    
  Step 6: PROCESS CORNERS (Phase 3+)
    Simulate at TT, FF, SS corners.
    Simulate at temperature extremes (-40C, 25C, 125C).
    Record: worst-case performance across corners.

  Step 7: MONTE CARLO (Phase 3+)
    Run 100+ Monte Carlo samples.
    Record: mean, sigma, yield at each metric.
```

#### 2.1.A Analog Considerations

**The validation protocol must include matching-specific steps.** Between Step 4 and Step 5, insert:

```
  Step 4b: MATCHING VERIFICATION
    For each matched pair:
      a. Extract WPE dVth mismatch from placement geometry
         dVth_WPE = |sum a_k*exp(-d_k_i/lambda_k) - sum a_k*exp(-d_k_j/lambda_k)|
      b. Extract LOD dId/Id mismatch from SA/SB values
         dId/Id = |f(SA_i,SB_i) - f(SA_j,SB_j)|
      c. Extract parasitic asymmetry from PEX
         dR = |R_net_a - R_net_b|
         dC = |C_net_a - C_net_b|
      d. Verify orientation chi match
      e. Verify dummy device presence and OD-merge
      f. Verify guard ring completeness
      g. Verify no metal over matched gates
      h. Compute total estimated sigma(dVth):
         sigma_total = sqrt(sigma_random^2 + dVth_WPE^2 + (dId/Id * (Vgs-Vth)/2)^2)
      i. Compare to matching tier target:
         PASS if sigma_total < tier_limit (minimal: 15 mV, moderate: 3 mV, exceptional: 0.3 mV)
```

**The validation protocol must include the 17-point checklist [Hastings, Ch. 15, Section 15.5].** After Step 3 (physical verification), run the automated version of Hastings' final layout checklist:

```
  Step 3b: FINAL LAYOUT CHECKLIST
    [ ] DRC diagnostics reviewed
    [ ] LVS topological errors = ZERO
    [ ] LVS parametric errors reviewed
    [ ] Electromigration compliance (all power wires)
    [ ] High-current lead resistances verified
    [ ] Antenna violations corrected
    [ ] ESD compliance (primary + CDM clamps)
    [ ] Latchup checks (guard rings on vulnerable diffusions)
    [ ] Dummy metal block zones for matched devices
    [ ] Noisy/sensitive signal separation verified
    [ ] Bondpads properly sized and located
    [ ] Kelvin connections properly located
    [ ] Corner exclusion zones implemented
    [ ] Scribe seals present and correct
    [ ] Substrate contacts filling empty die areas
    [ ] Die symbolization present
    [ ] Layout archived
```

#### 2.1.B Analog-Algorithm Interface

| Contract | Binding on | Rationale |
|---|---|---|
| Matching verification (Step 4b) must produce a per-pair report, not just a pass/fail | Validation framework | The worst pair determines yield; identifying it enables targeted optimization |
| The 17-point checklist must be automated where possible (items 1-10 can be automated; 11-17 may require manual review) | Validation framework | Manual checklist completion is error-prone and does not scale |
| Monte Carlo simulation must use process-calibrated mismatch models, not generic sigma values | Validation framework | Inaccurate mismatch models produce misleading yield estimates |

### 2.2 Aggregate Quality Metrics

| Metric | Definition | Target |
|--------|-----------|--------|
| DRC pass rate | % of benchmarks with 0 DRC violations | 100% |
| LVS pass rate | % of benchmarks with 0 LVS errors | 100% |
| Avg performance degradation | Mean of |degradation_pct| across all metrics and circuits | < 10% |
| Max performance degradation | Worst single metric degradation across all circuits | < 25% |
| Avg area ratio (auto/manual) | Mean of automated area / manual area | < 1.5 (ideally < 1.1) |
| Avg runtime (block-level) | Mean runtime for circuits with < 50 devices | < 5 min |
| Constraint extraction F1 | F1-score of auto-extracted vs. designer constraints | > 0.90 |
| Symmetry degree (d_SYM) | Mean wirelength-weighted symmetry ratio | > 0.70 |

#### 2.2.A Analog Considerations

**Additional aggregate quality metrics specific to analog:**

| Metric | Definition | Target | Source |
|---|---|---|---|
| Matching pass rate | % of matched pairs meeting their tier's sigma(dVth) target | 100% | [Hastings, Ch. 13] |
| Avg matching margin | Mean of (tier_limit - sigma_total) / tier_limit across all matched pairs | > 30% (sufficient design margin) | Engineering practice |
| Guard ring coverage rate | % of identified injectors with complete guard rings | 100% | [Hastings, Ch. 14] |
| IR drop compliance | % of analog supply pins with IR drop < 5% Vsupply | 100% | [00_ANALOG_PRINCIPLES, Section 3.3] |
| Parasitic budget compliance | % of sensitive nets with parasitic C within budget | 100% | [00_ANALOG_PRINCIPLES, Section 3.2] |
| Noisy/sensitive separation | % of noisy-sensitive signal pairs with adequate separation or shielding | 100% | [Hastings, Ch. 15] |
| Monte Carlo yield (Phase 3+) | % of benchmarks achieving > 99.7% (3-sigma) yield on all specs | 100% | Industry standard |
| Offset degradation ratio | automated_offset / manual_offset (lower is better) | < 2.0 | Matching by construction principle |

### 2.3 Regression Testing

After any code change:
1. Run Phase 1 benchmarks (< 5 min total). All must pass DRC/LVS.
2. Run Phase 2 benchmarks weekly (< 30 min total). Track performance metrics.
3. Run Phase 3 benchmarks before each release (< 4 hours total). Full validation.

#### 2.3.A Analog Considerations

**Regression testing must include matching regression.** A code change that improves area by 5% but worsens matching by 1% is a regression for analog. The regression test suite must track:
- Matching quality (sigma(dVth) per matched pair) across runs
- Alert if any matched pair's sigma(dVth) increases by more than 10% between runs
- Track the Pareto front (area vs. matching) and alert if any new run is Pareto-dominated by a previous run

---

## 3. PDK Abstraction Layer

### 3.1 Purpose

Decouple the engine from any specific foundry PDK. Technology-dependent parameters are isolated in configuration files; the engine's algorithms are technology-independent.

### 3.2 PDK Configuration File: `layers.json`

```json
{
  "technology": "TSMC40nm",
  "metal_stack": [
    {
      "layer": "M1",
      "gds_layer": 31,
      "direction": "horizontal",
      "width": [0.07, 0.14, 0.28],
      "pitch": 0.14,
      "spacing": 0.07,
      "end_to_end": 0.07,
      "min_length": 0.07,
      "max_length": null,
      "unit_R": 0.13,
      "unit_C": 0.038,
      "unit_CC": 0.020,
      "eol_width": 0.09,
      "eol_space": 0.09,
      "eol_within": 0.05,
      "min_area": 0.02,
      "min_step": 0.07,
      "max_edges_min_step": 2,
      "J_max_dc": 1.0,
      "J_max_ac": 5.0
    }
  ],
  "via_stack": [
    {
      "via": "V12",
      "lower": "M1",
      "upper": "M2",
      "width": 0.07,
      "spacing": 0.09,
      "enclosure_lower": 0.03,
      "enclosure_upper": 0.03,
      "R_per_cut": 5.0,
      "I_max_per_cut": 0.2
    }
  ],
  "devices": {
    "nmos_lvt": {
      "min_L": 0.04,
      "min_W": 0.12,
      "max_W_per_finger": 10.0,
      "poly_pitch": 0.10,
      "fin_pitch": null
    }
  },
  "lde_parameters": {
    "wpe": {
      "a_nmos": 15.0,
      "a_pmos": 20.0,
      "lambda_nmos": 2.5,
      "lambda_pmos": 3.0
    },
    "lod": {
      "K1_nmos": -0.02,
      "K1_pmos": 0.015,
      "K2_nmos": -0.001,
      "K2_pmos": 0.0008
    }
  },
  "matching": {
    "A_VT_nmos": 4.5,
    "A_VT_pmos": 5.2,
    "A_beta_nmos": 1.8,
    "A_beta_pmos": 2.5
  },
  "guard_ring": {
    "pplus_width": 0.3,
    "nplus_width": 0.3,
    "ring_to_active_spacing": 0.5,
    "max_latchup_distance": 25.0,
    "tap_spacing": 15.0
  },
  "antenna_rules": {
    "max_ratio_metal": 400,
    "max_ratio_poly": 200,
    "diode_cell": "ANTDIODE"
  },
  "density_rules": {
    "min_density": 0.20,
    "max_density": 0.80,
    "tile_size": 50.0
  },
  "em_blech": {
    "JL_product_threshold": 3000
  }
}
```

#### 3.2.A Analog Considerations

**Design rule robustness [FOLD, Section 3.4-3.5].** All design rules in `layers.json` should be the robust (production) values, not nominal process capability values. Robust design rules incorporate safety margins for manufacturing tolerances (overlay error, CD variation, etch bias); non-robust rules match only nominal process capability. If the PDK provides both robust and non-robust rule sets, always use the robust set. Analog layouts must never target non-robust rules because parametric yield depends on these margins -- a design rule violation that is "acceptable" for digital yield targets may cause systematic mismatch or reliability degradation in analog circuits.

**The layers.json must include analog-specific parameters that textbooks emphasize:**

```json
{
  "analog_extensions": {
    "routing_pitches": {
      "P_LTL_M1_um": 0.14,
      "P_VTV_M1_um": 0.17,
      "P_VTL_M1_um": 0.155,
      "comment": "Routing pitches per Hastings Ch. 15: P_LTL = W_m + S_m; P_VTV = W_v + 2*M_ov + S_m"
    },
    "high_voltage_spacing": {
      "voltage_dependent_metal_spacing": [
        {"max_voltage_V": 5.0, "min_spacing_um": 0.07},
        {"max_voltage_V": 12.0, "min_spacing_um": 0.20},
        {"max_voltage_V": 40.0, "min_spacing_um": 1.0}
      ],
      "comment": "Voltage-dependent spacing per Hastings Ch. 15 Section 15.4.4"
    },
    "em_temperature_derating": {
      "reference_temp_C": 100,
      "activation_energy_eV": 0.7,
      "current_exponent_n": 2,
      "comment": "Black's Law derating: D = exp[(Ea/k)*(1/T_d - 1/T_r)]^(1/n) per Hastings Ch. 15"
    },
    "substrate": {
      "type": "P_minus",
      "resistivity_ohm_cm": 15.0,
      "epi_thickness_um": 10.0,
      "thermal_conductivity_W_per_mK": 148,
      "comment": "Substrate properties for thermal and noise models per 00_ANALOG_PRINCIPLES"
    },
    "process_type": {
      "isolation": "STI",
      "well_type": "twin_well",
      "has_nbl": true,
      "has_deep_nplus": true,
      "has_deep_nwell": true,
      "has_dti": false,
      "comment": "Process architecture per Hastings Ch. 4"
    },
    "matching_tiers": {
      "minimal_mos_dVth_6sigma_mV": 15.0,
      "moderate_mos_dVth_6sigma_mV": 3.0,
      "exceptional_mos_dVth_6sigma_mV": 0.3,
      "minimal_res_accuracy_6sigma_pct": 1.0,
      "moderate_res_accuracy_6sigma_pct": 0.1,
      "exceptional_res_accuracy_6sigma_pct": 0.01,
      "comment": "Matching tier targets per Hastings Ch. 8 and Ch. 13"
    },
    "guard_ring_types": {
      "ecgr_efficiency_pct": 90,
      "hbgr_efficiency_pct": 98,
      "ecgr_min_width_um": 1.0,
      "hbgr_must_encircle": true,
      "comment": "Guard ring efficiencies per Hastings Ch. 14"
    },
    "esd": {
      "ground_ring_max_resistance_ohm": 1.0,
      "cdm_rc_time_constant_ns": 10.0,
      "hbm_peak_current_A": 1.3,
      "adiabatic_delta_T_max_C": 200.0,
      "comment": "ESD design parameters per Hastings Ch. 14"
    },
    "die_area_estimation": {
      "packing_factor_cmos_bicmos": 1.5,
      "routing_factor_dlm": 1.2,
      "die_level_packing_factor": 1.15,
      "padring_width_um": 250,
      "scribe_width_um": 70,
      "comment": "Die area estimation parameters per Hastings Ch. 15 Section 15.1"
    }
  }
}
```

### 3.3 PDK Porting Checklist

When migrating to a new PDK:

```
[ ] Fill layers.json with the new PDK's design rules
[ ] Create/adapt device generators (transistor, capacitor, resistor PCells)
    - Verify DRC-clean on a single device of each type
[ ] Extract LDE parameters from the PDK's compact model documentation
    - WPE: a, lambda per device type
    - LOD: K1, K2 per device type
[ ] Extract matching coefficients from the PDK's matching data
    - A_VT, A_beta per device type per corner
[ ] Configure guard ring and well-tap rules
[ ] Configure antenna rule ratios and protection diode cell name
[ ] Configure density fill rules (min/max, tile size)
[ ] Set EM limits (J_max per layer, Blech length product)
[ ] Generate test layouts for Phase 1 benchmarks
[ ] Validate DRC/LVS on test layouts
[ ] Retrain GNN symmetry model on circuits from the new PDK (optional but recommended)
[ ] Retrain VAE routing model on manual layouts from the new PDK (optional but recommended)
```

#### 3.3.A Analog Considerations

**Additional porting checklist items for analog:**

```
[ ] Verify Pelgrom coefficients (A_VT, A_beta) against PDK matching data
    - Run a simple diff pair Monte Carlo to confirm sigma(dVth) matches A_VT/sqrt(WL) prediction
[ ] Characterize WPE: place test devices at varying distances from well edge, simulate, extract a_k and lambda_k
[ ] Characterize LOD: place test devices with varying SA/SB, simulate, extract K1 and K2
[ ] Verify guard ring effectiveness: run TCAD or test-chip measurement to confirm ECGR/HBGR efficiencies
[ ] Measure substrate resistivity: needed for thermal model and substrate noise model
[ ] Configure voltage-dependent spacing rules for high-voltage analog pins
[ ] Configure EM temperature derating parameters (activation energy, current exponent)
[ ] Identify available analog device options:
    - Pocket-implant vs. non-pocket (non-pocket preferred for long-channel matching) [Hastings, Ch. 13]
    - Thin-oxide vs. thick-oxide (thin-oxide preferred: A_VT ~ 1 mV*um per nm tox) [Hastings, Ch. 13]
    - Poly resistor types (LSR, MSR, HSR) and their sheet resistances [Hastings, Ch. 4]
    - Capacitor types (poly-poly, MOS, MIM) and their matching potential [Hastings, Ch. 8]
[ ] Determine process type for guard ring selection:
    - Standard bipolar: ECGR marginal, rely on HCGR+HBGR
    - BiCMOS with NBL: ECGR >90%, HBGR >95%
    - CMOS without deep N-well: single guard ring 20-40 dB
    - CMOS with deep N-well: 40-60 dB isolation
[ ] Configure OPC behavior:
    - Analog uses less OPC than digital [Hastings, Ch. 3, Section 3.3.4]
    - Gate mask: at least mild OPC for submicron gate lengths
    - Moat/active mask: may use OPC
    - Other layers: typically no OPC needed
[ ] Verify coding grid vs. address unit compatibility
    - Coding unit must be a multiple of AU/scale [Hastings, Ch. 3, Section 3.3.1]
    - Process size adjusts must be multiples of this quantum
```

### 3.4 Technology-Independent vs. Technology-Dependent Components

| Component | Technology-dependent? | What changes per PDK |
|-----------|----------------------|---------------------|
| S0 device generators | YES | Device geometry, construction, DRC rules |
| S0 pattern selector | No (algorithm) | Parameters come from layers.json |
| S1 GNN architecture | No | Retrain weights on new circuits |
| S1 LDE analyzer | Parameterized | Coefficients from layers.json |
| S2 GP solver | No | Design-rule spacings from layers.json |
| S2 ILP formulation | No | Grid pitch from layers.json |
| S3 A* router | Parameterized | Layer costs, spacing rules from layers.json |
| S3 VAE model | No (architecture) | Retrain weights on new layouts |
| S4 DRC engine | YES | Full DRC deck (Calibre rule file) |
| S4 LVS engine | Parameterized | Device extraction rules |
| S4 PEX engine | YES | Extraction tech file |
| S4 SPICE simulator | Parameterized | Device models (.lib) |

#### 3.4.A Analog Considerations

**Additional technology-dependent components for analog:**

| Component | Technology-dependent? | What changes per PDK | Source |
|---|---|---|---|
| Matching tier targets (mV, %) | Yes | Process-specific A_VT, A_beta determine achievable matching | [Hastings, Ch. 8, Ch. 13] |
| Guard ring type selection | Yes | Depends on process (BiCMOS w/ NBL vs. std CMOS) | [Hastings, Ch. 14] |
| ESD protection device selection | Yes | Available devices depend on process (GGNMOS, LDMOS, PNP) | [Hastings, Ch. 14] |
| Substrate noise model | Yes | Depends on substrate resistivity, epi thickness, isolation type | [00_ANALOG_PRINCIPLES, Section 4.4] |
| Thermal model parameters | Yes | Depends on substrate thickness, thermal conductivity | [00_ANALOG_PRINCIPLES, Section 4.3] |
| OPC behavior | Parameterized | Analog uses less aggressive OPC; process-dependent | [Hastings, Ch. 3, Section 3.3.4] |
| Die area estimation factors | Parameterized | Packing factors differ per process generation | [Hastings, Ch. 15, Section 15.1] |

**No universal scaling laws exist for analog [Hastings, Ch. 3, Section 3.2.4].** The PDK abstraction layer must NOT assume that parameters from one node can be scaled to another. Each PDK requires independent characterization of all analog-relevant parameters.

---

## 4. Open Source Dependencies

| Tool | Purpose | License | Required? |
|------|---------|---------|-----------|
| Python 3.10+ | Orchestration, ML | PSF | Yes |
| PyTorch / TensorFlow | GNN and VAE training and inference | BSD/Apache | Yes |
| Gurobi / CPLEX / CBC | ILP solver for detailed placement | Commercial/EPL | Yes (one of) |
| KLayout | GDSII I/O, DRC (open-source alternative) | GPL | Optional |
| Magic VLSI | Open-source DRC/extraction | BSD | Optional |
| ngspice / Xyce | Open-source SPICE simulation | BSD | Optional |
| Calibre (Siemens) | Production DRC/LVS/PEX | Commercial | For production |
| Spectre (Cadence) | Production SPICE | Commercial | For production |
| ALIGN | Reference flow, benchmarks | BSD | Recommended |
| MAGICAL | Reference flow, benchmarks | BSD | Recommended |

---

## 5. Recommended Open-Source Starting Points

For a team starting from scratch:

1. **Start with ALIGN's open-source codebase** (github.com/ALIGN-analoglayout/ALIGN-public) as the skeleton. It provides netlist parsing, hierarchy extraction, primitive cell generation, sequence-pair placement, and gridded routing. Port to your target PDK.

2. **Integrate MAGICAL's constraint extraction and routing** (github.com/magical-eda/MAGICAL) for the GNN symmetry extractor and the silicon-proven detailed router.

3. **Implement ePlace-A/AP** (the DATE 2022 analytical placement) as a replacement for ALIGN's SA placer. The authors' code is not open-source, but the paper provides complete algorithmic detail.

4. **Train GeniusRoute's VAE** on manual layouts from your design team. The paper provides the full network architecture and training procedure.

5. **Build the physics-aware extensions** (LDE, thermal, EM/IR, shielding) on top of the integrated baseline, guided by the equations in document 06.

### 5.A Analog Considerations

**When starting from ALIGN or MAGICAL, the first analog-specific modifications should be:**

1. **Add matching tier annotations** to the constraint data model. Neither ALIGN nor MAGICAL distinguishes between minimal/moderate/exceptional matching -- they treat all matched pairs identically. This is the most impactful single improvement for analog quality.

2. **Add the analog cost terms to the routing objective.** The current ALIGN/MAGICAL routers optimize wirelength. Add cost terms for matched-gate keep-out, noise coupling, and parasitic budget compliance.

3. **Add the LDE equalization check.** Neither system verifies WPE equalization or LOD/SA/SB matching after placement. Add this as a post-placement verification step that feeds back to the placer.

4. **Add the density fill keep-out zones.** Neither system blocks dummy metal generation over matched transistor gates. This is a critical omission that causes systematic mismatch in production silicon.

5. **Add the 17-point final layout checklist** as an automated verification step. This catches the most common analog layout errors before tapeout [Hastings, Ch. 15, Section 15.5].

---

## Changelog — Phase 3 Fixes

| Finding ID | What Was Changed | Why |
|---|---|---|
| MISSING-09 | Added design rule robustness note to Section 3.2.A specifying that all design rules in layers.json should use robust (production) values, not nominal process capability values | FOLD Section 3.4 distinguishes robust from non-robust design rules; analog layouts targeting non-robust rules risk parametric yield loss |
