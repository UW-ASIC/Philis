# M2 annotator, segment 2: EXT-20

Branch `m2-annotator`, merged with `m2` at `be34be4` (MAT-07, MAT-08, MAT-10, MAT-12 and annotator segment 1 are in).
Spec: plan-01 `### EXT-20`, with 98-gap-critic C13 overriding step 2 (`Budget::Allowance(a)`, not
`Sigma1Mv(√(a²+σ²))`). dag-m2-m6: hard deps EXT-14/15/16, MAT-07, MAT-08 (all done). MAT-04/05 are done too. Resistor
`PhiZero` is MAT-12 step 3, which comes later and does not block this item. PLC-06 (not done in M1) is not needed. The
rotation and shape locks come from `matched_pairs` (PLC-03), which this item does not touch. Line numbers are at
`be34be4`.

Run `cargo test -p annotator -p analog -p library` with `PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.
It must be green before the commit `M2+ EXT-20: emission per matched set and compound`.

## EXT-20: emission per matched set and compound (class: do)

### Current code facts (plan corrections)

- `emit::placement(blocks, nl, p, offset_sigma_mv, policy)` (`backend/annotator/src/emit.rs:91-211`) loops over each
  top-level block's 2-device leaves (`:103-204`). For every DiffPair, CurrentMirror, Load or CascodePair leaf it emits:
  - a pairwise `MatchedSet { members: [a, b] }`, always with `class: Moderate` (`:141`);
  - its budget from `budget()` = `mismatch::choose(offset, None, None, kind)` (`:55-57`), so no allowance and no class
    limit are ever passed;
  - an `OrientationSet` Axis (hard) plus Φ by `phi_arm` (`:164-170`);
  - a `Symmetry` on `AxisId(block index)`, skipped for `bjt_ratioed_pair*` and for a pair that shares a device
    (`:128-134`).

  Stack leaves get Proximity (`:117-121`). CurrentMirror leaves get Proximity (`:171-174`). Each block's selfs get a
  self-`Symmetry` plus Proximity to the stage's DiffPair (`:192-199`). DTI goes per matched leaf (`:177-186`). The
  plan's "`emit.rs:138-239`" is stale. The leaf loop is `:103-204`.
- `lib.rs:175` calls `emit::placement` **before** the intent exists: compounds are built at `:276-278`, sets at
  `:295-358`, `set_pairs` at `:367`. Batch tagging and `touched` are at `:213-243`, and `coverage` reads `touched` at
  `:245-261`. Both run between those two points. `axis_count: blocks.len()` is at `lib.rs:408` (segment 1 D-b: switch
  it here).
- `analog::placement::MatchedSet` (`kernel/analog/src/placement/matched_set.rs:26-52`) has **no `class_explicit`
  field**. MAT-08 put the explicit-class rule into `mismatch::choose(offset, allowance, class_limit, kind)`
  (`kernel/analog/src/matching/mismatch.rs:130-137`): the caller passes `class_limit` only for an explicit class. The
  plan's `class_explicit = …` becomes "pass `class::limit(..)` iff `class_source != Role`".
- `MatchedSet::for_family(members, family, kind, class, coeffs, budget, areas_um2, tol_nm)` (MAT-10,
  `matched_set.rs:75-86`) is the constructor for every family. It sets `gm_over_id` and `sigma_rand_override` to
  `None`.
- `OrientCheck` has only `Axis` and `Phi` (`placement/orientation.rs:19-27`). It has no `PhiZero`, so plan step 3's
  resistor `PhiZero` waits for MAT-12 step 3 (dag note).
- `Compound.axis = AxisId(i)` for compound `i` (`symmetry.rs:361`). The seeds are DiffPair/Load/CascodePair leaves
  only (`lib.rs:265-273`), so a lone mirror (`mirror6`, the `a_lone_mirror_stage_is_symmetric_too` fixture) has **no
  compound** and gets no `Symmetry` (plan T5 "0 Symmetry" on `mirror6` holds).
- If unitization fails, the spec's `Member.parallel` is the finger count, which is not a unit count
  (`sets.rs:314-318`, `unit: None`). The equal-couple test must require `s.unit.is_some()`.
- Bipolar coefficients: decks carry `bjt_ka_pct_um` and `vbe_tc_uv_per_k`
  (`pdks/*.json`, registered `"unread (EXT-20)"` at `backend/verify/src/sidecar.rs:69,138`). `ProcessNumbers`
  (`backend/annotator/src/netrole.rs:149`) has no fields for them. R/C `k_a_pct_um` is per recipe (FLOW-06) and has
  no `ProcessNumbers` slot, so R/C sets stay unknown (MAT-10 acceptance: "explicit unknown where not").

### Decisions (informed defaults)

- **E-a. Allowance → budget (C13).** `s.allowance = Some(a)` with `kind == Voltage` gives `Budget::Allowance(a)`.
  Current and Ratio allowances are ignored until EXT-21 allocates them. Today they are always `None`. `Allowance` on a %
  ledger would be read as mV by `Budget::to_pct`.
- **E-b. Sets get Symmetry only through compounds.** A 1:1 mirror with no compound now gets a `MatchedSet`, Axis
  Orientation and (if Minimal) Proximity, but no `Symmetry`. This follows from plan step 1 and AA-23. The test
  `a_lone_mirror_stage_is_symmetric_too` (`tests.rs:661-683`) encodes the old per-stage rule. Rewrite it to the new
  contract (below), renamed `a_lone_mirror_is_matched_not_mirrored`. This is a spec-mandated change, not a loosened
  assertion.
- **E-c. Selfs.** Self-`Symmetry` entries come from `compound.selfs` (step 1). The Proximity of a block's declared
  `selfs` to its stage's DiffPair stays as in `emit.rs:192-199` (step 4, "declared selfs → P_B"). Stack leaves keep
  their Proximity (step 4, "prox couples").
- **E-d. Batch origin tagging is unchanged** (`lib.rs:213-243`, first touched device's block). Re-tagging set batches
  with `MatchSpec.origin` is outside EXT-20's scope.
- **E-e. `axis_count = intent.compounds.len().max(1)`.** Glue blocks already create unused axes today, so one spare
  axis is harmless. `frontend/library/src/lib.rs:1943` already applies `.max(1)`.

### Exact edits

1. `backend/annotator/src/netrole.rs` `ProcessNumbers`: add `pub bjt_ka_pct_um: Option<f32>` and
   `pub vbe_tc_uv_per_k: Option<f32>` (doc: deck cell keys, bipolar/diode `ka_pct_um` and `|dV_BE/dT|`, MAT-10).
   `frontend/library/src/lib.rs:1032` `annotation_with`: `bjt_ka_pct_um: pos("bjt_ka_pct_um"), vbe_tc_uv_per_k:
   pos("vbe_tc_uv_per_k")`. In `backend/verify/src/sidecar.rs:69,138`, change the consumer string to
   `"frontend/library/src/lib.rs annotation"`.
2. `backend/annotator/src/emit.rs`: replace `budget()` (`:53-57`) with
   ```rust
   /// A set's budget: an allocated Voltage allowance as itself (C13), else MAT-08's rule, with the
   /// class limit only for a User/Spec class.
   fn set_budget(s: &MatchSpec, offset_sigma_mv: Option<f32>) -> Budget
   ```
   Body: `match s.allowance { Some(a) if s.kind == MatchKind::Voltage => Budget::Allowance(a), _ =>
   mismatch::choose(offset_sigma_mv, None, (s.class_source != ClassSource::Role).then(|| class::limit(s.family,
   s.kind, s.class)).flatten(), s.kind) }`.
   Add `fn coeffs(nl, p, d: DeviceId, family: Family) -> Coeffs`. For Mos, use today's block from `:142-151`. For
   Bipolar/Diode, use `Coeffs { ka_pct_um: p.bjt_ka_pct_um, vbe_tc_uv_per_k: p.vbe_tc_uv_per_k, die_temp_k:
   p.die_temp_k, ..Default }`. For Resistor/Capacitor, use `Coeffs::default()` (unknown, FLOW-06).
   Add `fn area_um2(nl, drawn: &[Drawn], d) -> f32`. For a FET it is `gate_um2`. Otherwise it is `w·l·fingers·1e-6`
   from `drawn[d]` when both are `Some`, else `0.0`.
3. Change the signature to
   ```rust
   pub fn placement(intent: &analog::intent::Intent, blocks: &[Block], nl: &Netlist, drawn: &[crate::size::Drawn],
                    p: &ProcessNumbers, offset_sigma_mv: Option<f32>, policy: &crate::policy::Policy) -> Requirements<Layout>
   ```
   Then delete the leaf loop `:103-204` and emit:
   - **Per compound `c`**: build `syms` = `Symmetry{a,b,AxisId(c.axis.0)}` for each `(a,b)` in `c.pairs` such that
     some `s` in `intent.sets` has `s.unit.is_some()`, contains both, and their members have equal `(parallel,
     series)`. Add `Symmetry{d,d,axis}` for each `d` in `c.selfs`. If `syms` is non-empty, push
     `SymmetryGroup(syms)` to hard and cost. For every `(a,b)` in `c.pairs` (equal or not), push a `DtiBand` when
     `p.dti` is set, with `branch = BranchId(dti.len())` as today.
   - **Per spec `s`** (in `intent.sets` order): `members` = the spec's devices, with `s.reference` (if any) swapped to
     slot 0. Build `MatchedSet::for_family(members, s.family, s.kind, s.class, coeffs(slot 0), set_budget(s, offset),
     areas, tol_nm = p.lattice_nm.max(1) as f32 / 2.0)`. Push it to budget and cost: **one batch per set**.
   - **Orientation**: if `s.family` ∈ {Mos, Bipolar} and there are ≥ 2 members, push `OrientationSet{members,
     Axis}` to hard. Then by `phi_arm(s.class)`: `Some(true)` → `Phi` hard, `Some(false)` → `Phi` budget. Resistor
     `PhiZero` is skipped. Mark it with `// ponytail: resistor PhiZero is MAT-12 step 3`.
   - **Proximity** (budget + cost, `policy.proximity_nm`): for a Minimal set, one batch from slot 0 to every other
     member. For every Stack leaf in `block::leaves(blocks)`, one batch (as `:117-121`). For every block with a
     DiffPair leaf, the declared `selfs` to both pair devices (as `:192-198`). Moderate and Exceptional sets get none.
   - DTI is pushed once at the end, as `:206-209`.

   Rewrite the module doc table (`:1-35`) to describe the emission by compound and by set.
4. `backend/annotator/src/lib.rs`: move `let mut placement = emit::placement(...)` (`:175`), the Isolation call
   (`:197-201`, with `block_of`/`same_block` `:192-196`), the tagging loop for **both** placement and routing
   (`:209-243`) and `coverage` (`:245-261`) to after `sets::set_pairs` (`:367`) and before the rings block (`:368`),
   which keeps the `missing` order. `routing` extraction can stay where it is, but its tagging moves with the loop so
   ids stay in "placement, then routing" order. Pass `&intent, &blocks, netlist, &drawn`. Set `axis_count:
   intent.compounds.len().max(1)` (E-e).
5. Tests switch from `canon_intent` to `canon` (segment 1 D-c). `backend/annotator/tests/align_gold.rs:29` reads
   `canon(...)`. Each StrongARM gold pair is an equal couple in one set (`mp5`/`mp6` 4:4, the others 1:1, per the
   `strongarm` EXPECTED row), so T1 must still pass unchanged. If a pair drops out, report it; do not revert to
   `canon_intent`.

### Tests

- `backend/annotator/tests/corpus.rs::one_rule_per_set` (T5). For every circuit in `all()`, the `MatchedSet` batches
  in `p.placement.budget` (`kind() == "MatchedSet"`) number `p.intent.sets.len()`, and the sorted `touched` id set of
  batch `j` equals the sorted devices of `intent.sets[j]`. No device id appears in two batches of the same set. Also:
  `ota5t` has exactly 2 batches, `{XM1,XM2}` and `{XM3,XM4}`.
- `corpus.rs::ratioed_mirror_not_mirrored`. On `mirror6` with `cfg("mirror6")`:
  - 0 `Symmetry` (sum of `count()` over hard `kind()=="Symmetry"`);
  - 1 `MatchedSet` with 6 touched ids, the first being `MR` (verify `touched` order = members order; otherwise read
    `ledger_rows` `members.0`);
  - 5 `Proximity` entries in budget (sum of `count()`), every one including `MR`.

  Then clone the problem's intent, set `sets[0].class = Moderate, class_source = User`, call `emit::placement` again
  (`annotator::emit` and `annotator::size` are `pub`, `lib.rs:14,24`; rebuild `drawn` with `size::drawn`). Expect 0 `Symmetry` and 0 `Proximity`.
- `emit::tests::set_budget_follows_source` (unit, on hand-built `MatchSpec`s, Mos Voltage Moderate):
  - Role, no offset → `Budget::Eta(GRADIENT_SHARE)`;
  - User → `Budget::Sigma1Mv(0.5)` (3 mV / 6);
  - offset `Some(0.4)` → `Sigma1Mv(0.4)`;
  - `allowance Some(0.4)` on a Voltage set → `Budget::Allowance(0.4)`;
  - the same on a Current set (Role) → `Eta(GRADIENT_SHARE)`.
- `emit::tests::allocated_allowance_round_trips`. Take `crate::tests::ota()` with `avt_mv_um = [Some(5.0),
  Some(6.0)]` and `annotate` it. Set `intent.sets[i].allowance = Some(0.4)` on the Voltage set and re-run
  `placement`. On the 2-device layout of `tests.rs:710-722`, that set's `ledger_rows` give `rows[0].allowance ==
  0.4` exactly (`Budget::Allowance` passes through, C13) and `rows[0].sigma_rand > 0`.
- Existing tests to update to the new contract (not loosen):
  - `default_class_is_moderate_mos` (`emit.rs:346-357`): hard `Orientation` count = number of sets with family
    Mos/Bipolar and ≥ 2 members; budget `Orientation` count = those of them that are Moderate.
  - `a_lone_mirror_stage_is_symmetric_too` → `a_lone_mirror_is_matched_not_mirrored` (E-b). Expect 0 `Symmetry`
    entries, exactly 1 budget `MatchedSet` on ids `[0, 1]`, and 1 hard `Orientation`.
  - The `EXPECTED` columns `pairs/selfs/net_pairs/axes` in `corpus.rs:89-166` are `canon` of emitted Symmetry.
    Recompute them as each circuit's `COMPOUNDS` row (`:402-410`) restricted to equal couples in one set. `mirror6`
    becomes `&[], &[], &[], 0`, and `dac4`/`latch` etc. follow the rule. List every changed row in the commit
    message.
  - These must pass **unchanged**:
    - `diff_pair_emits_its_constraints`;
    - `ota` Symmetry count 3 (`tests.rs:~470`);
    - `telescopic_slot7_is_not_self_symmetric` (selfs == `[6]`);
    - `dti_bands_are_one_hard_batch…`;
    - `proximity_is_a_priced_budget_not_just_a_pull`;
    - `no_emitted_conflicts*`;
    - `a_cascode_stack_is_adjacent_not_matched`;
    - the MAT-16 ratio test (`tests.rs:~725`).

    If one fails, find out why before editing it.
- Commands:
  - `cargo test -p annotator -p analog`;
  - `cargo test -p library` (the flow and metadata tests read the placement arms);
  - acceptance T9: `bench local` on `dac4`, `bgr_core`, `pair`, `quad` and `ota5t` in `--release`, with DRC 0 and LVS
    MATCH, recorded in `m2-annotator-report.md`.

### Acceptance / risks

T5, AA-23, AA-24 closed. T9 DRC 0 / LVS MATCH. The new sets on `dac4` (an unknown Capacitor ledger, cost pull only)
and the dropped mirror Proximity change dp prices. That is expected (plan risk), but `bench local` must be re-run, not
assumed. `MatchedSet::class` docs ("Moderate until EXT-20 reads it from the intent", `matched_set.rs:35-36`) become
stale. Update that comment in the same commit.
