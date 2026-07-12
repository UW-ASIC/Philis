---
title: "The Art of Analog Layout - Master Index"
book: "The Art of Analog Layout, 3rd Edition"
author: "Alan Hastings"
tags:
  - analog-layout
  - AOAL
  - index
  - semiconductor
  - IC-design
---

# The Art of Analog Layout (3rd Edition)

**Author:** Alan Hastings
**Scope:** Comprehensive reference covering device physics, fabrication, layout techniques, passive/active device design, matching, reliability, and die assembly for analog integrated circuits.

---

## Chapter 1: Device Physics

Foundations of semiconductor behavior, from carrier dynamics through the operation of PN junctions, bipolar transistors, MOS transistors, and JFETs.

- [[ch01-semiconductors]] — 1.1 Semiconductors: Generation/recombination, extrinsic semiconductors, diffusion/drift
- [[ch01-pn-junctions]] — 1.2 PN Junctions: Depletion regions, PN/Zener/Schottky diodes, ohmic contacts
- [[ch01-bipolar-transistors]] — 1.3 Bipolar Transistors: Beta, I-V characteristics
- [[ch01-mos-transistors]] — 1.4 MOS Transistors: Threshold voltage, I-V characteristics
- [[ch01-jfet-transistors]] — 1.5 JFET Transistors: JFET operation and characteristics

---

## Chapter 2: Semiconductor Fabrication

The complete IC manufacturing flow from raw silicon through wafer processing, patterning, doping, metallization, and final assembly.

- [[ch02-silicon-manufacture]] — 2.1 Silicon Manufacture: Crystal growth, wafer manufacture, crystal structure
- [[ch02-patterning]] — 2.2 Patterning: Photoresists, exposure, development
- [[ch02-oxide-growth]] — 2.3 Oxide Growth and Removal: Oxide growth/deposition, removal, LOCOS
- [[ch02-diffusion-implantation]] — 2.4 Diffusion and Ion Implantation: Diffusion, ion implantation effects
- [[ch02-silicon-deposition]] — 2.5 Silicon Deposition and Etching: Epitaxy, poly deposition, silicon etching
- [[ch02-isolation]] — 2.6 Isolation: Junction isolation, dielectric isolation, wafer bonding
- [[ch02-interconnection]] — 2.7 Interconnection: Aluminum, refractory metals, silicidation, tungsten plugs, copper
- [[ch02-assembly]] — 2.8 Assembly: Mount/bond, packaging

---

## Chapter 3: Layout

The tools and rules that govern physical IC layout, including editors, design rule checking, and mask generation.

- [[ch03-layout-editors]] — 3.1 Layout Editors: Coordinates, grid, shapes, hierarchy, interchange formats (CIF/GDSII/OASIS)
- [[ch03-design-rules]] — 3.2 Design Rules: Geometric operations, rule checks, design rule construction, scalable rules
- [[ch03-pattern-generation]] — 3.3 Pattern Generation: Optical pattern generation, photolithography advances, MEBES, OPC

---

## Chapter 4: Representative Processes

Detailed fabrication sequences and available devices for standard bipolar, poly-gate CMOS, and analog BiCMOS process technologies.

- [[ch04-standard-bipolar]] — 4.1 Standard Bipolar: Fabrication sequence, available devices, process extensions
- [[ch04-poly-gate-cmos]] — 4.2 Poly-Gate CMOS: Fabrication sequence, available devices, LDD, drain-extended transistors
- [[ch04-analog-bicmos]] — 4.3 Analog BiCMOS: BiCMOS fabrication, all device types, 3.3V CMOS, dielectric isolation

---

## Chapter 5: Failure Mechanisms

Reliability hazards in analog ICs, covering electrical overstress, contamination, surface degradation, and parasitic minority-carrier effects including latchup.

- [[ch05-electrical-overstress]] — 5.1 Electrical Overstress: Self-heating, filamentation, electromigration, TDDB, ESD, antenna effect
- [[ch05-contamination]] — 5.2 Contamination: Dry corrosion, mobile ion contamination
- [[ch05-surface-effects]] — 5.3 Surface Effects: Hot-carrier injection, Zener walkout, NBTI, parasitic channels, substrate influence
- [[ch05-minority-carrier-injection]] — 5.4 Minority Carrier Injection: Minority carrier injection, latchup, debiasing, guard rings

---

## Chapter 6: Resistors

Design, layout, and variability analysis of integrated resistors, including all available resistor types and techniques for post-fabrication value adjustment.

- [[ch06-resistivity-sheet-resistance]] — 6.1-6.2 Resistivity, Sheet Resistance, and Layout: Resistivity, sheet resistance, resistor layout fundamentals
- [[ch06-resistor-variability]] — 6.3 Resistor Variability: Process, temperature, nonlinearity, contact resistance, hydrogenation
- [[ch06-resistor-parasitics]] — 6.4-6.5 Resistor Parasitics and Comparison: Parasitics, all resistor types compared (base, emitter, poly, NSD, PSD, thin-film...)
- [[ch06-adjusting-resistors]] — 6.6 Adjusting Resistor Values: Tweaking (sliding contacts, trombone), trimming (fuses, Zener zaps, laser trim)

---

## Chapter 7: Capacitors and Inductors

Integrated capacitor and inductor design, covering construction, parasitics, variability, and practical guidelines for on-chip passive components.

- [[ch07-capacitance]] — 7.1 Capacitance: Fringing, derating, junction cap, variability, parasitics, all capacitor types compared
- [[ch07-inductance]] — 7.2 Inductance: Inductor parasitics, construction, integration guidelines

---

## Chapter 8: Matching of Resistors and Capacitors

Sources of mismatch in passive components and systematic layout rules to minimize random and systematic variation in resistor and capacitor pairs.

- [[ch08-mismatch-causes]] — 8.1-8.2 Mismatch and Causes: Random variation, process biases, proximity, interconnect, NBL shadow, hydrogenation, temperature, stress, E-fields
- [[ch08-matching-rules]] — 8.3 Rules for Device Matching: Rules for resistor matching, rules for capacitor matching

---

## Chapter 9: Bipolar Transistors

Bipolar transistor operation, layout, and construction across standard bipolar, CMOS, and BiCMOS processes, including advanced structures like SiGe HBTs.

- [[ch09-bjt-operation]] — 9.1 Bipolar Transistor Operation: Beta variation, avalanche breakdown, saturation in NPN and lateral PNP
- [[ch09-standard-bipolar-transistors]] — 9.2 Standard Bipolar Small-Signal Transistors: Vertical NPN, substrate PNP, lateral PNP, high-voltage, super-beta
- [[ch09-cmos-bicmos-transistors]] — 9.3 CMOS and BiCMOS Small-Signal BJTs: CMOS PNP, shallow-well, BiCMOS NPN/PNP, fast BJTs, SiGe HBTs

---

## Chapter 10: Applications of Bipolar Transistors

Power BJT design and layout for high-current applications, plus systematic matching techniques for precision analog circuits.

- [[ch10-power-bjts]] — 10.1 Power Bipolar Transistors: Failure mechanisms, NPN/PNP power layouts, advanced power BJTs, saturation detection
- [[ch10-matching-bjts]] — 10.2-10.3 Matching Bipolar Transistors: Random variations, emitter degen, thermal/stress/NBL effects, matching rules

---

## Chapter 11: Diodes

Diode construction and layout in bipolar and CMOS processes, including Zener, Schottky, and power diodes, with matching considerations.

- [[ch11-diodes-standard-bipolar]] — 11.1 Diodes in Standard Bipolar: Diode-connected transistors, Zener diodes, Schottky diodes, power diodes
- [[ch11-diodes-cmos-bicmos]] — 11.2-11.3 Diodes in CMOS/BiCMOS and Matching: CMOS/BiCMOS junction and Schottky diodes, matching rules

---

## Chapter 12: Field-Effect Transistors

MOS transistor operation and layout in depth, plus nonvolatile memory structures and JFET construction across multiple process variants.

- [[ch12-mos-operation]] — 12.1 MOS Transistor Operation: Transconductance, Vth, CLM, velocity sat, short/narrow channel, subthreshold, GIDL, breakdown
- [[ch12-constructing-cmos]] — 12.2 Constructing CMOS Transistors: Coding, wells, channel stops, Vth adjust, multiple gate ox, scaling, drain engineering, variants, backgate
- [[ch12-nvm]] — 12.3 Nonvolatile Memory: Floating-gate transistor, single-poly EPROM, single-poly EEPROM
- [[ch12-jfet]] — 12.4 The JFET Transistor: JFET modeling and construction (Epi-FET, N-well, double-diffused, ion-implanted)

---

## Chapter 13: Applications of MOS Transistors

Power MOS transistor design including DMOS, plus comprehensive MOS matching analysis and common-centroid layout techniques.

- [[ch13-power-mos]] — 13.1 Power MOS Transistors: Conduction/switching losses, SOA, CMOS power, high-voltage, DMOS
- [[ch13-matching-mos]] — 13.2-13.3 Matching MOS Transistors: Geometry effects, diffusion/etch, bias-dependent, hydrogenation, gradients, common-centroid, rules

---

## Chapter 14: Special Topics

Cross-cutting layout concerns: merged device interactions, minority-carrier guard rings, single-level routing strategies, and ESD protection design.

- [[ch14-merged-devices]] — 14.1 Merged Devices: Minority carrier injection, debiasing, coupling, problematic/successful/low-risk mergers
- [[ch14-guard-rings]] — 14.2 Minority-Carrier Guard Rings: Guard rings for standard bipolar, CMOS, BiCMOS (electron and hole)
- [[ch14-interconnection]] — 14.3 Single-Level Interconnection: Mock layouts, crossing leads, tunnels
- [[ch14-esd-protection]] — 14.4 ESD Protection: Primary/secondary ESD devices, die-level strategies, guidelines

---

## Chapter 15: Assembling the Die

Top-level die planning from area estimation through floorplanning, padring construction, routing, and a final layout review checklist.

- [[ch15-die-area-estimation]] — 15.1 Die Area Estimation: Partitioning, populating, computing cell areas, cost estimation
- [[ch15-floorplanning]] — 15.2 Floorplanning: Floorplanning strategies
- [[ch15-padring]] — 15.3 Constructing the Padring: Scribe streets/seals, bondpads, solder bumps, copper pillars
- [[ch15-interconnection]] — 15.4-15.5 Top-Level Interconnection and Checklist: Vias, routing, star nodes, Kelvin, noisy/sensitive/HV/HC signals, final checklist
