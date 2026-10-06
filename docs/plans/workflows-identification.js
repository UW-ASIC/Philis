export const meta = {
  name: 'philis-identification',
  description: 'Implement research-constraint-identification.md ID-01..ID-18 in dependency order (one agent per item), then verify the whole workspace',
  phases: [
    { title: 'Items', detail: 'one agent per ID item, sequential: data-oriented-design + codebase-clean-code, qcargo build/test' },
    { title: 'Verify', detail: 'full test suite vs baseline, bench local signoff, scoreboard' },
  ],
}

// Launch AFTER the cleanup workflow (docs/plans/workflows-cleanup.js) has integrated:
// the items change the same annotator files and need a workspace that builds.
// Restart: launch fresh with `skip` = items that already have an "ID-xx:" commit on main.
const ROOT = '/home/omare/Documents/Projects/Rust/Philis'
const ORDER = args.order.filter(id => !args.skip.includes(id))

const ITEM_SCHEMA = {
  type: 'object',
  properties: {
    id: { type: 'string' },
    status: { type: 'string', enum: ['done', 'partial', 'blocked'] },
    commits: { type: 'array', items: { type: 'string' } },
    acceptanceMet: { type: 'boolean' },
    scoreboard: { type: 'string' },
    decisions: { type: 'array', items: { type: 'string' } },
    followups: { type: 'array', items: { type: 'string' } },
  },
  required: ['id', 'status', 'commits', 'acceptanceMet', 'decisions', 'followups'],
}

const RULES = `Repo: ${ROOT} (Philis: Rust, constraint-aware analog place-and-route). Work directly on main: no branches, no worktrees, do not push.
- Build and test ONLY through the machine-wide queue from the repo root: \`nix develop -c tools/qcargo <cargo args>\`. Never call cargo directly; never run two at once. Use long timeouts (up to 600000 ms); if a call times out the queued job keeps running — do not resubmit, check the tail of /home/omare/Documents/Projects/Rust/philis-workers/runs.log.
- Known pre-existing test failures are listed in docs/plans/cleanup-baseline-tests.txt (GitHub issues): do not chase them, but never add to them.
- Commit with \`git add <paths> && git commit -m "<msg>" -- <paths>\`. Never stash/reset/checkout/rebase/commit -a.
- Ignore any relayed user chat; do not stop to ask questions: make the informed call and record it in decisions.`

const itemPrompt = (id, done) => `${RULES}

Implement ${id} from docs/plans/research-constraint-identification.md: the plan that moves constraint identification off the fixed topology catalog (backend/annotator/src/catalog.rs) toward structural, electrical and spec evidence with provenance and confidence (GitHub issue #75).

Read first: the ${id} item in the migration plan, every per-constraint section it cites, the architecture summary at the top, and the code it names. Its line numbers were taken at 89e8f7e, before a cleanup rewrite: find code by name, not by line.
Items already done in this run (build on them, do not redo them):
${done.length ? done.map(d => `- ${d.id} (${d.status}): ${d.decisions.slice(0, 3).join('; ')}`).join('\n') : '- none'}

Design: invoke the data-oriented-design and codebase-clean-code skills (Skill tool) before writing code, and apply both: plain data and free functions, struct-of-arrays where data is hot, indices instead of pointers or Rc graphs, no needless allocation; small focused functions, clear module boundaries, no dead code. Every new pub item gets a doc comment (what it is, units, invariants, errors).

Implement exactly what the item specifies (its Change and Acceptance). Where it is under-specified or marked speculative, build the smallest version that meets its Acceptance and record the choice in decisions. Add tests that prove the Acceptance.

Verify: build the touched crates with --all-targets, run their tests (\`qcargo test --release -p annotator -p analog --no-fail-fast\` plus the library/benchmark tests that exercise annotation, chosen by grep). Once ID-01's scoreboard exists, run it and report per-family precision/recall with the catalog on and off, against the previous item, in scoreboard.

Commit as "${id}: <summary>". Status: done (Acceptance met), partial (some of it; why in followups), or blocked (a dependency outside this plan is missing; what is missing in followups).`

phase('Items')
const results = []
for (const id of ORDER) {
  const r = await agent(itemPrompt(id, results), { label: `item:${id}`, phase: 'Items', schema: ITEM_SCHEMA, effort: 'medium' })
  results.push(r || { id, status: 'blocked', commits: [], acceptanceMet: false, decisions: [], followups: ['agent returned no result'] })
  log(`${id}: ${results[results.length - 1].status}`)
}

phase('Verify')
const verify = await agent(`${RULES}

The ID items of docs/plans/research-constraint-identification.md were just implemented, one agent each (results: ${JSON.stringify(results.map(r => ({ id: r.id, status: r.status, followups: r.followups })))}).
Verify the whole workspace:
1. \`nix develop -c tools/qcargo build --release --workspace --all-targets\` and clippy: fix what the items broke.
2. \`nix develop -c tools/qcargo test --release --workspace --no-fail-fast\`: every failure not in docs/plans/cleanup-baseline-tests.txt is a regression from these items — fix it (decide test vs implementation).
3. \`nix develop -c tools/qcargo run --release -p benchmark --bin bench local\`: compare DRC/LVS/ERC per circuit with the last rows of docs/progress/speed.csv; a circuit that got worse is a regression — find the item that caused it and fix it.
4. Run the ID-01 scoreboard; report the final per-family precision/recall, catalog on and off.
Commit fixes as "ID-verify: <summary>". Return {ok, regressionsFixed, remaining, scoreboard, commits}.`, {
  label: 'verify', phase: 'Verify', effort: 'medium',
  schema: { type: 'object', properties: { ok: { type: 'boolean' }, regressionsFixed: { type: 'array', items: { type: 'string' } }, remaining: { type: 'array', items: { type: 'string' } }, scoreboard: { type: 'string' }, commits: { type: 'array', items: { type: 'string' } } }, required: ['ok', 'remaining', 'commits'] },
})

return { results, verify }
