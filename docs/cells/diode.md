# Diode — complete shape/variant catalog

**Device family:** junction / Zener / Schottky / varactor / ESD diodes.
**Framework:** one axis per section, per `docs/frontend/GENERATOR_MODULARITY.md` §4-5 — the
enumerator owns the choice space, one drawing routine consumes a `DiodeSpec`.
**Sources:** Hastings AOAL 3rd ed. ch11 (§11.1 standard bipolar, §11.2-11.3 CMOS/BiCMOS +
matching), ch14 (ESD), ch1 (junction physics), ch7 (varactors), ch10 (piezojunction).
**Diagrams:** `docs/cells/diode.html` views (a)–(f); simplified Excalidraw source in
`docs/cells/diode.excalidraw`.

**Code status:** `frontend/cells/src/generators/diode.rs` implements a single striped
junction layout; `DiodeCell.kind` (Junction/Schottky) is stored-not-read
(`frontend/cells/WARNING.md`) — both kinds currently draw identical geometry, and the
Schottky guard ring (mandatory, §5) is missing.

---

## §1 Construction kinds

The three broad categories — **PN junction, Zener, Schottky** — depend on different
conduction mechanisms and *never match across categories* [Hastings 11.3]. Within each,
several constructions exist:

| Kind | Structure | Current direction | Key constraint | HTML view |
|---|---|---|---|---|
| N+/P-well (NSD/P-epi) junction | NSD island in P-substrate/P-well; substrate = anode | **Vertical** (electrons diffuse down into P-epi, spread laterally tens of µm) | Cathode must stay ≥ substrate potential; superb ESD device (distributed ballasting, deep heat dissipation) [11.2.1] | (b), (f) analog |
| P+/N-well (PSD/N-well) junction | PSD anode island(s) inside N-well; NMoat ring = cathode | **Vertical** into well, laterally to cathode ring — really the E-B of a substrate PNP [11.2.1] | Forward bias injects into substrate unless hole-blocking guard exists; latchup rules if on a pin | (a), (f) |
| Well/substrate junction | N-well = cathode, P-epi = anode | Vertical | Cathode below substrate potential only; antenna/ESD use [11.2.1] | — |
| Diode-connected MOS | Gate+drain = anode (N) or per Fig. 11.14 A–F; 6 backgate configs | Lateral (channel) + body diode | Backgate wiring axis (separate / antiparallel / parallel body diode); substrate injection risk in non-isolated PMOS configs [11.2] | — |
| Diode-connected BJT | C+B shorted = anode, E = cathode; CBE order, merged C-B contact | Vertical (NPN preferred: high β, high f_T) | Must sit on a **beta plateau** for ratioed use; reverse limited to ⅔·BV_EBO [11.1.1, 11.3.1] | — |
| Zener — surface (E-B, PSD/N-well) | E-B junction or PSD in N-well + **poly field plate ring tied to anode** | Vertical avalanche at junction sidewall/surface | Zener walkout/walkback; ≤ ~100 µA/µm cathode periphery; field plate forces subsurface breakdown [11.1.2, 11.2.1] | (a) variant |
| Zener — buried/subsurface | P+ plug under enlarged emitter / DWell / high-energy implant | Vertical, **below thermalization depth** (~60-140 nm) | No walkout; ±50-200 mV V_BR control; 0TC point 5.0-5.4 V; Pelgrom's law does NOT apply (winner-takes-all) [11.1.2, 11.3.2] | — |
| Schottky | Silicide/metal barrier on lightly doped N moat (< 10¹⁷ cm⁻³) | Vertical (majority-carrier, fast) | **Guard ring around the Schottky contact is mandatory** (PSD/base field-relief ring) or field plate for small devices; barrier material availability is PDK-dependent (PtSi/PdSi/CoSi₂/NiSi yes, TiSi₂ no) [11.1.3, 11.2.3] | (d) |
| Varactor — junction | Reverse-biased PSD/N-well or B-C junction as C(V) | Vertical (displacement) | C ∝ (1 − V/φ₀)^−m, m ≈ ⅓ diffused; ±20 % absolute [ch7] | (a) reused |
| Varactor — MOSCAP | Accumulation-mode NMOS in N-well | Vertical (displacement) | Handled by the capacitor generator family; listed here for completeness | — |
| ESD diode | Large-periphery NSD/P-epi or PSD/N-well, interdigitated strips | Vertical per finger | ≤ ~3 Ω series R for 2 kV HBM (~1.3 A peak); ~50×50 µm²; **no silicide block** on NSD/P-epi (keeps R low, ballasting is inherent) [11.2.1, ch14] | (b) |
| Poly diode (PN / PIN) | Junction in poly stripe; PIN adds intrinsic gap | **Lateral** (in the poly film) | Never for matching (grain-boundary G-R variability); thermally fragile; lightly doped region ≥ ~5 µm drawn [11.2.1, 11.3 Rule 1] | — |

**Vertical vs lateral:** every monocrystalline junction diode above is a *vertical-current*
device — the junction plane is horizontal and current crosses it downward before spreading
to the surface cathode ring/fingers. This is why a 3D/cross-section view is warranted:
see HTML view **(f)**. Only poly diodes and the channel path of diode-connected MOS are
lateral.

---

## §2 Geometry axis — area vs perimeter

Junction current has an **area component** (bottom plane) and a **perimeter component**
(sidewall, different grading and curvature — ch1). The drawn A/P ratio is a real design
axis:

| Geometry | A/P | Optimizes | Use case | HTML view |
|---|---|---|---|---|
| Square / circular (32-64-gon) island | Max | Min peripheral random variation → **matching** [11.3 Rule 6]; min sidewall cap fraction | Matched diodes, references | (a) |
| Elongated stripe | Low | Series-R at high current (Rule 6 exception); Zener periphery current limit | High-current, PSD/N-well Zener strips between NMoat contacts [11.2.1, Fig 11.23] | (b) fingers |
| Multi-stripe interdigitated anode/cathode | Low, huge total periphery | **ESD**: minimum series R, distributed ballasting; PSD outermost strips for uniform NSD conduction [11.2.1] | ESD pads, power diodes without NBL | (b) |
| Ring anode (anode annulus, center cathode) | Medium | Uniform radial spreading; quatrefoil Zener flanged-metal variants [11.2.1, Fig 11.24] | Zener, Schottky cathode-surround | (d) inverse |
| Waffle (array of small anode islands in common cathode) | Tunable by island count/size | Periphery scales with island count at fixed area; contact redundancy; unit-based ratios | Mid-current diodes, unit arrays | (c) |
| Elongated Schottky contact + surrounding cathode contacts | — | Mitigates high CMOS Schottky series R (no NBL/sinker) [11.2.3] | CMOS Schottky, few-mA limit | (d) |

Rules that gate the axis: min feature 2-10× minimum size (Schottky barrier shifts below
~20 µm) [Rule 5]; very large single junctions worsen nonlinear-gradient sensitivity —
prefer **arrays of moderate devices** [Rule 5].

---

## §3 Formation

| Formation | Construction | Why | Source |
|---|---|---|---|
| Unit-diode array (matched/ratioed) | N identical unit junctions, interdigitated / common-centroid; identical junction geometry mandatory, isolation may be shared if only majority carriers cross it | ΔV_BE / bandgap-style ratios — but note these are usually **diode-connected BJTs**, and ratioed pairs demand a beta plateau; ratioed *Schottky* pairs are forbidden (size-dependent barrier) [11.3 Rules 3-5] | ch11.3 |
| Series stack | k diodes in separate isolation tanks, anode→cathode chained | Voltage > single junction: stacked E-B Zeners for >9 V clamps, + diode-connected BJTs to trim/compensate TC; ESD primary devices stacked (each ~2× standalone size) [11.1.2, ch14 Rule 11] | ch11.1, ch14 |
| Parallel fingers | Interdigitated strips on shared well/moat | Current capability, series-R reduction (ESD, power) [11.2.1] | ch11.2 |
| Anti-parallel pair | Two diodes, reversed | Ground-domain interconnect / clipping clamps [ch14] | ch14 |
| Cross-coupled quad (quatrefoil) | 4 circular Zeners in common tank, clover-shaped anode contacts | Best-effort Zener matching, still only 50-100 mV [11.3.2, Fig 11.24] | ch11.3 |

---

## §4 Orientations & transforms

See HTML view **(e)**.

| Case | Legal transforms | Notes |
|---|---|---|
| Square/circular junction diode | All 8 dihedral (R0/R90/R180/R270 × mirror) | Fully symmetric layout — orientation is a free axis for the placer; junction diodes have no channel/implant direction of their own |
| Stripe / finger diode | 8 transforms geometrically legal; R90 swaps bbox aspect | Pin sides move — placer-relevant, matching-neutral for a standalone device |
| Matched pairs/arrays | Devices must be **superimposable by translation only** if the layout is asymmetric (same rule as MOS in GENERATOR_MODULARITY §2.3); mirrored copies acceptable only when the device is mirror-symmetric about the mirroring axis | The current `diode.rs` MX-mirroring of odd instances is safe only because the drawn stripe is symmetric; once anode/cathode ends differentiate the layout, mirroring breaks matching |
| Directional-implant / NBL-shadow processes | Keep NBL shadow off the junction (enlarge NBL so shadow misses); Schottkies especially — barrier is at the surface [11.3 Rule 12] | Constrains placement inside the tank rather than the transform per se |
| Piezojunction sensitivity | Not transform-limited, but **placement**-limited: matched PN diodes ≥ 200 µm from die edges, never in corners, on die symmetry axes, die-center preferred [11.3 Rules 10-11; ch10 piezojunction] | I_S shifts with normal stress σ_x, σ_y; pairs on symmetry axes see equal stress |
| Zener corners | Remove/round corners of matched Zeners (circular/oval emitters) to avoid preferential corner breakdown [11.1.2, 11.3.2] | A shape sub-axis: rectangle vs octagon vs 32-gon |

---

## §5 Guard structures

| Structure | When | Construction | Source |
|---|---|---|---|
| **Schottky field-relief guard ring — mandatory** | Every guard-ringed Schottky (default for large devices) | PSD (CMOS) or base-diffusion (bipolar) ring enclosing the Schottky contact edge; eliminates edge field intensification, breakdown rises to BV_CBO / PSD-N-well avalanche | 11.1.3, 11.2.3; HTML (d) |
| Schottky field plate | Small Schottkies where ring area is prohibitive | Metal flange over thin oxide past the contact edge; less predictable breakdown, higher leakage than a ring | 11.1.3 |
| Hole-blocking guard ring | Power/ESD diodes in BiCMOS | N+ sinker ring + NBL floor around anode; ring width ≥ 2× t_epi above 100 mA; NBL extends to sinker outer edge | 11.1.4, 11.2.2 (Fig 11.18A) |
| Hole-collecting guard ring | Shallow-well processes without effective sinker | Zero-biased P-well ring; saturates — few mA only | 11.2.2 (Fig 11.18C) |
| **Electron-collecting guard ring (ECGR)** | Nearly all ESD diodes/devices — they inject electrons into substrate during negative strikes | N-well strip tied to supply between ESD device and victim circuitry suffices to cut lateral flow | ch14 |
| Poly field plate ring | PSD/N-well Zener, always | Tied to anode, forces subsurface avalanche, prevents walkout; metal plates ineffective (thick oxide) | 11.2.1 (Fig 11.16) |
| Well ties / tank connection | Every isolated diode | Never float the tank (parasitic substrate PNP bleeds anode current); tie to anode, cathode, or any node ≥ anode; avoid high-impedance nodes at µA levels (documented DWell Zener failure) | 11.1.2, 11.2.2 |
| Minority-carrier spacing | Merged matched diodes | Emitter-to-*unconnected*-emitter rule + 2-4 µm margin | 11.3 Rule 13 |

---

## §6 Contact variants

| Sub-axis | Domain | Rationale |
|---|---|---|
| Contact density | sparse / full array / single large opening | Maximize contacts + metal on ESD (min R) [11.2.1]; arrays of small contacts beat one large opening for electromigration periphery [11.1.4]; some PDKs *require* arrays — then Schottky needs fully silicided moat as the barrier [11.2.3] |
| Anode contact placement | center plug / distributed array / ring | Ring/flanged metal gives uniform vertical E-field on Zeners (quatrefoil); center-only concentrates current |
| Cathode contact placement | point / row / full surrounding ring | Ring cathode = uniform radial spreading resistance; PSD strips *between* NMoat contacts minimizes N-well-R mismatch on matched PSD/N-well diodes [11.3.1, Fig 11.23] |
| Silicide / ballast | full silicide / silicide-block ≥ 2 µm around contacts | NSD/P-epi ESD: NO block (keep silicide). PSD/N-well Zener: block (or +2 µm moat-over-contact in non-silicided flows) for pulse robustness [11.2.1] |
| Kelvin sensing | 2-terminal / 4-terminal (separate force & sense contacts on each electrode) | Precision V_F measurement / references where series-R drop matters (exceptional tier; same motive as resistor Kelvin heads, Hastings ch6) |
| Diffusion overlap of contact | minimum / +2 µm | Moderate/exceptional matching: enlarge overlap so misalignment doesn't modulate lateral current [11.3 Rule 14] |

---

## §7 Summary enumeration table

Axes for a future `DiodeSpec` (GENERATOR_MODULARITY §5 pattern). "Impl?" = present in
`diode.rs` today.

| Axis | Domain | Matching impact | Impl? |
|---|---|---|---|
| Construction kind | NSD/P-well ∥ PSD/N-well ∥ well/sub ∥ diode-MOS(6 cfg) ∥ diode-BJT ∥ Zener-surface ∥ Zener-buried ∥ Schottky ∥ varactor ∥ poly-PN/PIN | Categories never match each other; poly & surface Zener never match at all [Rule 1] | Partial — `DiodeKind::{Junction,Schottky}` enum exists but is **stored-not-read**; both draw the same layout |
| Island geometry | square / circle(32-64-gon) / stripe / ring / waffle | High — A/P ratio, Rule 6 | No — single stripe only |
| Finger/island count | 1..N interdigitated strips or M×N waffle | ESD/current axis; unit arrays for matched | Partial — n_devices copies at fixed pitch, no anode/cathode alternation |
| Formation | single / unit-array / series stack / parallel / antiparallel / quad | Unit arrays mandatory ≥ moderate; CC mandatory ≥ moderate [Rules 7-8] | Partial — flat interdig, no CC, no stack |
| Orientation/transform | 8 dihedral; superimposability for matched asym layouts | Medium (translation-only rule); placement stress rules [Rules 10-11] | No — hardcoded R0/MX alternation |
| Guard structure | none / Schottky ring / field plate / hole-block / ECGR / poly plate | Schottky ring mandatory; ESD ECGR mandatory | **No — missing entirely (blocks real Schottky support)** |
| Well tie / tank hookup | anode / cathode / supply / grounded sinker | Correctness, not matching | No |
| Contact scheme | density × placement × silicide-block × Kelvin | Rules 14, EM, ballast | No — one contact per terminal |
| Corner treatment | square / octagonal / rounded | Zener-critical | No |
| A/P target | continuous (island sizing) | Cap-vs-R Pareto axis for placer `variant_dims` | No |

**Missing prerequisites** (same as WARNING.md): per-kind PDK layer sets (nwell, psdm/nsdm,
silicide-block, NBL/sinker where available) and `AnalogParams` (current level → guard/
finger sizing; matching group → tier floors).

---

## Diagram index (docs/cells/diode.html)

| View | Content | Doc section |
|---|---|---|
| (a) | Square P+/N-well junction diode, N+ cathode ring | §1, §2, §6 |
| (b) | Interdigitated ESD diode fingers | §1, §2, §3 |
| (c) | 3×3 waffle diode | §2, §3 |
| (d) | Schottky with mandatory guard ring | §1, §5 |
| (e) | Stripe diode under 8 dihedral transforms | §4 |
| (f) | 3D isometric cutaway + 2D cross-section of P+/N-well vertical diode | §1 (vertical current) |
