# Q&A

Answers to design questions about the flow, with the measurements behind them.
Numbers are measured on the sky130 deck (`pdks/sky130.json`) unless stated.

---

## Q: Each device can be drawn as several different shapes that are all the same device, and we can change between them. Is that still the case?

**Yes**, and in the automated flow it is still a searched variable, not a fixed
recipe. Measured alternative counts out of `cells::mosfet::Mosfet::enumerate`:

| cell                                       | alternatives |
| ------------------------------------------ | -----------: |
| 1 device, W=420 (PDK minimum finger width) |            2 |
| 1 device, W=1680                           |            6 |
| matched pair, W=420                        |            6 |
| matched pair, W=1680                       |           16 |

Three axes:

- **`nf`** — *intended* as a refold (one total width drawn as 1 / 2 / 4 …
  fingers), but **currently a multiplier, which is a bug**. `feasible_nf`
  (`kernel/cells/src/mosfet.rs:575`) offers `nf` where `w_total / nf` lands
  inside the PDK finger window — i.e. it treats `nf` as a *split* — but `draw`
  uses `let finger_w = s.unit_w;` (`:144`) for every finger, so an `nf=4`
  variant draws **4× the requested W**. `cellgen::reference` then expands the
  *schematic's* `nf`, not the drawn one, so reference and layout disagree on
  device count as well as size. `chain4` at seed 1 escalates to nf 4/1/1/2 →
  8 drawn fingers against 4 reference cards → 12 `unpaired_device`. Seed 42
  keeps nf=1, which is the only reason that fixture is clean. **Not fixed**:
  one side has to change, and doing it properly needs the drawn finger count
  plumbed into `reference()`.
- **dummies per edge**
- **interleave pattern** — `Single` / `Cc1d` / `Cc2d`
  (`feasible_styles`, `kernel/cells/src/mosfet.rs:564`)

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

1. **A minimum-width device has only 2 alternatives** — the two dummy counts;
   the geometry is otherwise identical. The space grows with *total* width,
   because a refold has to divide it evenly and land inside the PDK's
   finger-width window.

2. **`dummies_per_edge` is never 0.** The generator only ever enumerates 1 or 2,
   so "are dummies worth their area here?" is currently *unaskable* — no device
   can be drawn without them. This is also why `Mos { dummies_per_edge: Some(0) }`
   degrades silently: nothing matches the filter and `adapter.rs:92` falls back
   to the unfiltered pool.

3. **Common-centroid patterns exist only for pairs.** `feasible_styles` adds
   `Cc1d`/`Cc2d` only when `n_devices == 2`; a matched quad gets `Single` only,
   so the classic 2x2 ABBA/BAAB centroid is not in the space.

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
