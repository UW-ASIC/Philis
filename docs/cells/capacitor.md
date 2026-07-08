# Capacitor Cell — Complete Shape/Variant Catalog

**Device:** CAPACITOR. **Sources:** Hastings AOAL ch7 (`docs/books/AOAL/ch07-capacitance.md`),
ch8 §8.3.2 (13 capacitor rules, `docs/books/AOAL/ch08-matching-rules.md`),
`docs/books/PNR_ANALOG/01_S0_CELL_GENERATION.md` §7.1, §7.A.1–7.A.2,
framing in `docs/frontend/GENERATOR_MODULARITY.md` §4.
**Diagrams:** `capacitor.html` views (a)–(f); simplified Excalidraw of (b)+(c) in `capacitor.excalidraw`.

---

## §1 Construction kinds, ranked by matching potential

Ranking per Hastings ch8 §8.3.2 and PNR_ANALOG §7.A.2 (best → worst):

| Rank | Kind | Stack direction | Structure | Matching notes | Diagram |
|---|---|---|---|---|---|
| 1 | Poly-metal, silicided lower electrode + thick LPCVD oxide | **VERTICAL** (plate over plate) | silicided poly bottom, metal/TiN top | Metallic electrodes eliminate poly depletion; thick homogeneous LPCVD oxide → low random mismatch, low dielectric relaxation [Hastings ch8 Rule 11] | (e) analogous stack |
| 2 | MIM (metal–insulator–metal) | **VERTICAL** (capm plate over met_n) | met_n bottom plate, thin dielectric, capm plate, via up to met_n+1 strap | Metallic electrodes cannot deplete → negligible V/T modulation ("gold standard") [Hastings ch7]; needs densification workaround (TiN barrier / silicided poly / ARC dielectric) | (a) top view, (e) exploded iso |
| 3 | MOM / finger (lateral flux) | **LATERAL** (interdigitated fingers), optionally stacked on multiple metals with vertical flux too | comb A/B fingers per layer; horizontal-bar, vertical-bar (via-connected pillar checkerboard), woven, fractal variants [Hastings ch7] | No extra masks; competitive when metal pitch < ILO thickness; matching set by litho of finger edges | (b), (d) |
| 4 | Poly-poly | **VERTICAL** (poly2 plate over poly1) | poly1 bottom, ONO/nitride/oxide dielectric, poly2 top | Upper-electrode poly depletion limits accuracy (~±5 % V-modulation); ONO adds soakage + asymmetric breakdown [Hastings ch7] | (e) small iso |
| 5 | MOSCAP (accumulation / inversion) | VERTICAL (gate over channel) | gate oxide cap; 4 types (N/PMOS × acc/inv) | Must bias ≥0.5–1 V overdrive beyond V_FB/V_th to sit deep in acc/inv; poly depletion + junction parasitics + leakage [Hastings ch7] | — |
| 6 | Junction | — (depletion region) | reverse-biased PN junction, plate or comb layout | **Unsuitable for matching**: ±20–50 % process variation, severe voltage/temperature depletion modulation [Hastings ch7/ch8] | — |

Also available but niche (not ranked for matching): stack/ILO caps (~0.05 fF/µm², high V rating), trench caps (100–500 nF/mm², special process) [Hastings ch7].

## §2 Unit geometry axis

| Sub-axis | Domain | Rule / source |
|---|---|---|
| Shape | **square** (exceptional tier — always), **rectangle ≤3:1** (moderate), complex shapes forbidden (OPC corner non-repeatability) | Hastings ch8 Rule 2 |
| Unit side | optimal **25–100 µm/side** — balances peripheral effects (small units worse) vs. gradient sensitivity (large units worse) | Hastings ch8 Rule 3; PNR_ANALOG §7.A.1 |
| Area (Pelgrom) | poly-poly A_C ≈ 0.5 %·µm: **±0.1 % → ~250 µm²; ±0.01 % → ~25 000 µm²** per matched pair; large caps subdivided into cross-coupled units | Hastings ch8 Rule 3 |
| Corner treatment | ch08 as excerpted covers corners via Rule 2 (avoid complex shapes; OPC corner effects not repeatable). Chamfered/rounded unit corners reduce peripheral field concentration and etch-corner variation — same periphery-minimization rationale; trench caps explicitly use corner rounding to reduce field intensification [Hastings ch7]. Treat as an optional per-PDK sub-axis: none / 45° chamfer / rounded | Hastings ch8 Rule 2, ch7 (trench corners) |
| Periphery/area matching for non-unit fraction | non-integer remainder cap sized so its **area-to-periphery ratio equals the unit's** | Hastings ch8 Rule 1 |

See diagram (a) — single MIM unit top view (plate inset, via array, strap).

## §3 Array formation

| Sub-axis | Domain | Rule / source |
|---|---|---|
| rows×cols | choose to **minimize aspect ratio** while fitting all units + perimeter dummies; compact row-column pattern with equal row/col spacing (e.g. 4×8 for 32 units) | Hastings ch8 Rule 4; PNR_ANALOG §7.1 step 2 |
| Ratio decomposition | each ratio → integer count of identical unit caps in **parallel** (1:2:4:8 → 1,2,4,8 units); **avoid series** (top/bottom-plate parasitic asymmetry — if forced, antiparallel) | Hastings ch8 Rule 1 |
| Non-integer ratios | one fractional cap with matched area/periphery ratio; **hazards:** fringing/width-bias no longer cancels exactly, partial units break dispersion, requires sensitivity analysis (analog of resistor partial segments) | Hastings ch8 Rules 1–2 |
| Unit assignment | **outward spiral** from center (minimizes centroid error, small arrays) vs. **block-chessboard** (partition into interleaved blocks, better for large arrays); both are common-centroid / cross-coupled assignments that cancel the dominant mismatch source — **dielectric thickness gradients** | PNR_ANALOG §7.1 step 3; Hastings ch8 Rule 9 |
| Cross-coupling floor | even two equal caps should be split into 2 sections each and cross-coupled | Hastings ch8 Rule 9 |
| LSB centroid hazard | a 1-unit cap cannot self-centroid; pair it point-symmetrically with a spare/dummy unit | corollary of Rule 9, shown in diagram (c) |

See diagram (c) — 4-bit 1:2:4:8 common-centroid array with perimeter dummy ring and owner labels.

## §4 Orientations & transforms

| Sub-axis | Domain | Notes |
|---|---|---|
| Unit-cell dihedral transforms | all 8 (R0/R90/R180/R270 × mirror) | Unit caps are usually **square and nominally symmetric → transforms mostly free**; piezocapacitance is orders of magnitude below piezoresistance so stress orientation is a non-issue at unit level [Hastings ch8 §8.2.8] |
| Plate/terminal asymmetry | mirrors/rotations that swap the via-strap or pin corner | Electrodes are **never interchangeable** (bottom-plate parasitic ≫ top) [Hastings ch7]; a transform that moves the top-plate strap changes lead capacitance → must be equalized (Rule 10) or transform rejected |
| Routing asymmetry | per-unit strap exit side | mirrored units need mirrored routing or lead-capacitance jogs; see §6 |
| MOM finger direction | H fingers vs. V fingers (R90 family) | behaves like resistor orientation: litho/etch bias differs per axis, so **all matched MOM units must share finger direction** (same-orientation rule, analog of resistor Rule 6) [Hastings ch8] |
| Poly-poly V-modulation trick | two equal sections in **opposite orientations** cross-coupled | cancels first-order poly-depletion voltage coefficient [Hastings ch7] |

See diagram (d) — MOM unit under the 8 dihedral transforms with legality notes.

## §5 Dummies & shielding

| Sub-axis | Domain | Rule / source |
|---|---|---|
| Perimeter dummies | ≥1 full ring on **all four sides**; moderate w/ shield: width ≥3× min; exceptional: **exact copies of the unit cap**, matched spacing | Hastings ch8 Rule 7; PNR_ANALOG §7.1 step 4 |
| Dummy connection | **both electrodes tied to a circuit node** (bottom plates to ground per PNR_ANALOG §7.1); never floating | Hastings ch8 Rule 7 |
| Electrostatic shield | plate above (and optionally below) array; contains fringing, lets leads cross, blocks external coupling; mandatory for moderate+ tiers | Hastings ch8 Rule 8 |
| Shield overhang | **≥50 µm without dummies; ≥5 µm with dummies**; digital signals never cross the shield | Hastings ch8 Rule 8; PNR_ANALOG §7.A.1 |
| Substrate isolation | **well / NBL under the array** (bottom-side shield), lower plate to low-impedance node | Hastings ch8 Rule 6 |
| Field-oxide uniformity | ≥10 µm from moat edges; no lower-level routing beneath (or a solid plate beneath) | Hastings ch8 Rule 5 |

See diagram (f) — cross-section with top shield, well/NBL bottom shield, overhang dimensions.

## §6 Plate assignment & routing variants

| Sub-axis | Domain | Rule / source |
|---|---|---|
| Bottom-plate node | bottom (high-parasitic) plate → **low-impedance node** (supply/gnd/driver output) | Hastings ch7 (MOS parasitics), ch8 Rule 6 |
| Top-plate treatment | sensitive node; **shield with ground metal above/below the route** | PNR_ANALOG §7.1 step 5 |
| Lead-capacitance matching | equalize interconnect parasitics with **jogs or dead-end branches**; lead-to-adjacent-metal spacing ≥2–3× ILO thickness; shield the interconnect network; verify with extraction | Hastings ch8 Rule 10 |
| Bus topology | top plate star vs. tree; bottom plate shared bus; via count vs. dispersion co-optimized | PNR_ANALOG §7.1 step 5 |
| Dielectric choice | **thick + homogeneous**: grown/LPCVD oxide > TEOS > ONO (relaxation, soakage >10 MHz, asymmetric breakdown from poly asperities) | Hastings ch8 Rule 11, ch7 |
| Series avoidance | no series unit chains; antiparallel if polarity reversal needed (offset depletion dips by 2V_th for PMOS-inv pairs) | Hastings ch8 Rule 1, ch7 |

## §7 Summary enumeration table

| Axis | Domain | Matching impact | Implemented in `capacitor.rs` today? |
|---|---|---|---|
| Construction kind | poly-metal-silicided / MIM / MOM-lateral / poly-poly / MOSCAP / (junction=reject) | Sets accuracy ceiling (§1 ranking) | Partial — `CapacitorKind` enum has 3 variants (VerticalAcrossLayers, HorizontalAcrossLayers, VerticalInOneLayer) as factory *input*, not enumerated axis; drawing is a generic 2-plate proxy |
| Unit shape | square / rect ≤3:1 / (corner chamfer option) | High (peripheral effects) — Rule 2 | No — plate w/l taken verbatim from netlist |
| Unit size | 25–100 µm side; Pelgrom area floor per tier | High (random mismatch) — Rule 3 | No — no area/tier sizing |
| Ratio decomposition | integer unit counts; fractional edge cap (A/P matched) | High — Rule 1 | Partial — `nf` used as unit count, but taken verbatim; no ratio solver, no fractional units |
| Array rows×cols | minimize aspect; equal spacings | Medium — Rule 4 | No — single row only |
| Assignment pattern | outward spiral / block-chessboard (common-centroid) | High (gradient cancellation) — Rule 9 | No — `PatternType::Cc1d` flag set for moderate+ multi-device, but geometry is a linear strip; no 2-D CC placement |
| Unit transforms | 8 dihedral; constrained by strap/finger asymmetry | Low (square units) / Medium (MOM finger dir, strap side) | No |
| Dummy ring | 0 / 1 / multiple rings; exact-copy units; both electrodes tied | High — Rule 7 | No dummies at all |
| Shielding | none / top plate / top+bottom (well/NBL); overhang ≥50 µm or ≥5 µm | High (moderate+ mandatory) — Rules 6/8 | No |
| Plate assignment | bottom→low-Z; top shielded | Medium (parasitics, noise) — Rule 6 | No — TOP/BOT pins emitted, no semantics |
| Lead-cap matching | jogs / dead-ends / shielded bus | Medium-high for DACs — Rule 10 | No — pins on first/last unit only, asymmetric |
| Dielectric | LPCVD oxide / nitride / TEOS / ONO | Medium (relaxation, hysteresis) — Rule 11 | No — PDK proxy layers only |
| Placement env. | die-center preference, ≥10 µm from moat, away from power | Low-medium (caps stress-tolerant) — Rules 5/12/13 | No (placer concern; needs metadata) |

**Gap summary:** today's generator emits one linear strip of nf plate pairs per device with 2 pins — effectively only the "unit count" axis exists, and even that is netlist-verbatim. Every matching-bearing axis (unit sizing, 2-D CC assignment, dummies, shield, lead balancing) is unimplemented, matching the pattern described in `GENERATOR_MODULARITY.md` §1.

## Diagram index (`capacitor.html`)

| View | Content | Backs section |
|---|---|---|
| (a) | Single MIM unit, top view: met_n bottom plate, inset capm plate, via array, top strap | §1, §2 |
| (b) | MOM interdigitated finger cap, top view: A/B combs | §1 (lateral flux) |
| (c) | 4-bit 1:2:4:8 CC array + dummy ring, owner-labeled chessboard assignment | §3, §5 |
| (d) | MOM unit under 8 dihedral transforms + legality notes | §4 |
| (e) | **3-D exploded isometrics**: MIM stack (met_n / dielectric / capm / via / met_n+1) and poly-poly stack | §1 (vertical kinds) |
| (f) | Shield cross-section: top shield + well/NBL below, overhang annotations | §5 |
