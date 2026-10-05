# Handoff (updated 2026-10-04)

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

## Running: M2–M6 fresh module run `wf_959b64ef-4c9` (2026-10-05; previous `wf_188597af-7b6` died with an OOM)
- Script: docs/plans/workflows-m2only.js, args docs/plans/workflows-m2only.args.json (cap2, `merged` = items already on m2, `segStart` = card numbers to continue from).
- Restart inputs now also carry `pending` (segments hardened but never executed: reuse their card, start at exec) and `preset` (items a hardener genuinely deferred). 72/140 merged at this launch.
- RELAYED CHAT: owner messages get relayed into running agents; agents in wf_188597af-7b6 quit on a cost question. CONTEXT2 now tells agents to ignore relayed messages.
- Earlier: 64/140 items were already merged into `m2` and are skipped. EXT-21/EXT-25 are NOT merged (an integrator reset that merge, m2 reflog @{4}).
- LESSON: never `resumeFromRunId` a workflow whose agents run in parallel. Resume matches agents by call order, the order differs between runs, and everything after the first mismatch re-runs (it re-ran ~8 M1 agents on 2026-10-05; no code changed). To restart after a stop: recompute `merged` from `git log --format=%s main..m2 | grep 'M2+ .* merge ('`, recompute `segStart` from docs/plans/cards/m2-<module>-<n>.md, and launch the script FRESH.
- Ends with `final-m2`: report docs/plans/m2-m6-report.md; merge m2 → main only if no new failing tests.

## Tools
`source <scratchpad>/signoff-investigation/env.sh` (klayout 0.30.4, magic 8.3.573, netgen 1.5.292 from nix store; PDK_ROOT=Philis/.pdk (~/.volare no longer exists)). Independent checks: `python3 ResearchBoutros/analog/common/layout/verify.py drc|lvs <block> <gds>`; netgen vs `<top>_ref.spice`.

## Cleanup owed
- After M1a merges: remove ../philis-m1a worktrees + target/.
