# Handoff (updated 2026-10-06)

## M1 outcome
- m1a (c2940c6) merged into main as f50ef00 "Merge M1 (P0 correctness)". Gate: m1a release suite 592 passed, 0 failed, 2 ignored (`hier_elaborate` fixed by FLOW-13). Main release build OK after merge.
- Items not done: PLC-06 (waits on owner decision). m1-report.md verdict: M1 exit criteria 5/8 met; MAT ±5 % band, CELL `mirror_ratio` single cell, field report 01 (tq_chain 1 DRC row, strongarm unmeasured) not met; OTA-class fixtures regressed vs M0 (WL +21.8 %, overuse 0→1458).
- The M2–M6 module run starts from m1a (branch m2 is cut from m1a).

## State of main (2a09c30)
- M0 merged (24 items; m0-report.md). dac4 green after GPurify 6341f18, so M0 criterion 2 is met; RTE-06 not needed (kept on `m0-rte06-rejected`). RTE-03 kept (`m0-rte03-original` holds the original).
- fix-export merged: labelled GDS, `<top>_ref.spice`, `philis run -o --seed --max-iters --starts`, `--version`.
- GPurify 6341f18; `pdks/decks/*` re-vendored by 3-way merge (PHILIS edits kept).
- Suite: 444 passed, 1 failed (`hier_elaborate` → M1a FLOW-13 step 0), 7 ignored.
- strongarm: philis CLEAN, klayout 0, magic 0, netgen vs `_ref.spice` MATCH. vs user schematic: differs only by dummies (owner decision pending).
- M0 and fix-export worktrees removed (56 GB freed).

## M2–M6 outcome (2026-10-06)
- Run stopped at 93/140 items. All WIP module branches merged into main (no other branches left, local or on GitHub). Unreviewed WIP commits are marked "WIP (M2–M6 run stopped)".
- Remaining 47 items + known bugs = GitHub issues (labels enhancement / bug).
- Deleted branch SHAs, for recovery: m0-rte03-original ae99602, m0-rte06-rejected f6ba021, experimental 01c25bf (also `refs/backup/experimental`).
- RTE-28 uncommitted WIP: docs/plans/wip/rte28-wip.patch.

## Parked: cleanup workflow (stopped 2026-10-06 by owner)
- All 26 units finished (step 1–3 each) and 4 of 10 seams (core-types, analog-annotator, annotator-library, cells-cellgen); none of it was ever compiled.
- Parked by one revert commit (9208782): main is the pre-cleanup code (3ed23a0) again, which builds; baseline: docs/plans/cleanup-baseline-tests.txt.
- Resume: `git revert 9208782`, `git apply docs/plans/wip/cleanup-seams-inflight.patch` (dp anneal/lib/locks, analog orientation: a seam cut off mid-edit), then launch docs/plans/workflows-cleanup.js fresh with skipUnits = all 26, skipSeams = the 4 above. Its integration agents do the first build.
- Conflict risk: work landed on the pre-cleanup code meanwhile (e.g. the identification items, the asap7/field-report fixes) will conflict with the revert-of-revert in the same files.

## Queued: constraint identification (main builds again; launch when the owner says)
- `docs/plans/workflows-identification.js`: ID-01..ID-18 of docs/plans/research-constraint-identification.md (#75), one agent per item, sequential, then a verify agent.
- args: {"order":["ID-02","ID-01","ID-07","ID-03","ID-04","ID-06","ID-09","ID-08","ID-05","ID-15","ID-10","ID-12","ID-13","ID-11","ID-14","ID-16","ID-17","ID-18"],"skip":[]}
- Restart: fresh launch, `skip` = items with an "ID-xx:" commit on main.

## Tools
`source <scratchpad>/signoff-investigation/env.sh` (klayout 0.30.4, magic 8.3.573, netgen 1.5.292 from nix store; PDK_ROOT=Philis/.pdk (~/.volare no longer exists)). Independent checks: `python3 ResearchBoutros/analog/common/layout/verify.py drc|lvs <block> <gds>`; netgen vs `<top>_ref.spice`.
