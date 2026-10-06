# Hastings ch. 9–11: bipolar transistors, bipolar applications, diodes — study for Philis

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson, 2023), Chapter 9 "Bipolar Transistors", Chapter 10 "Applications of Bipolar Transistors", Chapter 11 "Diodes".
Reftext file: `/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/reftext/hastings.txt`, lines **25761–34124** (read in full). PDF: `docs/ref/The Art of Analog Layout 3ed 2023 -- Ray Alan Hastings ...pdf`, PDF pages 429–574 (book pages 428–573; PDF page ≈ book page + 1).

Many equations, symbols and numeric values are blank in the `pdftotext` output. Every number below that is not visible in the reftext was read from the PDF page image (PDF pages opened across both passes: 433, 437–438, 450–459, 485–488, 491–495, 506–521, 524–529, 536–537, 541–542, 556, 564–571). Values still missing are marked "not recovered". Numbers marked *derived* are arithmetic on source numbers, not source statements. Page convention: PDF page of reftext line L = 1 + (form feeds in lines 1..L); book page = PDF page − 1 (checked on PDF 524 = book 523, "10.3 Rules").

Second pass (this revision): the whole range was re-read, every garbled value in §10.3/§11.3.4 re-checked against the page images, all Philis file:line citations re-verified, and one concrete defect in the BJT array generator was found and added (H09-49; it changes priority #1).

---

## 1. Coverage

### 1.1 Read chunks (Read tool, offset/limit; every line of 25761–34124 read)

Second pass (the reads backing this revision; a 2000-line read exceeds the tool's token cap, so chunks are ≤ 1100 lines):

| # | Lines (offset..end) | Note |
|---|---|---|
| 1 | 25761–26860 | ch. 9 intro → 9.2.1 (stretched NPN, Fig. 9.14) |
| 2 | 26861–27960 | 9.2.1 → 9.3.3 start |
| 3 | 27961–29060 | 9.3.3 → 10.1.1 Table 10.1 |
| 4 | 29061–30160 | Table 10.1 → 10.1.5 (Fig. 10.18) |
| 5 | 30161–31260 | 10.2 → 10.3 (minimal matching) |
| 6 | 31261–32260 | 10.3 → 11.1.2 buried Zeners (Fig. 11.6) |
| 7 | 32261–33260 | 11.1.2 → 11.2.1 poly PIN diodes |
| 8 | 33261–34124 | 11.2.1 → end of 11.5 (line 34124 = "Chapter 12") |

First pass used the boundaries 25761–26860, 26861–27960, 27961–29060, 29061–30160, 30161–31260, 31261–31820, 31821–32360, 32361–33460, 33461–34124. Both passes cover every line of 25761–34124.

PDF pages opened to recover garbled values (both passes): 433, 437–438, 450–459, 485–488, 491–495, 506–521, 524–529, 536–537, 541–542, 556, 564–571.

### 1.2 Every heading in the range (reftext line → PDF page)

- **Chapter 9 Bipolar Transistors** (25761 → 429)
  - 9.1 Bipolar Transistor Operation (25864 → 431)
    - 9.1.1 Beta Variation (26028 → 433)
    - 9.1.2 Avalanche Breakdown (26144 → 435)
    - 9.1.3 Saturation in NPN Transistors (26232 → 436)
    - 9.1.4 Saturation in Lateral PNP Transistors (26396 → 440)
  - 9.2 Standard Bipolar Small-Signal Transistors (26465 → 442)
    - 9.2.1 The Standard Bipolar Vertical NPN Transistor (26481 → 442); "Construction of Small-Signal NPN Transistors" (26750 → 446)
    - 9.2.2 The Standard Bipolar Substrate PNP Transistor (26960 → 449); "Construction of Small-Signal Substrate PNP Transistors" (27028 → 450)
    - 9.2.3 The Standard Bipolar Lateral PNP Transistor (27138 → 452); "Construction of Small-Signal Lateral PNP Transistors" (27226 → 454)
    - 9.2.4 High-Voltage Bipolar Transistors (27485 → 459); "Voltage Recognition Layers" (27579 → 460)
    - 9.2.5 Super-Beta NPN Transistors (27639 → 461)
  - 9.3 CMOS and BiCMOS Small-Signal Bipolar Transistors (27720 → 463)
    - 9.3.1 Analog CMOS PNP Transistors (27771 → 463)
    - 9.3.2 Shallow-Well Transistors (27890 → 465)
    - 9.3.3 Analog BiCMOS NPN Transistors (27958 → 466)
    - 9.3.4 Analog BiCMOS Lateral PNP Transistors (28065 → 469)
    - 9.3.5 Fast Bipolar Transistors (28147 → 470); "Washed-Emitter and Washed-Emitter-Base Transistors" (28270 → 472); "Polysilicon-Emitter Transistors" (28334 → 473); "Oxide-Isolated Transistors" (28458 → 475); "Silicon-Germanium Heterojunction Bipolar Transistors" (28605 → 478)
  - 9.4 Summary (28724 → 481); Selected Bibliography (28753); 9.5 Exercises (28773 → 482)
- **Chapter 10 Applications of Bipolar Transistors** (28893 → 484)
  - 10.1 Power Bipolar Transistors (28916 → 485)
    - 10.1.1 Failure Mechanisms of Bipolar Power Transistors (28948 → 485); "Emitter Debiasing" (28954); "Thermal Runaway" (29134 → 488); "Emitter Current Focusing" (29240 → 490)
    - 10.1.2 Standard Bipolar Power NPN Transistors (29304 → 491); "The Interdigitated-Emitter Transistor" (29353 → 492); "The Wide-Emitter Narrow-Contact Transistor" (29462 → 494); "The Christmas-Tree Transistor" (29577 → 495); "The Cruciform-Emitter Transistor" (29652 → 497); "Selecting a Power Transistor Layout" (29700 → 498)
    - 10.1.3 Standard Bipolar Power PNP Transistors (29749 → 498); "Power Substrate PNP Transistors" (29757); "Power Lateral PNP Transistors" (29802 → 499)
    - 10.1.4 Advanced Power Bipolar Transistors (29872 → 500)
    - 10.1.5 Saturation Detection and Limiting (30030 → 503)
  - 10.2 Matching Bipolar Transistors (30165 → 506)
    - 10.2.1 Random Variations (30206 → 506)
    - 10.2.2 Emitter Degeneration (30327 → 508)
    - 10.2.3 Thermal Gradients (30413 → 509)
    - 10.2.4 Mechanical Stress (30757 → 516)
    - 10.2.5 NBL Shadow (31097 → 521)
    - 10.2.6 Other Causes of Systematic Mismatch in Bipolar Transistors (31172 → 522)
  - 10.3 Rules for Bipolar Transistor Matching (31244 → 524)
    - 10.3.1 Rules for Matching Vertical Transistors (31279 → 524)
    - 10.3.2 Rules for Matching Lateral Transistors (31448 → 527)
  - 10.4 Summary (31584 → 530); Selected Bibliography (31631); 10.5 Exercises (31651 → 531)
- **Chapter 11 Diodes** (31746 → 534)
  - 11.1 Diodes in Standard Bipolar (31849 → 536)
    - 11.1.1 Diode-Connected Transistors (31858 → 536)
    - 11.1.2 Zener Diodes (31968 → 538); "The Emitter-Base Zener" (32088 → 539); "Buried Zeners" (32228 → 542); "High-voltage Zener Diodes" (32372 → 544)
    - 11.1.3 Schottky Diodes (32406 → 545); "Field-Plated Schottky Diodes" (32572 → 547); "Guard-Ringed Schottky Diodes" (32647 → 548); "Schottky Transistors" (32682 → 548)
    - 11.1.4 Base-Collector Power Diodes (32776 → 550)
  - 11.2 Diodes in CMOS and BiCMOS Processes (32909 → 553)
    - 11.2.1 CMOS Junction Diodes (32982 → 554); "The NSD/P-epi Diode" (33008); "The PSD/N-well Diode" (33075 → 555); "Poly Diodes" (33151 → 556)
    - 11.2.2 Analog BiCMOS Junction Diodes (33308 → 559); "Diode-Connected Bipolar Transistors" (33316); "BiCMOS Power Diodes" (33369); "The DMOS Buried Zener" (33458 → 561)
    - 11.2.3 CMOS and Analog BiCMOS Schottky Diodes (33509 → 562)
  - 11.3 Matching Diodes (33629 → 565)
    - 11.3.1 Matching PN Junction Diodes (33638 → 565)
    - 11.3.2 Matching Zener Diodes (33705 → 566)
    - 11.3.3 Matching Schottky Diodes (33784 → 567)
    - 11.3.4 Rules for Matching Diodes (33886 → 569)
  - 11.4 Summary (34022 → 572); Selected Bibliography (34046); 11.5 Exercises (34059 → 573)

---

## 2. Section-by-section digest

### Chapter 9 introduction (L25761–25863, PDF 429–430)
- History of point-contact, junction and planar transistors (L25763–25827).
- Bipolars are still used where MOS is worse: bandgap references, translinear circuits, thermal sensors, pulse-power circuits, ESD protection; SiGe/BiCMOS for very high speed (L25848–25853).
- Chapter plan: operation, beta, breakdown, runaway, saturation, then small-signal layout; ch. 10 covers matching and power (L25856–25858).

### 9.1 Bipolar Transistor Operation (L25864–26024, PDF 431–433)
- Four regions (cutoff, forward active, reverse active, saturation), Table 9.1 (L25865–25899).
- Thermal voltage V_T = kT/q ≈ 26 mV at 25 °C (Eq. 9.1, L25902–25913).
- Ebers–Moll (Eqs. 9.2–9.4); I_S = J_S·A_E; effective ≠ drawn emitter area because of sidewalls; two emitters placed adjacent in one base region have *less* than twice the effective area because their depletion regions approach or merge (L25942–25947).
- g_m = I_C/V_T depends only on current, not geometry (Eq. 9.8, L26014–26016).
- V_BE temperature coefficient ≈ −2 mV/°C; a 1 °C difference between two BJTs gives 8 % collector-current mismatch, i.e. 80,000 ppm/°C (L26020–26024, values from PDF 433).

### 9.1.1 Beta Variation (L26028–26140, PDF 433–435)
- Beta rises with temperature for heavy emitters; can triple over −40…125 °C; contributes to NPN thermal runaway; lateral PNP beta nearly flat vs T (L26033–26038).
- NPN beta roll-off at high current beyond ~10 µA/µm² of drawn emitter and at low current below ~10 pA/µm² (L26046–26053, PDF 433); E–B avalanche permanently degrades low-current beta (L26052–26053).
- Lateral PNP: lower peak beta, overlapping high/low roll-offs → peaked curve; peak beta occurs already in high-level injection (L26070–26077).
- Early effect, V_A ≈ 150 V for standard NPN; β = β0(1+V_CE/V_A) (Eq. 9.10), r_o (Eq. 9.11); β·V_A is the figure of merit (L26085–26140).

### 9.1.2 Avalanche Breakdown (L26144–26228, PDF 435–436)
- V_EBO ≈ 7 V, surface breakdown → avalanche-induced beta degradation; poly-emitter devices should not exceed ~½ V_EBO (L26153–26159).
- V_CBO 20–120 V, subsurface → no beta degradation; lateral PNP V_CEO ≈ V_CBO of NPN → popular for input stages (L26163–26169).
- V_CEO < V_CBO due to beta multiplication, snapback, trigger vs sustain voltages (L26172–26185); V_CES and V_CER lie between (L26189–26203).
- Collector–substrate / base–substrate junctions add limits for circuits referenced to a positive rail (L26224–26228).

### 9.1.3 Saturation in NPN Transistors (L26232–26390, PDF 436–440)
- Saturation lowers beta and adds µs-scale reverse recovery (L26238–26246).
- Junction-isolated NPN in saturation injects holes to substrate via a parasitic PNP → debiasing/latchup; guard rings needed above a few mA of base drive (L26271–26291).
- Base-current hogging in parallel NPN mirrors (Fig. 9.6, L26294–26309).
- Base-side ballasting: resistors in each base lead, sized inversely to emitter area (L26326–26332); a diffused ballast resistor must **not** share the NPN's tank, otherwise the parasitic PNP just moves (Fig. 9.8, L26347–26349).
- Schottky clamps (Pt/Pd silicide) prevent saturation (L26365–26373); saturating devices inject into shared tanks (L26388–26390).

### 9.1.4 Saturation in Lateral PNP Transistors (L26396–26459, PDF 440–441)
- Lateral PNP = emitter plug surrounded by annular collector in an epi tank; parasitic substrate PNPs Q_S1/Q_S2 (L26397–26406).
- Collector efficiency (Eq. 9.12) should exceed 95 %, often >99 % (L26421–26433); relies on NBL high-low junction; without NBL it drops below 0.5 (L26444–26446).
- A saturating lateral PNP dumps collector current to isolation, no hogging; cure: Schottky clamp or hole-blocking guard ring (L26449–26459).

### 9.2 Standard Bipolar Small-Signal Transistors (L26465–26477, PDF 442)
- Context: 40–60 V ratings, tens of mA, standard bipolar repurposed for analog.

### 9.2.1 The Standard Bipolar Vertical NPN (L26481–26746, PDF 442–446)
- Emitter-pipe limit on emitter doping (L26499–26506); β·V_A ≈ 20 kV (β 200 → V_A 100 V) (L26514–26516); beta spread spec (L26530–26536).
- NBL: bounds drift region, lowers lateral R_C; sinker ~120 % epi depth; extrinsic R_C split ~half NBL, half sinker; omitting sinker raises R_C dramatically (L26723–26746).
- Quasisaturation and Kirk effect limit usable current density (L26587–26715).
- Minimum intrinsic V_CE(sat) ≈ 18 mV for reverse beta ≈ 1 → vertical NPNs are poor sampling switches; lateral PNP or inverted NPN better (L26671–26675); forced beta ≤ β_F/10 (L26686–26687).

### "Construction of Small-Signal NPN Transistors" (L26750–26956, PDF 446–449)
- CEB (lower R_C) vs CBE; interchangeable for routing (L26751–26758).
- Drawn vs effective emitter area scales sublinearly, especially for small emitters (L26773–26778); small emitters have lower beta (peak 290 vs 520 in the 741 example) (L26781–26789).
- Emitter should overlap contact equally on all sides; contact as large as possible (L26798–26808); base overlaps emitter enough for lateral punchthrough + misalignment; base contact elongated to full base width on one side (L26812–26815).
- Tank holds max NBL; NBL touches/overlaps sinker (L26829–26831); sinker omitted → enlarge emitter around collector contact, still kΩ (L26834–26836).
- Stretched-collector/-base transistors raise capacitance/resistance; multilevel metal removes the need (L26840–26881).
- Compact emitter: high beta but pinched-base current crowding; 18 mV debias doubles current (L26884–26895). Narrow emitter with base contacts on both sides / double-base: base R ≈ ¼ (L26916–26951).

### 9.2.2 The Standard Bipolar Substrate PNP (L26960–27024, PDF 449–450)
- Collector is the substrate; free in process cost (L26960–26968); peak beta ≈ 100 in a 40 V process (L27000–27004).
- No debiasing if each substrate PNP ≤ ~1 mA and total substrate current ≤ ~10 mA with substrate contacts nearby; >1 mA needs extra substrate contacts around it (L27009–27016).
- Vertical conduction → scales with emitter area; matched devices use identical unit emitters separated by several µm of undepleted base (L27019–27024).

### "Construction of Small-Signal Substrate PNP" (L27028–27134, PDF 450–452)
- Standard, emitter-ringed and verti-lat styles (Fig. 9.18, L27029–27106); larger/compact emitters → higher beta (L27031–27040).
- Substrate contact should abut the device; required > ~1 mA (L27058–27062).
- Verti-lat and ordinary substrate PNPs should be field plated; emitter-ringed does not need it (L27090–27099).
- Large device: interdigitated wide (~25 µm) emitter stripes; width limited only by pinched-base R (may exceed 10 kΩ/□), so stripes wider than 25–50 µm are not advisable; unbroken substrate ring when DLM exists (L27109–27120, PDF 452). Note: sky130's deck already carries `bjt_max_emitter_stripe: 25000` nm (`pdks/sky130.json:51`), the same 25 µm figure, but no generator reads it.
- Substrate PNP beta starts rolling off at ~1 µA/µm² (vs ~30 µA/µm² for a vertical NPN); minimum-emitter emitter resistance ~100 Ω vs ~10 Ω for NPN; 40 V-process peak beta ~100 at < 2 µA/µm² (L26991–27004, PDF 450). These are the HLI-onset numbers H09-24 needs when a deck gives none.

### 9.2.3 The Standard Bipolar Lateral PNP (L27138–27222, PDF 452–454)
- Layout controls only base width and collector efficiency (L27155–27157); β·V_A roughly constant vs base width (L27143–27145).
- Lateral PNP beta rolls off beyond ~100 µA per minimum emitter and drops below 10 at ~250 µA; a deep-P+ emitter extends this to ~0.5 mA (L27158–27161, PDF 453). Oxide-overcoat laterals had peak β < 10; compressive nitride (hydrogen) raised it to > 50, modern > 500 (L27165–27174).
- Base-width definitions: drawn, surface, effective (Fig. 9.20); emitter and collector self-align; beta scales sublinearly with 1/W_B (L27177–27190).
- NBL under emitter raises beta; sidewall losses typically < 1 % (L27218–27222).

### "Construction of Small-Signal Lateral PNP" (L27226–27479, PDF 454–458)
- Circular emitter in circular collector hole; reverse beta ≪ forward beta (L27227–27233).
- Draw circles as polygons with segment count divisible by 4; 32 or 64 sides (L27248–27253); semisimple annulus acceptable (L27256–27260).
- Smaller and circular emitters → higher beta; collector = rectangle with circular hole (L27264–27278); NBL must enclose emitter and reach inner collector edge (L27282–27286).
- Field plate tied to emitter covering all exposed N-epi between drawn emitter and collector, overlapping collector by 2–3 µm (L27289–27311, PDF 455); without it beta fluctuates and I_C steps at 5–10 V.
- Split-collector laterals (½-½, ¼×4, 1/6-1/6-1/6-¼-¼, Fig. 9.22); identical, symmetric segments match within ±1 % (L27319–27355, PDF 456); split-collector mirrors cannot be degenerated (L27364–27368).
- Square laterals: only half/quarter collectors match (L27390–27414); hot-dog (elongated) and arrayed emitters (Fig. 9.25); size by drawn emitter periphery; Eq. 9.16 A_C = (P_E/P_EU)·(P_C/ΣP_C) (L27417–27479, PDF 458).

### 9.2.4 High-Voltage Bipolar Transistors (L27485–27575, PDF 459–460)
- V_CEO of the NPN sets the process voltage (Eq. 9.17); use the lowest voltage option to minimise isolation spacings (L27486–27509).
- Field plating and channel stops at high voltage; lateral PNP base always field plated (L27512–27520).
- Junction curvature: 120 V planar can break at 60 V; corners worse (L27524–27532).
- Fillet radius ≥ 150 % of junction depth, chamfers of similar length, both inside and outside corners; concentric fillets keep spacing (L27540–27554).
- Raising V_CEO via base resistors (V_CER) is dubious (L27571–27575).

### "Voltage Recognition Layers" (L27579–27635, PDF 460–461)
- HV spacings (base–base, HSR–HSR, base–iso, collector–base, NBL–iso, …) applied only where needed (L27580–27585).
- One recognition pseudolayer per voltage level except the highest (L27589–27592); enclose whole devices (Fig. 9.27A) or code per edge segment (Fig. 9.27B) (L27595–27635).
- LVS checks that the marker matches a per-symbol or per-parameter voltage rating (L27615–27625).

### 9.2.5 Super-Beta NPN (L27639–27713, PDF 461–462)
- Peak beta ~5000, V_A 2–3 V; high variability; low-current input stages only (L27656–27698).
- MOS replaced them; MOS diff pairs have larger offset and much larger 1/f noise than bipolar (L27702–27710).

### 9.3 CMOS and BiCMOS Small-Signal Bipolars (L27720–27767, PDF 463)
- Four process families; analog CMOS offers only a substrate PNP (L27736–27742); analog BiCMOS adds NBL + CDI NPN + lateral PNP (L27744–27750); power BiCMOS (L27754–27759); fast BiCMOS/SiGe (L27762–27767).

### 9.3.1 Analog CMOS PNP Transistors (L27771–27886, PDF 463–465)
- CMOS substrate PNP beta > 100 in old 10 V processes, barely > 1 in deep submicron (L27772–27777).
- Short-emitter effect: more contact/silicide on the emitter → lower beta; minimise emitter contacts; silicide block everywhere except under the contact (L27781–27793).
- Layout (Fig. 9.29): small square PMoat emitter, one centred contact, NMoat base ring enclosing it, contacts on one side of the ring only (L27804–27825); larger devices use minimum-width strips (L27829–27832).
- Retrograde wells kill the substrate PNP but enable lateral PNPs (L27835–27839); poly-ring lateral PNP, ring tied to emitter, β 50–100, collector efficiency > 0.9 with retrograde well, 0.1–0.2 without (L27843–27880); match V_CB to use minimum base width (L27883–27886).

### 9.3.2 Shallow-Well Transistors (L27890–27954, PDF 465–466)
- Shallow P-well in deep N-well → vertical NPN, β > 100 (L27899–27922); lateral CMOS NPN up to β 1000, collector efficiency > 0.99 (L27925–27927).
- Punchthrough and surface channels: add a PMoat channel-stop ring (also base contact) and a metal field plate from emitter (L27935–27943); collector R several kΩ (L27952–27954).

### 9.3.3 Analog BiCMOS NPN (L27958–28061, PDF 466–469)
- CDI NPN needs NBL; deep-N+ in collector contact for high current; without it ~1 kΩ and ≤ 1–2 mA (L27958–27971).
- Extended-base NPN (Fig. 9.32) and epi-base NPN (field plate required, touching deep-N+) (L27975–28010).
- DMOS NPN from DWell; poly gate field plate tied to emitter suppresses parasitic DMOS (L28013–28061).

### 9.3.4 Analog BiCMOS Lateral PNP (L28065–28143, PDF 469–470)
- Dedicated-base laterals can exceed CDI NPN beta; always minimum emitters; large devices by arrayed, not elongated, emitters (L28065–28090).
- PSD laterals: poor collector efficiency; widen collector or add deep-N+ hole-blocking ring (L28093–28098); shallow P-well collector with PMoat ring (L28102–28106); verti-lat in up-down isolation with blanket PBL, 10–20 V (L28109–28143).

### 9.3.5 Fast Bipolar Transistors (L28147–28718, PDF 470–481)
- Saturated logic slow (µs lifetimes); gold doping; Schottky clamps; ECL (L28148–28174).
- f_T (Eqs. 9.18–9.20), f_max depends on base resistance (Eq. 9.21) (L28178–28266).
- Washed-emitter / WEB (L28270–28330); poly-emitter: reflecting interface, higher beta, vulnerable to avalanche-induced beta degradation (keep reverse V_EB ≤ 1–2 V) and to EOS/ESD (L28334–28454).
- Oxide-isolated, walled-emitter, SSA, SIC (L28458–28601); SiGe/SiGe:C HBTs, f_T > 500 GHz; bipolar keeps g_m and 1/f advantages over CMOS (L28605–28718).

### 9.4 Summary, bibliography (L28724–28767, PDF 481)
- Small-signal bipolars are prized for g_m, matching, low 1/f, exponential law (bandgaps, translinear) (L28731–28738).

### 9.5 Exercises (L28773–28889, PDF 482–483)
- Base-ballast resistor sizing so a saturated device consumes ≤ 10 % of reference current (Ex. 9.5); layout exercises for CEB/CBE, narrow emitter, substrate PNP styles, split-collector lateral with emitter field plate, HV fillets, CMOS substrate/lateral PNPs, shallow-well/extended-base/poly-emitter NPNs.

### Chapter 10 introduction (L28893–28910, PDF 484)
- Bipolars: higher g_m, better matching, lower noise; pulse-power and ESD; matching only with proper layout.

### 10.1 Power Bipolar Transistors (L28916–28944, PDF 485)
- Power devices run in high-level injection, β ≥ 10 minimum acceptable; small-signal layouts OK up to ~10 mA / 100 mW; acute beyond 100 mA / 500 mW (L28922–28944).

### 10.1.1 Failure mechanisms (L28948–29300, PDF 485–491)
- **Emitter debiasing**: I_E ratio = exp(ΔV_BE/V_T) (Eq. 10.1); 4-finger example with 1/2/3 mV lead drops → 26 % (iterated 24 %) current skew at 400 mA (L28963–28991, PDF 486).
- Emitter ballasting; Table 10.1 (I_E4/I_E1 vs R_E = 0…1000 mΩ and interconnect R = 5…100 mΩ); rule: ballast drop 2–3 V_T = 50–75 mV → 500–750 mΩ, handles ≈ 25 mΩ interconnect; > 100 mV total finger-to-finger debias → redesign (L29007–29077, PDF 487).
- **Intrafinger debiasing** Eq. 10.2 ΔV_BE = L·R_S·I_E/(2W), keep ≤ 5 mV; example 300 µm × 30 µm, 12 mΩ/□, 50 mA → 3 mV; fixes: 2nd metal plate, more/shorter fingers, distributed ballast (L29080–29130, PDF 488).
- **Thermal runaway**: SOA, T_J max 125–175 °C; pulsed SOA with ≤ 5–10 % duty; hot-spot collapse from −2 mV/°C; filamentation; secondary breakdown always from debiasing → ballast (L29134–29236).
- **Emitter current focusing** at turn-off (RBSOA): centre of emitter turns off last; many small emitters or hollow emitters; FBSOA and RBSOA cannot both be optimised (L29240–29300, PDF 491).

### 10.1.2 Standard Bipolar Power NPN (L29304–29744, PDF 491–498)
- Linear mode: ≤ 150 µW/µm² and ≤ 10 µA/µm² of emitter; switched mode ≤ 20 µA/µm²; pulsed mode ≤ 1 µs pulses, ≥ 250 ns apart, RMS ≤ 20 µA/µm², metal per intermittent EM rules (L29316–29344, PDF 491–492).
- **Interdigitated emitter** (Fig. 10.7): per-finger ballast from ~1 □ emitter diffusion pairs (5 Ω/□ → 2.5 Ω, 50 mV at 20 mA → size fingers for 20 mA); intrafinger ≤ 5 mV; emitter width 8–25 µm; base contacts on both sides of every finger including the outermost; base debias ≤ 2–4 mV; comb > serpentine base metal; sinker width ≥ 2× epi thickness, sinkers on both sides → NBL R ÷ 4, a ring less; NBL to outer sinker edge; deep-N+ ring = hole-blocking guard ring; best RBSOA, poor FBSOA (L29353–29459, PDF 492–494).
- **Wide-emitter narrow-contact (WENC)**: distributed ballast, Eq. 10.3 ΔV_E = R_SE·W_O²·J_E (5 Ω/□, 15 µm, 20 µA/µm² → 23 mV), Eq. 10.4 ΔV_B = R_SB·W_O²·J_E/β (200 Ω/□, β 25 → 36 mV); overlap 10–30 µm; contact must stop short of finger ends with end overlap = side overlap; metal drops ≤ 5–10 mV; tie both ends of serpentine base lead (÷4); good FBSOA, poor RBSOA, fine for pulsed gate drivers (L29462–29573, PDF 494–495).
- **Christmas-tree / H-emitter**: excellent FBSOA, poor RBSOA; separating emitter banks tripled FBSOA (L29577–29648).
- **Cruciform**: 3-D distributed ballast, small-contact EM concern, compactness causes local heating → split into spaced sections (L29652–29696).
- Table 10.2 comparison (FBSOA/RBSOA/frequency/compactness/emitter sensing) (L29700–29744, PDF 498).

### 10.1.3 Standard Bipolar Power PNP (L29749–29868, PDF 498–500)
- Power substrate PNP: needs backside contact; thinned die, solder/silver sinter; rigid mount raises stress → place matched parts carefully, use common centroid (L29757–29791).
- Power lateral PNP: square or hexagonal arrays of minimum emitters, continuous deep-N+ ring (base contact + hole-blocking); 0.25–1 mA per emitter before β < 5; beta roll-off self-ballasts → nearly indestructible (L29802–29837); deep-P+ emitter extension 2–3× current density (L29841–29849).

### 10.1.4 Advanced Power Bipolar (L29872–30026, PDF 500–503)
- DLM CDI WENC (Fig. 10.14): base ring fully around each finger, emitter M2 plate, base M1 grid, collector M1 ring + U-shaped M2; extra metals thicken E/C plates (L29873–29887).
- RF SiGe power: very high current density; base vs emitter ballasting trade-off; subdividing into widely spaced sections; poly ballast resistors; cellular layouts of tiny emitters (L29911–29994).

### 10.1.5 Saturation Detection and Limiting (L30030–30159, PDF 503–505)
- Deep-N+/NBL hole-blocking ring; ≥ 100:1 doping ratio suffices (L30043–30047).
- Secondary (ring) collector: grounded = hole-collecting guard ring; tied to base = self-limiting; or saturation detector feeding anti-saturation loop (stability concern, no model) (L30063–30101).
- NPN > few mA base drive needs deep-N+ ring; base diffusion in collector = saturation detector; keep guard rings even with anti-saturation (L30104–30116); symbols (Fig. 10.18).

### 10.2 Matching Bipolar Transistors (L30165–30201, PDF 506)
- Actual emitter area sublinear in drawn area → match only identical unit emitters; ratios 4:1, 6:1, 8:1; > 8–10 units impractical (L30173–30178).
- ΔV_BE = V_T·ln N (Eq. 10.5); 1 % area mismatch → 0.25 mV (L30182–30195).

### 10.2.1 Random Variations (L30206–30323, PDF 506–508)
- I_S = J_S·A_E (Eq. 10.6); s_A = A_E·√(k_A²/(2A_E) + k_P²/(2P_E)) (Eq. 10.7); areal term dominates down to 16 µm² (fn. 24).
- s(I_C2/I_C1) = k_A/√A_E (Eq. 10.8); s(ΔV_BE) = k_A·V_T/√A_E (Eq. 10.9); typical k_A = 2 %·µm; 6×6 µm emitters → 0.33 % and 86 µV at 25 °C (L30243–30268, PDF 507).
- k_A independent of current density except retrograde-well BiCMOS devices, which only obey Eq. 10.8/10.9 at constant current density (L30272–30278).
- Beta variation matters for low-beta mirrors; trimmed Brokaw with PNP mirror suffers if worst-case β < 10 (L30281–30288).
- Keep matched arrays compact; circle/square/octagon emitters with matching contact shape; emitters 2–10× minimum width (4–100× minimum area) (L30300–30308).

### 10.2.2 Emitter Degeneration (L30327–30409, PDF 508–509)
- Transfers matching burden to resistors; resistors sized for equal drop (1X: 4 kΩ, 2X: 2 kΩ, 3X: 1.33 kΩ at 25/50/75 µA) (L30329–30338).
- ℜ_d/ℜ = V_T/(V_T+V_d) (Eq. 10.10): 50 mV → ÷3, 100 mV → ÷6 (L30353–30366).
- Early mismatch with degeneration I_C1/I_C2 ≅ 1 + ((V_CE1−V_CE2)/V_A)·(V_T/(V_T+V_d)) (Eq. 10.11) (L30369–30381).
- Laterals benefit (minimum emitters, low V_A); split collectors cannot be degenerated; resistors interdigitated for gradients; 250–500 mV for non-integer ratios, e.g. 3.4:1 from 3X+10 kΩ vs 1X+34 kΩ (L30384–30409).

### 10.2.3 Thermal Gradients (L30413–30753, PDF 509–516)
- −2 mV/°C; 80,000 ppm/°C; ±1 mV offsets need ±0.5 °C (L30414–30417, PDF 509).
- Diff-pair random offset trimmable; bipolar trim tracks temperature (unlike MOS) (L30451–30455).
- Thermal feedback: output-stage heat modulates input offsets; separate input and output stages (opposite die sides), common-centroid inputs; dual/quad cross-coupling (L30458–30477).
- Ratioed pair ΔV_BE = V_T·ln((I_C1/I_C2)·(A_E2/A_E1)) (Eq. 10.12, physically correct orientation), = (kT/q)·ln N for equal currents (Eq. 10.13); valid in low-level injection over up to eight decades (L30481–30522).
- SNS current: collector-current ratios unaffected, emitter-current ratios (diode-connected) affected (L30526–30539); 8:1 → 54 mV (L30540–30545).
- Ratioed quad ΔV_BE = V_T·ln(A_E1·A_E2/(A_E3·A_E4)) (Eq. 10.14); 4X,4X,1X,1X → 72 mV; 8:1 pair: 1 mV mismatch = 2 % error = 0.5 °C (L30567–30586, PDF 511).
- Cross-coupled quad (Fig. 10.23): emitters **and base contacts** common-centroid (thermoelectric potentials); collectors outside; CBE preferred over CEB because V_BE TC exceeds base-contact potential TC (L30589–30620, PDF 512).
- Optimal ratio: VPTAT ∝ ln N, gradient residue ∝ N → optimum 6:1–16:1; common 4:1/6:1/8:1, 8:1 most popular; quad 4:1:1:4 (L30628–30637).
- 4:1 as 2:1:2 (Fig. 10.24A); 8:1 as 3×3 eight-around-one, spanning ⅓ of 1-D, quadratic residue lowered by nearly 10× (L30641–30651, PDF 513).
- Merged tank OK (collector does not control current); merged base needs neutral base between emitters ≥ 2× base junction depth and larger base overlap (couple of µm) (L30667–30690); connected-emitter spacing forbidden for matched emitters (L30696–30700); laterals: one identical collector opening per emitter (L30704–30709).
- ABA 1-D arrays oriented with long axis parallel to isotherms; power device at centre of one die end, pair at other end on die axis (L30712–30718); multiples of 4:1 as two rows (16:1) with S2 parallel to isotherms; base contacts not CC → double-base (L30733–30745); ratioed quads = two stacked ratioed pairs on a common axis (L30749–30753).

### 10.2.4 Mechanical Stress (L30757–31094, PDF 516–521)
- Piezojunction ΔI_S = −I_S(ζ_L σ_L + ζ_T σ_T + ζ_LT τ_LT) (Eq. 10.15); circular lateral averages → ζ_R(σ_x+σ_y) (Eq. 10.16); vertical → ζ_T(σ_x+σ_y) (Eq. 10.17) (L30757–30848).
- Table 10.3 (10⁻¹¹ Pa⁻¹): (100) ζ_L NPN −28.4, PNP 8.9; ζ_R NPN 7.5, PNP 11.6; ζ_T NPN 43.4, PNP 13.3. (111) ζ_L 28.2 / 81.5; ζ_R 21.7 / 29.7; ζ_T 15.1 / −22.2 (PDF 516).
- 100 MPa in-plane → vertical NPN 0.4 mV (111) / 0.9 mV (100); lateral PNP 0.8 / 0.3 mV; equal stress cancels in ΔV_BE (L30862–30867).
- Package shift: mold cures at 150–200 °C; SOT-23 bandgaps −1.1 mV mean, 2.3 mV σ, mostly filler stress (L30876–30922, PDF 517–518); Brokaw V_bg (Eq. 10.18); normal filler stress Eq. 10.19.
- Placement: stress gradient smallest at die centre; best on die X/Y axes near centre; ≥ 250 µm from edges; never corners (Fig. 10.28); with a heat source: far side, 125–250 µm from the far edge, on a symmetry axis (Fig. 10.29) (L30946–30987, PDF 518–519).
- Filler stress is local, location-independent, not cancelled by CC → 15 µm Cu over nitride changed shift from −5.06 mV/2.64 mV σ to −2.26 mV/1.38 mV σ; polyimide helps; low-stress mold; PCB/QFN, long-term drift, hysteresis, ADR4520 example; on-chip stress sensor (L30987–31087).

### 10.2.5 NBL Shadow (L31097–31168, PDF 521–522)
- Pattern shift up to ~2× epi thickness; a 2:1 array with one shadowed emitter each → 0.5 % (0.13 mV) (L31098–31106).
- Fixes: all single-emitter devices; NBL oversized so shadow misses emitters; CEB array with axis parallel to shift; STI processes have no shadow (L31122–31160); laterals: keep shadow off the exposed base (L31164–31168).

### 10.2.6 Other Systematic Mismatch (L31172–31238, PDF 522–523)
- Early effect Eq. 10.20/10.21; V_A 150 V example → 0.7 % (ΔV_CE not recovered) (L31179–31201).
- Lateral collector efficiency varies with V_CE and collapses in saturation (L31204–31211).
- Cross-injection in merged laterals; separate tanks (or P-bars) (L31215–31227); depletion-merge in multi-emitter NPNs (L31231–31238).

### 10.3 Rules for Bipolar Transistor Matching (L31244–31275, PDF 524)
- Minimal: ±2 mV V_BE or ±8 % I_C; moderate: ±0.5 mV or ±2 % (±1 % bandgaps); exceptional: ±0.1 mV or ±0.5 % (needs trim or degeneration; laterals need heavy degeneration; cold β > 20); 6σ, 10 yr, −40…125 °C, plastic package (PDF 524).

### 10.3.1 Rules for Matching Vertical Transistors (L31279–31444, PDF 524–527)
- 18 rules: identical emitters; width ≥ 2× minimum (2 µm contact + 2×1 µm overlap = 4 µm min; matched 8–40 µm); circular/square; proximity; compactness/cross-coupled; even ratios 4:1–16:1 and 4:1:1:4–8:1:1:8; power-device distances (250 µm from ≥ 250 mW; 100–250 µm from > 50 mW); low-stress (≥ 250 µm from edge; CSP bump guidance); die axes; NBL shadow (150 % epi if unknown); emitter spacing in common base; base overlap +1–2 µm; ≤ ⅓–½ of HLI onset current; contact geometry = emitter geometry; degeneration 50/100/200 mV; equal V_CE (0.3–1 %/V); V_EB ≤ 50 % V_EBO; vertical PNP on (100).

### 10.3.2 Rules for Matching Lateral Transistors (L31448–31578, PDF 527–529)
- 14 rules: identical emitter and collector; minimum emitters; inner collector periphery = emitter shape, concentric; field plate the base; split collectors moderate matching (Gilbert: ±0.1 % cross-coupled); proximity (common base only if collector efficiency > 0.999); 250 µm from heat; low stress 250 µm; die axes; NBL overlap of inner collector ≥ 150 % epi; ≤ 30–50 % of HLI onset (≈ 1 µA per minimum emitter); contact geometry; degeneration ≥ 50/100 mV, 200–300 mV for 10:1; equal V_CE (0.5–2 %/V).

### 10.4 Summary, bibliography (L31584–31645, PDF 530)
- Vertical bipolars excel at pulse power, gate drivers, ESD; bipolar matching beats MOS with CC; bandgaps reach high accuracy with exceptional matching (L31592–31620).

### 10.5 Exercises (L31651–31740, PDF 531–533)
- Linear ≤ / switched ≤ current-density caps as in 10.1.2; power layouts (interdigitated, WENC, cruciform); degenerated lateral mirror (Fig. 10.20); 4:1:1:4 quad (merged emitters +spacing); Brokaw cell; op-amp cross-coupling; Gilbert core; die floorplan with power NPN (Ex. 10.11); Early/stress/package-shift arithmetic.

### Chapter 11 introduction (L31746–31843, PDF 534–535)
- Diode equation I = I_S(exp(V/(nV_T))−1) (Eq. 11.1); ideality n ≈ 1 for PN at low current, > 1 at high current or for some Schottkys (L31794–31807).
- Forward voltage (Eq. 11.3); PN diode TC ≈ −2 mV/°C, Schottky smaller (value not recovered) (L31831–31843).

### 11.1 Diodes in Standard Bipolar (L31849–31855, PDF 536)
- Diode-connected transistor, base-emitter Zener, Schottky (needs silicide + extra mask).

### 11.1.1 Diode-Connected Transistors (L31858–31964, PDF 536–538)
- R_S = R_B/β_F + R_E (Eq. 11.4); tolerate ≤ 400 mV ohmic collector drop at 25 °C, 200 mV at 150 °C; > few hundred µA → sinker; ≥ 10 mA → power layout with deep-N+ ring (L31870–31880, PDF 536).
- Minimum device R_S ≈ 10–20 Ω (L31896–31899); V_EBO 6–12 V (typ. 6.8 V), reverse bias ≤ ⅔ V_EBO (L31903–31908).
- CBE preferred; merged collector-base contact saves area (Fig. 11.2) (L31911–31918).
- ≈ 0.65 V at 1 µA/µm², 25 °C; doubling current adds 18 mV; −2 mV/°C (L31938–31943, PDF 537); substrate PNP diodes > 1 mA may saturate from debiasing (L31947–31950); diode-connected laterals lose 0.1–1 % to substrate → not for accurate current matching (L31956–31964).

### 11.1.2 Zener Diodes (L31968–32087, PDF 538–539)
- Zener (tunnelling) < 5–6 V, negative TC; avalanche above, positive TC; 5–6 V near-zero TC but soft breakdown and walkout/walkback (200–300 mV, sometimes > 1 V) (L32032–32061).
- Avalanche microplasmas → RTS noise, reduced at higher current density (L32065–32070); < 5 V soft breakdown makes low-voltage gate clamps impractical (L32079–32084).

### "The Emitter-Base Zener" (L32088–32226, PDF 539–542)
- V_EBO ≈ 6.8 V; walkout up to 250 mV (L32089–32118).
- Same layout as NPN; tank must be tied (anode, cathode, or ≥ anode), never floating; no sinker needed; NBL optional (L32121–32131).
- Round emitters against corner breakdown (L32146–32158); field plate only effective over thin oxide, biased positive (L32167–32178); current per µm periphery limit (value not recovered); Zener-zap fuse (L32186–32192).
- Nonisolated Zener sharing substrate potential saves area but is exposed to debiasing/noise (> few hundred µA injectors nearby) (L32196–32224).

### "Buried Zeners" (L32228–32370, PDF 542–544)
- Subsurface breakdown deeper than thermalization distance (140 nm electrons, 60 nm holes) → no walkout (L32229–32235).
- Emitter-in-iso, P+ plug (0TC 5.0–5.4 V), implanted buried Zener; tight current regulation (L32243–32345).

### "High-voltage Zener Diodes" (L32372–32403, PDF 544–545)
- Series stacks of E-B Zeners plus diode-connected transistors for TC compensation; isolation–NBL Zener ≈ 20 V, subsurface, higher power (L32373–32387).

### 11.1.3 Schottky Diodes (L32406–32570, PDF 545–547)
- Thermionic emission (Eq. 11.5/11.6); Table 11.1 barrier heights (Al 0.72, CoSi₂ 0.65, Au 0.80, Mo 0.68, NiSi 0.67, PdSi 0.75, PtSi 0.87, TiSi 0.61 V on N-Si) (L32419–32480).
- Barrier < 0.6 V → excessive hot leakage; N-doping must stay low to avoid tunnelling; optimum barrier 0.75–0.80 V (L32516–32531); aluminium Schottkys vary lot-to-lot (sinter) (L32535–32548).

### "Field-Plated Schottky Diodes" (L32572–32644, PDF 547–548)
- Metal flange as field plate over thinned oxide; NBL + deep-N+ cathode; full deep-N+ ring for high current also blocks holes (L32573–32632); shape of low-voltage Schottky unimportant → can fill leftover space (L32641–32643).

### "Guard-Ringed Schottky Diodes" (L32647–32678, PDF 548)
- Base field-relief ring removes edge field; BV = base/epi BV; guard ring for large diodes, field plate for small (L32648–32678).

### "Schottky Transistors" (L32682–32770, PDF 548–550)
- Clamp needs ≥ 150 mV lower forward voltage than B–C junction; Mo, PdSi, PtSi trade-offs; PtSi loses clamping at high T (L32684–32767).

### 11.1.4 Base-Collector Power Diodes (L32776–32903, PDF 550–552)
- Large single base anode, epi ballasts; fill anode with contact (L32787–32810); drift R (Eq. 11.7), circular NBL lateral R (Eq. 11.8) (L32816–32857).
- NBL + deep-N+ hole-blocking ring mandatory above a few mA; > 100 mA: ring width ≥ 2× epi, NBL to ring's outer edge (L32861–32871); emitter over sinker for contact (L32875–32879); stored charge; guard efficiency drops at high injection (L32882–32888).

### 11.2 Diodes in CMOS and BiCMOS (L32909–32978, PDF 553–554)
- Six diode-connected MOS configurations (Fig. 11.14); PMOS mirror reference uses the antiparallel-body-diode form; P-sub CMOS cannot stop substrate injection when a PMOS body diode conducts (L32923–32978).

### 11.2.1 CMOS Junction Diodes (L32982–33304, PDF 554–558)
- NSD/P-epi, N-well/P-epi, PSD/N-well junctions; used as ESD and antenna diodes (L32983–32989).
- NSD/P-epi ESD: 2 kV HBM → 1.3 A peak, τ ≈ 220 ns, R_S ≤ 3 Ω for ≈ 5 V; interdigitated NMoat/PMoat strips; inherent distributed ballast; fully silicide; latchup rules if pin-connected (L33014–33058).
- PSD/N-well: is a substrate PNP B–E junction; ESD antiparallel clamps; Zener BV ≥ ~7 V (avalanche) (L33076–33099); poly field-plate ring tied to anode forces subsurface breakdown (L33102–33108); ≤ 1–2 µA per µm of cathode periphery; outermost strips PSD; silicide-block ≥ 1–2 µm between contact and moat edge, or +1–2 µm moat-over-contact (L33128–33147, PDF 556).
- Poly diodes: grain-boundary G-R leakage, asperities → PIN form with wide drawn intrinsic region; thermally fragile (L33151–33302).

### 11.2.2 Analog BiCMOS Junction Diodes (L33308–33505, PDF 559–562)
- Diode-connected NPN preferred (collector+base = anode) (L33316–33322); many decks do not support merged C–B contacts (L33325–33329).
- Extended/shallow-well/DWell-base Zeners 6–20 V; surface ones walk out/back up to 250 mV (sometimes 1 V) (L33332–33346); avoid poly-emitter Zeners (L33354–33360).
- BiCMOS power diode (Fig. 11.18A) best: PSD anode, N-well + NBL + deep-N+ ring (hole-blocking) handles amps; NBL to outer edge of deep-N+ (L33369–33398); shallow-N-well variants; zero-biased hole-collecting ring only for a few mA (L33400–33454).
- DMOS buried Zener 6–9 V; isolation tied to anode normally, but never to high-impedance nodes (a field failure was fixed by grounding it) (L33458–33500).

### 11.2.3 CMOS and Analog BiCMOS Schottky Diodes (L33509–33622, PDF 562–564)
- CoSi₂ and NiSi CMOS silicides allow Schottkys; PSD field-relief ring (Fig. 11.21); high series R without NBL; interdigitated contacts; hole injection at high current; contact-array processes need fully silicided moat (L33511–33599); BiCMOS version with NBL/deep-N+ (Fig. 11.22) (L33603–33608).

### 11.3 Matching Diodes (L33629–33634, PDF 565)
- PN, Zener and Schottky diodes never match each other; within a class only with identical diffusions and layout.

### 11.3.1 Matching PN Junction Diodes (L33638–33702, PDF 565–566)
- Diode-connected BJTs follow 10.3.1/10.3.2; merged C–B contact emitter must obey unconnected-emitter spacing + several µm (L33645–33649).
- Emitter-current-controlled ratioed diode pairs need a beta plateau; CMOS/BiCMOS BJTs rarely have one → operate near peak beta or compensate base current (L33653–33666).
- PSD/N-well ratioed pairs: avoid; guard rings do not fix beta roll-off; N-well resistance × beta variation → use long thin PMoat strips between NMoat contacts (Fig. 11.23) (L33670–33682); poly diodes never matched (L33698–33702).

### 11.3.2 Matching Zener Diodes (L33705–33780, PDF 566–567)
- Winner-takes-all breakdown: Pelgrom does not apply; E-B Zener lateral ballast Eq. 11.9 R = J·R_S·r1·ln(r2/r1) gives only 4 mV (J = 10 µA/µm, 160 Ω/□, r1 = 4 µm, r2 = 8 µm) vs ~50 mV needed (L33715–33738, PDF 566).
- Quatrefoil cross-coupled layout (Fig. 11.24); surface Zeners differ 50–100 mV (up to 1 V); prefer PN/MOS stacks (L33746–33780).

### 11.3.3 Matching Schottky Diodes (L33784–33882, PDF 567–569)
- Al Schottkys worst; noble silicides better; must be guard-ringed; PtSi 250 µm² ideality 1.02 (L33797–33810).
- n > 1.1 → inhomogeneity; small diodes worse (Au/p-Si: n 1.006 at 100 µm, rises below 20 µm, 1.41 at 5 µm; barrier 0.791 → 0.623 V); avoid ratioed Schottky pairs (L33818–33826).
- n = 1 + T₀/T (Eq. 11.10), n = 1.1 ↔ T₀ ≈ 30 K; CoSi₂ n 1.06 → 1.50 from 900 to 1100 °C anneal (L33835–33860).
- Matched Schottkys: > 10–15 µm, identical, equal current density, guard-ringed, low current, compact shape, NBL/deep-N+ or annular cathode (L33868–33882).

### 11.3.4 Rules for Matching Diodes (L33886–34016, PDF 569–571)
- Classes as for BJTs: ±2 mV/±8 %, ±0.5 mV/±2 % (±1 % bandgaps, ±1–2 mV op-amps), ±0.1 mV/±0.5 % (only vertical BJTs with beta plateaus) (PDF 569).
- 14 rules: no poly/surface-Zener matching; ideality ≤ 1.1 minimal, < 1.05 (pref. < 1.03) moderate; beta plateau required; identical area-defining geometry; width 2–10× minimum (Schottky ≥ 10–15 µm; moderate diode-connected area 10–50 µm²); circle (32/64 sides) or square; proximity + CC mandatory for moderate/exceptional; compact cross-coupled; 250 µm from ≥ 250 mW, 100–250 µm from > 50 mW; low stress 250 µm from edges; die axes; NBL shadow (even minimal Schottkys); merged junction spacing +1–2 µm; base-over-emitter +1–2 µm (PDF 569–571).

### 11.4 Summary, bibliography (L34022–34055, PDF 572)
- ICs use PN, diode-connected BJTs and Schottkys; diode-connected NPNs match to a few mV and handle amps (L34031–34034).

### 11.5 Exercises (L34059–34120, PDF 573–574)
- Merged C–B diode layout, Zener TC stacking, field-plated/guard-ringed/power Schottkys, matched PSD/N-well pair (Ex. 11.8), BiCMOS power diode, thin-oxide field-plated Zener.

---

## 3. Actionable extraction

Conventions. "Class" = the matching class (H09-01). Philis stages: `annotator` (backend/annotator), `cells` (kernel/cells), `gp`/`dp` (placement), `gr`/`dr` (routing), `verify`, `flow` (frontend/library), `deck` (pdks/*.json). Constraint form follows Philis's Rule trait: *hard* (legalizer/DRC-like), *budget* (headroom/residual, Θ), *cost* (anneal/GD objective).

A cross-cutting data gap: Philis is block-level; die-location rules (H09-20/21) need a die context (die outline, block origin in die, wafer orientation, package type, external heat sources) that `library::run` does not take today. Without it, each die rule must report **unknown**, the way `CentroidGroup` does without unit data (`kernel/analog/src/placement/cc.rs:10-11`).

### H09-01 Matching-class taxonomy (minimal / moderate / exceptional)
- Kind: data-model
- Statement: BJTs and diodes: minimal = ±2 mV ΔV_BE or ±8 % ΔI_C; moderate = ±0.5 mV or ±2 % (±1 % bandgaps; ±1–2 mV untrimmed op-amps); exceptional = ±0.1 mV or ±0.5 % (needs post-package trim or matched emitter degeneration; lateral PNPs need heavy degeneration plus base-current cancellation and cold β > 20). All figures 6σ, 10-year life, −40…125 °C, plastic package.
- Source: §10.3, L31244–31275, PDF 524; §11.3.4, L33886–33905, PDF 569.
- Philis stage: annotator, flow.
- Automation recipe: add `MatchClass {Minimal, Moderate, Exceptional}` per matched group, from (a) user override, (b) circuit role (bandgap/PTAT core → Moderate; general op-amp input → Minimal; trimmed reference → Exceptional), (c) an offset target. Class sets the tolerance (6σ random + systematic ≤ class limit) and switches rules between hard/budget/cost: CC is cost for Minimal, hard for Moderate/Exceptional (H09-06); die-axis placement only for Moderate/Exceptional (H09-20).
- Beats hand layout because: every matched group carries an explicit, checkable accuracy target; rules scale with it instead of being applied uniformly or forgotten.
- Philis status: missing. Only a scalar `offset_sigma_mv` exists (`frontend/library/src/lib.rs:487`); no class notion (grep for Moderate/Exceptional/MatchLevel empty).

### H09-02 Bipolar random-mismatch model (Pelgrom for BJTs)
- Kind: formula
- Statement: I_S = J_S·A_E (Eq. 10.6); s_A = A_E·√(k_A²/(2A_E) + k_P²/(2P_E)) (Eq. 10.7); equal-V_BE current ratio s(I_C2/I_C1) = k_A/√A_E (Eq. 10.8); equal-current s(ΔV_BE) = k_A·V_T/√A_E (Eq. 10.9). Typical k_A = 2 %·µm; 6×6 µm emitters → 0.33 %, 86 µV at 25 °C. Areal term alone fits down to 16 µm²; with identical unit emitters the peripheral term folds into k_A. Retrograde-well BiCMOS devices: k_A rises at low current density, so area scaling only helps at constant current density. ΔV_BE = V_T·ln N (Eq. 10.5); 1 % area error → 0.25 mV.
- Source: §10.2, §10.2.1; L30182–30278; PDF 506–507.
- Philis stage: annotator, deck.
- Automation recipe: deck keys `bjt_ka_pct_um` (per device model, NPN/PNP) and optional `bjt_kp`. Annotator computes s(ΔV_BE) for each matched BJT pair from the unit emitter area and unit count (A_E = n_units·A_unit), and the random share of the class budget: 6·s ≤ η·class_limit. Solve for the minimum unit area: A_E ≥ (6·k_A·V_T/(η·ΔV_max))². *Derived example*: k_A = 0.02 µm, V_T = 25.7 mV, ΔV_max = 0.5 mV, η = 1 → A_E ≥ 38 µm². Emit the result as a sizing request to `cells` (H09-08) and as the systematic allowance (class limit − random share) for thermal/stress budgets (H09-15, H09-19).
- Beats hand layout because: the unit emitter is sized to a stated σ target instead of habit, and the leftover systematic budget is known numerically.
- Philis status: missing. `Pelgrom::new` uses A_VT of FETs (`backend/annotator/src/emit.rs:83`); `by_polarity` returns `None` for non-FETs (`emit.rs:105-111`), so BJTs get no mismatch model.

### H09-03 Identical unit emitters; ratio by unit count only
- Kind: rule
- Statement: Transistors with different emitter sizes or shapes match very poorly; actual emitter area is sublinear in drawn area. Ratios are built from identical unit emitters (4:1, 6:1, 8:1 common); ratios needing > 8–10 units are impractical (area, thermal sensitivity). Base and collector geometry matter much less. Laterals: identical emitter and identical collector geometry.
- Source: §10.2 L30173–30178, PDF 506; §10.3.1 rule 1 L31287–31291, PDF 524; §10.3.2 rule 1 L31456–31463, PDF 527; §9.2.2 L27019–27024, PDF 450.
- Philis stage: annotator, cells.
- Automation recipe: already the model: a BJT group's `Unitization.dev_nf` = unit counts. Add a check: if netlist BJT areas/`m` in a matched group are not integer multiples of a common unit, emit an annotator diagnostic and round to the nearest integer ratio (or refuse to match).
- Beats hand layout because: guarantees exact ratios in every generated variant.
- Philis status: implemented for grouping/drawing: `bjt_groups` groups by kind, W, L and base net (`frontend/library/src/cellgen.rs:717-731`), unit counts become `dev_nf` (`cellgen.rs:506-527`), and the generator draws one unit per count (`kernel/cells/src/bjt.rs:16-20`, `bjt.rs:55-65`). Missing: the non-integer-ratio check.

### H09-04 Ratio selection for ratioed pairs and quads
- Kind: heuristic / check
- Statement: VPTAT grows as ln N while the quadratic gradient residue grows ~linearly with N; optimum N lies between 6:1 and 16:1. Even ratios simplify CC arrays. Common: 4:1, 6:1, 8:1 (8:1 most popular); quad 4:1:1:4 favoured. Rule: even integer ratios 4:1–16:1 for pairs, 4:1:1:4–8:1:1:8 for quads; 2:1 / 2:1:1:2 only minimal; 64:1 too large; exceptional matching is hard in ratioed structures. Quad ΔV_BE = V_T·ln(A_E1·A_E2/(A_E3·A_E4)) (Eq. 10.14): 4X,4X,1X,1X → 72 mV; pair 8:1 → 54 mV.
- Source: §10.2.3 L30628–30637, L30567–30580, PDF 511–513; §10.3.1 rule 6 L31327–31333, PDF 525.
- Philis stage: annotator (check only; sizing is the designer's).
- Automation recipe: on a recognised ratioed pair/quad (H09-05), report N, ΔV_BE (V_T·ln N), sensitivity (%/°C, %/mV) and a warning when N is odd, < 4, or > 16 at Moderate class.
- Beats hand layout because: the ratio's error sensitivity is quantified before layout, not after silicon.
- Philis status: missing.

### H09-05 Ratioed-pair / bandgap-core recognition
- Kind: algorithm
- Statement: Ratioed pairs are two BJTs at different current densities; most common: equal collector currents with A_E2 = N·A_E1 (Eq. 10.12/10.13). ΔV_BE is valid only in low-level injection; if the circuit ratios *emitter* currents (diode-connected), low-current beta roll-off (SNS current) also affects it. Brokaw cell: Q1 (1X) / Q2 (NX), amplifier equalises collector currents, V_bg = V_BE1 + (2R1/R2)·ΔV_BE (Eq. 10.18; the V_T factor is not visible in the PDF rendering), ≈ 1.23 V.
- Source: §10.2.3 L30481–30545, PDF 509–511; §10.2.4 L30886–30916, PDF 517–518; §11.3.1 L33653–33666, PDF 565.
- Philis stage: annotator.
- Automation recipe: new BJT slot kinds in the pattern engine (`AnyBjt`, `SameBjtTypeAs`) with terminals B/C/E. Patterns: (1) ratioed pair = same type, shared base net, unequal unit counts; (2) diode-connected ratioed pair (C=B on each); (3) ratioed quad (Fig. 10.22B topology: two pairs, cross-coupled bases); (4) Brokaw core = ratioed pair + resistor between emitters (R2) + resistor to ground (R1). Output a `BlockKind::RatioedPair/RatioedQuad/BandgapCore` with devices, ratio, and associated resistors (R1/R2 must then be matched as a ratio, see H09-26 for degeneration-style resistor matching). Default class Moderate.
- Beats hand layout because: the bandgap's critical set (pair + its PTAT resistors + mirror) is identified automatically and receives CC, thermal, stress and location constraints together.
- Philis status: partial. `bjt_groups` clusters BJTs by geometry and base net (`frontend/library/src/cellgen.rs:717-731`), which catches the pair but not its topology or resistors. Pattern slots are FET-only (`backend/annotator/src/pattern.rs:18-22`); the catalog's `bandgap_mirror_pair` is a FET mirror (`backend/annotator/src/catalog.rs:1880-1884`).

### H09-06 Common-centroid required by class; linear-only cancellation
- Kind: rule
- Statement: Critical matched bipolars almost always use common-centroid layout. CC is strongly recommended for minimal and mandatory for moderate/exceptional (BJTs and diodes). Because I_C is exponential in V_BE, CC removes only the linear part of a thermal gradient; the nonlinear residue must be minimised by compactness.
- Source: §10.2.3 L30589–30592, L30615–30618, PDF 511–512; §10.3.1 rule 4 L31315–31319, PDF 525; §11.3.4 rule 7 L33957–33962, PDF 570.
- Philis stage: annotator, cells, dp.
- Automation recipe: for BJT groups of class ≥ Moderate emit `CentroidGroup` as budget (hard fail if residual > 0 at signoff); Minimal as cost. Inside a BJT array cell, verify the first moment of each device's units coincides (the generator's `unit_order` already aims for it).
- Beats hand layout because: centroid coincidence is measured exactly per device on every candidate.
- Philis status: partial, and wrong for most ratios. Centre-out unit ordering in `unit_order` (`kernel/cells/src/bjt.rs:205-231`) is tested only for 1:8 (`bjt.rs:271-281`); 1:2, 1:4, 1:6 and 2:2 come out with offset centroids (H09-49). `CentroidGroup` exists for arrays (`kernel/analog/src/placement/cc.rs:28`). No class-driven hardening, no in-cell centroid check.

### H09-07 Ratio-specific array topologies
- Kind: algorithm
- Statement: 4:1 → 2:1:2 cross (1X centred, 4X halves on both sides, rotated to bring emitters together, symmetric about axes through 1X) (Fig. 10.24A). 8:1 → 3×3 eight-around-one; spans ⅓ the 1-D length, lowering the quadratic residue by nearly 10×; base contacts form their own CC but offset slightly from the emitter centroid (Fig. 10.24B). 1-D ABA (Fig. 10.26A) when elongation is acceptable. Multiples of 4:1 (e.g. 16:1) → large device in two rows about axis S2 passing through the single emitter (Fig. 10.26B); base contacts then need double-base or an extended 1X. Ratioed quads = two ratioed pairs stacked so their primary axes coincide → 2-D CC; if impossible, treat each pair independently.
- Source: §10.2.3 L30641–30651, L30712–30753, PDF 513–516.
- Philis stage: cells.
- Automation recipe: extend `Bjt::enumerate` beyond `columns = ceil(√n)` to a variant per topology: `Cross212` (N=4), `EightAroundOne` (N=8, already the 3×3 case), `ABA` (row), `TwoRow` (N multiple of 4, N ≥ 8), `QuadStack` (two ratioed pairs sharing S1). Each variant exports its symmetry axes (S1 major, S2 minor) as cell metadata for H09-21. Score candidates by a gradient-residue metric: Σ over units of (quadratic moment difference) — the second moment of each device's emitter positions about the array centroid, equalised between devices.
- Beats hand layout because: all admissible topologies are generated and scored numerically (first and second moments), not picked from memory.
- Philis status: partial. Only square arrays for matched sets (`kernel/cells/src/bjt.rs:38-47`); 1:8 → 3×3 works (`bjt.rs:271-281`); 1:4 becomes a 2×3 grid with an empty slot and a centroid offset of (¼, ½) pitch instead of the book's 2:1:2 (H09-49); no ABA/two-row/quad-stack variants, no axis metadata.

### H09-08 Unit emitter size and width limits (vertical vs lateral)
- Kind: rule
- Statement: Vertical: matched emitter width ≥ 2× minimum drawn emitter width, where minimum = contact width + 2× emitter-over-contact (example 2 µm + 2×1 µm = 4 µm); matched emitters typically 8–40 µm wide (4–100× minimum area); upper end only when needed — exceptional matching prefers arrays of moderate emitters over one huge emitter. Lateral: use minimum-size emitters (larger emitters lower beta, hurting more than the area helps); circular preferred while its radius < centre-to-vertex of the square alternative. Diodes: area-defining width 2–10× minimum; moderately matched diode-connected transistors need 10–50 µm² active area; Schottky ≥ 10–15 µm.
- Source: §10.2.1 L30300–30308, PDF 507; §10.3.1 rule 2 L31292–31302, PDF 524; §10.3.2 rule 2 L31464–31469, PDF 527; §11.3.4 rule 5 L33935–33949, PDF 569–570.
- Philis stage: cells, deck, annotator.
- Automation recipe: the generator's unit emitter is `max(bjt_min_emitter_side, unit_w/l)` (`bjt.rs:91-92`). Add: `unit_side = clamp(2·min_emitter_width, size_from_H09-02, 10·min_emitter_width)` for vertical devices of class ≥ Minimal; laterals pinned to minimum. When the netlist fixes the emitter (PDK fixed-geometry BJTs such as sky130 pnp_05v5), keep it and report the achieved σ.
- Beats hand layout because: size is derived from the σ target and bounded by the gradient-sensitivity ceiling, per device type.
- Philis status: partial. Minimum side from deck (`pdks/sky130.json:52`, `bjt.rs:91-92`); no 2–10× policy.

### H09-09 Emitter shape: circle / octagon / square, concentric matching contact
- Kind: rule
- Statement: Use circular or square emitters (octagons also shown, Fig. 10.19); circles drawn as polygons with segment count divisible by 4 (32 or 64). Contact geometry must match emitter geometry and be concentric (circle-in-circle, octagon-in-octagon, square contact or square contact array in square emitter); a silicide block, when used, also matches. Exception: low-β devices with high base sheet R may match better with long thin emitters. Lateral: inner collector periphery must match emitter shape, concentric, equal base width on all sides.
- Source: §9.2.3 L27248–27253, PDF 454; §10.2.1 L30300–30305, PDF 507; §10.3.1 rules 3, 14 L31303–31314, L31404–31411, PDF 524–526; §10.3.2 rules 3, 12 L31470–31476, L31550–31555, PDF 527–528; §11.3.4 rule 6 L33950–33956, PDF 570.
- Philis stage: cells, deck.
- Automation recipe: deck flag `allows_non_manhattan` / `octagon_ok`; if set, generate an octagon emitter (45° edges on grid) with an octagonal contact region; else square emitter with a centred square contact array (current behaviour). Assert contact-array centroid = emitter centroid within one grid.
- Beats hand layout because: concentricity is exact and verified; polygonisation is symmetric by construction.
- Philis status: partial. Square emitter with a centred contact array (`kernel/cells/src/bjt.rs:156-176`); no octagon/circle option.

### H09-10 Short-emitter effect: few emitter contacts, silicide block
- Kind: rule
- Statement: In shallow-emitter CMOS PNPs, carriers reach the emitter contact/silicide and recombine, lowering beta; the more contact or silicide in the emitter, the lower the beta. Minimise emitter contacts; if needed, silicide-block everything except directly under the contact (skip if the minimum silicide-over-contact overlap leaves nothing to block). BiCMOS thin emitters: consider a reduced emitter contact.
- Source: §9.3.1 L27781–27807, PDF 463–464; §10.3.1 rule 14 L31408–31411, PDF 526.
- Philis stage: cells, deck.
- Automation recipe: CMOS substrate-PNP variant `EmitterContact::Single` (one centred cut) and, where the deck has a silicide-block layer (`sal_block` style), a block ring over the emitter minus the contact region with the deck's enclosure. Keep the current full contact array as a separate variant for high-current use; pick by op-point current density (low current → Single).
- Beats hand layout because: the beta/contact trade-off is an explicit variant, selected by the device's operating current.
- Philis status: missing. The emitter is filled with a contact array (`kernel/cells/src/bjt.rs:156-176`).

### H09-11 CMOS substrate PNP generator spec
- Kind: data-model / rule
- Statement: PMoat emitter (small square) with one centred contact inside an N-well base; base contact = NMoat ring enclosing the emitter, contacted on one side only (NMoat R ≪ pinched well R); P-substrate collector with substrate contacts adjacent (ideally abutting). Larger devices: one or more minimum-width emitter strips rather than a big square. Substrate current: ≤ ~1 mA per device and ≤ ~10 mA total with nearby contacts (standard bipolar figure); > 1 mA needs substrate contacts around it. Retrograde wells may preclude it.
- Source: §9.3.1 L27771–27839, PDF 463–465; §9.2.2 L27009–27016, L27055–27062, PDF 450–451; §11.1.1 L31947–31950, PDF 537.
- Philis stage: cells, annotator, verify.
- Automation recipe: current generator already draws emitter + n-tap base ring + p-tap collector ring (`bjt.rs:1-6`, `bjt.rs:178-185`). Add: (a) the single-contact emitter variant (H09-10); (b) op-point collector current per substrate PNP; if > 1 mA, require a full collector ring with contacts on all sides (budget), else one side suffices; (c) block-level sum of substrate-PNP collector currents reported against a deck `substrate_current_max`.
- Beats hand layout because: substrate-debias exposure is summed per block from the op-point, not guessed.
- Philis status: partial. Ring geometry exists (`kernel/cells/src/bjt.rs:178-185`); no current-driven sizing (`OpPoint` is FET-only, `frontend/library/src/oppoint.rs:13-29`).

### H09-12 Multiple emitters in a common base / merged devices
- Kind: rule / deck-requirement
- Statement: Adjacent emitters in one base region interfere and lose effective area. Neutral base between emitters ≥ 2× base junction depth, and base-over-emitter overlap raised similarly (usually +a couple of µm each). Space so depletion-edge-to-depletion-edge ≥ base junction depth; if unknown, use emitter-to-emitter spacing + base junction depth. Never use a "connected-emitter" spacing rule for matched emitters; use the unconnected rule + margin (diodes: +1–2 µm). Merging collectors (common tank) is fine for verticals. Laterals: never two emitters in one collector opening.
- Source: §9.1 L25942–25947, PDF 432; §10.2.3 L30667–30709, PDF 514–515; §10.2.6 L31231–31238, PDF 523; §10.3.1 rule 11 L31381–31386, PDF 526; §11.3.1 L33645–33649; §11.3.4 rule 13 L33998–34011, PDF 570–571.
- Philis stage: cells, deck, verify.
- Automation recipe: deck keys `bjt_base_junction_depth_nm`, and use `space(emitter, emitter)` for the unconnected case. Any merged-base variant spaces emitters by `space + junction_depth` (or `+2 µm` default) and grows the base by the same. `verify` adds a custom matched-emitter spacing check on marked groups.
- Beats hand layout because: the extra spacing is applied only where matching needs it, everywhere it needs it.
- Philis status: implemented by construction for the current generator: each unit has its own base ring (`kernel/cells/src/bjt.rs:178-181`), so no shared base. Missing for any future merged-base variant.

### H09-13 Base-over-emitter enlargement for moderate/exceptional
- Kind: rule
- Statement: Drawn base should overlap drawn emitter by 1–2 µm more than the layout rule for moderately/exceptionally matched vertical transistors and matched PN diodes (misalignment-induced lateral beta variation).
- Source: §10.3.1 rule 12 L31387–31391, PDF 526; §11.3.4 rule 14 L34012–34016, PDF 571.
- Philis stage: cells.
- Automation recipe: parameter `match_margin_nm` (default 1000 nm scaled to process; for deep-submicron decks scale to a deck value `match_overlap_margin_nm`) added to `base_gap` when class ≥ Moderate. For CMOS PNPs the analogue is the emitter diffusion-to-base-well enclosure.
- Beats hand layout because: applied consistently per class, never forgotten on one device of a pair.
- Philis status: missing (`base_gap` uses deck minima only, `kernel/cells/src/bjt.rs:101-105`).

### H09-14 Cross-coupled quad for equal pairs; base contacts also common-centroid; CBE orientation
- Kind: rule / algorithm
- Statement: Differential pairs use a 2-D cross-coupled quad (½Q1, ½Q2 diagonal). Emitters **and base contacts** must share a centroid (base-contact thermoelectric potentials); collector contacts need not. Put collector contacts on the outside of the array so emitters and base contacts come closer; use CBE rather than CEB because V_BE's TC exceeds the base-contact potential's TC. Exceptional: several cross-coupled pairs in parallel forming a 2-D CC array with more dispersion.
- Source: §10.2.3 L30589–30620, PDF 511–512; §10.3.1 rule 5 L31320–31326, PDF 525; §11.3.4 rule 8 L33963–33968, PDF 570.
- Philis stage: cells, annotator.
- Automation recipe: BJT diff pair (pattern: two same-type BJTs sharing emitter net, distinct bases/collectors) → `Bjt` variant `CrossQuad` (2×2 units, owners on diagonals, units mirrored so collector straps face outward). Metric: base-contact centroid offset per device = |centroid(base contacts of A) − centroid(base contacts of B)|; budget ≤ one grid for class ≥ Moderate.
- Beats hand layout because: a secondary centroid (base contacts) that designers overlook is measured and enforced.
- Philis status: missing, and the current output is the wrong pattern. Two equal 2-unit BJTs get a 2×2 grid (`bjt.rs:38-42`), but `unit_order` gives device 0 the whole top row and device 1 the bottom row (AA/BB, one pitch of centroid offset), not the diagonal quad (H09-49). Two 1-unit BJTs are drawn side by side (AB); the generator never splits a unit into halves as Fig. 10.23 requires. Per-unit concentric base rings make base centroid equal emitter centroid per unit (`bjt.rs:178-181`), which is right.

### H09-15 BJT thermal budget: −2 mV/°C → allowed ΔT
- Kind: formula / metric
- Statement: V_BE TC ≈ −2 mV/°C ⇒ I_C TC ≈ 80,000 ppm/°C (8 %/°C). ±1 mV offset ⇔ ±0.5 °C. For an 8:1 pair (54 mV) a 1 mV error is 2 % of PTAT, produced by only 0.5 °C.
- Source: §9.1 L26020–26024, PDF 433; §10.2.3 L30414–30417, L30584–30586, PDF 509, 511.
- Philis stage: annotator, dp.
- Automation recipe: `ThermalGradient.max_delta_mc` for BJT pairs = η·(class_limit_mV − random share)/(2 mV/K) × 1000. *Derived*: Moderate ±0.5 mV with no random share → 250 mK; Minimal ±2 mV → 1 K. Deck key `vbe_tc_uv_per_k` (default magnitude 2000) per BJT model.
- Beats hand layout because: the thermal tolerance comes from the device physics and the class, and the placer evaluates ΔT from the actual power map.
- Philis status: partial. `ThermalGradient` and the superposition field exist (`kernel/analog/src/placement/thermal.rs:7-21`, `kernel/core/src/thermal.rs:1-30`) and are emitted for FET pairs (`backend/annotator/src/emit.rs:176-181`); `thermal_limit_mc` falls back to a constant for non-FETs because `by_polarity` returns `None` (`emit.rs:96-111`).

### H09-16 Separation from power devices by power class
- Kind: rule
- Statement: Vertical BJTs, laterals and diodes. Minimal: ≥ 250 µm from major power devices (≥ 250 mW) and not adjacent to any device > ~50 mW. Moderate: ≥ 100–250 µm (laterals: ≥ 250 µm) from any device > 50 mW and on the opposite side of the die from major heat sources. Exceptional: as far as possible; consider die aspect ratio 1.5:1 or 2:1; sources ≥ 1 W preclude exceptional matching unless heavily degenerated. A heat-sunk package lowers far-field gradients if separation ≥ 1 mm. ~6 V Zeners are the only low-TC exception, still kept out of large gradients (contact potentials).
- Source: §10.3.1 rule 7 L31334–31344, PDF 525; §10.3.2 rule 7 L31509–31515, PDF 528; §11.3.4 rule 9 L33969–33979, PDF 570.
- Philis stage: annotator, gp, dp, flow.
- Automation recipe: from `OpPoint.power_uw` classify each device: major (≥ 250 mW), significant (> 50 mW). Emit per matched BJT/diode group a `HeatKeepout {group, source, min_dist_nm}` budget: Minimal 250 µm from major, "not adjacent" = not neighbours in the placement graph for > 50 mW; Moderate 100–250 µm plus a half-plane constraint (group centroid and source on opposite sides of the die/block centre). This complements `ThermalGradient` (which prices ΔT, not distance) and still works when the power map is uncertain.
- Beats hand layout because: separation is enforced from simulated dissipation for every heat source, including ones a designer does not think of as "power".
- Philis status: partial. The ΔT field covers the physics (`kernel/core/src/thermal.rs:1-30`); no distance/half-plane rule by power class.

### H09-17 Thermal feedback: separate input and output stages
- Kind: heuristic
- Statement: Output-stage signal-dependent heating propagates to input devices; with gains > 10,000 even weak coupling adds low-frequency poles/zeros. Minimise by separating input and output stages (input on one side of the die, output on the other), orienting and common-centroiding the inputs; duals/quads also couple between channels.
- Source: §10.2.3 L30458–30477, PDF 510.
- Philis stage: annotator, gp.
- Automation recipe: annotator tags the input diff pair and the output-stage devices (largest `|Id·Vds|` or devices driving the output port). Add a cost term maximising distance between the two sets along one die axis, and require the input pair's symmetry axis to point at the output stage (so both halves stay on one isotherm; same principle as the existing isotherm cost in `thermal.rs:9-12`).
- Beats hand layout because: the placer trades this separation against wirelength quantitatively.
- Philis status: partial (ΔT pricing via `ThermalGradient`, `kernel/analog/src/placement/thermal.rs:7-45`; no input/output separation term).

### H09-18 Die location for stress: centre, axes, edge and corner keep-outs
- Kind: rule
- Statement: Stress gradients are smallest at die centre (highest compressive stress there), increase toward edges and corners. Matched bipolars/PN diodes: place near centre when no significant heat source; ≥ 250 µm inside die edges; never near corners; moderate/exceptional arrays with major axis on a die symmetry axis (X or Y). With a heat source: heat source on one side, matched devices on the other side, ≥ 125–250 µm from the far edge, preferably on a symmetry axis — thermal beats stress. (111) dice: locating about the <211> axis is harmless. CSP/bumped dice: best in the centre of a group of four bumps near die centre, aligned with the axis through adjacent bumps; never near corner bumps. Rigid die attach worsens gradients.
- Source: §10.2.4 L30946–30987, PDF 517–519 (Figs. 10.28, 10.29); §10.3.1 rules 8–9 L31345–31367, PDF 525; §10.3.2 rules 8–9 L31516–31526, PDF 528; §11.3.4 rules 10–11 L33980–33990, PDF 570.
- Philis stage: flow, gp, dp.
- Automation recipe: optional `DieContext {die_rect, block_origin, orientation: (100)|(111), package: plastic|csp{bumps}, external_heat: [(x,y,P)]}` input. Rules: `EdgeKeepout(group, 250 µm)` hard; `CornerKeepout` hard; `OnDieAxis(group)` cost for Moderate, budget for Exceptional (distance of array major axis from die X or Y axis); with heat sources, a `FarSide` half-plane budget. Without DieContext: report unknown, but still place matched arrays on the *block's* symmetry axis (proxy).
- Beats hand layout because: location is optimised jointly with thermal distance instead of being negotiated late at chip assembly.
- Philis status: missing. Placement only clamps to the block canvas (`backend/gp/src/lib.rs:303`); no die context, no stress/edge rules (grep `die_edge|die_axis` empty).

### H09-19 Orientation of elongated arrays relative to isotherms
- Kind: rule
- Statement: 1-D ABA arrays are elongated; orient the longer axis of symmetry (S2) parallel to the anticipated isotherms. With one power device: power device at the centre of one end of the die, ratioed pair at the other end with its S1 on the die axis. Two-row arrays: S2 parallel to isotherms.
- Source: §10.2.3 L30712–30718, L30733–30737, PDF 515.
- Philis stage: dp, cells.
- Automation recipe: the BJT cell exports S1/S2 (H09-07). A cost term: angle between S2 and the local isotherm (perpendicular to ∇T at the array centroid from `Layout::live_delta_temp` field). Candidate variants include both rotations; dp picks the orientation minimising the second-moment ΔT between devices.
- Beats hand layout because: isotherms are computed from the power map rather than assumed.
- Philis status: missing (no axis metadata; the thermal rule prices pairwise ΔT only, `kernel/analog/src/placement/thermal.rs:28-40`).

### H09-20 Piezojunction stress sensitivity and device-type choice
- Kind: formula / deck-requirement
- Statement: ΔI_S = −I_S(ζ_L σ_L + ζ_T σ_T + ζ_LT τ_LT) (Eq. 10.15); circular laterals ΔI_S = −I_S ζ_R(σ_x+σ_y) (Eq. 10.16); verticals ΔI_S = −I_S ζ_T(σ_x+σ_y) (Eq. 10.17); vertical normal stress ΔI_S = −I_S ζ_L σ_z (Eq. 10.19). Table 10.3 (10⁻¹¹ Pa⁻¹): (100) ζ_L NPN −28.4 / PNP 8.9, ζ_R 7.5 / 11.6, ζ_T 43.4 / 13.3; (111) ζ_L 28.2 / 81.5, ζ_R 21.7 / 29.7, ζ_T 15.1 / −22.2. 100 MPa → V_BE shift vertical NPN 0.4 mV (111), 0.9 mV (100); lateral PNP 0.8 mV (111), 0.3 mV (100). Rule 18: on (100) prefer vertical-PNP-based bandgaps (ζ_T ≈ ⅓ of NPN); not on (111).
- Source: §10.2.4 L30757–30867, PDF 516–517; §10.3.1 rule 18 L31439–31444, PDF 527.
- Philis stage: deck, annotator.
- Automation recipe: deck `wafer_orientation` and `piezojunction` table per device type. Annotator reports each matched BJT group's sensitivity (mV per 100 MPa) and, for bandgap cores built from NPNs on (100), an advisory to use substrate/vertical PNPs. Placement stress rules (H09-18) weight by this sensitivity.
- Beats hand layout because: device choice and location are weighed by coefficient data rather than folklore.
- Philis status: missing.

### H09-21 Package shift and filler stress mitigation
- Kind: heuristic / deck-requirement
- Statement: Plastic mold (cured 150–200 °C) compresses the die radially; package shift lowers V_BE, larger when cold. SOT-23 bandgaps: −1.1 mV mean, 2.3 mV σ (mostly filler stress). Filler stress is local (particles ≤ a few hundred µm) and location-independent → CC and placement cannot cancel it. A 15 µm Cu layer over the nitride moved a bandgap from −5.06 mV/2.64 mV σ to −2.26 mV/1.38 mV σ; polyimide overcoats reduce filler stress but can add stress between tall Cu lines. Long-term drift and thermal hysteresis limit exceptional matching; on-chip stress sensors allow correction.
- Source: §10.2.4 L30876–30922, L30987–31087, PDF 517–520.
- Philis stage: flow, deck.
- Automation recipe: for Exceptional groups, if the deck defines a thick top metal or polyimide layer, add an optional "stress shield" plate over the array (flow option), and keep other tall thick-metal lines away from its perimeter (spacing rule from deck). Report package-shift risk in the signoff report.
- Beats hand layout because: shield insertion and the tall-metal keep-out are automatic and consistent.
- Philis status: missing.

### H09-22 NBL shadow / pattern shift (BiCMOS/bipolar decks)
- Kind: deck-requirement / rule
- Statement: NBL shadow shifts up to ~2× epi thickness. If it crosses one emitter of a multi-emitter device, ratio error (2:1 example → 0.5 %, 0.13 mV). Fix: NBL overlap of emitter ≥ pattern shift + emitter outdiffusion + two-mask misalignment; if direction unknown, all sides; if magnitude unknown, 150 % of epi thickness. Laterals: NBL over inner collector perimeter ≥ 150 % epi so the shadow misses the exposed base. Or orient CEB arrays with the main axis parallel to the shift so the shadow lands in contacts. STI/CMP processes have no NBL shadow. Minimally matched verticals may skip it; Schottkys must not be crossed even when minimally matched.
- Source: §10.2.5 L31097–31168, PDF 521–522; §10.3.1 rule 10 L31368–31380, PDF 525–526; §10.3.2 rule 10 L31527–31537, PDF 528; §11.3.4 rule 12 L33991–33997, PDF 570.
- Philis stage: deck, cells.
- Automation recipe: deck keys `nbl_shadow: {shift_nm, direction?}` (absent = STI, rule off). Generator grows NBL around emitters by `shift + outdiffusion + misalign` for class ≥ Moderate.
- Beats hand layout because: a process-conditional rule is applied from deck data; STI decks carry no penalty.
- Philis status: missing; not applicable to the current STI decks (sky130, gf180mcu, ihp_sg13g2), so low priority.

### H09-23 Equal collector-to-emitter voltages (Early effect, collector efficiency)
- Kind: check / formula
- Statement: I_C = I_S exp(V_BE/V_T)(1 + V_CE/V_A) (Eq. 10.20); ΔV_CE between matched devices gives systematic mismatch ≈ ΔV_CE/V_A (Eq. 10.21; example V_A = 150 V → 0.7 %). With degeneration the error scales by V_T/(V_T+V_d) (Eq. 10.11). Verticals V_A 100–300 V → 0.3–1 %/V; laterals V_A 50–200 V → 0.5–2 %/V, plus collector-efficiency variation that becomes severe near saturation. Moderate/exceptional need equal V_CE (cascodes).
- Source: §9.1.1 L26085–26140, PDF 434; §10.2.2 L30369–30381, PDF 509; §10.2.6 L31179–31211, PDF 522; §10.3.1 rule 16 L31421–31427, PDF 526; §10.3.2 rule 14 L31570–31578, PDF 529.
- Philis stage: flow (op-point), annotator, verify.
- Automation recipe: extend `OpPoint` to BJTs (I_C, I_B, V_BE, V_CE from ngspice `@q[ic]`, `@q[vce]`…). For each matched BJT group: error_% = |ΔV_CE|·(100/V_A)·V_T/(V_T+V_d); compare with the class budget; report as a circuit-level systematic contribution (layout cannot fix it, but the tool can refuse to spend layout budget on a pair already out of spec and point to the cause).
- Beats hand layout because: every matched pair gets an explicit systematic-error audit at the actual bias.
- Philis status: missing (`OpPoint` holds FET Id, V_DS headroom, gm only, `frontend/library/src/oppoint.rs:13-29`).

### H09-24 Operate matched BJTs below high-level injection; beta plateau for diode ratios
- Kind: check
- Statement: Ratioed pairs/quads must operate well below high-level-injection onset (found where the semilog I_C–V_BE line bends). Verticals: ≤ ⅓–½ of the onset current. Laterals: ≤ 30–50 % of the first deviation; can be as low as ~1 µA per minimum emitter → use hot-dog emitters if leakage then dominates. Standard NPN beta rolls off above ~10 µA/µm² drawn emitter and below ~10 pA/µm². Diode-connected ratioed pairs (emitter-current controlled) need a beta plateau; CMOS/BiCMOS bipolars usually lack one → operate at peak beta or compensate base current; PSD/N-well ratioed diodes should be avoided.
- Source: §9.1.1 L26046–26053, PDF 433; §10.2.3 L30512–30539, PDF 510; §10.3.1 rule 13 L31392–31403, PDF 526; §10.3.2 rule 11 L31538–31549, PDF 528; §11.3.1 L33653–33675, PDF 565; §11.3.4 rule 3 L33921–33926, PDF 569.
- Philis stage: flow, annotator.
- Automation recipe: offline characterisation (like the existing `benchmarks/characterize_mismatch.py` flow for FETs): sweep each deck BJT model, fit ln I_C vs V_BE, record `hli_onset_ua_per_unit` and `beta_peak_ua_per_unit` in the deck. At run time: J = I_C/n_units; flag J > ½·onset (vertical) or > 0.3–0.5·onset (lateral); flag diode-connected ratioed pairs whose units sit outside the plateau.
- Beats hand layout because: the current-density check runs on every unit of every ratioed device with model data.
- Philis status: missing.

### H09-25 Base–emitter reverse-bias limit
- Kind: check
- Statement: E–B avalanche degrades beta (surface hot carriers). Matched transistors: reverse V_EB ≤ ~50 % of V_EBO; diode-connected transistors ≤ ~⅔ V_EBO; poly-emitter devices ≤ ~½ V_EBO (1–2 V acceptable). Pin-connected devices need clamps against ESD/transients.
- Source: §9.1.2 L26153–26159, PDF 435; §9.3.5 L28423–28433, PDF 474; §10.3.1 rule 17 L31428–31438, PDF 526–527; §11.1.1 L31903–31908, PDF 537.
- Philis stage: verify (ERC/op-point), deck.
- Automation recipe: deck `bjt_vebo_mv` per model; in the op-point (and in any provided transient bench), max reverse V_EB per matched BJT ≤ 0.5·V_EBO → violation otherwise.
- Beats hand layout because: a reliability/matching hazard that is invisible in layout is checked from simulation.
- Philis status: missing.

### H09-26 Emitter degeneration: matching-burden transfer to resistors
- Kind: formula / algorithm
- Statement: ℜ_d/ℜ = V_T/(V_T+V_d) (Eq. 10.10): 50 mV → ÷3, 100 mV → ÷6. Degeneration resistors sized for equal drop (inverse to emitter area: 1X 4 kΩ, 2X 2 kΩ, 3X 1.33 kΩ). Rules: verticals — none for minimal, may help moderate with large gradients, often for exceptional; ≥ 50 mV moderate, ≥ 100 mV exceptional. Laterals — ≥ 50 mV minimal/moderate, ≥ 100 mV exceptional; 200–300 mV to set a 10:1 ratio between unlike emitters. 200 mV usually suffices for arbitrary ratios; 250–500 mV makes transistor size irrelevant (3.4:1 via 3X+10 kΩ vs 1X+34 kΩ). Not applicable to ratioed pairs/quads or split collectors. Degeneration resistors must themselves be matched (interdigitated against gradients).
- Source: §10.2.2 L30327–30409, PDF 507–509; §10.3.1 rule 15 L31412–31420, PDF 526; §10.3.2 rule 13 L31556–31569, PDF 528–529.
- Philis stage: annotator, dp.
- Automation recipe: pattern: BJTs with shared base net, each emitter to a common node through its own resistor. From op-point V_d = I_E·R. Emit: (a) resistor group as matched array (CC/interdigitated) with its own tolerance = class tolerance; (b) BJT group budget relaxed by (V_T+V_d)/V_T; (c) a warning if V_d is below the class minimum (50/100 mV).
- Beats hand layout because: the tool re-allocates the mismatch budget between transistors and resistors from the actual bias, so neither is over- or under-constrained.
- Philis status: missing. Degeneration patterns exist for FETs only (`backend/annotator/src/catalog.rs:466-470`, `catalog.rs:627-631`).

### H09-27 Base-side ballasting against current hogging
- Kind: rule
- Statement: Parallel-base NPNs where one may saturate: insert matched base resistors sized inversely to emitter area (half R for twice the area). A diffused base ballast resistor must not sit in the NPN's own tank (it forward-biases into the tank; Fig. 9.8). Exercise design target: a deeply saturated device takes ≤ 10 % of the reference current.
- Source: §9.1.3 L26294–26349, PDF 437–438; Ex. 9.5 L28785–28787.
- Philis stage: annotator, cells, dp.
- Automation recipe: detect base resistors on parallel-base BJT sets; emit (a) ratio-matching for the resistors (inverse to unit counts), (b) a placement isolation rule: a diffused resistor may not share the tank/well of the transistor it ballasts (`Isolation` rule already exists in `kernel/analog/src/placement/isolation.rs`).
- Beats hand layout because: the tank-merge error in Fig. 9.8 becomes a hard rule.
- Philis status: missing.

### H09-28 Guard rings and substrate contacts for bipolars and diodes
- Kind: rule
- Statement: Saturating NPN or lateral PNP inject holes to substrate. NPN with > a few mA base drive: continuous deep-N+ ring merged with NBL (hole-blocking; ≥ 100:1 doping ratio) that also lowers R_C; keep it even with anti-saturation circuits. Lateral PNP: deep-N+ ring or a secondary collector (grounded = hole-collecting guard; tied to base = self-limiting; or saturation detector — beware loop stability). Substrate PNP > 1 mA: substrate contacts around it. Power diodes > a few mA: NBL/deep-N+ hole-blocking ring; > 100 mA ring width ≥ 2× epi, NBL to the ring's outer edge. Merged laterals: saturating devices in separate tanks.
- Source: §9.1.3 L26285–26291, PDF 437; §9.1.4 L26457–26459; §9.2.2 L27009–27016; §10.1.5 L30030–30116, PDF 503–505; §11.1.4 L32861–32871, PDF 551; §11.2.2 L33369–33398, PDF 559–560.
- Philis stage: annotator, cells.
- Automation recipe: extend `GuardRingRequirement` emission (currently FET-only) to BJTs and diodes: substrate PNP → p-tap collector ring (existing), with contact density from I_C; forward-biased diodes/BJTs that may saturate (op-point V_CE < ~0.3 V or diode conducting) → `Hbgr`/`Ecgr` where the deck can build it (deep n-well ring for NPN in sky130's dnwell construction).
- Beats hand layout because: ring type and density follow the op-point current, per device.
- Philis status: partial. `GuardRingType` already has `Hbgr`/`Ebgr` (`kernel/analog/src/cell.rs:26-35`) but rings are only emitted for NMOS/PMOS (`backend/annotator/src/constraints.rs:66-71`); the BJT generator draws its own collector ring (`kernel/cells/src/bjt.rs:182-185`).

### H09-29 Field plates for lateral / epi-base / shallow-well / DMOS bipolars
- Kind: rule
- Statement: Lateral PNP: metal field plate tied to the emitter covering all exposed base between drawn emitter and drawn collector, overlapping the collector by 2–3 µm; required regardless of process voltage (otherwise beta drift, I_C steps at 5–10 V). Poly works as a plate; a CMOS lateral PNP defined by a poly ring gets a self-aligned base plus plate (ring tied to emitter). BiCMOS channel-stopped laterals: still recommended. Epi-base NPN: field plate over base except its contact, touching the deep-N+ ring. Shallow-well NPN: PMoat channel-stop ring plus metal plate from emitter. DMOS NPN: poly gate plate tied to emitter. Verti-lat: plate over the semicircular tank end.
- Source: §9.2.2 L27090–27099, PDF 451; §9.2.3 L27289–27311, PDF 455; §9.2.4 L27512–27520; §9.3.1 L27843–27849; §9.3.2 L27935–27943; §9.3.3 L28008–28010, L28058–28061; §10.3.2 rule 4 L31477–31485, PDF 527.
- Philis stage: cells, dr.
- Automation recipe: lateral-bipolar generator (H09-30) draws the plate on the lowest metal (or poly) as part of the emitter net pin shape; router must treat it as an emitter-net obstruction (no other nets over the exposed base on that layer).
- Beats hand layout because: the plate is always present and always on the emitter net.
- Philis status: missing (no lateral generator; grep for field plate empty).

### H09-30 Lateral PNP generator (circular emitter, split collectors)
- Kind: data-model / algorithm
- Statement: Minimum circular emitter (just enclosing a minimum circular contact), centred in a circular hole in a rectangular collector; drawn base width = r_collector_hole − r_emitter; one collector end extended for its contact; base contact by an emitter-diffusion strip at the tank end (no sinker needed); max NBL (at least emitter to inner collector edge). Square version allowed (fillet inner corners), but only half/quarter split collectors then. Split collectors: identical segments placed symmetrically match within ±1 %; size by periphery, A_C = (P_E/P_EU)·(P_C/ΣP_C) (Eq. 9.16). Larger devices: hot-dog emitter or arrays of minimum emitters (square or hex packing), each emitter with its own identical collector opening. Matched laterals: common tank only if collector efficiency > 0.999 (exceptional) / never saturating (moderate); otherwise separate identical tanks; P-bar isolation not recommended.
- Source: §9.2.3 L27226–27479, PDF 454–458; §9.3.4 L28085–28090; §10.1.3 L29802–29808; §10.2.3 L30704–30709; §10.3.2 rules 1–6 L31456–31508, PDF 527–528.
- Philis stage: cells, annotator, deck.
- Automation recipe: new `LateralPnp` cell only for decks declaring a lateral construction (e.g. a poly-ring CMOS lateral per Fig. 9.30, or BiCMOS NBL laterals). Data model: `split: Vec<u16>` sizes in periphery units → the generator partitions the collector ring into identical arcs; `Unitization.target_ratio` per collector via Eq. 9.16. Annotator maps a multi-collector netlist device or a mirror of laterals onto it.
- Beats hand layout because: exact periphery partitions and symmetric arcs, with ratios computed not estimated.
- Philis status: missing (the BJT generator is vertical only, `kernel/cells/src/bjt.rs:1-6`).

### H09-31 Split-collector matching limits
- Kind: rule
- Statement: Split-collector laterals reach moderate matching if all collectors are identical copies and none saturates; unequal segments cannot be ratioed accurately (gaps); one saturating collector destroys matching of all others; emitter degeneration impossible (common emitter); cross-coupled split-collector pairs reported ±0.1 % (Gilbert).
- Source: §9.2.3 L27350–27373, PDF 456; §10.2.2 L30387–30389; §10.3.2 rule 5 L31486–31493, PDF 527.
- Philis stage: annotator, verify.
- Automation recipe: class cap: split-collector groups ≤ Moderate; require identical segment geometry; op-point check that no split collector saturates.
- Beats hand layout because: the ceiling on achievable matching is stated per structure.
- Philis status: missing.

### H09-32 Vertical NPN unit construction details
- Kind: rule
- Statement: Emitter overlaps its contact equally on all sides; contact as large as allowed. Base overlaps emitter for lateral punchthrough + misalignment. Base contact elongated along the full base width (one side) or on both sides (double-base, R_B ≈ ¼). Tank holds max NBL; drawn NBL touches or overlaps the deep-N+ sinker; sinker elongated. CEB gives lower R_C; CBE preferred in matched quads (H09-14) and diode-connected transistors. Stretched devices only when routing forces it (multilevel metal removes need).
- Source: §9.2.1 L26751–26881, PDF 446–448; §9.2.1 L26945–26951, PDF 449; §11.1.1 L31911–31918, PDF 537.
- Philis stage: cells.
- Automation recipe: variants `BaseContact::{Ring, OneSide, TwoSides}`; `Order::{CBE, CEB}`; pick CBE for matched groups, CEB for single high-current devices (lower R_C). Routing over devices on upper metal is preferred to stretching.
- Beats hand layout because: variant choice is tied to the device's role.
- Philis status: partial. Concentric base ring per unit (`kernel/cells/src/bjt.rs:178-181`), which is the ring form; deck has unused stripe keys (`pdks/sky130.json:51-55`, only listed in `backend/verify/src/pdk.rs:1040-1044`).

### H09-33 Compact vs narrow emitters; emitter crowding
- Kind: rule / formula
- Statement: Large compact emitters: high beta but pinched-base resistance causes current crowding (18 mV of debias doubles emitter current), hurting matching and SOA. Narrow emitter fingers with base contacts on both sides: lower beta, faster, risk of runaway at high current density. Standard NPN beta rolls off above ~10 µA/µm² drawn emitter.
- Source: §9.2.1 L26884–26951, PDF 448–449; §9.1.1 L26046–26049, PDF 433.
- Philis stage: cells.
- Automation recipe: stripe/finger variant for units larger than a deck `bjt_max_emitter_stripe` (sky130 key exists), with base contacts on both sides; selection by op-point current density.
- Beats hand layout because: emitter geometry follows current density.
- Philis status: missing (key `bjt_max_emitter_stripe` in `pdks/sky130.json:51` is unused by `kernel/cells/src/bjt.rs`).

### H09-34 Emitter ballasting sizing (inter-finger)
- Kind: formula / algorithm
- Statement: Emitter-current ratio between fingers = exp(ΔV/V_T) (Eq. 10.1). Four-finger, 400 mA, 1/2/3 mV lead drops → ~24–26 % skew. Table 10.1 gives I_E4/I_E1 for R_E = 0…1000 mΩ vs interconnect 5…100 mΩ. Rule: ballast drop 2–3 V_T = 50–75 mV (500–750 mΩ in the example) controls up to ~25 mΩ finger-to-finger interconnect; if total finger-to-finger debias cannot be kept < 100 mV, redesign (aspect ratio, more pads, more/thicker metal). Interdigitated example: ~1 □ emitter diffusion pairs at 5 Ω/□ → 2.5 Ω per finger → 50 mV at 20 mA ⇒ size fingers for ~20 mA.
- Source: §10.1.1 L28954–29077, PDF 485–487; §10.1.2 L29372–29377, PDF 492.
- Philis stage: verify (post-route), cells.
- Automation recipe: for any multi-finger BJT carrying > 10 mA (small-signal layouts OK up to ~10 mA / 100 mW, L28940–28941): build the lumped network (per-finger BJT exponential + extracted emitter metal segments + ballast R) and solve by fixed-point iteration (the book's method) to get per-finger currents; budget: max/min finger current ≤ 1.1 (derived from Table 10.1's range; tune per design), and total debias ≤ 100 mV.
- Beats hand layout because: the current distribution is solved with extracted resistances rather than estimated once.
- Philis status: missing (no ballast/power-BJT support; grep `ballast` only a test comment).

### H09-35 Intra-finger and base-lead debiasing limits
- Kind: formula / check
- Statement: ΔV_BE along a finger = L·R_S·I_E/(2W) (Eq. 10.2) ≤ 5 mV (example: 50 mA, 300 µm × 30 µm, 12 mΩ/□ Al → 3 mV). Base-lead debias ≤ 2–4 mV (interdigitated); WENC metal drops ≤ 5–10 mV; feeding a serpentine base lead from both ends cuts debias by ~4×; comb base metal beats serpentine. Fixes: second metal plate over fingers via rows of vias, more shorter fingers, wider leads, distributed ballast.
- Source: §10.1.1 L29080–29130, PDF 488; §10.1.2 L29396–29403, L29557–29562, PDF 493, 495.
- Philis stage: dr, verify.
- Automation recipe: extend the IR rule (`kernel/analog/src/routing/ir.rs:1-15`, which bounds I·R_route per net) with a distributed-current mode: for a finger lead carrying uniformly injected current, use Eq. 10.2 instead of I·R_total (the uniform-injection drop is half of I·R). Budget 5 mV emitter, 2–4 mV base.
- Beats hand layout because: every finger lead is checked with the correct distributed formula.
- Philis status: partial (IR budget exists but charges the whole current to the worst path, `ir.rs:8-15`).

### H09-36 Power BJT layout selection and current-density limits
- Kind: rule / data-model
- Statement: Linear mode: ≤ 150 µW/µm² and ≤ 10 µA/µm² emitter (conservative); switched: ≤ 20 µA/µm² (RBSOA matters); pulsed (capacitive loads): pulses ≤ 1 µs, ≥ 250 ns apart, RMS ≤ 20 µA/µm², metal per intermittent-EM rules. Table 10.2: interdigitated — FBSOA fair (with per-finger ballast), RBSOA excellent, frequency excellent; WENC — good/good/good, best all-round; Christmas-tree (and H-emitter) — excellent FBSOA, poor RBSOA, never for switching or pulsed; cruciform — excellent FBSOA, good RBSOA, most compact. WENC: emitter-over-contact 10–30 µm from Eq. 10.3 ΔV_E = R_SE·W_O²·J_E and Eq. 10.4 ΔV_B = R_SB·W_O²·J_E/β; contact must stop short of finger ends (end overlap = side overlap). Interdigitated: emitter width 8–25 µm, base contacts on both sides of all fingers.
- Source: §10.1.2 L29304–29744, PDF 491–498; §10.1.4 L29872–29909, PDF 500–501.
- Philis stage: annotator, cells.
- Automation recipe: role from op-point/testbench: linear (V_CE > 0.5 V, continuous), switched, pulsed. Emitter area = I_max / J_limit(role). Generator family `PowerBjt{style}` chosen by role per Table 10.2.
- Beats hand layout because: area and style follow the SOA role numerically.
- Philis status: missing (out of current scope; lowest priority for analog-core P&R).

### H09-37 Split and space power sections; thermal spreading
- Kind: heuristic
- Statement: Separating H-emitter banks (predrive in between) tripled FBSOA; compact cruciform devices overheat locally — split into spaced sections; RF SiGe: subdividing into widely separated small sections resists hot-spot collapse.
- Source: §10.1.2 L29614–29623, L29693–29696, PDF 496–498; §10.1.4 L29956–29959, PDF 502.
- Philis stage: gp, dp.
- Automation recipe: a device split into k sections gets a spreading cost: maximise min pairwise distance between sections subject to wirelength (ΔT field from `kernel/core/src/thermal.rs` already models superposition; add peak-T budget per section).
- Beats hand layout because: section spacing is optimised against peak temperature, not guessed.
- Philis status: missing (peak-temperature budget not present; only pairwise ΔT).

### H09-38 Collector resistance: sinker and NBL geometry
- Kind: rule
- Statement: Sinker width ≥ 2× epi thickness (outdiffusion); drawn NBL extends at least to the sinker's outer edge; sinkers on both sides cut lateral NBL R by 4×, a full ring further; long thin devices lower NBL R; omit sinker only at low current (R_C several kΩ; CDI NPN ~1 kΩ, ≤ 1–2 mA). Diode-connected NPN: > few hundred µA → sinker; ≥ 10 mA → power layout with full ring; tolerate ≤ 400 mV collector drop at 25 °C, 200 mV at 150 °C.
- Source: §9.2.1 L26732–26746, L26829–26836; §9.3.3 L27961–27971; §10.1.2 L29419–29449, PDF 493; §11.1.1 L31870–31880, PDF 536.
- Philis stage: cells, deck.
- Automation recipe: for decks with sinker/NBL layers, derive sinker width and ring choice from I_C; for sky130's dnwell NPN the analogue is the n-well collector ring (current code) — add contact density from I_C.
- Beats hand layout because: R_C is sized from current.
- Philis status: partial. Collector ring drawn on the isolating n-well ring (`kernel/cells/src/bjt.rs:121-134`, `bjt.rs:182-197`); not current-sized.

### H09-39 Voltage recognition layers (HV spacing where needed)
- Kind: deck-requirement / algorithm
- Statement: HV rules widen specific spacings (base–base, HSR–HSR, base–HSR, base–iso, HSR–iso, collector–base, collector–iso, NBL–iso, deep-N+/iso, deep-N+/base). Instead of coding everything at the highest voltage, use recognition pseudolayers (one per level except the top), either enclosing whole devices or per edge (N edges default highest, P edges default lowest). LVS verifies the layer against a schematic voltage parameter/symbol.
- Source: §9.2.4 L27579–27635, PDF 460–461.
- Philis stage: annotator, cells, dr, verify, deck.
- Automation recipe: annotator derives each net's max |V| (supplies + op-point + user); cells/dr draw the deck's HV marker over devices/wires whose nets exceed the lower class; spacing lookup becomes voltage-class dependent (deck `spacing_by_voltage`). Verify: marker ↔ net-voltage consistency.
- Beats hand layout because: HV spacing is applied exactly where the netlist says, minimising area.
- Philis status: missing (grep for recognition/HV marker empty outside annotator naming).

### H09-40 Rounded junction corners for high voltage and Zeners
- Kind: rule
- Statement: Junction breakdown drops at corners (120 V planar may break at 60 V). When a base/HSR diffusion is pushed near its limit, fillet or chamfer every 90° vertex (inside and outside); fillet radius ≥ 150 % of junction depth, chamfer segment similar; fillets beat chamfers; concentric fillets keep spacings. E-B Zeners traditionally round/oval (test by photon emission for corner breakdown). Matched Zeners: large circular geometries, circularly symmetric contacts.
- Source: §9.2.4 L27524–27554, PDF 459–460; §11.1.2 L32146–32158, PDF 541; §11.3.2 L33746–33752, PDF 567.
- Philis stage: cells, deck.
- Automation recipe: generator option `corner = Fillet{r}|Chamfer{s}` on diffusion polygons when the deck allows 45° edges; enabled when the device's net voltage class is within x % of the diffusion's BV (deck key).
- Beats hand layout because: applied to every corner of every qualifying diffusion.
- Philis status: missing.

### H09-41 Diode-connected transistor generator
- Kind: data-model
- Statement: Collector+base = anode, emitter = cathode (NPN preferred: higher beta, faster). R_S = R_B/β_F + R_E (Eq. 11.4), ≈ 10–20 Ω minimum device. CBE layout; merged collector-base contact where the deck supports it (many modern decks do not). ≈ 0.65 V at 1 µA/µm², 25 °C; +18 mV per current doubling; −2 mV/°C. Substrate-PNP diodes: collector current to substrate, > 1 mA may debias; lateral-PNP diodes lose 0.1–1 % to substrate → not for accurate current matching.
- Source: §11.1.1 L31858–31964, PDF 536–538; §11.2.2 L33316–33329, PDF 559.
- Philis stage: cells, annotator.
- Automation recipe: annotator recognises C=B BJTs (diode-connected) and keeps them in the `Bjt` generator (not `Diode`); matched diode-connected groups follow the BJT rules (H09-02…H09-25). Merged C–B contact variant only if the deck declares it.
- Beats hand layout because: diode-connected BJTs inherit the full BJT matching treatment automatically.
- Philis status: partial. BJTs are generated regardless of connection (`kernel/cells/src/bjt.rs:26-48`); no diode-connection awareness.

### H09-42 Diode class compatibility and forbidden matches
- Kind: check
- Statement: PN, Zener and Schottky diodes never match across classes. Poly diodes and surface-breakdown Zeners do not reach even minimal matching; surface Zeners walk out 50–100 mV (up to 1 V); buried Zeners better but not Pelgrom (winner-takes-all). Prefer PN/MOS stacks over Zeners for matched references. Ratioed Schottky pairs should be avoided; Al Schottkys never matched. Ideality ≤ 1.1 for minimal, < 1.05 (preferably < 1.03) for moderate ratioed pairs. PSD/N-well ratioed pairs: avoid.
- Source: §11.3 L33629–33634; §11.3.1 L33670–33702; §11.3.2 L33705–33780; §11.3.3 L33784–33882; §11.3.4 rules 1–3 L33911–33926, PDF 565–569.
- Philis stage: annotator.
- Automation recipe: device-model tags in the deck (`diode_class: pn|zener_surface|zener_buried|schottky_silicide|poly`, `ideality`). The annotator refuses to form a matched group across classes or with forbidden classes, and emits a diagnostic naming the rule; for allowed ones it caps the class (e.g. buried Zener ≤ Minimal).
- Beats hand layout because: impossible matching requests are caught at annotation instead of after fabrication.
- Philis status: missing (diode generator has no class notion, `kernel/cells/src/diode.rs:1-35`).

### H09-43 Matched diode arrays (CC, identical area-defining geometry)
- Kind: rule / algorithm
- Statement: Area-defining geometry identical (PN: the junction; field-plated Schottky: the contact opening; guard-ringed Schottky: inner perimeter of the ring); enclosing isolation matters little; several diodes may share an isolation region if only majority carriers flow there. Proximity (adjacent; never more than a few hundred µm apart), compact clusters, cross-coupled pairs for equal diodes, multiple cross-coupled pairs for exceptional. Merged PN junctions +1–2 µm beyond the unconnected rule. Matched PSD/N-well diodes: long thin PMoat strips between NMoat contacts to cut well resistance (Fig. 11.23).
- Source: §11.3.1 L33670–33682, PDF 565–566; §11.3.4 rules 4, 7, 8, 13 L33927–33968, L33998–34011, PDF 569–571.
- Philis stage: cells, annotator, dp.
- Automation recipe: `Diode` generator gets a `CrossQuad`/CC array pattern (reuse `unit_order` from `bjt.rs:205-231`) and a PSD/N-well strip form; annotator emits `CentroidGroup`, `ThermalGradient` (−2 mV/°C) and heat keep-outs for matched diode groups.
- Beats hand layout because: diodes get the same centroid/thermal machinery as transistors.
- Philis status: partial. `Diode` offers `Single`/`Interdig` mirrored rows only (`kernel/cells/src/diode.rs:20-43`); the gf180 well form is flagged unusable (`diode.rs:15-18`).

### H09-44 Schottky diode construction and matching
- Kind: rule
- Statement: Use field-relief guard rings (base or PSD ring) for matched and higher-voltage Schottkys; field plates for small low-voltage ones; operate matched Schottkys low enough that the guard ring does not conduct; ≥ 10–15 µm dimensions; square/circle; NBL + deep-N+ plug or an annular cathode contact to lower series R; deep-N+ ring as hole-blocking for amps. Contact-array processes need a fully silicided moat anode. Barrier < 0.6 V → hot leakage; clamp needs ≥ 150 mV lower V_F than the B–C junction.
- Source: §11.1.3 L32406–32770, PDF 545–550; §11.2.3 L33509–33622, PDF 562–564; §11.3.3 L33868–33882, PDF 568–569.
- Philis stage: cells, deck.
- Automation recipe: only for decks with a Schottky device (none of the current decks declare one): generator with guard ring + annular cathode.
- Beats hand layout because: consistent guard-ring geometry is required for matching.
- Philis status: missing (grep Schottky empty); low priority.

### H09-45 Zener placement and connection rules
- Kind: rule / check
- Statement: E-B Zener tank must be tied to anode, cathode or a node ≥ anode, never floating (parasitic PNP bleeds anode current when hot); buried-Zener isolation (NBL/deep-N+) must not tie to high-impedance nodes (field failure: sub-µA Zener disturbed by electron injection; fixed by grounding the isolation). Nonisolated Zeners must stay away from structures injecting > a few hundred µA. Reference Zeners are run at higher current density to reduce RTS noise; E-B Zeners current-limited per µm of periphery; PSD/N-well Zeners ≤ 1–2 µA/µm of cathode periphery, outer strips PSD, silicide-block ≥ 1–2 µm between contact and moat edge (or +1–2 µm moat-over-contact) for ballast.
- Source: §11.1.2 L32065–32070, L32121–32131, L32196–32224, PDF 538–542; §11.2.1 L33128–33147, PDF 556; §11.2.2 L33493–33500, PDF 562.
- Philis stage: verify (ERC), annotator, cells.
- Automation recipe: ERC: isolation/tank nets of Zener devices must connect to a low-impedance node (a supply/ground or a net with a DC path of < R_max, deck default); injector-distance keep-out for nonisolated Zeners (same machinery as H09-16 with substrate-current sources).
- Beats hand layout because: a known field failure mode becomes a mechanical ERC.
- Philis status: missing.

### H09-46 ESD junction diodes (NSD/P-epi, PSD/N-well)
- Kind: rule / formula
- Statement: 2 kV HBM → ~1.3 A peak, τ ≈ 220 ns; to limit ~5 V across a forward diode, R_S ≤ 3 Ω. Interdigitated narrow NMoat/PMoat strips; NSD/P-epi has inherent distributed ballast → fully silicide, max contacts and metal; pin-connected diodes need latchup precautions; adding a P-well shrinks the device but concentrates heat → area needed for a given ESD level barely changes. PSD/N-well arrays: outermost strips PSD.
- Source: §11.2.1 L33014–33066, L33082–33092, L33128–33130, PDF 554–556.
- Philis stage: cells, verify.
- Automation recipe: if Philis places pad-level diodes: size strips from R_S ≤ (V_clamp)/I_peak using deck sheet/contact R; check latchup guard requirements for pin-connected diodes.
- Beats hand layout because: series resistance is computed from the deck, not rule-of-thumb area.
- Philis status: missing; outside current analog-core scope.

### H09-47 Anti-saturation / secondary-collector structures as annotations
- Kind: data-model
- Statement: Secondary (ring) collector around a lateral PNP collects reinjected holes when the primary saturates; connect to ground (hole-collecting guard), to base (self-limiting), or as a detector into predrive (loop stability not modelled). NPN: base diffusion in collector as saturation detector; keep guard rings anyway. Symbols (Fig. 10.18): thick base bar = special layout; slash on collector = deep-N+ ring; ring on collector lead = secondary collector/detector.
- Source: §10.1.5 L30063–30159, PDF 503–505.
- Philis stage: annotator, cells.
- Automation recipe: netlist device parameter (e.g. `ringcoll=1`) mapped to a generator option; annotator treats the ring net as a guard net.
- Beats hand layout because: the special-layout intent travels with the netlist.
- Philis status: missing.

### H09-48 Beta floor for mirror-sensitive references
- Kind: check
- Statement: Trimming cancels emitter-area mismatch but not base-current mismatch; a trimmed Brokaw with a PNP mirror suffers when worst-case mirror β < 10; exceptional matching needs minimum cold-temperature β > 20 for laterals. Deep-submicron CMOS substrate PNPs may have β barely > 1.
- Source: §10.2.1 L30281–30288, PDF 507; §10.3 L31269–31275, PDF 524; §9.3.1 L27772–27777, PDF 463.
- Philis stage: flow, annotator.
- Automation recipe: from BJT op-point (H09-23) compute β = I_C/I_B at corners; flag mirror BJTs with β < 10, and exceptional groups with cold β < 20.
- Beats hand layout because: a circuit-level limit on what layout can achieve is surfaced before layout effort is spent.
- Philis status: missing.

### H09-49 In-cell centroid audit of the BJT unit array (generator defect)
- Kind: check / algorithm
- Statement: Every matched bipolar array must be common-centroid in its emitters and base contacts (H09-06, H09-14); the ratio templates are 2:1:2 for 4:1, eight-around-one for 8:1, ABA rows, two-row arrays for multiples of 4:1, and the diagonal cross-coupled quad for equal pairs (Figs. 10.23, 10.24, 10.26). A centre-out fill is common-centroid only when each device occupies complete point-symmetric sets of slots.
- Source: §10.2.3 L30589–30753, PDF 511–516; §10.3.1 rules 4–6 L31315–31333, PDF 525.
- Philis stage: cells.
- Automation recipe (the defect): `unit_order` (`kernel/cells/src/bjt.rs:208-231`) sorts slots by squared radius, then by `atan2(dr, dc)`, and hands them to members smallest-first. Inside a partly used ring the angle order runs from −180° upward, so the first slots taken are all in the top half. Traced by hand from the code (not executed):
  - `dev_nf = [2, 2]` (equal pair, 2 units each): `enumerate` gives 2 columns (`bjt.rs:39-42`), all four slots share one radius, angle order is slots 0, 1, 3, 2, so device 0 = slots 0, 1 (top row) and device 1 = slots 3, 2 (bottom row). Pattern AA/BB, centroid offset 1 pitch in y.
  - `dev_nf = [1, 4]` (4:1): 3 columns × 2 rows. Device 0 = slot 1 (row 0, col 1); device 1 = slots 4, 0, 2, 5; slot 3 is left empty. Centroids (row, col): (0, 1) vs (0.5, 1.25), offset (½, ¼) pitch, and the cell is asymmetric.
  - `dev_nf = [1, 2]` (2:1): 2×2, device 0 = (0, 0), device 1 = (0, 1), (1, 1). Offset (½, 1) pitch.
  - `dev_nf = [1, 6]` (6:1): 3×3, device 1 takes the four edge centres and the two top corners, offset ⅓ pitch in y.
  - `dev_nf = [1, 8]` works (the only tested case, `bjt.rs:271-281`).
  Fix: (1) assign slots to each device in point-symmetric pairs (slot s and its 180° image about the grid centre), so every device's first moment equals the grid centre by construction; a device with an odd unit count needs the centre slot, so it forces an odd×odd grid (or a 1-D row, ABA) and at most one device may be odd; (2) for two equal devices, deal the pairs diagonally (quad); (3) for N = 4 emit the 2:1:2 row as well as the 3×3-minus-corners option and let dp choose; (4) replace the one-ratio test with a property test over `dev_nf` in {1..4}×{1..16}: for each device, Σ(unit positions) = count × centre, exactly, and no empty slot unless the empty slots are themselves point-symmetric. Dummy units (unconnected emitters) may fill symmetric holes when the deck allows them.
  Output: a cell whose per-device centroid offset is 0 by construction, plus a `centroid_offset_nm` field in the cell metadata that `CentroidGroup` (`kernel/analog/src/placement/cc.rs:28`) can read instead of falling back to "unknown".
- Beats hand layout because: every ratio the netlist asks for gets an exact, tested centroid, including the awkward ones (6:1, 2:1) where hand layouts usually settle for "close".
- Philis status: partial with a defect: correct only for 1:8 (and trivially for one device); 1:2, 1:4, 1:6 and 2:2 are drawn off-centroid today. One-unit equal pairs cannot be cross-coupled without splitting each device; for fixed-geometry PDK bipolars (e.g. sky130 `pnp_05v5`) splitting changes the netlist, so the annotator should advise m = 2 per side instead of drawing a split.

---

## 4. Top-15 priorities for Philis

1. **H09-49 + H09-14** — Fix `unit_order` so every device's units are point-symmetric about the array centre (diagonal quad for equal pairs, odd-count device on the centre slot) and add the property test; today 1:2, 1:4, 1:6 and 2:2 bipolar arrays are drawn off-centroid. Small, local change in `kernel/cells/src/bjt.rs:208-231` that removes a systematic error from every bandgap and bipolar pair Philis draws.
2. **H09-02 + H09-01** — BJT mismatch model (k_A, Eq. 10.8/10.9) and matching classes in the annotator: every other BJT/diode rule needs a σ and a tolerance; today BJTs get none (`emit.rs:105-111`).
3. **H09-15** — BJT/diode thermal limit from −2 mV/°C into the existing `ThermalGradient` (one-line class of fix in `by_polarity`/`thermal_limit_mc`); bandgap pairs are the most thermally sensitive matched devices Philis places.
4. **H09-05** — Ratioed-pair / quad / Brokaw-core recognition with BJT slot kinds, so the pair and its PTAT resistors get constraints as one block (currently only geometric grouping in `cellgen.rs:717-731`).
5. **H09-07 + H09-19** — Ratio-specific array topologies (2:1:2, 3×3, ABA, two-row, quad stack) with exported symmetry axes and isotherm-aware orientation, built on the H09-49 fill.
6. **H09-16** — Heat-source keep-out by power class (250 µm / 100–250 µm / opposite side) using the op-point power already extracted; robust even when the ΔT model is uncertain.
7. **H09-06** — Class-driven CC hardness (budget for Moderate/Exceptional, cost for Minimal) for BJT and diode arrays.
8. **H09-18** — Die context (edge/corner keep-outs, die axes, far-side placement) with "unknown" reporting when absent; needed to claim better-than-hand placement of references.
9. **H09-08 + H09-09 + H09-10** — Unit-emitter sizing policy (2–10× minimum, σ-driven), octagon/concentric emitter-contact option, and single-contact / silicide-blocked emitters (short-emitter effect) for CMOS PNPs.
10. **H09-23 + H09-24 + H09-48** — BJT op-point extraction (I_C, I_B, V_CE, V_EB) and the checks it enables: equal V_CE, below-HLI current density, beta plateau, β floor.
11. **H09-26** — Emitter-degeneration recognition: move the matching budget onto the resistors by V_T/(V_T+V_d) and constrain them as a matched array.
12. **H09-28** — Guard-ring/substrate-contact requirements for BJTs and diodes driven by op-point current (reuse `GuardRingRequirement`, which is FET-only today).
13. **H09-42 + H09-43** — Diode class compatibility checks and CC/cross-coupled matched diode arrays (diodes currently only `Single`/`Interdig`).
14. **H09-25 + H09-45** — Reliability ERC: V_EB ≤ 50 % V_EBO for matched devices; no floating or high-impedance Zener/diode isolation tanks.
15. **H09-30 + H09-29 + H09-39** — Lateral PNP generator with emitter-tied field plate and split collectors (Eq. 9.16), and voltage-recognition markers for HV spacing; lower priority because none of the current decks (sky130, gf180mcu, ihp_sg13g2) define laterals or HV recognition layers.
