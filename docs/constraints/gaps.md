# Gaps and Additional Sources Needed

Admitted coverage gaps and areas where the four audited books (AOAL, FOLD, ALS, PNR_ANALOG) provide insufficient guidance.

---

## 1. Inductor Layout Constraints

AOAL mentions inductor keepout (ProximityRule, ch06/6.4) but no book provides comprehensive inductor constraint coverage.

**Missing coverage**: Q-factor-aware geometry, metal stack selection, substrate shield patterns, center-tap symmetry.

**Existing code**: `inductor.rs` generator exists but constraint coverage is thin.

**Recommended sources**:
- Niknejad, *Electromagnetics for High-Speed Analog and Digital Communication Circuits*
- Mohan et al., *Simple Accurate Expressions for Planar Spiral Inductances*

---

## 2. FinFET / GAA-Specific Analog Constraints

All 4 books are bulk-CMOS/BiCMOS oriented. FinFET analog has different matching physics.

**Missing coverage**: Quantized width (integer fin count), CESL stress, fin-edge roughness, self-heating in fins.

**Recommended sources**:
- Enz & Vittoz, *Charge-Based MOS Transistor Modeling* (2006 ed. + FinFET supplement)
- Recent ISSCC/CICC analog FinFET papers

---

## 3. Advanced Node STI Stress Models

FOLD and PNR_ANALOG mention STI stress but do not provide the full tensor stress model needed for sub-28nm nodes.

**Missing coverage**: Compressive/tensile stress variation by crystal orientation and active region geometry (full tensor model).

**Existing code**: `LdeBound` SA/SB fields handle LOD; `StressConstraint` has `max_centroid_distance_um`. Neither models the full stress tensor.

**Recommended sources**:
- Komoda et al., IEDM proceedings on STI stress modeling

---

## 4. Copper Interconnect Reliability

AOAL and FOLD cover aluminum-era EM primarily. Copper dual-damascene has different failure modes.

**Missing coverage**: Stress-induced voiding at via interfaces, Cu barrier integrity, Cu-specific EM models.

**Existing code**: `ParasiticBudget` tracks R/C; proposed `ElectromigrationConstraint` uses Black's law (material-agnostic). Cu-specific failure modes not modeled.

**Recommended sources**:
- IRPS proceedings on Cu reliability
- Lienig & Thiele, *Fundamentals of Electromigration-Aware Integrated Circuit Design*

---

## 5. Mixed-Signal Isolation at System Level

PNR_ANALOG covers block-level isolation. System-level mixed-signal isolation is not treated.

**Missing coverage**: Substrate noise propagation across mm-scale die, triple-well strategies, deep trench isolation for RF.

**Existing code**: `IsolationConstraint` models block-level spacing. No system-level substrate noise propagation model.

**Recommended sources**:
- Donnay & Gielen, *Substrate Noise Coupling in Mixed-Signal ASICs* (Springer)

---

## 6. Power Integrity / PDN Constraints

All books touch IR drop (FOLD ch7.3, ALS ch7.3) but none provides a comprehensive PDN constraint model.

**Missing coverage**: Decoupling cap placement, power grid mesh density, target impedance vs frequency.

**Existing code**: `ParasiticBudget.max_r` and proposed `max_ir_drop_mv`. No PDN-specific constraint.

**Recommended sources**:
- Swaminathan & Engin, *Power Integrity Modeling and Design for Semiconductors and Systems*

---

## 7. Thermal Simulation Integration

AOAL and PNR_ANALOG provide thermal gradient formulas but no book covers runtime thermal simulation integration.

**Missing coverage**: FEM mesh, boundary conditions, iterative placement-thermal convergence.

**Existing code**: `ThermalTag` and `ThermalGradientConstraint` model budgets but not the simulation interface. This is an engine architecture gap, not a constraint type gap.

---

## 8. Process Variation / Monte Carlo Constraint Derivation

ALS ch4.5 (SensitivityBound) touches sensitivity analysis but no book covers systematic Monte Carlo-driven constraint derivation.

**Missing coverage**: How to translate process corner simulations into constraint thresholds automatically.

**Existing code**: Proposed `SensitivityBound` type (gap analysis 1.13). No Monte Carlo integration.

---

## 9. Cross-Audit Convergence Summary

Types found independently in 3+ of the 4 book audits, confirming high importance:

| Type/Enrichment | AOAL | FOLD | ALS | PNR_ANALOG |
|----------------|------|------|-----|------------|
| ElectromigrationConstraint | #10 | #29 | #19 | #20/#74 |
| GuardRingType expansion | #50 | #14 | -- | #25-26 |
| StressConstraint enrichment | #61 | #7/#9/#33 | -- | #11-12 |
| MatchingSpec enrichment | #54 | #11 | #34 | #3/#39 |
| CrosstalkExclusion + ShieldType | #43 | #22 | #13 | #19/#55 |
| ParasiticBudget enrichment | #58 | #20/#21 | #14/#24 | #16-17 |
| NetClass expansion | #56 | #23 | #16 | #52-53 |
| AntennaConstraint enrichment | #41 | #27-28 | -- | #20/#24 |
| IsolationConstraint enrichment | #51 | #15/#17 | -- | #23/#42/#51 |
| ThermalGradientConstraint enrichment | #62 | #8/#32 | -- | #21/#50 |

---

## 10. Documentation-Only Topics

Background knowledge extracted from audits. No code action needed; included for reference.

| Topic | Books | Summary |
|-------|-------|---------|
| Fabrication physics | FOLD ch1-2 | Physical origins of design rules; rules come from PDK |
| Circuit/layout data formats | FOLD ch3 | Infrastructure context upstream of constraint extraction |
| Design flow models | FOLD ch4 | Conceptual framework for constraint taxonomy |
| Netlist generation | FOLD ch5.1-5.2 | Upstream of constraint extraction |
| Verification (DRC/LVS/PEX) | FOLD ch5.4-5.5 | Downstream signoff; `ConstraintStage::Signoff` |
| SA cost functions | ALS ch1.1.3, 2.5 | Engine algorithm; `Contractable.priority()` |
| Constraint graph LP solver | ALS ch5.5 | Compaction engine algorithm |
| Template-based routing | ALS ch4.5.6 | Engine/methodology for template flows |
| Constraint Engineering System | ALS ch7.5 | CLP middleware architecture |
| ML models for analog PnR | PNR_ANALOG ch07 | GNN symmetry, VAE routing, GNN predictor |
| 29-concern layout checklist | PNR_ANALOG ch08 | Every concern maps to existing types |
| Analog vs digital complexity | ALS ch7.2.2 | Motivational context |
| FPAA routing bounds | ALS ch4.6.2 | Not applicable to full-custom flow |
| Integrated placement+routing | ALS ch4.5.4 | Engine architecture concern |
| Constraint transformation | ALS ch7.4.3 | Constraint-derivation engine concern |
| Constraint sensitivity analysis | ALS ch7.4.4 | Design-decision priority algorithm |
| Constraint generation from DRC | ALS ch5.3.3 | Methodology for retargeting flows |
| Pelgrom model derivation | PNR_ANALOG 00/1.1, ALS 2.2.2 | Formula for existing `max_dvth_mv` fields |
| Analog Hierarchy of Needs | PNR_ANALOG 00/5 | matching > isolation > parasitics > reliability > area |
