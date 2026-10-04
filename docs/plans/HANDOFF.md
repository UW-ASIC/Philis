# Handoff (updated 2026-10-03)

## State of main (2a09c30)
- M0 merged (24 items; m0-report.md). dac4 green after GPurify 6341f18, so M0 criterion 2 is met; RTE-06 not needed (kept on `m0-rte06-rejected`). RTE-03 kept (`m0-rte03-original` holds the original).
- fix-export merged: labelled GDS, `<top>_ref.spice`, `philis run -o --seed --max-iters --starts`, `--version`.
- GPurify 6341f18; `pdks/decks/*` re-vendored by 3-way merge (PHILIS edits kept).
- Suite: 444 passed, 1 failed (`hier_elaborate` → M1a FLOW-13 step 0), 7 ignored.
- strongarm: philis CLEAN, klayout 0, magic 0, netgen vs `_ref.spice` MATCH. vs user schematic: differs only by dummies (owner decision pending).
- M0 and fix-export worktrees removed (56 GB freed).

## Running (overnight, autonomous): M1 resume + M1→main + M2–M6 by module — run `wf_2ab8e777-05e`
- One script: ~/.claude/projects/-home-omare-Documents-Projects-Rust-Philis/a69953d9-0528-4795-8632-2cf98996a622/workflows/scripts/philis-m1-modules-wf_2ab8e777-05e.js, args {"cap": 2500000, "cap2": 15000000} (output tokens, per phase).
- All new/rerun agents: Opus 5.5, effort medium (no sonnet). 45 completed M1 agents replay from cache (KEEP list in the script).
- Flow: M1 batches finish → verify → plan update → `merge-m1-main` (gate: no new failing test vs main's 1 known) → `setup-m2` (branch m2 from m1a, worktrees ../philis-m2/<module>, removes merged ../philis-m1a worktrees) → 8 module owners run M2–M6 items in dag-m2-m6.json order, waiting only on hard deps; per segment: harden card → exec → review → fix → serial merge into m2 → `final-m2` (report m2-m6-report.md; merge m2→main only if no new failing tests).
- Resume after a cut-off: same Workflow call with resumeFromRunId "wf_2ab8e777-05e" and the same args. Interrupted agents are told to finish existing commits.
- Backup of the pre-edit script: <scratchpad>/m1-script.bak.js.

## Tools
`source <scratchpad>/signoff-investigation/env.sh` (klayout 0.30.4, magic 8.3.573, netgen 1.5.292 from nix store; PDK_ROOT=Philis/.pdk (~/.volare no longer exists)). Independent checks: `python3 ResearchBoutros/analog/common/layout/verify.py drc|lvs <block> <gds>`; netgen vs `<top>_ref.spice`.

## Cleanup owed
- After M1a merges: remove ../philis-m1a worktrees + target/.
