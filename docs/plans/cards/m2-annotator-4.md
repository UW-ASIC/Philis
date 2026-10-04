# M2 annotator, segment 4 (M3): GAP-09

Branch `m2-annotator` was fast-forwarded to `m2` at `7073e38`, which already holds segment 3 (EXT-17/18/23/24/26 and GAP-03). The spec is `98-gap-critic.md ### GAP-09` together with master §6 "EXT M3" ("GAP-09 finds 0 conflicts on the 17 corpus circuits and names both rule IDs of every injected conflict"). The dag gives the hard deps as EXT-14 and EXT-26, both done. Step 3 is a step-level dependency on FLOW-08, a flow-module item in the same milestone. GAP-09 does not need PLC-06. Line numbers are at `7073e38`.

Every commit must pass `PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk cargo test -p annotator -p analog -p library`. Commit as `M2+ GAP-09: pre-search conflict report`.

## GAP-09: pre-search over-constraint check (class: do; step 3 deferred to FLOW-08)

### Code facts (the plan's "Current" is stale)
- **(a) already half exists.** `symmetry::analyze` (`backend/annotator/src/symmetry.rs:162-222`) takes seeds in order: the sidecar seeds first, then the pattern seeds (`lib.rs:212-222`). When a `Seed::Devices` contradicts an existing pairing or net mate, analyze pushes a `"conflicting_seed"` diagnostic and drops that seed (`:178-185`), but the diagnostic names neither rule. A non-free `Seed::SelfDevice` is ignored silently (`:196-200`). A seed whose pair has unequal signatures is also skipped silently (`:175`). Sidecar seed ids are `ConstraintId(u32::MAX − entry)` (`sidecar.rs:43`, documented at `netrole.rs:144-146`). Pattern seed ids are `ConstraintId(leaf index)` (`lib.rs:221`). State does not record which seed made a pairing. No test asserts `"conflicting_seed"`.
- **(b) has one source today.** `emit::isolation` (`emit.rs:256-289`) skips the pairs that are `related` (same block, same set or same compound, `lib.rs:390-397`). Every pattern Proximity (`emit.rs:137, 200-221`) lies inside such a relation. The sidecar `GroupBlocks` pull (`emit.rs:222-229`, `g[0]` → each other member, at `policy.proximity_nm = 5000`) is not inside one. A group that holds an aggressor and a victim therefore gets Proximity ≤ 5 µm and Isolation ≥ `4·t_epi` on the same pair. That is 10 µm at the nominal epi, so it always conflicts when uncalibrated. The emitted batches are `Box<dyn RuleBatch>`, which do not expose distances, so a post-hoc `check(p: &Problem, ..)` cannot see (b). It has to run where both distances are known, before tagging.
- **(c)** The sidecar `Match` entry (`sidecar.rs:108-127`) pushes `(devices, class, kind)` with no size check. `lib.rs:254-255, 286` apply it to every set whose members the entry covers.
- **(d)** `Differential` pairs are the non-rail `net_pairs` of each compound, with `x ≠ y` (`extract.rs:111-118`). They form one routing budget batch.
- `Diagnostic { kind, devices, message }` is defined at `kernel/analog/src/intent.rs:278-282`. Every constructor is in `annotator` (18 sites).
- **Step 3's premise is stale.** `gp::Prices` (`backend/gp/src/lib.rs:26-140`, `LAMBDA_MAX` `:67`) prices **placement** budget batches only. Routing C budgets have no λ that could reach `LAMBDA_MAX`. Only the flow (`frontend/library/src/lib.rs:813-886`) reads `prices.saturated()`, which it reports as `binding`.

### Decisions (informed, recorded)
1. **Use the ids in the message instead of a new field.** A conflict is a `Diagnostic` with `kind: "conflict"` and `message = "ids <a>,<b>: <what>"`, where the ids are decimal `ConstraintId.0`. This makes no change to `Diagnostic` (18 sites) or to `Intent`. Nothing consumes the ids structurally yet. If a consumer appears, add `ids: Vec<ConstraintId>` then.
2. **Id namespaces.** (a) names seed ids: sidecar `u32::MAX − entry`, pattern = leaf index. These are what a user can map back to an entry. (b) names the `BatchMeta.id` of both emitted batches. (c) names only its sidecar entry, because the other side is a netlist fact and not a rule.
3. **Drop rule (step 2).** In (a) the earlier seed wins: user seeds come before pattern seeds, and user seeds go in entry order. All of them are S/M_S, so the survey's order gives no tie-break. In (b) the user `GroupBlocks` pull wins over the derived Isolation and the Isolation pair is removed. This is **[policy]**: EXT-26 already lets the user win everywhere else, and AA-13 exempts related devices in the same way. In (c) the user `Match` entry is dropped, because it contradicts the drawn sizes.
4. **(c) compares `DeviceKind`, model and L, not W.** A ratioed set differs in W by design (EXT-15 ratio inference, a 1:4 mirror), so a W check would flag legitimate Match entries. This deviates from the plan's "W".
5. **(d) is `kind: "asymmetric_net_pair"`, not `"conflict"`.** The spec says (d) is reported only and nothing is dropped. It is a netlist fact and not a clash between rules. Keeping it out of `"conflict"` keeps the "0 conflicts on the corpus" acceptance about rule clashes.
6. **(a)–(c) run where their data lives.** (a) runs inside `symmetry::analyze`, because the dropped seed never reaches `Problem`. (b) runs in `annotate_with`, because the distances are not readable from boxed batches. (c) runs in `sidecar::parse`, which also covers the library path through `AnnotationConfig::from_json`, `frontend/library/src/lib.rs:399-403`. Only (d) is the post-hoc `conflict::check`.

### Exact edits
1. **New `backend/annotator/src/conflict.rs`** (`pub mod conflict;` in `lib.rs`, alphabetical):
   ```rust
   //! GAP-09 (BAL2-48, NOTES-06): constraints that cannot all hold are reported
   //! by rule id and the lower-priority one dropped, instead of letting λ
   //! saturate at LAMBDA_MAX. (a) symmetry seeds: `symmetry::analyze`; (b) a
   //! sidecar pull vs Isolation: `annotate_with`; (c) a sidecar Match on unequal
   //! kind/model/L: `sidecar::parse`; (d) here.
   /// A `"conflict"` naming `ids` as `"ids a,b: what"` (decimal `ConstraintId.0`).
   pub(crate) fn diag(ids: &[ConstraintId], devices: Vec<DeviceId>, what: impl std::fmt::Display) -> Diagnostic
   /// (d): every Differential net pair (a compound's non-rail `net_pairs`, x ≠ y)
   /// whose nets are touched by different device counts: `"asymmetric_net_pair"`,
   /// message names the Differential batch id ("ids <id>: <x>/<y> <nx> vs <ny> devices").
   pub fn check(p: &Problem, nl: &Netlist) -> Vec<Diagnostic>
   ```
   `check` counts the distinct devices that have a terminal on the net (`nl.devices.iter().filter(|d| d.terminals.iter().any(|t| t.1 == n))`). It applies rail exclusion through `p.net_classes[n].class` ∉ {Supply, Ground, Substrate}, the same filter `extract.rs` uses. The batch id is the `meta().id` of the first `p.routing.budget` batch whose `kind()` ends with `"::Differential"`. Skip the pair when no such batch exists.
2. **(a) `symmetry.rs`.** `State` gains `cur: ConstraintId`, `by: Vec<Option<ConstraintId>>` (per device) and `net_by: Vec<Option<ConstraintId>>` (per net). Set `s.cur` to the seed's id at the top of each seed iteration (`:171`). `pair()` sets `by[d] = by[e] = Some(self.cur)`. `bind()` sets `net_by[x] = net_by[y] = Some(self.cur)`. Both self-marks (`:198`, `:234`) set `by[i]`. During the fixpoint loop `cur` keeps the last seed's id. That is harmless because conflicts arise only in the seed loop. Add a helper `fn owner(&self, a: u32, b: u32) -> Option<ConstraintId>` that returns `by[a].or(by[b])`, or else the `net_by` of the first terminal net of `a` where `!consistent_nets`. Replace the `"conflicting_seed"` push (`:179-183`) with `conflict::diag(&[id, owner], vec![a, b], "SymmetricBlocks pair contradicts an earlier pairing; dropped")`, writing `?`-free output by leaving `owner` out when it is `None`. The `SelfDevice` arm pushes `conflict::diag(&[id, by[d]], vec![d], "self-symmetric device already paired; dropped")` when `!s.free(d)` and `by[d] != Some(id)`.
3. **(b) `emit.rs`.** Extract `pub fn isolation_min_nm(kind: SubstrateKind, epi_nm: Option<i32>) -> i32` from `:266-272` and use it inside `isolation` (no behaviour change). **`lib.rs` before `:399`:**
   ```rust
   // GAP-09 (b): a sidecar GroupBlocks pull (≤ proximity_nm) on an aggressor–victim
   // pair that Isolation would push ≥ isolation_min_nm apart: the user pull wins.
   let iso_nm = emit::isolation_min_nm(p.substrate, p.epi_nm);
   let clash: Vec<(usize, usize, u32)> = cfg.groups.iter().flat_map(|(e, g)| g.iter().skip(1).map(move |m| (g[0].0 as usize, m.0 as usize, *e)))
       .filter(|&(x, y, _)| ((aggressor[x] && victim[y]) || (aggressor[y] && victim[x])) && !related(x, y) && cfg.policy.proximity_nm < iso_nm).collect();
   let related = |a: usize, v: usize| related(a, v) || clash.iter().any(|&(x, y, _)| (x, y) == (a, v) || (y, x) == (a, v));
   ```
   After the tagging loop (`:407-...`, before `coverage`), for each clash push `conflict::diag(&[user_id, iso_id], vec![x, y], format!("GroupBlocks Proximity ≤ {} nm vs Isolation ≥ {iso_nm} nm; Isolation dropped", cfg.policy.proximity_nm))`. Here `user_id` is the `meta().id` of the first `placement.budget` batch with `origin == Origin::User { index: e }`, and `iso_id` is that of the first `placement.budget` then `placement.cost` batch whose kind ends with `"::Isolation"`. Either id is left out when absent. The Isolation id is absent only when the dropped pair was the batch's last one; record this in the doc comment.
4. **(c) `sidecar.rs` `"Match"` arm (`:124-127`).** After `devices(..)` returns `Ok(ds)`, compute `size::drawn` for each device with a local `models` vector. If any device differs from `ds[0]` in `(nl.devices[d].kind, drawn.model, drawn.l_nm)`, push `crate::conflict::diag(&[id], ds, "Match on unequal kind/model/L; entry dropped")` and `continue`.
5. **(d) `lib.rs`.** Bind the `Problem` at `:500` as `let mut out = Problem { .. };`, then run `let d = conflict::check(&out, netlist); out.intent.diagnostics.extend(d); out`.
6. Update the module docs: `lib.rs:1-7` gets one clause, "then reports conflicts ([`conflict`])". Delete the `"conflicting_seed"` kind everywhere.

### Tests
- `backend/annotator/src/conflict.rs` `mod tests` uses `crate::tests::{ota, fet, nets}`. `ota6()` is `ota()` plus `fet("XM6", Nmos, 9, 9, 3, 3, 10_000, 1_000)` on a new net `x6` (same signature as XM1).
  - `sidecar_double_pairing_is_a_conflict`: `SymmetricBlocks` `[[XM1,XM2],[XM1,XM6]]` in one entry, parsed and annotated. Exactly one diagnostic has `kind == "conflict"`. Its devices are `[XM1, XM6]` (ids 0, 5) and its message starts with `"ids 4294967295,4294967295:"`. No compound pair contains device 5. `(0, 1)` is a pair.
  - `two_entries_name_both_ids`: the same pairs split over two entries. The message starts with `"ids 4294967294,4294967295:"`.
  - `match_on_unequal_l_is_a_conflict`: `Match [XM1, XM5] moderate` (L 1000 vs 2000). Assert `cfg.classes.is_empty()` and that one diagnostic is a `"conflict"` with devices `[0, 4]` and a message starting `"ids 4294967295:"`.
  - `asymmetric_differential_is_reported`: `ota()` plus an extra NMOS whose gate is on `vout1` only. Every `asymmetric_net_pair` message names the `meta().id` of the Differential batch, and the 2-vs-3 counts appear in it. Take that id from `p.routing.budget`. On plain `ota()`, assert the same diagnostic is absent **only if** the counts are equal. Read them first: in `ota()`, `vout1` is touched by XM1/XM3/XM4 and `vout2` by XM2/XM4, so check the mated nets before writing that assertion and do not tune the fixture to pass.
- `backend/annotator/src/emit.rs` `mod tests` (next to `strongarm_like`):
  - `proximity_below_isolation_is_a_conflict`: `strongarm_like()` with `cfg.groups = vec![(0, vec![XS, mn1])]`. Assert that `isolated(&p.placement.cost)` contains no `(XS, mn1)` pair in either order but still contains `(XS, mn2)`. Assert exactly one `"conflict"` diagnostic, with devices `{XS, mn1}`, whose message starts with `format!("ids {},{}:", user_id, iso_id)`. Both ids are read from the tagged batches as in edit 3.
- `backend/annotator/tests/corpus.rs`:
  - `clean_corpus_has_no_conflicts`: `for (name, src) in all()`, run `annotate(&net(src), &cfg(name))` and assert that no diagnostic has `kind == "conflict"`. The failure message names the circuit and the diagnostics.
- Command: `PDK_ROOT=… cargo test -p annotator` (then `-p analog -p library`).

### Step 3: deferred to FLOW-08 (flow module, M3)
The C_floor check (HPWL(pin centres) × the minimum C/nm over the routed layers > limit, reported as `infeasible_budget`) belongs in FLOW-08's epoch loop in `frontend/library/src/lib.rs`, which the flow module owns. FLOW-08 is not merged, and its hard dep is FLOW-09. The plan's motivation is wrong: routing C budgets have no λ (`gp::Prices` is placement-only), so the step is a report and does not exclude anything from ρ escalation. Hand-off: when FLOW-08 lands, add `infeasible_budget: Vec<String>` (budget kind and net) to `MetadataReport` (`frontend/library/src/metadata.rs:83`) and the library test `c_budget_below_hpwl_floor_is_reported` (10 aF, pins 100 µm apart). The acceptance clause "no λ reaches LAMBDA_MAX on a budget reported infeasible" is vacuous for routing budgets.

### Out of scope (reported, not fixed)
- A sidecar `SymmetricBlocks` pair with unequal signatures is still dropped silently (`symmetry.rs:175`). EXT-26 could report it as `sidecar_unsupported`.
- Pattern seed ids (leaf index) and `BatchMeta` ids share the small-integer range, so an id alone does not say which namespace it is from.
