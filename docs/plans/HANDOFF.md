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

## Running: M2–M6 module run `wf_83022c3c-20f` (2026-10-05, fifth launch; earlier wf_188597af-7b6, wf_959b64ef-4c9, wf_03e972cf-7f3, wf_f9fc79a5-541 died: OOM / session end)
- BUILD QUEUE (long-term-garbage-collection skill): every agent cargo command goes through `tools/qcargo` — two machine-wide flock queues (build: snapshot+compile, -j 6; test: running tests/programs, 2 threads) with per-worker reservation, four persistent build workers in ../philis-workers/w{1..4} (affinity: a worker sticks to its source tree; incremental on for release too) (snapshot of the caller's worktree via nix rsync, GC-rooted), -j 6 (measured: 8.0 GB peak, 1m05s full release build) and 2 test threads; collector keeps each worker target <= 15 GB; runs logged in ../philis-workers/runs.log (wait / compile / testwait / run seconds); memory sampled in ../philis-workers/mem.log (time, used MB, rustc count, test processes). Idle collection: `tools/qcargo --gc` hourly. Per-worktree target/ dirs were collected (they are garbage now).
- At most 3 agents at once (`maxActive`).
- RESTART (never resumeFromRunId — parallel modules break order-matched replay):
  1. `python3 docs/plans/restart_args.py <journal of every M2 run, oldest first> > args.json` (then add "maxActive": 3)
     (journals: ~/.claude/projects/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/subagents/workflows/<run>/journal.jsonl)
  2. Launch docs/plans/workflows-m2only.js FRESH with args.json. Each segment starts at the step after the last one that finished (exec / review / fix / merge); merged items are skipped; agents interrupted mid-step re-run and finish their own leftover commits.
- Agents are told to ignore relayed chat (a cost question once made exec agents quit).

## Tools
`source <scratchpad>/signoff-investigation/env.sh` (klayout 0.30.4, magic 8.3.573, netgen 1.5.292 from nix store; PDK_ROOT=Philis/.pdk (~/.volare no longer exists)). Independent checks: `python3 ResearchBoutros/analog/common/layout/verify.py drc|lvs <block> <gds>`; netgen vs `<top>_ref.spice`.

## Cleanup owed
- After M1a merges: remove ../philis-m1a worktrees + target/.
