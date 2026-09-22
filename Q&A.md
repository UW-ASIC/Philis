# Q&A

Answers to design questions about the flow, with the measurements behind them.
Numbers are measured on the sky130 deck (`pdks/sky130.json`) unless stated.

---

## Q: Each device can be drawn as several different shapes that are all the same device, and we can change between them. Is that still the case?

**Yes**, and in the automated flow it is still a searched variable, not a fixed
recipe. Measured alternative counts out of `cells::mosfet::Mosfet::enumerate`:

| cell                                       | alternatives |
| ------------------------------------------ | -----------: |
| 1 device, W=420 (PDK minimum finger width) |            3 |
| 1 device, W=1680                           |            3 |
| matched pair, W=420                        |            3 |
| matched pair, W=1680                       |            3 |
| matched pair, W=1680, nf=2                 |            9 |
| matched quad, W=1680, nf=1 or 2            |            3 |
| matched quad, W=1680, nf=4                 |            9 |

Two axes, and the finger count is **not** one of them:

- **`nf` is fixed by the schematic, not searched.** `feasible_nf`
  (`kernel/cells/src/mosfet.rs`) used to also offer *refolds* — the same total
  width redrawn as 2 / 4 / 8 narrower fingers — and every one of them drew the
  wrong device. The convention everywhere else in the flow is that `w` is the
  **finger** width and `nf` the finger count (`draw` gives every finger
  `s.unit_w`, and `cellgen::reference` emits one reference card per *schematic*
  finger at the full `w`), so an `nf=4` variant of a one-finger device drew a
  4×-wide transistor against a reference expecting one card. Forcing `nf=2` on
  the old generator fails `pair` outright with `lvs.unpaired_device`; the suite
  was green only because the placer happened to pick `nf=1`, which is not a
  property, it is luck. The refolds are gone. Restoring them means teaching
  `reference()` the *drawn* finger count — which `VariantSpace` currently
  discards, since it stores drawn `Macro`s and not the specs behind them.
- **dummies per edge** — `1`, `2` or `0`. Zero is a real point of the space:
  "are the dummies worth their area here?" is an answerable question. It is
  withheld from any group whose `Unitization` sets `dummy_required` (every
  annotator-recognised matched block), because there the dummies are a
  constraint and not a trade.
- **interleave pattern** — `Single` / `Cc1d` / `Cc2d`
  (`feasible_styles`, `kernel/cells/src/mosfet.rs`), for **any** member count,
  at any finger count that can form a centroid: even `nf` for a pair, `nf`
  divisible by 4 for three or more members. See limit 3 below.

### The two paths consume that space differently

**Automated** — fully searched. `cellgen::enumerate`
(`frontend/library/src/cellgen.rs:78`) draws *every* legal alternative into a
`gp::VariantSpace`; `Layout::variant` holds one chosen index per cell;
`cellgen::seed_assignment` (`:304`) prices the hypotheses to seed the search;
`dp` swaps variants as an annealing move; the outer loop escalates the variant
assignment when the middle tier stalls infeasible. This is PLAN §2's
"variant selection is a first-class search variable", and it is real.

**Manual (macroMaster / `elaborate`)** — not searched. `route_built` sets
`variant: vec![0; n]` (`frontend/library/src/elaborate.rs:234`) and there is no
`gp`/`dp` at all: the Composition's own `place`/`place_by` calls *are* the
placement. You pick the shape yourself through
`Mos { nf, pattern, dummies_per_edge }`.

> That manual choice only began working on 2026-09-21. Previously
> `adapter::draw_group_where` selected the smallest-footprint variant, which is
> always the 1-finger fold, so `Mos::new(kind, w, l, nf)` silently drew **one
> finger for every `nf`**. `nf` is now part of the variant filter.

### Three limits worth knowing

1. **A one-finger device has only 3 alternatives** — the three dummy counts;
   the geometry is otherwise identical. The space grows with the *schematic's*
   finger count, because that is what the interleave patterns have to work with.

2. **`dummies_per_edge = 0` is available, but it is opt-in on the manual path.**
   The generator enumerates it, so the placer can trade the dummies away (doing
   so cut `pair` from 10.3 fF to 7.8 fF of extracted parasitic). `Mos {
   dummies_per_edge: Some(0) }` now resolves instead of falling through to
   `adapter.rs`'s unfiltered pool. `None` still means *any non-zero count*: the
   adapter breaks ties on smallest area, so plain `None` would have silently
   redrawn every hand-placed device without its LOD/WPE dummies.

3. **Common centroid needs enough fingers, and only in one dimension.** A
   boundary between two different devices must land on a *source* region, and
   `draw` alternates drain/source by region parity — so fingers pair up and the
   two array ends fall to one device. A pair can therefore centroid at any even
   `nf`; three or more members need `nf` divisible by 4 (only one device can own
   the self-symmetric middle pair, so the rest need whole mirror-orbits). At
   `nf = 1` or `2` a quad has no centroid order that is not also a drawn short,
   and `Single` is the honest answer. The classic **2×2** ABBA/BAAB centroid is
   still out of reach for a different reason: `mosfet::draw` draws exactly one
   diffusion row and has no second-row concept at all.

---

## Q: This is free-form placement and routing? Are there specific layout techniques that can be researched with this automated P&R to see whether they are actually optimal?

### Free-form: yes for placement, lattice for routing

Placement is genuinely free-form analog, not standard-cell rows. Devices are
macros at arbitrary `(x, y)` with a D4 orientation; `gp` is analytical momentum
descent (HPWL + density + analog cost) and `dp` is simulated annealing over
position, orientation, variant and disjunctive-branch moves. There is no site
grid and no row structure.

Routing _is_ lattice-based — a track grid at a derived pitch (460 nm on sky130,
from `wire_width 320 + met1 spacing 140`), routed by negotiated PathFinder,
coarse (`gr`) then fine (`dr`).

### Why this is a good instrument for the question

Two properties matter more than the placer itself:

- **The free oracle.** `verify::LiveOracle` (`backend/verify/src/lib.rs:183`)
  gives per-move DRC and _real extracted_ PEX. So "is technique X better" is
  measurable against extracted parasitics rather than a model or an assertion.
  This is the architectural bet in PLAN, and it is what makes technique
  comparison empirical.
- **One extension point.** A layout technique enters as an `analog::Rule`
  (`kernel/analog/src/rule.rs:26` — `cost` / `satisfied` / `residual` /
  `project`), sorted into the hard / budget / cost arms of `Requirements`.
  If it needs new geometry, it also adds a variant axis.

Both the automated and the manual path run the same `cells` -> `gr` -> `dr` ->
`verify` back half, so a technique studied in one applies to the other.

### Questions the codebase is already shaped to answer

| Question                                                  | Hook                                                                      | Status                                                                                                                 |
| --------------------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| ABBA vs ABAB vs block interleave                          | `Pattern` variant axis + matching rules + PEX                             | **blocked for >2 devices** (limit 3 above)                                                                             |
| Do dummies pay for their area?                            | `dummies_per_edge` variant axis                                           | **blocked** (limit 2 above)                                                                                            |
| Merged/interdigitated vs separate devices + routing       | `cellgen` group collapse; its decline paths are explicit and logged       | ready                                                                                                                  |
| Shared vs private guard rings                             | `post_cell::guard_rings` `merge_gap` (`kernel/cells/src/post_cell.rs:45`) | ready                                                                                                                  |
| Does common-centroid really put partners on one isotherm? | `ThermalGradient` + power map                                             | needs `Config::op` (`frontend/library/src/lib.rs:107`); without a power map every ΔT is 0 and the rule passes vacuously |
| Is symmetry exact, or only near-exact?                    | PLAN §4a claims exact-by-construction                                     | checkable today                                                                                                        |

### Honest caveats before trusting it as a measurement instrument

Benchmarking layout techniques on a flow with its own bugs measures the bugs.
As of 2026-09-21, all five fixtures (`pair`, `quad`, `rc_filter`, `bjt_mirror`,
`chain4`) plus the hand-written 5T OTA sign off **LVS clean** — but:

- **NMOS devices are being drawn inside n-wells, and signoff cannot see it.**
  `post_cell.rs:263` gives an `Ecgr` ring `nsdm` (n+ implant), correct for
  "n+ in p-substrate" — but `:330` *also* draws `nwell` over the ring interior
  for `Ecgr`/`Ebgr`, and the annotator gives every NMOS an `Ecgr`. Measured on
  `chain4`: **3 of 4 diff rects sit fully inside an nwell**, and magic 8.3.573
  extracts **1 nfet from a 4-NMOS circuit**, with that device's drain and bulk
  on the same well node. GPurify's deck has no channel-vs-well polarity rule,
  so LVS reads clean. This is the strongest argument in the repo for keeping an
  independent extractor in the loop. **Not fixed** — the correction also moves
  `ring_halo` (`:392`) and the outer clearance (`:374`), i.e. every fixture's
  placement.
- The `nf` multiplier bug above: the variant search can silently pick a device
  2× or 4× the requested width, and which fixtures it bites is seed-dependent.
- The two variant-space holes (no `dummies=0`, no centroid beyond pairs).
- `chain4` reports ERC 100, all `floating_interconnect`. That one *is* a deck
  limitation rather than a layout defect: the rule fires on a polygon whose net
  reaches no device, the sky130 MOS recogniser is 3-terminal so bulk cannot
  bind, and `chain4` is the only fixture whose `VSS` is bulk-only
  (`XM1 a g b VSS`). Every other fixture ties VSS to a source.
- PLAN §5's convergence machinery is absent: no parallel tempering, no
  multiplier-stationarity termination. The search is a single trajectory, so
  "no better arrangement exists" is not certifiable — only "none was found".
