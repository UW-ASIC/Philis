# PNR Improvement Roadmap

Ranking: all items verified high-impact; ordered by impact/effort (S=small <1d, M=medium 1-3d, L=large 1-2wk). Checklist refs = `docs/PNR_ANALOG/08_OPTIMAL_LAYOUT_CHECKLIST.md`.

---

## 1. Cell generation

### 1.1 Dummy gate tie-off: polarity + physical contact stack — S
- **What**: dummies hardcode `supply_net = "VDD"` for both polarities; dummy poly has no contact stack -> physically floating despite doc claim.
- **Why**: AOAL ch13 13.2.2 + Rule 12 — dummy must sit in cutoff (GND for NMOS, VDD for PMOS); floating gate = charge-state-dependent edge environment. Checklist 2.6.
- **Where**: `crates/cell/src/mosfet.rs:1293` (hardcoded VDD), `:1313-1321` (bare poly), `:1252-1254` (stale doc).
- **Change**: match on `device_type` (pattern exists at `:1117`); emit licon+li+mcon+met1 stack on dummy poly endcap; in-cell met1 strap to bulk tap pad (same rail — simpler than router-tied `dummy:G` pin, since `power.rs:192-202` classifies by net name).
- **Payoff**: dummies actually in cutoff; removes floating-gate LVS/charging hazard on every Moderate+ MOS cell.

### 1.2 Interdig finger pitch: fold met1 limit into pitch clamp — S
- **What**: pitch clamps cover only poly.2 + licon.11; met1.2 spacing never read anywhere in repo; stagger workaround admits failure at n_ct_y==1.
- **Why**: AOAL ch13-power-mos Interdigitated Layout — p_M1 = W_ct + 2·M_mc + S_mm is a hard pitch floor. Matches known remaining m1.2 DRC class (memory: pad-bridge). Checklist 1.1.
- **Where**: `crates/cell/src/mosfet.rs:196-213` (clamps), `:1004-1014` (stagger + admitted failure `:1007-1008`).
- **Change**: after licon.11 clamp add `contact_pitch = contact_pitch.max(mcon_size + 2*m1_enc + met1_space)` (read met1.5/met1.2 same way licon rules read at `:756-795`). Skip p_via term — cell places no vias above mcon.
- **Payoff**: kills residual m1.2 violation class by construction, not heuristic.

### 1.3 Wire MatchingType into sizing (Mirror path is dead code) — S
- **What**: `pelgrom_sizing` has full Mirror branch (beta area + Hastings Table 13.6 L-floors) but sole caller hardcodes `MatchingType::Cross`; constraints' DiffPair/CurrentMirror classification never reaches sizing.
- **Why**: AOAL ch13 13.3 Rules 2-3 — current matching sizes L primarily, voltage matching sizes W·L; wrong axis = over/under-designed mirrors. Checklist 5.7, 2.4.
- **Where**: `crates/cell/src/mosfet.rs:151` (hardcoded Cross), `:1706-1754`, `:1823-1850` (dead Mirror path); `crates/cell/src/lib.rs:551` (no param); call site `crates/python/src/lib.rs:164-207`.
- **Change**: add `match_kind: MatchingType` to `generate_cell`/`mosfet::generate`; map constraints `CurrentMirror -> Mirror`, `DiffPair -> Cross`; pass group's MatchingSpec from pipeline.
- **Interacts**: do BEFORE 1.7 (unit-finger folding runs pelgrom once on unit device — needs correct axis).
- **Payoff**: activates ~130 lines of dead correct code; mirrors get L-dominated sizing.

### 1.4 Guard-ring width from well/epi depth — S
- **What**: PDK carries `p_epi_thickness/p_well_depth/n_well_depth` (written only in tests, zero readers); ring width = `min_guard_ring_width` only, type-blind.
- **Why**: AOAL ch14 14.2.3/14.2.4 — collecting ring width must ≥ well/epi depth to intercept carriers; minimum-width taps collect majority carriers only. Checklist 1.4, 2.7.
- **Where**: `crates/cell/src/guard.rs:140-147` (`ring_dimensions_from_pdk`, type-blind); `crates/core/src/pdk.rs:200-208` (unread fields).
- **Change**: pass `ring_type` in (in scope at `guard.rs:127`); NPlus width = `max(min_w, if retrograde {p_well_depth} else {p_epi_thickness})`; PPlus = `max(min_w, n_well_depth)`. Escalate depth-width only for injector cells; record fired rule in GuardRingSpec.
- **Interacts**: do WITH/AFTER 1.6 (width of a floating ring is moot).
- **Payoff**: rings become minority-carrier collectors, not decoration.

### 1.5 Resistor matched sequence: ratio-preserving interleave — S
- **What**: ABBA loop pushes `min(segs)/2` equal A/B counts — unequal segment counts silently truncated (larger device under-built), devices[2..] ignored entirely.
- **Why**: FOLD 6.6.1 Eq 6.10 — series unit splitting exists precisely to realize unequal ratios; head correction (`hastings_head_correction` at `:128-133`) already implemented. Checklist 2.5.
- **Where**: `crates/cell/src/resistor.rs:424-451` (truncating loop), `:493` (bbox from truncated seq).
- **Change**: replace fixed ABBA with `greedy_centroid_sequence` over per-device segment counts (proven pattern at `mosfet.rs:1998`); apply head correction to residual non-integer remainder; extend to N>2 devices.
- **Payoff**: ratioed resistor pairs (bias dividers, feedback) built at correct value — today they are silently wrong resistance.

### 1.6 Guard rings: contacted, implanted, welled — M
- **What**: `ring_band_shapes` emits diff + met1 only — no licon cuts, no psdm/nsdm, no well layer. Rings electrically floating + DRC/LVS-incomplete. Shared-ring path also pipeline-dead.
- **Why**: AOAL ch14 14.2.2 — un-contacted ring = no collection, no latch-up protection. Checklist 1.4 (Tier 1 non-negotiable).
- **Where**: `crates/cell/src/guard.rs:234-281`; recipe to mirror: `emit_bulk_tap_column` at `mosfet.rs:1078`.
- **Change**: add licon array + tap implant + li over diffusion bands, mirroring tap-column recipe. Defer NBL/blocking-ring overhang rules until PDK models deep-N+ (`GuardRingType::DeepNwell` variant is never constructed).
- **Payoff**: latch-up protection real; ring shapes pass DRC/LVS instead of generating errors.

### 1.7 Group-wide unit finger + unitization plumbing — M
- **What**: sizing/decomposition runs per-device; geometry then uses device-0 finger width for all (`:221-223`); `UnitizationConstraint` (unit_w, gcd target_ratio) extracted, contract declares CellGen consumption (`contract.rs:219-231`), zero readers. Interdig loop lays 1:2 mirrors out 1:1.
- **Why**: FOLD 6.6.1 MOS-FETs + AOAL ch12 Practical Takeaways — ratio only by finger count of identical unit fingers. Heterogeneous-W groups already occur via self_symmetric injection (`symmetry.rs:337-357`). Checklist 2.5, 5.7.
- **Where**: `crates/cell/src/mosfet.rs:144-167` (per-device sizing), `:221-223` (dev-0 width), `:1900-1928` (equal-count ABBA); `crates/constraints/src/unitization.rs:39-104`.
- **Change**: thread UnitizationConstraint into `generate_cell`; run `pelgrom_sizing` ONCE on unit device (MatchingType::Mirror — needs 1.3); `dev_nf[i] = target_ratio[i] * base_units`, `dev_w[i] = dev_nf[i]*wf`; route unequal-nf 2-device groups through `greedy_centroid_sequence`; warn on non-integer W/wf.
- **Interacts**: AFTER 1.3; WITH 1.8 (ratioed groups must form upstream first, else nothing to unitize).
- **Payoff**: ratioed mirrors laid out at correct ratio with matched unit fingers — currently either 1:1-mangled or unmatched solo cells.

### 1.8 Ratioed current-mirror detection (constraints, feeds 1.7) — M
- **What**: `device_signature` requires identical W -> `is_current_mirror` detects only 1:1-in-W mirrors; W-ratioed mirrors (bandgap case) fall out of matching groups entirely; unitization analog-block path is stub. (m/nf-ratioed same-W already works; BJT mirrors detected but never ratio-recorded.)
- **Why**: FOLD 6.5.2 — the section's central example IS the ratioed mirror. Checklist 1.3.
- **Where**: `crates/constraints/src/symmetry.rs:397-411`, `:442-444` (exact-W signature, duplicated at `blocks.rs:43-45`); `unitization.rs:46-47` (stub).
- **Change**: relax to same (DeviceType, L, model), accept W_a/W_b forming small integer ratio within tolerance; record ratio on MatchingPair; fill unitization stub; record BJT emitter ratios too.
- **Payoff**: ratioed groups reach cell gen as matched groups; unblocks 1.7's full value.

### 1.9 WPE clearance: physical, not diagnostic — M
- **What**: `compute_wpe_distance` raises only the RETURNED number ("Well boundary expanded" — lie); nwell drawn at difftap.8 ~0.18um regardless; bbox halo is poly2_spacing. Two verified findings, one fix.
- **Why**: AOAL ch13 Rule 19 / FOLD 6.6.3 tier ladder (1/2/5um) — WPE dVth 10-50mV inside ~5um of well edge. Checklist 2.9 (tiers: minimal ≥2um, moderate ≥3, exceptional ≥5). `placement/cost.rs:34-39` explicitly delegates physical WPE to cell gen; `verify/matching.rs:479` admits bbox proxies.
- **Where**: `crates/cell/src/mosfet.rs:1228-1243` (`emit_implants_and_well`), `:1355-1414` esp. `:1400-1407` (fictitious overwrite), `:339-346` (bbox halo).
- **Change**: inflate nwell rect (PMOS) + cell bbox by tier-keyed gate-to-well-edge clearance measured from outermost active gate; DELETE min_dist overwrite so diagnostic reports real geometry.
- **Interacts**: do BEFORE placement 2.3 (class-pair spacing table sized from residual well exposure — today PMOS nwell reaches cell edge).
- **Payoff**: WPE control actually exists; verify diagnostics stop reporting fiction.

### 1.10 LOD moat extension: emit it — M
- **What**: `od_extension` feeds only SA/SB diagnostics; emitted diffusion clips at last real finger; dummies on field poly; `DummyMoatExtension` constraint (3/5um) has zero consumers. Diagnostics describe geometry that doesn't exist.
- **Why**: AOAL ch13 13.2.2 LOD + Rule 12 — STI stress needs diffusion continuity past outer gates; up to 13% NMOS Idrive mismatch. Checklist 2.10.
- **Where**: `crates/cell/src/mosfet.rs:658-663`, `:821-849` (diff clip), `constraints/src/environment.rs:94-106` (unconsumed).
- **Change**: extend diff_x_start/end by tier-keyed moat (3um Moderate, 5um Exceptional) OR draw diffusion under edge dummies with dummy S/D shorted to supply (avoids parasitic-FET LVS issue noted `:819-821` — pairs naturally with 1.1's tie strap).
- **Interacts**: do WITH 1.1 (shared dummy contact geometry).
- **Payoff**: SA/SB diagnostics == emitted geometry; LOD equalization physical.

### 1.11 Injector isolation: series-R test + private rings + cluster exclusion — M
- **What**: injector detection uses fanout≤2 pad proxy (no series-R < 1kΩ walk); `GuardRingRequirement` consumed only by contract bookkeeping; `find_sharing_clusters` groups by (well, domain, ring_type) only — injector can share well with matched core. Debias (Q2) uncomputed (`well_sheet_resistance` zero readers). Mitigating: sharing path pipeline-dead today — fix before it goes live.
- **Why**: AOAL ch14 14.1.5 three-question checklist + Medium-Risk Mergers. Checklist 1.4, 2.7.
- **Where**: `crates/constraints/src/lib.rs:1122-1160` (`:1131-1138` weak proxy); `crates/cell/src/guard.rs:126` (type-only keying), `:295-305` (blind clustering).
- **Change**: (a) walk pad nets through resistors summing series R, flag <1kΩ; (b) plumb GuardRingRequirement into `generate_cell` -> private `ring_complete` ring for injectors; (c) `find_sharing_clusters` takes `injectors: &HashSet<u32>` -> singleton clusters; split clusters straddling IsolationConstraint sides.
- **Payoff**: substrate injectors physically isolated before well-sharing ships.

### 1.12 Resistor width from tolerance equation (Eq 6.12) — M
- **What**: width = tier multiplier of material min; no `dR = sqrt(dRs² + (2dW/W)²)` solve; no sheet-tolerance/linewidth PDK fields.
- **Why**: AOAL ch06 6.3.1 — width is the only knob against linewidth variation; tier multiplier is a guess. Checklist 5.7 analog.
- **Where**: `crates/cell/src/resistor.rs:280-288`; `crates/core/src/pdk.rs:54-77` (AnalogParams).
- **Change**: add `sheet_tolerance`/`linewidth_control_um` HashMaps (mirrors `resistor_min_widths` pattern); tier -> tolerance target; `W >= 2*dW/sqrt(tol² - dRs²)`; take max with multiplier width; pelgrom_warning when unreachable.
- **Payoff**: resistor widths derived from spec, not folklore.

---

## 2. Placement

### 2.1 Soft-cell variant reshape op in SA — M (merges 2 verified findings)
- **What**: full-geometry variants generated (OPT-26, "so the placer can swap variants") but PlacementProblem is dims-only; SA has 3 ops (rotate/swap/insert), no reshape; `variant_dims` read only by one test fixture; `same_variant_required` + `FailureClass::MissingCellVariants` zero readers; passive variants estimate-only.
- **Why**: ALS 1.5 benchmark table — soft caps/passives drive compact-area results; ALS 3.5.2 variant matching. Checklist 5.4, 5.1.
- **Where**: `crates/placement/src/lib.rs:260-267` (problem), `:308-351` (place); `sa.rs:229-249` (perturb); `mosfet.rs:79-119`, `capacitor.rs:362`, `resistor.rs:586` (generation).
- **Change**: (1) `variants: &[Vec<(i64,i64)>]` on PlacementProblem; (2) `UndoRecord::Reshape{node,w,h,variant_idx}` + Op4 (~p=0.1, singleton islands); (3) apply atomically across matching group (`same_variant_required`); (4) report chosen variant index in PlacedLayout -> GDS emission selects matching CellVariant; (5) promote passive variants to full geometry.
- **Interacts**: needs cost 3.2 (variant-aware pin offsets) or HPWL lies during reshape moves.
- **Payoff**: biggest single area lever; activates already-built generation half.

### 2.2 Per-pair min-distance matrix (isolation survives legalization) — M
- **What**: isolation is soft-only (cost weight 4.0); PlacementStructure has ONE uniform `min_spacing` (ponytail comment admits gap); `build_hcg` applies single scalar per edge -> LP can legally violate what SA satisfied.
- **Why**: ALS 3.6.3 — well separation is a min-distance CONSTRAINT (d_well / 2·d_well), not a preference. Checklist 2.7, 4.4.
- **Where**: `crates/placement/src/lib.rs:196-198`, `:45`; `hcg.rs:250` (uniform edge), `:97-180` (LP enforces nothing pairwise); soft term `cost.rs:432-440`, `:884-897`.
- **Change**: `min_dist_pairs: Vec<(u32,u32,i64)>` on PlacementStructure, populated from isolation constraints + well assignments; in `build_hcg`, listed pairs with y-overlap get `edge.min_distance = w_left + pair_dist`.
- **Interacts**: build table ONCE, consume in both SA packing and HCG (2.3 shares it) — else SA and legalizer disagree and SA results get destroyed.
- **Payoff**: isolation guarantees hard end-to-end.

### 2.3 Device-class-aware packing snap — M
- **What**: packer sees pure (w,h) rects; one blanket 10_000-ang spacing everywhere; `place()` takes `_pdk` unused; PMOS nwell reaches cell edge -> real nwell-spacing DRC exposure at cell abutment.
- **Why**: ALS 2.4.3 step 3 — well/guard-ring spacing is class-pair-dependent. Checklist 1.1, 4.4.
- **Where**: `crates/placement/src/btree.rs:1113-1149` (pack), `lib.rs:45`, `:197-198`, `sa.rs:151-155`, `hcg.rs:225-259`.
- **Change**: opaque class index per cell (u8, derived from CellRecord.device_type at resolve — preserves no-analog-types boundary at `lib.rs:258`) + class-pair spacing table from Pdk; snap contour x/y outward on cross-class coverage; same table in HCG edge weights.
- **Interacts**: AFTER cell 1.9 (WPE halo shrinks needed table values); WITH 2.2 (same table mechanism, one implementation).
- **Payoff**: kills abutment nwell/guard DRC class without blanket over-spacing.

### 2.4 True mirror-image symmetric partners (FN orientation) — M
- **What**: partners are position-mirrored TRANSLATED COPIES — `expand_into` forces `Orientation::N` on all island members; pin offsets applied orientation-blind; breaks matched.rs pin mirror-symmetry assumption. (Do NOT add unconstrained-pair detection — symmetry.rs Pass 3 `:106-141` already does it.)
- **Why**: AOAL ch15 Step 5 — mirror image, not translation; asymmetric pin geometry = asymmetric routing parasitics on diff pairs. Checklist 1.3, 2.3.
- **Where**: `crates/placement/src/asf.rs:130-135`; `cost.rs:701-705`; `python/src/lib.rs:352-368` (translation-only emission).
- **Change**: assign FN/MY to partner in `symmetry_island`/`expand_into`; orientation-aware pin transform in cost (3.1); extend GDS emission AND routing pin extraction to honor orientation — three consumers, one flag.
- **Interacts**: cost 3.1 FIRST, then asf, then emission/routing together (partial rollout = mirrored HPWL fighting unmirrored geometry).
- **Payoff**: symmetric routing becomes achievable by construction; matched.rs assumption made true.

### 2.5 Hierarchical ASF-B*-tree (float intra-group packing) — L
- **What**: symmetry pairs pre-fused into rigid islands; `NodeType::Hierarchy`/`insert_hierarchy` is dead scaffolding (zero callers); island whitespace invisible to SA; >8-pair groups fall to naive stack (`asf.rs:178-189`).
- **Why**: ALS 1.4.2.1 Property 1.2 — symmetric-feasible trees let group shape flex during SA; mirroring-by-construction subsumes invariant check. Checklist 5.1, 5.4.
- **Where**: `crates/placement/src/asf.rs:59-104`, `:16-17` (ponytail note); `btree.rs:34-43`, `:212`.
- **Change**: per-group ASF-B*-tree over pair representatives in Hierarchy node's `asf_idx`; shared-contour pack, mirror partners at expand; group-local SA ops. Full whitespace recovery additionally needs Contour-node splicing.
- **Interacts**: AFTER 2.4 (mirroring semantics settled first); AFTER 2.1 (reps already exhaustively min-area packed at `asf.rs:177` — variant reshape delivers cheaper area first; this item's marginal gain is group aspect flexibility + big groups).
- **Payoff**: real but last — smallest gain per effort in this section.

---

## 3. Cost function

### 3.1 Orientation-aware pin transform in MoveGeom::resolve — S
- **What**: pin offsets (dx,dy) applied with no orientation transform — rotate op (w/h swap) and any future mirror produce wrong HPWL/alignment pin positions.
- **Why**: prerequisite for 2.4; also today's rotate op already lies about pin geometry. AOAL ch15 Step 5 / checklist 2.3.
- **Where**: `crates/placement/src/cost.rs:701-705`.
- **Change**: transform (dx,dy) through cell orientation (N/FN/R90...) against cell dims before HPWL/straight-net/alignment eval.
- **Interacts**: BEFORE 2.4. Independent win even alone.
- **Payoff**: HPWL/alignment see true pin positions; unblocks mirror placement.

### 3.2 Variant-aware pin offsets — S
- **What**: cost reads one pin_map; reshape op (2.1) changes pin positions per variant.
- **Why**: ALS 1.5 — soft-cell optimization needs cost to track the chosen shape or SA optimizes fiction.
- **Where**: `cost.rs` pin tables built at construction; `PlacementProblem` (lib.rs:260-267).
- **Change**: per-variant pin_map indexed by node's current variant_idx (updated in Reshape undo record).
- **Interacts**: ship WITH 2.1, not before/after.
- **Payoff**: reshape moves scored honestly.

### 3.3 Shared class-pair/min-dist table across SA cost, packer, HCG — M
- **What**: today isolation soft term (weight 4.0), packer blanket spacing, HCG uniform edges are THREE independent notions of "far enough" — guaranteed disagreement.
- **Why**: ALS 3.6.3 + 2.4.3; checklist 2.13 (placement must anticipate downstream reality — here its own legalizer).
- **Where**: `cost.rs:432-440/884-897`, `lib.rs:45,196-198`, `hcg.rs:250`.
- **Change**: single `SpacingModel` in PlacementStructure (per-pair sparse list + class-pair dense table); soft cost evaluates distance-to-requirement from it; packer snaps by it; HCG edges read it. One source of truth.
- **Interacts**: this IS the shared mechanism for 2.2 + 2.3 — implement as one unit.
- **Payoff**: SA optimum survives legalization; isolation/DRC spacing consistent end-to-end.

---

## Dependency graph (do-before)

```
1.3 MatchingType ─────────┐
1.8 ratioed detection ────┼─> 1.7 unit-finger folding
1.1 dummy tie-off ────────┼─> 1.10 moat extension (shared dummy geometry)
1.6 ring contacts ────────┼─> 1.4 depth widths (width of floating ring moot)
1.9 WPE physical ─────────┼─> 2.3 class spacing (table sized from residual exposure)
3.1 pin transform ────────┼─> 2.4 mirror FN ──> 2.5 hierarchical ASF
2.1 reshape op <=WITH=> 3.2 variant pins
2.2 + 2.3 <=WITH=> 3.3 (one SpacingModel)
```

Checklist alignment: roadmap closes Tier-1 gaps 1.4 (rings floating), Tier-2 gaps 2.6/2.9/2.10 (dummy tie, WPE, LOD all currently diagnostic-only fiction), 2.5/5.7 (ratio + sizing axis), and Tier-5 5.1/5.4 (soft cells). Recurring defect pattern across cell crate: diagnostics report geometry that emission never draws (WPE `:1400-1407`, SA/SB `:2109-2157`, ring `ring_complete:true`, dummy doc `:1252`) — every such fix should delete the fictitious diagnostic in the same commit.