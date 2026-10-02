# Handoff (2026-10-01, usage limit)

## Branches / worktrees
- `main` (c3a1303): plans in docs/plans (00-MASTER-PLAN.md first), field reports 01/02 (02 has root cause: TinyTapeout ran an old packaged build).
- `fix-export` (73087b6), worktree `../philis-fix-export`: labelled GDS + named top cell, `<top>_ref.spice`, CLI `run -o --seed --max-iters --starts`, `--version`, GPurify bde681c. Full suite green (dac4 ERC fixed by GPurify). Strongarm: klayout 0, magic 0, netgen vs ref.spice MATCH.
  TODO: `cargo update -p gpurify` to **6341f18** (LVS pairs symmetric devices by params; fixes Philis's 4 false lvs.parameter_mismatch on strongarm), rerun suite, merge into main.
- `m0` + worktrees `../philis-m0/*`: M0 workflow (run wf_a7f209ae-cfe, script in ~/.claude/.../workflows/scripts/philis-m0-wf_a7f209ae-cfe.js). Resumed once; check `git -C ../philis-m0/integrate log --oneline eba9954..m0` for merged items. Resume with Workflow(scriptPath, resumeFromRunId) — first reset any chain worktree whose impl died after committing (as done for `size`).
  GAP-20 (doc) rejected on 2 minor points: fix by hand and merge.
  m0 branches from eba9954 (before GPurify bde681c): merging fix-export first then m0 may need the 4-terminal MOS test fix again.

## Open
- External vs user schematic still differs by dummy transistors (14 vs 9 in netgen). Decision for owner: keep dummies in `<top>_ref.spice` (current), or make dummies fully shorted / add them to schematic.
- `--interface` (fixed die + boundary pins) is a new feature, not implemented.
- GPurify: greedy (not Hungarian) symmetry pairing — ponytail note in refine.rs.

## Cleanup owed (user asked)
- When done: remove `../philis-m0/*/target`, `../philis-fix-export/target`, then `git worktree remove` them; scratchpad copies.
- GPurify session already cleaned its own scratch build.

## Tools
klayout/magic/netgen in nix store; `source <scratchpad>/signoff-investigation/env.sh`; checks via ResearchBoutros/analog/common/layout/verify.py {drc|lvs} <block> <gds>.
