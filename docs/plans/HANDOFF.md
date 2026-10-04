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

## STOPPED by owner (2026-10-04): M2–M6 module run, workflow run `wf_2ab8e777-05e`
- M1: merged to main (f50ef00). M2–M6: 64/140 items merged into branch `m2` (../philis-m2/integrate), 84 implemented; 247 agents done.
  Per module merged/total: reliability 12/12, analog-matching 15/16, annotator 14/19, perf 8/21, routing 7/23, cells 5/19, flow 2/10, placement 1/20. Not done so far: MAT-18 (deferred), CELL-11, PLC-07.
- At stop: uncommitted WIP saved to docs/plans/wip-patches/m2-{flow,integrate,reliability}.patch. flow and reliability edits left in their worktrees (resumed agents finish them); the integrate (m2) partial dr fix was reset to keep m2 clean (its patch is kept).
- History rewritten 2026-10-04 (Claude co-author trailers removed from all branches/tags; trees identical; main force-pushed to origin at a376052). Old SHAs quoted in the workflow script/journal (e.g. 2a09c30, m1a base) live on under refs/original/ and in /tmp/claude-1000/philis-before-rewrite.bundle — keep refs/original until the M2–M6 run has finished.
- RESUME (one call, same args): Workflow({scriptPath: "~/.claude/projects/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/workflows/scripts/philis-m1-modules-wf_2ab8e777-05e.js", resumeFromRunId: "wf_2ab8e777-05e", args: {"cap": 2500000, "cap2": 15000000}}).
  Finished agents replay from the journal; agents cut off mid-step rerun (Opus 5.5, effort medium) and are told to finish any commits they left. Module worktrees ../philis-m2/<module> keep their branches m2-<module>.
- Ends with `final-m2`: report docs/plans/m2-m6-report.md, merge m2 → main only if no new failing tests.
- Side branch `progress-charts` (worktree ../philis-snap): records every epoch's candidate for the README feedback chart; merge into main after m2 lands.
- Known regressions seen in snapshots on m2: dac4 DRC 6 / ERC 1 (0/20 clean candidates), tq_chain DRC 18; runtime up vs M0 (dac4 279 s vs 19 s). Check after landing.
- Disk: ../philis-m2/*/target/debug can be deleted when no cargo runs there.
- Planned follow-up: significance threshold for convergence (an improvement resets the stall counter only if > ~0.5% in C/area or any drop in |V|/spec miss), after the run (flow module edits that loop).

## Tools
`source <scratchpad>/signoff-investigation/env.sh` (klayout 0.30.4, magic 8.3.573, netgen 1.5.292 from nix store; PDK_ROOT=Philis/.pdk (~/.volare no longer exists)). Independent checks: `python3 ResearchBoutros/analog/common/layout/verify.py drc|lvs <block> <gds>`; netgen vs `<top>_ref.spice`.

## Cleanup owed
- After M1a merges: remove ../philis-m1a worktrees + target/.
