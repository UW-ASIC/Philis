# Resistor Cell Catalog — Complete Shape/Variant Space

**Framing:** `docs/frontend/GENERATOR_MODULARITY.md` §3 sketches the resistor axes; this
document is the exhaustive deep dive. Every axis below maps to a field in
`ResistorSpec { kind, n_segments }` (spec-enumeration architecture, GENERATOR_MODULARITY §5).

**Implemented today** (`frontend/cells/src/generators/resistor.rs`): straight poly bar with
licon/li head contacts and P/N pin tails, spec-driven fold (`ResistorSpec.n_segments`,
enumerated with **even counts enforced** for thermoelectric cancellation), factory returns
multiple shape variants (aspect-driven default + shallower alternative), `ResistorKind` as
a spec axis. `PatternType::Interdig` flag for multi-device moderate+ tiers (pattern flag
only — segments are not actually interleaved across devices). Everything else in this
catalog is unbuilt.

**Sources:** Hastings, *The Art of Analog Layout* 3rd ed. ch6 (`docs/books/AOAL/ch06-*.md`)
and ch8 (`ch08-matching-rules.md`); Lienig 6.6.1/6.6.5 via
`docs/books/PNR_ANALOG/01_S0_CELL_GENERATION.md` §7.2, §7.A.0, §7.A.3–7.A.7.

**Diagrams:** `docs/cells/resistor.html` figures (a)–(g), referenced per section below.
Simplified Excalidraw of views (b) and (c): `docs/cells/resistor.excalidraw`.

---

## §1 Construction kinds — material × geometry class

### 1.1 Materials

| Material | R_sh (Ω/□) | TCR (ppm/°C) | Key limits | Source |
|---|---|---|---|---|
| Poly, silicided (LSR) | ~5 | ~+3400 (CoSi₂) | Ti-silicide needs width ≥ 0.5 µm (C49→C54 phase) | Hastings 6.5.7, 6.3.1 |
| Poly, unsilicided gate | ~20 | >+1000 | silicide block mask; heads stay silicided | Hastings 6.5.7 |
| Poly, medium-sheet (MSR) | ~200 | ~0 ("0TC", ±500 ppm lot spread) | flat (untilted) poly deposition for analog fabs | Hastings 6.5.7 |
| Poly, high-sheet (HSR poly) | 500–1000 | ~−1500 | dehydrogenation drift (P-type); field plate >1 kΩ/□ | Hastings 6.5.7, 6.3.5 |
| Poly, very-high-sheet (VHSR) | ≥1000 | to −5000 | ±40% process variation; avoid for matching | Hastings 6.3.1, 6.5.7 |
| Diffused N+ (NSD) / P+ (PSD) | 30–100 (unsilicided) | ~+400 | shallow → low avalanche; junction parasitics; mainly ESD | Hastings 6.5.8 |
| Base diffusion (bipolar) | 100–250 | ~+1300 | body/tank bias required; NBL underneath always | Hastings 6.5.1 |
| Emitter diffusion | 2–10 | ~+400 | sub-Ω to ~200 Ω range; ballast/sense/crossunders | Hastings 6.5.2 |
| Well resistor (N-well) | 1–5 k | ~−6000 | field plate always; width ≥ 2–3× x_j; pinch plate → ~10 k | Hastings 6.5.9 |
| Pinch (base pinch / epi-FET) | 2–10 k / 5–50 k | ~+3500 | model as JFET; ±5–10% "matching"; noncritical only | Hastings 6.5.3, 6.5.5 |
| HSR (implanted high-sheet) | 1–10 k | ~−3000 | charge spreading; base heads; 20–30 V avalanche; hard to match even moderately | Hastings 6.5.4 |
| Thin-film nichrome / sichrome | low (ρ 0.1 / 1–20 µΩ·cm) | 50–100 / 0 to −150 | two-mask process cost; laser-trimmable to ±0.05% | Hastings 6.5.10, 6.6.2 |
| Metal | 0.03–0.05 | ~+3300 | 10 mΩ–5 Ω; current sense; Kelvin mandatory; TCR ≈ VPTAT TCR | Hastings 6.5.6 |

**Material ranking for matching** [Hastings ch8 Rule 1]:
**thin-film (nichrome > sichrome) > polysilicon > diffused/implanted.**
Thin-film: areal matching < 0.1%·µm, ~zero voltage modulation, near-zero piezoresistivity.
Poly: ~0.5%·µm, adequate for moderate matching. Diffused: sometimes better areal
coefficients but worse in every other respect (TCR, voltage modulation, piezoresistivity);
HSR struggles to reach even moderate. **Never mix materials in a matched group** — ≥5%
process mismatch, ≥1% over temperature [Rule 1].

### 1.2 Geometry class: lateral vs vertical

| Class | Current path | Examples | Notes |
|---|---|---|---|
| **Lateral** | in-plane along the film; R = R_sh·L/W ("count squares") | all of §1.1 as normally drawn | Hastings Eq. 6.3; everything in §2–§7 assumes lateral |
| **Vertical** | perpendicular to the wafer, through a body of thickness t; R = ρ·t/A | upright diffused plug (deep-N+/sinker body between top contact and NBL/buried layer), via/plug body used resistively, TFR vertical contact stack | resistance set by depth × area, not L/W; no fold/serpentine axis; matching by identical plug arrays only. See **HTML fig (g)** — isometric + cross-section. |

Vertical resistors trade the entire lateral shape space for a single (area, depth) knob;
they matter to the generator because their footprint is a compact square column rather
than a long bar — a fundamentally different `variant_dims` shape class.

Diagram: **fig (g)** (vertical, 3D isometric + side section); **fig (a)** (lateral bar).

---

## §2 Body construction axis

| Body style | Description | Tier legality | Source |
|---|---|---|---|
| Straight bar | single rectangle, contacts inside ends | any (matching from array context) | Hastings 6.2; **fig (a)**; `resistor.rs` today |
| Dogbone / dumbbell | body narrower than contact heads; enlarged ends | any; ΔR head correction < 0.3 □ (Table 6.3); head style irrelevant for matching if all segments identical | Hastings 6.2 |
| Serpentine (monolithic fold) | one meandering body, rectangular turns (+0.56 □/corner) or circular turns (+2.96 □/180°, high-voltage) | **minimal tier ONLY — cannot interdigitate** [Hastings ch8 Rule 16] | Hastings 6.2, 8.3.1; **fig (b)** |
| Arrayed unit segments | N identical rectangular bars joined in series by metal links | **required for moderate+** [Rule 16]; exceptional uses simple rectangles wide enough to need no dogbone | Hastings ch8 Rules 5/16; **fig (c)** |

**Ratio construction** [Rule 2, Rule 5; Lienig 6.6.1]: ratios are built from **identical
unit resistors (equal W AND L) in series — never by scaling L** of one body. Length
scaling fails because head resistance R_H does not scale:

```
R_total = R_sq·(l/w) + 2·R_H(w)
l_1 = r·l_2 + l_corr,   l_corr = 2(r−1)(R_H/R_sq)·w      [Lienig 6.6.1]
```

The l_corr correction uses *nominal* R_H, R_sq; process tolerance leaves residual error,
so the correction formula is a fallback, not the preferred path. Partial (non-unit)
segments require sensitivity analysis and, if used, must be inset equally at both ends
[Rule 8].

`resistor.rs` today: straight bar + monolithic serpentine only; no unit-segment arrays,
so the generator is capped at minimal tier for multi-segment bodies.

---

## §3 Segment / fold axis

| Sub-axis | Domain | Rule | Source |
|---|---|---|---|
| Segment count N | **EVEN required** for matched sets | Seebeck cancellation: half the segments carry current one way, half the other; contact-potential EMF α_S ≈ 50–500 µV/K, ΔT = 2 K → 0.1 mV → 0.4% mirror mismatch. Common-centroid alone does NOT cancel thermoelectrics (they arise per-segment). | [Hastings ch8 Rule 11, 8.2.7]; **fig (c)** arrows |
| Fold direction | left-first / right-first meander; heads oriented in **opposite directions** to cancel mask misalignment | serpentine contacts placed close together | Hastings 8.2.7 (Fig 8.23C); **fig (b)** |
| Terminal exit | both-terminals-same-side (even N) vs opposite ends (odd N) | same-side reduces thermoelectric EMF and simplifies routing | PNR_ANALOG §7.2; **fig (b)**, **fig (c)** |
| Inter-segment spacing | voltage-dependent poly spacing: max adjacent-segment voltage **V_seg = 2V/N** | TDDB avoidance; also cone-defect STI breakdown < 20 V (absent in LOCOS) | [Hastings 6.5.7; PNR_ANALOG §7.A.6] |
| Segment length floor | ≥ 10 µm per segment for <1% end-effect error | end effects Eq. 6.8; see also §8 tier floors | Hastings 6.2 |

`resistor.rs` today: `ResistorSpec.n_segments` enumerated by `feasible_segments()` with
**even-N enforced** when folding (thermoelectric cancellation). Factory returns the
aspect-driven default fold + a shallower alternative as distinct shape variants.

---

## §4 Orientations & transforms — exhaustive

### 4.1 The 8 dihedral transforms (D4) and matched-group legality

Applied to a lateral bar/array; see **fig (f)** for the full chart.

| # | Transform | Body axis after | Current axis flipped? | Legal within a matched group? |
|---|---|---|---|---|
| 1 | R0 (identity) | unchanged | no | Yes — reference |
| 2 | R90 | rotated 90° | — | **No** for mono-Si (piezoresistive π_L ≠ π_T; crystal axis changes). Poly/thin-film: geometrically legal, still discouraged (etch/anisotropy asymmetries); use only as the deliberate L-pair trick (§4.4) |
| 3 | R180 | unchanged | yes | Yes — preferred second element: cancels mask-misalignment bias and, when segments are series-connected, provides Rule-11 current reversal |
| 4 | R270 | rotated 90° | yes | Same as R90 |
| 5 | MX (mirror about x) | unchanged | yes (vertical bar) | Yes for symmetric bodies; asymmetric heads/taps must remain superimposable — non-self-aligned features can break equivalence |
| 6 | MY (mirror about y) | unchanged | yes (horizontal bar) | Yes, same caveat; `resistor.rs` uses MY for alternate serpentine segments today |
| 7 | MX·R90 (mirror about y=x) | rotated 90° | — | Same as R90 |
| 8 | MY·R90 (mirror about y=−x) | rotated 90° | — | Same as R90 |

Summary: **{R0, R180, MX, MY} legal; {R90, R270, diagonal mirrors} illegal for matched
mono-Si groups** and reserved for the perpendicular-pair trick otherwise. All devices of
one matched group must share orientation [Hastings ch8 Rule 6] — even minimal tier.

### 4.2 H vs V body orientation

`ResistorKind::{Horizontal, Vertical}` in `resistor.rs` is exactly this axis (both are
lateral; "Vertical" here = bar drawn vertically, not §1.2's vertical geometry class). It
is currently a factory input; GENERATOR_MODULARITY §3 flags it as an enumerated axis of
one factory. Whole-group H vs V is free for poly (isotropic) and constrained for mono-Si
by §4.3; mixing H and V *within* a group violates Rule 6.

### 4.3 Piezoresistivity by material [Hastings 8.2.8]

π coefficients, 10⁻¹¹ Pa⁻¹, lightly doped Si, 25 °C:

| Material / orientation | π_L | π_T | Layout implication |
|---|---|---|---|
| P-type mono, ⟨110⟩ on (100) | 71.8 | — | worst common case |
| P-type mono, ⟨100⟩ on (100) (45°) | 6.6 | — | minimum, but 45° layout **impractical** — use poly instead |
| N-type mono, ⟨110⟩ on (100) | −31.2 | — | **minimum for N-type = normal H/V layout** |
| N-type mono, ⟨100⟩ on (100) | −102.2 | — | avoid 45° for N-type |
| Any orientation, (111) wafer | 71.8 (P) / −31.6 (N) | — | orientation-free on (111) |
| Polysilicon (any) | 24 | 9.5 | **orientation-independent** (random grains) |
| Thin-film nichrome | ~0 | ~0 | orders of magnitude below silicon |

### 4.4 L-shaped perpendicular common-centroid pair

Because ΔR/R = π_L·σ_L + π_T·σ_T + π_S·τ_S, a resistor with **equal horizontal and
vertical lengths** sees (π_L+π_T)/2 on both stress components — first-order stress
cancellation. Practical form: **two identical common-centroid arrays oriented
perpendicular to each other**, each device taking half its segments from each array
[Hastings 8.2.8]. See **fig (d)**. This is the sanctioned use of R90 within a matched
context: whole sub-arrays rotate together, and every matched device spans both
orientations equally.

---

## §5 Patterns (multi-device arrangement)

| Pattern | Sequence | Tier | Notes | Diagram |
|---|---|---|---|---|
| Single | A…A | any (lone device) | `resistor.rs` today | fig (a) |
| Interdigitated | D A B A B A B D | moderate | 1-D gradient partial cancellation; segments (not whole devices) alternate | **fig (c)** |
| Common-centroid array | D A B B A D, (ABBA)ⁿ, 2-D banks | moderate/exceptional | obey symmetry, coincidence, dispersion, compactness [Rule 8, Table 8.4]; exceptional maximizes dispersion; long arrays → multiple banks each CC | fig (c) variant |
| L-shaped perpendicular CC pair | two perpendicular CC arrays, each device split half/half | exceptional (stress-critical) | §4.4 | **fig (d)** |

Placement context (feeds cell placement, not cell shape): minimal ≤ few hundred µm apart;
moderate adjacent/interdigitated; exceptional always common-centroid [Rule 7], near die
center on axes of symmetry [Rules 12–14], ≥ 1 µm/mW from power devices [Rule 13].

**Dummies** [Hastings ch8 Rule 9]:

| Tier | Diffused/implanted | Deposited (poly/thin-film) |
|---|---|---|
| Minimal | not required | **1 minimum-width dummy each end** |
| Moderate | full-width dummy each end | full-width dummy each end |
| Exceptional | full-width dummies each end | **multiple dummies spanning ≥ 10 µm each end** |

All segment spacings (dummy or active) identical; segment ends extend past active region
3× min width (moderate) / 5× (exceptional) [Rule 9]. Dummy terminals tied to a circuit
node (not floating) — same convention as capacitor Rule 7. Dummies are hatched in figs
(b), (c), (d).

---

## §6 Contact / head variants

| Variant | Description | Tier | Source |
|---|---|---|---|
| Single contact | one cut per head; current crowds at inner edge; ΔR ≈ ⅔[1 + ln(W/W_c) − W_c/W] squares | narrow bodies; fine when ≥10 □ long | Hastings 6.2 Eq. 6.8; **fig (a)** |
| Contact row | contacts spanning body width (W_c ≈ W_d) | wide bodies; minimizes lateral crowding; oversized/multiple contacts if segment < ~10 □ (contact-R variability) | Hastings 6.2, 6.3.4 |
| Dogbone head | enlarged head around contact; minimize head overlap W_o, match W_c to W_d | ΔR < 0.3 □; matching-neutral if all segments identical | Hastings 6.2 |
| Low-sheet head (HSR/high-sheet poly) | silicided or base-diffused head; R_head ≈ 0.7·R_sh,head·d/(W_head+ΔW) | drives the never-L-scale rule (§2) | Hastings 6.5.4 |
| **Kelvin 4-terminal** | separate **force** (full current, wide metal) and **sense** (µA, thin) leads per head | **exceptional** [Rule 23]; mandatory for metal sense resistors | Hastings 6.5.6; PNR_ANALOG §7.A.4; **fig (e)** |

Kelvin lead rules: single-level metal — sense taps on the resistor *side*, force lead
runs **≥ 2W straight past the star point before any bend**; double-level metal — sense on
upper metal tapping the head *center* (less sensitive to nonuniform current flow). Metal
and via resistance must scale proportionally to the target ratio across a matched group
[Rule 23].

`resistor.rs` today: one licon + li tail per head, single P/N pins — the "single contact"
row only.

---

## §7 Field plates & shielding

| Option | When | Source |
|---|---|---|
| None | R_sh ≤ ~200 Ω/□ poly, low-Z materials (conductivity modulation negligible) | Hastings ch8 Rule 15 |
| Grounded plate (well/tank potential) | any diffused/implanted resistor above 50% thick-field threshold; **all moderate-matched diffused with R_sh ≥ 1 kΩ/□**; all exceptional diffused; consider for higher-sheet poly | [Hastings ch8 Rules 3a/15/20]; unpinched N-well: always [6.5.9] |
| **Split plates tracking local potential** | **exceptional**; HSR matching < 0.1%; gap at resistor midpoint → equal-and-opposite fields cancel dielectric absorption (soakage) while still blocking charge spreading; needed when shield sheet > ~1 kΩ/□ or array voltage exceeds a few volts | [Hastings ch8 Rule 3a, 8.2.9] |
| Faraday shield vs overlying leads | no unconnected leads over moderate+ matched bodies [Rule 21]; if unavoidable, metal shield on reference node, overhang ≥ 5 µm; exceptional poly: met2 plate, no met1 over active bodies | Hastings 8.2.9, Fig 8.30 |

**Conductivity modulation** — the driver: an overlying lead's field
accumulates/depletes the resistor surface; met1 over HSR ≈ 0.1%/V, ±5% swings observed
on 3 kΩ/□ HSR; body/tank voltage modulates diffused bodies the same way (~1000 ppm/V for
200 Ω/□ P-type). High-sheet resistors are the vulnerable class; identical body/tank bias
on all matched segments is a prerequisite [Rule 3a; Hastings 6.3.3].

---

## §8 Size floors, power, and process shadows (tier constraints)

| Constraint | Minimal | Moderate | Exceptional | Source |
|---|---|---|---|---|
| Width ≥ (× min linewidth) | 150% | 200% | 400% | [Hastings ch8 Rule 4] |
| Segment length ≥ (× design-rule min) | 3× | 5× | 10× | [Rule 10] |
| Poly total length | — | — | ≥ 1000× grain diameter (≥ 50–100 µm), averages grain boundaries; also linearity needs L ≥ 1000× grain (safe >100 µm, manageable >30 µm) | [Rule 10; Hastings 6.3.3] |
| Random-mismatch budget (Pelgrom) | ≤ 75% of total | ≤ 50% | ≤ 25%; poly ±0.01% pair → ~100,000 µm²; trimming nearly always required | [Rule 3] |
| Array power | — | — | **≤ 1 mW total** (self-heating gradients are bias-dependent, β_V = α·R_s·t_ox/(R·W²·k_ox)) | [Rule 22; PNR_ANALOG §7.A.7] |
| Absolute width floors | poly ≥ 0.3 µm (bamboo effect), Ti-silicided ≥ 0.5 µm (C49/C54), diffused ≥ 2×x_j (dilution) | same | same | Hastings 6.3.1 |
| Moat/field-oxide clearance | — | ≥ 10 µm from moat edges; same field oxide under all segments | same | [Rule 17] |
| Gate-doping-block / other diffusion clearance | — | ≥ 150% of min spacing | same | [Rules 19/24]; poly-2 cap shadow ≥ poly-2 width [Rule 25] |
| **NBL shadow** (BiCMOS/LOCOS only) | verify no matched segment inside shadow: boundary = epi_thickness × pattern-shift-factor (50–150% on (111)) + alignment tolerance; overlap NBL ≥ 150% max epi thickness all sides if shift direction unknown; STI immune | same | same | [Rule 18; PNR_ANALOG §7.A.0] |

---

## §9 Summary enumeration table

Axes of a future `ResistorSpec` (GENERATOR_MODULARITY §5). M = affects matching grade,
G = geometry/parasitics only.

| Axis | Domain | Matching impact | In `resistor.rs` today? |
|---|---|---|---|
| Material | silicided/unsilicided/MSR/HSR/VHSR poly; NSD/PSD; base; emitter; N-well; pinch; HSR; nichrome/sichrome; metal | M — ranking thin-film > poly > diffused [Rule 1]; same material across group mandatory | No — implicit from netlist model (`rpo_01v8`), single layer stack |
| Geometry class | lateral / vertical (plug) | G — different bbox class; vertical has no fold axes | No — lateral only |
| Body construction | bar / dogbone / serpentine (minimal only) / arrayed unit segments (moderate+) | M — Rule 16 | Partial — bar + serpentine; no dogbone, no unit-segment arrays |
| Ratio construction | identical units in series (never L-scale; l_corr fallback) | M — Lienig 6.6.1 | No — W, L taken verbatim per device |
| Segment count N | even, ≥2 for matched (thermoelectric halves) | M — Rule 11 | Partial — N from aspect heuristic, parity not enforced |
| Fold direction / terminal exit | meander handedness; heads opposed; same-side terminals | M (thermoelectric/misalignment) + G (pin side) | Partial — fixed alternate-MY, exit follows parity |
| Inter-segment spacing | ≥ voltage-dependent rule, V_seg = 2V/N | G (reliability) | No — fixed `res_seg_gap` |
| Orientation (whole group) | H / V bar; {R0,R180,MX,MY} legal transforms; no 90° mix in group; material-dependent piezo table | M — Rules 6, 8.2.8 | Partial — `kind` is a factory input, not an enumerated/validated axis |
| L-shaped perpendicular pair | off / on (two perpendicular CC arrays) | M — piezo cancellation | No |
| Pattern | single / interdig ABAB / CC 1-D / CC banks / L-pair | M — Rules 7/8 | Partial — Interdig flag set, devices actually laid side-by-side |
| Dummies | tier table: 1 min-width / full-width / multiple ≥ 10 µm; equal spacing; tied-off | M — Rule 9 | No |
| Contact/head | single / row / dogbone / low-sheet head / Kelvin 4T (exceptional) | M (Rule 23) + G (pins) | No — single contact + tail only |
| Field plate | none / grounded / split-tracking (exceptional) | M — Rules 3a/20/21 | No |
| Width/length floors | 150/200/400% linewidth; 3/5/10× length; poly ≥1000 grains; Pelgrom area | M — Rules 3/4/10 | No — no tier validation |
| Power budget | ≤ 1 mW exceptional | M — Rule 22 | No — needs AnalogParams (operating point) |
| NBL shadow check | BiCMOS/LOCOS shadow exclusion / NBL overlap | M — Rule 18 | No — N/A on sky130 (STI) |
| Trim/tweak provisions | sliding contact/head, trombone, metal options, fuse/laser networks | G (area, pads) — enables ±0.1%/±0.05% post-fab | No |

---

## Diagram index (`resistor.html`)

| Fig | View | Catalog sections |
|---|---|---|
| (a) | straight bar, head contacts, P/N tails (matches `resistor.rs`) | §1.2, §2, §6 |
| (b) | serpentine 4-segment fold, dummies both ends | §2, §3, §5 |
| (c) | interdigitated A B A B A B unit-segment pair, met1 series links, current arrows (half up / half down) | §2, §3, §5 |
| (d) | L-shaped perpendicular CC pair | §4.4, §5 |
| (e) | Kelvin 4-terminal unit, wide force + thin sense | §6 |
| (f) | bar under all 8 dihedral transforms + legality | §4.1 |
| (g) | 3D isometric vertical resistor + side cross-section | §1.2 |
