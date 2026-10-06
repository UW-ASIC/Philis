# REF-00 — Prior reference study (docs/ref/Notes HTML handbook): audit and bridge to the new plans

**Source.** `docs/ref/Notes/` — the "Philis Layout Handbook" prepared 2026-09-23 (68 entries: 50 HTML pages, 11 JSON files, `search-data.js`, `moments.js`, `notes.js`, `verify_examples.js`, `validate_notes.py`, `notes.css`, and `diagrams/` with 56 SVGs). It synthesizes 13 PDFs (2,710 physical pages) into 30 teaching chapters, 13 source companions, an 84-family constraint catalog, 115 provenance records, a code-grounded roadmap and a citation audit.

**Reftext used for this study.** Each HTML page was converted to plain text with a stdlib HTML parser (script/style/nav/aside stripped, empty list bullets and blank-line runs collapsed) into
`/tmp/claude-1000/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/scratchpad/notes_txt/<page>.txt`.
All `Lnn` cites below refer to those files (e.g. `roadmap.txt L23`). JSON files were read with `jq`. Where a cite names an original book/paper page, it is the handbook's own locator (physical PDF page, 1-based), not re-derived here unless stated.

**Code status** was checked against the working tree on 2026-09-28 (HEAD `dad330c` plus the uncommitted changes listed by `git status`). The handbook reviewed the same commit but an older working tree (its code observations are dated 2026-09-23 11:53; many files it discusses were rewritten on 2026-09-27/28, e.g. `kernel/analog/src/routing/em.rs`, `frontend/library/src/perf.rs`, `kernel/core/src/units.rs`, `kernel/cells/src/cap_array.rs`). Several of its code observations are therefore stale; §3.2 lists them.

**Exact line range read:** every line of every converted HTML page (ranges in §1), plus the JSON/JS/PY content listed in §1.2.

**Second, independent pass (2026-09-28 ~18:50).** All 50 HTML pages were converted again with a separate parser into `scratchpad/notestext/<page>.txt` and every page was re-read in full from its first `# ` heading to the end (the lines before it are the shared 30-chapter navigation block): the 30 topic chapters, 13 companions, index, roadmap, coverage, audit, constraints, glossary, and theory-provenance (lines 86–785 in full; 785–2453 with the `//` comment templates filtered, 373 remaining lines read). That pass agreed with every digest bullet and every `Philis status` below. It re-checked about 40 of the code cites (no line drift found; one cite corrected in NOTES-57) and added the nuance in NOTES-03. `Lnn` cites stay on the `notes_txt` numbering; both text sets live in the same scratchpad.

---

## 1. Coverage

### 1.1 Read chunks (HTML → text), whole files

| File | Lines read | How |
|---|---|---|
| index.txt | 1–221 | `cat` (raw form, before blank-bullet cleanup; cleaned file is 221 lines) |
| roadmap.txt | 1–104 | `cat` |
| coverage.txt | 1–214 | `cat` |
| audit.txt | 1–108 | `cat` |
| constraints.txt | 1–322 | Read |
| glossary.txt | 1–324 | Read |
| theory-provenance.txt | 1–700 (Read); 700–2373 | lines 700–2373 read with the `// …` comment-template lines and blank lines filtered out; the templates restate the record fields (claim, reference, assumptions) verbatim, as verified on H01–H25 in lines 197–650 |
| topic-optimality … topic-validation (30 chapters) | each file in full: optimality 1–135, design-intent 1–97, fabrication 1–88, constraint-systems 1–118, performance-budgets 1–106, random-mismatch 1–144, gradient-matching 1–128, mos-layout 1–108, resistors 1–110, capacitor-devices 1–96, bipolar-diodes 1–92, dac-arrays 1–116, floorplanning 1–102, placement-representations 1–100, search-algorithms 1–98, layout-sizing 1–100, parasitics 1–114, signal-routing 1–108, wire-sizing 1–99, power-distribution 1–130, substrate-isolation 1–120, thermal 1–126, electromigration 1–142, protection 1–110, rf 1–109, manufacturability 1–100, performance-loop 1–100, robust-design 1–100, verification 1–99, validation 1–115 | Read |
| Source companions | hastings 1–374, lampaert 1–156, topological 1–194, centering 1–139, substrate 1–172, electromigration 1–138, pelgrom 1–82, drennan 1–90, cc-review 1–116, nth-order 1–117, dac 1–78, performance 1–74, todaes 1–80 | Read |

### 1.2 Non-HTML files

- `audit-report.json`, `validation-report.json`, `audit-appendix-e-crosscheck.json`: every field printed and read.
- `sources.json`: 13 entries (ids, filenames, page counts, SHA-256); entries 1–12 printed in full, entry 13 (todaes) has the same schema and matches `coverage.txt L177–187`.
- `handbook-manifest.json`: keys, counts (129,035 words, 30 chapters, 13 companions, 56 diagrams, 50 pages, 436 search sections), `code_base_commit dad330c…`, first chapter/diagram records.
- `audit-hastings.json`, `audit-optimization.json`, `audit-papers.json`, `audit-physics.json`: structure; all `corrections` (13 + 11 + 23 + 31); all `unresolved` (7 + 1 + 2 + 3); all `page_reviews` (truncated at 260 chars); all numerical checks; errata checks; new citations. Their `theory_records` duplicate `theory-provenance`.
- `citation-audit.json` (553 ledger rows): status counts and definitions; all 44 `corrected` rows; all 262 `qualified` rows (detail field, truncated at 330 chars). The 206 `supported` and 41 `not_claim` rows were not read one by one.
- `theory-provenance.json`: keys, counts (115 records, 84 mappings, 13 sources), first record. Content duplicates theory-provenance.html.
- `moments.js` (read to line 80), `verify_examples.js` (first 1,500 chars), `validate_notes.py` (first 40 lines), `search-data.js` (header only; 436 sections duplicating the HTML text). `notes.js` (offline section search: every entered term must match title+text, first 40 hits) was read in the second pass; `notes.css` is presentation only. The 56 `diagrams/*.svg` `<title>` strings were listed in the second pass (e.g. `topic-mos-layout-01` "Cell reflection can preserve centroid while reversing terminal direction", `topic-signal-routing-01` "Length equality is only one route-matching feature"); each is a teaching schematic, "not PDK-valid" per its caption, so no rule is extracted from a diagram alone.
- `moments.js` patterns: AABB, ABAB, ABBA, ABBA/BAAB (2 rows), ABBA/BAAB/BAAB/ABBA (4 rows); monomials x, y, x², xy, y², x²y. `verify_examples.js` asserts ABBA moment differences `[0,0,2,0,0,0]`, AABB Δx = −2, ABAB Δx = −1, 2-row Δx²y = 1, exact cancellation through order 1/2/3 for the three CC patterns and a 90°-rotated copy (32 certificates), and that moving one unit one pitch breaks Δx = 0.

### 1.3 Every section heading in the range (file: line heading)

- **index**: L17 How to use the handbook; L32 Part 1 The design contract (L35 optimal, L40 intent, L45 fabrication, L50 constraint architecture, L55 budgets); L59 Part 2 Matching (L62 random mismatch, L67 gradients, L72 MOS, L77 resistors, L82 capacitors, L87 bipolar, L92 DAC); L96 Part 3 Placement (L99 floorplanning, L104 representations, L109 search, L114 sizing); L118 Part 4 Interconnect (L121 parasitics, L126 signal routing, L131 wire sizing, L136 power, L141 substrate, L146 thermal, L151 EM, L156 protection, L161 RF); L165 Part 5 Closure (L168 manufacturability, L173 perf loop, L178 robust, L183 verification, L188 validation); L192 Working references (L194 catalog, L198 roadmap, L202 gradient lab, L206 glossary); L212 How to interpret the recommendations.
- **roadmap**: L11 Code-review scope and the existing foundation; L19 First: repair semantic and evidence gaps; L33 Second: preserve internal units and characterize matching models; L45 Third: make parasitic evaluation terminal-aware and close the circuit loop; L55 Fourth: strengthen routing quality and resource-aware search; L69 Fifth: connect current, power, isolation and reliability models; L79 Sixth: specialized generators and manufacturing closure; L87 Milestones.
- **coverage**: L11 What was read; L19 How to assess the evidence; L29 Source inventory (L33 Hastings, L45 Lampaert, L57 Topological, L69 Centering, L81 Substrate, L93 EM, L105 Pelgrom, L117 Drennan, L129 CC review, L141 Nth-order, L153 DAC, L165 Perf survey, L177 Wire sizing); L189 Known source boundaries and apparent inconsistencies; L203 Repository context and artifact validation.
- **audit**: L11 What was audited; L19 Citation review results; L65 Corrections that matter for implementation; L85 What remains unverified; L99 How to use the audited notes.
- **constraints**: L13 How to read and instantiate; L21 INT; L49 GEO; L77 MAT; L117 NET; L151 PWR; L179 ENV; L207 DEV; L235 ARR; L257 MFG; L279 EVD; L313 Turn the catalog into a run contract.
- **glossary**: L11 Units and naming; L31 Intent, optimization and models; L73 Geometry and device representation; L117 Matching, spatial fields and environment; L169 Interconnect, noise and passive behavior; L213 Manufacturing, verification and reliability; L277 Circuit metrics and evidence.
- **theory-provenance**: L13 What a trustworthy comment should preserve; L21 Which theory supports each constraint (84-row map, L25–193); L195 Device construction (H01–H25); L651 Optimization, budgets, representations and yield (O01–O26); L1140 Mismatch, substrate, thermal and reliability (P01–P25); L1624 Matching arrays and performance research (Q01–Q26); L2109 Derivations and foundational gaps (D01–D10); L2317 Original engineering policies (S01–S03).
- **topic-optimality**: L13 decide; L19 electrical object; L32 design contract; L50 mathematical definition; L70 hard/budgets/objectives/unknowns; L80 worked example; L96 what can be called optimal; L110 how to work toward the optimum.
- **topic-design-intent**: L13 recognize role; L26 three identities; L40 derive units; L48 kind of symmetry; L56 reason and provenance; L64 recognition uncertainty; L72 current Philis behavior; L78 validation cases.
- **topic-fabrication**: L13 masks→electrical object; L24 knowledge travels with geometry; L44 drawn vs effective; L59 process fact → rule; L77 process/package choices.
- **topic-constraint-systems**: L11 executable record; L19 semantic record; L43 spec→action chain; L56 intent first; L66 identities through merging; L76 inconsistent intent vs failed search; L89 four outcomes; L99 implementation sequence.
- **topic-performance-budgets**: L11 derived allowance; L19 remaining margin; L31 local response model; L50 lowering into geometry; L60 coupled region; L71 variation and model error; L79 refresh; L87 contract and checks.
- **topic-random-mismatch**: L11; §1 L17 three variations; §2 L27 area law; §3 L39 splitting; §4 L50 correlation; §5 L67 electrical quantity; §6 L75 width tradeoff; §7 L83 distance/area budget; §8 L91 mismatch ≠ noise; §9 L97 actions; §10 L113 evaluation loop; §11 L131 applying to Philis; L137 acceptance.
- **topic-gradient-matching**: L13 difference/ratio; L19 centroid equality; L35 patterns; L49 higher order; L57 finite areas; L65 environment; L78 proximity vs dispersion; L91 construct array; L109 ratio/odd-count cases; L117 constraints and tests.
- **topic-mos-layout**: L13 matching target; L21 voltage/current/orientation/LDE; L27 orientation; L66 worked construction; L89 zero centroid is not enough; L97 validation.
- **topic-resistors**: L13 tolerable error; L19 geometry/connectivity/fields; L55 thermoelectrics; L66 unequal weights; L74 segment-count sensitivity; L87 procedure.
- **topic-capacitor-devices**: L13 which capacitance; L24 charge-transfer network; L56 wrong total ratio; L69 unit-size optimum; L79 shielding/isolation; L87 PDK inputs.
- **topic-bipolar-diodes**: L13 quantity first; L26 matching rules; L42 worked examples; L59 diode modes; L69 procedure.
- **topic-dac-arrays**: L13 contract; L38 high-weight bits; L48 odd counts; L61 pattern families; L73 plate routing; L83 settling; L91 split bridge; L97 fit in Philis.
- **topic-floorplanning**: L13; L24 hierarchy contract; L32 pad-limited example; L45 routing feasibility in placement; L55 routing finishes the circuit; L71 workflow; L91 inputs/checks.
- **topic-placement-representations**: L11; L19 coordinates; L29 sequence pairs; L52 B* / islands; L67 constraint graphs/LP/hierarchy; L77 hybrid.
- **topic-search-algorithms**: L11; L19 exact constraints in moves; L32 lexicographic feasibility; L42 annealing; L64 disjunctive regions; L77 exact optimization; L87 experiments.
- **topic-layout-sizing**: L11; L17 variables; L27 bigger device/smaller cell; L35 worked parent example; L48 nested/joint; L61 templates; L71 retargeting; L81 Philis interfaces; L89 evidence.
- **topic-parasitics**: L13; L24 resistance; L36 capacitance; L49 two widths; L59 extracted object; L75 model hierarchy; L89 multiport matching; L97 current Philis; L105 validate.
- **topic-signal-routing**: L13 contract; L21 scarce resources; L35 differential; L50 crosstalk budget; L60 shield; L75 topology; L83 high-Z nodes; L91 route costs; L99 Philis tests.
- **topic-wire-sizing**: L13; L26 worked tradeoff; L38 reliability minima; L44 granularity; L52 parallel tracks; L58 algorithm; L76 surrogate; L84 Philis boundary; L90 tests.
- **topic-power-distribution**: L11; §1 L17; §2 L25; §3 L36; §4 L48; §5 L56 Kelvin; §6 L69 AC; §7 L81 return; §8 L89 joint selection; §9 L97 loop; §10 L115 Philis; L123 validation.
- **topic-substrate-isolation**: L11; §1 L17; §2 L25; §3 L36; §4 L46; §5 L57; §6 L65; §7 L73; §8 L83; §9 L89; §10 L107; L113.
- **topic-thermal**: L11; §1 L17; §2 L25; §3 L35; §4 L43; §5 L56; §6 L64; §7 L72; §8 L85; §9 L93; §10 L111; L119.
- **topic-electromigration**: L11; §1 L15; §2 L23; §3 L38; §4 L48; §5 L56; §6 L70; §7 L81; §8 L93; §9 L99; §10 L129; L135.
- **topic-protection**: L13; L19 paths/scenarios/time; L52 antenna stage graph; L71 ESD; L83 current/heat/dielectric/aging; L93 guard rings/sharing; L101 "protected".
- **topic-rf**: L13; L24 inductor; L41 inductors/power/JFET/HV; L51 one-third rule; L61 derivation; L76 power/HV/JFET/NVM; L90 capability envelope.
- **topic-manufacturability**: L13; L24 primitives; L46 via landing example; L61 final material; L69 evidence bundle; L89 voltage-aware legality/waivers.
- **topic-performance-loop**: L13 executable spec; L26 fidelities; L42 margin→budgets; L52 search variables; L65 alternatives; L73 learned models; L83 current Philis checks; L91 completion.
- **topic-robust-design**: L11; L23 global/local/spatial; L36 covariance validity; L46 worst-case distance; L56 "three sigma"; L69 yield uncertainty; L79 physical choices; L87 Philis.
- **topic-verification**: L13 check taxonomy; L38 result states; L48 emitted geometry; L56 device identity/LVS; L64 extraction quality; L74 modes/corners; L82 current Philis; L90 certificate.
- **topic-validation**: L13 falsifiable claims; L26 experiment catalog; L62 validation ladder; L74 fair comparison; L84 ablations; L92 yield uncertainty; L100 reuse xchecks; L106 reporting.
- **hastings**: L17 coverage; L63 optimal; L73 fabrication; L93 matching quantity/centroid/environment (L99 electrical centroids, L109 CC first-order); L117 resistors (L153 thermoelectrics); L159 capacitors; L191 MOS (L197 orientation); L231 bipolar/diodes; L247 inductors/power/HV (L257 one-third); L267 reliability; L295 guard rings/sharing; L303 routing/floorplanning; L319 Philis implications; L339 validation program.
- **lampaert**: L15; L23 coverage; §1 L45 margin; §2 L59 sensitivities; §3 L77 primitive alternatives; §4 L89 moves/routing demand; §5 L103 mismatch/thermal; §6 L115 routing; §7 L129 manufacturability/testability; L139 work order.
- **topological**: L15; L21 coverage; L43 comparison table; §1 L61 Balasa; §2 L79 Lin & Chang; §3 L93 Plantage; §4 L121 routing; §5 L139 retargeting; §6 L149 layout-aware sizing; §7 L161 constraint lifecycle; L177 adoption order.
- **centering**: L15; L23 coverage; L45 parameter classes; L55 covariance; L65 sensitivities; L75 worst cases; L90 yield; L102 Monte Carlo; L112 how the design should move; L122 integration.
- **substrate**: L15; L19 coverage; §1 L49; §2 L59; §3 L75; §4 L91; §5 L97; §6 L127; §7 L135; §8 L143; §9 L151.
- **electromigration**: L15; L21 map; §1 L35; §2 L43 (L55 contract); §3 L59; §4 L67; §5 L77 (L85 temperature-factor inconsistency); §6 L89; §7 L99; §8 L105 (L109 sequence); §9 L125; L131.
- **pelgrom**: L17 coverage; L21 derivation; L35 experiment lessons; L51 placement contract; L67 bandgap; L71 engine integration.
- **drennan**: L17; L21 local statistics vs sensitivity; L35 shortcuts fail; L41 mirrors/units; L61 characterization; L69 actions; L81 validation.
- **cc-review**: L17; L23 cancellation; L33 covariance; L49 environment; L65 route graph; L75 capacitors; L85 decision procedure; L101 code; L109 coverage.
- **nth-order**: L17 claim; L23 derivation; L35 recursion; L51 ABBA insufficient; L57 moment lab; L80 non-guarantees; L90 implementation/tests; L110 coverage.
- **dac**: L17; L21 problem; L29 families; L43 routing; L51 metrics; L63 results/uncertainty; L71 coverage.
- **performance**: L17; L21 proxies; L29 three approaches; L43 labels cost; L49 transfer; L55 tradeoffs; L61 future/adoption; L67 coverage.
- **todaes**: L17; L21 width as performance variable; L27 variables/objective; L33 WAGN inputs; L41 procedure; L59 evidence/limits; L67 Philis; L73 coverage.

---

## 2. Section-by-section digest

### 2.1 Front matter: index, coverage, audit, validation

**index.txt**
- Governing principle: optimality is relative to a circuit contract, process models, operating/variation scenarios and chosen objectives; the engine must separate required feasibility, limited physical proxies and preferences (L11–13).
- Scale: 30 chapters, 84 constraints, 13 companions, 56 SVGs, ~129,000 words (L11); 2,710 physical PDF pages, "not a transcription or a claim of line-by-line reading" (L216).
- Usage order: contract → mechanisms → catalog → loop/roadmap (L17–30).

**coverage.txt**
- Books were not read line by line; companions record depth (L13). Hastings has 592 pages of accessible descriptions after the index (L15).
- Notes separate source findings, derivations, illustrative calculations, code observations and proposals (L17).
- Process-specific inputs remain required: rule decks, mismatch/correlation data, compact models, extraction stacks, reliability limits, package models, testbenches (L25).
- Known source issues: Pelgrom item is the 1988 ESSCIRC 4-page paper (L191); nth-order uses 2^n units (L195); DAC settling-frequency expression is not an RC −3 dB pole (L197); performance-survey runtime percentages are inconsistent (L199); centering/EM/Hastings equation issues flagged (L201).
- Code observations reflect commit dad330c plus the then working tree and "are not recertified as current implementation facts" (L205).

**audit.txt**
- 553 original citation occurrences: 206 supported, 262 qualified, 44 corrected, 41 not_claim, 0 unresolved (L19–31). Per-source table L33–63.
- Key corrections: false Lampaert Eq. (4.11) erratum removed (L67); CC-review MOM figure relocated to Fig. 5/PDF 4, LOD figure to PDF 698 (L69); WAGN score is not a yield (L71); derived formulas labeled as derived (L73); dummy validity, cross-sectional current density, shunt currents in tree sums, stored energy vs heat (L75); Hastings App. E, EM via temperature factor and two centering equations kept as "apparent inconsistencies" (L77).
- Unverified: no pinpoint for 4kTR or kT/C in the corpus (L87); production values must come from the PDK (L89); specialty topics are surveys (L91); proposals are unproven (L95).

**audit-report.json / validation-report.json / audit-appendix-e-crosscheck.json**
- `engine_audited: false`, `source_experiments_reproduced: false`, `full_books_read_line_by_line_in_audit: false`.
- Validation: 4,423 local references, 753 PDF page links, 56 SVGs rendered, JS moment certificates (32 exact) pass; browser layout not rendered.
- App. E cross-check: E.6 sign, E.8 prefactor (G/γ not γ/G), E.14 missing square root; exact R_total = R_Si·u·coth(u); at u = 1 metal-only error of the 1/3 rule is 6.48 % vs 1.55 % of total R.

**Domain audits (JSON)**
- Hastings: 13 corrections (locators for LOD Fig. 13.52, §2.7 contacts/CMP, failsafe PDF 769, differential-voltage spacing PDF 820–821, aging PDF 236–244), 7 unresolved (no universal accuracy; App. E unconfirmed errata; PDK magnitudes external; algorithms are synthesis; specialty coverage survey-level; no kT/C or 4kTR locator; P/(2πκr) not in Hastings).
- Optimization: OPT-C01…C11 (Lampaert Eq. 4.11 correct; sᵀΣs column convention; Elmore labeled derived; ratio centroids normalized; exact-penalty qualification; Table 6.8 +43.8 % total vs +15.1 %/iteration); unresolved OPT-U02 (contour/simplex internals not reproduced).
- Papers: 23 text corrections (floating dummies can be qualified; gm/(1+gmRs) labeled derived; DAC §V setup; WAGN wording; KCL cut sums need shunt loads; kT/C labeled derivation with source gap).
- Physics: 31 corrections (substrate locators §8.8–8.9, EM PDF 90 and §4.6.4 PDF 137, Drennan σVgs = σId/gm needs absolute current, Blech symbols, thermal energy storage, isotherm Eq. 8.23 locator); unresolved: EM Eqs. 3.25–3.26 publisher status; no PDK data in sources; claim-level audit only.

### 2.2 roadmap.txt (the prior implementation plan)
- Existing foundation acknowledged: hard/budget/cost tiers, structure recognition, legal variants and merged macros, route/placement checks, drawn-geometry verification in the epoch loop, supply/ground classes passed to dr (L13).
- Order: (1) semantic/evidence gaps, (2) unit fidelity and characterized matching, (3) terminal-aware parasitics + circuit loop, (4) routing quality/resource-aware search, (5) current/power/isolation/reliability models, (6) specialized generators and manufacturing closure (L15, L19–85).
- First-tier items: antenna gate identity (L23), structured verification coverage (L25), unknown physical inputs (L27), stable rule identity and units (L29).
- Second tier: macro unit descriptors with owner, weight, active moments, terminal map, orientation, environment, access contract (L35–37); PDK-qualified mismatch coefficients replacing a fixed ratio (L39); don't call cost-arm preferences certificates (L41); family environment descriptors (L43).
- Third tier: electrical result vector beside total C (L47); retain stack/width/topology assumptions in the C→length budget and share margins (L49); OTA integration target with caching and promotion (L51); acceptance tests incl. equal-total-C discrimination (L53).
- Fourth tier table: differential per-layer/via/RC signatures (L59); qualified layer-pair coupling + victim transfer (L61); discrete performance-driven widths (L63); corridors/pin access/multiple topologies (L65); coupled resources (L67).
- Fifth: per-net currents into dr, typed EM contract, DC/RMS/peak, crowding, electrothermal iteration (L71–73); thermal calibration/confidence (L75); substrate contact network model (L77).
- Sixth: DAC array generator with bit identity, odd-count residuals, spiral/chessboard/block (L81); sizing/refolding only with equivalence contract (L83); manufacturing closure list (L85). Milestones L87–95.

### 2.3 constraints.txt (84-family catalog)
- Roles L/E/C/B/P (legality, electrical acceptance, construction invariant, budget/proxy, preference) (L15).
- Every instance needs ID, targets, units, scenarios, bounds, parent, source/model revision, derivation, evidence; outcomes pass/fail/unknown/n.a.; missing data ⇒ unknown (L17).
- Families: INT-01…08 (L25–47), GEO-01…08 (L53–75), MAT-01…12 (L81–115), NET-01…10 (L121–149), PWR-01…08 (L155–177), ENV-01…08 (L183–205), DEV-01…08 (L211–233), ARR-01…06 (L239–255), MFG-01…06 (L261–277), EVD-01…10 (L283–311).
- Run contract: select applicable families, derive children from electrical parents, check contradictions before search, preserve C invariants, resolve every L/E or report unknown (L313–317).

### 2.4 theory-provenance.txt and glossary.txt
- Record kinds: source result, derived result, foundation with reference gap, proposed policy (L15); comment templates must keep assumptions (L17).
- 84-row mapping (L25–193) ties each family to records; e.g. MAT-03 ↔ Q01·D01·H02, PWR-04 ↔ P16·P17·P18·P23, EVD-07 ↔ O16·S02.
- H01–H25 (L197–650): area-law scope, sensitivity-weighted centroid (Eq. 8.24–8.25), CC conditions, signed MOS orientation (Eq. 13.61), WPE/LOD, dummies, fill/hydrogenation, resistor corners 0.5587 □, thermoelectrics ET = SΔTC, lead scaling, capacitor perimeter/plate roles, BJT/diode family rules, antenna stages, guard-ring mechanisms, ESD coordination, via crowding, Kelvin, pitch incl. enclosure, exact one-finger model, RF surroundings, power-MOS SOA, Fourier law.
- O01–O26 (L651–1139): additive allowance, coupled region, signed sensitivities/adjoints, R and C geometry models, Pelgrom pair model, symmetry move groups, T0 calibration, segment current/via capability, critical area, sequence-pair sufficiency, symmetry islands, DTI disjunction, shape functions, difference constraints vs LP, layout-aware sizing domain, constraint lifecycle, yield definition, covariance propagation, ellipsoid worst case, region probabilities, zero-failure bound, mean-shift centering, LS transpose correction, Lagrangian curvature, normalized centroid weights, fault detectability.
- P01–P25 (L1140–1623): Pelgrom Eq. (1)/(3), orientation evidence, Drennan independent causes and JCJᵀ, fixed-current width result, splitting, substrate contact network, EQS equation, isolation sign convention, separation saturation, guard-ring return, buried shields, differential transfer, phase-dependent budgets, EM flux divergence, Black median, current metrics, current-aware topology, pin/cross-section, via sharing, fixed-lifetime temperature scaling, Blech domain, half-space ΔT = P/(2πkr), centroid-temperature assumptions.
- Q01–Q26 (L1624–2108): CC first moments, centroid ≠ environment, conditional pattern choice, MOS source paths, MOM units, polynomial model, recursive parity, DAC plate classes/families/routing/INL-DNL/Eq. 32/MOM bridge/inherited correlation/critical-bit shift, performance proxy gap, runtime inconsistency, transfer dependence, competing objectives, WAGN inputs/output/shortlist/parallel tracks/Table 7/limits.
- D01–D10, S01–S03 (L2109–2370): weighted centroid, finite-footprint moments, gm degeneration, RC stamp, charge sharing, toy optimal width, 4kTR gap, kT/C derivation, zero-failure bound, half-LSB settling; S01 provenance contract, S02 coverage/freshness, S03 search guarantee vocabulary.
- Glossary: units (µm² = 10⁶ nm², current per width ≠ A/m²), G/D/S/B order is not an electrical definition, W/nf/m semantics are PDK-specific (L13–29); 67 terms across intent, geometry, matching, interconnect, reliability, metrics (L31–319).

### 2.5 Part 1 chapters (contract)

**topic-optimality**
- Optimal = realizable + meets electrical/manufacturing/lifetime requirements + best justified tradeoff (L11).
- Contract table: electrical intent, operating domain, manufacturing model, reliability domain, optimization policy (L32–48).
- Formal: D(L;θ)=pass, gⱼ(F(L,u,0;θ)) ≤ 0 ∀ j,u; Y(L) = Pr_ξ[all pass ∀u] ≥ Y_req; minimize a vector (L50–64).
- Normalized residual max(0,(measured−limit)/scale) with explicit positive scale (L76); margins ≠ confidence bounds (L78).
- Worked table: lowest-C candidate A (80 fF, PM 54°) fails PM ≥ 60°; B and C are nondominated (L80–94).
- Claim vocabulary: feasible under checked model, best observed, nondominated, restricted-formulation optimum, global optimum (L96–106).

**topic-design-intent**
- Motifs (diff pair, mirror, stack, clock) are evidence, not instructions (L13–17).
- Three identities: schematic device, physical unit, macro candidate (L26–38).
- Unitization needs a declared composition law (series vs parallel) and reconstruction check (L40–46).
- Symmetry types differ: mirror, CC moments, orientation balance, route impedance, environment (L48–54).
- A derived constraint answers seven questions (requirement, mechanism, objects, evidence, enforcement, inapplicability, validation) (L56–62).
- Validation cases: permuted terminal storage, body domains, different source nets, rotated ABBA macro, series/parallel ratio, do-not-identify override (L78–92).

**topic-fabrication**
- Same N region can be contact/collector/diode/guard depending on stack and bias (L13–15).
- Engine actions: store variant/well/implant/terminal semantics; drawn vs effective geometry; allowed-transform set; fill as part of candidate; technology via templates; distinguish real process features from fallbacks (L24–42).
- Equal-width units under common etch bias: 2 µm strips +0.1 µm → equal 5 %; 1 µm vs 2 µm → 10 % vs 5 % ratio error (L48).
- Translate process fact → rule in six steps with validity domain (L59–75).

**topic-constraint-systems**
- Record fields: identity, kind, targets, units, predicate, mode, tolerance, scenarios, provenance, parents, model revision, evidence (L19–37).
- Worked chain: 2 mV drop @100 µA → R ≤ 20 Ω; via 4 Ω → 16 Ω; R□ = 0.08, W = 0.4 µm → L ≤ 80 µm (L43–52).
- Common centroid with weights: Σ_A wᵢxᵢ/Σ_A wᵢ = Σ_B wᵢxᵢ/Σ_B wᵢ; bbox-area weights change meaning when halos change (L66–72).
- Price identity by kind+ordinal is fragile (L74). Mirror-axis contradiction example (xA=10, xB=170, a=100) (L76–83); half-grid symmetric coordinates need doubled-coordinate representation (L85).
- Four outcomes; unknown ≠ not applicable; zero-length unrouted net passes a C cap but proves nothing (L89–97).

**topic-performance-budgets**
- Allowed shift L − P_proc,min ≤ ΔP ≤ U − P_proc,max (Lampaert PDF 34–37) (L19–29).
- ΔPⱼ ≈ Σ Sⱼᵢ(pᵢ − p₀ᵢ); units per sensitivity (L31–37).
- PM example: 0.02·C_A + 0.05·C_B ≤ 6°; box allocation (150, 60 fF) rejects feasible (200, 20) consuming 5° (L39–48, L60–65).
- Lowering: R = R□L/W, L_max = (R_max − R_via)W/R□; R_max ≤ εV/|I|; C ≈ c_a WL + c_f L + C_c; Elmore τ ≈ Rs(Cw + CL) + Rw(CL + Cw/2) (L50–58).
- Prices from active multipliers; freeze prices within an epoch (L67–69).
- σ²_P ≈ sᵀΣ_p s (example 0.671° vs 0.539° ignoring correlation) (L71–75). Refresh triggers and trust region ‖D⁻¹(p − p₀)‖ ≤ Δ (L79–85).

### 2.6 Part 2 chapters (matching and devices)

**topic-random-mismatch**
- Three variation classes: global, local random, systematic spatial/contextual (L17–25).
- σ²(ΔP) ≈ A²/(WL) + S²D²; pair vs single-device coefficient (√2) (L31–35); Drennan σ²_L ∝ 1/W, σ²_W ∝ 1/L (L37).
- Splitting at fixed area gives no statistical gain (L39–48); Var(ΔP) = 2σ²(1 − ρ); wᵀCw (L50–65).
- Δe ≈ JΔp, Var ≈ JCJᵀ; ΔVin ≈ ΔI/gm (L67–73). Fixed-current toy: threshold contribution ∝ 1/L², independent of W (L75–81).
- Distance budget example: A = 4 mV·µm, S = 0.001 mV/µm, 100 µm² → D ≤ 100 µm; 400 µm² → D ≤ 50 µm (L83–89).
- Mismatch ≠ temporal noise (L91–95); Philis MatchingPair constant criticized (L131–135).

**topic-gradient-matching**
- p̄A − p̄B = gₓ(x̄A − x̄B) + gᵧ(ȳA − ȳB) (L19–28). Pattern table AABB/ABAB/ABBA/ABBA-BAAB (L35–47).
- Second order needs x², xy, y² separately; radial alone misses x² − y² and xy (L49–55). ABBA Δx² = 2 (L53).
- Nonlinear response (C ∝ 1/t) leaves residuals (L57–63). Environment separate from centroid (L65–76).
- Var(e) = dᵀΣd; sweep correlation lengths, check PSD (L78–89). Seven-step array construction (L91–107). Odd-count impossibility on single-site grids (L109–115).

**topic-mos-layout**
- Voltage vs current matching; −2δV_TH/V_OV (L13–19). δI ≈ Σ ∂I/∂(V_TH, β, V_DS, V_BS) (L25).
- Signed orientation Φ = (1/N)ΣΦᵢ; equal signed values required (Eq. 13.61, PDF 708–711) (L27–31).
- Action table: identical units, WPE/LOD distances, end dummies + active extension, dummy ties, gate environment, no contacts/metal over critical gates, compact CC, bias dependence, aging, pocket implants, package location, no polarity ranking (L33–57).
- Seven-step matched-pair construction (L66–82); staged model (L89–95).

**topic-resistors**
- Actions: same material/width/unit, area from random budget, parallel current directions, compact banks, edge dummies, avoid short segments/bends, segmentation search, paired opposite traversals, substrate context, overlying metal, body voltage, self-heating, lead R ratio, distance from poly-poly caps (L23–53).
- ET = SΔTC; alternate traversal cancels (Eq. 8.34, Fig. 8.22) (L55–64).
- Sensitivity weights 1,1,¼,¼ for 10k+10k+(10k‖10k); weighted mean −1.2 (L66–72).
- Segment-count example: +40 Ω/+50 Ω on 10k/15k → 0.0667 % ratio error (L74–85).

**topic-capacitor-devices**
- Evaluate extracted capacitance matrix per switch phase (L13–22, L54).
- Action table: identical units, area/perimeter for non-integer, near-square units, equal pitch CC, same under-structure, dummies on all four sides (tied), shields, lead C matching, plate assignment, dielectric choice, gradient separation (L28–52).
- Lead example 203/102 → −0.49 % (L56–67). Unit-size optimum (L69–77). Shield needs low-impedance return (L79–85).

**topic-bipolar-diodes**
- Name the matched output (VBE, ratio, forward drop, breakdown) (L13–19).
- Vertical/lateral/PN/Zener/Schottky rules (L26–40).
- ΔVBE = VT ln N; 1 % area error → 258 µV; β 100 vs 80 → 64 µV; −1.8 mV/K × 0.05 K = 90 µV (L42–52).
- Collecting guard must intercept carrier path in 3D (L54–67). Seven-step procedure (L69–87).

**topic-dac-arrays**
- Binary bank 2^N units incl. electrical dummy; split bank separate top nodes (L13–36).
- DNL(k), INL(k) definitions; MSB +0.04 unit → DNL +0.0348 LSB (L38–44). kT/C labeled as derivation with source gap (L46).
- Odd-count C0/C1 cannot be exact CC (L48–59). Spiral/chessboard/block families (L61–71).
- Bottom plates: BFS components, branches, trunks, bridges; top plate MST (L73–81).
- t ≥ (N+1)ln2·τ vs paper f = 1/[2(N+2)ln2·τ] (L83–89). Bridge ΔC/C ≈ 2δ(1/H + 1/l) (L91–95).

### 2.7 Part 3 chapters (placement/search)

**topic-floorplanning**
- Macro interface contents: transforms, access, supplies, domains, guard interfaces, roles, thermal scenarios, exclusions (L24–30).
- Pad-limited example: 40 pads @100 µm + 400 µm → 1,400 µm; core 900 + 200 → 1,100 µm (L32–38).
- Channel accounting example (10 tracks → 3 free) (L45–53). Kelvin/star, noise, voltage spacing, capacity, thermal/stress, whole die, finalization checklist (L55–69).

**topic-placement-representations**
- Coordinates admit overlap; sequence pairs decode to nonoverlap; 6×5 worked packing (L19–38).
- Symmetric-feasible SP condition is sufficient, not necessary (L50). B* islands (L52–65). Difference constraints vs 4-variable symmetry equalities (L67–71). Hybrid comparison table (L77–89).

**topic-search-algorithms**
- Compound pair moves preserve xA + xB = 2a (L19–30). Lexicographic gate vs penalties; batch-count sensitivity (L32–40).
- Metropolis: T0 = −mean(ΔE⁺)/ln p0; freeze energy per epoch; keep archive (L42–62).
- DTI disjunction g ∈ [0,s_max] ∪ [d,∞); MIP g ≥ dz, g ≤ s_max + (g_max − s_max)z (L64–75). LP for fixed topology; area is bilinear (L77–85).

**topic-layout-sizing**
- Separate electrical from implementation variables; W_total = N_f·W_finger does not prove equivalence (L17–25).
- gm/C ∝ 1/√W at fixed L (derived) (L29). Worked: 72 µm² cell 54.9 MHz fails, 84 µm² cell 88.4 MHz passes 70 MHz (L35–46).
- Nested vs joint sizing; templates need domains; m^n table growth (L48–69). Retargeting preserves intent (L71–79).

### 2.8 Part 4 chapters (interconnect/reliability)

**topic-parasitics**
- R_path = ΣR_wire + ΣR_via + ΣR_contact; meshes need network solve (L24–34).
- Capacitor stamp +c/−c; (G + jωC)v = i (L36–47).
- Two-width example: gm,eff 4.55 → 4.76 mS vs pole 159 → 99.5 MHz (L49–57). Extraction steps (L59–73). Model hierarchy table (L75–87). Multiport matching |R_pos − R_neg| ≤ ΔR_max (L89–95).

**topic-signal-routing**
- Reserve pin access, supplies/returns/shields, pairs, then negotiation (L21–33).
- Equal length/layer set can hide RC mismatch (L35–48). ΔV_victim ≈ Cc/(Cv + Cc)·ΔV_agg; 1 fF/19 fF → 50 mV (L50–58).
- Shield as connected conductor (L60–73); route topology controls voltage distribution (L75–81); signed edge costs break Dijkstra assumptions (L91–97).

**topic-wire-sizing**
- τ(w) = (R_d + a/w)(C_L + bw) → w* = √(aC_L/(R_d b)) (L26–36). EM minima vs performance preferences (L38–42).
- Per-net vs per-segment granularity; paper uses per-net discrete, symmetric nets equal (L44–50). Parallel tracks ≠ one wide wire (L52–56). Seven-step algorithm (L58–74).

**topic-power-distribution**
- Classify nets by function (bias lines, references, returns) (L17–23). Tree KCL: 2 + 1 + 0.5 = 3.5 mA trunk (L25–34).
- 200 µm × 2 µm × 0.08 Ω/□ = 8 Ω → 28 mV; forward + return 39.25 mV (L36–46).
- gm·ΔV sensitivity: 1 mV at gm 2 mS = 2 % of 100 µA (L48–54). Kelvin (L56–67). |Z| ≤ ΔV/ΔI; C ≥ IΔt/ΔV (L69–79). Loop area/returns (L81–87). Joint width/layer/via/topology (L89–95).

**topic-substrate-isolation**
- Injection → propagation → reception (L17–23). Low-resistivity bulk makes distance saturate (L25–34).
- Grounded guard ring can inject via inductive return (L36–44); shared return drawing (L46–55).
- EQS PDE ∇·(σ∇φ) + ∂t∇·(ε∇φ) = 0 (L57–63). Differential: v_diff = (H1 − H2)v_agg; 1 % imbalance of 0.01 → −80 dB (L73–81). Latch-up separate (L83–87). Eight-step algorithm (L89–105). Philis Isolation distance only (L107–111).

**topic-thermal**
- Absolute T and ΔT are separate requirements (L11–15). Heat equation; ΔT = R_th P (L25–33).
- ΔT = P/(2πkr) derivation and limits (L35–41). Equal exposure beats proximity; ΔT ≈ ∇T·Δr (L43–54).
- 10 mW, k = 148: 1.075/0.538/0.269 K at 10/20/40 µm; 1 mV/K × 0.537 K = 0.537 mV (L56–62).
- Electrically weighted temperatures Δe ≈ ΣsᵢΔTᵢ (L64–70). Electrothermal loop (L72–83). Philis: hottest-member group, distance trial cost, zero-power ambiguity (L111–117).

**topic-electromigration**
- j_avg = I/(wt); 1 mA in 1×0.5 µm = 2×10⁵ A/cm² (L23–36). Branch waveforms before metrics (L38–46).
- 10 mA @10 % duty → 1/3.16/10 mA avg/RMS/peak; widths 1/1.58/2 µm; via cuts 3 → 4 with crowding 1.5 (L48–54).
- Fixed-lifetime scaling j_allow(T)/j_allow(T_ref) = exp[(Ea/nk)(1/T − 1/T_ref)]; 373 → 398 K gives 0.209 lifetime / 0.458 current (L56–68).
- Via arrays share unequally (L70–79). Blech |jL|_crit = ΩΔσ/(e|Z*|ρ), domain not route vertices (L81–91). Philis EM_UA_PER_UM criticized (L129–133).

**topic-protection**
- Mechanism table: self-heating, filamentation, EM, dielectric breakdown, antenna, ESD, latch-up, HCI/BTI, surface charge (L19–41).
- Antenna stage graph; 180 → 20 ratio with M2 bridge (L52–60). Confirmed nets[1] gate bug (L64).
- ESD path: 7 V + 0.8 V + 0.5 V = 8.3 V > 8 V (L79). Guard-ring types (tap/collecting/blocking) (L93–99).

**topic-rf**
- Inductor Q ≈ ωL/R: 12.6 → 8.4 with +1 Ω lead (L30). One-third rule and exact R_Si·u·coth u (L51–69). HV/annular/JFET/NVM require qualified primitives (L76–88). Capability envelope procedure (L90–104).

### 2.9 Part 5 chapters (closure)

**topic-manufacturability**
- Check taxonomy: width/neck, spacing, enclosure, notch/EOL, grid, density/fill, recognition, voltage-dependent geometry (L24–44).
- Via landing 80 + 2×30 = 140 nm vs 200 nm pitch conflict (L46–54). Odd widths put edges on half grid (L52).
- Fill changes C/stress/hydrogen; iterate fill→check→extract (L61–67). Evidence bundle (L69–87). Voltage-aware spacing uses max joint ΔV incl. startup (L89–95).

**topic-performance-loop**
- rᵢ = (fᵢ − uᵢ)/qᵢ, max over scenarios ≤ 0 (L13–24). Fidelity table and promotion by proven bounds (L26–40).
- 2x + y ≤ 10 shared vs caps (L42–50). Search variable levels (L52–63). Pareto archive (L65–71). WAGN as threshold classifier (L73–81). Philis: DRC/ERC/LVS/PEX per epoch, total-C scalar (L83–89).

**topic-robust-design**
- Y(x_d) = Pr_xs[∀x_r: all bounds pass] (L15–21). Shared g cancels: σ = 4.24 vs 28.6 mV if treated independent (L23–34).
- PSD test counterexample (ρ = −0.9 ×3) (L36–44). β = (U − f₀)/σ_f; β = 2 vs 4 example (L46–54). Ball/box/half-space probabilities 98.889/99.461/99.865 % (L56–67). Zero-failure bound 1 − 0.05^(1/N) (L69–77).

**topic-verification**
- Check taxonomy table (L13–36). States: pass/fail/n.a./unknown/evaluation failed (L38–46).
- Verify expanded polygons, late changes (L48–54). LVS recognizer omissions must be visible; G = D hides gate/drain swaps (L56–62).
- Philis: signoff treats skipped/refused stages as failures; unrecognized devices go to stderr; skipped rules logged once; boolean budget failure with 0 residual adds 0 to Θ (L82–88). Certificate contents (L90–94).

**topic-validation**
- Falsifiable claims (L13–24). 15-row experiment catalog (L26–60). Ladder: algebra → structures → circuits → full flow → silicon (L62–72).
- Fair comparisons, seeds, cache keys (L74–82). Ablations and counterexamples (L84–90). Zero-failure bound ≈ 3/n (L92–98). xcheck_pex.py re-computes the same formula; not independent for R/coupling (L100–104).

### 2.10 Source companions (digest only; each source has its own ref-plan study)
- **hastings** (L17–369): coverage table by chapter (L23–57); sensitivity-weighted centroid Eq. 8.24–8.25 (L101–107); CC is first-order (L109–115); resistor/capacitor/MOS/BJT tables duplicate chapters; direct Philis implications table (L319–337); 12-item validation program (L339–365).
- **lampaert** (L15–151): "guaranteed" performance claims are model predictions (L21); allowed shift (L45–57); two-sided normalized violation proposal vj = max(0,(L − P̂)/s,(P̂ − U)/s) (L67–71); variant descriptors (L77–87); separate move groups (L89–101); historical 10–15 % area penalty vs manual layout (L101); σ² = A²/(WL) + S²D²; summed |S|·3σ is not a σ (L103–113); segment physics routing, A* admissibility caveat (L115–127); critical area A(χ) = L·max(0, χ − s), λ = ∫A D dχ, yield e^−λ (L129–137).
- **topological** (L15–189): approach comparison table (L43–59); SP symmetric-feasible condition (L61–77); islands (L79–91); SMP/HSMPG priorities, center-based symmetry LP equations, DTI binaries, shape functions (L93–119); physical connectivity and net splitting, mirror routing, template domains (L121–137); retargeting (L139–147); layout-aware sizing and Table 6.8 denominators (L149–159); constraint lifecycle (L161–175).
- **centering** (L15–134): design centering ≠ geometric centering (L19); parameter classes (L45–53); C = DRD, whitening (L55–63); sensitivity scaling (L65–73); box/ellipsoid/nonlinear worst case, Hessian correction (L75–88); yield joint event, χ² ball 46.8 % at n = 10, β = 3 (L90–100); MC variance, zero-failure bound, importance sampling (L102–110); mean-shift gradient, LS transpose correction (L112–120).
- **substrate** (L15–167): isolation = loaded transfer (L57); model-choice table (L59–73); five-step budgeting (L75–89); activity signatures (L91–95); action/counterexample table incl. thin rings with good returns, 5 % differential mismatch (L97–125); Sherman–Morrison incremental updates (L135–141); repo integration (L143–149).
- **electromigration** (L15–133): reading map (L21–33); reliability contract per metal/via (L55–57); current classes (L59–65); branch current = signed cut sum incl. shunt loads (L67–75); via sharing and landing templates (L77–83); printed Eq. 3.25–3.26 inconsistency (L85–87); Blech/reservoirs/bamboo conditional (L89–97); precharacterized routing-pattern library (L99–103); invariants (monotonic width vs current/temperature/lifetime) (L125–129).
- **pelgrom** (L15–77): Eq. (1) and Eq. (3) (L21–33); 2.5 µm process A_VT0 30/35 mV·µm, S ≈ 4 µV/µm, S/A ≈ 1.33×10⁻⁴ µm⁻² (L43); D ≤ ηA/(S√WL), η = 0.3 raises σ by 4.4 % (L57); bandgap budget example (L67–69); MatchingPair S/A constant stronger than evidence (L73).
- **drennan** (L15–85): σ²_L ∝ 1/W etc. (L21–27); σ²_e ≈ Σ(∂e/∂pᵢ)²σ²_pᵢ (L29–33); σVgs = σId/gm in absolute units (L37); mirror geometry/polarity/splitting table (L41–55); Table III 3.86/1.52/3.05/1.41 % (L59); characterization interface (L61–67).
- **cc-review** (L15–111): CC cancellation assumptions (L23–31); dᵀΣd; splitting counterexample (L33–47); WPE/LOD/OD/gate-pitch table (L49–63); source-degeneration and route graph (L65–73); MOM structures (L75–83); six-step decision procedure (L85–99).
- **nth-order** (L15–112): 2^n units cancel polynomial fields to order n (L17–21); Mₐᵦ moments, radial insufficiency (L23–33); recursive parity construction, k·2^(n−1) (L35–49); ABBA 2q residual; ABBA/BAAB still has x²y (L51–55); certificate algorithm with cross-multiplication (L90–108).
- **dac** (L15–73): unit counts, split topology (L21–27); family table (L29–41); BFS/MST routing, parallel wires shift critical bit (L43–49); Eq. 32 frequency is not 1/(2πτ) (L55–59); Table II 1411 vs 339 MHz (not re-verified here); inherited ρ = 0.9, 1 mm, 10 ppm must not become defaults (L63–69).
- **performance** (L15–69): constraint proxies (L21–27); three approaches (L29–41); runtime percentages inconsistent (L43–47); transfer is metric-dependent (L49–53); MOBO vs weighted BO (L55–59).
- **todaes** (L15–75): Table 7 CM-OTA 1×→3×: UGF 451.0 → 511.4 MHz, gain 33.1 → 31.1 dB, FOM 0.92 → 0.90 (L25); FOM min(z/φ,1) (L27–31); WAGN features (L33–39); procedure with 10-candidate simulated shortlist (L41–57); limits (L59–65).

---

## 3. Actionable extraction

### 3.1 Status of every roadmap item (roadmap.txt) against the current tree

| # | Roadmap item (cite) | Status now | Evidence |
|---|---|---|---|
| R1 | Antenna gate identity: gate area on `nets[1]` (L23) | **Fixed** | `backend/annotator/src/extract.rs:33-38` uses `nets[0]` ("Terminal 0 is G"); stage-aware per-layer check `kernel/analog/src/routing/stack.rs:284-317`; deck stages `backend/verify/src/pdk.rs:572-610`; diode insertion `frontend/library/src/lib.rs:632-635`. Still positional, not a named-terminal accessor. |
| R2 | Structured verification coverage (L25) | **Missing** | Unrecognized devices and skipped rules only go to stderr: `backend/verify/src/lib.rs:110-116`, `:176-180`; reference builder returns a count `backend/verify/src/reference.rs:11`. |
| R3 | Unknown physical inputs recorded (L27) | **Partial** | `Problem::missing` `backend/annotator/src/lib.rs:46-71`; `Rule::known` `kernel/analog/src/rule.rs:55-59`; `RuleBatch::unknown` `rule.rs:174`; thermal "all-zero power is unknown" `kernel/analog/src/placement/thermal.rs:49-54`; unmeasured spec = full miss `frontend/library/src/perf.rs:72-80`; op provenance string `frontend/library/src/oppoint.rs:25-26`. Gap: an unresolved device still gets `power_uw = 0` (`oppoint.rs:175-184`). |
| R4 | Stable rule identity, units, provenance (L29) | **Missing** | Prices still bound by `(kind, ordinal)`: `kernel/analog/src/requirements.rs:16-18`, `backend/gp/src/lib.rs:25`, `:115-116`. Normalized residual exists (`kernel/core/src/report.rs:40-48`). |
| R5 | Macro unit descriptors (owner, weight, moments, orientation, environment) (L35-37) | **Implemented (core)** | `Unit{owner,x,y,weight,phi,sa,sb}` `kernel/core/src/units.rs:14-33`; `UnitLib` transforms with orientation `units.rs:101-122`. Missing: route-access contract, well/body environment per unit. |
| R6 | Process-qualified mismatch coefficients (L39) | **Implemented** | `MatchingPair.gradient_per_avt_um2` from deck, unknown when 0: `kernel/analog/src/placement/matching_pair.rs:17-37`, `:52-54`; deck values with provenance `pdks/gf180mcu.json:14-23`, `pdks/ihp_sg13g2.json:9-18`. Not bias-aware (see NOTES-18). |
| R7 | Matching/centroid/proximity only in cost arm (L41) | **Fixed** | Budget + cost arms: `backend/annotator/src/emit.rs:19-24`. |
| R8 | Family environment descriptors (L43) | **Partial** | MOS LOD per unit `units.rs:28-32`, LOD ΔVT in `kernel/analog/src/placement/cc.rs:36-40`, `:119-141`; WPE/OSE `kernel/analog/src/placement/environment.rs:6-47`. Resistor/capacitor/BJT descriptors absent. |
| R9 | Electrical result vector beyond total C (L47) | **Implemented, not default** | `frontend/library/src/perf.rs:1-8`, `:168-187`; spec-miss tier in `lex_key` `frontend/library/src/lib.rs:941-957`. Only exercised in `frontend/library/tests/perf_postlayout.rs` and `examples/op_demo`; `verify` still sets `report.cost = checker.total_cap_ff()` (`backend/verify/src/lib.rs:182`). |
| R10 | Budget provenance + shared margins (L49) | **Partial** | Coupled per-spec rows `kernel/analog/src/routing/performance.rs:8-33` from FD sensitivities `perf.rs:189-262`; wiring `lib.rs:192-227`. C only, nominal schematic point only; no R budget (`kernel/analog/src/routing/parasitic.rs:13-15`). |
| R11 | OTA integration target, caching, promotion (L51) | **Partial** | Promotion: simulate only if hard count can beat incumbent `lib.rs:356-360`. No cache keyed on geometry/model/testbench. |
| R12 | Acceptance tests (equal-total-C, PM failure) (L53) | **Partial** | Design supports it (`perf.rs:3-8`); no benchmark-level test found beyond `perf_postlayout.rs`. |
| R13 | Differential per-layer/via/RC signature (L59) | **Implemented (no coupling)** | `kernel/analog/src/routing/differential.rs:10-37`. |
| R14 | Qualified layer-pair coupling + victim transfer (L61) | **Partial** | Per-layer deck ε·t, summed over aggressors, same-layer only: `kernel/analog/src/routing/coupling.rs:14-28`, `:61-63`. |
| R15 | Discrete performance-driven widths (L63) | **Partial** | "Width follows need" (EM/IR) in `backend/dr/src/lib.rs:650-669`; no sensitivity-driven width search. |
| R16 | Route corridors / pin access / post-route feasibility in placement (L65) | **Partial** | Ring halos reserved `lib.rs:1195-1199`; routing negotiation history `gr::Negotiation` only feeds dr (`lib.rs:342`, `:566`); `gp::place` gets prices but no congestion (`lib.rs:575`). |
| R17 | Operating currents into dr; typed EM contract (L71) | **Implemented (DC)** | Tree branch currents `backend/dr/src/lib.rs:597-605`; per-layer deck limits, derating, Blech, cuts `kernel/analog/src/routing/em.rs:11-65`; hard rule `em.rs:78-96`; op currents → IrDrop `lib.rs:285-288`. |
| R18 | DC/RMS/peak, crowding, electrothermal (L73) | **Missing** | "DC average only" `em.rs:90`; one die temperature `frontend/library/src/elaborate.rs:242-248`. |
| R19 | Thermal calibration, confidence, weighted ΔT (L75) | **Partial** | Live-field trial pricing `kernel/core/src/thermal.rs:70-95`, `placement/thermal.rs:7-13`; still fixed k `thermal.rs:9-14`, hottest-member groups `thermal.rs:57-68`. |
| R20 | Substrate contact-network model (L77) | **Missing** | Distance only: `kernel/analog/src/placement/isolation.rs:7-17`; `ISOLATION_EPI_MULTIPLE = 4` `backend/annotator/src/emit.rs:250-256`. |
| R21 | Capacitor-array generator (L81) | **Implemented (binary/general)** | Spiral/Chessboard/BlockChessboard, dummy ring, INL/DNL metrics `kernel/cells/src/cap_array.rs:1-75`. Split DAC, bridge sizing and settling missing. |
| R22 | Sizing/refolding with equivalence contract (L83) | **Partial** | Schematic finger count preserved `kernel/cells/src/mosfet.rs:74-85`; fold floor per `docs/LAYOUT-FUNDAMENTALS.md:163-166`. No sizing transaction. |
| R23 | Manufacturing closure list (L85) | **Partial** | Fill `frontend/library/src/fill.rs:1-30` (tied, keep-outs); antenna done (R1); via arrays in dr; voltage-dependent spacing absent (no match in `backend/verify/src`); no re-extraction after fill (`lib.rs:403-420`). |
| R24 | Milestones (L87-95) | **Partial** | Certificates (R2 missing), matching (R5–R8 partial), electrical selection (R9 partial), broader search (R16 partial), robust closure (missing). |

### 3.2 Claims in the notes that are stale, imprecise or in tension with the sources

1. **Stale code observations (fixed since 2026-09-23).** `roadmap.txt L23`, `topic-protection.txt L64`, `hastings.txt L333` (antenna `nets[1]`) — fixed at `extract.rs:37`. `topic-random-mismatch.txt L133`, `pelgrom.txt L73` (`GRADIENT_PER_AVT_UM2 = 2.5e-4`) — now a deck input (`matching_pair.rs:34-37`). `topic-electromigration.txt L131`, `electromigration.txt L107` (`EM_UA_PER_UM = 1000`, scalar `net_current_ua`) — replaced by per-layer limits and tree currents (`em.rs:23-60`, `dr/src/lib.rs:597-669`). `topic-thermal.txt L115`, `hastings.txt L329` (distance trial cost) — now live ΔT (`placement/thermal.rs:28-33`). `topic-gradient-matching.txt L119`, `cc-review.txt L103` (bbox centroid) — now unit-weighted with unknown fallback (`cc.rs:48-62`, `:164-168`). `topic-parasitics.txt L101`, `cc-review.txt L107` (Differential = length + layer set) — now per-layer signature / stack RC (`differential.rs:10-27`). `topic-parasitics.txt L101` (one stack-wide coupling constant) — now per-layer deck ε·t (`coupling.rs:20-28`). `topic-performance-loop.txt L85`, `performance.txt L27` (final scalar is total C) — a spec-miss tier now precedes C (`lib.rs:941-957`), though C remains the last tier.
2. **Coverage gap: two sources not in the handbook.** `coverage.txt L11` claims all supplied PDFs (13, 2,710 pages). `docs/ref/` now also holds `Comparison_of_distance_mismatch_and_pair_matching_of_CMOS_devices.pdf` and `Precise_characterization_of_long-distance_mismatch_of_CMOS_devices.pdf` (both newer than the notes). The decks already take S_VT from the first (`pdks/gf180mcu.json:22-23`, `pdks/ihp_sg13g2.json:17-18`). Any distance-budget guidance in the notes (P01, `pelgrom.txt L57`) should be reread against those two papers (reftext `distance_vs_pair_mismatch.txt`, `long_distance_mismatch.txt`).
3. **H19 locator is imprecise.** `theory-provenance.txt` H19 cites Hastings "§15.4.4, Eqs 15.24–15.25 and Fig 15.29" for inner-corner via crowding. In the source, Eq. 15.24 is the minimum EM width and Eq. 15.25 is the temperature derating factor (hastings reftext L48765–48790, printed p. 820; example: 0.58 at 398 K). The crowding claim rests on the figure discussion, not those equations. The claim is right; the equation locator is misleading. (The code cites them correctly for derating: `elaborate.rs:242-244`.)
4. **Isolation rule in tension with P11.** `theory-provenance` P11 and `substrate.txt L101` warn that the multiple-of-epi-thickness separation is a process-specific example, not a universal rule. The code emits exactly `4 × t_epi` (`emit.rs:246-283`) and itself notes sky130 is bulk, where the plateau does not hold (`emit.rs:251-255`). The notes are right; the code is honest but still leans on it.
5. **Radial second moment is insufficient (notes vs code).** `nth-order.txt L29` and `topic-gradient-matching.txt L51` require x², xy and y² separately. `cap_array` scores `quad_um2` from ⟨r²⟩ and INL/DNL under `q·r²` only (`kernel/cells/src/cap_array.rs:60-66`). That misses anisotropic `x² − y²` and `xy` fields.
6. **Verified numbers.** Spot-checked against reftext and correct: Pelgrom A_VT0 30/35 mV·µm (pelgrom reftext L103) and S_VT0 ≈ 4 µV/µm (L90); bandgap 19/18/1.6/1.7 mV (L130–142); todaes Table 7 (todaes22 reftext L794–801); performance survey 92.89 %/7.11 % (perf_driven_survey reftext L183–184) and figure 22.55 %/75.42 % (L268, L275). Every worked arithmetic example in the chapters recomputes correctly (distance budget, sequence-pair 6×5, DNL 0.0348, EM 0.209/0.458, pulse/via 3→4, thermal 1.075 K, supply 39.25 mV, one-third 1.0981 Ω, β 0.997302/0.998134, zero-failure 0.000998). DAC Table II (1411/339 MHz) and Drennan Table III percentages could not be re-read from the garbled reftext and are **not verified here**.
7. **"Unresolved: 0" should be read narrowly.** `audit.txt L29` reports zero unresolved citation occurrences, while `audit.txt L85–95` and the domain JSONs list 13 unresolved scope items (noise-law gaps, errata status, PDK data). The zero refers to the ledger only, as the audit says (L97).

### 3.3 Entries

### NOTES-01 Executable design contract per run
- Kind: data-model
- Statement: Every run stores (a) electrical intent (terminal meanings, exact ratios, matched relations, bias domains, clocks, sensitive nodes), (b) operating domain (supply, temperature, load, modes, startup), (c) manufacturing model (PDK revision, extraction, LDE and variation models), (d) reliability domain (lifetime, mission profile, waveforms), (e) optimization policy (which specs are mandatory, tie-break order). A comparison is valid only under the same contract.
- Source: topic-optimality.txt L32–48 (synthesis of Lampaert PDF 20, CC review PDF 6–7, wire-sizing PDF 19); S01 (theory-provenance L2317+).
- Philis stage: flow, annotator.
- Automation recipe: extend `LibraryConfig` with a `Contract` struct (specs = `perf::Spec`, scenarios = list of `(supply, temp, corner)`, policy = ordered tie-break list); hash it into every `Epoch`; refuse to compare epochs across contracts.
- Beats hand layout because: the tool can prove that every candidate was judged against the same, complete, versioned spec set; hand layout relies on the designer's memory of conditions.
- Philis status: partial — `perf::PerfConfig{sim, testbench, specs}` `frontend/library/src/perf.rs:51-66`; single scenario; policy hard-coded (`C_TIE` footprint tie `lib.rs:916-935`).

### NOTES-02 Four-state outcomes; unknown never passes
- Kind: rule
- Statement: Each rule evaluation returns pass / fail / not-applicable (with a physical reason) / unknown (missing input) / evaluation-failed. Missing current, power, temperature, extraction or process data ⇒ unknown for dependent rules; a default numeric zero must not read as a pass.
- Source: constraints.txt L17; topic-constraint-systems.txt L89–97; topic-verification.txt L38–46; S02.
- Philis stage: verify, annotator, all rule crates.
- Automation recipe: keep `Rule::known`/`RuleBatch::unknown`; add `Status` enum to `metadata::MetadataReport` rows; make the final certificate require `unknown == 0` for mandatory families; make `OpPoint.power_uw` `Vec<Option<i32>>`.
- Beats hand layout because: an explicit ledger of unchecked items is produced automatically; humans silently skip checks they lack data for.
- Philis status: partial — `kernel/analog/src/rule.rs:50-59`, `:174`; `backend/annotator/src/lib.rs:46-71`; `placement/thermal.rs:44-54`; gap `oppoint.rs:175-184` (unresolved device → 0 µW).

### NOTES-03 Structured verification coverage certificate
- Kind: check
- Statement: Per candidate, record which DRC/LVS/ERC/PEX rules ran, which were skipped (with reason), which device families lacked recognizers (count and identities), and block certification when a required recognizer/rule is missing.
- Source: roadmap.txt L25; topic-verification.txt L56–62, L82–94; EVD-07 (constraints.txt L301–302).
- Philis stage: verify, flow.
- Automation recipe: return `Coverage{skipped_rules: Vec<(rule, why)>, unrecognized: Vec<device>}` from `signoff_with_intent`; attach to `Epoch`/`Solution`; add `engine/coverage` hard violation when a required device kind is unrecognized.
- Beats hand layout because: hand signoff decks silently ignore unrecognized devices; the tool can make omissions a failing, named item.
- Philis status: missing for these two items — stderr only `backend/verify/src/lib.rs:110-116`, `:176-180`. The container already exists: `metadata::MetadataReport` carries `missing` families and per-family `unknown` counts, and certification requires both to be empty (`frontend/library/src/metadata.rs:80-81`, `:106-108`; test `:436`). The fix is to push unrecognized device kinds and skipped signoff rules into that report instead of printing them.

### NOTES-04 Stable semantic IDs, units and provenance for every constraint
- Kind: data-model
- Statement: Each constraint instance carries stable ID, kind, targets, units, scope, enforcement tier, tolerance, scenarios, provenance (recognition rule, source record), parent requirement, model revision, evidence status. Price state binds by ID, not by (kind, ordinal).
- Source: topic-constraint-systems.txt L19–41, L74; roadmap.txt L29; S01.
- Philis stage: annotator, gp/dp (prices), flow (reports).
- Automation recipe: add `RuleMeta{id: u32, parent: Option<u32>, source: &'static str}` alongside each batch in `Requirements`; key `gp::Prices` on `id`; regression: insert a new same-kind batch and check prices stay attached.
- Beats hand layout because: every emitted geometry decision can be traced to a circuit reason and re-derived after an edit.
- Philis status: missing — `requirements.rs:16-18`; `gp/src/lib.rs:115-116`.

### NOTES-05 Parent→child lowering with invalidation
- Kind: algorithm
- Statement: Derive geometric children from electrical parents explicitly, e.g. ΔV ≤ 2 mV at 100 µA ⇒ R ≤ 20 Ω; with 4 Ω via, 16 Ω wire; R□ = 0.08 Ω/□, W = 0.4 µm ⇒ L ≤ 80 µm. Invalidate the child when width, layer, via count, topology or current changes; keep the parent.
- Source: topic-constraint-systems.txt L43–54; topic-performance-budgets.txt L50–58 (Lampaert Eq. 2.33, PDF 51).
- Philis stage: annotator, gr/dr.
- Automation recipe: store `(parent_id, assumptions{layer,width,vias,current})` on each `ParasiticBudget`/`IrDrop`; dr re-derives or evaluates parent directly when assumptions differ from realized geometry.
- Beats hand layout because: limits stay consistent with the realized geometry after every change.
- Philis status: partial — C budgets now evaluated on real stack C (`parasitic.rs:29-36`); IR drop from real path R (`ir.rs:8-27`); no parent links.

### NOTES-06 Pre-search contradiction check with conflict core
- Kind: check
- Statement: Before search, detect impossible fixed equalities (xA + xB = 2a with fixed xA = 10, xB = 170, a = 100 ⇒ a = 90 contradiction), empty variant domains, inconsistent ratios, impossible bounds, half-grid symmetric coordinates; report the minimal conflicting set instead of escalating penalties.
- Source: topic-constraint-systems.txt L76–87 (Topological PDF 299–302).
- Philis stage: annotator, gp.
- Automation recipe: build the linear equality system from `mirror_pairs`, fixed flags and axis constraints; Gaussian elimination on doubled coordinates; on inconsistency, emit a `Conflict` report naming the rules.
- Beats hand layout because: overconstraint is found in milliseconds, not after hours of failed runs.
- Philis status: missing — no conflict detection found in `backend/annotator/src`, `kernel/analog/src`, `frontend/library/src`.

### NOTES-07 Evidence freshness: re-extract after fill and repairs
- Kind: rule
- Statement: Fill, slots, shields, redundant vias and power reinforcement are candidate mutations; they invalidate parasitic, matching and performance evidence. Final certificate must refer to the final (post-fill) geometry hash.
- Source: topic-manufacturability.txt L61–67; topic-verification.txt L48–54; MFG-01, EVD-08.
- Philis stage: flow, verify.
- Automation recipe: after `fill::fill`, rerun `signoff_with_intent` and `perf::evaluate` on the filled geometry; report pre/post deltas; fail if a spec flips.
- Beats hand layout because: every final number is from the shipped geometry.
- Philis status: partial — fill is applied once to the winner and "the search never sees it" (`frontend/library/src/lib.rs:403-420`); metadata/perf reported are pre-fill (`lib.rs:421-438`).

### NOTES-08 Honest optimality claims and a feasible Pareto archive
- Kind: metric
- Statement: Report which claim holds: feasible under checked model; best observed (with seeds/budget); nondominated among evaluated; restricted-formulation optimum; never "global optimum". Keep a bounded archive of feasible nondominated candidates (area, C, spec margins, power).
- Source: topic-optimality.txt L96–106; topic-performance-loop.txt L65–71; S03; Graeb (centering.txt L65–73).
- Philis stage: flow, benchmarks.
- Automation recipe: keep `Vec<Epoch>` of nondominated feasible epochs; selection policy declared in the contract; report termination reason (budget exhausted, stall, converged).
- Beats hand layout because: exposes real tradeoff alternatives a human would not draw.
- Philis status: partial — single incumbent with `C_TIE` footprint tie (`lib.rs:916-935`), `RunStats.converged` (`lib.rs:378-381`); no archive.

### NOTES-09 Scenario-aware performance margin allocation
- Kind: formula
- Statement: For spec L ≤ P ≤ U and pre-layout process/scenario range [P_proc,min, P_proc,max], allowed layout shift L − P_proc,min ≤ ΔP ≤ U − P_proc,max, valid only if the shift is common; otherwise per-scenario bounds. Example: PM ≥ 60°, nominal 70°, worst scenario 66° ⇒ 6° available, not 10°.
- Source: topic-performance-budgets.txt L19–29; lampaert.txt L45–57 (Lampaert §2.2 Eqs. 2.1–2.7, PDF 34–37; O01).
- Philis stage: flow (perf), annotator.
- Automation recipe: run `perf::sensitivities` at each scenario (corner × temperature); headroom = min over scenarios; one `PerformanceBudget` row per (spec, scenario).
- Beats hand layout because: margin already consumed by process corners is never spent twice by routing.
- Philis status: partial — headroom at the nominal schematic point only (`perf.rs:230-262`).

### NOTES-10 Coupled sensitivity-weighted parasitic budget
- Kind: formula
- Statement: Σᵢ Sⱼᵢ(pᵢ − p₀ᵢ) ≤ headroomⱼ per spec j; do not split into independent per-net caps (box (150, 60 fF) rejects feasible (200, 20) with 0.02·200 + 0.05·20 = 5° ≤ 6°). Signs kept; C that helps a metric earns credit.
- Source: topic-performance-budgets.txt L31–48, L60–69; O02, O03.
- Philis stage: gr, dr, annotator.
- Automation recipe: already `PerformanceBudget`; extend `p` to series R per terminal branch and coupling C per net pair; recompute rows when topology leaves the trust region.
- Beats hand layout because: the router trades parasitics across nets by measured sensitivity.
- Philis status: implemented (C only) — `kernel/analog/src/routing/performance.rs:8-33`; rows `perf.rs:230-262`; wiring `lib.rs:192-227`.

### NOTES-11 Post-layout extracted simulation as a lexicographic tier
- Kind: check
- Statement: Evaluate the extracted circuit (C matrix, branch R, LOD) under the testbench; normalized miss rᵢ = (fᵢ − uᵢ)/qᵢ; any miss or unmeasured metric makes the candidate infeasible before area/C are compared.
- Source: topic-performance-loop.txt L13–24, L83–89; roadmap.txt L47–53; EVD-01.
- Philis stage: flow, verify.
- Automation recipe: make `performance` part of every benchmark fixture (OTA, bgr, dac4), not only tests; cache results keyed on (geometry hash, model, testbench).
- Beats hand layout because: every candidate is simulated post-layout; humans simulate one final layout.
- Philis status: implemented but opt-in — `perf.rs:168-187`, `lib.rs:941-957`, `lib.rs:356-360`; used only by `frontend/library/tests/perf_postlayout.rs` and `examples/op_demo`.

### NOTES-12 Multi-fidelity promotion with proven bounds
- Kind: algorithm
- Statement: For f ≤ U: a proven upper bound ≤ U certifies pass; a proven lower bound > U certifies fail; otherwise promote to higher fidelity. Candidates close relative to model error are both promoted.
- Source: topic-performance-loop.txt L26–40.
- Philis stage: flow.
- Automation recipe: fidelity ladder: placement estimate → routed stack RC → extracted PEX → ngspice; store per-stage model error; skip simulation only when the cheap bound is decisive.
- Beats hand layout because: compute is spent where the answer is uncertain.
- Philis status: partial — promotion only on hard-violation count (`lib.rs:356-360`).

### NOTES-13 Sensitivity refresh triggers and trust region
- Kind: heuristic
- Statement: Refresh sensitivities after topology change, resize, variant/finger change, operating-region change, new aggressor, or active scenario change; trust region ‖D⁻¹(p − p₀)‖ ≤ Δ; shrink when prediction error exceeds tolerance (1° predicted vs 3° actual ⇒ shrink).
- Source: topic-performance-budgets.txt L79–85; lampaert.txt L73–75; centering.txt L65–69.
- Philis stage: flow.
- Automation recipe: after each post-layout sim, compare Σ S·Δp prediction with actual metric change; if error > 30 % of headroom, recompute `sensitivities` around the extracted point.
- Beats hand layout because: the budget model stays calibrated to the current layout.
- Philis status: missing — sensitivities computed once from the schematic (`perf.rs:189-228`, `lib.rs:216`).

### NOTES-14 Correlated-parasitic reserve σ²_P = sᵀΣ_p s
- Kind: formula
- Statement: With sensitivity column s and parasitic covariance Σ_p, σ_P² ≈ sᵀΣ_p s (0.2² + 0.5² + 2·0.2·0.5·0.8 = 0.45 deg², σ = 0.671° vs 0.539° uncorrelated). Screen P_pred − kσ_P − e_model ≥ L; systematic model bias is not merged in quadrature.
- Source: topic-performance-budgets.txt L71–77 (Graeb PDF 111, 183; O18).
- Philis stage: flow (perf).
- Automation recipe: estimate Σ_p from seed-to-seed extraction spread; subtract kσ from headroom in `budget_rows`.
- Beats hand layout because: reserves are quantified, not guessed.
- Philis status: missing.

### NOTES-15 Terminal-resolved parasitic network
- Kind: data-model
- Statement: Keep node-resolved C matrix (ground and mutual, +c/−c stamp) and per-terminal branch R; equal total C on different nodes is different circuits (20 fF on high-Z vs driver node).
- Source: topic-parasitics.txt L13–47, L59–73; D04.
- Philis stage: verify, dr, flow.
- Automation recipe: already carry `CapMatrix` and `Parasitics.series`; add cell-internal li/licon R (per `docs/LAYOUT-FUNDAMENTALS.md:161-162`), and coupling between nets into the perf deck.
- Beats hand layout because: every node's loading is known and simulated.
- Philis status: implemented (partial R) — `backend/verify/src/lib.rs:65-67`; `perf.rs:16-27`; `kernel/analog/src/routing/stack.rs:153-167`.

### NOTES-16 Pelgrom distance budget with deck S/A and circuit η
- Kind: formula
- Statement: σ²(ΔP) = A²/(WL) + S²D² (pair coefficients). Allow σ_grad/σ_rand ≤ η ⇒ D ≤ ηA/(S√(WL)); η = 0.3 raises total σ by √1.09 − 1 = 4.4 %. Larger devices tighten D (100 µm² → 100 µm; 400 µm² → 50 µm at A = 4 mV·µm, S = 0.001 mV/µm).
- Source: topic-random-mismatch.txt L27–37, L83–89; pelgrom.txt L21–33, L51–57 (Pelgrom Eq. 1, PDF 1–2; P01).
- Philis stage: annotator, gp/dp.
- Automation recipe: as implemented; derive η from `offset_sigma_mv` when given; unknown when deck lacks S.
- Beats hand layout because: distance limits scale with actual device area and process data.
- Philis status: implemented — `matching_pair.rs:17-77`; `emit.rs:45-52` (η default 0.3); `backend/annotator/src/lib.rs:54-59`.

### NOTES-17 Mismatch coefficient convention (pair vs single, √2)
- Kind: deck-requirement
- Statement: Store whether A is a single-device or pair-difference coefficient; pair = √2 × single for independent devices; never multiply blindly.
- Source: topic-random-mismatch.txt L35; pelgrom.txt L27; P01.
- Philis stage: deck.
- Automation recipe: require an `avt_convention: "pair" | "single"` field next to `avt_*_mv_um`; convert once at load.
- Beats hand layout because: removes a silent 41 % error class.
- Philis status: partial — decks document pair values in the `_source` strings (`pdks/ihp_sg13g2.json:9-10`: "per device (× √2 = 5.5 per pair)"); no typed field.

### NOTES-18 Bias-aware mismatch propagation (JCJᵀ, σVgs = σId/gm)
- Kind: formula
- Statement: Δe ≈ JΔp, Var(e) ≈ JCJᵀ with J from the compact model at the actual bias; input-referred σVgs = σId/gm with absolute σId. Toy: at fixed current, threshold-driven relative current variance ∝ 1/L², independent of W.
- Source: topic-random-mismatch.txt L67–81; drennan.txt L21–39 (Drennan Eqs. 6–11, PDF 2–3; P05, P06).
- Philis stage: annotator, flow.
- Automation recipe: use `OpPoint.gm_us` and `id_ua` to convert A_VT area terms into offset per pair; allocate η from the circuit offset spec; flag area-only rankings for mirrors.
- Beats hand layout because: the tool budgets mismatch in the metric the circuit actually cares about.
- Philis status: partial — `oppoint.rs:12-28` provides gm; `emit.rs:59-64` (`systematic_allowance_mv`); no JCJᵀ.

### NOTES-19 Electrical unit centroids, not bounding boxes
- Kind: check
- Statement: Centroids use active-unit positions weighted by electrical contribution (Σwᵢxᵢ/Σwᵢ); halo or outline changes must not move them; ratio groups normalize by nominal total.
- Source: topic-gradient-matching.txt L19–28; topic-constraint-systems.txt L66–72; roadmap.txt L35–37; D01, O25.
- Philis stage: cells, gp/dp.
- Automation recipe: as implemented; add the test "change halo only ⇒ centroid unchanged; move one unit ⇒ changes".
- Beats hand layout because: exact weighted moments of every unit in every variant.
- Philis status: implemented — `kernel/core/src/units.rs:14-122`; `cc.rs:48-62`, unknown fallback `cc.rs:164-168`.

### NOTES-20 Higher-order moment certificate
- Kind: check
- Statement: For polynomial field order n, match normalized Mₐᵦ = Σwᵢxᵢᵃyᵢᵇ/Σwᵢ for every 1 ≤ a + b ≤ n (x², xy, y² separately; radial x² + y² is insufficient). ABBA: Δx² = 2 (pitch²); ABBA/BAAB: zero through order 2, Δx²y = 1.
- Source: topic-gradient-matching.txt L49–55; nth-order.txt L23–33, L51–55, L90–108 (Dai et al. §II–III, PDF 2–3; Q06, D02).
- Philis stage: cells, gp/dp, verify.
- Automation recipe: add second-moment terms to `CentroidGroup::used` weighted by deck/assumed curvature; in `cap_array` metrics replace ⟨r²⟩ with the three second moments and sweep anisotropic q_xx, q_xy, q_yy; integer cross-multiplied equality for exact certificates.
- Beats hand layout because: exact certificates of which gradient orders cancel.
- Philis status: missing in placement (`cc.rs:84-88` ponytail); radial-only in `cap_array.rs:60-66` (flag §3.2 item 5).

### NOTES-21 Nth-order recursive central-symmetric pattern
- Kind: algorithm
- Statement: Start with a verified first-order pattern (k units/device); reflect about a new center; keep labels for odd target order, swap A/B for even; units = k·2^(n−1). Check every monomial afterwards.
- Source: nth-order.txt L35–49 (§III, PDF 2–3; Q07).
- Philis stage: cells (mosfet, cap_array, resistor).
- Automation recipe: offer `Pattern::Cc2` (ABBA/BAAB) and `Cc3` (4-row) variants when device can be split into 4/8 legal units; score route cost; enable only when a curvature coefficient or sweep justifies it.
- Beats hand layout because: higher-order patterns are generated and certified without manual bookkeeping.
- Philis status: partial — MOS 1-D CC and 2-row variants (`kernel/cells/src/mosfet.rs:87-130`); no recursion.

### NOTES-22 Signed orientation equality (Φ)
- Kind: check
- Statement: Φ = (1/N)ΣΦᵢ, Φᵢ = ±1 by S→D direction; matched groups need equal signed Φ in both axes (3R1L = +½ equals 9R3L = +½). Geometric mirroring can reverse Φ.
- Source: topic-mos-layout.txt L27–31 (Hastings Eq. 13.61, PDF 708–711; H04).
- Philis stage: cells, dp.
- Automation recipe: extend the check from within-macro to across separately placed cells of one matched pair using `PlacedUnit.phi` after orientation; hard for Load/Mirror pairs.
- Beats hand layout because: orientation drift under hierarchy rotation is caught automatically.
- Philis status: partial — within merged macros `frontend/library/src/cellgen.rs:255-270`; not a placement rule across cells (`phi` unused in `kernel/analog/src`).

### NOTES-23 LDE environment equality (LOD, WPE, OSE, gate pitch)
- Kind: check
- Statement: Matched units need equal SA/SB (LOD), well-edge distance (WPE), OD spacing/width, gate pitch; zero centroid error can coexist with ABBA inner/outer LOD difference.
- Source: topic-mos-layout.txt L33–41; cc-review.txt L49–63 (§II-A2, Fig. 2, PDF 2; Q02); H06.
- Philis stage: cells, dp, flow (perf deck sa/sb).
- Automation recipe: as implemented for LOD/WPE/OSE; add PSE (poly spacing) and use BSIM WPE parameters (SCA/SCB) when the deck gives them; emit sa/sb/sca per instance to post-layout sim.
- Beats hand layout because: measured per-finger environment, including neighbors from other cells.
- Philis status: partial/implemented — `units.rs:28-32`; `cc.rs:36-40`, `:119-141`; `environment.rs:6-47` (ENV_TOL = 0.2 design tolerance); `perf.rs:24-50` (equivalent SA).

### NOTES-24 Qualified, tied dummies and active extension
- Kind: rule
- Statement: End dummies at array pitch with valid ties (polarity/well-appropriate rail); active extension to distance STI; capacitor arrays ringed on all four sides with tied dummies; resistor end dummies with ties.
- Source: topic-mos-layout.txt L37–41; topic-capacitor-devices.txt L40; topic-resistors.txt L33; H07.
- Philis stage: cells.
- Automation recipe: MOS and cap done; add resistor end dummies tied to a quiet net; verify dummy ties via LVS/ERC.
- Beats hand layout because: never forgotten, always consistent.
- Philis status: implemented (MOS, caps) — `mosfet.rs:14`, `:50-55`, `:270-293`; `cap_array.rs:5-9`; resistor dummies missing (`docs/LAYOUT-FUNDAMENTALS.md:185-188`).

### NOTES-25 Pattern competition judged electrically
- Kind: heuristic
- Statement: Generate clustered, interdigitated, 1-D CC and 2-D CC candidates where legal; rank by extracted circuit error, not by pattern name (small CC structures can lose to clustered when parasitic/LDE mismatch exceeds gradient benefit).
- Source: cc-review.txt L85–99 (§V, PDF 6–7; Q03); topic-gradient-matching.txt L35–47.
- Philis stage: cells, flow.
- Automation recipe: keep variants in `VariantSpace`; let perf tier decide between variants for sensitive groups; enable ABAB when drain boundaries can be separated.
- Beats hand layout because: every pattern is tried and simulated.
- Philis status: partial — `mosfet.rs:87-130` (interdig withheld because shared diffusion shorts drains, `mosfet.rs:87`); variant choice via lex key.

### NOTES-26 Covariance-based dispersion metric Var(e) = dᵀΣd
- Kind: metric
- Statement: With d = signed normalized contributions and Σ the unit covariance (PSD-checked), Var(e) = dᵀΣd; reordering independent identical units at fixed area changes nothing; correlation makes dispersion matter. Sweep correlation length; never adopt a paper's ρ = 0.9 or 1 mm as a default.
- Source: topic-gradient-matching.txt L78–89; cc-review.txt L33–47; dac.txt L63–69 (Q14).
- Philis stage: cells (cap_array), dp.
- Automation recipe: in `cap_array::metrics` compute dᵀΣd for exponential correlation over a swept length set; report worst over the sweep.
- Beats hand layout because: quantitative dispersion vs routing trade.
- Philis status: missing.

### NOTES-27 Sensitivity-weighted resistor centroid and segmentation
- Kind: formula
- Statement: X_R = Σ(∂R/∂Rᵢ)Xᵢ / Σ(∂R/∂Rᵢ) (Hastings Eq. 8.24–8.25); series weight 1, member of equal parallel pair ¼. Unit counts must scale with nominal values, else per-segment end resistance changes ratio (+40/+50 Ω on 10k/15k → 0.0667 %).
- Source: topic-resistors.txt L66–85; hastings.txt L99–107 (PDF 390; H02).
- Philis stage: cells (resistor), annotator.
- Automation recipe: compute Unit weights as ∂R/∂Rᵢ from the device's series/parallel composition; prefer identical segment values across ratioed resistors.
- Beats hand layout because: exact weights for mixed networks.
- Philis status: partial — resistor Unit weight = body W·L (`kernel/cells/src/resistor.rs:124`), correct only for equal series segments.

### NOTES-28 Thermoelectric traversal pairing
- Kind: check
- Statement: E_T = S·ΔT_C per segment; order series segments so contact-temperature EMFs alternate sign (E − E + E − E = 0); a CC score cannot detect this.
- Source: topic-resistors.txt L55–64 (Hastings Eq. 8.34, Fig. 8.22, PDF 401–402; H10).
- Philis stage: cells (resistor), verify.
- Automation recipe: record each segment's current direction; require equal count of up/down traversals per matched resistor; with a thermal field, sum signed ΔT at contacts.
- Beats hand layout because: invisible connectivity errors are checked on every layout.
- Philis status: missing (no match for thermoelectric/Seebeck in code).

### NOTES-29 Capacitor lead and switch-phase matrix matching
- Kind: check
- Statement: Match leads by extracted capacitance (not length); evaluate the C matrix per switch phase; lead parasitics in parallel break ratio (203/102 = 1.9902, −0.49 %).
- Source: topic-capacitor-devices.txt L44, L54–67; H13.
- Philis stage: cells, verify, flow.
- Automation recipe: extract per-plate terminal C from `CapMatrix`; compute ratio error per bit; add as budget.
- Beats hand layout because: ppm-level ratio errors from routing are measured.
- Philis status: partial — TOP never crosses bottom-plate routes, so C_TB is structurally absent (`cap_array.rs:12-17`); no ratio check from extracted matrix.

### NOTES-30 DAC pattern families with INL/DNL metrics
- Kind: algorithm
- Statement: Spiral (low dispersion, few vias), chessboard (max dispersion, many branches), block chessboard (intermediate); keep nondominated set on INL/DNL, route spread, vias, area.
- Source: topic-dac-arrays.txt L61–71; dac.txt L29–49 (Karmokar §IV-A, PDF 7–8; Q09, Q10).
- Philis stage: cells.
- Automation recipe: as implemented; select by perf tier when a DAC testbench exists.
- Beats hand layout because: exhaustive family enumeration with metrics.
- Philis status: implemented — `kernel/cells/src/cap_array.rs:27-75`.

### NOTES-31 Odd-count residual, split DAC and bridge sensitivity
- Kind: rule
- Statement: One-unit C0/C1 cannot be exact CC on a single-site grid; report residual. Split DAC keeps LSB/MSB top nodes separate, bridged only by the attenuation cap; bridge geometry matches ΔC/C ≈ 2δ(1/H + 1/l) of the unit.
- Source: topic-dac-arrays.txt L13–36, L48–59, L91–95 (Karmokar §II, §III-D, PDF 2–6; Q08, Q13).
- Philis stage: cells, annotator (recognize split DAC).
- Automation recipe: add split-DAC recognition and `Pattern::Split`; size bridge by solving target C and equal fractional edge sensitivity, then extract.
- Beats hand layout because: correct architecture preservation and bridge sensitivity matching.
- Philis status: partial — odd-count residual reported (`cap_array.rs:56-59`); split DAC and bridge missing.

### NOTES-32 DAC settling and critical-bit reevaluation
- Kind: formula
- Statement: Single-pole half-LSB settling t ≥ (N + 1)ln2·τ; the paper's f = 1/[2(N + 2)ln2·τ] is a separate metric; after widening/paralleling the slowest bit, recompute all bits (critical bit moves).
- Source: topic-dac-arrays.txt L81–89 (Karmokar Eq. 32, PDF 5; D10, Q12, Q15).
- Philis stage: cells, dr, flow.
- Automation recipe: extract per-bit bottom-plate R and C; τ_bit = R·C; require t_settle ≥ (N + 1)ln2·max τ; loop widen/parallel until stable.
- Beats hand layout because: every bit's delay measured each iteration.
- Philis status: missing — only proxies `route_spread`, `vias` (`cap_array.rs:68-72`).

### NOTES-33 Bipolar ratio arrays and current-definition errors
- Kind: formula
- Statement: ΔVBE = VT ln N; 1 % area error at N = 8 → 258 µV; β 100 vs 80 at equal emitter current → 64 µV; 0.05 K at −1.8 mV/K → 90 µV. Use identical unit emitters, centered 1:N, dummy ring.
- Source: topic-bipolar-diodes.txt L42–52; H14.
- Philis stage: cells (bjt), annotator.
- Automation recipe: bjt generator: 1:N centered array (N + 1 = 9 → 3×3 with the single unit at center), dummy ring, Units with weights; thermal budget from bandgap sensitivity.
- Beats hand layout because: centered, dummy-ringed arrays generated for every N.
- Philis status: missing (`docs/LAYOUT-FUNDAMENTALS.md:189` "BJT ratioed arrays: still open").

### NOTES-34 Compound symmetry-preserving moves
- Kind: algorithm
- Statement: Move the axis (translate pair), change pair separation symmetrically, swap compatible pairs; projection only for general constraints; coupled equalities solved simultaneously.
- Source: topic-search-algorithms.txt L19–30; lampaert.txt L89–95 (O06).
- Philis stage: dp.
- Automation recipe: as implemented; add reachability tests on enumerated small cases.
- Beats hand layout because: symmetry never breaks during optimization.
- Philis status: implemented — `backend/dp/src/lib.rs:304-324`, `:434-510`.

### NOTES-35 Lexicographic gate and separate release predicate
- Kind: rule
- Statement: Gate on (hard violations, hard residual, budget residual, objective); counts must not depend on batching; an aggregate residual must not hide one worsened mandatory rule; final release checks each mandatory rule against its own tolerance.
- Source: topic-search-algorithms.txt L32–40; topic-verification.txt L88.
- Philis stage: dp, flow.
- Automation recipe: count violated rules, not batches; enforce `Violation.margin > 0` for any failing budget (reject zero-margin failures at construction).
- Beats hand layout because: legality is never traded for score.
- Philis status: implemented with caveat — `dp/src/lib.rs:402`; `report.rs:17-29`; `margin 0 … reads as satisfied to Report::lex` (`report.rs:35-37`) remains.

### NOTES-36 Disjunctive spacing (DTI share/isolate) as branch moves
- Kind: algorithm
- Statement: Legal gap g ∈ [0, s_max] ∪ [d, ∞), d > s_max; flip branch and realize endpoint geometry in one trial; MIP form g ≥ dz, g ≤ s_max + (g_max − s_max)z with g_max ≥ d; limit total left+right stretch.
- Source: topic-search-algorithms.txt L64–75; topological.txt L109–113 (Strasser §3.3.2, PDF 135–137; O12).
- Philis stage: annotator, dp.
- Automation recipe: as implemented; add shared-stretch constraint for middle devices.
- Beats hand layout because: exact disjunctive legality handled automatically.
- Philis status: implemented — `kernel/analog/src/placement/dti.rs:18-40`; emitted `emit.rs:180-195`; branch flip 2.5 % of moves (`dp/src/lib.rs:304`). Stretch-sum limit not seen.

### NOTES-37 Annealing discipline: T0 calibration, frozen energy, seeds
- Kind: heuristic
- Statement: T0 = −mean(ΔE⁺)/ln p0; energy definition frozen within an epoch; archive best; compare several seeds under equal budgets; asymmetric repair proposals void textbook guarantees.
- Source: topic-search-algorithms.txt L42–62 (Lampaert Eqs. 4.37–4.38, PDF 117–118; O07).
- Philis stage: dp, flow.
- Automation recipe: log acceptance-rate curve; report distribution over `starts`.
- Beats hand layout because: reproducible, seed-controlled exploration.
- Philis status: implemented (partial) — `t0 = 0.02·mean|ΔPEX|` and Lampaert reference `dp/src/lib.rs:276-297`; prices frozen per anneal (`requirements.rs` contract); multistarts in config.

### NOTES-38 Symmetric-feasible topological seeds (SP / B* islands)
- Kind: algorithm
- Statement: Sequence pair (α, β): pos_α(x) < pos_α(y) ⇔ pos_β(sym(y)) < pos_β(sym(x)) is sufficient (not necessary) for symmetric feasibility; ASF-B* islands pack representatives; record the restriction.
- Source: topic-placement-representations.txt L29–65; topological.txt L61–91 (Balasa Eq. 1.1, PDF 49; Lin & Chang PDF 79–85; O10, O11).
- Philis stage: gp.
- Automation recipe: optional initializer producing coordinates for symmetric blocks; compare against current GP on routed outcomes at equal wall time.
- Beats hand layout because: many legal symmetric floorplans generated per second.
- Philis status: missing.

### NOTES-39 Constraint-graph / LP compaction with exact symmetry
- Kind: algorithm
- Statement: After topology selection, solve difference constraints xⱼ − xᵢ ≥ dᵢⱼ plus 4-variable symmetry equalities (x_a + x_b = 2x_s) and centroid equalities by LP; use doubled coordinates for half-grid axes; area is bilinear (enumerate aspect bounds).
- Source: topic-placement-representations.txt L67–71; topic-search-algorithms.txt L77–85; topological.txt L103–107, L139–145 (O14).
- Philis stage: dp (legalize).
- Automation recipe: replace sequential `project_hard` with a small LP per symmetric group; verify exact equality after grid snapping.
- Beats hand layout because: exact equalities with minimal area.
- Philis status: missing — projection-based repair `dp/src/lib.rs:371`.

### NOTES-40 Contextual variant frontier (don't prune on w/h alone)
- Kind: heuristic
- Statement: Keep a bounded Pareto set of variants over footprint, sensitive-pin access side, internal parasitics, environment; example: 72 µm² cell → 95 fF route → 54.9 MHz fails; 84 µm² cell → 40 fF → 88.4 MHz passes 70 MHz.
- Source: topic-layout-sizing.txt L35–46; topological.txt L115–119 (O13).
- Philis stage: cells, gp.
- Automation recipe: tag variants with pin-side classes; forbid dominance pruning across classes.
- Beats hand layout because: the parent-level trade is evaluated, not guessed.
- Philis status: partial — `VariantSpace` and escalation (`lib.rs:386-390`); pruning criteria not class-aware (not verified).

### NOTES-41 Route-aware placement feedback
- Kind: algorithm
- Statement: Reserve power/return/shield/pin-escape resources early; feed global-route congestion and electrical failure locations back to placement; retain multiple placements through routing.
- Source: topic-floorplanning.txt L45–53; roadmap.txt L65–67; lampaert.txt L97–101 (historical 10–15 % area penalty attributed to separated P&R, PDF 116, 172).
- Philis stage: gp, dp, gr.
- Automation recipe: export per-gcell overflow from `gr` after each epoch; add a congestion density term to `gp::place`; enlarge halos around overflowing macros.
- Beats hand layout because: placement and routing co-optimized every epoch.
- Philis status: partial — ring halos `lib.rs:1195-1199`; negotiation history only for dr (`lib.rs:342`, `:566`); no congestion input to `gp::place` (`lib.rs:575`).

### NOTES-42 Die/pad-limited floorplan awareness
- Kind: heuristic
- Statement: Die side = max(pad-ring requirement, core + boundary); a smaller core on a pad-limited die saves nothing (1,400 µm vs 1,100 µm example) — spend freed area on matching, taps, shields.
- Source: topic-floorplanning.txt L32–38.
- Philis stage: flow.
- Automation recipe: when a pad/package interface is given, compute the limiting term and switch the area objective off if pad-limited.
- Beats hand layout because: avoids optimizing a quantity that does not matter.
- Philis status: missing (block-level flow; low priority).

### NOTES-43 Differential route RC matching
- Kind: check
- Statement: Compare per-layer lengths/widths, via counts, terminal R and relevant C entries; tolerances from offset/CMRR sensitivity: |R_pos − R_neg| ≤ ΔR_max, |C_pos,k − C_neg,k| ≤ ΔC_max,k.
- Source: topic-parasitics.txt L89–95; topic-signal-routing.txt L35–48; MAT-11.
- Philis stage: dr, verify.
- Automation recipe: add coupling term to `Differential` (sum coupling from each aggressor to both sides); derive tolerance from `CommonNode`-style η·σ budget.
- Beats hand layout because: matched RC including neighbors, every route.
- Philis status: implemented without coupling — `differential.rs:10-27`.

### NOTES-44 Aggregate crosstalk as a transfer budget
- Kind: formula
- Statement: Fast-step charge sharing ΔV_v ≈ Cc/(Cv + Cc)·ΔV_a (1 fF/19 fF, 1 V → 50 mV); sum all aggressors with phase knowledge; include cross-layer overlap/fringe; victim role sets the budget.
- Source: topic-signal-routing.txt L50–58 (D05); NET-03.
- Philis stage: dr, verify.
- Automation recipe: extend `CouplingBudget` to adjacent-layer overlap C from the stack; budget in mV from victim total C and aggressor swing (Clock class).
- Beats hand layout because: every aggressor–victim pair on every layer is quantified.
- Philis status: partial — same-layer only (`coupling.rs:61-63`), per-layer ε·t (`coupling.rs:14-28`).

### NOTES-45 Shields as routed conductors with a quiet reference
- Kind: rule
- Statement: Shield = routed part of a named quiet reference; continuity, tie impedance and added victim load are part of the budget; floating or noisy shields get no credit.
- Source: topic-signal-routing.txt L60–73; substrate.txt L117–121 (P12, P13).
- Philis stage: dr, annotator.
- Automation recipe: as implemented; add top/bottom plate shields and tie-resistance check via `Stack::terminal_resistance_ohm`.
- Beats hand layout because: shield effectiveness measured, not assumed.
- Philis status: partial — `kernel/analog/src/routing/shield.rs:8-26` (one-sided, same-layer; tie impedance not measured).

### NOTES-46 Common-node, star and Kelvin topology
- Kind: rule
- Statement: Shared source/ground nets of matched devices: |R_a − R_b| ≤ η·σ_rand/I (10 Ω at 1 mS ≈ 1 % mirror error); Kelvin sense joins at the device terminal, not upstream of the force-path resistance.
- Source: topic-power-distribution.txt L48–67; topic-floorplanning.txt L57; H20.
- Philis stage: annotator, dr.
- Automation recipe: CommonNode done; add star seeding in dr at the members' centroid; add a Kelvin net attribute that forbids sense joins except on the target terminal polygon.
- Beats hand layout because: ΔR measured on the routed tree for every shared node.
- Philis status: partial — `kernel/analog/src/routing/common_node.rs:10-30`; Kelvin missing (no match in code).

### NOTES-47 Performance-driven discrete wire sizing
- Kind: algorithm
- Statement: τ(w) = (R_d + a/w)(C_L + bw) has an interior optimum w* = √(aC_L/(R_d b)); in circuits, width helps low-Z source paths and hurts high-Z poles (Table 7: UGF +13 %, gain −2 dB). Search legal discrete widths/parallel tracks on sensitivity-selected nets; EM minima are hard.
- Source: topic-wire-sizing.txt L13–74; todaes.txt L21–57 (Q20, Q24, Q25, D06).
- Philis stage: dr, flow.
- Automation recipe: rank nets by |∂f/∂R|·R vs |∂f/∂C|·C from `perf::sensitivities` (add R perturbation); try {1×, 2×, 3×} on top-k nets; accept by perf tier.
- Beats hand layout because: selective widening by measured sensitivity.
- Philis status: partial — need-based widening (`dr/src/lib.rs:650-669`); no performance-driven search.

### NOTES-48 Branch-current EM with waveform classes, via crowding, temperature
- Kind: formula
- Statement: Tree edge current = signed cut sum (incl. shunt loads in transients); avg/RMS/peak each sized (10 mA @10 %: 1/3.16/10 mA); via cuts n = max ceil(I_metric/I_cap,metric) × crowding factor (3 → 4 at 1.5); j_allow(T)/j_allow(T_ref) = exp[(Ea/nk)(1/T − 1/T_ref)]; Blech |jL| over a physical diffusion domain only.
- Source: topic-electromigration.txt L38–91; electromigration.txt L59–97 (Lienig Eqs. 3.1–3.8, 4.1; P18–P23).
- Philis stage: dr, flow, deck.
- Automation recipe: add RMS/peak limits to the deck EM record; carry per-terminal waveform classes from transient testbench; characterize a crowding factor per via-array template; use per-segment temperature when available.
- Beats hand layout because: every segment and cut checked against the right metric.
- Philis status: partial (DC) — `em.rs:11-96`; `dr/src/lib.rs:597-605`, `:650-669`, `:749`; `elaborate.rs:242-261`.

### NOTES-49 Terminal-resolved IR drop, forward + return
- Kind: check
- Statement: ΔV = ΣI_eR_e per branch, forward and return (32 + 7.25 = 39.25 mV example); allocate by path sensitivity (1 mV at gm 2 mS = 2 % current).
- Source: topic-power-distribution.txt L25–54; PWR-02.
- Philis stage: dr, verify.
- Automation recipe: solve the routed resistor network per net with terminal currents (port graph exists); include return nets; budget per terminal from headroom (`OpPoint.headroom_mv`).
- Beats hand layout because: every terminal's drop computed.
- Philis status: partial — worst-path bound (`ir.rs:8-27`); headroom available (`oppoint.rs:18-21`).

### NOTES-50 Stage-aware antenna check and repair
- Kind: algorithm
- Statement: Per etch stage, connected components built from layers ≤ stage; ratio = exposed area (or perimeter) / gate area; repair by upper-layer jumper (changes earlier components) or qualified diode (leakage/C cost); top layer cannot be jumped.
- Source: topic-protection.txt L43–64 (Hastings §5.1.6, PDF 228–230; H16).
- Philis stage: annotator, dr, verify.
- Automation recipe: as implemented; move diode credit to the stage where the diode connects (ponytail note `stack.rs:287-288`).
- Beats hand layout because: every gate net checked per stage.
- Philis status: implemented — `extract.rs:33-49`; `stack.rs:284-317`; `lib.rs:632-635`; `pdk.rs:572-610`.

### NOTES-51 Voltage-aware spacing from joint scenarios
- Kind: rule
- Statement: Spacing between conductors uses the maximum allowed pairwise ΔV over steady state, startup and sequencing, not voltage to ground.
- Source: topic-manufacturability.txt L89–95; topic-floorplanning.txt L61 (Hastings PDF 820–821).
- Philis stage: annotator, dr, verify, deck.
- Automation recipe: per-net voltage ranges from supplies/op; deck voltage-dependent spacing table; dr spacing lookup by ΔV class.
- Beats hand layout because: all net pairs checked, including startup cases.
- Philis status: missing (no voltage-dependent spacing in `backend/verify/src`).

### NOTES-52 Legal via landing templates in pitch planning
- Kind: rule
- Statement: Pitch must accommodate via landing (80 + 2×30 = 140 nm landing vs 100/100 line/space) and grid parity; compute landing templates first, reserve enlarged footprints.
- Source: topic-manufacturability.txt L46–54; H21.
- Philis stage: gr, dr.
- Automation recipe: derive per-layer landing width from deck; make track pitch ≥ landing + space where vias are allowed.
- Beats hand layout because: avoids late jogs that break matching.
- Philis status: partial — via arrays with cut spacing and deck-derived grid (commits 45305b7, 3bd9f9c; `dr/src/lib.rs:130-139` width-dependent spacing).

### NOTES-53 Thermal: equal exposure, weighted temperatures, known power
- Kind: formula
- Statement: ΔT = P/(2πkr) (half-space, derived); rank by equal transfer from all heaters over all power modes, not distance; group temperature = Σsᵢ ΔTᵢ with circuit weights, not hottest member; distinguish verified zero power from missing power.
- Source: topic-thermal.txt L35–70, L111–117 (P24, P25; Hastings Eq. 8.22–8.23, PDF 386–388).
- Philis stage: gp/dp, flow.
- Automation recipe: per-unit temperatures from `rise_at_mc` at unit positions; weight by unit weight; power modes from multiple op scenarios; `power_known` flag.
- Beats hand layout because: isotherm placement computed for every candidate.
- Philis status: partial — `kernel/core/src/thermal.rs:9-95`; `placement/thermal.rs:7-54`.

### NOTES-54 Substrate isolation as a loaded transfer
- Kind: algorithm
- Statement: Model injection → propagation → reception with contact network, backside and package impedance; distance saturates in low-ρ bulk; guard ring with shared inductive return can worsen noise; differential error = (H1 − H2)·v_agg.
- Source: topic-substrate-isolation.txt L17–105; substrate.txt L49–149 (P08–P15).
- Philis stage: annotator, gp/dp, verify.
- Automation recipe: stage 1 — compact resistive contact-network model per substrate type from deck; score victim body-to-source transfer; stage 2 — dedicated ring return nets in dr with measured R/L.
- Beats hand layout because: transfer computed for every aggressor/victim pair.
- Philis status: missing — distance rule only (`isolation.rs:7-17`; `emit.rs:246-283`).

### NOTES-55 Density fill with sensitive keep-outs
- Kind: algorithm
- Statement: Fill to density floor within block windows; tie fill; keep out of matched devices and sensitive nets; recheck extraction after fill.
- Source: topic-manufacturability.txt L61–67; MFG-01.
- Philis stage: flow.
- Automation recipe: as implemented; add post-fill re-extraction (NOTES-07).
- Beats hand layout because: consistent fill policy with keep-outs.
- Philis status: implemented (no re-extract) — `frontend/library/src/fill.rs:1-30`; `lib.rs:403-420`.

### NOTES-56 Critical-area defect exposure (separate from parametric yield)
- Kind: metric
- Statement: Bridging critical area A(χ) = L·max(0, χ − s); λ = ∫A(χ)D(χ)dχ; P(no fault) = e^−λ; spend spare margin on spacing/redundant vias only after specs pass.
- Source: lampaert.txt L129–137 (Lampaert Eqs. 5.16–5.24, PDF 153–155; O09).
- Philis stage: dr, flow.
- Automation recipe: compute parallel-run critical area per net pair; as last lexicographic tier only with a declared defect density.
- Beats hand layout because: quantitative DFM improvement at zero spec cost.
- Philis status: missing.

### NOTES-57 Discriminating validation catalog
- Kind: check
- Statement: Tests that change exactly one mechanism: halo-only change (centroid invariant), polynomial field sweep, equal-area splits, S/D reversal at fixed centroid, equal-length unequal-RC routes, shield return, wire sizing, substrate return, thermal power removal (→ unknown), via array feed, antenna distinct G/D, DAC families, equal-C different-node, omitted recognizer, macro rotate/retarget.
- Source: topic-validation.txt L26–60; hastings.txt L339–365.
- Philis stage: benchmarks, all.
- Automation recipe: one unit test per row in the owning crate; keep the list as a regression checklist.
- Beats hand layout because: each claimed mechanism is proven on counterexamples.
- Philis status: partial — e.g. isotherm tests `kernel/core/src/thermal.rs:144-158` (pair on one isotherm ⇒ ΔT = 0) and `:161-170` (live field follows a move), antenna diode test `frontend/library/tests/antenna_diode.rs`, cap_array tests; no omitted-recognizer or equal-C tests found.

### NOTES-58 Joint yield, β-distance and zero-failure confidence
- Kind: metric
- Statement: Y = Pr[∀x_r: all specs pass]; β = (U − f₀)/√(gᵀCg); ball/box/half-space probabilities differ (98.889/99.461/99.865 % at β = 3, n = 2); max-min β ≠ max yield ((3,3) → 0.997302 vs (2.9,8) → 0.998134); zero failures in N ⇒ p ≤ 1 − 0.05^(1/N) ≈ 3/N.
- Source: topic-robust-design.txt L15–77; centering.txt L90–110 (Graeb §§4.8, 6.1–6.3; O17–O21, D09).
- Philis stage: flow.
- Automation recipe: optional Monte Carlo on finalists with the PDK mismatch models (decks already characterize with `mc`); report joint pass rate with a confidence bound.
- Beats hand layout because: yield is measured, not asserted.
- Philis status: missing (no MC yield in flow; mismatch MC exists only as deck characterization `benchmarks/characterize_mismatch.py`).

### NOTES-59 Specialty-device capability envelope
- Kind: rule
- Statement: RF inductors, power, HV, annular, JFET, NVM need qualified primitives with frequency/voltage/SOA domain, transform restrictions and exclusion regions (fill, routing); unsupported structures are declared capability gaps.
- Source: topic-rf.txt L13–22, L90–104; hastings.txt L247–265.
- Philis stage: cells, flow.
- Automation recipe: each generator declares `domain` and `exclusion` shapes; fill and routing honor exclusions; flow refuses to optimize outside the declared domain.
- Beats hand layout because: exclusions are never violated by late fill.
- Philis status: partial — `kernel/cells/src/inductor.rs` exists; fill has no inductor-specific exclusion (no match in `fill.rs`).

### NOTES-60 Source-provenance comments in code
- Kind: data-model
- Statement: Code comments cite theory records by kind (source/derived/gap/policy) with assumptions; a background citation does not certify a derived formula.
- Source: theory-provenance.txt L13–19, per-record templates.
- Philis stage: all.
- Automation recipe: keep citing Hastings/Lienig/Pelgrom as the code does (`em.rs:11-15`, `matching_pair.rs:17-24`), and add record IDs (e.g. `[P22; derived]`) so the provenance survives review.
- Beats hand layout because: rules are auditable.
- Philis status: partial — citations present throughout; record-kind labels absent.

---

## 4. Top-15 priorities for Philis

1. **Structured coverage certificate** (NOTES-03, NOTES-02): without it a clean report can hide unrecognized devices or skipped rules; smallest change with the largest trust gain.
2. **Make the post-layout performance tier the default in benchmarks, with scenarios** (NOTES-11, NOTES-09, NOTES-01): the machinery exists; the flow still ranks most runs by total C.
3. **Re-extract and re-simulate after fill; report final-geometry evidence** (NOTES-07, NOTES-55): current reported metrics are pre-fill.
4. **Stable constraint IDs and parent links** (NOTES-04, NOTES-05): prices bound by ordinal are fragile; derived limits cannot be invalidated.
5. **Second-order moment terms (x², xy, y²) in CentroidGroup and cap_array** (NOTES-20, NOTES-21): cheap, exact, and fixes the radial-only gap.
6. **Coupling in Differential and cross-layer CouplingBudget** (NOTES-43, NOTES-44): matched routes are currently blind to asymmetric aggressors.
7. **Bias-aware mismatch budgets from the operating point** (NOTES-18, NOTES-16): gm and Id are already available; converts area rules into offset.
8. **Route congestion feedback into placement** (NOTES-41): Lampaert attributes the historical gap to hand layout to separated P&R.
9. **Performance-driven discrete wire sizing on sensitive nets** (NOTES-47, NOTES-10): sensitivities exist; add R perturbation and a width search.
10. **EM RMS/peak classes, via crowding factor, local temperature** (NOTES-48): DC-only sizing under-sizes switching and reference nets.
11. **Weighted thermal group temperature and known-power flag** (NOTES-53, NOTES-02): hottest-member ΔT mis-ranks CC arrays.
12. **Substrate transfer model and ring return impedance** (NOTES-54): replace the 4×t_epi distance proxy the notes and code both qualify.
13. **Pre-search contradiction detection** (NOTES-06): turns overconstraint into an explanation instead of wasted epochs.
14. **DAC settling and split-DAC support** (NOTES-32, NOTES-31): completes the capacitor-array generator for converter use.
15. **Signed orientation and thermoelectric checks across cells** (NOTES-22, NOTES-28): cheap connectivity checks a human routinely misses.
