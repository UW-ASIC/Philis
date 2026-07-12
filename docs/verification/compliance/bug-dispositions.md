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
| 9 | High | Fixed | exact union-based min-area prevents fragment double counting on the supported rectilinear path; all-angle migration remains Wave 2/3 |
| 10 | High | Fixed | must-overlap now requires an exact counterpart and rejects concave bbox-only contact; broader rule-language context remains |
| 11 | High | Open | general-polygon extension/max-width/PRL/wide-spacing/asymmetric-enclosure heuristics remain incomplete |
| 12 | High | Fixed | nested same-polarity material is not treated as a hole; paired contained opposite-polarity keyholes remain supported |
| 13 | High | Fixed | stitch cost is charged and bounded-search exhaustion returns `SearchLimit`, never an unproven optimum; production geometry integration and lithography/yield remain |
| 14 | High | Partial | production records/matcher and the checked adapter preserve configured four-terminal MOS body/well evidence; foundry-complete extraction and independent correlation remain |
| 15 | High | Fixed | checked GDS text/property evidence binds named nets end to end and ambiguity fails closed; independent general-input topology correlation remains a qualification gap |
| 16 | High | Partial | checked nested SREF/AREF hierarchy, explicit child-port maps, black boxes and equated cells are wired to production comparison; cross-boundary reductions, arbitrary mixed hierarchy and full-chip capacity remain |
| 17 | High | Fixed | exact rectilinear derived layers preserve one-DBU positive/negative offsets; arbitrary-angle boolean/offset remains unsupported |
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
| 29 | High | Fixed | checked/lossless GDS preserves records/properties/hierarchy and units through the accepted production evidence adapter; undeclared semantics fail closed |
| 30 | High | Open | complete round/diagonal/odd/negative PATH semantics and all-angle stroking remain Wave 2 |
| 31 | High | Fixed | backend final DRC no longer globally waives legacy density; richer family remains non-clean until configured |
| 32 | High | Fixed | duplicate/out-of-range GDS layer/datatype aliases are rejected |
| 33 | High | Fixed | conflicting MOS type/flavor markers stop extraction |
| 34 | High | Fixed | power solver residual scaling handles tiny IC loads; 1 pA × 1 MΩ regression is pinned |
| 35 | High | Fixed | non-covering full-window density configuration errors or explicitly uses partial windows |
| 36 | High | Fixed | nonconductive antenna diode markers require one exact net attribution; zero/multiple is error |
| 37 | High | Fixed | combined ESD/latch-up requires evidence for both halves; one configured half cannot yield clean |

## Batch A Wave 3 review blockers

The original seven defects and the later independent-review findings below are fixed on baseline
`f646d9c5b8b17fbab775a25f0fc499a873678add` with minimal reproducers,
negative/boundary tests, full gates and independent re-review. Closing these defects
accepts the documented subset; it does not complete the Wave 3 exit criteria.

| ID | Disposition | Blocking behavior | Required regression |
|---|---|---|---|
| W3-A | Fixed on `f646d9c` | truncated derived offset midpoint loses 1-DBU cells | one-DBU positive/negative offset and exact set comparison |
| W3-B | Fixed on `f646d9c` | same-color precolored stitch not selected/charged; incomplete bounded search may return nonoptimal success | stitch-cost optimum and search-limit distinction against brute force |
| W3-C | Fixed on `f646d9c` | production adapter can drop invalid `__geometry__` markers and report clean | invalid geometry remains blocking through checked facade |
| W3-D | Fixed on `f646d9c` | revoked/empty waiver set leaves stale dispositions | apply, revoke, empty and rerun lifecycle |
| W3-E | Fixed on `f646d9c` | hierarchy invalidation misses ancestor edits | ancestor edit invalidates every affected descendant/result |
| W3-F | Fixed on `f646d9c` | legacy overlap bbox false-clean and nested material misread as holes | missing counterpart, concave overlap and same-polarity nesting cases |
| W3-G | Fixed on `f646d9c` | density schema accepts `min > max` or out-of-range ratios | invalid ranges fail deck construction |
| W3-H | Fixed on `f646d9c` | point/nonadjacent contacts and external, disjoint or same-polarity retraced lobes can enter exact consumers | paired-keyhole positives plus each malformed-contact negative in both orientations |
| W3-I | Fixed on `f646d9c` | multi-keyhole decomposition depends on orientation or unbounded boundary work | reversed/multiple-keyhole equivalence and vertex/pair-work capacity failures |
| W3-J | Fixed on `f646d9c` | full-`i32` geometry, rule limits and repair arithmetic can panic, wrap or false-clean | translated extrema, maximum limits and typed repair/capacity regressions |
| W3-K | Fixed on `f646d9c` | density/fill/CMP windows overflow near `i32::MAX` or run unbounded at step 1 | origin/MAX equivalence plus exact 16M cap and cap+1 failure |
| W3-L | Fixed on `f646d9c` | exact preflight changes collapsed compatibility-GDS zero-area evidence | retained zero-width boundary remains a blocking zero-area marker |

## Disposition rules

Close a row only in the same review as its reproducer, negative test and focused/full
gate evidence. If a changed expected result is physical, record its units and mechanism.
Never close a general family because one hand-authored case passes. Score changes follow
the independent evidence rule in the [capability matrix](capability-matrix.md).
