# M2–M6 hard-dependency graph, by module

Source: §5 "Items" lists of `00-MASTER-PLAN.md` (M2–M6), each item's "Depends on" line in plan-01…08 and
`98-gap-critic.md`, the Status section and §6. Data: `dag-m2-m6.json` (`items`, `moved`, `dropped_deps`, `modules`).

**Hard** means the item needs that item's merged code before it can start. Items done in M0/M1 count as merged and are
left out. Consumers, producers, optional inputs, "Cross:" notes and later refinements are not hard. When only some
steps need an item, it is listed in `step_level` with the steps that can run first.

**Scope.** 140 IDs. Not counted as scheduled: mentions of done items (MAT-01, MAT-04, FLOW-05, PERF-01, PERF-09,
GAP-05, REL-03, RTE-32) and aliases (EXT-08 = GAP-09, PERF-25 = GAP-08, MAT-22 = GAP-02). If you count 139, one of
these was probably also excluded: PLC-10 (step 0 was M1), FLOW-13 (step 0 was M1) or PERF-20 (it appears in both M2
and M5).

**Check** (scratchpad script): the hard graph is acyclic, and no hard dependency is in a later milestone than the item
that needs it. Two items are split into slices, and their slices are checked the same way. Each module's order below is
one global topological order sorted by (milestone, master landing order). So hard edges plus each module's sequence
also form an acyclic graph, which means no two module agents can deadlock waiting on each other.

## Modules (execution order; `⟵` = hard wait on another module)

- **analog-matching** (16): **M2** GAP-01 · MAT-07 · MAT-08 · MAT-13 · **M3** GAP-02 · MAT-09 · MAT-11 · MAT-12 · MAT-10 · **M6** MAT-14 · MAT-15 · MAT-16 · MAT-17 · MAT-18 · MAT-21 · GAP-18
- **annotator** (19): **M2** EXT-12 ⟵ GAP-01 · EXT-13 · EXT-14 · EXT-15 · EXT-16 ⟵ GAP-01 · EXT-19 · EXT-20 ⟵ MAT-07, MAT-08 · **M3** EXT-17 · EXT-18 · EXT-23 · GAP-03 · EXT-24 · EXT-26 · GAP-09 · **M4** EXT-21 · EXT-25 · **M6** EXT-27 · EXT-28 · EXT-29
- **cells** (19): **M2** CELL-11 · CELL-12 ⟵ GAP-01, EXT-12 · CELL-13 · CELL-19 · **M3** CELL-14 ⟵ MAT-12, GAP-01, EXT-15 · CELL-16 ⟵ GAP-07 · CELL-17 · CELL-18 · **M6** CELL-15 ⟵ MAT-15 · CELL-21 ⟵ MAT-18, EXT-19 · CELL-20 · CELL-22 ⟵ REL-16 · CELL-23 ⟵ GAP-01 · CELL-25 · CELL-29 · GAP-11 ⟵ GAP-01, EXT-16 · GAP-14 ⟵ EXT-26 · GAP-15 · GAP-19 ⟵ EXT-18
- **placement** (20): **M2** PLC-07 · PLC-13 ⟵ GAP-01, MAT-07 · PLC-24 · PLC-29 ⟵ GAP-01 · **M3** PLC-18 · PLC-08 · PLC-09 · PLC-10 · PLC-11 · PLC-12 · PLC-28 ⟵ RTE-25 · PLC-21 · PLC-14 ⟵ MAT-07 · **M4** PLC-16 · PLC-15 ⟵ RTE-25 · PLC-17 · **M6** PLC-20 · PLC-22 ⟵ REL-07 · PLC-27 · PLC-25 ⟵ EXT-28
- **routing** (24): **M2** RTE-10 · RTE-11 · RTE-12 · RTE-13 · RTE-25 · RTE-26 (slice) · **M3** RTE-14 · RTE-16 ⟵ GAP-01, EXT-12 · RTE-18 · RTE-15 · RTE-17 · RTE-19 · RTE-20 · RTE-24 · **M4** RTE-26 · RTE-21 · RTE-22 · RTE-23 · **M6** RTE-28 · RTE-29 · RTE-30 · RTE-31 · RTE-27 ⟵ REL-12 · GAP-12 ⟵ REL-12
- **reliability** (12): **M2** GAP-06 · **M3** REL-10 · REL-04 · REL-05 · REL-14 · REL-07 · **M6** REL-16 ⟵ EXT-23, GAP-03 · REL-12 · REL-13 · REL-15 · REL-17 · GAP-13
- **perf** (22): **M2** PERF-10 · PERF-17 · PERF-20 (slice) ⟵ FLOW-09 · **M3** GAP-17 · PERF-18 · PERF-19 ⟵ FLOW-12 · PERF-16 · **M4** PERF-11 · PERF-13 · PERF-12 ⟵ EXT-17 · PERF-14 · PERF-15 ⟵ FLOW-08 · PERF-26 ⟵ CELL-19 · PERF-29 · GAP-08 ⟵ FLOW-08, RTE-21 · **M5** PERF-20 ⟵ FLOW-09, FLOW-12 · PERF-22 · PERF-23 ⟵ GAP-01, MAT-07, MAT-13 · PERF-21 ⟵ FLOW-12 · PERF-24 · **M6** PERF-27 · PERF-28 ⟵ RTE-22
- **flow** (10): **M2** FLOW-06 · GAP-07 · FLOW-09 · FLOW-12 · FLOW-13 · **M3** FLOW-08 · FLOW-10 · **M6** FLOW-11 · FLOW-15 · GAP-16

## Cycles resolved

- **MAT-07 / EXT-16 / EXT-15 / EXT-12.** EXT-12 creates `Family`/`MatchKind` itself if they are missing, and gets
  `MatchClass` from GAP-01 (C1), so it does not wait on MAT-07. Per D1, EXT-16 depends on GAP-01 instead of MAT-07.
  MAT-07 does not wait on EXT-16, because every set is Moderate until classes are inferred. Result: GAP-01 → {EXT-12 →
  … → EXT-16, MAT-07}.
- **MAT-08 ↔ EXT-20.** The limit table moved to GAP-01 (D13). EXT-20 applies MAT-08's budget rule, so MAT-08 comes
  first and EXT-20 depends on it.
- **CELL-14 ↔ MAT-12.** MAT-12 is pure pattern code with golden tests and comes first. CELL-14 wires `segment_row` into
  the generator and deletes `greedy_centroid` (C27). MAT-12's drawn-array acceptance is step-level on CELL-14.
- **CELL-23 ↔ REL-13.** REL-13 owns the formula and the `res_tox_nm` key (§6.3). CELL-23 does its class floor first; the
  `W_heat` term is step-level on REL-13 (both M6).
- **EXT-17 ↔ PERF-12.** PERF-12 is a non-blocking producer for EXT-17 (all fields optional). PERF-12 hard-depends on
  EXT-17 (the `Sensitivities` type).
- **PERF-12 ↔ RTE-21.** RTE-21 only consumes PERF-12's output. Only PERF-12 step 2 writes RTE-21's `DetailedCfg`
  fields, so that step is step-level.
- **REL-05 → REL-14.** They share `self_rise_mc`, so REL-05 goes first and REL-14 depends on it. Only REL-05's steps
  4–5 need GAP-06 (M2, same module).
- **REL-16 ↔ CELL-22.** REL-16 provides the predicate and comes first. CELL-22 owns the `well_bridges` signature and
  hard-depends on REL-16.
- **Drawer/producer pairs.** In each pair the pure-function item comes first and the drawer or consumer depends on it:
  MAT-15 → CELL-15, MAT-18 → CELL-21, MAT-21 → PERF-27 (PERF-27 fills `sigma_rand_override`).
- **Inside annotator.** EXT-13 builds the tree before EXT-14, EXT-15 and EXT-19 add their edges. EXT-14 step 8
  (`set_pairs`) lands with EXT-15. Both are step-level.

## Forward dependencies (item needs something from a later milestone)

All are reclassified as not hard. Nothing had to move.

- GAP-02 (M3) → MAT-16 (M6): its text says "S(L) when present", and it uses the worst-case `svt` until then.
- RTE-14 (M3) → REL-12, REL-17 (M6): marked "optional" in RTE-14's text. Its step 7 uses RTE-21's `F_k` ordering
  when that lands (M4).
- PERF-16 (M3) → PERF-11 (M4): only refines the net ranking.
- PERF-10 (M2) step 3 → PERF-15 (M4): step-level; lands with PERF-15.
- EXT-17 (M3) → PERF-12 (M4): producer.
- PLC-08 (M3) → PLC-15 (M4): the halo is zero until PLC-15.
- FLOW-12 (M2) → EXT-26 (M3): only the `--constraints` flag, which exits 2 until EXT-26 lands.
- **Split items.** These two items appear in two milestones in the master plan:
  - RTE-26: M2 slice = cases a, b, c, e on sky130; full item in M4 = four decks plus case d, which needs RTE-15.
  - PERF-20: M2 slice = seeds, `results.json`, `stage_ms`, GDS-bbox area, needs FLOW-09; full item in M5 needs
    PERF-10, PERF-13 and FLOW-12.

## Unscheduled or cut items

None of these is a hard dependency, so none is scheduled:

- **REL-06** (cut): CELL-13 depends on FLOW-05 instead (D19, done in M0).
- **REL-08** (cut): CELL-17's test and `ring_gap` are GAP-05 (D20, M1). Injector input is GAP-03, read by REL-07.
- **REL-09** (cut): replaced by GAP-04 (M1) and EXT-23 (D3–D5, D25, D28).
- **PLC-19** (cut): replaced by REL-15.

## Other non-hard calls worth knowing

- **Runtime data, not code.** EXT-21 and EXT-25 do not wait on PERF-12/13. The annotator cannot import library
  `perf` code; it codes against EXT-17's `SpecSens`, and only acceptance on real data needs PERF.
- **Working fallbacks stated in the item text.** These items do not wait on the listed inputs:
  - REL-07 runs without EXT-23 or GAP-03 (Clock-class fallback).
  - RTE-15 falls back without PLC-28 or PLC-21; the T5 bench check is measured once they merge.
  - RTE-17 and RTE-19 run without EXT-24.
  - RTE-18 runs without EXT-24/18.
  - CELL-12, PLC-13 and PLC-29 run without EXT-16 (class defaults to Moderate).
  - MAT-09 runs without EXT-17, except step 3.
- **Kept hard (needed to start):**
  - Everything that reads `MatchClass` or `mos_env`/`resistor_env`/`limit` waits on GAP-01.
  - RTE-16 waits on EXT-12 (`Unitization.class`).
  - PERF-19 waits on FLOW-12 (labelled GDS for LVS).
  - PLC-28 and PLC-15 wait on RTE-25.
  - GAP-08 waits on FLOW-08 and RTE-21 (step 3).
- **Done in M0/M1, so left out:** every MAT-01…06, EXT-02…11, CELL-01…10/30, PLC-01…06, RTE-02…09/32, REL-01…03,
  PERF-01…09/30, FLOW-01…05/07/14/16/17 and GAP-04/05/10/20 reference.
