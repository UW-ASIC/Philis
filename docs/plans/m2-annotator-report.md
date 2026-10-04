# M2+ annotator: segment report (EXT-12..16, EXT-19)

## Bench (`bench local`, seed 1, sky130, release, deterministic over 2 reruns)
Merge-base `c867abb` built from `git archive`, against review fixes 1. The EXT-19 head before the fixes (`7da4586`) is
from the review run.

| circuit | c867abb WL nm / area µm² / LVS | 7da4586 | review fixes 1 |
|---|---|---|---|
| dac4 | 468720 / 1633.1 / PARTIAL(16) | identical | identical |
| pair | 19025 / 8.6 / MATCH | identical | identical |
| quad | 69470 / 33.7 / MATCH | identical | identical |
| ota | 640430 / 2164.6 / MATCH | identical | identical |
| mirror_ratio | 177175 / 176.9 / MATCH | 159100 / 107.3 / MATCH | 159100 / 107.3 / MATCH |
| bgr_core | 340620 / 422.7 / PARTIAL(9) | 349285 / 422.7 / hard Symmetry XQ1/XQ2 | 345360 / 422.7 / PARTIAL(9) |

DRC 0 on every row.
- mirror_ratio: intended (EXT-15). The 1:2:4 mirror is one set and one Unitization, so area is 39 % lower and WL 10 % lower.
- bgr_core: EXT-19's `bjt_ratioed_pair` is a CurrentMirror leaf, and `emit.rs` turned it into a hard `Symmetry`
  (corpus `axes` 0 → 1). That change was not intended. A 1:8 ratioed pair is a centroid array (its Unitization), not a
  mirror image. Review fixes 1 emits it as matched only, with no Symmetry, and the bgr_core/brokaw corpus rows go back
  to their earlier `axes`. A residual of WL +4740 nm (+1.4 %) against `c867abb` remains, with the same area, DRC and
  LVS. Its cause was not isolated, so the T9 gate ("bgr_core unchanged") holds on area/DRC/LVS but not on WL.

## Review fixes 1
- Scale: debug 5.0 s → 1.0 s, release 437 → 121 ms. Profiling showed that the cost was the per-set linear scans
  (`matched_sets` origin lookups, roles, passive role, `set_pairs` sets², Perfect compounds×sets). It was not the
  `iter_mut().find` grouping, which accounted for no measurable time. Both are now indexed: `sets::group` (hashed) and
  `sets::device_index`/`inside`.
- Card D-l records the EXT-15 step 3 departure (schematic fingers, mixed-W/L sets skipped) as a CELL/FLOW follow-up.
- New tests: `per_set_unitization_follows_class` (folded {M7,M8} Minimal/Adjacent with no dummies; dac4 Ratio set below
  Moderate has no route matching), `exceptional_voltage_compound_is_perfect`, `sets::tests::set_pairs_maps_set_onto_set`.
- Open (EXT-14 step 8, spec): `annotate` cannot produce a non-empty `set_pairs`. Every compound couple is a `MatchSym`
  edge (`graph.rs:54`), so both mates land in one set and each set maps onto itself. The unit test covers
  `set_pairs`, but removing its call in `lib.rs` is still not caught. Owner: plan-01 EXT-14/EXT-20.

## EXT-20 (segment 2)
`bench local`, seed 1, sky130, release. Base is `1b1d020` (built from `git archive`), against EXT-20. The table
shows WL nm / area µm² / LVS. DRC is 0 on every row in both runs.

| circuit | 1b1d020 | EXT-20 |
|---|---|---|
| dac4 | 242275 / 1098.3 / PARTIAL(16) | identical |
| dac4_mim | 265465 / 3041.5 / MATCH | 263800 / 2803.9 / MATCH |
| mirror_ratio | 138590 / 51.2 / MATCH | 137725 / 51.2 / MATCH |
| bgr_core, pair, quad, ota, ota_constrained, tt_ota, bjt_mirror, chain4, rc_filter, res_m2, tq_chain | — | identical |

- T9: DRC is 0 on every row. LVS is MATCH on bgr_core, pair, quad and ota. dac4's PARTIAL(16) was already there at
  the base and is unchanged, so EXT-20 did not cause it, but it also means T9's "LVS MATCH" for dac4 is not met.
- dac4_mim: its capacitor bank is now a priced `MatchedSet` (an unknown ledger, pulled only by cost), which gives
  8 % less area. mirror_ratio: the mirror's pairwise Proximity is gone (it is one Minimal set pulled to its
  reference), and WL drops by 0.6 %.
- Departure from the card: a compound couple whose set has no unit (unitization failed or no unit deck) counts as
  equal when its drawn geometry `(w_finger, l, fingers, model)` matches. It does not drop out. Without this, the
  5T OTA under a default config lost its pair and load Symmetry (`a_differential_stage_is_symmetric_about_one_axis`).
- Departure from the card: in `align_gold`, T1 checks pairs, selfs and axes on `canon`, which reads the emitted
  Symmetry. Net pairs stay on `canon_intent`, because the emitted net pairs are routing's Differential, which comes
  only from DiffPair leaves and has no `(vin, vip)`.
