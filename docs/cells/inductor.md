# Inductor — complete shape/variant catalog

**Sources:** Hastings, *The Art of Analog Layout* 3rd ed. §7.2 (`docs/books/AOAL/ch07-inductance.md`,
cited below as [H7.2] with rule numbers from its "Practical Takeaways" list);
current implementation `frontend/cells/src/generators/inductor.rs`;
framing per `docs/frontend/GENERATOR_MODULARITY.md` (spec-enumeration → prune → generate → Pareto).
`docs/books/PNR_ANALOG/00_ANALOG_PRINCIPLES.md` contains **no inductor material** (grep for
inductor/spiral/inductance returns nothing) — it contributes nothing to this catalog.

Diagrams: `docs/cells/inductor.html` views (a)–(f), referenced per section.
Simplified editable version of view (a): `docs/cells/inductor.excalidraw`.

---

## §1 Construction kinds

Diagrams: (a) planar spiral, (c) symmetric, (d) PGS, (f) 3D stacked.

| Kind | Description | Status per [H7.2] |
|---|---|---|
| Planar spiral, single winding layer | Spiral body on top metal, jumper/underpass on the metal below to reach the innermost turn [H7.2 "Planar Spiral Inductors", Rule 2]. Metal layers strapped with vias count as **one** winding layer [Rule 7]. | The default construction; the only one the generator should need for most PDKs. |
| Stacked / multi-layer spiral (vertical structure — see 3D view (f)) | Series spirals on two metal layers joined by a via stack. Proximity-effect eddy losses between layers and interwinding capacitance "drastically lower the SRF"; practical designs limited to **1, at most 2 layers** [Rule 7, "Proximity Effects"]. Distinct from *strapping* (parallel layers = 1 winding, lowers DCR [Rule 3]). | Covered qualitatively; enumerate as `layers ∈ {1, 2}` with a strong prune penalty. |
| Symmetric / center-tapped spiral | Jumpers inserted into the spiral make both terminals electrically equivalent; benefits differential circuits directly and single-ended circuits via lower internal loss [H7.2 "Symmetric Inductors", Fig 7.23]. Center tap available at the symmetry point. | Covered; view (c). |
| Solenoid | N² inductance around a core; "does not lend itself to integration" [H7.2 "Solenoid Architecture"]. | Covered as *why not*; **not an on-chip generator target**. |
| Bond-wire inductor | ~5.6 nH per typical 1 mm bondwire [H7.2 "Bondwire Inductance"]. Package-level, not a layout cell. | Covered; out of scope for cell generation (no geometry on-die beyond pads). |
| MEMS (cavity-etched) | Substrate etched from beneath the spiral to kill eddy losses; "adds cost, requires specialized equipment" [H7.2 "Substrate Eddy Losses"]. | Mentioned only as a mitigation option; enumerate only if PDK exposes the etch layer. |

**Substrate shielding sub-axis** (orthogonal to kind, applies to planar/stacked):

| Shield | Description | Source |
|---|---|---|
| None | Q limited to ~5–10 on moderate-resistivity epi [H7.2 §7.2.2 "The Substrate Problem"]. | [H7.2] |
| Patterned ground shield (PGS) | Shield between spiral and silicon, **slotted radially**, slots perpendicular to spiral current; strip widths must satisfy the f_crit equation (Eq. 7.38). **Silicided poly** gives the best low-resistance / low-capacitance combination [H7.2 Fig 7.25]. View (d). | [H7.2 "Patterned Ground Shield"] |
| Deep-nwell isolation | **Not covered by sources** (ch07-inductance does not discuss DNW under inductors). Listed as a candidate axis only; needs PDK/EM validation before enumeration. | — |
| Elevation (thick metal / above overcoat / polyimide) | Dedicated ~3 µm thick RF metal, ideally above passivation; polyimide for further elevation [H7.2 "Elevating the Inductor", "Thick Metal"]. | [H7.2] |

## §2 Shape axis

Diagrams: (a) square, (b) octagonal, (c) symmetric.

| Shape | Trade-off [H7.2 "Planar Spiral Inductors"] |
|---|---|
| Circular | Lowest series R for given L, "hard to digitize" |
| Octagonal | Good compromise performance vs. digitization |
| Square | Easiest to implement, highest series R |
| Hexagonal | **Not covered by sources** (ch07 lists only the three above); include in the enum only if a PDK/EM model demands it |

Continuous/discrete parameters, with the empirical model
`L = K1·µ0·N²·d_avg / (1 + K2·ρ)`, `d_avg=(d_out+d_in)/2`, `ρ=(d_out−d_in)/(d_out+d_in)`,
`d_in = d_out − 2Np`, K1/K2 = 2.34/2.75 (square), 2.25/3.55 (octagonal) [H7.2 "Empirical Inductance Formula"]:

| Parameter | Domain / constraint | Source |
|---|---|---|
| Turns `N` | ≥1; L ∝ N² (diminished for planar — inner turns smaller, incomplete flux coupling) | [H7.2] |
| Outer diameter `d_out` | PDK min → area budget; 300 µm/10-turn example ≈ 18.6 nH ≈ practical ceiling without magnetic core | [H7.2] |
| Trace width `W` | ~10 µm optimal near 1 GHz; narrower → DCR, wider → skin/proximity loss [Rule 5] | [H7.2] |
| Turn spacing `S` | **Narrowest legal** — tighter coupling, higher L and Q; interwinding C inconsequential in single-layer spirals [Rule 6] | [H7.2] |
| Hollow ratio (`d_in`) | `d_in ≥ 5·W`, and ≥ `d_out/3` for larger inductors — center turns suffer severe eddy loss/current crowding [Rule 8] | [H7.2] |
| Fill factor `ρ` | Derived from the above; appears directly in the L formula denominator | [H7.2] |

## §3 Terminal / underpass variants

Diagrams: (a) underpass + P1/P2 ports, (c) center tap, (d)/(a) keep-out outline.

| Axis | Domain | Impact | Source |
|---|---|---|---|
| Underpass metal layer | Any metal below the spiral body, **except metal-1** (too close to substrate) [Rules 2, 3]; bondwire jumper is the alternative | Parasitic C, SRF | [H7.2] |
| Feed style | Single-ended (P outer / N inner via underpass) vs. differential (symmetric structure, both ports on outside) | Circuit topology match | [H7.2 "Symmetric Inductors"] |
| Center tap | None / tapped at symmetry point of a symmetric spiral | Required for differential VCOs etc. (structure per Fig 7.23) | [H7.2] |
| Lead routing | Short, direct, highest metal [Rule 11] | Lead parasitics | [H7.2] |
| Guard ring | Substrate contacts / guard rings around the inductor to mitigate debiasing/latchup on high-ρ substrates [Rule 1] | Latchup, bbox | [H7.2] |
| **Keep-out zone** | Radius ≥ **half the completed inductor width** for unconnected metal, dummy fill, poly, and *junctions*; nothing in the spiral center; no routes through the structure [Rules 4, 9, 10] | **Largest placer impact of any axis**: the effective cell bbox is spiral + keep-out annulus; must be in `CellRecord.variant_dims` | [H7.2] |

## §4 Orientations & transforms

Diagram: (e) chirality chart.

A spiral is **chiral**. Under the 8 dihedral transforms (R0/R90/R180/R270/MX/MY/MXR90/MYR90):

| Transform class | Winding direction (chirality) | Consequence |
|---|---|---|
| Rotations R0/R90/R180/R270 | **Preserved** (CW stays CW) | Free for the placer, subject to port access |
| Mirrors MX/MY/MXR90/MYR90 | **Flipped** (CW ↔ CCW) | Reverses the sign of mutual coupling M to any neighboring coil — matters for transformers/coupled pairs (§5); a mirrored instance is *not* electrically interchangeable in a coupled context |

Chirality/mutual-coupling sign is standard magnetics reasoning (dot convention); ch07-inductance
does not spell out transform rules — the placer-facing rule is derived here, not sourced.

- **45° routing legality:** octagonal spirals require non-Manhattan (45°) polygons; enumerate
  octagonal only when the PDK/deck permits 45° geometry (cf. the analogous 45°-PMOS gating in
  `GENERATOR_MODULARITY.md` §2.3). Square spirals are the Manhattan fallback.
- Isolated (uncoupled) inductors: all 8 transforms yield identical L/Q; expose all 8 to the placer.

## §5 Coupling structures

| Structure | Notes | Source |
|---|---|---|
| Integrated transformer / balun | Magnetically coupled spirals for isolation crossing or impedance transformation; "suffer even more from parasitic loss" — limited use as power combiners and baluns in RF ICs | [H7.2 "Integrated Transformers"] |
| Stacked coupled pair | Two-layer stacked structure (view (f)) reused as a 1:1 transformer; same proximity/SRF penalties as §1 stacked | [H7.2 Rule 7] |
| Interleaved planar coupled pair | **Not covered by sources** (ch07 does not describe interleaved-winding transformers) | — |
| Mutual-orientation rules | Sign of M set by relative chirality (§4): identical transforms → coupling adds; one mirrored → sign flips. Must be recorded per instance for transformer netlist polarity (dot convention). | derived, §4 |
| Neighbor spacing | Inductor-to-anything clearance ≥ half inductor width [Rule 4]; inductor-to-inductor coupling beyond that is an EM-simulation question — quantitative spacing-vs-M **not covered by sources** | [H7.2] |

## §6 Q-optimization knobs as enumeration axes

Each knob is an independent axis for the `InductorSpec` enumerator (per the
GENERATOR_MODULARITY pipeline: enumerate → analytic prune with the L formula, R_DC = ρℓ/A,
R_wind(f) = R_DC·(1 + f²/f_crit²), Q = 2πfL/Rs [H7.2] → generate survivors → Pareto to placer).

| Knob | Domain | Mechanism | Source |
|---|---|---|---|
| Trace width | ~10 µm optimum @1 GHz; sweep around it | DCR vs. skin/proximity [Rule 5] | [H7.2] |
| Width tapering (wider outer, narrower inner turns) | **Not covered by sources** — ch07 addresses uniform width only; do not invent taper profiles | — |
| Layer stacking (parallel, via-strapped) | top 2–3 layers in a 4–5 metal process; never metal-1 | Cuts DCR ~÷(layers); counts as one winding [Rule 3, Rule 7] | [H7.2] |
| Dedicated thick metal | if PDK has ~3 µm RF metal / above-overcoat metal | Best DCR + max substrate distance [Rule 2, "Thick Metal"] | [H7.2] |
| PGS | none / slotted silicided-poly / slotted metal; strip width from f_crit (Eq. 7.38) | Cuts substrate loss path [Fig 7.25] | [H7.2] |
| Hollow center | d_in ≥ max(5W, d_out/3) | Removes worst eddy/crowding turns [Rule 8] | [H7.2] |
| Eddy keep-out | ≥ half inductor width; dummy-fill exclusion; no junctions/plates | Eliminates neighboring eddy loss [Rules 4, 9, 10] | [H7.2] |
| Turn spacing | minimum legal | Coupling ↑, L ↑, Q ↑ [Rule 6] | [H7.2] |
| Elevation | standard stack / thick metal / above overcoat / polyimide | Distance from substrate ↓C, ↓eddy | [H7.2 "Elevating"] |

## §7 Summary enumeration table

| Axis | Domain | Performance impact | Implemented in `inductor.rs` today? |
|---|---|---|---|
| Construction kind | planar / stacked(≤2) / symmetric / (solenoid, bondwire, MEMS: non-targets) | L, Q, SRF, differential use | Planar only (rectangular-ring approximation; doc comment says octagonal but geometry is rectangles) |
| Shape | square / octagonal / circular (hex: not covered by sources) | series R for given L; 45° legality | **No** — concentric rectangles on `met1`; single hard-coded shape |
| Turns N | ≥1 | L ∝ N² (diminished) | Yes — `nf` (rings not translated to be concentric: each ring drawn from origin, acknowledged ponytail in code) |
| d_out | PDK min → budget | L, area | Yes — `l`, clamped to `ind_min_diameter` |
| Trace width W | ~10 µm optimum @1 GHz | DCR vs. AC loss | Yes — `w`, clamped to `ind_min_trace` |
| Spacing S | min legal | L, Q | Hard-coded `S = W` (violates Rule 6 "narrowest possible") |
| Hollow ratio d_in | ≥ max(5W, d_out/3) | eddy loss at center | **No** — loop only stops when ring ≤ 2W; no Rule-8 floor |
| Winding layer / stacking | top metal, strap 2–3, never M1 | DCR, C, SRF | **No** — fixed `met1` proxy (lowest metal — opposite of Rule 2/3) |
| Underpass layer | any lower metal except M1 | SRF, C | Partial — fixed `li`, drawn as center stub, not a routed underpass to the inner turn |
| Feed / center tap | single-ended / differential / center-tapped | topology match | Single-ended only (P/N pins) |
| Shield | none / PGS (slotted poly or metal) / elevation; DNW not covered by sources | Q (substrate loss) | **No** |
| Guard ring | none / substrate-contact ring | latchup on high-ρ substrate | **No** |
| Keep-out radius | ≥ half inductor width | placer bbox (dominant) | **No** — bbox is bare spiral; no exclusion halo published |
| Transforms | 8 dihedral; mirrors flip chirality | mutual-coupling sign | **No** — no orientation/chirality metadata |
| Coupled structures | transformer / stacked pair | RF baluns, combiners | **No** |
| Variant fan-out | Pareto set → `variant_dims` | placer choice | **No** — factory returns exactly 1 variant (same gap as resistor, GENERATOR_MODULARITY §1) |

Also outstanding in code: `DeviceType::Bjt` used as a proxy ("no Inductor variant yet"),
and `set_electrical(trace_w, outer_d, n_turns, trace_w)` overloads MOSFET-shaped fields.
