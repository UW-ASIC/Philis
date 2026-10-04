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

## EXT-23/24/26, GAP-03, EXT-17/18 (segment 3) and review fixes 3
`bench local`, seed 1, sky130, release. The table shows WL nm / routing overuse / area µm². DRC is 0 and LVS is
MATCH on every row. The columns are `75f5612` (before EXT-24), `db0e47a` (EXT-24) and HEAD (EXT-26 plus these
fixes). Each column was built from `git archive`.

| circuit | 75f5612 | db0e47a (ota only) | HEAD |
|---|---|---|---|
| ota | 645550 / 3043 / 1831.6 | 658830 / 5875 / 1983.9 | 658830 / 5875 / 1983.9 |
| ota_constrained, tt_ota | same as ota | — | same as ota |

- EXT-24 is the whole change. Each OTA now gets two `Differential` pairs plus Voltage-set G×D
  `CrosstalkExclusion`, so overuse grows 93 %, WL 2.1 % and area 8.3 %. EXT-26 and these fixes leave the OTA rows
  unchanged. This adds to the open OTA-class regression in the master Status, and it is not fixed here.
- `perf_postlayout::ota_probe_regions` (EXT-17 acceptance) stays red, as the card requires. The probe bench puts
  the pair and the tail in triode. Headroom: XM1/XM2 −264.8 mV, XM5 −297.8 mV, XM3/XM4 +1751 mV. The bench is
  not changed.
- Review fixes 3:
  - `substrate::tag` now reads its sets through `sets::device_index`. The 12.5k scale fixture `annotate` runs in
    0.27 s release (was 0.40 s) and 5.0 s debug (was 6.8 s; merge-base 3.25 s).
  - The CurrentSource-gate → Bias pass skips nets with `User` evidence.
  - Stars: a port is the feed and gives no device root. With an op that covers the node, a star needs
    Σ|I_others| > 0.01·Σ|I_members|.
  - Sidecar `GroupBlocks` batches carry `Origin::User { index: entry }`. `cfg.groups` is now
    `(entry, members)`.
  - The CLI test now checks that a malformed sidecar exits 2 from the parser.
- Out of scope (reported):
  - `emit.rs:101`, `elaborate.rs:170` and `benchmarks/src/bench.rs:236` re-annotate without evidence or the
    sidecar.
  - PERF-09 produces no `gds`/`vth`/`gmb`, so EXT-18's `z_ohm` and the vth rule do nothing inside the flow.
  - At the merge-base, `cargo test -p library`/`-p annotator` already fail in debug:
    - the `gp::Prices` hard/budget kind-conflict `debug_assert` (`backend/gp/src/lib.rs:87`);
    - `scale.rs`'s 2 s limit (still 5.0 s debug here; release passes).
  - So the card's per-commit rule cannot be met by this module alone. The card's `-p cli` should be `-p philis`.

## EXT-21/25 (segment 5)

- EXT-21's real-data acceptance and EXT-25 step L are pending: both read sensitivities from PERF-12
  (`perf::to_evidence`), which is not in m2. Only the unit/fixture tests cover them here.
- `perf_postlayout` (release, run in review 5): 17 pass, 1 fails, the already-documented `ota_probe_regions`.
- Drain-only nets lose their parasitic budget until a sidecar `Load` states it (AA-25). No metric was
  measured against the base for this change, so any moved metric is unknown.
- Review fixes 5: a set no spec's `d_vt` touches now gets no weight (was `Some(0.0)`, which D5 then demoted
  to Minimal); `ext21_minor_weight_and_allowance_set_class` covers D5 and the allowance-set 6σ class (DP
  allowance 1 mV, not 2.5: a 15 mV target is itself Minimal, so it could not tell D5 apart).
