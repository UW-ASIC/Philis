# Verification bug dispositions

False clean has higher severity than an extra marker. `Fixed` means a focused defect
is addressed on the documented baseline; it does not certify the surrounding capability.
`Partial` means the checked path improved but a compatibility/general-production gap
remains. `Integration pending` means code is not accepted on this baseline.

## Audited baseline register

| ID | Severity | Disposition | Finding / remaining boundary |
|---:|---|---|---|
| 1 | Critical | Fixed | constrained LVS series normalization reaches a safe fixpoint; general symmetric/cross-boundary reductions remain Wave 4 |
| 2 | Critical | Partial | signoff density/CMP checks explicit scope and exact rectangular union; legacy/general-polygon density remains incomplete |
| 3 | Critical | Partial | supported rectilinear LVS contact exact-rechecks bbox candidates; all-angle shared kernel remains Wave 2 |
| 4 | Critical | Partial | declared via decks bound cross-layer joins; explicit via-less legacy fallback remains risky compatibility behavior |
| 5 | Critical | Partial | signoff antenna uses extracted connectivity; legacy DRC CAR is not the tapeout gate |
| 6 | Critical | Partial | signoff antenna is PDK-driven/fail-closed; basic legacy rule remains simplified |
| 7 | Critical | Fixed | rule ID and kind are independent; duplicate IDs error; repeated kinds survive |
| 8 | High | Fixed | unknown/duplicate layer/connectivity/device/PEX refs and recognized-object fields error |
| 9 | High | Integration pending | exact union-based min-area correction belongs to Wave 3 fixes; fragment double-count false-clean blocks fan-in |
| 10 | High | Integration pending | general must-overlap semantics still require accepted Wave 3 exact fix; bbox-only absence can false-clean |
| 11 | High | Open | general-polygon extension/max-width/PRL/wide-spacing/asymmetric-enclosure heuristics remain incomplete |
| 12 | High | Integration pending | nested same-polarity material must not become a hole for enclosed-area/cheesing; fix/review pending |
| 13 | High | Integration pending | complete bounded coloring exists only on pending stream and has stitch/search-limit review defects |
| 14 | High | Partial | production records/matcher are body-aware; real layout adapter and full body/well extraction/correlation remain |
| 15 | High | Partial | production named-net seed machinery exists; end-to-end GDS label binding/correlation awaits adapter |
| 16 | High | Partial | abstract production hierarchy/binding exists; real child evidence adapter and full mixed hierarchy remain |
| 17 | High | Integration pending | exact derived-layer integration is on pending Wave 3 stream with one-DBU offset defect |
| 18 | High | Fixed | global-net remapping updates MOS terminals rather than leaving stale net IDs |
| 19 | High | Fixed | `extract_netlist` inherits `deck.lvs_cut_required` |
| 20 | High | Fixed | `run_lvs` enforces deck strict/floating policy; low-level `compare` intentionally has no deck policy |
| 21 | High | Fixed | ERC reports extraction error instead of an apparently clean empty result |
| 22 | High | Fixed | MIM capacitance converts nm²/µm² correctly rather than applying a 10^6 error |
| 23 | High | Fixed | supported rectilinear conductors receive both R and ground C; 386 aF breakdown is pinned |
| 24 | High | Open | per-net resistance remains scalar, not a distributed terminal-aware solve |
| 25 | High | Open | interlayer C lacks calibrated adjacency/dielectric/shield semantics and exact 3D geometry |
| 26 | Medium | Fixed | hard-coded fill/shield multipliers removed; effects stay explicitly unmodeled until process data exists |
| 27 | High | Fixed | floating-metal sentinel attribution is retained for capacitance reporting |
| 28 | High | Fixed | port-net lumped R maps a loaded internal node; endpointless scalar R is omitted with diagnostic |
| 29 | High | Partial | checked/lossless GDS now preserves records/properties/hierarchy and validates units; production evidence adapter remains pending |
| 30 | High | Open | complete round/diagonal/odd/negative PATH semantics and all-angle stroking remain Wave 2 |
| 31 | High | Fixed | backend final DRC no longer globally waives legacy density; richer family remains non-clean until configured |
| 32 | High | Fixed | duplicate/out-of-range GDS layer/datatype aliases are rejected |
| 33 | High | Fixed | conflicting MOS type/flavor markers stop extraction |
| 34 | High | Fixed | power solver residual scaling handles tiny IC loads; 1 pA × 1 MΩ regression is pinned |
| 35 | High | Fixed | non-covering full-window density configuration errors or explicitly uses partial windows |
| 36 | High | Fixed | nonconductive antenna diode markers require one exact net attribution; zero/multiple is error |
| 37 | High | Fixed | combined ESD/latch-up requires evidence for both halves; one configured half cannot yield clean |

## Batch A Wave 3 review blockers

These seven defects must be fixed with minimal reproducers, negative-direction tests,
full gates and independent re-review before the Wave 3 commits enter fan-in:

| ID | Blocking behavior | Required regression |
|---|---|---|
| W3-A | truncated derived offset midpoint loses 1-DBU cells | one-DBU positive/negative offset and exact set comparison |
| W3-B | same-color precolored stitch not selected/charged; incomplete bounded search may return nonoptimal success | stitch-cost optimum and search-limit distinction against brute force |
| W3-C | production adapter can drop invalid `__geometry__` markers and report clean | invalid geometry remains blocking through checked facade |
| W3-D | revoked/empty waiver set leaves stale dispositions | apply, revoke, empty and rerun lifecycle |
| W3-E | hierarchy invalidation misses ancestor edits | ancestor edit invalidates every affected descendant/result |
| W3-F | legacy overlap bbox false-clean and nested material misread as holes | missing counterpart, concave overlap and same-polarity nesting cases |
| W3-G | density schema accepts `min > max` or out-of-range ratios | invalid ranges fail deck construction |

## Disposition rules

Close a row only in the same review as its reproducer, negative test and focused/full
gate evidence. If a changed expected result is physical, record its units and mechanism.
Never close a general family because one hand-authored case passes. Score changes follow
the independent evidence rule in the [capability matrix](capability-matrix.md).
