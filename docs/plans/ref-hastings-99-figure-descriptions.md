# Hastings back matter: figure long descriptions, extracted for Philis (GAP-20)

Source: R. A. Hastings, *The Art of Analog Layout*, 3rd ed. (Pearson, 2023), back matter after Appendix E.
Reftext file: `scratchpad/reftext/hastings.txt` (pdftotext -layout, 73 937 lines). Line range read: **57404–73932**
(the 566 "Long description" blocks; L73933–73937 is blank page tail). Every block in that range was read.
Not studied, as GAP-20 directs: L52119–54715 (Index) and L54716–57403 (navigation contents, List of Figures from
L55433, List of Tables from L56293). Those are captions only.

Task: `98-gap-critic.md` GAP-20. Settles the values the plans left "read from the figure", "not given" or disputed,
and records agreement or disagreement per value with line numbers.

Numbers marked *derived* are arithmetic on cited values (Python in the session scratchpad).

---

## 1. Coverage and method

**Consecutive reads.** The 566 blocks were joined per block (blank lines, `Back` links and page footers dropped) and
read in order, chapter by chapter:

| Chapter | Blocks | Description lines |
|---|---|---|
| 1 | 29 | 57404–58173 |
| 2 | 45 (incl. Table 2.2) | 58179–59432 |
| 3 | 22 | 59438–60035 |
| 4 | 56 | 60041–61747 |
| 5 | 38 | 61753–62906 |
| 6 | 33 | 62912–63732 |
| 7 | 25 | 63738–64335 |
| 8 | 32 figures | 64341–65176 |
| 9 | 42 | 65182–66441 |
| 10 | 32 figures | 66447–67494 |
| 11 | 24 | 67500–68264 |
| 12 | 48 | 68270–69779 |
| 13 | 60 | 69785–71686 |
| 14 | 44 | 71692–73116 |
| 15 | 29 | 73122–73757 |
| App. B | 2 | 73763–73797 |
| App. D tables 8.45, 10.T1 | 2 | 73803–73863 |
| App. E | 3 | 73869–73932 |

**Which figure a block describes.** The blocks carry no figure number. The body text carries one
`Figure N.M Full Alternative Text` (or `Table …`) marker per described item, right after its caption. Both lists have
566 entries (563 figures, 3 tables: 2.2, 8.45, 10.T1), and the k-th block describes the k-th marker. The pairing was
checked on content at Figs 1.1, 1.2, 8.18–8.20, 10.24, 13.41, 15.1, E.3 and both App. D tables; every block read
afterwards fit its figure's caption. §5 lists the pairing for all 566.

**Page numbers.** PDF page = 1 + form feeds before the line's first character, the convention of the other
`ref-hastings-*` studies. pdftotext puts the form feed at the start of a page's first line, so a line that begins with
one is on the next page (every description block's `Long description` line does; checked against `pdftotext -f`).
The printed footer is PDF − 1 throughout, body and back matter alike (PDF p.1568, the last, prints 1567).

**How far to trust a description.** The blocks are the publisher's accessibility text, not the author's. They state
plot points and labels in words, but they contain transcription errors:
- Plot points outside the stated axis range: Fig 1.25 (to 26 V on a 0–24 V axis, L58052–58067), Fig 1.27, Fig 5.13
  (V_DS 22.5 on a 0–20 V axis, L62115–62132), Fig 14.21A ((400 ns, 0.2 A) on a 0–300 ns axis, L72334).
- Wrong increments: Fig 5.14 says "increments of 0.5 units" on a 6.70–6.90 V axis (L62138–62156; the image ticks
  are 0.05 V).
- Wrong device types or values: Fig 3.7 calls the CMOS NAND transistors "PNP/NPN" (L59584–59602); Fig 12.3 labels a
  curve "145 V", i.e. 1.45 V (L68336–68355); Fig 10.20 gives 24 µA next to 50 µA and 75 µA (L67049–67066).
- Jumbled order: the Fig 8.20 x-coordinates run 11.2, 11.21, 11.2, 11.3, 11.2, 11.3 (L64819–64820).
- Empty: Table 2.2's block is only "Back Back" (L58569–58575).

So a description value is secondary evidence. Where it bore on an open value, the PDF page image was opened as well:
PDF pp. 398–399 (Figs 8.19–8.20), 514 (Fig 10.24), 240 (Fig 5.14).

---

## 2. The open values GAP-20 lists

### 2.1 MAT open question 7: eq. 8.29 vs Fig. 8.20 — **resolved: the figure has j and k swapped**

Plan state (plan-02 §5 Q7, MAT-17 Risks; 00-MASTER-PLAN §6.4 item 8): eq. 8.29 as printed,
S = |(N+1)/(N+k) − (M+1)/(M+j)|, gives S = 0.028 at R0 = 10.34 kΩ and 0.0095 at 10.64 kΩ (*derived*), while
Fig. 8.20 and the text put the optima there with S ≈ 0.

Description (Fig 8.20, L64810–64823, desc PDF p.1254): axes R0 10–12 kΩ, S 0–0.08; points (10, 0.05),
(10.05, 0.02), **(10.4, 0.0001)**, (10.45, 0.005), (10.45, 0.062), (10.55, 0.07), (10.6, 0.05), **(10.65, 0.0001)**,
(11.2, 0.03), (11.21, 0.05), (11.2, 0.045), (11.3, 0.018), (11.2, 0.05), (11.3, 0.035), (12, 0.03) (L64818–64820).
The image (PDF p.399, figure; caption "R_M = 200 kΩ, R_N = 146 kΩ") shows the same curve: minima ≈ 0.001 at
≈ 10.34 and ≈ 10.64 kΩ, jumps at 10.43, 10.53, 11.11, 11.23 and 11.76 kΩ. The text (PDF p.398)
names the optima R0 = 10.34 kΩ and 10.64 kΩ, with M = 19, N = 14, j = 0.342, k = 0.120 at 10.34 kΩ (L23855–23858).

The jump positions are exactly the trunc boundaries of eqs. 8.30–8.31 (146/14 = 10.43, 200/19 = 10.53,
200/18 = 11.11, 146/13 = 11.23, 200/17 = 11.76), so the figure uses the book's M, N, j, k. What it plots is eq. 8.29
with **j and k exchanged**, S′ = |(N+1)/(N+j) − (M+1)/(M+k)|. Evaluated at the figure's points (*derived*):

| R0 (kΩ) | M | N | j | k | eq 8.29 printed | swapped S′ | image ≈ | description point (x, S) as printed |
|---|---|---|---|---|---|---|---|---|
| 10.00 | 20 | 14 | 0.000 | 0.600 | 0.0226 | 0.0520 | 0.052 | (10, 0.05) |
| 10.02 | 19 | 14 | 0.960 | 0.571 | 0.0275 | 0.0193 | 0.02 | (10.05, 0.02) |
| 10.34 | 19 | 14 | 0.342 | 0.120 | 0.0283 | 0.0002 | 0.001 | (10.4, 0.0001) |
| 10.42 | 19 | 14 | 0.194 | 0.012 | 0.0285 | 0.0048 | 0.005 | (10.45, 0.005) |
| 10.44 | 19 | 13 | 0.157 | 0.985 | 0.0429 | 0.0633 | 0.063 | (10.45, 0.062) |
| 10.52 | 19 | 13 | 0.011 | 0.878 | 0.0432 | 0.0699 | 0.07 | (10.55, 0.07) |
| 10.54 | 18 | 13 | 0.975 | 0.852 | 0.0094 | 0.0061 | 0.007 | — |
| 10.64 | 18 | 13 | 0.797 | 0.722 | 0.0095 | 0.0001 | 0.001 | (10.65, 0.0001) |
| 11.10 | 18 | 13 | 0.018 | 0.153 | 0.0099 | 0.0288 | 0.03 | (11.2, 0.03) |
| 11.12 | 17 | 13 | 0.986 | 0.129 | 0.0655 | 0.0498 | 0.05 | (11.21, 0.05) |
| 11.22 | 17 | 13 | 0.825 | 0.012 | 0.0661 | 0.0454 | 0.045 | (11.2, 0.045) |
| 11.24 | 17 | 12 | 0.794 | 0.989 | 0.0108 | 0.0155 | 0.016 | (11.3, 0.018) |
| 11.76 | 17 | 12 | 0.007 | 0.415 | 0.0113 | 0.0491 | 0.049 | (11.2, 0.05) |
| 11.77 | 16 | 12 | 0.992 | 0.404 | 0.0476 | 0.0357 | 0.036 | (11.3, 0.035) |
| 12.00 | 16 | 12 | 0.667 | 0.167 | 0.0485 | 0.0252 | 0.026 | (12, 0.03) |

The description's x-values past 11 kΩ are misprinted (it names the 11.76 kΩ peak "11.2"), and its (10.6, 0.05) fits
neither formula nor the image (≈ 0.003 at 10.6 kΩ); the S-values match S′ at the image's breakpoints.

A 1 Ω sweep of S′ over 10–12 kΩ has its minima at 10.343 kΩ (1e-5) and 10.642 kΩ (3e-5), the text's two optima
(*derived*). The printed eq. 8.29 has its minima at ≈10.53 kΩ (0.0094, just above the M = 19→18 step) and ≈11.23 kΩ
(0.0108) (*derived*).

Which one is right: the printed form. The text puts the partial segment jR0 in R_M and kR0 in R_N (L23808–23813;
eqs. 8.32–8.33). The App. D derivation (L50812–50866, PDF p.852) applies the process bias R = αR_i + β to every
segment, the partial one included, so R_M carries (M+1)β over (M+j)R0 and R_N carries (N+1)β over (N+k)R0: eq. 8.29
as printed. S′ pairs R_N's segment count with R_M's fraction and has no physical reading. Fig. 8.20 and the two optima
the text quotes come from a spreadsheet with the two fraction columns exchanged. The decomposition quoted at 10.34 kΩ
(M = 19, N = 14, j = 0.342, k = 0.120) is correct arithmetic but is not an optimum of eq. 8.29.

Consequence for MAT-17: the default "implement eq. 8.29 as printed" stands and needs no outside confirmation. Fig. 8.20
and the 10.34/10.64 kΩ optima must not be used as golden values for eq. 8.29. The decomposition test at 10.34 kΩ does
not depend on S and stays valid.

### 2.2 MAT-17 test values: Fig. 8.19 — **agree**

Description (Fig 8.19, L64790–64804; values L64797–64800): S for N = 1…15 is 1.8, 1.4, 0.5, 2.4, 0.7, 1.2, 2.1,
0.2, 1.6, 1.5, 0.4, 2.2, 1.0, 0.8, 2.3 (×1e-6, "approximately"). Eq. 8.27 with M = round(N·R_M/R_N), R_M = 200 kΩ,
R_N = 146 kΩ gives (*derived*, ×1e-6) 1.849, 1.301, 0.548, 2.397, 0.753, 1.096, 2.055, 0.205, 1.644, 1.507, 0.342,
2.192, 0.959, 0.890, 2.260. Every bar agrees within 0.11e-6. MAT-17's three smallest (N = 8, 11, 3 at 2.05e-7,
3.42e-7, 5.48e-7) are the description's three smallest bars (0.2, 0.4, 0.5 E-6).

### 2.3 MAT-02: Fig. 10.24 2:1:2 arrangement — **agree on topology; plan-02's "corner cells of a 3×3" is imprecise**

Description (Fig 10.24, L67162–67198, figure on PDF p.514): Part (A) places Layout 1 (emitter circle in the lower
middle, base contact near the top, collector bar above) "in the first and second quadrants and also in the middle of
the axes, placed with the center of the circle contact as the origin", and Layout 2 (the vertical mirror: emitter in the
upper middle, collector bar below) in the third and fourth quadrants (L67189–67193). Part (B) places Layout 1 in all
four quadrants, on each half-axis and at the centre: nine identical units, a 3×3 (L67194–67195).

So the four 4X units sit off both axes, one per quadrant, and the lower two are mirrored so that all emitters face the
horizontal axis. That agrees with plan-02 MAT-02 ("two 4X units in each side column, i.e. on the diagonals of the 1X")
and with `ref-hastings-09` H09 ("rotated to bring emitters together").

The description gives no offsets. The image (PDF p.514, re-opened) shows the two units of a side column abutting at the
horizontal axis: side-unit centres are one pitch out in x and half a unit height out in y, not on the 3×3 cell grid of
Part (B). Plan-02's "(the corner cells of a 3×3, r² = 8)" is therefore an approximation of the book's geometry; both
arrangements are exactly common-centroid. Part (B), the 8:1 eight-around-one, is a true 3×3 as the plans say.

### 2.4 H13-53 / H13-55: rule 19 and rule 23 distances — **not stated in any description**

These are body text of §13.3 rules 19 and 23 (L42625–42631, L42648–42654), with the numbers blank in the reftext
(`plan-04` verification log). No figure is attached to rules 12–24: the nearest described figures are 13.59 (marker
L42286) and 13.60 (L42850), and neither carries a distance. The values stay as the H13 study read them from PDF p.716
(EXC ≥ 5–10 µm or 2× well depth, MOD ≥ 3 µm, MIN ≥ 2 µm; rule 23 EXC 5–10 µm, MOD 3–5 µm).

### 2.5 PLC-13: WPE 5 % at 1.8 µm and 25 % at 0.95 µm — **not stated in any description**

The WPE data are body text right after the Fig 13.50 marker (L41301): "a current mismatch of 5% was observed when the
active gate of the nearer device is spaced [blank] from the well edge. This mismatch increased to 25% for a spacing of
[blank]" (L41302–41306). The descriptions of Fig 13.50 (implant scatter off the photoresist edge, L71285–71298) and
Fig 13.51 (half dummies; well edge moved out, L71304–71344) state no distance or percentage. The values stay as PLC-13
read them from PDF p.696.

### 2.6 H12-26 "angles" — **not stated in any description**

The halo statement ("reflected or rotated by 180° but not by 90°") is body text (L36480–36487). Fig 12.24 (pocket
implant, L69034–69052) and Fig 13.58 (tilted S/D implant: arrows "make a very small angle with the vertical", no value,
L71556–71569) give no angle. The only angles in the descriptions are Fig 13.43's device pairs at 0°, 45° and 90°
(orientation mismatch, L71085, L71089) and Fig 12.42's poly shapes rotated 90° (L69544–69578).

### 2.7 Other study values marked "not given", "not legible" or "read from the figure"

| Study value | Where | In a description? | Verdict |
|---|---|---|---|
| Required NBL/iso breakdown for an NBL tunnel | `ref-hastings-14` L214 (text L44312–44315) | Fig 14.20 (L72298–72320): layout only | not stated |
| Table 8.7 gauge factors G_T (P-type), G_L (N-type), π values | `ref-hastings-15` L212, L686 | Table 8.7 has no long description (only Tables 2.2, 8.45, 10.T1 do) | not stated |
| Eq 5.32, Eq 6.21 numerator, Eq 14.5 | `ref-hastings-15` L208, `-06` L155, `-14` L960 | equations, not figures | out of scope; not stated |
| Table 13.3 patterns "read from PDF p.710 figure" | `ref-hastings-13` L282 | Fig 13.59 (L71575–71596): left column A over B, right column B over A, gaps D, S, D | agrees for the cross-coupled 2-D pair; the 1-D strings are not in a description |
| Thresholds the text calls "not given" (fuse keep-out radius, HCI ΔV_DS, latch-up distance, W_nce_min, drain ballast RW, die-edge distance, EM chamfer threshold, ESD ΔT) | `ref-hastings-05` L491, L509, L527, L644; `-12` L240; `-14` L913; `-15` L340, L583, L662, L725 | no figure description states any of them | not stated |
| Built-in potential | `ref-hastings-01` L54 | Fig 1.12 labels V1, V2, V3 only (L57685–57697) | not stated |

### 2.8 Study values a description does state — cross-check

| Figure | Description (lines) | Study / plan value | Verdict |
|---|---|---|---|
| 5.5 HBM / MM | 150 pF, 1.5 kΩ, 2 kV; MM 200 pF, 2 kV (L61905–61918) | `ref-hastings-05` L106, L425; `plan-06` L567 (cites L61905) | agree |
| 5.6 CDM tester | > 100 MΩ charging resistor, 1 Ω disk resistor (L61931) | `ref-hastings-05` L106 (> 100 MΩ) | agree |
| 5.13 HCI duty contours | V_DS 0–20 V, V_GS 0–6 V; regions 0.1 %, 1 %, 10 %, 100 % (L62115–62132) | `ref-hastings-05` L141 | agree (description's parabola x-values exceed the axis) |
| 5.14A Zener walkout | "starts at the origin, rises vertically until (0, 6.84)", ends near 6.90 (L62145) | `ref-hastings-05` L144: 6.80 → ≈ 6.89 V | agree: the image's t-axis crosses V_R at 6.80, so "origin" is 6.80 V (PDF p.240) |
| 5.14B BiCMOS walk-back | peak ≈ 7.29, end ≈ 7.13 on a 7.10–7.30 axis (L62152) | text L13844–13845 (values on PDF p.240): 200–300 mV out, 300–500 mV back | different quantities: the figure is one example (image: 7.20 V start, ≈ +90 mV to the peak, then ≈ −170 mV back to ≈ 7.12 V) |
| 8.1 mismatch histogram | counts 1, 3, 3, 3, 5, 8, 4, 1, 2 (L64341–64354) | `ref-hastings-08` §8.1: ≥ 30 units | agree (30 units) |
| 8.2 capacitor areas | C1 area 1, C2 area 1.3 (L64360–64375) | — | recorded |
| 8.5 capacitor array | 20 squares, 5 rows × 4 columns, dummy ring (L64434–64437) | `ref-hastings-08` L83: 6 active, 14 dummies | agree (3×2 interior + 14 border = 20) |
| 8.8 jumpered array | 6 resistors + 2 end dummies; R1 on bars 2, 4, 5, 7, R2 on 3, 6 (L64501–64520) | — | recorded (active order A B A A B A) |
| 8.12 folded jumpers | 4 resistors + 2 dummies; R1 on bars 2, 5, R2 on 3, 4 (L64609–64633) | Table 8.5 1:1 ABBA | agree |
| 8.18 dogbone / Si-block segments | 10 bars, dummies 4th and 7th; 8 bars, dummies 4th and 6th (L64763–64784) | `ref-hastings-08` L125 (embedded dummies) | agree |
| 9.22 split collectors | halves; quarters; five parts C, E, D over A, B (L65740–65759) | `ref-hastings-09` L169: ½-½, ¼×4, 1/6-1/6-1/6-¼-¼ | agree |
| 13.41 20:1 mirror | M1A–M1D in series, M2A–M2E in parallel (L71012–71018) | `ref-hastings-13` L185, L499; `plan-01` `series_parallel_20_to_1` | agree |
| 13.57 pattern | gates A, B, B, A between dummies; drain in the middle, sources, "Drain A" (L71528–71550) | `ref-hastings-13` L279: ᴅAₛBᴅBₛAᴅ | agree |
| 14.21 HBM / CDM pulses | HBM: (0, 1 A) → peak (20 ns, 1.3 A) → (400 ns, 0.2 A); CDM: peak (0.25 ns, 5 A), (0.6 ns, −2 A), (0.8 ns, 0), (1 ns, −0.2 A) (L72326–72343) | `ref-hastings-14` L218: HBM ≈ 1.3 A after ≈ 15 ns, τ ≈ 230 ns; CDM > 5 A at ≈ 0.25 ns | agree; the description's peak is at 20 ns against the text's ≈ 15 ns (1.3·e^(−380/230) ≈ 0.25 A at 400 ns, *derived*) |
| 15.28A shield | metal-2 shield between noisy metal-3 and sensitive metal-1 (L73721–73736) | `audit-05` L368 | agree |
| Table 8.45 | ⟨110⟩ on (100): E 169 GPa, ν 0.064; ⟨100⟩ on (100): 130, 0.279; any on (111): 169, 0.262; oxide: 75, 0.17 (L73803–73832) | `ref-hastings-15` L695 | agree |
| Table 10.T1 | ζ11 PNP 8.9, NPN −28.4; ζ12 14.3, 43.4; ζ44 103.5, 13.1 (×1e-11 Pa⁻¹, L73838–73863) | `ref-hastings-15` L217, L704 | agree |

---

## 3. Corrections fed to the owning plans

- **plan-02 (MAT)**: open question 7 resolved (§2.1); MAT-17 Risks reworded; MAT-02's [1,4] note gets the Fig 10.24A
  geometry (§2.3).
- **plan-04 (PLC)**: verification-log note — rule 19/23 distances, the WPE points and the H12-26 statement are body
  text; no figure description states them (§2.4–2.6). No value changes.
- **00-MASTER-PLAN §6.4 item 8**: resolved; §7 index lists this document.
- **plan-03 (CELL)**: no change. The CELL-relevant values checked (Fig 8.5 dummy ring, rule 19) agree or are not stated.

---

## 4. Numeric values stated in the descriptions

Every description with a data value (plot points, component values, sizes, ratios, counts that define a pattern).
Pure labels (M1, Q2, Metal-1, Step 3) are omitted. Line ranges are the description blocks.

| Figure | Values stated |
|---|---|
| 1.20 (L57863–57880) | BJT I_C–V_CE: V_CE 0–40 V, I_C 0–5 mA; I_B = 10, 20, 30, 40 µA; points (0, 0.5), (1, 0.6), (38, 0.8), (39, 5.5); (0, 1.5), (1, 1.6), (35, 1.8), (38, 5.5); (1, 2.5), (34, 2.8), (37, 5.5); (1, 3.4), (33, 4), (36, 5.5) |
| 1.21 (L57886–57922) | MOS capacitor gate at 0 V, −3 V (accumulation), 1.5 V (depletion), 3 V (inversion); back gate 0 V |
| 1.25 (L58052–58067) | MOSFET I_D–V_DS: V_DS 0–24 V, I_D 0–2 mA; V_GS 0.5, 1.0, 1.5, 2.0 V; points run to (26, 3), outside the axis |
| 1.27 (L58101–58117) | JFET: V_DS 0–30 V, I_D 0–2 mA; V_GS −8, −6, −4, −2, 0 V; saturation 0, 0.2, 0.35, 0.65, 0.8–0.9 mA |
| 2.12 (L58439–58458) | 1.0 µm depth mark |
| 2.25 (L58767–58780) | V-groove on (100) Si at 35.26° |
| 2.42 (L59356–59367) | ≈ 300 dice per wafer |
| 3.1 (L59438–59449) | rectangle vertices (0, 0), (0, 3), (5, 3), (5, 0) |
| 3.6 (L59554–59578) | orientations R0, R90, R180, R270, MX, MY, MX90, MY90 |
| 3.7 (L59584–59602) | NAND W/L 1/0.5 for all four devices |
| 3.14 (L59816–59849) | notch 1.6 units; plate widths 3.6 and 1.6 units |
| 4.56 (L61724–61747) | bipolar gate R1 = 25 kΩ, W = 8 µm; CMOS NOR M1, M2 7/4, M3, M4 4/4 |
| 5.5 (L61899–61919) | HBM 150 pF, 1.5 kΩ, 2 kV; MM 200 pF, 2 kV |
| 5.6 (L61925–61941) | CDM: > 100 MΩ, 1 Ω disk resistor |
| 5.10 (L62021–62044) | mobile-ion bias 0 V, 10 V |
| 5.13 (L62115–62132) | V_DS 0–20 V, V_GS 0–6 V; 0.1 %, 1 %, 10 %, 100 % duty regions; vertices ≈ (19.5, 3.5), (17.5, 3.5), (15, 3.5) |
| 5.14 (L62138–62156) | (A) 6.70–6.90 V, fast rise to 6.84, end ≈ 6.90; (B) 7.10–7.30 V, peak ≈ 7.29, end ≈ 7.13 |
| 7.7 (L63850–63865) | junction C peaks at ≈ 0.7 V forward; avalanche at ≈ −7 V |
| 8.1 (L64341–64354) | histogram 1, 3, 3, 3, 5, 8, 4, 1, 2 units, axis 0–8 |
| 8.2 (L64360–64375) | C1 area 1, C2 area 1.3 |
| 8.5 (L64428–64441) | 20 unit capacitors, 5 × 4, dummy ring |
| 8.8 (L64501–64520) | 6 resistors + 2 dummies; R1 bars 2, 4, 5, 7; R2 bars 3, 6 |
| 8.12 (L64609–64633) | 4 resistors + 2 dummies; R1 bars 2, 5; R2 bars 3, 4 |
| 8.18 (L64763–64784) | 10 dogbones, dummies 4 and 7; 8 bars, dummies 4 and 6 |
| 8.19 (L64790–64804) | §2.2 |
| 8.20 (L64810–64823) | §2.1 |
| 9.2 (L65208–65223) | β_F vs I_C, 10 nA–100 mA, β 25–125: lateral PNP (20 nA, 50) → (100 µA, 80) → (4 mA, 30); NPN before avalanche (30 nA, 127) → (8 µA, 130) → (10 mA, 120) → (300 mA, 40); after avalanche from (20 nA, 40) |
| 9.3 (L65229–65248) | I_B = 10, 20, 30, 40 µA; Early voltage −V_A |
| 9.4 (L65254–65269) | BV_CEO, BV_CER, BV_CES curves: V_CE 10–60 V, I_C 25–75 mA; knees at ≈ 40, 50, 59 V |
| 9.27 (L65884–65911) | voltage recognition outlines V20, V40, V0 |
| 10.1 (L66447–66473) | emitter debiasing: 100 mA per finger, drops 1, 2, 3 mV across R1–R3 |
| 10.4 (L66519–66536) | NPN SOA: current limit 4 A to 7 V; power limit to (45 V, 700 mA); secondary breakdown to (60 V, 90 mA); voltage limit 60 V; 10 ms and 1 ms lines |
| 10.5 (L66542–66557) | RBSOA: 2 A to 38 V; V_BE = 0 V ends 44 V; −2 V from (39 V, 1.4 A) to 50 V; −4 V from (39 V, 0.8 A) to 50 V |
| 10.20 (L67049–67066) | degenerated lateral mirror: 1X, 2X, 2X, 1X; R 4 kΩ, 2 kΩ, 1.33 kΩ; 24 (sic), 50, 75 µA |
| 10.22 (L67093–67116) | ratioed quad 4X, 4X, 1X, 1X |
| 10.23 (L67122–67156) | cross-coupled quad of ½Q1, ½Q2 |
| 10.24 (L67162–67198) | §2.3 |
| 10.26 (L67237–67297) | ½Q2, Q1, ½Q2 arrays with 2 and 4 emitters per ½Q2 |
| 10.31 (L67423–67466) | bandgap Q1 8X, Q2 1X, R1 2.7 kΩ, R2 16.25 kΩ; op-amp PNP 25, 50, 25, 50, 50, NPN 128 × 3, C1 10 pF |
| 10.32 (L67472–67494) | emitter multiples 64, 128, 256, 64, 36, 64, 64, 36, 128, 64, 36 |
| 12.3 (L68336–68355) | V_GS 1, 1.2, 1.45 (printed "145"), 1.5 V; −1/λ intercept |
| 12.5 (L68402–68416) | subthreshold: V_GS 0–2.5 V; floor 2 fA to 0.3 V; V_t at (0.8 V, 1 nA); 0.1 µA at 2 V |
| 12.48 (L69740–69779) | W/L 5/2, 3/3, 4/2, 10/2 |
| 13.41 (L71003–71027) | 4 units in series, 5 in parallel |
| 13.43 (L71069–71092) | pairs at 0°, 45°, 90° |
| 13.60 (L71602–71686) | op-amp sizes: MP1, MP2 8/12; MP3 4(8/12); MP9, MP10 4(10/15); MP6 4/20; MP7, MP8 2(5/3); MP4, MP5 12(15/5); MN1 5/4; MN2, MN3 2(5/4); MN4, MN5 10/25; MN6, MN7 2(10/25); match groups MP1-MP2-MP3, MP4-MP5, MP6-(MP7-MP8) C/C, MP9-MP10, MN1-(MN2-MN3) C/C, MN4-MN5-(MN6-MN7 C/C) |
| 13.13 (L70111–70126) | I_D0 and 1.2·I_D0 at BV_D |
| 13.16–13.21 (L70180–70367) | contact/via counts per finger (25 per bar; 20 per row, 9/11 split; 12 in sets of 3) |
| 14.3 (L71747–71765) | ring contacts 9/6 and 15/13 per side |
| 14.9 (L71896–71958) | collector split 0.75 / 0.25 |
| 14.21 (L72326–72343) | §2.8 |
| 14.32 (L72692–72744) | ring 19 × 16 contacts; ½M2, M1, ½M2 |
| 14.43 (L73067–73083) | Q2 640 µm², Q3 64 µm², D1 250 µm² |
| 15.2 (L73138–73154) | R8 200 kΩ 6 µm HSR; R1 8 µm HSR; R2 12 kΩ 8 µm HSR; C1 40 pF; R7 50 kΩ 6 µm HSR; Q8 64 µm²; Q4 1X |
| 15.5 (L73195–73210) | AMP1 0.32 mm², BIAS 0.13 mm², AMP2 0.32 mm²; pins 1–8 |
| 15.7 (L73241–73258) | 30 mA branches, 60 mA to the 2-mil scribe street |
| 15.26 (L73676–73693) | 1 mA through R_m or R_m/2 |
| 15.29 (L73742–73757) | via arrays 3 × 3 numbered 1–9 at a 90° bend and on a straight run |
| B.2 (L73780–73797) | axes 0–3; triangle vertices given with duplicates ((1,0,0), (3,0,0), (3,0,0)) |
| Table 8.45, Table 10.T1 | §2.8 |

---

## 5. Figure index

Columns: body marker line and PDF page of the figure (the `Full Alternative Text` marker follows the caption; for 29
items, e.g. Figs 1.22 and 15.18, the marker starts the next page, one past the figure); the description block's line
range and PDF page. `D` = the block states data values (§4).

| Item | Body line | PDF | Description lines | Desc PDF | D |
|---|---|---|---|---|---|
| Figure 1.1 | 1569 | 27 | 57404–57425 | 977 |  |
| Figure 1.2 | 1624 | 28 | 57431–57442 | 978 |  |
| Figure 1.3 | 1750 | 30 | 57448–57461 | 979 |  |
| Figure 1.4 | 1822 | 31 | 57467–57491 | 980 |  |
| Figure 1.5 | 1894 | 32 | 57497–57510 | 981 |  |
| Figure 1.6 | 1953 | 33 | 57516–57528 | 982 |  |
| Figure 1.7 | 2057 | 35 | 57534–57552 | 983 |  |
| Figure 1.8 | 2163 | 37 | 57558–57607 | 984 |  |
| Figure 1.9 | 2211 | 38 | 57613–57627 | 986 |  |
| Figure 1.10 | 2253 | 39 | 57633–57658 | 987 |  |
| Figure 1.11 | 2304 | 40 | 57664–57679 | 988 |  |
| Figure 1.12 | 2337 | 40 | 57685–57697 | 989 |  |
| Figure 1.13 | 2438 | 42 | 57703–57719 | 990 |  |
| Figure 1.14 | 2460 | 42 | 57725–57738 | 991 |  |
| Figure 1.15 | 2558 | 44 | 57744–57758 | 992 |  |
| Figure 1.16 | 2645 | 45 | 57764–57783 | 993 |  |
| Figure 1.17 | 2731 | 47 | 57789–57808 | 994 |  |
| Figure 1.18 | 2811 | 48 | 57814–57831 | 995 |  |
| Figure 1.19 | 2848 | 49 | 57837–57857 | 996 |  |
| Figure 1.20 | 2937 | 51 | 57863–57880 | 997 | D |
| Figure 1.21 | 3028 | 52 | 57886–57922 | 998 | D |
| Figure 1.22 | 3104 | 53 | 57928–57949 | 999 |  |
| Figure 1.23 | 3128 | 54 | 57955–57992 | 1000 |  |
| Figure 1.24 | 3311 | 57 | 57998–58046 | 1001 |  |
| Figure 1.25 | 3390 | 58 | 58052–58067 | 1002 | D |
| Figure 1.26 | 3460 | 60 | 58073–58095 | 1003 |  |
| Figure 1.27 | 3528 | 61 | 58101–58117 | 1004 | D |
| Figure 1.28 | 3555 | 62 | 58123–58154 | 1005 |  |
| Figure 1.29 | 3714 | 66 | 58160–58173 | 1006 |  |
| Figure 2.1 | 3802 | 68 | 58179–58193 | 1007 |  |
| Figure 2.2 | 3867 | 70 | 58199–58216 | 1008 |  |
| Figure 2.3 | 3887 | 70 | 58222–58234 | 1009 |  |
| Figure 2.4 | 3922 | 71 | 58240–58252 | 1010 |  |
| Figure 2.5 | 3960 | 72 | 58258–58271 | 1011 |  |
| Figure 2.6 | 4002 | 73 | 58277–58304 | 1012 |  |
| Figure 2.7 | 4098 | 75 | 58310–58325 | 1013 |  |
| Figure 2.8 | 4230 | 77 | 58331–58369 | 1014 |  |
| Figure 2.9 | 4264 | 78 | 58375–58392 | 1015 |  |
| Figure 2.10 | 4290 | 78 | 58398–58411 | 1016 |  |
| Figure 2.11 | 4341 | 79 | 58417–58433 | 1017 |  |
| Figure 2.12 | 4366 | 79 | 58439–58458 | 1018 | D |
| Figure 2.13 | 4397 | 80 | 58464–58475 | 1019 |  |
| Figure 2.14 | 4436 | 81 | 58481–58510 | 1020 |  |
| Figure 2.15 | 4474 | 81 | 58516–58534 | 1021 |  |
| Figure 2.16 | 4528 | 83 | 58540–58563 | 1022 |  |
| Table 2.2 (empty) | 4599 | 84 | 58569–58575 | 1023 |  |
| Figure 2.17 | 4631 | 85 | 58576–58590 | 1024 |  |
| Figure 2.18 | 4690 | 86 | 58596–58617 | 1025 |  |
| Figure 2.19 | 4728 | 86 | 58623–58648 | 1026 |  |
| Figure 2.20 | 4780 | 87 | 58654–58668 | 1027 |  |
| Figure 2.21 | 4898 | 89 | 58674–58687 | 1028 |  |
| Figure 2.22 | 4925 | 89 | 58693–58708 | 1029 |  |
| Figure 2.23 | 4991 | 90 | 58714–58726 | 1030 |  |
| Figure 2.24 | 5047 | 91 | 58732–58761 | 1031 |  |
| Figure 2.25 | 5156 | 93 | 58767–58780 | 1032 | D |
| Figure 2.26 | 5201 | 94 | 58786–58811 | 1033 |  |
| Figure 2.27 | 5240 | 95 | 58817–58837 | 1034 |  |
| Figure 2.28 | 5308 | 96 | 58843–58877 | 1035 |  |
| Figure 2.29 | 5345 | 97 | 58883–58913 | 1036 |  |
| Figure 2.30 | 5401 | 98 | 58919–58960 | 1037 |  |
| Figure 2.31 | 5443 | 99 | 58966–58999 | 1039 |  |
| Figure 2.32 | 5485 | 100 | 59005–59018 | 1040 |  |
| Figure 2.33 | 5565 | 101 | 59024–59047 | 1041 |  |
| Figure 2.34 | 5587 | 102 | 59053–59066 | 1042 |  |
| Figure 2.35 | 5632 | 103 | 59072–59108 | 1043 |  |
| Figure 2.36 | 5719 | 104 | 59114–59146 | 1044 |  |
| Figure 2.37 | 5762 | 105 | 59152–59173 | 1045 |  |
| Figure 2.38 | 5839 | 106 | 59179–59200 | 1046 |  |
| Figure 2.39 | 5903 | 108 | 59206–59266 | 1047 |  |
| Figure 2.40 | 5934 | 109 | 59272–59330 | 1049 |  |
| Figure 2.41 | 5972 | 110 | 59336–59350 | 1051 |  |
| Figure 2.42 | 6025 | 111 | 59356–59367 | 1052 | D |
| Figure 2.43 | 6083 | 112 | 59373–59384 | 1053 |  |
| Figure 2.44 | 6158 | 113 | 59390–59432 | 1054 |  |
| Figure 3.1 | 6609 | 122 | 59438–59449 | 1055 | D |
| Figure 3.2 | 6634 | 122 | 59455–59468 | 1056 |  |
| Figure 3.3 | 6661 | 123 | 59474–59489 | 1057 |  |
| Figure 3.4 | 6711 | 124 | 59495–59516 | 1058 |  |
| Figure 3.5 | 6758 | 124 | 59522–59548 | 1059 |  |
| Figure 3.6 | 6820 | 125 | 59554–59578 | 1060 | D |
| Figure 3.7 | 6858 | 126 | 59584–59602 | 1061 | D |
| Figure 3.8 | 6936 | 127 | 59608–59679 | 1062 |  |
| Figure 3.9 | 7188 | 131 | 59685–59705 | 1064 |  |
| Figure 3.10 | 7220 | 131 | 59711–59731 | 1065 |  |
| Figure 3.11 | 7256 | 132 | 59737–59756 | 1066 |  |
| Figure 3.12 | 7287 | 132 | 59762–59784 | 1067 |  |
| Figure 3.13 | 7317 | 133 | 59790–59810 | 1068 |  |
| Figure 3.14 | 7356 | 134 | 59816–59849 | 1069 | D |
| Figure 3.15 | 7413 | 135 | 59855–59871 | 1070 |  |
| Figure 3.16 | 7495 | 136 | 59877–59898 | 1071 |  |
| Figure 3.17 | 7547 | 137 | 59904–59921 | 1072 |  |
| Figure 3.18 | 7583 | 137 | 59927–59946 | 1073 |  |
| Figure 3.19 | 7646 | 138 | 59952–59967 | 1074 |  |
| Figure 3.20 | 8163 | 146 | 59973–59997 | 1075 |  |
| Figure 3.21 | 8261 | 148 | 60003–60016 | 1076 |  |
| Figure 3.22 | 8399 | 150 | 60022–60035 | 1077 |  |
| Figure 4.1 | 8642 | 155 | 60041–60054 | 1078 |  |
| Figure 4.2 | 8689 | 156 | 60060–60071 | 1079 |  |
| Figure 4.3 | 8722 | 157 | 60077–60090 | 1080 |  |
| Figure 4.4 | 8745 | 157 | 60096–60111 | 1081 |  |
| Figure 4.5 | 8786 | 158 | 60117–60134 | 1082 |  |
| Figure 4.6 | 8814 | 158 | 60140–60158 | 1083 |  |
| Figure 4.7 | 8849 | 159 | 60164–60183 | 1084 |  |
| Figure 4.8 | 8895 | 160 | 60189–60210 | 1085 |  |
| Figure 4.9 | 8944 | 160 | 60216–60248 | 1086 |  |
| Figure 4.10 | 9055 | 162 | 60254–60277 | 1087 |  |
| Figure 4.11 | 9157 | 164 | 60283–60312 | 1088 |  |
| Figure 4.12 | 9222 | 165 | 60318–60345 | 1090 |  |
| Figure 4.13 | 9244 | 166 | 60351–60382 | 1091 |  |
| Figure 4.14 | 9274 | 166 | 60388–60419 | 1092 |  |
| Figure 4.15 | 9332 | 167 | 60425–60452 | 1093 |  |
| Figure 4.16 | 9367 | 168 | 60458–60473 | 1094 |  |
| Figure 4.17 | 9432 | 169 | 60479–60508 | 1095 |  |
| Figure 4.18 | 9488 | 170 | 60514–60543 | 1096 |  |
| Figure 4.19 | 9608 | 172 | 60549–60561 | 1097 |  |
| Figure 4.20 | 9654 | 173 | 60567–60582 | 1098 |  |
| Figure 4.21 | 9687 | 174 | 60588–60619 | 1099 |  |
| Figure 4.22 | 9714 | 174 | 60625–60642 | 1100 |  |
| Figure 4.23 | 9750 | 175 | 60648–60664 | 1101 |  |
| Figure 4.24 | 9795 | 175 | 60670–60687 | 1102 |  |
| Figure 4.25 | 9841 | 176 | 60693–60715 | 1103 |  |
| Figure 4.26 | 9908 | 177 | 60721–60738 | 1104 |  |
| Figure 4.27 | 9945 | 178 | 60744–60772 | 1105 |  |
| Figure 4.28 | 10109 | 180 | 60778–60808 | 1106 |  |
| Figure 4.29 | 10149 | 181 | 60814–60838 | 1107 |  |
| Figure 4.30 | 10188 | 181 | 60844–60863 | 1108 |  |
| Figure 4.31 | 10222 | 182 | 60869–60900 | 1109 |  |
| Figure 4.32 | 10287 | 183 | 60906–60936 | 1110 |  |
| Figure 4.33 | 10349 | 184 | 60942–60986 | 1111 |  |
| Figure 4.34 | 10396 | 185 | 60992–61020 | 1112 |  |
| Figure 4.35 | 10475 | 187 | 61026–61043 | 1113 |  |
| Figure 4.36 | 10518 | 187 | 61049–61068 | 1114 |  |
| Figure 4.37 | 10584 | 188 | 61074–61087 | 1115 |  |
| Figure 4.38 | 10674 | 190 | 61093–61110 | 1116 |  |
| Figure 4.39 | 10696 | 190 | 61116–61134 | 1117 |  |
| Figure 4.40 | 10734 | 191 | 61140–61161 | 1118 |  |
| Figure 4.41 | 10796 | 192 | 61167–61193 | 1119 |  |
| Figure 4.42 | 10832 | 192 | 61199–61236 | 1120 |  |
| Figure 4.43 | 10915 | 194 | 61242–61279 | 1121 |  |
| Figure 4.44 | 10956 | 194 | 61285–61315 | 1122 |  |
| Figure 4.45 | 11048 | 196 | 61321–61354 | 1123 |  |
| Figure 4.46 | 11102 | 197 | 61360–61401 | 1124 |  |
| Figure 4.47 | 11187 | 198 | 61407–61436 | 1125 |  |
| Figure 4.48 | 11272 | 200 | 61442–61470 | 1126 |  |
| Figure 4.49 | 11366 | 201 | 61476–61499 | 1127 |  |
| Figure 4.50 | 11434 | 202 | 61505–61529 | 1128 |  |
| Figure 4.51 | 11473 | 203 | 61535–61565 | 1129 |  |
| Figure 4.52 | 11554 | 204 | 61571–61632 | 1130 |  |
| Figure 4.53 | 11610 | 205 | 61638–61652 | 1132 |  |
| Figure 4.54 | 11631 | 206 | 61658–61674 | 1133 |  |
| Figure 4.55 | 11722 | 207 | 61680–61718 | 1134 |  |
| Figure 4.56 | 11812 | 210 | 61724–61747 | 1135 | D |
| Figure 5.1 | 12244 | 216 | 61753–61809 | 1136 |  |
| Figure 5.2 | 12390 | 218 | 61815–61828 | 1138 |  |
| Figure 5.3 | 12475 | 219 | 61834–61858 | 1139 |  |
| Figure 5.4 | 12626 | 221 | 61864–61893 | 1140 |  |
| Figure 5.5 | 12938 | 225 | 61899–61919 | 1141 | D |
| Figure 5.6 | 12984 | 226 | 61925–61941 | 1142 | D |
| Figure 5.7 | 13064 | 227 | 61947–61960 | 1143 |  |
| Figure 5.8 | 13214 | 229 | 61966–61994 | 1144 |  |
| Figure 5.9 | 13262 | 230 | 62000–62015 | 1145 |  |
| Figure 5.10 | 13399 | 232 | 62021–62044 | 1146 | D |
| Figure 5.11 | 13526 | 234 | 62050–62080 | 1147 |  |
| Figure 5.12 | 13638 | 237 | 62086–62109 | 1148 |  |
| Figure 5.13 | 13757 | 239 | 62115–62132 | 1149 | D |
| Figure 5.14 | 13835 | 240 | 62138–62156 | 1150 | D |
| Figure 5.15 | 13894 | 241 | 62162–62192 | 1151 |  |
| Figure 5.16 | 13979 | 242 | 62198–62213 | 1152 |  |
| Figure 5.17 | 14281 | 246 | 62219–62278 | 1153 |  |
| Figure 5.18 | 14357 | 247 | 62284–62316 | 1155 |  |
| Figure 5.19 | 14393 | 248 | 62322–62343 | 1156 |  |
| Figure 5.20 | 14454 | 249 | 62349–62371 | 1157 |  |
| Figure 5.21 | 14479 | 249 | 62377–62408 | 1158 |  |
| Figure 5.22 | 14545 | 251 | 62414–62456 | 1159 |  |
| Figure 5.23 | 14565 | 251 | 62462–62480 | 1161 |  |
| Figure 5.24 | 14595 | 252 | 62486–62506 | 1162 |  |
| Figure 5.25 | 14661 | 253 | 62512–62547 | 1163 |  |
| Figure 5.26 | 14770 | 255 | 62553–62578 | 1164 |  |
| Figure 5.27 | 14820 | 256 | 62584–62621 | 1165 |  |
| Figure 5.28 | 14881 | 257 | 62627–62643 | 1166 |  |
| Figure 5.29 | 14927 | 258 | 62649–62682 | 1167 |  |
| Figure 5.30 | 15090 | 260 | 62688–62706 | 1168 |  |
| Figure 5.31 | 15197 | 262 | 62712–62733 | 1169 |  |
| Figure 5.32 | 15487 | 265 | 62739–62752 | 1170 |  |
| Figure 5.33 | 15540 | 266 | 62758–62785 | 1171 |  |
| Figure 5.34 | 15591 | 267 | 62791–62812 | 1172 |  |
| Figure 5.35 | 15608 | 267 | 62818–62836 | 1173 |  |
| Figure 5.36 | 15640 | 268 | 62842–62860 | 1174 |  |
| Figure 5.37 | 15678 | 269 | 62866–62881 | 1175 |  |
| Figure 5.38 | 15725 | 270 | 62887–62906 | 1176 |  |
| Figure 6.1 | 16221 | 278 | 62912–62922 | 1177 |  |
| Figure 6.2 | 16284 | 279 | 62928–62939 | 1178 |  |
| Figure 6.3 | 16381 | 280 | 62945–62963 | 1179 |  |
| Figure 6.4 | 16425 | 281 | 62969–62981 | 1180 |  |
| Figure 6.5 | 16461 | 282 | 62987–63000 | 1181 |  |
| Figure 6.6 | 16505 | 283 | 63006–63024 | 1182 |  |
| Figure 6.7 | 16780 | 287 | 63030–63056 | 1183 |  |
| Figure 6.8 | 16896 | 289 | 63062–63079 | 1184 |  |
| Figure 6.9 | 17096 | 292 | 63085–63099 | 1185 |  |
| Figure 6.10 | 17109 | 292 | 63105–63128 | 1186 |  |
| Figure 6.11 | 17143 | 293 | 63134–63151 | 1187 |  |
| Figure 6.12 | 17165 | 293 | 63157–63183 | 1188 |  |
| Figure 6.13 | 17198 | 294 | 63189–63215 | 1189 |  |
| Figure 6.14 | 17369 | 297 | 63221–63243 | 1190 |  |
| Figure 6.15 | 17469 | 299 | 63249–63263 | 1191 |  |
| Figure 6.16 | 17501 | 299 | 63269–63283 | 1192 |  |
| Figure 6.17 | 17561 | 300 | 63289–63302 | 1193 |  |
| Figure 6.18 | 17635 | 301 | 63308–63327 | 1194 |  |
| Figure 6.19 | 17759 | 303 | 63333–63346 | 1195 |  |
| Figure 6.20 | 17821 | 304 | 63352–63371 | 1196 |  |
| Figure 6.21 | 17945 | 306 | 63377–63397 | 1197 |  |
| Figure 6.22 | 18056 | 307 | 63403–63421 | 1198 |  |
| Figure 6.23 | 18256 | 310 | 63427–63445 | 1199 |  |
| Figure 6.24 | 18299 | 311 | 63451–63465 | 1200 |  |
| Figure 6.25 | 18337 | 312 | 63471–63483 | 1201 |  |
| Figure 6.26 | 18479 | 314 | 63489–63511 | 1202 |  |
| Figure 6.27 | 18492 | 314 | 63517–63544 | 1203 |  |
| Figure 6.28 | 18605 | 316 | 63550–63565 | 1204 |  |
| Figure 6.29 | 18642 | 317 | 63571–63607 | 1205 |  |
| Figure 6.30 | 18704 | 318 | 63613–63639 | 1206 |  |
| Figure 6.31 | 18742 | 318 | 63645–63657 | 1207 |  |
| Figure 6.32 | 18846 | 320 | 63663–63684 | 1208 |  |
| Figure 6.33 | 18953 | 322 | 63690–63732 | 1209 |  |
| Figure 7.1 | 19187 | 327 | 63738–63748 | 1210 |  |
| Figure 7.2 | 19341 | 329 | 63754–63771 | 1211 |  |
| Figure 7.3 | 19363 | 330 | 63777–63788 | 1212 |  |
| Figure 7.4 | 19551 | 333 | 63794–63808 | 1213 |  |
| Figure 7.5 | 19623 | 334 | 63814–63824 | 1214 |  |
| Figure 7.6 | 19689 | 335 | 63830–63844 | 1215 |  |
| Figure 7.7 | 19795 | 336 | 63850–63865 | 1216 | D |
| Figure 7.8 | 19852 | 337 | 63871–63888 | 1217 |  |
| Figure 7.9 | 19998 | 339 | 63894–63918 | 1218 |  |
| Figure 7.10 | 20057 | 340 | 63924–63947 | 1219 |  |
| Figure 7.11 | 20122 | 341 | 63953–63967 | 1220 |  |
| Figure 7.12 | 20324 | 344 | 63973–63995 | 1221 |  |
| Figure 7.13 | 20366 | 345 | 64001–64028 | 1222 |  |
| Figure 7.14 | 20412 | 346 | 64034–64063 | 1223 |  |
| Figure 7.15 | 20481 | 347 | 64069–64094 | 1224 |  |
| Figure 7.16 | 20685 | 350 | 64100–64135 | 1225 |  |
| Figure 7.17 | 20717 | 351 | 64141–64165 | 1226 |  |
| Figure 7.18 | 20777 | 352 | 64171–64188 | 1227 |  |
| Figure 7.19 | 20798 | 352 | 64194–64205 | 1228 |  |
| Figure 7.20 | 20871 | 353 | 64211–64237 | 1229 |  |
| Figure 7.21 | 21068 | 356 | 64243–64258 | 1230 |  |
| Figure 7.22 | 21120 | 357 | 64264–64283 | 1231 |  |
| Figure 7.23 | 21184 | 358 | 64289–64300 | 1232 |  |
| Figure 7.24 | 21354 | 360 | 64306–64320 | 1233 |  |
| Figure 7.25 | 21462 | 362 | 64326–64335 | 1234 |  |
| Figure 8.1 | 21900 | 369 | 64341–64354 | 1235 | D |
| Figure 8.2 | 22268 | 375 | 64360–64375 | 1236 | D |
| Figure 8.3 | 22339 | 376 | 64381–64402 | 1237 |  |
| Figure 8.4 | 22423 | 377 | 64408–64422 | 1238 |  |
| Figure 8.5 | 22462 | 378 | 64428–64441 | 1239 | D |
| Figure 8.6 | 22534 | 379 | 64447–64463 | 1240 |  |
| Figure 8.7 | 22561 | 380 | 64469–64495 | 1241 |  |
| Figure 8.8 | 22632 | 381 | 64501–64520 | 1242 | D |
| Figure 8.9 | 22665 | 381 | 64526–64549 | 1243 |  |
| Figure 8.10 | 22715 | 382 | 64555–64584 | 1244 |  |
| Figure 8.11 | 22779 | 383 | 64590–64603 | 1245 |  |
| Figure 8.12 | 22893 | 385 | 64609–64633 | 1246 | D |
| Figure 8.13 | 23049 | 387 | 64639–64651 | 1247 |  |
| Figure 8.14 | 23119 | 388 | 64657–64673 | 1248 |  |
| Figure 8.15 | 23186 | 389 | 64679–64708 | 1249 |  |
| Figure 8.16 | 23323 | 391 | 64714–64733 | 1250 |  |
| Figure 8.17 | 23573 | 394 | 64739–64757 | 1251 |  |
| Figure 8.18 | 23754 | 397 | 64763–64784 | 1252 | D |
| Figure 8.19 | 23801 | 398 | 64790–64804 | 1253 | D |
| Figure 8.20 | 23874 | 399 | 64810–64823 | 1254 | D |
| Figure 8.21 | 23919 | 400 | 64829–64865 | 1255 |  |
| Figure 8.22 | 24016 | 401 | 64871–64902 | 1256 |  |
| Figure 8.23 | 24051 | 402 | 64908–64933 | 1257 |  |
| Figure 8.24 | 24097 | 403 | 64939–64971 | 1258 |  |
| Figure 8.25 | 24475 | 407 | 64977–64998 | 1259 |  |
| Figure 8.26 | 24553 | 409 | 65004–65027 | 1260 |  |
| Figure 8.27 | 24624 | 410 | 65033–65062 | 1261 |  |
| Figure 8.28 | 24814 | 412 | 65068–65086 | 1262 |  |
| Figure 8.29 | 24880 | 414 | 65092–65104 | 1263 |  |
| Figure 8.30 | 24921 | 415 | 65110–65128 | 1264 |  |
| Figure 8.31 | 24995 | 416 | 65134–65152 | 1265 |  |
| Figure 8.32 | 25072 | 417 | 65158–65176 | 1266 |  |
| Figure 9.1 | 25987 | 432 | 65182–65202 | 1267 |  |
| Figure 9.2 | 26069 | 434 | 65208–65223 | 1268 | D |
| Figure 9.3 | 26110 | 434 | 65229–65248 | 1269 | D |
| Figure 9.4 | 26223 | 436 | 65254–65269 | 1270 | D |
| Figure 9.5 | 26270 | 437 | 65275–65300 | 1271 |  |
| Figure 9.6 | 26325 | 438 | 65306–65323 | 1272 |  |
| Figure 9.7 | 26346 | 438 | 65329–65343 | 1273 |  |
| Figure 9.8 | 26364 | 439 | 65349–65373 | 1274 |  |
| Figure 9.9 | 26387 | 439 | 65379–65394 | 1275 |  |
| Figure 9.10 | 26420 | 440 | 65400–65427 | 1276 |  |
| Figure 9.11 | 26498 | 442 | 65433–65454 | 1277 |  |
| Figure 9.12 | 26613 | 444 | 65460–65479 | 1278 |  |
| Figure 9.13 | 26772 | 446 | 65485–65511 | 1279 |  |
| Figure 9.14 | 26865 | 447 | 65517–65553 | 1280 |  |
| Figure 9.15 | 26915 | 448 | 65559–65585 | 1281 |  |
| Figure 9.16 | 26944 | 449 | 65591–65614 | 1282 |  |
| Figure 9.17 | 26983 | 450 | 65620–65636 | 1283 |  |
| Figure 9.18 | 27054 | 451 | 65642–65668 | 1284 |  |
| Figure 9.19 | 27134 | 452 | 65674–65690 | 1285 |  |
| Figure 9.20 | 27212 | 453 | 65696–65711 | 1286 |  |
| Figure 9.21 | 27247 | 454 | 65717–65734 | 1287 |  |
| Figure 9.22 | 27349 | 456 | 65740–65759 | 1288 |  |
| Figure 9.23 | 27389 | 457 | 65765–65784 | 1289 |  |
| Figure 9.24 | 27410 | 457 | 65790–65814 | 1290 |  |
| Figure 9.25 | 27437 | 458 | 65820–65848 | 1291 |  |
| Figure 9.26 | 27570 | 460 | 65854–65878 | 1292 |  |
| Figure 9.27 | 27614 | 461 | 65884–65911 | 1293 | D |
| Figure 9.28 | 27694 | 462 | 65917–65942 | 1294 |  |
| Figure 9.29 | 27821 | 464 | 65948–65971 | 1295 |  |
| Figure 9.30 | 27870 | 465 | 65977–66004 | 1296 |  |
| Figure 9.31 | 27917 | 466 | 66010–66035 | 1297 |  |
| Figure 9.32 | 27994 | 467 | 66041–66066 | 1298 |  |
| Figure 9.33 | 28033 | 468 | 66072–66103 | 1299 |  |
| Figure 9.34 | 28135 | 470 | 66109–66136 | 1301 |  |
| Figure 9.35 | 28290 | 472 | 66142–66168 | 1302 |  |
| Figure 9.36 | 28330 | 473 | 66174–66194 | 1303 |  |
| Figure 9.37 | 28386 | 474 | 66200–66225 | 1304 |  |
| Figure 9.38 | 28487 | 475 | 66231–66269 | 1305 |  |
| Figure 9.39 | 28527 | 476 | 66275–66296 | 1306 |  |
| Figure 9.40 | 28557 | 477 | 66302–66348 | 1307 |  |
| Figure 9.41 | 28599 | 478 | 66354–66383 | 1309 |  |
| Figure 9.42 | 28664 | 479 | 66389–66441 | 1310 |  |
| Figure 10.1 | 29006 | 486 | 66447–66473 | 1312 | D |
| Figure 10.2 | 29028 | 487 | 66479–66495 | 1313 |  |
| Figure 10.3 | 29123 | 488 | 66501–66513 | 1314 |  |
| Figure 10.4 | 29165 | 489 | 66519–66536 | 1315 | D |
| Figure 10.5 | 29272 | 490 | 66542–66557 | 1316 | D |
| Figure 10.6 | 29295 | 491 | 66563–66593 | 1317 |  |
| Figure 10.7 | 29371 | 492 | 66599–66630 | 1318 |  |
| Figure 10.8 | 29418 | 493 | 66636–66662 | 1319 |  |
| Figure 10.9 | 29606 | 496 | 66668–66694 | 1320 |  |
| Figure 10.10 | 29648 | 497 | 66700–66730 | 1321 |  |
| Figure 10.11 | 29670 | 497 | 66736–66762 | 1322 |  |
| Figure 10.12 | 29822 | 499 | 66768–66785 | 1323 |  |
| Figure 10.13 | 29868 | 500 | 66791–66820 | 1324 |  |
| Figure 10.14 | 29903 | 501 | 66826–66851 | 1325 |  |
| Figure 10.15 | 30026 | 503 | 66857–66902 | 1326 |  |
| Figure 10.16 | 30062 | 503 | 66908–66942 | 1327 |  |
| Figure 10.17 | 30132 | 505 | 66948–66966 | 1328 |  |
| Figure 10.18 | 30159 | 505 | 66972–66996 | 1329 |  |
| Figure 10.19 | 30323 | 508 | 67002–67043 | 1330 |  |
| Figure 10.20 | 30352 | 508 | 67049–67066 | 1331 | D |
| Figure 10.21 | 30450 | 510 | 67072–67087 | 1332 |  |
| Figure 10.22 | 30566 | 512 | 67093–67116 | 1333 | D |
| Figure 10.23 | 30614 | 513 | 67122–67156 | 1334 | D |
| Figure 10.24 | 30666 | 514 | 67162–67198 | 1335 | D |
| Figure 10.25 | 30684 | 514 | 67204–67231 | 1336 |  |
| Figure 10.26 | 30732 | 515 | 67237–67297 | 1337 | D |
| Figure 10.27 | 30913 | 518 | 67303–67319 | 1339 |  |
| Figure 10.28 | 30981 | 519 | 67325–67352 | 1340 |  |
| Figure 10.29 | 31022 | 520 | 67358–67381 | 1341 |  |
| Figure 10.30 | 31121 | 521 | 67387–67417 | 1342 |  |
| Figure 10.31 | 31703 | 532 | 67423–67466 | 1343 | D |
| Figure 10.32 | 31724 | 532 | 67472–67494 | 1344 | D |
| Figure 11.1 | 31895 | 536 | 67500–67515 | 1345 |  |
| Figure 11.2 | 31937 | 537 | 67521–67550 | 1346 |  |
| Figure 11.3 | 31987 | 538 | 67556–67570 | 1347 |  |
| Figure 11.4 | 32145 | 540 | 67576–67598 | 1348 |  |
| Figure 11.5 | 32215 | 541 | 67604–67626 | 1349 |  |
| Figure 11.6 | 32268 | 542 | 67632–67661 | 1350 |  |
| Figure 11.7 | 32324 | 543 | 67667–67685 | 1351 |  |
| Figure 11.8 | 32368 | 544 | 67691–67711 | 1352 |  |
| Figure 11.9 | 32402 | 545 | 67717–67746 | 1353 |  |
| Figure 11.10 | 32613 | 547 | 67752–67783 | 1354 |  |
| Figure 11.11 | 32672 | 548 | 67789–67823 | 1355 |  |
| Figure 11.12 | 32731 | 549 | 67829–67858 | 1356 |  |
| Figure 11.13 | 32806 | 550 | 67864–67891 | 1357 |  |
| Figure 11.14 | 32941 | 553 | 67897–67948 | 1358 |  |
| Figure 11.15 | 33038 | 555 | 67954–67964 | 1360 |  |
| Figure 11.16 | 33127 | 556 | 67970–67999 | 1361 |  |
| Figure 11.17 | 33286 | 558 | 68005–68028 | 1362 |  |
| Figure 11.18 | 33394 | 560 | 68034–68086 | 1363 |  |
| Figure 11.19 | 33450 | 561 | 68092–68116 | 1365 |  |
| Figure 11.20 | 33487 | 561 | 68122–68153 | 1366 |  |
| Figure 11.21 | 33580 | 563 | 68159–68185 | 1367 |  |
| Figure 11.22 | 33623 | 564 | 68191–68222 | 1368 |  |
| Figure 11.23 | 33697 | 566 | 68228–68240 | 1369 |  |
| Figure 11.24 | 33766 | 567 | 68246–68264 | 1370 |  |
| Figure 12.1 | 34326 | 577 | 68270–68286 | 1371 |  |
| Figure 12.2 | 34485 | 579 | 68292–68330 | 1372 |  |
| Figure 12.3 | 34869 | 584 | 68336–68355 | 1373 | D |
| Figure 12.4 | 35007 | 586 | 68361–68396 | 1374 |  |
| Figure 12.5 | 35051 | 587 | 68402–68416 | 1375 | D |
| Figure 12.6 | 35239 | 590 | 68422–68435 | 1376 |  |
| Figure 12.7 | 35270 | 590 | 68441–68456 | 1377 |  |
| Figure 12.8 | 35325 | 592 | 68462–68487 | 1378 |  |
| Figure 12.9 | 35369 | 593 | 68493–68524 | 1379 |  |
| Figure 12.10 | 35438 | 594 | 68530–68551 | 1380 |  |
| Figure 12.11 | 35492 | 595 | 68557–68615 | 1381 |  |
| Figure 12.12 | 35533 | 596 | 68621–68667 | 1383 |  |
| Figure 12.13 | 35617 | 597 | 68673–68713 | 1385 |  |
| Figure 12.14 | 35651 | 598 | 68719–68757 | 1386 |  |
| Figure 12.15 | 35713 | 599 | 68763–68787 | 1387 |  |
| Figure 12.16 | 35808 | 601 | 68793–68809 | 1388 |  |
| Figure 12.17 | 35951 | 603 | 68815–68837 | 1389 |  |
| Figure 12.18 | 35984 | 603 | 68843–68868 | 1390 |  |
| Figure 12.19 | 36069 | 605 | 68874–68913 | 1391 |  |
| Figure 12.20 | 36098 | 606 | 68919–68946 | 1392 |  |
| Figure 12.21 | 36319 | 608 | 68952–68971 | 1393 |  |
| Figure 12.22 | 36365 | 609 | 68977–69004 | 1394 |  |
| Figure 12.23 | 36424 | 610 | 69010–69028 | 1395 |  |
| Figure 12.24 | 36451 | 611 | 69034–69052 | 1396 |  |
| Figure 12.25 | 36523 | 612 | 69058–69079 | 1397 |  |
| Figure 12.26 | 36595 | 613 | 69085–69099 | 1398 |  |
| Figure 12.27 | 36625 | 614 | 69105–69135 | 1399 |  |
| Figure 12.28 | 36659 | 615 | 69141–69171 | 1400 |  |
| Figure 12.29 | 36692 | 616 | 69177–69191 | 1401 |  |
| Figure 12.30 | 36718 | 616 | 69197–69217 | 1402 |  |
| Figure 12.31 | 36792 | 617 | 69223–69240 | 1403 |  |
| Figure 12.32 | 36833 | 618 | 69246–69268 | 1404 |  |
| Figure 12.33 | 36952 | 620 | 69274–69307 | 1405 |  |
| Figure 12.34 | 37001 | 621 | 69313–69335 | 1406 |  |
| Figure 12.35 | 37036 | 622 | 69341–69365 | 1407 |  |
| Figure 12.36 | 37228 | 625 | 69371–69388 | 1408 |  |
| Figure 12.37 | 37262 | 626 | 69394–69418 | 1409 |  |
| Figure 12.38 | 37353 | 627 | 69424–69442 | 1410 |  |
| Figure 12.39 | 37450 | 629 | 69448–69470 | 1411 |  |
| Figure 12.40 | 37538 | 630 | 69476–69500 | 1412 |  |
| Figure 12.41 | 37567 | 631 | 69506–69538 | 1413 |  |
| Figure 12.42 | 37613 | 632 | 69544–69578 | 1414 |  |
| Figure 12.43 | 37677 | 633 | 69584–69595 | 1415 |  |
| Figure 12.44 | 37883 | 636 | 69601–69625 | 1416 |  |
| Figure 12.45 | 37937 | 637 | 69631–69677 | 1417 |  |
| Figure 12.46 | 37990 | 638 | 69683–69712 | 1419 |  |
| Figure 12.47 | 38022 | 639 | 69718–69734 | 1420 |  |
| Figure 12.48 | 38108 | 641 | 69740–69779 | 1421 | D |
| Figure 13.1 | 38417 | 647 | 69785–69809 | 1422 |  |
| Figure 13.2 | 38467 | 648 | 69815–69838 | 1423 |  |
| Figure 13.3 | 38500 | 648 | 69844–69861 | 1424 |  |
| Figure 13.4 | 38529 | 649 | 69867–69883 | 1425 |  |
| Figure 13.5 | 38547 | 649 | 69889–69909 | 1426 |  |
| Figure 13.6 | 38682 | 651 | 69915–69938 | 1427 |  |
| Figure 13.7 | 38798 | 653 | 69944–69968 | 1428 |  |
| Figure 13.8 | 38886 | 654 | 69974–69995 | 1429 |  |
| Figure 13.9 | 38920 | 655 | 70001–70021 | 1430 |  |
| Figure 13.10 | 38940 | 655 | 70027–70048 | 1431 |  |
| Figure 13.11 | 38980 | 656 | 70054–70079 | 1432 |  |
| Figure 13.12 | 39019 | 657 | 70085–70105 | 1433 |  |
| Figure 13.13 | 39049 | 657 | 70111–70126 | 1434 | D |
| Figure 13.14 | 39156 | 659 | 70132–70154 | 1435 |  |
| Figure 13.15 | 39360 | 662 | 70160–70174 | 1436 |  |
| Figure 13.16 | 39412 | 663 | 70180–70199 | 1437 | D |
| Figure 13.17 | 39462 | 664 | 70205–70229 | 1438 | D |
| Figure 13.18 | 39522 | 665 | 70235–70261 | 1439 | D |
| Figure 13.19 | 39603 | 666 | 70267–70293 | 1440 | D |
| Figure 13.20 | 39677 | 667 | 70299–70321 | 1441 | D |
| Figure 13.21 | 39708 | 668 | 70327–70367 | 1442 | D |
| Figure 13.22 | 39771 | 669 | 70373–70401 | 1443 |  |
| Figure 13.23 | 39834 | 670 | 70407–70436 | 1444 |  |
| Figure 13.24 | 39906 | 671 | 70442–70475 | 1445 |  |
| Figure 13.25 | 39983 | 672 | 70481–70512 | 1446 |  |
| Figure 13.26 | 40009 | 673 | 70518–70544 | 1447 |  |
| Figure 13.27 | 40041 | 674 | 70550–70576 | 1448 |  |
| Figure 13.28 | 40078 | 674 | 70582–70607 | 1449 |  |
| Figure 13.29 | 40133 | 675 | 70613–70639 | 1450 |  |
| Figure 13.30 | 40158 | 676 | 70645–70681 | 1451 |  |
| Figure 13.31 | 40180 | 677 | 70687–70713 | 1453 |  |
| Figure 13.32 | 40236 | 678 | 70719–70749 | 1454 |  |
| Figure 13.33 | 40299 | 679 | 70755–70782 | 1455 |  |
| Figure 13.34 | 40350 | 680 | 70788–70805 | 1456 |  |
| Figure 13.35 | 40414 | 681 | 70811–70831 | 1457 |  |
| Figure 13.36 | 40426 | 681 | 70837–70870 | 1458 |  |
| Figure 13.37 | 40463 | 682 | 70876–70915 | 1459 |  |
| Figure 13.38 | 40494 | 683 | 70921–70939 | 1460 |  |
| Figure 13.39 | 40528 | 683 | 70945–70969 | 1461 |  |
| Figure 13.40 | 40612 | 685 | 70975–70997 | 1462 |  |
| Figure 13.41 | 40869 | 689 | 71003–71027 | 1463 | D |
| Figure 13.42 | 40922 | 690 | 71033–71063 | 1464 |  |
| Figure 13.43 | 41001 | 691 | 71069–71092 | 1465 | D |
| Figure 13.44 | 41032 | 691 | 71098–71122 | 1466 |  |
| Figure 13.45 | 41076 | 692 | 71128–71156 | 1467 |  |
| Figure 13.46 | 41146 | 693 | 71162–71176 | 1468 |  |
| Figure 13.47 | 41183 | 694 | 71182–71205 | 1469 |  |
| Figure 13.48 | 41227 | 694 | 71211–71239 | 1470 |  |
| Figure 13.49 | 41261 | 695 | 71245–71279 | 1471 |  |
| Figure 13.50 | 41301 | 696 | 71285–71298 | 1472 |  |
| Figure 13.51 | 41336 | 696 | 71304–71344 | 1473 |  |
| Figure 13.52 | 41441 | 698 | 71350–71374 | 1474 |  |
| Figure 13.53 | 41506 | 699 | 71380–71403 | 1475 |  |
| Figure 13.54 | 41633 | 701 | 71409–71431 | 1476 |  |
| Figure 13.55 | 41891 | 704 | 71437–71460 | 1477 |  |
| Figure 13.56 | 42049 | 706 | 71466–71522 | 1478 |  |
| Figure 13.57 | 42117 | 708 | 71528–71550 | 1480 |  |
| Figure 13.58 | 42161 | 709 | 71556–71569 | 1481 |  |
| Figure 13.59 | 42286 | 711 | 71575–71596 | 1482 |  |
| Figure 13.60 | 42850 | 721 | 71602–71686 | 1483 | D |
| Figure 14.1 | 42937 | 723 | 71692–71712 | 1485 |  |
| Figure 14.2 | 42956 | 723 | 71718–71741 | 1486 |  |
| Figure 14.3 | 43124 | 726 | 71747–71765 | 1487 | D |
| Figure 14.4 | 43180 | 727 | 71771–71793 | 1488 |  |
| Figure 14.5 | 43240 | 728 | 71799–71813 | 1489 |  |
| Figure 14.6 | 43282 | 729 | 71819–71833 | 1490 |  |
| Figure 14.7 | 43320 | 729 | 71839–71852 | 1491 |  |
| Figure 14.8 | 43372 | 730 | 71858–71890 | 1492 |  |
| Figure 14.9 | 43414 | 731 | 71896–71958 | 1493 | D |
| Figure 14.10 | 43679 | 735 | 71964–71988 | 1495 |  |
| Figure 14.11 | 43747 | 736 | 71994–72018 | 1496 |  |
| Figure 14.12 | 43824 | 738 | 72024–72046 | 1497 |  |
| Figure 14.13 | 43870 | 738 | 72052–72065 | 1498 |  |
| Figure 14.14 | 43896 | 739 | 72071–72119 | 1499 |  |
| Figure 14.15 | 43972 | 740 | 72125–72173 | 1501 |  |
| Figure 14.16 | 44098 | 743 | 72179–72212 | 1503 |  |
| Figure 14.17 | 44268 | 746 | 72218–72237 | 1504 |  |
| Figure 14.18 | 44292 | 746 | 72243–72264 | 1505 |  |
| Figure 14.19 | 44311 | 747 | 72270–72292 | 1506 |  |
| Figure 14.20 | 44329 | 747 | 72298–72320 | 1507 |  |
| Figure 14.21 | 44367 | 748 | 72326–72343 | 1508 | D |
| Figure 14.22 | 44402 | 749 | 72349–72378 | 1509 |  |
| Figure 14.23 | 44482 | 750 | 72384–72409 | 1510 |  |
| Figure 14.24 | 44628 | 752 | 72415–72447 | 1511 |  |
| Figure 14.25 | 44698 | 753 | 72453–72485 | 1512 |  |
| Figure 14.26 | 44772 | 754 | 72491–72515 | 1513 |  |
| Figure 14.27 | 44813 | 755 | 72521–72549 | 1514 |  |
| Figure 14.28 | 44848 | 756 | 72555–72569 | 1515 |  |
| Figure 14.29 | 44905 | 757 | 72575–72599 | 1516 |  |
| Figure 14.30 | 45011 | 758 | 72605–72627 | 1517 |  |
| Figure 14.31 | 45120 | 760 | 72633–72686 | 1518 |  |
| Figure 14.32 | 45219 | 761 | 72692–72744 | 1519 | D |
| Figure 14.33 | 45306 | 763 | 72750–72792 | 1521 |  |
| Figure 14.34 | 45396 | 764 | 72798–72818 | 1522 |  |
| Figure 14.35 | 45541 | 766 | 72824–72871 | 1523 |  |
| Figure 14.36 | 45583 | 767 | 72877–72924 | 1524 |  |
| Figure 14.37 | 45655 | 768 | 72930–72951 | 1525 |  |
| Figure 14.38 | 45692 | 769 | 72957–72982 | 1526 |  |
| Figure 14.39 | 45737 | 770 | 72988–73009 | 1527 |  |
| Figure 14.40 | 45818 | 771 | 73015–73026 | 1528 |  |
| Figure 14.41 | 45856 | 772 | 73032–73042 | 1529 |  |
| Figure 14.42 | 45960 | 773 | 73048–73061 | 1530 |  |
| Figure 14.43 | 46203 | 779 | 73067–73083 | 1531 | D |
| Figure 14.44 | 46223 | 779 | 73089–73116 | 1532 |  |
| Figure 15.1 | 46375 | 782 | 73122–73132 | 1533 |  |
| Figure 15.2 | 46683 | 786 | 73138–73154 | 1534 | D |
| Figure 15.3 | 46763 | 788 | 73160–73171 | 1535 |  |
| Figure 15.4 | 47122 | 793 | 73177–73189 | 1536 |  |
| Figure 15.5 | 47159 | 794 | 73195–73210 | 1537 | D |
| Figure 15.6 | 47194 | 795 | 73216–73235 | 1538 |  |
| Figure 15.7 | 47235 | 796 | 73241–73258 | 1539 | D |
| Figure 15.8 | 47261 | 797 | 73264–73274 | 1540 |  |
| Figure 15.9 | 47365 | 799 | 73280–73299 | 1541 |  |
| Figure 15.10 | 47432 | 800 | 73305–73323 | 1542 |  |
| Figure 15.11 | 47477 | 801 | 73329–73346 | 1543 |  |
| Figure 15.12 | 47586 | 802 | 73352–73367 | 1544 |  |
| Figure 15.13 | 47626 | 803 | 73373–73391 | 1545 |  |
| Figure 15.14 | 47665 | 804 | 73397–73413 | 1546 |  |
| Figure 15.15 | 47793 | 806 | 73419–73434 | 1547 |  |
| Figure 15.16 | 47839 | 806 | 73440–73457 | 1548 |  |
| Figure 15.17 | 47886 | 807 | 73463–73487 | 1549 |  |
| Figure 15.18 | 47943 | 808 | 73493–73517 | 1550 |  |
| Figure 15.19 | 47996 | 810 | 73523–73541 | 1551 |  |
| Figure 15.20 | 48033 | 810 | 73547–73557 | 1552 |  |
| Figure 15.21 | 48091 | 812 | 73563–73586 | 1553 |  |
| Figure 15.22 | 48196 | 813 | 73592–73614 | 1554 |  |
| Figure 15.23 | 48347 | 815 | 73620–73629 | 1555 |  |
| Figure 15.24 | 48386 | 816 | 73635–73648 | 1556 |  |
| Figure 15.25 | 48428 | 816 | 73654–73670 | 1557 |  |
| Figure 15.26 | 48528 | 818 | 73676–73693 | 1558 | D |
| Figure 15.27 | 48564 | 818 | 73699–73715 | 1559 |  |
| Figure 15.28 | 48665 | 820 | 73721–73736 | 1560 |  |
| Figure 15.29 | 48838 | 822 | 73742–73757 | 1561 | D |
| Figure B.1 | 49581 | 834 | 73763–73774 | 1562 |  |
| Figure B.2 | 49614 | 835 | 73780–73797 | 1563 | D |
| Table 8.45 | 51035 | 855 | 73803–73832 | 1564 | D |
| Table 10.T1 | 51336 | 861 | 73838–73863 | 1565 | D |
| Figure E.1 | 51785 | 871 | 73869–73885 | 1566 |  |
| Figure E.2 | 51942 | 874 | 73891–73909 | 1567 |  |
| Figure E.3 | 52012 | 875 | 73915–73932 | 1568 |  |
