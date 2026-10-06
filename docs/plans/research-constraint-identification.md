# Research: constraint identification beyond the topology catalog

Plan ID prefix: `ID-NN`. Area: `backend/annotator` (identification), `kernel/analog/src/intent.rs` (contract), and the evidence producers in `frontend/library` (`oppoint.rs`, `perf.rs`).

- Code was read at HEAD `89e8f7e` (2026-10-06). `file:line` references may drift a few lines while other agents edit. No cargo command was run (task rule).
- Citation keys `[KEY]` resolve in §4. Verification marks:
  - `V`: page or PDF fetched, title, venue and year confirmed.
  - `V-meta`: Crossref or publisher metadata confirmed; the method bullets come from abstracts, citations or snippets.
  - `L`: local copy in `docs/ref/`, read through its `docs/plans/ref-*.md` digest.
  - `U`: not verified.
- **speculative** marks a Philis idea not found in the literature searched (three parallel searches, about 90 sources screened, 70 kept).
- Confidence levels used throughout, defined in §3.1: `User` > `Proven` > `Corroborated` > `Inferred` > `Hint`.

---

## 1. Summary

### 1.1 The catalog problem

- **Recognition is a closed list.**
  - 91 hand-written FET/BJT patterns (`catalog.rs:2338`).
  - Matched by backtracking monomorphism (`pattern.rs:327-363`), then a greedy disjoint pick (`pattern.rs:455-475`).
  - A device in no pattern becomes glue: `Coverage::Unconstrained("no pattern")` (`lib.rs:599-614`).
- **Symmetry needs a catalog seed.** Seeds come from three places:
  - DiffPair/Load/CascodePair leaves (`lib.rs:216-226`);
  - sidecar seeds (`lib.rs:228`);
  - identical subcircuit instances (`lib.rs:229-248`).
  
  With no seed, `symmetry::analyze` (`symmetry.rs:181-248`) never starts. An unlisted differential topology then gets no compound, no `Differential` net pair and no CC half labels.
- **Downstream facts inherit the gap.** All of these key on template names or leaf kinds:
  - set kind (`class.rs:32-46`, used at `lib.rs:300-311`);
  - set role, which picks the class default (`lib.rs:325-350`);
  - "sensitive" devices (`lib.rs:189-196`);
  - digital, noisy and reference net classes (`classify.rs:153, 207-264`);
  - Stack proximity and stage-self pulls (`emit.rs:210-223`);
  - dummies on devices that are recognised but in no set (`constraints.rs:119-146`);
  - WPE/OSE pairs (`frontend/library/src/lib.rs:2783-2790`);
  - substrate balance (`emit.rs:306+`).
- **A template name is not the electrical role.** A name says what a block *is*. Layout needs what it *does*: which mismatch moves which spec, which node is high-Z, which current loop switches. The catalog encodes this only indirectly and only for listed shapes. False positives were the audit's top findings (AA-03, AA-05, AA-06, AA-15). The literature agrees: every GNN paper ends with hand rules that filter false positives [GAO21; CHEN21; XU24], and ALIGN's catalog-free search still reports false positives on level shifters and dummies [KUNAL20].
- **Much of the pipeline is already catalog-free**, and is the base to build on:
  - WL canonical labels (`pattern.rs:412-449`);
  - mated-net propagation (`symmetry.rs`);
  - shared-bias sets and ratio unitization (`sets.rs:50-209`);
  - passive and BJT procedural sets (`passive.rs`);
  - net classes from names, bulk, impedance and the testbench (`netrole.rs:36-111`, `classify.rs:188-317`);
  - current flow and signal flow (`flow.rs`);
  - device roles from the operating point (`evidence.rs:96-156`);
  - sensitivity allocation and budgets (`allocate.rs`, `budget.rs`);
  - substrate injectors (`substrate.rs:66-143`).

### 1.2 Recommended identification architecture

1. **Normalise first.** Build a typed device–pin–net graph:
   - terminal roles come from `terms.rs:10-20`, never from position;
   - rails are individualised and ports are marked;
   - passive P/N are interchangeable, and MOS S/D share one edge colour;
   - parallel and series identical devices are merged;
   - dummies with every terminal on a rail are dropped;
   - S/D are fixed by potential.
   
   This copies ALIGN's `ConfigureCompiler` pre-pass [ALIGN-SCHEMA].
2. **Structural invariants, with no catalog:**
   - colour refinement, extending `canonical_labels` and adding a DC-potential proxy (channel-path distance to Ground and to Supply [GAO21]);
   - involutive automorphisms with rails fixed [NAUTY14; HAO04];
   - local symmetry seeded at virtual-ground shared nets [MAG19] and at port pairs [KUNAL20];
   - translinear loops, equal-drive groups, dual CMOS networks, current paths.
3. **Symmetry and matching** use the existing mated-net fixpoint (`symmetry.rs`), but are seeded by those invariants instead of by template leaves.
4. **Electrical evidence confirms and quantifies:**
   - operating-point equality across a pair;
   - mirrored current waveforms [DHAR22];
   - commutation of the linearised conductance matrix with the swap (speculative);
   - sensitivity antisymmetry (speculative);
   - ∂f/∂C, ∂f/∂R and ∂f/∂C_c [CHOU93; CHAR94].
   
   Evidence can raise or lower confidence. It never invents a hard rule on its own.
5. **Specs and the testbench decide importance:** class, allowance, budgets and aggressor strength come from `Sensitivities` (`evidence.rs:48-73`) and transient/AC probes. Role defaults apply only when there is no spec.
6. **Learned and LLM output is a `Hint`.** It is promoted only when a structural or electrical check corroborates it. GNN and LLM extractors report 0.5–7 % false positives and need guard rules [CHEN21; XU24; LC24].
7. **The catalog shrinks to naming.** Patterns run last, to name blocks and to provide `Hint` seeds. Deleting the catalog must not lose any constraint on the gold corpus.
8. **Every fact carries `Origin` and `Confidence`.** Confidence picks the arm (hard, budget or cost). Conflicts resolve by User, then confidence, then the survey order M_S ≻ M_B ≻ P_B ≻ S ≻ P_N [BG11].
9. **The user sidecar stays on top** (ALIGN-compatible) and gains `Confirm`/`Reject` by constraint id.
10. **Measured, not argued.** A gold scoreboard (ID-01) runs over the 69 vendored ALIGN `*.const.json` files [ALIGN-GOLD] and `tests/corpus.rs`, catalog on against catalog off, and gates every migration step.

### 1.3 Catalog dependence today

| Constraint | Today's source | Catalog-dependent? |
|---|---|---|
| Symmetry pairs, selfs, net pairs, axes | leaf seeds, then propagation | **yes** (seeds) |
| Matched sets, ratios, unitization | MatchSym ∪ MatchBlock ∪ shared bias ∪ passive | partly (MatchBlock, BJT pairs) |
| Match kind, precision class | leaf kinds → role → class; spec; sidecar | **yes** when there is no spec |
| Array style, orientation | derived from class and set | inherits |
| LDE pairs, dummies | leaves; recognised blocks | **yes** |
| Proximity, hierarchy | declared roles, nets, instances | partly |
| Signal flow, current flow | `flow.rs` | no |
| Net classes | names, bulk, z, testbench; logic templates; leaves | partly |
| Differential routing, crosstalk, shield | compounds, Voltage sets, classes | inherits |
| Common node, star, Kelvin | sets plus op point | inherits |
| Parasitic budgets | class multiples; sensitivities | mostly no |
| Guard rings, isolation, balance | aggressor/victim tags; DiffPair blocks | partly |
| Thermal | op power plus sets | inherits |
| EM, IR, antenna | op currents; gate areas | no |
| ESD, latch-up | user-declared nets; port injectors | no, but mostly unidentified |
| Utilization, aspect | global config | not applicable |

---

## 2. Constraint by constraint

Each entry has five parts:
- **Protects**: the physics, in one line.
- **Today**: `file:line` evidence, and whether it depends on the catalog.
- **Research**: methods in the literature, with strengths and weaknesses.
- **Ideas**: what Philis could do.
- **Proposal**: inputs, algorithm, output, confidence, validation, effort and dependencies, pointing to a §3.4 plan item.

### 2.1 Symmetry: device pairs, self-symmetric devices, net pairs, axes

- **Protects:** symmetry cancels linear gradients (process, thermal, stress) to first order, and equalises the parasitics of the two halves of a differential path. That protects offset, even-order distortion, CMRR and PSRR [HAST ch.13; BG11 §3.2.1].
- **Today:**
  - **Seeds:**
    - DiffPair/Load/CascodePair leaves (`lib.rs:220-226`);
    - the sidecar (`lib.rs:228`);
    - identical-instance couples whose ports pair (`lib.rs:229-248`, `hier.rs:61-98`).
  - **Propagation** (`symmetry.rs:250-315`): each device takes its mutual unique best partner on mated nets, terminal by terminal, never chosen by id. A device is self-symmetric when every channel net is self-symmetric or a rail (`symmetry.rs:254-263`).
  - **Compounds:** a compound is a connected component through mated nets and channel-terminal self nets (`symmetry.rs:317-391`), with one axis per compound (`symmetry.rs:386-389`). It is Perfect when it contains an Exceptional Voltage set (`lib.rs:383-390`). Axis direction comes only from the sidecar (`lib.rs:251-253`).
  - **Catalog dependence: yes.** No seed means no compound. The WL labels exist (`pattern.rs:412-449`) but are only used to break ties.
- **Research:**
  - **Structural, library-based.** A sizing-rules pair library plus symmetry analysis, grouped into compounds that share an axis [MASS08; EICK10; EICK11; BG11 §3.2.2]. Arsintescu's symmetry extraction [ARS96]. Isomorphism/automorphism on a bipartite graph [HAO04]. Splitting the signal path from the bias first [ZHOU05].
  - **ALIGN** [ALIGN19; KUNAL20; ALIGN-FC]:
    - Works bottom-up and recursively, starting from every pair of non-power ports with equal weights. Power, ground and clock are stop points.
    - A SimGNN graph-edit-distance estimate handles approximate matches.
    - Results: 36 circuits, no false negatives, false positives in 4 (level shifters and dummies).
  - **MAGICAL signal-flow analysis** [MAG19]:
    - A diff, load or cascode pair seeds only when its shared source is a *virtual ground*, meaning not a rail.
    - It traverses along the current until the two paths meet.
    - Bias symmetry comes from common-gate and diode checks.
    - Each device gets at most one symmetry constraint.
  - **Spectral.** S³DET runs a Kolmogorov–Smirnov test on the Laplacian spectra of neighbourhoods sized by centrality. System level: 88.3 % accuracy, under 1.1 % false alarms [S3DET20].
  - **GNN:**
    - Supervised GraphSAGE using distance-to-VSS as a feature: TPR 0.917, FPR 0.74 % [GAO21].
    - Unsupervised: system-level F1 0.952, but device-level TPR 0.79 against MAGICAL's 0.84. Removing one resistor symmetry cost 3.1 dB SNDR [CHEN21].
    - GAT: F1 0.944 [XU24].
    - Learning constraints from expert layouts instead of labels [CG24].
  - **Simulation.** Symmetric elements show mirrored charge-flow (current) waveforms in an existing transient [DHAR22].
  - **Exact tools:**
    - nauty/Traces [NAUTY14] and bliss [BLISS07] for automorphisms.
    - Colour refinement runs in O((m+n) log n) [BBG13]. LVS netlist comparison is the same relabelling [GEMINI88].
    - Rust: `canonaut` (a pure-Rust port of nauty, 2026, very new), `nauty-pet` (C FFI), petgraph VF2 [CRATES].
  - **Trade-offs.**
    - Traversal methods are deterministic but need seeds, a library or tuned stop rules.
    - Spectral and GNN methods find candidates without seeds, but they score *similarity*, not consistency. They need thresholds or training data, and they do not check the electrical consequence.
    - I found no analog-layout work that uses nauty-style individualise-and-refine.
- **Ideas:**
  - **Strict symmetry as an involutive automorphism** of the coloured pin graph that fixes every rail and maps the port set to itself.
    - Its 2-cycles are the device pairs and the net pairs.
    - Fixed devices whose channel nets are fixed and not rails are the selfs.
    - The method is exact, needs no seed and does not depend on device order.
    - It finds fully differential stages, latches, StrongARM, Gilbert cells, VCO cores and rail-to-rail inputs.
    - Precedent: [HAO04] (automorphism) and [GEMINI88] (WL relabelling). Using individualise-and-refine to find involutions is speculative.
  - **Local symmetry for single-ended circuits.**
    - A 5T OTA has no automorphism, because the diode M3 differs from the output M4. Its load is still mirrored in layout.
    - Seed with two devices of the same signature that share a non-rail net on the same channel terminal (a virtual ground [MAG19]) and have distinct control nets.
    - Score each seed by the largest radius r over which the swap extends under the existing "mated or shared" rule (`symmetry.rs:85-101`). This is the radius-limited form of ALIGN's recursive comparison [KUNAL20].
  - **Port-pair seeds.** `Seed::Nets` already exists (`symmetry.rs:209-216`). Sources: names (`_p/_n`, `inp/inn`, `+/-`), `*.interface.json`, testbench AC sources in opposite phase, and complementary PULSE pairs. Precedent: ALIGN's equal-weight port pairs [ALIGN-FC].
  - **Electrical symmetry test** (speculative in this form):
    - Build the conductance matrix G at the op point from each device's gm, gds and gmb (`evidence.rs:35-46`).
    - A candidate involution P is electrical when ‖PGPᵀ − G‖/‖G‖ ≤ ε.
    - Equivalent stimulus form: drive the input pair with ±1 AC. Nets with v(x) ≈ −v(y) are net pairs; nets with v ≈ 0 are self nets (virtual grounds such as the tail).
    - Close to [DHAR22], which uses mirrored transient currents.
  - **Sensitivity antisymmetry** (speculative):
    - Per pair and per spec, a = |S_a + S_b| / (|S_a| + |S_b|), with S = ∂f/∂V_T from `Sensitivities::d_vt`.
    - a ≈ 0 means the pair carries the differential signal, so mismatch becomes offset.
    - a ≈ 1 means a common-mode role: the ratio matters, the mirror image does not.
    - Related: PARCAR derives matched node and branch pairs from sensitivities [CHOU93].
  - **Axis direction** (speculative): set the axis parallel to the signal-flow order (`flow.rs:27-66`). Stages then stack along the axis with the two halves on either side.
- **Proposal → ID-03, ID-04, ID-05.**
  - **In:** the normalised graph, `canon`, net classes, ports, and optionally `OpFacts` and `Sensitivities`.
  - **Algorithm:**
    1. Colour refinement with rails individualised and the DC-potential colour added.
    2. Search for involutions: individualise one device of a colour class of size ≥2, refine, and accept when the refined partition defines an involution. Cap the search nodes. No new dependency is needed; `canonaut` can serve as a test oracle.
    3. Generate virtual-ground and port-pair local candidates, scored by radius r.
    4. Order the seeds: User > automorphism > port pair > local (r descending, then canon) > catalog leaf (Hint).
    5. Run the existing `symmetry::analyze`.
    6. Run the electrical checks on each pair (ID-05).
  - **Out:** `Intent.compounds` keeps its shape. Each pair gains an `Origin` and a `Confidence` (ID-02).
  - **Confidence:**
    - automorphism: `Proven`;
    - local with r ≥ 3 plus op equality: `Corroborated`;
    - local only: `Inferred`;
    - catalog or LLM: `Hint`.
    
    A hard `SymmetryGroup` is emitted only at `Corroborated` or above.
  - **Validate**, with the catalog off:
    - `tests/align_gold.rs::strongarm_matches_align_gold` passes;
    - `tests/corpus.rs::compound_expectations` is unchanged for ota5t, three_stage, folded, gilbert, rail2rail, latch, strongarm and degen_pair;
    - `negative_corpus` (sc_switches, equal_fets, inv_chain) still has zero compounds;
    - `permutation_invariance` and `scale.rs` still pass.
  - **Effort and deps:** ID-03 M, ID-04 S, ID-05 M. Depends on ID-01 and ID-02.

### 2.2 Nested and hierarchical symmetry groups

- **Protects:** a group of symmetric groups (a Gilbert quad, two OTAs in a pseudo-differential stage) cancels gradients at every level. Islands stay compact.
- **Today:**
  - Compounds are flat; nested sets appear only as `Compound.set_pairs` (`sets.rs:350-382`).
  - Identical instances pair through `hier::same_template` (`hier.rs:61-66`); three or more become arrays (`hier.rs:68-89`). Both need `.subckt` boundaries.
  - A Gilbert quad is "two 2-member sets until a quad-set rule exists" (the `catalog.rs` ROLES comment).
- **Research:**
  - Hierarchical symmetry and symmetry islands [BG11 ch.2; KUNAL20].
  - ALIGN `CreateArray` when more than one valid group exists, and arrays when a net has more than 10 neighbours [ALIGN-FC].
  - System symmetry between sibling subcircuits [S3DET20; CHEN21]; reducing circuits to a unique canonical form [FEATS15].
  - HSMPG agglomeration [EICK11], already implemented at `graph.rs:122-187`.
- **Ideas:** the structure of the automorphism group *is* the hierarchy (speculative as an extraction rule):
  - two commuting involutions (Z₂×Z₂) form a quad;
  - an involution that swaps two connected components is a replica (SameTemplate) rather than a one-axis symmetry;
  - a cyclic generator with orbits of size ≥3 is an array or ring.
- **Proposal → ID-15.**
  - **In:** generators from ID-03.
  - **Algorithm:** group the generators by support. Commuting pairs become nested compounds. Component swaps become instance `MatchBlock` edges, with no subcircuit boundary needed.
  - **Out:** `Compound.parent: Option<u16>`, or a Symmetry `GroupNode` with children.
  - **Confidence:** `Proven`.
  - **Validate:** corpus `gilbert` gives one 4-member quad set; a flattened `two_ota` still pairs its instances.
  - **Effort:** M. **Deps:** ID-03.

### 2.3 Matched sets, ratios and unitization

- **Protects:** current ratios and voltage equality against random (Pelgrom) and systematic mismatch. Identical units make ratios exact [HAST rules 1 and 11; LAMP99 §4.2].
- **Today:**
  - Sets are components of the MatchSym ∪ MatchBlock edges (`sets.rs:211-336`). The edges come from:
    - **MatchBlock** from every pattern match's declared pairs (`graph.rs:69-84`). **Catalog-dependent.**
    - **Shared-bias stars** (`sets.rs:50-76`). Catalog-free: same kind, model and L; shared G and S; a diode or non-Signal gate; at least two drains.
    - **Passive sets** (`passive.rs:45-260`).
    - **BJT pairs** from the `bjt_*` templates (`lib.rs:259-262`). **Catalog-dependent.**
    - **Instance couples** (`graph.rs:93-97`).
  - Ratios and units: gcd of total widths on the grid, series units for unequal L, and the passive rules (`sets.rs:124-209`). Catalog-free.
- **Research:**
  - Pair library with generic constraints [MASS08]; constraint graph [EICK11].
  - Matching from sensitivity of performance to mismatch [CHAR93; MAL96].
  - Matched node and branch pairs from sensitivities [CHOU93].
  - Learned matching: heterogeneous GCN with mean F1 0.917 [ZHANG25]; edge classification [WU24 (U)].
  - ALIGN `GroupCaps`, `SameTemplate` and the `identify_array` flag [ALIGN-SCHEMA].
  - Mirrored current waveforms [DHAR22].
- **Ideas:**
  - **Equal-drive groups** generalise shared bias. Devices whose control-to-reference voltage is forced equal by shared nets (FET G and S, BJT B and E) are ratio-matched. When the op point shows every member in saturation, drop today's "diode or non-Signal gate" filter.
  - **Translinear loops** (the principle is [GIL75]; using it as a layout identification rule is speculative):
    - A KVL loop through at least two junctions of opposite orientation (B–E, or G–S in subthreshold), plus resistors, defines a ratio-critical set.
    - Covers bandgap ΔV_BE, PTAT, log/antilog and multipliers.
    - Replaces the `bjt_ratioed_pair_*` patterns.
  - **Sets found by sensitivity** (speculative): devices of one signature whose `d_vt` rows rank in the top quantile of spec variance and are anti-correlated across specs. They form a matched set even when no structure links them.
  - **Op-point clustering:** same signature, same region and |ΔV_GS| < 1 mV, plus a shared net, makes an `Inferred` candidate.
- **Proposal → ID-08 (translinear), ID-10 (sensitivity).**
  - **In:** the graph, `drawn`, `OpFacts`, `Sensitivities`.
  - **Algorithm:**
    - (a) Equal-drive groups, as today, relaxed by the op point.
    - (b) Translinear search: cycles in the graph of junction and resistor edges over non-rail nets.
    - (c) `d_vt` anti-correlation over the same-signature devices left unmatched.
  - **Out:** `MatchSpec` with `Origin::{SharedBias, Translinear, Sensitivity}`. `unitize` is unchanged.
  - **Confidence:**
    - equal-drive: `Proven`, since V_GS identity is structural;
    - translinear: `Corroborated` when the op point shows the junctions forward-active;
    - sensitivity: `Inferred`.
  - **Validate:** corpus `brokaw` and `bgr_core` keep their sets with the `bjt_*` patterns removed; `mirror6` and `sets::tests::three_stage_bias_group` are unchanged.
  - **Effort:** M each.

### 2.4 Matching precision class and match kind

- **Protects:** layout effort (common centroid, dummies, aspect ratio, distance) is spent where the spec needs it. Uses Hastings' minimal, moderate and exceptional classes [HAST §13.3].
- **Today:**
  - Class = User > Spec (6σ against the class limit) > Role (`class.rs:48-85`).
  - Role comes from leaf kinds (`lib.rs:325-350`). **Catalog-dependent.**
  - Kind is Voltage only for a DiffPair leaf or a BJT diff/ratioed template (`class.rs:32-46`). **Catalog-dependent.**
  - Sensitivities set the allowance and weight. A set under `minor_weight` becomes Minimal (`lib.rs:314-324, 357-360`; `allocate.rs:38-95`).
- **Research:**
  - A sensitivity-weighted offset budget split over the pairs [LAMP99 LAMP-09].
  - Mismatch-to-performance sensitivity [CHAR93].
  - Worst-case distance and yield [GRAEB07].
  - No source infers the class from structure alone.
- **Ideas:**
  - **Kind from structure, not from leaves.** All three predicates use facts that already exist.
    - Voltage: the members are a compound pair whose control nets are a mated net pair, so the signal enters on the controls.
    - Current: the members share their control net (equal V_GS).
    - Ratio: passives.
  - **Kind from sensitivity** (speculative): antisymmetry a ≈ 0 means Voltage; a ≈ 1 means Current.
  - **Role from signal-flow level:**
    - InputPair: a Voltage set at level 1 (`flow.rs:27-66`).
    - LoadOfPair: a set whose channel nets are the InputPair's mated drain nets.
    - BiasMirror: a shared-bias set gated by a Bias net.
  - **Class when there is no spec** (speculative):
    - Rank the sets by input-referred offset contribution |d_vt|/gain.
    - One DC sensitivity run gives d_vt: ngspice `.sens` [NGS], or the existing finite differences.
    - The top set is Moderate; sets below 10 % of the top are Minimal.
- **Proposal → ID-06.**
  - **In:** compounds, flow levels, shared-bias groups, and optionally `d_vt`.
  - **Algorithm:** replace the leaf test in `kind_of` and the `roles` vector (`lib.rs:300-305`) with the predicates above. Keep User > Spec.
  - **Out:** `MatchSpec.kind`, `class` and `class_source`.
  - **Confidence:** `Corroborated` when the predicate and the sensitivity agree, otherwise `Inferred`.
  - **Validate:** with the catalog off, `corpus.rs::class_sources` and `exceptional_voltage_compound_is_perfect` are unchanged.
  - **Effort:** S. **Deps:** ID-04.

### 2.5 Common-centroid, interdigitation and array style

- **Protects:** a 1-D common centroid cancels linear gradients; a 2-D one cancels planar gradients. Interdigitation cancels 1-D gradients for current sets with a shared source [HAST rule 8; CCREV].
- **Today:** derived by `style_of(class, kind, shares_source)` (`class.rs:87-98`, `lib.rs:361-367`). Catalog-free once the sets exist.
- **Research:**
  - CC layout for active and passive devices and for DAC arrays [CCREV; DACCC (L)].
  - Nth-order central symmetry [NTH (L)].
  - CC as an exact equality [BG11 eq. 3.15].
  - ALIGN expresses CC through `GroupCaps` and `PlaceSymmetric`; there is no CommonCentroid class [ALIGN-SCHEMA].
- **Ideas** (speculative): pick the style by the gradient term's share of the set's allowance. If an adjacent layout's S_VT·distance already fits, Adjacent is enough even at Moderate. The numbers already exist in the `MatchedSet` budget.
- **Proposal:** no new identification; it inherits ID-06 and ID-10. Existing test: `class::tests::style_by_class`.

### 2.6 Orientation

- **Protects:** equal current direction cancels orientation-dependent mobility, stress and tilt-implant asymmetry [HAST rule 7; LAMP99 LAMP-12].
- **Today:** one `OrientationSet` per MOS or bipolar set: Axis is hard, Φ depends on class (`emit.rs:193-203`). Catalog-free once the sets exist.
- **Research:**
  - Joint reorientation of matched groups [LAMP99].
  - Mirror and perfect symmetry types [BG11 BAL1-02].
  - ALIGN's `fix_source_drain` flag [ALIGN-SCHEMA].
- **Ideas:** take the physical drain and source from the sign of the op current I_D (bidirectional switches, transmission gates). Φ then compares actual current direction instead of netlist pin names. ALIGN fixes S/D the same way.
- **Proposal:** part of ID-02's normalisation. When the op point shows a member's I_D reversed, swap its D/S for the Φ check and report `current_reversed`. Test with a constructed pass-gate pair. **Effort:** S.

### 2.7 Layout-dependent effects (WPE, LOD/OSE) and dummies

- **Protects:** well-edge proximity and STI stress shift V_T and mobility differently in matched devices. Bias points can move 20–30 % [DREN06 (U); HAST ch.13].
- **Today:**
  - Set members get dummies by class (`constraints.rs:48-50`, used at `constraints.rs:106`).
  - But every device in a recognised block that is not in a set still gets `dummy_required: true` (`constraints.rs:119-146`). **Catalog-dependent.**
  - WPE/OSE pairs exist only for DiffPair/CurrentMirror/Load leaves (`frontend/library/src/lib.rs:2783-2790`). **Catalog-dependent.** The thresholds are fixed at the Moderate and Minimal tiers (`lib.rs:2772-2773`).
- **Research:**
  - LDE-aware analytical placement with all three effects modelled [OU15].
  - Sensitivity analysis chooses which LDE constraints matter [TZ20]. This is the identification step.
  - Dummies by matching class [HAST rule 12]; ALIGN removes dummies before analysis [ALIGN-SCHEMA].
- **Ideas:**
  - A device is LDE-critical if it is a member of a Moderate+ set, or if its |∂f/∂V_T| is in the top quantile. Convert ΔV_T to ΔI_D with ∂I_D/∂V_T = −gm, which uses op data that already exist.
  - Take thresholds from each set's class, not a fixed Moderate.
- **Proposal → ID-07.**
  - **Change:**
    - Environment pairs come from compound pairs and each set's (reference, member) couples.
    - The WPE threshold is `mos_env(set.class)`.
    - Dummies come only from sets; devices that are recognised but in no set get none.
  - **Confidence:** inherits the set's.
  - **Validate:** the corpus `dac4` inverters get no dummies; `ota5t` keeps its dummies and environment pairs; `bench local` stays at DRC 0 and LVS MATCH.
  - **Effort:** S. **Deps:** none.

### 2.8 Proximity and hierarchy clusters

- **Protects:** short connections (less R and C), shared wells and rings, and a smaller gradient distance between related devices [BG11 §3.2; EICK11].
- **Today:**
  - Requirement graph (`graph.rs:25-120`):
    - ProxBlock edges from declared roles (**catalog-dependent**), sidecar groups and arrays;
    - ProxNet stars per non-rail net, up to `pn_max_degree`, never crossing an instance boundary.
  - HSMPG (`graph.rs:122-187`).
  - Stack-leaf and stage-self pulls (`emit.rs:210-223`). **Catalog-dependent.**
  - Minimal-set pulls (`emit.rs:203-208`).
- **Research:**
  - HSMPG agglomeration in importance order [EICK11].
  - Functional hierarchy recognised by graph homomorphism [GL23].
  - ALIGN `GroupBlocks` and `PlaceCloser` [ALIGN-SCHEMA]; the local gold has 55 `GroupBlocks` entries.
  - Placement by system signal flow [SF20].
- **Ideas:**
  - **Electrical affinity** (speculative): weight each ProxNet edge by the net's normalised |∂f/∂C| (`d_c`). Without specs, weight by 1/z and current share. Run the existing Algorithm 3.1 on the weighted edges.
  - **Current-loop clusters** (speculative): devices in the same high-AC-current loop (output stage, its decoupling, its rail) belong together. The loops come from AC branch currents of one probe run.
  - **Cascode stacks without templates:** a FET whose source is another FET's drain, with no other channel device on that net and a Bias gate, gets Stack proximity. `DeviceRole::Cascode` already exists (`evidence.rs:139-143`).
- **Proposal:** Stack pulls go into ID-06 (S); weights go into ID-11 (M).
  - **Validate:** with the catalog off, `graph::tests::ota5t_tree` and `tail_in_two_requirements` are unchanged, and `folded` keeps its four Stack pulls.

### 2.9 Signal-flow and current-flow ordering

- **Protects:** short, monotone signal paths mean less coupling and cleaner routing. Stacked current branches keep vertical connections short.
- **Today:** catalog-free.
  - `stage_order`: BFS from control-only input nets (`flow.rs:22-66`).
  - `current_paths`: ground-to-supply chains, with idle branches pruned using op weights (`flow.rs:68-170`).
  - Sidecar orders come first (`lib.rs:432-457`).
- **Research:** in the literature the flow is usually given as an input, not inferred:
  - signal paths "specified … following guidance from experienced designers" [SF20];
  - monotonic current paths are given as input [WU12];
  - current-flow constraints on critical nets [OU13].
  
  Two exceptions infer it: MAGICAL traverses current from source to drain [MAG19], and Eick identifies signal paths structurally [EICK11]. Charge flow from a transient weights nets [DHAR22]. The local gold has 25 ALIGN `Order` entries [ALIGN-GOLD].
- **Ideas** (speculative): order nets by AC transfer magnitude from the input, e.g. level = ⌈log₁₀|H|⌉ from one AC probe. This fixes feedback loops where BFS levels collapse.
- **Proposal:** ID-01 measures the result against the 25 gold `Order` entries. Improvement is optional.

### 2.10 Net classes (supply, ground, clock, sensitive, bias, reference, noisy, digital static/switching, substrate)

- **Protects:** routing priority, spacing, shields, budgets and the aggressor/victim maps all key on the class [LAMP99 LAMP-48].
- **Today:**
  - **Rails:**
    - names, `0`, `!` and the root list (`netrole.rs:36-53`);
    - bulk inference when no rail is named (`netrole.rs:82-109`);
    - testbench DC extremes (`lib.rs:687-702`).
  - **Clock:** names (`netrole.rs:113-118`) and testbench switching (`lib.rs:700`).
  - **Structure:** Substrate if bulk-only. Sensitive if gates-only, or the gate of a "sensitive" leaf device (`classify.rs:347-373`; the leaves come from `lib.rs:189-196`, **catalog-dependent**).
  - **Refine step:**
    - DigitalSwitching and DigitalStatic from the LOGIC templates (`classify.rs:153, 207-235`). **Catalog-dependent.**
    - Noisy from `charge_pump_cell` (**catalog-dependent**) and from `_n` names (`classify.rs:236-246`).
    - Reference from bandgap sets, the `cascoded_reference` template and DAC references (`classify.rs:247-268`).
    - Sensitive from DAC plates, `_s` names and z ≥ 100 kΩ (`classify.rs:269-282`).
    - Bias from gate/diode-only nets (`classify.rs:158-174, 283-286`) and current-source gates (`lib.rs:466-475`).
- **Research:**
  - The four ILAC classes [LAMP99 §1.5.2.4]; net susceptibility classes [BG11 §3.1.3].
  - Routing with shields based on net classification [GAO10].
  - In ALIGN, `PowerPorts`/`GroundPorts`/`ClockPorts`/`NetConst`/`NetPriority` are user inputs, and clocks are stop points [ALIGN-SCHEMA]. Even ALIGN does not infer net classes.
  - GANA takes net type as an input feature [GANA20]. S³DET propagates analog/digital labels upward [S3DET20].
  - Voltage propagation from supplies and inputs assigns domains and on/off states [PERC].
- **Ideas:**
  - **Static CMOS by duality** (replaces LOGIC; speculative as a classifier):
    - For each net, find the PMOS network to Supply and the NMOS network to Ground.
    - If both exist, use the same gate nets, and are series/parallel duals, the net is a static CMOS output.
    - Covers inverter, NAND, NOR and AOI, in linear time per net.
  - **Classes from simulation** (ALIGN leaves these to the user [ALIGN-SCHEMA]; the rules here are speculative):
    - Supply/Ground: an ideal source node.
    - Clock: a PULSE source or a full-swing toggle.
    - Bias: DC-only with |∂v/∂v_in| ≈ 0.
    - Signal: non-zero transfer from an input.
    - Virtual ground: a shared non-rail source net [MAG19].
  - **Noisy is a switched node:** a channel net of a `DeviceRole::Switch` with a Clock or Digital gate, or any net with large C·|dV/dt| in the transient. Replaces `charge_pump_cell`.
  - **Reference by PSRR probe** (speculative): put an AC source on each Supply. A net with non-zero DC, |v/v_dd| ≪ 1 and low z that drives gates is a Reference.
  - **Sensitive by criticality:** the top quantile of Σ_j |∂f_j/∂C| / margin_j when sensitivities exist [CHAR94]; otherwise the z-rule, which exists today.
- **Proposal → ID-09.**
  - **Out:** `NetClassification.class` and `NetFacts.evidence`. `EvidenceLevel` already has Structure, OpPoint and Testbench.
  - **Confidence:** the evidence level.
  - **Validate**, with LOGIC, `charge_pump_cell` and `cascoded_reference` deleted:
    - `dac4` d0–d3 are DigitalStatic;
    - `inv_chain` is all digital;
    - the `strongarm` latch nodes stay Sensitive (Voltage-set gates are classified first, `classify.rs:224-227`).
  - **Effort:** M. **Deps:** ID-06.

### 2.11 Differential-pair routing

- **Protects:** equal R, C and coupling on both nets of a differential signal, so common-mode noise stays common [LAMP99 LAMP-40].
- **Today:** one `Differential` per compound non-rail net pair, in the budget arm (`extract.rs:111-119`), plus an asymmetric-degree report (`conflict.rs:20-25`). Inherits the catalog through compounds.
- **Research:**
  - ALIGN `SymmetricNets` (17 entries in the local gold) and MAGICAL `.symnet` files [ALIGN-SCHEMA; MAG19].
  - Matched node and branch pairs from sensitivity [CHOU93].
  - Symmetric routing needs symmetric placement [BG11 §3.1.2].
- **Ideas:** confirm by AC antisymmetry (§2.1). This separates real differential nets from mated bias nets: both halves' cascode-bias nets are mated but carry no signal, so they need matched routing but not pair routing.
- **Proposal:** inherits ID-03 and ID-05. `Differential` becomes hard only at `Corroborated`. Rename `corpus.rs::differential_comes_from_recognized_pairs` to `..._from_compounds`; it must be unchanged with the catalog off. **Effort:** S.

### 2.12 Crosstalk and total coupling

- **Protects:** capacitive injection from switching nets into high-Z or sensitive nets, including the sum of many weak aggressors [LAMP99 LAMP-39].
- **Today:**
  - `CrosstalkExclusion` for each Voltage set's G×D, and for every routed victim against every routed aggressor (`extract.rs:121-143`).
  - `CouplingBudget` per budgeted net, with aggressor weights (`extract.rs:179-191`).
  - Per-pair `d_cc` when the performance flow runs (`evidence.rs:71-72`; `perf.rs::coupling_params`).
- **Research:**
  - PARCAR splits sensitivity by sign (S⁺, S⁻), keeps bounds as loose as possible, and drops negligible parasitics [CHOU93; CHAR94]. The full Berkeley flow [MAL96].
  - Learned C prediction: R² 0.772 [PARA20]. Guidance from post-layout performance [XU25].
  - In GeniusRoute, clock routing alone moved a symmetric comparator's offset between 300 and 750 µV [GR19]. Ground-C sensitivity alone misses this.
  - Computed RC trade-offs beat designer net annotation [WS22].
- **Ideas:** rank each (victim, aggressor) pair by A_j·S_i and keep only pairs above a share of the victim's budget. Today every pair is emitted, which is O(V·A).
  - A_j, aggressor strength: swing × f, or max|dV/dt| from the testbench transient.
  - S_i, victim susceptibility: |∂f/∂C_c(i,j)|, or z_i·ω.
  
  This is the product form of [CHO08].
- **Proposal → ID-11.**
  - **Out:** fewer, weighted `CrosstalkExclusion` rows; `Aggressor.strength`.
  - **Confidence:** `Corroborated` with a transient, otherwise `Inferred` from the class.
  - **Validate:** fewer rules on `zz_strongarm` with every victim still covered; signoff unchanged.
  - **Effort:** M. **Deps:** transient waveforms (FLOW).

### 2.13 Shielding

- **Protects:** a grounded shield turns victim–aggressor coupling into victim–ground capacitance.
- **Today:** a `Shield` on every victim net whenever any aggressor exists. The return is an analog ground found by name, else an untouched ground, else a shared ground with a diagnostic (`extract.rs:160-205, 214-220`).
- **Research:**
  - Shield or space chosen by sensitivity [MAL96]; shielding driven by net class [GAO10].
  - ALIGN `NetConst` (shield) is a user input [ALIGN-SCHEMA].
- **Ideas:** shield only where ID-11's ranked coupling at minimum spacing exceeds the victim's budget. Choose the quiet return by the switching current measured on each ground net, not by name.
- **Proposal:** part of ID-11 (S extra).

### 2.14 Common-node, star and Kelvin connections

- **Protects:** shared return resistance turns one member's current into another member's V_GS error. Separating force and sense keeps sense paths free of current.
- **Today:**
  - Common nodes per set that shares S/E. A star when non-member current exceeds 1 % (with an op point) (`extract.rs:222-293`).
  - Kelvin only for a resistor between a Voltage set's gates (`extract.rs:294-308`).
  - Inherits the catalog through sets.
- **Research:**
  - Current-density-aware P&R [OU13]; multiport terminals with EM and IR [MART14].
  - Terminal current vectors that satisfy KCL [LIEN06].
- **Ideas:** force/sense split by KCL (speculative).
  - On every net, split the terminals into current-carrying ones (|I| from `oppoint.rs:88` `terminal_ua`) and zero-current ones (gates, high-Z inputs).
  - When the IR drop at the budgeted wire R exceeds a share of the sensing set's allowance, emit a Kelvin request with the gates as the sense side.
  - Generalises today's resistor rule to supply-referenced mirrors, sense resistors and bandgap emitters.
- **Proposal → ID-13.**
  - **Out:** `Intent.kelvins` and `stars`.
  - **Confidence:** `Corroborated`.
  - **Validate:** `mirror6` with a 1 mA leg gets a Kelvin; `ota5t` is unchanged.
  - **Effort:** S. **Deps:** op currents.

### 2.15 Parasitic budgets, performance-driven constraints and net criticality

- **Protects:** specs against wire R, C and coupling, spending margin where it matters [LAMP99 ch.2; CHOU93].
- **Today:**
  - Class multiples of the driven gate load (`classify.rs:25-35, 102-131`).
  - Sensitivity rows and R/C classes when the performance flow runs (`budget.rs:28`, `lib.rs:493-507`).
  - Allowances (`allocate.rs`).
  - Mostly catalog-free.
- **Research:**
  - **Constraint generation by sensitivity plus optimisation:** [CHOU93; CHAR93; CHAR94; MAL96], sensitivity-weighted placement cost [LAMP95], and the flexibility QP [LAMP99 LAMP-05]. In the router, rows are reweighted by sensitivity × violation [CHAR94].
  - **ALIGN** chains a maximum parasitic per node into maximum R, length and vias, but rejects simulation-heavy sensitivity as too costly [ALIGN21].
  - **Adjoint networks** give all sensitivities from two solves [DR69].
  - **Learned:** [PARA20; XU25; WS22]. Data collection is 92.9 % of the cost of an ML model [ASPDAC24].
- **Ideas:**
  - **Adjoint ∂f/∂C for every node from two AC solves** per spec and frequency: ∂H/∂C_k = −jω·v_k·v̂_k, where v̂ is the solution of the transposed network with the stimulus at the output [DR69]. This replaces 2N finite-difference runs. Building the transposed network by netlist transformation (R, C, VCCS) is speculative for the ngspice flow.
  - **Budgets PARCAR-style:** Σ S⁺·C ≤ margin per spec, with an equal-share or sensitivity-proportional split, and nets below a cumulative cutoff marked non-critical [CHAR94]. Philis has the rows already, so this is policy, not new identification.
  - **Learned prior** for circuits without a testbench (speculative): a small offline model maps (class, z, fan-out, flow level) to criticality, trained on Philis' own sensitivity runs. It enters as a `Hint`.
- **Proposal → ID-16** (adjoint, library-side) and ID-17 (the prior as a hint).
  - **Validate:** adjoint and finite-difference `d_c` agree within 5 % on `ota` and `tt_ota`.
  - **Effort:** M.

### 2.16 Guard rings (injector, aggressor, victim) and substrate isolation

- **Protects:** against minority-carrier injection and substrate bounce reaching sensitive bulks, and against latch-up triggers [CHARB01; HAST ch.14].
- **Today:**
  - Aggressors are devices on Clock, DigitalSwitching or Noisy nets, or capacitor-coupled to them (`substrate.rs:23-40`).
  - Victims are members of Moderate+ sets and FETs gated by Bias or Reference nets (`substrate.rs:41-62`).
  - Injectors are diffusions within `inj_series_ohm` of a pin, and forward-biased bulks (`substrate.rs:66-143`).
  - Rings by role (`rings.rs:48-60`).
  - Isolation and substrate balance (`emit.rs:268-309`). Balance uses DiffPair blocks, so it is **catalog-dependent**.
- **Research:**
  - **Injection → propagation → reception,** with specs turned into coupling bounds [CHARB01].
  - **Aggressor ranking** by per-gate injection waveform × switching activity [SUBW99]; review of coupling paths [VA98].
  - **Floorplanning:** noise N = Σ CG_ij·√∫S_i|H_j|², an explicit aggressor × victim product [CHO08]. Substrate-aware placement takes the roles as input [WRIGHT94].
  - **Localised rings in aggressor cells** cut noise by about 72 % [SF08].
  - **Latch-up ring checks** use aggressor "danger zones" [PERC-LU].
- **Ideas:**
  - **Aggressor strength:** switching charge C_drain·ΔV, or |dI/dt| from the transient. Rank instead of a yes/no flag.
  - **Victim susceptibility by substrate probe** (speculative): an AC source on each bulk or substrate net, measuring |H| to the outputs. A cheap proxy is gmb·|∂f/∂V_T| from the existing `d_vt` and op `gmb_us` (`evidence.rs:44`).
  - **Ring placement:** ring the aggressor when its term dominates, the victim otherwise [SF08].
  - **Balance:** pull aggressors onto the bisector of every `Corroborated` compound, not only DiffPair blocks.
- **Proposal → ID-11** (ranking), plus a one-line change in ID-18 (balance from compounds).
  - **Validate:** the `drv` tests (`substrate.rs:186-269`) are unchanged; the `zz_strongarm` clocked tail stays related (AA-13).
  - **Effort:** M.

### 2.17 Thermal

- **Protects:** thermal gradients from power devices shift matched pairs through the V_T and V_BE temperature coefficients [HAST ch.5; LAMP99 LAMP-13/14].
- **Today:**
  - `heat::separations` from op power per cell and the Moderate+ sets (`frontend/library/src/lib.rs:893-910`).
  - EM temperature from power (`lib.rs:950-968`).
  - `d_t` is left empty (`evidence.rs:69-70`).
- **Research:**
  - Thermal-driven placement with matching. Power devices are a manual input, and matched groups are put on isotherms [LIN09].
  - Thermal-offset sensitivity found by simulating per-device temperatures [LAMP99 LAMP-14].
- **Ideas:**
  - **d_t without new simulations** (speculative, first-order, ignores the mobility TC): ∂f/∂T_d ≈ (∂f/∂V_T,d)·(dV_T/dT), using the existing `d_vt` and the deck's |dV_T/dT|.
  - **Heat sources** by share of op power (exists). "Id·Vds much larger than the block median" is the same rule.
- **Proposal → ID-12.**
  - **Out:** `SpecSens.d_t` filled in `perf::to_evidence`.
  - **Validate:** on `ota` with a 10 mW output device, the input pair's thermal weight exceeds the mirror's.
  - **Effort:** S. **Deps:** sensitivities.

### 2.18 Electromigration and current bounds

- **Protects:** wire and via lifetime under DC, RMS and peak current [LIEN18].
- **Today:** one hard EM rule per routed net, from op terminal currents (`frontend/library/src/lib.rs:2605-2642`). RMS and peak are deferred (plan-06 REL-11). Catalog-free.
- **Research:**
  - **Which current to check** [LIEN06]:
    - RMS for analog DC nets;
    - average for bidirectional digital nets;
    - peak for ESD.
  - Terminal min/max current vectors that satisfy KCL; wires shorter than the Blech length are exempt [LIEN06].
  - Widths from current [OU13; MART14].
- **Ideas:** with fixed terminal currents, KCL fixes each tree edge's current uniquely. Nets that switch (switch nodes, output stages) need one `.tran` run to get RMS and peak.
- **Proposal:** waits on transient evidence (FLOW). The identification change is a per-net `current_kind {Dc, Rms, Avg}` from the net class: analog nets RMS, digital nets average.

### 2.19 IR drop

- **Protects:** against supply or return drops that push devices out of saturation or shift references.
- **Today:** `ir::budgets` from net current and headroom (`ir.rs:14-61`, called at `frontend/library/src/lib.rs:992-1000`). Catalog-free.
- **Research:** IR from branch currents [LIEN18 §3.3; MART14]; IR-aware power routing [WANG19].
- **Ideas:** budget each rail by ∂f/∂R_rail (`d_r` rows already exist). Rails that feed Reference or Bias generators get tighter budgets.
- **Proposal:** folded into ID-16 (sensitivity rows on rails). **Effort:** S.

### 2.20 Antenna

- **Protects:** thin gate oxide against plasma charging damage, which shifts V_T and so is also mismatch.
- **Today:** one hard `Antenna` per gate net over its gate area (`extract.rs:74-98`). Catalog-free.
- **Research:** a gate is at risk when its net has no diffusion below the current layer and the metal-to-gate ratio exceeds the limit. Fixed by diode or jumper insertion [HUANG04; JIANG06].
- **Ideas:** a tighter margin for gates of Moderate+ sets, since antenna damage there becomes mismatch (policy, speculative).
- **Proposal:** margin by set class in `policy.rs`. **Effort:** S.

### 2.21 ESD

- **Protects:** pad discharge paths (HBM, CDM): a clamp must exist, path resistance must be low, metal must be wide enough.
- **Today:** `EsdWidth` only for nets the user declares (`Config::esd`, `frontend/library/src/lib.rs:978-990`). There is no ESD class, and plan-06 REL-17 notes that pads and clamps are not recognised.
- **Research:**
  - ESDA checks [ESDA-TR18]:
    - an ESD device between pads;
    - pad-to-clamp resistance;
    - a primary path for every pin pair;
    - unintended parallel paths;
    - unprotected input gates and output drains;
    - inter-domain and CDM checks.
  - PERC recognises structures and propagates voltage from supplies and inputs [PERC].
  - Si2 OpenPDK has an ESD flow methodology [SI2-ESD].
  - ALIGN has no ESD constraint.
- **Ideas:**
  - **Ports are pads** when the netlist is top-level. A non-rail port touching a gate needs a secondary clamp or series R; touching a diffusion makes it a primary path.
  - **Clamp detection by structure:** a diode, or a FET with G=S=B on a rail and D on the port.
  - **Cross-domain gates:** a gate net driven from a different supply domain, found by voltage propagation through channels, gets a CDM check flag [PERC].
- **Proposal → ID-14.**
  - **Out:** `NetFacts.esd: Option<EsdRole>`, and diagnostics for unprotected ports.
  - **Confidence:** `Inferred`, since ports may not be pads; the sidecar confirms.
  - **Validate:** the `tt_ota.interface.json` ports get classified; signoff is unchanged.
  - **Effort:** M.

### 2.22 Latch-up

- **Protects:** against parasitic SCRs triggered by minority carriers injected near wells of the opposite type [HAST ch.14].
- **Today:**
  - Injector tags from pins and forward-biased bulks (`substrate.rs:66-143`).
  - Ring rows 1–4 (`rings.rs:48-60`).
  - Bulk-forward voltage findings (`frontend/library/src/reliability.rs:25-36`).
- **Research:**
  - A PERC check suite of 14 checks [PERC-LU]:
    - first and second rings exist;
    - ring width;
    - maximum aggressor-to-ring spacing;
    - unprotected victims in a danger zone around aggressors;
    - N-ring tied to VDD, P-ring tied to VSS.
  - External latch-up is injection from I/O; internal latch-up is handled with N-to-P spacing and tap density [ESDA-LU].
- **Ideas:** voltage propagation gives each well a potential range. An injector's well adjacent to an opposite-type well in a different domain then needs a spacing or ring constraint between those cells (speculative extension of REL-07).
- **Proposal:** part of ID-14 (S extra).

### 2.23 Utilization and aspect ratio

- **Protects:** area, fit to the floorplan slot, and routing congestion (driven by aspect ratio).
- **Today:** `Config.min_utilization` 0.6 (`frontend/library/src/lib.rs:105-108, 924-929`). Sidecar `AspectRatio`/`Boundary` are unsupported (`sidecar.rs:4-6`).
- **Research:**
  - Shape functions and Pareto fronts of aspect ratio [BG11 §3.4].
  - Floorplan measures used as sizing objectives are measured, not inferred [LOUR15].
  - ALIGN `AspectRatio`/`Boundary` are user inputs (2 `AspectRatio` entries in the local gold).
  - **No source infers these from the circuit.**
- **Ideas** (speculative): seed the aspect from structure. Columns ≈ number of signal stages; rows ≈ deepest current path (`flow.rs`); ×2 across the symmetry axis. Only a starting point for dp.
- **Proposal:** read sidecar `AspectRatio`/`Boundary` into the existing utilization rule (FLOW/PLC). Nothing to identify.

### 2.24 Learned and LLM proposals (cross-constraint)

- **What exists:**
  - GNN annotation and hierarchy [GANA20; GANA23]; GNN symmetry [GAO21; CHEN21; XU24]; matching [ZHANG25].
  - Learning from layouts [CG24; TAG22; RET18; YAO23]; placement quality prediction [PQP20; PEA20]; parasitics [PARA20]; routing guidance [GR19; XU25].
  - Subcircuit matching: a hypergraph NN plus exact matching [SMART25].
  - LLM agents and generators [LC24; LLANA24; PANDA26; ICLAD26; GENIE25].
  - Survey: [ZHU22].
- **Reported quality:**
  - LayoutCopilot: 96.8 % sanity pass over 1,250 requests, 92 % fully correct on 25 cases, with hand-written guardrails such as "a device cannot be in two symmetry pairs" [LC24].
  - GENIE-ASI: F1 1.0 on simple subcircuits, 0.31 on complex ones [GENIE25].
  - Direct prompting hallucinates [ATLAS26]; MLLMs are limited on AMS reasoning [AMSB25].
  - PANDA gives no accuracy figures [PANDA26].
- **±:** they cover shapes no rule anticipated. But they need data, return probabilities rather than proofs, and are not permutation-invariant by construction.
- **Use in Philis:**
  1. An offline producer writes sidecar JSON with a `confidence` and `source` per entry.
  2. The annotator ingests these as `Hint` facts.
  3. A hint becomes a constraint only when ID-03, ID-05 or ID-10 corroborate it.
  4. Uncorroborated hints are listed for the user to `Confirm`.
- **Proposal → ID-17.** **Effort:** S in tree; L for any model, which lives out of tree.

---

## 3. Cross-cutting

### 3.1 Provenance and confidence

- **`Origin`** (exists, `kernel/analog/src/intent.rs:19-31`) gains `Automorphism`, `LocalSymmetry { radius: u8 }`, `PortPair`, `Translinear`, `Electrical { test: &'static str }`, `Sensitivity` and `Hint { source: &'static str }`. `Pattern { template }` stays, for naming and Hint seeds.
- **New `Confidence { User, Proven, Corroborated, Inferred, Hint }`** on compound pairs (per pair), `MatchSpec`, `Aggressor`/`Victim`, `KelvinReq` and `StarReq`. For nets, `NetFacts.evidence` (`EvidenceLevel`) already plays this role.
- **Arm policy**, as one table in `policy.rs`:
  - `User`, `Proven`, `Corroborated` → hard (where the family has a hard arm) + budget + cost;
  - `Inferred` → budget + cost;
  - `Hint` → cost only, and listed in the report.
- **Conflict precedence**, extending `conflict.rs`: User ≻ confidence ≻ M_S ≻ M_B ≻ P_B ≻ S ≻ P_N [BG11 eq. 3.19] ≻ canonical order. One symmetry pair per device stays a hard invariant (already in `symmetry.rs`; also in [MAG19; LC24]).
- **Report:** for each device, the strongest origin that reaches it. "no pattern" becomes "no structural, electrical or user evidence".

### 3.2 How the catalog shrinks

1. **Today:** the catalog provides seeds, roles and classes.
2. **After ID-04, ID-06, ID-07 and ID-09:** catalog matches only add `Hint` seeds. On the corpus, every constraint has a non-catalog origin.
3. **After ID-18:**
   - Patterns run last, on the final compounds and sets, to **name** blocks (`Block.template`) for reports.
   - `AnnotationConfig.catalog = NamingOnly` is the default; `Full` is kept for one release for A/B comparison.
   - The `ROLES` table is deleted once ID-01 shows no recall loss.
   - Keep petgraph VF2 [VF2; CRATES] or the existing matcher (labelling-driven, as in [SUBG93]) only for naming.
4. A later option is to replace the flat catalog with a compositional pair library plus composition rules [MASS08], used for naming only.

### 3.3 User sidecar

- **It stays first in every precedence** (`lib.rs:228`, `classify.rs:204-206`, `class.rs:55-56`).
- **Add `Confirm { id }` and `Reject { id }`.** Keys are the stable `ConstraintId` plus the origin text; ids do not depend on device order (`corpus.rs::ids_survive_permutation`). A user can then accept a Hint, or veto an Inferred pair, without restating it.
- **Read more of ALIGN's vocabulary:**
  - `SameTemplate`, `NetConst` (shield, criticality), `NetPriority`, `AspectRatio`, `Boundary`, `GuardRing`, `ChargeFlow`;
  - the `ConfigureCompiler` flags: `merge_series_devices`, `merge_parallel_devices`, `remove_dummy_devices`, `fix_source_drain`, `identify_array` [ALIGN-SCHEMA].

### 3.4 Migration plan

Items are in order. Each keeps `tests/corpus.rs` green except for the rows it edits, following the EXT-01 convention.

#### ID-01 Identification scoreboard (gold corpus, catalog on/off)
- **Why:** there is no measure of precision or recall per family. `align_gold.rs` covers 1 of the 69 vendored ALIGN `*.const.json` files.
- **Change:**
  - Add `backend/annotator/tests/gold.rs`. It runs over every ALIGN example that has both `.sp` and `.const.json`, and skips when the vendored tree is absent.
  - Parse `SymmetricBlocks` (18), `SymmetricNets` (17), `GroupBlocks` (55), `PowerPorts`/`GroundPorts`/`ClockPorts` (32/31/7), `Order` (25) and `GroupCaps` (2). The counts are what the local gold contains [ALIGN-GOLD].
  - Score precision and recall per family.
  - Add `AnnotationConfig.catalog: CatalogUse { Full, HintsOnly, NamingOnly }` for the A/B runs.
  - Add the MAGICAL OTA and comparator netlists as symmetry-only expectations.
- **Acceptance:** a table per circuit and family. The baseline is committed as test expectations that characterise today's output without endorsing it.
- **Effort:** M. **Deps:** the parser must read ALIGN's hierarchical `.sp` (FLOW-07).

#### ID-02 Origin, Confidence and normalisation pre-pass
- **Why:**
  - Today a pair from a wildcard pattern and a pair from an exact automorphism are emitted the same way.
  - Dummies and S/D order create false positives [KUNAL20; ALIGN-SCHEMA].
- **Change:**
  - Add the §3.1 types in `intent.rs` and the arm table in `policy.rs`.
  - Add a normalisation pass before `canonical_labels`:
    - drop dummies with every terminal on a rail;
    - merge series and parallel identical devices (reusing the `constraints.rs:150-173` key);
    - fix S/D by op sign, else by potential distance;
    - add channel-path distance to Ground and to Supply to WL round 0 [GAO21].
- **Acceptance:** every batch has a non-default `BatchMeta.origin`; `corpus.rs` outputs are unchanged; a dummy-padded `ota5t` gives the same compounds.
- **Effort:** S. **Deps:** none.

#### ID-03 Strict symmetry by involutive automorphism
- **Why:** gives exact symmetry with no seed (§2.1).
- **Change:** add `backend/annotator/src/autsym.rs`, which does individualise-and-refine on the existing WL with rails fixed and returns involutions as `Seed::Devices` with `Origin::Automorphism`. No new dependency; `canonaut` is used as a dev-dependency oracle in one test [CRATES].
- **Acceptance:** with the catalog off, `latch`, `gilbert`, `rail2rail` and `strongarm` compounds match catalog-on; `negative_corpus` has zero compounds; `scale.rs` stays within its 2 s budget at 12.5 k devices.
- **Effort:** M. **Deps:** ID-01, ID-02.

#### ID-04 Local and port-pair seeds for single-ended circuits
- **Why:** single-ended stages have no automorphism.
- **Change:**
  - Virtual-ground candidates [MAG19], scored by radius r.
  - `Seed::Nets` from differential port names, `*.interface.json`, opposite-phase AC sources and complementary PULSE sources [ALIGN-FC].
  - Seed order as in §2.1.
- **Acceptance:** with the catalog off, `strongarm_matches_align_gold` passes and `ota5t`, `three_stage`, `folded`, `degen_pair` and `tt_ota` compounds are unchanged.
- **Effort:** S. **Deps:** ID-03.

#### ID-05 Electrical confirmation of symmetry
- **Why:** structure cannot tell a signal pair from a mated bias pair, or from an accidental automorphism.
- **Change:**
  - Per pair: op equality (|ΔI_D|, |ΔV_DS|), conductance-matrix commutation, and `d_vt` antisymmetry.
  - Optionally, mirrored transient current waveforms [DHAR22].
  - Set `Confidence`, and emit a `symmetry_unconfirmed` diagnostic where it fails.
- **Acceptance:**
  - On `ota` and `tt_ota` with the probe op point, every gold pair is `Corroborated`.
  - A constructed accidental automorphism (two independent identical bias legs) stays `Inferred` and is not made hard.
- **Effort:** M. **Deps:** ID-02, ID-03, op point; sensitivities are optional.

#### ID-06 Role-free match kind, set role, stacks and stage pulls
- **Why:** `lib.rs:300-350` and `emit.rs:210-223` read leaf kinds (§2.4, §2.8).
- **Change:**
  - Kind and role predicates over compounds, flow levels and shared bias.
  - Stack pulls from `DeviceRole::Cascode`, or from series couples sharing a single node.
  - Stage-self pulls from compound selfs.
- **Acceptance:** with the catalog off, `class_sources`, `exceptional_voltage_compound_is_perfect` and the `folded` Stack pulls are unchanged.
- **Effort:** S. **Deps:** ID-04.

#### ID-07 Dummies and LDE pairs by role
- **Why:** §2.7 (`constraints.rs:119-146`, `frontend/library/src/lib.rs:2783-2790`).
- **Change:** dummies only from sets, by class; environment pairs from compounds and sets; the WPE threshold per set class.
- **Acceptance:** the `dac4` inverters get no dummies; the `ota5t` environment pairs are its two set couples; `bench local` stays at DRC 0 and LVS MATCH.
- **Effort:** S. **Deps:** none (can land first).

#### ID-08 Translinear-loop sets
- **Why:** replaces the `bjt_*` patterns and the bandgap template dependence (`lib.rs:259-262`, `classify.rs:247-261`).
- **Change:** the junction-loop search (§2.3), with the Reference class taken from the loop's shared base or gate.
- **Acceptance:** `brokaw` and `bgr_core` sets and Reference nets are unchanged with the `bjt_*` patterns removed.
- **Effort:** M. **Deps:** ID-02.

#### ID-09 Net classes without logic templates
- **Why:** §2.10.
- **Change:** dual-network static CMOS detection, op rail-level digital detection, switched-node Noisy, and an optional PSRR probe for Reference.
- **Acceptance:** the corpus class census is unchanged with LOGIC, `charge_pump_cell` and `cascoded_reference` removed.
- **Effort:** M. **Deps:** ID-06.

#### ID-10 Matched sets found by sensitivity
- **Why:** catches matching that structure does not imply (§2.3).
- **Change:** `d_vt` anti-correlation over same-signature devices in no set, giving `MatchSpec { origin: Sensitivity, confidence: Inferred }`.
- **Acceptance:** a pseudo-differential pair coupled only through resistors is found; no new sets appear on `negative_corpus`.
- **Effort:** M. **Deps:** ID-02, `perf::to_evidence`.

#### ID-11 Ranked aggressor/victim coupling (crosstalk, shield, isolation)
- **Why:** §2.12, §2.13, §2.16. Today's rules are all-pairs yes/no.
- **Change:** add `Aggressor.strength` and `Victim.susceptibility`; emit only pairs above a share of the victim's budget; take the substrate balance from compounds.
- **Acceptance:** fewer rules on `zz_strongarm` with every victim still covered; `substrate.rs` tests unchanged.
- **Effort:** M. **Deps:** transient evidence; falls back to the class.

#### ID-12 Thermal sensitivity from `d_vt`
- **Why:** `d_t` is empty (`evidence.rs:69-70`).
- **Change:** `d_t = d_vt·|dV_T/dT|` in `perf::to_evidence`.
- **Acceptance:** a unit test on a two-device table; on `ota`, the input pair's thermal weight is above the mirror's.
- **Effort:** S. **Deps:** sensitivities.

#### ID-13 Force/sense and stars by KCL
- **Why:** Kelvin covers only one resistor shape (§2.14).
- **Change:** split each net's current per terminal; emit a Kelvin when the budgeted IR drop exceeds a share of the sensing set's allowance.
- **Acceptance:** a constructed high-current mirror gets a Kelvin; the corpus is otherwise unchanged.
- **Effort:** S. **Deps:** op currents.

#### ID-14 Port, domain and pad analysis (ESD, latch-up, cross-domain)
- **Why:** §2.21 and §2.22. Today only user-declared ESD nets are handled.
- **Change:**
  - Voltage propagation through channels to assign domains [PERC].
  - Port classification and structural clamp detection.
  - ESDA-style diagnostics [ESDA-TR18].
- **Acceptance:** the `tt_ota` ports are classified; signoff is unchanged.
- **Effort:** M. **Deps:** FLOW ports.

#### ID-15 Nested symmetry from the automorphism group
- **Why:** §2.2.
- **Change:** commuting involutions become nested compounds; component swaps become instance matching without needing subcircuits.
- **Acceptance:** `gilbert` gives one quad set of 4; a flattened `two_ota` pairs its instances.
- **Effort:** M. **Deps:** ID-03.

#### ID-16 Adjoint sensitivities
- **Why:** finite differences cost 2 runs per parameter (§2.15).
- **Change:** one transposed-network AC run per spec in `frontend/library/src/perf.rs` gives `d_c` for all nets from two solves; add rail `d_r` rows for IR.
- **Acceptance:** adjoint and finite-difference `d_c` agree within 5 % on `ota` and `tt_ota`.
- **Effort:** M. **Deps:** the PERF harness.

#### ID-17 Hint channel for learned and LLM proposals
- **Why:** §2.24.
- **Change:** sidecar entries with `confidence` and `source` become `Hint` facts, promoted only by corroboration; the report lists unconfirmed hints.
- **Acceptance:** a hinted pair that contradicts an automorphism is dropped with a diagnostic; a hinted pair confirmed by ID-05 is emitted.
- **Effort:** S in tree. **Deps:** ID-02, ID-05.

#### ID-18 Catalog to naming only
- **Why:** §3.2.
- **Change:** make `CatalogUse::NamingOnly` the default; patterns name blocks after identification; delete `ROLES`.
- **Acceptance:** on the ID-01 scoreboard, catalog-off recall is at least catalog-on recall for every family and precision is not lower; `bench local` is unchanged.
- **Effort:** M. **Deps:** ID-01, 04, 06, 07, 08, 09.

**Top 5, in order:** ID-01 (scoreboard), ID-02 (provenance and normalisation), ID-03 + ID-04 (catalog-free symmetry seeds, counted as one step), ID-06 (role-free kind and role), ID-07 (dummies and LDE by role).

### 3.5 Interchange formats and standards

- **No adopted public IEEE or Si2 standard for analog layout constraints exists.**
  - IPL Constraints 1.0 (2011) was members-only and appears dormant (dormant: U) [IPL11; MORSE12].
  - An ontology was proposed [KL11].
  - ESDA TR18 is the closest thing to a standard, and covers only ESD and latch-up checks [ESDA-TR18].
- **De facto formats:**
  - ALIGN JSON [ALIGN-SCHEMA], which Philis' sidecar already follows (`sidecar.rs:1-21`);
  - MAGICAL `.sym`/`.symnet` [MAG19];
  - Cadence finders that apply constraints after topology recognition [CAD-PAT]. The full Constraint Manager type list was not verified (U).
- **Recommendation:** keep ALIGN names for interop and add Philis-only kinds (`Match`, `NetClass`, `OffsetBudget`, `Kelvin`, `Load`, `IsolatedTub`, `Confirm`/`Reject`) as extensions, as `sidecar.rs` does today.

---

## 4. Sources

Format: key: title. Authors. Venue, year. URL. [verification]

**Local (`docs/ref/`, via digests)**
- BG11: *Analog Layout Synthesis: A Survey of Topological Approaches*. Graeb (ed.); ch.3 Strasser, Eick, Graeb, Schlichtmann. Springer, 2011. https://doi.org/10.1007/978-1-4419-6932-3 [V-meta, L]
- LAMP99: *Analog Layout Generation for Performance and Manufacturability*. Lampaert, Gielen, Sansen. Springer, 1999. https://doi.org/10.1007/978-1-4757-4501-6 [V-meta, L]
- GRAEB07: *Analog Design Centering and Sizing*. Graeb. Springer, 2007. Local PDF in `docs/ref/` [L]
- HAST: *The Art of Analog Layout*, 3rd ed. Hastings. Pearson, 2023. Digests `docs/plans/ref-hastings-*.md` [L]
- CHARB01: *Substrate Noise: Analysis and Optimization for IC Design*. Charbon, Gharpurey, Miliozzi, Meyer, Sangiovanni-Vincentelli. Kluwer/Springer, 2001. https://doi.org/10.1007/b100751 [V, L]
- LIEN18: *Fundamentals of Electromigration-Aware Integrated Circuit Design*. Lienig, Thiele. Springer, 2018. https://doi.org/10.1007/978-3-319-73558-0 [V, L]
- CCREV: "Common-Centroid Layout for Active and Passive Devices: A Review and the Road Ahead". ASP-DAC, 2022. https://doi.org/10.1109/ASP-DAC52403.2022.9712576 [V-meta, L]
- DACCC: "Constructive Placement and Routing for Common-Centroid Capacitor Arrays in Binary-Weighted and Split DACs". Local PDF, digest `ref-common-centroid-papers.md` [L]
- NTH: "An Nth-order central symmetrical layout pattern for nonlinear gradients cancellation". Local PDF, digest `ref-common-centroid-papers.md` [L]
- WS22: "Performance-driven Wire Sizing for Analog Integrated Circuits". Li, Lin, Madhusudan, Sharma, Sapatnekar, Harjani, Hu. ACM TODAES 28, 2022. https://doi.org/10.1145/3559542 [V-meta, L]
- ALIGN-GOLD: 69 vendored `benchmarks/competition/ALIGN/examples/**/*.const.json` files. Constraint counts (`grep`): GroupBlocks 55, PowerPorts 32, GroundPorts 31, Order 25, Align/HorizontalDistance/VerticalDistance 20 each, SymmetricBlocks 18, SymmetricNets 17, ConfigureCompiler 8, DoNotIdentify 7, ClockPorts 7, CompactPlacement 4, GroupCaps/AspectRatio/MultiConnection/PortLocation/DoNotUseLib 2 each, GuardRing 1, ChargeFlow 1. [V, local]

**Symmetry, matching and hierarchy (structural)**
- ALIGN19: "ALIGN: Open-Source Analog Layout Automation from the Ground Up". Kunal et al. DAC, 2019. https://doi.org/10.1145/3316781.3323471 [V-meta]
- ALIGN21: "ALIGN: A System for Automating Analog Layout". Dhar, Kunal et al. IEEE Design & Test 38(2), 2021. https://arxiv.org/abs/2008.10682 [V]
- KUNAL20: "A general approach for identifying hierarchical symmetry constraints for analog circuit layout". Kunal, Poojary, Dhar, Madhusudan, Harjani, Sapatnekar. ICCAD, 2020. https://arxiv.org/abs/2010.00051 [V]
- ALIGN-FC: ALIGN `align/compiler/find_constraint.py`. GitHub, master. https://github.com/ALIGN-analoglayout/ALIGN-public/blob/master/align/compiler/find_constraint.py [V]
- ALIGN-SCHEMA: ALIGN constraint schema (`align/schema/constraint.py` and docs). https://github.com/ALIGN-analoglayout/ALIGN-public/blob/master/align/schema/constraint.py ; https://align-analoglayout.github.io/ALIGN-public/modules/align.schema.html [V]
- GANA20: "GANA: Graph Convolutional Network Based Automated Netlist Annotation for Analog Circuits". Kunal et al. DATE, 2020. https://doi.org/10.23919/DATE48585.2020.9116329 [V-meta; the ~97 % accuracy figure is U]
- GANA23: "GNN-Based Hierarchical Annotation for Analog Circuits". Kunal et al. IEEE TCAD 42(9), 2023. https://doi.org/10.1109/TCAD.2023.3236269 [V, abstract]
- MAG19: "MAGICAL: Toward Fully Automated Analog IC Layout Leveraging Human and Machine Intelligence". Xu, Zhu, Liu, Lin, Li, Tang, Sun, Pan. ICCAD, 2019. https://doi.org/10.1109/ICCAD45719.2019.8942060 ; https://yibolin.com/publications/papers/ANALOG_ICCAD2019_Xu.pdf ; https://github.com/magical-eda/MAGICAL [V]
- S3DET20: "S³DET: Detecting System Symmetry Constraints for Analog Circuits with Graph Similarity". Liu et al. ASP-DAC, 2020. https://ieeexplore.ieee.org/document/9045109 [V]
- GAO21: "Layout Symmetry Annotation for Analog Circuits with Graph Neural Networks". Gao, Deng, Liu, Zhang, Pan, Lin. ASP-DAC, 2021. https://doi.org/10.1145/3394885.3431545 [V]
- CHEN21: "Universal Symmetry Constraint Extraction for Analog and Mixed-Signal Circuits with Graph Neural Networks". Chen, Zhu, Liu, Tang, Sun, Pan. DAC, 2021. https://doi.org/10.1109/DAC18074.2021.9586211 [V]
- XU24: "Graph Attention-Based Symmetry Constraint Extraction for Analog Circuits". Xu et al. IEEE TCAS-I, 2024. https://arxiv.org/abs/2312.14405 [V]
- ZHANG25: "MCE-HGCN: Heterogeneous Graph Convolution Network for Analog IC Matching Constraints Extraction". Zhang, Yin, Xu, Jia. Micromachines 16(6), 2025. https://doi.org/10.3390/mi16060677 [V-meta]
- WU24: "Matching constraint extraction for analog integrated circuits layout via edge classify". Wu et al. Integration 98, 2024. https://www.sciencedirect.com/science/article/abs/pii/S0167926024001032 [U]
- CG24: "Self-Learning and Transfer across Topologies of Constraints for AMS Layout Synthesis". K. Chen, Gielen. DATE, 2024. https://past.date-conference.com/proceedings-archive/2024/DATA/115_pdf_upload.pdf [V]. Journal version: ACM TODAES 31(3), 2026, https://doi.org/10.1145/3722556 [V-meta]
- YAO23: "Automatic Layout Symmetry Extraction for Analog Constraint Learning". Yao, Gao, Lin, Li. ISEDA, 2023. https://doi.org/10.1109/ISEDA59274.2023.10218680 [V-meta]
- MASS08: "The Sizing Rules Method for CMOS and Bipolar Analog Integrated Circuit Synthesis". Massier, Graeb, Schlichtmann. IEEE TCAD 27(12), 2008. https://doi.org/10.1109/TCAD.2008.2006143 [V]
- EICK10: "Automatic generation of hierarchical placement rules for analog integrated circuits". Eick, Strasser, Graeb, Schlichtmann. ISPD, 2010. https://doi.org/10.1145/1735023.1735039 [V]
- EICK11: "Comprehensive Generation of Hierarchical Placement Rules for Analog Integrated Circuits". Eick, Strasser, Lu, Schlichtmann, Graeb. IEEE TCAD 30(2), 2011. https://doi.org/10.1109/TCAD.2010.2097172 [V]
- GL23: "Learning from the Implicit Functional Hierarchy in an Analog Netlist". Graeb, Leibl. ISPD, 2023. https://doi.org/10.1145/3569052.3578921 [V]
- FEATS15: "FEATS: Framework for Explorative Analog Topology Synthesis". Meissner, Hedrich. IEEE TCAD 34(2), 2015. https://doi.org/10.1109/TCAD.2014.2376987 [V]
- ARS96: "A method for analog circuits visualization". Arsintescu. ICCD, 1996. https://doi.org/10.1109/ICCD.1996.563593 [V]
- HAO04: "Constraints generation for analog circuits layout" (parts 1 and 2). Hao, Dong, Chen, Hong, Su, Qu. ICCCAS, 2004. https://doi.org/10.1109/ICCCAS.2004.1346418 ; https://doi.org/10.1109/ICCCAS.2004.1346419 [V]
- ZHOU05: "Analog constraints extraction based on the signal flow analysis". Zhou, Dong, Hong, Hao, Chen. ASICON, 2005. https://doi.org/10.1109/ICASIC.2005.1611454 [V]
- GIL75: "Translinear circuits: a proposed classification". Gilbert. Electronics Letters 11(1), 1975. https://doi.org/10.1049/el:19750011 [V-meta]

**Graph algorithms and tools**
- NAUTY14: "Practical graph isomorphism, II". McKay, Piperno. J. Symbolic Computation 60, 2014. https://doi.org/10.1016/j.jsc.2013.09.003 [V]
- BLISS07: "Engineering an efficient canonical labeling tool for large and sparse graphs". Junttila, Kaski. ALENEX, 2007. https://doi.org/10.1137/1.9781611972870.13 [V]
- BBG13: "Tight Lower and Upper Bounds for the Complexity of Canonical Colour Refinement". Berkholz, Bonsma, Grohe. ESA, 2013. https://arxiv.org/abs/1509.08251 [V]
- GEMINI88: "GeminiII: A second generation layout validation program". Ebeling. ICCAD, 1988. https://doi.org/10.21236/ADA220731 [V-meta]
- SUBG93: "SubGemini: identifying subcircuits using a fast subgraph isomorphism algorithm". Ohlrich, Ebeling, Ginting, Sather. DAC, 1993. https://doi.org/10.1145/157485.164556 [V]
- VF2: "A (sub)graph isomorphism algorithm for matching large graphs". Cordella, Foggia, Sansone, Vento. IEEE TPAMI 26(10), 2004. https://doi.org/10.1109/TPAMI.2004.75 [V]
- SMART25: "SMART: Graph Learning-Boosted Subcircuit Matching for Large-Scale Analog Circuits". Tu et al. IEEE TCAD 44(10), 2025. https://doi.org/10.1109/TCAD.2025.3549701 [V]
- CRATES: crates.io entries, checked 2026-10:
  - `canonaut` 1.0.0: pure-Rust nauty port, created 2026-08, very new. https://crates.io/crates/canonaut [V]
  - `nauty-pet` 0.15.0: petgraph wrapper over nauty (C). https://crates.io/crates/nauty-pet [V]
  - `nauty-Traces-sys` 0.11.0. https://crates.io/crates/nauty-Traces-sys [V]
  - `petgraph` 0.8 `subgraph_isomorphisms_iter` (VF2). https://crates.io/crates/petgraph [V]

**Electrical, performance-driven and flow**
- CHOU93: "Automatic generation of parasitic constraints for performance-constrained physical design of analog circuits". Choudhury, Sangiovanni-Vincentelli. IEEE TCAD 12, 1993. https://doi.org/10.1109/43.205002 [V-meta]
- CHAR93: "Generalized constraint generation for analog circuit design". Charbon, Malavasi, Sangiovanni-Vincentelli. ICCAD, 1993. https://doi.org/10.1109/ICCAD.1993.580089 [V-meta; method details U]
- CHAR94: "A Performance-Driven Router for RF and Microwave Analog Circuit Design". Charbon, Holmlund, Sangiovanni-Vincentelli, Donecker. UCB/ERL M94/40, 1994. https://www2.eecs.berkeley.edu/Pubs/TechRpts/1994/ERL-94-40.pdf [V]
- MAL96: "Automation of IC layout with analog constraints". Malavasi, Charbon, Felt, Sangiovanni-Vincentelli. IEEE TCAD 15(8), 1996. https://doi.org/10.1109/43.511572 [V-meta]
- LAMP95: "A performance-driven placement tool for analog integrated circuits". Lampaert, Gielen, Sansen. IEEE JSSC 30(7), 1995. https://doi.org/10.1109/4.391116 [V-meta]
- DR69: "The Generalized Adjoint Network and Network Sensitivities". Director, Rohrer. IEEE Trans. Circuit Theory CT-16, 1969. https://doi.org/10.1109/tct.1969.1082965 [V-meta, via Crossref]
- NGS: ngspice manual, `.SENS` (DC or small-signal AC sensitivity). https://nmg.gitlab.io/ngspice-manual/analysesandoutputcontrol_batchmode/analyses/sens_dcorsmall-signalacsensitivityanalysis.html [V for syntax; BSIM4 instance-parameter support U]
- DHAR22: "A Charge Flow Formulation for Guiding Analog/Mixed-Signal Placement". Dhar et al. DATE, 2022. https://doi.org/10.23919/DATE54114.2022.9774621 [V-meta]
- PARA20: "ParaGraph: Layout Parasitics and Device Parameter Prediction using Graph Neural Networks". Ren, Kokai, Turner, Ku. DAC, 2020. https://doi.org/10.1109/DAC18072.2020.9218515 [V-meta]
- GR19: "GeniusRoute: A New Analog Routing Paradigm Using Generative Neural Network Guidance". Zhu et al. ICCAD, 2019. https://doi.org/10.1109/ICCAD45719.2019.8942164 [V]
- PQP20: "Towards decrypting the art of analog layout: placement quality prediction via transfer learning". Liu et al. DATE, 2020. https://doi.org/10.23919/DATE48585.2020.9116330 [V-meta]
- PEA20: "A customized graph neural network model for guiding analog IC placement". Li et al. ICCAD, 2020. https://doi.org/10.1145/3400302.3415624 [V-meta]
- XU25: "PARoute2: Enhanced Analog Routing via Performance-Driven Guidance Generation". Xu et al. IEEE TCAD 44(10), 2025. https://www.cse.cuhk.edu.hk/~byu/papers/J142-TCAD2025-PARoute.pdf [V]
- ASPDAC24: "Performance-Driven Analog Layout Automation: Current Status and Future Directions". Xu, Li, Ho, Yu, Zhu. ASP-DAC (invited), 2024. http://www.cse.cuhk.edu.hk/~byu/papers/C194-ASPDAC2024-AnalogPD-slides.pdf [V, slides]
- ZHU22: "Automating Analog Constraint Extraction: From Heuristics to Learning". Zhu, Chen, Liu, Pan. ASP-DAC, 2022. https://doi.org/10.1109/ASP-DAC52403.2022.9712488 [V-meta]
- TAG22: "TAG: Learning Circuit Spatial Embedding From Layouts". Zhu et al. ICCAD, 2022. https://arxiv.org/abs/2209.03465 [V]
- RET18: "Analog Placement Constraint Extraction and Exploration with the Application to Layout Retargeting". Xu, Basaran, Su, Pan. ISPD, 2018. https://doi.org/10.1145/3177540.3178245 [V-meta]
- SF20: "Effective Analog/Mixed-Signal Circuit Placement Considering System Signal Flow". Zhu, Chen, Liu, Tang, Sun, Pan. ICCAD, 2020. https://doi.org/10.1145/3400302.3415625 [V]
- WU12: "Performance-driven analog placement considering monotonic current paths". Wu et al. ICCAD, 2012. https://doi.org/10.1145/2429384.2429516 [V-meta]
- OU13: "Simultaneous analog placement and routing with current flow and current density considerations". Ou, Chang Chien, Chang. DAC, 2013. https://doi.org/10.1145/2463209.2488739 [V]
- GAO10: "Analog circuit shielding routing algorithm based on net classification". Gao, Shen, Cai, Yao. ISLPED, 2010. https://doi.org/10.1145/1840845.1840872 [V-meta]

**LLM**
- LC24: "LayoutCopilot: An LLM-Powered Multiagent Collaborative Framework for Interactive Analog Layout Design". Liu et al. arXiv 2406.18873, 2024; IEEE TCAD 44, 2025. https://arxiv.org/abs/2406.18873 [V]
- LLANA24: "LLANA: LLM-Enhanced Bayesian Optimization for Efficient Analog Layout Constraint Generation". Chen et al. arXiv, 2024. https://arxiv.org/abs/2406.05250 [V]
- PANDA26: "PANDA: An LLM-Enhanced Performance-Driven Analog Design Framework Bridging Design Intent and Layout Generation". Zhang et al. arXiv, 2026. https://arxiv.org/abs/2606.15052 [V]
- ICLAD26: "Simulation-Aware In-Context Policy Improvement for LLM-Aided Analog Layout Refinement". Liu, Wei, Gao, Pan. ICLAD, 2026. https://arxiv.org/abs/2608.13767 [V]
- GENIE25: "GENIE-ASI: Generative Instruction and Executable Code for Analog Subcircuit Identification". Pham et al. arXiv, 2025. https://arxiv.org/abs/2508.19393 [V]
- ATLAS26: ATLAS. arXiv, 2026. https://arxiv.org/abs/2607.14165 [V; cited only for its "direct prompting hallucinates" finding]
- AMSB25: AMSbench. arXiv, 2025. https://arxiv.org/abs/2505.24138 [V]

**Reliability, substrate, thermal and LDE**
- SUBW99: "Modeling digital substrate noise injection in mixed-signal ICs" (SubWave). Charbon, Miliozzi, Carloni, Ferrari, Sangiovanni-Vincentelli. IEEE TCAD 18(3), 1999. https://ptolemy.berkeley.edu/projects/embedded/asves/dsm/subwave/subwave.html [V]
- VA98: "Computer-aided design considerations for mixed-signal coupling in RF integrated circuits". Verghese, Allstot. IEEE JSSC 33(3), 1998. https://doi.org/10.1109/4.661197 [V]
- WRIGHT94: "Substrate-aware mixed-signal macro-cell placement in WRIGHT". Mitra, Rutenbar, Carley, Allstot. CICC, 1994. https://experts.illinois.edu/en/publications/substrate-aware-mixed-signal-macro-cell-placement-in-wright/ [V]
- CHO08: "Fast Substrate Noise Aware Floorplanning for Mixed Signal SOC Designs". Cho, Pan. IEEE TVLSI 16(12), 2008. https://www.cerc.utexas.edu/utda/publications/substrateNoise_TVLSI.pdf [V]
- SF08: "Methodology for Placing Localized Guard Rings to Reduce Substrate Noise in Mixed-Signal Circuits". Salman, Friedman. PRIME, 2008. https://hajim.rochester.edu/ece/sites/friedman/papers/PRIME_08.pdf [V]
- LIN09: "Thermal-driven Analog Placement Considering Device Matching". Lin, Zhang, Wong, Chang. DAC, 2009. http://cc.ee.ntu.edu.tw/~ywchang/Papers/dac09-thermal-placement.pdf [V]
- DREN06: "Implications of Proximity Effects for Analog Design". Drennan, Kniffin, Locascio. CICC, 2006. https://ieeexplore.ieee.org/document/4114933/ [U]
- OU15: "Layout-dependent-effects-aware analytical analog placement". Ou, Tseng, Liu, Wu, Chang. DAC, 2015. https://doi.org/10.1145/2744769.2744865 [V]
- TZ20: "LDE-aware Analog Layout Migration with OPC-inclusive Routing". Torabi, Zhang. ACM TODAES, 2020. https://doi.org/10.1145/3398190 [V]
- LIEN06: "Introduction to Electromigration-Aware Physical Design". Lienig. ISPD (invited), 2006. https://www.ifte.de/mitarbeiter/lienig/ispd06_emPaper_lienig.pdf [V]
- MART14: "Electromigration-aware and IR-Drop avoidance routing in analog multiport terminal structures". Martins, Lourenço, Canelas, Horta. DATE, 2014. https://doi.org/10.7873/DATE.2014.023 [V]
- WANG19: "IR-aware Power Net Routing for Multi-Voltage Mixed-Signal Design". Wang, Liou, Su, Lin. DATE, 2019. https://doi.org/10.23919/DATE.2019.8715166 [V-meta]
- HUANG04: "A polynomial time-optimal diode insertion/routing algorithm for fixing antenna problem". Huang et al. IEEE TCAD, 2004. https://doi.org/10.1109/TCAD.2003.819888 [V]
- JIANG06: "An optimal simultaneous diode/jumper insertion algorithm for antenna fixing". Jiang, Chang. ICCAD, 2006. https://doi.org/10.1109/ICCAD.2006.320034 [V]
- PERC: "Advanced electrical rule checking in IC reliability verification". Yan. Siemens (Calibre PERC) white paper, year U. https://static.sw.cdn.siemens.com/siemens-disw-assets/public/34xQH4zOfVrddyoNmX2oEc/en-US/Siemens-SW-Advanced-electrical-rule-checking-in-IC-WP-81822-C2.pdf [V]
- PERC-LU: "Safeguarding IC reliability: Calibre PERC's latch-up guard ring check". Siemens blog, 2025. https://blogs.sw.siemens.com/calibre/2025/09/23/safeguarding-ic-reliability-calibre-percs-latch-up-guard-ring-check/ [V]
- ESDA-TR18: "ESD Electronic Design Automation Checks" (ESD TR18.0-01). ESDA EDA Working Group. https://www.esda.org/assets/News/ESDEDA-InCompliance-part1.pdf [V]
- ESDA-LU: "What are external latch-up and internal latch-up?". ESDA. https://www.esda.org/news/what-are-external-latch-up-and-internal-latch-up/ [V]
- SI2-ESD: Si2 OpenPDK Coalition ESD design-flow methodology (news). https://www.chipestimate.com/Si2-and-rsquos-OpenPDK-Coalition-Releases-ESD-Design-Flow-Methodology/Semiconductor-IP-Core/news/20600 [V; date U]
- LOUR15: "Layout-Aware Sizing of Analog ICs using Floorplan & Routing Estimates for Parasitic Extraction". Lourenço, Martins, Horta. DATE, 2015. https://past.date-conference.com/proceedings-archive/2015/pdf/0411.pdf [V]

**Standards and formats**
- IPL11: IPL Alliance, "IPL Constraints 1.0" (news), 2011. https://www.chipestimate.com/IPL-Alliance-Delivers-Standard-for-Interoperable-Design-Constraints/Semiconductor-IP-Core/news/11196 [V; status now U]
- MORSE12: "Interoperable Design Constraints for Custom IC Design". Morse. IEEE Design & Test, 2012. https://doi.org/10.1109/MDT.2012.2182983 [V-meta]
- KL11: "An ontology for constraints in custom IC design". Krinke, Lienig. ECCTD, 2011. https://doi.org/10.1109/ECCTD.2011.6043355 [V-meta]
- CAD-PAT: "Circuit topology recognition with auto-interactive constraint application" (Cadence). US7735036B2, filed 2007. https://patents.google.com/patent/US7735036B2/en [V]
