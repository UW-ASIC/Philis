# MOSFET — Shape & Variant Catalog

**Device:** MOS field-effect transistor (planar bulk CMOS baseline; FinFET/GAA notes where quantization changes the rules).
**Terminals:** G (gate, in), S (source), D (drain), B (backgate/bulk — implicit via substrate/well unless isolated tank gives a 5th/7th terminal [Hastings ch12, 12.2.2]).
**Generator:** `frontend/cells/src/generators/mosfet.rs` (`MosfetFactory` → `MosfetSpec` →
`MosfetCell`). Today it implements: 4 patterns filtered by tier, **spec-driven `nf`
enumeration** (`feasible_nf`: refolds of W within PDK `min/max_finger_width`, matched
groups even-only for chi=0), factory returns a Pareto set of up to 8 shape variants
ordered best-first (grade → netlist-nf → squarest bbox, deduped by estimated dims),
multiplier folded as extra same-row fingers, gate-vertical only, single-side gate stub,
alternating S/D starting at S, **tier-driven dummy count** (1/edge default, 2/edge
exceptional), no guard ring, no orientation metadata.
**Framing doc:** `docs/frontend/GENERATOR_MODULARITY.md` §2 lists the axes; this file is the per-axis deep dive.
**Diagrams:** all figures live in `docs/cells/mosfet.html` (self-contained SVG); referenced per section below. Simplified Excalidraw copies of Fig. (a) and (c) in `docs/cells/mosfet.excalidraw`.

Sources: [Hastings ch12], [Hastings ch13], [PNR_ANALOG S0] = `docs/books/PNR_ANALOG/01_S0_CELL_GENERATION.md`, [Lienig 6.6].

---

## §1 Construction kinds

The *device* the PDK hands us, before any layout choice. Each row is a distinct model/mask recipe, not a geometry option.

| Kind | Construction | Analog notes | Source |
|---|---|---|---|
| Planar NMOS | N+ S/D in P-sub / P-well (twin-well ≤3.3 V) | baseline; NMOS matching often better than PMOS (process-dependent, PMOS up to 30–50 % worse gm mismatch) | [Hastings ch12 12.2.1, ch13 13.2.2] |
| Planar PMOS | P+ S/D in N-well | keep matched PMOS several µm inside the well (WPE); 2× more stress-sensitive than NMOS | [Hastings ch13, Rule 19] |
| Isolated NMOS | P-well/P-tank inside deep N-well or NBL+sinker; 5–7 terminals | independent backgate bias; DNW ring inflates bbox | [Hastings ch12 12.2.2] |
| Native (natural) | Vt-adjust implant blocked (`NatVt` mask) | near-zero Vt; source-follower / cascode bias uses | [Hastings ch12 12.2.4] |
| Thick-oxide / HV (I/O) | dual gate oxide (staged oxidation or etch-and-regrowth, `Moat-2`) | Vt mismatch ∝ t_ox — thick oxide needs ~10× area for same voltage matching | [Hastings ch12 12.2.5, ch13 Rule 6] |
| Pocket/halo device | tilted pocket implants (deep-submicron core device) | r_o ∝ √L not L; Pelgrom becomes A_VT/√(W·min(L,L_C)), L_C ≈ 1–2 µm — *avoid for current matching* | [Hastings ch12 12.2.7, ch13 Rule 5] |
| Analog-friendly (pocket-blocked) | pocket-block mask, or directional implant + vertical channel | restores r_o ∝ L and Pelgrom; directional variant forbids 90° rotation (see §4) | [Hastings ch12 12.2.7], [PNR_ANALOG S0 §3.A.3] |
| Drain-engineered (SDD/LDD/DDD, drain-extended) | asymmetric drain drift region | asymmetric ⇒ non-self-aligned: matched copies must be superimposable by *translation only* (§4) | [Hastings ch12 12.2.7, ch13 13.2.1] |
| Annular / serpentine | gate ring around drain / folded moat | annular: min C_d/W, no stringers; serpentine: trickle sources only, poor matching | [Hastings ch12 12.2.8] |
| FinFET / GAA | quantized W = N_fins·fin_pitch (or N_sheets); few legal L | ratios only by integer unit count; dummy fins mandatory; amplified LOD; self-heating | [PNR_ANALOG S0 §10] |

Cross-section of the planar stack: **Fig. (g)** in `mosfet.html`.

---

## §2 Formation axis (folding)

### 2.1 Finger count N_f

| Rule | Formula / bound | Source |
|---|---|---|
| Gate-resistance floor | `N_f ≥ ceil(sqrt(W_total·R_sh_poly / (3·L·R_g_target)))`; R_g_target = (1/10)/g_m RF, (1/5)/g_m precision, 1/g_m general | [PNR_ANALOG S0 §3.2] |
| W_f range | `W_f = W_total/N_f ∈ [min_Wf, max_Wf]` (PDK); matching floors: W_f ≥ 150 % min_W minimal, 200 % moderate, 400 % exceptional | [PNR_ANALOG S0 §3.B], [Hastings ch13 Rule 11] |
| Even N_f for matched | required for chi = 0 (§4) and ABBA/cross-quad decomposition | [Hastings ch13, Rule 7] |
| Odd vs even sections | odd ⇒ equal S/D finger counts (symmetric parasitics); even ⇒ choose drain-minimized | [Hastings ch12 12.2.8] |
| Pelgrom area | `W·L·N_units ≥ (A_VT/σ_target)²`; voltage-match by area, current-match by *L* (up to 240 µm exceptional @12 V/10 µA) | [Hastings ch13, Rules 2–3, Tables 13.5/13.6] |
| Pocket cap | effective L capped at L_C in Pelgrom computation | [Hastings ch13] |
| Identical sections | matched devices differ only in section *count*, never section W/L | [Hastings ch13, Rule 1] |
| FinFET | N_f = fin groups; W quantized, round to legal fin count | [PNR_ANALOG S0 §10] |

Shared-diffusion payoff: (N_f+1)/2 drain diffusions instead of N_f — up to 50 % junction-cap saving [Hastings ch12 12.2.8]. **Fig. (b)** shows the 4-finger S D S D S strip.

### 2.2 Multiplier folding

| Option | Meaning | Constraint |
|---|---|---|
| Same-row | m·N_f fingers in one strip (what `mosfet.rs` does today) | strip length ⇒ aspect limit below |
| Multi-row | fold into R rows, `N_f·m ≡ 0 (mod R)` | rows must share OD context per device; matched devices same-OD [Hastings ch13] |
| Separate islands | m distinct OD islands | *never* for matched devices (different LOD/WPE context) |

### 2.3 Rows and aspect ratio

Array aspect ratio limits [Hastings ch13, Rule 9]: voltage matching ≤ 3:1; current matching ≤ 10:1 minimal, ≤ 3:1 moderate, ≈ 1:1 exceptional. Each (N_f, R) pair is a distinct bbox point — the payload for `CellRecord.variant_dims`.

---

## §3 Patterns

Sequences use Hastings subscript notation: the subscript is the terminal facing *left* at that boundary (`D_S A_D B_S B_D A_S D` — dummies D at ends) [Hastings ch13, Table 13.3]. Figures: (a) Single, (b) multi-finger, (c) CC-1D, (d) CC-2D in `mosfet.html`.

| Pattern | Finger sequence | Cancels | Tier grade | Routing cost |
|---|---|---|---|---|
| Single | A A … A | nothing (n=1: trivially exceptional — no partner) | Exceptional (n=1), Minimal (n≥2) | minimal |
| Clustered | AA BB | nothing — centroids offset | never for precision | low |
| Interdig ABAB | A B A B … | partial 1-D linear gradient (centroids offset by ½ pitch) | Moderate | medium |
| CC-1D ABBA | `D_S A_D B_S B_D A_S D`, repeated `(ABBA)ⁿ` | linear gradient, one axis; chi = 0 with even N_f | Exceptional (1-D) | medium-high |
| CC-2D cross-quad | AB / BA (2×2); larger: AABB/BBAA tiles | linear gradient both axes; ~60 % of the residual mismatch of 1-D ABBA | Exceptional (best) | high |
| Width-direction interdig | fingers stacked in W direction, gates shared | needed when sources do **not** share a node (current mirrors — shared gate net) | Moderate–Exceptional | medium (no S/D merge conflicts) |
| Ratioed-unit | 1:N mirror interleaved: `D r N N r N N D` (2:1:2 for 1:4); 8-around-1 3×3 for 8:1 | linear + much of quadratic (2D spans ⅓ the distance) | Exceptional for ratioed | high |

Constraints on every CC pattern — the Five Rules [Hastings ch13, Table 13.2]: coincidence, symmetry, dispersion, compactness, orientation (equal chi). Adjacent fingers may share a diffusion only when same terminal type faces the boundary; otherwise insert a gap or switch to width-direction interdigitation [PNR_ANALOG S0 §4.A.3]. For n ≥ 3 devices the generator's greedy `finger_sequence` stub must be replaced by dispersion/symmetry-scored sequences.

---

## §4 Orientations & transforms

Figures: **Fig. (e)** — F-marker under all 8 transforms with legality badges; **Fig. (f)** — per-finger chi arrows.

### 4.1 The 8 dihedral transforms

D4 group acting on the cell. "Channel dir" = does the current-flow axis stay on the same crystal axis?

| Transform | Channel dir preserved | chi effect (1-D horiz. flow) | (a) Unmatched | (b) Matched | (c) Matched + directional pocket implant |
|---|---|---|---|---|---|
| R0 | yes | chi | legal | legal | **legal** |
| R90 | **no** (swaps π_L/π_T) | chi → chi_V | legal | forbidden (unless whole group rotates together *and* no directional implant) | **FORBIDDEN** |
| R180 | yes | chi → −chi | legal | legal iff chi = 0 or whole group transformed | **legal** |
| R270 | **no** | chi → −chi_V | legal | forbidden (as R90) | **FORBIDDEN** |
| MX (mirror about x-axis) | yes | chi (horiz. flow unchanged) | legal | legal iff chi preserved; forbidden for non-self-aligned devices | **legal** (mirrors OK) |
| MY (mirror about y-axis) | yes | chi → −chi | legal | legal iff chi = 0 or whole group | **legal** (mirrors OK) |
| MX∘R90 | **no** | — | legal | forbidden | **FORBIDDEN** |
| MY∘R90 | **no** | — | legal | forbidden | **FORBIDDEN** |

Rules behind the table:
- Orientation mismatch is the **#1 systematic error** — up to ~15 % gm [PNR_ANALOG S0 §9.A.1], several % Id [Hastings ch13 13.2.1]. All matched channels must run parallel [Hastings ch13, Rule 7].
- **Directional tilted pocket implants** (shot left–right so vertical analog channels escape pockets): only 0°/180° rotations; reflection allowed; **NEVER 90°** — it swaps digital/analog orientations [Hastings ch12 12.2.7], [PNR_ANALOG S0 §9.A.1a]. Generator must query `PDK.pocket_implant_direction` and export the no-90° flag to S2.
- **Non-self-aligned** (asymmetric drain-extended): matched copies superimposable by *translation only* — no rotation, no reflection (mirror images have opposite misalignment sensitivity) [Hastings ch13 13.2.1]. Effectively R0 only.

### 4.2 Per-finger chi metric

`chi_i = +1` if current flows right in finger i, `−1` if left; cell `chi = (1/N_f)·Σ chi_i` [Hastings ch13 13.2.6]. Matched devices need **equal chi (magnitude and sign)**; chi = 0 ideal ⇒ even N_f. 2-D arrays: both chi_H and chi_V must match. S D S D S with alternating flow gives chi = 0 (**Fig. (f)** top); all-same-direction fingers give chi = +1 (**Fig. (f)** bottom). Chi is fixed by the S/D left-facing assignment (§5), so the two axes are coupled.

### 4.3 Gate direction & 45° option

| Sub-axis | Domain | Notes |
|---|---|---|
| Gate direction | vertical (default — poly ∥ Y, current ∥ X, ⟨110⟩) / horizontal | horizontal = R90 of the cell ⇒ forbidden for matched under directional implants; `mosfet.rs` is gate-vertical only |
| 45° PMOS | ⟨100⟩ channel | π_L(PMOS ⟨110⟩) = −65e−11 Pa⁻¹ vs NMOS 30e−11 — 45° minimizes PMOS stress sensitivity; exceptional tier only, iff PDK allows non-Manhattan and no directional implants [Hastings ch13, Rule 24] |

---

## §5 S/D assignment variants

| Variant | Rule | Source |
|---|---|---|
| Drain-inner (default) | interior shared diffusions = drain (high-swing node) — minimizes C_d; only (N_f+1)/2 drain diffusions | [PNR_ANALOG S0 §3.3] |
| Source-inner | swap when source is the high-Z node (source follower) | [PNR_ANALOG S0 §3.3] |
| Left-facing terminal notation | subscript per boundary (`A_D B_S …`) decides diffusion-share legality (same terminal type merges) *and* per-finger chi | [Hastings ch13, Table 13.3], [PNR_ANALOG S0 §4.A.3] |
| Section parity | odd N_f ⇒ symmetric S/D caps; even ⇒ pick drain-minimized | [Hastings ch12 12.2.8] |

`mosfet.rs` today: fixed alternation `S D S D…` starting at S (idx parity) — drain-inner-ish but not a choice.

---

## §6 Gate contact variants

| Variant | Effect | When |
|---|---|---|
| Single-side (+ landing stub) | R_g factor 1/3 per finger | default; `mosfet.rs` today |
| Double-side | R_g ÷ 4 (1/12 vs 1/3) | RF / high-speed [PNR_ANALOG S0 §3.4] |
| Alternating side | equalizes parasitics along long arrays | long finger arrays |
| Metal gate strap vs poly comb | strap avoids poly etch-rate variation; comb only if interconnect poly ≥ 1 µm from moat | **straps required moderate+** [Hastings ch13, Rules 21–22] |

Never contact over the active gate for matched devices [Hastings ch13, Rule 16].

---

## §7 Dummies

Tier table [Hastings ch13, Rule 12], [PNR_ANALOG S0 §5.A.1] — see hatched dummies in **Fig. (c)**:

| Tier | Requirement |
|---|---|
| Minimal | optional (strongly recommended) |
| Moderate, L < 1 µm | full dummy transistor each end |
| Moderate, L > 1 µm | half dummy (single S/D termination, poly ≥ ~1 µm wide) acceptable |
| Exceptional | outermost dummy poly edge ≥ 3 µm from nearest active gate; **moat stretch ≥ 5 µm** beyond last active gate |

Styles: dummy poly only (what `mosfet.rs` draws — outside the diff) / half dummy / full dummy transistor (same OD — mandatory: an island dummy buffers nothing) / moat stretch. Connection styles: tied-off to backgate rail (gnd NMOS / VDD PMOS — default, device off), gate-net (identical boundary, costs C_gs), floating (avoid; current code leaves them floating) — connection must land in cell metadata for S1 parasitics [PNR_ANALOG S0 §5.2]. Also: uniform poly endcap extension ≥ 0.5 µm beyond rules for *all* gates incl. dummies [Rule 21]; block CMP dummy poly within 3 µm (exceptional) / 1 µm (moderate) [Rule 23]; FinFET: 2–4 dummy fins + ≥ 1 dummy poly per edge, always [PNR_ANALOG S0 §10].

---

## §8 Guard rings / well taps / body contact

| Sub-axis | Domain | Notes |
|---|---|---|
| Ring type | none / P+ substrate (NMOS, →VSS) / N+ well (PMOS, →VDD) / deep-N-well / combined | [PNR_ANALOG S0 §6.1] |
| Ring width | 1× PDK / 2× (sensitive; also 2× + half tap spacing on P− substrates) | [PNR_ANALOG S0 §6.A.7] |
| Tap spacing | latchup_max/2 default; ≤ 50 µm analog; ≤ 25 µm sensitive; 25–50 µm max to any backgate contact | [Hastings ch12 12.2.9] |
| Body contact style | butted (abutting source) / distributed plugs in source fingers / interdigitated strips / ring-only | [Hastings ch12 12.2.8–9] |
| Connection | ECGR → highest supply; taps → dedicated SUB net, not circuit GND | [PNR_ANALOG S0 §6.A.5/6.A.8] |
| WPE clearance | well edge ≥ 5 µm from exceptional matched gates (or 2× well depth) | [Hastings ch13, Rule 19] |

Ring inflates bbox by `2·(ring_width + spacing)` — must be reported in `variant_dims`.

---

## §9 Summary enumeration table

| Axis | Domain | Matching grade? | Geometry only? | In `mosfet.rs` today? |
|---|---|---|---|---|
| Construction kind (§1) | planar N/P, native, thick-ox, pocket vs analog-friendly, isolated, annular, FinFET | yes (Pelgrom law shape) | — | device_type N/P only |
| Finger count N_f (§2.1) | feasible divisors, W_f bounds, even-for-matched | yes (chi, Pelgrom) | bbox | no — netlist verbatim |
| Multiplier folding (§2.2) | same-row / multi-row / islands | yes (same-OD) | bbox | same-row only, fixed |
| Row count R (§2.3) | 1…, aspect ≤ 3:1 / ~1:1 | yes (aspect rule) | bbox family | no — 1 row |
| Pattern (§3) | Single, Clustered, ABAB, ABBA, cross-quad, width-interdig, ratioed-unit | yes (primary) | — | 4 of 7, tier-filtered |
| Gate direction (§4.3) | vertical / horizontal / 45° PMOS | yes (stress, implants) | bbox transpose | no — vertical only |
| Cell transform (§4.1) | 8 dihedral, legality per matching class | yes | — | no |
| Per-finger chi (§4.2) | chi_i = ±1, equal chi across matched | yes | — | no (implicit, untracked) |
| S/D assignment (§5) | drain-inner / source-inner / per-boundary | no | parasitics | fixed S-first parity |
| Gate contact (§6) | single / double / alternating; strap vs comb | strap: yes (Rule 22) | R_g, bbox | single stub only |
| Dummy count/style (§7) | 0/half/1/2; poly-only/full/moat-stretch | yes (tier floor) | bbox | fixed 1 poly-only/edge |
| Dummy connection (§7) | tied-off / gate-net / floating | marginal | parasitics | floating, fixed |
| Guard ring (§8) | none/P+/N+/DNW/combined; width; taps | yes (WPE, noise) | bbox inflation | no |
| Body contact (§8) | butted / distributed / ring-only | no (latchup) | area | no |
| Intra-cell strapping | layer, side vs over-active, EM width | yes (hydrogenation — no metal over matched gates, Rule 17) | parasitics | no (routing-level) |

---

## Figure index (`mosfet.html`)

| Fig | Content | Referenced from |
|---|---|---|
| (a) | single-finger NMOS, S/G/D pins, endcap + gate stub | §1, §3 |
| (b) | 4-finger shared-S/D strip, S D S D S | §2.1, §3 |
| (c) | matched pair CC-1D `D A B B A D`, hatched edge dummies | §3, §7 |
| (d) | CC-2D cross-quad AB/BA | §3 |
| (e) | F-marker under all 8 dihedral transforms + legality badges | §4.1 |
| (f) | per-finger chi arrows: chi = 0 vs chi = +1 | §4.2, §5 |
| (g) | one-finger front cross-section (substrate → poly stack) | §1 |
