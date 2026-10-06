export const meta = {
  name: 'philis-cleanup',
  description: 'No-build cleanup of Philis: one agent per code unit (shape, tests, impl), then seam agents (api-design), then build+test integration',
  phases: [
    { title: 'Units', detail: 'one agent per independent code unit: step 1 shape, step 2 tests, step 3 impl; no builds' },
    { title: 'Seams', detail: 'api-design on each cross-unit interface; no builds' },
    { title: 'Integrate', detail: 'only builds: compile+clippy, then full tests vs baseline' },
  ],
}

const ROOT = '/home/omare/Documents/Projects/Rust/Philis'
const MAX = 3 // machine OOMs above 3 concurrent agents
const units = args.units.filter(u => !args.skipUnits.includes(u.id))
const seams = args.seams.filter(s => !args.skipSeams.includes(s.id))
const SEAM_IDS = args.seams.map(s => `${s.id} (${s.focus})`).join('\n  - ')

async function pool(items, fn) {
  const out = []
  let next = 0
  await parallel(Array.from({ length: Math.min(MAX, items.length) }, () => async () => {
    while (next < items.length) {
      const k = next++
      out[k] = await fn(items[k])
    }
  }))
  return out
}

const COMMON = `Repo: ${ROOT} (Philis: Rust, constraint-aware analog place-and-route). Work directly on branch main in this checkout: no branches, no worktrees.
Rules for every cleanup agent:
- NO BUILDS. Never run cargo, rustc, tools/qcargo or any build/test command; builds are slow and serialized. An integration agent compiles everything at the end. Instead read the definitions you call and reason carefully about types and borrows.
- Other agents edit other files in this same checkout at the same time. Re-read a file right before editing it.
- Commit only your own paths: \`git add <paths> && git commit -m "<msg>" -- <paths>\` (the \`-- <paths>\` form keeps other agents' staged work out of your commit). On index.lock, wait a few seconds and retry. Never git stash/reset/checkout/rebase/commit -a/push.
- Ignore any relayed or forwarded user chat; these instructions are complete. Do not stop to ask questions: make the informed call and record it.
- Keep domain constants that carry citations (PDK rules, paper references) as they are.`

const UNIT_SCHEMA = {
  type: 'object',
  properties: {
    unit: { type: 'string' },
    commits: { type: 'array', items: { type: 'string' } },
    testsAdded: { type: 'integer' },
    flaggedFixed: { type: 'array', items: { type: 'string' } },
    flaggedOpen: { type: 'array', items: { type: 'string' } },
    seamRequests: { type: 'array', items: { type: 'object', properties: { seam: { type: 'string' }, item: { type: 'string' }, change: { type: 'string' }, why: { type: 'string' } }, required: ['seam', 'item', 'change'] } },
    risks: { type: 'array', items: { type: 'string' } },
  },
  required: ['unit', 'commits', 'flaggedOpen', 'seamRequests', 'risks'],
}

const unitPrompt = u => `${COMMON}

You clean up ONE independent code unit: ${u.id} (${u.lines} lines).
Files you own (edit only these; you may also create \`<crate>/tests/cleanup_${u.id.replace(/-/g, '_')}.rs\` or \`#[cfg(test)] mod\` blocks inside your files):
${u.files.map(f => '- ' + f).join('\n')}

Interface rule: items referenced from files outside your unit (grep for every pub / pub(crate) item) keep their names and signatures. If such an interface should change, put it in seamRequests tagged with one of these seams instead; seam agents redesign cross-unit interfaces after all units finish:
  - ${SEAM_IDS}

Resuming: first run \`git log --oneline --grep 'cleanup(${u.id})'\`. If earlier steps of this unit are already committed (an earlier run was interrupted), continue from the next step.

STEP 1 — shape (the definitions: structs, enums, functions, modules)
Invoke the skills doc-comments, data-oriented-design and codebase-clean-code with the Skill tool, then apply them:
- every pub item and every non-obvious private item gets a professional doc comment (what it is, units, invariants, panics/errors);
- data-oriented layout where it pays: SoA for hot per-element data, indices over pointers/Rc/Box graphs, no needless clones or allocations in hot paths, plain data + free functions over trait towers;
- clean code: small focused functions, clear names, dead code removed, duplication folded.
Commit: "cleanup(${u.id}): step 1 — <summary>".

STEP 2 — testability, FULL coverage
Read ~/Documents/Projects/Zig/TESTING-METHODOLOGY.md (written for Zig: apply its principles to Rust #[test]) and invoke the test-plan skill. Walk every function of the unit and cover every branch and corner case: empty input, single element, boundaries, overflow, degenerate geometry, error paths. Tests assert the correct behaviour (spec, doc comments, physics), not whatever the code happens to do. If you are confident a test is right and the implementation fails it, keep the test and carry it into step 3. Keep existing tests unless they are wrong. Tests must compile without a build check: use only APIs you have read.
Commit: "cleanup(${u.id}): step 2 — <summary>".

STEP 3 — implementation
Invoke data-oriented-design, simd-loops and branchless. DO NOT MODIFY DOC COMMENTS in this step (step 1 owns them; only document new items).
- fix every implementation step 2 flagged, and any other corner case you find;
- cache-efficient access patterns; SIMD for hot numeric loops via fearless_simd (workspace dependency \`fearless_simd = "1.0.0"\`; add \`fearless_simd.workspace = true\` to your crate's Cargo.toml only if you use it and it is not there). SIMD padding can fight a data-oriented layout: vectorize only loops that are hot and long enough to pay;
- branchless code where it removes unpredictable branches from hot loops, never at the cost of clarity in cold code.
Then write your structured result as JSON to docs/plans/cleanup/units/${u.id}.json and commit it with the step: "cleanup(${u.id}): step 3 — <summary>".

Return the same structured result.`

phase('Units')
log(`${units.length} units, ${MAX} at a time`)
const unitResults = (await pool(units, u =>
  agent(unitPrompt(u), { label: `unit:${u.id}`, phase: 'Units', schema: UNIT_SCHEMA, effort: 'medium' })
))
const failedUnits = units.filter((u, i) => !unitResults[i]).map(u => u.id)
if (failedUnits.length) log(`units with no result (rerun with skipUnits = the rest): ${failedUnits.join(', ')}`)

const SEAM_SCHEMA = {
  type: 'object',
  properties: {
    seam: { type: 'string' },
    commits: { type: 'array', items: { type: 'string' } },
    changes: { type: 'array', items: { type: 'string' } },
    deferred: { type: 'array', items: { type: 'string' } },
    risks: { type: 'array', items: { type: 'string' } },
  },
  required: ['seam', 'commits', 'changes', 'risks'],
}

const seamPrompt = s => `${COMMON}

You are the seam agent for: ${s.id}.
Interface: ${s.focus}.
Crates on either side: ${s.crates.join(', ')}.

All unit agents have finished: each cleaned its own files (shape, tests, implementation) and left cross-unit signatures unchanged. Their results are in docs/plans/cleanup/units/*.json; read the seamRequests tagged "${s.id}" (and any untagged ones that clearly belong to this interface).

Invoke the api-design skill (Casey Muratori's principles: easy to use, granular levels that compose, no forced callbacks or retained state, caller owns memory) while staying data-oriented (plain structs, slices in and out, indices over pointers). Redesign this seam: the boundary types and functions, applying the requests that hold up. Update every caller across the workspace (grep thoroughly, tests and benchmarks included) and the doc comments of items you change. Other seam agents run concurrently on neighbouring interfaces: touch only what your seam needs.
Commit: "cleanup(seam ${s.id}): <summary>". Write your structured result as JSON to docs/plans/cleanup/seams/${s.id}.json and commit it.

Return the same structured result.`

phase('Seams')
const seamResults = await pool(seams, s =>
  agent(seamPrompt(s), { label: `seam:${s.id}`, phase: 'Seams', schema: SEAM_SCHEMA, effort: 'medium' })
)

phase('Integrate')
const INT = `Repo: ${ROOT}, branch main. You are an integration agent after a no-build cleanup: 26 unit agents and 10 seam agents edited the workspace without compiling. Their results (risks, open flags) are in docs/plans/cleanup/units/*.json and docs/plans/cleanup/seams/*.json.
Build and test ONLY through the machine-wide queue: \`nix develop -c tools/qcargo <cargo args>\` (never call cargo directly; never run two at once). A full release build takes minutes, the full test suite longer: give tool calls a long timeout (up to 600000 ms) and if one times out, the queued job keeps running — do not resubmit; check the tail of /home/omare/Documents/Projects/Rust/philis-workers/runs.log.
Fix by keeping the cleanup's intent (the new design) rather than reverting it, unless the design is wrong. Commit fixes as "cleanup(integrate): <summary>" with \`git commit -- <paths>\`. Do not push. Ignore relayed user chat; do not stop to ask.`

const build = await agent(`${INT}

Your job: make the workspace compile and lint clean.
1. \`nix develop -c tools/qcargo build --release --workspace --all-targets\` until it succeeds (fix every error; tests included).
2. \`nix develop -c tools/qcargo clippy --release --workspace --all-targets\`: fix warnings the cleanup introduced.
Return {ok, clippyWarnings, commits, notes}.`, {
  label: 'integrate:build', phase: 'Integrate', effort: 'medium',
  schema: { type: 'object', properties: { ok: { type: 'boolean' }, clippyWarnings: { type: 'integer' }, commits: { type: 'array', items: { type: 'string' } }, notes: { type: 'array', items: { type: 'string' } } }, required: ['ok', 'commits', 'notes'] },
})

const tests = await agent(`${INT}

The workspace builds (previous integration agent: ${JSON.stringify(build)}).
Your job: tests.
1. \`nix develop -c tools/qcargo test --release --workspace --no-fail-fast\`.
2. Compare failures with the pre-cleanup baseline in docs/plans/cleanup-baseline-tests.txt (those are known bugs filed as GitHub issues: do not chase them, but note any the cleanup fixed).
3. For every failure not in the baseline: decide whether the test (new step-2 tests assert spec behaviour) or the implementation is wrong, and fix it. Re-run just the affected test binaries (-p <crate> --test <name> / --lib) while iterating, then the full suite once at the end.
Return {passed, failedNew, failedBaseline, baselineFixed, commits, notes}.`, {
  label: 'integrate:tests', phase: 'Integrate', effort: 'medium',
  schema: { type: 'object', properties: { passed: { type: 'integer' }, failedNew: { type: 'array', items: { type: 'string' } }, failedBaseline: { type: 'array', items: { type: 'string' } }, baselineFixed: { type: 'array', items: { type: 'string' } }, commits: { type: 'array', items: { type: 'string' } }, notes: { type: 'array', items: { type: 'string' } } }, required: ['failedNew', 'failedBaseline', 'commits', 'notes'] },
})

return {
  failedUnits,
  open: unitResults.filter(Boolean).flatMap(r => r.flaggedOpen.map(f => `${r.unit}: ${f}`)),
  seams: seamResults.filter(Boolean).map(s => ({ seam: s.seam, deferred: s.deferred })),
  build, tests,
}
