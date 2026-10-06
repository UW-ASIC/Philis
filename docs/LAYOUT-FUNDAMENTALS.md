# Layout fundamentals an analog P&R engine must honor — and where Philis stands

Scope: the device-, array-, placement-, routing-, substrate- and reliability-level techniques a hand-layout expert applies, each checked against the Philis tree at `dad330c` plus the working-tree changes. This file adds to `ref/Notes/constraints.html` (84 abstract requirement families) and `ref/Notes/roadmap.html`. Those files state the *contracts*. This file states the *concrete geometric techniques* and what the code actually does. It does not repeat the handbook's evidence or verification semantics.

**Status legend.**
- **BC**: by construction; the generator or router draws it.
- **S**: scored; a Rule/Θ term that search acts on.
- **M**: measured-only; computed or reported, but search cannot act on it, or it reads *unknown* for lack of data.
- **—**: missing.
- **K**: an already-known gap, listed briefly.

**Source keys.**
- **H**: Hastings, *Art of Analog Layout* 3e, 2023. Section numbers and book page numbers are used; physical PDF page ≈ book page + 1. The MOS rule list is §13.3 rules 1–24 (pp. 711–716), resistors §8.3.1, capacitors §8.3.2, vertical/lateral BJT §10.3.1/§10.3.2.
- **JM**: Johns/Martin/Carusone 2e, ch. 2.
- **PD**: Pelgrom & Duinmaijer, ESSCIRC 1988.
- **DM**: Drennan & McAndrew, JSSC 2003.
- **CCR**: Karmokar et al., "CC layout review", ASP-DAC 2022.
- **DACP**: Karmokar et al., TCAD 2023.
- **SL**: Schaper & Linnenbank, long-distance mismatch, TSM 2001.
- **LT**: Lienig & Thiele, EM-aware design, 2018.
- **CH**: Charbon et al., *Substrate Noise*, 2001.
- **LGS**: Lampaert/Gielen/Sansen 1999.
- **BSIM4**: BSIM4.5+ stress (SA/SB/SD) and WPE (SCA/SCB/SCC) models.
- **FAR**: Faricelli, CICC 2010.
- **OU**: Ou et al., "LDE-aware analytical analog placement", DAC 2015 / TCAD 35(8) 2016.

**sky130 facts used below.** Taken from the installed PDK and `pdks/sky130.json`:
- BSIM4 LOD is **active** in `sky130_fd_pr__nfet_01v8` and `pfet_01v8`: `saref/sbref` = 1.04–3.0 µm (binned), nfet `kvth0 = 9.8e-9`, `ku0 = -2.7e-8`, and pfet `ku0/kvth0` are corner-driven.
- There are **no WPE parameters** (`web/wec/kvth0we` are absent), so WPE is physical but invisible to simulation.
- Mismatch coefficients: `avt_n = 9.5`, `avt_p = 11.5 mV·µm` (MC-fitted). The gradient coefficient `S_VT` is absent, so every gradient-budget check reads *unknown*.
- Sheet resistances: li 12.8 Ω/□, poly 48.2 Ω/□, licon 585 (deck pex), met1 0.125 Ω/□.
- li has **no EM limit** in the PDK.

**Known gaps from the Razavi ch. 19 review, and where they stand now:**
- K1 (fixed): folding. `cellgen::folds` redraws each (kind, W, L) class as `k`× the fingers at `W/k`: a near-square row, fingers ≤ 20 µm, even per member for matched classes (ABBA) and odd for series stacks. The LVS reference folds alike; the netlist and simulation keep the schematic's W.
- K2 (fixed): dummies sit on the diffusion ends. Gate and outer S/D are tied to the bulk rail, and they are listed as LVS cards (`Macro::dummies`, `cellgen::dummy_cards`).
- K3 (fixed): the dummy count follows `dummy_required` and is no longer a variant, so matched cells drawn apart see one environment.
- K4 (fixed): parallel devices (same kind, W, L and nets) are one cell with one shared diffusion row. Series stacks with equal W are one chain row that shares each junction (Razavi Fig. 19.12, `Pattern::Chain`).
- K5 (fixed): `dr` prices a node over a matched cell for nets without a pin in it (`gr::KEEPOUT_COST`).
- K6 (fixed): even folds let mirror/load pairs merge. Route-matched pairs keep only mirror-pin orders, so their drains route as mirror images.
- K7 (partial): two-row cells share a tap strip (a blocked pair becomes the cross-coupled quad of Razavi Fig. 19.19; a wide device becomes Fig. 19.11 stacking). There is still no floorplan-level row or well sharing between cells.
- K8 (fixed): the LOD moat now extends past the bulk-tied outer dummy, and only where devices share a row, so no signal junction grows.
- Also fixed: S/D regions carry a full contact column, not one cut.

---

## 1. Constraint table

### 1a. Device level — MOS

| # | Constraint / technique | Why (mechanism, magnitude) | Source | Philis status | Check the engine should compute |
|---|---|---|---|---|---|
| 1 | Identical unit sections: same W, L, variant and oxide for every finger of a matched set | Effective W/L differ from drawn; short/narrow-channel effects give systematic mismatch | H §13.3 r1; JM p.98 KP; CCR §II ("identical units cancel all LDEs except LOD, WPE") | **BC**: one `unit_w/unit_l` per Unitization (`cell.rs:41-55`, `mosfet.rs:106-107`) | ∀ units u,v in set: `W_u=W_v ∧ L_u=L_v ∧ variant_u=variant_v` |
| 2 | Size for the matching class: voltage-matched → area (`σΔVT=A_VT/√WL`); current-matched → length | Pelgrom 1/√area. For moderate matching, H Table 13.4 gives 64 µm² at 1.8 V; Table 13.5 gives L ≥ 27 µm (NMOS, 1.8 V) | PD eq.(1); H r2, r3; DM (1/√area fails for wide/short and narrow/long devices) | **M**: `Pelgrom::new` turns `A_VT`, gate area and the offset spec into a gradient share η; η=0 when the random term already spends the spec (`annotator/emit.rs:74-78`). No resize (K1) | Report `σ_Vos = A_VT/√(W·L·nf)·√2` against spec per pair. For mirrors, `σ(ΔI/I) = (gm/I)·σ_VT`. Flag when the ratio > 1 |
| 3 | Avoid subthreshold for current-matched devices (STI stringers). Use V_ov ≥ 100 mV at all corners | Subthreshold hump makes mismatch blow up; not fixable by layout area | H r4, §13.2 "Stringers" p.697 | **—** (circuit/op-point) | From oppoint: flag matched devices with `V_GS − V_T < 100 mV`; propose finned/annular gate only if the PDK has it |
| 4 | Same channel orientation and signed current direction Φ = (1/N)Σφᵢ equal per device | Stress/tilted-implant orientation effects; severe for drain-extended devices | H r7, eq.13.61, pp.694-695 | **BC/S**: units carry `phi` (`mosfet.rs:178-184`). Merged macros are filtered by `currents_run_alike` (`library/cellgen.rs:160-176`). Multi-cell groups are not rotatable (`dp/lib.rs:558-560`). Nothing checks Φ for a matched pair that is not in a group | For every MatchingPair/CentroidGroup, including cross-cell: `Σφ_a·n_b == Σφ_b·n_a` on placed units (`UnitLib::placed` already applies `turn_dir`) |
| 5 | LOD/STI stress: equal SA/SB for every finger of a matched set, or SA/SB ≥ saref so the term vanishes | `ΔVT ∝ (1/n)Σ[1/(SAᵢ+L/2) + 1/(SBᵢ+L/2)]`. Stress shifts VT by ≥10 mV and gm by ≥10% (H p.696). In sky130 the model is live (`kvth0 = 9.8e-9`, `saref` up to 3 µm) | H §13.2 "LOD" pp.696-697, r12; CCR eq.(11); BSIM4 stress model | **K8 + S**: SA/SB per finger recorded at draw time (`Unit.sa/sb/lod`, `kernel/core/src/units.rs:31-32, 63`); the perf deck emits an equivalent `sa=/sb=` per MOS from the device's mean LOD (`frontend/library/src/perf.rs:33, 213-221`). No `sd`; simulated only with `performance` set | Per finger i: `SAᵢ = x_gateᵢ − x_OD_left`, `SBᵢ = x_OD_right − x_gateᵢ − L`. Per device `LODinv_d = mean(1/(SA+L/2)+1/(SB+L/2))`. Require `|LODinv_a − LODinv_b| ≤ ε` and emit `sa/sb/sd/nf` on the sim instance |
| 6 | OD-to-OD spacing (OSE): matched fingers see equal distance to the neighbouring active | STI width changes transverse stress over several µm; mobility shifts | H p.697 ("OD-to-OD stress"); FAR; OU (3 LDE sources: WPE, LOD, OSE) | **—** (neighbour cells are not examined) | For each matched unit, the distance to the nearest foreign `diff` edge in ±x/±y within 5 µm. Require equality across the set, or ≥ 5 µm |
| 7 | Poly spacing / gate-density environment (PSE): uniform poly pitch around every active gate; end dummies at the same pitch | Poly etch loading and stress liners give CD and VT shift for edge fingers | H r12, r21, r23; FAR | **K2**: dummies are off-diffusion at pitch `gate_l+sd_w` (`mosfet.rs:263-276`), with no neighbour poly check | Per matched finger: left/right poly-to-poly distance equal across the set. No foreign poly within 3–5 µm (moderate) or 5–10 µm (exceptional) (H r23) |
| 8 | WPE: gate-to-well-edge distance ≥ 3 µm (moderate), 5–10 µm (exceptional), **equal** across the set, NMOS included (distance to *foreign* nwell) | Scattered well-implant ions. Drennan'06 via H p.694: 5% ΔI near the edge, 25% closer | H §13.2 "WPE" pp.694-695, r19; JM p.98; BSIM4 SCA/SCB/SCC | **BC(partial)**: PMOS nwell inflated by 3 µm on matched groups (`mosfet.rs:353-372`). NMOS near a neighbour's nwell is **—**. sky130 model has no WPE, so simulation cannot see it | For each matched unit, `SC = min dist(gate, any nwell edge)`. Require `SC ≥ SC_min[class]` and `|SC_a − SC_b| ≤ δ`. For NMOS, include every placed cell's nwell |
| 9 | End dummies (full dummy on diffusion when L < 3 µm) with gates tied off; moat ≥ 5 µm (moderate) / 10 µm (exceptional) past the last active gate | Equalises etch, LOD and WPE for edge fingers | H r12 (p.713-714), Fig.13.52 | **K2/K3**: dummies tied to the bulk rail (`mosfet.rs:244-294`) | Every matched active finger has ≥ 1 on-diffusion neighbour gate at the unit pitch on both sides |
| 10 | S/D fully contacted along W (contact array at min pitch), both sides symmetric | Series R_S degenerates gm (`ΔI/I ≈ −gm·ΔR_S`); non-uniform current along the finger; contact EM | H §13.1 construction; JM §2.4.1 Fig.2.18 | **— (new)**: **one licon per S/D region** at mid-width (`mosfet.rs:220-231`). With the deck's licon at 585 (pex) and li at 12.8 Ω/□, hundreds of Ω sit in series with each finger | `n_cuts(region) = floor((W − 2·enc)/pitch)+1`. Check `R_S,finger = R_cut/n + R_li` ≤ `0.01/gm` (or the circuit's budget), and equality across the matched set |
| 11 | Metal (not poly) gate straps; gate contacted at both ends for wide or RF fingers | Poly strap geometry differs beside the channel (etch). Gate R thermal noise `4kT·R_g`, with `R_g ≈ R□·W_f/(3·nf·L)` one-sided and `/12` two-sided | H r22; Razavi RF / BSIM4 rgateMod | **—**: poly strap (`mosfet.rs:210-218`), one gate cut per finger on one end (`mosfet.rs:185-197`). At W_f = 5 µm and L = 0.15 µm, poly 48.2 Ω/□ gives ≈ 535 Ω per finger | `R_g` per device from the drawn poly. Require `4kT·R_g ≤ fraction of input noise` for the diff pair (`R_g ≪ 1/(gm)`). Straps on li/met1 via per-finger cuts |
| 12 | Equal gate poly extension on every finger, 1–2 µm beyond the rule minimum | Endcap and etch-rate variation | H r21 | **BC(partial)**: equal `poly_ext`, but only the rule minimum (`mosfet.rs:93,170`) | `ext_i = ext_j ≥ rule + 1 µm` for moderate/exceptional classes |
| 13 | No contacts on the active gate; no over-gate metal plates for moderate/exceptional matching; identical metal over every section otherwise | Hydrogen passivation and local processing raise mismatch | H r16, r17; H "Dummy metal and MOS matching" §13.2 | **K5**. No router keep-out over channels (`gr`/`dr` have no unit-aware blockage) | For each matched unit, the metal-area fraction per layer over `gate ∩ diff` must be equal across the set (≤ Δ%) or zero. Metal within 5–10 µm is "similar" |
| 14 | Block dummy-metal fill over matched arrays | Same as #13 | H r18 | **BC**: fill keeps out of matched cells (`library/fill.rs:16-17`) | Fill ∩ bbox(matched cell ⊕ 5 µm) = ∅ |
| 15 | Diffusion sharing: put the high-impedance node (drain) on the shared inner diffusions; minimise `C_db` | Junction C and perimeter; bandwidth | JM ch.1 (junction AD/PD), §2.4.1 | **BC**: single device region 0 = S, so drains are inner. Multi-device: shared sources (`mosfet.rs:148-152`) | `AD,PD` per device from the drawn regions, emitted to the sim deck; compare against the schematic `ad/pd` |
| 16 | Series-stack diffusion sharing (cascode M2.S = M1.D abutted, no contact) | Removes a contact and a junction; smaller node C at the cascode node | JM §2.4.1; common practice | **—** (Stack → `Proximity` only, `annotator/emit.rs:142-145`). Related to K4 | Detect `D(M1)=S(M2)`, same W, no other connection: offer a merged-diffusion variant |
| 17 | Body tap per device close and symmetric; tap on both sides for wide arrays (LU) | Backgate debiasing, latch-up, body-effect mismatch from unequal `R_body` | H §5.4.2-5.4.3, §12.2.9; CRATES open issue #5 (LU.2) | **BC(partial)**: one tap strip on one side per cell (`mosfet.rs:296-351`). No max diff-to-tap check | `max_{diff pt} dist(diff, tap) ≤ LU_max` (deck). For matched sets, equal tap distance per unit |
| 18 | Thin-oxide devices for matching; avoid pocket-implant devices at long L; diagonal PMOS only for exceptional matching | VT σ ∝ t_ox. Halo devices stop improving with L | H r5, r6, r24 | **—** (a sizing/circuit choice; out of layout scope) | Lint: warn when a matched set uses a thick-oxide variant where a thin one is legal |

### 1b. Device level — resistors, capacitors, BJTs

| # | Constraint / technique | Why | Source | Philis status | Check |
|---|---|---|---|---|---|
| 19 | Matched resistors: same material, identical segment W/L, same orientation | Width bias and head R. Orientation → piezoresistance | H §8.3.1 r1, r5, r6 | **BC**: one unit segment per Unitization; all segments vertical (`resistor.rs`) | Segment W/L/material equal; orientation equal |
| 20 | Interdigitated or common-centroid resistor arrays | Gradient cancellation | H §8.3.1 r8; H Table 8.4 | **— (new)**: `Interdig` is forbidden whenever `segments > 1` (`resistor.rs:35-37`, li jumpers would cross). Multi-segment matched resistors are only block-placed | Centroid of each resistor's segments coincides (reuse `CentroidGroup` over resistor units). Resistor units need `Unit` emission |
| 21 | Dummy segments on both array ends | Etch and edge environment | H §8.3.1 r9 | **—** (no resistor dummies) | Each end segment has a same-width poly neighbour at the array pitch |
| 22 | Minimum segment length ≥ 3×/5×/10× the rule minimum (minimal/moderate/exceptional) | Head/contact R variability | H §8.3.1 r10 | **BC(partial)**: `res_min_segment` 10 µm (`resistor.rs:159-175`) | `L_seg ≥ k_class · L_min` |
| 23 | Thermoelectric cancellation: even number of series segments, half in each current direction | Seebeck EMF at the heads | H §8.3.1 r11 | **BC**: segment counts are 1 or even, and the serpentine chain alternates direction (`resistor.rs:159-162`) | `Σ direction = 0` per series string |
| 24 | Interconnect R contributions scale with the ratio (R1 = 2·R2 ⇒ metal and via R also 2×); Kelvin taps for exceptional matching | Via/metal R is variable and not ratiometric | H §8.3.1 r23; H §15.4.4 "Star nodes and Kelvin connections" | **—** | `R_route(R1)/R_route(R2) = R1/R2 ± ε`, and `R_route ≪ allowed error·R` |
| 25 | No unconnected leads over matched resistors; field plate (met2) for exceptional poly resistors | Conductivity modulation, hydrogenation, coupling | H §8.3.1 r20, r21 | **—** (no keep-out) | Foreign metal ∩ resistor body = ∅, or a plate covers it tied to a quiet net |
| 26 | Matched resistor power dissipation limited; keep matched poly resistors away from large poly caps | Self-heating TCR error | H §8.3.1 r22, r25 | **M**: ThermalGradient covers devices with op-point power (`placement/thermal.rs:7-35`), not self-heating inside a resistor | `P = I²R` per segment ≤ limit; ΔT between matched resistors from the thermal field |
| 27 | Capacitors: identical square unit caps, CC or cross-coupled, dummy ring, electrostatic shield | Periphery/area; gradients; fringe fields | H §8.3.2 r1, r2, r7, r8, r9; JM §2.4.2 | **BC** for binary DACs (`cells/cap_array.rs:1-17`: point-symmetric, dummy ring). **—** for general matched caps: `capacitor.rs` draws **one merged plate per device, no pattern, no dummies, no shield** (`capacitor.rs:1-2,33-35`) | Unit-cap CC pattern for any Unitization of kind Capacitor. Centroid via `CentroidGroup`. Dummy ring present; shield plate tied to the quiet net |
| 28 | Non-integer ratios: equal area/perimeter ratio of the odd unit | Fringe C scales with perimeter | JM §2.4.2; H §8.3.2 r2 | **—** | For a non-unit cap: `A/P` equal to the unit cap's |
| 29 | Lead capacitance of every unit cap matched (vertical + lateral); leads spaced 2–3× the ILD or flanked by shields | Lead C adds directly to the ratio | H §8.3.2 r10; DACP §IV-B | **BC** for cap_array (TOP never crosses bottom routes, `cap_array.rs:12-17`). **—** elsewhere; the router is not told to keep out of plate stacks (`capacitor.rs:24-25`) | `|C_lead(a)·n_b − C_lead(b)·n_a| ≤ ε` from the extracted matrix; route keep-out over MOM/MIM plates |
| 30 | Bottom plate to the low-impedance node; nwell shield under the cap to decouple the substrate | Bottom-plate parasitic C to substrate noise | H §8.3.2 r6; JM §2.4.4 | **—** (pin P/N have no role; no well shield) | Bottom plate net ∈ {low-Z, ground} or a well is under the plate |
| 31 | BJT: identical emitters, ratios by unit count (4:1 to 16:1, even), CC with the unit in the centre, dummy ring | Emitter periphery, gradients, thermal | H §10.3.1 r1, r3, r5, r6 | **BC(partial)**: ratio by unit count (`bjt.rs:11-12`). Units are gridded in index order (`bjt.rs:78-80`): no centred 1:N, no dummies | Centroid(unit) = centroid(N units). The 3×3 or 5×5 pattern has the unit centred; outer ring of dummies |
| 32 | BJT: equal V_CE; contact geometry matches the emitter; stay below high-level injection | Early effect; emitter crowding | H §10.3.1 r13, r14, r16 | **—** (op-point) | Lint from oppoint: `|V_CE,a − V_CE,b| ≤ δ` |

### 1c. Matched-set level

| # | Constraint / technique | Why | Source | Philis status | Check |
|---|---|---|---|---|---|
| 33 | Coincidence: weighted centroids coincide (ABBA ≻ ABAB ≻ AABB) | Linear gradients cancel | H Table 8.4 r1 (PDF p.392); JM p.103 KP | **S**: `CentroidGroup` on physical units (`placement/cc.rs:6-30,118-124`), 1% coincidence tolerance | Already present. Keep the unit-moment check |
| 34 | Symmetry about both axes (cancels the quadratic term) and 2nd-moment equality | Nonlinear gradients | H Table 8.4 r2; Dai et al. ISCAS'05 (Nth-order) | **—** (`cc.rs:79-83` ponytail: 1st moment only) | `Σw·x² , Σw·y², Σw·xy` equal between sides within ε·extent² |
| 35 | Dispersion: many small CC sub-arrays rather than one large one | Limits the reach of nonlinear gradients | H Table 8.4 r3 | **—** | Count of sub-groups that each satisfy #33/#34 ≥ target |
| 36 | Compactness: array aspect ratio ≤ 3:1 (moderate), ≈ 1:1 (exceptional), per sub-array | Spindly arrays pick up residual gradients | H r9, Table 8.4 r4 | **—**: MOS is one diffusion row (`mosfet.rs:133-135`). An nf=16 pair is 32 fingers × ~0.8 µm ≈ 26 µm × 1.7 µm ≈ 15:1 | `max(w,h)/min(w,h)` of the active-unit hull ≤ `AR_max[class]` |
| 37 | 2-D common centroid (cross-coupled quad) for MOS | Better than 1-D, especially when more compact | H r10 | **—** (Cc1d only; the 2-D pattern exists only in `cap_array`) | Offer a 2-row AB/BA variant; checked by #33 plus #36 |
| 38 | Equal environment for every member: same dummy count (K3), well edge (#8), OD/poly neighbours (#6, #7), metal over gate (#13) | Every LDE cancels only if environments match | CCR §II; H r12 | **K3** plus gaps above | One "environment signature" per unit (SA, SB, SC, OSE, PSE, metal%). Require equality over a matched set |
| 39 | Proximity: matched devices adjacent, gradient term within budget `D ≤ η·A/(S·√WL)` | Pelgrom distance term; SL: long-distance mismatch grows for short channels and narrow resistors | PD eq.(1); SL | **S/M**: `MatchingPair` pull plus check (`placement/matching_pair.rs:17-36`). sky130 carries a proxy `svt_uv_per_um` flagged as not sky130 data (`pdks/sky130.json:109-110`), so the check runs on that proxy. `Proximity` edge gap (`placement/proximity.rs:7-14`) | Keep. Supply `S_VT` from a wafer/SL-style characterisation or a conservative default flagged *assumed* |
| 40 | Matched route R/C per terminal (drains, gates of a pair; capacitor top/bottom) | Unequal access gives an offset in dynamic and DC behaviour | CCR §V; H §8.3.2 r10 | **S**: `Differential` signature/RC (`routing/differential.rs:10-28`). dr mirror guide (`dr/lib.rs:1226-1290`). Symmetric nets are not fattened asymmetrically (`dr/lib.rs:548-568`) | Present. Add the terminal-resolved RC (Elmore to each unit pin) instead of the summed route |
| 41 | **Common-node (source/emitter/ground) resistance matched per member: star or Kelvin** | Mirror: `ΔI/I ≈ gm·ΔR_S`. Diff pair: `ΔV_os = I·ΔR_S`. Example: 100 µA, gm = 1 mS, ΔR = 10 Ω gives 1% | H §15.4.4; H §8.3.1 r23; SL (force/sense) | **—**: `Differential` covers pos/neg nets only; a shared source/rail net is never split per member | For each matched set on a common net N: `R(star → S_i)` from the routed tree; require `max−min ≤ ε/gm` (mirror) or `ε·V_os/I` (pair). The router seeds a star at the centroid |

### 1d. Placement / floorplan level

| # | Constraint / technique | Why | Source | Philis status | Check |
|---|---|---|---|---|---|
| 42 | Mirror symmetry of differential stages about one shared axis; self-symmetric tail on the axis | Matched parasitics and thermal symmetry | LGS ch.4; H r15 (die axis) | **S(hard)**: `SymmetryGroup` plus projection and compound moves (`placement/symmetry.rs:7-25,98-101`; `dp/lib.rs:311-324`) | Present |
| 43 | Matched devices on an isotherm of heat sources; separation ≈ 1 µm/mW from power devices | `Δ = TC·d·∂T/∂x` | H eq.8.23; H r14 | **S**: `ThermalGradient` from the live field (`placement/thermal.rs:7-35`). Power from the op-point | Present. Validate ΔT at the unit level, not the cell centre |
| 44 | Low-stress die region: not within 50–100 µm of the die edge; on a die symmetry axis; between bumps | Package stress gradients | H r13, r15; §10.3.1 r8, r9 | **—** (block-level engine) | Export a placement hint (matched-set centroid, class). Check at integration: `dist(edge) ≥ 100–250 µm` |
| 45 | Substrate isolation: noisy (clocked) devices far from sensitive ones; saturates past ~4× epi thickness | Resistive substrate transfer | CH ch.8; JM §2.4.4 | **S**: `Isolation` (`placement/isolation.rs:7-17`; emitted `annotator/emit.rs:250-273`) | Present. Add a transfer-impedance estimate (CH contact network) |
| 46 | DTI either-or banding | Legal trench geometry | process rule | **S(hard)**: `DtiBand` (`placement/dti.rs:7-28`) | Present |
| 47 | Shared well for same-body matched PMOS, equal well-edge distance | WPE (#8) and area | H r19; CCR §II | **BC(partial)**: each cell has its own nwell plus a halo (`mosfet.rs:353-372`); no cross-cell merge | For a set in separate cells: identical halos (true by identical cells). Offer a merge when the body nets are equal |
| 48 | Route-aware placement: reserve symmetric channels and pin access for matched/sensitive nets | A placement with no symmetric route fails MAT-11 | roadmap §4; ALIGN/MAGICAL flows | **M**: gr overflow only; no reservation | Per symmetric pair: a free mirrored corridor exists between the pins after placement |

### 1e. Routing level

| # | Constraint / technique | Why | Source | Philis status | Check |
|---|---|---|---|---|---|
| 49 | Symmetric routing of differential nets: equal length, layers, via count and neighbours | Matched RC and coupling | CCR §V; LGS ch.5 | **S** (#40) | Add the neighbour-environment term (coupling to each side equal) |
| 50 | Crosstalk: keep aggressors away from sensitive nets; total coupling budget | Victim noise | LGS eqs.2.29-2.33; JM §2.4.4 | **S**: `CrosstalkExclusion` (`routing/crosstalk.rs:10`), `CouplingBudget` (`routing/coupling.rs:31-40`), gr keep-away (`gr/lib.rs:283-307`) | Present. Add cross-layer (overlap) coupling from the deck `pex` |
| 51 | Shielding sensitive lines with grounded tracks; shield tied to a quiet, shield-only ground | Contains fringe fields; a noisy return injects noise | JM §2.4.4 Fig.2.36; H §8.3.2 r8 | **S/BC**: `Shield` (`routing/shield.rs:8-19`), dr generates shields (`dr/lib.rs:484-493`). Same-layer only; tie impedance is not measured | Plus top/bottom plates; reference ∉ noisy nets |
| 52 | Per-net C budgets and shared performance budgets from sensitivities | Node C sets poles, not total C | LGS ch.2; roadmap §3 | **S**: `ParasiticBudget` (`routing/parasitic.rs:9-15`), `PerformanceBudget` (`routing/performance.rs:8-17`) | Present |
| 53 | **Series R in the performance loop** (routes, li, contacts, gate) | Degeneration, gate noise, IR shift of the bias | LGS eq.2.33; roadmap §3 | **S (routed part)**: the perf deck adds one `Rpex_*` per terminal branch with routed R (`frontend/library/src/perf.rs:197-207`, from `Flow::parasitics`). Cell-internal (li, contact) and gate R are not in it | Emit an R network (or at least a lumped R per terminal branch from `Stack::path_resistance_ohm`) into the perf deck |
| 54 | Separate analog and digital supplies/grounds; no shared-impedance returns | Common-impedance coupling | JM §2.4.4 Fig.2.34; constraints NET-08 | **—** | For each pair of nets classed analog/digital supply: no shared segment; shared R between returns ≤ budget |
| 55 | Kelvin sense connections for references and sense resistors | Removes the IR drop from the measurement path | H §15.4.4; SL | **—** | Sense pin gets its own branch from the device terminal; `I_sense·R_sense-lead ≤ ε` |
| 56 | Noisy nets not routed over matched or sensitive devices (caps, resistors, gates) | Coupling plus hydrogenation | H §8.3.2 r8; §8.3.1 r21; H r17 | **—** (see #13, #25, #29) | Unit-aware keep-out layer per matched cell, with a priced cost in gr |
| 57 | Width sizing by criticality (wide supplies, narrow high-Z nodes) | R vs C tradeoff | todaes22yl; roadmap §4 | **BC(partial)**: supply/signal fatten caps, EM floor (`dr/lib.rs:107-133,536-568`) | Discrete width choice by sensitivity `∂f/∂R` vs `∂f/∂C` |

### 1f. Substrate / well / latch-up

| # | Constraint / technique | Why | Source | Philis status | Check |
|---|---|---|---|---|---|
| 58 | Guard rings: majority-carrier rings around sensitive devices; minority-carrier (collector) rings around injectors | Substrate noise collection, latch-up | H §5.4.4, §14.2; JM Fig.2.35 | **BC**: every MOS in a block requests a ring tied to its **bulk** (`annotator/constraints.rs:66-84`), drawn post-placement (`post_cell.rs:1-19`) on li only (`post_cell.rs:193-194`) | Ring type per role (victim vs aggressor). The ring's net should be a quiet/dedicated ground, not the device's noisy rail (CH ch.8 return impedance) |
| 59 | Ring return impedance low and not shared with the signal ground | A ring on a noisy return re-injects noise | CH ch.8; constraints ENV-06 | **M**: `max_ring_resistance_mohm` sizes the contact rows only | `R(ring → pad)` and shared segments with the signal ground from the routed tree |
| 60 | Latch-up: tap within the deck's distance of every diffusion; n+/p+ spacing | Parasitic SCR | H §5.4.2; JM §2.2.4 | **BC(local)**: per-cell tap strip. **—** global check; LU.x is not in the deck (CRATES #5) | `∀ diff: dist(diff, same-type tap) ≤ LU_max` over the final GDS |
| 61 | Well/substrate shields under sensitive routes and caps | Isolates from the substrate | JM §2.4.4 Fig.2.36 | **—** | Optional well under a victim route/cap, tied to the shield ground |

### 1g. Reliability — EM, antenna, HCI, self-heating

| # | Constraint / technique | Why | Source | Philis status | Check |
|---|---|---|---|---|---|
| 62 | DC EM per segment and per via array, temperature-derated (Black) | Void or hillock failure | LT eqs.3.5-3.11; H §5.1.3 | **S**: hard `Electromigration` per routed shape and via group on the final routes, pin-access jogs and cuts included (`routing/em.rs`; per-shape currents `routing/current.rs`, from `Routes::terms`). dr sizes widths and via arrays toward it (Θ) | Present |
| 63 | RMS (Joule) and peak limits; Blech exemption only where valid | AC signal nets self-heat | LT ch.3-4 | **—** (`em.rs:90` ponytail: DC only) | `I_rms ≤ J_rms·w·t` from transient; `I_peak` similarly |
| 64 | EM and current crowding **inside cells** (li, licon, single contacts) | li has no PDK EM limit; one cut carries the whole finger current | LT ch.4 (crowding); H §5.1.3 | **—**: one licon per S/D (`mosfet.rs:220-231`); li is not covered (deck note) | `I_finger / n_cuts ≤ I_cut,max`; li current density ≤ an assumed limit, flagged *assumed* |
| 65 | Antenna ratio per layer or stage, with diode or jumper repair | Plasma charging of gate oxide | H §5.1.6 pp.228-229; JM §2.2.3 | **S**: `Antenna` (`routing/antenna.rs:8-22`), using the real gate area (dad330c). **S** repair by diode insertion: `elaborate::antenna_diodes` (`frontend/library/src/elaborate.rs:459`) per epoch (`frontend/library/src/lib.rs:1474`), adopted into the schematic (`adopt_devices`, `lib.rs:2352`); `frontend/library/tests/antenna_diode.rs`. No layer-jump repair | Present. Add a layer-jump repair (route the gate-side segment on the top layer last) before diodes |
| 66 | Redundant vias / contacts on critical nets | Yield, EM, R variability ("via R notoriously variable", H r23) | H §8.3.1 r23; roadmap MFG-03 | **BC(partial)**: via arrays on fattened trunks only (`dr/lib.rs:606`) | ≥ 2 cuts at every layer change of a matched/critical net, same count per side |
| 67 | HCI/NBTI: matched devices under equal stress (V_DS, V_GS, duty) | Unequal aging gives drift in matching | H §13.2.3 (pp.698-704); H §5.3.1, §5.3.4 | **—** (circuit/op-point) | Lint: `|V_DS,a − V_DS,b|`, `|V_GS,a − V_GS,b|` for matched pairs over the testbench |
| 68 | Metal density or fill without disturbing sensitive or matched nodes | CMP; added C | H r18; JM §2.2.2 | **BC**: `library/fill.rs` (keep-outs, grounded fill) | Present |

---

## 2. Top 10 missing items, ranked by impact on mismatch and performance

**Status after the second pass** (the items below are kept as written, for reference):
1. LOD (fixed):
   - `Unit` carries `sa`/`sb`.
   - `CentroidGroup` prices the LOD ΔVT between sides using the deck's `lod_kvth0_{n,p}_mv_um` (sky130 9.8 / 32.9).
   - The post-layout deck gets an equivalent `sa`/`sb` per device (`perf::Parasitics`).
2. R in the post-layout loop (fixed, routed part):
   - Each device terminal gets its branch R from the routed port graph (`Stack::terminal_resistance_ohm`, `Rpex_*` cards).
   - Cell-internal li/licon R is not included.
3. MOS contacting (partly fixed):
   - A full contact column per S/D region.
   - The fold count now has a gm floor: `N ≥ √(5·gm·R□·W/3L)`, keeping gate R under 1/(5·gm).
   - Still open: metal gate straps, two-ended gate contacts, and a cell EM check (the deck has no licon EM limit).
4. Common-node R (fixed as a measured budget):
   - `CommonNode` gives each matched pair's shared source ΔR from its feed pins, with budget η·σ_rand/I_D.
   - No dedicated star routing.
5. Neighbour environment (fixed as a measured budget):
   - `Environment` checks WPE distance to the nwell-union edge (≥ 3 µm and equal across the pair) and OSE distance to foreign diffusion (equal).
   - PSE is not measured; the dummies set the poly pitch.
6. 2-D arrays (fixed): 2-row MOS variants (cross-coupled pair, 2-D centroid).
7. Metal over matched devices (fixed): dr charges foreign nets for crossing a matched cell (`gr::KEEPOUT_COST`).
8. General matched capacitors (fixed):
   - `cap_array` takes any integer-ratio set with a centrosymmetric grid and a dummy ring.
   - Dummies are tied to device 0's bottom plate.
9. Resistor matching (partly fixed):
   - Interleaved multi-segment variants with met1 jumpers.
   - Per-segment `Unit`s.
   - No end dummies yet: they need a tie net.
10. BJT ratioed arrays (fixed): centre-out point-symmetric unit fill (`kernel/cells/src/bjt.rs:16-21`, `pattern::centro_assign`); ratioed groups from `cellgen::bjt_groups` (`frontend/library/src/cellgen.rs:745`).

Also done:
- Width follows need: supply trunks go wide only when their IR budget breaks, and C-budgeted signals get their EM width only.
- Same-bulk PMOS cells share one well.
- Pin reservation includes the wire spacing, which fixed dac4's routing spacing violations.

1. **LOD (SA/SB) — now computed and simulated (#5 row); the text below is the original gap.** sky130's model *does* respond: `kvth0` and `ku0` are active and `saref` goes up to 3 µm. Yet Philis draws a blanket 3 µm moat and simulates without `sa/sb/sd`. This is both mismatch and systematic ΔVT/Δµ that the perf loop cannot see.
   - In `cells/mosfet.rs`, record `(SA, SB)` per finger in `pnr_core::Unit` next to `phi` (known exactly at draw time: gate x and diff extent).
   - Add a `LodBalance` placement rule, a sibling of `CentroidGroup`: per-device `mean(1/(SA+L/2)+1/(SB+L/2))`, equal across the set.
   - Emit `sa/sb/sd/nf` on the instance card in `library/oppoint.rs` and `perf.rs`, then size the moat to `saref`, not 3 µm.

2. **The post-layout loop was C-only (#53); routed branch R is in now (#53 row), cell-internal and gate R are not.** It gives no R for routes, li, licon or gates, so source degeneration, gate noise and bias IR shifts are never evaluated.
   - `library/perf.rs::deck` should add one resistor per routed branch (from `analog::routing::Stack::path_resistance_ohm`) plus cell-internal li/licon R. Start with a lumped R per terminal branch.
   - Rank epochs on the same `lex_key`.

3. **S/D and gate contacting in the MOS generator (#10, #11, #64).** One licon per S/D region, a poly gate strap, and a one-sided gate contact. This gives large and unequal R_S and R_g, contact EM with no check, and violates H r22.
   - In `mosfet.rs:220-231`, fill each S/D region with a licon column at the deck pitch over W, with an li/met1 strap.
   - Replace the poly strap (`:210-218`) with an li/met1 strap over per-finger gate cuts. Add a two-ended gate variant for wide fingers.
   - Add cell-level EM: `I_finger/n_cuts ≤ I_cut`.

4. **No common-node R matching or Kelvin/star routing (#41, #55, #24).** Shared source and ground nets of mirrors and pairs are routed as ordinary trees, so ΔR_S is unbounded (10 Ω at gm = 1 mS gives a 1% mirror error).
   - Add a routing rule `CommonNodeBalance { net, members }` that computes per-member `R(root → S_i)` on the routed tree (reuse `stack.rs` path R).
   - dr seeds a star at the members' centroid and grows equal branches, as the existing `mirror_guide` does for pairs.

5. **Neighbour environment of matched units: OSE, PSE and NMOS WPE (#6, #7, #8, #38).** Only the cell's own moat and halo are controlled. Neighbouring cells' diff, poly and nwell edges are never measured.
   - After placement, compute per-unit `SC` (nearest nwell edge), OD-to-OD and poly-to-poly distances from the placed geometry into an environment signature.
   - Add a placement rule that requires equality across a matched set and minimum SC by class. The OU TCAD'16 analytic forms are drop-in costs.

6. **MOS arrays are 1-D only: no 2-D common centroid, no aspect-ratio control (#36, #37, #34).** A long single row picks up residual nonlinear gradients, and centroid checks cover only the first moment.
   - Add a 2-row `Cc2d` MOS variant (AB/BA with a shared tap between rows) in `mosfet.rs` enumerate.
   - Add second-moment terms to `CentroidGroup::used` (`cc.rs:79-83`).
   - Add an `AR ≤ AR_max[class]` check on the active-unit hull.

7. **Metal over matched devices (#13, #25, #56; extends K5).** There is no unit-aware routing blockage for gates, resistor bodies or cap plates. `capacitor.rs:24-25` explicitly notes that the router is not told.
   - Each generator emits a `keepout` rect set (gate∩diff, resistor body, plate stack) in `Macro`.
   - gr prices it as a cost field. dr treats it as hard for exceptional matching, or requires an identical metal pattern per unit for the other classes.

8. **General matched capacitors (#27–#30).** Outside the DAC generator, a matched cap is one merged plate per device, with no CC, no dummies, no shield, and no lead-C matching.
   - Make `cap_array`'s pattern and dummy machinery generic: any capacitor Unitization with integer ratios gets a CC unit grid and a dummy ring.
   - Emit `Unit`s so `CentroidGroup` applies. Check lead C with the extracted matrix, as in #29.

9. **Resistor matching (#20, #21, #24, #25).** Interdigitation is disabled for `segments > 1` (`resistor.rs:35-37`), there are no end dummies, and nothing ratios the interconnect R.
   - Route inter-segment jumpers on met1 over li so interleaved segments do not short. Then allow `Interdig` for all segment counts.
   - Add dummy segments at the array ends and emit resistor `Unit`s.
   - Add a ratio check on jumper and via R per device.

10. **Ratioed BJT arrays (#31).** Units are gridded in index order, with no centred 1:N and no dummy ring. This dominates PTAT/bandgap accuracy.
    - In `bjt.rs`, place the unit device at the grid centre and the N others around it (a 3×3 for 1:8), plus an optional dummy ring.
    - Emit `Unit`s so `CentroidGroup` checks it. This depends on CRATES issue #1 (a real concentric BJT) for DRC cleanliness.

**Honourable mentions, not ranked:**
- RMS/peak EM (#63).
- A quiet return for guard rings (#58, #59).
- Series-stack diffusion sharing (#16).
- Measured sky130 `S_VT` to replace the flagged proxy (#39).
- A global latch-up distance check (#60).
- A layer-jump antenna repair before diodes (#65; diode insertion exists).

Sources (web): [Ou et al., LDE-aware analytical analog placement (DAC'15)](https://dl.acm.org/doi/10.1145/2744769.2744865), [TCAD version](https://ieeexplore.ieee.org/document/7329957/), [Faricelli, Layout-dependent proximity effects in deep nanoscale CMOS (CICC'10)](https://ieeexplore.ieee.org/document/5617407/), [Faricelli slides](https://ewh.ieee.org/r5/denver/sscs/Presentations/2009_04_Faricelli.pdf).
