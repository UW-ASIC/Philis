# BJT — Complete Shape/Variant Catalog

**Scope:** every axis of geometric variation a BJT cell can take in S0 cell generation.
**Sources:** Hastings AOAL 3ed ch9 (§9.1–9.3), ch10 (§10.2–10.3);
`docs/books/PNR_ANALOG/01_S0_CELL_GENERATION.md` §3.A.4, §4.A.2, §6.A.0/6.A.00, §9.A.5;
current code `frontend/cells/src/generators/bjt.rs`.
**Diagrams:** [bjt.html](bjt.html) views (a)–(f); simplified Excalidraw copies of (a),(c) in `bjt.excalidraw`.
**Framing:** this is the BJT instantiation of the axis-enumeration architecture in
`docs/frontend/GENERATOR_MODULARITY.md` §4–5 — one drawing routine, per-axis domain
functions, Pareto set to the placer.

---

## §1 Construction kinds

The first axis is not a parameter but the *device construction* itself — which process
layers form E/B/C. Diagram: [bjt.html view (a)](bjt.html#view-a) (vertical NPN),
[view (b)](bjt.html#view-b) (lateral PNP), [view (f)](bjt.html#view-f) (3D isometric of
the vertical NPN — vertical devices demand a 3D mental model, plan view alone is misleading).

| Kind | E / B / C layers | Current direction | Availability | Key layout obligations | Source |
|---|---|---|---|---|---|
| **Vertical NPN** (standard bipolar / BiCMOS CDI) | N+ emitter diff / P-base diff / N-epi tank + **NBL** + **deep-N+ sinker** | **Vertical** E→B→drift→NBL, laterally through NBL, up the sinker — see view (f) | Standard bipolar; BiCMOS (adds NBL to CMOS) | Fill tank with NBL; sinker elongated across tank; base overlaps emitter all sides; CEB vs CBE contact orders | Hastings ch9 §9.2.1, ch9 §9.3.3 |
| **Vertical PNP (CMOS parasitic / substrate)** | PMoat (or base-diff) emitter / N-well (N-epi) base / **P-substrate collector** | **Vertical**, into substrate — collector is not isolated | Any N-well CMOS; standard bipolar "substrate PNP" | Annular NMoat base ring; silicide-block emitter (short-emitter effect); I_C ≤ 1 mA/device, ≤ 10 mA total (substrate debias); substrate contacts adjacent | Hastings ch9 §9.2.2, §9.3.1 |
| **Lateral PNP** | Base-diff plug emitter / N-epi tank base / **annular** base-diff collector | **Lateral**, radial E→C through neutral base at surface | Standard bipolar; BiCMOS (needs NBL for η_C); retrograde-well CMOS (poly-ring self-aligned) | **Field plate over base REQUIRED**, tied to emitter, overlapping collector 2–3 µm [PNR_ANALOG 6.A.00; Hastings ch9 §9.2.3]; NBL under emitter to inner collector edge min; minimum-size emitter | Hastings ch9 §9.2.3 |
| **Split-collector lateral PNP** | As lateral PNP, collector ring split into 2/4/n identical arcs | Lateral, current ∝ subtended emitter periphery | Same as lateral PNP | Segments identical & symmetric about emitter (±2%); no emitter degeneration possible (shared emitter) | Hastings ch9 §9.2.3, ch10 §10.2.2 |
| **Verti-lat PNP** | PMoat plug / shallow N-well / isolated P-epi tank (or partial base-diff ring) | Mixed vertical + lateral | Shallow twin-well BiCMOS; standard bipolar "tombstone" | PBL blanket implant for V_CE 10–20 V; field plate exposed tank | Hastings ch9 §9.2.2, §9.3.4 |
| **Shallow-well NPN** | NMoat plug / shallow P-well / deep N-well | Vertical (lateral variant if retrograde P-well) | Triple/quad-well CMOS | Channel stop PMoat ring + field plate both required; R_C ~ kΩ (no NBL/sinker) | Hastings ch9 §9.3.2 |
| **Poly-emitter NPN (fast/SiGe HBT)** | As-doped poly emitter / implanted (SiGe) base / SIC + NBL collector | Vertical | Fast BiCMOS | Keep reverse V_EB ≤ 1–2 V (avalanche β degradation); two-stage ESD | Hastings ch9 §9.3.5 |

**Vertical-current devices (vertical NPN, vertical/substrate PNP) require the 3D view** —
the collector terminal at the surface is connected to the active collector *underneath*
the emitter via NBL + sinker; plan-view area does not correlate with current capability
the way it does for lateral/MOS devices. View (f) is normative.

---

## §2 Emitter geometry axis

Diagrams: view (a) square emitter; view (b) circular emitter.

| Sub-axis | Domain | Rule | Source |
|---|---|---|---|
| Emitter shape (vertical) | **square / circular / octagonal** | Compact shapes maximize area-to-periphery → higher β, better random matching. Circular drawn as polygon with side count divisible by 4 (32 or 64) | Hastings ch10 §10.3.1 Rule 3; PNR_ANALOG 3.A.4 |
| Emitter shape (lateral PNP) | circular preferred; square allowed with collector-corner fillets | Circular gives uniform base width; square has wider effective base at diagonals; radial symmetry enables arbitrary split-collectors | Hastings ch9 §9.2.3 |
| Unit emitter size (vertical) | **2×–10× minimum width; 8–16 µm typical for moderate matching** | Larger emitters cut K_A/√A_E random mismatch but raise nonlinear-gradient exposure — use *arrays of moderate units*, never one huge emitter | Hastings ch10 §10.2.1, Rule 2; PNR_ANALOG 3.A.4 |
| Unit emitter size (lateral PNP) | **ALWAYS minimum size** — hard rule | Larger emitters degrade β (graded-well drift field pulls carriers down, away from collector). Scale by **arraying** min emitters (hexagonal packing for large counts), **never elongate** | Hastings ch10 §10.3.2 Rule 2; ch9 §9.3.4; PNR_ANALOG 3.A.4 |
| Single large vs arrayed units | arrayed strongly preferred for matched devices | Peripheral mismatch term scales with areal term only when all devices are unit-emitter multiples; identical unit geometry is Rule 1 for both vertical and lateral | Hastings ch10 §10.3.1 Rule 1 |
| Multi-emitter stripes (power / current) | narrow stripes ≤ ~25 µm wide, base contacts both sides (double-base) | Avoids emitter crowding/debiasing (18 mV doubles local current); R_B ≈ ¼ of single-base. This is what `bjt.rs` `n_stripes` approximates today | Hastings ch9 §9.2.1 |
| Emitter contact shape | **match contact geometry to emitter geometry** (circular↔circular, square↔square); contact as large as possible inside emitter | Minimizes R_E; consider reduced contact in thin-emitter BiCMOS | Hastings ch10 §10.3.1 Rules 3, 14 |
| Scaling metric | vertical: emitter **area**; lateral: emitter **periphery** (M = (P_E/P_U)·(P_C/P_CA)) | | Hastings ch9 §9.2.3 |

---

## §3 Formation / array axis

Diagrams: [view (c)](bjt.html#view-c) 3×3 eight-around-one; [view (d)](bjt.html#view-d)
ratioed quad 4:1:1:4.

| Sub-axis | Domain | Rule | Source |
|---|---|---|---|
| Unit-emitter count per device | integer N; ratios by unit count only, **never by geometry scaling** | Different sizes/shapes match very poorly | Hastings ch10 §10.3.1 Rule 1 |
| Ratioed pair 4:1 | linear **2 : 1 : 2** (1× centered, 4× split around it) | Common-centroid, cancels linear gradients | Hastings ch10 fig 10.24A; PNR_ANALOG 4.A.2 |
| Ratioed pair 8:1 | **3×3 eight-around-one** (1× center, 8 units around) — view (c) | Spans ⅓ of a linear array → quadratic gradient residue cut ~10× | Hastings ch10 fig 10.24B; PNR_ANALOG 4.A.2 |
| Ratioed quad 4:1:1:4 | 2× (4×-unit) + 2× (1×) transistors, 2D CC — view (d) | ΔV_BE = V_T·ln(16) = **72 mV**; cancels linear thermal *and* stress gradients | Hastings ch10 Eq 10.14; PNR_ANALOG 4.A.2 |
| Optimal ratio window | **6:1–16:1** sweet spot; 4:1 / 6:1 / 8:1 popular (8:1 most) | VPTAT grows ln(N), span grows √N, nonlinear residue ~N → diminishing returns beyond 16:1 | Hastings ch10 §10.2.3; PNR_ANALOG 3.A.4, 4.A.2 |
| Diff pair | cross-coupled quad (2D CC); **base contacts** must also be common-centroid (thermoelectric potentials); collector contacts exempt | | Hastings ch10 fig 10.23 |
| CBE ring order | **C outside, B ring, E inside** preferred over CEB — view (a) | TC of V_BE > TC of base-contact potential → pack emitters closest for thermal matching. (CEB has marginally lower R_C — non-matched use only) | Hastings ch10 §10.2.3; ch9 §9.2.1; PNR_ANALOG 3.A.4 |
| Shared tank vs separate tank | vertical NPN: may share collector tank (base controls conduction); merged base possible with ≥ 2× base-junction-depth neutral spacing, **unconnected-emitter spacing rules even for connected emitters**. Lateral PNP: separate tank per device for matched (cross-injection, saturation) | | Hastings ch10 §10.2.3, §10.2.6 |
| Deep-N+ plug in shared tank | **mandatory minimum plug per NPN in a shared tank** — keeps R_tank < 100 Ω; without it R_tank > 6 kΩ → 600 mV debias at 150 °C → latchup | Insert even if PDK rules don't require it | PNR_ANALOG 6.A.0; Hastings ch14.1 |
| Array aspect | tight 2D clusters beat long rows; ≤ 3:1 (voltage), ~1:1 (exceptional) | | Hastings ch10 Rule 5; PNR_ANALOG 4.A.1 |
| Emitter degeneration hook | not a shape, but cell must expose Kelvin-able emitters when spec requests degeneration (50 mV→3×, 100 mV→6× matching); N/A for split-collector | | Hastings ch10 §10.2.2 |

---

## §4 Orientations & transforms

Diagram: [view (e)](bjt.html#view-e) — 8 dihedral transforms of an asymmetric
stripe-emitter NPN, plus orientation-immunity of the circular lateral PNP.

| Sub-axis | Domain | Rule | Source |
|---|---|---|---|
| Dihedral transforms | R0 / R90 / R180 / R270 / MX / MY / MX90 / MY90 | All 8 are *DRC-legal* for BJTs (no directional pocket implants as in MOS). For **matched groups**: devices must be superimposable by translation; a square/circular concentric cell is invariant under all 8; a stripe-emitter or single-side-base cell is only invariant under the subgroup fixing its axis | Hastings ch10, ch13 (superimposability); PNR_ANALOG 9.A.3 |
| Piezojunction — vertical NPN (100) Si | π_T = **−43.4 × 10⁻¹² Pa⁻¹** | Most stress-sensitive BJT; ~0.9 mV ΔV_BE per 100 MPa | Hastings ch10 §10.2.4; PNR_ANALOG 9.A.5 |
| Piezojunction — vertical PNP (100) Si | π_T = **−13.3 × 10⁻¹² Pa⁻¹** — **~3× less stress-sensitive** | **Prefer vertical PNP for bandgap references** on (100) Si — smaller package shift | Hastings ch10 §10.3.1 Rule 18; PNR_ANALOG 9.A.5 |
| Piezojunction — lateral PNP (100) Si | π_R = **+11.6 × 10⁻¹² Pa⁻¹**, small | ~0.3 mV per 100 MPa | Hastings ch10 §10.2.4; PNR_ANALOG 9.A.5 |
| Circular / annular emitters | **radially symmetric stress response → orientation-immune** | Both stress (Eq 10.16 uses σ_x+σ_y, rotation-invariant) and placement transforms are don't-cares — a real degree of freedom for the placer | PNR_ANALOG 9.A.5; Hastings ch10 §10.2.4 |
| Emitter degeneracy of shape | square emitter: 8-fold symmetric (all transforms equivalent); stripe emitter: 2 distinct orientations (H/V) × contact-side choice — transform changes bbox and pin sides | geometry consequence, see view (e) | — |
| Die-level placement (metadata, not shape) | matched arrays near die center, on die axes of symmetry, ≥ 200 µm from edges, never corners; ≥ 500 µm (minimal) / 100–200 µm (moderate) from power devices | Cell exports a placement-constraint record; enforcement is S2's job | Hastings ch10 §10.3.1 Rules 7–9 |

---

## §5 Dummies & guard structures

| Sub-axis | Domain | Rule | Source |
|---|---|---|---|
| Dummy emitters/units | 0 / 1 ring of dummy unit emitters at array edges (moderate+ matched arrays) | Edge units of an array see different etch/outdiffusion environments; identical-unit rule extends to identical *surroundings*. Shown hatched in views (c),(d) | Hastings ch10 Rule 1 (identical geometry incl. environment); analogous to ch13 MOS dummies |
| Guard rings | none / HCGR (PMoat collecting) / HBGR (deep-N+ + NBL blocking) / combined | HCGR ≈ unity efficiency, width ≥ N-well depth; HBGR > 95–98%, must fully encircle, NBL extends to outside edge of deep-N+; **HCGR+HBGR > 99%**. Full treatment out of scope here → guard-ring doc (see PNR_ANALOG 6.A.1–6.A.3, Hastings ch14). Cell axis = which ring(s) the BJT cell embeds (inflates bbox → placer-visible) | PNR_ANALOG 6.A.2; Hastings ch14 |
| Substrate PNP guard | substrate-contact ring sized to collector current | ≥ 1 mA needs extra contacts around device | Hastings ch9 §9.2.2 |
| NBL shadow / pattern shift | NBL oversize ≥ **150% of epi thickness** on all sides (unknown shift direction); or orient array's symmetry axis parallel to shift so shadow lands in collector/base contact, not emitter. Lateral PNP: shadow must not enter the exposed neutral base | Shadow intersecting one emitter of a ratioed pair → systematic area mismatch (1% area → 0.5% ratio shift, ~0.13 mV). STI processes: no shadow | Hastings ch10 §10.2.5, §10.3.1 Rule 10, §10.3.2 Rule 10 |

---

## §6 Contact variants

| Sub-axis | Domain | Rule | Source |
|---|---|---|---|
| Emitter contact | single centered / **array filling emitter** / washed (self-aligned) | As large as possible in emitter to cut R_E; match shape to emitter (§2); CMOS substrate PNP: *fewest* contacts + silicide block (short-emitter effect) | Hastings ch9 §9.2.1, §9.3.1; ch10 Rule 14 |
| Base contact | single-side stripe / full-width stripe / **double-side** (R_B ≈ ¼) / **ring** (annular, lateral & CMOS PNP) | Elongating to full base width cuts R_B free of area; matched arrays: base contacts must form their own common-centroid pattern | Hastings ch9 §9.2.1; ch10 §10.2.3 |
| Collector contact | **deep-N+ sinker to NBL** (R_C ~200 Ω; sinker elongated full tank width) / surface-only tank contact (R_C ~5 kΩ — low-current only) / sinker **ring** (extended-base NPN) | Sinker omission is a legitimate area/R_C trade at I_C < 1 mA; mandatory plug in shared tanks (§3) | Hastings ch9 §9.2.1, §9.3.3; PNR_ANALOG 6.A.0 |
| Kelvin base | second, force/sense-split base contact pair | For precision V_BE sensing (bandgap cores) — removes R_B·I_B error from the sensed junction voltage; analogous to resistor Kelvin heads | Hastings ch6 (Kelvin contacts), applied per ch10 precision context |
| Stretched variants (SLM legacy) | stretched-collector / stretched-base / tunnel-through-base | Lead routing through the device; DLM makes these obsolete — enumerate only for single-level-metal PDKs | Hastings ch9 §9.2.1 |

---

## §7 Summary enumeration table

Axis order = suggested implementation order. "In `bjt.rs` today" reflects
`frontend/cells/src/generators/bjt.rs` as of this writing: one hard-coded concentric
rectangle construction (collector rect, poly over everything as "base", emitter stripes
with current-crowding split, nwell for NPN), `PatternType::Single`, no arrays, no
orientation handling, no dummies, no guard/NBL/sinker layers, no field plate.

| # | Axis | Domain | Matching impact | In `bjt.rs` today? |
|---|---|---|---|---|
| 1 | Construction kind (§1) | vertical NPN / substrate-vertical PNP / lateral PNP / split-collector / verti-lat / shallow-well NPN / poly-emitter | Determines π coefficients, β, matching ceiling | ✗ — single generic construction; NPN/PNP only flips nwell presence |
| 2 | Emitter shape (§2) | square / circular(32-64 gon) / octagonal | area/periphery ratio → K_A term | ✗ — rectangles only |
| 3 | Unit emitter size (§2) | vertical 2–10× min (8–16 µm); lateral = min, always | random mismatch vs gradient exposure | ~ — `bjt_min_emitter_side` clamp + stripe split; no unit-emitter concept |
| 4 | Single vs arrayed / stripes (§2) | 1 large / N units / N stripes double-base | Rule-1 identical units | ~ — stripes for current crowding only |
| 5 | Unit count & ratio pattern (§3) | 2:1:2, 3×3 8-around-1, 4:1:1:4 quad, CC quad diff pair | THE precision axis (bandgap ΔV_BE) | ✗ — `PatternType::Single`, devices in a row |
| 6 | CBE/CEB ring order (§3) | CBE (matched) / CEB (R_C) | thermal matching | ✗ — fixed concentric order, no contact rings |
| 7 | Tank sharing + deep-N+ plug (§3) | shared / separate; plug per NPN mandatory | latchup, cross-injection | ✗ |
| 8 | Orientation / dihedral transform (§4) | 8 transforms; circular = immune | piezojunction, superimposability | ✗ |
| 9 | NPN↔vertical-PNP choice for bandgap (§4) | π_T −43.4 vs −13.3 ×10⁻¹² Pa⁻¹ | 3× package-shift reduction | ✗ |
| 10 | Dummy units (§5) | 0 / edge ring | edge-environment equalization | ✗ |
| 11 | Guard rings (§5) | none / HCGR / HBGR / both | substrate injection, latchup | ✗ |
| 12 | NBL oversize vs shadow (§5) | ≥150% epi thickness / axis-aligned | systematic area mismatch | ✗ — no NBL layer at all |
| 13 | Field plate, lateral PNP (§1/§6) | REQUIRED, emitter-tied, +2–3 µm over collector | β stability (hard functional rule) | ✗ |
| 14 | Contact variants (§6) | E array; B stripe/double/ring; C sinker/surface/ring; Kelvin base | R_E/R_B/R_C, base-contact CC | ~ — one contact per terminal; poly-as-base is a placeholder, not a real base diffusion |

**Biggest gaps, in impact order:** (5) ratioed arrays, (1) real construction kinds with
NBL/sinker/base-diff layers, (13) lateral-PNP field plate (functional correctness, not
just matching), (7) deep-N+ plug, (2)+(3) unit-emitter discipline.
