# Hastings ch.14: Special Topics (merged devices, guard rings, single-level interconnect, ESD)

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson 2023), Chapter 14 "Special Topics", printed pp. 721–779.
Reftext file: `scratchpad/reftext/hastings.txt`, lines **42876–46303** (all read).
PDF: `ref/The Art of Analog Layout 3ed 2023 -- Ray Alan Hastings ...pdf`. PDF page index = 1 + count of `\f` before the line; the printed page number is PDF page − 1. Because `pdftotext` drops inline math, I read PDF pages 723–725, 728–730, 735–736, 739–741, 745–746, 748–755, 758–775 as images to recover the equations and numbers. Every number below that the reftext leaves blank comes from those page images. Where a value is not legible in either source, the entry says "not given".

Glossary used below: NMoat/PMoat = N+/P+ active (NSD/PSD). ECGR/HCGR = electron-/hole-collecting guard ring. EBGR/HBGR = electron-/hole-blocking guard ring. NBL = N buried layer. Deep-N+ = sinker. DNW = deep N-well (the sky130 `dnwell`). HBM/CDM = human-body/charged-device model.

---

## 1. Coverage

Consecutive Read chunks on the reftext. A first 2000-line request was refused by the token cap, so I re-read in four chunks with no gaps:

| # | offset..end | content |
|---|---|---|
| 1 | 42876..43775 | Ch.14 intro, §14.1 (all), §14.2 intro, §14.2.1, first half of §14.2.2 |
| 2 | 43776..44675 | rest of §14.2.2, §14.2.3–14.2.6, §14.3 (all), §14.4 intro, §14.4.1 up to the buffered Zener |
| 3 | 44676..45575 | V_CES/V_CER, V_ECS, APD, dual diodes, thick-field NMOS, GGNMOS, GCNMOS, BTNMOS, active FET, MVSCR, lateral PNP, Table 14.2, §14.4.2 up to drain ballasting |
| 4 | 45576..46303 | Fig 14.36 caption, RC filters, CDM clamps, §14.4.3 (all), §14.5, bibliography, §14.6 exercises, "Chapter 15" header |

Section and subsection headings in the range, in order:
- Chapter 14 Special Topics (intro): 42876
- 14.1 Merged Devices: 42911
  - Minority Carrier Injection: 42968
  - Majority Carrier Debiasing: 42987
  - Capacitive Coupling: 43022
  - 14.1.1 Problematic Device Mergers: 43161
  - 14.1.2 Successful Device Mergers: 43341
  - 14.1.3 Low-Risk Device Mergers: 43445
  - 14.1.4 Medium-Risk Device Mergers: 43521
  - 14.1.5 Devising New Device Mergers: 43589
- 14.2 Minority-Carrier Guard Rings: 43623
  - 14.2.1 Standard Bipolar Electron Guard Rings: 43649
  - 14.2.2 Standard Bipolar Hole Guard Rings: 43718
  - 14.2.3 CMOS Electron Guard Rings: 43782
  - 14.2.4 CMOS Hole Guard Rings: 43841
  - 14.2.5 BiCMOS Electron Guard Rings: 43877
  - 14.2.6 BiCMOS Hole Guard Rings: 43954
- 14.3 Single-Level Interconnection: 44054
  - 14.3.1 Mock Layouts: 44075
  - 14.3.2 Techniques for Crossing Leads (and Table 14.1): 44142 / 44200
  - 14.3.3 Types of Tunnels: 44232
- 14.4 ESD Protection: 44335
  - 14.4.1 Primary ESD Protection: 44432
    - Buffered Zener: 44608
    - V_CES and V_CER Clamps: 44678
    - V_ECS Clamp: 44737
    - Antiparallel Diodes: 44791
    - Dual Diodes: 44827
    - Thick-Field NMOS: 44885
    - Grounded-Gate NMOS (GGNMOS): 44986
    - Gate-Coupled NMOS (GCNMOS): 45080
    - Backgate-Triggered NMOS (BTNMOS): 45167
    - Active FET: 45240
    - Medium-Voltage SCR (MVSCR): 45270
    - Lateral PNP: 45362
    - Table 14.2 Comparison of ESD devices: 45426
  - 14.4.2 Secondary ESD Protection: 45488
    - Emitter-Base Clamp: 45496
    - Drain Ballasting Resistors: 45553
    - RC Filters: 45601
    - CDM Clamps: 45659
  - 14.4.3 Die-Level ESD Protection Strategies: 45711
    - High-Current Metallization (incl. Table 14.3): 45822 / 45893
    - High-Current Resistors: 45982
    - Guidelines for Choosing Primary ESD Devices for Pad-Based Networks: 45999
- 14.5 Summary: 46124
- Selected Bibliography: 46138
- 14.6 Exercises: 46165

---

## 2. Section-by-section digest

### Chapter intro (42876–42909, PDF p.722)
- The chapter covers four specialized structures: merged devices, minority-carrier guard rings, tunnels (crossunders) and ESD protection (42878–42879).
- Merging saves area and sometimes improves performance. The costs are design effort and possible unexpected interactions (42882–42886).
- Guard rings stop minority carriers from one device reaching others. They suppress latchup and block noise, and they are hard to build in pure CMOS because it has no deep sinkers or buried layers (42890–42893).
- Die-to-die connections that never leave the package, and trimpads or testpads reachable only at wafer probe, do not need ESD protection (42903–42905).

### 14.1 Merged Devices (42911–42964, PDF p.723)
- Lateral diffusion is about 80% of junction depth. Two 10 µm-deep diffusions must be ≥16 µm apart, or about 20 µm once depletion is allowed for. Deep diffusions (isolation, wells) therefore cost area, and merging devices into a common tank or well removes them (42912–42917; numbers from PDF p.723).
- Three NPNs whose collectors share a node can share a tank, using about 70% of the separate area (42920–42923).
- Three PMOS on a common backgate net can share one N-well, and with a common gate one poly strip. This cuts area by more than half (42938–42942).
- Merged devices interact through three mechanisms: minority carriers, majority carriers (debiasing) and electric fields (capacitive coupling). Minority carriers have caused the most trouble (42957–42964).

### Minority Carrier Injection (42968–42983, PDF p.724)
- Injected minority carriers diffuse to other devices in the shared region. The parasitic BJTs are rarely modelled accurately, so simulation can mislead (42969–42971).
- The only sure prevention is to forward-bias no PN junction or Schottky contact in a shared region. A collector tied to a pin that is pulled below ground injects into the substrate (42971–42974).
- Fallbacks are a minority-carrier guard ring around the offender, more distance, or more current in vulnerable devices. Rings can leak, for example electrons diffusing under a standard-bipolar ECGR (42978–42983).

### Majority Carrier Debiasing (42987–43018, PDF p.724)
- Worked example: 1000 □ of 50 mΩ/□ metal carrying 100 µA drops 0.5 mV (42990–42991).
- Shared semiconductor regions debias more than metal. Example: 2 µA base current through a 500 Ω shared base drops 1 mV (43001–43004).
- A merged device on a pin can inject the full latchup test current, typically 100 mA. Through a 5 Ω deep-N+ sinker that gives 0.5 V of NBL debias, enough to forward-bias junctions (43007–43010).
- Do not merge pin-connected devices with debias-sensitive devices. Matched devices must not share a region whose debias lies in the critical path: matched emitter followers may share a tank (collector) but not a base (V_BE) (43014–43018).

### Capacitive Coupling (43022–43157, PDF pp.724–726)
- Eq 14.1: V_P = R·S·C_P, where S is the aggressor slew (V/s), R the victim node resistance and C_P the coupling capacitance (43023–43027).
- Case: a minimum-width M2 crosses a minimum-width M1 at right angles (1 µm wide, 10 kÅ ILD), giving C_P ≈ 0.05 fF. A 1 V/ns clock into a 200 kΩ node makes 50 mV spikes on a 100 mV signal (43032–43036). Hence digital signals must never be routed over sensitive analog circuitry (43042).
- Analog nodes that slew fast are also aggressors. About 1 fF between a charge-pump node swinging more than 20 V and a µA current mirror produced what looked like subharmonic oscillation. Parasitic back-annotation found it (43042–43046).
- Substrate coupling: slewing voltage drives current through junction capacitance into substrate resistance, then back out elsewhere. It rises with frequency (43058–43063).
- Countermeasures:
  - Substrate contacts around the injector attenuate more than contacts around the victim. Widening the ring helps only a little, so minimum width is normal. Injector-ring and victim-ring grounds should return separately to the ultimate ground, ideally on separate pins or at least separate bondwires (43072–43093).
  - N-type noise-blocking rings (NSD or N-well, deeper is better) tied to a clean supply or ground go outside the substrate-contact ring. They do not replace substrate contacts (43096–43103). Fig 14.3 shows the arrangement (43107–43119).
- Further options: backside connection to a P+ substrate (solder mounting) (43125–43128); distance plus rings on lightly doped substrates (43132–43135); NBL + deep-N+ isolation with separate package pins, which protected a mic preamp next to a roughly 1 A switcher (the attenuation figure is not legible), and is less effective at GHz because of bondwire inductance (43138–43145).
- Merge compatibility: sensitive with sensitive and noisy with insensitive are fine, noisy with sensitive is not. If they must be merged, use plentiful majority-carrier contacts (43153–43157).

### 14.1.1 Problematic Device Mergers (43161–43337, PDF pp.726–730)
- In a split-collector lateral PNP, a saturated collector reinjects holes into the others (Fig 14.4A). Two lateral PNPs sharing a tank see about ¼ of the saturated device's current cross over (Fig 14.4B). Unsaturated leakage under a collector is below 1% but grows as V_BC → 0 (43162–43194).
- Symmetric arrays at equal emitter current and V_BC cancel cross-injection, so even accurately matched LPNPs can share a tank (43195–43197).
- P-bars and N-bars block most cross-injection but not all. Ask what 1% leakage would do to the circuit, and if it would break it, use separate tanks. N-bars need heavy doping (43201–43209).
- An NPN driving an LPNP whose base is the NPN collector forms an SCR (Fig 14.5). Latchup occurs when Eq 14.2 holds: β_N·β_P·(1 − η_C) > 1. With β_N = 300 and β_P = 10 the bar needs η_C > 0.997 (99.7%), which cannot be assured. Never merge if the PNP could saturate (43212–43257; values PDF p.728).
- NPN plus base resistor sharing a tank contact (Fig 14.6): tank debias forward-biases the resistor end and closes a PNPN loop. The trigger debias is about 300 mV at 150 °C. At 100 µA that needs 3 kΩ of tank resistance, which is plausible without deep-N+ (hundreds to thousands of Ω) and unlikely with it (≤100 Ω) (43283–43299; PDF p.729).
- NPN plus Schottky in a shared tank (Fig 14.7): the P guard ring of the Schottky injects once the diode drop exceeds a PN V_F. Even a field-plated Schottky injects about 0.5% minority current, which latches if β_NPN > 200. A collector contact not fully enclosed by N+ is itself a Schottky injector (43302–43337; PDF p.730).

### 14.1.2 Successful Device Mergers (43341–43441, PDF pp.730–732)
- Merged Darlington (Fig 14.8) uses a shared tank and a deep-N+ bar contact (5–10 Ω vertical), handling hundreds of mA without Q2 saturating. Only saturating NPNs and large power devices need full deep-N+ rings (43346–43353).
- Base contacts double as turnoff-resistor heads. The HSR implant runs in behind the base contact to keep it clear of the emitter. The whole layout is routable in single-level metal (43386–43392).
- UC3842-style input stage (Fig 14.9): split-collector LPNPs with the large collector merged into isolation. Lateral PNPs replace substrate PNPs so their collectors act as P-bars. Balanced pairs (Q7/Q8) tolerate symmetric cross-injection. Matched merges get identical emitter strips, and a tank contact on one of them has little effect on matching (43395–43441).

### 14.1.3 Low-Risk Device Mergers (43445–43517, PDF pp.732–733)
Use these whenever possible (43446–43447):
1. Sections of one matched device. These are more compact and so less gradient-sensitive, but watch tank or well modulation and emitter spacing (43450–43458).
2. Fingers of power devices. A dispersed layout (banks with other circuitry between, or interleaved deep-N+ strips) lowers peak temperature (43459–43468).
3. Sense transistor with its power device. Best is two equal sense sections on an axis of symmetry, each about halfway between the centre and the periphery, so each sits at the mean temperature. Embedding a single sense device in the middle is worse because that is the hottest spot. Adjacent on a symmetry axis is the least good (43469–43481).
4. Back-to-back LDMOS with a common drain: interdigitate the two sources and drop the drain fingers (43482–43487).
5. Schottky-clamped NPN (43488–43497).
6. I²L multi-emitter inverse-mode NPN (43498–43504).
7. Darlington (43509–43512).
8. Base turnoff resistors in the NPN tank. A substrate-tied end may run into isolation, with a substrate contact nearby (43513–43517).

### 14.1.4 Medium-Risk Device Mergers (43521–43585, PDF pp.733–734)
1. MOS in a common backgate. An output transistor (S/D on a pin that is neither a supply nor substrate ground) can inject into the shared backgate or trigger CMOS latchup. Prefer its own backgate region. Otherwise add rings and low-R backgate contacts on the adjacent complementary devices. Always provide low-R backgate ties, scattered through the structure when there is no low-R sublayer. Devices with deliberately forward-biased backgates (charge pumps, clamps) must be marked and given their own region (43526–43547).
2. Diffused resistors in a common tank. Pin-connected resistors get their own tank and possibly rings, and noisy resistors must not share with sensitive ones (43548–43555).
3. Lateral PNPs on one base net. Saturating ones go in their own tank (43556–43566).
4. Split-collector LPNP. Cross-injection cannot be blocked, so split the device and isolate the saturating part (43567–43573).
5. Zeners. Legal only if the tank voltage is at or above the Zener cathode voltage (43574–43578).
6. Shared-collector BJTs. Add a deep-N+ plug so the collector does not debias. A fluctuating tank voltage couples into merged bases (43579–43585).

### 14.1.5 Devising New Device Mergers (43589–43617, PDF p.734)
- Three questions to ask of any merger. (1) Can any merged device inject minority carriers? Sources are saturating BJTs, forward-biased Schottkys and pin-connected diffusions. (2) Can any merged device debias the shared region? A deep-N+ plug handles tens of mA. (3) Can noise coupling upset the circuit? NMOS in a P-epi tank has high backgate R. When in doubt, separate them (43595–43612).
- CMOS/BiCMOS practice is shared backgates only, because other mergers save little area (43615–43617).

### 14.2 Minority-Carrier Guard Rings (43623–43645, PDF p.735)
- Latchup is intermittent and application-dependent, and simulation or test rarely finds it (43624–43626).
- Causes are pins driven above the supply or below ground by ESD, supply interruptions, inductive kickback or inductive spiking. Diffusions reached through deposited resistors below about 50 kΩ still count. Capacitor-coupled diffusions (charge pumps) can also trigger (43629–43638; 50 kΩ from PDF p.734).
- Ring the injectors, which are fewer than the potential victims. ESD devices at the periphery can share one ring that separates the core from ESD and pads (43641–43645).

### 14.2.1 Standard Bipolar Electron Guard Rings (43649–43714, PDF pp.735–736)
- Standard bipolar can build an ECGR but not an EBGR: deep-N+ + NBL + emitter, tied to the highest supply to deepen the depletion region and resist debias. Grounded rings are shallower and debias easily. Put a supply-tied second ECGR around a grounded ring that could saturate (43650–43658).
- Supply-tied tanks lying between an injector and sensitive circuits are effective ECGRs, with as much NBL and deep-N+ as possible. An ECGR is most efficient right next to the injector (43680–43689).
- Without deep-N+ (Fig 14.10B) the ring is debias-prone and useful only when tied to a supply (43693–43695).
- On a P- substrate these rings are marginal, and many designers rely on spacing plus hole rings. Gate drivers and inductive-load drivers still need ECGRs as near the offender as possible (43698–43705). A P+ substrate makes them far more effective (43709–43714).

### 14.2.2 Standard Bipolar Hole Guard Rings (43718–43778, PDF pp.736–737)
- HCGR: reverse-biased base diffusion collecting holes, grounded or on a negative supply. It can merge with isolation if it collects no more than a few mA and contains substrate contacts. Rings collecting larger currents must not merge (43718–43733).
- HBGR: deep-N+ must completely encircle the injector, and drawn NBL must reach the outer edge of drawn deep-N+. Efficiency is above 95% and often above 98%. An HCGR inside an HBGR exceeds 99% (43753–43763).
- Hole rings mainly prevent cross-injection between merged parts. P-regions on pins exposed to about 100 mA get their own tanks with nearby substrate contacts (43771–43778).

### 14.2.3 CMOS Electron Guard Rings (43782–43837, PDF pp.737–738)
- CMOS is more latchup-prone than bipolar. Retrograde P-wells act like a buried layer (43783–43792).
- Substrate-contact rings around NMOS are majority-carrier guard rings. They pin the backgate but do not block or collect minority carriers (43795–43799).
- NMoat ECGR: NSD is shallow and electrons pass under it. Low CMOS supply voltages cannot deepen the depletion region, so the only lever is width: at least the P-epi thickness, or the P-well depth in retrograde processes (43803–43809).
- N-well ECGR: contact it with as much NMoat as possible, tie it to a supply, and expect it to be of doubtful value in deep lightly doped wells. If the ring must be grounded, a wide NMoat ring beats an N-well ring (43825–43831).
- Rings are often omitted in favour of backgate contacts plus spacing, but a wide NMoat ECGR "can significantly improve latchup immunity" when there is room (43835–43837).

### 14.2.4 CMOS Hole Guard Rings (43841–43873, PDF p.738)
- A hole ring needs a retrograde N-well whose bottom doping is at least 10× the middle (43842–43846).
- Pure CMOS cannot build an HBGR (no sinker or trench). An HCGR is a PMoat ring in the N-well around the injector, grounded, with width at least the N-well depth (43849–43855).
- The common alternative is NMoat backgate rings around PMOS, which are majority-carrier rings (43871–43873).

### 14.2.5 BiCMOS Electron Guard Rings (43877–43950, PDF pp.738–740)
- In a single-well 20 V CDI BiCMOS, an ECGR of NBL + deep-N+ + NMoat + N-well reaches above 90% efficiency. With deep-N+ it can be grounded even at latchup test current (100–200 mA). Without it, tie to a supply and compute the vertical resistance (43897–43907; numbers PDF p.738).
- An isolated NMOS in a P-tank cut off by NBL and deep-N+ (or by DTI with a tilted As implant) is a 100%-efficient ECGR (43915–43933).
- P- substrates are far more latchup-prone and ECGRs weaken on them. Many such designs needed re-spins. SOI is not immune when PMOS and NMOS share a tank: isolate known injectors and scatter many backgate contacts through both wells (43936–43950).

### 14.2.6 BiCMOS Hole Guard Rings (43954–44048, PDF pp.740–741)
- CDI HBGR: NBL under the N-well plus an encircling deep-N+. Drawn NBL and drawn N-well both extend at least to the outer edge of drawn deep-N+. N+/N− doping ratio ≥ about 100:1. With shallow-well surface doping above 10¹⁸ cm⁻³, deep-N+ needs above 10²⁰ cm⁻³, otherwise holes leak through the sidewalls (seen with wells built for <3 V) (43973–43985).
- A permeable NBL can make an HBGR increase substrate injection. HCGRs are unaffected (43993–43996).
- Twin-well BiCMOS with a PBL: the PBL + P-well around a shallow N-well is a 100% HCGR. Contact it with a PMoat ring, grounded, as wide as possible. More space to deep-N+ lowers vertical resistance (44008–44024).
- DTI HBGR: trench ring + NBL across the tank, NBL drawn to the outer trench edge. Measured permeation was under 5% (44027–44044).

### 14.3 Single-Level Interconnection (44054–44071, PDF p.743)
- With two or more metals, placement is limited only by matching and packing, and autorouters can route analog blocks if given keep-outs, width rules and shielding guidance (44055–44059).
- Single-metal areas still exist, for example under M2/M3 capacitors placed over active circuitry (44069–44071).

### 14.3.1 Mock Layouts (44075–44138, PDF pp.743–744)
- Mock layouts (rectangles marked E/B/C, strips for resistors, "TC" for tank contacts) are tried in several arrangements to minimize tunnels (44076–44082).
- Matched sets are arranged symmetrically about one axis (44099–44101).
- Non-integer ratio R4/R3 = 6.441 (160 Ω/□, 621 Ω and 4 kΩ): with R3 as the unit, R4 needs 7 segments and its centroid cannot align. Eight partial segments leave R3 objectionably short (3.88 □). Better is six 666.7 Ω segments for R4 plus one partial central segment for R3 with a sliding contact for trim. R5/R6 (18.75 □ each) become 2 × 9.375 □ interdigitated (44105–44113; values PDF p.744).
- A 6:1 ratioed pair splits the larger device into two halves either side of the smaller one. Matched minimum-size LPNPs on one base net share a tank side by side (44116–44120).
- Leads route through resistor arrays and between stretched transistor terminals, so no tunnels are needed (44124–44128). Paper dolls are the historical placement-search aid (44131–44138).

### 14.3.2 Techniques for Crossing Leads (44142–44228, PDF pp.744–745)
1. Cross over resistors, except field-plated ones, sensitive ones, or lightly doped ones such as 2 kΩ/□ HSR, which suffer voltage modulation (44148–44153).
2. Rearrange device terminals (CEB vs CBE; CBE has slightly more R_C) (44154–44158).
3. Stretch devices to pass leads between terminals. This adds R/C, and if one matched device is stretched, stretch all of them (44159–44164).
4. Route through merged tunnels (a base with two contacts). Large currents cause debias (44170–44175).
5. Insert tunnels. They cost area, R and C, must not carry high-current leads, and must respect junction voltages. The circuit designer approves each one and resimulates with it (44176–44183).
6. Rearrange bondpads, which needs system and package approval (44184–44188).
- Table 14.1 annotation scheme (44200–44228):
  - Power: no tunnels, minimum width, red.
  - Noisy: do not cross sensitive leads or the devices they connect to, yellow.
  - Sensitive: no tunnels, no substrate contacts in sensitive ground leads, green.
  - Noncritical: none.
  - Also list matched components and devices needing guard rings (44195–44196).

### 14.3.3 Types of Tunnels (44232–44329, PDF pp.745–747)
- Sheet resistances: base 100–200 Ω/□; emitter, deep-N+ and NBL about 10 Ω/□. Only base can share a tank without merging signals (44233–44236).
- A base tunnel adds a few hundred Ω and tens to hundreds of fF. Widening it trades R for C and leakage (44240–44245).
- Emitter tunnels sit in their own tank and NBL is not worth adding (44251–44254). Emitter-in-iso breaks down at a few volts, so use it in ground leads, or for signals if breakdown is ≥6 V. Capacitance is about 1.5 fF/µm² (44269–44273).
- A stacked tunnel (emitter + deep-N+ + NBL) reaches 3–5 Ω/□ (44293–44296). An NBL tunnel bridges two tanks and the isolation between them acts as a P-bar. The required NBL/iso breakdown value is not legible (44312–44315).

### 14.4 ESD Protection intro (44335–44428, PDF pp.748–749)
- Standards are 1 kV HBM and 250 V CDM, and many customers still ask for 2 kV HBM / 500 V CDM (44344–44346; PDF p.747).
- A 2 kV HBM pulse rises in about 15 ns to about 1.3 A and decays with τ ≈ 230 ns, so thermal runaway is possible. A 500 V CDM pulse exceeds 5 A in about 0.25 ns and lasts 1–2 ns, so filamentation is possible but runaway is not (44347–44353).
- System-level IEC 61000-4-2 8 kV means about 30 A peak in under 1 ns, then about 16 A over 50–100 ns. That is normally handled off-chip (44368–44374).
- Worked example on a 5 V NMOS gate:
  - A 7 V, 2 Ω Zener clamps 2 kV HBM to 9.6 V, which the oxide survives briefly (limit about 10 V).
  - 500 V CDM (about 12 A) gives 31 V, so the Zener alone fails.
  - Adding 500 Ω plus a 50 Ω secondary Zener limits the current to 44 mA and the oxide sees 9.2 V.
  - This defines primary and secondary protection (44382–44428; numbers PDF p.748).

### 14.4.1 Primary ESD Protection (44432–44604, PDF pp.748–751)
- A primary device must keep the HBM peak below junction breakdowns and the CDM peak below field and interlevel oxide breakdown. Large junctions may be self-protecting, but filament-prone devices (lightly doped drift regions) cannot absorb HBM regardless of size (44433–44448).
- Clipping device: V_c (defined at 1 µA, sometimes 100 nA) above the maximum operating voltage; V_pk (at 1.3 A) below junction and oxide breakdown; I_f well above I_pk. Obtain I_f by TLP. Leakage rises after ESD stress (44451–44509).
- Snapback device: the same checks with I_t2 ≫ I_HBM, plus one of (a) V_h above the maximum operating voltage, (b) V_t1 above the operating voltage and available current below I_h, or (c) V never above V_t1. Do not rely on (c). Low-V_h devices belong only on current-limited pins (44512–44558).
- Rate-fired device: trigger on about V_pk in under 50 ns (roughly 1 V/ns). Do not use it where slew exceeds 100 V/µs unless it has a disable circuit. Pins that need system-level ESD cannot use disable-type rate-fired protection (44562–44604; values PDF p.750).

### Buffered Zener (44608–44674, PDF pp.751–752)
- A Zener drives an NPN, dividing the Zener R by β to reach a few Ω. Zeners can be stacked below the NPN's V_CER (44609–44614).
- R1 is about 1 kΩ. It lowers leakage gain, raises V_CER and resists dV/dt turn-on (44629–44633).
- 300–600 µm² of emitter gives 2 kV HBM. Negative strikes forward-bias collector-substrate. Layout is all in one tank with a shared deep-N+ (44637–44655).
- It achieved more than 2 kV HBM / 200 V MM in 20 V BiCMOS. It injects electrons, so it needs an ECGR. It shows some rate firing, which a smaller R1 reduces (44659–44674).

### V_CES and V_CER Clamps (44678–44732, PDF pp.752–754)
- V_t1 runs from V_CES (about 65 V for a 40 V NPN) down to V_CER, set by R1. V_h = V_CEO(sus), about 45 V (44679–44684).
- Fillet the base corners. Base overlap of contact is 1–2 µm above the rule, emitter overlap a couple of µm (doubtful value) (44705–44719).
- 300–500 µm² of emitter gives 2 kV HBM. Needs an ECGR (44723–44728).

### V_ECS Clamp (44737–44788, PDF pp.754–755)
- Reverse-active NPN: V_t1 ≈ V_EBO and V_h ≈ 60–80% of V_EBO. Under 600 µm² gave 2 kV HBM / 200 V MM, and larger devices survived 10 kV (44738–44780).
- Two emitters share one base contact. It seldom needs an ECGR because the collector is grounded with NBL and a large sinker (44754–44758).
- Stacking 2–3 is possible but series R forces larger devices, so a buffered Zener or V_CES may be smaller (44784–44788).

### Antiparallel Diodes (44791–44823, PDF p.755)
- Used between separate grounds. D1 is NMoat/P-epi interdigitated with substrate contacts. D2 is PMoat/N-well, effectively a substrate PNP (44792–44799).
- Minimum-width fingers and spacing, in near-square arrays to limit metal R. Size is set by the allowed series R (a few Ω). Robust, and needs an ECGR (44814–44823).

### Dual Diodes (44827–44881, PDF pp.755–756)
- D1 goes to substrate and D2 to another pin (usually the supply) protected by its own clamp. The D2 connection must be wide metal. The rule is that ESD path metal between any two pins is ≤ 2 Ω (44828–44853).
- Not failsafe: it forward-biases whenever the pin exceeds the supply, including during supply sequencing (44857–44863).
- Failsafe devices do not rely on other pins: V_CES, V_CER, V_ECS, buffered Zener, APD (44871–44873).
- Dual diodes are small, add low capacitance and suit high-speed pins. Needs an ECGR (44873–44881).

### Thick-Field NMOS (44885–44982, PDF pp.756–758)
- A lateral NPN snaps back to about 50% of the NMoat/P-epi breakdown. A metal gate tied to the drain lowers the trigger and must overlap the drain junction (44906–44937).
- Drain overlap of contact is several µm over the rule, for ballast and to keep heat off the contact. Fillet or chamfer the drain corners. Block silicide except under contacts (44928–44932).
- Multi-finger: the first finger to trigger dies unless V_t2 > V_t1, so add per-finger ballast. Needs an ECGR. Obsolete in modern processes (44963–44982).

### GGNMOS (44986–45076, PDF pp.758–759)
- Avalanche then parasitic NPN snapback. All fingers must turn on together, which requires V_t2 ≫ V_t1 (44988–45050).
- Wider NMoat-over-contact improves robustness. LDD hurts. Silicided S/D is fragile unless silicide is blocked several µm back (45027–45033).
- L = minimum for that device style. Overlap is 1–3 µm above minimum, and silicide-block extension is 1–3 µm. Total width is about 600 µm for 2 kV HBM. Designers characterize arrays of test structures (45054–45070; numbers PDF p.759).
- Needs an ECGR (45073–45076).

### GCNMOS (45080–45163, PDF pp.759–760)
- A GGNMOS triggers slightly above BV_DSS, so it cannot protect same-style NMOS without series R (45081–45084).
- Gate coupling lowers V_t1 to about ½ of the zero-V_GS value when V_GS ≈ ½ V_GD. Keep V_GS low so current flows through the BJT rather than the channel (45088–45126).
- R1·C1 = 20–50 ns. Disable circuit: R2·C2 ≥ 1 µs, with M2 holding off M1 while powered (45129–45143).
- It can protect same-style NMOS without series resistors. C1 is a PMOS gate cap and R1 a small poly resistor. R2/C2 can serve several clamps. A PMOS C1 and M1 (and M2) must sit behind an ECGR (45146–45163).

### BTNMOS (45167–45236, PDF pp.760–762)
- Deliberate backgate debias lowers V_t1 and can raise I_t2, so it is often smaller than a GCNMOS (45173–45186).
- Substrate-pumped NMOS: predrive M2 is split into two halves either side of M1, M2's source goes to a PMoat ring enclosing both, and C1 is a small PMOS cap with an HSR poly R1 (45195–45225).
- Needs a disable circuit on high-slew pins and an ECGR. An N-well strip on the supply also cuts lateral majority flow (45231–45236).

### Active FET (45240–45266, PDF p.762)
- Pure MOS conduction, rate-fired. Protection scales linearly with W (1 kV → 2 kV by doubling W). Always the largest option ("big FET") (45241–45260).
- Needs a disable circuit on slewing pins and an ECGR (45264–45266).

### MVSCR (45270–45358, PDF pp.762–764)
- V_h can be about 1 V and I_h only a few mA, so it latches by design. Use it only if the maximum operating voltage is below V_h, or the supply current is below I_h (45271–45282).
- Triggered by an NMoat/P-epi Zener, so V_t ≈ BV_DSS. A high-holding-current variant reached 70 mA, still below what many pins source. Stacking erases the area benefit (45286–45351).
- Needs an ECGR (45357–45358).

### Lateral PNP (45362–45416, PDF pp.764–765)
- A CMOS lateral PNP snaps back deeply thanks to base conductivity modulation, which suits high-voltage pins (45363–45368).
- Closely interdigitated narrow emitter and collector fingers. No ballasting needed (holes filament less) (45397–45401).
- The floating-base variant has lower V_t (V_CEO), almost no snapback and no pin-to-N-well injection, but still gets an ECGR (45404–45416).

### Table 14.2 (45426–45483, PDF p.765)

| Device | Behavior | Typical voltages | Area | ECGR |
|---|---|---|---|---|
| Buffered Zener | clipping (some rate-fired) | V_c = 0.7 V_EBO | medium | yes |
| V_CES clamp | snapback (some rate-fired) | V_t1 = V_CES, V_h = V_CEO(sus) | medium | yes |
| V_ECS clamp | snapback | V_t1 = V_EBO, V_h = 0.8 V_EBO | medium | no |
| APD | clipping | V_c = 0.3 V | small | yes |
| Dual diodes | clipping | V_c = V_DD | small | yes |
| Thick-field NMOS | snapback | V_t1 ≈ 0.9 BV_DSS, V_h ≈ ½ BV_DSS | medium | yes |
| GGNMOS | snapback | V_t1 = BV_DSS, V_h ≈ ½ BV_DSS | medium | yes |
| GCNMOS | rate-fired | ½ BV_DSS > V_t1 > BV_DSS (as printed), V_h ≈ ½ BV_DSS | medium-large | yes |
| BTNMOS | snapback (some rate-fired) | same as GCNMOS | medium | yes |
| Active FET | rate-fired | V_c ≈ 0.9 BV_DSS, V_t2 ≈ 2 V | large | yes |
| MVSCR | snapback | V_t = BV_DSS, V_h very low | small | yes |
| Lateral PNP | snapback | V_h ≈ ½ V_t1 | small | yes |

### 14.4.2 Secondary ESD Protection (45488–45494, PDF p.765)
- Vulnerable structures are bipolar E-B junctions, some MOS drains, and gate oxides.

### Emitter-Base Clamp (45496–45549, PDF pp.765–766)
- Reverse E-B avalanche permanently degrades low-current β in standard-bipolar NPNs and all poly-emitter BJTs. The resulting matching loss counts as an ESD failure (45497–45528).
- A V_CES primary (60 V trigger, 40 V snapback) cannot protect a junction that avalanches below 10 V (45509–45513).
- Fix (Fig 14.35B): a diode-connected Q5 clamps V_EB, R1 limits current, and a balancing R2 = R1 goes on the other input of the diff pair to cancel the base-current I·R offset (45542–45549).

### Drain Ballasting Resistors (45553–45597, PDF pp.766–767)
- An output NMOS triggers like a GCNMOS through C_GD, so a parallel GGNMOS does not help. Options: a low-V_t rate-fired clamp, an enlarged and ballasted output device, or a GGNMOS/GCNMOS plus a series-limiting R (45554–45566).
- RW product: R_ballast = (RW)/W_finger. Example: 5 mm·Ω with 50 µm fingers needs 100 Ω. Per-finger resistors of 100 Ω on five fingers give 20 Ω effective and still meet RW (45584–45597; numbers PDF p.767).

### RC Filters (45601–45655, PDF pp.767–768)
- Oxide stress limits:
  - A 50 Å oxide withstands about 24 MV/cm for 100 ns.
  - A 90 Å 5 V oxide withstands about 22 V during an ESD event.
  - A 5 V GCNMOS (6 V V_h, 4 Ω) plus 2 Ω of metal reaches 13.8 V at 1.3 A (safe) but 46 V at 10 A CDM (fails) (45604–45620).
- 2 Ω of metal at 10 A is 20 V, so secondary protection must sit near the protected gate and tie to that circuit's own supply and ground (45623–45626).
- RC filter: R1·C1 ≥ 10 ns, so C1 = 0.25 pF gives R1 ≥ 40 kΩ. C1 sits near M2 (short ground). R1 can be anywhere on the line because it limits current, not voltage (45630–45641).

### CDM Clamps (45659–45707, PDF pp.768–769)
- Series R1 plus small single-finger GGNMOS clamps from gate to source (M4) and source to gate (M5), each meeting RW (45662–45670).
- An ECGR goes around M4/M5 unless R1 > about 25 kΩ (45670–45672).
- The version with an extra GGNMOS is failsafe and preferred. Omit clamps for absent gate types (45693–45701).
- Clamps sit near the protected oxides, with rail taps joined near the protected devices and a lead wide enough for the CDM current (45704–45707).

### 14.4.3 Die-Level ESD Protection Strategies (45711–45819, PDF pp.769–771)
- Pad-based network: every pin except substrate ground gets a primary device to a substrate ring (45718–45722).
- Eq 14.3: ΔV = V_E2(I_pk) + V_E3(−I_pk) + I_pk·R_VSS, with R_VSS ≤ 2 Ω giving 2.6 V at 2 kV HBM (45738–45753).
- Rail-based network: dual diodes to VDD and VSS rails plus one rail clamp. Eq 14.4: ΔV = V_D2(I_pk) + I_pk·R_VDD + V_E1(I_pk) + I_pk·R_VSS + V_D3(I_pk), with diode drops about 2–3 V each and R_VDD, R_VSS ≤ 1 Ω each (45757–45782).
- Analog favours pad-based because multiple sequenced supplies make dual diodes (non-failsafe) risky (45784–45787).
- Custom network (Fig 14.40): separate GND and PGND with APD between them. The output clamp returns to PGND so negative output transients do not disturb substrate ground (45791–45819).

### High-Current Metallization (45822–45976, PDF pp.771–773)
- Keep HBM metal drop ≤ about 3 V, so metal R ≤ 2.3 Ω at 1.3 A. In pad-based networks 2 Ω goes to the substrate ring. The worst case is between diametrically opposed pads (45827–45831).
- Eq 14.5 ring width: W_G ≅ R_S·(W_D + L_D − 4d_C + 2√2·d_C)/(4 Ω), where d_C is the corner exclusion (45835–45841, Fig 14.41).
- Rail-based: widen both rails to a combined 2 Ω, or distribute rail clamps (45857–45861).
- Use the process maximum R_S at 25 °C and stack all metals (45865–45867).
- Fusing: "20X/40X EM" rules lack justification. Instead limit the adiabatic rise (often 50 °C). Eq 14.6: A = √(ρ·τ·I_pk²/(2·C_V·ΔT)), τ = 225 ns. With 1.3 A and 50 °C, Al needs 6.5 µm² and Cu 4.3 µm², so 0.5 µm Al needs ≥ 13 µm width (45870–45889).
- Table 14.3 (ρ µΩ·cm, C_V J/°C/cm³): Al 2.7/2.42, CoSi₂ 15/0.56, Cu 1.7/3.45, NiSi 10.5/3.45, Si varies/1.66, TiSi₂ (C54) 15/0.85 (45893–45932).
- Inside corners get 45° chamfers extending back into the lead "at least half the thickness of the lead" (d_cf) (45943–45945).
- Stress slots are oriented along current flow, or the lead is replaced by parallel narrower leads (45961–45968).
- Layer changes: overlap the leads and fill the overlap with as many vias as fit (45972–45976).

### High-Current Resistors (45982–45995, PDF p.774)
- Eq 14.6 also applies to resistors, with ρ = R_s·t.
- Example: 5 kÅ poly at 50 Ω/□ gives 2.5 mΩ·cm. A 200 Ω resistor between an 8 V CDM clamp and a 12 V primary carries 4 V / 200 Ω = 20 mA. At 50 °C it needs 3.7 µm², or 7 µm width (45984–45990).
- Silicided: current flows in the silicide, so 0.2 µm TiSi₂ needs 0.40 µm², or 2.0 µm width (45993–45995).

### Guidelines for Choosing Primary ESD Devices (45999–46118, PDF pp.774–776)
1. Substrate-ground pads need no primary device, but the ring must be wide enough (46006–46009).
2. Pads tied by stout metal may share one primary device. The metal must also carry package and board imbalance current, ideally as a solid plate (46010–46024).
3. Separate grounds are joined by APD (46025–46031).
4. Separate pads bonded to one pin (Kelvin) each need their own primary device, because of about 1 nH/mm of bondwire and several volts per nH at CDM (46032–46042).
5. CMOS inputs: GGNMOS or GCNMOS primary plus failsafe GGNMOS CDM clamps near the input devices. Use BTNMOS if the source can sustain snapback (46043–46056).
6. BiCMOS inputs: a V_ECS primary (V_t 7–12 V) for about 5 V pins, needing no ECGR. The CDM clamp must handle the V_ECS peak (46057–46063).
7. Large MOS outputs self-protect using the ESD device's layout (silicide block), plus an ECGR and ample substrate contacts (46064–46072).
8. Large BJT collector-base junctions self-protect. NPN emitters on pins need APD (46073–46077).
9. CMOS supply pins: GCNMOS if V_pk < BV_DSS of the protected devices and V_h > max V_op, otherwise an active FET with a disable circuit tied to the rail (46078–46087).
10. High-voltage pins: PNP is the best general choice, then power Zener (possible rate-firing), V_CES or V_CER if the current cannot sustain snapback (46088–46098).
11. Stacking: each of two stacked devices is about 2× size, so the stack is about 4× (46099–46105).
12. Last resort: active FET, possibly from LDMOS (46106–46118).

### 14.5 Summary (46124–46133, PDF p.777)
- Mergers, guard rings and tunnels get scant coverage in the literature. The ESD monographs target device specialists rather than layout designers.

### Selected Bibliography (46138–46159, PDF p.777)
- Davis 1981 (tunnels), Iorga 2008 (noise coupling), Semenov et al. 2008 (advanced-CMOS ESD), Vashchenko and Shibkov 2010 (analog/power ESD).

### 14.6 Exercises (46165–46302, PDF pp.778–780)
- 14.1 merged-tank area ratio. 14.2/14.3 merger risks and mitigations. 14.4/14.5 Darlington with field plates. 14.6 totem-pole driver. 14.7 latchup-proofing an op amp without deep-N+/NBL. 14.8 single-metal flip-flop with well and substrate contacts. 14.9/14.10 pad ring and bondpads. 14.11 APD. 14.12 10-section GGNMOS. 14.13 CDM structure inside a grounded ECGR. 14.14 V_ECS clamp with an encircling deep-N+ HBGR. 14.15 ESD plan for an 8-pin comparator.
- These confirm the chapter's automation targets: pad ring, ESD cells, rings and ESD pin planning.

---

## 3. Actionable extraction

Philis evidence below comes from a quick grep only (this is not an audit). Key facts used repeatedly:
- The SPICE front end skips `.subckt` directives and so drops the port list (`frontend/library/src/parse.rs:36-37`). `Net` holds only a name (`kernel/core/src/netlist.rs:35-37`).
- Net roles are name-based: Supply, Ground, Clock, Signal (`backend/annotator/src/netrole.rs:11-16,28-46`).
- Guard rings are requested for every MOS in a matched block and tied to its bulk (`backend/annotator/src/constraints.rs:66-84`). They are drawn as that bulk's tap: `Ecgr` is a p+ ring for NMOS, `Hcgr` an n+ ring in n-well for PMOS (`kernel/cells/src/post_cell.rs:310-322`).
- The substrate isolation rule treats only Clock-class devices as aggressors (`backend/annotator/src/emit.rs:266-287`).

### H14-01 Port-aware netlist and injector classification
- Kind: data-model
- Statement: a diffusion on a package pin that is neither a supply nor substrate ground can be driven above supply or below ground and inject minority carriers. This also holds through deposited resistors below about 50 kΩ. Capacitor-coupled diffusions (charge pumps) and devices with deliberately forward-biased backgates are injectors too.
- Source: §14.2 (43629–43638, PDF p.735); §14.1.4 item 1 (43526–43547, PDF p.733); §14.1 (42972–42974).
- Philis stage: flow (parse), annotator.
- Automation recipe:
  - Keep `.subckt` ports as `Net.is_port`, and let the user supply pin attributes (`pin_class ∈ {supply, substrate_gnd, power_gnd, input_gate, output, hv}`).
  - Mark a device as an injector if any S/D, collector, emitter, diode or resistor-body terminal is on a non-rail port, or connects to one through series R < 50 kΩ (sum of resistor values on the path).
  - Also mark it if a capacitor's other plate carries a large-swing node (op-point swing or charge-pump pattern), or its bulk is not tied to a rail or its own source.
  - Output: `injector: Vec<bool>` and `injector_reason`.
- Beats hand layout because: every pin path is enumerated, including resistor-coupled ones below 50 kΩ that designers miss. Hastings notes these devices "are not always easily identified" (43544–43545).
- Philis status: **missing**. Ports are dropped (`parse.rs:36-37`) and `Net` has no port flag (`netlist.rs:35-37`).

### H14-02 Per-device merge attributes
- Kind: data-model
- Statement: whether devices may share a well, tank or backgate depends on three properties: minority-carrier injection, debias of the shared region by current through it, and noise (noisy vs sensitive).
- Source: §14.1.5 questions 1–3 (43595–43612, PDF p.734); §14.1 (42957–42964).
- Philis stage: annotator.
- Automation recipe: per device, emit `injector` (H14-01), `noisy` (clock or fast-slew net per H14-08, switching output), `sensitive` (matched block or Sensitive net class) and `i_well_ua` (expected current through the shared region: substrate or backgate current from the op point, or the 100 mA latchup test current for pin devices). Store these on `Problem` for cells, dp and verify.
- Beats hand layout because: the attributes are computed for every device instead of being remembered by a designer.
- Philis status: **partial**. `sensitive` exists (`backend/annotator/src/block.rs:60-62`, `lib.rs:121-126`). Noisy is Clock-only (`emit.rs:270-273`). There is no injector flag and no well-current field.

### H14-03 Merge-legality algorithm (shared well or backgate)
- Kind: algorithm
- Statement: a set S may share a well if and only if all four hold:
  1. All devices in S have the same bulk net.
  2. No device in S is an injector, unless every other member tolerates cross-injection (Hastings: saturating BJTs, forward-biased Schottkys and pin diffusions should be avoided).
  3. The worst debias V = I_well·R_share is below the forward-bias margin (H14-05).
  4. S does not mix noisy and sensitive devices.
- Source: §14.1.5 (43595–43612); §14.1.4 items 1–2 (43526–43555); Capacitive Coupling (43153–43157).
- Philis stage: cells (`well_bridges`), cells (guard-ring clustering), dp (as a legality predicate).
- Automation recipe: before `well_bridges` joins two PMOS wells, and before ring clustering merges two rings, evaluate the predicate on the union. Rejected pairs get a spacing floor of 2 × well spacing so the placer does not abut them.
- Beats hand layout because: the check is exhaustive and applies to every pair the annealer tries.
- Philis status: **partial**. `well_bridges` merges any neighbouring PMOS on the same bulk (`kernel/cells/src/post_cell.rs:376-387`) and ring clustering merges same-net, same-type rings (`post_cell.rs:79-93`). Neither checks injector, noise or debias.

### H14-04 Area value of merging
- Kind: metric, heuristic
- Statement: lateral diffusion is about 0.8 × junction depth, so two 10 µm diffusions sit ≥16 µm apart, or about 20 µm with depletion. Three NPNs in one tank use about 70% of separate tanks. Three PMOS in one well with a shared gate use less than 50%.
- Source: §14.1 (42912–42942, PDF p.723).
- Philis stage: dp (cost), cells.
- Automation recipe: in dp, reward abutting legal same-bulk devices (H14-03) with −k·(well edges removed). Count well figures and total well perimeter as a reported metric.
- Beats hand layout because: the placer can evaluate every legal merge instead of the few a designer tries.
- Philis status: **partial**. Well bridges exist after placement (`post_cell.rs:376-387`). There is no dp cost term for well sharing (LAYOUT-FUNDAMENTALS K7 notes "no floorplan-level row or well sharing between cells").

### H14-05 Shared-region debias check
- Kind: formula, check
- Statement: V_debias = I·R_share, where R_share = R_s·(squares) for metal or diffusion, or the vertical and lateral well/tank resistance.
  - Metal: 1000 □ × 50 mΩ/□ × 100 µA = 0.5 mV.
  - Shared base: 2 µA × 500 Ω = 1 mV.
  - Pin injection: 100 mA × 5 Ω sinker = 0.5 V.
  - Latch trigger: about 300 mV at 150 °C. At 100 µA that is 3 kΩ.
- Source: Majority Carrier Debiasing (42990–43010, PDF p.724); §14.1.1 Fig 14.6 (43295–43299, PDF p.729).
- Philis stage: verify, annotator (budget).
- Automation recipe: for each merged well and each ring, estimate R_share from well sheet resistance (deck `nwell_rs`) × distance from the injecting diffusion to the nearest tap. Flag V_debias ≥ 0.3 V for pin-injector test current (100 mA), or ≥ a user matching budget for matched sets. The output is a Θ residual.
- Beats hand layout because: the numbers are computed per well rather than judged by eye.
- Philis status: **partial**. Metal IR drop exists (`kernel/analog/src/routing/ir.rs:19-63`). There is no well or substrate debias model, and the deck has no well sheet resistance key (not checked in detail).

### H14-06 No resistive shared region inside a matched set's critical path
- Kind: rule
- Statement: matched devices may share a region whose voltage barely affects the matched parameter (NPN collectors, tank), but not one that directly sets it (a shared base sets V_BE). In MOS terms, matched devices may share a well, but a shared well with current flowing through it shifts V_T by body effect.
- Source: Majority Carrier Debiasing (43014–43018, PDF p.724).
- Philis stage: annotator, cells.
- Automation recipe: for matched sets whose bulk ≠ source (body effect active), require equal tap distance per unit and forbid other current-carrying devices in the same well (H14-03 predicate with sensitivity = body-effect coefficient).
- Beats hand layout because: the rule is applied to every matched set, including ones where body effect is easy to overlook.
- Philis status: **missing** (LAYOUT-FUNDAMENTALS #17 lists "equal tap distance per unit" as partial).

### H14-07 Crossing-capacitance spike check (Eq 14.1)
- Kind: formula, check
- Statement: V_P = R·S·C_P. An M1×M2 right-angle crossing (1 µm × 1 µm, 10 kÅ ILD) gives C_P ≈ 0.05 fF. A 1 V/ns aggressor into a 200 kΩ victim gives 50 mV. Rule: never route switching digital over sensitive analog.
- Source: Capacitive Coupling Eq 14.1 (43023–43042, PDF pp.723–724).
- Philis stage: gr (keep-out), verify (check), dr.
- Automation recipe:
  - For each (victim, aggressor) pair, compute Σ over inter-layer overlaps of C_ov (area × deck plate cap) plus lateral C.
  - Take R_victim from the op point (node impedance ≈ 1/g_out) or a budget default, and S from the aggressor's slew (clock period or op point).
  - Require V_P ≤ fraction × victim signal amplitude.
  - In gr, add over-device keep-out cost for noisy nets over sensitive devices (not only foreign cells).
- Beats hand layout because: every crossing is computed. Hastings' own case was found only by back-annotation (43046).
- Philis status: **partial**. Lateral same-layer coupling only (`kernel/analog/src/routing/coupling.rs:20-22`: "Only shapes separated on one axis and overlapping on the other ... couple"). Crosstalk is same-layer only (`crosstalk.rs:10-11`). The gr keep-out is class-blind (`backend/gr/src/lib.rs:749-757`). Crossing (overlap) capacitance is **missing**.

### H14-08 Slew-based aggressor classification
- Kind: heuristic
- Statement: large-swing, fast-slewing analog nodes are aggressors too. Example: a charge-pump node swinging more than 20 V couples through about 1 fF into a µA mirror and mimics subharmonic oscillation.
- Source: Capacitive Coupling (43042–43046, PDF p.725).
- Philis stage: annotator.
- Automation recipe: a net is noisy if its NetClass is Clock, or it belongs to a matched `charge_pump_cell` pattern, or its transient or op-point swing × frequency exceeds a threshold (user-supplied swing/slew, or the pattern default). Feed these nets to H14-07 and to the `Isolation` emit.
- Beats hand layout because: every net is classified by its behaviour, not only nets whose names look like clocks.
- Philis status: **partial**. Aggressors are Clock-class only (`backend/annotator/src/emit.rs:270-273`). The charge-pump pattern exists (`catalog.rs:1017-1023`) but is not used as an aggressor source.

### H14-09 Substrate-contact ring around the injector first, at minimum width
- Kind: rule
- Statement: substrate contacts around a noise injector attenuate more than contacts around the victim. Widening the ring helps only modestly, so use minimum width. Rings around victims are an addition.
- Source: Capacitive Coupling (43072–43086, PDF p.725).
- Philis stage: annotator, cells.
- Automation recipe: emit a `GuardRingRequirement` (majority, substrate tap) for each noisy or injector device with `min_width` = deck minimum. Victim rings stay optional.
- Beats hand layout because: every injector is ringed, not only the ones the designer noticed.
- Philis status: **missing** for injectors. Rings are requested only for matched-block FETs (`backend/annotator/src/constraints.rs:66-84`).

### H14-10 Separate return paths for injector and victim rings
- Kind: rule
- Statement: the grounds of the injector ring and the victim ring return by separate leads to the ultimate ground: separate pins ideally, otherwise separate bondwires to one pin.
- Source: Capacitive Coupling (43086–43093, PDF p.725); §14.4.3 Fig 14.40 GND/PGND (45791–45804).
- Philis stage: annotator (net split), gr/dr (star routing), verify.
- Automation recipe: give ring nets distinct logical sub-nets (`gnd_noisy_ring`, `gnd_quiet_ring`) that may join only at a declared star point or port. Check that the routed tree shares no segment between them.
- Beats hand layout because: shared segments are detected exhaustively from the routed tree.
- Philis status: **missing** (LAYOUT-FUNDAMENTALS #59 is marked M: "`max_ring_resistance_mohm` sizes the contact rows only").

### H14-11 N-type noise-blocking guard ring
- Kind: rule, generator
- Statement: an N-type ring (NSD, or deeper N-well, which is more effective but larger) outside the substrate-contact ring of an injector, tied to a clean supply or ground. It partially blocks the conductive path and extracts some capacitive noise. It does not replace substrate contacts.
- Source: Capacitive Coupling Fig 14.3 (43096–43119, PDF pp.725–726).
- Philis stage: cells (post_cell ring type), annotator.
- Automation recipe: add `GuardRingType::NoiseBlockNwell`, drawn as an nwell band with an n+ tap and li/met1 pin, concentric outside the majority ring (gap = deck nwell-to-psdm spacing). Tie it to a quiet supply. Emit it for noisy devices when a sensitive block is within N·epi.
- Beats hand layout because: the ring is added wherever the proximity condition holds, not just where remembered.
- Philis status: **missing**.

### H14-12 Deep isolation for sensitive blocks next to switchers
- Kind: rule
- Statement: isolating sensitive MOS from the substrate with buried layer plus sinker (in CMOS terms, DNW for NMOS) with separate ground pins gives very high attenuation. Hastings' audio example heard no noise (attenuation value not legible). It is less effective at GHz because of bondwire and pin inductance. An NMOS in a P-tank cut off by NBL and deep-N+ is a 100% ECGR.
- Source: Capacitive Coupling (43138–43145, PDF p.726); §14.2.5 (43915–43933, PDF pp.738–739).
- Philis stage: cells (DNW tub generator), annotator.
- Automation recipe: when the deck has `dnwell` and a sensitive NMOS block lies within the isolation distance of an aggressor, emit `IsolatedTub{devices, tie_net = quiet supply}`. Cells draws the DNW + nwell ring around the block (sky130 rule set). Its ground goes on a separate return (H14-10).
- Beats hand layout because: the tub decision follows from computed aggressor distances rather than judgement.
- Philis status: **missing** for MOS. DNW is used only for NPN isolation (`kernel/cells/src/bjt.rs:34,129-138`).

### H14-13 Noisy/sensitive merge compatibility
- Kind: rule
- Statement: sensitive may merge with sensitive, and noisy with insensitive (for example digital PMOS in one well). Noisy with sensitive is forbidden. If unavoidable, add plentiful majority-carrier contacts. NMOS in a P-epi tank has high backgate R and couples even when fully ringed.
- Source: Capacitive Coupling (43153–43157, PDF p.726); §14.1.5 Q3 (43607–43612).
- Philis stage: cells, dp.
- Automation recipe: this is clause 4 of the H14-03 predicate. When it forces a merge, raise the tap density (H14-24).
- Beats hand layout because: every candidate merge is checked against the matrix.
- Philis status: **missing**.

### H14-14 SCR loop-gain criterion (Eq 14.2)
- Kind: formula, check
- Statement: latchup occurs if β_N·β_P·(1 − η_C) > 1, where η_C is the fraction collected or blocked by an intervening bar or ring. The required η_C > 1 − 1/(β_N·β_P). With β_N = 300 and β_P = 10 that is > 0.997. A bar cannot guarantee this, so an NPN must not drive a merged LPNP that could saturate. The same holds for BiCMOS NPN + LPNP in one N-well.
- Source: §14.1.1 Fig 14.5, Eq 14.2 (43241–43257, PDF p.728).
- Philis stage: verify, annotator.
- Automation recipe: for every PNPN path found in the merged-well graph (a P-diffusion in an N-region adjacent to an N-diffusion in a P-region, joined by nets), estimate β from deck parasitic-BJT data (not given here) and the ring efficiency by ring type (H14-27). Report loop gain.
- Beats hand layout because: every PNPN path in the merged-well graph is enumerated rather than spotted by inspection.
- Philis status: **missing** (no BJT mergers and no latchup model; CRATES open issue #5 "LU.2").

### H14-15 Merged lateral PNPs
- Kind: rule
- Statement:
  - Saturating LPNPs go in their own tank.
  - Unsaturated leakage is < 1% but grows as V_BC → 0.
  - A saturating collector in a two-LPNP tank sends about ¼ of its current across.
  - A symmetric array with equal I_E and V_BC cancels cross-injection, so matched LPNPs can share.
  - A split-collector device whose segment may saturate must be split.
- Source: §14.1.1 (43181–43209); §14.1.4 items 3–4 (43556–43573, PDF pp.727–734).
- Philis stage: annotator (bjt), cells (bjt).
- Automation recipe: for BJT blocks, share a tank only if no member saturates (op point V_CE > V_CE(sat) margin) or the arrangement is symmetric with equal op-point currents.
- Beats hand layout because: the saturation check is run on op-point data for every member.
- Philis status: **missing** (the BJT generator draws per-device isolation, `bjt.rs:34`).

### H14-16 Resistor sharing a tank or well contact with a transistor (Fig 14.6 loop)
- Kind: check
- Statement: a resistor whose high end sits at the well-contact potential, sharing that contact with a transistor that draws current through the well, can forward-bias into the well once the debias exceeds about 300 mV at 150 °C. That is a PNPN loop. A low-R well tie (deep-N+, ≤ 100 Ω) removes the risk.
- Source: §14.1.1 (43283–43299, PDF p.729).
- Philis stage: verify, cells.
- Automation recipe: when a well-resistor (or P-diffusion resistor in an n-well) shares a well with another device, compute I_other × R_tie and require < 0.3 V. Otherwise use separate wells or more ties.
- Beats hand layout because: the product is computed for every shared well.
- Philis status: **missing**.

### H14-17 Contacts fully enclosed by heavily doped diffusion
- Kind: check, deck-requirement
- Statement: a contact touching lightly doped N-epi or N-well outside its N+ forms a Schottky injector (about 0.5% minority current, enough for latch if β > 200).
- Source: §14.1.1 (43326–43337, PDF p.730).
- Philis stage: deck, verify.
- Automation recipe: DRC enclosure of every well-tie contact by n+ implant and diffusion. This is standard deck enclosure; confirm it exists for tap contacts in all four decks.
- Beats hand layout because: the check is mechanical over every contact.
- Philis status: **partial**. Relies on deck enclosure rules read by `verify` (`backend/verify/src/pdk.rs`), not checked in detail.

### H14-18 Default-allowed (low-risk) mergers
- Kind: rule
- Statement: always merge sections of one matched device, fingers of a power device, a sense device with its power device, back-to-back common-drain LDMOS, Schottky-clamped NPNs, Darlingtons and base turnoff resistors. For matched merged sections, watch well modulation and keep emitters or fingers apart enough.
- Source: §14.1.3 (43445–43517, PDF pp.732–733).
- Philis stage: cells, annotator.
- Automation recipe: whitelist these patterns in the H14-03 predicate: same device id, or a known pattern from `catalog.rs`.
- Beats hand layout because: the whitelist is applied consistently.
- Philis status: **partial**. Unitized devices share a cell (`constraints.rs:37-64`). There is no power/sense or LDMOS handling.

### H14-19 Power-device dispersal for peak temperature
- Kind: heuristic
- Statement: a power device dissipating significant power can be split into banks with other circuitry between them, or interleaved with sinker strips, to lower peak temperature.
- Source: §14.1.3 item 2 (43459–43468, PDF p.732).
- Philis stage: gp/dp (thermal), cells.
- Automation recipe: for devices with op-point power above a threshold, allow the unitizer to split into k banks. Evaluate peak T with the existing thermal model and choose k by cost.
- Beats hand layout because: candidate bank counts are scored by the thermal model instead of chosen by feel.
- Philis status: **partial**. A thermal rule exists (`kernel/analog/src/placement/thermal.rs`). There is no bank splitting.

### H14-20 Sense-device placement in a power device
- Kind: algorithm
- Statement: best is two equal sense sections on an axis of symmetry of the power device, each about halfway between centre and periphery, so each sits at the device's mean temperature. Next best is one sense device embedded between two halves (worse, because the centre is hottest). Last is adjacent to the power device on a symmetry axis.
- Source: §14.1.3 item 3 (43469–43481, PDF p.732).
- Philis stage: annotator (detect sense/power ratio pair), dp/gp (constraint), cells.
- Automation recipe:
  - Detect a mirror pair with ratio ≥ 10 and high power on the big device.
  - Compute the power device's temperature map (thermal.rs) and its mean T̄.
  - Place the sense halves on the symmetry axis at the contour T = T̄. The residual is |T_sense − T̄|.
- Beats hand layout because: the isotherm is computed rather than estimated as "halfway".
- Philis status: **missing** (no sense or power pattern found in `catalog.rs`).

### H14-21 Well bias must dominate enclosed P-regions
- Kind: check
- Statement: emitter-base Zeners (and any P-diffusion) may share a tank only if the tank voltage is ≥ the Zener cathode (or P-region) voltage at all times. Series Zener strings follow the same rule at the cathode end.
- Source: §14.1.4 item 5 (43574–43578, PDF p.734).
- Philis stage: verify (op point), annotator.
- Automation recipe: for each shared n-well, V_well(op) ≥ max V(P-diffusions inside) + margin. Violations split the well.
- Beats hand layout because: op-point voltages are checked for every enclosed diffusion.
- Philis status: **missing**.

### H14-22 Output transistors in their own backgate region
- Kind: rule
- Statement: an MOS whose S/D is on a non-supply, non-substrate pin should sit in its own backgate region. If merged, add minority-carrier rings and low-R backgate contacts on nearby complementary devices, because electrons from an output NMOS can debias an adjacent PMOS well and trigger CMOS latchup.
- Source: §14.1.4 item 1 (43526–43538, PDF p.733).
- Philis stage: annotator, cells.
- Automation recipe: an H14-01 injector with kind MOS gets `own_well = true`, a `GuardRingRequirement{ring_type = ECGR (true, N-type), tie = highest supply}` (H14-28), and a spacing floor to opposite-type wells.
- Beats hand layout because: every output device is found from the port list and treated the same way.
- Philis status: **missing**.

### H14-23 Deliberately forward-biased backgates
- Kind: rule
- Statement: charge-pump devices and certain clamps with forward-biased backgates must be marked, given their own backgate region (or merged only with tolerant devices), and ringed if the current is large.
- Source: §14.1.4 item 1 (43543–43547, PDF p.733); §14.2 (43634–43638).
- Philis stage: annotator.
- Automation recipe: a device is flagged when its bulk net is neither a rail nor its own source and op-point V_SB (V_BS) exceeds about −0.3 V (forward), or when it matches a charge-pump pattern. It is then an injector (H14-01).
- Beats hand layout because: the op-point test finds devices that are "not always easily identified" (43544–43545).
- Philis status: **missing**.

### H14-24 Low-resistance, scattered backgate contacts
- Kind: rule
- Statement: merged MOS need low-R backgate ties, scattered through the structure when there is no low-R sublayer. Large MOS can use integrated backgate contacts. SOI or DTI tanks holding both PMOS and NMOS need many ties in both wells.
- Source: §14.1.4 item 1 (43539–43542, PDF p.733); §14.2.5 (43945–43950, PDF p.740).
- Philis stage: cells (mosfet tap strips), verify (latchup distance).
- Automation recipe: require max distance from any diffusion to a same-type tap ≤ LU_max (deck), plus `tap_pitch` inside wide devices. Add integrated (butted) taps as a mosfet variant.
- Beats hand layout because: the tap distance is checked for every diffusion point, globally.
- Philis status: **partial**. Per-cell tap strip and ring `tap_pitch_nm: 2_000` (`constraints.rs:79`). No global LU distance check (LAYOUT-FUNDAMENTALS #60; CRATES "LU.2").

### H14-25 Diffused and well resistors in common wells
- Kind: rule
- Statement: resistors merged in a common tank or well must never forward-bias into it. Pin-connected resistors get their own tank and possibly rings. Resistors a circuit may forward-bias must be identified and isolated. Noisy resistors must not share with sensitive ones.
- Source: §14.1.4 item 2 (43548–43555, PDF p.733).
- Philis stage: annotator, cells (resistor.rs).
- Automation recipe: apply the H14-03 predicate to resistor bodies in wells. Check the op-point terminal voltages against the well tie (H14-21).
- Beats hand layout because: every merged resistor's terminals are checked against the tie voltage.
- Philis status: **missing** (not checked in detail).

### H14-26 Ring the injectors, and share one periphery ring for ESD and pads
- Kind: algorithm
- Statement: enclose each minority-carrier injector with a suitable minority-carrier guard ring, since injectors are fewer than victims. Peripheral ESD devices share one ring separating the core from ESD devices and bondpads.
- Source: §14.2 (43641–43645, PDF p.735).
- Philis stage: annotator, cells, flow (top-level floorplan).
- Automation recipe: emit ring requirements for H14-01 injectors (ECGR or HCGR by polarity per H14-28/29) instead of, or in addition to, victim rings. At top level, draw one ECGR band between the pad/ESD ring and the core.
- Beats hand layout because: ring coverage follows from the injector list, so none is skipped.
- Philis status: **missing**. Rings are only on matched-block FETs (`constraints.rs:66-84`).

### H14-27 Guard-ring taxonomy: majority vs minority
- Kind: data-model
- Statement: rings of substrate or backgate contacts (P+ around NMOS, N+ around PMOS) are majority-carrier guard rings. They pin the backgate and supply recombination carriers but do not block or collect minority carriers. Minority-carrier rings are opposite-type diffusions: an ECGR is N-type in P and collects electrons; an HCGR is P-type in N and collects holes.
- Source: §14.2.3 (43795–43799, PDF p.737); §14.2.4 (43871–43873, PDF p.739).
- Philis stage: kernel/analog (types), cells, annotator.
- Automation recipe: rename what Philis draws today to `MajorityTap{polarity}`. Keep `Ecgr`/`Hcgr`/`Ebgr`/`Hbgr` for true minority rings, which are drawn with opposite-type diffusion (N+ ring + nwell in P-sub for ECGR; P+ ring in nwell for HCGR) and tied to a supply or ground per H14-28/29, not to the device bulk.
- Beats hand layout because: the ring's purpose is encoded in its type and cannot be confused by naming.
- Philis status: **mislabelled**. `Ecgr` is drawn as a p+ substrate tap tied to NMOS bulk and `Hcgr` as an n+ tap in the PMOS well (`kernel/cells/src/post_cell.rs:310-322`; `backend/annotator/src/constraints.rs:67-71,83`). By Hastings these are majority-carrier rings. The type doc in `kernel/analog/src/cell.rs:26-34` says "Electron-collecting (n+ in p-sub)", which contradicts the drawing.

### H14-28 CMOS ECGR construction
- Kind: rule, generator
- Statement:
  - NMoat ECGR: width ≥ P-epi thickness (non-retrograde) or ≥ P-well depth (retrograde). Low CMOS supplies cannot deepen the depletion region, so width is the only lever.
  - N-well ECGR: as much NMoat contact as possible, tied to a supply, not ground. A deep lightly doped N-well is of doubtful use.
  - If the ring must be grounded, prefer a wide NMoat ECGR over an N-well ECGR.
  - Place the ring as near the injector as possible.
- Source: §14.2.3 (43803–43837, PDF pp.737–738); §14.2.1 (43687–43689).
- Philis stage: cells (post_cell), deck.
- Automation recipe: deck keys `p_epi_thickness_nm` and `pwell_depth_nm`, then `ecgr_width = max(min_width, depth)`. The ring is an nsdm tap band in nwell, tied to the highest supply by default. The ring gap is the minimum legal (ECGR "immediately adjacent to the source").
- Beats hand layout because: the ring width is derived from deck depth for every ring.
- Philis status: **missing** (see H14-27). The deck key `p_epi_thickness` exists but is a ring-depth default (`backend/annotator/src/emit.rs:250-253`).

### H14-29 CMOS HCGR construction
- Kind: rule, generator
- Statement: useful only with a retrograde N-well whose bottom doping is ≥ 10× the middle. The HCGR is a PMoat ring in the N-well around the injector, grounded, with width ≥ N-well depth. Pure CMOS cannot build an HBGR.
- Source: §14.2.4 (43842–43855, PDF p.738).
- Philis stage: cells, deck.
- Automation recipe: deck flag `nwell_retrograde: bool` and `nwell_depth_nm`. Generate the HCGR (psdm tap band inside the injector's nwell, tied to ground) only when the flag is true. Otherwise emit a majority N+ tap ring plus spacing.
- Beats hand layout because: the ring is gated on process data rather than habit.
- Philis status: **missing**.

### H14-30 ECGR bias, double ring and debias check
- Kind: rule, check
- Statement: tie an ECGR to the highest supply, which gives a deeper depletion region and less debias. A grounded ECGR that might saturate is enclosed by a second, supply-tied ECGR. Without a low-R path to the collecting region, compute the vertical R and check that the supply headroom covers I_collect·R.
- Source: §14.2.1 (43653–43658, PDF p.735); §14.2.5 (43905–43907, PDF p.739).
- Philis stage: annotator (tie net), verify.
- Automation recipe: tie net = highest-voltage Supply-class net. Check I_latchup (100–200 mA) × R_ring_vertical < V_supply − margin.
- Beats hand layout because: the tie choice and the headroom check are automatic for every ring.
- Philis status: **missing**.

### H14-31 Supply-tied wells as free ECGRs
- Kind: heuristic (placement)
- Statement: supply-tied N-regions placed between an injector and sensitive circuitry are very effective electron collectors.
- Source: §14.2.1 (43680–43689, PDF pp.735–736).
- Philis stage: gp/dp.
- Automation recipe: in the placement cost, reward PMOS wells tied to VDD that lie on the straight segment between an injector and a sensitive device (the ray intersects the well bbox). Credit this in the `Isolation` residual.
- Beats hand layout because: the placer can test many arrangements for this geometric condition.
- Philis status: **missing**.

### H14-32 Isolated NMOS as a 100% ECGR, and P− substrate caveat
- Kind: rule
- Statement: an NMOS in a P-tank floored by NBL and walled by deep-N+ or DTI is enclosed by a 100%-efficient ECGR. An NBL + deep-N+ ring alone exceeds 90% efficiency. P− substrates are far more latchup-prone and ECGRs weaken there. "No amount of guard rings and substrate contacts will guarantee" survival under severe kickback.
- Source: §14.2.5 (43897–43943, PDF pp.738–740).
- Philis stage: cells (DNW tub), deck.
- Automation recipe: prefer a DNW tub (sky130 `dnwell`) for injector NMOS or victim NMOS when the deck has it (H14-12). Deck key `substrate: p_minus | p_plus` sets the ring efficiency used in H14-14.
- Beats hand layout because: tub selection and efficiency accounting follow from deck data.
- Philis status: **missing** for MOS (DNW is only in `bjt.rs`).

### H14-33 HBGR drawing rules (buried-layer processes)
- Kind: deck-requirement, generator
- Statement: an HBGR must fully encircle the injector. Drawn NBL extends to the outer edge of drawn deep-N+ (and of the trench for DTI). Drawn N-well extends at least to the outer edge of deep-N+. The N+/N− doping ratio should be ≥ 100:1, and the deep-N+ core > 10²⁰ cm⁻³ when well surface doping is > 10¹⁸. Efficiency > 95–98%, and > 99% with an HCGR inside.
- Source: §14.2.2 (43753–43763, PDF p.737); §14.2.6 (43973–44033, PDF p.741).
- Philis stage: cells, deck.
- Automation recipe: ring generator enclosure constraints: `enc(nbl, sinker_outer) ≥ 0` and `enc(nwell, sinker_outer) ≥ 0`. Deck fields for doping ratio are informational only.
- Beats hand layout because: the enclosure constraints are enforced by the generator every time.
- Philis status: **missing** (decks are CMOS; relevant if a BiCMOS deck is added).

### H14-34 Ring current capacity; merging a ring with substrate taps
- Kind: rule
- Statement: rings expected to collect large currents must not be merged with isolation or substrate ties, because debias occurs even with contacts in the ring. Rings collecting ≤ a few mA may merge if they contain substrate contacts. Pin P-regions exposed to about 100 mA get their own tank with nearby substrate contacts.
- Source: §14.2.2 (43729–43733, 43776–43778, PDF pp.736–737).
- Philis stage: cells (ring sizing), verify.
- Automation recipe: size ring contacts and width from I_collect (the injector's test current, default 100 mA) against max debias (0.3 V) rather than a fixed `max_ring_resistance_mohm`. Forbid ring-with-substrate-tap merging when I_collect > a few mA.
- Beats hand layout because: ring sizing follows from the expected current for every ring.
- Philis status: **partial**. Rings are sized by contact count against a fixed 100 Ω (`constraints.rs:81`; `post_cell.rs:26`).

### H14-35 Net-class annotations for routing (Table 14.1)
- Kind: data-model, rule
- Statement:
  - Power: no tunnels, width ≥ spec.
  - Noisy: do not cross sensitive leads or the devices they connect to.
  - Sensitive: no tunnels, no substrate contacts in sensitive ground leads.
  - Noncritical: none.
  - Also list matched components and devices needing guard rings.
- Source: §14.3.2 Table 14.1 (44192–44228, PDF p.745).
- Philis stage: annotator (NetClass), gr/dr.
- Automation recipe: map NetClass to router policy.
  - Supply/Ground: min width from EM/IR, no poly or diffusion jumpers.
  - Clock/noisy: keep-out over sensitive devices and nets (H14-07).
  - Sensitive: no jumpers, and no ring or tap may connect into a sensitive ground net (substrate taps must not feed the sensitive ground).
- Beats hand layout because: the policy is applied to every net by class instead of relying on colour-coded schematics.
- Philis status: **partial**. NetClass has Sensitive, Clock, Supply, Ground and Substrate (`kernel/analog/src/metadata.rs:18-26`). The router keep-out is not class-aware (`backend/gr/src/lib.rs:749-757`).

### H14-36 Routing over resistors
- Kind: rule
- Statement: leads may cross resistors except field-plated resistors, resistors carrying sensitive signals (noise coupling) and lightly doped high-sheet resistors (voltage modulation, for example 2 kΩ/□ HSR).
- Source: §14.3.2 item 1 (44148–44153, PDF p.744).
- Philis stage: gr/dr.
- Automation recipe: over-device keep-out for high-sheet resistor cells (deck `rpm`/`urpm` or R_s above a threshold) for all nets. Over sensitive resistors, keep out only noisy nets. Low-R_s resistors are allowed.
- Beats hand layout because: the keep-outs are derived from resistor type and net class for every crossing.
- Philis status: **missing** (keep-out is class-blind, `gr/src/lib.rs:749-757`).

### H14-37 Terminal-order variants to remove crossings
- Kind: heuristic
- Statement: changing the terminal order (CEB vs CBE, and by analogy S/D/G pin sides on a MOS) often removes crossings at a small parasitic cost.
- Source: §14.3.2 item 2 (44154–44158, PDF p.744).
- Philis stage: cells (variants), dp.
- Automation recipe: enumerate pin-side variants per cell. Let dp or gr score each variant by crossing count and wirelength and keep the best. This is already the shape of the Philis variant loop.
- Beats hand layout because: all variants are scored rather than a few tried.
- Philis status: **partial**. The variant choice exists (`frontend/library/src/lib.rs:396`).

### H14-38 Stretch matched devices together
- Kind: rule
- Statement: if one member of a matched group is stretched to pass a lead, stretch all of them the same way.
- Source: §14.3.2 item 3 (44159–44164, PDF p.744).
- Philis stage: cells.
- Automation recipe: `same_variant_required` applies to stretched variants as well.
- Beats hand layout because: the constraint is enforced mechanically.
- Philis status: **implemented** (`backend/annotator/src/constraints.rs:60`, `same_variant_required: true`).

### H14-39 Non-metal jumpers (tunnels) need approval and back-annotation
- Kind: rule
- Statement: tunnels add area, R and C (a base tunnel is a few hundred Ω and tens to hundreds of fF; emitter-in-iso is about 1.5 fF/µm²; a stacked tunnel is 3–5 Ω/□). They must not carry high-current leads, must respect junction voltages, and must be approved and resimulated.
- Source: §14.3.2 item 5 (44176–44183); §14.3.3 (44233–44296, PDF pp.745–746).
- Philis stage: gr/dr, verify (PEX).
- Automation recipe: if the router ever uses poly or li as a crossunder, forbid it on Supply/Ground/Sensitive classes (H14-35) and include it in PEX and the op-point re-check.
- Beats hand layout because: every jumper is back-annotated and class-checked, not just the ones a reviewer approves.
- Philis status: **not checked** in detail. The router's layer set was not audited.

### H14-40 Mock-layout crossing-count metric
- Kind: metric
- Statement: the main single-level (or local single-layer) problem is arranging components to minimize crossings. Designers iterate mock layouts or paper dolls.
- Source: §14.3.1 (44076–44082, 44131–44138, PDF pp.743–744).
- Philis stage: dp (cost), gr.
- Automation recipe: for any region restricted to one routing layer (for example under a MIM cap keep-out), count flight-line crossings after placement and add a cost. This is the automated equivalent of paper dolls.
- Beats hand layout because: the annealer evaluates thousands of arrangements rather than a few sketches.
- Philis status: **missing** (no crossing metric found).

### H14-41 Common centroid for non-integer ratios, and ratioed pairs
- Kind: algorithm
- Statement:
  - For a ratio that is not a simple integer (R4/R3 = 6.441), do not unitize on the small device, which gives 7 segments with misaligned centroids or 8 partial segments leaving R3 at 3.88 □.
  - Instead take n equal segments of the large device (6 × 666.7 Ω) and one central partial segment for the small device, with a sliding contact for trim. Reject segments below a minimum square count.
  - A 6:1 pair splits the large device into halves either side of the small one.
- Source: §14.3.1 (44105–44120, PDF p.744).
- Philis stage: annotator (Unitization), cells.
- Automation recipe: when `target_ratio` is not integral within tolerance, try both unitizations and score centroid error and minimum segment squares. Keep the central-partial-segment option.
- Beats hand layout because: both unitizations are scored and the better one kept.
- Philis status: **partial**. Unitization is integer-ratio (`backend/annotator/src/constraints.rs:48-52`). Common-centroid exists (`kernel/analog/src/placement/cc.rs`).

### H14-42 ESD pin plan (per-pad requirement)
- Kind: data-model
- Statement:
  - Every pin except substrate ground needs a primary device; substrate ground relies on the ring width.
  - Pads joined by stout on-chip metal may share one device.
  - Separate pads bonded to one pin (Kelvin) each need their own device, because of about 1 nH/mm of bondwire and several volts per nH at CDM.
  - Separate grounds are joined by APD.
- Source: §14.4.3 guidelines 1–4 (46006–46042, PDF p.774).
- Philis stage: flow (top level), annotator.
- Automation recipe: from ports (H14-01) plus user pin classes, emit an `EsdPlan{pad → device type}`. Verify that each non-substrate pad has a primary device net-connected within ≤ R_max metal (H14-54).
- Beats hand layout because: every pad is covered by construction.
- Philis status: **missing**. Only an `esd_diode_clamp` recognition pattern exists (`backend/annotator/src/catalog.rs:1811-1815`).

### H14-43 ESD device acceptance criteria
- Kind: check
- Statement:
  - All devices: V_c (at 1 µA, sometimes 100 nA) > V_op,max; V_pk (at 1.3 A for 2 kV HBM) < junction BV and < oxide BV; I_f or I_t2 ≫ I_pk (from TLP).
  - Snapback also needs one of: V_h > V_op,max; I_available < I_h (with V_t1 > V_op); or V never exceeds V_t1 (discouraged).
  - Rate-fired also needs one of: pin slew ≤ 100 V/µs, or a disable circuit (preferred). Disable-type devices are not allowed on pins needing system-level ESD.
- Source: §14.4.1 (44451–44604, PDF pp.749–751).
- Philis stage: verify.
- Automation recipe: a table-driven check per pad using the device parameters from Table 14.2 (H14-45) and user pin specs (V_op,max, slew, available current).
- Beats hand layout because: every pad is checked against every criterion.
- Philis status: **missing**.

### H14-44 HBM/CDM constants and clamp arithmetic
- Kind: formula
- Statement:
  - HBM 2 kV: I_pk ≈ 1.3 A, rise about 15 ns, τ ≈ 225–230 ns.
  - CDM 500 V: > 5 A in about 0.25 ns, lasting 1–2 ns. Worked examples use 10–12 A.
  - Clamp: V = V_BV + I·R_s. Example: 7 V + 1.3 A × 2 Ω = 9.6 V, and at 12 A, 31 V.
  - Secondary: I = (V_primary − V_secondary)/R_series. Example: 44 mA through 500 Ω into a 50 Ω Zener gives 9.2 V.
- Source: §14.4 (44344–44421, PDF pp.747–748).
- Philis stage: verify, deck.
- Automation recipe: constants for H14-43, H14-53 and H14-55.
- Beats hand layout because: the same numbers are applied to every pad.
- Philis status: **missing**.

### H14-45 ESD device table (behaviour, voltages, area, ECGR)
- Kind: data-model
- Statement: Table 14.2 (see §2 digest). Every listed device except the V_ECS clamp needs an ECGR between it and the rest of the die. The floating-base LPNP still gets one because of a capacitive injection pulse. A PMOS C1 and the M2 disable device of a GCNMOS also sit behind the ring.
- Source: Table 14.2 (45426–45483, PDF p.765); per-device text 44661–45416.
- Philis stage: annotator, cells.
- Automation recipe: when an ESD pattern is recognized (or supplied as a macro), emit a ring requirement `ECGR(tie = highest supply)` enclosing the ESD cluster, and a spacing floor to core devices.
- Beats hand layout because: the ring follows automatically from the recognized device.
- Philis status: **missing**.

### H14-46 GGNMOS generator rules
- Kind: rule (cells)
- Statement:
  - L = minimum for the device style.
  - Drain NMoat overlap of contact 1–3 µm above minimum; silicide block pulled back 1–3 µm from the NMoat edge (silicided S/D otherwise fragile).
  - No LDD if optional.
  - Total W ≈ 600 µm for 2 kV HBM.
  - Multi-finger turn-on requires V_t2 > V_t1 (use ballast).
  - Characterize an array of variants.
- Source: GGNMOS (45027–45070, PDF p.759); thick-field NMOS layout notes (44928–44932).
- Philis stage: cells (a new ESD generator), deck (silicide-block layer, rpo/sab).
- Automation recipe: parametric generator (W_total, W_finger, overlap, SAB pullback) with values from the deck or user. Use fillets or chamfers on drain corners where the deck allows non-Manhattan geometry.
- Beats hand layout because: the generator applies the same checked parameters to every instance.
- Philis status: **missing**.

### H14-47 GCNMOS, BTNMOS and active-FET generator rules
- Kind: rule (cells)
- Statement:
  - GCNMOS: R1·C1 = 20–50 ns; disable R2·C2 ≥ 1 µs; C1 a PMOS gate cap; R1 a small poly resistor; V_GS kept low. It can protect same-style NMOS without series R.
  - Substrate-pumped BTNMOS: predrive M2 split into two halves either side of M1, with M2's source on a PMoat ring enclosing both.
  - Active FET: protection linear in W (1 kV → 2 kV by doubling W). Needs a disable circuit on slewing pins.
- Source: GCNMOS (45121–45163); BTNMOS (45195–45236); Active FET (45241–45266), PDF pp.760–762.
- Philis stage: cells, annotator.
- Automation recipe: macro templates. Symmetric M2 halves are a Symmetry constraint about M1's axis.
- Beats hand layout because: the template fixes the geometry and the symmetry constraint is enforced.
- Philis status: **missing**.

### H14-48 Diode ESD (APD and dual diodes) generator rules
- Kind: rule (cells)
- Statement: minimum-width NMoat/PMoat fingers at minimum spacing, in square or near-square arrays to limit metal R. Size from allowed series R (≤ a few Ω). D1 interdigitates cathode fingers with substrate contacts. The dual-diode D2 cathode goes to the protected supply pin through wide metal (ESD metal between pins ≤ 2 Ω). Needs an ECGR.
- Source: APD (44792–44823); Dual Diodes (44828–44881), PDF pp.755–756.
- Philis stage: cells (diode.rs array variant).
- Automation recipe: extend `kernel/cells/src/diode.rs` with an interdigitated-array variant. Choose finger count and length to minimize R_series + R_metal subject to aspect ≈ 1.
- Beats hand layout because: finger count and length are optimized numerically.
- Philis status: **partial**. A diode generator exists (`kernel/cells/src/diode.rs`, not checked for ESD arrays).

### H14-49 SCR and lateral-PNP ESD rules
- Kind: rule
- Statement:
  - SCR: V_h can be about 1 V and I_h a few mA (an improved design reached 70 mA). Use only if V_op,max < V_h or the available current < I_h. Stacking erases its area advantage.
  - Lateral PNP: closely interdigitated narrow emitter and collector fingers; no ballast needed. The floating-base variant has lower V_t and almost no snapback, but still gets an ECGR.
- Source: MVSCR (45271–45358); Lateral PNP (45363–45416), PDF pp.762–765.
- Philis stage: verify, cells.
- Automation recipe: feed H14-43. Build the LPNP array generator the same way as H14-48.
- Beats hand layout because: the hold-in conditions are checked against pin specs for every pad.
- Philis status: **missing**.

### H14-50 Bipolar ESD clamps (buffered Zener, V_CES/V_CER, V_ECS)
- Kind: rule (cells)
- Statement:
  - Buffered Zener: 300–600 µm² of emitter for 2 kV, R1 ≈ 1 kΩ, all in one tank with a shared sinker.
  - V_CES/V_CER: 300–500 µm² for 2 kV, filleted base corners, base overlap of contact 1–2 µm above the rule.
  - V_ECS: < 600 µm² for 2 kV / 200 V MM, two emitters sharing a base contact, no ECGR. V_t 7–12 V in BiCMOS; stacking two protects 12 V.
- Source: 44608–44788, 46057–46063, 46099–46105 (PDF pp.751–755, 774–775).
- Philis stage: cells (bjt.rs), deck.
- Automation recipe: parametric BJT-clamp templates (emitter area as a function of the HBM target, linear scaling per 44723–44724).
- Beats hand layout because: emitter area is sized from the HBM target by formula.
- Philis status: **missing**.

### H14-51 RW ballast product
- Kind: formula
- Statement: R_ballast = RW/W_finger. Example: RW = 5 mm·Ω with 50 µm fingers needs 100 Ω. Per-finger resistors R_i = RW/W_finger give an effective R_eff = R_i/N (five fingers × 100 Ω = 20 Ω) while still meeting RW.
- Source: Drain Ballasting Resistors (45584–45597, PDF p.767).
- Philis stage: cells (multi-finger output NMOS, CDM clamps), annotator.
- Automation recipe: for an output NMOS on a pin (H14-01) that is not self-protecting, generate per-finger drain ballast (SAB-extended drain or series poly) of RW/W_finger. RW comes from the deck (value not given in the book).
- Beats hand layout because: the ballast value is computed per finger width.
- Philis status: **missing**.

### H14-52 Secondary (CDM) protection placed at the protected gate
- Kind: rule (placement)
- Statement:
  - The CDM clamp or RC filter goes near the gate oxide it protects, not at the bondpad (2 Ω of metal at 10 A is 20 V), and connects to that circuit's own supply and ground.
  - RC: R1·C1 ≥ 10 ns (0.25 pF → 40 kΩ). C1 goes next to the protected device. R1 can be anywhere on the line.
  - The CDM clamp's rail taps join near the protected transistors, and its lead is wide enough for the CDM current.
  - Omit the ECGR around the clamp devices if R1 > about 25 kΩ.
- Source: RC Filters (45623–45641); CDM Clamps (45662–45707), PDF pp.767–769; guideline 5 (46043–46056).
- Philis stage: annotator (recognize), gp/dp (Proximity), gr/dr.
- Automation recipe:
  - Emit a `Proximity(clamp, protected_gate_device, max ≈ minimum spacing)`.
  - Emit a rail-join constraint: the clamp's ground joins the protected device's source net within L_max.
  - Emit an EM width for the clamp lead from the CDM current.
  - Check R1·C1 ≥ 10 ns from netlist values.
- Beats hand layout because: proximity and rail joins are hard constraints the router must meet.
- Philis status: **missing**. `Proximity` exists as a rule type (`kernel/analog/src/placement/proximity.rs`) but is not emitted for ESD.

### H14-53 Gate-oxide ESD voltage check
- Kind: check
- Statement: V_ox = V_h + I_pk·(R_int + R_metal) must be ≤ the oxide limit for the event duration.
  - Example: a 90 Å 5 V oxide takes about 22 V.
  - A 5 V GCNMOS (6 V, 4 Ω) + 2 Ω metal gives 13.8 V at 1.3 A (pass) but 46 V at 10 A (fail).
  - 50 Å oxide: about 24 MV/cm for 100 ns.
- Source: RC Filters (45604–45620, PDF p.767).
- Philis stage: verify (with PEX metal R).
- Automation recipe: for each pad-to-gate path, use PEX metal R (already computed by the signoff PEX) and the device table to compute V_ox for HBM and CDM currents.
- Beats hand layout because: the extracted metal R is included on every path.
- Philis status: **missing**.

### H14-54 Pad-based vs rail-based network ΔV (Eq 14.3, Eq 14.4)
- Kind: formula, check
- Statement:
  - Pad-based: ΔV = V_E2(I_pk) + V_E3(−I_pk) + I_pk·R_VSS, with R_VSS ≤ 2 Ω (2.6 V at 1.3 A).
  - Rail-based: ΔV = V_D2 + I_pk·R_VDD + V_E1 + I_pk·R_VSS + V_D3, with diodes about 2–3 V each and R_VDD, R_VSS ≤ 1 Ω each.
  - Analog prefers pad-based: failsafe, multiple sequenced supplies. Power parts split GND and PGND, with the output clamp returning to PGND.
- Source: 14.4.3 (45718–45819, PDF pp.769–771).
- Philis stage: verify, flow (top level).
- Automation recipe: extract ring and rail resistances between every pad pair (worst case: diametrically opposed pads) and evaluate ΔV per pair.
- Beats hand layout because: all pad pairs are evaluated, not only the worst one a designer picks.
- Philis status: **missing** (no pad ring).

### H14-55 Substrate ESD ring width (Eq 14.5)
- Kind: formula
- Statement: total HBM metal ≤ about 3 V / 1.3 A = 2.3 Ω, of which 2 Ω goes to the ring. W_G ≅ R_S·(W_D + L_D − 4d_C + 2√2·d_C)/(4 Ω), where d_C is the corner exclusion. The equation is rendered on PDF p.771 and the reftext line 45835 is blank; the √2·d_C term is the chamfered corner diagonal of Fig 14.41. Use the process-maximum R_S at 25 °C and stack all metals.
- Source: High-Current Metallization Eq 14.5 (45827–45867, PDF pp.771–772).
- Philis stage: flow (pad ring generator), verify.
- Automation recipe: size the ring from die W and L, the deck's max R_S for each metal combined in parallel, and d_C.
- Beats hand layout because: the width is computed from die size and deck R_S every time.
- Philis status: **missing**.

### H14-56 Adiabatic ESD metal and resistor cross-section (Eq 14.6)
- Kind: formula
- Statement: A = √(ρ·τ·I_pk²/(2·C_V·ΔT)), with τ = 225 ns and ΔT typically 50 °C.
  - Table 14.3 (ρ µΩ·cm / C_V J/°C/cm³): Al 2.7/2.42, Cu 1.7/3.45, CoSi₂ 15/0.56, NiSi 10.5/3.45, TiSi₂ 15/0.85, Si varies/1.66.
  - At 1.3 A: Al 6.5 µm², Cu 4.3 µm². 0.5 µm Al needs ≥ 13 µm width.
  - Resistors use ρ = R_s·t and I = ΔV/R. Example: 200 Ω, 2.5 mΩ·cm poly at 20 mA needs 3.7 µm² (7 µm wide). Silicided 0.2 µm TiSi₂ needs 0.40 µm² (2.0 µm).
  - "20X/40X EM" rules have little justification.
- Source: 45870–45995 (PDF pp.772–774).
- Philis stage: dr (width), verify, deck (ρ, C_V, t per layer).
- Automation recipe: for nets marked ESD-path, set min width = A/t_layer as a hard dr width, and check it after routing. Deck adds `c_v` per metal (Table 14.3 defaults by material).
- Beats hand layout because: every ESD-path segment gets a physics-based width, not a multiplier.
- Philis status: **missing**. EM width rules exist (`kernel/analog/src/routing/em.rs:28-52`) but there is no fusing or adiabatic rule.

### H14-57 ESD-path metal geometry
- Kind: rule (dr)
- Statement: add 45° chamfers to inside corners, extending back into the lead d_cf "at least half the thickness of the lead". Orient stress slots along current flow (not blocking finger-to-bus or bus-to-ring flow), or use parallel narrower leads. On layer changes, overlap the leads and fill the overlap with the maximum via array.
- Source: 45943–45976 (PDF p.773), Fig 14.42.
- Philis stage: dr.
- Automation recipe: for ESD-class nets, post-process inside corners with chamfer polygons (if the deck allows 45°; otherwise stair-step fill). Fill via arrays over the whole overlap.
- Beats hand layout because: every corner and via overlap on ESD nets is treated the same way.
- Philis status: **missing**. dr fills same-net notches (`backend/dr/src/lib.rs:819-870`) but has no chamfers or max-via fill found.

### H14-58 Bondpad sharing and Kelvin-pad ESD rules
- Kind: rule
- Statement: bondpads tied by stout metal share one primary device. That metal must also carry package and board imbalance currents (EM), ideally as one metal plate. Separately bonded pads on one pin need separate protection (1 nH/mm bondwire). Separate grounds are joined by APD. Stacked primary devices are each about 2× size, so a stack of two is about 4× area.
- Source: guidelines 2–4, 11 (46010–46042, 46099–46105, PDF pp.774–775).
- Philis stage: flow, verify.
- Automation recipe: extensions of the H14-42 plan and the EM checks.
- Beats hand layout because: every multi-pad net is checked.
- Philis status: **missing**.

### H14-59 Balancing resistor for an input ESD series resistor
- Kind: rule (matching)
- Statement: when an ESD series resistor R1 (with a diode-connected E-B clamp) is inserted in one input of a bipolar diff pair, add R2 = R1 in the other input so that base-current I·R offsets cancel.
- Source: Emitter-Base Clamp Fig 14.35B (45542–45549, PDF p.766).
- Philis stage: annotator.
- Automation recipe: if a diff pair's inputs have unequal series R (because of ESD), emit a MatchingPair or equal-value constraint on the two series resistors. Warn if R2 is missing.
- Beats hand layout because: the check runs on every diff pair whose inputs pass through an ESD resistor.
- Philis status: **missing** (MatchingPair exists in `kernel/analog/src/placement/matching_pair.rs`; the ESD-input check does not).

### H14-60 Output and supply pin protection choice
- Kind: heuristic
- Statement:
  - A large MOS output self-protects if it uses the ESD device's layout (silicide block) plus an ECGR and ample substrate contacts. Otherwise use a GGNMOS + series R, or per-finger ballast (H14-51).
  - CMOS supply pins need a GCNMOS (V_pk < BV_DSS, V_h > V_op,max) or an active FET with a disable circuit tied to the rail.
  - High-voltage pins: PNP first.
- Source: Drain Ballasting (45554–45566); guidelines 7–10 (46064–46098, PDF pp.766, 775).
- Philis stage: annotator, flow.
- Automation recipe: a decision table in the `EsdPlan` builder (H14-42).
- Beats hand layout because: the table is applied to every pin.
- Philis status: **missing**.

---

## 4. Top-15 priorities for Philis

1. **H14-01**: keep `.subckt` ports and classify injectors (pin diffusions, resistor paths < 50 kΩ, charge pumps, forward-biased bulks). Every latchup, merge and ESD rule below depends on it, and today the ports are discarded (`parse.rs:36-37`).
2. **H14-27**: rename today's rings to majority-carrier taps and add true minority rings. The drawn `Ecgr`/`Hcgr` are bulk taps (`post_cell.rs:310-322`), which contradicts their own type docs (`cell.rs:26-34`) and Hastings' definition.
3. **H14-26 + H14-22 + H14-28**: ring the injectors with supply-tied N-type ECGRs as near the injector as possible, and give output devices their own backgate region. This is the core latchup defence and is currently absent (rings go only on matched victims, `constraints.rs:66-84`).
4. **H14-03 + H14-13**: add a merge-legality predicate (injector, debias, noisy/sensitive) to `well_bridges` and ring clustering. The current code merges on bulk-net equality alone (`post_cell.rs:79-93,376-387`).
5. **H14-07**: measure crossing-overlap coupling V = R·S·C and add a class-aware keep-out that stops noisy nets routing over sensitive devices. Current coupling is lateral-only (`coupling.rs:20-22`) and gr keep-out is class-blind (`gr/src/lib.rs:749-757`).
6. **H14-08**: classify aggressors by slew and swing (charge pumps, switch nodes), not only clock names (`emit.rs:270-273`). This feeds H14-07, `Isolation` and ring emission.
7. **H14-24**: global latchup tap-distance check plus scattered backgate ties for merged MOS. This is the missing LU.x check (CRATES #5, LAYOUT-FUNDAMENTALS #60).
8. **H14-10 + H14-35**: separate return nets for injector and victim rings, and class-aware routing policy (no taps on sensitive grounds, no jumpers on power or sensitive nets). This closes LAYOUT-FUNDAMENTALS #59.
9. **H14-09 + H14-11**: minimum-width substrate-tap ring around each noisy device, plus an N-well noise-blocking ring outside it when sensitive blocks are near.
10. **H14-05 + H14-16 + H14-21**: debias checks on shared wells (I·R < 0.3 V, well bias ≥ enclosed P-regions). Turns the "shared region" risk into a Θ residual.
11. **H14-12 + H14-32**: DNW tubs for sensitive or injector NMOS when the deck has `dnwell`, with a separate ground return. The generator code already knows DNW for BJTs (`bjt.rs:129-138`).
12. **H14-52 + H14-53**: place secondary CDM protection at the protected gate (Proximity plus rail-join), and check gate-oxide ESD voltage using PEX metal R. Low effort because `Proximity` and PEX already exist.
13. **H14-56 + H14-57**: adiabatic width, chamfers and via fill for ESD-class nets in dr. The EM infrastructure (`em.rs`) is the natural host.
14. **H14-41**: non-integer-ratio unitization with a central partial segment and a minimum-squares check. Improves matching of real resistor ratios beyond integer `target_ratio` (`constraints.rs:48-52`).
15. **H14-20 + H14-19**: sense-device placement at the mean-temperature isotherm and power-device bank splitting via the existing thermal model. This is placement quality a hand layout rarely computes.
