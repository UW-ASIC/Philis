# Handoff (updated 2026-10-03)

## State of main (2a09c30)
- M0 merged (24 items; m0-report.md). dac4 green after GPurify 6341f18, so M0 criterion 2 is met; RTE-06 not needed (kept on `m0-rte06-rejected`). RTE-03 kept (`m0-rte03-original` holds the original).
- fix-export merged: labelled GDS, `<top>_ref.spice`, `philis run -o --seed --max-iters --starts`, `--version`.
- GPurify 6341f18; `pdks/decks/*` re-vendored by 3-way merge (PHILIS edits kept).
- Suite: 444 passed, 1 failed (`hier_elaborate` → M1a FLOW-13 step 0), 7 ignored.
- strongarm: philis CLEAN, klayout 0, magic 0, netgen vs `_ref.spice` MATCH. vs user schematic: differs only by dummies (owner decision pending).
- M0 and fix-export worktrees removed (56 GB freed).

## Running: M1a (input + extraction), workflow run `wf_7dc71ab5-4f7` (first launch wf_b1fea248-4a9 deferred everything: cap measured session-wide; fixed)
- Script: ~/.claude/projects/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/workflows/scripts/philis-m1a-wf_b1fea248-4a9.js
- Branch `m1a` at ../philis-m1a/integrate; chains input / ext-core / ext-slot / ext-misc at ../philis-m1a/<chain> (branches m1a-<chain>).
- Soft cap: args.softCapOutputTokens = 1.5M output tokens; past it no new item starts (status `deferred-cap`); panel/plan-update skipped.
- Resume: Workflow({scriptPath, resumeFromRunId: "wf_7dc71ab5-4f7"}). Before resuming, check each chain worktree: if an impl died after committing, `git reset --hard` it to the last m1a merge it contains (see M0 notes).
- Next: M1b (MAT + CELL + FLOW-16), M1c (PLC + RTE). Commit to main between each.

## Tools
`source <scratchpad>/signoff-investigation/env.sh` (klayout 0.30.4, magic 8.3.573, netgen 1.5.292 from nix store; PDK_ROOT=Philis/.pdk (~/.volare no longer exists)). Independent checks: `python3 ResearchBoutros/analog/common/layout/verify.py drc|lvs <block> <gds>`; netgen vs `<top>_ref.spice`.

## Cleanup owed
- After M1a merges: remove ../philis-m1a worktrees + target/.
