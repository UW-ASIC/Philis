export const meta = {
  name: 'philis-m2-m6-modules',
  description: 'Philis M2–M6 by module (fresh run): skips items already merged into m2, harden→exec→review→fix→merge per segment, then verify and land',
  phases: [
    { title: 'Harden' }, { title: 'Execute' }, { title: 'Review' }, { title: 'Integrate' }, { title: 'Close-out' },
  ],
}
const ROOT = '/home/omare/Documents/Projects/Rust/Philis'
// Every agent: session model (Opus 5.5), effort medium. Agents are told earlier
// interrupted attempts may have left commits/edits behind.
const RESUME_NOTE = `\nRESUME NOTE: an earlier attempt of this step may have been interrupted after committing or editing part of the work. First check git log and git status in your worktree; verify and finish existing work rather than redoing it.`
function ag(prompt, opts) {
  const o = { ...opts, effort: 'medium' }
  delete o.model
  return agent(prompt + RESUME_NOTE, o)
}
const HARD = { type: 'object', properties: {
  card_path: { type: 'string' },
  items: { type: 'array', items: { type: 'object', properties: { id: { type: 'string' }, cls: { type: 'string', enum: ['mechanical', 'judgment', 'defer'] }, reason: { type: 'string' } }, required: ['id', 'cls', 'reason'] } },
}, required: ['card_path', 'items'] }
const EXEC = { type: 'object', properties: {
  results: { type: 'array', items: { type: 'object', properties: { id: { type: 'string' }, status: { type: 'string', enum: ['done', 'escalate', 'blocked'] }, note: { type: 'string' } }, required: ['id', 'status', 'note'] } },
  touched_crates: { type: 'array', items: { type: 'string' } },
}, required: ['results', 'touched_crates'] }
const REVIEW = { type: 'object', properties: { issues: { type: 'array', items: { type: 'string' } }, verdict: { type: 'string' } }, required: ['issues', 'verdict'] }
const INTEG = { type: 'object', properties: { status: { type: 'string', enum: ['merged', 'failed'] }, sha: { type: 'string' }, notes: { type: 'string' } }, required: ['status', 'sha', 'notes'] }

// ======================= M2–M6 by module =======================
const W2 = '/home/omare/Documents/Projects/Rust/philis-m2'
const INT2 = W2 + '/integrate'
const MODS = {"analog-matching":[["GAP-01","M2",[]],["MAT-07","M2",["GAP-01"]],["MAT-08","M2",["MAT-07","GAP-01"]],["MAT-13","M2",[]],["GAP-02","M3",["GAP-01"]],["MAT-09","M3",["MAT-07"]],["MAT-11","M3",[]],["MAT-12","M3",[]],["MAT-10","M3",["MAT-07"]],["MAT-14","M6",[]],["MAT-15","M6",[]],["MAT-16","M6",[]],["MAT-17","M6",[]],["MAT-18","M6",["MAT-11"]],["MAT-21","M6",["MAT-09"]],["GAP-18","M6",["MAT-11"]]],"annotator":[["EXT-12","M2",["GAP-01"]],["EXT-13","M2",["EXT-12"]],["EXT-14","M2",["EXT-12"]],["EXT-15","M2",["EXT-12","EXT-13","EXT-14"]],["EXT-16","M2",["EXT-15","GAP-01"]],["EXT-19","M2",["EXT-15"]],["EXT-20","M2",["EXT-14","EXT-15","EXT-16","MAT-07","MAT-08"]],["EXT-17","M3",["EXT-12"]],["EXT-18","M3",["EXT-17"]],["EXT-23","M3",["EXT-17","EXT-18","EXT-20"]],["GAP-03","M3",["EXT-12","EXT-18","EXT-23"]],["EXT-24","M3",["EXT-14","EXT-18","EXT-20","EXT-23"]],["EXT-26","M3",["EXT-14","EXT-16","EXT-18"]],["GAP-09","M3",["EXT-14","EXT-26"]],["EXT-21","M4",["EXT-17","EXT-20"]],["EXT-25","M4",["EXT-17","EXT-18","EXT-24"]],["EXT-27","M6",["EXT-13","EXT-14"]],["EXT-28","M6",["EXT-13","EXT-17"]],["EXT-29","M6",["EXT-16","EXT-17"]]],"cells":[["CELL-11","M2",[]],["CELL-12","M2",["CELL-11","GAP-01","EXT-12"]],["CELL-13","M2",[]],["CELL-19","M2",[]],["CELL-14","M3",["CELL-12","MAT-12","GAP-01","EXT-15"]],["CELL-16","M3",["GAP-07"]],["CELL-17","M3",[]],["CELL-18","M3",[]],["CELL-15","M6",["MAT-15"]],["CELL-21","M6",["MAT-18","EXT-19"]],["CELL-20","M6",["CELL-13","CELL-19"]],["CELL-22","M6",["REL-16"]],["CELL-23","M6",["CELL-12","GAP-01"]],["CELL-25","M6",[]],["CELL-29","M6",["CELL-12"]],["GAP-11","M6",["GAP-01","EXT-16"]],["GAP-14","M6",["EXT-26","CELL-17"]],["GAP-15","M6",["CELL-19"]],["GAP-19","M6",["EXT-18"]]],"placement":[["PLC-07","M2",[]],["PLC-13","M2",["PLC-07","GAP-01","MAT-07"]],["PLC-24","M2",["PLC-07"]],["PLC-29","M2",["GAP-01"]],["PLC-18","M3",[]],["PLC-08","M3",["PLC-07"]],["PLC-09","M3",["PLC-08","PLC-18"]],["PLC-10","M3",["PLC-08","PLC-09"]],["PLC-11","M3",["PLC-09"]],["PLC-12","M3",["PLC-08"]],["PLC-28","M3",["PLC-08","RTE-25"]],["PLC-21","M3",[]],["PLC-14","M3",["MAT-07"]],["PLC-16","M4",[]],["PLC-15","M4",["PLC-08","RTE-25"]],["PLC-17","M4",[]],["PLC-20","M6",["PLC-08"]],["PLC-22","M6",["PLC-08","REL-07"]],["PLC-27","M6",["PLC-08","PLC-10"]],["PLC-25","M6",["EXT-28","PLC-20"]]],"routing":[["RTE-10","M2",[]],["RTE-11","M2",["RTE-10"]],["RTE-12","M2",["RTE-10"]],["RTE-13","M2",[]],["RTE-25","M2",["RTE-10","RTE-13"]],["RTE-14","M3",["RTE-12"]],["RTE-16","M3",["GAP-01","EXT-12"]],["RTE-18","M3",["RTE-10"]],["RTE-15","M3",["RTE-10","RTE-12"]],["RTE-17","M3",["RTE-12","RTE-15"]],["RTE-19","M3",["RTE-12","RTE-18"]],["RTE-20","M3",["RTE-18"]],["RTE-24","M3",["RTE-14","RTE-15"]],["RTE-26","M4",["RTE-10","RTE-11","RTE-12","RTE-15"]],["RTE-21","M4",["RTE-18"]],["RTE-22","M4",["RTE-12","RTE-21"]],["RTE-23","M4",["RTE-12"]],["RTE-28","M6",["RTE-10"]],["RTE-29","M6",["RTE-15"]],["RTE-30","M6",["RTE-21","RTE-23"]],["RTE-31","M6",["RTE-21"]],["RTE-27","M6",["RTE-12","RTE-13","REL-12"]],["GAP-12","M6",["RTE-14","REL-12"]]],"reliability":[["GAP-06","M2",[]],["REL-10","M3",[]],["REL-04","M3",[]],["REL-05","M3",[]],["REL-14","M3",["REL-05"]],["REL-07","M3",[]],["REL-16","M6",["EXT-23","GAP-03"]],["REL-12","M6",[]],["REL-13","M6",[]],["REL-15","M6",[]],["REL-17","M6",[]],["GAP-13","M6",[]]],"perf":[["PERF-10","M2",[]],["PERF-17","M2",[]],["GAP-17","M3",[]],["PERF-18","M3",[]],["PERF-19","M3",["FLOW-12"]],["PERF-16","M3",["PERF-19"]],["PERF-11","M4",["PERF-10"]],["PERF-13","M4",["PERF-11"]],["PERF-12","M4",["PERF-11","PERF-13","EXT-17"]],["PERF-14","M4",["PERF-13"]],["PERF-15","M4",["PERF-10","PERF-11","PERF-12","FLOW-08"]],["PERF-26","M4",["CELL-19"]],["PERF-29","M4",["PERF-11","PERF-13","PERF-15"]],["GAP-08","M4",["FLOW-08","RTE-21"]],["PERF-20","M5",["PERF-10","PERF-13","FLOW-09","FLOW-12"]],["PERF-22","M5",["PERF-10","PERF-11","PERF-13","PERF-15"]],["PERF-23","M5",["PERF-16","GAP-01","MAT-07","MAT-13"]],["PERF-21","M5",["PERF-19","PERF-20","PERF-22","FLOW-12"]],["PERF-24","M5",["PERF-13","PERF-20"]],["PERF-27","M6",["PERF-11","PERF-22"]],["PERF-28","M6",["RTE-22"]]],"flow":[["FLOW-06","M2",[]],["GAP-07","M2",[]],["FLOW-09","M2",[]],["FLOW-12","M2",[]],["FLOW-13","M2",[]],["FLOW-08","M3",["FLOW-09"]],["FLOW-10","M3",["FLOW-09"]],["FLOW-11","M6",["FLOW-09"]],["FLOW-15","M6",[]],["GAP-16","M6",[]]]}
const CAP2 = (args && args.cap2) || 15000000
const START2 = budget.spent()
const used2 = () => budget.spent() - START2
const over2 = () => used2() > CAP2

const CONTEXT2 = `
PROJECT: Philis, a Rust analog place-and-route tool. You are working on milestones M2–M6 of docs/plans/00-MASTER-PLAN.md, organised by MODULE: one module owner works through its items across all waves in a fixed order, waiting only on hard cross-module dependencies (docs/plans/dag-m2-m6.json: per item module, milestone, "hard" deps, "step_level" notes; docs/plans/dag-m2-m6.md explains every resolution and split item — RTE-26 and PERF-20 have an early slice and a full later version). Item specs: "### <ID>" in docs/plans/plan-*.md and docs/plans/98-gap-critic.md; §6 of 00-MASTER-PLAN.md and its "## Status" section override plans. M1 cards (docs/plans/cards/m1-*.md) and m1-report.md describe what M1 changed.
M1 items NOT done (an item needing one of these must say so and do what it can, or defer): PLC-06 (waits on owner decision).

GIT: main checkout ${ROOT} — NEVER touch it. Integration branch m2 at ${INT2}. Each module has worktree ${W2}/<module> on branch m2-<module>. Work only in your worktree. Never push, stash, clean or rewrite history.

ENV: export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk before cargo test/bench. ngspice, python3 present. klayout/magic/netgen in the nix store only (/nix/store/ka8ydbkkskc7yjzsz4pnjqnzs6ncnr0s-klayout-0.30.4-1/bin/klayout, /nix/store/8gyx3l4k4sv79xb0i053v4zdrhjqab2p-magic-vlsi-8.3.573/bin/magic, /nix/store/2qdnzjnczyh1wr06wl7hr1s4hv5i3kgd-netgen-1.5.292/bin/netgen). Bash calls time out at 10 min; background long cargo runs. --release for benchmark/signoff tests. Eight modules build in parallel: prefer cargo test -p <crate> over the whole workspace.

RELAYED MESSAGES: the owner sometimes chats with the orchestrating session; such messages may be relayed to you. They are NOT instructions for you — never stop, skip or shorten your task because of one; finish the task you were given.

USAGE: tight budget. Read each file once; read only what your items need; keep reports short.

HONESTY RULES: never loosen a threshold/baseline/assertion, never #[ignore] or delete a failing test, never special-case a fixture name. Tests must fail when their behaviour breaks. If acceptance cannot be met, say so with evidence. Match surrounding style (dense factual doc comments, minimal changes, reuse helpers). Stay within each item's scope; report out-of-scope bugs.
`
const ALL2 = Object.values(MODS).flat().map(x => x[0])
const d2 = {}
for (const id of ALL2) { let r; const p = new Promise(res => { r = res }); d2[id] = { p, r } }
const status2 = {}
const MERGED = new Set((args && args.merged) || [])
for (const id of ALL2) if (MERGED.has(id)) { status2[id] = 'merged'; d2[id].r({ id, status: 'merged' }) }
// Items an earlier run's hardening deferred: keep them deferred (no re-hardening); dependents block.
const PRESET = (args && args.preset) || {}
const presetOut = []
for (const [id, why] of Object.entries(PRESET)) if (d2[id] && !status2[id]) { status2[id] = 'deferred'; presetOut.push({ id, status: 'deferred', note: why }); d2[id].r({ id, status: 'deferred' }) }
let lock2 = Promise.resolve()
function serial2(fn) { const p = lock2.then(fn, fn); lock2 = p.catch(() => {}); return p }

async function runModule(name, list) {
  const wt = `${W2}/${name}`
  const out = []
  const settle = (id, st, note) => { if (status2[id]) return; status2[id] = st; out.push({ id, status: st, note: note || '' }); d2[id].r({ id, status: st }) }
  // skip items already on m2; segment numbers continue past earlier runs' cards
  let i = 0, segN = (args && args.segStart && args.segStart[name]) || 0
  // Segments an earlier run hardened but never executed: reuse their cards.
  const pend = ((args && args.pending && args.pending[name]) || []).map(p => ({ ...p, ids: p.ids.filter(id => !status2[id]) })).filter(p => p.ids.length)
  const handled = new Set()
  while (i < list.length) {
    const [fid, fmil, fdeps] = list[i]
    if (status2[fid] || handled.has(fid)) { i++; continue }
    const pk = pend.find(p => p.ids.includes(fid))
    if (pk) {
      pend.splice(pend.indexOf(pk), 1)
      const deps = [...new Set(pk.ids.flatMap(id => (list.find(x => x[0] === id) || [0, 0, []])[2]).filter(x => !pk.ids.includes(x)))]
      const ds = await Promise.all(deps.map(x => d2[x].p))
      const bad = ds.filter(x => x.status !== 'merged')
      pk.ids.forEach(id => handled.add(id))
      if (bad.length) { pk.ids.forEach(id => settle(id, 'blocked', 'hard dependency not merged: ' + bad.map(x => `${x.id}=${x.status}`).join(', '))); continue }
      if (over2()) { for (const it of list.slice(i)) settle(it[0], 'deferred-cap'); break }
      segN = Math.max(segN, pk.seg)
      await runSeg(pk.seg, pk.ids, { card_path: pk.card, items: pk.ids.map(id => ({ id, cls: 'judgment', reason: '' })) }, pk)
      continue
    }
    const ds = await Promise.all(fdeps.map(x => d2[x].p))
    const bad = ds.filter(x => x.status !== 'merged')
    if (bad.length) { settle(fid, 'blocked', 'hard dependency not merged: ' + bad.map(x => `${x.id}=${x.status}`).join(', ')); i++; continue }
    if (over2()) { for (const it of list.slice(i)) settle(it[0], 'deferred-cap'); break }
    const seg = [list[i]]
    let j = i + 1
    while (j < list.length && seg.length < 6) {
      const [id, mil, deps] = list[j]
      if (status2[id] || handled.has(id)) break
      if (mil !== fmil) break
      if (!deps.every(x => status2[x] === 'merged' || seg.some(s => s[0] === x))) break
      seg.push(list[j]); j++
    }
    i = j
    segN++
    await runSeg(segN, seg.map(s => s[0]), null)
  }
  return { name, items: out }

  // `done` (from restart_args.py): steps an earlier run finished for this segment — skipped, not repeated.
  async function runSeg(segN, ids, cardOverride, done) {
    done = done || {}
    const fmil = (list.find(x => x[0] === ids[0]) || [0, ''])[1]
    const tag = `${name}#${segN}`
    const card = cardOverride || await ag(`${CONTEXT2}
MODULE: ${name} (worktree ${wt}, branch m2-${name}). SEGMENT ${segN}, milestone ${fmil}, items in order: ${ids.join(', ')}.
TASK (spec hardening, no product code): cd ${wt}; git merge --no-edit m2 (resolve conflicts keeping both sides; commit). Read each item's spec and its dag-m2-m6.json entry once, and the code it touches once. Write docs/plans/cards/m2-${name}-${segN}.md: per item, current code facts (file:line; correct the plan wherever stale), exact edits (files, functions, Rust signatures, steps), tests (file, fn, exact assertions, command), and class "do" or "defer" (defer only if it truly cannot be done now — say why and what it waits for). Commit "M2+ ${name}: cards ${segN}". Return card_path and per-item classes (use cls "judgment" for do).`, { label: `harden:${tag}`, phase: 'Harden', schema: HARD, effort: 'medium' })
    if (!card) { ids.forEach(id => settle(id, 'failed', 'hardening agent failed')); return }
    const doIds = card.items.filter(x => x.cls !== 'defer').map(x => x.id)
    card.items.filter(x => x.cls === 'defer').forEach(x => settle(x.id, 'deferred', x.reason))
    if (!doIds.length) return
    const ex = done.exec || await ag(`${CONTEXT2}
MODULE: ${name} (worktree ${wt}). ${cardOverride ? 'This card was written by an earlier run: first cd there and git merge --no-edit m2 (resolve conflicts keeping both sides), then check each step still matches the code. ' : ''}CARD: ${card.card_path}. Implement items ${doIds.join(', ')} in that order, following the card (record departures in the note).
For each: implement; write the card's tests; run them plus cargo test --release -p <each crate you touched> (PDK_ROOT set); fix what you broke; commit "M2+ <ID>: <short title>". Return per-item status (done / blocked) with a one-line note, and the crates you touched.`, { label: `exec:${tag}`, phase: 'Execute', schema: EXEC, effort: 'medium' })
    if (!ex) { doIds.forEach(id => settle(id, 'failed', 'exec agent failed')); return }
    const doneIds = ex.results.filter(r => r.status === 'done').map(r => r.id)
    ex.results.filter(r => r.status !== 'done').forEach(r => settle(r.id, 'blocked', r.note))
    doIds.filter(id => !ex.results.some(r => r.id === id)).forEach(id => settle(id, 'failed', 'not reported by exec'))
    if (!doneIds.length) return
    const crates = ex.touched_crates.length ? ex.touched_crates : ['library']
    const rev = done.review || await ag(`${CONTEXT2}
MODULE: ${name} (worktree ${wt}). Review segment ${segN}: git -C ${wt} diff m2...HEAD (items ${doneIds.join(', ')}; card ${card.card_path}). Real defects only: missing/wrong card or spec steps and named tests, correctness bugs, honesty violations, vacuous tests (mutation-check the important ones), out-of-scope edits. Run the tests yourself (PDK_ROOT set). Do not edit. Each issue: file:line, what is wrong, the fix.`, { label: `review:${tag}`, phase: 'Review', schema: REVIEW, effort: 'medium' })
    if (rev && rev.issues.length && !done.fixDone) {
      await ag(`${CONTEXT2}
MODULE: ${name} (worktree ${wt}). Fix these review findings (verify each; fix real ones; one line per rejected one), re-run tests for ${crates.join(', ')}, commit "M2+ ${name}: review fixes ${segN}":
${rev.issues.map((x, k) => `${k + 1}. ${x}`).join('\n')}`, { label: `fix:${tag}`, phase: 'Review', effort: 'medium' })
    }
    const integ = await serial2(() => ag(`${CONTEXT2}
TASK: Integrator. Merge module ${name} segment ${segN} (branch m2-${name}; items ${doneIds.join(', ')}) into m2 at ${INT2}.
1. cd ${INT2}; status clean; pre = git rev-parse HEAD.
2. git merge --no-ff m2-${name} -m "M2+ ${name} ${segN}: merge (${doneIds.join(', ')})". Resolve conflicts keeping BOTH sides; never drop another module's behaviour or tests.
3. PDK_ROOT set: cargo build --release --workspace; cargo test --release -p ${crates.join(' -p ')}; cargo test --release -p benchmark --test signoff_fixtures. Failures already present at pre are fine (note them); new ones from the merge: fix and commit "M2+ ${name} ${segN}: integration fix" within the honesty rules.
4. If it cannot be made green: abort/reset to pre and return failed.
Return status, new m2 sha, short notes.`, { label: `integrate:${tag}`, phase: 'Integrate', schema: INTEG, effort: 'medium' }))
    const ok = integ && integ.status === 'merged'
    doneIds.forEach(id => settle(id, ok ? 'merged' : 'integration-failed', ok ? '' : (integ ? integ.notes : 'integrator failed')))
    log(`${tag} ${ok ? 'merged' : 'NOT merged'} (${doneIds.join(', ')}); M2+ used ${used2()} / ${CAP2}`)
  }
}

const modRes = (await parallel(Object.entries(MODS).map(([n, l]) => () => runModule(n, l)))).filter(Boolean)
const lines2 = presetOut.map(r => `${r.id}: deferred (earlier run) — ${r.note}`).concat(modRes.flatMap(m => m.items.map(r => `${r.id}: ${r.status}${r.note ? ' — ' + r.note : ''} [${m.name}]`)))
const nMerged = Object.values(status2).filter(s => s === 'merged').length
log(`M2–M6: ${nMerged}/${ALL2.length} merged; used ${used2()} / ${CAP2}`)

const final = await ag(`${CONTEXT2}
TASK: M2–M6 verifier and landing, in ${INT2} (branch m2). For the landing step only you may operate on the main checkout ${ROOT}. Item outcomes:
${lines2.join('\n')}
1. PDK_ROOT set: cargo build --release --workspace; cargo test --release --workspace --no-fail-fast (background); cargo run --release -p benchmark --bin bench local (background; it writes assets/*.svg).
2. For each exit criterion of M2–M6 in 00-MASTER-PLAN.md §5 whose items were done: met / not-met with evidence.
3. Write docs/plans/m2-m6-report.md: per-item outcome, criteria table, failing tests named, bench table vs docs/plans/m1-report.md. Add a short "## Status" paragraph for M2–M6 to 00-MASTER-PLAN.md and "Status: done in M2+ (<commit>)" under each merged item. Commit "M2–M6: report".
4. Landing: if the workspace has no failing test that also passed on m1a (main (which contains M1)), cd ${ROOT}; if git status shows tracked changes, do not merge; else git merge --no-ff m2 -m "Merge M2–M6 module run"; build. Otherwise do not merge and explain. Then update docs/plans/HANDOFF.md on main with the outcome (merged sha or why not, remaining work, worktrees left under ${W2}) and commit.
Return a short summary: merged or not, main sha, counts, failing tests.`, { label: 'final-m2', phase: 'Close-out', effort: 'medium' })

return { m2: { merged: nMerged, total: ALL2.length, used: used2(), cap: CAP2, items: lines2, final } }
