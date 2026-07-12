# Cross-Stage Review of Enriched PNR Analog Specifications

Version: 1.0 -- June 2026
Reviewer scope: All enriched specs (00-10), original checklist (08), and 11 unassigned textbook files.

---

## 1. Handoff Gaps

### GAP-01: S0 does not export per-device DC bias current (Id) for S1/S3
- **Severity:** HIGH
- **Files:** `01_S0_CELL_GENERATION.md`, `03_S2_S3_S4_INTERFACES.md`
- **Detail:** S0's CellRecord metadata (Section 2.A) includes `thermal_power_mW` but the interface spec's "Data Gaps" table in `03_S2_S3_S4_INTERFACES.md` explicitly flags "Per-device DC bias current (Id)" as missing. S3 needs per-net current for EM wire sizing, and S1 needs it for parasitic budget computation. The CellRecord has no `bias_current_mA` field.
- **Fix:** Add `bias_current_mA: float` to the CellRecord in `01_S0_CELL_GENERATION.md` Section 2.A metadata, and add a `BiasCurrentTag` to the S1 ConstraintRecord in `03_S2_S3_S4_INTERFACES.md`.

### GAP-02: S0 does not export per-device overdrive voltage (Vgs-Vth) for S1
- **Severity:** HIGH
- **Files:** `01_S0_CELL_GENERATION.md`, `03_S2_S3_S4_INTERFACES.md`
- **Detail:** The interface spec's "Data Gaps" table flags "Per-device overdrive voltage (Vgs-Vth)" as missing. S1 needs this for Pelgrom current mismatch calculation `sigma^2(dId)/Id^2 = 4*sigma^2(dVth)/(Vgs-Vth)^2 + sigma^2(d_beta/beta)` and for the subthreshold guard (Rule 4: avoid Veff < 100 mV). The S0->S1 interface in Section 11.A of `01_S0_CELL_GENERATION.md` mentions that S1 should pass Vgs-Vth to S0 but does not specify that S0 must store and re-export it.
- **Fix:** Add `overdrive_voltage_mV: float` to the CellRecord metadata. Ensure S1 populates it from .op data and S0 stores it after the S0-S1 iteration converges.

### GAP-03: S1 net classification does not flow to S3's A* cost function as structured input
- **Severity:** MEDIUM
- **Files:** `02_S1_CONSTRAINT_EXTRACTION.md`, `05_S3_ROUTING_DETAILED.md`
- **Detail:** S1 defines 12 net classes (Section 5.A.1: differential, clock, power, ground, substrate, guard_ring, matched_pair, high_impedance, compensation, ESD_bus, sensitive, general). S3's routing spec (Section 6.3.A) defines analog cost terms with `coupling_weight[j]` that depends on net classification pairs. However, the RoutedLayout data structure in `03_S2_S3_S4_INTERFACES.md` does not carry net class forward from S1 to S3 -- the `Route` record has `net_name` but no `net_class`. S3 must look up each net's class from the S1 constraint file during routing.
- **Fix:** Add `net_class: enum` to the `Route` record in `03_S2_S3_S4_INTERFACES.md` to ensure net classification propagates through the pipeline without requiring S3 to re-parse S1's constraint file.

### GAP-04: S0's hydrogenation zone rectangle not consumed by S4 density fill engine
- **Severity:** HIGH
- **Files:** `01_S0_CELL_GENERATION.md`, `03_S2_S3_S4_INTERFACES.md`
- **Detail:** S0 produces a `hydrogenation_safe_zone` rectangle per matched cell (Section 2.A). S4's density fill engine needs this as a keep-out zone for dummy metal generation. The interface spec's "Data Gaps" table identifies this as "Keep-out pseudolayer for density fill" and flags it as missing. The S4 sign-off spec in `03_S2_S3_S4_INTERFACES.md` mentions pseudolayers but the data flow from S0->S4 is not explicit -- S4 must derive keep-out zones from S0 metadata that has passed through S2 and S3 without guarantee of preservation.
- **Fix:** Add a `metal_keepout_zones: list<Rectangle>` field to the PlacedLayout and RoutedLayout data structures in `03_S2_S3_S4_INTERFACES.md`, populated from S0's hydrogenation zone metadata during placement. S4's fill engine consumes this directly.

### GAP-05: S1's crosstalk exclusion matrix not consumed by S3's obstacle union
- **Severity:** MEDIUM
- **Files:** `02_S1_CONSTRAINT_EXTRACTION.md`, `05_S3_ROUTING_DETAILED.md`
- **Detail:** S1 emits a `crosstalk_exclusions` list (Section 5.A.2) with `(net_a, net_b, min_spacing)` tuples. S3's routing spec (Section 6.2.A) mentions that the obstacle union must include "shield tracks already placed" and "guard ring metal" but does not explicitly mention consuming S1's crosstalk exclusion matrix as routing constraints. The S3 A* cost function includes `Cost_coupling` (Section 6.3.A) but this is a soft cost, not a hard spacing enforcement derived from S1's exclusion matrix.
- **Fix:** In `05_S3_ROUTING_DETAILED.md` Section 6.2.B, add: "The A* router MUST enforce S1's crosstalk exclusion matrix as hard minimum-spacing constraints between the specified net pairs, in addition to the soft Cost_coupling penalty."

### GAP-06: Package/die stress model input not defined in any stage
- **Severity:** MEDIUM
- **Files:** `00_MASTER_ARCHITECTURE.md`, `04_S2_PLACEMENT_DETAILED.md`
- **Detail:** The master architecture (Section 4.A) identifies "Package/die information" as a missing input, noting that die position in package affects mechanical stress distribution. S2's detailed spec (Section 2.6.A) discusses mechanical stress placement and references die center, but no stage defines the data structure for package CTE, die position, or die size. The `03_S2_S3_S4_INTERFACES.md` "Data Gaps" table confirms this: "Package/die stress model" is listed as missing.
- **Fix:** Add a `PackageInfo` input structure to `00_MASTER_ARCHITECTURE.md` Section 4: `{die_width_um, die_height_um, package_type, mold_cte_ppm_per_C, die_attach_type}`. S2 consumes this for mechanical stress placement constraints.

### GAP-07: S2's placed layout does not export the thermal map to S4 for sign-off validation
- **Severity:** LOW
- **Files:** `04_S2_PLACEMENT_DETAILED.md`, `03_S2_S3_S4_INTERFACES.md`
- **Detail:** S2 computes a thermal map during GP (Section 2.6) using the Green's function model. The PlacedLayout analog enrichment in `03_S2_S3_S4_INTERFACES.md` includes `thermal_map: 2D array`. However, S4's sign-off spec does not specify that it validates thermal gradient across matched pairs against the thermal budget from S1. The thermal map is produced but not checked at sign-off.
- **Fix:** Add a thermal gradient validation step to S4's verification sequence in `03_S2_S3_S4_INTERFACES.md` Section 1.A: "For each matched pair, verify |T(device_i) - T(device_j)| < dT_tolerance from S1's LDE bounds."

---

## 2. Constraint Coverage vs. Checklist (08_OPTIMAL_LAYOUT_CHECKLIST.md)

### COV-01: Checklist item 4.3 -- Double/multi-patterning rules: no concrete mechanism
- **Severity:** LOW
- **Files:** `05_S3_ROUTING_DETAILED.md`
- **Detail:** Checklist item 4.3 identifies double/multi-patterning rules as a manufacturability concern at S3. No enriched spec addresses multi-patterning coloring constraints or coloring-aware routing. The routing spec discusses DRC compliance generically but has no mechanism for decomposition-aware routing.
- **Fix:** Add a note in `05_S3_ROUTING_DETAILED.md` Section 11.A: "For advanced nodes requiring multi-patterning, the router must assign coloring to metal shapes and avoid coloring conflicts. This is deferred to Phase 4 (advanced node)."

### COV-02: Checklist item 4.4 -- Well/implant density rules: no concrete mechanism
- **Severity:** LOW
- **Files:** `04_S2_PLACEMENT_DETAILED.md`
- **Detail:** Checklist item 4.4 identifies well/implant density rules as a manufacturability concern at S2. No enriched spec addresses well density balancing during placement. The placement spec discusses metal density (via S4 fill) but not well/implant density.
- **Fix:** Add a note in `04_S2_PLACEMENT_DETAILED.md` Section 4.A: "The ILP should include a well density balance constraint: per-tile N-well density within [PDK.min_well_density, PDK.max_well_density]. Imbalanced well density causes implant dose non-uniformity."

### COV-03: Checklist item 3.5 -- ESD protection: partially covered
- **Severity:** MEDIUM
- **Files:** `01_S0_CELL_GENERATION.md`, `02_S1_CONSTRAINT_EXTRACTION.md`
- **Detail:** Checklist item 3.5 identifies ESD protection as a reliability concern at S0. The master architecture (Section S0.A) discusses ESD protection cell generation including GGNMOS ballasting and CDM proximity. S1's constraint file includes `esd_requirements`. However, no spec defines the ESD ground ring resistance budget (max 2 ohms between any two pads, per FOLD 7.4) or the detailed placement rule that CDM secondary protection must be placed near the gate oxides it protects, not at the bondpad.
- **Fix:** In `02_S1_CONSTRAINT_EXTRACTION.md` Section 5.A.1, expand the ESD_bus net class entry to include: "max_ring_resistance_ohm: 2.0, CDM_clamp_max_distance_from_gate_um: 50." In `01_S0_CELL_GENERATION.md`, add CDM proximity as a constraint for secondary ESD cell placement.

### COV-04: Checklist item 5.6 -- Layout aesthetics/regularity: no concrete mechanism
- **Severity:** LOW
- **Files:** `05_S3_ROUTING_DETAILED.md`
- **Detail:** Checklist item 5.6 identifies layout aesthetics and regularity as an optimization concern. The VAE routing guide (Section 2.A) implicitly learns regularity from expert layouts, but no enriched spec defines a quantitative regularity metric or penalizes irregular routing patterns.
- **Fix:** No action required. The VAE guide captures this implicitly. Add a comment in `05_S3_ROUTING_DETAILED.md` Section 2.A: "Layout regularity is captured implicitly by the VAE model trained on expert layouts. No explicit regularity metric is needed."

---

## 3. Contradictions Between Enriched Specs

### CONTRA-01: Wirelength minimization vs. parasitic equalization jogs
- **Severity:** MEDIUM
- **Files:** `04_S2_PLACEMENT_DETAILED.md` (Section 2.1.A), `05_S3_ROUTING_DETAILED.md` (Section 8.A)
- **Detail:** The placement spec's wirelength term W(v) drives devices closer together to minimize HPWL. The routing spec's parasitic matching verification (Section 8.A) prescribes adding serpentine jogs or dead-end stubs to equalize parasitic capacitance between matched nets, which intentionally increases wirelength. These two objectives trade off: the placement spec does not acknowledge that minimizing wirelength for matched nets may be counterproductive if it creates asymmetric pin access that requires more equalization jogs later.
- **Fix:** In `04_S2_PLACEMENT_DETAILED.md` Section 2.1.A, add: "For matched net pairs, the wirelength objective should target symmetric pin-to-pin distances rather than minimum total wirelength. Asymmetric placement that appears HPWL-optimal may require costly parasitic equalization jogs during routing."

### CONTRA-02: "Use highest metal for high-Z nodes" vs. "use lowest-C metal for matched nets"
- **Severity:** LOW
- **Files:** `05_S3_ROUTING_DETAILED.md` (Section 12.A Rule 1 vs. Rule 3)
- **Detail:** Rule 1 says high-impedance nodes should be routed on the highest available metal layer (lowest C to substrate). Rule 3 says matched nets must use the same metal layers for corresponding segments. If one net of a matched pair is high-impedance (e.g., a cascode drain of a diff pair output), Rule 1 pushes it to top metal while Rule 3 requires both matched nets on the same layer. These rules can conflict when only one net of a matched pair is high-impedance.
- **Resolution:** This is not a true contradiction -- Rule 3 takes precedence for matched nets. Both nets must go on the same layer, and that layer should be the highest feasible. The specs already state matching is priority 1. However, the tradeoff should be made explicit.
- **Fix:** In `05_S3_ROUTING_DETAILED.md` Section 12.A, add after Rule 3: "When Rule 1 and Rule 3 conflict (one net of a matched pair is high-Z), Rule 3 prevails: both nets are routed on the same layer, chosen as the highest layer that satisfies both nets' routing constraints."

### CONTRA-03: S0 validation checklist says "DRC clean on cell in isolation" vs. S2 guard ring space reservation
- **Severity:** LOW
- **Files:** `01_S0_CELL_GENERATION.md` (Section 12), `04_S2_PLACEMENT_DETAILED.md` (Section 4.1.A)
- **Detail:** S0's validation checklist (item 1) requires each cell to be DRC-clean in isolation. However, S2's ILP formulation adds guard ring space reservation as an area overhead, implying that the guard ring geometry may extend beyond the cell bounding box after placement. Cells that are DRC-clean in isolation may develop DRC errors when guard rings from adjacent cells interact (e.g., minimum spacing violations between neighboring guard rings).
- **Fix:** In `01_S0_CELL_GENERATION.md` Section 12, clarify: "DRC clean applies to the cell including its guard ring inflation. S2 must verify inter-cell DRC after placement, as guard ring interactions between adjacent cells are not covered by per-cell DRC."

---

## 4. Missing Analog Knowledge from Unassigned Textbook Files

### MISSING-01: Conductivity modulation and voltage-dependent resistor matching (ch08-mismatch-causes)
- **Severity:** HIGH
- **Files to update:** `00_ANALOG_PRINCIPLES.md`, `01_S0_CELL_GENERATION.md`
- **Detail:** Hastings Ch. 8 (Sections 8.2.9) describes conductivity modulation as a mismatch mechanism for high-sheet resistors. Electric fields from leads routed over resistors or from body/tank bias differences cause up to 0.1%/V systematic mismatch. The enriched specs discuss hydrogenation blocking of metal over resistors but do not address conductivity modulation. Specifically:
  - Body/tank modulation: different operating voltages on matched resistor segments require separate tanks, each biased to the positive end of its segment.
  - Field plate (Faraday shield) design: needed for Rsh > 1 kOhm/sq; split field plates for HSR resistors requiring < 1% matching to combat dielectric absorption.
  - Charge spreading: mobile ion contamination causing long-term drift under bias.
- **Fix:** In `00_ANALOG_PRINCIPLES.md` Section 2, add a new subsection "2.5 Conductivity Modulation and Dielectric Absorption" covering: body/tank bias matching for resistors, field plate requirements for Rsh > 1 kOhm/sq, and split field plates for exceptional HSR matching. In `01_S0_CELL_GENERATION.md` Section 7.A.3, add a rule: "For high-sheet resistors (Rsh > 200 Ohm/sq), ensure identical body/tank bias on all matched segments. For Rsh > 1 kOhm/sq, generate an electrostatic field plate; for exceptional matching, use split field plates."

### MISSING-02: NBL shadow effect on resistor matching (ch08-mismatch-causes)
- **Severity:** MEDIUM
- **Files to update:** `01_S0_CELL_GENERATION.md`
- **Detail:** Hastings Ch. 8 (Section 8.2.5) describes the N-buried layer shadow effect: during epitaxial growth, surface discontinuities from patterned NBL shift laterally (pattern shift on (111) wafers: 50-150% of epi thickness; pattern distortion on (100)). If the NBL shadow intersects a matched resistor, it alters its value. HSR resistors are especially vulnerable. STI/CMP processes are immune. The enriched specs do not mention NBL shadow.
- **Fix:** In `01_S0_CELL_GENERATION.md` Section 7.A, add: "For BiCMOS processes without STI (using LOCOS), the cell generator must verify that the NBL shadow does not intersect matched resistor segments. Increase NBL overlap on the pattern-shift side by at least the epi thickness plus alignment tolerance."

### MISSING-03: Surface effects -- HCI, NBTI, parasitic channels (ch05-surface-effects)
- **Severity:** MEDIUM
- **Files to update:** `02_S1_CONSTRAINT_EXTRACTION.md`
- **Detail:** Hastings Ch. 5.3 describes three surface effects that cause parametric drift and impact matching over the product lifetime:
  - Hot-Carrier Injection (HCI): maximized when Vgs ~ 0.4*Vds; increases channel length by 0.5-1.0 um provides extra voltage margin; cascodes equalize Vds across matched input pairs.
  - NBTI: PMOS Vth drifts under negative gate bias; matched PMOS at different Vgs values develop mismatch over time. Oxynitride dielectrics (common in advanced nodes) substantially worsen NBTI.
  - Parasitic channels and charge spreading: field plates needed for lateral PNP base surfaces; channel stops for high-voltage P-type regions.
  None of these appear in the enriched S1 constraint extraction spec as constraint generation triggers.
- **Fix:** In `02_S1_CONSTRAINT_EXTRACTION.md` Section 2.A, add triggers: (1) "If matched PMOS pair has differential Vgs bias > 100 mV, flag for NBTI-induced mismatch over product lifetime." (2) "If NMOS operates with Vgs ~ 0.4*Vds and Vds > 2V, flag for HCI susceptibility; recommend cascode or increased L." (3) "For every lateral PNP transistor, generate a field plate constraint covering the base between emitter and collector."

### MISSING-04: Merged device rules and debiasing analysis (ch14-merged-devices)
- **Severity:** MEDIUM
- **Files to update:** `02_S1_CONSTRAINT_EXTRACTION.md`, `01_S0_CELL_GENERATION.md`
- **Detail:** Hastings Ch. 14.1 describes the three failure mechanisms of merged devices (minority carrier injection, debiasing, capacitive coupling) and provides specific rules:
  - NPN merged with base resistor can latch up if tank resistance exceeds ~6 kOhm (debiasing > 600 mV at 150C). Even a minimum deep-N+ plug reduces tank R to ~100 Ohm, preventing latchup.
  - Devices drawing significant current through shared wells/tanks create IR drops that can forward-bias junctions.
  - Noisy and sensitive devices must never share a tank/well due to capacitive coupling.
  The enriched specs discuss guard rings and latchup prevention but do not address the debiasing analysis for merged devices (i.e., computing whether current through shared well resistance can forward-bias a junction).
- **Fix:** In `02_S1_CONSTRAINT_EXTRACTION.md` Section 2.A, add: "For devices sharing a well/tank, compute the IR drop across the shared well resistance: V_debias = I_device * R_well. If V_debias > 500 mV at worst-case temperature, flag for separate well isolation or deep-N+ plug insertion." In `01_S0_CELL_GENERATION.md` Section 6.A, add: "For BiCMOS processes, every NPN transistor in a shared tank must have at least a minimum deep-N+ plug to keep tank resistance below 100 Ohm."

### MISSING-05: Integrated inductors and RF routing (ch07-inductance)
- **Severity:** LOW
- **Files to update:** `05_S3_ROUTING_DETAILED.md`
- **Detail:** Hastings Ch. 7.2 provides 11 guidelines for integrating planar spiral inductors, including: keep unconnected metal at least half the inductor width away, do not place circuitry in the empty center, remove dummy metal and poly fill near inductors, do not place junctions beneath inductors. The routing spec does not address inductor keep-out zones or the special routing constraints for RF circuits with integrated inductors.
- **Fix:** In `05_S3_ROUTING_DETAILED.md` Section 12.A, add: "For circuits containing integrated inductors (RF applications), the router must enforce an inductor keep-out zone of at least half the inductor outer diameter. No routing metal, dummy fill, or active circuitry may be placed within this zone. Dummy metal and poly fill must be blocked within this zone. This is deferred to Phase 4 (advanced node / RF)."

### MISSING-06: CMOS construction details -- pocket implant blocking and directional implants (ch12-constructing-cmos)
- **Severity:** HIGH
- **Files to update:** `01_S0_CELL_GENERATION.md`
- **Detail:** Hastings Ch. 12.2.7 describes pocket (halo) implants and their devastating impact on analog matching: (1) output resistance scales as sqrt(L) instead of L, breaking the linear ro vs. L scaling analog designers rely on; (2) random Vth variation does not follow Pelgrom's 1/sqrt(WL) law. Two mitigations are described: (a) block pocket implants from long-channel analog transistors using extra masks, or (b) use directional tilted implants shot only left-right so that horizontally-oriented digital transistors receive pockets while vertically-oriented analog transistors do not. The second approach imposes a layout constraint: blocks can be reflected or rotated 180 degrees but NEVER 90 degrees. The S0 spec mentions the pocket-implant modified Pelgrom model (Section 3.A.3) but does not address pocket implant blocking or the 90-degree rotation prohibition for directional implant processes.
- **Fix:** In `01_S0_CELL_GENERATION.md` Section 3.A.3, add: "If the PDK uses directional tilted pocket implants, analog transistors must be oriented orthogonally to the implant direction (typically vertical channel for horizontal implants). In this case, matched analog cells MUST NOT be rotated by 90 degrees -- only 0 and 180 degree rotations are permitted. The cell generator must query PDK.pocket_implant_direction and enforce this constraint." Also add to `01_S0_CELL_GENERATION.md` Section 9.A: "In directional-implant processes, the rotation prohibition is absolute: 90-degree rotation swaps digital and analog transistor orientations and causes catastrophic matching degradation."

### MISSING-07: Single-level interconnection mock layout methodology (ch14-interconnection)
- **Severity:** LOW
- **Files to update:** None (informational only)
- **Detail:** Hastings Ch. 14.3 describes mock layouts and tunnel types for single-level metal processes. While largely historical, the mock layout methodology (rough sketches to minimize crossings) and the annotated schematic convention (red/yellow/green for power/noisy/sensitive leads) remain relevant for local routing-constrained regions (e.g., under capacitors that occupy upper metals). The enriched specs do not address this scenario.
- **Fix:** No action needed for current scope. The mock layout concept is implicitly covered by the VAE routing guide. The annotated schematic convention maps to S1's net classification system.

### MISSING-08: Overvoltage protection -- ESD ground ring and multi-domain considerations (FOLD 7.4)
- **Severity:** MEDIUM
- **Files to update:** `02_S1_CONSTRAINT_EXTRACTION.md`, `05_S3_ROUTING_DETAILED.md`
- **Detail:** FOLD Section 7.4.1 provides specific ESD layout rules not in the enriched specs: (1) total resistance between any two chip points should not exceed ~2 ohms; (2) supply lines should run at the chip periphery for direct access from all bond pads; (3) for multiple power domains, antiparallel diode coupling between ground lines enables ESD current sharing; (4) chamfer outer corners of Metal1 interconnects near ESD devices to minimize local field peaks; (5) the power supply concept and ESD concept must be defined at the beginning of the layout phase as they impact the entire floorplan.
- **Fix:** In `02_S1_CONSTRAINT_EXTRACTION.md` Section 5.A.1, expand the ESD_bus net class: "max_total_resistance_ohm: 2.0, topology: peripheral_ring, chamfer_corners: true." In `05_S3_ROUTING_DETAILED.md`, add to Section 6.6.A: "ESD bus metal corners must be chamfered (45-degree bevel) to prevent current crowding during ESD events."

### MISSING-09: Design rule robustness and overlay error budgets (FOLD 3.4-3.5)
- **Severity:** LOW
- **Files to update:** `10_BENCHMARK_VALIDATION_PDK.md`
- **Detail:** FOLD Section 3.4 distinguishes between robust and non-robust design rules. Robust rules incorporate safety margins for manufacturing tolerances; non-robust rules match nominal process capability only. The enriched PDK abstraction layer in `10_BENCHMARK_VALIDATION_PDK.md` defines design rules as single values (min width, spacing) without indicating whether they are robust or non-robust. For analog, using non-robust rules is risky because parametric yield depends on these margins.
- **Fix:** In `10_BENCHMARK_VALIDATION_PDK.md` Section 3.2.A, add a note: "All design rules in layers.json should be the robust (production) values, not nominal process capability values. If the PDK provides both, use the robust set. Analog layouts must never target non-robust rules."

### MISSING-10: Layout retargeting and compaction-based process migration (ALS 5.1, 5.5, 5.7)
- **Severity:** LOW
- **Files to update:** `00_MASTER_ARCHITECTURE.md`
- **Detail:** ALS Chapter 5 describes compaction-based layout retargeting for process migration, including the multivariable constraint-graph simplex method that handles symmetry constraints natively. This is relevant to the PNR engine's technology portability story (Section 8 of the master architecture): if a layout exists for one process, retargeting can produce a layout for another process faster than running the full PNR pipeline from scratch. The enriched specs address technology portability through PDK abstraction but do not mention layout retargeting as a migration strategy.
- **Fix:** In `00_MASTER_ARCHITECTURE.md` Section 8, add: "For process migration of existing layouts, compaction-based retargeting (ALS Ch. 5) offers an alternative to full re-synthesis: the source layout's placement and routing are preserved while adapting to new design rules and device dimensions. The multivariable constraint-graph simplex method supports symmetry and hierarchy constraints natively. This approach is complementary to the full PNR pipeline and may be faster for incremental process migrations."

### MISSING-11: Parasitic channel and charge spreading prevention (ch05-surface-effects, ch12-constructing-cmos)
- **Severity:** MEDIUM
- **Files to update:** `01_S0_CELL_GENERATION.md`
- **Detail:** Hastings Ch. 5.3.5 and Ch. 12.2.3 describe parasitic channel formation when a conductor crosses thick field oxide above improperly doped silicon. Six conditions must exist: lightly doped backgate, source, drain, gate conductor, sufficient Vgs, and nonzero Vds. Preventive measures include channel stop implants and field plates. For CMOS, poly leads of high-voltage transistors must be retracted inside the N-well to avoid parasitic PMOS channels beneath poly. Every lateral PNP transistor needs a field plate on the base surface. The enriched specs discuss latchup prevention via guard rings but do not address parasitic channel prevention via field plates or poly retraction.
- **Fix:** In `01_S0_CELL_GENERATION.md` Section 6.A, add: "For high-voltage analog (Vds > 75% of thick-field threshold), the cell generator must insert channel stops (minimum-width NSD strips) or field plates (poly or lower metal connected to highest supply) to prevent parasitic channel formation. For lateral PNP transistors, always generate a field plate covering the base surface between emitter and collector, connected to the emitter terminal. For CMOS high-voltage PMOS, ensure poly gate leads are retracted inside the N-well by at least the N-well overlap of PSD plus 1-2 um."

---

## 5. Summary Statistics

| Category | Count | HIGH | MEDIUM | LOW |
|----------|-------|------|--------|-----|
| Handoff Gaps (GAP) | 7 | 2 | 3 | 2 |
| Constraint Coverage (COV) | 4 | 0 | 1 | 3 |
| Contradictions (CONTRA) | 3 | 0 | 1 | 2 |
| Missing Knowledge (MISSING) | 11 | 2 | 5 | 4 |
| **Total** | **25** | **4** | **10** | **11** |

---

## 6. Priority Fix Order

The following HIGH-severity findings should be addressed first:

1. **GAP-01** (bias current missing from S0 output) -- Blocks S3 wire sizing and S1 parasitic budgets.
2. **GAP-02** (overdrive voltage missing) -- Blocks accurate Pelgrom current mismatch calculation.
3. **GAP-04** (hydrogenation keep-out not flowing to S4) -- Allows density fill to silently destroy matching in production silicon.
4. **MISSING-01** (conductivity modulation) -- Unaddressed mismatch mechanism for high-sheet resistors.
5. **MISSING-06** (pocket implant blocking / 90-degree rotation prohibition) -- Can cause catastrophic matching failure in directional-implant processes.

After these, address MEDIUM findings in order: MISSING-03 (surface effects), MISSING-04 (merged device debiasing), MISSING-08 (ESD ground ring), MISSING-11 (parasitic channels), GAP-03 (net class propagation), GAP-05 (crosstalk exclusion enforcement), GAP-06 (package info), COV-03 (ESD detail), CONTRA-01 (wirelength vs. jogs).
