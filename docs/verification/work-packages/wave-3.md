# Wave 3 work packages — production DRC

Goal: load and execute every operation used by one selected foundry deck, with exact
marker geometry and deterministic debug artifacts. The reviewed Batch A subset is an
accepted `partial`/`foundation` base on `f646d9c`: its review blockers are closed,
but selected-deck inventory, calibrated process data, golden markers and independent
correlation are absent, so no package meets its exit yet.

## D3.1 — derived layers, variables and production deck schema

| Field | Requirement |
|---|---|
| Read first | [`production.rs`](../../../verify/src/drc/production.rs), [`derived.rs`](../../../verify/src/drc/derived.rs), [`params.rs`](../../../verify/src/params.rs), exact geometry |
| Prerequisites | G2.1 accepted and `EXT-DECK` satisfied |
| Owned files | accepted `verify/src/drc/{production,derived}.rs`; DRC-owned sections of `verify/src/schema.rs` and `verify/src/params.rs`; their inline parser/derived tests |
| Forbidden files | `verify/src/drc/mod.rs` execution kernels, `verify/src/{lvs,pex,signoff}/**`, `verify/conformance/manifest.json`, and `verify/correlation/**` |
| API outcome | versioned typed AST for boolean/sizing expressions, measurement variables, units, tables, predicates and contexts; unsupported operation stops deck loading |
| Tasks | exact derived expressions; rational dimensional evaluation; dependency cycle detection; table monotonicity/range validation; stable rule/model IDs; source spans and capability declaration |
| Unsupported/non-goals | proprietary syntax not in selected inventory stays explicit; no stringly evaluated expressions or unit coercion |
| Tests | valid nested expressions; unknown op/layer/property/unit and cycle negatives; equality/1-DBU table boundaries; adversarial deep graphs, overflow and duplicate IDs |
| Focused/full gates | schema/params/derived tests, then global gate and selected-deck load audit |
| Acceptance/evidence | every selected operation maps to schema+implementation owner+negative test; no parsed-unused fields |
| Score promotion | only after golden-correlated markers for each operation |
| Parallel/fan-in hazards | schema lands before execution packages; coordinate shared typed measurement API once |

## D3.2 — exact context-aware rule execution

| Field | Requirement |
|---|---|
| Read first | [`drc/mod.rs`](../../../verify/src/drc/mod.rs), D3.1 AST, G2.1 kernel, hierarchy index |
| Prerequisites | G2.1, G2.4 and D3.1 accepted; `EXT-DECK` satisfied |
| Owned files | `verify/src/drc/mod.rs`, planned `verify/src/drc/rules/**`, `verify/conformance/generator/drc_gen.rs`, and DRC marker cases under planned `verify/correlation/corpus/drc/**` |
| Forbidden files | `verify/src/geometry/**`, `verify/src/{lvs,pex,signoff}/**`, `verify/conformance/manifest.json`, and correlation dispositions under `verify/correlation/**/dispositions*.json` |
| API outcome | exact conditional/table-driven width, spacing, PRL, EOL, enclosure, cut, density and antenna results with same/different-net, voltage, region, cell and hierarchy contexts |
| Tasks | common measurement objects; exact union/hole semantics; context provider interfaces; cut classes/via arrays/min-cut; rule-reach declaration for tiling; physical marker rings and values |
| Unsupported/non-goals | advanced-node families absent from selected deck remain rejected at load; no approximate clean under missing connectivity/voltage context |
| Tests | per-context positive/negative pairs; exact table boundaries; concave/hole/seam/angle cases; adversarial missing context, overlapping fragments and equivalent hierarchy paths |
| Focused/full gates | kernel filters + generated matrix, global gate, flat/hier/tiled equivalence |
| Acceptance/evidence | selected deck operation matrix has negative and golden marker case including measurement/units/path |
| Score promotion | per family only after independent golden correlation |
| Parallel/fan-in hazards | partition by rule family only after shared measurement/context interfaces merge |

## D3.3 — complete bounded decomposition solver

| Field | Requirement |
|---|---|
| Read first | accepted [`coloring.rs`](../../../verify/src/drc/coloring.rs) and its tests, G2.1 contact API |
| Prerequisites | G2.1, D3.1 and D3.2 accepted; `BA-W3-REVIEW` satisfied for the existing solver fixes |
| Owned files | accepted `verify/src/drc/coloring.rs` and coloring-only fixtures under planned `verify/correlation/corpus/drc/coloring/**` |
| Forbidden files | `verify/src/drc/{deck,derived,results,invalidation,scheduler}.rs`, `verify/src/geometry/**`, `verify/conformance/manifest.json`, and every fallback that converts `SearchLimit` into a proof |
| API outcome | deterministic complete bounded solver with precolors, legal stitches, objective/cost, conflict witness, and distinct `SearchLimit` |
| Tasks | canonical graph; constraint propagation; exhaustive/branch-and-bound completeness within bounds; stitch accounting; unsat core/conflict geometry; stable tie-breaks |
| Unsupported/non-goals | unbounded full-chip solve; if bounds exhaust, return `SearchLimit`, never clean/violation proof |
| Tests | colorable/noncolorable/precolored/stitch cases; optimality boundaries; adversarial graph order, symmetric graphs, timeout/node limit and same-color stitch regression |
| Focused/full gates | solver property/differential tests, DRC/global gates, thread-order replay |
| Acceptance/evidence | brute-force agreement on bounded random graphs; reproducible witness maps to geometry |
| Score promotion | only after selected-deck decomposition marker correlation |
| Parallel/fan-in hazards | solver independent after graph contract; marker integration waits for D3.2 result identity |

## D3.4 — fill generation and calibrated CMP

| Field | Requirement |
|---|---|
| Read first | [`signoff/density_cmp.rs`](../../../verify/src/signoff/density_cmp.rs), DRC density, G2.1 booleans |
| Prerequisites | G2.1, G2.4, D3.1 and D3.2 accepted; `EXT-DECK` and `EXT-PROCESS` satisfied for fill/CMP models |
| Owned files | accepted `verify/src/drc/fill.rs`; planned `verify/src/drc/cmp.rs`; CMP/fill-owned schema records in `verify/src/schema.rs`; planned `verify/correlation/corpus/drc/{fill,cmp}/**` |
| Forbidden files | `verify/src/signoff/density_cmp.rs` except through a separately reviewed shared-model API commit, `verify/src/{lvs,pex}/**`, `verify/conformance/manifest.json`, and hard-coded process constants anywhere |
| API outcome | deterministic fill proposal/generation with exclusions and connectivity policy; multilevel calibrated CMP prediction with provenance |
| Tasks | legal fill cells/patterns; keepouts/critical-net/context exclusions; exact density windows/gradients; iterative levels and thickness transfer; before/after artifacts |
| Unsupported/non-goals | generic model coefficients; absent calibration is `NOT_RUN`/`ERROR`, never default clean |
| Tests | fill reaches legal window; forbidden regions untouched; min/max boundary; adversarial narrow exclusions, hierarchy seams, overlapping shapes and unstable iterations |
| Focused/full gates | fill/CMP analytic tests, deterministic output hashes, global gate |
| Acceptance/evidence | foundry model revision and calibration structures correlate within accepted limits |
| Score promotion | only with independent CMP/fill correlation |
| Parallel/fan-in hazards | generator and model can split after shared exact density contract; coordinate output ownership |

## D3.5 — result database, waivers, incremental/distributed execution

| Field | Requirement |
|---|---|
| Read first | accepted [`results.rs`](../../../verify/src/drc/results.rs), [`hierarchy_index.rs`](../../../verify/src/hierarchy_index.rs), correlation disposition logic |
| Prerequisites | G2.4, D3.1 and D3.2 accepted |
| Owned files | accepted `verify/src/drc/results.rs`; planned scheduler/restart modules; DRC result/waiver/invalidation tests in those files |
| Forbidden files | `verify/src/bin/correlation.rs`, `verify/correlation/schemas/**`, `verify/src/{geometry,lvs,pex,signoff}/**`, and checked-in wildcard or unowned disposition files |
| API outcome | stable marker DB and fingerprints/provenance, ancestor-aware invalidation, restartable deterministic distributed tile execution |
| Tasks | canonical geometry fingerprints; source/deck/model hashes; revoke/expiry clearing; dependency DAG; content-addressed tile artifacts; deterministic fan-in and failure recovery |
| Unsupported/non-goals | distributed transport vendor choice may stay adapter-level; partial worker failure cannot yield clean |
| Tests | waiver apply/revoke/expiry; ancestor/derived dependency edits; seam dedupe; crash/restart; adversarial reorder/duplicate/stale worker results |
| Focused/full gates | DB/invalidation tests, distributed simulation, global gate, deterministic hashes across worker counts |
| Acceptance/evidence | full and incremental results byte-equivalent; every marker cross-probes and every waiver has exact provenance |
| Score promotion | infrastructure alone does not promote score; capacity/correlation evidence handled in Wave 7 |
| Parallel/fan-in hazards | wait for final identity schema; DB and scheduler can split only across a frozen artifact contract |

## Wave 3 exit

Every operation used by the selected deck has schema, exact implementation, negative
test and independently correlated golden marker. Unsupported syntax stops loading.
The DRC target is at least 38/44 only after correlation; foundry qualification remains
an external Wave 7 decision.
